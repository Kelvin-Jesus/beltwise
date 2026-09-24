// 2D camera: center in tile units, zoom in CSS pixels per tile.

export const MIN_ZOOM = 2;
export const MAX_ZOOM = 160;

export class Camera {
  x = 0;
  y = 0;
  zoom = 40;
  /** CSS size of the canvas. */
  width = 1;
  height = 1;
  /** Device pixels per CSS pixel actually used for the drawing buffer. */
  pixelRatio = 1;

  constructor(
    private readonly worldW: number,
    private readonly worldH: number,
  ) {}

  screenToWorldX(sx: number): number {
    return this.x + (sx - this.width * 0.5) / this.zoom;
  }

  screenToWorldY(sy: number): number {
    return this.y + (sy - this.height * 0.5) / this.zoom;
  }

  /** Zooms by `factor` keeping the world point under screen point (sx, sy) fixed. */
  zoomAt(sx: number, sy: number, factor: number): void {
    const wx = this.screenToWorldX(sx);
    const wy = this.screenToWorldY(sy);
    this.zoom = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, this.zoom * factor));
    this.x = wx - (sx - this.width * 0.5) / this.zoom;
    this.y = wy - (sy - this.height * 0.5) / this.zoom;
    this.clamp();
  }

  panBy(dxCss: number, dyCss: number): void {
    this.x -= dxCss / this.zoom;
    this.y -= dyCss / this.zoom;
    this.clamp();
  }

  /** Keeps at least part of the world on screen. */
  clamp(): void {
    this.x = Math.min(this.worldW, Math.max(0, this.x));
    this.y = Math.min(this.worldH, Math.max(0, this.y));
  }

  get left(): number {
    return this.x - (this.width * 0.5) / this.zoom;
  }
  get top(): number {
    return this.y - (this.height * 0.5) / this.zoom;
  }
  get right(): number {
    return this.x + (this.width * 0.5) / this.zoom;
  }
  get bottom(): number {
    return this.y + (this.height * 0.5) / this.zoom;
  }
}
