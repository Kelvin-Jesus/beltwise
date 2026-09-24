// Always-visible HUD: objective and planet cards, action buttons, build bar, selection card
// and toasts. Updated at a few Hz and only where values changed, so it adds no per-frame
// DOM work.

import { GoalKind, NO_RECIPE, Stat, TOOL_COPY, TOOL_DELETE, TOOL_MOVE, TOOL_PASTE } from './constants';
import {
  ALT_BASE,
  CATEGORIES,
  FREE,
  HIDDEN_CATEGORY,
  formatCount,
  formatPower,
  recipeName,
  type Content,
  type Stack,
} from './content';
import type { Engine } from './engine';
import { buildingIcon, type Icons } from './icons';

export type Panel = 'research' | 'planet' | 'core' | 'menu' | 'ark';

export interface HudHandlers {
  onTool(tool: number): void;
  onRotate(): void;
  onUndo(): void;
  onOpen(panel: Panel): void;
  onFullscreen(): void;
  onOverlay(): void;
  /** Default recipe (255 = Auto) or sorter filter for the selected building. */
  onPlaceOption(recipe: number, filter: number): void;
  /** Blueprint actions from the selection card. */
  onBlueprint(action: 'save' | 'cancel'): void;
}

const svg = (d: string, extra = '') => `<svg viewBox="0 0 24 24" ${extra}>${d}</svg>`;
const line = (d: string, w = 2.4) =>
  svg(`<path d="${d}" stroke="currentColor" stroke-width="${w}" fill="none" stroke-linecap="round" stroke-linejoin="round"/>`);

export const SVG = {
  move: svg('<path d="M12 2l3 3h-2v6h6V9l3 3-3 3v-2h-6v6h2l-3 3-3-3h2v-6H5v2l-3-3 3-3v2h6V5H9z"/>'),
  delete: line('M6 6l12 12M18 6L6 18', 2.6),
  copy: line('M8 8h11v11H8zM5 16V5h11'),
  research: svg('<path d="M9 2h6v2h-1v5.3l5.2 8.7A2 2 0 0 1 17.5 21h-11a2 2 0 0 1-1.7-3L10 9.3V4H9zm3 9l-2.4 4h4.8z"/>'),
  core: svg('<path d="M12 2l9 5v10l-9 5-9-5V7zm0 2.3L5.5 8 12 11.6 18.5 8zM5 9.7v6.1l6 3.4v-6.1zm14 0l-6 3.4v6.1l6-3.4z"/>'),
  ark: svg('<path d="M12 2c3 2.5 4.5 6 4.5 10.5L18 16l-2.5 1.2-1-2.2h-5l-1 2.2L6 16l1.5-3.5C7.5 8 9 4.5 12 2zm0 5a1.8 1.8 0 1 0 0 3.6A1.8 1.8 0 0 0 12 7zm-2 11h4l-2 4z"/>'),
  menu: line('M4 6h16M4 12h16M4 18h16'),
  fullscreen: line('M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5'),
  undo: line('M9 5L4 10l5 5M4 10h10a6 6 0 0 1 0 12h-3'),
  eye: line('M2 12s3.6-7 10-7 10 7 10 7-3.6 7-10 7S2 12 2 12zM12 9a3 3 0 1 0 0 6 3 3 0 0 0 0-6z', 2),
  rotate:
    '<svg viewBox="0 0 100 100"><path d="M50 16 V4 L70 20 L50 36 V26 A24 24 0 1 0 74 50 H84 A34 34 0 1 1 50 16Z" fill="currentColor"/><path class="dir" d="M50 50 L78 50 M68 40 L78 50 L68 60" stroke="#fbbf24" stroke-width="7" fill="none" stroke-linecap="round" stroke-linejoin="round"/></svg>',
  planet:
    '<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="7" fill="#4ade80"/><path d="M2 14c4 2 16 0 20-4" stroke="#fbbf24" stroke-width="2" fill="none"/></svg>',
  bolt: svg('<path d="M13 2L4 14h7l-2 8 9-12h-7z"/>'),
  wreck: svg('<path d="M4 15l6-9 9 3-3 9zM2 20h20" stroke="currentColor" stroke-width="2" fill="none" stroke-linejoin="round"/>'),
};

const CATEGORY_ICONS = [
  line('M3 8h14l-3-3M21 16H7l3 3'),
  svg('<path d="M14 3l7 7-3 3-7-7zM11 6l-8 8 3 4 8-8M5 16l-2 5 5-2" stroke="currentColor" stroke-width="2" fill="none" stroke-linejoin="round"/>'),
  svg(
    '<circle cx="12" cy="12" r="3.5" fill="none" stroke="currentColor" stroke-width="2.2"/><path d="M12 2v4M12 18v4M2 12h4M18 12h4M4.9 4.9l2.8 2.8M16.3 16.3l2.8 2.8M4.9 19.1l2.8-2.8M16.3 7.7l2.8-2.8" stroke="currentColor" stroke-width="2.2" stroke-linecap="round"/>',
  ),
  svg('<path d="M13 2L4 14h7l-2 8 9-12h-7z" fill="currentColor"/>'),
  svg('<circle cx="12" cy="12" r="8" fill="none" stroke="currentColor" stroke-width="2.2"/><path d="M4 12c3 2 13 2 16 0M12 4c-3 3-3 13 0 16" stroke="currentColor" stroke-width="1.8" fill="none"/>'),
];

function $(id: string): HTMLElement {
  return document.getElementById(id)!;
}

export function isUnlocked(engine: Engine, research: number): boolean {
  return research === FREE || engine.isResearched(research);
}

export function canAfford(storage: Uint32Array, cost: Stack[]): boolean {
  return cost.every(([item, n]) => storage[item] >= n);
}

/** Cost chips: icon + have/need, red when short. */
export function costChips(icons: Icons, storage: Uint32Array, cost: Stack[], content: Content): string {
  if (!cost.length) return '<span class="chip free">Free</span>';
  return cost
    .map(([item, n]) => {
      const have = storage[item];
      return `<span class="chip${have < n ? ' short' : ''}" title="${content.items[item].name}">${icons.img(item, 18)}${formatCount(Math.min(have, 99999))}/${formatCount(n)}</span>`;
    })
    .join('');
}

/** Whether recipe `r` of building `kind` can be chosen now. */
export function recipeOpen(engine: Engine, kind: number, r: number): boolean {
  const rc = engine.content.buildings[kind].recipes[r];
  if (!rc) return false;
  if (rc.unlock === FREE) return true;
  if (rc.unlock >= ALT_BASE) return (engine.stats[Stat.Alts] & (1 << (rc.unlock - ALT_BASE))) !== 0;
  return engine.isResearched(rc.unlock);
}

/** Items the player has seen (made, stored or delivered): the sorter filter choices. */
export function knownItems(engine: Engine): number[] {
  const out: number[] = [];
  const s = engine.storage;
  const r = engine.rates;
  const n = engine.itemCount;
  for (let i = 1; i < n; i++) {
    if (s[i] > 0 || r[i] > 0 || r[n * 2 + i] > 0) out.push(i);
  }
  return out;
}

export class Hud {
  tool = TOOL_MOVE;
  category = 1;
  showPerf = false;
  overlay = false;
  /** Default recipe per building kind and the sorter filter for placement. */
  readonly placeRecipe = new Map<number, number>();
  placeFilter = 0;
  blueprintInfo = '';
  private readonly last: Record<string, string | number> = {};
  private toastTimer = 0;
  private toolsKey = '';
  private readonly content: Content;

  constructor(
    private readonly engine: Engine,
    private readonly icons: Icons,
    h: HudHandlers,
  ) {
    const content = engine.content;
    this.content = content;
    $('btn-research').insertAdjacentHTML('afterbegin', SVG.research);
    $('btn-core').innerHTML = SVG.core;
    $('btn-ark').insertAdjacentHTML('afterbegin', SVG.ark);
    $('btn-menu').insertAdjacentHTML('afterbegin', SVG.menu);
    $('btn-fullscreen').innerHTML = SVG.fullscreen;
    $('undo').innerHTML = SVG.undo;
    $('rotate').innerHTML = SVG.rotate;
    $('overlay').innerHTML = SVG.eye;
    $('btn-research').addEventListener('click', () => h.onOpen('research'));
    $('btn-core').addEventListener('click', () => h.onOpen('core'));
    $('btn-ark').addEventListener('click', () => h.onOpen('ark'));
    $('btn-menu').addEventListener('click', () => h.onOpen('menu'));
    $('btn-fullscreen').addEventListener('click', () => h.onFullscreen());
    $('planet').addEventListener('click', () => h.onOpen('planet'));
    $('undo').addEventListener('click', () => h.onUndo());
    $('rotate').addEventListener('click', () => h.onRotate());
    $('overlay').addEventListener('click', () => h.onOverlay());
    if (!document.fullscreenEnabled) $('btn-fullscreen').hidden = true;

    const cats = $('cats');
    cats.innerHTML =
      `<button class="cat" data-tool="${TOOL_MOVE}" title="Move / inspect (Q)">${SVG.move}<span>Move</span></button>` +
      `<button class="cat danger" data-tool="${TOOL_DELETE}" title="Delete (X)">${SVG.delete}<span>Delete</span></button>` +
      `<button class="cat" data-tool="${TOOL_COPY}" title="Copy an area as a blueprint (B)">${SVG.copy}<span>Copy</span></button>` +
      '<i class="sep"></i>' +
      CATEGORIES.map(
        (c, i) =>
          `<button class="cat" data-cat="${i}" style="--accent:${c.accent}" title="${c.name}">${CATEGORY_ICONS[i]}<span>${c.name}</span></button>`,
      ).join('');
    cats.addEventListener('click', (e) => {
      const b = (e.target as HTMLElement).closest('button');
      if (!b) return;
      if (b.dataset.tool) h.onTool(Number(b.dataset.tool));
      else if (b.dataset.cat) this.setCategory(Number(b.dataset.cat));
    });
    $('tools').addEventListener('click', (e) => {
      const b = (e.target as HTMLElement).closest('button');
      if (b?.dataset.kind) h.onTool(Number(b.dataset.kind));
    });
    $('selection').addEventListener('click', (e) => {
      const b = (e.target as HTMLElement).closest<HTMLElement>('[data-sel]');
      if (!b) return;
      const [act, v] = b.dataset.sel!.split(':');
      if (act === 'recipe') {
        this.placeRecipe.set(this.tool, Number(v));
        h.onPlaceOption(Number(v), this.placeFilter);
      } else if (act === 'filter') {
        this.placeFilter = Number(v);
        h.onPlaceOption(NO_RECIPE, this.placeFilter);
      } else if (act === 'bp') {
        h.onBlueprint(v as 'save' | 'cancel');
      }
      this.last.selection = '\u0000';
    });
    this.setCategory(1);
  }

  setCategory(cat: number): void {
    this.category = cat;
    for (const b of $('cats').querySelectorAll<HTMLElement>('[data-cat]')) {
      b.classList.toggle('active', Number(b.dataset.cat) === cat);
    }
    this.toolsKey = '';
    this.refreshTools();
  }

  /** Buildings of the current category that research has unlocked. */
  unlockedIn(cat: number): number[] {
    const out: number[] = [];
    this.content.buildings.forEach((b, k) => {
      if (b.category === cat && b.category !== HIDDEN_CATEGORY && isUnlocked(this.engine, b.research)) out.push(k);
    });
    return out;
  }

  refreshTools(): void {
    const kinds = this.unlockedIn(this.category);
    const key = `${this.category}:${kinds.join(',')}:${this.tool}`;
    if (key === this.toolsKey) return;
    this.toolsKey = key;
    const content = this.content;
    const locked = content.buildings.filter(
      (b) => b.category === this.category && !isUnlocked(this.engine, b.research),
    ).length;
    $('tools').innerHTML =
      kinds
        .map((k) => {
          const b = content.buildings[k];
          return `<button class="tool${this.tool === k ? ' active' : ''}" data-kind="${k}" title="${b.name}">${this.icons.img(buildingIcon(k), 34)}<span>${b.name}</span></button>`;
        })
        .join('') + (locked ? `<div class="locked-hint">+${locked} in research</div>` : '');
    for (const b of $('cats').querySelectorAll<HTMLElement>('[data-tool]')) {
      b.classList.toggle('active', Number(b.dataset.tool) === this.tool);
    }
  }

  setTool(tool: number): void {
    this.tool = tool;
    if (tool >= 0) {
      const cat = this.content.buildings[tool]?.category;
      if (cat !== undefined && cat !== this.category && cat !== HIDDEN_CATEGORY) this.setCategory(cat);
    }
    this.toolsKey = '';
    this.refreshTools();
    ($('rotate') as HTMLButtonElement).disabled = tool < 0 && tool !== TOOL_PASTE;
    this.last.selection = '\u0000'; // force a redraw, even to "nothing selected"
  }

  setDirection(dir: number): void {
    const arrow = $('rotate').querySelector<SVGElement>('.dir');
    if (arrow) arrow.style.transform = `rotate(${dir * 90}deg)`;
  }

  setOverlay(on: boolean): void {
    this.overlay = on;
    $('overlay').classList.toggle('on', on);
  }

  private set(id: string, value: string | number, apply: (el: HTMLElement) => void): void {
    if (this.last[id] === value) return;
    this.last[id] = value;
    apply($(id));
  }

  /** Refreshes every HUD value that changed. Call a few times per second. */
  update(perf: string): void {
    const { engine, content, icons } = this;
    const s = engine.stats;
    const storage = engine.storage;

    // Objective.
    const oi = s[Stat.Objective];
    const obj = content.objectives[oi];
    const have = engine.stat64(Stat.GoalHaveLo);
    const need = Math.max(1, engine.stat64(Stat.GoalNeedLo));
    const kind = s[Stat.GoalKind];
    const a = s[Stat.GoalA];
    const text = obj ? obj.text : kind === GoalKind.Ti ? `Reach ${formatCount(need)} Ti` : `Deliver ${content.items[a]?.name ?? ''}`;
    this.set('obj-num', oi + 1, (el) => (el.textContent = String(oi + 1)));
    this.set('obj-text', text, (el) => (el.textContent = text));
    const iconIndex =
      kind === GoalKind.Build
        ? buildingIcon(a)
        : kind === GoalKind.Deliver
          ? a
          : kind === GoalKind.Research
            ? buildingIcon(content.tech[a]?.unlocks[0] ?? content.tech[a]?.recipes[0]?.[0] ?? 3)
            : -2;
    this.set('obj-icon', iconIndex, (el) => {
      const img = kind === GoalKind.Ark ? SVG.ark : kind === GoalKind.Salvage ? SVG.wreck : SVG.planet;
      (el as HTMLImageElement).src =
        iconIndex === -2
          ? 'data:image/svg+xml,' + encodeURIComponent(img.replace('<svg', '<svg xmlns="http://www.w3.org/2000/svg" fill="#fbbf24"'))
          : icons.src(iconIndex);
    });
    const count =
      kind === GoalKind.Ti
        ? `${formatCount(have)} / ${formatCount(need)} Ti`
        : kind === GoalKind.Build || kind === GoalKind.Deliver || kind === GoalKind.Salvage
          ? `${formatCount(have)} / ${formatCount(need)}`
          : kind === GoalKind.Ark
            ? `Phase ${Math.min(have + 1, need)} of 5`
            : '';
    this.set('obj-count', count, (el) => (el.textContent = count));
    const pct = Math.min(100, Math.round((100 * have) / need));
    this.set('obj-bar', pct, (el) => (el.style.width = `${pct}%`));
    const reward = (obj?.reward ?? []).map(([it, n]) => `${icons.img(it, 16)}${n}`).join(' ');
    this.set('obj-reward', reward, (el) => (el.innerHTML = reward ? `Reward ${reward}` : ''));

    // Planet.
    const stage = s[Stat.Stage];
    const ti = engine.stat64(Stat.TiLo);
    const st = content.stages;
    this.set('stage-name', stage, (el) => (el.textContent = `${st[stage].name}`));
    const tiText = formatCount(ti);
    this.set('ti', tiText, (el) => (el.textContent = tiText));
    const rate = s[Stat.TiRateX100] / 100;
    const rateText = rate > 0 ? `+${formatCount(rate)}/s` : 'no growth yet';
    this.set('ti-rate', rateText, (el) => (el.textContent = rateText));
    const next = st[stage + 1];
    const sp = next ? Math.min(100, ((ti - st[stage].ti) / (next.ti - st[stage].ti)) * 100) : 100;
    this.set('stage-bar', Math.round(sp * 10), (el) => (el.style.width = `${sp}%`));

    // Power: demand against supply; red when some network falls short.
    const demand = s[Stat.PowerDemand];
    const supply = s[Stat.PowerSupply];
    const sat = s[Stat.PowerSat];
    const powerText = `${formatPower(demand)} / ${formatPower(supply)}`;
    const powerKey = `${powerText}|${sat < 999 ? 1 : 0}`;
    this.set('power', powerKey, (el) => {
      el.innerHTML = `${SVG.bolt}<span>${powerText}</span>`;
      el.classList.toggle('short', sat < 999);
      el.title = sat < 999 ? `Power short: machines run at ${Math.round(sat / 10)}%` : 'Power use / available';
    });

    // Meteor shower warning.
    const showerIn = s[Stat.ShowerIn];
    const shower = s[Stat.ShowerActive] ? 'Meteor shower!' : showerIn ? `Meteors in ${Math.ceil(showerIn / 60)}s` : '';
    this.set('shower', shower, (el) => {
      el.textContent = shower;
      el.hidden = !shower;
    });

    // Research badge: how many nodes could be researched right now (repeatables excluded).
    let ready = 0;
    content.tech.forEach((t, i) => {
      if (!t.repeat && engine.x.fx_tech_state(i) === 1 && canAfford(storage, t.cost)) ready++;
    });
    this.set('research-badge', ready, (el) => {
      el.textContent = String(ready);
      el.hidden = ready === 0;
    });

    // Ark badge: something in storage the current phase still needs.
    const phase = s[Stat.ArkPhase];
    const ark = content.ark[phase];
    let arkReady = 0;
    if (ark) {
      ark.cost.forEach(([item, n], k) => {
        if (s[Stat.ArkPaid + k] < n && storage[item] > 0) arkReady = 1;
      });
    }
    this.set('ark-badge', arkReady + phase * 2, (el) => {
      el.hidden = !arkReady;
    });
    const probes = s[Stat.Probes];
    this.set('menu-badge', probes, (el) => {
      el.textContent = String(probes);
      el.hidden = probes === 0;
    });

    this.set('undo-state', s[Stat.UndoDepth] > 0 ? 1 : 0, () => {
      ($('undo') as HTMLButtonElement).disabled = s[Stat.UndoDepth] === 0;
    });

    this.updateSelection(storage);
    if (this.showPerf) this.set('perf', perf, (el) => (el.textContent = perf));
  }

  /** The card above the build bar: what the tool does, costs, and its options. */
  private updateSelection(storage: Uint32Array): void {
    const { content, icons, engine } = this;
    const tool = this.tool;
    let sel = '';
    let interactive = false;
    if (tool >= 0) {
      const b = content.buildings[tool];
      const power = b.power && b.class !== 'battery' ? `<span class="chip power">${SVG.bolt}${b.class === 'generator' ? '+' : ''}${formatPower(b.power)}</span>` : '';
      sel = `${icons.img(buildingIcon(tool), 28)}<div class="sel-main"><b>${b.name}</b><small>${b.desc}</small></div><div class="sel-cost">${power}${costChips(icons, storage, b.cost, content)}</div>`;
      const options: string[] = [];
      if (b.class === 'crafter' && b.recipes.length > 1) {
        const current = this.placeRecipe.get(tool) ?? NO_RECIPE;
        options.push(`<button class="opt${current === NO_RECIPE ? ' on' : ''}" data-sel="recipe:${NO_RECIPE}">Auto</button>`);
        b.recipes.forEach((r, k) => {
          if (!recipeOpen(engine, tool, k)) return;
          options.push(
            `<button class="opt${current === k ? ' on' : ''}${r.unlock >= ALT_BASE ? ' alt' : ''}" data-sel="recipe:${k}" title="${recipeName(content, r)}">${icons.img(r.output[0], 18)}${r.unlock >= ALT_BASE ? '★' : ''}</button>`,
          );
        });
      }
      if (b.class === 'sorter') {
        options.push(`<button class="opt${this.placeFilter === 0 ? ' on' : ''}" data-sel="filter:0">Any</button>`);
        for (const it of knownItems(engine).slice(0, 18)) {
          options.push(`<button class="opt${this.placeFilter === it ? ' on' : ''}" data-sel="filter:${it}" title="${content.items[it].name}">${icons.img(it, 18)}</button>`);
        }
      }
      if (options.length) {
        interactive = true;
        sel += `<div class="sel-opts">${options.join('')}</div>`;
      }
    } else if (tool === TOOL_PASTE) {
      interactive = true;
      sel = `<div class="sel-main"><b>Paste blueprint</b><small>${this.blueprintInfo} · tap to place, ⟳ to rotate</small></div><div class="sel-opts"><button class="opt" data-sel="bp:save">Save</button><button class="opt" data-sel="bp:cancel">Done</button></div>`;
    } else if (tool === TOOL_COPY) {
      sel = `<div class="sel-main"><b>Copy</b><small>Drag over buildings to copy them as a blueprint.</small></div>`;
    }
    this.set('selection', sel, (el) => {
      el.innerHTML = sel;
      el.hidden = !sel;
      el.classList.toggle('interactive', interactive);
    });
  }

  togglePerf(): void {
    this.showPerf = !this.showPerf;
    $('perf').hidden = !this.showPerf;
  }

  toast(message: string, kind: 'info' | 'good' | 'bad' | 'epic' = 'info', ms = 2600): void {
    const el = $('toast');
    el.innerHTML = message;
    el.className = `show ${kind}`;
    clearTimeout(this.toastTimer);
    this.toastTimer = window.setTimeout(() => (el.className = kind), ms);
  }
}

