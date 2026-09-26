# Known issues and pitfalls

Keep this current: add what you find and don't fix, remove what you fix. Pitfalls are
mistakes already made once in this codebase; each cost real time to track down.

## Open issues

| Area | Issue | Notes |
| --- | --- | --- |
| Content text | The Crushing research says "stone into sand, sand into glass", but glass is a Smelter recipe; the Crusher only makes sand. | `content.rs`, tech `crushing`. |
| Content text | Tunnels pair up to `TUNNEL_RANGE` = 6 tiles apart, but the Tunnel description, the guide and the tunnel demo say "up to 5 tiles". | `types.rs`, `content.rs` (tunnel desc), `codex.ts`, `scenes.ts`. |
| Rendering | At far zoom (below 5 px per tile) belts come from the ground pass as separate small squares, so belt lines look dotted, while close up belts fill their tiles. | `shaders.ts` ground pass, `u_map` branch. The tile map has no direction data. |
| Memory | The guide's demo player keeps its engine instance, its WebGL context and a second copy of the icon atlas (about 11 MB of GPU memory with mipmaps, plus 3 MB of instance buffers) alive after the first demo. | `demo.ts`. Could share one context or free it after a while on low-memory devices. |
| UI | Blueprint naming, deletion, New planet and Launch use the browser's `prompt()` / `confirm()`. They work but look out of place, and some embedded browsers block them. | `main.ts`. |
| CI | GitHub warns that actions built for Node 20 are forced onto Node 24, and that `ubuntu-latest` moves to Ubuntu 26 from 2026-10-19. Watch the first runs after that date. | `.github/workflows/deploy.yml`. |
| Tests | Only the engine has tests. The TypeScript side (UI, renderer, demo scenes) is checked by typecheck and by hand in the browser. | See skill `verify-in-browser`. |

## Limits (by design, but easy to trip over)

- `BUILDING_COUNT` must stay ≤ 40: the far-zoom shader takes a `u_kindColor[40]` array
  (`KINDS` in `renderer.ts`). Today: 38. Raising it means resizing both.
- Atlas: 16×8 cells. Items use cells 1–63 (their id), buildings 64 + kind, 126 is the
  drone, 127 the tunnel exit. So `ITEM_COUNT ≤ 64` and `BUILDING_COUNT ≤ 62` there.
- Deposits are items 1–9 (`RESOURCE_COUNT`); the ground shader's colour table has 10 slots.
- At most 65,536 sprites per frame, with 4,608 kept for overlays after items.
- The world is 512×512 tiles (`WORLD_SIZE`); saves store their size.
- Saves write per-item arrays (storage, deliveries, offline rates) and the repeatable
  research levels without a length. Adding an item or a repeatable research makes old
  saves unreadable unless `save.rs` bumps `VERSION` and reads the previous layout, the way
  `load_v1` handles `V1_ITEMS`.
- Bit sets in the save and stats: research ≤ 64 nodes (`u64`, `RESEARCHED_LO/HI`),
  achievements ≤ 64, alternates ≤ 32, cosmetics ≤ 32. Ark phases have at most 4 cost
  entries (`ark_paid: [u32; 4]`).
- iPhone Safari caps web pages at 60 fps whatever the in-game cap says.
- Offline progress counts at most 8 hours, at the rates recorded when the game was saved.

## Pitfalls (already hit once)

**Rust and the engine**
- Edition 2024 reserves `gen`: it can't be a variable name.
- `content.rs` glob-imports both `it::*` and `tech::*`. A tech constant named like an item
  is ambiguous, which is why the techs are `SILICA` and `POLLINATORS`, not `SILICON`/`SEEDS`.
- Save bytes include the per-minute rate windows. Tests that compare saves of worlds with
  different histories must call `clear_rate_history()` first.
- Placing a machine over a belt replaces the belt without a word. Generated layouts
  (benchmark, demo scenes) must only place machines on empty tiles.
- The machine list is swap-removed: after `Machines::remove`, fix the moved machine's
  `grid.ent` back-reference (every caller does; copy that pattern).
- Allocation crept into the render path once (the night-light list). Keep scratch vectors
  on `World` and clear them per frame.
- Sandbox code paths must not assume the Core has stock (a sandbox overclock once
  underflowed the shard count).
- Clippy runs with `-D warnings` in CI (it has caught `collapsible_if`,
  `needless_range_loop`, `manual_memcpy`); run it before pushing.

**The boundary and the web host**
- Typed-array views over Wasm memory silently go stale when memory grows or the world is
  replaced (`fx_init`, `fx_load`, `fx_launch`, `fx_demo`). Always read through the
  `Engine` getters or call `refreshViews()`.
- Sheets replace their `innerHTML` when the HTML string changes. A canvas or input placed
  inside is destroyed unless re-attached after the render (`DemoPlayer.attach`); guide
  pages skip the periodic rebuild entirely.
- The HUD caches the selection card's HTML: when tool state changes without changing that
  HTML, reset `last.selection` or the card won't update.
- Frame pacing: accumulating raw intervals drifted (a 60 cap ran at 59.3, 120 at 90). The
  hybrid `FramePacer` fixes it; after touching it, test caps 60, 120 and Max on 60 and
  120/144 Hz displays.
- Belt stripes repeat every half tile. The stripe phase wraps at one tile and belt stubs
  sit exactly half a tile off their belt, so changing the stripe period breaks both.
- Fog uploads a sub-rectangle with `UNPACK_ROW_LENGTH` / `SKIP_PIXELS` / `SKIP_ROWS`;
  reset them afterwards or every later texture upload is garbled.

**Demo scenes (`scenes.ts`)**
- The demo map is 64×40 with scene (0, 0) at world (30, 20): keep scenes within about ±12
  tiles horizontally and ±8 vertically, leaving room for feeds and exits past the view.
- `wet: true` raises the heat and pressure meters so lakes hold water, which also changes
  the ground's look. `grid: true` turns off free power (power demos only). The Core only
  exists when `core` is set, and a missing Core supplies no power.
- Scenes are checked only by playing them: after editing, run the demo sweep in skill
  `verify-in-browser`.

**Assets and tooling**
- `design/logos/make-icons.sh`: ImageMagick `-fx` reserves single-letter names such as
  `a` and `b`, and PNGs need `-depth 8` or they come out 16-bit and several times larger.
- A broad `.gitignore` rule once hid a new asset (the README banner). Check `git status`
  after adding files.
- `wasm-opt` must be told every Wasm feature rustc emits (`WASM_FEATURES` in
  `scripts/build.mjs`). The toolchain is pinned (`rust-toolchain.toml`, 1.96.0) so local
  and CI builds match; a Rust upgrade may need that list updated.

## Environment notes (the owner's machine)

- Language versions come from mise. The machine's default Node is 26, but this repo pins
  Node 22 in `mise.toml` (like CI, and inside the `>=22.20 <25` range the Better Harness
  CLI requires). Agent shells start with the default and don't switch per directory, so a
  bare `node` is 26: run Node tools as `mise x -- <cmd>`.

- A global command-rewriting hook (`rtk`) wraps shell commands. A long compound command
  mixing `ls` with a heredoc once hung for two minutes: keep shell steps short, and make
  multi-line edits with the Edit tool or a script file.
- The dev server sometimes keeps printing an old build error after the fix, and the page
  needs a reload after each engine rebuild (about 4 s). Restart the `dev` server if
  changes stop showing up.
