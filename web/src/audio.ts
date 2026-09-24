// Tiny synthesized sound effects (no audio files). The AudioContext starts on the first
// user gesture, as browsers require.

type Sfx =
  | 'place'
  | 'remove'
  | 'error'
  | 'research'
  | 'objective'
  | 'stage'
  | 'click'
  | 'salvage'
  | 'achievement'
  | 'warning'
  | 'impact'
  | 'ark'
  | 'launch';

export class Sound {
  private ctx: AudioContext | null = null;
  private master: GainNode | null = null;
  private lastPlace = 0;
  enabled = true;

  constructor() {
    try {
      this.enabled = localStorage.getItem('beltwise.sound') !== '0';
    } catch {
      /* default on */
    }
    const unlock = () => {
      this.ensure();
      window.removeEventListener('pointerdown', unlock);
      window.removeEventListener('keydown', unlock);
    };
    window.addEventListener('pointerdown', unlock);
    window.addEventListener('keydown', unlock);
  }

  private ensure(): AudioContext | null {
    if (!this.ctx) {
      const Ctor = window.AudioContext ?? (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
      if (!Ctor) return null;
      this.ctx = new Ctor();
      this.master = this.ctx.createGain();
      this.master.gain.value = 0.18;
      this.master.connect(this.ctx.destination);
    }
    if (this.ctx.state === 'suspended') void this.ctx.resume();
    return this.ctx;
  }

  setEnabled(on: boolean): void {
    this.enabled = on;
    try {
      localStorage.setItem('beltwise.sound', on ? '1' : '0');
    } catch {
      /* not persisted */
    }
  }

  private tone(freq: number, start: number, dur: number, type: OscillatorType, vol = 1, slide = 0): void {
    const ctx = this.ctx!;
    const o = ctx.createOscillator();
    const g = ctx.createGain();
    const t = ctx.currentTime + start;
    o.type = type;
    o.frequency.setValueAtTime(freq, t);
    if (slide) o.frequency.exponentialRampToValueAtTime(freq * slide, t + dur);
    g.gain.setValueAtTime(0.0001, t);
    g.gain.exponentialRampToValueAtTime(vol, t + 0.01);
    g.gain.exponentialRampToValueAtTime(0.0001, t + dur);
    o.connect(g).connect(this.master!);
    o.start(t);
    o.stop(t + dur + 0.02);
  }

  play(name: Sfx): void {
    if (!this.enabled || !this.ensure()) return;
    switch (name) {
      case 'place': {
        const now = performance.now();
        if (now - this.lastPlace < 55) return; // drags place many tiles; keep it a gentle patter
        this.lastPlace = now;
        this.tone(520, 0, 0.06, 'triangle', 0.7, 0.7);
        break;
      }
      case 'click':
        this.tone(900, 0, 0.03, 'square', 0.25);
        break;
      case 'remove':
        this.tone(240, 0, 0.09, 'sawtooth', 0.35, 0.5);
        break;
      case 'error':
        this.tone(150, 0, 0.09, 'square', 0.35);
        this.tone(120, 0.1, 0.12, 'square', 0.35);
        break;
      case 'research':
        [523, 659, 784].forEach((f, i) => this.tone(f, i * 0.07, 0.25, 'triangle', 0.8));
        break;
      case 'objective':
        this.tone(784, 0, 0.18, 'sine', 0.9);
        this.tone(1175, 0.09, 0.3, 'sine', 0.8);
        break;
      case 'stage':
        [392, 523, 659, 784, 1047].forEach((f, i) => this.tone(f, i * 0.09, 0.6, 'triangle', 0.7));
        this.tone(196, 0, 1.2, 'sine', 0.6);
        break;
      case 'salvage':
        [330, 440, 660].forEach((f, i) => this.tone(f, i * 0.06, 0.18, 'square', 0.3));
        break;
      case 'achievement':
        [659, 880, 1319].forEach((f, i) => this.tone(f, i * 0.08, 0.35, 'sine', 0.7));
        break;
      case 'warning':
        this.tone(880, 0, 0.15, 'sawtooth', 0.3);
        this.tone(660, 0.2, 0.15, 'sawtooth', 0.3);
        break;
      case 'impact':
        this.tone(90, 0, 0.35, 'sawtooth', 0.5, 0.4);
        break;
      case 'ark':
        [262, 330, 392, 523].forEach((f, i) => this.tone(f, i * 0.12, 0.9, 'triangle', 0.6));
        this.tone(131, 0, 1.6, 'sine', 0.7);
        break;
      case 'launch':
        this.tone(60, 0, 3.5, 'sawtooth', 0.6, 4);
        [523, 659, 784, 1047, 1319].forEach((f, i) => this.tone(f, 1.2 + i * 0.15, 1.2, 'triangle', 0.5));
        break;
    }
  }
}
