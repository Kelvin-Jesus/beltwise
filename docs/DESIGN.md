# Beltwise: design notes (v2)

Guiding rule: every system must add a *decision* or a *spectacle*, never just busywork.
Complexity unlocks tier by tier, so a new player meets one idea at a time. All numbers
below live in `engine/src/content.rs`; this file explains the intent behind them.

## Progression

Two tracks run side by side, and both are always visible in the HUD:

- **The Ark** (factory track). Five phases paid from Core storage. Completing phase *n*
  unlocks research tier *n + 1* and visibly grows the structure around the Core (launch
  pad and towers, hull, beacons, habitat domes, and finally a rocket). Phase 5 launches the
  colony to a new planet.
- **Terraforming** (planet track). Terraformers raise Heat, Pressure, Oxygen and Biomass;
  their sum is the Terraform Index, which climbs through 12 stages. Stages change the world
  (ice retreats, clouds, lakes, rain, lichen, moss, grass, flowers and butterflies, forests,
  flocks of birds), pay a reward, and gate some research.

| Tier | Theme | Unlocked by | Target play time |
| --- | --- | --- | --- |
| 1 | Landing: ores, ingots, plates, wire, gears, steel, circuits, heaters, power poles, coal power | start | 0–35 min |
| 2 | Atmosphere: silicon, solar, reinforced plates, vaporizers, bio lab, water pumps | Ark 1 | 35–80 min |
| 3 | Chemistry: oil, plastic, fuel, computers, batteries, radar, drones, recycler, greenhouses | Ark 2 | 1.3–2.5 h |
| 4 | Titanium and life: frames, express belts, pollinator hives, hatcheries, repeatable research | Ark 3 | 2.5–3.5 h |
| 5 | Nuclear and stars: uranium, reactors, meteorite alloy, quantum cores | Ark 4 | 3.5–5 h |
| ∞ | Launch (prestige), repeatable research, endless goals | Ark 5 | — |

A 53-step objective chain walks through all of it, then endless goals take over.

## Anti-tedium decisions

- Fluids are items on belts (water, crude oil, fuel): one logistics system to master.
- **Drones** instead of trains: link two ports in the inspector, done.
- **Power** is forgiving: the Core powers everything within 20 tiles (40 MW), poles cover
  11×11 tiles and link automatically within 10, generators burn fuel only as fast as the
  grid draws power, and a short grid slows machines down instead of breaking anything.
- Machines pick their recipe from the first input (Auto) unless the player chooses one,
  in the build bar (before placing) or the inspector (after).
- Removing a building refunds it fully, including installed shards, amplifiers and stored
  goods. Undo covers every gesture. Blueprints copy whole modules, with recipes.
- Offline progress: deliveries and terraforming continue at the recorded rates while the
  game is closed (up to 8 hours), shown on return.
- Meteors never destroy anything; they only leave meteorite deposits behind.

## Systems

- **Power**: coal generator 60 MW, solar 8 MW × daylight, fuel generator 150 MW, reactor
  1.2 GW, battery bank 3 GJ at up to 150 MW. Networks are the connected poles (plus the Core);
  satisfaction = supply ÷ demand.
- **Overclock**: power shards add +50% speed each (up to 3; power × 1.69 / 2.46 / 3.29).
  Amplifiers double output at 4× power. Both come from wrecks, achievements and the shop.
- **Exploration**: fog of war. Buildings reveal around them (belts 3, machines 6, poles 9);
  radars reveal a 70-tile circle over ~35 s. 24 wrecks hold 10 story logs, 6 data probes
  (pick 1 of 3 alternate recipes each, 12 alternates in total), supply caches that get richer
  with distance, power shards and 2 amplifiers.
- **Deposits** have purity: impure 0.5×, normal 1×, pure 2× (richer further out).
- **Day/night**: an 8-minute day. Solar follows it; working buildings light up at night.
- **Weather and life**: snow while the air is thin, clouds, rain once there is water,
  lichen on bare rock, flowers and butterflies, and flocks of birds once wildlife thrives.
- **Meteor showers**: from Ark phase 2, every ~12 minutes (6 on Ember), announced 30 s ahead.
- **Economy**: the Recycler turns surplus into credits; the shop sells shards, amplifiers and
  cosmetics (belt colours, Core trims). Achievements (33) and every new stage also pay credits (and some, shards).
- **Stats**: per-item made/used/delivered per minute; the problem view (eye button) shows
  power coverage and badges on stuck machines.
- **Planets**: Glacia (start), Ember (warm start, scarce ice, lava highlands you can't build
  on, twice the meteors), Thalassa (frozen sea, few islands). Each launch: +10% production.

## Guide and live demos

- The guide has twelve short lessons; the codex has a page for every item (what it's for,
  every way to make it, every use, what it pays for) and every building (how to use it,
  recipes, cost, unlock). "What makes it" and "what uses it" are computed from the recipe
  tables, so new content documents itself.
- Any page plays a live demo on request. A second engine instance (the same compiled Wasm
  module, a 64x40 map) builds the scene with sandbox rules and the game's own renderer
  draws it into a canvas in the page: an item's maker belting it into a user, a building at
  work, or a lesson built step by step with captions. Supplies and sinks sit just past the
  edge of the view, so belts run in and out of frame like part of a larger factory.
- Demo maps copy the player's planet, stage and belt colour, so they look like their world.
  Slow recipes play faster (the speed shows in a corner); lakes appear only in scenes that
  need water. The demo only ticks while its page is open, and costs well under 0.1 ms a frame.
- Help is one tap away: the objective card opens the lesson or page for the current goal,
  items in the Core open their page, and building cards and the inspector have a **?**.

## Performance budget

Simulation stays O(segments + machines) per tick with no allocation; passive buildings
(poles, batteries, wrecks) are skipped by the tick loop. Rendering stays at two draw calls:
lights are additive sprites in the same instanced pass, weather is a few hundred sprites,
fog/power/deposits are textures in the ground pass, and fog updates upload only the changed
rectangle. Far zoom draws buildings from a tile texture. Target: 120 FPS on capable
devices, 60 on low-end ones, with a frame-rate cap and quality presets in the menu.
