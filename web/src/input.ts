// Touch-first input.
//   Touch: one finger builds with the selected tool (or pans with Move); two fingers always
//          pan + pinch-zoom. A touch only commits after it moves or lifts, so starting a
//          pinch never drops a stray building. Tapping a building with Move inspects it.
//   Mouse: left builds (or pans with Move), right drags erase, middle drags pan, wheel zooms.
//   Keys:  1-9 tools in the open category, Tab next category, Q move, X delete, R rotate,
//          Z / Ctrl+Z undo, F fullscreen, Esc close/cancel, WASD/arrows pan, +/- zoom.

import { CursorFlag, DX, DY, Kind, Reason, TOOL_COPY, TOOL_DELETE, TOOL_MOVE, TOOL_PASTE } from './constants';
import type { Camera } from './camera';
import type { Engine } from './engine';

/** CSS px a touch may wander before it counts as a drag instead of a tap. */
const TAP_SLOP = 10;
const KEY_PAN_SPEED = 900; // CSS px per second

type Gesture = 'none' | 'pending' | 'pan' | 'build' | 'erase' | 'pinch' | 'dead' | 'select';

interface Pointer {
  id: number;
  x: number;
  y: number;
}

export interface InputHandlers {
  onPlaced(kind: number): void;
  onRemoved(): void;
  /** A placement was refused (only reported once per gesture). */
  onRefused(reason: number, kind: number, x: number, y: number): void;
  onInspect(x: number, y: number): void;
  onRotate(dir: number): void;
  onKey(key: string, e: KeyboardEvent): boolean;
  /** Copy tool: the selection rectangle changed (tiles, inclusive), or null when done. */
  onSelect(rect: [number, number, number, number] | null): void;
  onCopy(x0: number, y0: number, x1: number, y1: number): void;
  onPaste(x: number, y: number, rot: number): void;
}

export class Input {
  tool = TOOL_MOVE;
  dir = 0;

  private readonly pointers: Pointer[] = [];
  private gesture: Gesture = 'none';
  private startX = 0;
  private startY = 0;
  private tileX = 0;
  private tileY = 0;
  private moved = false;
  private refused = false;
  private pinchDist = 1;
  private pinchZoom = 1;
  private pinchWX = 0;
  private pinchWY = 0;
  private readonly keys = new Set<string>();
  private cursorKey = '';
  private cursorX = 0;
  private cursorY = 0;
  private cursorVisible = false;

  constructor(
    private readonly canvas: HTMLCanvasElement,
    private readonly cam: Camera,
    private readonly engine: Engine,
    private readonly h: InputHandlers,
  ) {
    canvas.addEventListener('pointerdown', this.onDown);
    canvas.addEventListener('pointermove', this.onMove);
    canvas.addEventListener('pointerup', this.onUp);
    canvas.addEventListener('pointercancel', this.onUp);
    canvas.addEventListener('pointerleave', (e) => {
      if (e.pointerType === 'mouse' && this.gesture === 'none') this.setCursor(0, 0, false);
    });
    canvas.addEventListener('wheel', this.onWheel, { passive: false });
    canvas.addEventListener('contextmenu', (e) => e.preventDefault());
    // iOS Safari: stop its own page pinch-zoom from fighting ours.
    document.addEventListener('gesturestart', (e) => e.preventDefault());
    window.addEventListener('keydown', this.onKeyDown);
    window.addEventListener('keyup', (e) => this.keys.delete(e.key.toLowerCase()));
    window.addEventListener('blur', () => this.keys.clear());
  }

  setTool(tool: number): void {
    this.tool = tool;
    this.refreshCursor();
  }

  rotate(step: number): void {
    this.dir = (this.dir + step + 4) & 3;
    this.h.onRotate(this.dir);
    this.refreshCursor();
  }

  /** Per-frame work: keyboard panning. */
  update(dt: number): void {
    if (this.keys.size === 0) return;
    const k = this.keys;
    const vx = +(k.has('a') || k.has('arrowleft')) - +(k.has('d') || k.has('arrowright'));
    const vy = +(k.has('w') || k.has('arrowup')) - +(k.has('s') || k.has('arrowdown'));
    if (vx || vy) this.cam.panBy(vx * KEY_PAN_SPEED * dt, vy * KEY_PAN_SPEED * dt);
  }

  // ---- Pointers ---------------------------------------------------------------------------

  private local(e: PointerEvent | WheelEvent): [number, number] {
    const r = this.canvas.getBoundingClientRect();
    return [e.clientX - r.left, e.clientY - r.top];
  }

  private tileAt(x: number, y: number): [number, number] {
    return [Math.floor(this.cam.screenToWorldX(x)), Math.floor(this.cam.screenToWorldY(y))];
  }

  private get building(): boolean {
    return this.tool >= 0;
  }

  /** Tools where a drag pans the view and a tap acts. */
  private get tapTool(): boolean {
    return this.tool === TOOL_MOVE || this.tool === TOOL_PASTE;
  }

  private onDown = (e: PointerEvent): void => {
    e.preventDefault();
    if (this.pointers.length >= 2) return;
    try {
      this.canvas.setPointerCapture(e.pointerId);
    } catch {
      /* synthetic or already-released pointer: capture is only a convenience */
    }
    const [x, y] = this.local(e);
    this.pointers.push({ id: e.pointerId, x, y });
    if (this.pointers.length === 2) {
      this.beginPinch();
      return;
    }
    this.startX = x;
    this.startY = y;
    [this.tileX, this.tileY] = this.tileAt(x, y);
    this.moved = false;
    this.refused = false;
    if (e.pointerType === 'mouse') {
      if (e.button === 1) this.gesture = 'pan';
      else if (e.button === 2 || (e.button === 0 && this.tool === TOOL_DELETE)) this.beginErase();
      else if (e.button === 0 && this.tool === TOOL_COPY) this.beginSelect();
      else if (e.button === 0 && this.building) this.beginBuild();
      else if (e.button === 0) this.gesture = 'pending'; // Move/paste: pan on drag, act on click
    } else {
      this.gesture = 'pending';
      this.setCursor(this.tileX, this.tileY, this.tool !== TOOL_MOVE && this.tool !== TOOL_COPY);
    }
  };

  private onMove = (e: PointerEvent): void => {
    const [x, y] = this.local(e);
    const p = this.pointers.find((q) => q.id === e.pointerId);
    if (!p) {
      if (e.pointerType === 'mouse') {
        const [tx, ty] = this.tileAt(x, y);
        this.setCursor(tx, ty, this.tool !== TOOL_MOVE && this.tool !== TOOL_COPY);
      }
      return;
    }
    const dx = x - p.x;
    const dy = y - p.y;
    p.x = x;
    p.y = y;
    switch (this.gesture) {
      case 'pinch':
        this.updatePinch();
        break;
      case 'pan':
        this.cam.panBy(dx, dy);
        break;
      case 'pending':
        if (Math.hypot(x - this.startX, y - this.startY) > TAP_SLOP) {
          if (this.tapTool) {
            this.gesture = 'pan';
            this.cam.panBy(x - this.startX, y - this.startY);
          } else if (this.tool === TOOL_COPY) {
            this.beginSelect();
            this.dragSelect(x, y);
          } else if (this.tool === TOOL_DELETE) {
            this.beginErase();
            this.dragErase(x, y);
          } else {
            this.beginBuild();
            this.dragBuild(x, y);
          }
        }
        break;
      case 'build':
        this.dragBuild(x, y);
        break;
      case 'erase':
        this.dragErase(x, y);
        break;
      case 'select':
        this.dragSelect(x, y);
        break;
    }
  };

  private onUp = (e: PointerEvent): void => {
    const i = this.pointers.findIndex((q) => q.id === e.pointerId);
    if (i < 0) return;
    this.pointers.splice(i, 1);
    if (this.gesture === 'pinch' || this.gesture === 'dead') {
      // Ignore the finger left over from a pinch until everything is lifted.
      this.gesture = this.pointers.length > 0 ? 'dead' : 'none';
      return;
    }
    const committed = e.type === 'pointerup';
    if (this.gesture === 'select') {
      this.h.onSelect(null);
      if (committed) this.h.onCopy(this.selX, this.selY, this.tileX, this.tileY);
    } else if (this.gesture === 'pending' && committed) {
      // A tap.
      if (this.tool === TOOL_MOVE || this.tool === TOOL_COPY) this.h.onInspect(this.tileX, this.tileY);
      else if (this.tool === TOOL_PASTE) this.h.onPaste(this.tileX, this.tileY, this.dir);
      else if (this.tool === TOOL_DELETE) {
        this.beginErase();
      } else {
        this.beginBuild();
        this.endBuild();
      }
    } else if (this.gesture === 'build' && committed) {
      this.endBuild();
    }
    this.gesture = 'none';
    if (e.pointerType !== 'mouse' && this.tool !== TOOL_PASTE) this.setCursor(0, 0, false);
  };

  private onWheel = (e: WheelEvent): void => {
    e.preventDefault();
    let d = e.deltaY;
    if (e.deltaMode === 1) d *= 16;
    else if (e.deltaMode === 2) d *= 400;
    const [x, y] = this.local(e);
    // Trackpad pinches arrive as ctrl+wheel with small deltas.
    this.cam.zoomAt(x, y, Math.exp(-d * (e.ctrlKey ? 0.01 : 0.0015)));
  };

  private beginPinch(): void {
    const [a, b] = this.pointers;
    const mx = (a.x + b.x) / 2;
    const my = (a.y + b.y) / 2;
    this.pinchDist = Math.max(1, Math.hypot(a.x - b.x, a.y - b.y));
    this.pinchZoom = this.cam.zoom;
    this.pinchWX = this.cam.screenToWorldX(mx);
    this.pinchWY = this.cam.screenToWorldY(my);
    this.gesture = 'pinch';
    this.setCursor(0, 0, false);
  }

  private updatePinch(): void {
    const [a, b] = this.pointers;
    const cam = this.cam;
    const mx = (a.x + b.x) / 2;
    const my = (a.y + b.y) / 2;
    cam.zoom = this.pinchZoom;
    cam.zoomAt(cam.width / 2, cam.height / 2, Math.hypot(a.x - b.x, a.y - b.y) / this.pinchDist);
    // Keep the world point that started under the fingers' midpoint under it.
    cam.x = this.pinchWX - (mx - cam.width / 2) / cam.zoom;
    cam.y = this.pinchWY - (my - cam.height / 2) / cam.zoom;
    cam.clamp();
  }

  // ---- Blueprint selection ------------------------------------------------------------------

  private selX = 0;
  private selY = 0;

  private beginSelect(): void {
    this.gesture = 'select';
    this.selX = this.tileX;
    this.selY = this.tileY;
    this.h.onSelect([this.selX, this.selY, this.tileX, this.tileY]);
  }

  private dragSelect(x: number, y: number): void {
    [this.tileX, this.tileY] = this.tileAt(x, y);
    this.h.onSelect([this.selX, this.selY, this.tileX, this.tileY]);
  }

  // ---- Building ---------------------------------------------------------------------------

  private place(x: number, y: number, dir: number): void {
    const kind = this.tool;
    const reason = this.engine.x.fx_check_place(x, y, kind);
    if (reason === Reason.Ok) {
      const before = this.engine.x.fx_tile(x, y);
      if (this.engine.x.fx_place(x, y, kind, dir) && this.engine.x.fx_tile(x, y) !== before) this.h.onPlaced(kind);
    } else if (!this.refused && !(kind === Kind.Belt && reason === Reason.Occupied)) {
      this.refused = true;
      this.h.onRefused(reason, kind, x, y);
    }
  }

  private erase(x: number, y: number): void {
    if (this.engine.x.fx_remove(x, y)) this.h.onRemoved();
  }

  private beginBuild(): void {
    this.engine.x.fx_edit_begin();
    this.gesture = 'build';
    this.moved = false;
    // Belts wait for the drag direction; machines go down immediately.
    if (this.tool !== Kind.Belt) {
      this.place(this.tileX, this.tileY, this.dir);
      this.moved = true;
    }
    this.setCursor(this.tileX, this.tileY, true);
  }

  private endBuild(): void {
    if (!this.moved) this.place(this.tileX, this.tileY, this.dir);
  }

  private beginErase(): void {
    this.engine.x.fx_edit_begin();
    this.gesture = 'erase';
    this.erase(this.tileX, this.tileY);
    this.setCursor(this.tileX, this.tileY, true);
  }

  /** Walks 4-connected from the last tile to the pointer so fast drags leave no gaps. */
  private dragBuild(x: number, y: number): void {
    const [tx, ty] = this.tileAt(x, y);
    while (tx !== this.tileX || ty !== this.tileY) {
      const dx = tx - this.tileX;
      const dy = ty - this.tileY;
      const d = Math.abs(dx) >= Math.abs(dy) ? (dx > 0 ? 0 : 2) : dy > 0 ? 1 : 3;
      const nx = this.tileX + DX[d];
      const ny = this.tileY + DY[d];
      if (this.tool === Kind.Belt) {
        // Point the tile we leave at the new one: this is what turns a drag into corners.
        this.place(this.tileX, this.tileY, d);
        this.place(nx, ny, d);
        if (d !== this.dir) this.rotate(d - this.dir);
      } else {
        this.place(nx, ny, this.dir);
      }
      this.tileX = nx;
      this.tileY = ny;
      this.moved = true;
    }
    this.setCursor(tx, ty, true);
  }

  private dragErase(x: number, y: number): void {
    const [tx, ty] = this.tileAt(x, y);
    while (tx !== this.tileX || ty !== this.tileY) {
      const dx = tx - this.tileX;
      const dy = ty - this.tileY;
      if (Math.abs(dx) >= Math.abs(dy)) this.tileX += Math.sign(dx);
      else this.tileY += Math.sign(dy);
      this.erase(this.tileX, this.tileY);
    }
    this.setCursor(tx, ty, true);
  }

  // ---- Placement preview ------------------------------------------------------------------

  private setCursor(tx: number, ty: number, visible: boolean): void {
    this.cursorX = tx;
    this.cursorY = ty;
    this.cursorVisible = visible;
    const del = this.tool === TOOL_DELETE || this.gesture === 'erase';
    const paste = this.tool === TOOL_PASTE ? CursorFlag.Paste : 0;
    const flags = visible ? CursorFlag.Visible | (del ? CursorFlag.Delete : paste) : 0;
    const kind = Math.max(0, this.tool);
    const key = `${tx},${ty},${kind},${this.dir},${flags}`;
    if (key === this.cursorKey) return;
    this.cursorKey = key;
    this.engine.x.fx_set_cursor(tx, ty, kind, this.dir, flags);
  }

  refreshCursor(): void {
    this.cursorKey = '';
    const visible = this.tool === TOOL_PASTE || (this.cursorVisible && this.tool !== TOOL_MOVE && this.tool !== TOOL_COPY);
    this.setCursor(this.cursorX, this.cursorY, visible);
  }

  /** Centres the paste preview on the screen (touch devices have no hover). */
  centerCursor(): void {
    [this.cursorX, this.cursorY] = this.tileAt(this.cam.width / 2, this.cam.height / 2);
    this.refreshCursor();
  }

  // ---- Keyboard ---------------------------------------------------------------------------

  private onKeyDown = (e: KeyboardEvent): void => {
    if ((e.target as HTMLElement)?.closest?.('input, textarea')) return;
    const k = e.key.toLowerCase();
    if ((e.metaKey || e.ctrlKey) && k === 'z') {
      e.preventDefault();
      this.h.onKey('undo', e);
      return;
    }
    if (e.metaKey || e.ctrlKey || e.altKey) return;
    if (k === 'r') this.rotate(e.shiftKey ? -1 : 1);
    else if (k === '+' || k === '=') this.cam.zoomAt(this.cam.width / 2, this.cam.height / 2, 1.25);
    else if (k === '-' || k === '_') this.cam.zoomAt(this.cam.width / 2, this.cam.height / 2, 0.8);
    else if ('wasd'.includes(k) || k.startsWith('arrow')) this.keys.add(k);
    else if (!this.h.onKey(k, e)) return;
    e.preventDefault();
  };
}
