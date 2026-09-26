---
name: add-content
description: Checklist for adding or changing Beltwise game content (items, buildings, recipes, alternates, research, objectives, stages, achievements, logs, shop entries, planets) and every file that must change with it, from engine tables and save compatibility to icons, codex text, demo scenes, tests and docs. Use whenever engine/src/content.rs changes or a new mechanic needs a building.
---

# Adding game content

`engine/src/content.rs` is the single source of truth (ADR 0005). The UI reads it as JSON,
so most of the UI updates itself, but art, codex text, save compatibility and a few
mirrors do not. Work through the relevant section, then the "Always" list.

## Ground rules

- Ids are saved: **append only**, never renumber, reorder or reuse.
- Limits: `BUILDING_COUNT ≤ 40` (38 used), `ITEM_COUNT ≤ 64`, research ≤ 64 nodes,
  achievements ≤ 64, alternates ≤ 32, recipes ≤ 3 inputs, deposits are items 1–9 (all
  used), Ark phases ≤ 4 cost entries. Raising a limit is an engine and shader change, not
  a content change: see docs/KNOWN-ISSUES.md → Limits.
- UI text is English, short, and says what the thing is for.

## A new item

1. `content.rs`: a constant in `mod it` (next free id), bump `ITEM_COUNT`, add an
   `item(key, name, value)` to `ITEMS` (`value` = credits a Recycler pays).
2. **Save format change.** Per-item arrays are saved without a length: bump `VERSION` in
   `save.rs` and teach `load` to read the previous version's `ITEM_COUNT` (see how
   `load_v1` uses `V1_ITEMS`). Add a test that a save written before the change still loads.
3. Recipes that make and use it (see "A recipe"); give it at least one use.
4. `web/src/icons.ts`: an entry in `ITEM_ART` (Canvas2D, drawn in a 128 px cell centred on
   the origin; look at neighbours for style).
5. `web/src/codex.ts`: a sentence in `ITEM_NOTES` and the key in one of `ITEM_GROUPS`.
6. A new raw resource can't be a deposit (ids 1–9 are taken) without widening
   `RESOURCE_COUNT`, `RESOURCES` and `RESOURCE_COLORS` in `renderer.ts`, `u_resColor` in
   `shaders.ts`, worldgen placement and `DEPOSIT_NOTES`.

## A new building

1. `content.rs`: a constant in `mod bk` (next free id, after `WRECK` = 37), bump
   `BUILDING_COUNT`, and a `BuildingDef` in `BUILDINGS`: `key`, `name`, one-line `desc`,
   `class`, `category` (build-bar tab), `cost`, `recipes`, `power` (kW), `research`
   (tech id or `FREE`), plus `deposits` / `fuel` / `store` / `meter` + `points` as the
   class needs.
2. Unlock it: `Effect::Unlock(bk::NEW)` on a research node (or `FREE`).
3. `icons.ts`: an entry in `BUILDING_ART` (drawn on top of the shared `body()`; output
   classes get the arrow automatically).
4. `codex.ts`: a tip in `BUILDING_TIPS` saying how to use it (crafters and terraformers
   fall back to a generic tip).
5. If the UI needs to special-case it, add it to `Kind` in `constants.ts`.

**A new class** (new behaviour, not just new recipes) also needs:
- `Class` variant and its `key()`; `Machines::accept` / `process` / `emitted` /
  `cycle_ticks` / `configure`; `PASSIVE` if it never acts per tick.
- `render.rs`: `takes_from` / `out_faces` (belt stubs) and any badge; the inspector in
  `panels.ts` (`inspect()`); placement or removal rules in `world.rs`.
- `save.rs` if the machine carries new state; `blueprint.rs` if that state should copy.
- A demo in `scenes.ts` → `building()` (every placeable building must have one).
- Engine tests for the behaviour in `tests.rs`.

## A recipe

- Standard: `r(inputs, output, ticks)` (free) or `rt(..., tech)` (unlocked by research).
  Alternates: `alt(name, ..., k)` with the next alternate index; bump `ALT_COUNT`. Every
  alternate index must be used exactly once (the consistency test checks).
- On Auto, a machine takes the first unlocked standard recipe that uses the first input to
  arrive, so recipe order within a building matters.
- `ticks` at 60 per second; slow recipes play faster in demos automatically.

## Research, objectives, stages, achievements, logs, shop

- Research (`TECH`, `mod tech`): append new nodes after the repeatables; keep `requires`
  acyclic, `tier` 1–5 (tier n needs n − 1 Ark phases), `stage` for terraforming gates.
  Repeatables must stay contiguous from `REPEAT_FIRST` (their level index is
  `t - REPEAT_FIRST`), so a new *repeatable* also bumps `REPEAT_COUNT`, changes the save
  layout (`levels`) and needs a wider `stat::LEVELS` block (mirror in `constants.ts`).
- Objectives: `OBJECTIVES` is a fixed-size array (update its length). Players' progress is
  an index, so append; inserting shifts everyone's current objective.
- Achievements: append (saved as a bit set). Stages: thresholds in `STAGES` and
  `mod stage`; rewards are paid when crossed. Logs are read in order (saved as a count).

## Always

1. `cargo test -p engine --locked` (includes `content_tables_are_consistent` and the save
   round trips), clippy, `npm run typecheck`, `npm run build`.
2. In the browser (skill `verify-in-browser`): it appears in the build bar or research,
   its codex page reads well, and every demo on that page plays (run the demo sweep).
3. Update README (feature counts such as "35 buildings", "26 products") and DESIGN.md
   numbers if they changed.
