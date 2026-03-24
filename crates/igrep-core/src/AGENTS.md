# igrep-core/src — Library Modules

## OVERVIEW

10 public modules implementing n-gram indexing and search. No re-exports — `lib.rs` is just `pub mod` declarations.

## MODULE MAP

| Module | Lines | Purpose | Key Functions |
|--------|-------|---------|---------------|
| `types.rs` | 113 | Shared types: `DocId`(u32), `Query`, `IndexConfig`, `SearchResult` | `IndexConfig::default()` |
| `ngram.rs` | 301 | Sparse n-gram extraction using bigram weight boundaries | `build_all()`, `build_covering()`, `hash_ngram()` |
| `trigram.rs` | ~200 | Trigram extraction with probabilistic position/next-char masks | `extract_trigrams()`, `TrigramWithMasks` |
| `frequency_table.rs` | 285 | Hand-tuned 256-byte rarity table for bigram weight scoring | `bigram_weight(b1, b2) -> u32` |
| `query.rs` | 301 | Regex-to-n-gram query compiler via `regex_syntax` HIR | `regex_to_query(pattern) -> Query` |
| `posting.rs` | ~600 | Varint (LEB128) encode/decode, sorted set operations | `intersect()`, `union()`, `encode_posting_list()` |
| `index/` | — | On-disk index read/write (see `index/AGENTS.md`) | `IndexReader`, `IndexWriter` |
| `search.rs` | 369 | End-to-end search: query plan -> index lookup -> regex verify | `search()`, `search_with_options()` |
| `walker.rs` | 231 | .gitignore-aware file walker, binary detection | `walk()`, `is_binary()` |
| `git_overlay.rs` | 494 | Git delta detection, stale result filtering | `GitOverlay::detect_changes()`, `merge_results()` |

## DATA FLOW

```
regex pattern
  -> query.rs: regex_to_query()        # HIR parse, extract covering n-grams, build Query tree
  -> index/reader.rs: evaluate_query()  # Binary search mmap lookup, decode postings, intersect/union
  -> candidate DocIds                   # Typically <1% of total files
  -> search.rs: search()               # Read candidates, full regex verify, collect matches
  -> Vec<SearchResult>
```

## CONVENTIONS (MODULE-SPECIFIC)

- **All content lowercased** before n-gram extraction — both at index time (`writer.rs`) and query time (`query.rs`).
- **`build_all` vs `build_covering`** — `build_all` emits every n-gram (for indexing). `build_covering` emits minimal subset (for querying). Both use `bigram_weight` for segment boundaries.
- **`SMALL_CLASS_ENUM_LIMIT = 10`** in `query.rs` — Regex character classes with >10 members degrade to `QueryOp::All` (match everything).
- **Property tests** — `ngram.rs` and `posting.rs` use `proptest` for invariant checking. Keep these when modifying algorithms.
- **Test helper pattern** — `fixtures_dir()` navigates from `CARGO_MANIFEST_DIR` to repo-root `test-fixtures/`. Duplicated in walker, search, integration tests.

## ANTI-PATTERNS

- **Do not call `intersect`/`union`/`subtract` on unsorted input** — They assume sorted `&[DocId]` and silently produce wrong results otherwise.
- **`bigram_weight` must be deterministic** — Same inputs, same output. The index depends on this for reproducible n-gram boundaries.
- **Do not add `unwrap()` in non-test code** — Follow the existing `?`/`bail!`/`.with_context()` pattern.
