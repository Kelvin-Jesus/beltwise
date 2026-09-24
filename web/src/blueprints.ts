// Blueprint library: named blueprints saved on this device, plus share codes. The engine
// owns the active blueprint; this module only stores and exchanges its bytes.

import { BP_CELL_BYTES } from './constants';

const KEY = 'beltwise.blueprints';
const CODE_PREFIX = 'BW1.';

export interface StoredBlueprint {
  id: string;
  name: string;
  cells: number;
  /** Serialized cells, base64. */
  data: string;
}

function toBase64(bytes: Uint8Array): string {
  let s = '';
  for (let i = 0; i < bytes.length; i += 0x8000) s += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  return btoa(s);
}

function fromBase64(b64: string): Uint8Array | null {
  try {
    const s = atob(b64);
    const out = new Uint8Array(s.length);
    for (let i = 0; i < s.length; i++) out[i] = s.charCodeAt(i);
    return out;
  } catch {
    return null;
  }
}

export class Blueprints {
  private list: StoredBlueprint[] = [];

  constructor() {
    try {
      const raw = JSON.parse(localStorage.getItem(KEY) ?? '[]');
      if (Array.isArray(raw)) this.list = raw.filter((b) => b && typeof b.data === 'string');
    } catch {
      this.list = [];
    }
  }

  private persist(): boolean {
    try {
      localStorage.setItem(KEY, JSON.stringify(this.list));
      return true;
    } catch {
      return false;
    }
  }

  all(): StoredBlueprint[] {
    return this.list;
  }

  get(id: string): StoredBlueprint | undefined {
    return this.list.find((b) => b.id === id);
  }

  bytes(id: string): Uint8Array | null {
    const b = this.get(id);
    return b ? fromBase64(b.data) : null;
  }

  add(name: string, bytes: Uint8Array): StoredBlueprint | null {
    const bp: StoredBlueprint = {
      id: `${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`,
      name,
      cells: Math.floor(bytes.length / BP_CELL_BYTES),
      data: toBase64(bytes),
    };
    this.list.unshift(bp);
    return this.persist() ? bp : null;
  }

  rename(id: string, name: string): void {
    const b = this.get(id);
    if (b && name.trim()) {
      b.name = name.trim().slice(0, 40);
      this.persist();
    }
  }

  remove(id: string): void {
    this.list = this.list.filter((b) => b.id !== id);
    this.persist();
  }

  /** A text code for sharing (base64 of the cells). */
  code(id: string): string {
    const b = this.get(id);
    return b ? `${CODE_PREFIX}${b.data}` : '';
  }

  /** Bytes from a share code, or null if it isn't one. */
  static parse(code: string): Uint8Array | null {
    const t = code.trim();
    if (!t.startsWith(CODE_PREFIX)) return null;
    const bytes = fromBase64(t.slice(CODE_PREFIX.length));
    return bytes && bytes.length > 0 && bytes.length % BP_CELL_BYTES === 0 ? bytes : null;
  }
}
