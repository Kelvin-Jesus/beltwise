# 0001. The simulation is Rust compiled to Wasm, behind a raw C ABI

- Status: Accepted
- Date: 2026-09-24

## Context

A factory game simulates tens of thousands of belt items and thousands of machines every
tick, on phones, while rendering at 60–120 FPS. JavaScript engines handle this, but GC
pauses and object-heavy code make frame times unpredictable. The game must also be small
to download and run offline.

## Decision

- The whole simulation is Rust (`engine/`, std only, no crates) compiled to
  `wasm32-unknown-unknown`. The toolchain is pinned (`rust-toolchain.toml`) so local and CI
  builds emit the same Wasm features, which `wasm-opt` must be told about.
- JS talks to it through plain `extern "C"` exports (`fx_*`) that take and return numbers.
  No wasm-bindgen and no imports.
- Bulk data (the sprite instance buffer, stats, storage, tile maps, saves) is read in place
  through typed-array views over Wasm memory. Nothing is copied or allocated per frame.
- An `ABI_VERSION` is checked at start-up against the UI.

## Consequences

- Predictable, GC-free frame times and a ~200 KB engine.
- Every export change touches three places (`lib.rs`, `constants.ts`, `engine.ts`) and
  bumps the ABI version. Shared layouts (stats block, inspector words, sprite ids) are
  duplicated in TypeScript and must be kept in sync by hand
  (see ARCHITECTURE.md → "Keep in sync").
- Views over Wasm memory go stale when memory grows or the world is replaced; the host
  must refresh them.
- Debugging goes through tests (`cargo test`) and the dev handle `window.fx`, not a
  debugger across the boundary.
