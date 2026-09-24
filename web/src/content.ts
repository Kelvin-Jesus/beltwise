// Typed view of the static game content the engine exports as JSON (engine/src/content.rs).

export type Stack = [item: number, count: number];

export interface ItemDef {
  key: string;
  name: string;
  /** Credits a Recycler pays for one. */
  value: number;
}

export interface RecipeDef {
  inputs: Stack[];
  output: Stack;
  ticks: number;
  /** 255 = free, < 128 = tech id, >= 128 = alternate (128 + index). */
  unlock: number;
  /** Alternates have names; standard recipes are named after their output. */
  name: string;
}

export type BuildingClass =
  | 'none'
  | 'belt'
  | 'core'
  | 'drill'
  | 'crafter'
  | 'terraformer'
  | 'splitter'
  | 'tunnel'
  | 'incinerator'
  | 'pole'
  | 'generator'
  | 'battery'
  | 'sorter'
  | 'storage'
  | 'drone'
  | 'radar'
  | 'recycler'
  | 'pump'
  | 'wreck';

export interface BuildingDef {
  key: string;
  name: string;
  desc: string;
  class: BuildingClass;
  category: number;
  meter: number;
  points: number;
  /** Tech that unlocks it, or 255 when free. */
  research: number;
  /** Consumers: kW drawn while working. Generators: kW produced. Batteries: kW moved. */
  power: number;
  /** Generators: fuel item and kJ per item. */
  fuel: Stack;
  /** Batteries: capacity in kJ. */
  store: number;
  cost: Stack[];
  deposits: number[];
  recipes: RecipeDef[];
}

export interface TechDef {
  key: string;
  name: string;
  desc: string;
  stage: number;
  tier: number;
  repeat: boolean;
  cost: Stack[];
  requires: number[];
  unlocks: number[];
  /** Recipes this node unlocks, as [building, recipe index]. */
  recipes: [number, number][];
}

export interface StageDef {
  name: string;
  desc: string;
  ti: number;
}

export interface MeterDef {
  name: string;
  full: number;
}

export interface ObjectiveDef {
  text: string;
  kind: 'build' | 'deliver' | 'research' | 'ti' | 'stage' | 'ark' | 'salvage';
  a: number;
  b: number;
  reward: Stack[];
}

export interface ArkDef {
  name: string;
  desc: string;
  cost: Stack[];
}

export interface AchievementDef {
  key: string;
  name: string;
  desc: string;
  credits: number;
  shards: number;
}

export interface LogDef {
  title: string;
  text: string;
}

export interface ShopDef {
  key: string;
  name: string;
  desc: string;
  price: number;
  kind: 'item' | 'cosmetic';
  /** Item id or cosmetic bit. */
  a: number;
  b: number;
}

export interface PlanetDef {
  key: string;
  name: string;
  desc: string;
  unlock: number;
}

export interface Content {
  items: ItemDef[];
  buildings: BuildingDef[];
  tech: TechDef[];
  ark: ArkDef[];
  stages: StageDef[];
  meters: MeterDef[];
  objectives: ObjectiveDef[];
  achievements: AchievementDef[];
  logs: LogDef[];
  /** Alternate recipes in unlock order, as [building, recipe index]. */
  alternates: [number, number][];
  shop: ShopDef[];
  planets: PlanetDef[];
  purity: string[];
  beltSpeeds: number[];
  storageCap: number;
  poleRadius: number;
  poleLink: number;
  coreRadius: number;
  corePower: number;
  clockPct: number[];
  legacyPct: number;
}

export const FREE = 255;
export const ALT_BASE = 128;
export const HIDDEN_CATEGORY = 255;

export const CATEGORIES = [
  { name: 'Logistics', accent: '#94a3b8' },
  { name: 'Extraction', accent: '#f59e0b' },
  { name: 'Manufacturing', accent: '#38bdf8' },
  { name: 'Power', accent: '#facc15' },
  { name: 'Terraforming', accent: '#34d399' },
] as const;

export const METER_COLORS = ['#fb923c', '#a78bfa', '#38bdf8', '#4ade80'];

/** Belt stripe colours, by cosmetic index. */
export const BELT_COLORS = ['#525a66', '#f59e0b', '#2dd4bf', '#a78bfa', '#f43f5e'];

/** Compact number formatting: 1234 -> "1.23k", 25000000 -> "25.0M". */
export function formatCount(n: number): string {
  if (n < 1000) return String(Math.floor(n));
  const units = ['k', 'M', 'B', 'T', 'P'];
  let i = -1;
  while (n >= 1000 && i < units.length - 1) {
    n /= 1000;
    i++;
  }
  return `${n.toFixed(n < 10 ? 2 : n < 100 ? 1 : 0)}${units[i]}`;
}

/** Power in kW as a short string: "850 kW", "38 MW", "1.2 GW". */
export function formatPower(kw: number): string {
  if (kw < 1000) return `${Math.round(kw)} kW`;
  if (kw < 1_000_000) return `${(kw / 1000).toFixed(kw < 10_000 ? 1 : 0)} MW`;
  return `${(kw / 1_000_000).toFixed(2)} GW`;
}

/** Display name of a recipe: the alternate's name or the output item's. */
export function recipeName(content: Content, r: RecipeDef): string {
  return r.name || content.items[r.output[0]]?.name || 'Terraform';
}
