# Issues — instant-grep

(none yet)

## Resolved
- `lsp_diagnostics` initially failed because `rust-analyzer` component was missing from the active Rust toolchain.
- Fixed by running `rustup component add rust-analyzer`; diagnostics on `ngram.rs` then returned clean.
