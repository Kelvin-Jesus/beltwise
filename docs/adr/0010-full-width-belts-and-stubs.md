# 0010. Belts fill their tiles; stubs connect them to buildings

- Status: Accepted
- Date: 2026-09-25

## Context

Belts covered 92% of their tile and building bodies sit inset in theirs, so ground showed
between neighbouring belts, at side-loads, and wherever a belt met a machine or the Core.
The owner found the factory looked disconnected.

## Decision

- Belts fill their whole tile. The rails darken into a thin seam at the edge so parallel
  lines stay distinct.
- Where a belt feeds a building, or a building outputs onto a belt, `render.rs` adds a
  short belt stub (a belt sprite with a length in `param` and flags in the high rotation
  bits) that runs under the building. Machines that hand items straight to each other get
  a stub under both.
- Stubs appear only on faces that really connect. The rule lives in `takes_from` and
  `out_faces`, which mirror `Machines::accept` and `process`.

## Consequences

- Connections read at a glance, and a belt aimed at a face that accepts nothing shows no
  stub.
- Changing a machine's input or output faces now means updating the render mirror too.
- Stubs sit exactly half a tile off their belt and rely on the stripe pattern repeating
  every half tile.
