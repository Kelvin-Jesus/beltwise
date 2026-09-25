//! The world: tile grid (structure-of-arrays), placement rules, undo, research and the
//! fixed-step tick that drives every subsystem.

use crate::belts::Belts;
use crate::blueprint::BpCell;
use crate::content::{
    ACHIEVEMENTS, ALT_BASE, BELT_SPEEDS, BUILDING_COUNT, BUILDINGS, CORE_POWER, CRAFT_PCT, Class, Cond,
    DRILL_PCT, Effect, FREE, ITEM_COUNT, LEGACY_PCT, METER_FULL, POLE_RADIUS, REPEAT_FIRST, TECH, TECH_COUNT,
    bk, goal_parts, it, meter,
};
use crate::core::Core;
use crate::machines::{ENTRANCE, EXIT, FULL, Machine, Machines, NO_NET, NO_RECIPE, PASSIVE, flag};
use crate::render::{Cursor, INSTANCE_CAP, Instance};
use crate::sky::{self, Sky};
use crate::types::{NONE, TICKS_PER_SEC, TUNNEL_RANGE, dir};
use crate::worldgen;

pub const CORE_SIZE: i32 = 4;
pub const STATS_LEN: usize = 96;
/// Fog radius revealed at the landing site.
const LANDING_SIGHT: i32 = 22;

/// Indices into the stats block exported to JS (all `u32`; 64-bit values are split lo/hi).
pub mod stat {
    pub const TICK: usize = 0;
    pub const DELIVERED: usize = 1;
    pub const RATE: usize = 2;
    pub const STAGE: usize = 3;
    pub const TI_LO: usize = 4;
    pub const TI_HI: usize = 5;
    pub const TI_RATE_X100: usize = 6;
    /// Four meters, lo/hi each.
    pub const METERS: usize = 7;
    /// Four meter rates per second, x100.
    pub const METER_RATES: usize = 15;
    pub const OBJECTIVE: usize = 19;
    pub const GOAL_HAVE_LO: usize = 20;
    pub const GOAL_HAVE_HI: usize = 21;
    pub const GOAL_NEED_LO: usize = 22;
    pub const GOAL_NEED_HI: usize = 23;
    pub const ITEMS: usize = 24;
    pub const BELTS: usize = 25;
    pub const SEGMENTS: usize = 26;
    pub const MACHINES: usize = 27;
    pub const INSTANCES: usize = 28;
    pub const BELT_SPEED: usize = 29;
    pub const CORE_X: usize = 30;
    pub const CORE_Y: usize = 31;
    pub const CORE_SIZE: usize = 32;
    pub const RESEARCHED_LO: usize = 33;
    pub const RESEARCHED_HI: usize = 34;
    pub const REVISION: usize = 35;
    pub const UNDO_DEPTH: usize = 36;
    pub const GOAL_KIND: usize = 37;
    pub const GOAL_A: usize = 38;
    pub const MAP_REV: usize = 39;
    pub const ARK_PHASE: usize = 40;
    pub const CREDITS_LO: usize = 41;
    pub const CREDITS_HI: usize = 42;
    /// Power across every network (kW): generator capacity online, and demand.
    pub const POWER_SUPPLY: usize = 43;
    pub const POWER_DEMAND: usize = 44;
    /// Worst satisfaction among networks with demand (x1000).
    pub const POWER_SAT: usize = 45;
    pub const BATTERY_MJ: usize = 46;
    pub const BATTERY_CAP_MJ: usize = 47;
    pub const POWER_REV: usize = 48;
    pub const FOG_REV: usize = 49;
    /// Tiles revealed since JS last acknowledged: x0, y0, x1, y1 (exclusive).
    pub const FOG_DIRTY: usize = 50;
    pub const RES_REV: usize = 54;
    /// Sunlight 0..=1000.
    pub const DAYLIGHT: usize = 55;
    /// Ticks until the next meteor shower, while one is announced; 0 otherwise.
    pub const SHOWER_IN: usize = 56;
    pub const SHOWER_ACTIVE: usize = 57;
    pub const ACH_LO: usize = 58;
    pub const ACH_HI: usize = 59;
    pub const PROBES: usize = 60;
    pub const ALTS: usize = 61;
    pub const LOGS: usize = 62;
    pub const SALVAGED: usize = 63;
    pub const LEGACY: usize = 64;
    pub const PLANET: usize = 65;
    pub const COSMETICS: usize = 66;
    pub const BELT_COLOR: usize = 67;
    pub const TRIM: usize = 68;
    /// Repeatable research levels (four).
    pub const LEVELS: usize = 69;
    pub const METEORS: usize = 73;
    pub const NETS: usize = 74;
    pub const DRONES_FLYING: usize = 75;
    pub const WATER_X1000: usize = 76;
    pub const ARK_PAID: usize = 77;
    pub const LAST_ACH: usize = 81;
    pub const BP_CELLS: usize = 82;
}

/// Why a placement is refused (0 = allowed).
pub mod reason {
    pub const OK: u8 = 0;
    pub const BOUNDS: u8 = 1;
    pub const LOCKED: u8 = 2;
    pub const OCCUPIED: u8 = 3;
    pub const NO_DEPOSIT: u8 = 4;
    pub const DEPOSIT_LOCKED: u8 = 5;
    pub const COST: u8 = 6;
    pub const FOG: u8 = 7;
    pub const LAVA: u8 = 8;
    pub const NO_WATER: u8 = 9;
    pub const WRONG_DEPOSIT: u8 = 10;
}

/// Tile layers, one flat array per attribute.
pub struct Grid {
    pub w: i32,
    pub h: i32,
    pub kind: Vec<u8>,
    pub dir: Vec<u8>,
    /// Deposit under this tile (an item id, 0 = none).
    pub res: Vec<u8>,
    /// Deposit purity (0 impure, 1 normal, 2 pure).
    pub purity: Vec<u8>,
    /// Machine index for machine tiles.
    pub ent: Vec<u32>,
    /// Fog of war: 255 once explored.
    pub seen: Vec<u8>,
}

impl Grid {
    pub fn new(w: i32, h: i32) -> Self {
        let n = (w * h) as usize;
        Grid {
            w,
            h,
            kind: vec![bk::EMPTY; n],
            dir: vec![0; n],
            res: vec![0; n],
            purity: vec![1; n],
            ent: vec![NONE; n],
            seen: vec![0; n],
        }
    }

    #[inline(always)]
    pub fn index(&self, x: i32, y: i32) -> u32 {
        if x >= 0 && y >= 0 && x < self.w && y < self.h { (y * self.w + x) as u32 } else { NONE }
    }

    /// Neighbour of tile `t` in direction `d`, or `NONE` past the map edge.
    #[inline(always)]
    pub fn step(&self, t: u32, d: u8) -> u32 {
        let w = self.w as u32;
        self.index((t % w) as i32 + dir::dx(d), (t / w) as i32 + dir::dy(d))
    }

    #[inline(always)]
    pub fn xy(&self, t: u32) -> (i32, i32) {
        ((t % self.w as u32) as i32, (t / self.w as u32) as i32)
    }
}

#[derive(Clone, Copy, Debug)]
struct Op {
    tile: u32,
    before: (u8, u8),
}

/// Edit history, grouped by gesture.
#[derive(Default)]
pub struct Undo {
    ops: Vec<Op>,
    groups: Vec<usize>,
    pub replaying: bool,
}

const UNDO_GROUPS: usize = 64;
const RATE_WINDOW: usize = 60;

/// Per-item production, consumption and delivery over a sliding minute.
pub struct Rates {
    hist: Vec<[u32; ITEM_COUNT * 3]>,
    pos: usize,
    filled: u32,
    sums: [u32; ITEM_COUNT * 3],
    /// Per minute: produced, consumed, delivered (ITEM_COUNT each). Exported to JS.
    pub per_min: [u32; ITEM_COUNT * 3],
}

impl Default for Rates {
    fn default() -> Self {
        Rates {
            hist: vec![[0; ITEM_COUNT * 3]; RATE_WINDOW],
            pos: 0,
            filled: 0,
            sums: [0; ITEM_COUNT * 3],
            per_min: [0; ITEM_COUNT * 3],
        }
    }
}

impl Rates {
    pub fn has_data(&self) -> bool {
        self.filled > 0
    }

    fn push(&mut self, second: &[u32; ITEM_COUNT * 3]) {
        let old = &self.hist[self.pos];
        for k in 0..ITEM_COUNT * 3 {
            self.sums[k] = self.sums[k] - old[k] + second[k];
        }
        self.hist[self.pos] = *second;
        self.pos = (self.pos + 1) % RATE_WINDOW;
        self.filled = (self.filled + 1).min(RATE_WINDOW as u32);
        // Extrapolate while the window fills, but never from less than 10 seconds.
        let secs = self.filled.max(10);
        for k in 0..ITEM_COUNT * 3 {
            self.per_min[k] = self.sums[k] * 60 / secs;
        }
    }
}

pub struct World {
    pub grid: Grid,
    pub belts: Belts,
    pub machines: Machines,
    pub core: Core,
    pub tick: u32,
    pub seed: u32,
    pub planet: u8,
    /// RGBA per tile: elevation, moisture, detail (static, for the ground shader).
    pub terrain: Vec<u8>,
    pub instances: Vec<Instance>,
    pub cursor: Cursor,
    pub stats: [u32; STATS_LEN],
    /// Ignore costs, research and fog (benchmarks and tests).
    pub sandbox: bool,
    /// Bumped on structural edits (JS refreshes its tile-map texture).
    pub map_rev: u32,
    /// Bumped when deposits change (meteor impacts).
    pub res_rev: u32,
    pub fog_rev: u32,
    pub fog_dirty: [i32; 4],
    /// Power network per tile, and a 0/1 coverage map for the overlay.
    pub power_cov: Vec<u16>,
    pub power_map: Vec<u8>,
    pub power_dirty: bool,
    pub power_rev: u32,
    pub core_net: u16,
    /// Radar tiles (few, so the per-tick scan stays tiny).
    pub radars: Vec<u32>,
    pub sky: Sky,
    pub rates: Rates,
    /// Delivery and meter rates recorded at the last save, for offline progress.
    pub offline_items: [u32; ITEM_COUNT],
    pub offline_meters: [u64; meter::COUNT],
    /// Scratch lists for the renderer (kept to avoid per-frame allocation).
    pub visible_machines: Vec<u32>,
    pub glows: Vec<(f32, f32, u32, u8)>,
    pub blueprint: Vec<BpCell>,
    /// Recipe and sorter filter applied to buildings placed from now on.
    pub place_recipe: u8,
    pub place_filter: u8,
    /// Results for JS (salvage rewards, offline summary).
    pub report: Vec<u32>,
    /// Launch animation progress (0..=255), set by JS while the Ark lifts off.
    pub launch: u8,
    pub undo: Undo,
}

impl World {
    pub fn new(w: i32, h: i32, seed: u32) -> World {
        World::new_planet(w, h, seed, 0, 0)
    }

    pub fn new_planet(w: i32, h: i32, seed: u32, planet: u8, legacy: u32) -> World {
        let planet = planet.min(crate::content::PLANETS.len() as u8 - 1);
        let mut grid = Grid::new(w, h);
        let (cx, cy) = (w / 2 - CORE_SIZE / 2, h / 2 - CORE_SIZE / 2);
        for y in cy..cy + CORE_SIZE {
            for x in cx..cx + CORE_SIZE {
                let t = grid.index(x, y) as usize;
                grid.kind[t] = bk::CORE;
            }
        }
        let mut terrain = vec![0; (w * h * 4) as usize];
        let wrecks = worldgen::generate(&mut grid, &mut terrain, seed, planet, cx, cy, CORE_SIZE);
        let n = grid.kind.len();
        let mut world = World {
            belts: Belts::new(n),
            grid,
            machines: Machines::default(),
            core: Core::new(cx, cy, CORE_SIZE),
            tick: 0,
            seed,
            planet,
            terrain,
            instances: Vec::with_capacity(INSTANCE_CAP),
            cursor: Cursor::default(),
            stats: [0; STATS_LEN],
            sandbox: false,
            map_rev: 0,
            res_rev: 0,
            fog_rev: 0,
            fog_dirty: [i32::MAX, i32::MAX, 0, 0],
            power_cov: vec![NO_NET; n],
            power_map: vec![0; n],
            power_dirty: true,
            power_rev: 0,
            core_net: NO_NET,
            radars: Vec::new(),
            sky: Sky::new(seed),
            rates: Rates::default(),
            offline_items: [0; ITEM_COUNT],
            offline_meters: [0; meter::COUNT],
            visible_machines: Vec::new(),
            glows: Vec::new(),
            blueprint: Vec::new(),
            place_recipe: NO_RECIPE,
            place_filter: 0,
            report: Vec::new(),
            launch: 0,
            undo: Undo::default(),
        };
        world.core.legacy = legacy;
        world.core.planet = planet;
        let start_heat = crate::content::PLANETS[planet as usize].start_heat;
        if start_heat > 0 {
            world.core.set_meters([start_heat, 0, 0, 0]);
        }
        for (t, contents) in wrecks {
            let mut m = Machine::new(bk::WRECK, 0, t);
            m.aux = contents as u32;
            world.grid.kind[t as usize] = bk::WRECK;
            let i = world.machines.add(m);
            world.grid.ent[t as usize] = i;
        }
        world.reveal(cx + CORE_SIZE / 2, cy + CORE_SIZE / 2, LANDING_SIGHT);
        world.retune();
        world.core.begin_goal();
        world.update_stats();
        world
    }

    // ---- Terrain queries --------------------------------------------------------------------

    #[inline]
    pub fn elevation(&self, t: u32) -> f32 {
        self.terrain[t as usize * 4] as f32 / 255.0
    }

    /// Height of the lakes, mirroring the ground shader: rises with pressure, faster once
    /// the planet is warm.
    pub fn water_level(&self) -> f32 {
        let lvl = |v: u64, full: u64| {
            (((1.0 + v as f32).log10() - 2.0) / ((full as f32).log10() - 2.0)).clamp(0.0, 1.0)
        };
        let heat = lvl(self.core.meters[0], METER_FULL[0]);
        let water = lvl(self.core.meters[1], METER_FULL[1]) * (0.35 + 0.65 * heat);
        -0.08 + 0.5 * water
    }

    pub fn is_water(&self, t: u32) -> bool {
        self.elevation(t) < self.water_level() - 0.02
    }

    /// Ember's volcanic highlands can't be built on.
    pub fn is_lava(&self, t: u32) -> bool {
        self.planet == 1 && self.elevation(t) > worldgen::LAVA_LINE
    }

    // ---- Editing ------------------------------------------------------------------------

    /// Whether `k` could be placed at (x, y); see [`reason`].
    pub fn check_place(&self, x: i32, y: i32, k: u8) -> u8 {
        let t = self.grid.index(x, y);
        if t == NONE {
            return reason::BOUNDS;
        }
        if matches!(k, bk::EMPTY | bk::CORE | bk::WRECK) || k as usize >= BUILDING_COUNT {
            return reason::LOCKED;
        }
        let ti = t as usize;
        let def = &BUILDINGS[k as usize];
        if !self.sandbox && !self.core.is_unlocked(def.research) {
            return reason::LOCKED;
        }
        if !self.sandbox && self.grid.seen[ti] == 0 {
            return reason::FOG;
        }
        if self.is_lava(t) {
            return reason::LAVA;
        }
        let cur = self.grid.kind[ti];
        if k == bk::BELT {
            return if cur == bk::EMPTY || cur == bk::BELT { reason::OK } else { reason::OCCUPIED };
        }
        if cur == k {
            return reason::OK; // rotate in place
        }
        if cur != bk::EMPTY && cur != bk::BELT {
            return reason::OCCUPIED;
        }
        match def.class {
            Class::Drill => {
                let res = self.grid.res[ti];
                if res == 0 {
                    return reason::NO_DEPOSIT;
                }
                if !def.deposits.contains(&res) {
                    return reason::WRONG_DEPOSIT;
                }
                if !self.sandbox && !self.core.can_mine(res) {
                    return reason::DEPOSIT_LOCKED;
                }
            }
            Class::Pump if !self.is_water(t) => return reason::NO_WATER,
            Class::Terraformer if k == bk::HATCHERY => {
                let wet = (0..4).any(|d| {
                    let n = self.grid.step(t, d);
                    n != NONE && self.is_water(n)
                });
                if !wet && !self.is_water(t) {
                    return reason::NO_WATER;
                }
            }
            _ => {}
        }
        if !self.sandbox && !self.core.can_afford(def.cost) {
            return reason::COST;
        }
        reason::OK
    }

    pub fn can_place(&self, x: i32, y: i32, k: u8) -> bool {
        self.check_place(x, y, k) == reason::OK
    }

    /// Places a building. A belt over a belt re-orients it (drag-to-build corners); a
    /// machine over a belt replaces it; a machine over the same machine rotates it.
    pub fn place(&mut self, x: i32, y: i32, k: u8, d: u8) -> bool {
        if !self.can_place(x, y, k) {
            return false;
        }
        let t = self.grid.index(x, y);
        let ti = t as usize;
        let d = d & 3;
        let before = (self.grid.kind[ti], self.grid.dir[ti]);
        if before == (k, d) {
            return true;
        }
        if k == bk::BELT {
            self.grid.kind[ti] = bk::BELT;
            self.grid.dir[ti] = d;
            self.belts.add_tile(t);
            if before.0 == bk::EMPTY {
                self.reveal(x, y, 3);
            }
        } else if before.0 == k {
            let i = self.grid.ent[ti] as usize;
            self.grid.dir[ti] = d;
            self.machines.list[i].dir = d;
            if BUILDINGS[k as usize].class == Class::Tunnel {
                self.unpair(i);
                self.pair_tunnel(i);
            }
        } else {
            if before.0 == bk::BELT {
                self.grid.kind[ti] = bk::EMPTY;
                self.belts.remove_tile(t);
            }
            let def = &BUILDINGS[k as usize];
            if !self.sandbox {
                self.core.pay(def.cost);
            }
            self.grid.kind[ti] = k;
            self.grid.dir[ti] = d;
            let mut m = Machine::new(k, d, t);
            match def.class {
                Class::Drill => {
                    m.aux = self.grid.res[ti] as u32;
                    m.purity = self.grid.purity[ti];
                }
                Class::Battery | Class::Storage => m.aux = 0,
                Class::Radar => m.aux = 7,
                Class::Sorter => m.filter = self.place_filter,
                _ => {}
            }
            let r = self.place_recipe;
            if def.recipes.len() > 1 && (r as usize) < def.recipes.len() && self.recipe_unlocked(k, r) {
                m.recipe = r;
                m.flags |= flag::LOCKED;
            }
            let i = self.machines.add(m);
            self.grid.ent[ti] = i;
            match def.class {
                Class::Tunnel => self.pair_tunnel(i as usize),
                Class::Radar => self.radars.push(t),
                _ => {}
            }
            let sight = match def.class {
                Class::Pole => 9,
                Class::Radar => 7,
                _ => 6,
            };
            self.reveal(x, y, sight);
            self.power_dirty = true;
        }
        self.map_rev = self.map_rev.wrapping_add(1);
        self.record(t, before, (k, d));
        true
    }

    pub fn remove(&mut self, x: i32, y: i32) -> bool {
        let t = self.grid.index(x, y);
        if t == NONE {
            return false;
        }
        let ti = t as usize;
        let before = (self.grid.kind[ti], self.grid.dir[ti]);
        match before.0 {
            bk::BELT => {
                self.grid.kind[ti] = bk::EMPTY;
                self.belts.remove_tile(t);
            }
            bk::EMPTY | bk::CORE | bk::WRECK => return false,
            k => {
                let i = self.grid.ent[ti] as usize;
                let peer = self.unpair(i);
                let m = self.machines.list[i];
                if !self.sandbox {
                    self.core.refund(BUILDINGS[k as usize].cost);
                }
                // Shards, amplifiers and stored goods go back to the Core.
                self.core.stored[it::POWER_SHARD as usize] += m.shards as u32;
                if m.amplified() {
                    self.core.stored[it::AMPLIFIER as usize] += 1;
                }
                if m.class() == Class::Storage && m.aux != NONE && m.held != it::NONE {
                    self.core.stored[m.held as usize] += m.aux;
                }
                match m.class() {
                    Class::DronePort => self.unlink_port(t),
                    Class::Radar => self.radars.retain(|&r| r != t),
                    _ => {}
                }
                if let Some(moved) = self.machines.remove(i) {
                    self.grid.ent[moved as usize] = i as u32;
                }
                self.grid.ent[ti] = NONE;
                self.grid.kind[ti] = bk::EMPTY;
                // The orphaned end may find a new partner.
                if peer != NONE {
                    let j = self.grid.ent[peer as usize] as usize;
                    self.pair_tunnel(j);
                }
                self.power_dirty = true;
            }
        }
        self.map_rev = self.map_rev.wrapping_add(1);
        self.record(t, before, (bk::EMPTY, 0));
        true
    }

    /// Rebuilds belt and power topology if edits happened since the last tick/frame.
    #[inline]
    pub fn ensure_built(&mut self) {
        if self.belts.dirty {
            self.belts.rebuild(&self.grid);
        }
        if self.power_dirty {
            self.rebuild_power();
        }
    }

    /// Whether building `kind` may run recipe `r` (research and alternates).
    pub fn recipe_unlocked(&self, kind: u8, r: u8) -> bool {
        self.sandbox || self.machines.recipes_unlocked[kind as usize] & (1 << r) != 0
    }

    // ---- Machine settings (inspector) -----------------------------------------------------------

    pub(crate) fn machine_at(&self, x: i32, y: i32) -> Option<usize> {
        let t = self.grid.index(x, y);
        if t == NONE || self.grid.ent[t as usize] == NONE {
            return None;
        }
        Some(self.grid.ent[t as usize] as usize)
    }

    /// Picks a crafter's recipe (`NO_RECIPE` returns it to Auto). Buffered inputs of the old
    /// recipe go back to the Core.
    pub fn set_recipe(&mut self, x: i32, y: i32, r: u8) -> bool {
        let Some(i) = self.machine_at(x, y) else { return false };
        let m = self.machines.list[i];
        let def = &BUILDINGS[m.kind as usize];
        if def.class != Class::Crafter
            || (r != NO_RECIPE && (r as usize >= def.recipes.len() || !self.recipe_unlocked(m.kind, r)))
        {
            return false;
        }
        if m.recipe != NO_RECIPE {
            for (s, &(item, _)) in def.recipes[m.recipe as usize].inputs.iter().enumerate() {
                self.core.stored[item as usize] += m.inv[s] as u32;
            }
        }
        let m = &mut self.machines.list[i];
        m.recipe = r;
        m.inv = [0; 3];
        m.timer = 0;
        if r == NO_RECIPE {
            m.flags &= !flag::LOCKED;
        } else {
            m.flags |= flag::LOCKED;
        }
        true
    }

    pub fn set_filter(&mut self, x: i32, y: i32, item: u8) -> bool {
        let Some(i) = self.machine_at(x, y) else { return false };
        let m = &mut self.machines.list[i];
        if m.class() != Class::Sorter || item as usize >= ITEM_COUNT {
            return false;
        }
        m.filter = item;
        true
    }

    /// Installs or removes power shards (0..=3), taking them from or returning them to the
    /// Core. Returns false if the Core is short.
    pub fn set_shards(&mut self, x: i32, y: i32, n: u8) -> bool {
        let Some(i) = self.machine_at(x, y) else { return false };
        let m = self.machines.list[i];
        let class = m.class();
        let tunable = matches!(
            class,
            Class::Drill
                | Class::Pump
                | Class::Crafter
                | Class::Terraformer
                | Class::Generator
                | Class::Recycler
        );
        if !tunable || n > 3 {
            return false;
        }
        let have = self.core.stored[it::POWER_SHARD as usize];
        let need = n.saturating_sub(m.shards) as u32;
        if !self.sandbox && have < need {
            return false;
        }
        // Sandbox installs shards it doesn't have: never below zero.
        self.core.stored[it::POWER_SHARD as usize] =
            have.saturating_sub(need) + m.shards.saturating_sub(n) as u32;
        let mut m = m;
        m.shards = n;
        self.machines.configure(&mut m);
        self.machines.list[i] = m;
        if n == 3 {
            self.core.flags |= crate::core::event::CLOCK250;
        }
        true
    }

    pub fn set_amplifier(&mut self, x: i32, y: i32, on: bool) -> bool {
        let Some(i) = self.machine_at(x, y) else { return false };
        let m = self.machines.list[i];
        if !matches!(m.class(), Class::Drill | Class::Pump | Class::Crafter | Class::Terraformer)
            || m.amplified() == on
        {
            return false;
        }
        let stock = &mut self.core.stored[it::AMPLIFIER as usize];
        if on {
            if *stock == 0 && !self.sandbox {
                return false;
            }
            *stock = stock.saturating_sub(1);
            self.core.flags |= crate::core::event::AMPLIFIED;
        } else {
            *stock += 1;
        }
        let mut m = m;
        m.flags ^= flag::AMP;
        self.machines.configure(&mut m);
        self.machines.list[i] = m;
        true
    }

    // ---- Tunnels ----------------------------------------------------------------------------

    /// Pairs tunnel `i` with the nearest free end facing the same way within range: an
    /// unpaired entrance behind it makes `i` its exit; one ahead of it becomes `i`'s exit.
    fn pair_tunnel(&mut self, i: usize) {
        let (d, tile) = (self.machines.list[i].dir, self.machines.list[i].tile);
        for (look, i_is_exit) in [(dir::opposite(d), true), (d, false)] {
            let mut t = tile;
            for k in 1..=TUNNEL_RANGE {
                t = self.grid.step(t, look);
                if t == NONE {
                    break;
                }
                if self.grid.kind[t as usize] != bk::TUNNEL {
                    continue;
                }
                let j = self.grid.ent[t as usize] as usize;
                let other = self.machines.list[j];
                if other.dir != d {
                    continue;
                }
                if other.rr == ENTRANCE && other.aux == NONE {
                    if i_is_exit {
                        self.link(j, i, k as u8);
                    } else {
                        self.link(i, j, k as u8);
                    }
                    return;
                }
                break; // the nearest same-facing piece is taken
            }
        }
        let m = &mut self.machines.list[i];
        m.rr = ENTRANCE;
        m.aux = NONE;
    }

    fn link(&mut self, entrance: usize, exit: usize, dist: u8) {
        let (te, tx) = (self.machines.list[entrance].tile, self.machines.list[exit].tile);
        let e = &mut self.machines.list[entrance];
        e.rr = ENTRANCE;
        e.aux = tx;
        e.inv[0] = dist;
        self.machines.alloc_queue(entrance);
        let x = &mut self.machines.list[exit];
        x.rr = EXIT;
        x.aux = te;
        self.machines.release_queue(exit);
    }

    /// Breaks tunnel `i`'s pairing, if any. Returns the former partner's tile.
    fn unpair(&mut self, i: usize) -> u32 {
        let m = self.machines.list[i];
        if BUILDINGS[m.kind as usize].class != Class::Tunnel || m.aux == NONE {
            return NONE;
        }
        let j = self.grid.ent[m.aux as usize] as usize;
        for k in [i, j] {
            self.machines.release_queue(k);
            let p = &mut self.machines.list[k];
            p.aux = NONE;
            p.rr = ENTRANCE;
        }
        m.aux
    }

    // ---- Undo -----------------------------------------------------------------------------

    fn record(&mut self, tile: u32, before: (u8, u8), after: (u8, u8)) {
        if !self.undo.replaying && before != after {
            if self.undo.groups.is_empty() {
                self.undo.groups.push(0);
            }
            self.undo.ops.push(Op { tile, before });
        }
    }

    /// Starts a new undo group (one per gesture).
    pub fn edit_begin(&mut self) {
        let u = &mut self.undo;
        if u.groups.last() == Some(&u.ops.len()) {
            return; // previous group is still empty
        }
        u.groups.push(u.ops.len());
        if u.groups.len() > UNDO_GROUPS {
            let cut = u.groups[1];
            u.ops.drain(..cut);
            u.groups.remove(0);
            for g in &mut u.groups {
                *g -= cut;
            }
        }
    }

    pub fn undo_depth(&self) -> u32 {
        let u = &self.undo;
        u.groups.iter().filter(|&&g| g < u.ops.len()).count() as u32
    }

    /// Reverts the most recent gesture. Returns false if there is nothing to undo.
    pub fn undo(&mut self) -> bool {
        while let Some(start) = self.undo.groups.pop() {
            if start >= self.undo.ops.len() {
                continue; // empty group
            }
            let ops: Vec<Op> = self.undo.ops.drain(start..).collect();
            self.undo.replaying = true;
            let (recipe, filter) = (self.place_recipe, self.place_filter);
            self.place_recipe = NO_RECIPE;
            self.place_filter = 0;
            let w = self.grid.w as u32;
            for op in ops.iter().rev() {
                let (x, y) = ((op.tile % w) as i32, (op.tile / w) as i32);
                let now = self.grid.kind[op.tile as usize];
                if op.before.0 != bk::EMPTY && op.before.0 == now {
                    self.place(x, y, op.before.0, op.before.1); // re-orient or rotate back
                    continue;
                }
                if now != bk::EMPTY {
                    self.remove(x, y);
                }
                if op.before.0 != bk::EMPTY {
                    self.place(x, y, op.before.0, op.before.1);
                }
            }
            self.place_recipe = recipe;
            self.place_filter = filter;
            self.undo.replaying = false;
            return true;
        }
        false
    }

    // ---- Research and the Ark -----------------------------------------------------------------

    pub fn research(&mut self, t: u8) -> bool {
        if self.core.research(t) {
            self.retune();
            self.update_stats();
            true
        } else {
            false
        }
    }

    /// Re-derives every tuning value from research, repeatable levels, alternates and
    /// launches. Idempotent; runs after research, loading and launching.
    pub fn retune(&mut self) {
        let c = &self.core;
        let (mut belt, mut drill_tier, mut craft_tier, mut heater) = (1u8, 1u8, 1u8, 1u32);
        let (mut drill_bonus, mut factory_bonus, mut terra_bonus, mut power_bonus) = (0u32, 0u32, 0u32, 0u32);
        for (t, def) in TECH.iter().enumerate() {
            if !c.researched[t] {
                continue;
            }
            let level = if def.repeat { c.levels[t - REPEAT_FIRST as usize] as u32 } else { 1 };
            for e in def.effects {
                match *e {
                    Effect::BeltTier(n) => belt = belt.max(n),
                    Effect::DrillTier(n) => drill_tier = drill_tier.max(n),
                    Effect::CraftTier(n) => craft_tier = craft_tier.max(n),
                    Effect::HeaterBoost => heater = 2,
                    Effect::DrillBonus(p) => drill_bonus += p as u32 * level,
                    Effect::FactoryBonus(p) => factory_bonus += p as u32 * level,
                    Effect::TerraBonus(p) => terra_bonus += p as u32 * level,
                    Effect::PowerBonus(p) => power_bonus += p as u32 * level,
                    Effect::Unlock(_) | Effect::Ore(_) => {}
                }
            }
        }
        let legacy = c.legacy * LEGACY_PCT;
        let ms = &mut self.machines;
        let speed = BELT_SPEEDS[belt as usize - 1];
        self.belts.speed = speed;
        ms.belt_speed = speed;
        ms.drill_pct = DRILL_PCT[drill_tier as usize - 1] * (100 + drill_bonus + legacy) / 100;
        ms.craft_pct = CRAFT_PCT[craft_tier as usize - 1] * (100 + factory_bonus + legacy) / 100;
        ms.terra_pct = 100 + terra_bonus;
        ms.power_pct = 100 + power_bonus;
        ms.heater_mult = heater;
        for (k, def) in BUILDINGS.iter().enumerate() {
            let mut mask = 0u32;
            for (j, rc) in def.recipes.iter().enumerate() {
                let open = match rc.unlock {
                    FREE => true,
                    u if u >= ALT_BASE => c.alts & (1 << (u - ALT_BASE)) != 0,
                    u => (u as usize) < TECH_COUNT && c.researched[u as usize],
                };
                if open || self.sandbox {
                    mask |= 1 << j;
                }
            }
            ms.recipes_unlocked[k] = mask;
        }
        ms.configure_all();
    }

    /// Sends what the Core holds towards the current Ark phase. Returns true when that
    /// completes the phase.
    pub fn ark_contribute(&mut self) -> bool {
        let done = self.core.ark_contribute();
        if done {
            self.update_stats();
        }
        done
    }

    // ---- Simulation -----------------------------------------------------------------------

    /// Hands `it`, travelling in direction `md`, to whatever occupies tile `t`.
    fn offer(&mut self, t: u32, it: u8, md: u8) -> bool {
        if t == NONE {
            return false;
        }
        match self.grid.kind[t as usize] {
            bk::BELT => self.belts.insert_from(&self.grid, t, it, md),
            bk::CORE => self.core.accept(it),
            bk::EMPTY | bk::WRECK => false,
            _ => self.machines.accept(self.grid.ent[t as usize], it, md),
        }
    }

    /// Zipper merging: when two segments side-load into the same belt tile from opposite
    /// sides, the one that has waited longer goes first. Without this the segment updated
    /// first would win every tie and starve the other side of a T-junction.
    fn should_yield(&self, s: u32) -> bool {
        let seg = &self.belts.segs[s as usize];
        let (tt, md) = (seg.target_tile, seg.target_dir);
        if tt == NONE || self.grid.kind[tt as usize] != bk::BELT || self.grid.dir[tt as usize] == md {
            return false;
        }
        let rival = self.grid.step(tt, md);
        if rival == NONE
            || self.grid.kind[rival as usize] != bk::BELT
            || self.grid.dir[rival as usize] != dir::opposite(md)
        {
            return false;
        }
        let rs = self.belts.tile_seg[rival as usize];
        rs != NONE
            && rs != s
            && self.belts.front_ready(rs) != 0
            && self.belts.segs[rs as usize].wait > seg.wait
    }

    pub fn tick(&mut self) {
        self.ensure_built();
        self.machines.now = self.tick;
        self.machines.daylight = sky::daylight(self.tick);
        for oi in 0..self.belts.order.len() {
            let s = self.belts.order[oi];
            self.belts.advance(s);
            let it = self.belts.front_ready(s);
            if it == 0 {
                continue;
            }
            let seg = &self.belts.segs[s as usize];
            let (tt, td) = (seg.target_tile, seg.target_dir);
            if !self.should_yield(s) && self.offer(tt, it, td) {
                self.belts.pop_front(s);
                self.belts.segs[s as usize].wait = 0;
            } else {
                let seg = &mut self.belts.segs[s as usize];
                seg.wait = seg.wait.saturating_add(1);
            }
        }
        for i in 0..self.machines.list.len() {
            if PASSIVE[self.machines.list[i].kind as usize] {
                continue;
            }
            if let Some(e) = self.machines.process(i) {
                for k in 0..e.n {
                    let face = e.faces[k as usize];
                    let t = self.grid.step(e.tile, face);
                    if self.offer(t, e.item, face) {
                        self.machines.emitted(i, k);
                        break;
                    }
                }
            }
        }
        self.tick_drones();
        self.tick_radars();
        // Demo maps may have no Core (it sits off the map): then it supplies nothing.
        let core_supply = if self.core.x >= 0 { CORE_POWER } else { 0 };
        self.machines.settle_power(self.core_net, core_supply);
        let gain = core::mem::take(&mut self.machines.meter_gain);
        if gain != [0; meter::COUNT] {
            self.core.add_meters(&gain);
        }
        let credits = core::mem::take(&mut self.machines.credits_gain);
        if credits > 0 {
            self.core.credits += credits;
            self.core.credits_recycled += credits;
        }
        self.core.tick();
        self.tick_sky();
        if self.tick.is_multiple_of(TICKS_PER_SEC) {
            self.second();
        }
        if self.tick.is_multiple_of(15) {
            self.core.check_goal(&self.machines.counts);
        }
        self.tick = self.tick.wrapping_add(1);
        self.update_stats();
    }

    /// Once a second: rate windows and achievements.
    fn second(&mut self) {
        let mut sec = [0u32; ITEM_COUNT * 3];
        let ms = &mut self.machines;
        sec[..ITEM_COUNT].copy_from_slice(&ms.produced);
        sec[ITEM_COUNT..ITEM_COUNT * 2].copy_from_slice(&ms.consumed);
        sec[ITEM_COUNT * 2..].copy_from_slice(&self.core.delivered_now);
        ms.produced = [0; ITEM_COUNT];
        ms.consumed = [0; ITEM_COUNT];
        self.core.delivered_now = [0; ITEM_COUNT];
        self.rates.push(&sec);
        self.check_achievements();
    }

    pub fn machine_count(&self) -> u32 {
        (self.machines.list.len() - self.machines.counts[bk::WRECK as usize] as usize) as u32
    }

    fn check_achievements(&mut self) {
        let supply: u64 = self.machines.nets.iter().map(|n| n.last_supply as u64).sum();
        let stored_mj: u64 = self.machines.nets.iter().map(|n| n.stored / 60_000).sum();
        let researched = self.core.researched.iter().filter(|&&r| r).count() as u32;
        let all_research = TECH.iter().enumerate().all(|(t, _)| self.core.researched[t]);
        let stored_total: u64 = self.core.stored.iter().map(|&n| n as u64).sum();
        let (machines, belts, rate) =
            (self.machine_count(), self.belts.list.len() as u32, self.core.rate_per_min());
        let c = &self.core;
        for (i, a) in ACHIEVEMENTS.iter().enumerate() {
            if c.achievements & (1 << i) != 0 {
                continue;
            }
            let met = match a.cond {
                Cond::Delivered(n) => c.total_delivered >= n,
                Cond::Machines(n) => machines >= n,
                Cond::Belts(n) => belts >= n,
                Cond::Rate(n) => rate >= n,
                Cond::Stage(s) => c.stage >= s,
                Cond::Researched(n) => researched >= n,
                Cond::AllResearch => all_research,
                Cond::Built(k, n) => self.machines.counts[k as usize] >= n,
                Cond::PowerMw(n) => supply >= n as u64 * 1000,
                Cond::Wrecks(n) => c.salvaged >= n,
                Cond::Logs(n) => c.logs as u32 >= n,
                Cond::Ark(n) => c.ark_phase >= n,
                Cond::Credits(n) => c.credits_recycled >= n,
                Cond::Stored(n) => stored_total >= n,
                Cond::Clock250 => c.flags & crate::core::event::CLOCK250 != 0,
                Cond::Amplified => c.flags & crate::core::event::AMPLIFIED != 0,
                Cond::Blueprint => c.flags & crate::core::event::BLUEPRINT != 0,
                Cond::DroneLink => c.flags & crate::core::event::DRONE_LINK != 0,
                Cond::Launches(n) => c.legacy >= n,
                Cond::BatteryMj(n) => stored_mj >= n as u64,
            };
            if met {
                let c = &mut self.core;
                c.achievements |= 1 << i;
                c.credits += a.credits as u64;
                c.stored[it::POWER_SHARD as usize] += a.shards as u32;
                c.last_achievement = i as u32;
                c.revision += 1;
                return; // one per second, so each gets its own toast
            }
        }
    }

    pub fn update_stats(&mut self) {
        let c = &self.core;
        let (have, need) = c.goal_progress(&self.machines.counts);
        let (kind, a, _) = goal_parts(c.goal().0);
        let goal_kind = match kind {
            "build" => 0,
            "deliver" => 1,
            "research" => 2,
            "ti" => 3,
            "stage" => 4,
            "ark" => 5,
            _ => 6,
        };
        let ti_rate: u64 = (0..meter::COUNT).map(|m| c.meter_rate_x100(m)).sum();
        let undo_depth = self.undo_depth();
        let machines = self.machine_count();
        let researched = c.researched.iter().enumerate().fold(0u64, |acc, (i, &r)| acc | ((r as u64) << i));
        let (mut supply, mut demand, mut sat, mut stored, mut capacity) = (0u64, 0u64, 1000u32, 0u64, 0u64);
        for n in &self.machines.nets {
            supply += n.last_supply as u64;
            demand += n.last_demand as u64;
            stored += n.stored;
            capacity += n.capacity;
            if n.last_demand > 0 {
                sat = sat.min(n.sat * 1000 / FULL);
            }
        }
        let flying = self.machines.ports.iter().filter(|p| p.tile != NONE && p.state != 0).count() as u32;
        let water = ((self.water_level() + 1.0) * 1000.0) as u32;
        let s = &mut self.stats;
        s[stat::TICK] = self.tick;
        s[stat::DELIVERED] = c.total_delivered as u32;
        s[stat::RATE] = c.rate_per_min();
        s[stat::STAGE] = c.stage as u32;
        s[stat::TI_LO] = c.ti as u32;
        s[stat::TI_HI] = (c.ti >> 32) as u32;
        s[stat::TI_RATE_X100] = ti_rate.min(u32::MAX as u64) as u32;
        for m in 0..meter::COUNT {
            s[stat::METERS + m * 2] = c.meters[m] as u32;
            s[stat::METERS + m * 2 + 1] = (c.meters[m] >> 32) as u32;
            s[stat::METER_RATES + m] = c.meter_rate_x100(m).min(u32::MAX as u64) as u32;
        }
        s[stat::OBJECTIVE] = c.objective;
        s[stat::GOAL_HAVE_LO] = have as u32;
        s[stat::GOAL_HAVE_HI] = (have >> 32) as u32;
        s[stat::GOAL_NEED_LO] = need as u32;
        s[stat::GOAL_NEED_HI] = (need >> 32) as u32;
        s[stat::GOAL_KIND] = goal_kind;
        s[stat::GOAL_A] = a as u32;
        s[stat::ITEMS] = self.belts.item_count;
        s[stat::BELTS] = self.belts.list.len() as u32;
        s[stat::SEGMENTS] = self.belts.segs.len() as u32;
        s[stat::MACHINES] = machines;
        s[stat::BELT_SPEED] = self.belts.speed;
        s[stat::CORE_X] = c.x as u32;
        s[stat::CORE_Y] = c.y as u32;
        s[stat::CORE_SIZE] = c.size as u32;
        s[stat::RESEARCHED_LO] = researched as u32;
        s[stat::RESEARCHED_HI] = (researched >> 32) as u32;
        s[stat::REVISION] = c.revision;
        s[stat::UNDO_DEPTH] = undo_depth;
        s[stat::MAP_REV] = self.map_rev;
        s[stat::ARK_PHASE] = c.ark_phase as u32;
        s[stat::CREDITS_LO] = c.credits as u32;
        s[stat::CREDITS_HI] = (c.credits >> 32) as u32;
        s[stat::POWER_SUPPLY] = supply.min(u32::MAX as u64) as u32;
        s[stat::POWER_DEMAND] = demand.min(u32::MAX as u64) as u32;
        s[stat::POWER_SAT] = sat;
        s[stat::BATTERY_MJ] = (stored / 60_000) as u32;
        s[stat::BATTERY_CAP_MJ] = (capacity / 60_000) as u32;
        s[stat::POWER_REV] = self.power_rev;
        s[stat::FOG_REV] = self.fog_rev;
        s[stat::FOG_DIRTY..stat::FOG_DIRTY + 4].copy_from_slice(&self.fog_dirty.map(|v| v as u32));
        s[stat::RES_REV] = self.res_rev;
        s[stat::DAYLIGHT] = self.machines.daylight * 1000 / FULL;
        let (shower_in, active) = self.sky.announce(self.tick);
        s[stat::SHOWER_IN] = shower_in;
        s[stat::SHOWER_ACTIVE] = active as u32;
        s[stat::ACH_LO] = c.achievements as u32;
        s[stat::ACH_HI] = (c.achievements >> 32) as u32;
        s[stat::PROBES] = c.probes as u32;
        s[stat::ALTS] = c.alts;
        s[stat::LOGS] = c.logs as u32;
        s[stat::SALVAGED] = c.salvaged;
        s[stat::LEGACY] = c.legacy;
        s[stat::PLANET] = self.planet as u32;
        s[stat::COSMETICS] = c.cosmetics;
        s[stat::BELT_COLOR] = c.belt_color as u32;
        s[stat::TRIM] = c.trim as u32;
        for (k, &l) in c.levels.iter().enumerate() {
            s[stat::LEVELS + k] = l as u32;
        }
        s[stat::METEORS] = self.sky.landed;
        s[stat::NETS] = self.machines.nets.len() as u32;
        s[stat::DRONES_FLYING] = flying;
        s[stat::WATER_X1000] = water;
        s[stat::ARK_PAID..stat::ARK_PAID + 4].copy_from_slice(&c.ark_paid);
        s[stat::LAST_ACH] = c.last_achievement;
        s[stat::BP_CELLS] = self.blueprint.len() as u32;
    }

    /// Forgets the rate windows (tests compare saves of worlds with different histories).
    pub fn clear_rate_history(&mut self) {
        self.rates = Rates::default();
        self.core.clear_rate_history();
    }

    /// Clears the fog-dirty rectangle once JS has uploaded it.
    pub fn fog_ack(&mut self) {
        self.fog_dirty = [i32::MAX, i32::MAX, 0, 0];
        self.stats[stat::FOG_DIRTY..stat::FOG_DIRTY + 4].copy_from_slice(&self.fog_dirty.map(|v| v as u32));
    }

    // ---- Benchmark ------------------------------------------------------------------------

    /// Stress scene: `loops` closed belt loops (200x3 tiles, two segments each) packed with
    /// items, plus `loops * 20` production lines (drill -> smelter -> press -> incinerator)
    /// that keep ~4 machines each busy. 50 loops = ~35k belts, ~45k items, ~4k machines.
    pub fn build_benchmark(&mut self, loops: u32) {
        let sandbox = self.sandbox;
        self.sandbox = true;
        // Power the lines for real (poles and reactors below), so the benchmark pays for
        // the power simulation too.
        self.machines.free_power = false;
        self.undo.replaying = true;
        self.grid.seen.fill(255);
        self.fog_dirty = [0, 0, self.grid.w, self.grid.h];
        self.fog_rev += 1;
        // Clear the wrecks so the test pattern is the same on every seed.
        for t in 0..self.grid.kind.len() {
            if self.grid.kind[t] == bk::WRECK {
                let i = self.grid.ent[t] as usize;
                if let Some(moved) = self.machines.remove(i) {
                    self.grid.ent[moved as usize] = i as u32;
                }
                self.grid.kind[t] = bk::EMPTY;
                self.grid.ent[t] = NONE;
            }
        }
        let (w, h) = (200, 3);
        let x0 = 4;
        let mut rng = 0x9e37_79b9u32;
        let mut bottom = 4;
        for l in 0..loops as i32 {
            let y0 = 4 + l * (h + 1);
            let (x1, y1) = (x0 + w - 1, y0 + h - 1);
            if y1 >= self.grid.h / 2 - 8 || x1 >= self.grid.w - 1 {
                break;
            }
            for x in x0..x1 {
                self.place(x, y0, bk::BELT, dir::E);
            }
            for y in y0..y1 {
                self.place(x1, y, bk::BELT, dir::S);
            }
            for x in (x0 + 1..=x1).rev() {
                self.place(x, y1, bk::BELT, dir::W);
            }
            for y in (y0 + 1..=y1).rev() {
                self.place(x0, y, bk::BELT, dir::N);
            }
            bottom = y1 + 3;
        }
        // Production lines below the loops, clear of the core, powered by pole rows.
        let (cx, cy) = (self.core.x, self.core.y);
        let mut lines = loops * 20;
        let mut y = bottom.max(cy + CORE_SIZE + 3);
        'rows: while lines > 0 && y < self.grid.h - 2 {
            let mut x = 4;
            while x + 19 < self.grid.w - 2 {
                if lines == 0 {
                    break 'rows;
                }
                if (y - cy).abs() < 8 && (x - cx).abs() < 26 {
                    x += 20;
                    continue;
                }
                let t = self.grid.index(x, y);
                self.grid.res[t as usize] = it::IRON_ORE;
                self.place(x, y, bk::DRILL, dir::E);
                for dx in 1..5 {
                    self.place(x + dx, y, bk::BELT, dir::E);
                }
                self.place(x + 5, y, bk::SMELTER, dir::E);
                for dx in 6..10 {
                    self.place(x + dx, y, bk::BELT, dir::E);
                }
                self.place(x + 10, y, bk::PRESS, dir::E);
                for dx in 11..15 {
                    self.place(x + dx, y, bk::BELT, dir::E);
                }
                self.place(x + 15, y, bk::INCINERATOR, dir::E);
                lines -= 1;
                x += 20;
            }
            y += 2;
        }
        // Poles over the production rows (never on belts) and reactors by the Core, so
        // every machine is powered for real.
        let step = POLE_RADIUS * 2;
        let mut wants = vec![false; self.grid.kind.len()];
        for m in &self.machines.list {
            wants[m.tile as usize] = true;
        }
        let mut py = 2;
        while py < self.grid.h {
            let mut px = 2;
            while px < self.grid.w {
                let mut near = false;
                for y in (py - POLE_RADIUS).max(0)..(py + POLE_RADIUS + 1).min(self.grid.h) {
                    for x in (px - POLE_RADIUS).max(0)..(px + POLE_RADIUS + 1).min(self.grid.w) {
                        near |= wants[(y * self.grid.w + x) as usize];
                    }
                }
                if near {
                    for (dx, dy) in [(0, 0), (1, 0), (0, 1), (-1, 0), (0, -1), (1, 1), (-1, -1)] {
                        let t = self.grid.index(px + dx, py + dy);
                        if t != NONE
                            && self.grid.kind[t as usize] == bk::EMPTY
                            && self.place(px + dx, py + dy, bk::POLE, 0)
                        {
                            break;
                        }
                    }
                }
                px += step;
            }
            py += step;
        }
        for (dx, dy) in [(0, -6), (1, -6), (2, -6), (3, -6)] {
            let (x, y) = (cx + dx, cy + dy);
            if self.place(x, y, bk::REACTOR, 0) {
                let i = self.grid.ent[self.grid.index(x, y) as usize] as usize;
                self.machines.list[i].inv[0] = 10;
            }
        }
        self.ensure_built();
        for s in 0..self.belts.segs.len() as u32 {
            if self.belts.segs[s as usize].len < 100 {
                continue; // only the long loops get pre-filled
            }
            rng ^= rng << 13;
            rng ^= rng >> 17;
            rng ^= rng << 5;
            self.belts.fill(s, 1 + (rng % 24) as u8);
        }
        self.undo.replaying = false;
        self.sandbox = sandbox;
        self.update_stats();
    }
}
