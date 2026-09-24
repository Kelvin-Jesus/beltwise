// Player settings (persisted per device): frame-rate cap, graphics quality, night darkness.

/** Frame-rate cap; 0 renders at the display's own rate. */
export type FpsCap = 0 | 30 | 60 | 120;
export type Quality = 'auto' | 'high' | 'medium' | 'low';

export const FPS_CAPS: FpsCap[] = [30, 60, 120, 0];
export const QUALITIES: Quality[] = ['auto', 'high', 'medium', 'low'];

const KEY = 'beltwise.settings';

export class Settings {
  fps: FpsCap = 0;
  quality: Quality = 'auto';
  /** Darken the world at night (visual only; solar power follows the cycle either way). */
  night = true;

  static load(): Settings {
    const s = new Settings();
    try {
      const raw = JSON.parse(localStorage.getItem(KEY) ?? '{}') as Partial<Settings>;
      if (FPS_CAPS.includes(raw.fps as FpsCap)) s.fps = raw.fps as FpsCap;
      if (QUALITIES.includes(raw.quality as Quality)) s.quality = raw.quality as Quality;
      if (typeof raw.night === 'boolean') s.night = raw.night;
    } catch {
      /* defaults */
    }
    return s;
  }

  save(): void {
    try {
      localStorage.setItem(KEY, JSON.stringify({ fps: this.fps, quality: this.quality, night: this.night }));
    } catch {
      /* not persisted */
    }
  }

  /** Highest drawing-buffer scale this quality allows, given the device pixel ratio. */
  maxPixelRatio(dpr: number): number {
    switch (this.quality) {
      case 'low':
        return 1;
      case 'medium':
        return Math.min(dpr, 1.5);
      default:
        return Math.min(dpr, 2);
    }
  }

  /** Only Auto lowers the resolution by itself when frames run late. */
  get adaptive(): boolean {
    return this.quality === 'auto';
  }

  /** Effect level for shaders and particles: 2 full, 1 reduced, 0 minimal. */
  get effects(): number {
    return this.quality === 'low' ? 0 : this.quality === 'medium' ? 1 : 2;
  }
}

const COMMON_RATES = [30, 48, 50, 60, 72, 75, 90, 100, 120, 144, 165, 180, 240];

/** Snaps a measured refresh rate to the nearest common display rate. */
export function snapHz(hz: number): number {
  let best = COMMON_RATES[0];
  for (const r of COMMON_RATES) if (Math.abs(r - hz) < Math.abs(best - hz)) best = r;
  return Math.abs(best - hz) < 6 ? best : Math.round(hz);
}

/**
 * Frame pacing for a cap below the display rate. requestAnimationFrame keeps firing at the
 * display rate; `ready` says whether this callback should render.
 *  - Cap at or above the display rate: every frame.
 *  - Display rate a multiple of the cap (60 on 120 Hz, 30 on 60 Hz): every nth refresh,
 *    judged by elapsed time with half a refresh of tolerance, so jitter never skips one.
 *  - Anything else (120 on 144 Hz): a time budget, which averages the cap exactly.
 */
export class FramePacer {
  private budget = 0;
  private last = 0;
  private lastShown = 0;
  private samples = 0;
  /** Smoothed interval between rAF callbacks: the display's real refresh period. */
  rafMs = 1000 / 60;

  ready(now: number, cap: number): boolean {
    const raw = this.last ? now - this.last : this.rafMs;
    this.last = now;
    if (raw > 2 && raw < 50) this.rafMs += (raw - this.rafMs) * (this.samples++ < 30 ? 0.2 : 0.02);
    const refresh = 1000 / this.rafMs;
    if (cap <= 0 || cap >= refresh * 0.97) return this.show(now);
    const ratio = refresh / cap;
    const n = Math.round(ratio);
    if (Math.abs(ratio - n) < 0.08) {
      return now - this.lastShown >= (n - 0.5) * this.rafMs ? this.show(now) : false;
    }
    const interval = 1000 / cap;
    this.budget = Math.min(2 * interval, this.budget + raw);
    if (this.budget < interval - Math.min(1.5, this.rafMs * 0.25)) return false;
    this.budget -= interval;
    return this.show(now);
  }

  private show(now: number): boolean {
    this.lastShown = now;
    return true;
  }

  get displayHz(): number {
    return snapHz(1000 / this.rafMs);
  }
}
