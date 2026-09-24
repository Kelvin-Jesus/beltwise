// Save games: autosave to localStorage (base64 of the engine's binary snapshot) plus
// export/import as files. localStorage is synchronous, so the save on `pagehide` really
// lands before a mobile browser freezes the tab.

import type { Camera } from './camera';
import type { Engine } from './engine';

const KEY = 'beltwise.save.v2';
const CAM_KEY = 'beltwise.cam';

function toBase64(bytes: Uint8Array): string {
  let s = '';
  for (let i = 0; i < bytes.length; i += 0x8000) {
    s += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  }
  return btoa(s);
}

function fromBase64(b64: string): Uint8Array {
  const s = atob(b64);
  const out = new Uint8Array(s.length);
  for (let i = 0; i < s.length; i++) out[i] = s.charCodeAt(i);
  return out;
}

export class Saves {
  lastSaved = 0;
  /** Saving is paused while the benchmark borrows the engine. */
  paused = false;

  constructor(
    private readonly engine: Engine,
    private readonly cam: Camera,
  ) {}

  hasLocal(): boolean {
    try {
      return !!localStorage.getItem(KEY);
    } catch {
      return false;
    }
  }

  /**
   * Loads the autosave into the engine. Returns when it was saved (seconds since the epoch,
   * 0 if unknown), or null if there is none or it is invalid.
   */
  loadLocal(): number | null {
    try {
      const b64 = localStorage.getItem(KEY);
      if (!b64) return null;
      const savedAt = this.engine.load(fromBase64(b64));
      if (savedAt === null) return null;
      const cam = JSON.parse(localStorage.getItem(CAM_KEY) ?? 'null');
      if (cam && Number.isFinite(cam.x)) Object.assign(this.cam, { x: cam.x, y: cam.y, zoom: cam.zoom });
      return savedAt;
    } catch {
      return null;
    }
  }

  saveLocal(): boolean {
    if (this.paused) return false;
    try {
      localStorage.setItem(KEY, toBase64(this.engine.save()));
      localStorage.setItem(CAM_KEY, JSON.stringify({ x: this.cam.x, y: this.cam.y, zoom: this.cam.zoom }));
      this.lastSaved = Date.now();
      return true;
    } catch {
      return false; // quota exceeded or storage disabled; export still works
    }
  }

  clearLocal(): void {
    try {
      localStorage.removeItem(KEY);
      localStorage.removeItem(CAM_KEY);
    } catch {
      /* nothing to clear */
    }
  }

  exportFile(): void {
    const blob = new Blob([this.engine.save() as BlobPart], { type: 'application/octet-stream' });
    const a = document.createElement('a');
    const d = new Date();
    const stamp = `${d.getFullYear()}${String(d.getMonth() + 1).padStart(2, '0')}${String(d.getDate()).padStart(2, '0')}-${String(d.getHours()).padStart(2, '0')}${String(d.getMinutes()).padStart(2, '0')}`;
    a.href = URL.createObjectURL(blob);
    a.download = `beltwise-${stamp}.bwsave`;
    a.click();
    setTimeout(() => URL.revokeObjectURL(a.href), 1000);
  }

  /** Opens a file picker; resolves true if a valid save was loaded. */
  importFile(): Promise<boolean> {
    return new Promise((resolve) => {
      const input = document.createElement('input');
      input.type = 'file';
      input.accept = '.bwsave,application/octet-stream';
      input.onchange = async () => {
        const file = input.files?.[0];
        if (!file) return resolve(false);
        const ok = this.engine.load(new Uint8Array(await file.arrayBuffer())) !== null;
        if (ok) this.saveLocal();
        resolve(ok);
      };
      input.click();
    });
  }

  describe(): string {
    if (!this.lastSaved) return '';
    const s = Math.round((Date.now() - this.lastSaved) / 1000);
    return `Last saved ${s < 5 ? 'just now' : s < 60 ? `${s}s ago` : `${Math.round(s / 60)} min ago`}.`;
  }
}
