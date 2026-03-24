use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};

use crate::types::DocId;

/// Represents the set of file-level changes between the indexed commit and the
/// current working tree.  Used to decide which base-index results are still
/// valid and which files need a fresh index pass.
pub struct GitOverlay {
    /// The commit SHA the base index was built from, if any.
    pub base_commit: String,
    /// Files that exist in the base index but have been modified since.
    pub modified_files: HashSet<PathBuf>,
    /// Files that existed in the base index but have been deleted.
    pub deleted_files: HashSet<PathBuf>,
    /// Files that are new (untracked or added) since the base commit.
    pub new_files: Vec<PathBuf>,
}

const META_FILENAME: &str = "index.meta";

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

impl GitOverlay {
    /// Detect file-level changes between the stored index commit and the
    /// current working tree.
    ///
    /// If no `index.meta` exists in `index_dir`, the overlay will have an
    /// empty `base_commit` and report *no* changes — the caller should treat
    /// the index as stale and perform a full rebuild.
    pub fn detect_changes(index_dir: &Path, repo_root: &Path) -> Result<GitOverlay> {
        let stored_sha = read_commit_sha(index_dir)?;

        let head_sha = current_head_sha(repo_root)?;

        match stored_sha {
            None => {
                // No stored commit — everything is new, nothing to overlay.
                Ok(GitOverlay {
                    base_commit: String::new(),
                    modified_files: HashSet::new(),
                    deleted_files: HashSet::new(),
                    new_files: Vec::new(),
                })
            }
            Some(base_sha) => {
                // 1. Files changed between stored commit and HEAD
                let committed_changes =
                    git_diff_names(repo_root, &[&base_sha, &head_sha, "--name-only"])?;

                // 2. Unstaged changes in the working tree
                let unstaged = git_diff_names(repo_root, &["--name-only"])?;

                // 3. Staged (cached) changes
                let staged = git_diff_names(repo_root, &["--cached", "--name-only"])?;

                // 4. Untracked files
                let untracked = git_ls_untracked(repo_root)?;

                // Merge all change sets
                let mut modified = HashSet::new();
                let mut deleted = HashSet::new();

                for path in committed_changes
                    .iter()
                    .chain(unstaged.iter())
                    .chain(staged.iter())
                {
                    let full = repo_root.join(path);
                    if full.exists() {
                        modified.insert(path.clone());
                    } else {
                        deleted.insert(path.clone());
                    }
                }

                // Untracked files are always "new"
                let new_files: Vec<PathBuf> = untracked
                    .into_iter()
                    .filter(|p| !modified.contains(p))
                    .collect();

                Ok(GitOverlay {
                    base_commit: base_sha,
                    modified_files: modified,
                    deleted_files: deleted,
                    new_files,
                })
            }
        }
    }
}

/// Filter base-index results to exclude documents whose files have been
/// modified or deleted.
///
/// `file_list` maps `(DocId, PathBuf)` for every document in the base index.
/// Returns only doc IDs whose files are still unchanged.
pub fn merge_results(
    base_results: &[DocId],
    overlay: &GitOverlay,
    file_list: &[(DocId, PathBuf)],
) -> Vec<DocId> {
    // Build reverse lookup: DocId -> PathBuf
    let lookup: std::collections::HashMap<DocId, &PathBuf> =
        file_list.iter().map(|(id, p)| (*id, p)).collect();

    let stale: HashSet<&PathBuf> = overlay
        .modified_files
        .iter()
        .chain(overlay.deleted_files.iter())
        .collect();

    base_results
        .iter()
        .copied()
        .filter(|id| {
            match lookup.get(id) {
                Some(path) => !stale.contains(path),
                // Unknown doc ID — keep it (conservative)
                None => true,
            }
        })
        .collect()
}

/// Persist the given commit SHA to `index_dir/index.meta`.
pub fn write_commit_sha(index_dir: &Path, sha: &str) -> Result<()> {
    fs::create_dir_all(index_dir).context("creating index directory")?;
    let meta_path = index_dir.join(META_FILENAME);
    fs::write(&meta_path, sha).with_context(|| format!("writing {}", meta_path.display()))?;
    Ok(())
}

/// Read the stored commit SHA from `index_dir/index.meta`.
/// Returns `None` if the file does not exist.
pub fn read_commit_sha(index_dir: &Path) -> Result<Option<String>> {
    let meta_path = index_dir.join(META_FILENAME);
    if !meta_path.exists() {
        return Ok(None);
    }
    let contents = fs::read_to_string(&meta_path)
        .with_context(|| format!("reading {}", meta_path.display()))?;
    let sha = contents.trim().to_string();
    if sha.is_empty() {
        Ok(None)
    } else {
        Ok(Some(sha))
    }
}

/// Get the current HEAD commit SHA of the repository at `repo_root`.
pub fn current_head_sha(repo_root: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["-C", &repo_root.to_string_lossy(), "rev-parse", "HEAD"])
        .output()
        .context("running git rev-parse HEAD")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git rev-parse HEAD failed: {}", stderr.trim());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Run `git -C {repo_root} diff {extra_args...}` and return the list of file
/// paths from the output.
fn git_diff_names(repo_root: &Path, extra_args: &[&str]) -> Result<Vec<PathBuf>> {
    let mut cmd = Command::new("git");
    cmd.args(["-C", &repo_root.to_string_lossy(), "diff"]);
    for arg in extra_args {
        cmd.arg(arg);
    }

    let output = cmd.output().context("running git diff")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git diff failed: {}", stderr.trim());
    }

    Ok(parse_path_lines(&output.stdout))
}

/// Run `git -C {repo_root} ls-files --others --exclude-standard` to discover
/// untracked files.
fn git_ls_untracked(repo_root: &Path) -> Result<Vec<PathBuf>> {
    let output = Command::new("git")
        .args([
            "-C",
            &repo_root.to_string_lossy(),
            "ls-files",
            "--others",
            "--exclude-standard",
        ])
        .output()
        .context("running git ls-files")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git ls-files failed: {}", stderr.trim());
    }

    Ok(parse_path_lines(&output.stdout))
}

/// Parse new-line separated file paths from raw git output.
fn parse_path_lines(raw: &[u8]) -> Vec<PathBuf> {
    let text = String::from_utf8_lossy(raw);
    text.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .map(PathBuf::from)
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Helper: create a temporary git repository with an initial commit
    /// containing the given files.  Returns `(temp_dir, repo_root)`.
    fn make_git_repo(files: &[(&str, &str)]) -> (TempDir, PathBuf) {
        let tmp = TempDir::new().expect("create temp dir");
        let root = tmp.path().to_path_buf();

        // git init
        run_git(&root, &["init"]);
        // Configure user for commits
        run_git(&root, &["config", "user.email", "test@test.com"]);
        run_git(&root, &["config", "user.name", "Test"]);

        // Create files and add them
        for (name, content) in files {
            let file_path = root.join(name);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).expect("create parent dirs");
            }
            fs::write(&file_path, content).expect("write file");
            run_git(&root, &["add", name]);
        }

        // Initial commit
        run_git(&root, &["commit", "-m", "initial"]);

        (tmp, root)
    }

    fn run_git(repo_root: &Path, args: &[&str]) {
        let output = Command::new("git")
            .args(["-C", &repo_root.to_string_lossy()])
            .args(args)
            .output()
            .unwrap_or_else(|e| panic!("git {:?} failed to execute: {}", args, e));
        assert!(
            output.status.success(),
            "git {:?} failed:\nstdout: {}\nstderr: {}",
            args,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }

    #[test]
    fn test_read_write_commit_sha_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let index_dir = tmp.path();

        // Initially no SHA stored
        assert_eq!(read_commit_sha(index_dir).unwrap(), None);

        // Write and read back
        write_commit_sha(index_dir, "abc123def456").unwrap();
        assert_eq!(
            read_commit_sha(index_dir).unwrap(),
            Some("abc123def456".to_string())
        );

        // Overwrite
        write_commit_sha(index_dir, "000111222333").unwrap();
        assert_eq!(
            read_commit_sha(index_dir).unwrap(),
            Some("000111222333".to_string())
        );
    }

    #[test]
    fn test_read_commit_sha_empty_file() {
        let tmp = TempDir::new().unwrap();
        let index_dir = tmp.path();
        fs::write(index_dir.join(META_FILENAME), "").unwrap();
        assert_eq!(read_commit_sha(index_dir).unwrap(), None);
    }

    #[test]
    fn test_current_head_sha() {
        let (_tmp, root) = make_git_repo(&[("hello.txt", "hello")]);
        let sha = current_head_sha(&root).unwrap();
        assert!(!sha.is_empty());
        // SHA is 40 hex characters
        assert_eq!(sha.len(), 40);
        assert!(sha.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_detect_changes_no_base_commit() {
        let (_tmp, root) = make_git_repo(&[("a.txt", "a")]);
        let index_dir = TempDir::new().unwrap();

        // No index.meta -> overlay says "no base commit"
        let overlay = GitOverlay::detect_changes(index_dir.path(), &root).unwrap();
        assert!(overlay.base_commit.is_empty());
        assert!(overlay.modified_files.is_empty());
        assert!(overlay.deleted_files.is_empty());
        assert!(overlay.new_files.is_empty());
    }

    #[test]
    fn test_detect_changes_no_changes() {
        let (_tmp, root) = make_git_repo(&[("a.txt", "aaa"), ("b.txt", "bbb")]);
        let index_dir = TempDir::new().unwrap();

        // Store the current HEAD as the base
        let head = current_head_sha(&root).unwrap();
        write_commit_sha(index_dir.path(), &head).unwrap();

        let overlay = GitOverlay::detect_changes(index_dir.path(), &root).unwrap();
        assert_eq!(overlay.base_commit, head);
        assert!(overlay.modified_files.is_empty());
        assert!(overlay.deleted_files.is_empty());
        assert!(overlay.new_files.is_empty());
    }

    #[test]
    fn test_detect_changes_modified_file() {
        let (_tmp, root) = make_git_repo(&[("a.txt", "aaa"), ("b.txt", "bbb")]);
        let index_dir = TempDir::new().unwrap();

        let head = current_head_sha(&root).unwrap();
        write_commit_sha(index_dir.path(), &head).unwrap();

        // Modify a.txt (unstaged)
        fs::write(root.join("a.txt"), "modified").unwrap();

        let overlay = GitOverlay::detect_changes(index_dir.path(), &root).unwrap();
        assert!(overlay.modified_files.contains(&PathBuf::from("a.txt")));
        assert!(!overlay.deleted_files.contains(&PathBuf::from("a.txt")));
        assert!(!overlay.modified_files.contains(&PathBuf::from("b.txt")));
    }

    #[test]
    fn test_detect_changes_new_untracked_file() {
        let (_tmp, root) = make_git_repo(&[("a.txt", "aaa")]);
        let index_dir = TempDir::new().unwrap();

        let head = current_head_sha(&root).unwrap();
        write_commit_sha(index_dir.path(), &head).unwrap();

        // Add a new untracked file
        fs::write(root.join("new_file.txt"), "new content").unwrap();

        let overlay = GitOverlay::detect_changes(index_dir.path(), &root).unwrap();
        assert!(overlay.new_files.contains(&PathBuf::from("new_file.txt")));
        assert!(overlay.modified_files.is_empty());
        assert!(overlay.deleted_files.is_empty());
    }

    #[test]
    fn test_detect_changes_deleted_file() {
        let (_tmp, root) = make_git_repo(&[("a.txt", "aaa"), ("b.txt", "bbb")]);
        let index_dir = TempDir::new().unwrap();

        let head = current_head_sha(&root).unwrap();
        write_commit_sha(index_dir.path(), &head).unwrap();

        // Delete b.txt (unstaged)
        fs::remove_file(root.join("b.txt")).unwrap();

        let overlay = GitOverlay::detect_changes(index_dir.path(), &root).unwrap();
        assert!(overlay.deleted_files.contains(&PathBuf::from("b.txt")));
        assert!(!overlay.modified_files.contains(&PathBuf::from("b.txt")));
    }

    #[test]
    fn test_detect_changes_staged_modification() {
        let (_tmp, root) = make_git_repo(&[("a.txt", "aaa")]);
        let index_dir = TempDir::new().unwrap();

        let head = current_head_sha(&root).unwrap();
        write_commit_sha(index_dir.path(), &head).unwrap();

        // Modify and stage
        fs::write(root.join("a.txt"), "staged changes").unwrap();
        run_git(&root, &["add", "a.txt"]);

        let overlay = GitOverlay::detect_changes(index_dir.path(), &root).unwrap();
        assert!(overlay.modified_files.contains(&PathBuf::from("a.txt")));
    }

    #[test]
    fn test_detect_changes_committed_change() {
        let (_tmp, root) = make_git_repo(&[("a.txt", "aaa")]);
        let index_dir = TempDir::new().unwrap();

        let head = current_head_sha(&root).unwrap();
        write_commit_sha(index_dir.path(), &head).unwrap();

        // Make a new commit that modifies a.txt
        fs::write(root.join("a.txt"), "committed change").unwrap();
        run_git(&root, &["add", "a.txt"]);
        run_git(&root, &["commit", "-m", "modify a"]);

        let overlay = GitOverlay::detect_changes(index_dir.path(), &root).unwrap();
        assert!(overlay.modified_files.contains(&PathBuf::from("a.txt")));
    }

    #[test]
    fn test_merge_results_filters_deleted_and_modified() {
        let overlay = GitOverlay {
            base_commit: "abc123".to_string(),
            modified_files: HashSet::from([PathBuf::from("modified.txt")]),
            deleted_files: HashSet::from([PathBuf::from("deleted.txt")]),
            new_files: vec![PathBuf::from("new.txt")],
        };

        let file_list: Vec<(DocId, PathBuf)> = vec![
            (0, PathBuf::from("clean.txt")),
            (1, PathBuf::from("modified.txt")),
            (2, PathBuf::from("deleted.txt")),
            (3, PathBuf::from("also_clean.txt")),
        ];

        let base_results: Vec<DocId> = vec![0, 1, 2, 3];
        let filtered = merge_results(&base_results, &overlay, &file_list);

        // Only clean files should remain
        assert_eq!(filtered, vec![0, 3]);
    }

    #[test]
    fn test_merge_results_empty_overlay() {
        let overlay = GitOverlay {
            base_commit: "abc123".to_string(),
            modified_files: HashSet::new(),
            deleted_files: HashSet::new(),
            new_files: Vec::new(),
        };

        let file_list: Vec<(DocId, PathBuf)> =
            vec![(0, PathBuf::from("a.txt")), (1, PathBuf::from("b.txt"))];

        let base_results: Vec<DocId> = vec![0, 1];
        let filtered = merge_results(&base_results, &overlay, &file_list);

        // No changes → all results pass through
        assert_eq!(filtered, vec![0, 1]);
    }

    #[test]
    fn test_merge_results_unknown_doc_id_preserved() {
        let overlay = GitOverlay {
            base_commit: "abc123".to_string(),
            modified_files: HashSet::from([PathBuf::from("a.txt")]),
            deleted_files: HashSet::new(),
            new_files: Vec::new(),
        };

        // file_list doesn't contain doc_id 99
        let file_list: Vec<(DocId, PathBuf)> = vec![(0, PathBuf::from("a.txt"))];

        let base_results: Vec<DocId> = vec![0, 99];
        let filtered = merge_results(&base_results, &overlay, &file_list);

        // 0 is filtered (modified), 99 is kept (unknown → conservative)
        assert_eq!(filtered, vec![99]);
    }
}
