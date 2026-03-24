# PROJECT KNOWLEDGE BASE

**Generated:** 2026-03-24
**Commit:** 2ec09b2
**Branch:** main

## OVERVIEW

Blazing-fast regex code search using pre-built sparse n-gram indexes. Builds an index once, then answers queries by reading only candidate files — 10-50x faster than ripgrep on large repos. Rust workspace, two crates.

## STRUCTURE

```
instant-grep/
├── crates/
│   ├── igrep-core/        # Library: all indexing + search logic (10 modules)
│   │   ├── src/            # See crates/igrep-core/src/AGENTS.md
│   │   ├── benches/        # Criterion: ngram_bench, search_bench, index_bench
│   │   └── tests/          # integration.rs (grep oracle), parallel_indexing.rs
│   └── igrep-cli/          # Two thin binaries (no lib.rs)
│       └── src/bin/
│           ├── igrep.rs        # Search CLI (clap derive, 134 lines)
│           └── igrep_index.rs  # Indexing CLI (rayon parallel, 168 lines)
├── test-fixtures/          # 27+ fixture files (multi-language, edge cases)
│   └── EXPECTED.md         # Manifest: pattern -> expected file/line matches
├── .igrep/                 # Pre-built index of this repo (for dogfooding)
└── Cargo.toml              # Workspace root, all dep versions centralized
```

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Core algorithm (n-gram extraction) | `igrep-core/src/ngram.rs` | `build_covering` is the key function |
| Regex-to-query planning | `igrep-core/src/query.rs` | `regex_to_query()` via `regex-syntax` HIR |
| Index binary format | `igrep-core/src/index/` | See `index/AGENTS.md` |
| End-to-end search pipeline | `igrep-core/src/search.rs` | query index -> candidate files -> regex verify |
| File walking (.gitignore) | `igrep-core/src/walker.rs` | Uses `ignore` crate |
| Git delta detection | `igrep-core/src/git_overlay.rs` | Stale file filtering |
| Shared types/config | `igrep-core/src/types.rs` | `DocId`, `Query`, `IndexConfig`, `SearchResult` |
| CLI arguments | `igrep-cli/src/bin/igrep.rs:11-44` | clap derive struct |
| Add test fixture | `test-fixtures/` + `EXPECTED.md` | Update `walker.rs::test_walk_exact_count` (expects 27) |

## CODE MAP

| Symbol | Type | Location | Role |
|--------|------|----------|------|
| `search` | fn | `search.rs:11` | Primary search entry point |
| `search_with_options` | fn | `search.rs:87` | Search with case-insensitive flag |
| `IndexReader` | struct | `index/reader.rs:14` | mmap-based index reader |
| `IndexWriter` | struct | `index/writer.rs:18` | In-memory index builder |
| `regex_to_query` | fn | `query.rs:16` | Regex pattern -> Query tree |
| `build_covering` | fn | `ngram.rs:65` | Sparse covering n-gram selection |
| `bigram_weight` | fn | `frequency_table.rs:156` | Rarity scoring for n-gram boundaries |
| `GitOverlay` | struct | `git_overlay.rs:13` | File-level change detection |

## CONVENTIONS

- **Error handling**: `anyhow::Result` everywhere (including library crate). `.with_context()` on all I/O. `bail!` for validation.
- **No custom lints**: No rustfmt.toml, clippy.toml, or crate-level `#![deny]`. Uses all defaults.
- **Workspace deps**: All versions in root `Cargo.toml`, referenced via `.workspace = true`.
- **Lowercase indexing**: All content lowercased before n-gram extraction (`b.to_ascii_lowercase()`). Case-insensitive by design at index level.
- **Atomic writes**: Index files written via `NamedTempFile` + `persist()` for crash safety.
- **Test fixtures at repo root**: Referenced via `env!("CARGO_MANIFEST_DIR")` parent traversal.

## ANTI-PATTERNS (THIS PROJECT)

- **Posting list inputs MUST be sorted** — `intersect()`, `union()`, `subtract()` silently produce wrong results on unsorted input.
- **Single unsafe block** — `index/reader.rs:43` (`Mmap::map`). Do not add more. File must not be modified while reader is alive.
- **No `unwrap()` in hot paths** — Only in tests, fixed-size byte slices (guaranteed by prior validation), or `deque.back().unwrap()` where loop invariant guarantees non-empty.
- **No feature flags** — Zero conditional compilation. Don't add `#[cfg(feature)]` without discussion.
- **`walker.rs` exact count test** — `test_walk_exact_count` asserts exactly 27 files. Adding/removing fixtures requires updating this.

## COMMANDS

```bash
# Dev
cargo build                              # Debug build
cargo build --release                    # Release build (needed for igrep binary)
cargo install --path crates/igrep-cli    # Install both binaries

# Test
cargo test -p igrep-core                 # Unit + integration tests (~145 tests)
cargo test -p igrep-core -- --ignored    # Include ignored tests

# Bench
cargo bench -p igrep-core               # Criterion benchmarks (HTML reports)

# Search (dogfood)
./target/release/igrep -n 'pattern' .           # Search with line numbers
./target/release/igrep -l 'pattern' .           # File paths only
./target/release/igrep -f '\.rs$' -n 'fn main' .  # Filter by file type

# Rebuild index
./target/release/igrep-index . -o .igrep
```

## NOTES

- **CI is minimal** — Only `cargo build` + `cargo test` on ubuntu-latest. No clippy, fmt, or cross-platform.
- **`anyhow` in library** — Unconventional; libraries typically use `thiserror` for typed errors. Intentional choice here.
- **`tempfile` is a runtime dep** — Used for atomic writes during indexing (not just tests).
- **Frequency table is hand-tuned** — `frequency_table.rs` contains a 256-byte rarity table trained on source code. Common bigrams (` t`, `th`, `he`) get LOW weight; rare bigrams get HIGH weight. The sparse n-gram algorithm depends on this.
- **Index format version 1** — Magic bytes `IGREP\x00\x01\x00`. Three files: `index.postings` (varint-encoded), `index.lookup` (12-byte fixed entries), `index.files` (tab-separated).
