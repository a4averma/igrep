
## git_overlay.rs — Completed

**File**: `crates/igrep-core/src/git_overlay.rs`

### What was built:
- `GitOverlay` struct: `base_commit`, `modified_files` (HashSet), `deleted_files` (HashSet), `new_files` (Vec)
- `detect_changes(index_dir, repo_root)`: Reads stored SHA from `index.meta`, runs git diff/ls-files to classify changes into modified/deleted/new
- `merge_results(base_results, overlay, file_list)`: Filters out doc IDs whose files are stale (modified or deleted)
- `write_commit_sha` / `read_commit_sha`: Simple text file persistence for SHA
- `current_head_sha`: Runs `git rev-parse HEAD`

### Tests (13 passing):
- read/write SHA roundtrip, empty file handling
- current_head_sha returns valid 40-char hex
- detect_changes: no base commit, no changes, modified, new untracked, deleted, staged, committed changes
- merge_results: filters deleted+modified, passes through empty overlay, preserves unknown doc IDs

### Design decisions:
- `index.meta` is a plain text file with just the SHA (as specified)
- Unknown doc IDs in merge_results are preserved conservatively
- `detect_changes` with no base commit returns empty overlay (caller should do full rebuild)
- All git commands use `-C {repo_root}` pattern
- `lib.rs` already had `pub mod git_overlay;` declared

## igrep CLI — Completed

**File**: `crates/igrep-cli/src/bin/igrep.rs`

### What was built:
- Full clap-based CLI with `pattern` (positional) and `path` (default `.`) args
- `--index-dir`: override index location (default: `<path>/.igrep`)
- `-l/--files-with-matches`: print only matching file paths
- `-c/--count`: print match count per file (`path:count`)
- `-n/--line-number`: include line numbers in output (`path:line:content`)
- `-i/--ignore-case`: case-insensitive regex search
- `-f/--file-filter`: regex filter on file paths
- Exit codes: 0=matches found, 1=no matches, 2=error

### API usage:
- `search::search_with_options(&reader, pattern, path, ignore_case)` for line results
- `search::search_files_with_options(&reader, pattern, path, ignore_case)` for `-l` mode
- `IndexReader::open(&index_dir)` to open the index
- `regex::Regex::new(file_filter)` for client-side path filtering

### Dependencies added:
- `regex.workspace = true` in `crates/igrep-cli/Cargo.toml`

## igrep-core integration false-negative parity test — Completed

**File**: `crates/igrep-core/tests/integration.rs`

### What was built:
- New integration test that builds a fixture index once, then compares `igrep_core::search::search_files_only` against `grep` file-level matches.
- Added 22 cross-language patterns (`fn main`, `TODO`, `pub fn`, `function`, `class`, `package`, `interface`, `async`, `error`, etc.).
- Assertion is strict no-false-negatives parity: for each pattern, `grep_set ⊆ igrep_set`.

### Key implementation details:
- `setup_index` strips fixture root prefix before calling `IndexWriter::add_file`, so `IndexReader` stores relative paths compatible with search APIs.
- `grep` is invoked with macOS-safe flags: `grep -ErlI` (no `-P`), and results are filtered to match walker behavior (exclude hidden paths and `binary.bin`).
- Test asserts fixture drift safety (`grep` should return non-empty for each chosen pattern).

### Verification:
- `lsp_diagnostics` clean on `crates/igrep-core/tests/integration.rs`.
- `cargo test -p igrep-core --test integration` passes (`1 passed, 0 failed`).

## Criterion Benchmarks — Completed

**Files**: `crates/igrep-core/benches/ngram_bench.rs`, `crates/igrep-core/benches/search_bench.rs`

### What was built:
- `ngram_bench.rs`: 4 benchmark groups
  - `build_all_ngrams` at 100/1000/10000 byte inputs (~3µs/33µs/320µs)
  - `build_covering_ngrams` at same sizes (~800ns/6.8µs/69µs)
  - `posting_encode_decode_1000`: varint roundtrip on 334 doc IDs (~900ns)
  - `hash_ngram`: CRC32 on 16-byte input (~2.7ns)
- `search_bench.rs`: regex_to_query decomposition benchmarks
  - Literal ("fn main" ~750ns, "MAX_FILE_SIZE" ~1µs)
  - Alternation ("TODO|FIXME" ~1.6µs)
  - Regex with classes (struct\s+\w+ ~2.7µs, pub\s+(fn|struct)\s+\w+ ~4.9µs)

### Cargo.toml changes:
- Added `[lib] bench = false` to prevent default harness from breaking `--quick`
- Added `[[bench]]` entries for both bench files with `harness = false`

### Verification:
- `cargo bench -p igrep-core -- --quick` completes without error
- All 152 existing tests + integration test still pass
