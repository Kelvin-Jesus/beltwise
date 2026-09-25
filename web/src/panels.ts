// Sheets: research, the Ark, planet, core (storage, production, power), menu, shop,
// achievements, journal, blueprints, wrecks and the building inspector. One sheet is open
// at a time; while open it re-renders at 2 Hz so counts and affordability stay live.

import { Found, Info, MachineFlag, NO_RECIPE, NONE, Stat } from './constants';
import {
  ALT_BASE,
  METER_COLORS,
  formatCount,
  formatPower,
  recipeName,
  type Content,
  type RecipeDef,
  type Stack,
} from './content';
import type { Engine } from './engine';
import { SVG, canAfford, costChips, knownItems, recipeOpen } from './hud';
import { buildingIcon, type Icons } from './icons';

export type SheetKind =
  | 'research'
  | 'planet'
  | 'core'
  | 'menu'
  | 'inspect'
  | 'info'
  | 'ark'
  | 'wreck'
  | 'probe'
  | 'shop'
  | 'achievements'
  | 'journal'
  | 'blueprints';

export interface MenuState {
  sound: boolean;
  perf: boolean;
  canInstall: boolean;
  iosInstall: boolean;
  standalone: boolean;
  fullscreen: boolean;
  fullscreenSupported: boolean;
  lastSave: string;
  version: string;
  fps: number;
  quality: string;
  night: boolean;
  displayHz: number;
}

export interface BlueprintEntry {
  id: string;
  name: string;
  cells: number;
}

export interface PanelActions {
  research(t: number): void;
  menu(action: string): void;
  /** Inspector actions: rotate, remove, recipe:r, filter:i, shards:n, amp:0|1, link, unlink. */
  inspectAction(action: string, x: number, y: number): void;
  menuState(): MenuState;
  ark(): void;
  launch(planet: number): void;
  salvage(x: number, y: number): void;
  chooseAlt(k: number): void;
  buy(i: number): void;
  cosmetic(bit: number): void;
  blueprint(action: string, id: string): void;
  blueprints(): BlueprintEntry[];
}

function $(id: string): HTMLElement {
  return document.getElementById(id)!;
}

const STATUS = ['Idle', 'Working', 'Waiting for input', 'Output blocked', 'No power', 'Out of fuel', 'Not linked'];
const TIER_NAMES = ['Landing', 'Atmosphere', 'Chemistry', 'Titanium & Life', 'Nuclear & Stars'];

export class Sheets {
  kind: SheetKind | null = null;
  private inspectAt: [number, number] = [0, 0];
  private infoTitle = '';
  private infoHtml = '';
  private lastHtml = '';
  private coreTab: 'storage' | 'production' | 'power' = 'storage';

  constructor(
    private readonly engine: Engine,
    private readonly icons: Icons,
    private readonly actions: PanelActions,
  ) {
    $('sheet-close').addEventListener('click', () => this.close());
    $('sheet-backdrop').addEventListener('click', () => this.close());
    $('sheet-body').addEventListener('click', (e) => {
      const b = (e.target as HTMLElement).closest<HTMLElement>('[data-act]');
      if (!b || (b as HTMLButtonElement).disabled) return;
      const act = b.dataset.act!;
      const [name, value] = act.split(':');
      switch (name) {
        case 'research':
          actions.research(Number(value));
          break;
        case 'i':
          actions.inspectAction(act.slice(2), ...this.inspectAt);
          break;
        case 'tab':
          this.coreTab = value as typeof this.coreTab;
          break;
        case 'open':
          this.open(value as SheetKind);
          return;
        case 'ark':
          actions.ark();
          break;
        case 'launch':
          actions.launch(Number(value));
          break;
        case 'salvage':
          actions.salvage(...this.inspectAt);
          return;
        case 'alt':
          actions.chooseAlt(Number(value));
          return;
        case 'buy':
          actions.buy(Number(value));
          break;
        case 'wear':
          actions.cosmetic(Number(value));
          break;
        case 'bp':
          actions.blueprint(value, b.dataset.id ?? '');
          break;
        default:
          actions.menu(act);
      }
      this.render(true);
    });
  }

  get content(): Content {
    return this.engine.content;
  }

  open(kind: SheetKind, at?: [number, number]): void {
    if (at) this.inspectAt = at;
    const fresh = this.kind !== kind;
    this.kind = kind;
    $('sheet').hidden = false;
    $('sheet-backdrop').hidden = kind === 'inspect';
    $('sheet').classList.toggle('compact', kind === 'inspect');
    this.render(true);
    if (fresh) $('sheet-body').scrollTop = 0;
  }

  close(): void {
    this.kind = null;
    $('sheet').hidden = true;
    $('sheet-backdrop').hidden = true;
  }

  /** Shows static content (help, results, logs). */
  showInfo(title: string, html: string): void {
    this.infoTitle = title;
    this.infoHtml = html;
    this.open('info');
  }

  /** Re-renders the open sheet (called at 2 Hz and after actions). */
  render(force = false): void {
    if (!this.kind) return;
    let title = '';
    let html = '';
    switch (this.kind) {
      case 'research':
        title = 'Research';
        html = this.research();
        break;
      case 'planet':
        title = 'Planet';
        html = this.planet();
        break;
      case 'core':
        title = 'Core';
        html = this.core();
        break;
      case 'menu':
        title = 'Beltwise';
        html = this.menu();
        break;
      case 'ark':
        title = 'The Ark';
        html = this.ark();
        break;
      case 'shop':
        title = 'Shop';
        html = this.shop();
        break;
      case 'achievements':
        title = 'Achievements';
        html = this.achievements();
        break;
      case 'journal':
        title = 'Journal';
        html = this.journal();
        break;
      case 'blueprints':
        title = 'Blueprints';
        html = this.blueprints();
        break;
      case 'probe':
        title = 'Data probe';
        html = this.probe();
        break;
      case 'wreck': {
        const r = this.wreck();
        if (!r) return this.close();
        [title, html] = r;
        break;
      }
      case 'inspect': {
        const r = this.inspect();
        if (!r) return this.close();
        [title, html] = r;
        break;
      }
      case 'info':
        title = this.infoTitle;
        html = this.infoHtml;
        break;
    }
    if (!force && html === this.lastHtml) return;
    this.lastHtml = html;
    $('sheet-title').textContent = title;
    const body = $('sheet-body');
    const scroll = body.scrollTop;
    body.innerHTML = html;
    body.scrollTop = scroll;
  }

  private stack(item: number, n: number | string, size = 20): string {
    return `<span class="stack" title="${this.content.items[item]?.name ?? ''}">${this.icons.img(item, size)}${n}</span>`;
  }

  /** "inputs → output" for a recipe. */
  recipeLine(r: RecipeDef, b?: { class: string; points: number; meter: number }): string {
    const ins = r.inputs.map(([it, n]) => this.stack(it, n)).join('');
    const out =
      b?.class === 'terraformer'
        ? `<span class="stack" style="color:${METER_COLORS[b.meter]}">+${b.points} ${this.content.meters[b.meter].name}</span>`
        : this.stack(r.output[0], `×${r.output[1]}`);
    return `${ins}<span class="arrow">→</span>${out}<small>${(r.ticks / 60).toFixed(1)}s</small>`;
  }

  // ---- Research --------------------------------------------------------------------------

  private techCost(t: number): Stack[] {
    return this.content.tech[t].cost.map(([item], k) => [item, this.engine.x.fx_tech_cost(t, k)] as Stack);
  }

  private research(): string {
    const { engine, content, icons } = this;
    const storage = engine.storage;
    const s = engine.stats;
    const stage = s[Stat.Stage];
    const phase = s[Stat.ArkPhase];
    const tiers: string[] = [];
    for (let tier = 1; tier <= 5; tier++) {
      const open: string[] = [];
      const locked: string[] = [];
      const done: string[] = [];
      content.tech.forEach((t, i) => {
        if (t.tier !== tier || t.repeat) return;
        const state = engine.x.fx_tech_state(i);
        const unlocks =
          t.unlocks.map((k) => icons.img(buildingIcon(k), 28, content.buildings[k].name)).join('') +
          t.recipes.map(([b, r]) => icons.img(content.buildings[b].recipes[r].output[0], 26, recipeName(content, content.buildings[b].recipes[r]))).join('');
        if (state === 2) {
          done.push(`<div class="tech done">${unlocks}<b>${t.name}</b></div>`);
          return;
        }
        const cost = this.techCost(i);
        const reqs = t.requires.filter((r) => engine.x.fx_tech_state(r) !== 2).map((r) => content.tech[r].name);
        const why = [
          reqs.length ? `Needs ${reqs.join(', ')}` : '',
          t.stage > stage ? `Stage: ${content.stages[t.stage].name}` : '',
          t.tier > phase + 1 ? `Ark: ${content.ark[t.tier - 2].name}` : '',
        ]
          .filter(Boolean)
          .join(' · ');
        const afford = canAfford(storage, cost);
        const card = `<div class="tech ${state === 1 ? (afford ? 'ready' : 'short') : 'locked'}">
          <div class="tech-head">${unlocks}<div><b>${t.name}</b><small>${t.desc}</small></div></div>
          <div class="tech-foot"><div class="chips">${costChips(icons, storage, cost, content)}</div>
          ${state === 1 ? `<button class="btn${afford ? ' primary' : ''}" data-act="research:${i}" ${afford ? '' : 'disabled'}>Research</button>` : `<small class="why">${why}</small>`}</div></div>`;
        (state === 1 ? open : locked).push(card);
      });
      const total = open.length + locked.length + done.length;
      if (!total) continue;
      const tierLocked = tier > phase + 1;
      const head = `<h3 class="tier${tierLocked ? ' locked' : ''}">Tier ${tier} · ${TIER_NAMES[tier - 1]} <small>${done.length}/${total}</small>${tierLocked ? `<span class="lock">Complete the Ark's ${content.ark[tier - 2].name}</span>` : ''}</h3>`;
      if (tierLocked && tier > phase + 2) {
        tiers.push(head);
        continue;
      }
      tiers.push(
        head +
          (open.length ? `<div class="tech-list">${open.join('')}</div>` : '') +
          (locked.length ? `<div class="tech-list">${locked.join('')}</div>` : '') +
          (done.length ? `<div class="done-list">${done.join('')}</div>` : ''),
      );
    }
    // Repeatable research, once reachable.
    const reps: string[] = [];
    content.tech.forEach((t, i) => {
      if (!t.repeat) return;
      const state = engine.x.fx_tech_state(i);
      const level = engine.x.fx_tech_level(i);
      if (state === 0 && level === 0) return;
      const cost = this.techCost(i);
      const afford = canAfford(storage, cost);
      reps.push(`<div class="tech ${state === 1 ? (afford ? 'ready' : 'short') : 'locked'}">
        <div class="tech-head"><div class="lvl">${level}</div><div><b>${t.name}</b><small>${t.desc}</small></div></div>
        <div class="tech-foot"><div class="chips">${costChips(icons, storage, cost, content)}</div>
        <button class="btn${afford ? ' primary' : ''}" data-act="research:${i}" ${state === 1 && afford ? '' : 'disabled'}>Level ${level + 1}</button></div></div>`);
    });
    if (reps.length) tiers.push(`<h3>Continuous</h3><div class="tech-list">${reps.join('')}</div>`);
    return tiers.join('');
  }

  // ---- The Ark ----------------------------------------------------------------------------

  private ark(): string {
    const { engine, content, icons } = this;
    const s = engine.stats;
    const storage = engine.storage;
    const phase = s[Stat.ArkPhase];
    const steps = content.ark
      .map((a, i) => `<li class="${i < phase ? 'reached' : i === phase ? 'current' : ''}"><div><b>${i + 1}. ${a.name}</b><small>${a.desc}</small></div></li>`)
      .join('');
    let current = '';
    if (phase < content.ark.length) {
      const a = content.ark[phase];
      let any = false;
      const rows = a.cost
        .map(([item, n], k) => {
          const paid = s[Stat.ArkPaid + k];
          const pct = Math.min(100, (paid / n) * 100);
          const have = storage[item];
          if (paid < n && have > 0) any = true;
          return `<div class="ark-row">${icons.img(item, 28)}<div class="ark-main"><div class="ark-head"><b>${content.items[item].name}</b><span>${formatCount(paid)} / ${formatCount(n)}</span></div><div class="bar"><i style="width:${pct}%"></i></div><small>${formatCount(have)} in the Core</small></div></div>`;
        })
        .join('');
      current = `<div class="ark-card"><div class="ark-title"><span class="label">Phase ${phase + 1} of 5</span><b>${a.name}</b><small>${a.desc}</small></div>${rows}
        <div class="row"><button class="btn primary" data-act="ark" ${any ? '' : 'disabled'}>Send to the Ark</button></div>
        <p class="hint">Deliver parts to the Core, then send them here. Partial loads count.</p></div>`;
    } else {
      const legacy = s[Stat.Legacy];
      const planets = content.planets
        .map((p, i) => {
          const ok = engine.x.fx_can_launch(i) === 1;
          return `<div class="planet-card${ok ? '' : ' locked'}"><b>${p.name}</b><small>${p.desc}</small>${ok ? `<button class="btn primary" data-act="launch:${i}">Launch to ${p.name}</button>` : `<small class="why">After ${p.unlock} launch${p.unlock === 1 ? '' : 'es'}</small>`}</div>`;
        })
        .join('');
      current = `<div class="ark-card done"><div class="ark-title"><span class="label">Complete</span><b>The Ark is ready</b>
        <small>Launch it to settle a new world. You keep your credits, cosmetics and achievements, and every launch makes all production ${content.legacyPct}% faster (now +${legacy * content.legacyPct}%). Or stay and keep building here.</small></div>${planets}</div>`;
    }
    return `${current}<h3>Phases</h3><ol class="stages">${steps}</ol>`;
  }

  // ---- Planet -------------------------------------------------------------------------------

  private planet(): string {
    const { engine, content } = this;
    const s = engine.stats;
    const stage = s[Stat.Stage];
    const ti = engine.stat64(Stat.TiLo);
    const meters = content.meters
      .map((m, i) => {
        const v = engine.stat64(Stat.Meters + i * 2);
        const rate = s[Stat.MeterRates + i] / 100;
        const pct = Math.min(100, (Math.log10(1 + v) / Math.log10(1 + m.full)) * 100);
        return `<div class="meter"><div class="meter-head"><b style="color:${METER_COLORS[i]}">${m.name}</b><span>${formatCount(v)}${rate > 0 ? ` <small>+${formatCount(rate)}/s</small>` : ''}</span></div><div class="bar"><i style="width:${pct}%;background:${METER_COLORS[i]}"></i></div></div>`;
      })
      .join('');
    const stages = content.stages
      .map((st, i) => {
        const unlocks = content.tech.filter((t) => t.stage === i && i > 0).map((t) => t.name);
        const cls = i < stage ? 'reached' : i === stage ? 'current' : '';
        const reward = [st.credits ? `${formatCount(st.credits)} cr` : '', st.shards ? `${st.shards} shard${st.shards > 1 ? 's' : ''}` : '']
          .filter(Boolean)
          .join(' + ');
        return `<li class="${cls}"><div><b>${st.name}</b><small>${st.desc}${unlocks.length ? ` Unlocks research: ${unlocks.join(', ')}.` : ''}</small></div><span>${i === 0 ? '' : formatCount(st.ti) + ' Ti'}${reward ? `<em>${reward}</em>` : ''}</span></li>`;
      })
      .join('');
    const planet = content.planets[s[Stat.Planet]];
    const day = s[Stat.Daylight] / 10;
    return `<div class="ti-big"><b>${formatCount(ti)}</b> Ti <small>Terraform Index · ${planet?.name ?? ''} · daylight ${Math.round(day)}%</small></div>
      <p class="hint">Terraformers burn goods to raise these meters. Their sum is the Terraform Index; every stage changes the planet and unlocks new research.</p>
      ${meters}<h3>Stages</h3><ol class="stages">${stages}</ol>`;
  }

  // ---- Core -----------------------------------------------------------------------------------

  private core(): string {
    const tab = (id: string, label: string) =>
      `<button class="${this.coreTab === id ? 'on' : ''}" data-act="tab:${id}">${label}</button>`;
    const head = `<div class="seg tabs">${tab('storage', 'Storage')}${tab('production', 'Production')}${tab('power', 'Power')}</div>`;
    const body = this.coreTab === 'production' ? this.production() : this.coreTab === 'power' ? this.power() : this.storage();
    return head + body;
  }

  private storage(): string {
    const { engine, content } = this;
    const storage = engine.storage;
    const s = engine.stats;
    const cells = content.items
      .map((it, i) => (i > 0 && storage[i] > 0 ? `<div class="cell">${this.icons.img(i, 36)}<b>${formatCount(storage[i])}</b><small>${it.name}</small></div>` : ''))
      .join('');
    return `<p class="hint">Everything belted into the Core lands here. It pays for buildings, research and the Ark.
      Delivering <b>${formatCount(s[Stat.Rate])}</b> items/min · <b>${formatCount(engine.credits)}</b> credits.</p><div class="storage">${cells || '<p>Empty.</p>'}</div>`;
  }

  private production(): string {
    const { engine, content, icons } = this;
    const r = engine.rates;
    const n = engine.itemCount;
    const rows: string[] = [];
    for (let i = 1; i < n; i++) {
      const made = r[i];
      const used = r[n + i];
      const got = r[2 * n + i];
      if (!made && !used && !got) continue;
      const net = made - used;
      rows.push(`<tr><td>${icons.img(i, 20)} ${content.items[i].name}</td><td>${formatCount(made)}</td><td>${formatCount(used)}</td><td class="${net < 0 ? 'neg' : net > 0 ? 'pos' : ''}">${net > 0 ? '+' : ''}${formatCount(Math.abs(net)) === '0' ? '0' : (net < 0 ? '−' : '') + formatCount(Math.abs(net))}</td><td>${formatCount(got)}</td></tr>`);
    }
    return `<p class="hint">Per minute, over the last minute. A negative balance means something upstream can't keep up: turn on the problem view (eye button) to find stuck machines.</p>
      ${rows.length ? `<table class="stats"><thead><tr><th>Item</th><th>Made</th><th>Used</th><th>Balance</th><th>To Core</th></tr></thead><tbody>${rows.join('')}</tbody></table>` : '<p>Nothing produced yet.</p>'}`;
  }

  private power(): string {
    const { engine } = this;
    const s = engine.stats;
    const sat = s[Stat.PowerSat] / 10;
    const nets: string[] = [];
    for (let k = 0; k < s[Stat.Nets]; k++) {
      const info = engine.netInfo(k);
      if (!info) continue;
      const [demand, supply, netSat, stored, cap, flow, batteries, core] = info;
      if (!demand && !supply && !cap && !core) continue;
      nets.push(`<tr><td>${core ? 'Core grid' : `Grid ${k + 1}`}</td><td>${formatPower(demand)}</td><td>${formatPower(supply)}</td><td class="${netSat < 1000 ? 'neg' : ''}">${Math.round(netSat / 10)}%</td><td>${batteries ? `${formatCount(stored)}/${formatCount(cap)} MJ ${flow > 0 ? '▲' : flow < 0 ? '▼' : ''}` : '—'}</td></tr>`);
    }
    return `<div class="ti-big"><b>${formatPower(s[Stat.PowerDemand])}</b> of ${formatPower(s[Stat.PowerSupply])} <small>${sat < 100 ? `Short of power: machines run at ${Math.round(sat)}%` : 'All machines have full power'}</small></div>
      <p class="hint">The Core powers everything within ${this.content.coreRadius} tiles. Power Poles carry it further (and link up within ${this.content.poleLink} tiles); generators add more. When a grid runs short, its machines slow down; nothing breaks.</p>
      ${nets.length ? `<table class="stats"><thead><tr><th>Grid</th><th>Use</th><th>Supply</th><th>Met</th><th>Battery</th></tr></thead><tbody>${nets.join('')}</tbody></table>` : ''}`;
  }

  // ---- Menu, shop, achievements, journal, blueprints -----------------------------------------------

  private menu(): string {
    const m = this.actions.menuState();
    const s = this.engine.stats;
    const btn = (act: string, label: string, extra = '') => `<button class="btn" data-act="${act}" ${extra}>${label}</button>`;
    const seg = (name: string, current: string | number | boolean, options: [string | number | boolean, string][]) =>
      `<div class="seg" role="radiogroup">${options
        .map(([v, label]) => `<button data-act="${name}:${v}" role="radio" aria-checked="${v === current}" class="${v === current ? 'on' : ''}">${label}</button>`)
        .join('')}</div>`;
    const hz = m.displayHz;
    const fpsHint =
      m.fps > hz
        ? `Your screen is running at ${hz} Hz, so the game can't go above ${hz} FPS here. On iPhone, Safari limits web pages to 60 FPS.`
        : `Screen: ${hz} Hz. Lower caps save battery.`;
    const achieved = this.content.achievements.filter((_, i) => this.engine.hasAchievement(i)).length;
    const probes = s[Stat.Probes];
    return `<div class="menu">
      <h3>Progress</h3>
      <div class="grid2">
        ${btn('open:achievements', `Achievements <small>${achieved}/${this.content.achievements.length}</small>`)}
        ${btn('open:journal', `Journal <small>${s[Stat.Logs]}/${this.content.logs.length}</small>`)}
        ${btn('open:shop', `Shop <small>${formatCount(this.engine.credits)} cr</small>`)}
        ${btn('open:blueprints', 'Blueprints')}
        ${probes ? btn('open:probe', `Analyse probe <small class="hot">${probes}</small>`) : ''}
      </div>
      <h3>Game</h3>
      <div class="row">${btn('save', 'Save now')}${btn('export', 'Export save')}${btn('import', 'Import save')}</div>
      <p class="hint">Autosaves every 30 seconds and when you leave. ${m.lastSave}</p>
      <div class="row">${btn('help', 'How to play')}${btn('new', 'New planet', 'data-danger')}</div>
      <h3>Graphics</h3>
      <div class="field"><span>Frame rate</span>${seg('fps', m.fps, [[30, '30'], [60, '60'], [120, '120'], [0, 'Max']])}</div>
      <p class="hint">${fpsHint}</p>
      <div class="field"><span>Quality</span>${seg('quality', m.quality, [['auto', 'Auto'], ['high', 'High'], ['medium', 'Medium'], ['low', 'Low']])}</div>
      <div class="field"><span>Night</span>${seg('night', m.night, [[true, 'Dark'], [false, 'Off']])}</div>
      <p class="hint">Auto lowers the resolution by itself if frames run late. Low renders at 1× and skips clouds, weather and night lights. Night only changes the look: solar panels follow the sun either way.</p>
      <h3>Display</h3>
      <div class="row">
        ${m.fullscreenSupported ? btn('fullscreen', m.fullscreen ? 'Exit fullscreen' : 'Fullscreen') : ''}
        ${btn('sound', m.sound ? 'Sound: on' : 'Sound: off')}
        ${btn('perf', m.perf ? 'Hide stats' : 'Show stats')}
      </div>
      ${m.canInstall ? `<div class="row">${btn('install', 'Install app (offline)')}</div>` : ''}
      ${m.iosInstall && !m.standalone ? '<p class="hint">To install on iPhone: tap <b>Share</b> then <b>Add to Home Screen</b>. It then runs fullscreen and offline.</p>' : ''}
      ${m.standalone ? '<p class="hint">Installed: runs offline.</p>' : ''}
      <h3>Performance</h3>
      <div class="row">${btn('bench', 'Run benchmark')}</div>
      <p class="hint">Builds a 30k-belt, 4k-machine test factory in a separate world, flies the camera through it and reports frame times. Your game is restored afterwards.</p>
      <p class="about">Beltwise ${m.version} · Rust + WebAssembly + WebGL2</p>
    </div>`;
  }

  private shop(): string {
    const { engine, content, icons } = this;
    const s = engine.stats;
    const credits = engine.credits;
    const owned = s[Stat.Cosmetics];
    const cards = content.shop
      .map((d, i) => {
        if (d.kind === 'item') {
          const ok = credits >= d.price;
          return `<div class="shop-card">${icons.img(d.a, 36)}<div><b>${d.name}</b><small>${d.desc}</small></div><button class="btn${ok ? ' primary' : ''}" data-act="buy:${i}" ${ok ? '' : 'disabled'}>${formatCount(d.price)} cr</button></div>`;
        }
        const has = (owned & (1 << d.a)) !== 0;
        const wearing = d.a < 8 ? s[Stat.BeltColor] === d.a : s[Stat.Trim] === d.a - 8;
        const swatch = `<i class="swatch c${d.a}"></i>`;
        const action = wearing
          ? '<span class="chip free">In use</span>'
          : has
            ? `<button class="btn" data-act="wear:${d.a}">Use</button>`
            : `<button class="btn${credits >= d.price ? ' primary' : ''}" data-act="buy:${i}" ${credits >= d.price ? '' : 'disabled'}>${formatCount(d.price)} cr</button>`;
        return `<div class="shop-card">${swatch}<div><b>${d.name}</b><small>${d.desc}</small></div>${action}</div>`;
      })
      .join('');
    const defaults = `<div class="row">${s[Stat.BeltColor] !== 0 ? '<button class="btn" data-act="wear:0">Default belts</button>' : ''}${s[Stat.Trim] !== 0 ? '<button class="btn" data-act="wear:8">Gold trim</button>' : ''}</div>`;
    return `<div class="ti-big"><b>${formatCount(credits)}</b> credits <small>Earned from Recyclers and achievements</small></div>${cards}${defaults}`;
  }

  private achievements(): string {
    const { engine, content } = this;
    const items = content.achievements
      .map((a, i) => {
        const done = engine.hasAchievement(i);
        return `<div class="ach${done ? ' done' : ''}"><div><b>${done ? '★ ' : ''}${a.name}</b><small>${a.desc}</small></div><span>+${formatCount(a.credits)} cr${a.shards ? ` · ${a.shards} shard${a.shards > 1 ? 's' : ''}` : ''}</span></div>`;
      })
      .join('');
    return `<div class="ach-list">${items}</div>`;
  }

  private journal(): string {
    const n = this.engine.stats[Stat.Logs];
    const logs = this.content.logs
      .slice(0, n)
      .map((l, i) => `<div class="log"><b>${i + 1}. ${l.title}</b><p>${l.text}</p></div>`)
      .join('');
    return `<p class="hint">Logs from the first expedition, recovered from wrecks. ${n}/${this.content.logs.length} found.</p>${logs || '<p>No logs yet. Explore: wrecks lie scattered across the map.</p>'}`;
  }

  private blueprints(): string {
    const list = this.actions.blueprints();
    const rows = list
      .map(
        (b) =>
          `<div class="bp"><div><b>${b.name}</b><small>${b.cells} buildings</small></div><div class="row"><button class="btn primary" data-act="bp:use" data-id="${b.id}">Paste</button><button class="btn" data-act="bp:share" data-id="${b.id}">Share</button><button class="btn" data-act="bp:rename" data-id="${b.id}">Rename</button><button class="btn" data-act="bp:delete" data-id="${b.id}" data-danger>Delete</button></div></div>`,
      )
      .join('');
    return `<p class="hint">Use <b>Copy</b> in the build bar and drag over part of your factory. Saved blueprints stay on this device; <b>Share</b> copies a code anyone can import.</p>
      <div class="row"><button class="btn" data-act="bp:import">Import code</button></div>${rows || '<p>No saved blueprints yet.</p>'}`;
  }

  private probe(): string {
    const { engine, content } = this;
    const n = engine.stats[Stat.Probes];
    if (!n) return '<p>No data probes to analyse. They turn up in wrecks.</p>';
    const packed = engine.x.fx_probe_options();
    const opts = [packed & 255, (packed >> 8) & 255, (packed >> 16) & 255].filter((k) => k !== 255);
    const cards = opts
      .map((k) => {
        const [b, r] = content.alternates[k];
        const def = content.buildings[b];
        const rc = def.recipes[r];
        return `<div class="alt-card"><div class="alt-head">${this.icons.img(buildingIcon(b), 30)}<div><b>${rc.name}</b><small>${def.name}</small></div></div><div class="recipe">${this.recipeLine(rc)}</div><button class="btn primary" data-act="alt:${k}">Unlock</button></div>`;
      })
      .join('');
    return `<p class="hint">The probe holds designs from the first expedition. Pick one alternate recipe to unlock (${n} probe${n > 1 ? 's' : ''} left). Choose it on a machine in the inspector.</p>${cards}`;
  }

  // ---- Wrecks and the inspector ---------------------------------------------------------------

  private wreck(): [string, string] | null {
    const [x, y] = this.inspectAt;
    const t = this.engine.x.fx_tile(x, y);
    if ((t & 255) !== 37) return null;
    return [
      'Wreck',
      `<div class="inspect"><div class="inspect-head">${this.icons.img(buildingIcon(37), 44)}<div><b>Crashed pod</b><small>From the first expedition. Something may still be inside: supplies, a data probe, power shards, or a log.</small></div></div>
      <div class="row"><button class="btn primary" data-act="salvage">Salvage</button></div></div>`,
    ];
  }

  /** Describes what a salvaged wreck held (from the engine report). */
  salvageHtml(report: Uint32Array): [string, string] {
    const { content } = this;
    const kind = report[0];
    if (kind === Found.Log) {
      const log = content.logs[report[1]];
      return [`Log ${report[1] + 1}: ${log.title}`, `<div class="log big"><p>${log.text}</p></div><p class="hint">Saved to your journal (menu).</p>`];
    }
    if (kind === Found.Probe) {
      return ['Data probe', `<p>A data probe with designs from the first expedition.</p><div class="row"><button class="btn primary" data-act="open:probe">Analyse it</button></div>`];
    }
    if (kind === Found.Credits) return ['Data probe', `<p>You already know every design on it: sold for <b>${report[1]}</b> credits.</p>`];
    const items: string[] = [];
    for (let k = 1; k + 1 < report.length; k += 2) items.push(this.stack(report[k], `+${report[k + 1]}`, 28));
    const title = kind === Found.Shards ? 'Power shards' : kind === Found.Amplifier ? 'An amplifier!' : 'Supply cache';
    const hint =
      kind === Found.Shards || kind === Found.Amplifier
        ? '<p class="hint">Install it on a machine from the inspector: shards overclock, amplifiers double output.</p>'
        : '';
    return [title, `<div class="loot">${items.join('')}</div><p class="hint">Added to the Core.</p>${hint}`];
  }

  private inspect(): [string, string] | null {
    const { engine, content, icons } = this;
    const info = engine.inspect(...this.inspectAt);
    if (!info) return null;
    const kind = info[Info.Kind];
    const b = content.buildings[kind];
    const recipeIndex = info[Info.Recipe];
    const status = info[Info.Status];
    const storage = engine.storage;
    const parts: string[] = [];
    if (b.class === 'crafter' || b.class === 'terraformer') {
      const rc = recipeIndex !== NO_RECIPE ? b.recipes[recipeIndex] : null;
      if (rc) {
        const ins = rc.inputs.map(([it, n], k) => this.stack(it, `${info[Info.Inv + k]}/${n}`)).join('');
        const out =
          b.class === 'terraformer'
            ? `<span class="stack" style="color:${METER_COLORS[b.meter]}">+${b.points} ${content.meters[b.meter].name}</span>`
            : this.stack(rc.output[0], `×${rc.output[1] * ((info[Info.Flags] & MachineFlag.Amp) ? 2 : 1)}`);
        parts.push(`<div class="recipe">${ins}<span class="arrow">→</span>${out}<small>${(rc.ticks / 60 / (info[Info.SpeedPct] / 100)).toFixed(1)}s</small></div>`);
      } else {
        parts.push('<p class="hint">Waiting for its first input (Auto), or pick a recipe below.</p>');
      }
      if (b.class === 'crafter' && b.recipes.length > 1) {
        const locked = (info[Info.Flags] & MachineFlag.Locked) !== 0;
        const opts = [`<button class="opt${!locked ? ' on' : ''}" data-act="i:recipe:${NO_RECIPE}">Auto</button>`];
        b.recipes.forEach((r, k) => {
          if (!recipeOpen(engine, kind, k)) return;
          const on = locked && recipeIndex === k;
          opts.push(`<button class="opt${on ? ' on' : ''}${r.unlock >= ALT_BASE ? ' alt' : ''}" data-act="i:recipe:${k}" title="${recipeName(content, r)}">${icons.img(r.output[0], 18)}<span>${recipeName(content, r)}</span></button>`);
        });
        parts.push(`<div class="opts">${opts.join('')}</div>`);
      }
    } else if (b.class === 'drill') {
      const purity = content.purity[info[Info.Purity]] ?? '';
      parts.push(`<div class="recipe"><span class="arrow">Mining</span>${this.stack(info[Info.Aux], '')}<small>${purity} deposit · ${formatCount((60 * info[Info.SpeedPct]) / 100)}/min</small></div>`);
    } else if (b.class === 'pump') {
      parts.push(`<div class="recipe"><span class="arrow">Pumping</span>${this.stack(20, '')}<small>${formatCount((120 * info[Info.SpeedPct]) / 100)}/min</small></div>`);
    } else if (b.class === 'tunnel') {
      parts.push(`<p class="hint">${info[Info.Aux] === NONE ? 'Not connected: place another tunnel facing the same way up to 5 tiles ahead.' : info[Info.B] === 0 ? `Entrance · ${info[Info.A]} items underground` : 'Exit'}</p>`);
    } else if (b.class === 'sorter') {
      const f = info[Info.Filter];
      const opts = [`<button class="opt${f === 0 ? ' on' : ''}" data-act="i:filter:0">Any</button>`];
      for (const it of knownItems(engine)) {
        opts.push(`<button class="opt${f === it ? ' on' : ''}" data-act="i:filter:${it}" title="${content.items[it].name}">${icons.img(it, 20)}</button>`);
      }
      parts.push(`<p class="hint">${f ? `${content.items[f].name} goes straight on; everything else goes left and right.` : 'No filter: everything goes straight on.'}</p><div class="opts">${opts.join('')}</div>`);
    } else if (b.class === 'storage') {
      const n = info[Info.Aux] === NONE ? 0 : info[Info.Aux];
      const held = info[Info.Held];
      parts.push(`<div class="recipe">${held ? this.stack(held, formatCount(n)) : '<span class="arrow">Empty</span>'}<small>${formatCount(n)} / ${content.storageCap}</small></div><div class="bar"><i style="width:${(100 * n) / content.storageCap}%"></i></div>`);
    } else if (b.class === 'generator') {
      const fuel = b.fuel[0];
      parts.push(
        fuel
          ? `<div class="recipe">${this.stack(fuel, `${info[Info.Inv]} queued`)}<small>${formatPower(info[Info.Draw])} while burning · current fuel ${info[Info.Extra]}%</small></div>`
          : `<div class="recipe"><span class="arrow">Sunlight</span><small>${formatPower((info[Info.Draw] * engine.stats[Stat.Daylight]) / 1000)} now of ${formatPower(info[Info.Draw])}</small></div>`,
      );
    } else if (b.class === 'drone') {
      const linked = info[Info.Aux] !== NONE;
      const tx = info[Info.Aux] % engine.width;
      const ty = Math.floor(info[Info.Aux] / engine.width);
      const flight = ['Parked', 'Flying out', 'Unloading', 'Flying home'][info[Info.Extra]] ?? '';
      parts.push(`<p class="hint">${linked ? `Sends to the port at ${tx}, ${ty}. ${flight}.` : 'Not linked. Link it to another Drone Port; everything fed in gets flown there.'} Waiting to send: ${info[Info.A]} · received: ${info[Info.B]}.</p>
        <div class="row"><button class="btn primary" data-act="i:link">${linked ? 'Change target' : 'Link to a port'}</button>${linked ? '<button class="btn" data-act="i:unlink">Unlink</button>' : ''}</div>`);
    } else if (b.class === 'radar') {
      const r = info[Info.Aux];
      parts.push(`<p class="hint">${r >= 70 ? 'Scan complete: 70 tiles.' : `Scanning: ${r} of 70 tiles.`}</p>`);
    } else if (b.class === 'battery') {
      const net = info[Info.Net];
      const n = net !== NONE ? engine.netInfo(net) : null;
      parts.push(`<p class="hint">${n ? `Grid batteries: ${formatCount(n[3])} / ${formatCount(n[4])} MJ.` : 'Not on a grid: place it near a Power Pole.'}</p>`);
    }
    // Power.
    if (b.power > 0 && b.class !== 'generator' && b.class !== 'battery') {
      const net = info[Info.Net];
      const sat = info[Info.Sat] / 10;
      parts.push(
        net === NONE
          ? `<p class="warn">${SVG.bolt} No power here. Build a Power Pole within ${content.poleRadius} tiles, linked to your grid.</p>`
          : `<p class="hint power-line">${SVG.bolt} ${formatPower(info[Info.Draw])} while working · grid ${Math.round(sat)}%${sat < 100 ? ' (short: add generators)' : ''}</p>`,
      );
    } else if (b.class === 'generator' && info[Info.Net] === NONE) {
      parts.push(`<p class="warn">${SVG.bolt} Not connected: place it within ${content.poleRadius} tiles of a Power Pole or the Core.</p>`);
    }
    // Overclocking and amplifiers.
    if (['drill', 'pump', 'crafter', 'terraformer', 'generator', 'recycler'].includes(b.class)) {
      const shards = info[Info.Shards];
      const have = storage[36];
      const amp = (info[Info.Flags] & MachineFlag.Amp) !== 0;
      const ampOk = ['drill', 'pump', 'crafter', 'terraformer'].includes(b.class) && (amp || storage[37] > 0);
      if (shards > 0 || have > 0 || ampOk) {
        parts.push(`<div class="clock"><span>${icons.img(36, 20)} Overclock</span><button class="btn small" data-act="i:shards:${shards - 1}" ${shards > 0 ? '' : 'disabled'}>−</button><b>${content.clockPct[shards]}%</b><button class="btn small" data-act="i:shards:${shards + 1}" ${shards < 3 && have > 0 ? '' : 'disabled'}>+</button><small>${have} shard${have === 1 ? '' : 's'} in the Core</small></div>`);
        if (ampOk) {
          parts.push(`<div class="clock"><span>${icons.img(37, 20)} Amplifier</span><button class="btn small${amp ? ' primary' : ''}" data-act="i:amp:${amp ? 0 : 1}">${amp ? 'Installed: remove' : 'Install'}</button><small>×2 output, ×4 power</small></div>`);
        }
      }
    }
    const pct = Math.round((info[Info.Progress] / 255) * 100);
    const html = `<div class="inspect">
      <div class="inspect-head">${icons.img(buildingIcon(kind), 44)}<div><b>${b.name}</b><small class="status s${status}">${STATUS[status] ?? ''}</small></div></div>
      ${parts.join('')}
      ${info[Info.Progress] ? `<div class="bar"><i style="width:${pct}%"></i></div>` : ''}
      <div class="row"><button class="btn" data-act="i:rotate">Rotate</button><button class="btn" data-act="i:remove" data-danger>Remove</button></div>
    </div>`;
    return [b.name, html];
  }
}

/** Short first-run guide, also reachable from the menu. */
export const HELP_HTML = `<div class="help">
  <p><b>Welcome to Beltwise.</b> You landed on a frozen world with a Core. Build a factory, then use it to bring the planet to life and launch the Ark.</p>
  <ol>
    <li><b>Extract:</b> pick <i>Drill</i> (Extraction tab) and tap an ore deposit. It outputs in the arrow's direction; <i>Rotate</i> turns the next building.</li>
    <li><b>Belt:</b> drag with <i>Belt</i> (Logistics tab) from the drill to a <i>Smelter</i>, then on to the <b>Core</b>. Corners are automatic.</li>
    <li><b>Spend:</b> the Core stores everything. Materials pay for new buildings and <b>Research</b> (flask button).</li>
    <li><b>Power:</b> the Core powers machines near it. Further out, <i>Power Poles</i> carry it; generators add more.</li>
    <li><b>Explore:</b> building pushes back the fog. Tap wrecks to salvage supplies, stories and new designs.</li>
    <li><b>Terraform and launch:</b> Heaters, Vaporizers, Greenhouses and more raise the Terraform Index; the <b>Ark</b> (rocket button) unlocks new tiers and, finally, a new planet.</li>
  </ol>
  <p>Two fingers pan and zoom. Tap a building with <i>Move</i> to inspect it. The eye button shows power coverage and stuck machines. Removing a building refunds it; <i>Undo</i> reverts your last gesture.</p>
</div>`;

