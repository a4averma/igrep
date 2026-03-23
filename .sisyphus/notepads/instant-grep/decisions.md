# Decisions — instant-grep

## Pre-planned Decisions
- Language: Rust
- Index strategy: Full pipeline (trigrams → masks → sparse n-grams)
- Deliverable: Library + CLI (igrep-core + igrep-cli)
- Weight function: Pre-computed frequency table from OSS corpus
- Index format: Two files (lookup table mmap'd + postings on disk)
- Git versioning: Base index at commit SHA + in-memory dirty overlay
- Case-insensitive: Index lowercased text, verify with case-insensitive regex

## Task: ngram.rs — BuildAll extraction details
- Kept `build_all` API generic over `weight_fn` (`Fn(u8, u8) -> u32`) and wired `build_all_ngrams` to `bigram_weight` for production extraction.
- Added an explicit 2-byte input fast path emitting one n-gram to satisfy required behavior (`"ab" -> ["ab"]`).
- Enforced `max_ngram_length` in wrapper-level collection rather than altering core monotone-stack extraction semantics.
