use std::fs;
use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};
use tempfile::NamedTempFile;

use crate::ngram::{build_all_ngrams, hash_ngram};
use crate::posting::encode_posting_list;
use crate::types::{DocId, IndexConfig, IndexMeta, NgramHash};

const POSTINGS_MAGIC: &[u8; 8] = b"IGREP\x00\x01\x00";

pub struct IndexWriter {
    config: IndexConfig,
    entries: Vec<(NgramHash, DocId)>,
    file_paths: Vec<(DocId, String)>,
}

impl IndexWriter {
    pub fn new(config: IndexConfig) -> Self {
        Self {
            config,
            entries: Vec::new(),
            file_paths: Vec::new(),
        }
    }

    pub fn add_file(&mut self, doc_id: DocId, path: &str, content: &[u8]) {
        // Store n-grams from lowercased content so both case-sensitive and
        // case-insensitive queries can share the same index.  Case-sensitive
        // queries may see slightly more candidates (false positives) but the
        // regex verification pass eliminates them.
        let lowered: Vec<u8> = content.iter().map(|b| b.to_ascii_lowercase()).collect();
        let ngrams = build_all_ngrams(&lowered, &self.config);
        for ngram in &ngrams {
            self.entries.push((hash_ngram(ngram), doc_id));
        }
        self.file_paths.push((doc_id, path.to_string()));
    }

    /// Returns the number of (ngram_hash, doc_id) pairs collected so far.
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    pub fn finish(mut self, output_dir: &Path) -> Result<IndexMeta> {
        fs::create_dir_all(output_dir).with_context(|| {
            format!(
                "failed to create index output directory: {}",
                output_dir.display()
            )
        })?;

        self.entries.sort_unstable();

        let mut postings_data = Vec::new();
        let mut lookup_entries: Vec<(NgramHash, u64)> = Vec::new();

        let mut i = 0;
        while i < self.entries.len() {
            let hash = self.entries[i].0;
            let mut docs = Vec::new();
            while i < self.entries.len() && self.entries[i].0 == hash {
                docs.push(self.entries[i].1);
                i += 1;
            }

            let offset = postings_data.len() as u64;
            lookup_entries.push((hash, offset));
            encode_posting_list(&docs, &mut postings_data);
        }

        let mut postings_bytes = Vec::with_capacity(POSTINGS_MAGIC.len() + postings_data.len());
        postings_bytes.extend_from_slice(POSTINGS_MAGIC);
        postings_bytes.extend_from_slice(&postings_data);
        write_atomic(&output_dir.join("index.postings"), &postings_bytes)?;

        let mut lookup_bytes = Vec::with_capacity(lookup_entries.len() * 12);
        for (hash, offset) in &lookup_entries {
            lookup_bytes.extend_from_slice(&hash.to_le_bytes());
            lookup_bytes.extend_from_slice(&offset.to_le_bytes());
        }
        write_atomic(&output_dir.join("index.lookup"), &lookup_bytes)?;

        self.file_paths.sort_unstable_by_key(|(doc_id, _)| *doc_id);
        let mut files_bytes = Vec::new();
        for (doc_id, path) in &self.file_paths {
            files_bytes.extend_from_slice(doc_id.to_string().as_bytes());
            files_bytes.push(b'\t');
            files_bytes.extend_from_slice(path.as_bytes());
            files_bytes.push(b'\n');
        }
        write_atomic(&output_dir.join("index.files"), &files_bytes)?;

        Ok(IndexMeta {
            version: 1,
            commit_sha: None,
            file_count: self.file_paths.len() as u32,
            ngram_count: lookup_entries.len() as u32,
        })
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .context("cannot atomically write to path without parent directory")?;

    let mut tmp = NamedTempFile::new_in(parent).with_context(|| {
        format!(
            "failed to create temporary file for atomic write: {}",
            path.display()
        )
    })?;
    tmp.write_all(bytes)
        .with_context(|| format!("failed writing bytes to temp file for {}", path.display()))?;
    tmp.flush()
        .with_context(|| format!("failed flushing temp file for {}", path.display()))?;
    tmp.persist(path)
        .map_err(|e| e.error)
        .with_context(|| format!("failed renaming temp file into {}", path.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn finish_writes_index_files_and_expected_formats() {
        let config = IndexConfig::default();
        let mut writer = IndexWriter::new(config.clone());

        let files = vec![
            (0u32, "a.txt", b"abcdefg".as_slice()),
            (1u32, "b.txt", b"abcxyz".as_slice()),
            (2u32, "c.txt", b"xyzabc".as_slice()),
        ];

        for (doc_id, path, content) in &files {
            writer.add_file(*doc_id, path, content);
        }

        let out_dir = tempfile::tempdir().unwrap();
        let meta = writer.finish(out_dir.path()).unwrap();

        let postings_path = out_dir.path().join("index.postings");
        let lookup_path = out_dir.path().join("index.lookup");
        let files_path = out_dir.path().join("index.files");

        assert!(postings_path.exists());
        assert!(lookup_path.exists());
        assert!(files_path.exists());

        let postings = fs::read(&postings_path).unwrap();
        assert!(postings.starts_with(POSTINGS_MAGIC));

        let mut unique_hashes: HashSet<NgramHash> = HashSet::new();
        for (_, _, content) in &files {
            let lowered: Vec<u8> = content.iter().map(|b| b.to_ascii_lowercase()).collect();
            for ngram in build_all_ngrams(&lowered, &config) {
                unique_hashes.insert(hash_ngram(&ngram));
            }
        }

        let lookup = fs::read(&lookup_path).unwrap();
        assert_eq!(lookup.len(), unique_hashes.len() * 12);

        let files_txt = String::from_utf8(fs::read(&files_path).unwrap()).unwrap();
        assert_eq!(files_txt.lines().count(), 3);

        assert_eq!(meta.version, 1);
        assert_eq!(meta.commit_sha, None);
        assert_eq!(meta.file_count, 3);
        assert_eq!(meta.ngram_count as usize, unique_hashes.len());
    }

    #[test]
    fn finish_with_no_files_writes_valid_empty_index() {
        let writer = IndexWriter::new(IndexConfig::default());
        let out_dir = tempfile::tempdir().unwrap();

        let meta = writer.finish(out_dir.path()).unwrap();

        let postings = fs::read(out_dir.path().join("index.postings")).unwrap();
        assert_eq!(postings, POSTINGS_MAGIC.to_vec());

        let lookup = fs::read(out_dir.path().join("index.lookup")).unwrap();
        assert!(lookup.is_empty());

        let files = fs::read(out_dir.path().join("index.files")).unwrap();
        assert!(files.is_empty());

        assert_eq!(meta.version, 1);
        assert_eq!(meta.commit_sha, None);
        assert_eq!(meta.file_count, 0);
        assert_eq!(meta.ngram_count, 0);
    }
}
