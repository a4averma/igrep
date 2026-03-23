## walker.rs — Completed

**Functions implemented:**
- `pub fn is_binary(path: &Path) -> bool` — reads first 16KB, returns true if null byte found
- `pub fn walk(root: &Path, config: &IndexConfig) -> Result<Vec<(DocId, PathBuf)>>` — gitignore-aware file walker

**Key details:**
- Uses `ignore::WalkBuilder` with `.git_ignore(true)`, `.hidden(true)`, `.follow_links(false)`, `.sort_by_file_path()`
- Skips: directories, files > max_file_size, binary files (when skip_binary=true)
- Returns sorted paths with sequential DocIds (0, 1, 2, ...)
- test-fixtures: 29 files total, 27 returned (excludes binary.bin + .hidden_file.txt)

**Tests: 13 passing**
- walk count, binary exclusion, hidden exclusion, empty file inclusion, nested file inclusion
- sort order, sequential DocIds, empty directory, is_binary on binary/text/empty, skip_binary=false
