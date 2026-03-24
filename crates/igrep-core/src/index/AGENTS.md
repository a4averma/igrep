# index/ — On-Disk Index Format

## OVERVIEW

Memory-mapped index reader + atomic index writer. Three files on disk, binary search for lookups.

## FILES ON DISK

| File | Format | Purpose |
|------|--------|---------|
| `index.postings` | Magic(8B) + varint-encoded posting lists | N-gram hash -> sorted DocId lists |
| `index.lookup` | Fixed 12-byte entries: `hash(4B LE) + offset(8B LE)` | Sorted by hash, binary-searched |
| `index.files` | `{doc_id}\t{relative_path}\n` per line | DocId -> file path mapping |
| `index.meta` | Plain text commit SHA | Git overlay base commit |

## READER (reader.rs)

- `IndexReader::open()` — Validates magic bytes `IGREP\x00\x01\x00`, mmaps lookup table, loads file list.
- `binary_search_lookup()` — Binary search on 12-byte fixed entries in mmap'd data. `try_into().unwrap()` is safe because slice sizes are guaranteed by `LOOKUP_ENTRY_SIZE` validation on open.
- `evaluate_query()` — Recursive query tree evaluation. `And` intersects, `Or` unions, `All` returns all DocIds, `None` returns empty.
- **Single `unsafe` block (line 43)** — `Mmap::map()`. The index file must not be modified/truncated while the reader is alive.

## WRITER (writer.rs)

- `process_file_to_map()` — Pure function, safe for parallel use via rayon. Lowercases content, extracts n-grams, returns `HashMap<NgramHash, Vec<DocId>>`.
- `merge_maps()` — Merge two hash maps (used in rayon reduce step).
- `IndexWriter::finish()` — Sorts hashes, encodes posting lists, writes all 3 files atomically via `NamedTempFile::persist()`.
- `NgramEntry(NgramHash, DocId)` — Tuple struct for sort-based bulk ingestion.

## INVARIANTS

- Lookup table entries must be sorted by hash (binary search depends on this).
- Posting lists encode sorted DocIds as varint deltas.
- DocIds in `index.files` must be contiguous starting from 0 (validated on read).
- `LOOKUP_ENTRY_SIZE = 12` — Any file whose length % 12 != 0 is rejected.

## PARALLEL INDEXING PIPELINE

```
walk() -> Vec<(DocId, PathBuf)>
  -> rayon par_iter: process_file_to_map() per file
  -> rayon fold+reduce: merge_maps()
  -> IndexWriter::add_map_bulk()
  -> IndexWriter::finish()    # Sort, encode, atomic write
```

Byte-identity between sequential and parallel paths is verified in `tests/parallel_indexing.rs`.
