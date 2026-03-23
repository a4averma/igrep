# instant-grep: Fast Regex Search with N-gram Indexes

## TL;DR

> **Quick Summary**: Build a Rust-based fast regex search tool that indexes source code using sparse n-gram techniques (from Cursor's blog post by Vicent Marti). Implements a full pipeline from classic trigrams through probabilistic masks to sparse n-grams with frequency-based weight functions, using mmap'd two-file index format and git-based versioning.
> 
> **Deliverables**:
> - `igrep-core` library crate — sparse n-gram engine, posting lists, regex decomposition, index format, git overlay
> - `igrep-cli` binary crate — `igrep-index` (build) and `igrep` (search) CLI tools
> - Pre-computed bigram frequency table (256×256 = 64KB, embedded via `include_bytes!`)
> - Comprehensive TDD test suite with property-based tests
> - Criterion benchmarks targeting Linux kernel corpus
> - Test fixtures with deterministic, known-answer test cases
> 
> **Estimated Effort**: Large (20+ tasks across 4 waves)
> **Parallel Execution**: YES - 4 waves
> **Critical Path**: Task 1 → Task 3 → Task 5 → Task 10 → Task 14 → Task 17 → Task 20 → F1-F4

---

## Context

### Original Request
Build "instant-grep" in Rust or Go, inspired by Vicent Marti's Cursor blog post on fast regex search indexing for agent tools. Include test cases and benchmarks. Implement the full pipeline: classic trigrams → probabilistic masks → sparse n-grams.

### Interview Summary
**Key Discussions**:
- **Language**: Rust — better mmap (memmap2), zero-copy deserialization, regex-syntax crate, no GC interference
- **Index strategy**: Full pipeline — classic trigrams, probabilistic masks (nextMask/locMask), sparse n-grams with frequency-based weights
- **Deliverable**: Library + CLI — reusable crate with two CLI tools (igrep-index, igrep)
- **Target scale**: Large monorepos (1-10+ GB source code)
- **Index updates**: Git-based versioning — base index at commit SHA, dirty overlay layer
- **Test strategy**: TDD with criterion benchmarks
- **Weight function**: Pre-computed bigram frequency table from OSS corpus
- **CLI design**: Two commands (igrep-index builds, igrep searches)

**Research Findings**:
- **google/codesearch** (Go): Classic trigram index, V2 γ-coded format, ~20% of corpus size. 2-pass radix sort construction. Cox 5-property regex decomposition algorithm.
- **sourcegraph/zoekt** (Go): Positional trigrams with byte offsets, B+-tree index, bloom filter augmentation. Shards capped at uint32 (4GB).
- **GitHub Blackbird**: Sparse n-grams at scale (45M repos, 115TB). Monotone-stack O(n) algorithm. Sharding by git blob SHA. Abandoned bloom filter masks due to saturation.
- **danlark1/sparse_ngrams** (C++): Reference implementation ~100 lines. build_all (monotone stack, ≤2n-2 outputs) + build_covering (deque, ≤n-2 outputs).
- **Rust ecosystem**: memmap2, regex-syntax, ignore, crc32fast, criterion — all production-proven in ripgrep, Turbopack, Qdrant, YARA-X.
- **Russ Cox 2012**: Full regex→trigram decomposition: 5 properties (emptyable, exact, prefix, suffix, match), information-saving/discarding transforms, AND/OR query tree.

### Metis Review
**Identified Gaps** (addressed):
- **Hash function for n-gram keys**: Resolved → CRC32-C via `crc32fast` (hardware-accelerated). 32-bit hashes, sorted for binary search.
- **Posting list ID space**: Resolved → file IDs (uint32), not byte offsets. Simpler, sufficient for file-level candidate filtering.
- **Frequency table generation**: Resolved → ship as `include_bytes!` embedded `[u32; 65536]` table. One-time offline generation step.
- **Index size constraints**: Resolved → uint32 file IDs, uint64 postings file offsets. Max 4B files, unlimited postings size.
- **Fallback for undecomposable regex**: Resolved → when query tree is `QAll`, fall back to brute-force grep over all indexed files.
- **Git overlay strategy**: Resolved → in-memory overlay. Startup: `git diff HEAD --name-only` + `git ls-files --others`. Query both, union results, exclude deleted file IDs.
- **Concurrency model**: Resolved → 2-pass indexing. Pass 1: parallel file walk + n-gram extraction → sharded temp files. Pass 2: sort each shard, write sequential posting lists + lookup table.
- **Case-insensitive search**: Resolved → index lowercased text, run regex verification on original file content. Simpler than query expansion.
- **Binary file detection**: Resolved → null byte in first 16KB → skip. Files >50MB → skip (configurable).
- **Index corruption/versioning**: Resolved → magic header + version number + trailer checksum.

---

## Work Objectives

### Core Objective
Build a production-quality Rust library and CLI tool that indexes source code repositories using sparse n-gram inverted indexes, enabling sub-second regex searches on multi-GB monorepos that currently take 15+ seconds with ripgrep.

### Concrete Deliverables
- Cargo workspace: `igrep-core` (library) + `igrep-cli` (binary)
- Core library with: sparse n-gram engine, posting lists, regex decomposition, index reader/writer, git overlay
- CLI tools: `igrep-index` (build index) and `igrep` (search with grep-compatible output)
- Embedded bigram frequency table (64KB, `include_bytes!`)
- Test fixtures directory with deterministic test corpus
- Property-based test suite (proptest)
- Criterion benchmark suite
- README with usage examples

### Definition of Done
- [ ] `cargo test --workspace` passes with 0 failures
- [ ] `cargo bench --workspace` runs without errors
- [ ] `igrep-index ./test-fixtures && igrep "fn main" ./test-fixtures` returns correct results
- [ ] For 20+ regex patterns, `igrep -l` returns same files as `grep -rPl` (zero false negatives)
- [ ] Property tests pass: build_covering ⊆ build_all, output bounds, encode/decode roundtrips
- [ ] Index size ≤ 30% of corpus size on test fixtures
- [ ] `cargo clippy --workspace` has zero warnings

### Must Have
- Sparse n-gram build_all and build_covering algorithms with frequency-based weight function
- Regex → AND/OR query tree decomposition (Cox 5-property algorithm)
- Two-file mmap'd index format (lookup table + postings file) with atomic writes
- Delta-encoded varint posting lists with intersection (AND) and union (OR)
- Parallel file walking with .gitignore support (via `ignore` crate)
- Git-based index versioning with in-memory dirty overlay
- CLI with grep-compatible output format and standard flags (-l, -c, -n, -i)
- Zero false negatives guarantee (index may over-select, never under-select)
- Binary file detection and skipping
- Max n-gram length cap of 16

### Must NOT Have (Guardrails)
- No incremental index updates, mutable posting lists, or compaction
- No relevance ranking, scoring, or result ordering beyond file path sort
- No server mode, HTTP API, watch mode, or distributed features
- No sharding or multi-index merging
- No language-aware parsing, AST analysis, or semantic understanding
- No Unicode normalization — byte-level indexing only
- No custom output formats (JSON, structured) — grep-like text only
- No regex syntax extensions beyond RE2-compatible (via regex-syntax)
- No compression of index files (raw mmap'd format for zero-copy reads)
- No big-endian encoding (local tool, matches native x86/ARM byte order)
- No excessive comments, over-abstraction, or generic names (data/result/item/temp)

---

## Verification Strategy

> **ZERO HUMAN INTERVENTION** — ALL verification is agent-executed. No exceptions.

### Test Decision
- **Infrastructure exists**: NO (greenfield project)
- **Automated tests**: TDD (Red-Green-Refactor)
- **Framework**: Rust built-in `#[test]` + `proptest` for property tests + `criterion` for benchmarks
- **Setup**: Task 1 initializes the Cargo workspace with test infrastructure

### QA Policy
Every task MUST include agent-executed QA scenarios.
Evidence saved to `.sisyphus/evidence/task-{N}-{scenario-slug}.{ext}`.

- **Library modules**: Use Bash (`cargo test`) — run specific test module, assert pass count
- **CLI tools**: Use interactive_bash (tmux) — run CLI commands, validate stdout/stderr, check exit codes
- **Benchmarks**: Use Bash (`cargo bench`) — run benchmark, verify it completes without errors
- **Integration**: Use Bash — run igrep pipeline end-to-end, diff output against grep

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Foundation — project scaffolding + core types + frequency table):
├── Task 1:  Initialize Cargo workspace + test infrastructure [quick]
├── Task 2:  Generate and embed bigram frequency table [unspecified-high]
├── Task 3:  Core types module (DocId, NgramHash, Query, PostingList) [quick]
└── Task 4:  Test fixtures directory with known-answer corpus [quick]

Wave 2 (Core Algorithms — MAX PARALLEL, all pure functions):
├── Task 5:  Sparse n-gram engine: build_all (monotone stack) [deep]
├── Task 6:  Sparse n-gram engine: build_covering (deque) [deep]
├── Task 7:  Classic trigram extraction + probabilistic masks [unspecified-high]
├── Task 8:  Varint encoding/decoding + delta-encoded posting lists [unspecified-high]
├── Task 9:  Posting list intersection (AND) and union (OR) [unspecified-high]
└── Task 10: Regex → query tree decomposition (Cox 5-property) [deep]

Wave 3 (I/O + Integration — index format, file walking, search pipeline):
├── Task 11: Index writer (2-pass: extract → sort → write) [deep]
├── Task 12: Index reader (mmap + binary search + posting decode) [deep]
├── Task 13: File walker with binary detection + .gitignore [unspecified-high]
├── Task 14: End-to-end search pipeline (index → query → verify) [deep]
├── Task 15: Git-based index versioning + dirty overlay [unspecified-high]
└── Task 16: Case-insensitive search support [unspecified-high]

Wave 4 (CLI + Polish — user-facing tools, benchmarks, docs):
├── Task 17: igrep-index CLI command with clap [quick]
├── Task 18: igrep search CLI command with grep-compatible output [unspecified-high]
├── Task 19: CLI flags: -l, -c, -n, -i, -f [quick]
├── Task 20: Criterion benchmarks (index build + search latency) [unspecified-high]
└── Task 21: Integration tests: igrep vs grep correctness [deep]

Wave FINAL (After ALL tasks — 4 parallel reviews, then user okay):
├── Task F1: Plan compliance audit (oracle)
├── Task F2: Code quality review (unspecified-high)
├── Task F3: Real manual QA (unspecified-high)
└── Task F4: Scope fidelity check (deep)
-> Present results -> Get explicit user okay
```

### Critical Path
Task 1 → Task 3 → Task 5 → Task 10 → Task 14 → Task 17 → Task 21 → F1-F4 → user okay

### Parallel Speedup
- Wave 1: 4 tasks parallel
- Wave 2: 6 tasks parallel (core algorithms are all pure functions, no I/O dependencies)
- Wave 3: 6 tasks parallel (depend on Wave 2 outputs but independent of each other)
- Wave 4: 5 tasks parallel
- ~70% faster than sequential

### Dependency Matrix

| Task | Depends On | Blocks |
|------|-----------|--------|
| 1 | — | 2,3,4,5,6,7,8,9,10 |
| 2 | 1 | 5,6,7 |
| 3 | 1 | 5,6,7,8,9,10,11,12,14 |
| 4 | 1 | 14,21 |
| 5 | 2,3 | 6,11,14 |
| 6 | 2,3,5 | 10,11,14 |
| 7 | 2,3 | 11,14,16 |
| 8 | 3 | 9,11,12 |
| 9 | 3,8 | 11,12,14 |
| 10 | 3,6 | 14 |
| 11 | 5,6,7,8,9,13 | 12,14,17 |
| 12 | 8,9,11 | 14,18 |
| 13 | 1 | 11,14 |
| 14 | 10,11,12,4 | 15,16,17,18,21 |
| 15 | 14 | 18 |
| 16 | 7,14 | 19 |
| 17 | 11,14 | 20,21 |
| 18 | 12,14,15 | 19,21 |
| 19 | 18,16 | 21 |
| 20 | 17,18 | F1-F4 |
| 21 | 14,17,19,4 | F1-F4 |

### Agent Dispatch Summary

- **Wave 1**: **4 tasks** — T1 `quick`, T2 `unspecified-high`, T3 `quick`, T4 `quick`
- **Wave 2**: **6 tasks** — T5 `deep`, T6 `deep`, T7 `unspecified-high`, T8 `unspecified-high`, T9 `unspecified-high`, T10 `deep`
- **Wave 3**: **6 tasks** — T11 `deep`, T12 `deep`, T13 `unspecified-high`, T14 `deep`, T15 `unspecified-high`, T16 `unspecified-high`
- **Wave 4**: **5 tasks** — T17 `quick`, T18 `unspecified-high`, T19 `quick`, T20 `unspecified-high`, T21 `deep`
- **FINAL**: **4 tasks** — F1 `oracle`, F2 `unspecified-high`, F3 `unspecified-high`, F4 `deep`

---

## TODOs

- [x] 1. Initialize Cargo workspace with igrep-core and igrep-cli crates

  **What to do**:
  - Create a Cargo workspace at the project root with two member crates: `crates/igrep-core` (lib) and `crates/igrep-cli` (bin)
  - Root `Cargo.toml` with `[workspace]` members, shared `[workspace.dependencies]` for: `regex-syntax = "0.8"`, `regex = "1"`, `memmap2 = "0.9"`, `ignore = "0.4"`, `crc32fast = "1"`, `clap = { version = "4", features = ["derive"] }`, `proptest = "1"`, `criterion = { version = "0.5", features = ["html_reports"] }`, `anyhow = "1"`, `bstr = "1"`, `tempfile = "3"`
  - `crates/igrep-core/Cargo.toml`: lib crate with deps from workspace, `[dev-dependencies]` for proptest + criterion
  - `crates/igrep-cli/Cargo.toml`: bin crate depending on `igrep-core`, clap
  - `crates/igrep-cli/Cargo.toml` should have `[[bin]]` entries for both `igrep-index` and `igrep` (two separate binaries)
  - `crates/igrep-core/src/lib.rs`: empty pub modules declared (ngram, trigram, posting, query, index, walker, search, git_overlay, types, frequency_table)
  - `crates/igrep-cli/src/main.rs`: minimal main that just prints "igrep" for now
  - Create `crates/igrep-cli/src/bin/igrep_index.rs` and `crates/igrep-cli/src/bin/igrep.rs` as minimal bin entry points
  - Initialize git repo: `git init`, create `.gitignore` (target/, *.swp, .DS_Store)
  - Verify: `cargo check --workspace` passes, `cargo test --workspace` passes (0 tests)

  **Must NOT do**:
  - Do not add any implementation code — just scaffolding
  - Do not add README yet (Task 21 handles docs)

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Project scaffolding with known structure, no algorithmic complexity
  - **Skills**: []
  - **Skills Evaluated but Omitted**:
    - `git-master`: Simple `git init`, no complex git ops needed

  **Parallelization**:
  - **Can Run In Parallel**: NO (must be first)
  - **Parallel Group**: Wave 1 (first task)
  - **Blocks**: Tasks 2, 3, 4, 5, 6, 7, 8, 9, 10, 13
  - **Blocked By**: None

  **References**:

  **Pattern References**:
  - No existing code in this repo (greenfield). Follow ripgrep's workspace pattern: `crates/` subdirectory with `lib` and `bin` crates.

  **External References**:
  - Cargo workspace docs: https://doc.rust-lang.org/cargo/reference/workspaces.html
  - ripgrep workspace structure: `crates/regex/`, `crates/ignore/`, `crates/cli/` — one library crate + one binary crate pattern

  **WHY Each Reference Matters**:
  - Cargo workspace docs: Exact syntax for `[workspace]` and `[workspace.dependencies]` inheritance
  - ripgrep structure: Proven pattern for Rust search tools — library crate with CLI wrapper

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Workspace compiles cleanly
    Tool: Bash
    Preconditions: Project directory exists at /Users/adityaverma/Developer/instant-grep
    Steps:
      1. Run `cargo check --workspace`
      2. Assert exit code is 0
      3. Run `cargo test --workspace`
      4. Assert exit code is 0 and output contains "0 passed" or "running 0 tests"
    Expected Result: Both commands exit 0, no compilation errors
    Failure Indicators: Any "error[E" in output, non-zero exit code
    Evidence: .sisyphus/evidence/task-1-workspace-compiles.txt

  Scenario: Both binary targets exist
    Tool: Bash
    Preconditions: Workspace initialized
    Steps:
      1. Run `cargo build --workspace`
      2. Check that `target/debug/igrep-index` or `target/debug/igrep_index` exists
      3. Check that `target/debug/igrep` exists
      4. Run each binary, verify it exits without panic
    Expected Result: Both binaries compile and run without crash
    Failure Indicators: Missing binary, panic output, non-zero exit code
    Evidence: .sisyphus/evidence/task-1-binaries-exist.txt

  Scenario: Git repo initialized with .gitignore
    Tool: Bash
    Preconditions: Workspace created
    Steps:
      1. Run `git status` in project root
      2. Assert output does NOT contain "not a git repository"
      3. Run `cat .gitignore`
      4. Assert it contains "target/" and ".DS_Store"
    Expected Result: Git repo exists, .gitignore has essential entries
    Failure Indicators: "fatal: not a git repository", missing .gitignore
    Evidence: .sisyphus/evidence/task-1-git-init.txt
  ```

  **Commit**: YES
  - Message: `chore: initialize cargo workspace with igrep-core and igrep-cli crates`
  - Files: `Cargo.toml, crates/**, .gitignore`
  - Pre-commit: `cargo check --workspace`

- [x] 2. Generate and embed bigram frequency table for sparse n-gram weights

  **What to do**:
  - Create `crates/igrep-core/src/frequency_table.rs` with a module that provides bigram weights
  - Generate a `[u32; 65536]` frequency table where index = `byte1 * 256 + byte2` and value = weight (higher = rarer)
  - For the weight function: use the known frequency distribution of byte pairs in source code. Printable ASCII pairs that are common in code (e.g., `"th"`, `"he"`, `"in"`, `" t"`, `"e "`) get LOW weights. Rare/unusual pairs (e.g., `"qz"`, `"#!"`, `"zx"`) get HIGH weights.
  - Approach: Write a build script (`crates/igrep-core/build.rs`) OR a standalone binary (`crates/igrep-core/src/bin/gen_freq_table.rs`) that:
    1. Scans a corpus of source code files (can use the Rust stdlib source at `$(rustc --print sysroot)/lib/rustlib/src/rust/library/` as a readily available corpus)
    2. Counts all byte-pair occurrences
    3. Ranks by frequency (most common = rank 0, least common = rank 65535)
    4. Writes as a binary file `crates/igrep-core/data/bigram_freq.bin` (256KB)
  - In `frequency_table.rs`: `static BIGRAM_WEIGHTS: &[u32; 65536] = include_bytes!("../data/bigram_freq.bin")` (via transmute or zerocopy)
  - Provide `pub fn bigram_weight(b1: u8, b2: u8) -> u32` that returns the weight
  - Alternative simpler approach: hardcode a reasonable approximation based on known English/code character frequencies without needing a corpus scan. Use ASCII printable frequency data.
  - Write unit tests:
    - `space + common_letter` has lower weight than `rare_punct + rare_punct`
    - `bigram_weight(b'e', b' ') < bigram_weight(b'q', b'z')`
    - All 65536 entries are populated (no zeros in unexpected places)

  **Must NOT do**:
  - Do not download external corpora — use Rust stdlib source or a reasonable approximation
  - Do not over-engineer: a good approximation is better than a perfect table

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Requires data generation, file I/O, and understanding of character frequency distributions
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 3, 4 after Task 1)
  - **Parallel Group**: Wave 1 (with Tasks 3, 4)
  - **Blocks**: Tasks 5, 6, 7
  - **Blocked By**: Task 1

  **References**:

  **Pattern References**:
  - `danlark1/sparse_ngrams/src/sparse_ngrams.cc:12-19` — The reference HashBigram function uses multiplicative hash. We want frequency-based instead.

  **External References**:
  - Cursor blog post "Sparse N-grams" section: describes using character-pair frequency from open-source corpus as weight function
  - ClickHouse `sparseGramsImpl.h`: Uses CRC32-C as weight function (simpler but less optimal)

  **WHY Each Reference Matters**:
  - danlark1 HashBigram: Shows the API contract — `fn(byte, byte) -> u32` — that build_all/build_covering expect
  - Blog post: Explains why frequency-based is better than CRC32 (rare pairs become boundaries → longer more selective n-grams)

  **Acceptance Criteria**:

  **If TDD:**
  - [ ] Test file exists with tests for weight ordering and completeness
  - [ ] `cargo test -p igrep-core -- frequency_table` → PASS

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Weight function returns sane ordering
    Tool: Bash (cargo test)
    Preconditions: frequency_table module implemented
    Steps:
      1. Run `cargo test -p igrep-core -- frequency_table`
      2. Assert all tests pass
      3. Verify test output includes assertions that common bigrams < rare bigrams
    Expected Result: All ordering tests pass
    Failure Indicators: Test failure showing common pair has higher weight than rare pair
    Evidence: .sisyphus/evidence/task-2-frequency-tests.txt

  Scenario: All 65536 entries are populated
    Tool: Bash (cargo test)
    Preconditions: Frequency table generated
    Steps:
      1. Run specific test that iterates all 65536 entries
      2. Assert no entry is u32::MAX or 0 where unexpected
    Expected Result: Table is fully populated
    Failure Indicators: Any assertion failure about missing entries
    Evidence: .sisyphus/evidence/task-2-table-complete.txt
  ```

  **Commit**: YES
  - Message: `feat(core): add embedded bigram frequency table for sparse n-gram weights`
  - Files: `crates/igrep-core/src/frequency_table.rs, crates/igrep-core/data/bigram_freq.bin`
  - Pre-commit: `cargo test -p igrep-core`

- [x] 3. Core types module (DocId, NgramHash, Query, PostingList)

  **What to do**:
  - Create `crates/igrep-core/src/types.rs` with foundational types used across all modules:
    - `pub type DocId = u32;` — file identifier (up to 4B files)
    - `pub type NgramHash = u32;` — CRC32 hash of an n-gram
    - `pub struct PostingList(pub Vec<DocId>);` — sorted list of doc IDs. Implement `intersect(&self, other: &PostingList) -> PostingList` and `union(&self, other: &PostingList) -> PostingList` as stubs (implemented in Task 9)
    - `pub enum QueryOp { All, None, And, Or }` — query tree operations
    - `pub struct Query { pub op: QueryOp, pub trigrams: Vec<NgramHash>, pub children: Vec<Query> }` — AND/OR tree of required n-grams
    - `pub struct IndexConfig { pub max_ngram_length: usize, pub max_file_size: u64, pub skip_binary: bool }` with `Default` impl (max_ngram_length=16, max_file_size=50MB, skip_binary=true)
    - `pub struct SearchResult { pub file_path: PathBuf, pub line_number: usize, pub line_content: String }`
    - `pub struct IndexMeta { pub version: u32, pub commit_sha: Option<String>, pub file_count: u32, pub ngram_count: u32 }`
  - Add `pub mod types;` to lib.rs
  - Write unit tests for Default impls and basic type construction

  **Must NOT do**:
  - Do not implement posting list operations (Task 8/9)
  - Do not implement Query evaluation (Task 10/14)

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Type definitions with derives, no complex logic
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 2, 4)
  - **Parallel Group**: Wave 1 (with Tasks 2, 4)
  - **Blocks**: Tasks 5, 6, 7, 8, 9, 10, 11, 12, 14
  - **Blocked By**: Task 1

  **References**:

  **Pattern References**:
  - `google/codesearch/index/read.go` — `type postEntry uint64` encoding trigram (24 bits) + fileID (40 bits)
  - `google/codesearch/index/regexp.go:15-35` — `Query` struct with `Op`, `Trigram` fields, `QAll/QNone/QAnd/QOr` operations

  **External References**:
  - Russ Cox blog post: Query tree structure section — AND/OR tree of trigram sets

  **WHY Each Reference Matters**:
  - codesearch postEntry: Shows the compact encoding pattern — we use separate types but same concept
  - codesearch Query: Direct model for our QueryOp/Query types — same algebra

  **Acceptance Criteria**:

  **If TDD:**
  - [ ] Tests for Default impl values
  - [ ] `cargo test -p igrep-core -- types` → PASS

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Types compile and have correct defaults
    Tool: Bash (cargo test)
    Preconditions: types module implemented
    Steps:
      1. Run `cargo test -p igrep-core -- types`
      2. Assert IndexConfig::default().max_ngram_length == 16
      3. Assert IndexConfig::default().max_file_size == 50 * 1024 * 1024
    Expected Result: All type tests pass with correct default values
    Failure Indicators: Compilation error or wrong default values
    Evidence: .sisyphus/evidence/task-3-types-tests.txt
  ```

  **Commit**: YES
  - Message: `feat(core): add core types module (DocId, NgramHash, Query, PostingList)`
  - Files: `crates/igrep-core/src/types.rs, crates/igrep-core/src/lib.rs`
  - Pre-commit: `cargo test -p igrep-core`

- [x] 4. Test fixtures directory with known-answer corpus

  **What to do**:
  - Create `test-fixtures/` at project root with a small, deterministic test corpus of ~20-30 files
  - Files should cover diverse scenarios:
    - `test-fixtures/simple.rs`: Contains `fn main() { println!("hello world"); }`, `TODO: fix this`, `use std::io;`
    - `test-fixtures/patterns.rs`: Contains `struct MyStruct`, `#[derive(Debug)]`, `pub fn example()`, `impl Display for`
    - `test-fixtures/empty.rs`: Empty file (0 bytes)
    - `test-fixtures/binary.bin`: A file with null bytes (binary detection test)
    - `test-fixtures/large_line.js`: A minified JS file with one very long line (>2000 chars)
    - `test-fixtures/unicode.py`: File with UTF-8 characters, Python code with `# -*- coding: utf-8 -*-`
    - `test-fixtures/nested/deep/module.rs`: Nested directory structure test
    - `test-fixtures/.hidden_file.txt`: Hidden file
    - `test-fixtures/no_extension`: File without extension
    - `test-fixtures/case_test.rs`: Contains both `FooBar` and `foobar` for case sensitivity testing
    - Additional 10-15 Rust/Python/JS/Go files with known patterns planted at known locations
  - Create `test-fixtures/EXPECTED.md`: Document what patterns exist in which files (ground truth for integration tests)
  - Create `.gitignore` entry in test-fixtures to exclude `*.bin` if desired, or include it intentionally for binary detection tests

  **Must NOT do**:
  - Do not create excessively large files (keep total corpus under 1MB)
  - Do not include real-world code with licensing issues — write original test content

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: File creation with deterministic content, no logic
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 2, 3)
  - **Parallel Group**: Wave 1 (with Tasks 2, 3)
  - **Blocks**: Tasks 14, 21
  - **Blocked By**: Task 1

  **References**:

  **Pattern References**:
  - google/codesearch `index/write.go:22-36` — Skip criteria: NUL bytes, invalid UTF-8, long lines (>2000 chars), >20K distinct trigrams

  **WHY Each Reference Matters**:
  - codesearch skip criteria: Defines what kinds of edge-case files we need in our test fixtures (binary, long lines, etc.)

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Test fixtures exist with correct content
    Tool: Bash
    Preconditions: test-fixtures directory created
    Steps:
      1. Run `find test-fixtures -type f | wc -l`
      2. Assert count >= 20
      3. Run `grep -r "fn main" test-fixtures/` — assert matches simple.rs
      4. Run `grep -r "TODO" test-fixtures/` — assert matches expected files
      5. Run `test -s test-fixtures/empty.rs` — assert file exists but is empty
      6. Run `xxd test-fixtures/binary.bin | head -1` — assert contains null bytes
    Expected Result: All fixture files exist with expected content
    Failure Indicators: Missing files, wrong content, wrong file count
    Evidence: .sisyphus/evidence/task-4-fixtures-exist.txt
  ```

  **Commit**: YES
  - Message: `test: add test fixtures directory with known-answer corpus`
  - Files: `test-fixtures/**/*`
  - Pre-commit: none (no code to test)

- [x] 5. Sparse n-gram engine: build_all (monotone stack algorithm)

  **What to do**:
  - Create `crates/igrep-core/src/ngram.rs` with the `build_all` function
  - Implement the monotone-stack algorithm that extracts ALL sparse n-grams from an input byte slice:
    ```
    pub fn build_all(input: &[u8], weight_fn: &dyn Fn(u8, u8) -> u32, consumer: &mut dyn FnMut(&[u8]))
    ```
  - Algorithm (from danlark1/sparse_ngrams reference):
    1. Iterate through each bigram position i (0 to len-2)
    2. Compute weight = weight_fn(input[i], input[i+1])
    3. Maintain a monotone stack of (weight, position) where weights are non-increasing
    4. When new weight > stack top: pop, emit n-gram spanning from popped position to i+2. Handle equal-weight gluing.
    5. If stack non-empty after popping, emit n-gram from stack top to i+2
    6. Push new (weight, position)
  - Also implement a convenience function: `pub fn build_all_ngrams(input: &[u8], config: &IndexConfig) -> Vec<Vec<u8>>` that uses the frequency table
  - Implement `pub fn hash_ngram(ngram: &[u8]) -> NgramHash` using CRC32 via `crc32fast`
  - Write RED tests FIRST:
    - Test with "chester " (from reference impl) → verify exact output matches C++ reference: `["che", "hes", "ches", "est", "chest", "ste", "ter", "ster", "er "]` (order may vary based on weight function, but set should match)
    - Test with empty string → no n-grams emitted
    - Test with 1-char string → no n-grams emitted
    - Test with 2-char string → exactly 1 n-gram (the bigram itself)
    - Test with "abcdef" → verify all outputs are at least 2 bytes long
    - Property test: output count ≤ 2*(input.len() - 2) for any input of length ≥ 2
    - Property test: every emitted n-gram is a substring of the input

  **Must NOT do**:
  - Do not implement build_covering here (Task 6)
  - Do not use CRC32 as the weight function — use the frequency table from Task 2

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Core algorithm with subtle stack manipulation, correctness is critical. Property-based testing required.
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 6, 7, 8, 9, 10)
  - **Parallel Group**: Wave 2
  - **Blocks**: Tasks 6, 11, 14
  - **Blocked By**: Tasks 2, 3

  **References**:

  **Pattern References**:
  - `danlark1/sparse_ngrams/src/sparse_ngrams.cc:29-58` — The exact C++ build_all implementation (monotone stack). Port this to Rust.
  - `danlark1/sparse_ngrams/testing/sparse_ngrams_test.cc:57-65` — Test vectors: "chester" produces specific n-grams

  **External References**:
  - Cursor blog "Sparse N-grams" section: Interactive visualization of the build_all algorithm step-by-step
  - `crc32fast` crate docs: For hash_ngram implementation

  **WHY Each Reference Matters**:
  - danlark1 C++ code: Direct algorithm reference — the Rust port should produce identical outputs for identical weight functions
  - danlark1 tests: Known-answer test vectors to verify correctness
  - Blog visualization: Explains the step-by-step stack operation if the code is unclear

  **Acceptance Criteria**:

  **If TDD:**
  - [ ] Test file with ≥5 unit tests + 2 property tests written BEFORE implementation
  - [ ] `cargo test -p igrep-core -- ngram::tests` → PASS (all tests green)

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: build_all produces correct n-grams for known input
    Tool: Bash (cargo test)
    Preconditions: ngram module with build_all implemented
    Steps:
      1. Run `cargo test -p igrep-core -- ngram::tests::test_build_all`
      2. Assert all tests pass including the "chester" reference vector
    Expected Result: All unit tests pass, output matches reference implementation
    Failure Indicators: Wrong n-gram set, missing n-grams, extra n-grams
    Evidence: .sisyphus/evidence/task-5-build-all-tests.txt

  Scenario: Property tests hold for random inputs
    Tool: Bash (cargo test)
    Preconditions: proptest property tests written
    Steps:
      1. Run `cargo test -p igrep-core -- ngram::tests::prop`
      2. Assert property tests pass (output count bound, substring property)
    Expected Result: No property test failures after 256+ random cases
    Failure Indicators: proptest shrinking output showing counterexample
    Evidence: .sisyphus/evidence/task-5-build-all-proptest.txt
  ```

  **Commit**: YES
  - Message: `feat(core): implement build_all sparse n-gram extraction (monotone stack)`
  - Files: `crates/igrep-core/src/ngram.rs, crates/igrep-core/src/lib.rs`
  - Pre-commit: `cargo test -p igrep-core`

- [x] 6. Sparse n-gram engine: build_covering (deque algorithm)

  **What to do**:
  - Add `build_covering` to `crates/igrep-core/src/ngram.rs`:
    ```
    pub fn build_covering(input: &[u8], weight_fn: &dyn Fn(u8, u8) -> u32, max_ngram_length: usize, consumer: &mut dyn FnMut(&[u8]))
    ```
  - Implement the deque-based algorithm (from danlark1/sparse_ngrams reference):
    1. Use a VecDeque of (weight, position) instead of a Vec
    2. Max length guard: if span from front to current exceeds max_ngram_length, emit from front and pop_front
    3. Same monotone property as build_all but with front-popping for length control
    4. Drain remaining stack at end, emitting spanning n-grams
  - Also implement: `pub fn build_covering_ngrams(input: &[u8], config: &IndexConfig) -> Vec<Vec<u8>>` convenience wrapper
  - Write RED tests FIRST:
    - "chester " → verify covering set is subset and covers the input
    - Reference test: "chester" → 2 covering n-grams (from C++ tests)
    - "for(int i=42" → 3 covering n-grams (from C++ tests)
    - Property test: covering count ≤ input.len() - 2
    - Property test: every covering n-gram is present in the build_all output (for same input and weight function)
    - Property test: covering n-grams tile the input (concatenation of covering n-grams reconstructs substrings that fully cover input)

  **Must NOT do**:
  - Do not modify build_all — only add build_covering

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Algorithm with deque + monotone property + max length cap. Property testing against build_all.
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 7, 8, 9, 10 — but depends on Task 5 for shared module)
  - **Parallel Group**: Wave 2 (after Task 5 lands)
  - **Blocks**: Tasks 10, 11, 14
  - **Blocked By**: Tasks 2, 3, 5

  **References**:

  **Pattern References**:
  - `danlark1/sparse_ngrams/src/sparse_ngrams.cc:60-100` — The exact C++ build_covering implementation (deque-based)
  - `danlark1/sparse_ngrams/testing/sparse_ngrams_test.cc:57-65` — Covering test vectors: "chester" → 2 n-grams, "for(int i=42" → 3 n-grams

  **External References**:
  - Cursor blog "Sparse N-grams" section (build_covering visualization): Shows how covering produces fewer lookups

  **WHY Each Reference Matters**:
  - danlark1 C++ covering code: Direct algorithm port target
  - danlark1 tests: Known-answer vectors that prove covering ⊆ all

  **Acceptance Criteria**:

  **If TDD:**
  - [ ] Test file with ≥4 unit tests + 3 property tests written BEFORE implementation
  - [ ] `cargo test -p igrep-core -- ngram::tests::test_build_covering` → PASS

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: build_covering produces minimal covering set
    Tool: Bash (cargo test)
    Preconditions: build_covering implemented
    Steps:
      1. Run `cargo test -p igrep-core -- ngram::tests::test_build_covering`
      2. Assert "chester" produces exactly 2 covering n-grams
      3. Assert covering set count < build_all set count for same input
    Expected Result: Covering set is smaller than build_all output
    Failure Indicators: Covering set same size or larger than build_all
    Evidence: .sisyphus/evidence/task-6-build-covering-tests.txt

  Scenario: Covering n-grams are subset of build_all n-grams
    Tool: Bash (cargo test)
    Preconditions: Both build_all and build_covering implemented
    Steps:
      1. Run `cargo test -p igrep-core -- ngram::tests::prop_covering_subset`
      2. Assert property holds for 256+ random inputs
    Expected Result: Every covering n-gram exists in the build_all output
    Failure Indicators: Counterexample where a covering n-gram is NOT in build_all output
    Evidence: .sisyphus/evidence/task-6-covering-subset-proptest.txt
  ```

  **Commit**: YES
  - Message: `feat(core): implement build_covering sparse n-gram extraction (deque)`
  - Files: `crates/igrep-core/src/ngram.rs`
  - Pre-commit: `cargo test -p igrep-core`

- [x] 7. Classic trigram extraction and probabilistic masks

  **What to do**:
  - Create `crates/igrep-core/src/trigram.rs` with classic trigram extraction and bloom filter masks
  - Implement:
    - `pub fn extract_trigrams(input: &[u8]) -> Vec<[u8; 3]>` — slide 3-byte window, collect all trigrams
    - `pub fn trigram_to_hash(trigram: &[u8; 3]) -> NgramHash` — CRC32 hash of 3 bytes
    - `pub struct TrigramWithMasks { pub trigram: [u8; 3], pub next_mask: u8, pub loc_mask: u8 }`
    - `pub fn extract_trigrams_with_masks(input: &[u8]) -> Vec<TrigramWithMasks>` — for each trigram at position p: `loc_mask |= 1 << (p % 8)`, `next_mask |= 1 << (hash(input[p+3]) % 8)` (if p+3 < len)
    - `pub fn check_adjacency(mask_a: u8, mask_b: u8) -> bool` — `(mask_a << 1) & mask_b != 0` (shifted AND check)
    - `pub fn check_next_char(next_mask: u8, expected_char: u8) -> bool` — `next_mask & (1 << (hash(expected_char) % 8)) != 0`
  - Write RED tests FIRST:
    - "the cat sat" → extract trigrams → verify count and contents
    - locMask + adjacency check: trigrams "the" at pos 0 and "he " at pos 1 → adjacency check returns true
    - nextMask check: "the" followed by " " → check_next_char with ' ' returns true
    - Bloom filter false positive: demonstrate that masks are probabilistic (show a case where check returns true but actual char doesn't match)
    - Empty/short input: strings < 3 chars produce no trigrams

  **Must NOT do**:
  - Do not build an inverted index here — just extraction and mask operations
  - Do not implement the full Blackbird approach — just the building blocks

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Multiple related functions with bit manipulation, needs careful testing of probabilistic properties
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 5, 6, 8, 9, 10)
  - **Parallel Group**: Wave 2
  - **Blocks**: Tasks 11, 14, 16
  - **Blocked By**: Tasks 2, 3

  **References**:

  **Pattern References**:
  - Cursor blog "Trigram Queries with Probabilistic Masks" section — nextMask and locMask visualization with worked examples
  - `google/codesearch/index/write.go` — Trigram extraction via sliding 3-byte window

  **External References**:
  - Cursor blog "Putting it all together" section — Interactive trigram index demo

  **WHY Each Reference Matters**:
  - Blog mask section: Exact specification of how nextMask/locMask work, with shift-AND adjacency check
  - codesearch write.go: Shows the standard trigram extraction pattern in production code

  **Acceptance Criteria**:

  **If TDD:**
  - [ ] Tests written BEFORE implementation for each function
  - [ ] `cargo test -p igrep-core -- trigram` → PASS

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Trigram extraction produces correct count
    Tool: Bash (cargo test)
    Preconditions: trigram module implemented
    Steps:
      1. Run `cargo test -p igrep-core -- trigram::tests`
      2. Assert "the cat sat" produces exactly 9 trigrams (11 chars - 2)
      3. Assert adjacency check passes for consecutive trigrams
    Expected Result: All trigram tests pass
    Failure Indicators: Wrong trigram count, failed adjacency check
    Evidence: .sisyphus/evidence/task-7-trigram-tests.txt

  Scenario: Probabilistic masks provide filtering power
    Tool: Bash (cargo test)
    Preconditions: Mask functions implemented
    Steps:
      1. Run mask-specific tests
      2. Verify nextMask correctly encodes following character
      3. Verify locMask shift-AND correctly identifies adjacent trigrams
    Expected Result: Masks correctly filter in valid and filter out some invalid candidates
    Failure Indicators: False negatives (mask rejects a true positive)
    Evidence: .sisyphus/evidence/task-7-mask-tests.txt
  ```

  **Commit**: YES
  - Message: `feat(core): implement classic trigram extraction and probabilistic masks`
  - Files: `crates/igrep-core/src/trigram.rs, crates/igrep-core/src/lib.rs`
  - Pre-commit: `cargo test -p igrep-core`

- [x] 8. Varint encoding/decoding and delta-encoded posting lists

  **What to do**:
  - Create `crates/igrep-core/src/posting.rs` with posting list encoding/decoding
  - Implement varint encoding (LEB128-style, 7 bits per byte, MSB as continuation bit):
    - `pub fn encode_varint(value: u32, buf: &mut Vec<u8>)` — encode single u32
    - `pub fn decode_varint(data: &[u8], pos: &mut usize) -> Option<u32>` — decode single u32, advance pos
  - Implement delta-encoded posting list:
    - `pub fn encode_posting_list(doc_ids: &[DocId], buf: &mut Vec<u8>)` — sort doc_ids, delta-encode, varint each delta. Prepend count as varint.
    - `pub fn decode_posting_list(data: &[u8], pos: &mut usize) -> Vec<DocId>` — read count, decode varint deltas, reconstruct absolute doc IDs
  - Write RED tests FIRST:
    - Varint roundtrip: encode(0) → [0x00], encode(127) → [0x7F], encode(128) → [0x80, 0x01], encode(u32::MAX) → 5 bytes
    - Delta encoding: [7, 18, 19, 22, 25, 63] → deltas [7, 11, 1, 3, 3, 38] (from Zobel & Moffat)
    - Roundtrip property: for any sorted Vec<DocId>, encode → decode produces identical vec
    - Empty list: encode/decode of empty list works
    - Single element: encode/decode of [42] works
    - Property test: decoded list is always sorted and matches input

  **Must NOT do**:
  - Do not implement Elias γ-coding (varint is simpler and sufficient for local indexes)
  - Do not implement roaring bitmaps (overkill for single-machine use)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Binary encoding with bit manipulation, correctness-critical for index integrity
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 5, 6, 7, 9, 10)
  - **Parallel Group**: Wave 2
  - **Blocks**: Tasks 9, 11, 12
  - **Blocked By**: Task 3

  **References**:

  **Pattern References**:
  - `google/codesearch/index/read.go:31-45` — γ-coded posting list decoding (we use varint instead, but same concept)
  - Zobel & Moffat 2006 §4: "the sorted list 7, 18, 19, 22, 23, 25, 63... can be represented by gaps 7, 11, 1, 3, 1, 2, 38..."

  **External References**:
  - LEB128 encoding: https://en.wikipedia.org/wiki/LEB128 — the standard varint format

  **WHY Each Reference Matters**:
  - codesearch read.go: Shows production posting list decode pattern (our varint replaces their γ-code)
  - Zobel paper: Provides the theoretical foundation and example delta sequences for testing

  **Acceptance Criteria**:

  **If TDD:**
  - [ ] Tests for varint encode/decode roundtrip, delta encode/decode roundtrip
  - [ ] `cargo test -p igrep-core -- posting::tests` → PASS

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Varint roundtrip for edge cases
    Tool: Bash (cargo test)
    Preconditions: varint functions implemented
    Steps:
      1. Run `cargo test -p igrep-core -- posting::tests::test_varint`
      2. Verify 0, 1, 127, 128, 16383, 16384, u32::MAX all roundtrip correctly
    Expected Result: All edge case values encode→decode to original
    Failure Indicators: Decoded value doesn't match encoded value
    Evidence: .sisyphus/evidence/task-8-varint-roundtrip.txt

  Scenario: Posting list roundtrip preserves sorted order
    Tool: Bash (cargo test)
    Preconditions: posting list encode/decode implemented
    Steps:
      1. Run `cargo test -p igrep-core -- posting::tests::prop_roundtrip`
      2. Property: for any sorted Vec<DocId>, encode→decode == original
    Expected Result: 256+ random cases all pass
    Failure Indicators: Counterexample showing data loss or reordering
    Evidence: .sisyphus/evidence/task-8-posting-roundtrip.txt
  ```

  **Commit**: YES
  - Message: `feat(core): implement varint encoding and delta-encoded posting lists`
  - Files: `crates/igrep-core/src/posting.rs, crates/igrep-core/src/lib.rs`
  - Pre-commit: `cargo test -p igrep-core`

- [x] 9. Posting list intersection (AND) and union (OR)

  **What to do**:
  - Add to `crates/igrep-core/src/posting.rs`:
    - `pub fn intersect(a: &[DocId], b: &[DocId]) -> Vec<DocId>` — linear merge-intersection of two sorted lists. O(n+m). Both inputs must be sorted; output is sorted.
    - `pub fn union(a: &[DocId], b: &[DocId]) -> Vec<DocId>` — linear merge-union of two sorted lists. O(n+m).
    - `pub fn intersect_many(lists: &[&[DocId]]) -> Vec<DocId>` — intersect multiple lists. Start with shortest list for early termination (if any list is empty, result is empty).
    - `pub fn union_many(lists: &[&[DocId]]) -> Vec<DocId>` — union multiple lists.
    - `pub fn subtract(a: &[DocId], b: &[DocId]) -> Vec<DocId>` — remove all elements of b from a. Needed for git overlay (exclude deleted files from base index results).
  - Write RED tests FIRST:
    - intersect([1,3,5,7], [2,3,6,7]) → [3,7]
    - union([1,3,5], [2,3,6]) → [1,2,3,5,6]
    - intersect with empty list → empty
    - union with empty list → other list
    - subtract([1,2,3,4,5], [2,4]) → [1,3,5]
    - Property test: intersect(a,b) ⊆ a and intersect(a,b) ⊆ b
    - Property test: a ⊆ union(a,b) and b ⊆ union(a,b)
    - Property test: output of all operations is sorted

  **Must NOT do**:
  - Do not optimize with SIMD or galloping search — simple linear merge is sufficient for v1

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Well-known algorithms but need thorough edge case testing
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 5, 6, 7, 10)
  - **Parallel Group**: Wave 2
  - **Blocks**: Tasks 11, 12, 14
  - **Blocked By**: Tasks 3, 8

  **References**:

  **Pattern References**:
  - `google/codesearch/index/read.go:561-615` — `postingQuery` using `postingAnd` (intersect) and `postingOr` (union) with early termination

  **WHY Each Reference Matters**:
  - codesearch postingQuery: Shows the exact AND/OR evaluation with early termination on empty intersection — production-proven pattern

  **Acceptance Criteria**:

  **If TDD:**
  - [ ] Tests for intersect, union, subtract with known answers + property tests
  - [ ] `cargo test -p igrep-core -- posting::tests` → PASS (including new tests)

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Set operations produce correct results
    Tool: Bash (cargo test)
    Preconditions: intersect/union/subtract implemented
    Steps:
      1. Run `cargo test -p igrep-core -- posting::tests::test_set_ops`
      2. Verify known-answer tests for intersect, union, subtract
    Expected Result: All set operation tests pass
    Failure Indicators: Wrong elements in output, unsorted output
    Evidence: .sisyphus/evidence/task-9-set-ops-tests.txt

  Scenario: Empty list edge cases handled
    Tool: Bash (cargo test)
    Preconditions: Set operations implemented
    Steps:
      1. Run tests with empty inputs
      2. intersect([], anything) == []
      3. union([], x) == x
      4. subtract(x, []) == x
    Expected Result: All empty-list edge cases pass
    Failure Indicators: Panic on empty input, wrong result
    Evidence: .sisyphus/evidence/task-9-empty-edge-cases.txt
  ```

  **Commit**: YES
  - Message: `feat(core): implement posting list intersection (AND) and union (OR)`
  - Files: `crates/igrep-core/src/posting.rs`
  - Pre-commit: `cargo test -p igrep-core`

- [x] 10. Regex → query tree decomposition (Cox 5-property algorithm)

  **What to do**:
  - Create `crates/igrep-core/src/query.rs` with the regex-to-query-tree decomposition
  - Parse regex using `regex_syntax::parse()` to get an `Hir` (High-level Intermediate Representation)
  - Implement the Russ Cox 5-property analysis by walking the `Hir` tree:
    - `struct RegexInfo { can_empty: bool, exact: Option<Vec<Vec<u8>>>, prefix: Vec<Vec<u8>>, suffix: Vec<Vec<u8>>, query: Query }`
    - `pub fn regex_to_query(pattern: &str) -> Result<Query, Error>` — main entry point
    - `fn analyze(hir: &Hir) -> RegexInfo` — recursive walker:
      - `HirKind::Literal(bytes)` → exact = {bytes}, extract n-grams from it
      - `HirKind::Concat(parts)` → cross-product of children, extract boundary n-grams from suffix×prefix
      - `HirKind::Alternation(alts)` → union of children, query = OR of child queries
      - `HirKind::Repetition { min: 0, .. }` → can_empty = true, query = ALL
      - `HirKind::Repetition { min: n, max: n, .. }` → repeat child analysis n times
      - `HirKind::Repetition { min: n, .. }` → keep prefix/suffix from child, query from child
      - `HirKind::Class(small)` → enumerate codepoints as alternations (limit: ≤10 codepoints)
      - `HirKind::Class(large)` → return ALL (too many alternatives)
      - `HirKind::Look(_)` → anchors don't produce n-grams, return empty exact
    - Information-saving transforms: before discarding exact/prefix/suffix, extract n-grams and AND them into the query
    - Information-discarding transforms: cap exact set at 7, prefix/suffix set at 20, string lengths at max_ngram_length
  - Use `build_covering` (from Task 6) to extract sparse n-grams from the required literals, not just trigrams
  - Write RED tests FIRST:
    - `"foo"` → query requires n-grams from "foo"
    - `"foo|bar"` → query is OR(n-grams("foo"), n-grams("bar"))
    - `"foo.*bar"` → query is AND(n-grams("foo"), n-grams("bar"))
    - `"[abc]def"` → query is OR(n-grams("adef"), n-grams("bdef"), n-grams("cdef"))
    - `".*"` → query is ALL
    - `"fo+"` → query requires n-grams from "fo" (prefix preserved)
    - `"MAX_FILE_SIZE"` → query requires specific n-grams (long literal → many required n-grams)
    - Case from Russ Cox: `"DATAKIT"` → query requires "AKI" AND "ATA" AND "DAT" AND "KIT" AND "TAK"

  **Must NOT do**:
  - Do not implement query evaluation against the index (Task 14)
  - Do not support PCRE features (lookahead, backreferences) — RE2-compatible only

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Most complex algorithm in the project — recursive tree walk with 5 properties, cross-products, information transforms. Must match Russ Cox's proven algebra.
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 7, 8, 9 — but depends on Task 6 for build_covering)
  - **Parallel Group**: Wave 2 (after Task 6)
  - **Blocks**: Task 14
  - **Blocked By**: Tasks 3, 6

  **References**:

  **Pattern References**:
  - `google/codesearch/index/regexp.go:332-631` — Full Go implementation of analyze(), the 5-property regex → query tree algorithm. Direct port target.
  - `google/codesearch/index/regexp.go:632-711` — Information-saving/discarding transforms

  **External References**:
  - Russ Cox blog (https://swtch.com/~rsc/regexp/regexp4.html) — Complete formal specification of the 5 properties and their rules for each regex operation. THE authoritative reference.
  - `regex-syntax` crate `hir::HirKind` enum — Rust AST types that correspond to Go's `regexp/syntax` operations
  - `BurntSushi/ripgrep/crates/regex/src/literal.rs:125-437` — ripgrep's literal extraction from HIR (simpler approach, good for comparison)

  **WHY Each Reference Matters**:
  - codesearch regexp.go: The production implementation of the Cox algorithm — line-by-line port reference
  - Cox blog: Formal specification of the algebra — when in doubt about correctness, this is the source of truth
  - regex-syntax HirKind: The actual Rust types being walked — maps to Go's `syntax.Op` variants
  - ripgrep literal.rs: Shows a simpler literal-only extraction approach — useful if full Cox analysis is too complex for a first pass

  **Acceptance Criteria**:

  **If TDD:**
  - [ ] Tests for each regex pattern type (literal, alternation, concat, repetition, class)
  - [ ] `cargo test -p igrep-core -- query::tests` → PASS

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Literal patterns produce correct query
    Tool: Bash (cargo test)
    Preconditions: regex_to_query implemented
    Steps:
      1. Run `cargo test -p igrep-core -- query::tests::test_literals`
      2. "foo" produces AND query with n-grams from "foo"
      3. "MAX_FILE_SIZE" produces AND query with multiple required n-grams
    Expected Result: Literal patterns decompose into AND of required n-grams
    Failure Indicators: Query is ALL when it should have required n-grams
    Evidence: .sisyphus/evidence/task-10-literal-queries.txt

  Scenario: Complex patterns decompose correctly
    Tool: Bash (cargo test)
    Preconditions: regex_to_query supports all HirKind variants
    Steps:
      1. Run `cargo test -p igrep-core -- query::tests::test_complex`
      2. "foo|bar" → OR query
      3. "foo.*bar" → AND query with n-grams from both "foo" and "bar"
      4. ".*" → ALL query
      5. "[abc]def" → OR of 3 AND queries
    Expected Result: All complex patterns produce correct query tree structure
    Failure Indicators: Wrong query operation (AND vs OR), missing n-grams
    Evidence: .sisyphus/evidence/task-10-complex-queries.txt
  ```

  **Commit**: YES
  - Message: `feat(core): implement regex to query tree decomposition (Cox algorithm)`
  - Files: `crates/igrep-core/src/query.rs, crates/igrep-core/src/lib.rs`
  - Pre-commit: `cargo test -p igrep-core`

- [x] 11. Index writer (2-pass construction: extract → sort → write)

  **What to do**:
  - Create `crates/igrep-core/src/index/mod.rs` and `crates/igrep-core/src/index/writer.rs`
  - Implement the 2-pass index construction algorithm:
    - **Pass 1** — Extraction: For each file, extract all sparse n-grams (via `build_all`), hash each n-gram (CRC32), and produce `(NgramHash, DocId)` pairs. Write pairs to sharded temp files (256 buckets, shard by top 8 bits of hash). Also extract classic trigrams with masks for Phase 2 enrichment.
    - **Pass 2** — Sort & Write: For each shard bucket, sort by (NgramHash, DocId). Merge sorted shards. Write:
      1. Postings file: for each unique NgramHash, write a delta-encoded varint posting list of DocIds
      2. Lookup table: sorted array of `(NgramHash: u32, Offset: u64)` pairs pointing into postings file
      3. Header: magic bytes `b"IGREP\x00\x01\x00"` (8 bytes), then file count (u32), ngram count (u32)
      4. Trailer: checksum of the file, magic bytes for end-of-file detection
  - Use `tempfile` crate for temp shard files
  - Write index files atomically: write to `index.postings.tmp` and `index.lookup.tmp`, then rename to `index.postings` and `index.lookup`
  - Memory bound: configurable max in-memory buffer size (default 64MB like codesearch). Flush to temp when exceeded.
  - API: `pub struct IndexWriter { config: IndexConfig }` with methods:
    - `pub fn new(config: IndexConfig) -> Self`
    - `pub fn add_file(&mut self, doc_id: DocId, content: &[u8]) -> Result<()>` — extract n-grams, buffer pairs
    - `pub fn finish(self, output_dir: &Path) -> Result<IndexMeta>` — flush, sort, write files
  - Also maintain a file list: `index.files` — newline-separated list of (DocId, file_path) mappings
  - Write tests:
    - Add 3 small files, finish, verify output files exist and have correct header magic
    - Roundtrip with reader (deferred to after Task 12 — for now, verify file format manually)

  **Must NOT do**:
  - Do not implement the reader — that's Task 12
  - Do not implement parallel extraction (single-threaded writer is fine for v1 correctness; Task 13's walker handles parallelism)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Complex multi-pass algorithm with temp file management, binary format writing, atomic renames
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 12, 13, 15, 16 in Wave 3)
  - **Parallel Group**: Wave 3
  - **Blocks**: Tasks 12, 14, 17
  - **Blocked By**: Tasks 5, 6, 7, 8, 9, 13

  **References**:

  **Pattern References**:
  - `google/codesearch/index/write.go:22-36` — 2-pass construction with in-memory buffer + temp file flush
  - `google/codesearch/index/write.go:100-124` — postEntry encoding (trigram << 40 | fileID)
  - `google/codesearch/index/write.go:767-821` — 2-pass 12-bit radix sort

  **External References**:
  - Cursor blog "All this, in your machine" section: Two-file layout diagram (lookup table + postings file)

  **WHY Each Reference Matters**:
  - codesearch write.go: Production index construction algorithm — memory bounding, temp file strategy, sort approach
  - Blog layout diagram: Shows the exact two-file format with hash→offset lookup table and sequential posting lists

  **Acceptance Criteria**:

  **If TDD:**
  - [ ] Tests for add_file + finish producing valid output files
  - [ ] `cargo test -p igrep-core -- index::writer` → PASS

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Index writer produces valid output files
    Tool: Bash (cargo test)
    Preconditions: IndexWriter implemented
    Steps:
      1. Create writer, add 3 test files with known content
      2. Call finish() with a temp directory
      3. Assert index.postings file exists and starts with magic header
      4. Assert index.lookup file exists
      5. Assert index.files file exists and lists 3 files
    Expected Result: All three index files created with correct headers
    Failure Indicators: Missing files, wrong magic bytes, panic during write
    Evidence: .sisyphus/evidence/task-11-writer-output.txt

  Scenario: Atomic write prevents corruption
    Tool: Bash (cargo test)
    Preconditions: IndexWriter implemented
    Steps:
      1. Verify that during finish(), .tmp files are created first
      2. After finish(), .tmp files no longer exist (renamed to final names)
    Expected Result: Atomic rename strategy works
    Failure Indicators: .tmp files left behind, final files missing
    Evidence: .sisyphus/evidence/task-11-atomic-write.txt
  ```

  **Commit**: YES
  - Message: `feat(core): implement index writer with 2-pass construction`
  - Files: `crates/igrep-core/src/index/mod.rs, crates/igrep-core/src/index/writer.rs`
  - Pre-commit: `cargo test -p igrep-core`

- [x] 12. Index reader (mmap + binary search + posting decode)

  **What to do**:
  - Create `crates/igrep-core/src/index/reader.rs`
  - Implement the index reader that uses mmap'd lookup table and reads postings from disk:
    - `pub struct IndexReader` with:
      - `lookup_mmap: Mmap` — memory-mapped lookup table file
      - `postings_file: File` — postings file (read at offset via seek+read)
      - `meta: IndexMeta` — parsed header info
      - `file_list: Vec<PathBuf>` — loaded file list
    - `pub fn open(index_dir: &Path) -> Result<Self>` — validate magic header, mmap lookup table with `madvise(Random)`, open postings file
    - `pub fn lookup(&self, ngram_hash: NgramHash) -> Option<Vec<DocId>>` — binary search on mmap'd lookup table to find offset, then read posting list from postings file at that offset, delta-decode
    - `pub fn evaluate_query(&self, query: &Query) -> Vec<DocId>` — recursively evaluate AND/OR query tree:
      - `QueryOp::All` → return all doc IDs (0..file_count)
      - `QueryOp::None` → return empty
      - `QueryOp::And` → intersect posting lists for all trigrams, then recursively intersect with children
      - `QueryOp::Or` → union posting lists for all trigrams, then recursively union with children
    - `pub fn doc_id_to_path(&self, doc_id: DocId) -> Option<&Path>` — look up file path for a doc ID
  - Use `memmap2::Mmap` for the lookup table, with `advise(Advice::Random)` for random access pattern
  - Binary search implementation: lookup table is sorted `(u32, u64)` pairs = 12 bytes each. Binary search by hash.
  - Write tests:
    - Roundtrip: build index with writer, open with reader, lookup known n-grams → get correct doc IDs
    - evaluate_query with AND → intersection works
    - evaluate_query with OR → union works
    - evaluate_query with ALL → returns all docs
    - Binary search correctness: lookup for existing hash finds it, lookup for non-existing returns None

  **Must NOT do**:
  - Do not implement file content reading/matching — that's Task 14
  - Do not cache posting lists in memory — read from disk each time (mmap for lookup only)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: mmap handling, binary search over raw bytes, query tree evaluation
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 13, 15, 16 in Wave 3 — but needs Task 11 for index files)
  - **Parallel Group**: Wave 3 (after Task 11)
  - **Blocks**: Tasks 14, 18
  - **Blocked By**: Tasks 8, 9, 11

  **References**:

  **Pattern References**:
  - `google/codesearch/index/read.go:348-402` — `findListV2` binary search over posting blocks
  - `google/codesearch/index/read.go:561-615` — `postingQuery` evaluating AND/OR query tree against posting lists
  - Turbopack `static_sorted_file.rs` — mmap with per-region madvise hints

  **External References**:
  - `memmap2` crate: `Mmap::map()`, `advise(Advice::Random)`
  - Cursor blog "All this, in your machine" diagram: Shows mmap'd lookup table with binary search → offset → postings file read

  **WHY Each Reference Matters**:
  - codesearch findListV2: Production binary search over posting blocks — exact algorithm to port
  - codesearch postingQuery: Production query evaluation — AND/OR with early termination
  - Turbopack: Shows real Rust madvise usage pattern

  **Acceptance Criteria**:

  **If TDD:**
  - [ ] Roundtrip test: writer → reader → lookup → correct results
  - [ ] `cargo test -p igrep-core -- index::reader` → PASS

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Writer → Reader roundtrip
    Tool: Bash (cargo test)
    Preconditions: Both writer and reader implemented
    Steps:
      1. Build index from 3 files with known content ("hello world", "foo bar", "hello foo")
      2. Open index with reader
      3. Lookup n-grams from "hello" → get doc IDs for files 0 and 2
      4. Lookup n-grams from "foo" → get doc IDs for files 1 and 2
    Expected Result: Lookups return correct doc ID sets
    Failure Indicators: Wrong doc IDs, missing entries, decode errors
    Evidence: .sisyphus/evidence/task-12-roundtrip.txt

  Scenario: Query evaluation produces correct candidates
    Tool: Bash (cargo test)
    Preconditions: evaluate_query implemented
    Steps:
      1. Build AND query for n-grams from "hello foo"
      2. Evaluate → should return doc ID 2 (the only file containing both)
      3. Build OR query for n-grams from "hello" OR "bar"
      4. Evaluate → should return doc IDs 0, 1, 2
    Expected Result: AND produces intersection, OR produces union
    Failure Indicators: AND returns too many docs, OR returns too few
    Evidence: .sisyphus/evidence/task-12-query-eval.txt
  ```

  **Commit**: YES
  - Message: `feat(core): implement mmap'd index reader with binary search`
  - Files: `crates/igrep-core/src/index/reader.rs`
  - Pre-commit: `cargo test -p igrep-core`

- [x] 13. File walker with binary detection and .gitignore support

  **What to do**:
  - Create `crates/igrep-core/src/walker.rs`
  - Implement a file walker using the `ignore` crate for gitignore-aware, parallel directory traversal:
    - `pub struct FileWalker { config: IndexConfig }`
    - `pub fn walk(root: &Path, config: &IndexConfig) -> Result<Vec<(DocId, PathBuf)>>` — walk directory, assign doc IDs, return list of indexable files
    - File filtering:
      - Skip hidden files (configurable)
      - Respect .gitignore (via `ignore::WalkBuilder::git_ignore(true)`)
      - Skip binary files: read first 16KB, if contains null byte → skip
      - Skip files > config.max_file_size (default 50MB)
      - Skip files with very long lines (>10000 chars) as a heuristic for minified code
    - Sort results by path for deterministic doc ID assignment
  - Write tests:
    - Walk test-fixtures → returns expected file count (excludes binary.bin)
    - Binary detection: file with null bytes is skipped
    - .gitignore respect: create temp dir with .gitignore, verify ignored files are excluded
    - Hidden file handling: .hidden_file.txt is skipped by default
    - Empty directory: returns empty list

  **Must NOT do**:
  - Do not implement parallel file reading for index construction (walker only discovers files)
  - Do not follow symlinks by default

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Integration with `ignore` crate, file I/O, binary detection heuristic
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 11, 12, 15, 16)
  - **Parallel Group**: Wave 3
  - **Blocks**: Tasks 11, 14
  - **Blocked By**: Task 1

  **References**:

  **Pattern References**:
  - `BurntSushi/ripgrep/crates/ignore/src/lib.rs` — `WalkBuilder` API with `git_ignore`, `hidden`, `build_parallel`
  - `google/codesearch/index/write.go:22-36` — File skip criteria (NUL bytes, invalid UTF-8, long lines, many trigrams)

  **External References**:
  - `ignore` crate docs: WalkBuilder configuration options

  **WHY Each Reference Matters**:
  - ripgrep ignore crate: The exact API we're using — WalkBuilder configuration pattern
  - codesearch skip criteria: Defines which files to exclude — we match this for compatibility

  **Acceptance Criteria**:

  **If TDD:**
  - [ ] Tests using test-fixtures directory
  - [ ] `cargo test -p igrep-core -- walker::tests` → PASS

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Walker discovers correct files in test fixtures
    Tool: Bash (cargo test)
    Preconditions: walker implemented, test-fixtures exist
    Steps:
      1. Walk test-fixtures directory
      2. Assert binary.bin is NOT in results (binary detection)
      3. Assert empty.rs IS in results (empty files are indexable)
      4. Assert nested/deep/module.rs IS in results (recursive walk)
    Expected Result: Correct files discovered, binary files skipped
    Failure Indicators: Binary file included, valid file excluded
    Evidence: .sisyphus/evidence/task-13-walker-tests.txt
  ```

  **Commit**: YES
  - Message: `feat(core): implement parallel file walker with binary detection`
  - Files: `crates/igrep-core/src/walker.rs, crates/igrep-core/src/lib.rs`
  - Pre-commit: `cargo test -p igrep-core`

- [x] 14. End-to-end search pipeline (index → query → verify)

  **What to do**:
  - Create `crates/igrep-core/src/search.rs` with the full search pipeline:
    - `pub struct SearchEngine { reader: IndexReader, config: IndexConfig }`
    - `pub fn search(reader: &IndexReader, pattern: &str, root: &Path) -> Result<Vec<SearchResult>>`:
      1. Parse regex: `regex_to_query(pattern)` → Query tree
      2. Evaluate query: `reader.evaluate_query(&query)` → candidate DocIds
      3. If query is ALL: use all indexed files as candidates
      4. For each candidate file: load file content, run actual `regex::Regex::find()` on each line
      5. Collect matching lines into `SearchResult` entries
      6. Return results sorted by file path, then line number
    - Handle edge cases:
      - Regex that matches empty string → every file matches, but only lines where regex finds a match are returned
      - Candidate file that no longer exists (stale index) → skip with warning, don't error
      - Very large candidate files → stream line-by-line, don't load entire file
  - Also implement `pub fn search_files_only(reader: &IndexReader, pattern: &str) -> Result<Vec<PathBuf>>` — returns just file paths (for `-l` flag), can short-circuit after first match per file
  - Write integration tests using test-fixtures:
    - Search for "fn main" → returns matches in simple.rs
    - Search for "TODO" → returns matches in expected files
    - Search for "struct\s+\w+" → returns matches in patterns.rs
    - Search for "nonexistent_string_xyz" → returns empty
    - Search for ".*" → returns all non-empty files
    - Verify: for every pattern, igrep results are a superset of what `grep -rP` would find (no false negatives)

  **Must NOT do**:
  - Do not implement CLI output formatting (Task 18)
  - Do not implement case-insensitive search (Task 16)
  - Do not implement git overlay (Task 15)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Integration of all core components, critical correctness properties (zero false negatives)
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO (depends on almost everything in Wave 2 and Wave 3)
  - **Parallel Group**: Wave 3 (last task in wave)
  - **Blocks**: Tasks 15, 16, 17, 18, 21
  - **Blocked By**: Tasks 10, 11, 12, 4

  **References**:

  **Pattern References**:
  - `google/codesearch/cmd/csearch/csearch.go` — End-to-end search: parse regex, load index, get candidates, grep each file
  - All prior tasks: This task wires together Tasks 5-12

  **External References**:
  - `regex` crate: `Regex::find()` for line-level matching in verification step

  **WHY Each Reference Matters**:
  - codesearch csearch: Shows the complete search flow — index lookup → file load → regex match → output

  **Acceptance Criteria**:

  **If TDD:**
  - [ ] Integration tests with test-fixtures for 10+ patterns
  - [ ] `cargo test -p igrep-core -- search::tests` → PASS

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: End-to-end search returns correct results
    Tool: Bash (cargo test)
    Preconditions: Full pipeline (writer + reader + search) working
    Steps:
      1. Build index on test-fixtures
      2. Search for "fn main" → assert simple.rs is in results
      3. Search for "TODO" → assert expected files are in results
      4. Search for "nonexistent_xyz" → assert empty results
    Expected Result: All searches return correct, verified results
    Failure Indicators: Missing expected matches (false negative!), crash
    Evidence: .sisyphus/evidence/task-14-e2e-search.txt

  Scenario: Zero false negatives guarantee
    Tool: Bash (cargo test)
    Preconditions: test-fixtures indexed
    Steps:
      1. For each of 10 test patterns, run search
      2. For each pattern, also run grep -rP on test-fixtures
      3. Assert: every file in grep results is also in igrep results
    Expected Result: igrep ⊇ grep for all patterns (no false negatives)
    Failure Indicators: A file found by grep but not by igrep
    Evidence: .sisyphus/evidence/task-14-no-false-negatives.txt
  ```

  **Commit**: YES
  - Message: `feat(core): implement end-to-end search pipeline`
  - Files: `crates/igrep-core/src/search.rs, crates/igrep-core/src/lib.rs`
  - Pre-commit: `cargo test -p igrep-core`

- [x] 15. Git-based index versioning with dirty overlay

  **What to do**:
  - Create `crates/igrep-core/src/git_overlay.rs`
  - Implement git-based index versioning:
    - `pub struct GitOverlay { base_commit: String, modified_files: HashMap<PathBuf, Vec<u8>>, deleted_files: HashSet<PathBuf>, new_file_ngrams: HashMap<NgramHash, Vec<DocId>> }`
    - `pub fn detect_changes(index_dir: &Path, repo_root: &Path) -> Result<GitOverlay>`:
      1. Read stored commit SHA from `index.meta` file
      2. Run `git diff <stored-sha> HEAD --name-only` to find changed files since index was built
      3. Run `git ls-files --others --exclude-standard` to find new untracked files
      4. Run `git diff --name-only` (unstaged) and `git diff --cached --name-only` (staged) for dirty working tree
      5. For each changed/new file: read content, extract sparse n-grams, build in-memory posting lists
      6. For each deleted file: add to deleted set
    - `pub fn merge_results(base_results: Vec<DocId>, overlay: &GitOverlay, file_list: &[PathBuf]) -> Vec<DocId>`:
      1. Filter out doc IDs whose paths are in deleted_files
      2. Filter out doc IDs whose paths are in modified_files (stale entries)
      3. Add doc IDs from new_file_ngrams overlay
      4. Return merged, deduplicated, sorted list
    - Store commit SHA in `index.meta` file alongside the index
  - Write tests:
    - Create temp git repo, add files, commit, build index
    - Modify a file, detect_changes finds it
    - Add new file, detect_changes finds it
    - Delete a file, detect_changes identifies the deletion
    - merge_results correctly excludes deleted and includes new

  **Must NOT do**:
  - Do not handle merge commits, submodules, or worktrees
  - Do not persist the overlay to disk — it's in-memory only
  - Do not implement incremental index rebuilding

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Git command integration, HashMap-based overlay logic, temp repo test setup
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 16 in Wave 3)
  - **Parallel Group**: Wave 3 (after Task 14)
  - **Blocks**: Task 18
  - **Blocked By**: Task 14

  **References**:

  **Pattern References**:
  - Cursor blog "All this, in your machine" section: "We control the state of the index by basing it off a commit... User and agent changes are stored as a layer on top."

  **WHY Each Reference Matters**:
  - Blog description: Exact specification of the git overlay strategy — commit-based base with dirty layer

  **Acceptance Criteria**:

  **If TDD:**
  - [ ] Tests using temp git repo with commits and modifications
  - [ ] `cargo test -p igrep-core -- git_overlay::tests` → PASS

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Detect changes in dirty working tree
    Tool: Bash (cargo test)
    Preconditions: Temp git repo with initial commit + modifications
    Steps:
      1. Create temp dir, git init, add files, commit
      2. Build index at this commit
      3. Modify one file, add a new file, delete one file
      4. Run detect_changes
      5. Assert modified file is in modified_files
      6. Assert new file is in overlay n-grams
      7. Assert deleted file is in deleted_files
    Expected Result: All three change types correctly detected
    Failure Indicators: Missing change detection, git command failure
    Evidence: .sisyphus/evidence/task-15-git-overlay.txt
  ```

  **Commit**: YES
  - Message: `feat(core): implement git-based index versioning with dirty overlay`
  - Files: `crates/igrep-core/src/git_overlay.rs, crates/igrep-core/src/lib.rs`
  - Pre-commit: `cargo test -p igrep-core`

- [x] 16. Case-insensitive search support

  **What to do**:
  - Modify the index writer (Task 11) and search pipeline (Task 14) to support case-insensitive search:
    - Index writer: for each file, extract n-grams from BOTH original content AND lowercased content. Store lowercased n-grams in a separate set of posting lists (or use a flag/prefix to distinguish).
    - Simpler approach (recommended): During index construction, extract n-grams from lowercased content only. At query time for `-i`, lowercase the query's extracted n-grams before lookup. During verification, use `regex::RegexBuilder::case_insensitive(true)`.
    - Even simpler approach (recommended for v1): Always index the lowercased content. For case-sensitive search, the index may produce more false positives (which are filtered by the verification step). For case-insensitive search, the index is naturally correct.
  - Modify `search.rs`:
    - `pub fn search_with_options(reader: &IndexReader, pattern: &str, root: &Path, case_insensitive: bool) -> Result<Vec<SearchResult>>`
    - If case_insensitive: use `RegexBuilder::new(pattern).case_insensitive(true).build()?` for verification
  - Modify `query.rs`:
    - When building the query for case-insensitive mode, lowercase all literal bytes before extracting n-grams
  - Write tests:
    - Index file containing "FooBar", search for "foobar" with case_insensitive=true → match
    - Search for "FOOBAR" case-insensitive → same match
    - Search for "foobar" case-sensitive → no match (FooBar doesn't match)
    - Use case_test.rs from test-fixtures

  **Must NOT do**:
  - Do not implement Unicode case folding — ASCII toLower is sufficient for source code
  - Do not maintain two separate index files (one per case)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Cross-cutting concern touching index writer, query builder, and search pipeline
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Task 15)
  - **Parallel Group**: Wave 3
  - **Blocks**: Task 19
  - **Blocked By**: Tasks 7, 14

  **References**:

  **Pattern References**:
  - Russ Cox blog: Case-insensitive section showing query expansion (abc → {ABC, ABc, AbC, ...} — 8 variations per trigram). We avoid this by indexing lowercased text.
  - `regex::RegexBuilder::case_insensitive(true)` — Rust regex API for case-insensitive matching

  **WHY Each Reference Matters**:
  - Cox blog: Shows why query-time expansion is expensive (exponential) — motivates our index-lowercased approach
  - regex crate: The API we use for case-insensitive verification

  **Acceptance Criteria**:

  **If TDD:**
  - [ ] Case-insensitive search tests with test-fixtures
  - [ ] `cargo test -p igrep-core -- search::tests::case_insensitive` → PASS

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Case-insensitive search finds mixed-case matches
    Tool: Bash (cargo test)
    Preconditions: Case-insensitive support implemented
    Steps:
      1. Index test-fixtures (which contain "FooBar" and "foobar" in case_test.rs)
      2. Search for "foobar" with case_insensitive=true
      3. Assert case_test.rs is in results
      4. Search for "FOOBAR" case_insensitive=true → same result
    Expected Result: Both searches find the mixed-case content
    Failure Indicators: Case-insensitive search misses valid matches
    Evidence: .sisyphus/evidence/task-16-case-insensitive.txt
  ```

  **Commit**: YES
  - Message: `feat(core): implement case-insensitive search support`
  - Files: `crates/igrep-core/src/search.rs, crates/igrep-core/src/query.rs, crates/igrep-core/src/index/writer.rs`
  - Pre-commit: `cargo test -p igrep-core`

- [x] 17. igrep-index CLI command with clap

  **What to do**:
  - Implement the `igrep-index` binary in `crates/igrep-cli/src/bin/igrep_index.rs`:
    - Parse CLI args with clap (derive API):
      - `<PATHS>` — one or more directories/files to index (required, positional)
      - `--output-dir, -o` — where to write index files (default: `.igrep/` in the first PATH)
      - `--max-file-size` — skip files larger than this (default: 50MB)
      - `--no-git-ignore` — don't respect .gitignore
      - `--verbose, -v` — print progress (files indexed, n-grams extracted, index size)
    - Main flow:
      1. Walk paths using FileWalker (Task 13)
      2. Create IndexWriter (Task 11)
      3. For each file: read content, add to writer
      4. Finish writer, print summary
    - Output on success: `Indexed N files (M n-grams) in X.Xs. Index written to PATH`
    - Exit code 0 on success, 1 on error (with error message to stderr)
  - Write basic smoke tests:
    - Run igrep-index on test-fixtures → verify index files are created
    - Run with --verbose → verify progress output
    - Run on non-existent path → verify exit code 1 and error message

  **Must NOT do**:
  - Do not implement incremental re-indexing (always full rebuild)
  - Do not implement watch mode

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: CLI wrapper around existing library functions, standard clap pattern
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 18, 19, 20, 21)
  - **Parallel Group**: Wave 4
  - **Blocks**: Tasks 20, 21
  - **Blocked By**: Tasks 11, 14

  **References**:

  **Pattern References**:
  - `google/codesearch/cmd/cindex/cindex.go` — cindex CLI: accepts paths, writes index, reports stats

  **External References**:
  - `clap` crate docs (derive API): For arg parsing pattern

  **WHY Each Reference Matters**:
  - codesearch cindex: Shows the CLI UX pattern — accept paths, print summary, report errors

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: igrep-index builds index successfully
    Tool: Bash
    Preconditions: Full pipeline working
    Steps:
      1. Run `cargo build --release -p igrep-cli`
      2. Run `target/release/igrep-index ./test-fixtures`
      3. Assert exit code is 0
      4. Assert `.igrep/index.postings` and `.igrep/index.lookup` exist in test-fixtures
      5. Assert stdout contains "Indexed" and a file count
    Expected Result: Index built, files created, success message printed
    Failure Indicators: Non-zero exit, missing index files, no output
    Evidence: .sisyphus/evidence/task-17-igrep-index-cli.txt

  Scenario: Error handling for invalid path
    Tool: Bash
    Preconditions: igrep-index binary built
    Steps:
      1. Run `target/release/igrep-index /nonexistent/path 2>/tmp/stderr.txt; echo $?`
      2. Assert exit code is 1
      3. Assert /tmp/stderr.txt contains error message about path
    Expected Result: Graceful error with exit code 1
    Failure Indicators: Exit 0 on error, panic, no error message
    Evidence: .sisyphus/evidence/task-17-error-handling.txt
  ```

  **Commit**: YES
  - Message: `feat(cli): implement igrep-index command with clap`
  - Files: `crates/igrep-cli/src/bin/igrep_index.rs, crates/igrep-cli/Cargo.toml`
  - Pre-commit: `cargo test -p igrep-cli`

- [x] 18. igrep search CLI command with grep-compatible output

  **What to do**:
  - Implement the `igrep` binary in `crates/igrep-cli/src/bin/igrep.rs`:
    - Parse CLI args with clap:
      - `<PATTERN>` — regex pattern (required, positional)
      - `<PATH>` — root directory to search (optional, default: current dir)
      - `--index-dir` — path to index directory (default: `.igrep/` in PATH)
    - Main flow:
      1. Open IndexReader (Task 12)
      2. Optionally detect git changes and build overlay (Task 15)
      3. Run search (Task 14)
      4. Print results in grep format: `file_path:line_number:line_content`
    - Output format:
      - Default: `path/to/file.rs:42:    let x = foo();`
      - Colorized output when stdout is a terminal (use `termcolor` or `ansi_term` crate, or keep it simple with ANSI escape codes)
    - Exit code: 0 if matches found, 1 if no matches, 2 on error
  - Write tests:
    - Build index on test-fixtures, run igrep "fn main" → verify output contains expected file:line:content
    - Run igrep with non-matching pattern → exit code 1
    - Run with missing index → error message and exit code 2

  **Must NOT do**:
  - Do not implement colored output highlighting in v1 (can add later)
  - Do not implement streaming/pagination

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: CLI with output formatting, git overlay integration, exit code semantics
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 17, 19, 20, 21)
  - **Parallel Group**: Wave 4
  - **Blocks**: Tasks 19, 21
  - **Blocked By**: Tasks 12, 14, 15

  **References**:

  **Pattern References**:
  - `google/codesearch/cmd/csearch/csearch.go` — csearch CLI: accepts pattern + flags, prints file:line:match

  **External References**:
  - grep man page: exit code conventions (0=match, 1=no match, 2=error)

  **WHY Each Reference Matters**:
  - codesearch csearch: Production CLI pattern for indexed grep — our direct model
  - grep exit codes: Standard convention that users expect

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: igrep produces grep-compatible output
    Tool: Bash
    Preconditions: Index built on test-fixtures
    Steps:
      1. Run `target/release/igrep "fn main" ./test-fixtures`
      2. Assert output contains lines in format "path:line_num:content"
      3. Assert output contains "simple.rs" with correct line number
      4. Assert exit code is 0
    Expected Result: grep-format output with correct matches
    Failure Indicators: Wrong format, missing matches, wrong exit code
    Evidence: .sisyphus/evidence/task-18-igrep-output.txt

  Scenario: No matches returns exit code 1
    Tool: Bash
    Preconditions: Index built
    Steps:
      1. Run `target/release/igrep "zzz_nonexistent_pattern_zzz" ./test-fixtures; echo $?`
      2. Assert exit code is 1
      3. Assert stdout is empty
    Expected Result: Clean exit with code 1, no output
    Failure Indicators: Exit 0 when no matches, error output
    Evidence: .sisyphus/evidence/task-18-no-match-exit.txt
  ```

  **Commit**: YES
  - Message: `feat(cli): implement igrep search command with grep-compatible output`
  - Files: `crates/igrep-cli/src/bin/igrep.rs`
  - Pre-commit: `cargo test -p igrep-cli`

- [x] 19. CLI flags: -l, -c, -n, -i, -f

  **What to do**:
  - Add standard grep flags to the `igrep` CLI:
    - `-l, --files-with-matches` — print only file paths, not matching lines. Use `search_files_only()` for efficiency (stop after first match per file).
    - `-c, --count` — print file path and match count: `file.rs:3`
    - `-n, --line-number` — prefix each output line with line number (default behavior, but explicitly flag it)
    - `-i, --ignore-case` — case-insensitive search (uses Task 16's implementation)
    - `-f, --file-filter <REGEX>` — only search files whose paths match this regex (like csearch's `-f` flag)
    - `--no-index` — bypass index, brute-force grep all files (useful for debugging/comparison)
  - Modify igrep.rs to handle these flags by adjusting the search call and output formatting
  - Write tests for each flag combination:
    - `-l` only prints file paths
    - `-c` prints counts
    - `-i` finds case-insensitive matches
    - `-f "\.rs$"` only searches .rs files
    - `--no-index` produces same results as indexed search (just slower)

  **Must NOT do**:
  - Do not add flags not in this list (no --json, --xml, --context, etc.)
  - Do not implement -A/-B/-C (context lines) in v1

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Adding clap flags and adjusting output format — straightforward modifications
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 20, 21)
  - **Parallel Group**: Wave 4
  - **Blocks**: Task 21
  - **Blocked By**: Tasks 18, 16

  **References**:

  **Pattern References**:
  - `google/codesearch/cmd/csearch/csearch.go` — csearch flags: -c, -f, -h, -i, -l, -n

  **WHY Each Reference Matters**:
  - codesearch csearch: Shows the standard flag set for indexed grep tools

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: -l flag prints only file paths
    Tool: Bash
    Preconditions: Index built on test-fixtures
    Steps:
      1. Run `target/release/igrep -l "fn main" ./test-fixtures`
      2. Assert output contains file paths only (no line numbers or content)
      3. Assert output contains "simple.rs"
    Expected Result: File-path-only output
    Failure Indicators: Output contains ":" followed by line content
    Evidence: .sisyphus/evidence/task-19-flag-l.txt

  Scenario: -i flag enables case-insensitive search
    Tool: Bash
    Preconditions: Index built, case_test.rs has mixed case content
    Steps:
      1. Run `target/release/igrep -i "foobar" ./test-fixtures`
      2. Assert output includes match from case_test.rs
    Expected Result: Case-insensitive match found
    Failure Indicators: No matches when FooBar exists in file
    Evidence: .sisyphus/evidence/task-19-flag-i.txt
  ```

  **Commit**: YES
  - Message: `feat(cli): add -l, -c, -n, -i, -f flags`
  - Files: `crates/igrep-cli/src/bin/igrep.rs`
  - Pre-commit: `cargo test -p igrep-cli`

- [x] 20. Criterion benchmarks (index build + search latency)

  **What to do**:
  - Create `crates/igrep-core/benches/` directory with benchmark files:
    - `benches/index_bench.rs`:
      - Benchmark: `build_all` n-gram extraction throughput (bytes/sec) on synthetic content
      - Benchmark: `build_covering` n-gram extraction throughput
      - Benchmark: Full index construction on test-fixtures
      - Benchmark: Posting list encode/decode throughput
      - Parameterized: Vary input size (1KB, 10KB, 100KB, 1MB)
    - `benches/search_bench.rs`:
      - Benchmark: Query evaluation (index lookup only) for literal patterns
      - Benchmark: Query evaluation for complex regex patterns
      - Benchmark: Full search (index lookup + file verification) on test-fixtures
      - Benchmark: Binary search on lookup table (varying table sizes)
      - Parameterized: Vary query type (literal, simple regex, complex regex)
    - `benches/ngram_bench.rs`:
      - Benchmark: Frequency table lookup throughput
      - Benchmark: CRC32 hash throughput for n-grams
      - Benchmark: Trigram extraction vs sparse n-gram extraction (comparison)
  - All benchmarks use `criterion::Criterion` with proper `black_box()` to prevent optimization
  - Include `#[bench]` attribute and `criterion_group!` / `criterion_main!` macros
  - Ensure benchmarks compile and run (even if performance isn't optimized yet)

  **Must NOT do**:
  - Do not optimize code to hit specific benchmark targets — just measure current performance
  - Do not require downloading the Linux kernel for benchmarks (use test-fixtures for now, document how to bench on larger corpora in README)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Multiple benchmark files, parameterized benchmarks, criterion configuration
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 19, 21)
  - **Parallel Group**: Wave 4
  - **Blocks**: F1-F4
  - **Blocked By**: Tasks 17, 18

  **References**:

  **External References**:
  - `criterion` crate docs: BenchmarkGroup, parameterized benchmarks
  - neptune-core/divan benchmark examples: Attribute-based benchmark patterns

  **WHY Each Reference Matters**:
  - criterion docs: Exact API for bench groups, parameterized inputs, HTML reports

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Benchmarks compile and run
    Tool: Bash
    Preconditions: All library code working
    Steps:
      1. Run `cargo bench --workspace -- --quick`
      2. Assert exit code is 0
      3. Assert output contains benchmark timing results
    Expected Result: All benchmarks run without errors
    Failure Indicators: Compilation error, benchmark panic, timeout
    Evidence: .sisyphus/evidence/task-20-benchmarks-run.txt
  ```

  **Commit**: YES
  - Message: `bench: add criterion benchmarks for indexing and search`
  - Files: `crates/igrep-core/benches/*.rs, crates/igrep-core/Cargo.toml`
  - Pre-commit: `cargo bench --workspace -- --quick`

- [x] 21. Integration tests: igrep vs grep correctness

  **What to do**:
  - Create `tests/integration/` directory at workspace root (or `crates/igrep-core/tests/integration.rs`)
  - Write comprehensive integration tests that verify igrep produces correct results by comparing against `grep -rP`:
    - For each of 20+ regex patterns:
      1. Build index on test-fixtures (once, in test setup)
      2. Run igrep search
      3. Run `grep -rPl <pattern> test-fixtures/` via `std::process::Command`
      4. Assert: every file returned by grep is also returned by igrep (zero false negatives)
      5. Record false positive rate (files igrep returns that grep doesn't — acceptable, just measure)
    - Test patterns covering all regex features:
      - Pure literals: `"fn main"`, `"use std"`, `"TODO"`
      - Character classes: `"[A-Z][a-z]+"`, `"[0-9]{3}"`
      - Alternation: `"foo|bar"`, `"struct|enum|trait"`
      - Repetition: `"a+"`, `"\\w{4,}"`
      - Wildcards: `"fn\\s+\\w+"`, `"#\\[.*\\]"`
      - Complex: `"pub\\s+(fn|struct|enum)\\s+\\w+"`, `"impl.*for\\s+\\w+"`
      - Edge case: `".*"` (matches everything), `""` (empty pattern)
    - Also test line-level output correctness (not just file matching): for 5 patterns, compare `igrep -n` output line numbers against `grep -rPn` output
  - Create a `#[test] fn no_false_negatives_comprehensive()` that runs all patterns and fails if ANY false negative is detected

  **Must NOT do**:
  - Do not test performance (that's Task 20)
  - Do not test CLI flags (that's Task 19)

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Comprehensive comparison testing, subprocess management, result parsing and comparison
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 20)
  - **Parallel Group**: Wave 4
  - **Blocks**: F1-F4
  - **Blocked By**: Tasks 14, 17, 19, 4

  **References**:

  **Pattern References**:
  - Metis review AC1/AC2: Specified the exact comparison approach (igrep output vs grep output, diff must be empty)

  **WHY Each Reference Matters**:
  - Metis acceptance criteria: This is the definitive correctness test — if these pass, the tool is correct

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Zero false negatives for all test patterns
    Tool: Bash
    Preconditions: Full pipeline built and working
    Steps:
      1. Run `cargo test --test integration -- --test-threads=1`
      2. Assert ALL 20+ pattern comparisons pass
      3. Assert no false negatives detected
    Expected Result: 0 false negatives across all patterns
    Failure Indicators: Any pattern where grep finds a file that igrep misses
    Evidence: .sisyphus/evidence/task-21-integration-tests.txt

  Scenario: False positive rate is reasonable
    Tool: Bash
    Preconditions: Integration tests complete
    Steps:
      1. Check test output for false positive counts per pattern
      2. Assert average false positive rate < 20% (igrep returns at most 20% more files than grep)
    Expected Result: Reasonable false positive rate
    Failure Indicators: >50% false positive rate on any pattern
    Evidence: .sisyphus/evidence/task-21-false-positive-rate.txt
  ```

  **Commit**: YES
  - Message: `test: add integration tests comparing igrep vs grep`
  - Files: `tests/integration/*.rs`
  - Pre-commit: `cargo test --test integration`

---

## Final Verification Wave

> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.

- [x] F1. **Plan Compliance Audit** — `oracle`
  Read the plan end-to-end. For each "Must Have": verify implementation exists (read file, run `cargo test`, run CLI command). For each "Must NOT Have": search codebase for forbidden patterns (`grep -r "TODO\|FIXME\|as any\|unwrap()" src/`) — reject with file:line if found. Check evidence files exist in `.sisyphus/evidence/`. Compare deliverables against plan.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [x] F2. **Code Quality Review** — `unspecified-high`
  Run `cargo clippy --workspace -- -D warnings` + `cargo test --workspace` + `cargo bench --workspace -- --quick`. Review all source files for: unnecessary `unwrap()`, empty error handling, `todo!()` macros, commented-out code, unused imports. Check AI slop: excessive comments, over-abstraction, generic names.
  Output: `Build [PASS/FAIL] | Clippy [PASS/FAIL] | Tests [N pass/N fail] | Files [N clean/N issues] | VERDICT`

- [x] F3. **Real Manual QA** — `unspecified-high`
  Start from clean state (`cargo build --release`). Execute EVERY QA scenario from EVERY task — follow exact steps, capture evidence. Test cross-task integration (index build → search → git overlay working together). Test edge cases: empty files, binary files, very large files, regex matching empty string. Save to `.sisyphus/evidence/final-qa/`.
  Output: `Scenarios [N/N pass] | Integration [N/N] | Edge Cases [N tested] | VERDICT`

- [x] F4. **Scope Fidelity Check** — `deep`
  For each task: read "What to do", read actual implementation (`git diff` or file contents). Verify 1:1 — everything in spec was built (no missing), nothing beyond spec was built (no creep). Check "Must NOT do" compliance across all tasks. Detect cross-task contamination. Flag unaccounted files.
  Output: `Tasks [N/N compliant] | Contamination [CLEAN/N issues] | Unaccounted [CLEAN/N files] | VERDICT`

---

## Commit Strategy

| After Task(s) | Commit Message | Files | Pre-commit |
|---------------|---------------|-------|------------|
| 1 | `chore: initialize cargo workspace with igrep-core and igrep-cli crates` | Cargo.toml, crates/*/Cargo.toml, crates/*/src/lib.rs | `cargo check --workspace` |
| 2 | `feat(core): add embedded bigram frequency table for sparse n-gram weights` | crates/igrep-core/src/frequency_table.rs, data/bigram_freq.bin | `cargo test -p igrep-core` |
| 3 | `feat(core): add core types module (DocId, NgramHash, Query, PostingList)` | crates/igrep-core/src/types.rs | `cargo test -p igrep-core` |
| 4 | `test: add test fixtures directory with known-answer corpus` | test-fixtures/**/* | — |
| 5 | `feat(core): implement build_all sparse n-gram extraction (monotone stack)` | crates/igrep-core/src/ngram.rs | `cargo test -p igrep-core` |
| 6 | `feat(core): implement build_covering sparse n-gram extraction (deque)` | crates/igrep-core/src/ngram.rs | `cargo test -p igrep-core` |
| 7 | `feat(core): implement classic trigram extraction and probabilistic masks` | crates/igrep-core/src/trigram.rs | `cargo test -p igrep-core` |
| 8 | `feat(core): implement varint encoding and delta-encoded posting lists` | crates/igrep-core/src/posting.rs | `cargo test -p igrep-core` |
| 9 | `feat(core): implement posting list intersection (AND) and union (OR)` | crates/igrep-core/src/posting.rs | `cargo test -p igrep-core` |
| 10 | `feat(core): implement regex to query tree decomposition (Cox algorithm)` | crates/igrep-core/src/query.rs | `cargo test -p igrep-core` |
| 11 | `feat(core): implement index writer with 2-pass construction` | crates/igrep-core/src/index/writer.rs | `cargo test -p igrep-core` |
| 12 | `feat(core): implement mmap'd index reader with binary search` | crates/igrep-core/src/index/reader.rs | `cargo test -p igrep-core` |
| 13 | `feat(core): implement parallel file walker with binary detection` | crates/igrep-core/src/walker.rs | `cargo test -p igrep-core` |
| 14 | `feat(core): implement end-to-end search pipeline` | crates/igrep-core/src/search.rs | `cargo test -p igrep-core` |
| 15 | `feat(core): implement git-based index versioning with dirty overlay` | crates/igrep-core/src/git_overlay.rs | `cargo test -p igrep-core` |
| 16 | `feat(core): implement case-insensitive search support` | crates/igrep-core/src/search.rs | `cargo test -p igrep-core` |
| 17 | `feat(cli): implement igrep-index command with clap` | crates/igrep-cli/src/main.rs, crates/igrep-cli/src/index_cmd.rs | `cargo test -p igrep-cli` |
| 18 | `feat(cli): implement igrep search command with grep-compatible output` | crates/igrep-cli/src/search_cmd.rs | `cargo test -p igrep-cli` |
| 19 | `feat(cli): add -l, -c, -n, -i, -f flags` | crates/igrep-cli/src/search_cmd.rs | `cargo test -p igrep-cli` |
| 20 | `bench: add criterion benchmarks for indexing and search` | benches/*.rs | `cargo bench --workspace -- --quick` |
| 21 | `test: add integration tests comparing igrep vs grep` | tests/integration/*.rs | `cargo test --test integration` |

---

## Success Criteria

### Verification Commands
```bash
# All tests pass
cargo test --workspace                    # Expected: 0 failures

# Clippy clean
cargo clippy --workspace -- -D warnings   # Expected: 0 warnings

# Benchmarks run
cargo bench --workspace -- --quick        # Expected: completes without error

# CLI smoke test
cargo build --release
target/release/igrep-index ./test-fixtures
target/release/igrep "fn main" ./test-fixtures    # Expected: matches in known files
target/release/igrep -l "TODO" ./test-fixtures     # Expected: known file count

# Zero false negatives (for each test pattern)
for p in "fn\s+\w+" "use\s+std" "struct\s+\w+" "#\[derive" "pub\s+fn"; do
  target/release/igrep -l "$p" ./test-fixtures | sort > /tmp/actual.txt
  grep -rPl "$p" ./test-fixtures | sort > /tmp/expected.txt
  diff /tmp/actual.txt /tmp/expected.txt
done
# Expected: no diff output (igrep returns at least all files grep returns)
```

### Final Checklist
- [ ] All "Must Have" features implemented and tested
- [ ] All "Must NOT Have" guardrails verified (no forbidden patterns)
- [ ] All property-based tests pass (sparse n-gram invariants)
- [ ] All integration tests pass (igrep ⊇ grep for 20+ patterns)
- [ ] Index size ≤ 30% of test corpus size
- [ ] Benchmark suite runs successfully
- [ ] CLI tools work end-to-end
- [ ] README with usage examples exists
