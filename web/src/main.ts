// Entry point: fixed 60 Hz simulation, render at display rate (60/90/120/144 Hz) with
// interpolation. The per-frame path allocates nothing (the HUD formats a few strings at
// 5 Hz), so the GC has essentially nothing to collect during play.

import { Sound } from './audio';
import { Bench } from './bench';
import { Blueprints } from './blueprints';
import { Camera } from './camera';
import {
  Found,
  Kind,
  Lod,
  MAX_STEPS_PER_FRAME,
  NO_RECIPE,
  Reason,
  Stat,
  TICK_MS,
  TOOL_COPY,
  TOOL_DELETE,
  TOOL_MOVE,
  TOOL_PASTE,
  WORLD_SIZE,
} from './constants';
import { BELT_COLORS, formatCount } from './content';
import { Engine } from './engine';
import { Hud } from './hud';
import { buildingIcon, Icons } from './icons';
import { Input } from './input';
import { HELP_HTML, Sheets } from './panels';
import { Pwa } from './pwa';
import { hexToRgb, Renderer, type FrameParams } from './renderer';
import { Saves } from './save';
import { FramePacer, FPS_CAPS, QUALITIES, Settings, type FpsCap, type Quality } from './settings';
import { Title } from './title';

/** Zoom thresholds (CSS px per tile) for level of detail. */
const LOD_ITEMS = 8;
const LOD_STRUCTURES = 5;
const HUD_INTERVAL_MS = 200;
const AUTOSAVE_MS = 30_000;
const VERSION = __DEV__ ? 'dev' : '2.0';

function fatal(err: unknown): void {
  console.error(err);
  const el = document.getElementById('fatal')!;
  el.textContent = `Beltwise could not start: ${err instanceof Error ? err.message : String(err)}`;
  el.hidden = false;
}

function $(id: string): HTMLElement {
  return document.getElementById(id)!;
}

function duration(secs: number): string {
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  return h ? `${h}h ${m}m` : `${m} min`;
}

async function main(): Promise<void> {
  const params = new URLSearchParams(location.search);
  const canvas = document.getElementById('game') as HTMLCanvasElement;
  const engine = await Engine.load(WORLD_SIZE, WORLD_SIZE, Number(params.get('seed')) || 20260924);
  const content = engine.content;
  const cam = new Camera(engine.width, engine.height);
  const saves = new Saves(engine, cam);
  const sound = new Sound();
  const pwa = new Pwa();
  const settings = Settings.load();
  const pacer = new FramePacer();
  const icons = new Icons(content);
  const library = new Blueprints();

  const centerOnCore = (): void => {
    const s = engine.stats;
    cam.x = s[Stat.CoreX] + s[Stat.CoreSize] / 2;
    cam.y = s[Stat.CoreY] + s[Stat.CoreSize] / 2;
  };
  centerOnCore();
  const firstRun = !saves.hasLocal();
  const savedAt = firstRun ? null : saves.loadLocal();
  if (!firstRun && savedAt === null) centerOnCore();

  const renderer = new Renderer(canvas, engine, content, icons.atlas);

  let input: Input;
  let perfText = '';
  let blueprint: Uint8Array | null = null;
  let linkFrom: [number, number] | null = null;

  const setPlaceOptions = (): void => {
    const tool = hud.tool;
    engine.x.fx_set_place(tool >= 0 ? (hud.placeRecipe.get(tool) ?? NO_RECIPE) : NO_RECIPE, hud.placeFilter);
  };

  const hud = new Hud(engine, icons, {
    onTool: (tool) => selectTool(tool),
    onRotate: () => input.rotate(1),
    onUndo: () => undo(),
    onOpen: (panel) => sheets.open(panel),
    onFullscreen: () => void pwa.toggleFullscreen(),
    onOverlay: () => {
      hud.setOverlay(!hud.overlay);
      sound.play('click');
      if (hud.overlay) hud.toast('Showing power coverage and stuck machines', 'info', 1800);
    },
    onPlaceOption: () => {
      setPlaceOptions();
      sound.play('click');
    },
    onBlueprint: (action) => {
      if (action === 'cancel') return selectTool(TOOL_MOVE);
      if (!blueprint) return;
      const name = prompt('Name this blueprint', `Blueprint ${library.all().length + 1}`);
      if (name === null) return;
      hud.toast(library.add(name || 'Blueprint', blueprint) ? `Saved <b>${name}</b> to your blueprints` : 'Could not save (storage full)', 'good');
    },
  });

  const activateBlueprint = (bytes: Uint8Array): void => {
    const cells = engine.loadBlueprint(bytes);
    if (!cells) {
      hud.toast('That blueprint is empty', 'bad');
      return;
    }
    blueprint = bytes;
    hud.blueprintInfo = `${cells} building${cells === 1 ? '' : 's'}`;
    selectTool(TOOL_PASTE);
    input.centerCursor();
    sheets.close();
  };

  const sheets = new Sheets(engine, icons, {
    research(t) {
      if (engine.x.fx_research(t)) {
        sound.play('research');
        const tech = content.tech[t];
        const unlocks = tech.unlocks.map((k) => `${icons.img(buildingIcon(k), 20)} ${content.buildings[k].name}`).join(', ');
        hud.toast(`Researched <b>${tech.name}</b>${unlocks ? ` · ${unlocks}` : ''}`, 'good');
        hud.refreshTools();
      } else sound.play('error');
    },
    menu: (action) => void menuAction(action),
    inspectAction(action, x, y) {
      const [name, v] = action.split(':');
      const value = Number(v);
      switch (name) {
        case 'remove':
          engine.x.fx_edit_begin();
          if (engine.x.fx_remove(x, y)) sound.play('remove');
          sheets.close();
          return;
        case 'rotate': {
          engine.x.fx_edit_begin();
          const t = engine.x.fx_tile(x, y);
          engine.x.fx_place(x, y, t & 255, (((t >> 8) & 255) + 1) & 3);
          break;
        }
        case 'recipe':
          engine.x.fx_set_recipe(x, y, value);
          break;
        case 'filter':
          engine.x.fx_set_filter(x, y, value);
          break;
        case 'shards':
          if (!engine.x.fx_set_shards(x, y, value)) return void sound.play('error');
          break;
        case 'amp':
          if (!engine.x.fx_set_amp(x, y, value)) return void sound.play('error');
          break;
        case 'link':
          linkFrom = [x, y];
          sheets.close();
          selectTool(TOOL_MOVE);
          hud.toast('Tap the Drone Port this one should send to', 'info', 5000);
          return;
        case 'unlink':
          engine.x.fx_link(x, y, -1, -1);
          break;
      }
      sound.play('click');
    },
    menuState: () => ({
      sound: sound.enabled,
      perf: hud.showPerf,
      canInstall: pwa.canInstall,
      iosInstall: pwa.ios,
      standalone: pwa.standalone,
      fullscreen: pwa.fullscreen,
      fullscreenSupported: pwa.fullscreenSupported,
      lastSave: saves.describe(),
      version: VERSION,
      fps: settings.fps,
      quality: settings.quality,
      night: settings.night,
      displayHz: pacer.displayHz,
    }),
    ark() {
      const phase = engine.stats[Stat.ArkPhase];
      if (engine.x.fx_ark()) {
        sound.play('ark');
        const next = content.ark[phase + 1];
        hud.toast(
          `<b>The Ark: ${content.ark[phase].name} complete!</b>${next ? ` · Tier ${phase + 2} research unlocked` : ' · Ready to launch'}`,
          'epic',
          6000,
        );
        hud.refreshTools();
      } else sound.play('place');
    },
    launch: (planet) => void launch(planet),
    salvage(x, y) {
      const found = engine.x.fx_salvage(x, y);
      if (found === Found.Nothing) {
        sound.play('error');
        return;
      }
      sound.play('salvage');
      const report = engine.report();
      const [title, html] = sheets.salvageHtml(report);
      sheets.showInfo(title, html);
    },
    chooseAlt(k) {
      if (engine.x.fx_choose_alt(k)) {
        sound.play('research');
        const [b, r] = content.alternates[k];
        hud.toast(`Unlocked <b>${content.buildings[b].recipes[r].name}</b> for the ${content.buildings[b].name}`, 'good', 4000);
        if (engine.stats[Stat.Probes] > 0) sheets.open('probe');
        else sheets.close();
        hud.refreshTools();
      } else sound.play('error');
    },
    buy(i) {
      if (engine.x.fx_buy(i)) {
        sound.play('research');
        hud.toast(`Bought <b>${content.shop[i].name}</b>`, 'good');
      } else sound.play('error');
    },
    cosmetic(bit) {
      if (engine.x.fx_cosmetic(bit)) sound.play('click');
    },
    blueprint(action, id) {
      switch (action) {
        case 'use': {
          const bytes = library.bytes(id);
          if (bytes) activateBlueprint(bytes);
          break;
        }
        case 'share': {
          const code = library.code(id);
          if (navigator.clipboard?.writeText) {
            navigator.clipboard.writeText(code).then(
              () => hud.toast('Share code copied to the clipboard', 'good'),
              () => prompt('Copy this blueprint code', code),
            );
          } else prompt('Copy this blueprint code', code);
          break;
        }
        case 'rename': {
          const name = prompt('New name', library.get(id)?.name ?? '');
          if (name) library.rename(id, name);
          break;
        }
        case 'delete':
          if (confirm(`Delete "${library.get(id)?.name}"?`)) library.remove(id);
          break;
        case 'import': {
          const code = prompt('Paste a blueprint code (starts with BW1.)');
          if (!code) break;
          const bytes = Blueprints.parse(code);
          if (!bytes) {
            hud.toast('That is not a blueprint code', 'bad');
            break;
          }
          library.add('Imported', bytes);
          activateBlueprint(bytes);
          break;
        }
      }
    },
    blueprints: () => library.all(),
  });

  const selectTool = (tool: number): void => {
    if (tool === TOOL_PASTE && !blueprint) tool = TOOL_COPY;
    input.setTool(tool);
    hud.setTool(tool);
    setPlaceOptions();
    if (sheets.kind === 'inspect') sheets.close();
    if (tool === TOOL_PASTE) input.centerCursor();
    sound.play('click');
  };

  const undo = (): void => {
    if (engine.x.fx_undo()) sound.play('remove');
  };

  const reasonText = (reason: number, kind: number): string => {
    const b = content.buildings[kind];
    switch (reason) {
      case Reason.Locked:
        return `${b.name} needs research: <b>${content.tech[b.research]?.name ?? '?'}</b>`;
      case Reason.Cost: {
        const storage = engine.storage;
        const missing = b.cost
          .filter(([it, n]) => storage[it] < n)
          .map(([it, n]) => `${icons.img(it, 18)}${n - storage[it]} ${content.items[it].name}`)
          .join(', ');
        return `Not enough materials in the Core: need ${missing}`;
      }
      case Reason.NoDeposit:
        return `${b.name} must be placed on a resource deposit`;
      case Reason.WrongDeposit:
        return b.deposits.length === 1
          ? `${b.name} only works on ${content.items[b.deposits[0]].name} seeps`
          : 'Oil seeps need an Oil Pump';
      case Reason.DepositLocked:
        return 'Research this resource first (Titanium, Nuclear, Oil Processing or Xenometallurgy)';
      case Reason.Occupied:
        return 'Something is already there';
      case Reason.Fog:
        return 'Unexplored land: build closer, or place a Power Pole or Radar to see further';
      case Reason.Lava:
        return 'Lava: nothing can be built here. Tunnel under it or fly over with drones';
      case Reason.NoWater:
        return kind === Kind.WaterPump ? 'Water Pumps go on a lake' : `${b.name} must touch a lake`;
      default:
        return 'Out of bounds';
    }
  };

  const selRect = $('selrect');
  input = new Input(canvas, cam, engine, {
    onPlaced: () => sound.play('place'),
    onRemoved: () => sound.play('remove'),
    onRefused(reason, kind) {
      sound.play('error');
      hud.toast(reasonText(reason, kind), 'bad');
    },
    onInspect(x, y) {
      const t = engine.x.fx_tile(x, y);
      if (t === 0xffffffff) return;
      const kind = t & 255;
      const res = (t >> 16) & 255;
      const fog = (t & 0x80000000) !== 0;
      if (linkFrom) {
        const [fx, fy] = linkFrom;
        linkFrom = null;
        if (kind === Kind.DronePort && engine.x.fx_link(fx, fy, x, y)) {
          sound.play('research');
          hud.toast('Linked: everything fed into the first port now flies here', 'good');
          sheets.open('inspect', [fx, fy]);
        } else {
          sound.play('error');
          hud.toast('Linking cancelled: that is not another Drone Port', 'bad');
        }
        return;
      }
      if (fog) {
        hud.toast('Unexplored land', 'info', 1200);
        return;
      }
      if (kind === Kind.Wreck) {
        sheets.open('wreck', [x, y]);
        sound.play('click');
      } else if (kind > 2) {
        sheets.open('inspect', [x, y]);
        sound.play('click');
      } else if (kind === 2) sheets.open('core');
      else if (res) {
        const purity = content.purity[(t >> 24) & 3];
        hud.toast(`${icons.img(res, 20)} ${purity} ${content.items[res].name} deposit: place a ${res === 8 ? 'Oil Pump' : 'Drill'} on it`, 'info');
      } else if ((t & 0x40000000) !== 0) hud.toast('A lake: Water Pumps and Hatcheries can use it', 'info');
      else if (sheets.kind === 'inspect') sheets.close();
    },
    onRotate: (dir) => hud.setDirection(dir),
    onKey(k, e) {
      if (k === 'escape') {
        linkFrom = null;
        if (sheets.kind) sheets.close();
        else selectTool(TOOL_MOVE);
      } else if (k === 'q') selectTool(TOOL_MOVE);
      else if (k === 'x') selectTool(TOOL_DELETE);
      else if (k === 'b') selectTool(TOOL_COPY);
      else if (k === 'v') selectTool(TOOL_PASTE);
      else if (k === 'o') hud.setOverlay(!hud.overlay);
      else if (k === 'z' || k === 'undo') undo();
      else if (k === 'f') void pwa.toggleFullscreen();
      else if (k === 't') sheets.open('research');
      else if (k === 'c') sheets.open('core');
      else if (k === 'p') sheets.open('planet');
      else if (k === 'k') sheets.open('ark');
      else if (k === 'f3' || k === '`') hud.togglePerf();
      else if (k === 'tab') hud.setCategory((hud.category + (e.shiftKey ? 4 : 1)) % 5);
      else if (k >= '1' && k <= '9') {
        const kinds = hud.unlockedIn(hud.category);
        const kind = kinds[Number(k) - 1];
        if (kind !== undefined) selectTool(kind);
      } else return false;
      return true;
    },
    onSelect(rect) {
      selection = rect;
      selRect.hidden = !rect;
    },
    onCopy(x0, y0, x1, y1) {
      const bytes = engine.captureBlueprint(x0, y0, x1, y1);
      if (!bytes) {
        hud.toast('Nothing to copy there: drag over buildings', 'info');
        return;
      }
      sound.play('research');
      activateBlueprint(bytes);
      hud.toast(`Copied ${hud.blueprintInfo}. Tap to paste, ⟳ to rotate, Save to keep it`, 'good', 3500);
    },
    onPaste(x, y, rot) {
      engine.x.fx_edit_begin();
      const cells = blueprint ? blueprint.length / 8 : 0;
      const placed = engine.x.fx_bp_paste(x, y, rot);
      if (placed) sound.play('place');
      else sound.play('error');
      if (placed < cells) hud.toast(`Placed ${placed} of ${cells}: the rest is blocked, locked or unaffordable`, placed ? 'info' : 'bad');
    },
  });
  let selection: [number, number, number, number] | null = null;
  hud.setTool(TOOL_MOVE);
  hud.setDirection(0);
  if (__DEV__) Object.assign(window, { fx: { engine, cam, renderer, input, hud, sheets, library } });

  const worldChanged = (): void => {
    renderer.uploadWorld();
    hud.refreshTools();
    lastRevision = -1;
    lastAch = engine.stats[Stat.LastAch];
  };

  const bench = new Bench(engine, cam, {
    onWorldChanged: worldChanged,
    onDone(html) {
      saves.paused = false;
      sheets.showInfo('Benchmark results', html);
      document.body.classList.remove('benchmarking');
    },
  });

  // The Ark lifts off: a short cinematic, then a new planet.
  let launching = false;
  async function launch(planet: number): Promise<void> {
    if (launching || !engine.x.fx_can_launch(planet)) return;
    const p = content.planets[planet];
    if (!confirm(`Launch the Ark to ${p.name}? This planet stays behind; you keep credits, cosmetics and achievements.`)) return;
    launching = true;
    sheets.close();
    saves.saveLocal();
    sound.play('launch');
    centerOnCore();
    const zoom0 = cam.zoom;
    document.body.classList.add('launching');
    const t0 = performance.now();
    await new Promise<void>((resolve) => {
      const step = (now: number) => {
        const t = Math.min(1, (now - t0) / 4200);
        engine.x.fx_set_launch(Math.round(t * 255));
        cam.zoom = zoom0 * (1 - 0.5 * t * t);
        $('launch-rocket').style.setProperty('--t', String(t));
        if (t < 1) requestAnimationFrame(step);
        else resolve();
      };
      requestAnimationFrame(step);
    });
    if (engine.x.fx_launch(planet)) {
      worldChanged();
      centerOnCore();
      cam.zoom = initialZoom();
      saves.saveLocal();
      lastObjective = engine.stats[Stat.Objective];
      lastStage = engine.stats[Stat.Stage];
      hud.toast(`<b>Welcome to ${p.name}</b> · ${p.desc} Production is now +${engine.stats[Stat.Legacy] * content.legacyPct}%.`, 'epic', 8000);
      sound.play('stage');
    }
    document.body.classList.remove('launching');
    launching = false;
  }

  async function menuAction(action: string): Promise<void> {
    sound.play('click');
    const [name, value] = action.split(':');
    if (name === 'fps' && FPS_CAPS.includes(Number(value) as FpsCap)) {
      settings.fps = Number(value) as FpsCap;
      settings.save();
      return;
    }
    if (name === 'quality' && QUALITIES.includes(value as Quality)) {
      settings.quality = value as Quality;
      settings.save();
      applyQuality();
      return;
    }
    if (name === 'night') {
      settings.night = value === 'true';
      settings.save();
      return;
    }
    switch (action) {
      case 'save':
        hud.toast(saves.saveLocal() ? 'Game saved' : 'Could not save (storage full or disabled). Try Export.', 'info');
        break;
      case 'export':
        saves.exportFile();
        break;
      case 'import':
        if (await saves.importFile()) {
          worldChanged();
          sheets.close();
          centerOnCore();
          hud.toast('Save loaded', 'good');
        } else hud.toast('That file is not a Beltwise save', 'bad');
        break;
      case 'new':
        if (confirm('Start over on a new planet? Your current game will be replaced (export it first to keep it).')) {
          engine.reset((Math.random() * 0xffffffff) >>> 0);
          saves.clearLocal();
          centerOnCore();
          cam.zoom = initialZoom();
          worldChanged();
          sheets.close();
          saves.saveLocal();
        }
        break;
      case 'help':
        sheets.showInfo('How to play', HELP_HTML);
        break;
      case 'fullscreen':
        await pwa.toggleFullscreen();
        break;
      case 'sound':
        sound.setEnabled(!sound.enabled);
        break;
      case 'perf':
        hud.togglePerf();
        break;
      case 'install':
        await pwa.install();
        break;
      case 'bench':
        sheets.close();
        saves.saveLocal();
        saves.paused = true;
        document.body.classList.add('benchmarking');
        bench.begin();
        hud.toast('Benchmark running: 32k belts, 43k items, 4k machines · about 20 seconds', 'info', 19_000);
        break;
    }
  }

  // Drawing-buffer resolution: device pixel ratio capped by the quality setting. On Auto it
  // is also lowered when the device can't hold its frame rate (fill rate is the usual limit
  // on low-end phones) and raised again once frames are smooth.
  const forced = Number(params.get('dpr'));
  let maxRatio = 1;
  let ratio = 1;
  let avgMs = 16.7;
  let slowFor = 0;
  let fastFor = 0;
  let settleUntil = performance.now() + 3000;
  const resize = (): void => {
    cam.width = Math.max(1, canvas.clientWidth);
    cam.height = Math.max(1, canvas.clientHeight);
    cam.pixelRatio = ratio;
    renderer.resize(Math.round(cam.width * ratio), Math.round(cam.height * ratio));
  };
  const applyQuality = (): void => {
    maxRatio = forced > 0 ? forced : settings.maxPixelRatio(window.devicePixelRatio || 1);
    ratio = maxRatio;
    slowFor = fastFor = 0;
    settleUntil = performance.now() + 1500;
    resize();
  };
  new ResizeObserver(resize).observe(canvas);
  applyQuality();
  const initialZoom = () => Math.min(44, Math.max(18, Math.min(cam.width, cam.height) / 22));
  if (firstRun) cam.zoom = initialZoom();

  canvas.addEventListener('webglcontextlost', (e) => {
    e.preventDefault();
    saves.saveLocal();
    fatal(new Error('the graphics context was lost. Reload the page to continue; your game was saved.'));
  });

  // Persistence.
  setInterval(() => saves.saveLocal(), AUTOSAVE_MS);
  document.addEventListener('visibilitychange', () => {
    if (document.hidden) saves.saveLocal();
    settleUntil = performance.now() + 3000;
  });
  window.addEventListener('pagehide', () => saves.saveLocal());

  pwa.onUpdate = () => hud.toast('Beltwise was updated. Reload to get the new version.', 'info', 6000);
  pwa.register();

  // Production while the game was closed.
  const welcomeBack = (): void => {
    if (!savedAt) return;
    const away = Math.floor(Date.now() / 1000 - savedAt);
    if (away < 120 || !engine.x.fx_offline(away)) return;
    const r = engine.report();
    const secs = r[0];
    const ti = r[1] + r[2] * 4294967296;
    const items: string[] = [];
    for (let k = 3; k + 1 < r.length; k += 2) {
      items.push(`<span class="stack">${icons.img(r[k], 24)}+${formatCount(r[k + 1])}</span>`);
    }
    if (!items.length && !ti) return;
    sheets.showInfo(
      'Welcome back',
      `<p>You were away for <b>${duration(away)}</b>${away > secs ? ` (production counts up to ${duration(secs)})` : ''}. Your factory kept working:</p>
      <div class="loot">${items.join('')}</div>${ti ? `<p class="hint">Terraforming: <b>+${formatCount(ti)} Ti</b></p>` : ''}`,
    );
  };

  const title = new Title(!firstRun && savedAt !== null, {
    onPlay() {
      sound.play('click');
      if (firstRun) sheets.showInfo('How to play', HELP_HTML);
      else welcomeBack();
    },
    onHelp: () => sheets.showInfo('How to play', HELP_HTML),
  });
  if (params.has('bench') || params.has('notitle')) title.hide();

  // ---- Frame loop ---------------------------------------------------------------------

  const terra = new Float32Array(4);
  const terraTarget = new Float32Array(4);
  const ambient = new Float32Array([1, 1, 1]);
  const stripe = new Float32Array(3);
  const fp: FrameParams = {
    cam,
    instances: 0,
    beltPhase: 0,
    time: 0,
    terra,
    map: false,
    fx: 2,
    ambient,
    overlay: 0,
    planet: 0,
    stripe,
    stage: 0,
  };
  const level = (v: number, full: number) =>
    Math.min(1, Math.max(0, (Math.log10(1 + v) - 2) / (Math.log10(full) - 2)));
  const stripeColors = BELT_COLORS.map(hexToRgb);

  let last = performance.now();
  let acc = 0;
  let frames = 0;
  let frameTime = 0;
  let simTime = 0;
  let buildTime = 0;
  let drawTime = 0;
  let hudAt = 0;
  let sheetAt = 0;
  let lastRevision = -1;
  let lastObjective = engine.stats[Stat.Objective];
  let lastStage = engine.stats[Stat.Stage];
  let lastAch = engine.stats[Stat.LastAch];
  let lastShowerIn = 0;
  let lastMeteors = engine.stats[Stat.Meteors];
  let dark = 0;

  const frame = (now: number): void => {
    requestAnimationFrame(frame);
    // The benchmark measures the device, so it ignores the frame-rate cap.
    if (!pacer.ready(now, bench.running ? 0 : settings.fps)) return;
    let dt = now - last;
    last = now;
    if (dt > 250) dt = 250; // back from a background tab: don't fast-forward

    input.update(dt / 1000);
    if (bench.running) bench.update(now);
    if (title.visible) title.update(cam, dt, engine);

    acc += dt;
    let steps = Math.floor(acc / TICK_MS);
    if (steps > MAX_STEPS_PER_FRAME) {
      steps = MAX_STEPS_PER_FRAME;
      acc = 0;
    } else {
      acc -= steps * TICK_MS;
    }
    const t0 = performance.now();
    if (steps > 0) engine.x.fx_tick(steps);
    const t1 = performance.now();
    const alpha = acc / TICK_MS;

    // Nightfall: cool and dim, with warm light around dawn and dusk.
    const s = engine.stats;
    const day = s[Stat.Daylight] / 1000;
    const night = settings.night ? 1 - day : 0;
    dark += (night - dark) * (1 - Math.exp(-dt / 400));
    const glowWarm = 4 * day * (1 - day);
    ambient[0] = (1 - dark * 0.62) * (1 + glowWarm * 0.06);
    ambient[1] = (1 - dark * 0.56) * (1 - glowWarm * 0.02);
    ambient[2] = (1 - dark * 0.38) * (1 - glowWarm * 0.08);

    engine.refreshViews();
    const fx = settings.effects;
    const tool = hud.tool;
    const powerTool = tool >= 0 && (content.buildings[tool].category === 3 || content.buildings[tool].power > 0);
    const flags =
      (cam.zoom >= LOD_ITEMS ? Lod.Items : 0) |
      (cam.zoom >= LOD_STRUCTURES ? Lod.Structures : 0) |
      (hud.overlay ? Lod.Status : 0) |
      (settings.night && fx > 0 && dark > 0.1 ? Lod.Lights : 0) |
      (fx > 0 && cam.zoom >= LOD_STRUCTURES ? Lod.Weather : 0);
    const count = engine.x.fx_render(cam.left, cam.top, cam.right, cam.bottom, alpha, flags);
    const t2 = performance.now();

    // Terraforming visuals ease towards the meters so stage changes animate.
    const meters = content.meters;
    terraTarget[0] = level(engine.stat64(Stat.Meters), meters[0].full);
    terraTarget[1] = level(engine.stat64(Stat.Meters + 2), meters[1].full) * (0.35 + 0.65 * terraTarget[0]);
    terraTarget[2] = level(engine.stat64(Stat.Meters + 4), meters[2].full);
    terraTarget[3] = level(engine.stat64(Stat.Meters + 6), meters[3].full);
    const ease = 1 - Math.exp(-dt / 900);
    for (let i = 0; i < 4; i++) terra[i] += (terraTarget[i] - terra[i]) * ease;

    // Items are drawn one tick behind (interpolated), so stripes use the same clock. The
    // stripe pattern repeats every half tile, so the phase can wrap at 1 tile seamlessly.
    fp.instances = count;
    fp.beltPhase = ((s[Stat.Tick] - 1 + alpha) * engine.beltSpeed()) % 1;
    fp.time = now / 1000;
    fp.map = cam.zoom < LOD_STRUCTURES;
    fp.fx = fx;
    fp.overlay = hud.overlay ? 1 : powerTool ? 0.75 : 0;
    fp.planet = s[Stat.Planet];
    const stageNow = s[Stat.Stage];
    const here = content.stages[stageNow];
    const next = content.stages[stageNow + 1];
    const into = next ? Math.min(1, Math.max(0, (engine.stat64(Stat.TiLo) - here.ti) / (next.ti - here.ti))) : 0;
    fp.stage += (stageNow + into - fp.stage) * ease;
    stripe.set(stripeColors[s[Stat.BeltColor]] ?? stripeColors[0]);
    renderer.draw(fp);
    const t3 = performance.now();

    if (selection) {
      // Copy tool: outline the tiles being selected.
      const [ax, ay, bx, by] = selection;
      const x0 = (Math.min(ax, bx) - cam.left) * cam.zoom;
      const y0 = (Math.min(ay, by) - cam.top) * cam.zoom;
      selRect.style.transform = `translate(${x0}px, ${y0}px)`;
      selRect.style.width = `${(Math.abs(bx - ax) + 1) * cam.zoom}px`;
      selRect.style.height = `${(Math.abs(by - ay) + 1) * cam.zoom}px`;
    }

    if (bench.running) bench.record({ dt, simMs: t1 - t0, buildMs: t2 - t1, drawMs: t3 - t2, instances: count });

    // Adaptive resolution (Auto quality): step down after 2 s of frames 30% slower than the
    // target (the cap, or the display rate), probe back up after 30 s of smooth frames.
    // Hidden/just-resumed pages are ignored since their rAF is throttled.
    avgMs += (dt - avgMs) * 0.05;
    const target = Math.max(pacer.rafMs, settings.fps > 0 ? 1000 / settings.fps : 0);
    if (document.hidden || now < settleUntil || bench.running || !settings.adaptive) {
      slowFor = fastFor = 0;
    } else if (avgMs > target * 1.3 + 1) {
      fastFor = 0;
      slowFor += dt;
      if (slowFor > 2000 && ratio > 1) {
        ratio = Math.max(1, ratio - 0.25);
        slowFor = 0;
        settleUntil = now + 1000;
        resize();
      }
    } else {
      slowFor = 0;
      fastFor = avgMs < target * 1.05 + 0.5 ? fastFor + dt : 0;
      if (fastFor > 30000 && ratio < maxRatio) {
        ratio = Math.min(maxRatio, ratio + 0.25);
        fastFor = 0;
        settleUntil = now + 1000;
        resize();
      }
    }

    frames++;
    frameTime += dt;
    simTime += t1 - t0;
    buildTime += t2 - t1;
    drawTime += t3 - t2;
    if (now - hudAt >= HUD_INTERVAL_MS) {
      const n = Math.max(1, frames);
      if (hud.showPerf) {
        perfText =
          `${((frames * 1000) / Math.max(1, frameTime)).toFixed(0)} fps · ${(frameTime / n).toFixed(1)} ms · ${settings.fps || 'max'}/${pacer.displayHz}Hz\n` +
          `sim ${(simTime / n).toFixed(2)} · build ${(buildTime / n).toFixed(2)} · gpu ${(drawTime / n).toFixed(2)} ms\n` +
          `${formatCount(count)} sprites · ${formatCount(s[Stat.Items])} items\n` +
          `${formatCount(s[Stat.Belts])} belts · ${formatCount(s[Stat.Machines])} machines · ${ratio}x`;
      }
      frames = frameTime = simTime = buildTime = drawTime = 0;
      hudAt = now;
      if (!bench.running) {
        hud.update(perfText);
        events();
      } else if (hud.showPerf) hud.update(perfText);
    }
    if (sheets.kind && now - sheetAt > 500) {
      sheetAt = now;
      sheets.render();
    }
  };

  /** Toasts and sounds for things that happened in the simulation. */
  const events = (): void => {
    const s = engine.stats;
    const showerIn = s[Stat.ShowerIn];
    if (showerIn && !lastShowerIn) {
      sound.play('warning');
      hud.toast(`<b>Meteor shower incoming!</b> · Meteorites will land near the Core in ${Math.ceil(showerIn / 60)} s`, 'epic', 5000);
    }
    lastShowerIn = showerIn;
    const meteors = s[Stat.Meteors];
    if (meteors > lastMeteors) sound.play('impact');
    lastMeteors = meteors;
    const rev = s[Stat.Revision];
    if (rev === lastRevision) return;
    lastRevision = rev;
    hud.refreshTools();
    const obj = s[Stat.Objective];
    if (obj > lastObjective) {
      const done = content.objectives[obj - 1];
      const reward = (done?.reward ?? []).map(([it, n]) => `${icons.img(it, 18)}${n}`).join(' ');
      hud.toast(`Objective complete${done ? `: ${done.text}` : ''}${reward ? ` · +${reward}` : ''}`, 'good', 3500);
      sound.play('objective');
    }
    const stage = s[Stat.Stage];
    if (stage > lastStage) {
      const st = content.stages[stage];
      const reward = [
        st.credits ? `+${formatCount(st.credits)} credits` : '',
        st.shards ? `${icons.img(36, 18)} +${st.shards} power shard${st.shards > 1 ? 's' : ''}` : '',
      ]
        .filter(Boolean)
        .join(' · ');
      hud.toast(`<b>${st.name}</b> · ${st.desc}${reward ? ` · ${reward}` : ''}`, 'epic', 6000);
      sound.play('stage');
    }
    const ach = s[Stat.LastAch];
    if (ach !== lastAch && ach < content.achievements.length) {
      const a = content.achievements[ach];
      hud.toast(`<b>★ ${a.name}</b> · ${a.desc} · +${formatCount(a.credits)} credits${a.shards ? ` and ${a.shards} power shard${a.shards > 1 ? 's' : ''}` : ''}`, 'good', 4500);
      sound.play('achievement');
    }
    lastAch = ach;
    lastObjective = obj;
    lastStage = stage;
  };
  requestAnimationFrame(frame);

  if (params.has('bench')) void menuAction('bench');
}

main().catch(fatal);
