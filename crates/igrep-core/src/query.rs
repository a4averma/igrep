use std::collections::BTreeSet;

use anyhow::Error;
use regex_syntax::{
    hir::{Class, Hir, HirKind},
    parse,
};

use crate::{
    ngram::{build_covering_ngrams, hash_ngram},
    types::{IndexConfig, NgramHash, Query, QueryOp},
};

const SMALL_CLASS_ENUM_LIMIT: usize = 10;

pub fn regex_to_query(pattern: &str) -> Result<Query, Error> {
    if pattern.is_empty() {
        return Ok(all_query());
    }

    let hir = parse(pattern)?;
    let config = IndexConfig::default();
    Ok(analyze_hir(&hir, &config))
}

fn analyze_hir(hir: &Hir, config: &IndexConfig) -> Query {
    match hir.kind() {
        HirKind::Empty | HirKind::Look(_) => all_query(),
        HirKind::Literal(lit) => literal_query(&lit.0, config),
        HirKind::Capture(capture) => analyze_hir(&capture.sub, config),
        HirKind::Concat(parts) => {
            and_query(parts.iter().map(|part| analyze_hir(part, config)).collect())
        }
        HirKind::Alternation(alts) => {
            or_query(alts.iter().map(|alt| analyze_hir(alt, config)).collect())
        }
        HirKind::Repetition(rep) => {
            if rep.min == 0 {
                all_query()
            } else {
                analyze_hir(&rep.sub, config)
            }
        }
        HirKind::Class(class) => class_query(class, config),
    }
}

fn class_query(class: &Class, config: &IndexConfig) -> Query {
    if let Some(literal) = class.literal() {
        return literal_query(&literal, config);
    }

    match enumerate_small_class(class, SMALL_CLASS_ENUM_LIMIT) {
        Some(literals) if literals.is_empty() => none_query(),
        Some(literals) if literals.len() == 1 => literal_query(&literals[0], config),
        Some(literals) => or_query(
            literals
                .iter()
                .map(|literal| literal_query(literal, config))
                .collect(),
        ),
        None => all_query(),
    }
}

fn enumerate_small_class(class: &Class, limit: usize) -> Option<Vec<Vec<u8>>> {
    let mut literals = Vec::new();

    match class {
        Class::Bytes(bytes) => {
            for range in bytes.ranges() {
                for b in range.start()..=range.end() {
                    literals.push(vec![b]);
                    if literals.len() > limit {
                        return None;
                    }
                }
            }
        }
        Class::Unicode(unicode) => {
            for range in unicode.ranges() {
                for cp in u32::from(range.start())..=u32::from(range.end()) {
                    let ch = char::from_u32(cp)?;
                    let mut encoded = [0_u8; 4];
                    literals.push(ch.encode_utf8(&mut encoded).as_bytes().to_vec());
                    if literals.len() > limit {
                        return None;
                    }
                }
            }
        }
    }

    Some(literals)
}

fn literal_query(literal: &[u8], config: &IndexConfig) -> Query {
    let mut hashes = BTreeSet::<NgramHash>::new();
    for ngram in build_covering_ngrams(literal, config) {
        hashes.insert(hash_ngram(&ngram));
    }

    if hashes.is_empty() {
        all_query()
    } else {
        Query {
            op: QueryOp::And,
            trigrams: hashes.into_iter().collect(),
            children: vec![],
        }
    }
}

fn and_query(children: Vec<Query>) -> Query {
    let mut flattened_children = Vec::new();
    let mut trigrams = BTreeSet::new();

    for child in children.into_iter().map(simplify_query) {
        match child.op {
            QueryOp::None => return none_query(),
            QueryOp::All => {}
            QueryOp::And if child.children.is_empty() => {
                for trigram in child.trigrams {
                    trigrams.insert(trigram);
                }
            }
            _ => flattened_children.push(child),
        }
    }

    if flattened_children.is_empty() && trigrams.is_empty() {
        return all_query();
    }

    if flattened_children.len() == 1 && trigrams.is_empty() {
        return flattened_children.pop().unwrap();
    }

    Query {
        op: QueryOp::And,
        trigrams: trigrams.into_iter().collect(),
        children: flattened_children,
    }
}

fn or_query(children: Vec<Query>) -> Query {
    let mut flattened_children = Vec::new();

    for child in children.into_iter().map(simplify_query) {
        match child.op {
            QueryOp::All => return all_query(),
            QueryOp::None => {}
            QueryOp::Or if child.trigrams.is_empty() => {
                flattened_children.extend(child.children);
            }
            _ => flattened_children.push(child),
        }
    }

    if flattened_children.is_empty() {
        return none_query();
    }

    if flattened_children.len() == 1 {
        return flattened_children.pop().unwrap();
    }

    Query {
        op: QueryOp::Or,
        trigrams: vec![],
        children: flattened_children,
    }
}

fn simplify_query(query: Query) -> Query {
    if query.children.is_empty() {
        return match query.op {
            QueryOp::And if query.trigrams.is_empty() => all_query(),
            QueryOp::Or if query.trigrams.is_empty() => none_query(),
            _ => query,
        };
    }

    match query.op {
        QueryOp::And => {
            let mut children = query.children;
            if !query.trigrams.is_empty() {
                children.push(Query {
                    op: QueryOp::And,
                    trigrams: query.trigrams,
                    children: vec![],
                });
            }
            and_query(children)
        }
        QueryOp::Or => or_query(query.children),
        _ => query,
    }
}

fn all_query() -> Query {
    Query {
        op: QueryOp::All,
        trigrams: vec![],
        children: vec![],
    }
}

fn none_query() -> Query {
    Query {
        op: QueryOp::None,
        trigrams: vec![],
        children: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect_hashes(query: &Query, out: &mut BTreeSet<NgramHash>) {
        out.extend(query.trigrams.iter().copied());
        for child in &query.children {
            collect_hashes(child, out);
        }
    }

    fn literal_hashes(s: &str) -> BTreeSet<NgramHash> {
        let config = IndexConfig::default();
        build_covering_ngrams(s.as_bytes(), &config)
            .into_iter()
            .map(|ngram| hash_ngram(&ngram))
            .collect()
    }

    #[test]
    fn literal_builds_and_query_with_expected_hashes() {
        let query = regex_to_query("foo").unwrap();
        assert_eq!(query.op, QueryOp::And);

        let mut actual = BTreeSet::new();
        collect_hashes(&query, &mut actual);
        assert_eq!(actual, literal_hashes("foo"));
    }

    #[test]
    fn alternation_builds_or_query() {
        let query = regex_to_query("foo|bar").unwrap();
        assert_eq!(query.op, QueryOp::Or);
        assert_eq!(query.children.len(), 2);
    }

    #[test]
    fn concat_with_wildcard_ands_literal_constraints() {
        let query = regex_to_query("foo.*bar").unwrap();
        assert_eq!(query.op, QueryOp::And);

        let mut actual = BTreeSet::new();
        collect_hashes(&query, &mut actual);

        let mut expected = literal_hashes("foo");
        expected.extend(literal_hashes("bar"));
        assert!(expected.is_subset(&actual));
    }

    #[test]
    fn fully_wildcard_pattern_is_all() {
        let query = regex_to_query(".*").unwrap();
        assert_eq!(query.op, QueryOp::All);
    }

    #[test]
    fn empty_pattern_is_all() {
        let query = regex_to_query("").unwrap();
        assert_eq!(query.op, QueryOp::All);
    }

    #[test]
    fn longer_literal_generates_multiple_hashes() {
        let query = regex_to_query("MAX_FILE_SIZE").unwrap();
        assert_eq!(query.op, QueryOp::And);

        let mut actual = BTreeSet::new();
        collect_hashes(&query, &mut actual);
        assert!(actual.len() >= 2);
    }

    #[test]
    fn small_class_then_literal_is_decomposable() {
        let query = regex_to_query("[abc]def").unwrap();
        assert_ne!(query.op, QueryOp::None);
    }

    #[test]
    fn large_class_falls_back_to_all() {
        let query = regex_to_query("[a-zA-Z0-9_]").unwrap();
        assert_eq!(query.op, QueryOp::All);
    }
}
