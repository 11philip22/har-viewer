# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added
- Created initial Rust project scaffold for a client-only WebAssembly HAR viewer (`Leptos` CSR + `Trunk`).
- Added app/module structure and web bootstrap files (`src/*`, `index.html`, `style/main.css`).
- Implemented HAR scanner/indexer/parser pipeline with summary indexing and on-demand detail loading.
- Implemented filtering, sorting, selection state, and virtualized HTTP history table.
- Added local HAR import via file picker and drag-and-drop.
- Added raw HTTP message formatter helpers:
  - request message builder (`METHOD path HTTP/x` + headers + body)
  - response message builder (`HTTP/x status reason` + headers + body)
  - JSON prettification when body is valid JSON

### Changed
- Updated `Cargo.toml` with WebAssembly and app dependencies (`leptos`, `web-sys`, `gloo-file`, `gloo-timers`, `serde`, `serde_json`, `url`, `thiserror`) and `cdylib` output support.
- Added `gloo-file` `futures` feature to support async file reads in WASM.
- Replaced the bottom tabbed inspector with a fixed split view:
  - `request:` pane on the left
  - `response:` pane on the right
- Extended `EntryDetail`/parser output with request and response HTTP version fields plus explicit request method/path for raw message rendering.

### Fixed
- Fixed HAR parse failure for numeric fields encoded as strings (for example `"bodySize": "23093"`).
- Parser now accepts flexible number representations (numeric or string) for:
  - `time`
  - `status`
  - `headersSize`
  - `bodySize`
  - `content.size`
  - timing values (`blocked`, `dns`, `connect`, `ssl`, `send`, `wait`, `receive`)
- Removed UI truncation in the history table (no ellipsis/clipping; full values available via scrolling).

### Tests
- Added/maintained unit tests for:
  - scanner correctness and escaped-string handling
  - summary parsing and optional fields
  - detail loading by selected byte range
  - string-encoded numeric parsing
  - HTTP version fallback when missing
  - request/response raw message formatting
  - JSON pretty-print behavior (valid JSON vs invalid JSON)
  - host fallback rendering when request `Host` header is absent
- Verified with:
  - `cargo test`
  - `cargo check --target wasm32-unknown-unknown`
