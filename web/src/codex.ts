// The guide: short tutorial topics, and the codex, a page for every item and building (what
// it is for, how to get it, what uses it). Pages offer live demos on request: the scene is
// built on a second engine instance and rendered by the game itself (demo.ts, scenes.ts).
// Pages hold no live counts, so the sheet's twice-a-second refresh never rebuilds them
// while a demo plays.

import { CATEGORIES, METER_COLORS, formatPower, recipeName, type Content, type RecipeDef, type Stack } from './content';
import type { Engine } from './engine';
import { isUnlocked, recipeOpen } from './hud';
import { buildingIcon, type Icons } from './icons';
import { TOPIC_SCENES, isAlternate, makers, paysFor, uses, type Maker, type Use } from './scenes';

export type CodexPage =
  | { view: 'guide' }
  | { view: 'topic'; id: string }
  | { view: 'codex' }
  | { view: 'item'; id: number }
  | { view: 'building'; id: number };

/** What each item is for, in a sentence or two. */
const ITEM_NOTES: Record<string, string> = {
  iron_ore: 'The most common ore. Smelt it into iron ingots, the backbone of every early build.',
  copper_ore: 'Smelt it into copper ingots for wire, circuits and batteries.',
  stone: 'Building material for smelters, generators and foundries. Crushed, it becomes sand.',
  coal: 'Fuel and reagent: it feeds Coal Generators and Heaters and alloys iron into steel.',
  ice: 'Frozen water. Melt it into water, or boil it into the sky with a Vaporizer to raise Pressure.',
  titanium_ore: 'A light, strong metal found further from the landing site. Needs Titanium research to mine.',
  uranium_ore: 'Radioactive ore far from the landing site. Packed into fuel rods for Reactors and Thermal Cores.',
  crude_oil: 'Pumped from oil seeps with an Oil Pump and refined into plastic and fuel.',
  meteorite: 'Space rock left behind by meteor showers. Mine the craters and forge it into alien alloy.',
  iron_ingot: 'Refined iron. It pays for your first buildings and is pressed into plates or alloyed into steel.',
  copper_ingot: 'Refined copper. Drawn into wire, and needed for drills and batteries.',
  sand: 'Crushed stone. It becomes glass and silicon, and algae grows in it.',
  glass: 'Smelted from sand. Greenhouses, labs, oxygenators and solar panels need plenty of it.',
  iron_plate: 'The universal part: gears, circuits, reinforced plates and most buildings use it.',
  copper_wire: 'Carries current: circuits, motors, poles and generators.',
  gear: 'Moving parts. Gears and wire make motors.',
  steel: 'Iron alloyed with coal. Tougher buildings, reinforced plates, frames and fuel rods.',
  circuit: 'Etched from wire and plates: the brains of mid-game machines and of computers.',
  motor: 'Gears plus wire. Pumps, drones, radars and the Ark need them.',
  water: 'Melted from ice, or pumped from lakes once they form. Grows algae and feeds greenhouses and hives.',
  titanium_ingot: 'Smelted titanium. Pressed into plates, or fused with meteorite into alien alloy.',
  titanium_plate: 'Light armour for frames, Thermal Cores and the Ark.',
  algae: 'Grown from water and sand. Oxygenators turn it into oxygen, and it brews fertilizer.',
  fertilizer: 'Algae and coal brewed together. Greenhouses and Hatcheries need it, and so do seeds.',
  fuel_rod: 'Uranium sealed in steel. A Reactor turns one into a huge amount of power; a Thermal Core into Heat.',
  frame: 'Titanium plates on a steel skeleton. The big late buildings and the Ark are made of them.',
  silicon: 'Refined from sand and coal. Solar panels, computers and dense batteries.',
  plastic: 'Refined from crude oil. Batteries and computers.',
  fuel: 'Refined from crude oil. Fuel Generators burn it for 150 MW, and the Ark needs it to launch.',
  battery: 'Copper and plastic. Battery Banks and Drone Ports are built from them.',
  computer: 'Circuits, plastic and silicon: radars, reactors, drones, quantum cores and the Ark.',
  reinforced_plate: 'Steel bolted to iron plates. Heavy buildings and the Ark’s hull.',
  alien_alloy: 'Meteorite fused with titanium, the stuff quantum cores are made of.',
  quantum_core: 'The pinnacle of the tech tree: alien alloy and a computer. The Ark’s last phase needs them.',
  seeds: 'Germinated from algae and fertilizer. Pollinator Hives spread them across the planet.',
  power_shard: 'Found in wrecks and earned from stages and achievements. Up to three in a machine overclock it to 250%.',
  amplifier: 'A rare find in wrecks, or bought in the shop. Doubles a machine’s output at four times the power.',
};

/** Where deposits of each raw resource turn up. */
const DEPOSIT_NOTES: Record<string, string> = {
  iron_ore: 'Common everywhere, including right next to the landing site.',
  copper_ore: 'Common everywhere, including next to the landing site.',
  stone: 'Common everywhere.',
  coal: 'Common everywhere.',
  ice: 'Common; there is a patch just north of the landing site.',
  titanium_ore: 'Further out (30 to 110 tiles). Needs Titanium research.',
  uranium_ore: 'Far out (70 tiles and more). Needs Nuclear research.',
  crude_oil: 'Oil seeps, 30 to 150 tiles out. Needs Oil Processing.',
  meteorite: 'Craters left by meteor showers near the Core, once the Ark’s Hull is built. Needs Xenometallurgy.',
};

const ITEM_GROUPS: { name: string; keys: string[] }[] = [
  {
    name: 'Raw resources',
    keys: ['iron_ore', 'copper_ore', 'stone', 'coal', 'ice', 'titanium_ore', 'uranium_ore', 'crude_oil', 'meteorite'],
  },
  {
    name: 'Materials',
    keys: ['iron_ingot', 'copper_ingot', 'sand', 'glass', 'water', 'steel', 'silicon', 'plastic', 'titanium_ingot'],
  },
  {
    name: 'Parts',
    keys: ['iron_plate', 'copper_wire', 'gear', 'circuit', 'motor', 'reinforced_plate', 'titanium_plate', 'frame', 'battery', 'computer'],
  },
  { name: 'Life', keys: ['algae', 'fertilizer', 'seeds'] },
  { name: 'Energy and exotics', keys: ['fuel', 'fuel_rod', 'alien_alloy', 'quantum_core'] },
  { name: 'Upgrades', keys: ['power_shard', 'amplifier'] },
];

const CRAFTER_TIP =
  'Inputs go in through the back and sides; the product comes out the front, where the arrow points. With several recipes it starts on Auto and makes whatever the first input allows: tap it to lock a recipe.';

/** How to use each building: what its one-line description leaves out. */
const BUILDING_TIPS: Record<string, string> = {
  belt: 'A belt feeds whatever its arrow points at. A belt that ends against the side of another merges into it, and the two lines share it.',
  core: 'Belt anything into any side. It supplies 40 MW to everything within 20 tiles. You can’t build, move or remove it.',
  drill: 'It mines forever. Tap a deposit first to see its purity: impure deposits mine at half speed, pure ones (gold rim) at double.',
  oil_pump: 'Oil seeps lie 30 to 150 tiles from the landing site. The pump pushes crude oil out toward its arrow.',
  water_pump: 'Lakes only form once the planet is warm and the air thick (the Liquid Water stage). Until then, melt ice in a Melter.',
  heater: 'Feed it coal from any side. Heat is the first meter to raise: it melts the ice caps, and warmth lets lakes form.',
  thermal: 'One fuel rod gives 5,000 Heat, as much as 250 coal burned in Heaters.',
  hatchery: 'Place it on the shore, touching a lake, and feed it algae and fertilizer from any side.',
  splitter: 'Outputs that are full or missing are skipped, so it also works with just two outputs, or as an overflow.',
  tunnel: 'Both ends must face the way items travel. Belts and buildings can sit on the tiles in between, so lines can cross.',
  incinerator: 'A safety valve for lines that would otherwise clog. Feed it from any side.',
  pole: 'The eye button shows its coverage. A line of poles carries power from the Core’s field, or from generators, to distant machines.',
  coal_gen: 'Belt coal into any side, and keep it inside a pole’s or the Core’s coverage: otherwise its power has nowhere to go.',
  fuel_gen: 'Belt fuel into any side and keep it inside a grid’s coverage. Like every generator, it burns only as fast as the grid draws power.',
  reactor: 'Belt fuel rods into it inside a grid’s coverage. At full load one rod lasts 50 seconds; at light load, much longer.',
  solar: 'Needs no fuel and no belts, just a place inside a grid’s coverage. Night covers about a third of the 8-minute day.',
  battery_bank: 'Place it inside a grid’s coverage. It charges whenever generators make more than the grid uses.',
  sorter: 'Choose the item on the building card before placing, or later in the inspector. With no item chosen, everything goes straight on.',
  storage: 'Put one before a slow machine to soak up bursts. Removing it sends its contents to the Core.',
  drone_port: 'Feed items into one port, tap it, choose Link, then tap the receiving port. The receiving port outputs through its front.',
  radar: 'Needs power while it scans, then goes idle. Place it where you plan to expand.',
  recycler: 'Needs power. Advanced parts pay far more than raw ore: each item’s codex page shows its value.',
};

interface Topic {
  id: string;
  title: string;
  blurb: string;
  html: string;
  /** Codex pages worth a look afterwards: `b:key` buildings, `i:key` items. */
  see: string[];
}

const TOPICS: Topic[] = [
  {
    id: 'start',
    title: 'First steps',
    blurb: 'Mine, belt, smelt, deliver',
    html: `<p>You landed next to the <b>Core</b>. Everything you belt into it is stored there, and pays for new buildings, research and, later, the Ark.</p>
      <ol>
        <li><b>Drill</b> (Extraction tab): tap an ore deposit to place it. It mines the tile under it and pushes the ore out toward its arrow. ⟳ turns the arrow before you place.</li>
        <li><b>Belt</b> (Logistics tab): drag from the drill. Belts carry items toward their arrows, and corners appear by themselves.</li>
        <li><b>Smelter</b>: place it where the belt ends. Ore goes in the back or sides, ingots come out the front.</li>
        <li>Belt the ingots into the Core. The objective at the top then shows your next step: tap it for help.</li>
      </ol>`,
    see: ['b:drill', 'b:belt', 'b:smelter', 'b:core'],
  },
  {
    id: 'controls',
    title: 'Controls',
    blurb: 'Move, build, inspect, undo',
    html: `<ul>
        <li><b>Look around</b>: drag with two fingers to pan and pinch to zoom. With <i>Move</i> selected, one finger pans too. Mouse: drag with <i>Move</i>, wheel to zoom.</li>
        <li><b>Build</b>: pick a tab, then a building. Tap to place one, drag to place a line. ⟳ rotates what you place next.</li>
        <li><b>Inspect</b>: with <i>Move</i>, tap a building to see what it’s doing, pick its recipe, overclock it, or remove it. Removing refunds the cost.</li>
        <li><b>Undo</b> (↶) reverts your last gesture. <i>Delete</i> removes what you drag over.</li>
        <li>The <b>eye</b> button shows power coverage and marks stuck machines.</li>
        <li>Keyboard: R rotate, Q move, X delete, Z undo, B copy, V paste, O overlay, T research, C Core, K Ark, P planet, 1–9 buildings, Tab next tab.</li>
      </ul>`,
    see: [],
  },
  {
    id: 'belts',
    title: 'Belts',
    blurb: 'Moving items, merging lines',
    html: `<p>Belts move items toward their arrows, up to 225 a minute (research raises that to 300, then 450).</p>
      <ul>
        <li>A belt feeds whatever its arrow points at: a machine, the Core, or another belt.</li>
        <li>A belt that ends against the <b>side</b> of another merges into it, and the two lines share it.</li>
        <li>Drag to lay a line; turn while dragging to make corners. Belts are free, so build as many as you like.</li>
      </ul>`,
    see: ['b:belt', 'b:splitter', 'b:tunnel'],
  },
  {
    id: 'machines',
    title: 'Machines and recipes',
    blurb: 'Inputs, outputs, Auto and locked recipes',
    html: `<p>Every machine takes one tile. It takes inputs through its back and sides and puts its product out the front, where the arrow points.</p>
      <ul>
        <li>Machines with several recipes start on <b>Auto</b>: they make whatever the first input to arrive allows. Tap one and pick a recipe to lock it.</li>
        <li>For recipes with two or three inputs, bring each on its own belt from a different side, or mix them on one belt.</li>
        <li>A machine stops when its output is blocked, when an input runs out, or without power. The eye button marks stuck machines.</li>
        <li>The <b>Codex</b> lists every recipe: what makes each item, and what uses it.</li>
      </ul>`,
    see: ['b:press', 'b:assembler', 'b:foundry'],
  },
  {
    id: 'logistics',
    title: 'Splitting, sorting, crossing',
    blurb: 'Splitters, sorters, tunnels, storage',
    html: `<ul>
        <li><b>Splitter</b>: deals what comes in from behind to its front, left and right in turn. Feed several machines from one line.</li>
        <li><b>Sorter</b>: its chosen item goes straight on, everything else to the sides. Pull one item out of a mixed belt.</li>
        <li><b>Tunnel</b>: an entrance and an exit facing the same way, up to 5 tiles apart. Items pass under belts and buildings.</li>
        <li><b>Storage</b> buffers up to 500 of one item before a slow machine; an <b>Incinerator</b> destroys surplus.</li>
      </ul>`,
    see: ['b:splitter', 'b:sorter', 'b:tunnel', 'b:storage', 'b:incinerator'],
  },
  {
    id: 'power',
    title: 'Power',
    blurb: 'Poles, generators, batteries',
    html: `<p>Most machines need power: their cards show how much (⚡). The Core supplies 40 MW to everything within 20 tiles.</p>
      <ul>
        <li><b>Power Poles</b> carry power 5 tiles around them and link to poles up to 10 tiles away, forming a grid.</li>
        <li><b>Generators</b> add supply: coal (60 MW), solar (8 MW by day), fuel (150 MW), reactors (1.2 GW). They burn fuel only as fast as the grid draws power.</li>
        <li>When a grid needs more than it has, all of its machines slow down together. Nothing breaks: add generators.</li>
        <li><b>Battery Banks</b> store surplus and cover shortfalls, such as solar panels at night.</li>
        <li>The eye button shows coverage; the Core’s Power tab shows each grid.</li>
      </ul>`,
    see: ['b:pole', 'b:coal_gen', 'b:solar', 'b:battery_bank'],
  },
  {
    id: 'research',
    title: 'Research and objectives',
    blurb: 'What to do next, and how to unlock it',
    html: `<ul>
        <li>The <b>objective</b> at the top is your next step, with a reward. Tap it for help.</li>
        <li><b>Research</b> (flask button) unlocks buildings, recipes and upgrades, paid from the Core’s storage.</li>
        <li>Some research also needs a terraforming stage or an Ark phase; locked cards say what’s missing.</li>
        <li>Upgrades such as faster belts, drills and machines apply to everything you have already built.</li>
      </ul>`,
    see: ['b:core'],
  },
  {
    id: 'terraform',
    title: 'Terraforming',
    blurb: 'Heat, pressure, oxygen, biomass',
    html: `<p>Terraformers burn goods to raise four meters: <b style="color:${METER_COLORS[0]}">Heat</b>, <b style="color:${METER_COLORS[1]}">Pressure</b>, <b style="color:${METER_COLORS[2]}">Oxygen</b> and <b style="color:${METER_COLORS[3]}">Biomass</b>. Their sum is the Terraform Index (Ti).</p>
      <ul>
        <li>Each stage changes the planet (ice retreats, lakes fill, moss and forests spread), pays credits or power shards, and unlocks research.</li>
        <li>Start with the Heater and coal. Later come Vaporizers (ice), Oxygenators (algae), Greenhouses, Pollinator Hives, Hatcheries and the Thermal Core.</li>
        <li>Lakes appear once the planet is warm and the air thick; then Water Pumps and Hatcheries have water to use.</li>
        <li>Tap the planet card (top right) to see every meter and stage.</li>
      </ul>`,
    see: ['b:heater', 'b:vaporizer', 'b:oxygenator', 'b:greenhouse'],
  },
  {
    id: 'explore',
    title: 'Exploring',
    blurb: 'Fog, wrecks, purity, meteors',
    html: `<ul>
        <li>The map starts hidden. Building reveals the land around it; Power Poles see further, and a Radar reveals up to 70 tiles.</li>
        <li><b>Wrecks</b> of the first expedition hold supplies, logs, power shards, amplifiers and data probes. Tap one to salvage it.</li>
        <li><b>Data probes</b> unlock alternate recipes (★), often better than the standard ones.</li>
        <li>Deposits come <b>impure</b> (half speed), normal, or <b>pure</b> (double speed, gold rim). Rarer ores lie further out.</li>
        <li>Once the Ark’s Hull is built, meteor showers leave meteorite deposits near the Core.</li>
      </ul>`,
    see: ['b:radar', 'i:meteorite'],
  },
  {
    id: 'boost',
    title: 'Boosting machines',
    blurb: 'Power shards and amplifiers',
    html: `<ul>
        <li><b>Power shards</b> overclock a machine: one, two or three give 150%, 200% or 250% speed, for more power. Install them from the inspector.</li>
        <li>An <b>amplifier</b> doubles a machine’s output, at four times the power.</li>
        <li>Shards come from wrecks, terraforming stages and achievements; amplifiers from wrecks and the shop.</li>
        <li>Continuous research adds a few percent to mining, factories, terraforming or power with every level.</li>
      </ul>`,
    see: ['i:power_shard', 'i:amplifier'],
  },
  {
    id: 'drones',
    title: 'Drones and blueprints',
    blurb: 'Long-distance delivery, copy and paste',
    html: `<ul>
        <li><b>Drone Ports</b>: feed items into one, tap it, choose <i>Link</i>, then tap another port. Its drone flies loads across the map, over lava and lakes.</li>
        <li><b>Blueprints</b>: pick <i>Copy</i> and drag over part of your factory, then tap to paste copies (⟳ rotates). Save them from the paste card; the menu lists them and makes share codes.</li>
      </ul>`,
    see: ['b:drone_port'],
  },
  {
    id: 'ark',
    title: 'The Ark',
    blurb: 'Five phases to a new world',
    html: `<p>The Ark is the colony’s ship, built in five phases from goods delivered to the Core (rocket button, then Send).</p>
      <ul>
        <li>Each phase unlocks the next research tier.</li>
        <li>The last phase launches the Ark to a new planet. You keep credits, cosmetics and achievements, and every launch makes all production 10% faster.</li>
        <li>Or stay, and keep terraforming until the planet is fully alive.</li>
      </ul>`,
    see: ['i:quantum_core', 'i:alien_alloy', 'i:fuel_rod'],
  },
];

/** The demo slot: a poster that plays `scene` on request; the player takes its place. */
function demoSlot(scene: string, label = 'See it in action'): string {
  return `<div class="demo-slot"><button class="demo-poster" type="button" data-act="demo:${scene}"><span class="demo-play">▶</span>${label}</button></div>`;
}

const PLAY = (scene: string, title: string) =>
  `<button class="play" type="button" data-act="demo:${scene}" title="${title}" aria-label="${title}">▶</button>`;

export class Codex {
  constructor(
    private readonly engine: Engine,
    private readonly icons: Icons,
  ) {}

  private get content(): Content {
    return this.engine.content;
  }

  /** Title and body of a page. */
  render(page: CodexPage, canGoBack: boolean): [string, string] {
    const nav = `<div class="cx-nav">${canGoBack ? '<button class="btn" data-act="cx:back">‹ Back</button>' : ''}<button class="btn${page.view === 'guide' ? ' on' : ''}" data-act="cx:guide">Guide</button><button class="btn${page.view === 'codex' ? ' on' : ''}" data-act="cx:codex">Codex</button></div>`;
    switch (page.view) {
      case 'guide':
        return ['Guide', nav + this.guide()];
      case 'topic': {
        const t = TOPICS.find((x) => x.id === page.id) ?? TOPICS[0];
        return [t.title, nav + this.topic(t)];
      }
      case 'codex':
        return ['Codex', nav + this.index()];
      case 'item':
        return [this.content.items[page.id]?.name ?? 'Item', nav + this.item(page.id)];
      case 'building':
        return [this.content.buildings[page.id]?.name ?? 'Building', nav + this.building(page.id)];
    }
  }

  // ---- Links ---------------------------------------------------------------------------------------

  private itemLink(i: number, size = 18): string {
    const it = this.content.items[i];
    return `<button class="cx-chip" type="button" data-act="cx:item:${i}">${this.icons.img(i, size)}${it.name}</button>`;
  }

  private bldLink(k: number, size = 18): string {
    const b = this.content.buildings[k];
    return `<button class="cx-chip" type="button" data-act="cx:bld:${k}">${this.icons.img(buildingIcon(k), size)}${b.name}</button>`;
  }

  /** An item stack that opens the item's page. */
  private stack(item: number, n: string): string {
    const it = this.content.items[item];
    return `<button class="stack link" type="button" data-act="cx:item:${item}" title="${it.name}">${this.icons.img(item, 20)}${n}</button>`;
  }

  private recipe(r: RecipeDef, b: { class: string; points: number; meter: number }): string {
    const ins = r.inputs.map(([it, n]) => this.stack(it, String(n))).join('');
    const out =
      b.class === 'terraformer'
        ? `<span class="stack" style="color:${METER_COLORS[b.meter]}">+${b.points} ${this.content.meters[b.meter].name}</span>`
        : this.stack(r.output[0], `×${r.output[1]}`);
    return `<div class="recipe">${ins}<span class="arrow">→</span>${out}<small>${(r.ticks / 60).toFixed(r.ticks % 60 ? 2 : 0)} s</small></div>`;
  }

  private costChips(cost: Stack[]): string {
    if (!cost.length) return '<span class="chip free">Free</span>';
    return cost.map(([i, n]) => `<button class="chip link" type="button" data-act="cx:item:${i}" title="${this.content.items[i].name}">${this.icons.img(i, 16)}${n}</button>`).join('');
  }

  /** "Research: X" when the building or recipe is still locked. */
  private lockNote(research: number): string {
    if (isUnlocked(this.engine, research)) return '';
    const t = this.content.tech[research];
    return t ? `<small class="lock">Research: ${t.name}</small>` : '';
  }

  private recipeLock(kind: number, r: number): string {
    const rc = this.content.buildings[kind].recipes[r];
    if (recipeOpen(this.engine, kind, r)) return isAlternate(rc) ? '<small class="alt">★ Alternate (unlocked)</small>' : '';
    if (isAlternate(rc)) return '<small class="lock">★ Alternate: found on data probes</small>';
    return this.lockNote(rc.unlock) || this.lockNote(this.content.buildings[kind].research);
  }

  // ---- Guide -------------------------------------------------------------------------------------

  private guide(): string {
    const cards = TOPICS.map(
      (t, i) =>
        `<button class="gd-card" type="button" data-act="cx:topic:${t.id}"><b><span class="num">${i + 1}</span>${t.title}</b><small>${t.blurb}</small>${TOPIC_SCENES[t.id] ? '<em>▶ demo</em>' : ''}</button>`,
    ).join('');
    return `<p class="hint">Short lessons, most with a live demo played by the game itself. For any item or building, open the <b>Codex</b>: tap an item in the Core, the <b>?</b> on a building card, or <i>How it works</i> in the inspector.</p>
      <div class="gd-list">${cards}</div>
      <button class="gd-card wide" type="button" data-act="cx:codex"><b>Codex</b><small>Every item and building: what it is for, how to get it, and a demo of each.</small></button>`;
  }

  private topic(t: Topic): string {
    const i = TOPICS.indexOf(t);
    const scenes = TOPIC_SCENES[t.id] ?? [];
    let demo = '';
    if (scenes.length) {
      demo = demoSlot(`topic:${t.id}:0`, `Watch: ${scenes[0].label}`);
      if (scenes.length > 1) {
        demo += `<div class="cx-demos">${scenes.map((s, k) => `<button class="opt" type="button" data-act="demo:topic:${t.id}:${k}">▶ ${s.label}</button>`).join('')}</div>`;
      }
    }
    const see = t.see
      .map((s) => {
        const [kind, key] = s.split(':');
        if (kind === 'b') {
          const k = this.content.buildings.findIndex((b) => b.key === key);
          return k >= 0 ? this.bldLink(k) : '';
        }
        const it = this.content.items.findIndex((x) => x.key === key);
        return it > 0 ? this.itemLink(it) : '';
      })
      .join('');
    const prev = TOPICS[i - 1];
    const next = TOPICS[i + 1];
    return `<div class="gd-body">${demo}${t.html}</div>
      ${see ? `<h3>In the codex</h3><div class="cx-links">${see}</div>` : ''}
      <div class="cx-pager">${prev ? `<button class="btn" data-act="cx:topic:${prev.id}">‹ ${prev.title}</button>` : '<span></span>'}${next ? `<button class="btn primary" data-act="cx:topic:${next.id}">${next.title} ›</button>` : '<button class="btn primary" data-act="cx:codex">Open the Codex ›</button>'}</div>`;
  }

  // ---- Codex index ------------------------------------------------------------------------------

  private index(): string {
    const c = this.content;
    const byKey = new Map(c.items.map((it, i) => [it.key, i]));
    const items = ITEM_GROUPS.map((g) => {
      const tiles = g.keys
        .map((key) => byKey.get(key))
        .filter((i): i is number => i !== undefined)
        .map((i) => `<button class="cx-tile" type="button" data-act="cx:item:${i}">${this.icons.img(i, 34)}<span>${c.items[i].name}</span></button>`)
        .join('');
      return `<h4>${g.name}</h4><div class="cx-grid">${tiles}</div>`;
    }).join('');
    const buildings = CATEGORIES.map((cat, ci) => {
      const tiles = c.buildings
        .map((b, k) => ({ b, k }))
        .filter(({ b }) => b.category === ci)
        .map(({ b, k }) => {
          const locked = !isUnlocked(this.engine, b.research);
          return `<button class="cx-tile${locked ? ' locked' : ''}" type="button" data-act="cx:bld:${k}">${this.icons.img(buildingIcon(k), 34)}<span>${b.name}</span></button>`;
        })
        .join('');
      return `<h4 style="color:${cat.accent}">${cat.name}</h4><div class="cx-grid">${tiles}</div>`;
    }).join('');
    const core = this.content.buildings.findIndex((b) => b.class === 'core');
    return `<p class="hint">Tap anything to see what it’s for, how to get it, and a live demo. Faded buildings still need research.</p>
      <h3>Items</h3>${items}<h3>Buildings</h3><div class="cx-grid">${core >= 0 ? `<button class="cx-tile" type="button" data-act="cx:bld:${core}">${this.icons.img(buildingIcon(core), 34)}<span>Core</span></button>` : ''}</div>${buildings}`;
  }

  // ---- Items ---------------------------------------------------------------------------------------

  private item(i: number): string {
    const c = this.content;
    const it = c.items[i];
    if (!it || i === 0) return '<p>Unknown item.</p>';
    const group = ITEM_GROUPS.find((g) => g.keys.includes(it.key))?.name ?? 'Item';
    const ms = makers(c, i);
    const us = uses(c, i);
    const makeRows = ms.map((m, k) => this.makerRow(i, m, k)).join('');
    const useRows = us.map((u, k) => this.useRow(i, u, k)).join('');
    return `<div class="cx-head">${this.icons.img(i, 52)}<div><b>${it.name}</b><small>${group} · a Recycler pays ${it.value} credit${it.value === 1 ? '' : 's'}</small></div></div>
      <p class="cx-lead">${ITEM_NOTES[it.key] ?? ''}</p>
      ${demoSlot(`item:${i}`)}
      <h3>How to get it</h3><div class="cx-rows">${makeRows}</div>
      <h3>What it’s for</h3><div class="cx-rows">${useRows || '<p class="hint">Nothing uses it directly.</p>'}</div>`;
  }

  private makerRow(item: number, m: Maker, k: number): string {
    const c = this.content;
    const name = c.items[item].name;
    const play = PLAY(`make:${item}:${k}`, 'Show how it is made');
    switch (m.kind) {
      case 'mine': {
        const b = c.buildings[m.building];
        return `<div class="cx-row">${this.icons.img(buildingIcon(m.building), 32)}<div class="cx-main"><b>${b.name}</b> on ${name} deposits<small>${DEPOSIT_NOTES[c.items[item].key] ?? ''}</small>${this.lockNote(b.research)}</div>${play}</div>`;
      }
      case 'pump': {
        const b = c.buildings[m.building];
        return `<div class="cx-row">${this.icons.img(buildingIcon(m.building), 32)}<div class="cx-main"><b>${b.name}</b> on a lake<small>Lakes form from the Liquid Water stage on.</small>${this.lockNote(b.research)}</div>${play}</div>`;
      }
      case 'craft': {
        const b = c.buildings[m.building];
        const r = b.recipes[m.recipe];
        return `<div class="cx-row">${this.icons.img(buildingIcon(m.building), 32)}<div class="cx-main"><button class="cx-name" type="button" data-act="cx:bld:${m.building}">${b.name}${isAlternate(r) ? ` · ${r.name}` : ''}</button>${this.recipe(r, b)}${this.recipeLock(m.building, m.recipe)}</div>${play}</div>`;
      }
      case 'find':
        return `<div class="cx-row"><div class="cx-main"><b>Not manufactured</b><small>${c.items[item].key === 'amplifier' ? 'Found in wrecks far from the landing site, or bought in the shop.' : 'Found in wrecks, and paid out by terraforming stages and achievements.'}</small></div></div>`;
    }
  }

  private useRow(item: number, u: Use, k: number): string {
    const c = this.content;
    const play = PLAY(`use:${item}:${k}`, 'Show this use');
    switch (u.kind) {
      case 'craft': {
        const b = c.buildings[u.building];
        const r = b.recipes[u.recipe];
        return `<div class="cx-row">${this.icons.img(r.output[0], 32)}<div class="cx-main"><button class="cx-name" type="button" data-act="cx:bld:${u.building}">${b.name}: ${recipeName(c, r)}</button>${this.recipe(r, b)}${this.recipeLock(u.building, u.recipe)}</div>${play}</div>`;
      }
      case 'terraform': {
        const b = c.buildings[u.building];
        return `<div class="cx-row">${this.icons.img(buildingIcon(u.building), 32)}<div class="cx-main"><button class="cx-name" type="button" data-act="cx:bld:${u.building}">${b.name}</button>${this.recipe(b.recipes[0], b)}${this.lockNote(b.research)}</div>${play}</div>`;
      }
      case 'burn': {
        const b = c.buildings[u.building];
        const secs = b.fuel[1] / Math.max(1, b.power);
        return `<div class="cx-row">${this.icons.img(buildingIcon(u.building), 32)}<div class="cx-main"><button class="cx-name" type="button" data-act="cx:bld:${u.building}">${b.name}</button><small>Burned for ${formatPower(b.power)}: one lasts ${secs >= 60 ? `${Math.round(secs / 60)} min` : `${Math.round(secs)} s`} at full load.</small>${this.lockNote(b.research)}</div>${play}</div>`;
      }
      case 'install':
        return `<div class="cx-row"><div class="cx-main"><b>Install in a machine</b><small>${c.items[item].key === 'amplifier' ? 'Tap a drill, pump, crafter or terraformer and choose Install: twice the output, four times the power.' : 'Tap a machine and press + next to Overclock: each shard adds 50% speed, up to 250%.'}</small></div>${play}</div>`;
      case 'core': {
        const pays = paysFor(c, item);
        const parts = [
          pays.buildings.length ? `<div class="cx-links">${pays.buildings.map((b) => this.bldLink(b, 16)).join('')}</div>` : '',
          pays.tech.length ? `<small>Research: ${pays.tech.map((t) => c.tech[t].name).join(', ')}</small>` : '',
          pays.ark.length ? `<small>The Ark: ${pays.ark.map((a) => c.ark[a].name).join(', ')}</small>` : '',
        ].join('');
        return `<div class="cx-row">${this.icons.img(buildingIcon(this.content.buildings.findIndex((b) => b.class === 'core')), 32)}<div class="cx-main"><b>Deliver it to the Core</b>${parts}</div>${play}</div>`;
      }
    }
  }

  // ---- Buildings ------------------------------------------------------------------------------------

  private building(k: number): string {
    const c = this.content;
    const b = c.buildings[k];
    if (!b) return '<p>Unknown building.</p>';
    const cat = CATEGORIES[b.category];
    const power =
      b.class === 'generator'
        ? `+${formatPower(b.power)}`
        : b.class === 'battery'
          ? `stores ${Math.round(b.store / 1000)} MJ`
          : b.power
            ? `uses ${formatPower(b.power)}`
            : b.class === 'core'
              ? `+${formatPower(c.corePower)}`
              : 'no power needed';
    const tip = BUILDING_TIPS[b.key] ?? (b.class === 'crafter' ? CRAFTER_TIP : b.class === 'terraformer' ? `Feed it from any side; it has no output. Every cycle raises the planet’s ${c.meters[b.meter]?.name ?? ''} meter.` : '');
    const parts: string[] = [];
    if (b.recipes.length) {
      const rows = b.recipes
        .map((r, j) => {
          const out = b.class === 'terraformer' ? '' : `<b>${recipeName(c, r)}</b>`;
          return `<div class="cx-row"><div class="cx-main">${out}${this.recipe(r, b)}${this.recipeLock(k, j)}</div></div>`;
        })
        .join('');
      parts.push(`<h3>${b.class === 'terraformer' ? 'Burns' : 'Recipes'}</h3><div class="cx-rows">${rows}</div>`);
    }
    if (b.deposits.length) {
      parts.push(`<h3>Mines</h3><div class="cx-links">${b.deposits.map((d) => this.itemLink(d)).join('')}</div>`);
    }
    if (b.class === 'pump') parts.push(`<h3>Pumps</h3><div class="cx-links">${this.itemLink(c.items.findIndex((x) => x.key === 'water'))}</div>`);
    if (b.class === 'generator' && b.fuel[0]) {
      parts.push(`<h3>Burns</h3><div class="cx-links">${this.itemLink(b.fuel[0])}</div>`);
    }
    if (b.class !== 'core') {
      const unlock = b.research === 255 ? 'Available from the start.' : `Unlocked by research: <b>${c.tech[b.research]?.name ?? '?'}</b> (tier ${c.tech[b.research]?.tier ?? 1})${isUnlocked(this.engine, b.research) ? ' ✓' : ''}.`;
      parts.push(`<h3>Cost</h3><div class="chips">${this.costChips(b.cost)}</div><p class="hint">${unlock} Removing it refunds the full cost.</p>`);
    }
    return `<div class="cx-head">${this.icons.img(buildingIcon(k), 52)}<div><b>${b.name}</b><small><span style="color:${cat?.accent ?? '#fbbf24'}">${cat?.name ?? 'Colony'}</span> · ${power}</small></div></div>
      <p class="cx-lead">${b.desc}</p>${tip ? `<p class="cx-tip">${tip}</p>` : ''}
      ${demoSlot(`bld:${k}`)}
      ${parts.join('')}`;
  }
}
