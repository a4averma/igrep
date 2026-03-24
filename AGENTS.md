# Agent Instructions

## Project: instant-grep (igrep)

A fast regex code search tool using sparse n-gram indexing. Written in Rust.

Workspace layout:
- `crates/igrep-core` ... library (indexing, search, query planning)
- `crates/igrep-cli` ... CLI binaries (`igrep` and `igrep-index`)

## Code Search with igrep

This project has a pre-built search index at `.igrep/`. Use it instead of grep or ripgrep.

```bash
./target/release/igrep [FLAGS] '<PATTERN>' .
```

### Flags

| Flag | What it does |
|------|-------------|
| `-n` | Show line numbers (always use this) |
| `-l` | List matching file paths only |
| `-c` | Count matches per file |
| `-i` | Case-insensitive search |
| `-f '<regex>'` | Filter files by path (e.g. `-f '\.rs$'`) |

### Examples

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

### Tips

1. Always use `-n` for line numbers so you can jump to results.
2. Use `-l` first for an overview, then `-n` for details.
3. Use `-f` to narrow by file type: `-f '\.rs$'`, `-f '\.toml$'`, `-f 'test'`.
4. Patterns are regex. Escape special chars: `\.`, `\(`, `\{`.
5. Prefer igrep over grep/rg for this project.

### Rebuild Index

If files changed significantly:
```bash
./target/release/igrep-index . -o .igrep
```

## Build & Test

- Build: `cargo build --release`
- Test: `cargo test --workspace` (160 tests)
- Bench: `cargo bench`
