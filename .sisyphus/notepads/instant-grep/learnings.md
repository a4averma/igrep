# Learnings — instant-grep

## Session 1: ses_2e418c70cffeZCV6yDHpOmXaXi
Starting execution of instant-grep plan.

## Task 4: frequency_table.rs — Bigram Frequency Table
- **Approach**: Static `BYTE_RARITY` table (256 entries, `const`-initialized) + `bigram_weight(b1, b2) = rarity[b1] * 256 + rarity[b2]`
- **Range**: Naturally spans 0..=65535 (full u16 range in u32) without clamping or modular arithmetic
- **Key design**: Tiered rarity scores — space/common letters (0–15), common punctuation (25–39), digits (36–49), uppercase (63–88), control/high-ASCII (200 default)
- **Tests**: 7 tests covering common pairs, rare pairs, ordering invariant, space+letter, all 65536 entries in range, deterministic, and granular ordering
- **Note**: rust-analyzer not available in this toolchain — rely on `cargo test`/`cargo build` for verification
- **Convention**: `static` array with `const`-evaluable initializer (mutable `let mut` inside const block) works in Rust 2021 edition

## Task 6: test-fixtures/ — Deterministic Test Corpus
- **29 files** created across Rust (.rs x9), Go (.go x4), Python (.py x4), JS (.js x3), TSX (.tsx x1), TS (.ts x1), TXT (.txt x1), BIN (.bin x1), JSON (.json x1), MD (.md x1), hidden (.hidden_file.txt), no-ext (no_extension)
- **Total corpus size**: ~26KB (well under 100KB limit)
- **Key edge cases**: empty file (0 bytes), binary with null bytes, minified JS (2510-char line), Unicode (Japanese/German/Chinese/Russian/emoji), nested deep path, hidden file, extensionless file
- **Planted patterns**: TODO (13 files), FIXME (11 files), pub struct (5 files), pub fn (8 files), pub enum (3 files), FooBar/foobar/FOOBAR case variants
- **EXPECTED.md**: Comprehensive map of pattern→files for integration test assertions
- **Gotcha**: Write tool adds a trailing newline even when writing empty content. Use `printf '' > file` for truly 0-byte files

## Task: trigram.rs — Classic Trigram Extraction & Probabilistic Masks
- **Functions**: `extract_trigrams`, `trigram_to_hash`, `extract_trigrams_with_masks`, `check_adjacency`, `check_next_char`
- **Design**: `extract_trigrams` returns all trigrams (with duplicates) via 3-byte sliding window; `extract_trigrams_with_masks` groups by trigram value and merges masks with OR
- **Hash**: `trigram_to_hash` uses `crc32fast::hash` for NgramHash (u32); `hash_byte` uses low byte of crc32 for single-byte hashing in mask operations
- **Masks**: `loc_mask` encodes position mod 8; `next_mask` encodes hash of following byte mod 8. Both are 8-bit probabilistic bloom-like filters
- **Adjacency**: `check_adjacency(a, b)` = `((a << 1) | (a >> 7)) & b != 0` — handles wrap-around from bit 7→0
- **Tests**: 24 tests covering extraction (basic, duplicates, empty, short, exact-3), hashing (deterministic, different), masks (grouping, loc, next, empty), adjacency (adjacent, non-adjacent, wrap, all positions, real trigrams), next_char (basic, no-next, zero)
- **Note**: rust-analyzer not available — cargo test is the verification method

## Task 8: posting.rs — Varint Encoding & Delta-Encoded Posting Lists
- **Varint**: LEB128-style, 7 bits per byte, MSB=continuation bit. u32::MAX encodes to 5 bytes.
- **Delta encoding**: Sort + dedup input, prepend varint count, then varint-encode deltas between consecutive IDs
- **Decode**: Read count, then accumulate deltas to reconstruct absolute DocIds
- **Edge cases handled**: empty list, single element, unsorted input (sorts internally), duplicates (deduped), large IDs, multiple lists in same buffer, truncated data returns None/empty
- **Tests**: 20 unit tests + 3 proptest properties (varint roundtrip, posting list roundtrip, decoded-always-sorted)
- **All 23 tests pass** including proptest with random inputs up to 200 elements in 0..1_000_000
- **Convention**: `encode_posting_list` does sort+dedup internally, so callers don't need to pre-sort

## Task: ngram.rs — Sparse n-gram BuildAll monotone stack
- Ported the core loop from `danlark1/sparse_ngrams` (`BuildAllNgrams`) with stack pop-on-strictly-greater behavior and equal-weight left-glue collapse.
- Added `hash_ngram` using `crc32fast::hash` and `build_all_ngrams` wrapper that filters by `IndexConfig.max_ngram_length`.
- Added 8 tests total (6 unit + 2 proptest) covering empty/short inputs, two-byte edge case, substring invariant, count upper bound, and deterministic hash behavior.
- Verified with `cargo test -p igrep-core -- ngram` (all passing).


## Task: posting.rs — Set Operations (intersect, union, subtract)
- **Functions**: `intersect`, `union`, `intersect_many`, `union_many`, `subtract` — all operate on sorted `&[DocId]` slices
- **Algorithm**: Simple linear two-pointer merge for all operations. O(n+m) time.
- **intersect_many**: Sorts by list length (shortest first) for early termination when accumulator empties
- **union_many**: Sequential fold — no length-based optimization needed
- **Tests**: 22 new unit tests + 6 property tests (intersect subset-of-both, union contains-both, sorted output for all 3 ops, subtract excludes-b)
- **Total**: 51 posting tests pass (23 existing + 22 unit + 6 property)

## Task: query.rs — Regex HIR to Query decomposition
- Added `regex_to_query(pattern: &str) -> Result<Query, anyhow::Error>` using `regex_syntax::parse` and recursive HIR analysis.
- Implemented safe decomposition strategy: literals -> AND of covering n-gram hashes; concat -> AND; alternation -> OR; repetition with `min=0` -> `QueryOp::All`; `Capture` unwrap; `Look`/`Empty` -> `All`.
- Character classes: exact singleton class uses literal path; small classes (`<=10` candidates) enumerate into OR-of-literals; large classes fall back to `All`; empty class maps to `None`.
- Added query simplification helpers (`and_query`, `or_query`, `simplify_query`) to flatten and remove neutral terms while preserving zero-false-negative filtering.
- Added tests for: literal, alternation, concat with wildcard, full wildcard, empty pattern, long literal (`MAX_FILE_SIZE`), small class+literal, large class fallback.
- Verified with `cargo test -p igrep-core -- query` (passing) and clean LSP diagnostics for `query.rs`.

## Task: index/writer.rs — 2-pass index writer with atomic outputs
- Implemented `IndexWriter` with in-memory `(NgramHash, DocId)` collection and per-doc `(DocId, path)` tracking.
- Added `add_file` integration with `build_all_ngrams` + `hash_ngram`, and `finish` pipeline: sort/group hashes, delta-encode postings via `encode_posting_list`, emit lookup table (`u32 hash + u64 offset`, LE), write `index.postings`, `index.lookup`, and `index.files` using atomic temp-file rename.
- `index.postings` now starts with magic header `IGREP\0\x01\0`; lookup offsets are relative to payload start immediately after that header.
- Added tests for populated and empty index outputs, including magic verification, lookup byte-size (`unique_hashes * 12`), file-list line count, and `IndexMeta` fields.
- Verified with clean diagnostics on `index/mod.rs` + `index/writer.rs` and `cargo test -p igrep-core -- index` (3 passing tests).

## Task: index/reader.rs — mmap lookup reader + query evaluation
- Added `IndexReader::open` to validate postings magic (`IGREP\0\x01\0`), `mmap` `index.lookup`, parse `index.files` into doc-id-indexed `Vec<PathBuf>`, and construct `IndexMeta`.
- Implemented binary-search lookup over 12-byte lookup entries (`u32 hash`, `u64 posting offset`) and posting decode flow by seeking into `index.postings` payload and calling `decode_posting_list`.
- Implemented recursive `evaluate_query` across `QueryOp::{All,None,And,Or}` using posting-list `intersect` / `union`.
- Added `doc_id_to_path` and `file_count` accessors.
- Added roundtrip tests with `IndexWriter` for: lookup hit/miss, AND/OR/ALL/NONE evaluation, and doc-id path mapping.
- Verified with clean diagnostics (`index/mod.rs`, `index/reader.rs`) and `cargo test -p igrep-core -- index` (10 passing index-filtered tests).

## Task: search.rs — end-to-end search pipeline
- Implemented `search(reader, pattern, root)` pipeline: regex decomposition (`regex_to_query`) → candidate doc IDs (`evaluate_query`) → regex verification against file lines.
- Added stale-index tolerance (`!exists()` skip) and resilient content loading (`read_to_string` with lossy byte fallback for `search`).
- Implemented `search_files_only(reader, pattern, root)` with per-file short-circuit and deterministic sorted `Vec<PathBuf>` output.
- Added 5 fixture-backed tests that build an index first via `walk` + `IndexWriter`, then validate patterns: `fn main`, `TODO`, `nonexistent_string_xyz_123`, `struct\\s+\\w+`, `pub fn`.
- Verified with clean diagnostics for `crates/igrep-core/src/search.rs` and `cargo test -p igrep-core -- search` (5/5 passing).
