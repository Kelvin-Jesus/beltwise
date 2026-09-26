# 0007. Power networks by union-find; shortages slow machines, never break them

- Status: Accepted
- Date: 2026-09-24

## Context

Power adds planning depth, but punishing failure modes (machines stopping dead, fuel
wasted, cascading blackouts) are tedious on a phone.

## Decision

- Networks are the connected components of the Core and power poles: a union-find over
  pole positions (bucketed for long lines), then one coverage fill. Rebuilt only after
  edits.
- The Core supplies 40 MW within 20 tiles; poles cover 5 tiles and link within 10.
- Each tick, a network's satisfaction is supply ÷ demand (capped at 1). Every consumer's
  work is scaled by it, so a short grid slows everything evenly. Nothing breaks.
- Generators burn fuel in proportion to load. Batteries are pooled per network and charge
  from surplus, discharging when supply falls short.

## Consequences

- One multiplication per machine per tick; the model is easy to explain in the UI
  (the power card, the overlay, the Core's Power tab).
- Battery charge is pooled per network, so it is redistributed to individual batteries
  before networks are rebuilt or saved (`sync_batteries`).
