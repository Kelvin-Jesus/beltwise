// Offline play and installation: service worker registration, the install prompt (Chrome /
// Android), iOS "Add to Home Screen" detection and the Fullscreen API.

interface BeforeInstallPromptEvent extends Event {
  prompt(): Promise<void>;
  userChoice: Promise<{ outcome: 'accepted' | 'dismissed' }>;
}

export class Pwa {
  private installEvent: BeforeInstallPromptEvent | null = null;
  /** Called when a new version finished installing in the background. */
  onUpdate: () => void = () => {};

  constructor() {
    window.addEventListener('beforeinstallprompt', (e) => {
      e.preventDefault();
      this.installEvent = e as BeforeInstallPromptEvent;
    });
    window.addEventListener('appinstalled', () => (this.installEvent = null));
  }

  /** Registers the service worker (production builds on HTTPS or localhost only). */
  register(): void {
    if (__DEV__ || !('serviceWorker' in navigator) || !window.isSecureContext) return;
    const hadController = !!navigator.serviceWorker.controller;
    navigator.serviceWorker.register(new URL('sw.js', document.baseURI)).catch(() => {
      /* offline support is optional */
    });
    navigator.serviceWorker.addEventListener('controllerchange', () => {
      if (hadController) this.onUpdate();
    });
  }

  get canInstall(): boolean {
    return !!this.installEvent;
  }

  async install(): Promise<void> {
    if (!this.installEvent) return;
    await this.installEvent.prompt();
    this.installEvent = null;
  }

  get standalone(): boolean {
    return (
      window.matchMedia('(display-mode: standalone), (display-mode: fullscreen)').matches ||
      (navigator as unknown as { standalone?: boolean }).standalone === true
    );
  }

  get ios(): boolean {
    return /iphone|ipad|ipod/i.test(navigator.userAgent) || (navigator.platform === 'MacIntel' && navigator.maxTouchPoints > 1);
  }

  get fullscreenSupported(): boolean {
    return !!document.fullscreenEnabled;
  }

  get fullscreen(): boolean {
    return !!document.fullscreenElement;
  }

  async toggleFullscreen(): Promise<void> {
    try {
      if (document.fullscreenElement) await document.exitFullscreen();
      else await document.documentElement.requestFullscreen({ navigationUI: 'hide' });
    } catch {
      /* denied or unsupported */
    }
  }
}
