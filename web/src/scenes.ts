// Scenes for the guide's live demos (played by demo.ts). Item scenes come straight from the
// game's recipe tables: the machine that makes an item feeding one that uses it. Buildings,
// logistics, power and the tutorial topics have hand-made scenes. Scene ids:
//   item:I        how item I is made and first used     make:I:K  its K-th way to be made
//   use:I:K       its K-th use                          bld:K     building kind K at work
//   topic:NAME    a guide topic

import { ALT_BASE, FREE, recipeName, type Content, type RecipeDef } from './content';
import type { Scene, SceneCtx, Side } from './demo';

const E = 0;
const S = 1;
const N = 3;
const FEED_SIDES: Side[] = ['w', 'n', 's'];

/** How an item is obtained. */
export type Maker =
  | { kind: 'mine'; building: number }
  | { kind: 'pump'; building: number }
  | { kind: 'craft'; building: number; recipe: number }
  /** Not made at all: found in wrecks, earned, or bought. */
  | { kind: 'find' };

/** What an item is good for. */
export type Use =
  | { kind: 'craft'; building: number; recipe: number }
  | { kind: 'terraform'; building: number }
  | { kind: 'burn'; building: number }
  /** Installed into machines (power shards, amplifiers). */
  | { kind: 'install' }
  /** Delivered to the Core to pay for buildings, research or the Ark. */
  | { kind: 'core' };

/** Content lookups by key (ids are stable, but keys read better in scenes). */
class Keys {
  private readonly items = new Map<string, number>();
  private readonly kinds = new Map<string, number>();
  constructor(c: Content) {
    c.items.forEach((d, i) => this.items.set(d.key, i));
    c.buildings.forEach((d, k) => this.kinds.set(d.key, k));
  }
  item(key: string): number {
    return this.items.get(key) ?? 0;
  }
  kind(key: string): number {
    return this.kinds.get(key) ?? 0;
  }
}

const keyCache = new WeakMap<Content, Keys>();
function keys(c: Content): Keys {
  let k = keyCache.get(c);
  if (!k) keyCache.set(c, (k = new Keys(c)));
  return k;
}

export function isAlternate(r: RecipeDef): boolean {
  return r.unlock >= ALT_BASE && r.unlock !== FREE;
}

/** Ways to obtain `item`: mining first, then standard recipes, pumps, then alternates. */
export function makers(c: Content, item: number): Maker[] {
  const mine: Maker[] = [];
  const craft: Maker[] = [];
  const pump: Maker[] = [];
  const alts: Maker[] = [];
  c.buildings.forEach((b, k) => {
    if (b.class === 'drill' && b.deposits.includes(item)) mine.push({ kind: 'mine', building: k });
    if (b.class === 'pump' && c.items[item]?.key === 'water') pump.push({ kind: 'pump', building: k });
    if (b.class !== 'crafter') return;
    b.recipes.forEach((r, j) => {
      if (r.output[0] === item) (isAlternate(r) ? alts : craft).push({ kind: 'craft', building: k, recipe: j });
    });
  });
  const all = [...mine, ...craft, ...pump, ...alts];
  return all.length ? all : [{ kind: 'find' }];
}

/** What `item` is used for, most direct first. */
export function uses(c: Content, item: number): Use[] {
  const out: Use[] = [];
  const alts: Use[] = [];
  c.buildings.forEach((b, k) => {
    if (b.class === 'crafter') {
      b.recipes.forEach((r, j) => {
        if (r.inputs.some(([i]) => i === item)) (isAlternate(r) ? alts : out).push({ kind: 'craft', building: k, recipe: j });
      });
    } else if (b.class === 'terraformer' && b.recipes[0]?.inputs.some(([i]) => i === item)) {
      out.push({ kind: 'terraform', building: k });
    } else if (b.class === 'generator' && item !== 0 && b.fuel[0] === item) {
      out.push({ kind: 'burn', building: k });
    }
  });
  out.push(...alts);
  const key = c.items[item]?.key;
  if (key === 'power_shard' || key === 'amplifier') out.push({ kind: 'install' });
  const pays = paysFor(c, item);
  if (pays.buildings.length || pays.tech.length || pays.ark.length) out.push({ kind: 'core' });
  return out;
}

/** Buildings, research and Ark phases whose cost includes `item`. */
export function paysFor(c: Content, item: number): { buildings: number[]; tech: number[]; ark: number[] } {
  const has = (cost: [number, number][]) => cost.some(([i]) => i === item);
  return {
    buildings: c.buildings.flatMap((b, k) => (b.category !== 255 && has(b.cost) ? [k] : [])),
    tech: c.tech.flatMap((t, k) => (has(t.cost) ? [k] : [])),
    ark: c.ark.flatMap((a, k) => (has(a.cost) ? [k] : [])),
  };
}

/** Machine speed that makes a `ticks`-long cycle take about two seconds at most. */
function speedFor(ticks: number): number {
  return Math.min(400, Math.max(100, Math.ceil(ticks / 120) * 100));
}

// ---- Item chains ---------------------------------------------------------------------------------

/**
 * The item's maker (on the left) belting it into a user (on the right). Either end may be
 * missing: the item then arrives from off-screen, or leaves off-screen.
 */
function chain(c: Content, item: number, maker: Maker | undefined, use: Use | undefined, focus?: 'maker' | 'user'): Scene {
  if (use?.kind === 'burn') return power(c, use.building);
  if (use?.kind === 'install' || maker?.kind === 'find') return boost(c);
  const name = c.items[item].name;
  const made = maker !== undefined;
  // Three belt tiles between maker and user leave room for both labels on a phone.
  const ux = made ? 4 : 0;
  const parts: string[] = [];
  let ticks = 60;
  let vertical = false;
  let wet = false;
  const b = (k: number) => c.buildings[k];

  if (maker?.kind === 'mine') parts.push(`A ${b(maker.building).name} mines ${name} from the deposit under it`);
  else if (maker?.kind === 'pump') {
    parts.push(`A ${b(maker.building).name} draws water from a lake`);
    wet = true;
  } else if (maker?.kind === 'craft') {
    const r = b(maker.building).recipes[maker.recipe];
    ticks = Math.max(ticks, r.ticks);
    vertical ||= r.inputs.length > 1;
    const ins = r.inputs.map(([i]) => c.items[i].name).join(' + ');
    parts.push(`The ${b(maker.building).name} makes ${name} from ${ins}${isAlternate(r) ? ` (alternate: ${r.name})` : ''}`);
  }
  let userRecipe: RecipeDef | null = null;
  if (use?.kind === 'craft') {
    userRecipe = b(use.building).recipes[use.recipe];
    ticks = Math.max(ticks, userRecipe.ticks);
    vertical ||= userRecipe.inputs.length > 1;
    const out = recipeName(c, userRecipe);
    parts.push(`${made ? 'then the' : 'The'} ${b(use.building).name} uses ${made ? 'it' : name} to make ${out}`);
  } else if (use?.kind === 'terraform') {
    const t = b(use.building);
    ticks = Math.max(ticks, t.recipes[0].ticks);
    vertical ||= t.recipes[0].inputs.length > 1;
    wet ||= t.key === 'hatchery';
    const fuel = made ? 'it' : t.recipes[0].inputs.map(([i]) => c.items[i].name).join(' and ');
    parts.push(`${made ? 'then the' : 'The'} ${t.name} burns ${fuel}: +${t.points} ${c.meters[t.meter].name} per cycle`);
  } else if (use?.kind === 'core') {
    parts.push(`${made ? 'then it goes' : `${name} goes`} to the Core, to pay for buildings, research and the Ark`);
  }
  const speed = speedFor(ticks);
  const top = vertical ? -3 : -2;
  const right = use?.kind === 'core' ? ux + 4 : use ? ux + 2 : 3;

  return {
    caption: parts.join(', ') + '.',
    core: use?.kind === 'core' ? [ux, -2] : undefined,
    wet,
    speed,
    warmup: Math.min(1200, 300 + Math.round((ticks * 250) / speed)),
    build(k: SceneCtx) {
      // The maker.
      if (maker?.kind === 'mine') {
        k.ore(-1, -1, 0, 1, item);
        k.put(0, 0, maker.building, E, { label: b(maker.building).name, focus: focus === 'maker' });
      } else if (maker?.kind === 'pump') {
        k.pond(-1, -1, 0, 1);
        k.put(0, 0, maker.building, E, { label: b(maker.building).name, focus: focus === 'maker' });
      } else if (maker?.kind === 'craft') {
        const r = b(maker.building).recipes[maker.recipe];
        k.put(0, 0, maker.building, E, { recipe: maker.recipe, label: b(maker.building).name, focus: focus === 'maker' });
        r.inputs.forEach(([i], s) => feed(k, c, 0, 0, FEED_SIDES[s], i, 'left'));
      }
      // The item on its way.
      if (made && use) {
        k.path([
          [1, 0],
          [3, 0],
        ]);
        k.label(2, 0, name, { at: 'above', icon: item });
      } else if (made) {
        k.exit(0, 0, 'e');
        k.label(1.5, 0, name, { at: 'above', icon: item });
      } else if (use) {
        feed(k, c, ux, 0, 'w', item);
      }
      // The user.
      if (use?.kind === 'craft' && userRecipe) {
        k.put(ux, 0, use.building, E, { recipe: use.recipe, label: b(use.building).name, focus: focus === 'user' });
        const others = userRecipe.inputs.map(([i]) => i).filter((i) => i !== item);
        others.forEach((i, s) => feed(k, c, ux, 0, s === 0 ? 'n' : 's', i));
        k.exit(ux, 0, 'e');
        const out = userRecipe.output[0];
        k.label(ux + 1.5, 0, c.items[out].name, { at: 'above', icon: out });
      } else if (use?.kind === 'terraform') {
        const t = b(use.building);
        if (t.key === 'hatchery') k.pond(ux + 1, -1, ux + 2, 1);
        k.put(ux, 0, use.building, E, { label: t.name, focus: focus === 'user' });
        const others = t.recipes[0].inputs.map(([i]) => i).filter((i) => i !== item);
        others.forEach((i, s) => feed(k, c, ux, 0, s === 0 ? 'n' : 's', i));
        k.watch({ kind: 'meter', x: ux, y: 0, meter: t.meter });
      } else if (use?.kind === 'core') {
        k.watch({ kind: 'delivered', x: ux + 1.5, y: 1 });
      }
      k.view(made ? -2 : ux - 3, top, right, -top);
    },
  };
}

/** A supply belted in from off-screen, labelled with what it carries `dist` tiles up the belt
 * (vertical belts get their label on `beside`). */
function feed(
  k: SceneCtx,
  c: Content,
  x: number,
  y: number,
  side: Side,
  item: number,
  beside: 'left' | 'right' = 'right',
  dist = 2,
): void {
  k.feed(x, y, side, item);
  const name = c.items[item].name;
  if (side === 'w') k.label(x - dist, y, name, { at: 'above', icon: item });
  else if (side === 'e') k.label(x + dist, y, name, { at: 'above', icon: item });
  else k.label(x, y + (side === 'n' ? -dist : dist), name, { at: beside, icon: item });
}

// ---- Hand-made scenes ------------------------------------------------------------------------------

/** A generator powering a small line through two poles; one machine is out of reach. */
function power(c: Content, gen: number, battery = false): Scene {
  const K = keys(c);
  const g = c.buildings[gen];
  const fuel = g.fuel[0];
  const drill = K.kind('drill');
  const pole = K.kind('pole');
  const iron = K.item('iron_ore');
  return {
    caption: `${fuel ? `The ${g.name} burns ${c.items[fuel].name}` : `${g.name}s catch the sun`}; Power Poles carry the power ${c.poleRadius} tiles around them and link up to ${c.poleLink} tiles apart. The drill below is out of reach, so it has no power.`,
    grid: true,
    overlay: true,
    badges: true,
    warmup: 600,
    build(k) {
      if (fuel) {
        k.put(0, 0, gen, E, { label: g.name, focus: !battery });
        feed(k, c, 0, 0, 'w', fuel);
      } else {
        k.put(0, -1, gen, E);
        k.put(0, 0, gen, E, { focus: true });
        k.put(0, 1, gen, E, { label: g.name });
      }
      if (battery) {
        k.put(1, 2, K.kind('battery_bank'), E, { label: 'Battery Bank', focus: true });
        k.watch({ kind: 'battery' });
      }
      k.put(2, -2, pole, E, { label: 'Pole' });
      k.put(8, -2, pole, E, { label: 'Pole' });
      k.ore(5, 0, 5, 1, iron);
      k.put(5, 0, drill, E, { label: 'Drill' });
      k.path([
        [6, 0],
        [7, 0],
      ]);
      k.put(8, 0, K.kind('smelter'), E, { label: 'Smelter' });
      k.exit(8, 0, 'e');
      k.ore(5, 4, 5, 5, iron);
      k.put(5, 4, drill, E, { label: 'No power' });
      k.exit(5, 4, 'e');
      k.watch({ kind: 'power' });
      k.view(-2, -3, 10, 5);
    },
  };
}

/** Three drills on the same ore: plain, overclocked, amplified. */
function boost(c: Content): Scene {
  const K = keys(c);
  const drill = K.kind('drill');
  const iron = K.item('iron_ore');
  return {
    caption: 'Three drills on the same ore: as built, overclocked with 3 power shards (250% speed, more power), and with an amplifier (twice the output, four times the power).',
    warmup: 240,
    build(k) {
      const rows: [number, string][] = [
        [-2, '100%'],
        [0, '250% · 3 shards'],
        [2, '×2 · amplifier'],
      ];
      for (const [y, label] of rows) {
        k.ore(-1, y, 0, y, iron);
        k.put(0, y, drill, E, { label });
        k.exit(0, y, 'e');
      }
      k.shards(0, 0, 3);
      k.amp(0, 2);
      k.view(-2, -3, 6, 3);
    },
  };
}

function belts(c: Content): Scene {
  const K = keys(c);
  const drill = K.kind('drill');
  return {
    caption: 'Belts carry items toward their arrows. A belt that ends against the side of another merges into it, and corners are automatic when you drag.',
    warmup: 300,
    build(k) {
      // Two drills, so both lines have gaps to merge into.
      k.ore(-4, -1, -3, 0, K.item('iron_ore'));
      k.put(-3, 0, drill, E);
      k.path(
        [
          [-2, 0],
          [4, 0],
          [4, 2],
        ],
        S,
      );
      k.ore(0, -4, 1, -3, K.item('copper_ore'));
      k.put(1, -3, drill, S);
      k.path([
        [1, -2],
        [1, -1],
      ]);
      k.exit(4, 2, 's');
      k.label(1, -1, 'merges', { at: 'left' });
      k.label(4, 0, 'corner', { at: 'right' });
      k.view(-5, -4, 6, 3);
    },
  };
}

function machines(c: Content): Scene {
  const K = keys(c);
  const asm = K.kind('assembler');
  const motor = K.item('motor');
  const recipe = Math.max(0, c.buildings[asm].recipes.findIndex((r) => r.output[0] === motor && !isAlternate(r)));
  return {
    caption: 'Machines take inputs through the back and sides and put the product out the front, where the arrow points. This Assembler turns gears and wire into motors.',
    speed: 200,
    warmup: 600,
    build(k) {
      k.put(0, 0, asm, E, { recipe, label: 'Assembler', focus: true });
      feed(k, c, 0, 0, 'w', K.item('gear'));
      feed(k, c, 0, 0, 'n', K.item('copper_wire'));
      k.exit(0, 0, 'e');
      k.label(1.5, 0, 'Motor', { at: 'above', icon: motor });
      k.view(-3, -3, 3, 2);
    },
  };
}

function splitter(c: Content): Scene {
  const K = keys(c);
  const smelter = K.kind('smelter');
  return {
    caption: 'A Splitter deals what comes in from behind to its front, left and right in turn: one ore line feeds three smelters.',
    warmup: 480,
    build(k) {
      k.put(0, 0, K.kind('splitter'), E, { label: 'Splitter', focus: true });
      feed(k, c, 0, 0, 'w', K.item('iron_ore'));
      k.path([
        [1, 0],
        [2, 0],
      ]);
      k.path([
        [0, -1],
        [0, -2],
        [2, -2],
      ]);
      k.path([
        [0, 1],
        [0, 2],
        [2, 2],
      ]);
      for (const y of [-2, 0, 2]) {
        k.put(3, y, smelter, E);
        k.exit(3, y, 'e');
      }
      k.label(3, 2, 'Smelters');
      k.view(-3, -3, 5, 3);
    },
  };
}

function sorter(c: Content): Scene {
  const K = keys(c);
  const drill = K.kind('drill');
  const copper = K.item('copper_ore');
  return {
    caption: 'A Sorter sends its chosen item straight on (copper ore here) and everything else to the sides.',
    warmup: 360,
    build(k) {
      // A mixed belt: iron and copper drills share one line into the sorter.
      k.ore(-5, -1, -4, 0, K.item('iron_ore'));
      k.put(-4, 0, drill, E);
      k.path([
        [-3, 0],
        [-1, 0],
      ]);
      k.ore(-3, -3, -2, -2, copper);
      k.put(-2, -2, drill, S);
      k.path([[-2, -1]], S);
      k.put(0, 0, K.kind('sorter'), E, { filter: copper, label: 'Sorter', focus: true });
      k.exit(0, 0, 'e');
      k.exit(0, 0, 'n');
      k.exit(0, 0, 's');
      k.label(1.5, 0, 'Copper Ore', { at: 'above', icon: copper });
      k.label(0, 1.5, 'the rest', { at: 'right' });
      k.view(-6, -3, 3, 3);
    },
  };
}

function tunnel(c: Content): Scene {
  const K = keys(c);
  const kind = K.kind('tunnel');
  return {
    caption: 'Tunnels carry items under whatever is in the way: place an entrance, then an exit facing the same way up to 5 tiles ahead.',
    warmup: 360,
    build(k) {
      k.put(0, 0, kind, E, { label: 'Entrance', focus: true });
      feed(k, c, 0, 0, 'w', K.item('iron_ore'));
      k.put(3, 0, kind, E, { label: 'Exit' });
      k.exit(3, 0, 'e');
      k.path(
        [
          [1, -1],
          [1, 1],
        ],
        S,
      );
      k.feed(1, -1, 'n', K.item('copper_ore'));
      k.exit(1, 1, 's');
      k.path(
        [
          [2, 1],
          [2, -1],
        ],
        N,
      );
      k.feed(2, 1, 's', K.item('coal'));
      k.exit(2, -1, 'n');
      k.view(-3, -2, 5, 2);
    },
  };
}

function storage(c: Content): Scene {
  const K = keys(c);
  const asm = K.kind('assembler');
  const gear = K.item('gear');
  const recipe = Math.max(0, c.buildings[asm].recipes.findIndex((r) => r.output[0] === gear && !isAlternate(r)));
  return {
    caption: 'Storage buffers what arrives faster than the next machine can use it, and passes it on through its front.',
    warmup: 240,
    build(k) {
      k.put(0, 0, K.kind('storage'), E, { label: 'Storage', focus: true });
      feed(k, c, 0, 0, 'w', K.item('iron_plate'));
      k.path([
        [1, 0],
        [2, 0],
      ]);
      k.put(3, 0, asm, E, { recipe, label: 'Assembler' });
      k.exit(3, 0, 'e');
      k.watch({ kind: 'stored', x: 0, y: 0 });
      k.view(-3, -2, 5, 2);
    },
  };
}

function sink(c: Content, key: string, item: string, caption: string): Scene {
  const K = keys(c);
  const kind = K.kind(key);
  return {
    caption,
    warmup: 240,
    build(k) {
      k.put(0, 0, kind, E, { label: c.buildings[kind].name, focus: true });
      feed(k, c, 0, 0, 'w', K.item(item));
      if (key === 'recycler') k.watch({ kind: 'credits', x: 0, y: 0 });
      k.view(-4, -2, 2, 2);
    },
  };
}

function drones(c: Content): Scene {
  const K = keys(c);
  const port = K.kind('drone_port');
  return {
    caption: 'Link a Drone Port to another (inspector → Link) and its drone flies whatever is fed in across the map, over anything in the way.',
    warmup: 300,
    build(k) {
      k.put(0, 0, port, E, { label: 'Sends', focus: true });
      feed(k, c, 0, 0, 'w', K.item('iron_plate'));
      k.put(9, 0, port, E, { label: 'Receives' });
      k.exit(9, 0, 'e');
      k.link(0, 0, 9, 0);
      k.view(-3, -2, 11, 2);
    },
  };
}

function radar(c: Content): Scene {
  const K = keys(c);
  return {
    caption: 'Building pushes back the fog. A Radar keeps widening what you can see, up to 70 tiles; wrecks and deposits appear as the land is revealed.',
    fog: true,
    speed: 50,
    warmup: 0,
    loop: 16,
    build(k) {
      k.put(0, 0, K.kind('radar'), E, { label: 'Radar', focus: true });
      for (const [x, y] of [
        [-9, -3],
        [8, 4],
        [-6, 5],
        [10, -4],
      ])
        k.wreck(x, y);
      k.ore(5, -4, 6, -3, K.item('copper_ore'));
      k.ore(-9, 2, -8, 3, K.item('coal'));
      k.view(-13, -6, 13, 6);
    },
  };
}

function terraform(c: Content): Scene {
  const K = keys(c);
  const heater = K.kind('heater');
  const vapor = K.kind('vaporizer');
  return {
    caption: 'Terraformers burn goods to raise the planet’s meters: a Heater burns coal for Heat, a Vaporizer boils ice into the sky for Pressure.',
    speed: 200,
    warmup: 300,
    build(k) {
      k.put(0, -1, heater, E, { label: 'Heater' });
      feed(k, c, 0, -1, 'w', K.item('coal'));
      k.watch({ kind: 'meter', x: 0, y: -1, meter: c.buildings[heater].meter });
      k.put(0, 2, vapor, E, { label: 'Vaporizer' });
      feed(k, c, 0, 2, 'w', K.item('ice'));
      k.watch({ kind: 'meter', x: 0, y: 2, meter: c.buildings[vapor].meter });
      k.view(-4, -3, 3, 4);
    },
  };
}

/** The first lesson, built step by step: drill, belts, smelter, Core. */
function firstSteps(c: Content): Scene {
  const K = keys(c);
  const belt = (x: number) => (k: SceneCtx) => k.path([[x, 0]], E);
  return {
    caption: 'An iron deposit near your Core.',
    core: [3, -2],
    warmup: 0,
    loop: 26,
    build(k) {
      k.ore(-6, -1, -5, 1, K.item('iron_ore'));
      k.label(-5.5, 1, 'Iron deposit');
      k.watch({ kind: 'delivered', x: 4.5, y: 1 });
      k.view(-7, -3, 7, 3);
    },
    steps: [
      {
        at: 1.6,
        caption: '1 · A Drill mines the tile under it and outputs toward its arrow.',
        run: (k) => k.put(-5, 0, K.kind('drill'), E, { label: 'Drill' }),
      },
      { at: 3.4, caption: '2 · Drag belts from the drill: they carry the ore.', run: belt(-4) },
      { at: 3.7, run: belt(-3) },
      {
        at: 5.2,
        caption: '3 · A Smelter turns the ore into iron ingots.',
        run: (k) => k.put(-2, 0, K.kind('smelter'), E, { label: 'Smelter' }),
      },
      { at: 7, caption: '4 · Belt the ingots into the Core. They pay for buildings and research.', run: belt(-1) },
      { at: 7.25, run: belt(0) },
      { at: 7.5, run: belt(1) },
      { at: 7.75, run: belt(2) },
      { at: 12, caption: 'That’s a factory. Now make it bigger: more drills, more smelters, new machines.' },
    ],
  };
}

// ---- Registry -------------------------------------------------------------------------------------

/** The scene for a building kind. */
function building(c: Content, kind: number): Scene | null {
  const b = c.buildings[kind];
  const K = keys(c);
  switch (b.class) {
    case 'belt':
      return belts(c);
    case 'core':
      return chain(c, K.item('iron_ingot'), makers(c, K.item('iron_ingot'))[0], { kind: 'core' });
    case 'drill':
      return chain(c, b.deposits[0], { kind: 'mine', building: kind }, undefined, 'maker');
    case 'pump':
      return chain(c, K.item('water'), { kind: 'pump', building: kind }, undefined, 'maker');
    case 'crafter': {
      const r = b.recipes.findIndex((rc) => !isAlternate(rc));
      return chain(c, b.recipes[r].output[0], { kind: 'craft', building: kind, recipe: r }, undefined, 'maker');
    }
    case 'terraformer':
      return chain(c, b.recipes[0].inputs[0][0], undefined, { kind: 'terraform', building: kind }, 'user');
    case 'splitter':
      return splitter(c);
    case 'sorter':
      return sorter(c);
    case 'tunnel':
      return tunnel(c);
    case 'storage':
      return storage(c);
    case 'incinerator':
      return sink(c, b.key, 'stone', 'An Incinerator destroys whatever is fed into it: a way out for surplus that would clog a line.');
    case 'recycler':
      return sink(c, b.key, 'gear', 'A Recycler breaks down whatever is fed in for credits (worth more for advanced parts), spent in the shop.');
    case 'drone':
      return drones(c);
    case 'radar':
      return radar(c);
    case 'pole':
      return power(c, K.kind('coal_gen'));
    case 'generator':
      return power(c, kind);
    case 'battery':
      return power(c, K.kind('coal_gen'), true);
    default:
      return null;
  }
}

/** Scenes for guide topics (some topics have several). */
export const TOPIC_SCENES: Record<string, { label: string; make: (c: Content) => Scene }[]> = {
  start: [{ label: 'First factory', make: firstSteps }],
  belts: [{ label: 'Belts', make: belts }],
  machines: [{ label: 'Machines', make: machines }],
  logistics: [
    { label: 'Splitter', make: splitter },
    { label: 'Sorter', make: sorter },
    { label: 'Tunnel', make: tunnel },
    { label: 'Storage', make: storage },
  ],
  power: [
    { label: 'Power grid', make: (c) => power(c, keys(c).kind('coal_gen')) },
    { label: 'Batteries', make: (c) => power(c, keys(c).kind('coal_gen'), true) },
  ],
  research: [
    {
      label: 'Deliver to the Core',
      make: (c) => {
        const plate = keys(c).item('iron_plate');
        return chain(c, plate, makers(c, plate)[0], { kind: 'core' });
      },
    },
  ],
  terraform: [{ label: 'Terraformers', make: terraform }],
  explore: [{ label: 'Radar and wrecks', make: radar }],
  boost: [{ label: 'Shards and amplifiers', make: boost }],
  drones: [{ label: 'Drones', make: drones }],
};

/** Builds the scene for an id (see the top of this file), or null. */
export function sceneFor(c: Content, id: string): Scene | null {
  const [kind, a, b] = id.split(':');
  const n = Number(a);
  const k = Number(b);
  switch (kind) {
    case 'item': {
      if (!c.items[n] || n === 0) return null;
      const m = makers(c, n);
      const u = uses(c, n);
      return chain(c, n, m[0].kind === 'find' ? undefined : m[0], u[0]);
    }
    case 'make': {
      const m = makers(c, n)[k];
      return m ? chain(c, n, m, uses(c, n)[0], 'maker') : null;
    }
    case 'use': {
      const m = makers(c, n)[0];
      const u = uses(c, n)[k];
      return u ? chain(c, n, m.kind === 'find' ? undefined : m, u, 'user') : null;
    }
    case 'bld':
      return c.buildings[n] ? building(c, n) : null;
    case 'topic':
      return TOPIC_SCENES[a]?.[Number.isFinite(k) ? k : 0]?.make(c) ?? null;
    default:
      return null;
  }
}

/** Whether a building has a demo (everything placeable does). */
export function hasBuildingScene(c: Content, kind: number): boolean {
  return building(c, kind) !== null;
}

