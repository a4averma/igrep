use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use igrep_core::index::writer::{merge_maps, process_file_to_map, IndexWriter};
use igrep_core::types::IndexConfig;
use igrep_core::walker::walk;
use rayon::prelude::*;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test-fixtures")
}

fn sequential_index_bytes(root: &Path, config: &IndexConfig) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let files = walk(root, config).unwrap();
    let mut writer = IndexWriter::new(config.clone());

    for (doc_id, full_path) in files {
        let rel_path = full_path.strip_prefix(root).unwrap_or(full_path.as_path());
        let content = fs::read(&full_path).unwrap_or_default();
        writer.add_file(doc_id, &rel_path.to_string_lossy(), &content);
    }

    let out = tempfile::tempdir().unwrap();
    writer.finish(out.path()).unwrap();

    (
        fs::read(out.path().join("index.postings")).unwrap(),
        fs::read(out.path().join("index.lookup")).unwrap(),
        fs::read(out.path().join("index.files")).unwrap(),
    )
}

fn parallel_index_bytes(root: &Path, config: &IndexConfig) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let files = walk(root, config).unwrap();

    let (merged_map, file_paths) = files
        .par_iter()
        .map(|(doc_id, full_path)| {
            let rel_path = full_path.strip_prefix(root).unwrap_or(full_path.as_path());
            let content = fs::read(full_path).unwrap_or_default();
            let map = process_file_to_map(&content, *doc_id, config);
            let file_paths = vec![(*doc_id, rel_path.to_string_lossy().to_string())];

            (map, file_paths)
        })
        .reduce(
            || (HashMap::new(), Vec::new()),
            |mut left, mut right| {
                merge_maps(&mut left.0, right.0);
                left.1.append(&mut right.1);
                left
            },
        );

    let mut writer = IndexWriter::new(config.clone());
    writer.add_map_bulk(merged_map, file_paths);

    let out = tempfile::tempdir().unwrap();
    writer.finish(out.path()).unwrap();

    (
        fs::read(out.path().join("index.postings")).unwrap(),
        fs::read(out.path().join("index.lookup")).unwrap(),
        fs::read(out.path().join("index.files")).unwrap(),
    )
}

fn assert_parallel_matches_sequential(root: &Path, config: &IndexConfig) {
    let (sequential_postings, sequential_lookup, sequential_files) =
        sequential_index_bytes(root, config);
    let (parallel_postings, parallel_lookup, parallel_files) = parallel_index_bytes(root, config);

    assert_eq!(sequential_postings, parallel_postings);
    assert_eq!(sequential_lookup, parallel_lookup);
    assert_eq!(sequential_files, parallel_files);
}

#[test]
fn parallel_pipeline_matches_sequential() {
    let root = fixtures_dir();
    let config = IndexConfig::default();
    assert_parallel_matches_sequential(&root, &config);
}

#[test]
fn parallel_pipeline_with_thread_counts() {
    let root = fixtures_dir();
    let config = IndexConfig::default();

    for thread_count in [1usize, 2, 4] {
        rayon::ThreadPoolBuilder::new()
            .num_threads(thread_count)
            .build()
            .unwrap()
            .install(|| assert_parallel_matches_sequential(&root, &config));
    }
}
