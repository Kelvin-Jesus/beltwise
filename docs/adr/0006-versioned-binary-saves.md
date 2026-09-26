# 0006. Versioned binary saves that store only what can't be regenerated

- Status: Accepted
- Date: 2026-09-24

## Context

Saves go to localStorage (limited, synchronous) on every autosave, on tab hide and on page
exit, and can be exported as files. They must be small, fast to write, and keep loading
after the game changes.

## Decision

- A little-endian binary snapshot with magic `BWSV` and a `u16` version (now 2), written
  and read in `save.rs`; the web side stores it as base64 (`beltwise.save.v2`).
- Only non-derivable state is stored. Terrain, deposits and wrecks are regenerated from the
  seed and patched (meteorite deposits added, salvaged wrecks removed); belt segments,
  power networks and research tuning are rebuilt on load.
- Old versions keep loading through explicit migration code (v1: item ids from 8 up shift
  by two, timers are rescaled, finished objectives fast-forward).
- Loading validates as it reads and leaves the current world untouched on failure.

## Consequences

- Saves stay small, and worldgen must stay deterministic for a given seed and planet.
- Item, building and tech ids can never be renumbered; new ones are appended. Per-item
  arrays and the repeatable levels are written without a length, so appending an item or
  a repeatable research is itself a format change.
- Any change to what is stored bumps `VERSION`, adds a migration, and extends the
  round-trip and old-version tests in `tests.rs`.
