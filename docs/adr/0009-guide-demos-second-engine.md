# 0009. Guide demos run on a second engine instance

- Status: Accepted
- Date: 2026-09-25

## Context

The guide and codex explain every item and building, and the owner asked for animations of
each use case rendered live by the game on request. Recorded videos would be large, go
stale with every visual change, and not work offline.

## Decision

- `demo.ts` creates a second instance of the same compiled Wasm module with its own small
  world (64×40) and its own `Renderer` on a canvas inside the guide sheet.
- `fx_demo` resets that world to a blank sandbox map (flat ground, no wrecks, noon), and a
  few sandbox-only helpers paint deposits and lakes, turn a Storage into a bottomless
  supply, speed machines up, and drop wrecks. They refuse to act on a non-sandbox world.
- Scenes are data (`scenes.ts`): item scenes are generated from the recipe tables (the
  item's maker belting it into a user); buildings and lessons have hand-made scenes.
  Supplies and sinks sit past the edge of the view.
- The demo copies the player's planet, stage and belt colour, only ticks while its page is
  open, and plays only when asked.

## Consequences

- Every item and building has an accurate, current demo for almost no download size.
- The player never touches the real game world.
- The second instance keeps its memory and a second WebGL context alive after first use
  (see KNOWN-ISSUES.md).
- Scenes can only be checked by playing them; the sweep in skill `verify-in-browser`
  plays them all.
