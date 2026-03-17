# har-viewer

Client-side HAR viewer built with Rust, Leptos, and WebAssembly.

## Overview

`har-viewer` loads a HAR file in the browser and lets you inspect requests/responses with filtering, sorting, and split request/response panes.  
No backend is required.

## Features

- Local HAR import via file picker or drag-and-drop
- Indexed parsing for large HAR files
- Toolbar filters (search, method, status group, MIME)
- Sortable HTTP history table
- Split inspector for raw request and response messages
- Light/dark theme toggle

## Requirements

- Rust (stable)
- `wasm32-unknown-unknown` target
- Trunk (`cargo install trunk`)

## Setup

```powershell
rustup target add wasm32-unknown-unknown
```

## Run (development)

```powershell
trunk serve
```

Then open the local URL printed by Trunk (typically `http://127.0.0.1:8080`).

## Build

```powershell
trunk build --release
```

## Test

```powershell
cargo test
cargo check --target wasm32-unknown-unknown
```

## Project Structure

- `src/ui/` UI components and interaction logic
- `src/har/` HAR scanning, parsing, and message formatting
- `src/state/` app state and selection/sort/filter behavior
- `src/filter/` filter model
- `style/main.css` app styling
- `index.html` Trunk entry point

## Notes

- The app is intended to run in the browser. Running the native binary prints a message and exits.
