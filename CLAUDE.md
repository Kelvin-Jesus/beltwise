# Beltwise: agent guide

Beltwise is a touch-first factory automation and terraforming game that runs in the browser
and installs as an offline PWA. The simulation is Rust compiled to WebAssembly
(`engine/`); a small TypeScript host (`web/`) renders it with raw WebGL2 and draws the UI.
Live at https://kelvin-jesus.github.io/beltwise/; repo `Kelvin-Jesus/beltwise`.

Read before changing things: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) (how it fits
together, what must stay in sync), [docs/KNOWN-ISSUES.md](docs/KNOWN-ISSUES.md) (open
problems and pitfalls already hit), [docs/adr/](docs/adr/) (why it is built this way),
[docs/DESIGN.md](docs/DESIGN.md) (game design intent and numbers).

## Working with the owner

- The owner writes in Portuguese: reply in Portuguese. Code, comments, docs, commit
  messages and all in-game text are English.
- Commit and push verified work to `main` as you go. Pushing to `main` deploys to GitHub
  Pages, so only push what passes the checks below, then watch the run
  (`gh run list --limit 1`, `gh run watch <id> --exit-status`).
- Pure refactors are welcome only when they serve the task; keep diffs focused.

## Commands

| Task | Command |
| --- | --- |
| Dev server (rebuilds Rust and TS on save) | `npm run dev` → http://localhost:8000 (`HOST=0.0.0.0` for LAN) |
| Production build to `dist/` | `npm run build` |
| Serve `dist/` like Pages (`/beltwise/`) | `npm run preview` → http://localhost:4173/beltwise/ |
| Engine tests (55 + 1 ignored benchmark) | `npm test` (= `cargo test -p engine`) |
| Headless benchmark | `npm run bench` |
| Typecheck | `npm run typecheck` |
| Format / lint Rust | `cargo fmt --all`, `cargo clippy -p engine --all-targets --locked -- -D warnings` |

In the desktop app, start servers with the browser pane's `preview_start` and the names in
`.claude/launch.json` (`dev`, `preview`) instead of running them in a shell.

## Definition of done

Run what CI runs (`.github/workflows/deploy.yml`); all must pass:

```bash
cargo fmt --all --check
cargo clippy -p engine --all-targets --locked -- -D warnings
cargo test -p engine --locked
npm run typecheck
npm run build
```

Then, for anything visible, look at it in the browser pane on desktop and at 375×812
(skill `verify-in-browser`). Update the docs the change touches: README (features, test
count, layout), DESIGN.md (numbers), an ADR for a new architectural decision, and
KNOWN-ISSUES.md for anything found but not fixed. Commit messages: imperative subject
under ~72 characters, a body that says why.

## Map

```
engine/src/   content.rs (all game data) · world.rs (grid, placement, tick) · belts.rs
              machines.rs · power.rs · core.rs · render.rs (instance buffer) · save.rs
              demo.rs (guide demo maps) · lib.rs (the C ABI) · tests.rs
web/src/      main.ts (loop) · engine.ts (Wasm + views) · renderer.ts + shaders.ts
              hud.ts · panels.ts (sheets) · codex.ts + scenes.ts + demo.ts (guide)
              icons.ts (procedural art) · input.ts · save.ts · pwa.ts · bench.ts
web/index.html  all CSS and the HUD markup;  web/sw.js  service worker template
scripts/      build.mjs (cargo → wasm-opt → esbuild → hashed assets → sw.js), preview.mjs
```

## Invariants (break these and things fail far from the change)

1. **The ABI.** Adding or changing an export: bump `ABI_VERSION` in `engine/src/lib.rs`
   and `web/src/constants.ts`, and declare it in `Exports` in `web/src/engine.ts`.
2. **Mirrored tables.** The stats block, inspector layout, sprite ids, LOD flags, reason
   and status codes and building kinds are defined in Rust and mirrored in
   `constants.ts` (sprite ids also as literals in `shaders.ts`). Change both sides; the
   full list is in ARCHITECTURE.md → "Keep in sync".
3. **Saves.** Item, building and tech ids are part of the save format: append, never
   renumber or reorder. Per-item arrays are written without a length, so even appending
   an item (or a repeatable research) changes the layout: bump `VERSION` in `save.rs`,
   keep the previous version loading, and extend the save tests.
4. **Hot paths allocate nothing:** `World::tick`, `World::render` and the `main.ts` frame
   loop. Reuse scratch buffers kept on `World`; the instance buffer never grows.
5. **Machine IO rules** live in `Machines::accept`/`process`; `render.rs`
   (`takes_from`, `out_faces`) mirrors them for belt stubs and `scenes.ts` assumes them.
6. **Hard limits:** `BUILDING_COUNT ≤ 40` (far-zoom shader uniform), `ITEM_COUNT ≤ 64`
   (atlas), deposits are items 1–9, 65,536 sprite instances per frame.
7. **Content is data.** New items and buildings go in `content.rs`; the UI reads them from
   the engine's JSON. Follow skill `add-content` for everything else to touch.
8. **Typed-array views over Wasm memory go stale** when memory grows or the world is
   replaced: read through `engine.stats` / `storage` / `rates` (they refresh), never cache.
9. **Sheets rebuild their HTML** (every 500 ms when it changes, and after actions). Live
   DOM inside a sheet must be re-attached after a render, as `DemoPlayer.attach` does.
10. **Relative URLs only**, so the site works under any Pages sub-path.

## Debugging

- Dev builds expose `window.fx` = `{ engine, cam, renderer, input, hud, sheets, library,
  demo }`. `fx.engine.x` is the raw Wasm API: `fx_sandbox(1)` (free, unlocked building),
  `fx_sandbox_grant(n, ark)`, `fx_sandbox_meters(heat, pressure, oxygen, biomass)`.
- URL parameters: `?notitle` skips the title, `?seed=123`, `?dpr=1`, `?bench`.
- F3 (or `fx.hud.togglePerf()`) shows fps, sim/build/GPU ms, sprites and items.
- The dev server saves to its own origin's localStorage (`beltwise.save.v2`): testing
  there never touches the live game. Undo test layouts and `fx_sandbox(0)` afterwards.
- Engine tests build worlds with helpers in `tests.rs`: `world()` (sandbox, free power),
  `campaign()` (costs and research apply), `powered()` (real power grid).
