use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use igrep_core::index::reader::IndexReader;
use igrep_core::index::writer::IndexWriter;
use igrep_core::search;
use igrep_core::types::IndexConfig;
use igrep_core::walker::walk;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test-fixtures")
}

fn setup_index(root: &Path) -> (tempfile::TempDir, IndexReader) {
    let config = IndexConfig::default();
    let files = walk(root, &config).unwrap();

    let mut writer = IndexWriter::new(config);
    for (doc_id, full_path) in files {
        let rel_path = full_path.strip_prefix(root).unwrap_or(full_path.as_path());
        let content = std::fs::read(&full_path).unwrap_or_default();
        writer.add_file(doc_id, &rel_path.to_string_lossy(), &content);
    }

    let tmp = tempfile::tempdir().unwrap();
    writer.finish(tmp.path()).unwrap();
    let reader = IndexReader::open(tmp.path()).unwrap();
    (tmp, reader)
}

fn should_compare_path(rel_path: &Path) -> bool {
    if rel_path
        .components()
        .any(|c| c.as_os_str().to_string_lossy().starts_with('.'))
    {
        return false;
    }

    rel_path
        .file_name()
        .map_or(true, |name| name != "binary.bin")
}

fn grep_files(root: &Path, pattern: &str) -> HashSet<String> {
    let output = Command::new("grep")
        .args(["-ErlI", pattern])
        .arg(root)
        .output();

    match output {
        Ok(o) if o.status.success() || o.status.code() == Some(1) => {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter_map(|line| {
                    let p = Path::new(line);
                    p.strip_prefix(root).ok().or(Some(p)).and_then(|rel| {
                        if should_compare_path(rel) {
                            Some(rel.to_string_lossy().to_string())
                        } else {
                            None
                        }
                    })
                })
                .collect()
        }
        Ok(o) => panic!(
            "grep failed for pattern '{pattern}' with status {:?}: {}",
            o.status.code(),
            String::from_utf8_lossy(&o.stderr)
        ),
        Err(e) => panic!("failed to run grep for pattern '{pattern}': {e}"),
    }
}

fn igrep_files(reader: &IndexReader, pattern: &str, root: &Path) -> HashSet<String> {
    match search::search_files_only(reader, pattern, root) {
        Ok(files) => files
            .into_iter()
            .filter(|p| should_compare_path(p))
            .map(|p| p.to_string_lossy().to_string())
            .collect(),
        Err(_) => HashSet::new(),
    }
}

#[test]
fn igrep_has_zero_false_negatives_against_grep_file_set() {
    let root = fixtures_dir();
    let (_tmp, reader) = setup_index(&root);

    let patterns = [
        "fn main",
        "use std",
        "TODO",
        "FIXME",
        "struct",
        "pub fn",
        "import",
        "function",
        "class",
        "return",
        "def ",
        "package",
        "export",
        "println",
        "const ",
        "interface",
        "derive",
        "fmt",
        "async",
        "error",
        "impl",
        "match",
    ];

    for pattern in patterns {
        let grep_set = grep_files(&root, pattern);
        let igrep_set = igrep_files(&reader, pattern, &root);

        assert!(
            !grep_set.is_empty(),
            "fixture drift: grep returned no files for pattern '{pattern}'"
        );

        let missing: Vec<&String> = grep_set.difference(&igrep_set).collect();
        assert!(
            missing.is_empty(),
            "false negatives for pattern '{pattern}'. Missing files: {:?}. grep={:?}, igrep={:?}",
            missing,
            grep_set,
            igrep_set
        );
    }
}
