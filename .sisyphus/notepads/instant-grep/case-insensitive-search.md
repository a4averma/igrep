# Case-Insensitive Search Implementation

## Changes Made

### 1. `crates/igrep-core/src/index/writer.rs`
- `add_file()` now lowercases content via `to_ascii_lowercase()` before extracting n-grams
- The index stores only lowercase n-grams, shared by both case-sensitive and case-insensitive queries
- Case-sensitive queries may see more false positive candidates, but regex verification eliminates them

### 2. `crates/igrep-core/src/query.rs`
- `literal_query()` now lowercases literal bytes before hashing to match the lowercase index
- Test helper `literal_hashes()` updated to match

### 3. `crates/igrep-core/src/search.rs`
- Added `search_with_options(reader, pattern, root, case_insensitive)` → `Vec<SearchResult>`
- Added `search_files_with_options(reader, pattern, root, case_insensitive)` → `Vec<PathBuf>`
- Both use `RegexBuilder::new(pattern).case_insensitive(flag).build()` for verification
- Existing `search()` and `search_files_only()` unchanged (case-sensitive by default)

### 4. Tests Added (5 new, all passing)
- `case_insensitive_search_finds_all_casings` — "foobar" matches FooBar, foobar, FOOBAR
- `case_insensitive_uppercase_pattern_finds_all_casings` — "FOOBAR" matches all casings
- `case_sensitive_search_excludes_wrong_casing` — "foobar" doesn't match "FooBar"
- `case_sensitive_search_finds_exact_match` — "FooBar" finds exact match
- `case_insensitive_files_with_options` — file-only search finds case_test.rs

## Design Decision
ASCII-only case folding. Single shared index (lowercase n-grams). No Unicode case folding.
Total: 152 tests pass.
