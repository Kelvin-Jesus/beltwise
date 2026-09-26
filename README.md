<p align="center"><img src="design/logos/beltwise-banner.png" alt="Beltwise" width="720"></p>

<p align="center"><b><a href="https://kelvin-jesus.github.io/beltwise/">Play in your browser</a></b> · installs as an app · works offline</p>

# Beltwise

Build a factory on a frozen planet, then use it to bring the planet back to life.
Beltwise is a small, fast, touch-first automation game in the spirit of *Builderment*,
*Mindustry* and *The Planet Crafter*. It runs in any modern browser, installs as an app
and plays offline.

- **Factory:** 35 buildings turn 9 raw resources (including oil and meteorites) into 26 products, from iron plates to quantum cores. Pick recipes per machine, unlock alternates, overclock with power shards, double output with amplifiers.
- **Power:** the Core powers its surroundings; poles, coal, solar, fuel and nuclear generators and batteries carry the rest. A short grid slows machines down instead of breaking them.
- **The Ark:** a five-phase megaproject that unlocks each new tier and, finally, launches your colony to a new planet (Glacia, Ember, Thalassa) with a permanent production bonus.
- **Terraforming:** twelve stages of a planet coming back to life: ice retreats, clouds gather, lakes fill, rain and snow, lichen, moss, grass, flowers and butterflies, forests and birds. Every stage pays credits (and sometimes power shards).
- **Exploration:** fog of war, radar, and 24 wrecks of the first expedition with supplies, a story told in 10 logs, and data probes that unlock alternate recipes.
- **Tools that save time:** blueprints (copy, paste, rotate, share codes), drones for long hauls, per-item production stats, a power and problem overlay, full refunds and undo.
- **Learn by watching:** a guide of short lessons, and a codex page for every item and building: what it's for, what makes it, what uses it. Each one plays a live demo on request: the game builds the scene on a second, tiny engine instance and renders it right in the page. Tap the objective, an item in the Core, or the **?** on a building card.
- **Always a next step:** a 53-step objective chain, 33 achievements, a shop, repeatable research and endless goals. The factory keeps producing while the game is closed.
- **Offline PWA:** install it to your home screen and it runs fullscreen without a connection. Autosaves; you can export and import saves (older saves still load).
- **Fast:** a 32k-belt, 43k-item, 4.4k-machine factory with a live power grid costs about 0.2 ms of simulation per frame, and the renderer uses 2 draw calls per frame at any size. Frame-rate cap (30/60/120/Max) and quality presets in the menu; an in-game benchmark measures your device.

## Stack

| Layer | Choice | Why |
| --- | --- | --- |
| Simulation | **Rust → `wasm32-unknown-unknown`**, std only, no crates | Predictable, GC-free loops over flat arrays. All game content is data (`engine/src/content.rs`). |
| JS ↔ Wasm | **Raw C ABI** (`extern "C"`), no wasm-bindgen | Plain numbers across the boundary. Bulk data (sprite instances, stats, storage, maps, saves) is read in place through typed-array views. Zero imports. |
| Rendering | **Raw WebGL2** from a small TypeScript host | One instanced draw for every sprite plus one full-screen ground pass. All art is procedural: icons are drawn with Canvas2D into an atlas at start-up, and belts, the Core and terrain are shaders. There are no image files. |
| Offline | **Service worker** + web app manifest | Precaches exactly the current build's hashed assets. The page is network-first so updates arrive immediately. |
| Build | **cargo + wasm-opt + esbuild** | Binaryen and esbuild are pinned npm devDependencies, and CI uses the same versions. |

## Performance

The simulation is designed to cost almost nothing, whatever the factory size.

- **Belts are segments with gap encoding.** Consecutive belts form one segment that stores
  only the gaps between items. A tick advances each segment in O(1), and fully blocked
  segments are skipped. The update order is downstream-first, so hand-offs never lose a
  tick. Every belt speed (225, 300 or 450 items/min) divides the item spacing exactly, so
  compressed belts run at their exact rated throughput. T-junctions zipper-merge fairly.
- **Machines are data-driven.** One crafter implementation reads recipes from tables.
  Recipes are picked from the first input that arrives, so there's no recipe UI to manage.
  Machines live in a dense array.
- **No per-frame allocations.** The renderer uploads the instance buffer straight out of
  Wasm memory (`bufferSubData(…, wasmBytes, ptr, len)`) into triple-buffered VBOs.
- **Culling and level of detail.** Only visible tiles are scanned. Below 8 px per tile,
  items aren't drawn. Below 5 px per tile, belts and machines aren't drawn as sprites
  either: the ground pass paints them from a per-tile kind texture, so a zoomed-out frame
  costs the same for any factory size.
- **Fill rate.** Terrain noise comes from a tiling texture, not per-pixel hashing, and the
  drawing-buffer resolution steps down automatically if a device can't hold its frame rate.

**Benchmark** (Menu → *Run benchmark*, or open with `?bench`): it builds 32,100 belts, 43,100
items and 4,000 busy machines in a separate world, flies the camera through four shots, and
then restores your game.

| Shot (Apple M4, 2× DPR, 768×1024) | FPS | 1% low | Sim ms/frame | Render CPU ms | Sprites |
| --- | --- | --- | --- | --- | --- |
| Close-up: 4k busy machines | 60 | 54 | 0.16 | 0.18 | 394 |
| Mid zoom: 40k moving items | 60 | 54 | 0.18 | 0.58 | 7,901 |
| Dense items, zooming in | 60 | 54 | 0.20 | 0.52 | 6,359 |
| Whole map (far LOD) | 60 | 54 | 0.19 | 0.15 | 730 |

Headless: `npm run bench`. Natively, one simulation tick of that scene (power grid included)
takes about 33 µs.

## Controls

| | Touch | Mouse / keyboard |
| --- | --- | --- |
| Build | pick a building, tap or drag | left click / drag |
| Remove (full refund) | Delete tool | right drag, or `X` tool |
| Inspect, salvage, link drones | tap with *Move* | click with *Move* |
| Blueprint | *Copy*, drag over buildings, tap to paste | `B`, drag; `V` pastes the last one |
| Pan / zoom | two fingers | middle drag, `WASD`; wheel, `+` `-` |
| Rotate next building or blueprint | ⟳ button | `R` (`Shift+R` reverses) |
| Undo last gesture | ↶ button | `Z` / `Ctrl+Z` |
| Power and problem view | eye button | `O` |
| Tools | build bar | `1`–`9`, `Tab` switches category, `Q` move, `Esc` |
| Panels | top-right buttons | `T` research, `K` Ark, `C` core, `P` planet, `F` fullscreen, `F3` stats |
| Guide and codex | tap the objective, an item in the Core, or **?** on a building | `H` |

A touch only commits once it moves past a 10 px slop or lifts, so starting a pinch never
places a stray building.

## Develop

Prerequisites: [rustup](https://rustup.rs) and Node ≥ 20 (`mise.toml` pins Node 22, the
version CI uses, for [mise](https://mise.jdx.dev) users). The pinned Rust toolchain and
wasm target install themselves from `rust-toolchain.toml`.

```bash
npm install
npm run dev        # http://localhost:8000, rebuilds Rust and TS on save (HOST=0.0.0.0 for LAN)
npm test           # engine tests: belts, merges, tunnels, recipes, research, saves, undo
npm run bench      # headless benchmark
npm run build      # production build to dist/ (wasm-opt, hashed assets, service worker)
npm run preview    # serves dist/ under /beltwise/ like GitHub Pages; prints a LAN URL for phones
```

URL parameters: `?bench` runs the benchmark, `?seed=123` starts a different world,
`?dpr=1` forces the render resolution, `?notitle` skips the title screen.

Before changing the code, read [CLAUDE.md](CLAUDE.md) (commands, the checks CI runs,
invariants), [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) and
[docs/KNOWN-ISSUES.md](docs/KNOWN-ISSUES.md). They are written for coding agents and
people alike.

## Deploy to GitHub Pages

1. Push to a GitHub repository with `main` as the default branch.
2. In **Settings → Pages → Build and deployment**, set **Source** to **GitHub Actions**.
3. Push to `main`. [`.github/workflows/deploy.yml`](.github/workflows/deploy.yml) runs fmt,
   clippy, tests and typecheck, then builds and deploys `dist/`.

Every URL is relative, so the site works at `https://<user>.github.io/<repo>/` under any
repository name. Assets carry content hashes, and the engine checks its ABI version against
the UI at start-up. HTTPS (which Pages provides) is required for install and offline play.

## Layout

```
engine/src/
  content.rs     items, buildings, recipes, research, the Ark, objectives, achievements,
                 logs, shop, planets (+ JSON for the UI)
  belts.rs       segment-based belt transport
  machines.rs    every machine class, overclocking, per-item production counters
  power.rs       power networks: pole union-find, coverage, batteries
  core.rs        storage, research, the Ark, meters and stages, objectives, progress
  world.rs       grid, placement rules, tunnels, undo, retuning, tick, stats
  drones.rs  explore.rs  sky.rs  blueprint.rs  progress.rs
                 drone ports; fog, radar and wrecks; day/night and meteors;
                 blueprints; shop, launches and offline progress
  render.rs      culled, level-of-detail instance buffer (lights, weather, drones)
  save.rs        binary save format (versioned, validated, deterministic; loads v1)
  worldgen.rs    terrain fields, deposits with purity, wrecks
  demo.rs        blank sandbox maps for the guide's live demos
  lib.rs         the C ABI; tests.rs has 55 tests and a benchmark
web/src/
  main.ts        loop, LOD, frame pacing, events, blueprints, launch cinematic
  engine.ts      Wasm loading, views, save/load
  renderer.ts    WebGL2 passes and textures;  shaders.ts  GLSL
  icons.ts       procedural Canvas2D art for every item and building
  hud.ts         cards, build bar, selection card;  panels.ts  every sheet
  codex.ts       the guide's lessons and the codex pages
  scenes.ts      demo scenes, built from the recipe tables;  demo.ts  plays them live
  input.ts       touch / mouse / keyboard;  camera.ts  settings.ts  title.ts
  blueprints.ts  save.ts  audio.ts  pwa.ts  bench.ts
web/sw.js        service worker template;  web/public/  icons, logo and manifest
design/logos/    the logo sheet and make-icons.sh, which builds every icon from it
docs/            DESIGN.md (design intent), ARCHITECTURE.md (how it fits together),
                 KNOWN-ISSUES.md (open problems and pitfalls), adr/ (decision records)
CLAUDE.md        guide for coding agents; .claude/ holds their settings and skills
```

## Adding content

Add the item or building to `content.rs`: its id, its name, and recipes or costs. Hook it
into the research tree and give it an icon function in `icons.ts`. The UI (build bar,
research, inspector, core storage, codex and its demos) picks it up from the engine's JSON
automatically; add a line about it to `ITEM_NOTES` or `BUILDING_TIPS` in `codex.ts`.
`content_tables_are_consistent` checks the tables for mistakes.

## Roadmap ideas

- An optional hostile mode (meteors that damage, shields), a minimap, cloud saves.
- More life on screen (butterflies around hives, fish in stocked lakes), more planets.
