# 0005. All game content is data in `content.rs`, sent to the UI as JSON

- Status: Accepted
- Date: 2026-09-24

## Context

Items, buildings, recipes, research, stages, objectives and the rest change often while
balancing. Duplicating them in Rust and TypeScript would drift.

## Decision

- `engine/src/content.rs` is the single source of truth: plain `const` tables read directly
  by the simulation.
- `content_json()` serializes the same tables; the UI parses them once at start-up and
  builds the build bar, research tree, inspector, codex and demo scenes from them.
- Behaviour is data-driven: one crafter implementation runs every recipe; building
  classes (`Class`) select behaviour; research effects are an `Effect` list.
- `content_tables_are_consistent` (in `tests.rs`) validates the tables: acyclic research
  prerequisites and tiers, recipe limits (at most 3 inputs) and unlocks, every alternate
  used exactly once, objective and stage references, and well-formed JSON.

## Consequences

- Balancing is a one-file change, and the UI and the codex document new content by
  themselves.
- Only presentation lives in TypeScript: icon art (`icons.ts`), codex text (`codex.ts`),
  demo scene lookups (`scenes.ts`). New content needs entries there too (skill
  `add-content`).
- Ids are part of the save format (ADR 0006): tables are append-only.
