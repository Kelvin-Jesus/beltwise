// Title screen: the logo over the live world while the camera drifts around the Core.

import type { Camera } from './camera';
import { Stat } from './constants';
import type { Engine } from './engine';

export interface TitleHandlers {
  onPlay(): void;
  onHelp(): void;
}

/** Placeholder mark until the final logo lands: a half-frozen, half-green planet ringed by a belt. */
export const LOGO_SVG = `<svg viewBox="0 0 120 120" aria-hidden="true">
  <defs><clipPath id="lg-p"><circle cx="60" cy="60" r="30"/></clipPath></defs>
  <g clip-path="url(#lg-p)"><rect x="20" y="20" width="80" height="80" fill="#7dd3fc"/><path d="M60 20h40v80H60c10-12 10-28 0-40s-10-28 0-40z" fill="#34d399"/></g>
  <circle cx="60" cy="60" r="30" fill="none" stroke="#0b1016" stroke-width="3"/>
  <ellipse cx="60" cy="62" rx="52" ry="17" fill="none" stroke="#1f2937" stroke-width="12" transform="rotate(-18 60 62)"/>
  <ellipse cx="60" cy="62" rx="52" ry="17" fill="none" stroke="#f59e0b" stroke-width="4" stroke-dasharray="7 9" transform="rotate(-18 60 62)"/>
  <path d="M60 29a31 31 0 0 1 0 62" fill="none" stroke="rgba(255,255,255,.18)" stroke-width="2"/>
  <rect x="98" y="42" width="11" height="11" rx="2" fill="#f59e0b" stroke="#0b1016" stroke-width="2" transform="rotate(-18 103 47)"/>
  <rect x="10" y="68" width="10" height="10" rx="2" fill="#f59e0b" stroke="#0b1016" stroke-width="2" transform="rotate(-18 15 73)"/>
</svg>`;

export class Title {
  visible = true;
  private t = 0;
  private readonly el: HTMLElement;

  constructor(hasSave: boolean, h: TitleHandlers) {
    this.el = document.getElementById('title')!;
    this.el.innerHTML = `<div class="title-inner">
      <div class="title-logo">${LOGO_SVG}</div>
      <h1>Beltwise</h1>
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
