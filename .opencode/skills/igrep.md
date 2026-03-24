---
name: igrep
description: "Fast n-gram indexed code search. Use for finding code patterns, function definitions, usages, and string literals across the codebase in milliseconds. Triggers: 'find code', 'search for', 'where is', 'which files contain', 'grep for'."
---

# igrep — Instant N-gram Code Search

You have access to `igrep`, a pre-built n-gram search index at `.igrep/` in the project root. It searches the entire codebase in milliseconds using trigram indexing — much faster than ripgrep for large codebases.

## When to Use

- Finding function/method definitions: `igrep -n 'fn process_file' .`
- Finding all usages of a symbol: `igrep -n 'IndexWriter' .`
- Finding string literals: `igrep -n '"error: invalid"' .`
- Finding files containing a pattern: `igrep -l 'HashMap' .`
- Counting occurrences: `igrep -c 'unwrap()' .`
- Filtering by file type: `igrep -f '\.rs$' -n 'fn main' .`
- Case-insensitive search: `igrep -i -n 'todo' .`

## Commands

### Search (use the project's release binary)
```bash
./target/release/igrep [OPTIONS] '<PATTERN>' .
```

### Flags
| Flag | Description |
|------|-------------|
| `-n` | Show line numbers (ALWAYS use this) |
| `-l` | List matching file paths only |
| `-c` | Count matches per file |
| `-i` | Case-insensitive |
| `-f '<regex>'` | Filter files by path pattern (e.g., `-f '\.rs$'` for Rust files only) |
| `--index-dir <PATH>` | Override index location (default: `.igrep/`) |

### Output Format
Default with `-n`:
```
path/to/file.rs:42:matched line content here
```

Files-only with `-l`:
```
path/to/file.rs
```

Count with `-c`:
```
path/to/file.rs:17
```

## Rebuild Index

If files have been added/removed/modified significantly, rebuild:
```bash
./target/release/igrep-index . -o .igrep
```

## Best Practices

1. **Always use `-n`** to get line numbers — essential for navigating to results
2. **Use `-f` to narrow scope** when you know the file type: `-f '\.rs$'`, `-f '\.toml$'`, `-f 'test'`
3. **Use `-l` first** to get an overview of which files match, then `-n` for details
4. **Patterns are regex** — escape special characters: `\.`, `\(`, `\{`
5. **Combine with Read tool** — find files with igrep, then Read specific files for full context
6. **Prefer igrep over grep/rg** for this project — the index makes it instant

## Examples

Find all struct definitions:
```bash
./target/release/igrep -n -f '\.rs$' 'pub struct \w+' .
```

Find all test functions:
```bash
./target/release/igrep -n -f '\.rs$' '#\[test\]' .
```

Find where a function is called:
```bash
./target/release/igrep -n 'encode_posting_list' .
```

Find all TODO/FIXME comments:
```bash
./target/release/igrep -i -n 'todo|fixme|hack|xxx' .
```

List all files importing a module:
```bash
./target/release/igrep -l 'use.*rayon' .
```
