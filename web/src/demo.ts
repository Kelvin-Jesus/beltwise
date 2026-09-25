// Live demos for the guide. A second engine instance runs a small sandbox map; scripted
// scenes (scenes.ts) build a few buildings on it, and the game's own renderer draws it into
// a canvas inside the guide sheet, so every item and building is shown really working.
// This file plays scenes: it fits the camera, runs belts in from supplies hidden past the
// edge of the view and out to hidden sinks, steps through timed captions, and pins labels
// and live readouts over the canvas. Nothing here touches the player's world.

import { Camera } from './camera';
import { DX, DY, Lod, Stat, TICK_MS } from './constants';
import { BELT_COLORS, METER_COLORS, formatCount, formatPower, type Content } from './content';
import { Engine } from './engine';
import type { Icons } from './icons';
import { hexToRgb, Renderer, type FrameParams } from './renderer';

/** Demo map size, and the world tile that scene coordinates (0, 0) map to. */
const W = 64;
const H = 40;
const OX = 30;
const OY = 20;
/** Margin around a scene's bounds, in tiles. */
const PAD = 0.6;
const BELT = 1;
const STORAGE = 28;
const INCINERATOR = 20;
/** Ground elevation that holds a lake once the planet is wet. */
const LAKE = 0;
/** Meters that guarantee liquid water (a little past the Liquid Water stage). */
const WET_HEAT = 90_000;
const WET_PRESSURE = 70_000;

export type Side = 'e' | 's' | 'w' | 'n';
/** Direction ids (E, S, W, N) of each side. */
export const SIDE: Record<Side, number> = { e: 0, s: 1, w: 2, n: 3 };
const SIDES: Side[] = ['e', 's', 'w', 'n'];

export type TagSide = 'below' | 'above' | 'left' | 'right';

export interface LabelOptions {
  /** Where the label sits relative to the tile (default below). */
  at?: TagSide;
  /** Item or building icon (atlas index) shown before the text. */
  icon?: number;
}

export interface PutOptions {
  /** Recipe index to lock (crafters with several recipes). */
  recipe?: number;
  /** Sorter filter. */
  filter?: number;
  label?: string;
  /** Pulsing ring: the building this scene is about. */
  focus?: boolean;
}

/** Live readouts pinned to the scene. */
export type Watch =
  /** Floating "+N Heat" whenever that meter grows. */
  | { kind: 'meter'; x: number; y: number; meter: number }
  /** A tag counting items delivered to the Core. */
  | { kind: 'delivered'; x: number; y: number }
  /** Floating "+N cr" as a Recycler earns credits. */
  | { kind: 'credits'; x: number; y: number }
  /** A tag counting what a Storage holds. */
  | { kind: 'stored'; x: number; y: number }
  /** Power used / available, in the caption bar. */
  | { kind: 'power' }
  /** Battery charge, in the caption bar. */
  | { kind: 'battery' };

/** What scenes build with. Coordinates are scene tiles; directions are E, S, W, N = 0..3. */
export interface SceneCtx {
  readonly content: Content;
  /** A building; `dir` is where its output arrow points. */
  put(x: number, y: number, kind: number, dir?: number, opt?: PutOptions): void;
  /** Belts along the points (straight runs), each tile pointing at the next; the last tile
   * points `end` (default: the way the last run goes). */
  path(points: [number, number][], end?: number): void;
  ore(x0: number, y0: number, x1: number, y1: number, item: number, purity?: number): void;
  /** Low ground that holds water (scenes that need it are `wet`). */
  pond(x0: number, y0: number, x1: number, y1: number): void;
  /** `item` arrives from off-screen on `side` and enters the building at (x, y). */
  feed(x: number, y: number, side: Side, item: number): void;
  /** What leaves (x, y) toward `side` runs off-screen. */
  exit(x: number, y: number, side: Side): void;
  label(x: number, y: number, text: string, opt?: LabelOptions): void;
  focus(x: number, y: number): void;
  /** Frames these tiles instead of everything built (scenes that grow over time). */
  view(x0: number, y0: number, x1: number, y1: number): void;
  link(x: number, y: number, tx: number, ty: number): void;
  shards(x: number, y: number, n: number): void;
  amp(x: number, y: number): void;
  wreck(x: number, y: number): void;
  watch(w: Watch): void;
}

export interface Step {
  /** Seconds after the scene starts. */
  at: number;
  caption?: string;
  run?(c: SceneCtx): void;
}

export interface Scene {
  caption: string;
  /** Top-left tile of the Core (4x4), if the scene has one. */
  core?: [number, number];
  /** Machines need a real power grid; otherwise everything runs without one. */
  grid?: boolean;
  fog?: boolean;
  /** Lakes hold water. */
  wet?: boolean;
  /** Show power coverage. */
  overlay?: boolean;
  /** Show problem badges on stuck machines. */
  badges?: boolean;
  /** Machine speed in percent (slow recipes play faster). */
  speed?: number;
  /** Ticks simulated before the first frame, so belts are already busy. Default 360. */
  warmup?: number;
  /** Seconds before the scene starts over. */
  loop?: number;
  build(c: SceneCtx): void;
  steps?: Step[];
}

interface Tag {
  x: number;
  y: number;
  el: HTMLElement;
  at: TagSide | 'center';
}

interface Live {
  w: Watch;
  last: number;
  /** Gain not shown yet, and when (scene seconds) the last one floated up. */
  pending: number;
  shown: number;
  el?: HTMLElement;
}

export interface DemoQuality {
  /** Effect level (settings). */
  fx: number;
  /** Highest device pixel ratio to render at. */
  ratio: number;
}

export class DemoPlayer {
  /** The player: canvas, labels and caption bar. Moved into whichever slot plays. */
  readonly el: HTMLElement;
  /** What is playing (a scene id), or '' when stopped. */
  id = '';
  private readonly canvas: HTMLCanvasElement;
  private readonly layer: HTMLElement;
  private readonly captionEl: HTMLElement;
  private readonly liveEl: HTMLElement;
  private readonly speedEl: HTMLElement;
  private engine: Engine | null = null;
  private renderer: Renderer | null = null;
  private loading: Promise<boolean> | null = null;
  private readonly cam = new Camera(W, H);
  private scene: Scene | null = null;
  private slot: HTMLElement | null = null;
  private ctx!: SceneCtx;
  private t = 0;
  private acc = 0;
  private next = 0;
  private hudAt = 0;
  private deliveredBase = 0;
  private bounds = [0, 0, 0, 0];
  private framed: number[] | null = null;
  /** The visible rectangle in scene tiles (left, top, right, bottom). */
  private vis = [0, 0, 0, 0];
  private laidOut = false;
  private deferred: (() => void)[] = [];
  private tags: Tag[] = [];
  private lives: Live[] = [];
  private readonly terra = new Float32Array(4);
  private readonly ambient = new Float32Array([1, 1, 1]);
  private readonly stripe = new Float32Array(3);
  private readonly fp: FrameParams;

  constructor(
    private readonly main: Engine,
    private readonly icons: Icons,
    private readonly quality: () => DemoQuality,
  ) {
    this.el = document.createElement('div');
    this.el.className = 'demo';
    this.el.innerHTML = `<div class="demo-view"><canvas aria-label="Live demo"></canvas><div class="demo-layer"></div>
      <div class="demo-hud"><span class="demo-live"></span><span class="demo-speed" hidden></span></div></div>
      <div class="demo-bar"><span class="demo-caption" aria-live="polite"></span>
      <button class="demo-btn" type="button" data-act="demo:replay" title="Play again" aria-label="Play again">↻</button>
      <button class="demo-btn" type="button" data-act="demo:stop" title="Close the demo" aria-label="Close the demo">✕</button></div>`;
    this.canvas = this.el.querySelector('canvas')!;
    this.layer = this.el.querySelector('.demo-layer')!;
    this.captionEl = this.el.querySelector('.demo-caption')!;
    this.liveEl = this.el.querySelector('.demo-live')!;
    this.speedEl = this.el.querySelector('.demo-speed')!;
    this.fp = {
      cam: this.cam,
      instances: 0,
      beltPhase: 0,
      time: 0,
      terra: this.terra,
      map: false,
      fx: 2,
      ambient: this.ambient,
      overlay: 0,
      planet: 0,
      stripe: this.stripe,
      stage: 0,
    };
    new ResizeObserver(() => {
      if (this.scene && this.laidOut) this.fit();
    }).observe(this.canvas);
  }

  get playing(): boolean {
    return this.scene !== null;
  }

  /** Plays `scene` in `slot` (an element in the guide sheet). */
  async play(id: string, scene: Scene, slot: HTMLElement): Promise<void> {
    this.id = id;
    this.attach(slot);
    this.setCaption('Loading…');
    if (!(await this.ensure())) {
      this.setCaption('Demos need WebGL2, which is not available here.');
      return;
    }
    if (this.id !== id) return; // something else was asked for meanwhile
    this.scene = scene;
    this.start();
  }

  /** Moves the player into `slot` (after the sheet re-rendered its HTML). */
  attach(slot: HTMLElement): void {
    if (this.slot && this.slot !== slot) this.slot.classList.remove('playing');
    this.slot = slot;
    slot.classList.add('playing');
    if (this.el.parentElement !== slot) slot.appendChild(this.el);
  }

  stop(): void {
    this.scene = null;
    this.id = '';
    this.slot?.classList.remove('playing');
    this.slot = null;
    this.el.remove();
  }

  replay(): void {
    if (this.scene) this.start();
  }

  private ensure(): Promise<boolean> {
    this.loading ??= (async () => {
      try {
        this.engine = await Engine.load(W, H, 1);
        this.renderer = new Renderer(this.canvas, this.engine, this.engine.content, this.icons.atlas);
        this.canvas.addEventListener('webglcontextlost', (e) => {
          e.preventDefault();
          this.stop();
          this.loading = Promise.resolve(false);
        });
        return true;
      } catch (err) {
        console.warn('Demo player unavailable', err);
        return false;
      }
    })();
    return this.loading;
  }

  // ---- Building a scene ---------------------------------------------------------------------

  private start(): void {
    const e = this.engine!;
    const scene = this.scene!;
    const m = this.main;
    const meters = [0, 1, 2, 3].map((i) => m.stat64(Stat.Meters + i * 2));
    if (scene.wet) {
      meters[0] = Math.max(meters[0], WET_HEAT);
      meters[1] = Math.max(meters[1], WET_PRESSURE);
    }
    const core = scene.core ? [OX + scene.core[0], OY + scene.core[1]] : [-1, -1];
    const options = (scene.grid ? 0 : 1) | (scene.fog ? 2 : 0);
    e.x.fx_demo(m.stats[Stat.Planet], core[0], core[1], options, meters[0], meters[1], meters[2], meters[3]);
    e.x.fx_demo_speed(scene.speed ?? 100);
    e.refreshViews();
    this.t = 0;
    this.acc = 0;
    this.next = 0;
    this.hudAt = 0;
    this.bounds = [Infinity, Infinity, -Infinity, -Infinity];
    this.framed = null;
    this.laidOut = false;
    this.deferred = [];
    this.tags = [];
    this.lives = [];
    this.layer.textContent = '';
    this.liveEl.textContent = '';
    const speed = scene.speed ?? 100;
    this.speedEl.hidden = speed <= 100;
    this.speedEl.textContent = `${(speed / 100).toFixed(speed % 100 ? 1 : 0)}× speed`;
    if (scene.core) this.grow(scene.core[0], scene.core[1], scene.core[0] + 3, scene.core[1] + 3);
    this.ctx = this.makeCtx(e);
    scene.build(this.ctx);
    this.fit();
    this.deliveredBase = e.stats[Stat.Delivered];
    const warm = scene.warmup ?? 360;
    if (warm > 0) e.x.fx_tick(warm);
    this.renderer!.uploadWorld();
    this.setCaption(scene.caption);
    this.setLook();
    this.watchTick(true);
  }

  private makeCtx(e: Engine): SceneCtx {
    const x = e.x;
    const at = (tx: number, ty: number): [number, number] => [OX + tx, OY + ty];
    const ctx: SceneCtx = {
      content: e.content,
      put: (tx, ty, kind, dir = 0, opt = {}) => {
        x.fx_set_place(opt.recipe ?? 255, opt.filter ?? 0);
        x.fx_place(...at(tx, ty), kind, dir);
        x.fx_set_place(255, 0);
        this.grow(tx, ty, tx, ty);
        if (opt.label) ctx.label(tx, ty, opt.label);
        if (opt.focus) ctx.focus(tx, ty);
      },
      path: (points, end) => {
        for (let k = 0; k + 1 < points.length; k++) {
          const [ax, ay] = points[k];
          const [bx, by] = points[k + 1];
          const d = dirTo(ax, ay, bx, by);
          const n = Math.max(Math.abs(bx - ax), Math.abs(by - ay));
          for (let s = 0; s < n; s++) {
            x.fx_place(...at(ax + DX[d] * s, ay + DY[d] * s), BELT, d);
            this.grow(ax + DX[d] * s, ay + DY[d] * s, ax + DX[d] * s, ay + DY[d] * s);
          }
        }
        const last = points[points.length - 1];
        const prev = points[points.length - 2] ?? last;
        const d = end ?? (points.length > 1 ? dirTo(prev[0], prev[1], last[0], last[1]) : 0);
        x.fx_place(...at(last[0], last[1]), BELT, d);
        this.grow(last[0], last[1], last[0], last[1]);
      },
      ore: (x0, y0, x1, y1, item, purity = 1) => {
        for (let ty = y0; ty <= y1; ty++) for (let tx = x0; tx <= x1; tx++) x.fx_demo_ore(...at(tx, ty), item, purity);
        this.grow(x0, y0, x1, y1);
      },
      pond: (x0, y0, x1, y1) => {
        for (let ty = y0; ty <= y1; ty++) for (let tx = x0; tx <= x1; tx++) x.fx_demo_ground(...at(tx, ty), LAKE);
        this.grow(x0, y0, x1, y1);
      },
      feed: (tx, ty, side, item) => this.defer(() => this.runIn(tx, ty, side, item)),
      exit: (tx, ty, side) => this.defer(() => this.runOut(tx, ty, side)),
      label: (tx, ty, text, opt = {}) => {
        const el = document.createElement('span');
        el.className = 'demo-tag';
        if (opt.icon !== undefined) el.innerHTML = this.icons.img(opt.icon, 14);
        el.append(text);
        this.pin({ x: tx, y: ty, el, at: opt.at ?? 'below' });
      },
      focus: (tx, ty) => {
        const el = document.createElement('span');
        el.className = 'demo-focus';
        this.pin({ x: tx, y: ty, el, at: 'center' });
      },
      view: (x0, y0, x1, y1) => {
        this.framed = [x0, y0, x1, y1];
      },
      link: (ax, ay, bx, by) => void x.fx_link(...at(ax, ay), ...at(bx, by)),
      shards: (tx, ty, n) => void x.fx_set_shards(...at(tx, ty), n),
      amp: (tx, ty) => void x.fx_set_amp(...at(tx, ty), 1),
      wreck: (tx, ty) => {
        x.fx_demo_wreck(...at(tx, ty));
      },
      watch: (w) => {
        const live: Live = { w, last: this.readWatch(w), pending: 0, shown: -1 };
        if (w.kind === 'delivered' || w.kind === 'stored') {
          live.el = document.createElement('span');
          live.el.className = 'demo-tag count';
          this.pin({ x: w.x, y: w.y, el: live.el, at: w.kind === 'stored' ? 'above' : 'below' });
        }
        this.lives.push(live);
      },
    };
    return ctx;
  }

  private grow(x0: number, y0: number, x1: number, y1: number): void {
    const b = this.bounds;
    b[0] = Math.min(b[0], x0);
    b[1] = Math.min(b[1], y0);
    b[2] = Math.max(b[2], x1);
    b[3] = Math.max(b[3], y1);
  }

  /** Belts that start off-screen need the camera, so they wait for the first layout. */
  private defer(f: () => void): void {
    if (this.laidOut) f();
    else this.deferred.push(f);
  }

  /** Whether tile (tx, ty) shows at all (scene coordinates). */
  private visible(tx: number, ty: number): boolean {
    const [l, t, r, b] = this.vis;
    return tx + 1 > l && tx < r && ty + 1 > t && ty < b;
  }

  /** A supply of `item` past the edge of the view, belted into (tx, ty) from `side`. */
  private runIn(tx: number, ty: number, side: Side, item: number): void {
    const x = this.engine!.x;
    const s = SIDE[side];
    const d = (s + 2) & 3; // items travel away from that side
    let k = 1;
    while (this.visible(tx + DX[s] * k, ty + DY[s] * k) && k < 30) k++;
    for (let i = 1; i <= k; i++) x.fx_place(OX + tx + DX[s] * i, OY + ty + DY[s] * i, BELT, d);
    const [sx, sy] = [OX + tx + DX[s] * (k + 1), OY + ty + DY[s] * (k + 1)];
    x.fx_place(sx, sy, STORAGE, d);
    x.fx_demo_source(sx, sy, item);
  }

  /** Belts from (tx, ty) toward `side` until past the edge of the view, into a sink. */
  private runOut(tx: number, ty: number, side: Side): void {
    const x = this.engine!.x;
    const d = SIDE[side];
    let k = 1;
    while (this.visible(tx + DX[d] * k, ty + DY[d] * k) && k < 30) k++;
    for (let i = 1; i <= k; i++) x.fx_place(OX + tx + DX[d] * i, OY + ty + DY[d] * i, BELT, d);
    x.fx_place(OX + tx + DX[d] * (k + 1), OY + ty + DY[d] * (k + 1), INCINERATOR, d);
  }

  /** Fits the camera to the scene and places labels. */
  private fit(): void {
    const cw = this.canvas.clientWidth || 320;
    const ch = this.canvas.clientHeight || 180;
    const q = this.quality();
    const ratio = Math.max(1, Math.min(q.ratio, window.devicePixelRatio || 1, 2));
    const cam = this.cam;
    cam.width = cw;
    cam.height = ch;
    cam.pixelRatio = ratio;
    this.renderer!.resize(Math.round(cw * ratio), Math.round(ch * ratio));
    const b = this.framed ?? (Number.isFinite(this.bounds[0]) ? this.bounds : [0, 0, 0, 0]);
    const w = b[2] - b[0] + 1 + 2 * PAD;
    const h = b[3] - b[1] + 1 + 2 * PAD;
    cam.zoom = Math.min(cw / w, ch / h);
    cam.x = OX + (b[0] + b[2] + 1) / 2;
    cam.y = OY + (b[1] + b[3] + 1) / 2;
    this.vis = [cam.left - OX, cam.top - OY, cam.right - OX, cam.bottom - OY];
    if (!this.laidOut) {
      this.laidOut = true;
      for (const f of this.deferred) f();
      this.deferred = [];
    }
    for (const tag of this.tags) this.place(tag);
  }

  private pin(tag: Tag): void {
    this.tags.push(tag);
    this.layer.appendChild(tag.el);
    if (this.laidOut) this.place(tag);
  }

  private place(tag: Tag): void {
    const cam = this.cam;
    const z = cam.zoom;
    const px = (OX + tag.x + 0.5 - cam.left) * z;
    const py = (OY + tag.y + 0.5 - cam.top) * z;
    const st = tag.el.style;
    switch (tag.at) {
      case 'center':
        st.width = st.height = `${z * 1.3}px`;
        st.transform = `translate(${px}px, ${py}px) translate(-50%, -50%)`;
        break;
      case 'above':
        st.transform = `translate(${px}px, ${py - 0.52 * z}px) translate(-50%, -100%)`;
        break;
      case 'left':
        st.transform = `translate(${px - 0.56 * z}px, ${py}px) translate(-100%, -50%)`;
        break;
      case 'right':
        st.transform = `translate(${px + 0.56 * z}px, ${py}px) translate(0, -50%)`;
        break;
      default:
        st.transform = `translate(${px}px, ${py + 0.52 * z}px) translate(-50%, 0)`;
    }
  }

  private setCaption(text: string): void {
    this.captionEl.textContent = text;
  }

  /** Terraforming look (from the demo world's meters) and the player's belt colour. */
  private setLook(): void {
    const e = this.engine!;
    const s = e.stats;
    const meters = e.content.meters;
    const level = (v: number, full: number) =>
      Math.min(1, Math.max(0, (Math.log10(1 + v) - 2) / (Math.log10(full) - 2)));
    this.terra[0] = level(e.stat64(Stat.Meters), meters[0].full);
    this.terra[1] = level(e.stat64(Stat.Meters + 2), meters[1].full) * (0.35 + 0.65 * this.terra[0]);
    this.terra[2] = level(e.stat64(Stat.Meters + 4), meters[2].full);
    this.terra[3] = level(e.stat64(Stat.Meters + 6), meters[3].full);
    const stage = s[Stat.Stage];
    const here = e.content.stages[stage];
    const next = e.content.stages[stage + 1];
    const ti = e.stat64(Stat.TiLo);
    this.fp.stage = stage + (next ? Math.min(1, Math.max(0, (ti - here.ti) / (next.ti - here.ti))) : 0);
    this.fp.planet = s[Stat.Planet];
    const color = BELT_COLORS[this.main.stats[Stat.BeltColor]] ?? BELT_COLORS[0];
    this.stripe.set(hexToRgb(color));
  }

  // ---- Playing -------------------------------------------------------------------------------

  /** Advances and draws the demo; the game's frame loop calls this while a scene plays. */
  frame(dt: number, now: number): void {
    const scene = this.scene;
    const e = this.engine;
    if (!scene || !e || !this.renderer) return;
    if (!this.el.isConnected) {
      this.stop();
      return;
    }
    this.t += dt / 1000;
    const steps = scene.steps;
    while (steps && this.next < steps.length && steps[this.next].at <= this.t) {
      const step = steps[this.next++];
      step.run?.(this.ctx);
      if (step.caption) this.setCaption(step.caption);
    }
    if (scene.loop && this.t >= scene.loop) {
      this.start();
      return;
    }
    this.acc += dt;
    let n = Math.floor(this.acc / TICK_MS);
    if (n > 4) {
      n = 4;
      this.acc = 0;
    } else this.acc -= n * TICK_MS;
    if (n > 0) e.x.fx_tick(n);
    const alpha = this.acc / TICK_MS;
    const cam = this.cam;
    const flags = Lod.Items | Lod.Structures | (scene.badges ? Lod.Status : 0);
    const fp = this.fp;
    fp.instances = e.x.fx_render(cam.left, cam.top, cam.right, cam.bottom, alpha, flags);
    fp.beltPhase = ((e.stats[Stat.Tick] - 1 + alpha) * e.beltSpeed()) % 1;
    fp.time = now / 1000;
    fp.fx = this.quality().fx;
    fp.overlay = scene.overlay ? 1 : 0;
    this.renderer.draw(fp);
    if (now - this.hudAt > 150) {
      this.hudAt = now;
      this.watchTick(false);
    }
  }

  private readWatch(w: Watch): number {
    const e = this.engine!;
    const s = e.stats;
    switch (w.kind) {
      case 'meter':
        return e.stat64(Stat.Meters + w.meter * 2);
      case 'delivered':
        return s[Stat.Delivered];
      case 'credits':
        return e.credits;
      case 'stored': {
        const info = e.inspect(OX + w.x, OY + w.y);
        return info && info[10] !== 0xffffffff ? info[10] : 0;
      }
      default:
        return 0;
    }
  }

  private watchTick(reset: boolean): void {
    const e = this.engine!;
    const s = e.stats;
    const live: string[] = [];
    for (const l of this.lives) {
      const w = l.w;
      if (w.kind === 'power') {
        live.push(`⚡ ${formatPower(s[Stat.PowerDemand])} used of ${formatPower(s[Stat.PowerSupply])}`);
        continue;
      }
      if (w.kind === 'battery') {
        const cap = s[Stat.BatteryCapMj];
        live.push(`Battery ${cap ? Math.round((100 * s[Stat.BatteryMj]) / cap) : 0}%`);
        continue;
      }
      const v = this.readWatch(w);
      if (reset) l.last = v;
      if (w.kind === 'delivered') {
        l.el!.textContent = `${formatCount(Math.max(0, v - this.deliveredBase))} delivered`;
      } else if (w.kind === 'stored') {
        l.el!.textContent = `${formatCount(v)} stored`;
      } else {
        // Gains float up from the building, batched so they stay readable.
        l.pending += Math.max(0, v - l.last);
        l.last = v;
        if (l.pending > 0 && this.t - l.shown > 0.8) {
          const text =
            w.kind === 'meter' ? `+${formatCount(l.pending)} ${e.content.meters[w.meter].name}` : `+${formatCount(l.pending)} cr`;
          this.float(w.x, w.y, text, w.kind === 'meter' ? METER_COLORS[w.meter] : '#fbbf24');
          l.pending = 0;
          l.shown = this.t;
        }
      }
    }
    this.liveEl.textContent = live.join(' · ');
  }

  private float(x: number, y: number, text: string, color: string): void {
    const el = document.createElement('span');
    el.className = 'demo-float';
    el.textContent = text;
    el.style.color = color;
    const cam = this.cam;
    el.style.left = `${(OX + x + 0.5 - cam.left) * cam.zoom}px`;
    el.style.top = `${(OY + y + 0.1 - cam.top) * cam.zoom}px`;
    this.layer.appendChild(el);
    setTimeout(() => el.remove(), 1400);
  }
}

function dirTo(ax: number, ay: number, bx: number, by: number): number {
  if (bx > ax) return 0;
  if (by > ay) return 1;
  if (bx < ax) return 2;
  return 3;
}

/** The side of (x, y) that faces (tx, ty) — handy when wiring scenes. */
export function sideTowards(x: number, y: number, tx: number, ty: number): Side {
  return SIDES[dirTo(x, y, tx, ty)];
}
