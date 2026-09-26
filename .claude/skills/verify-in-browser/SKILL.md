---
name: verify-in-browser
description: Launch Beltwise and verify a change in the real game in the browser pane — start the dev server, drive the game through the window.fx dev handle, build a test layout with sandbox cheats, screenshot on desktop and phone sizes, sweep every guide demo, read the perf overlay, clean up the dev save, and check a deploy on the live site. Use for anything visible (rendering, UI, sheets, demos, input) and before pushing.
---

# Verifying Beltwise in the browser

Engine logic is covered by `cargo test`; everything on screen is verified by looking.

## Start

1. `preview_start` with name `dev` (from `.claude/launch.json`): http://localhost:8000,
   rebuilding Rust and TypeScript on save. Use `preview` (port 4173, `/beltwise/`) to test
   a production build (`npm run build` first) or the service worker.
2. Open `http://localhost:8000/?notitle` (skips the title screen). After an engine change
   wait for "rebuilt engine.wasm" in the server logs (about 4 s), then reload.
3. Check the console for errors (`read_console_messages` with `onlyErrors`).

## Drive it

Dev builds expose `window.fx = { engine, cam, renderer, input, hud, sheets, library, demo }`.

```js
// A test layout next to the Core, free and unlocked, then look at it closely.
const { engine: e, cam } = window.fx; const x = e.x; const s = e.stats;
const cx = s[30], cy = s[31];               // Stat.CoreX / CoreY
x.fx_sandbox(1); x.fx_edit_begin();          // one undo group for everything below
x.fx_demo_ore(cx - 9, cy, 1, 1);             // iron ore deposit (sandbox only)
x.fx_place(cx - 9, cy, 3, 0);                // drill facing east (kinds: content.rs bk)
for (let i = 8; i >= 1; i--) x.fx_place(cx - i, cy, 1, 0); // belt into the Core
x.fx_tick(600);                              // ten seconds of simulation
cam.x = cx - 4; cam.y = cy + 0.5; cam.zoom = 70; // px per tile; 130+ for close-ups
```

- Sheets: `fx.sheets.open('research')`, `fx.sheets.openCodex({ view: 'item', id: 14 })`
  (views: `guide`, `topic` + id, `codex`, `item`, `building`), `fx.sheets.close()`.
- Late-game content: `x.fx_sandbox_grant(5000, 4)` (items + Ark phases),
  `x.fx_sandbox_meters(heat, pressure, oxygen, biomass)` (terraforming look).
- Perf: `fx.hud.togglePerf()` (F3) shows fps, sim/build/GPU ms, sprites and items.

## Look at it

- Desktop screenshot, then `resize_window` preset `mobile` (375×812), reload, repeat.
  Reset with preset `desktop` when done.
- Night dims the world in the dev save; judge colours in the guide demos (always noon) or
  after the in-game day comes round.
- Rendering changes: check straight belts, corners, side-loads, belts into machines and
  the Core, and far zoom (`cam.zoom = 4`).

## Sweep every guide demo

After touching `scenes.ts`, `demo.ts`, `codex.ts`, content or rendering, play them all:

```js
const S = fx.sheets, c = fx.engine.content, errors = [];
addEventListener('error', (ev) => errors.push(String(ev.message)));
let played = 0;
async function playAll(page) {
  S.openCodex(page);
  for (const b of document.querySelectorAll('#sheet-body [data-act^="demo:"]')) {
    if (/demo:(replay|stop)/.test(b.dataset.act)) continue;
    b.click(); await new Promise((r) => setTimeout(r, 60)); played++;
    if (!fx.demo.playing) errors.push('not playing: ' + b.dataset.act);
  }
}
for (let i = 1; i < c.items.length; i++) await playAll({ view: 'item', id: i });
for (let k = 2; k < c.buildings.length; k++) if (c.buildings[k].category !== 255 || k === 2) await playAll({ view: 'building', id: k });
for (const t of ['start', 'belts', 'machines', 'logistics', 'power', 'research', 'terraform', 'explore', 'boost', 'drones']) await playAll({ view: 'topic', id: t });
S.close(); ({ played, errors });
```

Expect about 237 demos and no errors. To check that a scene really works, tick its engine
(`fx.demo.engine.x.fx_tick(900)`) and read `fx.demo.engine.stats` / `.rates`: items on
belts (`stats[24]`) and production should be non-zero, except for pure logistics scenes.
A machine reported as blocked is usually just backpressure.

## Clean up

- `x.fx_undo()` reverts the test layout (one undo group), then `x.fx_sandbox(0)`.
- The dev server's save lives in localhost's localStorage, separate from the live game.

## After pushing

```bash
gh run list --limit 1
gh run watch <id> --exit-status
```

Then open https://kelvin-jesus.github.io/beltwise/?notitle and confirm the new build is
served (the `main.js?v=` hash in the page matches the one `npm run build` printed); play
whatever changed.
