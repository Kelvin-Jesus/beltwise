// In-game benchmark: builds a large test factory in a separate world, flies the camera
// through close-up, mid and map zoom levels, records frame metrics, then restores the
// player's game exactly as it was.

import type { Camera } from './camera';
import { Stat } from './constants';
import type { Engine } from './engine';

interface Shot {
  name: string;
  from: [x: number, y: number, zoom: number];
  to: [x: number, y: number, zoom: number];
  seconds: number;
}

const SHOTS: Shot[] = [
  { name: 'Close-up: 4k busy machines', from: [60, 285, 38], to: [300, 300, 38], seconds: 5 },
  { name: 'Mid zoom: 40k moving items', from: [104, 40, 12], to: [104, 190, 12], seconds: 5 },
  { name: 'Dense items, zooming in', from: [60, 60, 9], to: [60, 60, 30], seconds: 4 },
  { name: 'Whole map (far LOD)', from: [256, 200, 2.2], to: [256, 200, 5.5], seconds: 4 },
];
const WARMUP_S = 1.5;

export interface FrameSample {
  dt: number;
  simMs: number;
  buildMs: number;
  drawMs: number;
  instances: number;
}

interface ShotStats {
  frames: FrameSample[];
}

export class Bench {
  running = false;
  private snapshot: Uint8Array | null = null;
  private camBefore = { x: 0, y: 0, zoom: 0 };
  private start = 0;
  private shots: ShotStats[] = [];
  private counts = { belts: 0, items: 0, machines: 0 };

  constructor(
    private readonly engine: Engine,
    private readonly cam: Camera,
    private readonly hooks: { onWorldChanged(): void; onDone(html: string): void },
  ) {}

  begin(): void {
    const { engine, cam } = this;
    this.snapshot = engine.save();
    this.camBefore = { x: cam.x, y: cam.y, zoom: cam.zoom };
    engine.reset(0xbe7c);
    engine.x.fx_sandbox(1);
    engine.x.fx_bench(50);
    engine.x.fx_tick(180); // let the production lines fill up
    const s = engine.stats;
    this.counts = { belts: s[Stat.Belts], items: s[Stat.Items], machines: s[Stat.Machines] };
    this.hooks.onWorldChanged();
    this.shots = SHOTS.map(() => ({ frames: [] }));
    this.start = performance.now();
    this.running = true;
  }

  /** Drives the camera; returns false once the run is over. */
  update(now: number): boolean {
    let t = (now - this.start) / 1000 - WARMUP_S;
    const first = SHOTS[0];
    if (t < 0) {
      [this.cam.x, this.cam.y, this.cam.zoom] = first.from;
      return true;
    }
    for (const shot of SHOTS) {
      if (t < shot.seconds) {
        const k = t / shot.seconds;
        const e = k * k * (3 - 2 * k);
        this.cam.x = shot.from[0] + (shot.to[0] - shot.from[0]) * e;
        this.cam.y = shot.from[1] + (shot.to[1] - shot.from[1]) * e;
        this.cam.zoom = shot.from[2] * Math.pow(shot.to[2] / shot.from[2], e);
        return true;
      }
      t -= shot.seconds;
    }
    this.finish();
    return false;
  }

  record(sample: FrameSample): void {
    let t = (performance.now() - this.start) / 1000 - WARMUP_S;
    if (t < 0) return;
    for (let i = 0; i < SHOTS.length; i++) {
      if (t < SHOTS[i].seconds) {
        this.shots[i].frames.push(sample);
        return;
      }
      t -= SHOTS[i].seconds;
    }
  }

  private finish(): void {
    this.running = false;
    const html = this.report();
    if (this.snapshot) this.engine.load(this.snapshot);
    Object.assign(this.cam, this.camBefore);
    this.snapshot = null;
    this.hooks.onWorldChanged();
    this.hooks.onDone(html);
  }

  private report(): string {
    const rows = SHOTS.map((shot, i) => {
      const f = this.shots[i].frames;
      if (!f.length) return `<tr><td>${shot.name}</td><td colspan="5">no frames</td></tr>`;
      const dts = f.map((x) => x.dt).sort((a, b) => a - b);
      const avg = (fn: (x: FrameSample) => number) => f.reduce((s, x) => s + fn(x), 0) / f.length;
      const fps = 1000 / avg((x) => x.dt);
      const low = 1000 / dts[Math.min(dts.length - 1, Math.floor(dts.length * 0.99))];
      return `<tr><td>${shot.name}</td><td><b>${fps.toFixed(0)}</b></td><td>${low.toFixed(0)}</td><td>${avg((x) => x.simMs).toFixed(2)}</td><td>${avg((x) => x.buildMs + x.drawMs).toFixed(2)}</td><td>${Math.round(avg((x) => x.instances)).toLocaleString()}</td></tr>`;
    }).join('');
    const all = this.shots.flatMap((s) => s.frames);
    const fps = all.length ? (1000 * all.length) / all.reduce((s, x) => s + x.dt, 0) : 0;
    const gl = document.createElement('canvas').getContext('webgl2');
    const dbg = gl?.getExtension('WEBGL_debug_renderer_info');
    const gpu = dbg ? String(gl!.getParameter(dbg.UNMASKED_RENDERER_WEBGL)) : 'unknown GPU';
    const c = this.counts;
    return `<div class="bench">
      <div class="ti-big"><b>${fps.toFixed(0)}</b> fps <small>average over ${all.length} frames</small></div>
      <p class="hint">${c.belts.toLocaleString()} belts · ${c.items.toLocaleString()} items · ${c.machines.toLocaleString()} machines, simulated at 60 ticks/s while rendering.</p>
      <table><thead><tr><th>Shot</th><th>FPS</th><th>1% low</th><th>Sim ms</th><th>Render CPU ms</th><th>Sprites</th></tr></thead><tbody>${rows}</tbody></table>
      <p class="hint">${gpu} · ${Math.round(this.cam.width)}×${Math.round(this.cam.height)} CSS px at ${this.cam.pixelRatio}x. Sim ms = Wasm simulation time per frame; Render CPU = instance building + GPU submit.</p>
    </div>`;
  }
}
