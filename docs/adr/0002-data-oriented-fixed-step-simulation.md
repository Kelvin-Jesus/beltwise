# 0002. Data-oriented world, fixed 60 Hz steps, interpolated rendering

- Status: Accepted
- Date: 2026-09-24

## Context

Displays run at 60, 90, 120 or 144 Hz, and phones throttle. The simulation has to give the
same results at any frame rate (and for offline catch-up and tests), while motion must look
smooth at the display's rate.

## Decision

- The world is structure-of-arrays: one flat array per tile attribute (`kind`, `dir`,
  `res`, `purity`, `ent`, `seen`) and one dense, swap-removed `Vec<Machine>`.
- The simulation advances in fixed 1/60 s ticks (`fx_tick(n)`), at most 8 per frame; time
  beyond that is dropped rather than spiralling.
- Rendering interpolates moving things between the last two ticks with `alpha`.
- Every machine is 1×1 and outputs through its front face; buildings that never act per
  tick (poles, batteries, radars, incinerators, wrecks) are listed in `PASSIVE` and
  skipped by the tick loop.
- Frame pacing (`FramePacer`) handles the 30/60/120/Max caps independently of the
  simulation clock.

## Consequences

- Deterministic simulation: tests, saves and offline progress don't depend on frame rate.
- Linear scans over flat arrays keep the tick cheap (about 33 µs natively for the
  benchmark factory).
- Machine indices move on removal; the grid's back-reference must be fixed by the caller.
- New per-tick behaviour must not be put on a `PASSIVE` class.
