# Architecture

How Beltwise fits together, for anyone (human or agent) about to change it. The why
behind the big choices is in [adr/](adr/); open problems and traps are in
[KNOWN-ISSUES.md](KNOWN-ISSUES.md).

```
 ┌──────────────── browser ────────────────────────────────────────────────┐
 │  web/src (TypeScript)                                                    │
 │   main.ts ── frame loop: input → fx_tick(n) → fx_render(view) → draw    │
 │     │            ▲ numbers in, numbers out (C ABI, no bindgen)           │
 │     ▼            │ typed-array views straight into Wasm memory           │
 │   renderer.ts ◄──┴── instance buffer, stats, storage, tile maps          │
 │   hud.ts · panels.ts · codex.ts ── DOM UI, reads content JSON + stats    │
 │   demo.ts ── a second engine instance for the guide's live demos        │
 │                                                                          │
 │  engine.wasm (Rust, engine/src) ── the whole simulation                  │
 └──────────────────────────────────────────────────────────────────────────┘
```

## The engine (`engine/src`)

One `World` holds the game. It lives in a static `State` in `lib.rs` (single-threaded Wasm,
so there is exactly one; the guide's demos get their own Wasm instance instead).

| Module | Owns |
| --- | --- |
| `content.rs` | Every table: items, buildings (class, cost, recipes, power, fuel), research, Ark phases, stages, objectives, achievements, logs, shop, planets. `content_json()` exports them for the UI. |
| `world.rs` | The tile grid as structure-of-arrays (`kind`, `dir`, `res`, `purity`, `ent`, `seen`), placement rules (`check_place`, `reason` codes), tunnels, undo groups, `retune` (research → speeds), the fixed-step `tick`, and the stats block. |
| `belts.rs` | Belt transport: runs of belt tiles form segments that store the gaps between items. |
| `machines.rs` | Every 1×1 building: a dense `Vec<Machine>` (swap-remove), input acceptance, per-tick processing, power draw, the `PASSIVE` table of kinds the tick loop skips. |
| `power.rs` | Power networks: union-find over the Core and poles, coverage fill, batteries. Rebuilt lazily after edits, never per tick. |
| `core.rs` | Core storage, research state, meters and stages, the objective chain, the Ark, credits, achievements. |
| `drones.rs` `explore.rs` `sky.rs` `progress.rs` `blueprint.rs` | Drone ports; fog, radar and wrecks; day/night and meteor showers; shop, launches and offline progress; blueprint capture, serialization and paste. |
| `render.rs` | Builds the per-frame instance buffer for a view rectangle. |
| `save.rs` | The binary save format. |
| `worldgen.rs` | Terrain fields, deposits with purity, wrecks, all from the seed. |
| `demo.rs` | Blank sandbox maps for the guide's demos (bottomless supplies, speed-up). |
| `lib.rs` | The exported C ABI (`fx_*`). |

### A tick (60 per second)

`World::tick`: rebuild belt and power topology if an edit dirtied it → advance every belt
segment downstream-first and hand its front item to whatever the belt points at →
process every non-passive machine and push its output to the neighbour on its output face
→ drones → radars → settle power (satisfaction per network; the Core adds 40 MW to its
own) → apply terraforming meter gains and credits → Core bookkeeping → sky (meteors) →
once a second: per-item rate windows and achievements → every 15 ticks: objective check →
refresh the stats block.

Units: belt positions are in `SUB` = 96 sub-units per tile, items at least `MIN_GAP` = 48
apart, belt speeds 3/4/6 sub-units per tick (225/300/450 items/min). Machine work is in
1/64 of a tick (`SPEED_ONE`), scaled by power satisfaction in 1/1024 (`FULL`). Machines
output through their front face (`dir`) and take input through the other faces
(splitters, sorters and tunnel entrances only from behind).

### A frame's instance buffer

`World::render(x0, y0, x1, y1, alpha, flags)` writes 16-byte records
`{x: f32, y: f32, sprite: u8, rot: u8, size: u8, param: u8, color: u32}` in draw order:
belts (and belt stubs running under buildings they connect to) → machines with badges →
the Core → items (interpolated by `alpha`) → drones → meteors → weather → night lights →
placement ghosts. Only visible tiles are scanned. `flags` are the `lod` bits (items,
structures, status badges, lights, weather); below 8 px per tile items are skipped and
below 5 px per tile belts and machines come from the ground pass's tile map instead.

### Saves

`save.rs` writes a little-endian snapshot with magic `BWSV` and a `u16` version (now 2):
only what can't be derived. Terrain, deposits and wrecks are regenerated from the seed and
patched (meteorite deposits added, salvaged wrecks removed); belt and power topology and
research tuning are rebuilt. Version 1 saves still load (item ids from 8 up shift by two,
timers are rescaled). `tests.rs` checks a deterministic round trip and v1 loading.

## The boundary

- Exports are plain numbers (`fx_place(x, y, kind, dir)`, `fx_tick(n)`...). Bulk data is
  read in place: `fx_instances()`, `fx_stats()`, `fx_storage()`, `fx_rates()`,
  `fx_kinds()`, `fx_fog()`, `fx_power_map()`, `fx_terrain()` return pointers into linear
  memory, which `engine.ts` wraps in typed-array views.
- Views die when memory grows or the world is replaced (new game, load, launch).
  `Engine.refreshViews()` rebuilds them; the `stats`/`storage`/`rates` getters call it.
- `ABI_VERSION` is checked at start-up so a stale cached JS never drives a newer engine.
- The Wasm module is compiled once (`compile()` in `engine.ts`); every `Engine.load` makes
  a new instance with its own memory. The game uses one, the guide's demo player another.

## The web host (`web/src`)

| File | Role |
| --- | --- |
| `main.ts` | Start-up, the frame loop (fixed 60 Hz steps with interpolation, `FramePacer` for 30/60/120/max caps, adaptive resolution), LOD flags, events → toasts and sounds, wiring of every handler. |
| `renderer.ts` + `shaders.ts` | Two draw calls: a full-screen ground pass (terrain, terraforming, deposits, fog, power overlay, far-zoom building map, night) and one instanced sprite pass streamed from Wasm memory. World textures upload only when their revision counter in the stats block changes; fog uploads just the changed rectangle. |
| `icons.ts` | Draws every item and building with Canvas2D into a 16×8 atlas of 128 px cells at start-up (items at their id, buildings at 64 + kind). No image files besides the logo. |
| `hud.ts` | Objective and planet cards, action buttons, build bar, selection card, toasts. Updates at 5 Hz, only where values changed. |
| `panels.ts` | Sheets: research, Ark, planet, Core, menu, shop, achievements, journal, blueprints, probes, wrecks, inspector, and the guide/codex host. |
| `codex.ts` | Guide lessons and codex pages (text, recipe rows, links, demo slots). |
| `scenes.ts` | Demo scenes: item chains generated from the recipe tables, hand-made scenes for buildings and lessons. |
| `demo.ts` | The demo player: second engine instance, its own `Renderer` on a canvas inside the sheet, camera fit, off-screen supplies and sinks, labels, captions, live readouts. |
| `input.ts` + `camera.ts` | Touch, mouse and keyboard; pan/zoom camera in tile units. |
| `save.ts` `blueprints.ts` `settings.ts` | localStorage (`beltwise.save.v2`, `beltwise.cam`, `beltwise.blueprints`, `beltwise.settings`), file export/import, share codes (`BW1.` + base64). |
| `pwa.ts` + `web/sw.js` | Install prompt, fullscreen, service worker registration and update toast. |
| `bench.ts` | The in-game benchmark (builds a stress world, flies the camera, restores the save). |

## Build and deploy

`scripts/build.mjs`: `cargo build --release --target wasm32-unknown-unknown` → `wasm-opt -O3`
(feature flags must match what rustc 1.96 emits) → esbuild bundle → `index.html` with a
content-hash query → `sw.js` with the exact asset list → copy `web/public`. Dev mode writes
to `.dev/` and serves on port 8000; production writes `dist/`. CI
(`.github/workflows/deploy.yml`) runs fmt, clippy, tests, typecheck and build, then deploys
`dist/` to GitHub Pages on every push to `main`.

## Keep in sync

| Rust (source of truth) | Mirrored in |
| --- | --- |
| `lib.rs` `ABI_VERSION`, every `fx_*` signature | `constants.ts` `ABI_VERSION`, `engine.ts` `Exports` |
| `world.rs` `stat::*`, `STATS_LEN` | `constants.ts` `Stat`, `STATS_LEN` |
| `lib.rs` `fx_inspect` layout, `INFO_LEN` | `constants.ts` `Info`, `INFO_LEN` |
| `render.rs` `sprite::*` | `constants.ts` `Sprite`; numeric literals in `shaders.ts` (`128u`...`141u`) |
| `render.rs` `lod::*`, `cursor::*` | `constants.ts` `Lod`, `CursorFlag` |
| `world.rs` `reason::*` | `constants.ts` `Reason` and the messages in `main.ts` `reasonText` |
| `machines.rs` `status::*`, `flag::*` | `constants.ts` `Status`, `MachineFlag`; `panels.ts` `STATUS` labels |
| `explore.rs` `found::*` | `constants.ts` `Found` |
| `content.rs` building ids the UI special-cases | `constants.ts` `Kind` |
| `blueprint.rs` cell size | `constants.ts` `BP_CELL_BYTES` |
| `machines.rs` `accept` / `process` input and output faces | `render.rs` `takes_from` / `out_faces`; assumptions in `scenes.ts` |
| `content.rs` `BUILDING_COUNT`, deposit ids | `renderer.ts` `KINDS` (40) and `RESOURCES` (10), `u_kindColor[40]` / `u_resColor[10]` in `shaders.ts` |
| Item and building keys | `icons.ts` art tables, `codex.ts` notes and tips, `scenes.ts` lookups |
| `types.rs` `TICKS_PER_SEC` | `constants.ts` `TICK_MS`, `TICKS_PER_SEC` |

## Performance budget

Simulation cost scales with belt segments and machines, not tiles: blocked segments are
skipped and passive buildings never enter the tick loop. Rendering is two draw calls at
any factory size; the per-frame path allocates nothing on either side of the boundary.
Reference numbers (Apple M4, 32k belts, 43k items, 4k machines): about 0.2 ms of
simulation per frame and 0.2–0.6 ms of render CPU; natively about 33 µs per tick. Measure
with `npm run bench` (headless) or `?bench` (in the browser) before and after anything
that touches `tick`, `render`, belts, machines or shaders.
