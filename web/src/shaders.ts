// Shaders. Items and buildings come from the procedural atlas (icons.ts); belts, the Core
// and Ark, weather, lights and overlays are signed-distance shapes; the ground is a
// full-screen pass that paints the terrain from the world's elevation/moisture fields,
// animates terraforming, and applies fog of war, the power overlay and nightfall.

/** Shared helpers. */
const common = /* glsl */ `
float cover(float d, float px) { return clamp(0.5 - d / px, 0.0, 1.0); }
float sdCircle(vec2 p, float r) { return length(p) - r; }
float sdBox(vec2 p, vec2 b) {
  vec2 d = abs(p) - b;
  return length(max(d, 0.0)) + min(max(d.x, d.y), 0.0);
}
float sdRoundBox(vec2 p, vec2 b, float r) { return sdBox(p, b - r) - r; }
float sdSegment(vec2 p, vec2 a, vec2 b) {
  vec2 pa = p - a, ba = b - a;
  float h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
  return length(pa - ba * h);
}
// Atlas cell 'cell' (16x8 grid), local coords p in [-0.5, 0.5]^2.
vec4 atlas(sampler2D tex, uint cell, vec2 p) {
  vec2 c = vec2(float(cell % 16u), float(cell / 16u));
  return texture(tex, (c + clamp(p, -0.5, 0.5) + 0.5) * vec2(0.0625, 0.125));
}
uint ihash(uvec2 v) {
  uint h = v.x * 0x27d4eb2du ^ v.y * 0x165667b1u;
  h ^= h >> 15; h *= 0x2c1b3c6du; h ^= h >> 12;
  return h;
}
float hash01(vec2 p) { return float(ihash(uvec2(ivec2(floor(p)) + 32768)) & 0xffffu) / 65535.0; }
`;

// ---- Ground pass ---------------------------------------------------------------------------

export const groundVS = /* glsl */ `#version 300 es
void main() {
  vec2 p = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2));
  gl_Position = vec4(p * 2.0 - 1.0, 0.0, 1.0);
}`;

export const groundFS = /* glsl */ `#version 300 es
precision highp float;
precision highp int;
precision highp usampler2D;
uniform vec2 u_viewport;   // device pixels
uniform vec3 u_cam;        // center x, center y (tiles), zoom (device px per tile)
uniform ivec2 u_world;
uniform usampler2D u_res;  // deposit item id (r) and purity (g) per tile
uniform usampler2D u_kinds;// building kind per tile (far-zoom map)
uniform usampler2D u_power;// 1 where a power network reaches
uniform sampler2D u_terrain; // elevation, moisture, detail (bilinear)
uniform sampler2D u_fog;   // explored = 1 (bilinear, for soft edges)
uniform sampler2D u_atlas;
uniform sampler2D u_noise; // tiling value noise: 16 lattice cells per repeat; rgb smooth, a random
uniform vec4 u_terra;      // heat, water, air, life in 0..1
uniform float u_time;
uniform float u_map;       // 1 = draw buildings from u_kinds (far zoom)
uniform float u_fx;        // effect level: 2 full, 1 reduced, 0 minimal
uniform float u_overlay;   // power coverage overlay strength
uniform int u_planet;      // 0 Glacia, 1 Ember, 2 Thalassa
uniform vec3 u_ambient;    // daylight tint
uniform vec3 u_kindColor[40];
uniform vec3 u_resColor[10];
out vec4 outColor;
${common}
// One texture read instead of a dozen hashes per pixel (fill rate matters on phones).
float tnoise(vec2 p) { return texture(u_noise, p * 0.0625).r; }

vec3 terrain(vec2 world, float px) {
  vec4 T = texture(u_terrain, world / vec2(u_world));
  float d = T.b, m = T.g;
  bool fx1 = u_fx > 0.5, fx2 = u_fx > 1.5;
  // A little low-frequency wobble hides the texel grid in coastlines and ice edges.
  float e = T.r + (fx1 ? (tnoise(world * 0.7) - 0.5) * 0.06 : 0.0);
  float zoomFade = clamp(u_cam.z / 24.0, 0.0, 1.0);
  float grain = fx1 ? (texture(u_noise, world * 0.14).g - 0.5) * 0.07 * zoomFade : 0.0;
  // Bare rock, by planet: rusty (Glacia), basalt (Ember), slate (Thalassa).
  vec3 lo = u_planet == 1 ? vec3(0.13, 0.11, 0.11) : u_planet == 2 ? vec3(0.20, 0.23, 0.26) : vec3(0.26, 0.20, 0.18);
  vec3 hi = u_planet == 1 ? vec3(0.30, 0.24, 0.21) : u_planet == 2 ? vec3(0.36, 0.40, 0.43) : vec3(0.40, 0.34, 0.30);
  vec3 col = mix(lo, hi, smoothstep(0.25, 0.75, d));
  col *= 0.8 + T.r * 0.45;
  col += grain;
  float heat = sqrt(u_terra.x);
  float ice = 0.0;
  if (u_planet == 1) {
    // Ember: the highlands are molten; the crust glows through its cracks.
    float lava = smoothstep(0.735, 0.745, T.r);
    float crack = fx1 ? smoothstep(0.58, 0.72, tnoise(world * 0.8 + vec2(u_time * 0.03, 0.0)) * 0.7 + tnoise(world * 2.3) * 0.3) : 0.5;
    vec3 molten = mix(vec3(0.16, 0.05, 0.03), vec3(1.0, 0.42, 0.08), crack);
    molten += fx2 ? 0.08 * sin(u_time * 2.0 + world.x * 0.7 + world.y * 0.5) : 0.0;
    col = mix(col, molten, lava);
  } else {
    // Ice caps (and Thalassa's frozen sea) retreat as the planet warms.
    float iceLine = mix(0.58, 1.08, heat);
    ice = smoothstep(iceLine, iceLine + 0.04, e + (d - 0.5) * 0.1);
    if (u_planet == 2) ice = max(ice, smoothstep(0.02, -0.02, e - mix(0.34, -0.2, heat)));
    float sparkle = fx2 ? step(0.97, texture(u_noise, world * 0.047).a) * 0.12 * zoomFade : 0.0;
    col = mix(col, vec3(0.70, 0.80, 0.90) + grain + sparkle, ice * 0.92);
  }
  // Water fills the basins as the atmosphere thickens.
  float wl = mix(-0.08, 0.42, u_terra.y);
  float depth = wl - e;
  float water = smoothstep(-0.005, 0.015, depth) * (u_planet == 1 ? 1.0 - smoothstep(0.72, 0.74, T.r) : 1.0);
  vec3 wcol = mix(vec3(0.14, 0.40, 0.56), vec3(0.05, 0.18, 0.34), clamp(depth * 6.0, 0.0, 1.0));
  if (fx2) wcol += 0.04 * sin(u_time * 1.4 + (world.x * 0.9 + world.y * 0.6)) * (1.0 - clamp(depth * 8.0, 0.0, 1.0));
  col = mix(col, wcol, water);
  // Life spreads over damp ground, then forests appear.
  float life = u_terra.w;
  float fertile = m * 0.7 + d * 0.2 + (1.0 - abs(e - 0.45) * 1.6) * 0.2;
  float veg = smoothstep(1.05 - life, 1.12 - life, fertile) * smoothstep(0.0, 0.06, life) * (1.0 - water) * (1.0 - ice);
  if (u_planet == 1) veg *= 1.0 - smoothstep(0.7, 0.735, T.r);
  vec3 grass = mix(vec3(0.18, 0.36, 0.14), vec3(0.36, 0.56, 0.22), d) + grain;
  col = mix(col, grass, veg);
  if (fx1 && life > 0.55 && veg > 0.5) {
    vec2 cell = floor(world * 2.0);
    vec2 o = vec2(hash01(cell + 17.0), hash01(cell + 41.0)) * 0.5 + 0.25;
    float tree = cover(length(fract(world * 2.0) - o) - 0.22, px * 2.0);
    float forest = step(1.0 - (life - 0.55) * 1.6, hash01(cell)) * zoomFade;
    col = mix(col, vec3(0.10, 0.27, 0.12), tree * forest * 0.9);
  }
  // Atmosphere: a dusty cast that clears as the air fills with oxygen.
  float air = u_terra.z;
  col = mix(col * vec3(1.07, 0.94, 0.86), col * vec3(0.96, 1.0, 1.04), air);
  // Cloud shadows once there is an atmosphere to hold them.
  if (fx2) {
    float cloud = smoothstep(0.55, 0.85, texture(u_noise, world * 0.0022 + vec2(u_time, u_time * 0.55) * 0.0013).b);
    col *= 1.0 - cloud * u_terra.y * 0.22;
  }
  return col;
}

void main() {
  vec2 frag = gl_FragCoord.xy;
  vec2 world = vec2(u_cam.x + (frag.x - 0.5 * u_viewport.x) / u_cam.z,
                    u_cam.y - (frag.y - 0.5 * u_viewport.y) / u_cam.z);
  float px = 1.0 / u_cam.z;
  ivec2 tile = ivec2(floor(world));
  if (any(lessThan(tile, ivec2(0))) || any(greaterThanEqual(tile, u_world))) {
    float h = step(0.5, fract((world.x + world.y) * 0.125));
    outColor = vec4(mix(vec3(0.05, 0.055, 0.07), vec3(0.06, 0.066, 0.08), h) * u_ambient, 1.0);
    return;
  }
  vec2 f = fract(world) - 0.5;
  vec3 c = terrain(world, px);
  float fog = texture(u_fog, world / vec2(u_world)).r;
  if (u_fx > 0.5) fog = smoothstep(0.25, 0.75, fog + (tnoise(world * 1.3) - 0.5) * 0.35);

  uvec2 rp = texelFetch(u_res, tile, 0).rg;
  if (rp.x != 0u && fog > 0.5) {
    // Colour-coded pad so deposits read at a glance, even zoomed out. Pure deposits get a
    // bright rim, impure ones look washed out.
    float pad = sdRoundBox(f, vec2(0.47), 0.14);
    vec3 rc = u_resColor[min(rp.x, 9u)];
    if (rp.y == 0u) rc = mix(rc, vec3(0.45), 0.45);
    c = mix(c, mix(c * 0.45, rc, 0.55), cover(pad, px) * 0.9);
    vec3 rim = rp.y == 2u ? vec3(1.0, 0.86, 0.35) : rc * 1.15;
    c = mix(c, rim, cover(abs(pad + 0.02) - (rp.y == 2u ? 0.03 : 0.018), px) * (rp.y == 2u ? 0.9 : 0.6));
    if (u_cam.z > 10.0) {
      vec4 icon = atlas(u_atlas, rp.x, f / 0.72);
      c = mix(c, icon.rgb / max(icon.a, 0.001), icon.a * 0.85);
    }
  }

  if (u_map > 0.5) {
    uint k = texelFetch(u_kinds, tile, 0).r;
    if (k != 0u && (fog > 0.5 || k == 2u)) {
      float r = k == 1u ? 0.28 : 0.42;
      c = mix(c, u_kindColor[min(k, 39u)], cover(sdRoundBox(f, vec2(r), 0.08), px));
    }
  }

  if (u_overlay > 0.0) {
    // Power coverage: a warm wash with a crisp outline where coverage ends.
    float on = float(texelFetch(u_power, tile, 0).r);
    float edge = 0.0;
    for (int k = 0; k < 4; k++) {
      ivec2 o = k == 0 ? ivec2(1, 0) : k == 1 ? ivec2(-1, 0) : k == 2 ? ivec2(0, 1) : ivec2(0, -1);
      ivec2 n = clamp(tile + o, ivec2(0), u_world - 1);
      float other = float(texelFetch(u_power, n, 0).r);
      float side = k < 2 ? f.x * float(o.x) : f.y * float(o.y);
      edge = max(edge, (on - other) * cover(abs(side - 0.46) - 0.03, px));
    }
    c = mix(c, c * 0.8 + vec3(0.30, 0.25, 0.02), on * 0.45 * u_overlay);
    c = mix(c, vec3(1.0, 0.85, 0.2), edge * u_overlay);
  }

  float fade = clamp((u_cam.z - 14.0) / 20.0, 0.0, 1.0);
  float edge = 0.5 - max(abs(f.x), abs(f.y));
  c = mix(c, c * 0.8, (1.0 - smoothstep(0.0, px, edge)) * fade * 0.6);
  vec2 m = abs(fract(world / 16.0 + 0.5) - 0.5) * 16.0;
  c = mix(c, c * 1.18 + 0.02, (1.0 - smoothstep(0.0, px * 1.5, min(m.x, m.y))) * 0.35);

  // Unexplored land: dark, with a faint survey grid.
  vec3 dark = vec3(0.045, 0.05, 0.065) + (tnoise(world * 0.35) - 0.5) * 0.02;
  vec2 g = abs(fract(world / 8.0) - 0.5) * 8.0;
  dark += (1.0 - smoothstep(0.0, px * 1.2, min(g.x, g.y))) * 0.025;
  c = mix(dark, c, fog);
  outColor = vec4(c * u_ambient, 1.0);
}`;

// ---- Sprite pass ---------------------------------------------------------------------------

export const spriteVS = /* glsl */ `#version 300 es
layout(location = 0) in vec2 a_corner;  // unit quad, [-0.5, 0.5]^2
layout(location = 1) in vec2 a_pos;     // instance center (tiles)
layout(location = 2) in uvec4 a_misc;   // sprite, rotation, size (1/16 tile), param
layout(location = 3) in vec4 a_color;   // tint
uniform vec4 u_cam;   // center x, center y, world->clip scale x, scale y
uniform float u_zoom; // device px per tile
out vec2 v_p;
out vec4 v_color;
flat out uint v_sprite;
flat out uint v_param;
flat out uint v_rot;
flat out uint v_extra;
flat out float v_px;
void main() {
  float size = float(a_misc.z) * 0.0625;
  uint r = a_misc.y & 3u;
  vec2 c = a_corner;
  // Shader shapes that aren't oriented (core, glow, weather...) ignore the rotation.
  if (a_misc.x == 131u) r = 0u;
  vec2 rc = r == 0u ? c : r == 1u ? vec2(-c.y, c.x) : r == 2u ? -c : vec2(c.y, -c.x);
  gl_Position = vec4((a_pos + rc * size - u_cam.xy) * u_cam.zw, 0.0, 1.0);
  v_p = c;
  v_color = a_color;
  v_sprite = a_misc.x;
  v_param = a_misc.w;
  v_rot = r;
  v_extra = a_misc.y >> 2u;
  v_px = 1.0 / max(size * u_zoom, 1.0);
}`;

export const spriteFS = /* glsl */ `#version 300 es
precision highp float;
precision highp int;
in vec2 v_p;
in vec4 v_color;
flat in uint v_sprite;
flat in uint v_param;
flat in uint v_rot;
flat in uint v_extra;
flat in float v_px;
uniform sampler2D u_atlas;
uniform float u_phase; // belt travel in tiles, so stripes move with the items
uniform float u_time;
uniform vec3 u_ambient;
uniform vec3 u_stripe; // belt stripe colour (cosmetic)
out vec4 outColor;
${common}
const vec3 RAIL = vec3(0.12, 0.13, 0.16);
const vec3 BELT = vec3(0.21, 0.23, 0.27);

vec4 belt(uint spr, vec2 p, float px) {
  float s, l;
  if (spr == 128u) {
    s = p.x + 0.5;
    l = p.y;
  } else {
    if (spr == 130u) p.y = -p.y; // left turn = mirrored right turn
    vec2 v = p - vec2(-0.5, 0.5);
    l = length(v) - 0.5;
    s = atan(v.x, -v.y) * 0.63661977;
  }
  float e = abs(l);
  vec3 c = mix(RAIL, BELT, cover(e - 0.36, px));
  float stripe = fract((s - u_phase) * 2.0 + e * 0.9);
  float chev = smoothstep(0.0, 0.08, stripe) * (1.0 - smoothstep(0.22, 0.30, stripe));
  c = mix(c, u_stripe, chev * cover(e - 0.30, px));
  return vec4(c, 1.0) * cover(e - 0.46, px);
}

// The Core fills the inner 4/5 of its quad; the Ark grows around and on top of it.
vec4 core(vec2 p, float px, float progress, uint phase, uint trim, float launch) {
  vec3 gold = trim == 1u ? vec3(0.85, 0.88, 0.92) : trim == 2u
    ? 0.6 + 0.4 * cos(6.2831 * (vec3(0.0, 0.33, 0.67) + u_time * 0.08 + p.x))
    : vec3(0.98, 0.75, 0.14);
  float d = sdRoundBox(p, vec2(0.40), 0.06);
  vec3 c = mix(gold * 0.55, vec3(0.20, 0.23, 0.28), cover(d + 0.016, px));
  c = mix(c, vec3(0.26, 0.30, 0.36), cover(sdRoundBox(p, vec2(0.34), 0.04), px));
  float a = cover(d, px);
  // Terraforming progress ring towards the next stage.
  float ring = abs(length(p) - 0.24) - 0.022;
  float ang = fract(atan(p.x, -p.y) * 0.15915494 + 1.0);
  vec3 ringCol = ang < progress ? vec3(0.30, 0.85, 0.45) : vec3(0.08, 0.09, 0.11);
  c = mix(c, ringCol, cover(ring, px));
  vec2 q = abs(p);
  float side = min(sdBox(vec2(q.x - 0.37, p.y), vec2(0.024, 0.16)), sdBox(vec2(p.x, q.y - 0.37), vec2(0.16, 0.024)));
  c = mix(c, gold, cover(side, px) * 0.8);
  // Phase 1, Foundation: a launch pad ring and four scaffold towers at the corners.
  if (phase >= 1u) {
    float pad = abs(length(p) - 0.31) - 0.012;
    c = mix(c, vec3(0.85, 0.87, 0.9), cover(pad, px) * 0.7);
    vec2 t = q - vec2(0.43);
    float tower = sdCircle(t, 0.055);
    float lit = 0.6 + 0.4 * smoothstep(0.05, -0.03, length(t + vec2(0.015)));
    c = mix(c, vec3(0.58, 0.62, 0.68) * lit, cover(tower, px));
    c = mix(c, vec3(0.2, 0.22, 0.25), cover(abs(length(t) - 0.03) - 0.006, px) * cover(tower, px));
    a = max(a, cover(tower, px));
    // Phase 3, Systems: beacons on the towers.
    if (phase >= 3u) {
      float blink = 0.5 + 0.5 * sin(u_time * 3.0 + p.x * 9.0);
      c = mix(c, vec3(1.0, 0.25, 0.2) * (0.6 + blink), cover(length(t) - 0.02, px));
    }
  }
  // Phase 2, Hull: the Ark's hull ring.
  if (phase >= 2u) {
    float hull = abs(length(p) - 0.2) - 0.035;
    float seams = step(0.5, fract(atan(p.y, p.x) * 1.9099));
    c = mix(c, mix(vec3(0.62, 0.66, 0.72), vec3(0.78, 0.82, 0.88), seams), cover(hull, px));
  }
  // Phase 4, Habitat: greenhouse domes on the hull.
  if (phase >= 4u) {
    for (int k = 0; k < 4; k++) {
      float an = float(k) * 1.5708 + 0.7854;
      vec2 dp = p - vec2(cos(an), sin(an)) * 0.2;
      float dome = sdCircle(dp, 0.055);
      c = mix(c, mix(vec3(0.3, 0.8, 0.5), vec3(0.8, 1.0, 0.9), cover(sdCircle(dp - vec2(-0.015), 0.02), px)), cover(dome, px));
    }
  }
  // The planet in the middle, greener as the world comes alive; once the Ark is complete, a
  // rocket stands on the pad instead (and is gone once launched).
  if (phase >= 5u) {
    if (launch < 0.01) {
      vec2 rp = vec2(p.x, p.y + 0.02);
      float body = sdRoundBox(rp, vec2(0.05, 0.15), 0.05);
      float fins = sdBox(vec2(abs(rp.x) - 0.06, rp.y - 0.1), vec2(0.03, 0.04));
      c = mix(c, vec3(0.92, 0.94, 0.97), cover(min(body, fins), px));
      c = mix(c, vec3(0.2, 0.6, 0.95), cover(sdCircle(rp + vec2(0.0, 0.05), 0.022), px));
      float glow = 0.5 + 0.5 * sin(u_time * 5.0);
      c += vec3(1.0, 0.6, 0.2) * cover(sdCircle(rp - vec2(0.0, 0.17), 0.03), px) * glow;
    }
  } else {
    float planet = sdCircle(p, 0.14);
    vec3 pc = mix(vec3(0.62, 0.42, 0.32), vec3(0.28, 0.62, 0.42), progress * 0.3 + 0.2);
    pc *= 0.75 + 0.35 * smoothstep(0.14, -0.08, length(p - vec2(-0.05, -0.05)));
    c = mix(c, pc, cover(planet, px));
  }
  return vec4(c, 1.0) * a;
}

vec4 deleteMark(vec2 p, float px) {
  float box = abs(sdRoundBox(p, vec2(0.45), 0.1)) - 0.035;
  float x = min(sdSegment(p, vec2(-0.18), vec2(0.18)), sdSegment(p, vec2(-0.18, 0.18), vec2(0.18, -0.18))) - 0.04;
  float pulse = 0.75 + 0.25 * sin(u_time * 6.0);
  return vec4(0.95, 0.28, 0.28, 1.0) * max(cover(box, px), cover(x, px)) * pulse;
}

float sdBolt(vec2 p) {
  float d = sdSegment(p, vec2(0.08, -0.26), vec2(-0.1, 0.02));
  d = min(d, sdSegment(p, vec2(-0.1, 0.02), vec2(0.1, -0.02)));
  d = min(d, sdSegment(p, vec2(0.1, -0.02), vec2(-0.08, 0.26)));
  return d - 0.045;
}

// Problem badge: a coloured disc with a glyph.
vec4 badge(uint s, vec2 p, float px) {
  vec3 col = s == 4u ? vec3(0.94, 0.27, 0.27) : s == 3u ? vec3(0.96, 0.62, 0.04) : s == 5u ? vec3(0.98, 0.45, 0.09) : vec3(0.45, 0.5, 0.58);
  float disc = sdCircle(p, 0.44);
  float glyph;
  if (s == 4u || s == 5u) glyph = sdBolt(p);
  else if (s == 3u) glyph = min(sdBox(p - vec2(0.0, -0.06), vec2(0.05, 0.16)), sdCircle(p - vec2(0.0, 0.2), 0.06));
  else glyph = min(min(sdCircle(p - vec2(-0.18, 0.0), 0.06), sdCircle(p, 0.06)), sdCircle(p - vec2(0.18, 0.0), 0.06));
  vec3 c = mix(col, vec3(1.0), cover(glyph, px));
  c = mix(vec3(0.06), c, cover(disc + 0.05, px));
  float pulse = 0.85 + 0.15 * sin(u_time * 5.0);
  return vec4(c * pulse, 1.0) * cover(disc, px);
}

vec4 meteor(vec2 p, float px, float f) {
  // Head at the centre, tail streaming up-left (it falls towards the lower right).
  vec2 dir = normalize(vec2(0.54, 0.84));
  float along = dot(p, dir);
  float across = abs(p.x * dir.y - p.y * dir.x);
  float tail = smoothstep(-0.45, 0.0, along) * (1.0 - smoothstep(0.0, 0.05, along));
  float width = 0.02 + 0.06 * (along + 0.45);
  float t = (1.0 - smoothstep(0.0, width, across)) * tail;
  float head = 1.0 - smoothstep(0.0, 0.07, length(p));
  vec3 c = mix(vec3(1.0, 0.45, 0.1), vec3(1.0, 0.95, 0.8), head);
  float a = max(t * 0.8, head);
  return vec4(c * a, 0.0);
}

vec4 flash(vec2 p, float px, float age) {
  float r = length(p);
  float ring = 1.0 - smoothstep(0.0, 0.04, abs(r - 0.1 - age * 0.35));
  float core = (1.0 - smoothstep(0.0, 0.25, r)) * (1.0 - age);
  vec3 c = vec3(1.0, 0.7, 0.35) * (ring * (1.0 - age) + core * 1.5);
  return vec4(c, 0.0);
}

vec4 bird(vec2 p, float px, float flap) {
  float w = mix(-0.12, 0.14, flap);
  float d = min(sdSegment(p, vec2(0.0, 0.05), vec2(-0.32, w)), sdSegment(p, vec2(0.0, 0.05), vec2(0.32, w))) - 0.045;
  return vec4(vec3(0.1, 0.11, 0.13), 1.0) * cover(d, px);
}

void main() {
  vec4 c;
  uint s = v_sprite;
  bool emissive = false;
  if (s < 128u) {
    vec2 p = v_p;
    if (s == 126u) {
      // Drone: the rotors blur as they spin.
      float spin = float(v_param) / 255.0 * 6.2831;
      c = atlas(u_atlas, s, p);
      for (int k = 0; k < 4; k++) {
        vec2 hub = vec2(k < 2 ? -0.265 : 0.265, (k & 1) == 0 ? -0.265 : 0.265);
        vec2 q = p - hub;
        float ang = atan(q.y, q.x) + spin * (k == 0 || k == 3 ? 1.0 : -1.0);
        float blade = abs(sin(ang)) < 0.2 && length(q) < 0.15 ? 0.5 : 0.0;
        c.rgb = mix(c.rgb, vec3(0.9), blade * c.a);
      }
    } else {
      c = atlas(u_atlas, s, p);
    }
    if (s >= 64u && s < 126u) {
      uint r = v_rot;
      vec2 q = r == 0u ? p : r == 1u ? vec2(-p.y, p.x) : r == 2u ? -p : vec2(p.y, -p.x);
      if (v_param > 0u) {
        // Work progress bar along the building's screen-bottom edge.
        float prog = float(v_param) / 255.0;
        float track = sdBox(q - vec2(0.0, 0.40), vec2(0.30, 0.035));
        float fill = sdBox(q - vec2(-0.30 + 0.30 * prog, 0.40), vec2(0.30 * prog, 0.035));
        c = mix(c, vec4(0.05, 0.06, 0.08, 1.0), cover(track, v_px) * 0.85);
        c = mix(c, vec4(0.30, 0.85, 0.45, 1.0), cover(fill, v_px));
      }
      // Overclock pips (power shards) and the amplifier's ring.
      uint shards = v_extra & 3u;
      for (uint k = 0u; k < shards; k++) {
        float pip = sdCircle(q - vec2(0.36 - float(k) * 0.1, -0.38), 0.035);
        c = mix(c, vec4(1.0, 0.85, 0.2, 1.0), cover(pip, v_px));
      }
      if ((v_extra & 4u) != 0u) {
        float ring = abs(sdRoundBox(q, vec2(0.47), 0.12)) - 0.02;
        float pulse = 0.6 + 0.4 * sin(u_time * 3.0);
        c = mix(c, vec4(0.93, 0.35, 0.95, 1.0), cover(ring, v_px) * pulse);
      }
    }
  } else if (s <= 130u) {
    c = belt(s, v_p, v_px);
  } else if (s == 131u) {
    c = core(v_p, v_px, float(v_param) / 255.0, v_extra & 7u, (v_extra >> 3u) & 3u, v_color.r);
    outColor = vec4(c.rgb * u_ambient, c.a);
    return;
  } else if (s == 132u) {
    c = deleteMark(v_p, v_px);
    emissive = true;
  } else if (s == 133u) {
    // Night light: additive (alpha 0), soft falloff.
    float r = length(v_p) * 2.0;
    float glow = pow(max(0.0, 1.0 - r), 2.2) * float(v_param) / 255.0;
    outColor = vec4(v_color.rgb * glow * 0.9, 0.0);
    return;
  } else if (s == 134u) {
    c = badge(v_param, v_p, v_px);
    emissive = true;
  } else if (s == 135u) {
    outColor = meteor(v_p, v_px, float(v_param) / 255.0);
    return;
  } else if (s == 136u) {
    outColor = flash(v_p, v_px, float(v_param) / 255.0);
    return;
  } else if (s == 137u) {
    // Rain: a thin slanted streak.
    float d = sdSegment(v_p, vec2(-0.08, -0.45), vec2(0.08, 0.45)) - 0.02;
    c = vec4(0.75, 0.85, 1.0, 1.0) * cover(d, v_px) * 0.45;
  } else if (s == 138u) {
    float d = sdCircle(v_p, 0.3);
    c = vec4(1.0) * (1.0 - smoothstep(-0.15, 0.1, d)) * 0.85;
    emissive = true;
  } else if (s == 139u) {
    c = bird(v_p, v_px, float(v_param) / 255.0);
  } else {
    // Soft shadow.
    float d = length(v_p) * 2.0;
    float a = (1.0 - smoothstep(0.3, 1.0, d)) * v_color.a;
    outColor = vec4(0.0, 0.0, 0.0, a);
    return;
  }
  // Atlas and shapes are premultiplied; tint multiplies both, then nightfall.
  vec3 light = emissive ? vec3(1.0) : u_ambient;
  outColor = vec4(c.rgb * v_color.rgb * light, c.a) * v_color.a;
}`;
