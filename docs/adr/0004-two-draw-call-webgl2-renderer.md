# 0004. Raw WebGL2, two draw calls, sprites streamed from Wasm memory, procedural art

- Status: Accepted
- Date: 2026-09-24

## Context

The renderer must hold 60+ FPS on low-end phones with thousands of visible sprites, stay
tiny, and work offline. Draw-call count and fill rate are the usual limits on mobile GPUs.

## Decision

- Raw WebGL2 from a small TypeScript host; no engine or framework.
- Two draw calls per frame: a full-screen ground pass (terrain, terraforming, deposits,
  fog, power overlay, far-zoom building map, night) and one instanced pass for every
  sprite.
- The engine writes 16-byte sprite records into a fixed-capacity buffer in Wasm memory;
  JS uploads it with one `bufferSubData` into triple-buffered VBOs.
- Level of detail: below 8 px per tile items are skipped; below 5 px per tile buildings
  are painted by the ground pass from a per-tile kind texture, so zoomed-out frames cost
  the same for any factory.
- World textures re-upload only when their revision counter changes; fog uploads only the
  changed rectangle. Resolution steps down automatically when frames run late.
- All art is procedural: items and buildings are drawn with Canvas2D into an atlas at
  start-up; belts, the Core and effects are signed-distance shapes in the shader. The only
  image files are the logo and app icons.

## Consequences

- Rendering cost is flat with factory size; no image assets to download or cache.
- Sprite ids, atlas layout and uniform array sizes are hard-coded on both sides
  (`BUILDING_COUNT ≤ 40`, `ITEM_COUNT ≤ 64`): adding content can hit these limits.
- New visual kinds need shader code, not just images.
