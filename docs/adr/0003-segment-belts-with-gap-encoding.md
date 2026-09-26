# 0003. Belts are segments that store the gaps between items

- Status: Accepted
- Date: 2026-09-24

## Context

Belts dominate a factory's entity count: the benchmark has 32k belt tiles carrying 43k
items. Updating every item every tick is too slow on phones.

## Decision

- Consecutive belt tiles form one segment; a segment stores its items as the gaps between
  them (positions in `SUB` = 96 sub-units per tile, at least `MIN_GAP` = 48 apart).
  Advancing a segment is O(1): shrink the front gap.
- Fully blocked segments are skipped. Segments update downstream-first, so a hand-off
  never loses a tick.
- Belt speeds (3, 4, 6 sub-units per tick, i.e. 225, 300, 450 items/min) divide the item
  spacing exactly, so compressed belts run at their rated throughput.
- Side-loads zipper-merge: at a T-junction the side that has waited longer goes first.
- Topology is rebuilt lazily after edits, never per tick.

## Consequences

- Cost scales with segments, not items or tiles.
- Rendering walks only segments whose bounds touch the view.
- Belt code is subtle (segment splits at long chains, corners, tunnels); `tests.rs` has
  targeted tests for throughput, merges, compression, loops and item positions across
  rebuilds. Extend them when touching `belts.rs`.
