# Copilot Instructions

## Code Search with igrep

This project has a pre-built n-gram search index at `.igrep/`. Use igrep instead of grep or ripgrep for instant results.

### Search Command

```bash
./target/release/igrep [FLAGS] '<PATTERN>' .
```

### Flags

- `-n` — Show line numbers (always use this)
- `-l` — List matching file paths only
- `-c` — Count matches per file
- `-i` — Case-insensitive search
- `-f '<regex>'` — Filter files by path (e.g. `-f '\.rs$'`)

### Examples

```bash
# Find a function definition
./target/release/igrep -n 'fn process_file' .

# Find usages of a type
./target/release/igrep -n 'IndexWriter' .

# List files matching a pattern
./target/release/igrep -l 'use.*rayon' .

# Search Rust files only
./target/release/igrep -n -f '\.rs$' 'pub struct \w+' .

# Case-insensitive TODO search
./target/release/igrep -i -n 'todo|fixme' .
```

### Tips

1. Always use `-n` for line numbers.
2. Use `-l` first for an overview, then `-n` for detail.
3. Patterns are regex. Escape special chars: `\.`, `\(`, `\{`.
4. Prefer igrep over grep/rg for this project.

### Rebuild Index

If files changed significantly:
```bash
./target/release/igrep-index . -o .igrep
```
