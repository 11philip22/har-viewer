# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added
- Created initial Rust project scaffold for a client-only WebAssembly HAR viewer (`Leptos` CSR + `Trunk`).
- Added app entrypoints and module structure:
  - `src/lib.rs`
  - `src/main.rs`
  - `src/har/*`
  - `src/filter/*`
  - `src/state/*`
  - `src/ui/*`
- Added web app bootstrap files:
  - `index.html`
  - `style/main.css`
- Implemented HAR domain model and interfaces:
  - `EntrySummary`, `EntryDetail`, `EntryRange`, `IndexResult`, `IndexStats`
  - `HarIndexer::index`, `HarIndexer::load_detail`
  - `HarIndexer::index_cooperative` (cooperative async indexing for UI responsiveness)
- Implemented Stage A HAR scanner (`log.entries` byte-range extraction) without loading full JSON trees.
- Implemented Stage B entry parsing for:
  - table summaries (method/host/path/status/mime/sizes/duration)
  - on-demand detail loading (request/response line, headers, bodies, timings)
- Implemented filter/query engine:
  - global text search
  - method filter
  - status group filter (1xx-5xx)
  - MIME category filter
  - duration min/max
- Implemented app state store with:
  - loaded bytes/ranges/summaries
  - selection and tab state
  - stable sorting (column + direction)
  - detail cache
  - indexing progress + error state
- Implemented Burp-style HTTP history UI shell:
  - top virtualized history table
  - bottom inspector with `Request`, `Response`, `Headers`, `Timings` tabs
  - horizontal and vertical draggable splitters
  - keyboard row navigation
  - sortable table headers
  - status/progress footer
- Implemented local HAR loading UX:
  - file picker
  - drag-and-drop import
- Added responsive styling/theme for desktop/mobile layouts.

### Changed
- Updated `Cargo.toml` with WebAssembly and app dependencies (`leptos`, `web-sys`, `gloo-file`, `gloo-timers`, `serde`, `serde_json`, `url`, `thiserror`, etc.) and `cdylib` output support.
- Added `gloo-file` `futures` feature to support async file reads in WASM.

### Fixed
- Fixed HAR parse failure for files that encode numeric fields as strings (example: `"bodySize": "23093"`).
- Parser now accepts flexible number representations (numeric or string) for:
  - `time`
  - `status`
  - `headersSize`
  - `bodySize`
  - `content.size`
  - timing values (`blocked`, `dns`, `connect`, `ssl`, `send`, `wait`, `receive`)
- Added regression test coverage for string-encoded numeric HAR values.

### Tests
- Added/maintained unit tests for:
  - scanner correctness and escaped-string handling
  - summary parsing and optional fields
  - detail loading via selected byte range
  - filter behavior
  - synthetic large-payload indexing guard
  - string-encoded numeric field parsing
- Verified with:
  - `cargo test`
  - `cargo check --target wasm32-unknown-unknown`
