# Test Fixtures - Expected Patterns

This file documents which patterns exist in which files for deterministic testing.

## Pattern: `fn main()`
- `simple.rs` (line 4)
- `server.go` (line 9, as `func main()`)

## Pattern: `func main()`
- `server.go` (line 9)

## Pattern: `TODO`
- `simple.rs`: "TODO: fix this"
- `large_line.js`: "TODO: optimize this minified bundle"
- `nested/deep/module.rs`: "TODO: return something meaningful"
- `.hidden_file.txt`: "TODO: ensure hidden file search works correctly"
- `server.go`: "TODO: add proper error handling"
- `data_processor.py`: "TODO: add streaming support"
- `component.tsx`: "TODO: add debounce to search input"
- `types.ts`: "TODO: add support for file type filters"
- `error_handling.rs`: "TODO: implement std::error::Error trait"
- `config.go`: "TODO: support TOML and YAML formats"
- `cli.py`: "TODO: add --hidden flag to include dotfiles"
- `test_search.py`: "TODO: implement actual file walking test"
- `imports.rs`: "TODO: add async imports with tokio"

## Pattern: `FIXME`
- `patterns.rs`: "FIXME: add more colors"
- `unicode.py`: "FIXME: handle edge cases with surrogate pairs"
- `no_extension`: "FIXME: should we detect shebang lines?"
- `utils.go`: "FIXME: make this case-insensitive"
- `component.tsx`: "FIXME: replace with actual API call"
- `app.js`: "FIXME: sanitize input properly"
- `traits.rs`: "FIXME: implement Searchable for IndexEntry"
- `regex_patterns.rs`: "FIXME: handle Unicode properly in trigram extraction"
- `helpers.js`: "FIXME: handle all special regex characters"
- `data_processor.py`: "FIXME: support regex patterns"
- `walker.go`: "FIXME: add support for .gitignore parsing"

## Pattern: `use std::io`
- `simple.rs` (line 1)
- `error_handling.rs` (line 2)

## Pattern: `struct MyStruct`
- `patterns.rs` (line 4)

## Pattern: `#[derive(Debug)]`
- `patterns.rs` (line 3)
- `traits.rs` (line 13, as `#[derive(Debug, Clone)]`)
- `error_handling.rs` (line 5)

## Pattern: `pub fn example()`
- `patterns.rs` (line 9)

## Pattern: `impl Display for` / `impl fmt::Display for`
- `patterns.rs` (line 16)
- `error_handling.rs` (line 12)

## Pattern: `pub struct`
- `patterns.rs`: `pub struct MyStruct`
- `case_test.rs`: `pub struct FooBar`
- `traits.rs`: `pub struct IndexEntry`
- `error_handling.rs`: `pub struct ErrorStats`
- `imports.rs`: `pub struct ImportTest`

## Pattern: `pub fn`
- `patterns.rs`: `pub fn example()`
- `case_test.rs`: `pub fn new()`, `pub fn test_case_sensitivity()`
- `nested/deep/module.rs`: `pub fn deep_function()`
- `traits.rs`: `pub fn new()`, `pub fn compute_trigrams()`
- `error_handling.rs`: `pub fn read_file_safe()`
- `regex_patterns.rs`: `pub fn match_literal()`, `pub fn match_word_boundary()`, `pub fn build_trigram_map()`
- `multiline.rs`: `pub fn multiline_string()`, `pub fn contains_special_chars()`, `pub fn blank_lines_test()`, `pub fn this_is_a_very_long_function_name_that_should_still_be_searchable()`
- `imports.rs`: `pub fn new()`, `pub fn shared()`

## Pattern: `pub enum`
- `patterns.rs`: `pub enum Color`
- `error_handling.rs`: `pub enum SearchError`
- `regex_patterns.rs`: `pub enum MatchType`

## Pattern: `FooBar` (case-sensitive)
- `case_test.rs`: `pub struct FooBar`, `FooBar::new()`, `FooBar { count: 0 }`

## Pattern: `foobar` (case-sensitive)
- `case_test.rs`: `let foobar = 42;`

## Pattern: `FOOBAR` (case-sensitive)
- `case_test.rs`: `let FOOBAR = 99;`

## Pattern: `package main`
- `server.go` (line 1)

## Pattern: `import "fmt"` / `import fmt`
- `server.go` (imports `"fmt"`)
- `utils.go` (imports `"fmt"`)
- `config.go` (imports `"fmt"`)
- `walker.go` (imports various)

## Pattern: `def ` (Python function)
- `unicode.py`: `def process_text()`
- `data_processor.py`: `def load_file()`, `def process()`, `def find_pattern()`
- `cli.py`: `def parse_args()`, `def main()`, `def format_match()`
- `test_search.py`: `def test_literal_match()`, `def test_case_insensitive()`, etc.

## Pattern: `class ` (Python class)
- `unicode.py`: `class TextProcessor`
- `data_processor.py`: `class DataProcessor`, `class Config`
- `cli.py`: `class OutputFormatter`
- `test_search.py`: `class TestPatternSearch`, `class TestFileWalker`

## Pattern: `import os`
- `data_processor.py` (line 3)
- `cli.py` (line 4)
- `test_search.py` (line 4)

## Pattern: `function` (JS)
- `app.js`: `function createApp()`, `function parseQuery()`
- `helpers.js`: `function debounce()`, `function escapeRegex()`, `function truncateLine()`
- `large_line.js`: `function shortFunc()`

## Pattern: `const`
- `app.js`: `const express`, `const PORT`, `const app`
- `helpers.js`: `const MAX_LINE_LENGTH`, `const TRUNCATION_MARKER`
- `component.tsx`: `const mockData`
- `types.ts`: `const DEFAULT_OPTIONS`

## Pattern: `export default`
- `component.tsx`: `export default function SearchComponent`
- `helpers.js`: `export default { debounce, escapeRegex, truncateLine }`

## Pattern: `useState`
- `component.tsx` (lines 10-11)

## Special Files

### empty.rs
- 0 bytes, no content

### binary.bin
- Contains null bytes (\x00), not valid UTF-8
- Should be detected as binary and skipped

### large_line.js
- Line 1 is >2500 characters (minified JS variable assignment)
- Tests line length truncation

### unicode.py
- Contains Japanese: こんにちは世界
- Contains German: München
- Contains emoji: ✅
- Contains Chinese: 搜索测试
- Contains Russian: Привет мир

### .hidden_file.txt
- Dotfile, should be skipped by default, found when hidden files enabled
- Contains: `HIDDEN_MARKER_123`

### no_extension
- File with no extension, has shebang `#!/bin/sh`

### nested/deep/module.rs
- Tests deep directory traversal

### whitespace.txt
- Tests tab vs space indentation
- Contains blank lines (single and double)
- Contains trailing whitespace

### config.json
- JSON file with configuration data
- Tests non-code file handling

## File Count Summary

| Extension | Count | Files |
|-----------|-------|-------|
| .rs       | 8     | simple, patterns, empty, case_test, error_handling, traits, regex_patterns, multiline, imports, nested/deep/module |
| .go       | 4     | server, utils, config, walker |
| .py       | 4     | unicode, data_processor, cli, test_search |
| .js       | 3     | large_line, app, helpers |
| .tsx      | 1     | component |
| .ts       | 1     | types |
| .txt      | 1     | whitespace |
| .bin      | 1     | binary |
| .json     | 1     | config |
| .md       | 1     | EXPECTED |
| (hidden)  | 1     | .hidden_file.txt |
| (none)    | 1     | no_extension |
| **Total** | **22+** | |
