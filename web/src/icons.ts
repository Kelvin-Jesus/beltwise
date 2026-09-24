// Procedural art: every item and building is drawn once at start-up with Canvas2D into a
// 2048x1024 atlas (16x8 cells of 128px). The GPU samples the atlas; the HUD reuses the same
// drawings as small data-URL images. No image files to download, and adding content is
// just adding a draw function.

import { ATLAS_CELL, ATLAS_COLS, ATLAS_ROWS, Kind, Sprite } from './constants';
import { CATEGORIES, type Content } from './content';

type Ctx = CanvasRenderingContext2D;

const OUTLINE = '#101318';

function lighten(hex: string, amt: number): string {
  const n = parseInt(hex.slice(1), 16);
  const f = (v: number) => Math.max(0, Math.min(255, Math.round(v + (amt > 0 ? (255 - v) * amt : v * amt))));
  const r = f((n >> 16) & 255);
  const g = f((n >> 8) & 255);
  const b = f(n & 255);
  return `#${((r << 16) | (g << 8) | b).toString(16).padStart(6, '0')}`;
}

/** Deterministic pseudo-random for stable rock shapes. */
function rand(seed: number): () => number {
  let s = seed >>> 0 || 1;
  return () => {
    s ^= s << 13;
    s ^= s >>> 17;
    s ^= s << 5;
    return ((s >>> 0) % 10000) / 10000;
  };
}

function stroke(ctx: Ctx, width = 5): void {
  ctx.lineWidth = width;
  ctx.strokeStyle = OUTLINE;
  ctx.lineJoin = 'round';
  ctx.stroke();
}

function rockPath(ctx: Ctx, r: number, seed: number, points = 9): void {
  const rnd = rand(seed);
  ctx.beginPath();
  for (let i = 0; i < points; i++) {
    const a = (i / points) * Math.PI * 2;
    const rr = r * (0.78 + rnd() * 0.3);
    const x = Math.cos(a) * rr;
    const y = Math.sin(a) * rr * 0.86;
    if (i === 0) ctx.moveTo(x, y);
    else ctx.lineTo(x, y);
  }
  ctx.closePath();
}

function rock(ctx: Ctx, base: string, fleck: string | null, seed: number, glow = false): void {
  rockPath(ctx, 40, seed);
  const g = ctx.createLinearGradient(-30, -30, 30, 36);
  g.addColorStop(0, lighten(base, 0.18));
  g.addColorStop(1, lighten(base, -0.25));
  ctx.fillStyle = g;
  ctx.fill();
  stroke(ctx);
  if (fleck) {
    const rnd = rand(seed * 7 + 3);
    if (glow) {
      ctx.shadowColor = fleck;
      ctx.shadowBlur = 10;
    }
    ctx.fillStyle = fleck;
    for (let i = 0; i < 6; i++) {
      const a = rnd() * Math.PI * 2;
      const d = rnd() * 22;
      ctx.beginPath();
      const s = 4 + rnd() * 5;
      ctx.moveTo(Math.cos(a) * d, Math.sin(a) * d - s);
      ctx.lineTo(Math.cos(a) * d + s, Math.sin(a) * d);
      ctx.lineTo(Math.cos(a) * d, Math.sin(a) * d + s);
      ctx.lineTo(Math.cos(a) * d - s, Math.sin(a) * d);
      ctx.fill();
    }
    ctx.shadowBlur = 0;
  }
}

function ingot(ctx: Ctx, base: string): void {
  ctx.beginPath();
  ctx.moveTo(-40, 20);
  ctx.lineTo(-28, -18);
  ctx.lineTo(28, -18);
  ctx.lineTo(40, 20);
  ctx.closePath();
  const g = ctx.createLinearGradient(0, -18, 0, 20);
  g.addColorStop(0, lighten(base, 0.35));
  g.addColorStop(1, lighten(base, -0.2));
  ctx.fillStyle = g;
  ctx.fill();
  stroke(ctx);
  ctx.fillStyle = lighten(base, 0.55);
  ctx.fillRect(-22, -12, 30, 5);
}

function plate(ctx: Ctx, base: string, rivets: boolean): void {
  ctx.beginPath();
  ctx.roundRect(-34, -30, 68, 60, 8);
  const g = ctx.createLinearGradient(-34, -30, 34, 30);
  g.addColorStop(0, lighten(base, 0.3));
  g.addColorStop(1, lighten(base, -0.2));
  ctx.fillStyle = g;
  ctx.fill();
  stroke(ctx);
  ctx.strokeStyle = lighten(base, 0.5);
  ctx.lineWidth = 3;
  ctx.strokeRect(-26, -22, 52, 44);
  if (rivets) {
    ctx.fillStyle = lighten(base, -0.35);
    for (const [x, y] of [
      [-22, -18],
      [22, -18],
      [-22, 18],
      [22, 18],
    ]) {
      ctx.beginPath();
      ctx.arc(x, y, 3.5, 0, Math.PI * 2);
      ctx.fill();
    }
  }
}

function gear(ctx: Ctx, color: string, r = 34, teeth = 10): void {
  ctx.beginPath();
  for (let i = 0; i < teeth * 2; i++) {
    const a = (i / (teeth * 2)) * Math.PI * 2;
    const rr = i % 2 === 0 ? r : r * 0.78;
    const a0 = a - Math.PI / (teeth * 2) * 0.55;
    const a1 = a + Math.PI / (teeth * 2) * 0.55;
    ctx.lineTo(Math.cos(a0) * rr, Math.sin(a0) * rr);
    ctx.lineTo(Math.cos(a1) * rr, Math.sin(a1) * rr);
  }
  ctx.closePath();
  const g = ctx.createRadialGradient(-8, -8, 4, 0, 0, r);
  g.addColorStop(0, lighten(color, 0.35));
  g.addColorStop(1, lighten(color, -0.2));
  ctx.fillStyle = g;
  ctx.fill();
  stroke(ctx, 4);
  ctx.beginPath();
  ctx.arc(0, 0, r * 0.3, 0, Math.PI * 2);
  ctx.fillStyle = OUTLINE;
  ctx.fill();
}

function drop(ctx: Ctx, color: string): void {
  ctx.beginPath();
  ctx.moveTo(0, -40);
  ctx.bezierCurveTo(14, -18, 32, 0, 32, 14);
  ctx.arc(0, 14, 32, 0, Math.PI, false);
  ctx.bezierCurveTo(-32, 0, -14, -18, 0, -40);
  ctx.closePath();
  const g = ctx.createLinearGradient(-20, -30, 20, 40);
  g.addColorStop(0, lighten(color, 0.4));
  g.addColorStop(1, lighten(color, -0.25));
  ctx.fillStyle = g;
  ctx.fill();
  stroke(ctx);
  ctx.fillStyle = 'rgba(255,255,255,0.75)';
  ctx.beginPath();
  ctx.ellipse(-11, 12, 5, 9, 0.4, 0, Math.PI * 2);
  ctx.fill();
}

const ITEM_ART: Record<string, (ctx: Ctx) => void> = {
  iron_ore: (c) => rock(c, '#6b6560', '#b8c6d8', 11),
  copper_ore: (c) => rock(c, '#6b5d52', '#f08a3c', 23),
  stone: (c) => {
    rock(c, '#8f8b86', null, 37);
    c.strokeStyle = 'rgba(0,0,0,0.3)';
    c.lineWidth = 3;
    c.beginPath();
    c.moveTo(-14, -10);
    c.lineTo(0, 2);
    c.lineTo(-4, 16);
    c.stroke();
  },
  coal: (c) => rock(c, '#2a2a30', '#6a6a75', 41),
  ice: (c) => {
    const pts = [
      [0, -42],
      [26, -14],
      [20, 30],
      [-18, 34],
      [-30, -6],
    ];
    c.beginPath();
    pts.forEach(([x, y], i) => (i ? c.lineTo(x, y) : c.moveTo(x, y)));
    c.closePath();
    const g = c.createLinearGradient(-30, -40, 30, 34);
    g.addColorStop(0, '#effaff');
    g.addColorStop(1, '#6fc3ee');
    c.fillStyle = g;
    c.fill();
    stroke(c);
    c.strokeStyle = 'rgba(255,255,255,0.9)';
    c.lineWidth = 3;
    c.beginPath();
    c.moveTo(-12, -14);
    c.lineTo(4, -30);
    c.moveTo(-6, 6);
    c.lineTo(12, -12);
    c.stroke();
  },
  titanium_ore: (c) => rock(c, '#5d6470', '#e6f4ff', 53),
  uranium_ore: (c) => rock(c, '#3c3f38', '#7dff5a', 67, true),
  iron_ingot: (c) => ingot(c, '#a9b6c6'),
  copper_ingot: (c) => ingot(c, '#e08a4a'),
  titanium_ingot: (c) => ingot(c, '#d6e8f5'),
  sand: (c) => {
    c.beginPath();
    c.moveTo(-42, 28);
    c.quadraticCurveTo(-10, -44, 42, 28);
    c.closePath();
    const g = c.createLinearGradient(0, -20, 0, 28);
    g.addColorStop(0, '#f5dfae');
    g.addColorStop(1, '#c9a764');
    c.fillStyle = g;
    c.fill();
    stroke(c);
    c.fillStyle = '#a88a4c';
    const rnd = rand(5);
    for (let i = 0; i < 14; i++) c.fillRect(-26 + rnd() * 52, -4 + rnd() * 26, 3, 3);
  },
  glass: (c) => {
    c.beginPath();
    c.roundRect(-30, -36, 60, 72, 6);
    c.fillStyle = 'rgba(160, 225, 255, 0.55)';
    c.fill();
    stroke(c);
    c.strokeStyle = 'rgba(255,255,255,0.9)';
    c.lineWidth = 5;
    c.beginPath();
    c.moveTo(-16, 18);
    c.lineTo(10, -22);
    c.moveTo(-2, 24);
    c.lineTo(18, -6);
    c.stroke();
  },
  iron_plate: (c) => plate(c, '#9fadbe', false),
  titanium_plate: (c) => plate(c, '#cfe3f2', true),
  copper_wire: (c) => {
    c.lineCap = 'round';
    for (let i = 0; i < 4; i++) {
      c.beginPath();
      c.ellipse(-18 + i * 12, 0, 10, 30, 0, 0, Math.PI * 2);
      c.strokeStyle = OUTLINE;
      c.lineWidth = 11;
      c.stroke();
      c.strokeStyle = i % 2 ? '#f0a060' : '#d9773a';
      c.lineWidth = 6;
      c.stroke();
    }
  },
  gear: (c) => gear(c, '#9aa6b5'),
  steel: (c) => {
    // I-beam
    c.beginPath();
    c.moveTo(-36, -30);
    c.lineTo(36, -30);
    c.lineTo(36, -18);
    c.lineTo(8, -18);
    c.lineTo(8, 18);
    c.lineTo(36, 18);
    c.lineTo(36, 30);
    c.lineTo(-36, 30);
    c.lineTo(-36, 18);
    c.lineTo(-8, 18);
    c.lineTo(-8, -18);
    c.lineTo(-36, -18);
    c.closePath();
    const g = c.createLinearGradient(-36, -30, 36, 30);
    g.addColorStop(0, '#9fb0c4');
    g.addColorStop(1, '#4b5a6c');
    c.fillStyle = g;
    c.fill();
    stroke(c);
  },
  circuit: (c) => {
    c.beginPath();
    c.roundRect(-36, -32, 72, 64, 6);
    c.fillStyle = '#23864c';
    c.fill();
    stroke(c);
    c.strokeStyle = '#f2c14e';
    c.lineWidth = 3;
    c.beginPath();
    c.moveTo(-28, -18);
    c.lineTo(-10, -18);
    c.lineTo(-10, 0);
    c.moveTo(28, 18);
    c.lineTo(10, 18);
    c.lineTo(10, 0);
    c.moveTo(-28, 20);
    c.lineTo(-16, 20);
    c.stroke();
    c.fillStyle = '#1b1f26';
    c.fillRect(-12, -12, 24, 24);
    c.fillStyle = '#f2c14e';
    for (let i = -1; i <= 1; i++) {
      c.fillRect(-18, i * 7 - 1.5, 5, 3);
      c.fillRect(13, i * 7 - 1.5, 5, 3);
    }
  },
  motor: (c) => {
    c.beginPath();
    c.roundRect(-34, -24, 56, 48, 10);
    const g = c.createLinearGradient(0, -24, 0, 24);
    g.addColorStop(0, '#8a9ab0');
    g.addColorStop(1, '#4a5668');
    c.fillStyle = g;
    c.fill();
    stroke(c);
    c.fillStyle = '#d9773a';
    for (let i = 0; i < 4; i++) c.fillRect(-26 + i * 11, -20, 6, 40);
    c.fillStyle = '#c6d0dc';
    c.fillRect(22, -5, 16, 10);
    c.strokeStyle = OUTLINE;
    c.lineWidth = 3;
    c.strokeRect(22, -5, 16, 10);
  },
  water: (c) => drop(c, '#3fa7ff'),
  algae: (c) => {
    const blobs = [
      [-14, 6, 20],
      [12, 10, 18],
      [0, -14, 18],
    ];
    for (const [x, y, r] of blobs) {
      c.beginPath();
      c.arc(x, y, r, 0, Math.PI * 2);
      c.fillStyle = '#4fbf62';
      c.fill();
      stroke(c, 4);
    }
    c.fillStyle = '#9ff0a8';
    for (const [x, y] of [
      [-18, 0],
      [8, 6],
      [-4, -18],
    ]) {
      c.beginPath();
      c.arc(x, y, 4, 0, Math.PI * 2);
      c.fill();
    }
  },
  fertilizer: (c) => {
    c.beginPath();
    c.moveTo(-28, -28);
    c.lineTo(28, -28);
    c.lineTo(34, 34);
    c.lineTo(-34, 34);
    c.closePath();
    c.fillStyle = '#9b7b52';
    c.fill();
    stroke(c);
    c.fillStyle = '#7a5f3c';
    c.fillRect(-28, -36, 56, 10);
    c.beginPath();
    c.ellipse(0, 6, 9, 16, 0.6, 0, Math.PI * 2);
    c.fillStyle = '#6cc24a';
    c.fill();
  },
  fuel_rod: (c) => {
    c.beginPath();
    c.roundRect(-14, -40, 28, 80, 8);
    c.fillStyle = '#8a97a6';
    c.fill();
    stroke(c);
    c.shadowColor = '#8cff5a';
    c.shadowBlur = 14;
    c.fillStyle = '#8cff5a';
    c.fillRect(-7, -26, 14, 52);
    c.shadowBlur = 0;
  },
  frame: (c) => {
    c.beginPath();
    c.rect(-34, -34, 68, 68);
    c.rect(-22, -22, 44, 44);
    c.fillStyle = '#cfe3f2';
    c.fill('evenodd');
    stroke(c);
    c.strokeStyle = '#9fb7c9';
    c.lineWidth = 6;
    c.beginPath();
    c.moveTo(-22, -22);
    c.lineTo(22, 22);
    c.stroke();
  },
  crude_oil: (c) => {
    drop(c, '#2b2a33');
    c.fillStyle = 'rgba(160, 120, 255, 0.55)';
    c.beginPath();
    c.ellipse(8, 20, 12, 5, -0.4, 0, Math.PI * 2);
    c.fill();
    c.fillStyle = 'rgba(80, 220, 180, 0.45)';
    c.beginPath();
    c.ellipse(-4, 26, 9, 3.5, 0.2, 0, Math.PI * 2);
    c.fill();
  },
  meteorite: (c) => {
    rock(c, '#3a3346', null, 91);
    c.shadowColor = '#c084fc';
    c.shadowBlur = 12;
    c.strokeStyle = '#e9d5ff';
    c.lineWidth = 3.5;
    c.lineCap = 'round';
    c.beginPath();
    c.moveTo(-20, -8);
    c.lineTo(-4, 2);
    c.lineTo(-10, 18);
    c.moveTo(-4, 2);
    c.lineTo(14, -6);
    c.lineTo(22, 8);
    c.stroke();
    c.shadowBlur = 0;
  },
  silicon: (c) => {
    c.beginPath();
    c.arc(0, 0, 36, 0.35 * Math.PI, 2.65 * Math.PI);
    c.closePath();
    const g = c.createLinearGradient(-30, -30, 30, 30);
    g.addColorStop(0, '#c8d4ee');
    g.addColorStop(1, '#56627e');
    c.fillStyle = g;
    c.fill();
    stroke(c);
    c.strokeStyle = 'rgba(255,255,255,0.35)';
    c.lineWidth = 2;
    for (let k = -2; k <= 2; k++) {
      c.beginPath();
      c.moveTo(k * 11, -28);
      c.lineTo(k * 11, 28);
      c.moveTo(-28, k * 11);
      c.lineTo(28, k * 11);
      c.stroke();
    }
  },
  plastic: (c) => {
    for (const [dx, dy, col] of [
      [-8, 8, '#e6dfd2'],
      [0, 0, '#f4efe6'],
      [8, -8, '#ffffff'],
    ] as const) {
      c.beginPath();
      c.roundRect(-30 + dx, -20 + dy, 52, 38, 10);
      c.fillStyle = col;
      c.fill();
      stroke(c, 4);
    }
    c.fillStyle = 'rgba(56, 189, 248, 0.5)';
    c.fillRect(-12, -18, 24, 5);
  },
  fuel: (c) => {
    c.beginPath();
    c.roundRect(-28, -26, 56, 64, 8);
    const g = c.createLinearGradient(-28, 0, 28, 0);
    g.addColorStop(0, '#f05a4a');
    g.addColorStop(1, '#9c2318');
    c.fillStyle = g;
    c.fill();
    stroke(c);
    c.fillStyle = '#5a1810';
    c.fillRect(6, -38, 14, 14);
    c.strokeStyle = OUTLINE;
    c.lineWidth = 3;
    c.strokeRect(6, -38, 14, 14);
    c.strokeStyle = 'rgba(255,255,255,0.45)';
    c.lineWidth = 5;
    c.beginPath();
    c.moveTo(-16, -10);
    c.lineTo(12, 26);
    c.moveTo(12, -10);
    c.lineTo(-16, 26);
    c.stroke();
  },
  battery: (c) => {
    c.beginPath();
    c.roundRect(-22, -34, 44, 72, 8);
    c.fillStyle = '#2a3140';
    c.fill();
    stroke(c);
    c.fillStyle = '#c6d0dc';
    c.fillRect(-8, -42, 16, 9);
    const g = c.createLinearGradient(0, 30, 0, -26);
    g.addColorStop(0, '#16a34a');
    g.addColorStop(1, '#86efac');
    c.fillStyle = g;
    c.fillRect(-14, -8, 28, 38);
    c.fillStyle = '#fef08a';
    c.beginPath();
    c.moveTo(4, -28);
    c.lineTo(-8, -10);
    c.lineTo(0, -10);
    c.lineTo(-4, 4);
    c.lineTo(8, -14);
    c.lineTo(0, -14);
    c.closePath();
    c.fill();
  },
  computer: (c) => {
    c.beginPath();
    c.roundRect(-38, -30, 76, 60, 8);
    c.fillStyle = '#1f2632';
    c.fill();
    stroke(c);
    c.fillStyle = '#0b1a24';
    c.fillRect(-28, -20, 56, 30);
    c.shadowColor = '#38bdf8';
    c.shadowBlur = 10;
    c.fillStyle = '#7dd3fc';
    for (let k = 0; k < 3; k++) c.fillRect(-22, -14 + k * 8, 20 + ((k * 13) % 24), 4);
    c.shadowBlur = 0;
    c.fillStyle = '#f2c14e';
    for (let k = -3; k <= 3; k++) c.fillRect(k * 9 - 2, 18, 4, 8);
  },
  reinforced_plate: (c) => {
    plate(c, '#7d8ea3', true);
    c.strokeStyle = '#3c4a5c';
    c.lineWidth = 6;
    c.beginPath();
    c.moveTo(-24, -20);
    c.lineTo(24, 20);
    c.moveTo(24, -20);
    c.lineTo(-24, 20);
    c.stroke();
  },
  alien_alloy: (c) => {
    c.beginPath();
    for (let k = 0; k < 6; k++) {
      const a = (k / 6) * Math.PI * 2 + Math.PI / 6;
      c.lineTo(Math.cos(a) * 38, Math.sin(a) * 30);
    }
    c.closePath();
    const g = c.createLinearGradient(-36, -30, 36, 30);
    g.addColorStop(0, '#5eead4');
    g.addColorStop(0.5, '#a78bfa');
    g.addColorStop(1, '#f472b6');
    c.fillStyle = g;
    c.fill();
    stroke(c);
    c.strokeStyle = 'rgba(255,255,255,0.6)';
    c.lineWidth = 3;
    c.beginPath();
    c.moveTo(-18, -10);
    c.lineTo(0, -20);
    c.lineTo(18, -10);
    c.stroke();
  },
  quantum_core: (c) => {
    glowCircle(c, 0, 0, 26, '#ffffff', '#22d3ee');
    c.strokeStyle = OUTLINE;
    c.lineWidth = 4;
    c.beginPath();
    c.arc(0, 0, 26, 0, Math.PI * 2);
    c.stroke();
    c.lineWidth = 3.5;
    for (const [rot, col] of [
      [0.5, '#f0abfc'],
      [-0.5, '#67e8f9'],
    ] as const) {
      c.strokeStyle = col;
      c.beginPath();
      c.ellipse(0, 0, 42, 14, rot, 0, Math.PI * 2);
      c.stroke();
    }
  },
  seeds: (c) => {
    for (const [x, y, r] of [
      [-16, 8, -0.5],
      [14, 10, 0.4],
      [0, -12, 0],
    ] as const) {
      c.save();
      c.translate(x, y);
      c.rotate(r);
      c.beginPath();
      c.ellipse(0, 0, 11, 17, 0, 0, Math.PI * 2);
      c.fillStyle = '#b07a3c';
      c.fill();
      stroke(c, 4);
      c.restore();
    }
    c.strokeStyle = '#4ade80';
    c.lineWidth = 5;
    c.lineCap = 'round';
    c.beginPath();
    c.moveTo(0, -26);
    c.quadraticCurveTo(10, -40, 22, -36);
    c.stroke();
  },
  power_shard: (c) => {
    c.shadowColor = '#fde047';
    c.shadowBlur = 16;
    c.beginPath();
    c.moveTo(0, -42);
    c.lineTo(20, -6);
    c.lineTo(8, 40);
    c.lineTo(-18, 10);
    c.lineTo(-12, -18);
    c.closePath();
    const g = c.createLinearGradient(-18, -40, 18, 40);
    g.addColorStop(0, '#fffbe0');
    g.addColorStop(1, '#eab308');
    c.fillStyle = g;
    c.fill();
    c.shadowBlur = 0;
    stroke(c, 4);
    c.strokeStyle = 'rgba(255,255,255,0.8)';
    c.lineWidth = 3;
    c.beginPath();
    c.moveTo(-4, -26);
    c.lineTo(4, 20);
    c.stroke();
  },
  amplifier: (c) => {
    glowCircle(c, 0, 0, 30, '#fdf4ff', '#d946ef');
    c.strokeStyle = OUTLINE;
    c.lineWidth = 4;
    c.beginPath();
    c.arc(0, 0, 30, 0, Math.PI * 2);
    c.stroke();
    c.strokeStyle = '#fbcfe8';
    c.lineWidth = 4;
    for (let k = 0; k < 3; k++) {
      c.beginPath();
      c.arc(0, 0, 38, k * 2.09, k * 2.09 + 1.1);
      c.stroke();
    }
    c.fillStyle = OUTLINE;
    c.beginPath();
    c.arc(0, 0, 8, 0, Math.PI * 2);
    c.fill();
  },

};

// ---- Buildings -------------------------------------------------------------------------------

function body(ctx: Ctx, accent: string, terraformer: boolean): void {
  ctx.beginPath();
  ctx.roundRect(-54, -54, 108, 108, 16);
  const g = ctx.createLinearGradient(0, -54, 0, 54);
  g.addColorStop(0, terraformer ? '#2f4a47' : '#3d4654');
  g.addColorStop(1, terraformer ? '#1d302e' : '#272d37');
  ctx.fillStyle = g;
  ctx.fill();
  ctx.lineWidth = 6;
  ctx.strokeStyle = OUTLINE;
  ctx.stroke();
  ctx.beginPath();
  ctx.roundRect(-44, -44, 88, 88, 10);
  ctx.strokeStyle = 'rgba(255,255,255,0.07)';
  ctx.lineWidth = 3;
  ctx.stroke();
  // Corner bolts.
  ctx.fillStyle = 'rgba(0,0,0,0.35)';
  for (const [x, y] of [
    [-40, -40],
    [40, -40],
    [-40, 40],
    [40, 40],
  ]) {
    ctx.beginPath();
    ctx.arc(x, y, 3.5, 0, Math.PI * 2);
    ctx.fill();
  }
  ctx.fillStyle = accent;
  ctx.fillRect(-44, 38, 88, 4);
}

function outArrow(ctx: Ctx, accent: string): void {
  ctx.beginPath();
  ctx.moveTo(40, -13);
  ctx.lineTo(56, 0);
  ctx.lineTo(40, 13);
  ctx.closePath();
  ctx.fillStyle = accent;
  ctx.fill();
  ctx.lineWidth = 3;
  ctx.strokeStyle = OUTLINE;
  ctx.stroke();
}

function bolt(ctx: Ctx, x: number, y: number, scale: number): void {
  ctx.save();
  ctx.translate(x, y);
  ctx.scale(scale, scale);
  ctx.beginPath();
  ctx.moveTo(6, -22);
  ctx.lineTo(-12, 4);
  ctx.lineTo(0, 4);
  ctx.lineTo(-6, 22);
  ctx.lineTo(12, -4);
  ctx.lineTo(0, -4);
  ctx.closePath();
  ctx.fillStyle = '#facc15';
  ctx.fill();
  ctx.lineWidth = 3;
  ctx.strokeStyle = OUTLINE;
  ctx.stroke();
  ctx.restore();
}

function glowCircle(ctx: Ctx, x: number, y: number, r: number, inner: string, outer: string): void {
  const g = ctx.createRadialGradient(x, y, 1, x, y, r);
  g.addColorStop(0, inner);
  g.addColorStop(1, outer);
  ctx.fillStyle = g;
  ctx.beginPath();
  ctx.arc(x, y, r, 0, Math.PI * 2);
  ctx.fill();
}

const BUILDING_ART: Record<string, (ctx: Ctx, accent: string) => void> = {
  drill: (c, a) => {
    gear(c, '#8a7650', 30, 8);
    c.beginPath();
    c.arc(0, 0, 12, 0, Math.PI * 2);
    c.fillStyle = a;
    c.fill();
  },
  smelter: (c) => {
    c.beginPath();
    c.roundRect(-30, -30, 60, 56, 8);
    c.fillStyle = '#4a4f59';
    c.fill();
    stroke(c, 4);
    c.beginPath();
    c.moveTo(-16, 20);
    c.lineTo(-16, 0);
    c.arc(0, 0, 16, Math.PI, 0);
    c.lineTo(16, 20);
    c.closePath();
    const g = c.createLinearGradient(0, -16, 0, 20);
    g.addColorStop(0, '#ffd166');
    g.addColorStop(1, '#ff6b1a');
    c.fillStyle = g;
    c.fill();
    c.fillStyle = '#5b606b';
    c.fillRect(8, -44, 12, 16);
  },
  crusher: (c) => {
    c.save();
    c.translate(-14, 0);
    gear(c, '#8d949e', 18, 8);
    c.restore();
    c.save();
    c.translate(14, 0);
    gear(c, '#8d949e', 18, 8);
    c.restore();
  },
  press: (c, a) => {
    c.fillStyle = '#6b7482';
    c.fillRect(-6, -40, 12, 22);
    c.beginPath();
    c.roundRect(-26, -20, 52, 14, 4);
    c.fillStyle = a;
    c.fill();
    stroke(c, 3);
    c.beginPath();
    c.roundRect(-30, 8, 60, 14, 3);
    c.fillStyle = '#a3afbd';
    c.fill();
    stroke(c, 3);
  },
  assembler: (c, a) => {
    gear(c, a, 24, 8);
    c.strokeStyle = '#d6dde6';
    c.lineWidth = 7;
    c.lineCap = 'round';
    c.beginPath();
    c.moveTo(-30, 30);
    c.lineTo(-14, 14);
    c.stroke();
  },
  foundry: (c) => {
    c.beginPath();
    c.moveTo(-26, -24);
    c.lineTo(26, -24);
    c.lineTo(18, 24);
    c.lineTo(-18, 24);
    c.closePath();
    c.fillStyle = '#4a4f59';
    c.fill();
    stroke(c, 4);
    const g = c.createLinearGradient(0, -24, 0, 0);
    g.addColorStop(0, '#ffe08a');
    g.addColorStop(1, '#ff7a1a');
    c.fillStyle = g;
    c.fillRect(-22, -22, 44, 12);
  },
  electronics: (c) => {
    c.fillStyle = '#23864c';
    c.fillRect(-26, -26, 52, 52);
    c.fillStyle = '#1b1f26';
    c.fillRect(-14, -14, 28, 28);
    c.fillStyle = '#f2c14e';
    for (let i = -2; i <= 2; i++) {
      c.fillRect(i * 9 - 2, -34, 4, 8);
      c.fillRect(i * 9 - 2, 26, 4, 8);
    }
  },
  melter: (c) => {
    c.beginPath();
    c.roundRect(-18, -34, 36, 32, 5);
    c.fillStyle = '#bfe9ff';
    c.fill();
    stroke(c, 3);
    c.strokeStyle = '#ff7a3c';
    c.lineWidth = 5;
    c.lineCap = 'round';
    for (let i = -1; i <= 1; i++) {
      c.beginPath();
      c.moveTo(i * 14, 30);
      c.quadraticCurveTo(i * 14 + 7, 18, i * 14, 8);
      c.stroke();
    }
  },
  biolab: (c) => {
    c.beginPath();
    c.moveTo(-8, -36);
    c.lineTo(8, -36);
    c.lineTo(8, -12);
    c.lineTo(28, 26);
    c.lineTo(-28, 26);
    c.lineTo(-8, -12);
    c.closePath();
    c.fillStyle = 'rgba(210, 240, 255, 0.35)';
    c.fill();
    stroke(c, 4);
    c.beginPath();
    c.moveTo(-18, 8);
    c.lineTo(18, 8);
    c.lineTo(26, 24);
    c.lineTo(-26, 24);
    c.closePath();
    c.fillStyle = '#4fbf62';
    c.fill();
  },
  enricher: (c) => {
    glowCircle(c, 0, 0, 34, '#fff3a0', '#e0b100');
    c.fillStyle = OUTLINE;
    for (let i = 0; i < 3; i++) {
      c.beginPath();
      c.moveTo(0, 0);
      c.arc(0, 0, 28, (i * 2 * Math.PI) / 3 - 0.5, (i * 2 * Math.PI) / 3 + 0.5);
      c.closePath();
      c.fill();
    }
    c.beginPath();
    c.arc(0, 0, 7, 0, Math.PI * 2);
    c.fill();
  },
  heater: (c) => {
    for (let i = 0; i < 4; i++) {
      const x = -24 + i * 16;
      c.beginPath();
      c.roundRect(x - 5, -30, 10, 60, 5);
      const g = c.createLinearGradient(0, -30, 0, 30);
      g.addColorStop(0, '#ffcf6a');
      g.addColorStop(1, '#ff4d1a');
      c.fillStyle = g;
      c.fill();
      stroke(c, 3);
    }
  },
  vaporizer: (c) => {
    c.beginPath();
    c.arc(0, 16, 18, 0, Math.PI * 2);
    c.fillStyle = '#1b2a33';
    c.fill();
    stroke(c, 4);
    c.fillStyle = 'rgba(235, 245, 255, 0.95)';
    for (const [x, y, r] of [
      [-10, -12, 12],
      [8, -20, 14],
      [2, -36, 10],
    ]) {
      c.beginPath();
      c.arc(x, y, r, 0, Math.PI * 2);
      c.fill();
    }
  },
  oxygenator: (c) => {
    for (const [x, y, r] of [
      [-14, 10, 16],
      [14, -4, 13],
      [-4, -24, 10],
    ]) {
      c.beginPath();
      c.arc(x, y, r, 0, Math.PI * 2);
      c.fillStyle = 'rgba(120, 200, 255, 0.35)';
      c.fill();
      c.lineWidth = 4;
      c.strokeStyle = '#7cd0ff';
      c.stroke();
    }
    c.fillStyle = '#e8f6ff';
    c.font = 'bold 20px system-ui, sans-serif';
    c.textAlign = 'center';
    c.textBaseline = 'middle';
    c.fillText('O₂', -14, 11);
  },
  greenhouse: (c) => {
    c.beginPath();
    c.arc(0, 18, 34, Math.PI, 0);
    c.closePath();
    c.fillStyle = 'rgba(190, 240, 255, 0.35)';
    c.fill();
    stroke(c, 4);
    c.strokeStyle = 'rgba(255,255,255,0.4)';
    c.lineWidth = 2;
    c.beginPath();
    c.moveTo(0, 18);
    c.lineTo(0, -16);
    c.moveTo(-24, 18);
    c.quadraticCurveTo(-18, -6, 0, -16);
    c.moveTo(24, 18);
    c.quadraticCurveTo(18, -6, 0, -16);
    c.stroke();
    c.fillStyle = '#4fbf62';
    c.beginPath();
    c.ellipse(-7, 4, 7, 12, -0.6, 0, Math.PI * 2);
    c.ellipse(8, 2, 7, 12, 0.6, 0, Math.PI * 2);
    c.fill();
  },
  thermal: (c) => {
    glowCircle(c, 0, 0, 38, '#fffbe0', '#ff6b1a');
    c.lineWidth = 4;
    c.strokeStyle = 'rgba(40, 20, 10, 0.7)';
    for (const r of [16, 26]) {
      c.beginPath();
      c.arc(0, 0, r, 0, Math.PI * 2);
      c.stroke();
    }
  },
  splitter: (c, a) => {
    c.strokeStyle = a;
    c.lineWidth = 8;
    c.lineCap = 'round';
    c.lineJoin = 'round';
    for (const [x, y, dx, dy] of [
      [30, 0, -8, 0],
      [0, -30, 0, 8],
      [0, 30, 0, -8],
    ]) {
      c.beginPath();
      c.moveTo(-30, 0);
      c.lineTo(0, 0);
      c.lineTo(x, y);
      c.stroke();
      c.beginPath();
      c.moveTo(x + dy * 1.2, y - dx * 1.2);
      c.lineTo(x, y);
      c.lineTo(x - dy * 1.2, y + dx * 1.2);
      c.stroke();
    }
  },
  tunnel: (c) => tunnelArt(c, true),
  incinerator: (c) => {
    c.beginPath();
    c.roundRect(-28, -4, 56, 32, 5);
    c.fillStyle = '#3a3f48';
    c.fill();
    stroke(c, 4);
    c.beginPath();
    c.moveTo(0, -40);
    c.bezierCurveTo(22, -18, 20, -4, 0, 6);
    c.bezierCurveTo(-20, -4, -22, -18, 0, -40);
    const g = c.createLinearGradient(0, -40, 0, 6);
    g.addColorStop(0, '#ffe08a');
    g.addColorStop(1, '#ff4d1a');
    c.fillStyle = g;
    c.fill();
  },
  pole: (c) => {
    c.fillStyle = '#3a414b';
    c.beginPath();
    c.arc(0, 0, 18, 0, Math.PI * 2);
    c.fill();
    stroke(c, 4);
    c.strokeStyle = '#8b95a3';
    c.lineWidth = 6;
    c.lineCap = 'round';
    c.beginPath();
    c.moveTo(-32, -18);
    c.lineTo(32, -18);
    c.stroke();
    for (const x of [-30, 0, 30]) {
      c.fillStyle = '#7dd3fc';
      c.beginPath();
      c.arc(x, -18, 6, 0, Math.PI * 2);
      c.fill();
    }
    bolt(c, 0, 4, 1);
  },
  coal_gen: (c) => {
    c.beginPath();
    c.roundRect(-32, -18, 48, 46, 6);
    c.fillStyle = '#4a4f59';
    c.fill();
    stroke(c, 4);
    const g = c.createLinearGradient(0, 0, 0, 22);
    g.addColorStop(0, '#ffd166');
    g.addColorStop(1, '#ff5a1a');
    c.fillStyle = g;
    c.fillRect(-24, 2, 32, 18);
    c.fillStyle = '#5b606b';
    c.fillRect(18, -40, 14, 56);
    c.strokeStyle = OUTLINE;
    c.lineWidth = 3;
    c.strokeRect(18, -40, 14, 56);
    bolt(c, -8, -26, 0.8);
  },
  solar: (c) => {
    c.beginPath();
    c.roundRect(-44, -40, 88, 80, 6);
    c.fillStyle = '#1e3a8a';
    c.fill();
    stroke(c, 4);
    c.strokeStyle = 'rgba(147, 197, 253, 0.8)';
    c.lineWidth = 2.5;
    for (let k = -1; k <= 1; k++) {
      c.beginPath();
      c.moveTo(k * 22, -38);
      c.lineTo(k * 22, 38);
      c.stroke();
    }
    for (let k = -2; k <= 2; k++) {
      c.beginPath();
      c.moveTo(-42, k * 16);
      c.lineTo(42, k * 16);
      c.stroke();
    }
    c.fillStyle = 'rgba(255,255,255,0.18)';
    c.beginPath();
    c.moveTo(-44, -40);
    c.lineTo(-4, -40);
    c.lineTo(-44, 0);
    c.closePath();
    c.fill();
  },
  fuel_gen: (c) => {
    c.beginPath();
    c.ellipse(-12, 0, 20, 32, 0, 0, Math.PI * 2);
    const g = c.createLinearGradient(-32, 0, 8, 0);
    g.addColorStop(0, '#ef4444');
    g.addColorStop(1, '#7f1d1d');
    c.fillStyle = g;
    c.fill();
    stroke(c, 4);
    c.save();
    c.translate(22, 0);
    gear(c, '#9aa6b5', 16, 8);
    c.restore();
    bolt(c, -12, 0, 0.8);
  },
  reactor: (c) => {
    c.beginPath();
    c.arc(0, 0, 38, 0, Math.PI * 2);
    c.fillStyle = '#9ca3af';
    c.fill();
    stroke(c, 5);
    c.beginPath();
    c.arc(0, 0, 26, 0, Math.PI * 2);
    c.fillStyle = '#374151';
    c.fill();
    glowCircle(c, 0, 0, 18, '#f0fdf4', '#22c55e');
    c.fillStyle = OUTLINE;
    for (let i = 0; i < 3; i++) {
      c.beginPath();
      c.moveTo(0, 0);
      c.arc(0, 0, 14, (i * 2 * Math.PI) / 3 - 0.45, (i * 2 * Math.PI) / 3 + 0.45);
      c.closePath();
      c.fill();
    }
  },
  battery_bank: (c) => {
    for (const x of [-26, 0, 26]) {
      c.beginPath();
      c.roundRect(x - 10, -30, 20, 60, 6);
      c.fillStyle = '#2a3140';
      c.fill();
      stroke(c, 3);
      c.fillStyle = '#4ade80';
      c.fillRect(x - 6, -4, 12, 28);
      c.fillStyle = '#c6d0dc';
      c.fillRect(x - 5, -36, 10, 6);
    }
  },
  sorter: (c, a) => {
    c.strokeStyle = a;
    c.lineWidth = 8;
    c.lineCap = 'round';
    c.lineJoin = 'round';
    c.beginPath();
    c.moveTo(-30, 0);
    c.lineTo(32, 0);
    c.moveTo(22, -10);
    c.lineTo(32, 0);
    c.lineTo(22, 10);
    c.stroke();
    c.strokeStyle = 'rgba(148, 163, 184, 0.7)';
    c.lineWidth = 6;
    c.beginPath();
    c.moveTo(-8, 0);
    c.lineTo(-8, -32);
    c.moveTo(-8, 0);
    c.lineTo(-8, 32);
    c.stroke();
    c.beginPath();
    c.moveTo(-8, -10);
    c.lineTo(4, 0);
    c.lineTo(-8, 10);
    c.lineTo(-20, 0);
    c.closePath();
    c.fillStyle = '#fbbf24';
    c.fill();
    stroke(c, 3);
  },
  storage: (c) => {
    c.beginPath();
    c.roundRect(-36, -30, 72, 62, 6);
    const g = c.createLinearGradient(0, -30, 0, 32);
    g.addColorStop(0, '#a16207');
    g.addColorStop(1, '#713f12');
    c.fillStyle = g;
    c.fill();
    stroke(c, 4);
    c.strokeStyle = 'rgba(0,0,0,0.45)';
    c.lineWidth = 4;
    c.beginPath();
    c.moveTo(-36, -8);
    c.lineTo(36, -8);
    c.stroke();
    c.fillStyle = '#d6dde6';
    c.fillRect(-8, -14, 16, 12);
  },
  drone_port: (c, a) => {
    c.beginPath();
    c.arc(0, 0, 36, 0, Math.PI * 2);
    c.fillStyle = '#2a3140';
    c.fill();
    c.lineWidth = 5;
    c.strokeStyle = a;
    c.stroke();
    c.strokeStyle = '#e2e8f0';
    c.lineWidth = 7;
    c.lineCap = 'round';
    c.beginPath();
    c.moveTo(-14, -18);
    c.lineTo(-14, 18);
    c.moveTo(14, -18);
    c.lineTo(14, 18);
    c.moveTo(-14, 0);
    c.lineTo(14, 0);
    c.stroke();
  },
  radar: (c) => {
    c.beginPath();
    c.arc(0, 8, 14, 0, Math.PI * 2);
    c.fillStyle = '#4a4f59';
    c.fill();
    stroke(c, 4);
    c.beginPath();
    c.ellipse(0, -6, 36, 22, -0.5, Math.PI * 0.05, Math.PI * 1.05);
    c.closePath();
    const g = c.createLinearGradient(-30, -20, 30, 20);
    g.addColorStop(0, '#e2e8f0');
    g.addColorStop(1, '#94a3b8');
    c.fillStyle = g;
    c.fill();
    stroke(c, 4);
    c.strokeStyle = '#22d3ee';
    c.lineWidth = 3;
    for (const r of [44, 52]) {
      c.beginPath();
      c.arc(-6, 4, r, -1.2, -0.5);
      c.stroke();
    }
  },
  recycler: (c) => {
    c.strokeStyle = '#4ade80';
    c.lineWidth = 8;
    c.lineCap = 'round';
    for (let k = 0; k < 3; k++) {
      const a0 = (k * 2 * Math.PI) / 3 - Math.PI / 2 + 0.25;
      const a1 = a0 + 1.5;
      c.beginPath();
      c.arc(0, 4, 28, a0, a1);
      c.stroke();
      const ex = Math.cos(a1) * 28;
      const ey = Math.sin(a1) * 28 + 4;
      c.beginPath();
      c.moveTo(ex, ey);
      c.lineTo(ex + Math.cos(a1 + 2.2) * 12, ey + Math.sin(a1 + 2.2) * 12);
      c.stroke();
    }
    glowCircle(c, 0, 4, 11, '#fef9c3', '#eab308');
  },
  oil_pump: (c) => {
    c.strokeStyle = '#52525b';
    c.lineWidth = 8;
    c.lineCap = 'round';
    c.beginPath();
    c.moveTo(-10, 30);
    c.lineTo(0, -6);
    c.lineTo(10, 30);
    c.stroke();
    c.save();
    c.rotate(-0.25);
    c.beginPath();
    c.roundRect(-36, -14, 64, 12, 5);
    c.fillStyle = '#f59e0b';
    c.fill();
    stroke(c, 3);
    c.beginPath();
    c.moveTo(-36, -18);
    c.quadraticCurveTo(-50, -8, -36, 4);
    c.lineTo(-30, -8);
    c.closePath();
    c.fillStyle = '#f59e0b';
    c.fill();
    stroke(c, 3);
    c.restore();
    c.fillStyle = '#2b2a33';
    c.beginPath();
    c.ellipse(-34, 30, 10, 5, 0, 0, Math.PI * 2);
    c.fill();
  },
  water_pump: (c) => {
    c.fillStyle = '#6b7482';
    c.fillRect(-8, -36, 16, 40);
    c.strokeStyle = OUTLINE;
    c.lineWidth = 3;
    c.strokeRect(-8, -36, 16, 40);
    drop(c, '#3fa7ff');
    c.strokeStyle = '#7dd3fc';
    c.lineWidth = 4;
    for (const y of [30, 40]) {
      c.beginPath();
      c.moveTo(-40, y);
      c.quadraticCurveTo(-20, y - 8, 0, y);
      c.quadraticCurveTo(20, y + 8, 40, y);
      c.stroke();
    }
  },
  refinery: (c) => {
    for (const [x, h, col] of [
      [-18, 62, '#e2e8f0'],
      [16, 48, '#cbd5e1'],
    ] as const) {
      c.beginPath();
      c.roundRect(x - 13, 30 - h, 26, h, 10);
      c.fillStyle = col;
      c.fill();
      stroke(c, 4);
    }
    c.strokeStyle = '#f59e0b';
    c.lineWidth = 6;
    c.beginPath();
    c.moveTo(-5, -4);
    c.lineTo(3, -4);
    c.moveTo(-5, 14);
    c.lineTo(3, 14);
    c.stroke();
    c.fillStyle = '#2b2a33';
    c.fillRect(-28, 6, 20, 5);
  },
  hive: (c) => {
    const hex = (x: number, y: number, r: number) => {
      c.beginPath();
      for (let k = 0; k < 6; k++) {
        const a = (k / 6) * Math.PI * 2;
        c.lineTo(x + Math.cos(a) * r, y + Math.sin(a) * r);
      }
      c.closePath();
    };
    for (const [x, y] of [
      [0, 0],
      [-24, -14],
      [24, -14],
      [-24, 14],
      [24, 14],
      [0, -28],
      [0, 28],
    ]) {
      hex(x, y, 15);
      c.fillStyle = '#fbbf24';
      c.fill();
      stroke(c, 3);
    }
    c.fillStyle = '#1f1300';
    c.beginPath();
    c.ellipse(6, 0, 9, 6, 0, 0, Math.PI * 2);
    c.fill();
    c.fillStyle = 'rgba(255,255,255,0.8)';
    c.beginPath();
    c.ellipse(4, -8, 7, 4, -0.5, 0, Math.PI * 2);
    c.fill();
  },
  hatchery: (c) => {
    c.strokeStyle = '#7dd3fc';
    c.lineWidth = 4;
    for (const y of [20, 32]) {
      c.beginPath();
      c.moveTo(-40, y);
      c.quadraticCurveTo(-20, y - 8, 0, y);
      c.quadraticCurveTo(20, y + 8, 40, y);
      c.stroke();
    }
    c.beginPath();
    c.ellipse(-4, -8, 26, 14, 0, 0, Math.PI * 2);
    c.fillStyle = '#fb923c';
    c.fill();
    stroke(c, 4);
    c.beginPath();
    c.moveTo(20, -8);
    c.lineTo(38, -22);
    c.lineTo(38, 6);
    c.closePath();
    c.fillStyle = '#fb923c';
    c.fill();
    stroke(c, 4);
    c.fillStyle = OUTLINE;
    c.beginPath();
    c.arc(-18, -12, 3.5, 0, Math.PI * 2);
    c.fill();
  },

};

function tunnelArt(c: Ctx, entrance: boolean): void {
  c.beginPath();
  c.moveTo(-30, 30);
  c.lineTo(-30, -6);
  c.arc(0, -6, 30, Math.PI, 0);
  c.lineTo(30, 30);
  c.closePath();
  c.fillStyle = '#0d0f13';
  c.fill();
  c.lineWidth = 6;
  c.strokeStyle = '#6b7482';
  c.stroke();
  c.strokeStyle = entrance ? '#94a3b8' : '#e2e8f0';
  c.lineWidth = 7;
  c.lineCap = 'round';
  c.beginPath();
  if (entrance) {
    c.moveTo(-16, -2);
    c.lineTo(8, -2);
    c.moveTo(-2, -12);
    c.lineTo(8, -2);
    c.lineTo(-2, 8);
  } else {
    c.moveTo(-8, 8);
    c.lineTo(16, 8);
    c.moveTo(6, -2);
    c.lineTo(16, 8);
    c.lineTo(6, 18);
  }
  c.stroke();
}

/** A crashed pod of the first expedition: hull, scorch, hazard stripes and a beacon. */
function wreckArt(c: Ctx): void {
  c.fillStyle = 'rgba(20, 14, 10, 0.55)';
  c.beginPath();
  c.ellipse(4, 10, 54, 34, 0.3, 0, Math.PI * 2);
  c.fill();
  c.save();
  c.rotate(-0.45);
  c.beginPath();
  c.roundRect(-40, -22, 76, 44, 20);
  const g = c.createLinearGradient(0, -22, 0, 22);
  g.addColorStop(0, '#cbd5e1');
  g.addColorStop(1, '#64748b');
  c.fillStyle = g;
  c.fill();
  stroke(c, 5);
  c.save();
  c.clip();
  c.fillStyle = '#f59e0b';
  for (let k = -3; k <= 3; k++) {
    c.beginPath();
    c.moveTo(k * 14 - 6, 22);
    c.lineTo(k * 14 + 2, 22);
    c.lineTo(k * 14 + 14, 8);
    c.lineTo(k * 14 + 6, 8);
    c.closePath();
    c.fill();
  }
  c.restore();
  c.fillStyle = '#0f172a';
  c.beginPath();
  c.roundRect(-26, -12, 22, 14, 5);
  c.fill();
  c.fillStyle = 'rgba(125, 211, 252, 0.6)';
  c.fillRect(-23, -10, 8, 4);
  // A torn-off end.
  c.fillStyle = '#334155';
  c.beginPath();
  c.moveTo(36, -18);
  c.lineTo(44, -6);
  c.lineTo(38, 2);
  c.lineTo(46, 14);
  c.lineTo(34, 20);
  c.closePath();
  c.fill();
  c.restore();
  // Scattered debris and a blinking beacon.
  c.fillStyle = '#94a3b8';
  for (const [x, y, r] of [
    [-44, 26, 5],
    [40, 34, 4],
    [-20, 40, 3],
    [48, -30, 3],
  ]) {
    c.fillRect(x - r, y - r, r * 2, r * 1.4);
  }
  glowCircle(c, 18, -30, 9, '#fff7ed', '#f97316');
}

/** A delivery drone seen from above: X frame, four rotors and a lit body. */
function droneArt(c: Ctx): void {
  c.strokeStyle = '#334155';
  c.lineWidth = 9;
  c.lineCap = 'round';
  c.beginPath();
  c.moveTo(-34, -34);
  c.lineTo(34, 34);
  c.moveTo(34, -34);
  c.lineTo(-34, 34);
  c.stroke();
  for (const [x, y] of [
    [-34, -34],
    [34, -34],
    [-34, 34],
    [34, 34],
  ]) {
    c.beginPath();
    c.arc(x, y, 20, 0, Math.PI * 2);
    c.fillStyle = 'rgba(203, 213, 225, 0.35)';
    c.fill();
    c.lineWidth = 3;
    c.strokeStyle = 'rgba(226, 232, 240, 0.8)';
    c.stroke();
    c.beginPath();
    c.arc(x, y, 5, 0, Math.PI * 2);
    c.fillStyle = '#0f172a';
    c.fill();
  }
  c.beginPath();
  c.roundRect(-18, -16, 36, 32, 9);
  const g = c.createLinearGradient(0, -16, 0, 16);
  g.addColorStop(0, '#f8fafc');
  g.addColorStop(1, '#94a3b8');
  c.fillStyle = g;
  c.fill();
  stroke(c, 4);
  c.beginPath();
  c.roundRect(-10, -8, 20, 16, 4);
  c.fillStyle = '#f59e0b';
  c.fill();
  glowCircle(c, 0, 0, 5, '#ecfeff', '#22d3ee');
}

export class Icons {
  readonly atlas: HTMLCanvasElement;
  private readonly urls = new Map<number, string>();

  constructor(content: Content) {
    this.atlas = document.createElement('canvas');
    this.atlas.width = ATLAS_COLS * ATLAS_CELL;
    this.atlas.height = ATLAS_ROWS * ATLAS_CELL;
    const ctx = this.atlas.getContext('2d')!;
    content.items.forEach((item, id) => {
      const art = ITEM_ART[item.key];
      if (art && id > 0) this.cell(ctx, id, (c) => art(c), 1);
    });
    content.buildings.forEach((b, kind) => {
      if (b.class === 'wreck') {
        this.cell(ctx, Sprite.Building + kind, wreckArt);
        return;
      }
      const art = BUILDING_ART[b.key];
      if (!art) return;
      const accent = CATEGORIES[b.category]?.accent ?? '#94a3b8';
      this.cell(ctx, Sprite.Building + kind, (c) => {
        body(c, accent, b.class === 'terraformer');
        art(c, accent);
        if (['drill', 'crafter', 'tunnel', 'storage', 'pump', 'drone'].includes(b.class)) outArrow(c, accent);
      });
    });
    const tunnelAccent = CATEGORIES[0].accent;
    this.cell(ctx, Sprite.TunnelExit, (c) => {
      body(c, tunnelAccent, false);
      tunnelArt(c, false);
      outArrow(c, tunnelAccent);
    });
    this.cell(ctx, Sprite.Drone, droneArt);
  }

  private cell(ctx: Ctx, index: number, draw: (c: Ctx) => void, scale = 1): void {
    const x = (index % ATLAS_COLS) * ATLAS_CELL;
    const y = Math.floor(index / ATLAS_COLS) * ATLAS_CELL;
    ctx.save();
    ctx.beginPath();
    ctx.rect(x, y, ATLAS_CELL, ATLAS_CELL);
    ctx.clip();
    ctx.translate(x + ATLAS_CELL / 2, y + ATLAS_CELL / 2);
    ctx.scale(scale, scale);
    draw(ctx);
    ctx.restore();
  }

  /** Data URL of an atlas cell, for <img> tags in the HUD (cached). */
  url(index: number): string {
    let u = this.urls.get(index);
    if (!u) {
      const c = document.createElement('canvas');
      c.width = 64;
      c.height = 64;
      const x = (index % ATLAS_COLS) * ATLAS_CELL;
      const y = Math.floor(index / ATLAS_COLS) * ATLAS_CELL;
      c.getContext('2d')!.drawImage(this.atlas, x, y, ATLAS_CELL, ATLAS_CELL, 0, 0, 64, 64);
      u = c.toDataURL();
      this.urls.set(index, u);
    }
    return u;
  }

  /** Image source for an atlas index; -1 is the belt (drawn by the shader in-game). */
  src(index: number): string {
    return index < 0 ? BELT_ICON : this.url(index);
  }

  /** `<img>` markup for an atlas index. */
  img(index: number, size = 20, alt = ''): string {
    return `<img class="icon" src="${this.src(index)}" width="${size}" height="${size}" alt="${alt}" draggable="false">`;
  }
}

/** Atlas index of a building's icon. */
export function buildingIcon(kind: number): number {
  return kind === Kind.Belt ? -1 : Sprite.Building + kind;
}

/** The belt is drawn by the shader, so its icon is a small SVG. */
export const BELT_ICON =
  'data:image/svg+xml,' +
  encodeURIComponent(
    '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><rect x="4" y="16" width="92" height="68" rx="8" fill="#22262d"/><rect x="4" y="26" width="92" height="48" fill="#3a414b"/><path d="M20 34 L36 50 L20 66 M46 34 L62 50 L46 66 M72 34 L88 50 L72 66" stroke="#8b95a3" stroke-width="7" fill="none" stroke-linecap="round"/></svg>',
  );
