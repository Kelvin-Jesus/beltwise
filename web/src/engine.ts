// Loads the Wasm simulation core and exposes typed, allocation-free access to it.

import { ABI_VERSION, INFO_LEN, Stat, STATS_LEN } from './constants';
import type { Content } from './content';

/** Raw exports of engine/src/lib.rs. Pointers are byte offsets into `memory`. */
export interface Exports {
  memory: WebAssembly.Memory;
  fx_abi(): number;
  fx_init(width: number, height: number, seed: number): void;
  fx_tick(steps: number): void;
  fx_render(x0: number, y0: number, x1: number, y1: number, alpha: number, flags: number): number;
  fx_kinds(): number;
  fx_instances(): number;
  fx_instance_cap(): number;
  fx_stats(): number;
  fx_storage(): number;
  fx_item_count(): number;
  fx_rates(): number;
  fx_resources(): number;
  fx_purity(): number;
  fx_fog(): number;
  fx_fog_ack(): void;
  fx_power_map(): number;
  fx_terrain(): number;
  fx_width(): number;
  fx_height(): number;
  fx_belt_speed(): number;
  fx_content(): number;
  fx_content_len(): number;
  fx_report(): number;
  fx_report_len(): number;
  fx_place(x: number, y: number, kind: number, dir: number): number;
  fx_check_place(x: number, y: number, kind: number): number;
  fx_remove(x: number, y: number): number;
  fx_set_place(recipe: number, filter: number): void;
  fx_tile(x: number, y: number): number;
  fx_set_cursor(x: number, y: number, kind: number, dir: number, flags: number): void;
  fx_edit_begin(): void;
  fx_undo(): number;
  fx_set_recipe(x: number, y: number, r: number): number;
  fx_set_filter(x: number, y: number, item: number): number;
  fx_set_shards(x: number, y: number, n: number): number;
  fx_set_amp(x: number, y: number, on: number): number;
  fx_link(x: number, y: number, tx: number, ty: number): number;
  fx_inspect(x: number, y: number): number;
  fx_net_info(net: number): number;
  fx_research(t: number): number;
  fx_tech_state(t: number): number;
  fx_tech_level(t: number): number;
  fx_tech_cost(t: number, k: number): number;
  fx_ark(): number;
  fx_salvage(x: number, y: number): number;
  fx_probe_options(): number;
  fx_choose_alt(k: number): number;
  fx_buy(i: number): number;
  fx_cosmetic(bit: number): number;
  fx_can_launch(planet: number): number;
  fx_launch(planet: number): number;
  fx_set_launch(t: number): void;
  fx_offline(secs: number): number;
  fx_bp_capture(x0: number, y0: number, x1: number, y1: number): number;
  fx_bp_write(): number;
  fx_bp_ptr(): number;
  fx_bp_buffer(len: number): number;
  fx_bp_read(): number;
  fx_bp_paste(x: number, y: number, rot: number): number;
  fx_save(now: number): number;
  fx_save_ptr(): number;
  fx_load_buffer(len: number): number;
  fx_load_saved_at(): number;
  fx_load(): number;
  fx_bench(loops: number): void;
  fx_sandbox(on: number): void;
  fx_sandbox_meters(heat: number, pressure: number, oxygen: number, biomass: number): void;
  fx_sandbox_grant(n: number, ark: number): void;
}

export class Engine {
  readonly x: Exports;
  readonly width: number;
  readonly height: number;
  readonly instanceCap: number;
  readonly content: Content;
  readonly itemCount: number;
  private buffer: ArrayBuffer;
  private bytes: Uint8Array;
  private statsView: Uint32Array;
  private storageView: Uint32Array;
  private ratesView: Uint32Array;

  private constructor(x: Exports, width: number, height: number) {
    this.x = x;
    this.width = width;
    this.height = height;
    this.instanceCap = x.fx_instance_cap();
    this.itemCount = x.fx_item_count();
    this.buffer = x.memory.buffer;
    this.bytes = new Uint8Array(this.buffer);
    this.statsView = new Uint32Array(this.buffer, x.fx_stats(), STATS_LEN);
    this.storageView = new Uint32Array(this.buffer, x.fx_storage(), this.itemCount);
    this.ratesView = new Uint32Array(this.buffer, x.fx_rates(), this.itemCount * 3);
    const json = new TextDecoder().decode(new Uint8Array(this.buffer, x.fx_content(), x.fx_content_len()));
    this.content = JSON.parse(json) as Content;
  }

  static async load(width: number, height: number, seed: number): Promise<Engine> {
    // Resolved against this module's URL, so it works under any GitHub Pages sub-path.
    const url = new URL(__WASM_FILE__, import.meta.url);
    const init: RequestInit = __DEV__ ? { cache: 'no-store' } : {};
    let instance: WebAssembly.Instance;
    try {
      ({ instance } = await WebAssembly.instantiateStreaming(fetch(url, init), {}));
    } catch {
      // Servers that don't send `application/wasm` break streaming compilation.
      const bytes = await (await fetch(url, init)).arrayBuffer();
      ({ instance } = await WebAssembly.instantiate(bytes, {}));
    }
    const x = instance.exports as unknown as Exports;
    if (x.fx_abi() !== ABI_VERSION) {
      throw new Error(`Engine ABI ${x.fx_abi()} does not match the UI (${ABI_VERSION}). Reload to update.`);
    }
    x.fx_init(width, height, seed);
    return new Engine(x, width, height);
  }

  /**
   * Views over linear memory are invalidated when Wasm memory grows, and the stats/storage
   * blocks move when the world is replaced (new game, load, launch). Cheap; call per frame.
   */
  refreshViews(): void {
    const buf = this.x.memory.buffer;
    const stats = this.x.fx_stats();
    const storage = this.x.fx_storage();
    const rates = this.x.fx_rates();
    if (
      buf !== this.buffer ||
      stats !== this.statsView.byteOffset ||
      storage !== this.storageView.byteOffset ||
      rates !== this.ratesView.byteOffset
    ) {
      this.buffer = buf;
      this.bytes = new Uint8Array(buf);
      this.statsView = new Uint32Array(buf, stats, STATS_LEN);
      this.storageView = new Uint32Array(buf, storage, this.itemCount);
      this.ratesView = new Uint32Array(buf, rates, this.itemCount * 3);
    }
  }

  /** Whole linear memory as bytes; pair with `instancePtr` for zero-copy GPU uploads. */
  get memoryBytes(): Uint8Array {
    this.refreshViews();
    return this.bytes;
  }

  get instancePtr(): number {
    return this.x.fx_instances();
  }

  /** Always valid, even right after a call that grew memory. */
  get stats(): Uint32Array {
    this.refreshViews();
    return this.statsView;
  }

  /** Core storage per item id. */
  get storage(): Uint32Array {
    this.refreshViews();
    return this.storageView;
  }

  /** Per minute and item: produced [0, n), consumed [n, 2n), delivered [2n, 3n). */
  get rates(): Uint32Array {
    this.refreshViews();
    return this.ratesView;
  }

  get mapRev(): number {
    return this.stats[Stat.MapRev];
  }

  /** 64-bit stat as a JS number (exact up to 2^53). */
  stat64(lo: number): number {
    const s = this.stats;
    return s[lo] + s[lo + 1] * 4294967296;
  }

  get credits(): number {
    return this.stat64(Stat.CreditsLo);
  }

  isResearched(t: number): boolean {
    const s = this.stats;
    return t < 32 ? (s[Stat.ResearchedLo] & (1 << t)) !== 0 : (s[Stat.ResearchedHi] & (1 << (t - 32))) !== 0;
  }

  hasAchievement(i: number): boolean {
    const s = this.stats;
    return i < 32 ? (s[Stat.AchLo] & (1 << i)) !== 0 : (s[Stat.AchHi] & (1 << (i - 32))) !== 0;
  }

  beltSpeed(): number {
    return this.x.fx_belt_speed();
  }

  private copy(ptr: number, len: number): Uint8Array {
    return new Uint8Array(this.x.memory.buffer, ptr, len).slice();
  }

  /** Per-tile byte map views (valid until memory grows; don't hold on to them). */
  view(ptr: number, bytesPerTile = 1): Uint8Array {
    return new Uint8Array(this.x.memory.buffer, ptr, this.width * this.height * bytesPerTile);
  }

  resources(): Uint8Array {
    return this.copy(this.x.fx_resources(), this.width * this.height);
  }

  purity(): Uint8Array {
    return this.view(this.x.fx_purity());
  }

  terrain(): Uint8Array {
    return this.copy(this.x.fx_terrain(), this.width * this.height * 4);
  }

  kinds(): Uint8Array {
    return this.view(this.x.fx_kinds());
  }

  fog(): Uint8Array {
    return this.view(this.x.fx_fog());
  }

  powerMap(): Uint8Array {
    return this.view(this.x.fx_power_map());
  }

  /** Words from the last salvage or offline catch-up. */
  report(): Uint32Array {
    return new Uint32Array(this.x.memory.buffer, this.x.fx_report(), this.x.fx_report_len()).slice();
  }

  save(): Uint8Array {
    const len = this.x.fx_save(Date.now() / 1000);
    return this.copy(this.x.fx_save_ptr(), len);
  }

  /**
   * Replaces the world with a save. Returns the wall-clock time it was saved at (seconds,
   * 0 if unknown), or null (world untouched) if it is invalid.
   */
  load(data: Uint8Array): number | null {
    const ptr = this.x.fx_load_buffer(data.length);
    new Uint8Array(this.x.memory.buffer, ptr, data.length).set(data);
    const savedAt = this.x.fx_load_saved_at();
    const ok = this.x.fx_load() === 1;
    this.refreshViews();
    return ok ? savedAt : null;
  }

  /** Starts a fresh world. */
  reset(seed: number): void {
    this.x.fx_init(this.width, this.height, seed);
    this.refreshViews();
  }

  /** Inspector block for the building at (x, y), or null. */
  inspect(tx: number, ty: number): Uint32Array | null {
    const ptr = this.x.fx_inspect(tx, ty);
    return ptr ? new Uint32Array(this.x.memory.buffer, ptr, INFO_LEN).slice() : null;
  }

  /** Power network summary: demand, supply, sat x1000, stored MJ, capacity MJ, flow, batteries, core. */
  netInfo(net: number): Int32Array | null {
    const ptr = this.x.fx_net_info(net);
    return ptr ? new Int32Array(this.x.memory.buffer, ptr, 8).slice() : null;
  }

  /** Copies the rectangle into the active blueprint and returns its serialized bytes. */
  captureBlueprint(x0: number, y0: number, x1: number, y1: number): Uint8Array | null {
    if (!this.x.fx_bp_capture(x0, y0, x1, y1)) return null;
    const len = this.x.fx_bp_write();
    return this.copy(this.x.fx_bp_ptr(), len);
  }

  /** Makes `bytes` the active blueprint; returns its cell count. */
  loadBlueprint(bytes: Uint8Array): number {
    const ptr = this.x.fx_bp_buffer(bytes.length);
    new Uint8Array(this.x.memory.buffer, ptr, bytes.length).set(bytes);
    return this.x.fx_bp_read();
  }
}
