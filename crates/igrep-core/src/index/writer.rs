use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};
use tempfile::NamedTempFile;

use crate::ngram::{build_all_ngrams, hash_ngram};
use crate::posting::encode_posting_list;
use crate::types::{DocId, IndexConfig, IndexMeta, NgramHash};

const POSTINGS_MAGIC: &[u8; 8] = b"IGREP\x00\x01\x00";

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct NgramEntry(pub NgramHash, pub DocId);

pub struct IndexWriter {
    config: IndexConfig,
    postings_map: HashMap<NgramHash, Vec<DocId>>,
    entry_count: usize,
    file_paths: Vec<(DocId, String)>,
}

/// Process file content into n-gram hash entries without requiring an `IndexWriter`.
///
/// This is the pure, per-file unit of work used during parallel indexing.
/// It lowercases the content, extracts n-grams according to `config`, and
pub fn process_file_ngrams(content: &[u8], doc_id: DocId, config: &IndexConfig) -> Vec<NgramEntry> {
    let lowered: Vec<u8> = content.iter().map(|b| b.to_ascii_lowercase()).collect();
    let ngrams = build_all_ngrams(&lowered, config);
    ngrams
        .iter()
        .map(|ngram| NgramEntry(hash_ngram(ngram), doc_id))
        .collect()
}

pub fn process_file_to_map(
    content: &[u8],
    doc_id: DocId,
    config: &IndexConfig,
) -> HashMap<NgramHash, Vec<DocId>> {
    let lowered: Vec<u8> = content.iter().map(|b| b.to_ascii_lowercase()).collect();
    let ngrams = build_all_ngrams(&lowered, config);
    let mut map: HashMap<NgramHash, Vec<DocId>> = HashMap::new();
    for ngram in &ngrams {
        map.entry(hash_ngram(ngram)).or_default().push(doc_id);
    }
    map
}

pub fn merge_maps(
    target: &mut HashMap<NgramHash, Vec<DocId>>,
    source: HashMap<NgramHash, Vec<DocId>>,
) {
    for (hash, mut doc_ids) in source {
        target.entry(hash).or_default().append(&mut doc_ids);
    }
}

impl IndexWriter {
    pub fn new(config: IndexConfig) -> Self {
        Self {
            config,
            postings_map: HashMap::new(),
            entry_count: 0,
            file_paths: Vec::new(),
        }
    }

    pub fn add_file(&mut self, doc_id: DocId, path: &str, content: &[u8]) {
        let file_map = process_file_to_map(content, doc_id, &self.config);
        for doc_ids in file_map.values() {
            self.entry_count += doc_ids.len();
        }
        merge_maps(&mut self.postings_map, file_map);
        self.file_paths.push((doc_id, path.to_string()));
    }

    pub fn add_entries_bulk(&mut self, entries: Vec<NgramEntry>, file_paths: Vec<(DocId, String)>) {
        self.entry_count += entries.len();
        for entry in entries {
            self.postings_map.entry(entry.0).or_default().push(entry.1);
        }
        self.file_paths.extend(file_paths);
    }

    pub fn add_map_bulk(
        &mut self,
        map: HashMap<NgramHash, Vec<DocId>>,
        file_paths: Vec<(DocId, String)>,
    ) {
        for doc_ids in map.values() {
            self.entry_count += doc_ids.len();
        }
        merge_maps(&mut self.postings_map, map);
        self.file_paths.extend(file_paths);
    }

    /// Returns the number of (ngram_hash, doc_id) pairs collected so far.
    pub fn entry_count(&self) -> usize {
        self.entry_count
    }

    pub fn finish(mut self, output_dir: &Path) -> Result<IndexMeta> {
        fs::create_dir_all(output_dir).with_context(|| {
            format!(
                "failed to create index output directory: {}",
                output_dir.display()
            )
        })?;

        let mut hashes: Vec<NgramHash> = self.postings_map.keys().copied().collect();
        hashes.sort_unstable();

        let mut postings_data = Vec::new();
        let mut lookup_entries: Vec<(NgramHash, u64)> = Vec::with_capacity(hashes.len());

        for hash in &hashes {
            let doc_ids = self.postings_map.remove(hash).unwrap_or_default();
            let offset = postings_data.len() as u64;
            lookup_entries.push((*hash, offset));
            encode_posting_list(&doc_ids, &mut postings_data);
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

    fn collect_bulk_inputs(
        files: &[(DocId, &str, &[u8])],
        config: &IndexConfig,
    ) -> (Vec<NgramEntry>, Vec<(DocId, String)>) {
        let mut entries = Vec::new();
        let mut file_paths = Vec::new();

        for (doc_id, path, content) in files {
            let lowered: Vec<u8> = content.iter().map(|b| b.to_ascii_lowercase()).collect();
            for ngram in build_all_ngrams(&lowered, config) {
                entries.push(NgramEntry(hash_ngram(&ngram), *doc_id));
            }
            file_paths.push((*doc_id, (*path).to_string()));
        }

        (entries, file_paths)
    }

    fn expected_index_bytes_from_sequential_sort(
        mut entries: Vec<NgramEntry>,
    ) -> (Vec<u8>, Vec<u8>) {
        entries.sort_unstable();

        let mut postings_data = Vec::new();
        let mut lookup_entries: Vec<(NgramHash, u64)> = Vec::new();

        let mut i = 0;
        while i < entries.len() {
            let hash = entries[i].0;
            let mut docs = Vec::new();
            while i < entries.len() && entries[i].0 == hash {
                docs.push(entries[i].1);
                i += 1;
            }
            let offset = postings_data.len() as u64;
            lookup_entries.push((hash, offset));
            encode_posting_list(&docs, &mut postings_data);
        }

        let mut postings_bytes = Vec::with_capacity(POSTINGS_MAGIC.len() + postings_data.len());
        postings_bytes.extend_from_slice(POSTINGS_MAGIC);
        postings_bytes.extend_from_slice(&postings_data);

        let mut lookup_bytes = Vec::with_capacity(lookup_entries.len() * 12);
        for (hash, offset) in &lookup_entries {
            lookup_bytes.extend_from_slice(&hash.to_le_bytes());
            lookup_bytes.extend_from_slice(&offset.to_le_bytes());
        }

        (postings_bytes, lookup_bytes)
    }

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

    #[test]
    fn test_add_entries_bulk_equivalent_to_add_file() {
        let config = IndexConfig::default();
        let files = vec![
            (0u32, "a.txt", b"abcdefg".as_slice()),
            (1u32, "b.txt", b"abcxyz".as_slice()),
            (2u32, "c.txt", b"xyzabc".as_slice()),
        ];

        let mut sequential_writer = IndexWriter::new(config.clone());
        for (doc_id, path, content) in &files {
            sequential_writer.add_file(*doc_id, path, content);
        }
        let sequential_out_dir = tempfile::tempdir().unwrap();
        let sequential_meta = sequential_writer.finish(sequential_out_dir.path()).unwrap();

        let (entries, file_paths) = collect_bulk_inputs(&files, &config);
        let mut bulk_writer = IndexWriter::new(config);
        bulk_writer.add_entries_bulk(entries, file_paths);
        let bulk_out_dir = tempfile::tempdir().unwrap();
        let bulk_meta = bulk_writer.finish(bulk_out_dir.path()).unwrap();

        assert_eq!(bulk_meta.version, sequential_meta.version);
        assert_eq!(bulk_meta.commit_sha, sequential_meta.commit_sha);
        assert_eq!(bulk_meta.file_count, sequential_meta.file_count);
        assert_eq!(bulk_meta.ngram_count, sequential_meta.ngram_count);

        let sequential_lookup = fs::read(sequential_out_dir.path().join("index.lookup")).unwrap();
        let bulk_lookup = fs::read(bulk_out_dir.path().join("index.lookup")).unwrap();
        assert_eq!(bulk_lookup, sequential_lookup);
    }

    #[test]
    fn test_finish_par_sort_matches_sequential_sort() {
        let config = IndexConfig::default();
        let entries = vec![
            NgramEntry(20, 4),
            NgramEntry(3, 9),
            NgramEntry(20, 2),
            NgramEntry(1, 7),
            NgramEntry(3, 1),
            NgramEntry(8, 5),
            NgramEntry(1, 2),
            NgramEntry(8, 3),
            NgramEntry(20, 1),
        ];
        let file_paths = vec![(4, "d.txt".to_string()), (9, "i.txt".to_string())];

        let (expected_postings, expected_lookup) =
            expected_index_bytes_from_sequential_sort(entries.clone());

        let mut writer = IndexWriter::new(config);
        writer.add_entries_bulk(entries, file_paths);
        let out_dir = tempfile::tempdir().unwrap();
        writer.finish(out_dir.path()).unwrap();

        let postings = fs::read(out_dir.path().join("index.postings")).unwrap();
        let lookup = fs::read(out_dir.path().join("index.lookup")).unwrap();
        assert_eq!(postings, expected_postings);
        assert_eq!(lookup, expected_lookup);
    }

    #[test]
    fn test_parallel_index_byte_identity() {
        let config = IndexConfig::default();
        let files = vec![
            (
                0u32,
                "src/a.rs",
                b"fn main() { println!(\"A\"); }".as_slice(),
            ),
            (
                1u32,
                "src/b.rs",
                b"pub fn calc(x: i32) -> i32 { x + 1 }".as_slice(),
            ),
            (
                2u32,
                "README.md",
                b"Instant grep with sparse ngrams".as_slice(),
            ),
            (
                3u32,
                "notes.txt",
                b"Case INSENSITIVE test content".as_slice(),
            ),
        ];

        let mut sequential_writer = IndexWriter::new(config.clone());
        for (doc_id, path, content) in &files {
            sequential_writer.add_file(*doc_id, path, content);
        }
        let sequential_out_dir = tempfile::tempdir().unwrap();
        sequential_writer.finish(sequential_out_dir.path()).unwrap();

        let (entries, file_paths) = collect_bulk_inputs(&files, &config);
        let mut bulk_writer = IndexWriter::new(config);
        bulk_writer.add_entries_bulk(entries, file_paths);
        let bulk_out_dir = tempfile::tempdir().unwrap();
        bulk_writer.finish(bulk_out_dir.path()).unwrap();

        let sequential_postings =
            fs::read(sequential_out_dir.path().join("index.postings")).unwrap();
        let sequential_lookup = fs::read(sequential_out_dir.path().join("index.lookup")).unwrap();
        let sequential_files = fs::read(sequential_out_dir.path().join("index.files")).unwrap();

        let bulk_postings = fs::read(bulk_out_dir.path().join("index.postings")).unwrap();
        let bulk_lookup = fs::read(bulk_out_dir.path().join("index.lookup")).unwrap();
        let bulk_files = fs::read(bulk_out_dir.path().join("index.files")).unwrap();

        assert_eq!(bulk_postings, sequential_postings);
        assert_eq!(bulk_lookup, sequential_lookup);
        assert_eq!(bulk_files, sequential_files);
    }

    #[test]
    fn test_add_entries_bulk_empty() {
        let mut writer = IndexWriter::new(IndexConfig::default());
        writer.add_entries_bulk(Vec::new(), Vec::new());

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

    #[test]
    fn test_add_entries_bulk_single_file() {
        let config = IndexConfig::default();
        let files = vec![(42u32, "only.txt", b"OneFileContent".as_slice())];

        let mut sequential_writer = IndexWriter::new(config.clone());
        for (doc_id, path, content) in &files {
            sequential_writer.add_file(*doc_id, path, content);
        }
        let sequential_out_dir = tempfile::tempdir().unwrap();
        let sequential_meta = sequential_writer.finish(sequential_out_dir.path()).unwrap();

        let (entries, file_paths) = collect_bulk_inputs(&files, &config);
        let mut bulk_writer = IndexWriter::new(config);
        bulk_writer.add_entries_bulk(entries, file_paths);
        let bulk_out_dir = tempfile::tempdir().unwrap();
        let bulk_meta = bulk_writer.finish(bulk_out_dir.path()).unwrap();

        assert_eq!(bulk_meta.version, sequential_meta.version);
        assert_eq!(bulk_meta.commit_sha, sequential_meta.commit_sha);
        assert_eq!(bulk_meta.file_count, 1);
        assert_eq!(bulk_meta.ngram_count, sequential_meta.ngram_count);

        let sequential_postings =
            fs::read(sequential_out_dir.path().join("index.postings")).unwrap();
        let sequential_lookup = fs::read(sequential_out_dir.path().join("index.lookup")).unwrap();
        let sequential_files = fs::read(sequential_out_dir.path().join("index.files")).unwrap();

        let bulk_postings = fs::read(bulk_out_dir.path().join("index.postings")).unwrap();
        let bulk_lookup = fs::read(bulk_out_dir.path().join("index.lookup")).unwrap();
        let bulk_files = fs::read(bulk_out_dir.path().join("index.files")).unwrap();

        assert_eq!(bulk_postings, sequential_postings);
        assert_eq!(bulk_lookup, sequential_lookup);
        assert_eq!(bulk_files, sequential_files);
    }
}
