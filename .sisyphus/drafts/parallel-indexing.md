# Draft: Parallel N-gram Indexing & Sorting

## Requirements (confirmed)
- Speed up n-gram indexing and sorting using multiple CPU cores
- Target: `IndexWriter` pipeline in `crates/igrep-core/src/index/writer.rs`

## Codebase Understanding

### Current Architecture (Single-threaded)
The indexing pipeline in `igrep_index.rs` (CLI) works as follows:

1. **Walk** (`walker.rs`): Collect files from directory, respecting .gitignore + binary/size filters → `Vec<(DocId, PathBuf)>` (already sorted by path)
2. **Read + N-gram** (CLI loop → `IndexWriter::add_file`): For each file sequentially:
   - Read file content from disk
   - Lowercase the content
   - Call `build_all_ngrams()` → generates variable-length n-grams using sparse n-gram algo
   - Hash each n-gram with CRC32 → `(NgramHash, DocId)` pairs
   - Push into `self.entries: Vec<(NgramHash, DocId)>`
3. **Sort** (`IndexWriter::finish`): `self.entries.sort_unstable()` — sort ALL (hash, doc_id) pairs
4. **Group + Encode**: Walk sorted entries, group by hash, delta-encode posting lists
5. **Write**: Atomic writes to `index.postings`, `index.lookup`, `index.files`

### Key Bottleneck Analysis
- **`add_file` loop**: Called sequentially for each file. File I/O + n-gram generation + hashing is per-file CPU work. This is embarrassingly parallel — each file is independent.
- **`sort_unstable()`**: Sorts the full `Vec<(NgramHash, DocId)>` which can be millions of entries. This is a single-threaded sort. Can use parallel sort (rayon).
- **`entries` accumulation**: Currently uses `&mut self` — pushes into a shared Vec. Must change for parallelism.

### Data Structures
- `entries: Vec<(NgramHash, DocId)>` — NgramHash=u32, DocId=u32 → 8 bytes per entry
- `file_paths: Vec<(DocId, String)>` — file path mapping
- No threading/concurrency primitives exist currently

### Dependencies (current)
- Rust 2021 edition
- `crc32fast` for hashing
- `memmap2` for index reading
- `ignore` crate (has built-in parallel walker!)
- `criterion` for benchmarks
- `proptest` for property tests
- No `rayon` or threading crate currently

### Existing Test Infrastructure
- Unit tests in every module
- Property tests with `proptest` in ngram.rs and posting.rs
- Integration test comparing against `grep` results
- Criterion benchmarks for n-gram generation and posting encoding
- Test command: `cargo test`
- Benchmark command: `cargo bench`

## Technical Decisions (confirmed)
- **Concurrency library**: Rayon (data parallelism, par_iter, par_sort)
- **Parallel file walking**: YES — use `ignore::WalkBuilder::build_parallel()`
- **API design**: Modify IndexWriter to accept parallel input (e.g. `from_entries()` or similar)
- **Test strategy**: TDD — write failing tests first, then implement
- **Parallel sort**: `par_sort_unstable()` from rayon (drop-in replacement)

## Scope Boundaries
- INCLUDE: Parallelizing file walking, n-gram processing, sorting
- INCLUDE: Rayon dependency addition
- INCLUDE: TDD tests for parallel correctness
- INCLUDE: Benchmarks for measuring speedup
- EXCLUDE: Parallel search (search.rs) — not requested
- EXCLUDE: Changes to the index format or on-disk representation
