// WebGL2 renderer: two draw calls per frame.
//   1. Ground: a full-screen triangle painting terrain, terraforming, deposits, fog of war,
//      the power overlay and nightfall from world textures. At far zoom it also draws every
//      building from a per-tile kind map, so a zoomed-out frame costs the same however
//      large the factory is.
//   2. Sprites: every visible belt, item, machine, drone, light and particle as one
//      instanced quad, streamed straight out of Wasm memory with a single bufferSubData.
// World textures are only re-uploaded when the engine says they changed (revision
// counters in the stats block), and fog updates upload just the changed rectangle.

import { INSTANCE_BYTES, Stat } from './constants';
import { CATEGORIES, type Content } from './content';
import type { Camera } from './camera';
import type { Engine } from './engine';
import { groundFS, groundVS, spriteFS, spriteVS } from './shaders';

/** Deposit pad colours, by resource item key. */
const RESOURCE_COLORS: Record<string, string> = {
  iron_ore: '#6f8db3',
  copper_ore: '#d9773a',
  stone: '#8f8b86',
  coal: '#2a2b30',
  ice: '#9fdcf5',
  titanium_ore: '#dfe9f3',
  uranium_ore: '#63d94a',
  crude_oil: '#3b2f4d',
  meteorite: '#b36bff',
};
const KINDS = 40;
const RESOURCES = 10;

/** Rotating instance buffers so we never write into one the GPU may still be reading. */
const RING = 3;

function compile(gl: WebGL2RenderingContext, vs: string, fs: string): WebGLProgram {
  const prog = gl.createProgram()!;
  for (const [type, src] of [
    [gl.VERTEX_SHADER, vs],
    [gl.FRAGMENT_SHADER, fs],
  ] as const) {
    const sh = gl.createShader(type)!;
    gl.shaderSource(sh, src);
    gl.compileShader(sh);
    if (!gl.getShaderParameter(sh, gl.COMPILE_STATUS)) {
      throw new Error(`Shader compile failed: ${gl.getShaderInfoLog(sh)}`);
    }
    gl.attachShader(prog, sh);
  }
  gl.linkProgram(prog);
  if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) {
    throw new Error(`Program link failed: ${gl.getProgramInfoLog(prog)}`);
  }
  return prog;
}

/**
 * Seamlessly tiling value noise: RGB are three independent smooth fields with `cells`
 * lattice cells per repeat, A is per-texel white noise.
 */
function noiseTexture(size: number, cells: number): Uint8Array {
  let seed = 0x2545f491;
  const rnd = () => {
    seed ^= seed << 13;
    seed ^= seed >>> 17;
    seed ^= seed << 5;
    return (seed >>> 0) / 4294967296;
  };
  const lattice = [0, 1, 2].map(() => Array.from({ length: cells * cells }, rnd));
  const out = new Uint8Array(size * size * 4);
  const step = size / cells;
  const smooth = (t: number) => t * t * (3 - 2 * t);
  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      const fx = x / step;
      const fy = y / step;
      const ix = Math.floor(fx);
      const iy = Math.floor(fy);
      const sx = smooth(fx - ix);
      const sy = smooth(fy - iy);
      const i = (y * size + x) * 4;
      for (let c = 0; c < 3; c++) {
        const L = lattice[c];
        const at = (u: number, v: number) => L[((v + cells) % cells) * cells + ((u + cells) % cells)];
        const top = at(ix, iy) + (at(ix + 1, iy) - at(ix, iy)) * sx;
        const bot = at(ix, iy + 1) + (at(ix + 1, iy + 1) - at(ix, iy + 1)) * sx;
        out[i + c] = Math.round((top + (bot - top) * sy) * 255);
      }
      out[i + 3] = Math.floor(rnd() * 256);
    }
  }
  return out;
}

export function hexToRgb(hex: string): [number, number, number] {
  const n = parseInt(hex.slice(1), 16);
  return [((n >> 16) & 255) / 255, ((n >> 8) & 255) / 255, (n & 255) / 255];
}

export interface FrameParams {
  cam: Camera;
  instances: number;
  beltPhase: number;
  time: number;
  /** Terraforming levels 0..1: heat, water, air, life. */
  terra: Float32Array;
  /** Draw buildings from the tile map (far zoom). */
  map: boolean;
  /** Effect level: 2 full, 1 reduced, 0 minimal (quality setting). */
  fx: number;
  /** Daylight tint (multiplies the world). */
  ambient: Float32Array;
  /** Power coverage overlay strength (0 = hidden). */
  overlay: number;
  planet: number;
  /** Belt stripe colour. */
  stripe: Float32Array;
}

type Tex = 'res' | 'kinds' | 'terrain' | 'atlas' | 'fog' | 'power';
const UNIT: Record<Tex, number> = { res: 0, kinds: 1, terrain: 2, atlas: 3, fog: 5, power: 6 };

export class Renderer {
  readonly gl: WebGL2RenderingContext;
  private readonly ground: WebGLProgram;
  private readonly sprites: WebGLProgram;
  private readonly emptyVao: WebGLVertexArrayObject;
  private readonly vaos: WebGLVertexArrayObject[] = [];
  private readonly vbos: WebGLBuffer[] = [];
  private readonly tex: Record<Tex, WebGLTexture>;
  private frame = 0;
  private mapRev = -1;
  private fogRev = -1;
  private resRev = -1;
  private powerRev = -1;
  private resBuf: Uint8Array;
  private readonly u: Record<string, WebGLUniformLocation | null> = {};

  constructor(
    readonly canvas: HTMLCanvasElement,
    private readonly engine: Engine,
    content: Content,
    atlas: HTMLCanvasElement,
  ) {
    const gl = canvas.getContext('webgl2', {
      alpha: false,
      antialias: false, // shapes are analytically anti-aliased in the shaders
      depth: false,
      stencil: false,
      premultipliedAlpha: true,
      preserveDrawingBuffer: false,
      powerPreference: 'high-performance',
    });
    if (!gl) throw new Error('WebGL2 is not available on this device/browser.');
    this.gl = gl;
    this.resBuf = new Uint8Array(engine.width * engine.height * 2);

    this.ground = compile(gl, groundVS, groundFS);
    this.sprites = compile(gl, spriteVS, spriteFS);
    for (const n of [
      'u_viewport',
      'u_cam',
      'u_world',
      'u_terra',
      'u_time',
      'u_map',
      'u_fx',
      'u_overlay',
      'u_planet',
      'u_ambient',
      'u_kindColor',
      'u_resColor',
    ]) {
      this.u['g' + n] = gl.getUniformLocation(this.ground, n);
    }
    for (const n of ['u_cam', 'u_zoom', 'u_phase', 'u_time', 'u_ambient', 'u_stripe']) {
      this.u['s' + n] = gl.getUniformLocation(this.sprites, n);
    }

    const mk = () => gl.createTexture()!;
    this.tex = { res: mk(), kinds: mk(), terrain: mk(), atlas: mk(), fog: mk(), power: mk() };

    // Atlas: premultiplied, mipmapped (sprites are drawn at many sizes).
    gl.activeTexture(gl.TEXTURE0 + UNIT.atlas);
    gl.bindTexture(gl.TEXTURE_2D, this.tex.atlas);
    gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, true);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, atlas);
    gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, false);
    gl.generateMipmap(gl.TEXTURE_2D);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR_MIPMAP_LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);

    // Tiling noise for the ground shader (cheaper than procedural noise per pixel).
    gl.activeTexture(gl.TEXTURE4);
    gl.bindTexture(gl.TEXTURE_2D, gl.createTexture());
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, 128, 128, 0, gl.RGBA, gl.UNSIGNED_BYTE, noiseTexture(128, 16));
    gl.generateMipmap(gl.TEXTURE_2D);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR_MIPMAP_LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.REPEAT);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.REPEAT);

    this.uploadWorld();

    gl.useProgram(this.ground);
    for (const [name, unit] of [
      ['u_res', UNIT.res],
      ['u_kinds', UNIT.kinds],
      ['u_terrain', UNIT.terrain],
      ['u_atlas', UNIT.atlas],
      ['u_noise', 4],
      ['u_fog', UNIT.fog],
      ['u_power', UNIT.power],
    ] as const) {
      gl.uniform1i(gl.getUniformLocation(this.ground, name), unit);
    }
    const colors = new Float32Array(KINDS * 3);
    content.buildings.forEach((b, k) => {
      if (k >= KINDS) return;
      const hex =
        b.class === 'belt'
          ? '#4b5563'
          : b.class === 'core'
            ? '#fbbf24'
            : b.class === 'wreck'
              ? '#f97316'
              : (CATEGORIES[b.category]?.accent ?? '#94a3b8');
      colors.set(hexToRgb(hex), k * 3);
    });
    gl.uniform3fv(this.u.gu_kindColor, colors);
    const res = new Float32Array(RESOURCES * 3);
    content.items.forEach((it, i) => {
      if (i < RESOURCES && RESOURCE_COLORS[it.key]) res.set(hexToRgb(RESOURCE_COLORS[it.key]), i * 3);
    });
    gl.uniform3fv(this.u.gu_resColor, res);
    gl.useProgram(this.sprites);
    gl.uniform1i(gl.getUniformLocation(this.sprites, 'u_atlas'), UNIT.atlas);

    this.emptyVao = gl.createVertexArray()!;
    const quad = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, quad);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-0.5, -0.5, 0.5, -0.5, -0.5, 0.5, 0.5, 0.5]), gl.STATIC_DRAW);
    const bytes = engine.instanceCap * INSTANCE_BYTES;
    for (let i = 0; i < RING; i++) {
      const vao = gl.createVertexArray()!;
      gl.bindVertexArray(vao);
      gl.bindBuffer(gl.ARRAY_BUFFER, quad);
      gl.enableVertexAttribArray(0);
      gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 8, 0);
      const vbo = gl.createBuffer()!;
      gl.bindBuffer(gl.ARRAY_BUFFER, vbo);
      gl.bufferData(gl.ARRAY_BUFFER, bytes, gl.DYNAMIC_DRAW);
      gl.enableVertexAttribArray(1);
      gl.vertexAttribPointer(1, 2, gl.FLOAT, false, INSTANCE_BYTES, 0);
      gl.vertexAttribDivisor(1, 1);
      gl.enableVertexAttribArray(2);
      gl.vertexAttribIPointer(2, 4, gl.UNSIGNED_BYTE, INSTANCE_BYTES, 8);
      gl.vertexAttribDivisor(2, 1);
      gl.enableVertexAttribArray(3);
      gl.vertexAttribPointer(3, 4, gl.UNSIGNED_BYTE, true, INSTANCE_BYTES, 12);
      gl.vertexAttribDivisor(3, 1);
      this.vaos.push(vao);
      this.vbos.push(vbo);
    }
    gl.bindVertexArray(null);
    gl.disable(gl.DEPTH_TEST);
    gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA); // premultiplied alpha
  }

  private dataTexture(tex: Tex, internal: number, format: number, data: Uint8Array, linear: boolean): void {
    const gl = this.gl;
    const { width, height } = this.engine;
    gl.activeTexture(gl.TEXTURE0 + UNIT[tex]);
    gl.bindTexture(gl.TEXTURE_2D, this.tex[tex]);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
    gl.texImage2D(gl.TEXTURE_2D, 0, internal, width, height, 0, format, gl.UNSIGNED_BYTE, data);
    const filter = linear ? gl.LINEAR : gl.NEAREST;
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, filter);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, filter);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  }

  /** (Re)uploads every world map; call after a new game, loading a save or launching. */
  uploadWorld(): void {
    const gl = this.gl;
    this.dataTexture('terrain', gl.RGBA8, gl.RGBA, this.engine.terrain(), true);
    this.mapRev = this.fogRev = this.resRev = this.powerRev = -1;
    this.syncRes(true);
    this.syncFog(true);
    this.syncKinds();
    this.syncPower(true);
  }

  /** Deposits (with purity): changes only when meteors land. */
  private syncRes(force = false): void {
    const rev = this.engine.stats[Stat.ResRev];
    if (!force && rev === this.resRev) return;
    this.resRev = rev;
    const res = this.engine.resources();
    const purity = this.engine.purity();
    const buf = this.resBuf;
    for (let i = 0; i < res.length; i++) {
      buf[i * 2] = res[i];
      buf[i * 2 + 1] = purity[i];
    }
    this.dataTexture('res', this.gl.RG8UI, this.gl.RG_INTEGER, buf, false);
  }

  /** Fog of war: only the rectangle revealed since the last upload is sent. */
  private syncFog(force = false): void {
    const gl = this.gl;
    const s = this.engine.stats;
    const rev = s[Stat.FogRev];
    if (!force && rev === this.fogRev) return;
    this.fogRev = rev;
    const { width, height } = this.engine;
    const x0 = s[Stat.FogDirty];
    const y0 = s[Stat.FogDirty + 1];
    const x1 = Math.min(width, s[Stat.FogDirty + 2]);
    const y1 = Math.min(height, s[Stat.FogDirty + 3]);
    if (force || x0 >= x1 || y0 >= y1) {
      this.dataTexture('fog', gl.R8, gl.RED, this.engine.fog(), true);
    } else {
      gl.activeTexture(gl.TEXTURE0 + UNIT.fog);
      gl.bindTexture(gl.TEXTURE_2D, this.tex.fog);
      gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
      gl.pixelStorei(gl.UNPACK_ROW_LENGTH, width);
      gl.pixelStorei(gl.UNPACK_SKIP_PIXELS, x0);
      gl.pixelStorei(gl.UNPACK_SKIP_ROWS, y0);
      gl.texSubImage2D(gl.TEXTURE_2D, 0, x0, y0, x1 - x0, y1 - y0, gl.RED, gl.UNSIGNED_BYTE, this.engine.fog());
      gl.pixelStorei(gl.UNPACK_ROW_LENGTH, 0);
      gl.pixelStorei(gl.UNPACK_SKIP_PIXELS, 0);
      gl.pixelStorei(gl.UNPACK_SKIP_ROWS, 0);
    }
    this.engine.x.fx_fog_ack();
  }

  /** Refreshes the per-tile building map (far-zoom view) when the world changed. */
  private syncKinds(): void {
    const rev = this.engine.mapRev;
    if (rev === this.mapRev) return;
    this.mapRev = rev;
    this.dataTexture('kinds', this.gl.R8UI, this.gl.RED_INTEGER, this.engine.kinds(), false);
  }

  /** Power coverage, for the overlay (only uploaded while the overlay is visible). */
  private syncPower(force = false): void {
    const rev = this.engine.stats[Stat.PowerRev];
    if (!force && rev === this.powerRev) return;
    this.powerRev = rev;
    this.dataTexture('power', this.gl.R8UI, this.gl.RED_INTEGER, this.engine.powerMap(), false);
  }

  resize(width: number, height: number): void {
    if (this.canvas.width !== width || this.canvas.height !== height) {
      this.canvas.width = width;
      this.canvas.height = height;
    }
  }

  draw(p: FrameParams): void {
    const gl = this.gl;
    const w = this.canvas.width;
    const h = this.canvas.height;
    const zoom = p.cam.zoom * p.cam.pixelRatio; // device px per tile
    gl.viewport(0, 0, w, h);
    this.syncFog();
    this.syncRes();
    if (p.map) this.syncKinds();
    if (p.overlay > 0) this.syncPower();

    // Pass 1: ground (overwrites every pixel, so no clear is needed).
    gl.disable(gl.BLEND);
    gl.useProgram(this.ground);
    gl.bindVertexArray(this.emptyVao);
    gl.uniform2f(this.u.gu_viewport, w, h);
    gl.uniform3f(this.u.gu_cam, p.cam.x, p.cam.y, zoom);
    gl.uniform2i(this.u.gu_world, this.engine.width, this.engine.height);
    gl.uniform4fv(this.u.gu_terra, p.terra);
    gl.uniform1f(this.u.gu_time, p.time);
    gl.uniform1f(this.u.gu_map, p.map ? 1 : 0);
    gl.uniform1f(this.u.gu_fx, p.fx);
    gl.uniform1f(this.u.gu_overlay, p.overlay);
    gl.uniform1i(this.u.gu_planet, p.planet);
    gl.uniform3fv(this.u.gu_ambient, p.ambient);
    gl.drawArrays(gl.TRIANGLES, 0, 3);

    if (p.instances === 0) return;

    // Pass 2: all sprites, straight from Wasm memory.
    const slot = this.frame++ % RING;
    gl.bindVertexArray(this.vaos[slot]);
    gl.bindBuffer(gl.ARRAY_BUFFER, this.vbos[slot]);
    gl.bufferSubData(gl.ARRAY_BUFFER, 0, this.engine.memoryBytes, this.engine.instancePtr, p.instances * INSTANCE_BYTES);
    gl.enable(gl.BLEND);
    gl.useProgram(this.sprites);
    gl.uniform4f(this.u.su_cam, p.cam.x, p.cam.y, (2 * zoom) / w, (-2 * zoom) / h);
    gl.uniform1f(this.u.su_zoom, zoom);
    gl.uniform1f(this.u.su_phase, p.beltPhase);
    gl.uniform1f(this.u.su_time, p.time);
    gl.uniform3fv(this.u.su_ambient, p.ambient);
    gl.uniform3fv(this.u.su_stripe, p.stripe);
    gl.drawArraysInstanced(gl.TRIANGLE_STRIP, 0, 4, p.instances);
    gl.bindVertexArray(null);
  }
}
