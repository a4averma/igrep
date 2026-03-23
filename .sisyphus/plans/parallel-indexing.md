# Parallel N-gram Indexing & Sorting

## TL;DR

> **Quick Summary**: Add multi-core parallelism to igrep's indexing pipeline using Rayon. Parallelize file reading + n-gram extraction (embarrassingly parallel per-file work) and replace the single-threaded sort with `par_sort_unstable()`. The parallel path must produce byte-identical index output to the current sequential path.
> 
> **Deliverables**:
> - Rayon dependency added to workspace
> - `IndexWriter::add_entries_bulk()` method for accepting externally-collected entries
> - `par_sort_unstable()` in `IndexWriter::finish()`
> - Parallel indexing pipeline in CLI (`igrep_index.rs`)
> - Progress reporting with `AtomicUsize` for parallel file processing
> - TDD tests verifying byte-identity between sequential and parallel paths
> - Criterion benchmark for end-to-end indexing throughput
> 
> **Estimated Effort**: Medium
> **Parallel Execution**: YES — 3 waves
> **Critical Path**: Task 1 → Task 3 → Task 5 → Task 7 → Task 8 → Task 9 → F1–F4

---

## Context

### Original Request
Speed up the process of indexing and sorting n-grams using multiple CPU cores.

### Interview Summary
**Key Discussions**:
- **Concurrency library**: Rayon — industry-standard data parallelism for Rust, provides `par_iter`, `par_sort_unstable`, work stealing
- **Parallel file walking**: User requested it, but Metis analysis revealed `ignore::build_parallel()` is incompatible with sorted output (breaks deterministic DocId assignment). **Deferred** — parallel processing of files after sequential walk provides 95%+ of the speedup
- **API design**: Modify `IndexWriter` to accept parallel input via a new `add_entries_bulk()` method, keeping existing `add_file()` for backward compatibility
- **Test strategy**: TDD — write failing tests first, then implement to pass them

**Research Findings**:
- The indexing pipeline is 100% single-threaded. No concurrency primitives exist.
- `IndexWriter::add_file(&mut self)` is the structural barrier — exclusive mutable reference prevents parallelism
- Entry type is `(u32, u32)` — 8 bytes per entry, implements `Ord`, perfect for `par_sort_unstable()`
- Rayon is already a transitive dependency via criterion — zero additional compile cost
- `IndexReader::open()` validates contiguous DocIds (reader.rs:66-72) — DocId assignment must remain deterministic
- The oracle integration test (comparing igrep against system `grep`) is the strongest correctness guarantee

### Metis Review
**Identified Gaps** (addressed):
- **Parallel walking incompatibility**: `build_parallel()` can't produce sorted output → deferred to future work. Walk stays sequential.
- **DocId ordering risk**: Pre-assign DocIds during sequential walk, pass as inputs to parallel phase
- **Memory amplification**: N threads × up to 50MB file reads. Documented, use Rayon defaults.
- **Progress reporting**: Current sequential `eprint!` would interleave. Use `AtomicUsize` counter.
- **Byte-identity requirement**: Parallel and sequential paths MUST produce identical index files.
- **Missing acceptance criteria**: Added explicit byte-diff tests, thread-count variation tests, oracle test verification
- **Missing benchmark**: No end-to-end indexing benchmark exists. Added as a task.

---

## Work Objectives

### Core Objective
Parallelize the CPU-intensive phases of igrep's indexing pipeline (file reading + n-gram extraction, and entry sorting) using Rayon, achieving significant speedup on multi-core machines while maintaining byte-identical output.

### Concrete Deliverables
- `rayon = "1"` in workspace dependencies
- `IndexWriter::add_entries_bulk()` method in `crates/igrep-core/src/index/writer.rs`
- `par_sort_unstable()` in `IndexWriter::finish()` 
- Parallel pipeline in `crates/igrep-cli/src/bin/igrep_index.rs` using `rayon::par_iter`
- `AtomicUsize`-based progress reporting in the CLI
- TDD test suite: byte-identity tests, thread-count variation tests
- `index_bench.rs` Criterion benchmark for end-to-end indexing throughput

### Definition of Done
- [ ] `cargo test --workspace` passes with 0 failures
- [ ] `cargo test --test integration` passes (oracle test vs grep)
- [ ] New byte-identity test passes: sequential and parallel produce identical `index.postings`, `index.lookup`, `index.files`
- [ ] `cargo bench --bench index_bench -- --test` compiles and runs
- [ ] No `unsafe` code introduced

### Must Have
- Rayon-based parallel file processing (par_iter over files, per-file n-gram extraction)
- Parallel sort via `par_sort_unstable()` in `finish()`
- `add_entries_bulk()` method on IndexWriter
- Byte-identical output between sequential and parallel paths
- TDD tests written before implementation
- Backward compatibility: `add_file()` still works as before

### Must NOT Have (Guardrails)
- NO changes to on-disk index format (magic bytes, lookup entry size, varint encoding, files format)
- NO changes to `add_file(&mut self, ...)` signature
- NO parallelization of `finish()`'s encoding loop (lines 60-72 of writer.rs) — only the sort
- NO modifications to `walker.rs` (walk stays sequential and sorted)
- NO `unsafe` blocks
- NO CLI flags for thread configuration (use Rayon defaults)
- NO changes to error handling semantics (`unwrap_or_default()` stays)
- NO logging framework additions
- NO parallel search (search.rs is out of scope)
- NO AI slop: excessive comments, over-abstraction, unnecessary wrapper types

---

## Verification Strategy

> **ZERO HUMAN INTERVENTION** — ALL verification is agent-executed. No exceptions.

### Test Decision
- **Infrastructure exists**: YES (cargo test, proptest, criterion benchmarks, integration tests)
- **Automated tests**: TDD — write failing tests first, then implement
- **Framework**: cargo test (built-in), criterion (benchmarks)
- **TDD flow**: RED (failing test) → GREEN (minimal impl) → REFACTOR

### QA Policy
Every task MUST include agent-executed QA scenarios.
Evidence saved to `.sisyphus/evidence/task-{N}-{scenario-slug}.{ext}`.

- **Library code**: Use Bash (`cargo test`) — run specific test functions, assert pass/fail
- **CLI binaries**: Use Bash — build and run `igrep-index`, verify output files
- **Benchmarks**: Use Bash (`cargo bench`) — verify compilation and execution

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Start Immediately — foundation):
├── Task 1: Add rayon dependency [quick]
├── Task 2: TDD — Write failing tests for add_entries_bulk + par_sort [deep]

Wave 2 (After Wave 1 — core implementation):
├── Task 3: Implement add_entries_bulk on IndexWriter (depends: 1, 2) [unspecified-high]
├── Task 4: Replace sort_unstable with par_sort_unstable in finish() (depends: 1, 2) [quick]
├── Task 5: TDD — Write failing test for parallel CLI pipeline (depends: 1) [deep]

Wave 3 (After Wave 2 — CLI + benchmarks):
├── Task 6: Extract per-file n-gram processing into a pure function (depends: 3) [quick]
├── Task 7: Implement parallel indexing pipeline in CLI (depends: 3, 4, 5, 6) [deep]
├── Task 8: Update CLI progress reporting with AtomicUsize (depends: 7) [quick]
├── Task 9: Add end-to-end indexing benchmark (depends: 3, 4) [unspecified-high]

Wave FINAL (After ALL tasks — 4 parallel reviews, then user okay):
├── Task F1: Plan compliance audit (oracle)
├── Task F2: Code quality review (unspecified-high)
├── Task F3: Real manual QA (unspecified-high)
└── Task F4: Scope fidelity check (deep)
-> Present results -> Get explicit user okay
```

### Dependency Matrix

| Task | Depends On | Blocks | Wave |
|------|-----------|--------|------|
| 1 | — | 2, 3, 4, 5, 6, 7, 8, 9 | 1 |
| 2 | — | 3, 4 | 1 |
| 3 | 1, 2 | 6, 7, 9 | 2 |
| 4 | 1, 2 | 7 | 2 |
| 5 | 1 | 7 | 2 |
| 6 | 3 | 7 | 3 |
| 7 | 3, 4, 5, 6 | 8 | 3 |
| 8 | 7 | — | 3 |
| 9 | 3, 4 | — | 3 |

### Agent Dispatch Summary

- **Wave 1**: **2 tasks** — T1 → `quick`, T2 → `deep`
- **Wave 2**: **3 tasks** — T3 → `unspecified-high`, T4 → `quick`, T5 → `deep`
- **Wave 3**: **4 tasks** — T6 → `quick`, T7 → `deep`, T8 → `quick`, T9 → `unspecified-high`
- **FINAL**: **4 tasks** — F1 → `oracle`, F2 → `unspecified-high`, F3 → `unspecified-high`, F4 → `deep`

---

## TODOs

- [ ] 1. Add Rayon Dependency to Workspace

  **What to do**:
  - Add `rayon = "1"` to `[workspace.dependencies]` in the root `Cargo.toml`
  - Add `rayon.workspace = true` to `[dependencies]` in `crates/igrep-core/Cargo.toml`
  - Run `cargo check --workspace` to verify compilation

  **Must NOT do**:
  - Do NOT add rayon to igrep-cli's Cargo.toml (it will use it transitively through igrep-core)
  - Do NOT add any rayon feature flags

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []
    - Reason: Simple dependency addition — 2 file edits

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Task 2)
  - **Blocks**: Tasks 2, 3, 4, 5, 6, 7, 8, 9
  - **Blocked By**: None

  **References**:
  - `Cargo.toml:5-16` — Workspace dependencies section where `rayon = "1"` goes
  - `crates/igrep-core/Cargo.toml:9-17` — Core crate dependencies section where `rayon.workspace = true` goes
  - Note: Rayon is already a transitive dependency via `criterion 0.5` in dev-dependencies, so this adds zero compile cost

  **Acceptance Criteria**:
  - [ ] `cargo check --workspace` exits with code 0
  - [ ] `cargo test --workspace` exits with code 0 (no regressions)
  - [ ] `rayon = "1"` appears in root `Cargo.toml` `[workspace.dependencies]`
  - [ ] `rayon.workspace = true` appears in `crates/igrep-core/Cargo.toml` `[dependencies]`

  **QA Scenarios:**

  ```
  Scenario: Rayon dependency compiles successfully
    Tool: Bash
    Preconditions: Clean workspace state
    Steps:
      1. Run `cargo check --workspace`
      2. Assert exit code is 0
      3. Run `cargo test --workspace --no-run` to verify test compilation
      4. Assert exit code is 0
    Expected Result: Both commands succeed with exit code 0
    Failure Indicators: Compilation errors mentioning rayon, unresolved import errors
    Evidence: .sisyphus/evidence/task-1-rayon-dep-compiles.txt

  Scenario: Existing tests still pass after dependency addition
    Tool: Bash
    Preconditions: Rayon added to both Cargo.toml files
    Steps:
      1. Run `cargo test --workspace 2>&1`
      2. Assert output contains "test result: ok"
      3. Assert exit code is 0
    Expected Result: All existing tests pass with no regressions
    Failure Indicators: Any "FAILED" in test output, non-zero exit code
    Evidence: .sisyphus/evidence/task-1-tests-pass.txt
  ```

  **Commit**: YES (group 1)
  - Message: `build: add rayon dependency to workspace`
  - Files: `Cargo.toml`, `crates/igrep-core/Cargo.toml`
  - Pre-commit: `cargo check --workspace`

- [ ] 2. TDD — Write Failing Tests for add_entries_bulk, par_sort, and Byte-Identity

  **What to do**:
  - In `crates/igrep-core/src/index/writer.rs`, add new tests to the existing `#[cfg(test)] mod tests` block:
    1. `test_add_entries_bulk_equivalent_to_add_file` — calls `add_entries_bulk()` with pre-collected entries and verifies the `finish()` output is byte-identical to an index built with sequential `add_file()` calls using the same data
    2. `test_finish_par_sort_matches_sequential_sort` — verifies that the sorted entries from `finish()` produce byte-identical postings/lookup files regardless of sort implementation
    3. `test_parallel_index_byte_identity` — builds two indexes from the same input data (one via `add_file` loop, one via `add_entries_bulk` with collected entries) and byte-compares all 3 output files
    4. `test_add_entries_bulk_empty` — verifies empty entries produce valid empty index
    5. `test_add_entries_bulk_single_file` — verifies single file works
  - These tests should call methods that don't exist yet (`add_entries_bulk`), so they will fail to compile. That's expected for TDD RED phase. Mark them with `#[test]` and add comments noting they are TDD RED tests.
  - Also create `crates/igrep-core/tests/parallel_indexing.rs` with a failing test `parallel_pipeline_matches_sequential` that will be implemented in Task 5.

  **Must NOT do**:
  - Do NOT implement `add_entries_bulk` yet — only write the test code
  - Do NOT modify existing tests

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []
    - Reason: TDD test design requires careful thought about byte-identity verification, understanding the IndexWriter's internal data flow, and setting up proper test fixtures

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Task 1)
  - **Blocks**: Tasks 3, 4
  - **Blocked By**: None (can write tests independently, but tests won't compile until Task 1 is done)

  **References**:
  - `crates/igrep-core/src/index/writer.rs:127-202` — Existing test module with test patterns to follow. Note the `tempfile::tempdir()` pattern and how tests verify file contents.
  - `crates/igrep-core/src/index/writer.rs:14-18` — `IndexWriter` struct definition showing `entries: Vec<(NgramHash, DocId)>` and `file_paths: Vec<(DocId, String)>` — these are the fields `add_entries_bulk` will populate
  - `crates/igrep-core/src/index/writer.rs:29-39` — `add_file` method showing how entries are currently collected (lowercase → build_all_ngrams → hash → push)
  - `crates/igrep-core/src/index/writer.rs:47-102` — `finish` method showing the sort + encode + write pipeline
  - `crates/igrep-core/src/ngram.rs:55-63` — `build_all_ngrams` function and `hash_ngram` — these would be called externally in the parallel pipeline
  - `crates/igrep-core/tests/integration.rs:20-35` — Integration test `setup_index` helper pattern

  **Acceptance Criteria**:
  - [ ] 5 new test functions exist in `writer.rs` test module
  - [ ] 1 new test file exists at `crates/igrep-core/tests/parallel_indexing.rs`
  - [ ] Tests reference `add_entries_bulk` method (which doesn't exist yet)
  - [ ] Tests will fail to compile until Task 3 implements the method — this is correct TDD RED phase

  **QA Scenarios:**

  ```
  Scenario: TDD tests exist and reference the planned API
    Tool: Bash
    Preconditions: Task 1 complete (rayon available)
    Steps:
      1. Run `grep -c "add_entries_bulk" crates/igrep-core/src/index/writer.rs`
      2. Assert count >= 5 (references in test functions)
      3. Run `grep -c "fn test_" crates/igrep-core/src/index/writer.rs`
      4. Assert count >= 7 (2 existing + 5 new)
    Expected Result: New test functions exist referencing the planned API
    Failure Indicators: Missing test functions, wrong method names
    Evidence: .sisyphus/evidence/task-2-tdd-tests-exist.txt

  Scenario: Tests fail to compile (TDD RED phase)
    Tool: Bash
    Preconditions: Tests written, add_entries_bulk not yet implemented
    Steps:
      1. Run `cargo test -p igrep-core --lib -- writer 2>&1`
      2. Assert output contains compilation error referencing "add_entries_bulk"
    Expected Result: Compilation error about missing method — confirms TDD RED phase
    Failure Indicators: Tests compile successfully (would mean API already exists or tests are wrong)
    Evidence: .sisyphus/evidence/task-2-tdd-red-phase.txt
  ```

  **Commit**: YES (group 2)
  - Message: `test: add failing TDD tests for parallel IndexWriter API`
  - Files: `crates/igrep-core/src/index/writer.rs`, `crates/igrep-core/tests/parallel_indexing.rs`
  - Pre-commit: N/A (tests won't compile — this is expected for TDD)

---

- [ ] 3. Implement add_entries_bulk on IndexWriter

  **What to do**:
  - Add `pub fn add_entries_bulk(&mut self, entries: Vec<(NgramHash, DocId)>, file_paths: Vec<(DocId, String)>)` to the `impl IndexWriter` block in `crates/igrep-core/src/index/writer.rs`
  - The method should extend `self.entries` with the provided entries and extend `self.file_paths` with the provided file_paths
  - This is a simple bulk append — no deduplication needed (the `finish()` method handles that)
  - Verify all TDD tests from Task 2 pass

  **Must NOT do**:
  - Do NOT change the `add_file` method
  - Do NOT change the `finish` method
  - Do NOT add any sorting or deduplication logic — `finish()` handles that

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []
    - Reason: Straightforward implementation but needs to satisfy TDD tests and maintain backward compatibility

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 4, 5)
  - **Blocks**: Tasks 6, 7, 9
  - **Blocked By**: Tasks 1, 2

  **References**:
  - `crates/igrep-core/src/index/writer.rs:14-18` — `IndexWriter` struct with `entries` and `file_paths` fields
  - `crates/igrep-core/src/index/writer.rs:20-45` — Existing `new()`, `add_file()`, `entry_count()` methods — follow the same style
  - `crates/igrep-core/src/types.rs:3-4` — `DocId = u32`, `NgramHash = u32` type aliases
  - `crates/igrep-core/src/index/writer.rs:127-202` — Tests from Task 2 that must now pass

  **Acceptance Criteria**:
  - [ ] `add_entries_bulk` method exists on `IndexWriter`
  - [ ] `cargo test -p igrep-core --lib -- writer` passes all tests (including new TDD tests)
  - [ ] `cargo test --workspace` passes with 0 failures
  - [ ] Byte-identity test passes: index from `add_entries_bulk` matches index from sequential `add_file`

  **QA Scenarios:**

  ```
  Scenario: add_entries_bulk produces identical index to add_file
    Tool: Bash
    Preconditions: Implementation complete, TDD tests written
    Steps:
      1. Run `cargo test -p igrep-core --lib -- test_add_entries_bulk_equivalent_to_add_file`
      2. Assert exit code is 0
      3. Run `cargo test -p igrep-core --lib -- test_parallel_index_byte_identity`
      4. Assert exit code is 0
    Expected Result: Both tests pass — byte-identical output
    Failure Indicators: Test failure mentioning "left != right" on byte comparison
    Evidence: .sisyphus/evidence/task-3-bulk-api-passes.txt

  Scenario: All existing tests still pass (no regressions)
    Tool: Bash
    Preconditions: add_entries_bulk implemented
    Steps:
      1. Run `cargo test --workspace 2>&1`
      2. Assert output contains "test result: ok"
      3. Assert exit code is 0
    Expected Result: Zero test failures across entire workspace
    Failure Indicators: Any "FAILED" in output
    Evidence: .sisyphus/evidence/task-3-no-regressions.txt
  ```

  **Commit**: YES (group 3)
  - Message: `feat(index): add add_entries_bulk method to IndexWriter`
  - Files: `crates/igrep-core/src/index/writer.rs`
  - Pre-commit: `cargo test --workspace`

- [ ] 4. Replace sort_unstable with par_sort_unstable in finish()

  **What to do**:
  - In `crates/igrep-core/src/index/writer.rs`, add `use rayon::slice::ParallelSliceMut;` to the imports
  - Change `self.entries.sort_unstable()` (line 55) to `self.entries.par_sort_unstable()`
  - This is a single-line change (plus import) — `par_sort_unstable` is a drop-in replacement for `sort_unstable` on `Vec<T: Ord + Send>`
  - `(NgramHash, DocId)` = `(u32, u32)` implements `Ord + Send`, so this works directly

  **Must NOT do**:
  - Do NOT parallelize the encoding loop (lines 60-72)
  - Do NOT change the `file_paths.sort_unstable_by_key()` call (line 86) — it's a small vec, not worth parallelizing
  - Do NOT add custom sort comparators

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []
    - Reason: Single-line change plus import

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 3, 5)
  - **Blocks**: Task 7
  - **Blocked By**: Tasks 1, 2

  **References**:
  - `crates/igrep-core/src/index/writer.rs:55` — The exact line to change: `self.entries.sort_unstable()`
  - `crates/igrep-core/src/index/writer.rs:1-11` — Import section where `use rayon::slice::ParallelSliceMut;` goes
  - Rayon docs: `par_sort_unstable` has identical semantics to `sort_unstable` but parallelizes across CPU cores

  **Acceptance Criteria**:
  - [ ] `self.entries.par_sort_unstable()` replaces `self.entries.sort_unstable()` in `finish()`
  - [ ] `use rayon::slice::ParallelSliceMut;` is in the imports
  - [ ] `cargo test --workspace` passes with 0 failures
  - [ ] Byte-identity tests from Task 2 still pass (parallel sort produces same output)

  **QA Scenarios:**

  ```
  Scenario: par_sort_unstable produces identical index output
    Tool: Bash
    Preconditions: par_sort_unstable replaces sort_unstable
    Steps:
      1. Run `cargo test -p igrep-core --lib -- test_finish_par_sort`
      2. Assert exit code is 0
      3. Run `cargo test -p igrep-core --lib -- writer`
      4. Assert all writer tests pass
    Expected Result: All tests pass — parallel sort produces byte-identical index
    Failure Indicators: Byte comparison failures in postings/lookup files
    Evidence: .sisyphus/evidence/task-4-par-sort-passes.txt

  Scenario: Integration test passes with parallel sort
    Tool: Bash
    Preconditions: par_sort_unstable in place
    Steps:
      1. Run `cargo test --test integration`
      2. Assert exit code is 0
    Expected Result: Oracle test (igrep vs grep) passes
    Failure Indicators: False negatives — grep finds files that igrep misses
    Evidence: .sisyphus/evidence/task-4-integration-passes.txt
  ```

  **Commit**: YES (group 4)
  - Message: `perf(index): replace sort_unstable with par_sort_unstable`
  - Files: `crates/igrep-core/src/index/writer.rs`
  - Pre-commit: `cargo test --workspace`

- [ ] 5. TDD — Write Failing Test for Parallel CLI Pipeline

  **What to do**:
  - In `crates/igrep-core/tests/parallel_indexing.rs` (created in Task 2), flesh out the test `parallel_pipeline_matches_sequential`:
    1. Use the existing `test-fixtures` directory as input
    2. Build index using the current sequential path: `walk()` → `for each file: writer.add_file()` → `writer.finish()`
    3. Build index using the parallel path: `walk()` → `rayon::par_iter` over files → collect entries → `writer.add_entries_bulk()` → `writer.finish()`
    4. Byte-compare all 3 output files between both indexes
  - Add a test `parallel_pipeline_with_thread_counts` that runs with `rayon::ThreadPoolBuilder::new().num_threads(N)` for N in {1, 2, 4}
  - The parallel path should use `build_all_ngrams` and `hash_ngram` from `igrep_core::ngram` directly

  **Must NOT do**:
  - Do NOT implement the parallel pipeline in the CLI yet — only write the integration-level test
  - Do NOT modify existing integration tests

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []
    - Reason: Integration test design with parallel execution, thread pool configuration, byte comparison across multiple files

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 3, 4)
  - **Blocks**: Task 7
  - **Blocked By**: Task 1

  **References**:
  - `crates/igrep-core/tests/integration.rs:20-35` — `setup_index` helper pattern to follow
  - `crates/igrep-core/src/ngram.rs:8,51-53` — `build_all_ngrams`, `hash_ngram` — public functions to call directly in parallel path
  - `crates/igrep-core/src/index/writer.rs:29-39` — `add_file` internals showing the per-file processing logic to replicate externally
  - `crates/igrep-core/src/walker.rs:26-69` — `walk()` function returning `Vec<(DocId, PathBuf)>`
  - `crates/igrep-core/src/types.rs:24-29` — `IndexConfig` with defaults

  **Acceptance Criteria**:
  - [ ] `parallel_pipeline_matches_sequential` test exists and passes (after Tasks 3+4)
  - [ ] `parallel_pipeline_with_thread_counts` test exists and passes
  - [ ] `cargo test --test parallel_indexing` passes with 0 failures
  - [ ] Tests verify byte-identity across `index.postings`, `index.lookup`, `index.files`

  **QA Scenarios:**

  ```
  Scenario: Parallel pipeline integration tests pass
    Tool: Bash
    Preconditions: Tasks 3 and 4 complete (add_entries_bulk + par_sort available)
    Steps:
      1. Run `cargo test --test parallel_indexing 2>&1`
      2. Assert output contains "test result: ok"
      3. Assert exit code is 0
    Expected Result: All parallel pipeline tests pass with byte-identical output
    Failure Indicators: Byte mismatch between sequential and parallel index files
    Evidence: .sisyphus/evidence/task-5-parallel-tests-pass.txt

  Scenario: Thread count variation produces consistent results
    Tool: Bash
    Preconditions: Tests written with ThreadPoolBuilder configuration
    Steps:
      1. Run `cargo test --test parallel_indexing -- thread_counts`
      2. Assert exit code is 0
    Expected Result: Identical index output with 1, 2, and 4 threads
    Failure Indicators: Results differ across thread counts (race condition)
    Evidence: .sisyphus/evidence/task-5-thread-count-variation.txt
  ```

  **Commit**: YES (group 5)
  - Message: `test: add parallel pipeline integration tests with byte-identity checks`
  - Files: `crates/igrep-core/tests/parallel_indexing.rs`
  - Pre-commit: `cargo test --test parallel_indexing`

---

- [ ] 6. Extract Per-File N-gram Processing into a Pure Function

  **What to do**:
  - In `crates/igrep-core/src/index/writer.rs`, extract the per-file processing logic from `add_file` into a standalone public function:
    ```rust
    pub fn process_file_ngrams(content: &[u8], doc_id: DocId, config: &IndexConfig) -> Vec<(NgramHash, DocId)>
    ```
  - This function takes file content, lowercases it, calls `build_all_ngrams`, hashes each n-gram, and returns the `(hash, doc_id)` pairs
  - Refactor `add_file` to call this new function internally (no behavior change)
  - This function is the unit of work that will be called in parallel from the CLI

  **Must NOT do**:
  - Do NOT change the behavior of `add_file`
  - Do NOT add parallelism here — this is just extraction/refactoring

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []
    - Reason: Simple extract-method refactoring

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 3 (sequential dependency on Task 3)
  - **Blocks**: Task 7
  - **Blocked By**: Task 3

  **References**:
  - `crates/igrep-core/src/index/writer.rs:29-39` — `add_file` method containing the logic to extract:
    - Line 34: `let lowered: Vec<u8> = content.iter().map(|b| b.to_ascii_lowercase()).collect();`
    - Line 35: `let ngrams = build_all_ngrams(&lowered, &self.config);`
    - Lines 36-38: hash loop pushing `(hash_ngram(ngram), doc_id)`
  - `crates/igrep-core/src/ngram.rs:8,51-53` — `build_all_ngrams`, `hash_ngram` used inside

  **Acceptance Criteria**:
  - [ ] `process_file_ngrams` function exists and is `pub`
  - [ ] `add_file` calls `process_file_ngrams` internally
  - [ ] `cargo test --workspace` passes with 0 failures (behavioral equivalence)
  - [ ] Integration test still passes

  **QA Scenarios:**

  ```
  Scenario: Refactored add_file is behaviorally equivalent
    Tool: Bash
    Preconditions: process_file_ngrams extracted, add_file refactored
    Steps:
      1. Run `cargo test --workspace 2>&1`
      2. Assert output contains "test result: ok"
      3. Assert exit code is 0
      4. Run `cargo test --test integration`
      5. Assert exit code is 0
    Expected Result: All tests pass — refactoring is behavior-preserving
    Failure Indicators: Any test failure (would indicate extraction changed semantics)
    Evidence: .sisyphus/evidence/task-6-refactor-passes.txt
  ```

  **Commit**: YES (group 6)
  - Message: `refactor(index): extract process_file_ngrams pure function`
  - Files: `crates/igrep-core/src/index/writer.rs`
  - Pre-commit: `cargo test --workspace`

- [ ] 7. Implement Parallel Indexing Pipeline in CLI

  **What to do**:
  - In `crates/igrep-cli/src/bin/igrep_index.rs`, replace the sequential file processing loop (lines 70-96) with a Rayon-based parallel pipeline:
    1. Keep the sequential `walk()` call and file list collection (lines 43-57) — DocId assignment stays sequential
    2. Replace the `for (i, (doc_id, abs_path, rel_path)) in all_files.iter().enumerate()` loop with:
       ```rust
       use rayon::prelude::*;
       use igrep_core::index::writer::process_file_ngrams;
       
       let results: Vec<(Vec<(NgramHash, DocId)>, (DocId, String))> = all_files
           .par_iter()
           .map(|(doc_id, abs_path, rel_path)| {
               let content = std::fs::read(abs_path).unwrap_or_default();
               // Track progress via AtomicUsize (Task 8)
               let entries = process_file_ngrams(&content, *doc_id, &config);
               let rel_str = rel_path.to_string_lossy().to_string();
               (entries, (*doc_id, rel_str))
           })
           .collect();
       ```
    3. Flatten results and pass to `writer.add_entries_bulk()`:
       ```rust
       let mut all_entries = Vec::new();
       let mut all_paths = Vec::new();
       for (entries, file_info) in results {
           all_entries.extend(entries);
           all_paths.push(file_info);
       }
       writer.add_entries_bulk(all_entries, all_paths);
       ```
    4. The rest of `finish()` stays the same (now uses `par_sort_unstable` from Task 4)
  - Add `rayon` to `igrep-cli/Cargo.toml` if needed (for `par_iter` in the binary)

  **Must NOT do**:
  - Do NOT change the walker or file discovery logic
  - Do NOT change the `finish()` call
  - Do NOT add thread pool configuration CLI flags
  - Do NOT change error handling (`unwrap_or_default` stays)

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []
    - Reason: Core parallelization work — needs careful handling of data collection, flattening, and ensuring the pipeline matches the sequential path exactly

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 3 (depends on Tasks 3, 4, 5, 6)
  - **Blocks**: Task 8
  - **Blocked By**: Tasks 3, 4, 5, 6

  **References**:
  - `crates/igrep-cli/src/bin/igrep_index.rs:63-96` — Current sequential loop to replace (read file → writer.add_file → progress)
  - `crates/igrep-cli/src/bin/igrep_index.rs:43-57` — File collection loop (stays sequential)
  - `crates/igrep-cli/src/bin/igrep_index.rs:99-110` — Post-processing (finish + git SHA) — stays unchanged
  - `crates/igrep-core/src/index/writer.rs` — `add_entries_bulk` (from Task 3) and `process_file_ngrams` (from Task 6)
  - `crates/igrep-core/src/types.rs:3-4` — `DocId`, `NgramHash` types

  **Acceptance Criteria**:
  - [ ] Sequential `for` loop replaced with `par_iter().map().collect()` pattern
  - [ ] `add_entries_bulk` called with collected results
  - [ ] `cargo test --workspace` passes with 0 failures
  - [ ] `cargo test --test integration` passes (oracle test)
  - [ ] `cargo test --test parallel_indexing` passes (byte-identity)
  - [ ] `cargo build -p igrep-cli --release` succeeds

  **QA Scenarios:**

  ```
  Scenario: Parallel CLI builds and all tests pass
    Tool: Bash
    Preconditions: All Wave 2 tasks complete
    Steps:
      1. Run `cargo build -p igrep-cli --release`
      2. Assert exit code is 0
      3. Run `cargo test --workspace 2>&1`
      4. Assert output contains "test result: ok"
      5. Run `cargo test --test integration`
      6. Assert exit code is 0
    Expected Result: Binary builds, all tests pass including oracle test
    Failure Indicators: Build errors, test failures, false negatives in grep comparison
    Evidence: .sisyphus/evidence/task-7-parallel-cli.txt

  Scenario: CLI produces valid index on real directory
    Tool: Bash
    Preconditions: Release binary built
    Steps:
      1. Run `./target/release/igrep-index crates/ --output-dir /tmp/igrep-test-index 2>&1`
      2. Assert output contains "Indexed" and a file count > 0
      3. Assert `/tmp/igrep-test-index/index.postings` exists
      4. Assert `/tmp/igrep-test-index/index.lookup` exists
      5. Assert `/tmp/igrep-test-index/index.files` exists
      6. Run `./target/release/igrep --index-dir /tmp/igrep-test-index "fn main" crates/`
      7. Assert output contains at least one result
    Expected Result: Index is created and searchable
    Failure Indicators: Missing index files, zero results for known pattern, crash
    Evidence: .sisyphus/evidence/task-7-cli-index-search.txt

  Scenario: Parallel and sequential produce byte-identical indexes
    Tool: Bash
    Preconditions: Both code paths available
    Steps:
      1. Run `cargo test --test parallel_indexing -- parallel_pipeline_matches_sequential`
      2. Assert exit code is 0
    Expected Result: Byte-identical output files
    Failure Indicators: Byte differences in any of the 3 index files
    Evidence: .sisyphus/evidence/task-7-byte-identity.txt
  ```

  **Commit**: YES (group 7)
  - Message: `feat(cli): implement parallel indexing pipeline with rayon`
  - Files: `crates/igrep-cli/src/bin/igrep_index.rs`, possibly `crates/igrep-cli/Cargo.toml`
  - Pre-commit: `cargo test --workspace`

- [ ] 8. Update CLI Progress Reporting for Parallel Execution

  **What to do**:
  - In `crates/igrep-cli/src/bin/igrep_index.rs`, update the progress reporting to work with parallel execution:
    1. Create an `AtomicUsize` counter for files processed and an `AtomicU64` for bytes processed
    2. Inside the `par_iter` closure, increment the counters with `fetch_add(1, Ordering::Relaxed)`
    3. Spawn a separate thread (or use Rayon's `scope`) to periodically read the counters and print progress every 500ms
    4. After `par_iter().collect()` completes, join the progress thread and print final stats
  - Keep the same progress format: `[pct%] done/total files (MB) — files/s`
  - The "Sorting N n-gram entries..." message stays as-is (it's after the parallel phase)

  **Must NOT do**:
  - Do NOT use `println!` or `eprintln!` inside the `par_iter` closure (would interleave)
  - Do NOT add external dependencies (use `std::sync::atomic`)

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []
    - Reason: Straightforward AtomicUsize counter replacement

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 3 (depends on Task 7)
  - **Blocks**: None
  - **Blocked By**: Task 7

  **References**:
  - `crates/igrep-cli/src/bin/igrep_index.rs:66-97` — Current progress reporting code with sequential counter, 500ms interval, eprint! format
  - `std::sync::atomic::AtomicUsize`, `AtomicU64`, `Ordering::Relaxed` — for thread-safe counters

  **Acceptance Criteria**:
  - [ ] Progress uses `AtomicUsize`/`AtomicU64` counters
  - [ ] No `eprintln!`/`eprint!` inside `par_iter` closures
  - [ ] `cargo build -p igrep-cli` succeeds
  - [ ] Progress output shows during parallel indexing (manual verification via QA)

  **QA Scenarios:**

  ```
  Scenario: Progress reporting works during parallel indexing
    Tool: Bash
    Preconditions: Parallel CLI with AtomicUsize progress
    Steps:
      1. Run `cargo build -p igrep-cli --release`
      2. Assert exit code is 0
      3. Run `./target/release/igrep-index crates/ --output-dir /tmp/igrep-progress-test 2>&1`
      4. Assert output contains percentage (e.g., "100.0%")
      5. Assert output contains "Sorting"
      6. Assert output contains "Indexed"
    Expected Result: Progress percentages and final stats are printed
    Failure Indicators: No progress output, garbled/interleaved output, crash
    Evidence: .sisyphus/evidence/task-8-progress-reporting.txt
  ```

  **Commit**: YES (group 8)
  - Message: `fix(cli): update progress reporting for parallel execution`
  - Files: `crates/igrep-cli/src/bin/igrep_index.rs`
  - Pre-commit: `cargo build -p igrep-cli`

- [ ] 9. Add End-to-End Indexing Benchmark

  **What to do**:
  - Create `crates/igrep-core/benches/index_bench.rs` with Criterion benchmarks:
    1. `bench_index_end_to_end` — generate synthetic data (10,000 files × 1KB each as `Vec<(DocId, String, Vec<u8>)>`), measure full pipeline: `add_entries_bulk` + `finish()` time
    2. `bench_par_sort_vs_sequential` — generate a `Vec<(u32, u32)>` with 1M entries, compare `sort_unstable` vs `par_sort_unstable` (to quantify the sort speedup independently)
    3. `bench_process_file_ngrams` — measure the per-file n-gram extraction on various file sizes (1KB, 10KB, 100KB)
  - Add `[[bench]] name = "index_bench" harness = false` to `crates/igrep-core/Cargo.toml`
  - Synthetic data should use deterministic content (seeded byte patterns) for reproducible benchmarks

  **Must NOT do**:
  - Do NOT benchmark on test-fixtures (too small — 27 files)
  - Do NOT modify existing benchmarks

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []
    - Reason: Needs careful benchmark design with synthetic data generation and proper Criterion setup

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 6, 7, 8 — only depends on 3, 4)
  - **Blocks**: None
  - **Blocked By**: Tasks 3, 4

  **References**:
  - `crates/igrep-core/benches/ngram_bench.rs` — Existing benchmark patterns to follow (criterion_group, criterion_main, BenchmarkId, synthetic input generation)
  - `crates/igrep-core/benches/search_bench.rs` — Another benchmark example
  - `crates/igrep-core/Cargo.toml:23-29` — Existing `[[bench]]` entries to follow for adding `index_bench`
  - `crates/igrep-core/src/index/writer.rs` — `IndexWriter::new()`, `add_entries_bulk()`, `process_file_ngrams()`, `finish()`

  **Acceptance Criteria**:
  - [ ] `crates/igrep-core/benches/index_bench.rs` exists with 3 benchmark functions
  - [ ] `[[bench]] name = "index_bench" harness = false` in `Cargo.toml`
  - [ ] `cargo bench --bench index_bench -- --test` compiles and runs (quick test mode)
  - [ ] `cargo bench --bench index_bench` produces criterion HTML reports

  **QA Scenarios:**

  ```
  Scenario: Indexing benchmark compiles and runs
    Tool: Bash
    Preconditions: Tasks 3, 4 complete
    Steps:
      1. Run `cargo bench --bench index_bench -- --test 2>&1`
      2. Assert exit code is 0
    Expected Result: Benchmark compiles and quick-test passes
    Failure Indicators: Compilation errors, runtime panics
    Evidence: .sisyphus/evidence/task-9-bench-compiles.txt

  Scenario: Full benchmark suite produces results
    Tool: Bash
    Preconditions: Benchmark file exists
    Steps:
      1. Run `cargo bench --bench index_bench 2>&1 | head -50`
      2. Assert output contains "bench_index_end_to_end"
      3. Assert output contains "bench_par_sort_vs_sequential"
    Expected Result: All 3 benchmarks produce timing results
    Failure Indicators: Missing benchmark names, panics during measurement
    Evidence: .sisyphus/evidence/task-9-bench-results.txt
  ```

  **Commit**: YES (group 9)
  - Message: `bench: add end-to-end indexing benchmark`
  - Files: `crates/igrep-core/benches/index_bench.rs`, `crates/igrep-core/Cargo.toml`
  - Pre-commit: `cargo bench --bench index_bench -- --test`

---

## Final Verification Wave (MANDATORY — after ALL implementation tasks)

> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.

- [ ] F1. **Plan Compliance Audit** — `oracle`
  Read the plan end-to-end. For each "Must Have": verify implementation exists (read file, run command). For each "Must NOT Have": search codebase for forbidden patterns — reject with file:line if found. Check evidence files exist in .sisyphus/evidence/. Compare deliverables against plan.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [ ] F2. **Code Quality Review** — `unspecified-high`
  Run `cargo clippy --workspace` + `cargo test --workspace`. Review all changed files for: `unsafe` blocks, `unwrap()` in non-test code, commented-out code, unused imports, AI slop (excessive comments, over-abstraction). Verify no new warnings.
  Output: `Build [PASS/FAIL] | Clippy [PASS/FAIL] | Tests [N pass/N fail] | Files [N clean/N issues] | VERDICT`

- [ ] F3. **Real Manual QA** — `unspecified-high`
  Start from clean state. Build release binary. Run `igrep-index` on the project's own `crates/` directory. Verify index files are created. Run `igrep` to search the index. Compare `igrep-index` output stats against sequential build (same file count, same n-gram count). Measure wall-clock time for both paths. Save evidence to `.sisyphus/evidence/final-qa/`.
  Output: `Scenarios [N/N pass] | Byte-identity [PASS/FAIL] | Speedup [Nx] | VERDICT`

- [ ] F4. **Scope Fidelity Check** — `deep`
  For each task: read "What to do", read actual diff (git log/diff). Verify 1:1 — everything in spec was built (no missing), nothing beyond spec was built (no creep). Check "Must NOT do" compliance. Detect cross-task contamination. Flag unaccounted changes.
  Output: `Tasks [N/N compliant] | Contamination [CLEAN/N issues] | Unaccounted [CLEAN/N files] | VERDICT`

---

## Commit Strategy

| Commit | Message | Files | Pre-commit |
|--------|---------|-------|------------|
| 1 | `build: add rayon dependency to workspace` | `Cargo.toml`, `crates/igrep-core/Cargo.toml` | `cargo check --workspace` |
| 2 | `test: add failing TDD tests for parallel IndexWriter API` | `crates/igrep-core/src/index/writer.rs` (tests) | `cargo test -p igrep-core -- writer` (expect failures) |
| 3 | `feat(index): add add_entries_bulk method to IndexWriter` | `crates/igrep-core/src/index/writer.rs` | `cargo test --workspace` |
| 4 | `perf(index): replace sort_unstable with par_sort_unstable` | `crates/igrep-core/src/index/writer.rs` | `cargo test --workspace` |
| 5 | `test: add failing TDD test for parallel CLI pipeline` | `crates/igrep-core/tests/parallel_indexing.rs` | compile check |
| 6 | `refactor(index): extract process_file_ngrams pure function` | `crates/igrep-core/src/index/writer.rs` or `ngram.rs` | `cargo test --workspace` |
| 7 | `feat(cli): implement parallel indexing pipeline with rayon` | `crates/igrep-cli/src/bin/igrep_index.rs` | `cargo test --workspace` |
| 8 | `fix(cli): update progress reporting for parallel execution` | `crates/igrep-cli/src/bin/igrep_index.rs` | `cargo build -p igrep-cli` |
| 9 | `bench: add end-to-end indexing benchmark` | `crates/igrep-core/benches/index_bench.rs`, `Cargo.toml` | `cargo bench --bench index_bench -- --test` |

---

## Success Criteria

### Verification Commands
```bash
cargo test --workspace                    # Expected: all tests pass, 0 failures
cargo test --test integration             # Expected: oracle test passes
cargo test --test parallel_indexing       # Expected: byte-identity tests pass
cargo bench --bench index_bench -- --test # Expected: benchmark compiles
cargo clippy --workspace                  # Expected: no new warnings
```

### Final Checklist
- [ ] All "Must Have" deliverables present
- [ ] All "Must NOT Have" guardrails respected
- [ ] All tests pass (existing + new)
- [ ] Byte-identical output between sequential and parallel paths
- [ ] No `unsafe` code
- [ ] No changes to on-disk format
