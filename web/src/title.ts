// Title screen: the logo over the live world while the camera drifts around the Core.

import type { Camera } from './camera';
import { Stat } from './constants';
import type { Engine } from './engine';

export interface TitleHandlers {
  onPlay(): void;
  onHelp(): void;
}

export class Title {
  visible = true;
  private t = 0;
  private readonly el: HTMLElement;

  constructor(hasSave: boolean, h: TitleHandlers) {
    this.el = document.getElementById('title')!;
    this.el.innerHTML = `<div class="title-inner">
      <img class="title-mark" src="logo-mark.webp" width="384" height="384" alt="" draggable="false">
      <h1><img class="title-word" src="logo-wordmark.webp" width="681" height="156" alt="Beltwise" draggable="false"></h1>
      <p class="tagline">Build a factory. Wake a frozen world. Launch the Ark.</p>
      <button class="btn primary big" id="title-play">${hasSave ? 'Continue' : 'Land'}</button>
      <button class="btn ghost" id="title-help">How to play</button>
    </div>`;
    this.el.hidden = false;
    document.body.classList.add('title');
    document.getElementById('title-play')!.addEventListener('click', () => {
      this.hide();
      h.onPlay();
    });
    document.getElementById('title-help')!.addEventListener('click', () => {
      this.hide();
      h.onHelp();
    });
  }

  hide(): void {
    if (!this.visible) return;
    this.visible = false;
    this.el.classList.add('out');
    document.body.classList.remove('title');
    setTimeout(() => (this.el.hidden = true), 450);
  }

  /** Drifts the camera slowly around the Core behind the title. */
  update(cam: Camera, dt: number, engine: Engine): void {
    this.t += dt / 1000;
    const s = engine.stats;
    const cx = s[Stat.CoreX] + s[Stat.CoreSize] / 2;
    const cy = s[Stat.CoreY] + s[Stat.CoreSize] / 2;
    cam.x = cx + Math.cos(this.t * 0.08) * 7;
    cam.y = cy + Math.sin(this.t * 0.08) * 5 + 2;
    cam.zoom = Math.max(14, Math.min(cam.width, cam.height) / 30);
  }
}
