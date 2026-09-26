# Architecture decision records

Short records of the decisions that shape Beltwise: what was decided, why, and what it
costs. Read the relevant ones before reworking a subsystem; if you change a decision,
write a new record that supersedes the old one instead of editing history.

| # | Decision | Status |
| --- | --- | --- |
| [0001](0001-rust-wasm-engine-raw-c-abi.md) | The simulation is Rust compiled to Wasm, behind a raw C ABI | Accepted |
| [0002](0002-data-oriented-fixed-step-simulation.md) | Data-oriented world, fixed 60 Hz steps, interpolated rendering | Accepted |
| [0003](0003-segment-belts-with-gap-encoding.md) | Belts are segments that store the gaps between items | Accepted |
| [0004](0004-two-draw-call-webgl2-renderer.md) | Raw WebGL2, two draw calls, sprites streamed from Wasm memory, procedural art | Accepted |
| [0005](0005-content-as-data.md) | All game content is data in `content.rs`, sent to the UI as JSON | Accepted |
| [0006](0006-versioned-binary-saves.md) | Versioned binary saves that store only what can't be regenerated | Accepted |
| [0007](0007-power-slows-never-breaks.md) | Power networks by union-find; shortages slow machines, never break them | Accepted |
| [0008](0008-static-offline-pwa-on-pages.md) | A static, offline-first PWA deployed to GitHub Pages | Accepted |
| [0009](0009-guide-demos-second-engine.md) | Guide demos run on a second engine instance | Accepted |
| [0010](0010-full-width-belts-and-stubs.md) | Belts fill their tiles; stubs connect them to buildings | Accepted |

## Template

```markdown
# NNNN. Title in the imperative or as a statement

- Status: Proposed | Accepted | Superseded by NNNN
- Date: YYYY-MM-DD

## Context
What forces are at play; what problem needs deciding.

## Decision
What we do.

## Consequences
What gets easier, what gets harder, what must now be kept in sync.
```
