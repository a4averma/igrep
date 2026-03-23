use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::Result;
use ignore::WalkBuilder;

use crate::types::{DocId, IndexConfig};

/// Check if a file is binary by reading the first 16KB and looking for null bytes.
pub fn is_binary(path: &Path) -> bool {
    let mut buf = [0u8; 16384];
    let Ok(mut file) = File::open(path) else {
        return false;
    };
    let Ok(n) = file.read(&mut buf) else {
        return false;
    };
    buf[..n].contains(&0)
}

/// Walk a directory tree, respecting .gitignore rules, skipping hidden files,
/// binary files (if configured), and files exceeding the size limit.
///
/// Returns a sorted list of (DocId, PathBuf) pairs with sequential DocIds.
pub fn walk(root: &Path, config: &IndexConfig) -> Result<Vec<(DocId, PathBuf)>> {
    let walker = WalkBuilder::new(root)
        .git_ignore(true)
        .hidden(true)
        .sort_by_file_path(|a, b| a.cmp(b))
        .follow_links(false)
        .build();

    let mut files: Vec<PathBuf> = Vec::new();

    for entry in walker {
        let entry = entry?;

        // Skip directories — we only want files
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }

        let path = entry.into_path();

        // Skip files exceeding max size
        if let Ok(metadata) = std::fs::metadata(&path) {
            if metadata.len() > config.max_file_size {
                continue;
            }
        }

        // Skip binary files if configured
        if config.skip_binary && is_binary(&path) {
            continue;
        }

        files.push(path);
    }

    // Assign sequential DocIds
    let result: Vec<(DocId, PathBuf)> = files
        .into_iter()
        .enumerate()
        .map(|(i, path)| (i as DocId, path))
        .collect();

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixtures_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("test-fixtures")
    }

    #[test]
    fn test_walk_returns_files_from_fixtures() {
        let config = IndexConfig::default();
        let results = walk(&fixtures_dir(), &config).unwrap();
        // 29 total files minus binary.bin (binary) minus .hidden_file.txt (hidden) = 27
        assert!(
            results.len() >= 20,
            "Expected at least 20 files, got {}",
            results.len()
        );
    }

    #[test]
    fn test_walk_excludes_binary_file() {
        let config = IndexConfig::default();
        let results = walk(&fixtures_dir(), &config).unwrap();
        let paths: Vec<&Path> = results.iter().map(|(_, p)| p.as_path()).collect();
        for path in &paths {
            assert!(
                !path.ends_with("binary.bin"),
                "binary.bin should be excluded"
            );
        }
    }

    #[test]
    fn test_walk_excludes_hidden_files() {
        let config = IndexConfig::default();
        let results = walk(&fixtures_dir(), &config).unwrap();
        let paths: Vec<&Path> = results.iter().map(|(_, p)| p.as_path()).collect();
        for path in &paths {
            assert!(
                !path.ends_with(".hidden_file.txt"),
                ".hidden_file.txt should be excluded"
            );
        }
    }

    #[test]
    fn test_walk_includes_empty_file() {
        let config = IndexConfig::default();
        let results = walk(&fixtures_dir(), &config).unwrap();
        let has_empty = results.iter().any(|(_, p)| p.ends_with("empty.rs"));
        assert!(has_empty, "empty.rs should be included in results");
    }

    #[test]
    fn test_walk_includes_nested_files() {
        let config = IndexConfig::default();
        let results = walk(&fixtures_dir(), &config).unwrap();
        let has_nested = results
            .iter()
            .any(|(_, p)| p.ends_with("nested/deep/module.rs"));
        assert!(
            has_nested,
            "nested/deep/module.rs should be included in results"
        );
    }

    #[test]
    fn test_walk_results_sorted_by_path() {
        let config = IndexConfig::default();
        let results = walk(&fixtures_dir(), &config).unwrap();
        let paths: Vec<&PathBuf> = results.iter().map(|(_, p)| p).collect();
        for window in paths.windows(2) {
            assert!(
                window[0] <= window[1],
                "Results not sorted: {:?} > {:?}",
                window[0],
                window[1]
            );
        }
    }

    #[test]
    fn test_walk_doc_ids_sequential() {
        let config = IndexConfig::default();
        let results = walk(&fixtures_dir(), &config).unwrap();
        for (i, (doc_id, _)) in results.iter().enumerate() {
            assert_eq!(
                *doc_id, i as DocId,
                "DocId should be sequential: expected {}, got {}",
                i, doc_id
            );
        }
    }

    #[test]
    fn test_walk_empty_directory() {
        let dir = tempfile::tempdir().unwrap();
        let config = IndexConfig::default();
        let results = walk(dir.path(), &config).unwrap();
        assert!(
            results.is_empty(),
            "Empty directory should return empty vec"
        );
    }

    #[test]
    fn test_is_binary_detects_binary() {
        let binary_path = fixtures_dir().join("binary.bin");
        assert!(
            is_binary(&binary_path),
            "binary.bin should be detected as binary"
        );
    }

    #[test]
    fn test_is_binary_text_file() {
        let text_path = fixtures_dir().join("simple.rs");
        assert!(!is_binary(&text_path), "simple.rs should not be binary");
    }

    #[test]
    fn test_is_binary_empty_file() {
        let empty_path = fixtures_dir().join("empty.rs");
        assert!(!is_binary(&empty_path), "empty.rs should not be binary");
    }

    #[test]
    fn test_walk_skip_binary_disabled() {
        let config = IndexConfig {
            skip_binary: false,
            ..IndexConfig::default()
        };
        let results = walk(&fixtures_dir(), &config).unwrap();
        let has_binary = results.iter().any(|(_, p)| p.ends_with("binary.bin"));
        assert!(
            has_binary,
            "binary.bin should be included when skip_binary is false"
        );
    }

    #[test]
    fn test_walk_exact_count() {
        let config = IndexConfig::default();
        let results = walk(&fixtures_dir(), &config).unwrap();
        // 29 total - 1 hidden - 1 binary = 27
        assert_eq!(
            results.len(),
            27,
            "Expected exactly 27 files, got {}. Files: {:?}",
            results.len(),
            results.iter().map(|(_, p)| p).collect::<Vec<_>>()
        );
    }
}
