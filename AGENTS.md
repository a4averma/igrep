# Agent Instructions

## Project: instant-grep (igrep)

A fast regex code search tool using sparse n-gram indexing. Written in Rust.

## Architecture

Workspace with two crates:
- `crates/igrep-core` -- Library: indexing, search, query planning
- `crates/igrep-cli` -- Binaries: `igrep` (search) and `igrep-index` (indexing)

### Core modules (igrep-core)
- `ngram.rs` -- N-gram extraction and hashing (sparse variable-length n-grams, CRC32)
- `posting.rs` -- Posting list encoding/decoding (varint delta compression), set operations
- `index/writer.rs` -- Index builder: HashMap<NgramHash, Vec<DocId>> aggregation, file writing
- `index/reader.rs` -- Index reader: mmap-based lookup, binary search, query evaluation
- `search.rs` -- Full search pipeline: index lookup -> candidate files -> regex verification
- `query.rs` -- Regex -> n-gram query planning (AND/OR trees)
- `walker.rs` -- File discovery respecting .gitignore
- `types.rs` -- Shared types: DocId, NgramHash, IndexConfig, SearchResult, Query

### Key types (types.rs)
- `DocId = u32`, `NgramHash = u32` -- core identifiers
- `Query { op: QueryOp, trigrams: Vec<NgramHash>, children: Vec<Query> }` -- AND/OR tree
- `PostingList(Vec<DocId>)` -- sorted list of document IDs per n-gram

### Data flow
1. **Indexing**: walk files -> extract n-grams -> HashMap<hash, doc_ids> aggregation (no global sort) -> sort keys -> encode posting lists (varint delta) -> write 3 files: `index.postings`, `index.lookup`, `index.files`
2. **Search**: parse regex -> extract required n-grams -> query index (mmap + binary search) -> intersect posting lists -> verify candidates with actual regex
3. **Parallelism**: CLI uses rayon `par_iter` with fold/reduce for parallel indexing

## Conventions

- No `unsafe` except the single mmap in reader.rs
- Error handling: `anyhow::Result` for public APIs, `.unwrap_or_default()` for file reads during indexing
- Tests: unit tests in each module, property tests with `proptest`, integration test comparing against `grep`
- Keep functions small, avoid deep nesting
- No AI slop: no excessive comments, no over-abstraction

## Build & Test

```bash
cargo build --release
cargo test --workspace    # ~160 tests
cargo bench
cargo clippy --workspace
```

## Code Search with igrep

This project has a pre-built search index at `.igrep/`. Use it instead of grep or ripgrep.

```bash
./target/release/igrep [FLAGS] '<PATTERN>' .
```

| Flag | What it does |
|------|-------------|
| `-n` | Show line numbers (always use this) |
| `-l` | List matching file paths only |
| `-c` | Count matches per file |
| `-i` | Case-insensitive search |
| `-f '<regex>'` | Filter files by path (e.g. `-f '\.rs$'`) |

```bash
# Find a function definition
./target/release/igrep -n 'fn process_file' .

# Find all usages of a type
./target/release/igrep -n 'IndexWriter' .

# List files containing a pattern
./target/release/igrep -l 'use.*rayon' .

# Search only Rust files, case-insensitive
./target/release/igrep -i -n -f '\.rs$' 'todo|fixme' .

# Count occurrences
./target/release/igrep -c 'unwrap()' .
```

**Tips**: Always use `-n`. Use `-l` first for overview. Use `-f` to narrow by file type. Patterns are regex, escape special chars. Prefer igrep over grep/rg for this project.

**Rebuild index** if files changed significantly: `./target/release/igrep-index . -o .igrep`
