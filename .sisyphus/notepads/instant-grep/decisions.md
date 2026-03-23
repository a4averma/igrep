# Decisions — instant-grep

## Pre-planned Decisions
- Language: Rust
- Index strategy: Full pipeline (trigrams → masks → sparse n-grams)
- Deliverable: Library + CLI (igrep-core + igrep-cli)
- Weight function: Pre-computed frequency table from OSS corpus
- Index format: Two files (lookup table mmap'd + postings on disk)
- Git versioning: Base index at commit SHA + in-memory dirty overlay
- Case-insensitive: Index lowercased text, verify with case-insensitive regex
