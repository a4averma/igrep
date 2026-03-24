use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use regex::{Regex, RegexBuilder};

use crate::index::reader::IndexReader;
use crate::query::regex_to_query;
use crate::types::SearchResult;

pub fn search(reader: &IndexReader, pattern: &str, root: &Path) -> Result<Vec<SearchResult>> {
    let query = regex_to_query(pattern)?;
    let candidates = reader.evaluate_query(&query);
    let re = Regex::new(pattern)?;

    let mut results = Vec::new();

    for doc_id in candidates {
        let Some(rel_path) = reader.doc_id_to_path(doc_id) else {
            continue;
        };

        let full_path = root.join(rel_path);
        if !full_path.exists() {
            continue;
        }

        let content = match fs::read_to_string(&full_path) {
            Ok(c) => c,
            Err(_) => match fs::read(&full_path) {
                Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
                Err(_) => continue,
            },
        };

        for (line_num, line) in content.lines().enumerate() {
            if re.is_match(line) {
                results.push(SearchResult {
                    file_path: rel_path.to_path_buf(),
                    line_number: line_num + 1,
                    line_content: line.to_string(),
                });
            }
        }
    }

    results.sort_by(|a, b| {
        a.file_path
            .cmp(&b.file_path)
            .then(a.line_number.cmp(&b.line_number))
    });

    Ok(results)
}

pub fn search_files_only(reader: &IndexReader, pattern: &str, root: &Path) -> Result<Vec<PathBuf>> {
    let query = regex_to_query(pattern)?;
    let candidates = reader.evaluate_query(&query);
    let re = Regex::new(pattern)?;

    let mut matching_files = Vec::new();

    for doc_id in candidates {
        let Some(rel_path) = reader.doc_id_to_path(doc_id) else {
            continue;
        };

        let full_path = root.join(rel_path);
        if !full_path.exists() {
            continue;
        }

        let content = match fs::read_to_string(&full_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        if content.lines().any(|line| re.is_match(line)) {
            matching_files.push(rel_path.to_path_buf());
        }
    }

    matching_files.sort();
    Ok(matching_files)
}

pub fn search_with_options(
    reader: &IndexReader,
    pattern: &str,
    root: &Path,
    case_insensitive: bool,
) -> Result<Vec<SearchResult>> {
    let query = regex_to_query(pattern)?;
    let candidates = reader.evaluate_query(&query);
    let re = RegexBuilder::new(pattern)
        .case_insensitive(case_insensitive)
        .build()?;

    let mut results = Vec::new();

    for doc_id in candidates {
        let Some(rel_path) = reader.doc_id_to_path(doc_id) else {
            continue;
        };

        let full_path = root.join(rel_path);
        if !full_path.exists() {
            continue;
        }

        let content = match fs::read_to_string(&full_path) {
            Ok(c) => c,
            Err(_) => match fs::read(&full_path) {
                Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
                Err(_) => continue,
            },
        };

        for (line_num, line) in content.lines().enumerate() {
            if re.is_match(line) {
                results.push(SearchResult {
                    file_path: rel_path.to_path_buf(),
                    line_number: line_num + 1,
                    line_content: line.to_string(),
                });
            }
        }
    }

    results.sort_by(|a, b| {
        a.file_path
            .cmp(&b.file_path)
            .then(a.line_number.cmp(&b.line_number))
    });

    Ok(results)
}

pub fn search_files_with_options(
    reader: &IndexReader,
    pattern: &str,
    root: &Path,
    case_insensitive: bool,
) -> Result<Vec<PathBuf>> {
    let query = regex_to_query(pattern)?;
    let candidates = reader.evaluate_query(&query);
    let re = RegexBuilder::new(pattern)
        .case_insensitive(case_insensitive)
        .build()?;

    let mut matching_files = Vec::new();

    for doc_id in candidates {
        let Some(rel_path) = reader.doc_id_to_path(doc_id) else {
            continue;
        };

        let full_path = root.join(rel_path);
        if !full_path.exists() {
            continue;
        }

        let content = match fs::read_to_string(&full_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        if content.lines().any(|line| re.is_match(line)) {
            matching_files.push(rel_path.to_path_buf());
        }
    }

    matching_files.sort();
    Ok(matching_files)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::index::{reader::IndexReader, writer::IndexWriter};
    use crate::types::IndexConfig;
    use crate::walker::walk;

    fn fixtures_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("test-fixtures")
    }

    fn build_fixture_index() -> (tempfile::TempDir, IndexReader, PathBuf) {
        let root = fixtures_dir();
        let config = IndexConfig::default();
        let files = walk(&root, &config).unwrap();

        let mut writer = IndexWriter::new(config);
        for (doc_id, path) in files {
            let bytes = fs::read(&path).unwrap();
            let relative = path.strip_prefix(&root).unwrap_or(path.as_path());
            writer.add_file(doc_id, &relative.to_string_lossy(), &bytes);
        }

        let index_dir = tempfile::tempdir().unwrap();
        writer.finish(index_dir.path()).unwrap();
        let reader = IndexReader::open(index_dir.path()).unwrap();
        (index_dir, reader, root)
    }

    fn contains_file(paths: &[PathBuf], suffix: &str) -> bool {
        paths.iter().any(|p| p.ends_with(Path::new(suffix)))
    }

    #[test]
    fn search_finds_fn_main_in_simple_rs() {
        let (_index_dir, reader, root) = build_fixture_index();

        let results = search(&reader, "fn main", &root).unwrap();
        assert!(!results.is_empty());
        assert!(
            results
                .iter()
                .any(|r| r.file_path.ends_with(Path::new("simple.rs"))),
            "expected simple.rs in results"
        );
    }

    #[test]
    fn search_finds_todo_markers() {
        let (_index_dir, reader, root) = build_fixture_index();

        let results = search(&reader, "TODO", &root).unwrap();
        assert!(!results.is_empty());

        let files: Vec<PathBuf> = results.iter().map(|r| r.file_path.clone()).collect();
        assert!(contains_file(&files, "simple.rs"));
        assert!(contains_file(&files, "nested/deep/module.rs"));
    }

    #[test]
    fn search_returns_empty_for_nonexistent_pattern() {
        let (_index_dir, reader, root) = build_fixture_index();

        let results = search(&reader, "nonexistent_string_xyz_123", &root).unwrap();
        assert!(results.is_empty());

        let files_only = search_files_only(&reader, "nonexistent_string_xyz_123", &root).unwrap();
        assert!(files_only.is_empty());
    }

    #[test]
    fn search_matches_struct_regex() {
        let (_index_dir, reader, root) = build_fixture_index();

        let results = search(&reader, r"struct\s+\w+", &root).unwrap();
        assert!(!results.is_empty());
        assert!(
            results
                .iter()
                .any(|r| r.file_path.ends_with(Path::new("patterns.rs"))),
            "expected patterns.rs in struct regex results"
        );
    }

    #[test]
    fn search_files_only_matches_pub_fn_in_multiple_rust_files() {
        let (_index_dir, reader, root) = build_fixture_index();

        let files = search_files_only(&reader, "pub fn", &root).unwrap();
        assert!(files.len() >= 2, "expected multiple matching rust files");
        assert!(contains_file(&files, "patterns.rs"));
        assert!(contains_file(&files, "regex_patterns.rs"));

        for window in files.windows(2) {
            assert!(window[0] <= window[1], "results must be sorted");
        }
    }

    #[test]
    fn case_insensitive_search_finds_all_casings() {
        let (_index_dir, reader, root) = build_fixture_index();

        let results = search_with_options(&reader, "foobar", &root, true).unwrap();
        let matched_lines: Vec<&str> = results.iter().map(|r| r.line_content.as_str()).collect();

        assert!(
            results
                .iter()
                .any(|r| r.file_path.ends_with(Path::new("case_test.rs"))),
            "expected case_test.rs in results"
        );

        assert!(
            matched_lines.iter().any(|l| l.contains("FooBar")),
            "case-insensitive search for 'foobar' should match 'FooBar'"
        );
        assert!(
            matched_lines.iter().any(|l| l.contains("foobar")),
            "case-insensitive search for 'foobar' should match 'foobar'"
        );
        assert!(
            matched_lines.iter().any(|l| l.contains("FOOBAR")),
            "case-insensitive search for 'foobar' should match 'FOOBAR'"
        );
    }

    #[test]
    fn case_insensitive_uppercase_pattern_finds_all_casings() {
        let (_index_dir, reader, root) = build_fixture_index();

        let results = search_with_options(&reader, "FOOBAR", &root, true).unwrap();

        assert!(
            results.iter().any(|r| r.line_content.contains("FooBar")),
            "case-insensitive 'FOOBAR' should match 'FooBar'"
        );
        assert!(
            results.iter().any(|r| r.line_content.contains("foobar")),
            "case-insensitive 'FOOBAR' should match 'foobar'"
        );
    }

    #[test]
    fn case_sensitive_search_excludes_wrong_casing() {
        let (_index_dir, reader, root) = build_fixture_index();

        let results = search_with_options(&reader, "foobar", &root, false).unwrap();
        let case_test_results: Vec<&SearchResult> = results
            .iter()
            .filter(|r| r.file_path.ends_with(Path::new("case_test.rs")))
            .collect();

        for r in &case_test_results {
            assert!(
                r.line_content.contains("foobar"),
                "case-sensitive 'foobar' should not match line: {}",
                r.line_content
            );
        }
    }

    #[test]
    fn case_sensitive_search_finds_exact_match() {
        let (_index_dir, reader, root) = build_fixture_index();

        let results = search_with_options(&reader, "FooBar", &root, false).unwrap();
        assert!(
            results
                .iter()
                .any(|r| r.file_path.ends_with(Path::new("case_test.rs"))
                    && r.line_content.contains("FooBar")),
            "case-sensitive 'FooBar' should match 'FooBar' in case_test.rs"
        );
    }

    #[test]
    fn case_insensitive_files_with_options() {
        let (_index_dir, reader, root) = build_fixture_index();

        let files = search_files_with_options(&reader, "foobar", &root, true).unwrap();
        assert!(
            contains_file(&files, "case_test.rs"),
            "case-insensitive file search should find case_test.rs"
        );
    }
}
