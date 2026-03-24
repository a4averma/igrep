use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use memmap2::Mmap;

use crate::posting::{decode_posting_list, intersect, union};
use crate::types::{DocId, IndexMeta, NgramHash, Query, QueryOp};

pub const MAGIC: &[u8; 8] = b"IGREP\x00\x01\x00";
const LOOKUP_ENTRY_SIZE: usize = 12;

pub struct IndexReader {
    lookup_mmap: Mmap,
    postings_file: File,
    meta: IndexMeta,
    file_list: Vec<PathBuf>,
}

impl IndexReader {
    pub fn open(index_dir: &Path) -> Result<Self> {
        let postings_path = index_dir.join("index.postings");
        let mut postings_file = File::open(&postings_path)
            .with_context(|| format!("open {}", postings_path.display()))?;

        let mut magic = [0_u8; MAGIC.len()];
        postings_file
            .read_exact(&mut magic)
            .with_context(|| format!("read magic from {}", postings_path.display()))?;
        if &magic != MAGIC {
            bail!(
                "invalid postings magic in {}: expected {:?}, got {:?}",
                postings_path.display(),
                MAGIC,
                magic
            );
        }

        let lookup_path = index_dir.join("index.lookup");
        let lookup_file =
            File::open(&lookup_path).with_context(|| format!("open {}", lookup_path.display()))?;
        let lookup_mmap = unsafe {
            Mmap::map(&lookup_file)
                .with_context(|| format!("mmap lookup table {}", lookup_path.display()))?
        };
        if lookup_mmap.len() % LOOKUP_ENTRY_SIZE != 0 {
            bail!(
                "invalid lookup table size {} in {}",
                lookup_mmap.len(),
                lookup_path.display()
            );
        }

        let files_path = index_dir.join("index.files");
        let files_content = std::fs::read_to_string(&files_path)
            .with_context(|| format!("read {}", files_path.display()))?;
        let mut file_list = Vec::new();
        for (line_no, line) in files_content.lines().enumerate() {
            let (doc_id_raw, path_raw) = line
                .split_once('\t')
                .with_context(|| format!("malformed index.files line {}", line_no + 1))?;
            let doc_id: DocId = doc_id_raw
                .parse()
                .with_context(|| format!("invalid doc id at line {}", line_no + 1))?;
            if doc_id as usize != file_list.len() {
                bail!(
                    "non-contiguous doc id {} at line {} (expected {})",
                    doc_id,
                    line_no + 1,
                    file_list.len()
                );
            }
            file_list.push(PathBuf::from(path_raw));
        }

        let meta = IndexMeta {
            version: 1,
            commit_sha: None,
            file_count: file_list.len() as u32,
            ngram_count: (lookup_mmap.len() / LOOKUP_ENTRY_SIZE) as u32,
        };

        Ok(Self {
            lookup_mmap,
            postings_file,
            meta,
            file_list,
        })
    }

    pub fn lookup(&self, ngram_hash: NgramHash) -> Option<Vec<DocId>> {
        let posting_offset = self.binary_search_lookup(ngram_hash)?;
        let mut postings = self.postings_file.try_clone().ok()?;

        postings
            .seek(SeekFrom::Start(MAGIC.len() as u64 + posting_offset))
            .ok()?;
        let mut data = Vec::new();
        postings.read_to_end(&mut data).ok()?;

        let mut pos = 0;
        Some(decode_posting_list(&data, &mut pos))
    }

    pub fn evaluate_query(&self, query: &Query) -> Vec<DocId> {
        match query.op {
            QueryOp::All => (0..self.meta.file_count).collect(),
            QueryOp::None => vec![],
            QueryOp::And => {
                let mut result: Option<Vec<DocId>> = None;

                for &hash in &query.trigrams {
                    let list = self.lookup(hash).unwrap_or_default();
                    result = Some(match result {
                        Some(acc) => intersect(&acc, &list),
                        None => list,
                    });
                }

                for child in &query.children {
                    let child_result = self.evaluate_query(child);
                    result = Some(match result {
                        Some(acc) => intersect(&acc, &child_result),
                        None => child_result,
                    });
                }

                result.unwrap_or_else(|| (0..self.meta.file_count).collect())
            }
            QueryOp::Or => {
                let mut result: Option<Vec<DocId>> = None;

                for &hash in &query.trigrams {
                    let list = self.lookup(hash).unwrap_or_default();
                    result = Some(match result {
                        Some(acc) => union(&acc, &list),
                        None => list,
                    });
                }

                for child in &query.children {
                    let child_result = self.evaluate_query(child);
                    result = Some(match result {
                        Some(acc) => union(&acc, &child_result),
                        None => child_result,
                    });
                }

                result.unwrap_or_default()
            }
        }
    }

    pub fn doc_id_to_path(&self, doc_id: DocId) -> Option<&Path> {
        self.file_list.get(doc_id as usize).map(PathBuf::as_path)
    }

    pub fn file_count(&self) -> u32 {
        self.meta.file_count
    }

    fn binary_search_lookup(&self, target_hash: NgramHash) -> Option<u64> {
        let entry_count = self.lookup_mmap.len() / LOOKUP_ENTRY_SIZE;
        let data = &self.lookup_mmap[..];

        let mut lo = 0usize;
        let mut hi = entry_count;
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let offset = mid * LOOKUP_ENTRY_SIZE;
            let hash = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
            match hash.cmp(&target_hash) {
                std::cmp::Ordering::Less => lo = mid + 1,
                std::cmp::Ordering::Greater => hi = mid,
                std::cmp::Ordering::Equal => {
                    let posting_offset =
                        u64::from_le_bytes(data[offset + 4..offset + 12].try_into().unwrap());
                    return Some(posting_offset);
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    use crate::index::writer::IndexWriter;
    use crate::ngram::hash_ngram;
    use crate::types::{IndexConfig, Query, QueryOp};

    fn build_test_index() -> (tempfile::TempDir, IndexReader, NgramHash, NgramHash) {
        let dir = tempdir().unwrap();
        let mut writer = IndexWriter::new(IndexConfig::default());

        writer.add_file(0, "a.txt", b"ab");
        writer.add_file(1, "b.txt", b"ab");
        writer.add_file(2, "c.txt", b"cd");
        writer.finish(dir.path()).unwrap();

        let reader = IndexReader::open(dir.path()).unwrap();
        (dir, reader, hash_ngram(b"ab"), hash_ngram(b"cd"))
    }

    #[test]
    fn open_and_lookup_roundtrip() {
        let (_dir, reader, hash_ab, hash_cd) = build_test_index();

        assert_eq!(reader.file_count(), 3);
        assert_eq!(reader.lookup(hash_ab), Some(vec![0, 1]));
        assert_eq!(reader.lookup(hash_cd), Some(vec![2]));
    }

    #[test]
    fn lookup_missing_hash_returns_none() {
        let (_dir, reader, hash_ab, hash_cd) = build_test_index();
        let missing = hash_ab ^ hash_cd ^ 0xA5A5_5A5A;
        assert_eq!(reader.lookup(missing), None);
    }

    #[test]
    fn evaluate_query_and_intersects_constraints() {
        let (_dir, reader, hash_ab, _) = build_test_index();

        let query = Query {
            op: QueryOp::And,
            trigrams: vec![hash_ab],
            children: vec![Query {
                op: QueryOp::And,
                trigrams: vec![hash_ab],
                children: vec![],
            }],
        };

        assert_eq!(reader.evaluate_query(&query), vec![0, 1]);
    }

    #[test]
    fn evaluate_query_or_unions_constraints() {
        let (_dir, reader, hash_ab, hash_cd) = build_test_index();

        let query = Query {
            op: QueryOp::Or,
            trigrams: vec![hash_cd],
            children: vec![Query {
                op: QueryOp::And,
                trigrams: vec![hash_ab],
                children: vec![],
            }],
        };

        assert_eq!(reader.evaluate_query(&query), vec![0, 1, 2]);
    }

    #[test]
    fn evaluate_query_all_returns_all_docs() {
        let (_dir, reader, _, _) = build_test_index();
        let query = Query {
            op: QueryOp::All,
            trigrams: vec![],
            children: vec![],
        };
        assert_eq!(reader.evaluate_query(&query), vec![0, 1, 2]);
    }

    #[test]
    fn evaluate_query_none_returns_empty() {
        let (_dir, reader, _, _) = build_test_index();
        let query = Query {
            op: QueryOp::None,
            trigrams: vec![],
            children: vec![],
        };
        assert!(reader.evaluate_query(&query).is_empty());
    }

    #[test]
    fn doc_id_to_path_maps_ids_to_paths() {
        let (_dir, reader, _, _) = build_test_index();

        assert_eq!(reader.doc_id_to_path(0), Some(Path::new("a.txt")));
        assert_eq!(reader.doc_id_to_path(1), Some(Path::new("b.txt")));
        assert_eq!(reader.doc_id_to_path(2), Some(Path::new("c.txt")));
        assert_eq!(reader.doc_id_to_path(3), None);
    }
}
