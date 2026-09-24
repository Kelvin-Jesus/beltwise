//! Machines: every 1x1 building except belts, driven by the tables in `content.rs`.
//!
//! Stored densely (swap-remove) so the per-tick loop is a linear scan. A machine outputs
//! through its front face (`dir`) and takes input through the other faces. Crafters either
//! run the recipe the player picked or, on Auto, the first unlocked recipe that uses the
//! first input to arrive.
//!
//! Work is measured in 1/64 of a tick: every tick a machine adds its `speed` (64 at 100%)
//! scaled by its power network's satisfaction, so overclocking, research bonuses and
//! brown-outs are all the same multiplication.

use crate::content::{
    ALT_BASE, BUILDING_COUNT, BUILDINGS, CLOCK_PCT, CLOCK_POWER_PCT, Class, DRILL_TICKS, ITEM_COUNT, ITEMS,
    Item, PUMP_TICKS, PURITY_PCT, STORAGE_CAP, bk, it, meter,
};
use crate::types::{MIN_GAP, NONE, SUB, dir};

pub const NO_RECIPE: u8 = 255;
pub const NO_NET: u16 = u16::MAX;
pub const NO_QUEUE: u16 = u16::MAX;
const QUEUE_CAP: usize = 16;
/// Progress units per tick at 100% speed.
pub const SPEED_ONE: u32 = 64;
/// Satisfaction and load are fixed point with this denominator.
pub const FULL: u32 = 1024;
/// Ticks a Recycler spends on each item.
const RECYCLE_TICKS: u32 = 10;
/// Items a Recycler or generator buffers.
const RECYCLER_BUFFER: u8 = 4;
const FUEL_BUFFER: u8 = 10;

/// `Machine::flags` bits.
pub mod flag {
    /// The recipe was chosen by the player (never switched automatically).
    pub const LOCKED: u8 = 1;
    /// An amplifier is installed: double output, four times the power.
    pub const AMP: u8 = 2;
}

/// What a machine is doing (`Machine::status`), for the inspector and the problem overlay.
pub mod status {
    pub const IDLE: u8 = 0;
    pub const WORKING: u8 = 1;
    pub const NO_INPUT: u8 = 2;
    pub const BLOCKED: u8 = 3;
    pub const NO_POWER: u8 = 4;
    pub const NO_FUEL: u8 = 5;
    pub const UNLINKED: u8 = 6;
}

/// Tunnel roles (stored in `Machine::rr`).
pub const ENTRANCE: u8 = 0;
pub const EXIT: u8 = 1;

#[derive(Clone, Copy, Debug)]
pub struct Machine {
    pub kind: u8,
    pub dir: u8,
    pub recipe: u8,
    /// Splitter/sorter: round-robin cursor. Tunnel: role.
    pub rr: u8,
    pub tile: u32,
    /// Input buffer per recipe input. Tunnel entrance: `inv[0]` is the tunnel length.
    /// Generators: `inv[0]` is buffered fuel.
    pub inv: [u8; 3],
    /// Finished items waiting to leave (drills, pumps, crafters); Recycler: items queued.
    pub out: u8,
    /// Splitter/sorter: the item in transit. Storage: the item stored.
    pub held: Item,
    pub flags: u8,
    /// Power shards installed (0..=3).
    pub shards: u8,
    /// Drills: deposit purity (0 impure, 1 normal, 2 pure).
    pub purity: u8,
    /// Sorter: the item it sends straight on (0 = pass everything).
    pub filter: u8,
    /// Ticks spent without progress (saturating), so the overlay ignores brief hiccups.
    pub stall: u8,
    /// See [`status`].
    pub status: u8,
    /// Progress towards the current cycle, in 1/`SPEED_ONE` ticks.
    pub timer: u16,
    /// Progress per tick at full power.
    pub speed: u16,
    pub net: u16,
    /// Tunnel entrance: index into the queue pool. Drone port: index into the port pool.
    pub queue: u16,
    /// Drill: the item it mines. Tunnel: the paired tile, or `NONE`. Storage: item count.
    /// Generator: energy left in the burning fuel item (kW-ticks). Battery: charge share
    /// (kW-ticks, synced on save). Drone port: target tile, or `NONE`. Radar: reveal radius.
    /// Wreck: what it holds (kind << 8 | index).
    pub aux: u32,
    /// Power drawn while working (kW); generators: output at full load.
    pub draw: u32,
}

impl Machine {
    pub fn new(kind: u8, dir: u8, tile: u32) -> Self {
        let recipes = BUILDINGS[kind as usize].recipes;
        Machine {
            kind,
            dir,
            // Single-recipe buildings are fixed from the start.
            recipe: if recipes.len() == 1 { 0 } else { NO_RECIPE },
            rr: 0,
            tile,
            inv: [0; 3],
            out: 0,
            held: it::NONE,
            flags: 0,
            shards: 0,
            purity: 1,
            filter: 0,
            stall: 0,
            status: status::IDLE,
            timer: 0,
            speed: SPEED_ONE as u16,
            net: NO_NET,
            queue: NO_QUEUE,
            aux: NONE,
            draw: 0,
        }
    }

    #[inline]
    pub fn class(&self) -> Class {
        BUILDINGS[self.kind as usize].class
    }

    pub fn amplified(&self) -> bool {
        self.flags & flag::AMP != 0
    }
}

/// Items in transit through a tunnel.
#[derive(Clone, Copy)]
pub struct TunnelQueue {
    pub items: [Item; QUEUE_CAP],
    pub ready: [u32; QUEUE_CAP],
    pub head: u8,
    pub len: u8,
    pub last_in: u32,
}

impl Default for TunnelQueue {
    fn default() -> Self {
        TunnelQueue { items: [0; QUEUE_CAP], ready: [0; QUEUE_CAP], head: 0, len: 0, last_in: 0 }
    }
}

/// A fixed-capacity FIFO of items (drone port inboxes and outboxes).
#[derive(Clone, Copy)]
pub struct ItemRing<const N: usize> {
    pub items: [Item; N],
    pub head: u8,
    pub len: u8,
}

impl<const N: usize> Default for ItemRing<N> {
    fn default() -> Self {
        ItemRing { items: [0; N], head: 0, len: 0 }
    }
}

impl<const N: usize> ItemRing<N> {
    pub fn push(&mut self, item: Item) -> bool {
        if self.len as usize >= N {
            return false;
        }
        self.items[(self.head as usize + self.len as usize) % N] = item;
        self.len += 1;
        true
    }
    pub fn front(&self) -> Option<Item> {
        (self.len > 0).then(|| self.items[self.head as usize])
    }
    pub fn pop(&mut self) -> Option<Item> {
        let it = self.front()?;
        self.head = ((self.head as usize + 1) % N) as u8;
        self.len -= 1;
        Some(it)
    }
    pub fn free(&self) -> usize {
        N - self.len as usize
    }
    pub fn iter(&self) -> impl Iterator<Item = Item> + '_ {
        (0..self.len as usize).map(|k| self.items[(self.head as usize + k) % N])
    }
}

pub const PORT_BOX: usize = 48;
pub const DRONE_CARGO: usize = 24;

/// Drone flight states.
pub mod flight {
    pub const HOME: u8 = 0;
    pub const OUTBOUND: u8 = 1;
    /// Hovering over the target until its outbox has room.
    pub const UNLOADING: u8 = 2;
    pub const RETURNING: u8 = 3;
}

/// A drone port's buffers and its drone.
#[derive(Clone, Copy, Default)]
pub struct Port {
    /// The port's tile, or `NONE` for a free pool slot.
    pub tile: u32,
    pub inbox: ItemRing<PORT_BOX>,
    pub outbox: ItemRing<PORT_BOX>,
    pub cargo: ItemRing<DRONE_CARGO>,
    pub state: u8,
    /// Ticks into the current leg, and the leg's length.
    pub t: u16,
    pub dur: u16,
    /// Ticks the inbox has waited for a full load.
    pub wait: u16,
    /// Where the drone is flying (tile), fixed when it takes off.
    pub dest: u32,
}

/// What a machine wants to push out this tick: `item` through the first of `faces` (relative
/// to the machine at `tile`) whose neighbour accepts it.
#[derive(Clone, Copy, Debug)]
pub struct Emit {
    pub item: Item,
    pub tile: u32,
    pub faces: [u8; 3],
    pub n: u8,
}

/// One power network: every consumer and generator in the coverage of a connected set of
/// poles (and the Core).
#[derive(Clone, Copy, Default, Debug)]
pub struct Net {
    /// Accumulated during the tick (kW).
    pub demand: u32,
    pub supply: u32,
    /// Share of demand met, and share of generator capacity used, from the last tick.
    pub sat: u32,
    pub load: u32,
    /// Battery charge and capacity (kW-ticks) and the fastest they can move power (kW).
    pub stored: u64,
    pub capacity: u64,
    pub rate: u32,
    pub batteries: u32,
    pub has_core: bool,
    /// Last tick's totals, for display.
    pub last_demand: u32,
    pub last_supply: u32,
    pub last_flow: i64,
}

pub struct Machines {
    pub list: Vec<Machine>,
    pub counts: [u16; BUILDING_COUNT],
    pub queues: Vec<TunnelQueue>,
    free_queues: Vec<u16>,
    pub ports: Vec<Port>,
    free_ports: Vec<u16>,
    /// Current tick (tunnel timing).
    pub now: u32,
    /// Tuning from research, repeatable bonuses and launches (percent).
    pub belt_speed: u32,
    pub drill_pct: u32,
    pub craft_pct: u32,
    pub terra_pct: u32,
    pub power_pct: u32,
    pub heater_mult: u32,
    /// Sunlight for solar panels (0..=FULL).
    pub daylight: u32,
    /// Machines outside any network run anyway (sandbox and tests).
    pub free_power: bool,
    /// Unlocked recipes per building (bit per recipe index).
    pub recipes_unlocked: [u32; BUILDING_COUNT],
    /// Terraforming points produced since the core last drained them.
    pub meter_gain: [u64; meter::COUNT],
    pub incinerated: u64,
    pub credits_gain: u64,
    /// Items made and used this second, per item (drained into rate windows).
    pub produced: [u32; ITEM_COUNT],
    pub consumed: [u32; ITEM_COUNT],
    pub nets: Vec<Net>,
}

impl Default for Machines {
    fn default() -> Self {
        Machines {
            list: Vec::new(),
            counts: [0; BUILDING_COUNT],
            queues: Vec::new(),
            free_queues: Vec::new(),
            ports: Vec::new(),
            free_ports: Vec::new(),
            now: 0,
            belt_speed: crate::content::BELT_SPEEDS[0],
            drill_pct: 100,
            craft_pct: 100,
            terra_pct: 100,
            power_pct: 100,
            heater_mult: 1,
            daylight: FULL,
            free_power: false,
            recipes_unlocked: [0; BUILDING_COUNT],
            meter_gain: [0; meter::COUNT],
            incinerated: 0,
            credits_gain: 0,
            produced: [0; ITEM_COUNT],
            consumed: [0; ITEM_COUNT],
            nets: Vec::new(),
        }
    }
}

/// Kinds whose machines never act in `process` (the tick loop skips them): poles, batteries
/// and radars are handled per network or per radar, incinerators only accept, wrecks wait.
pub static PASSIVE: [bool; BUILDING_COUNT] = {
    let mut a = [false; BUILDING_COUNT];
    let mut k = 0;
    while k < BUILDING_COUNT {
        a[k] = matches!(
            BUILDINGS[k].class,
            Class::None | Class::Pole | Class::Battery | Class::Radar | Class::Incinerator | Class::Wreck
        );
        k += 1;
    }
    a
};

/// Ticks one cycle takes at 100% speed.
pub fn cycle_ticks(m: &Machine) -> u32 {
    let def = &BUILDINGS[m.kind as usize];
    match def.class {
        Class::Drill => DRILL_TICKS as u32,
        Class::Pump => PUMP_TICKS as u32,
        Class::Crafter | Class::Terraformer if m.recipe != NO_RECIPE => {
            def.recipes[m.recipe as usize].ticks as u32
        }
        Class::Recycler => RECYCLE_TICKS,
        _ => 1,
    }
}

impl Machines {
    pub fn add(&mut self, mut m: Machine) -> u32 {
        self.counts[m.kind as usize] += 1;
        self.configure(&mut m);
        if m.class() == Class::DronePort {
            m.queue = self.alloc_port(m.tile);
        }
        self.list.push(m);
        (self.list.len() - 1) as u32
    }

    /// Removes machine `i` (swap-remove). Returns the tile of the machine that moved into
    /// slot `i`, if any, so the caller can fix its grid back-reference.
    pub fn remove(&mut self, i: usize) -> Option<u32> {
        let m = self.list[i];
        self.counts[m.kind as usize] -= 1;
        match m.class() {
            Class::Tunnel => self.release_queue(i),
            Class::DronePort => self.release_port(m.queue),
            _ => {}
        }
        self.list.swap_remove(i);
        self.list.get(i).map(|moved| moved.tile)
    }

    /// Recomputes a machine's speed and power draw from its shards, amplifier and the
    /// current research bonuses.
    pub fn configure(&self, m: &mut Machine) {
        let def = &BUILDINGS[m.kind as usize];
        let pct = match def.class {
            Class::Drill => self.drill_pct * PURITY_PCT[m.purity.min(2) as usize] / 100,
            Class::Pump => self.drill_pct,
            Class::Crafter => self.craft_pct,
            _ => 100,
        };
        let clock = CLOCK_PCT[m.shards.min(3) as usize];
        m.speed = (SPEED_ONE * pct * clock / 10_000).clamp(1, u16::MAX as u32) as u16;
        m.draw = match def.class {
            Class::Generator => def.power * self.power_pct / 100 * clock / 100,
            Class::Battery => 0,
            _ => {
                let amp = if m.amplified() { crate::content::AMP_POWER_MULT } else { 1 };
                def.power * CLOCK_POWER_PCT[m.shards.min(3) as usize] / 100 * amp
            }
        };
    }

    /// Re-applies `configure` to every machine (after research or loading).
    pub fn configure_all(&mut self) {
        let mut list = core::mem::take(&mut self.list);
        for m in &mut list {
            self.configure(m);
        }
        self.list = list;
    }

    /// Crafting progress in 0..=255 for rendering.
    pub fn progress(&self, m: &Machine) -> u8 {
        let full = cycle_ticks(m) * SPEED_ONE;
        (m.timer as u32 * 255 / full.max(1)).min(255) as u8
    }

    // ---- Pools ------------------------------------------------------------------------------

    pub fn alloc_queue(&mut self, i: usize) {
        if self.list[i].queue != NO_QUEUE {
            return;
        }
        let q = match self.free_queues.pop() {
            Some(q) => {
                self.queues[q as usize] = TunnelQueue::default();
                q
            }
            None => {
                self.queues.push(TunnelQueue::default());
                (self.queues.len() - 1) as u16
            }
        };
        self.list[i].queue = q;
    }

    pub fn release_queue(&mut self, i: usize) {
        let q = self.list[i].queue;
        if q != NO_QUEUE {
            self.free_queues.push(q);
            self.list[i].queue = NO_QUEUE;
        }
    }

    fn alloc_port(&mut self, tile: u32) -> u16 {
        let port = Port { tile, dest: NONE, ..Default::default() };
        match self.free_ports.pop() {
            Some(p) => {
                self.ports[p as usize] = port;
                p
            }
            None => {
                self.ports.push(port);
                (self.ports.len() - 1) as u16
            }
        }
    }

    fn release_port(&mut self, p: u16) {
        if p != NO_QUEUE {
            self.ports[p as usize].tile = NONE;
            self.free_ports.push(p);
        }
    }

    /// Items currently underground in tunnel entrance `i`.
    pub fn queue_len(&self, i: usize) -> u8 {
        let m = &self.list[i];
        if m.class() != Class::Tunnel || m.queue == NO_QUEUE { 0 } else { self.queues[m.queue as usize].len }
    }

    /// Share of its power demand machine `m` gets this tick (0..=FULL).
    #[inline]
    pub fn sat_of(&self, m: &Machine) -> u32 {
        if m.draw == 0 || m.class() == Class::Generator {
            FULL
        } else if m.net == NO_NET {
            if self.free_power { FULL } else { 0 }
        } else {
            self.nets[m.net as usize].sat
        }
    }

    /// The first unlocked standard recipe of `kind` that takes `item`.
    fn find_recipe(&self, kind: u8, item: Item) -> u8 {
        let mask = self.recipes_unlocked[kind as usize];
        for (i, rc) in BUILDINGS[kind as usize].recipes.iter().enumerate() {
            let standard = rc.unlock < ALT_BASE || rc.unlock == crate::content::FREE;
            if standard && mask & (1 << i) != 0 && rc.inputs.iter().any(|&(x, _)| x == item) {
                return i as u8;
            }
        }
        NO_RECIPE
    }

    // ---- Simulation ---------------------------------------------------------------------------

    /// Offers `item`, travelling in direction `md`, to machine `i`.
    pub fn accept(&mut self, i: u32, item: Item, md: u8) -> bool {
        let now = self.now;
        let speed = self.belt_speed;
        let sat = self.sat_of(&self.list[i as usize]);
        let m = &mut self.list[i as usize];
        let class = m.class();
        let from_front = md == dir::opposite(m.dir);
        match class {
            Class::Crafter | Class::Terraformer => {
                if from_front && class == Class::Crafter {
                    return false; // never through an output face
                }
                let (kind, recipe) = (m.kind, m.recipe);
                if recipe == NO_RECIPE {
                    let r = self.find_recipe(kind, item);
                    if r == NO_RECIPE {
                        return false;
                    }
                    self.list[i as usize].recipe = r;
                }
                let m = &mut self.list[i as usize];
                let rc = &BUILDINGS[m.kind as usize].recipes[m.recipe as usize];
                for (slot, &(x, n)) in rc.inputs.iter().enumerate() {
                    if x == item {
                        if m.inv[slot] < n.max(1) * 2 {
                            m.inv[slot] += 1;
                            return true;
                        }
                        return false;
                    }
                }
                false
            }
            Class::Splitter | Class::Sorter => {
                if md == m.dir && m.held == it::NONE {
                    m.held = item;
                    return true;
                }
                false
            }
            Class::Storage => {
                if from_front {
                    return false;
                }
                if m.aux == 0 || m.aux == NONE {
                    m.held = item;
                    m.aux = 1;
                    return true;
                }
                if m.held == item && m.aux < STORAGE_CAP {
                    m.aux += 1;
                    return true;
                }
                false
            }
            Class::Tunnel => {
                if m.rr != ENTRANCE || md != m.dir || m.aux == NONE || m.queue == NO_QUEUE {
                    return false;
                }
                let q = &mut self.queues[m.queue as usize];
                // Keep underground spacing identical to belt spacing.
                let spacing = MIN_GAP.div_ceil(speed);
                if q.len as usize >= QUEUE_CAP || (q.len > 0 && now.wrapping_sub(q.last_in) < spacing) {
                    return false;
                }
                let dist = m.inv[0] as u32; // tunnel length, set when paired
                let slot = (q.head as usize + q.len as usize) % QUEUE_CAP;
                q.items[slot] = item;
                q.ready[slot] = now.wrapping_add((dist * SUB).div_ceil(speed));
                q.len += 1;
                q.last_in = now;
                true
            }
            Class::Incinerator => {
                self.incinerated += 1;
                self.consumed[item as usize] += 1;
                true
            }
            Class::Recycler => {
                if from_front || sat == 0 || m.out >= RECYCLER_BUFFER {
                    return false;
                }
                m.out += 1;
                self.credits_gain += ITEMS[item as usize].value as u64;
                self.consumed[item as usize] += 1;
                true
            }
            Class::Generator => {
                let fuel = BUILDINGS[m.kind as usize].fuel.0;
                if fuel != it::NONE && item == fuel && m.inv[0] < FUEL_BUFFER {
                    m.inv[0] += 1;
                    return true;
                }
                false
            }
            Class::DronePort => {
                if from_front || m.aux == NONE || m.queue == NO_QUEUE {
                    return false;
                }
                self.ports[m.queue as usize].inbox.push(item)
            }
            _ => false,
        }
    }

    /// Records whether a machine made progress, for the overlay's stall counter.
    #[inline]
    fn set_status(m: &mut Machine, s: u8) {
        m.status = s;
        m.stall = if s == status::WORKING { 0 } else { m.stall.saturating_add(1) };
    }

    #[inline]
    fn demand(nets: &mut [Net], m: &Machine) {
        if m.net != NO_NET && m.draw > 0 {
            nets[m.net as usize].demand += m.draw;
        }
    }

    /// Advances machine `i` by one tick and returns what it wants to output.
    #[inline]
    pub fn process(&mut self, i: usize) -> Option<Emit> {
        let now = self.now;
        let sat = self.sat_of(&self.list[i]);
        let m = &mut self.list[i];
        let def = &BUILDINGS[m.kind as usize];
        let front = [m.dir, 0, 0];
        let amp = if m.amplified() { 2 } else { 1 };
        match def.class {
            Class::Drill | Class::Pump => {
                let item = if def.class == Class::Pump { it::WATER } else { m.aux as Item };
                let cap = 2 * amp;
                if m.out < cap {
                    Self::demand(&mut self.nets, m);
                    let speed = m.speed as u32 * sat / FULL;
                    if speed == 0 {
                        Self::set_status(m, status::NO_POWER);
                    } else {
                        m.timer = m.timer.saturating_add(speed as u16);
                        if m.timer as u32 >= cycle_ticks(m) * SPEED_ONE {
                            m.timer = 0;
                            m.out += amp;
                            self.produced[item as usize] += amp as u32;
                        }
                        Self::set_status(m, status::WORKING);
                    }
                } else {
                    Self::set_status(m, status::BLOCKED);
                }
                (m.out > 0).then_some(Emit { item, tile: m.tile, faces: front, n: 1 })
            }
            Class::Crafter => {
                if m.recipe == NO_RECIPE {
                    Self::set_status(m, status::IDLE);
                    return None;
                }
                let rc = &def.recipes[m.recipe as usize];
                let (out_item, out_n) = rc.output;
                let out_n = out_n * amp;
                let ready = rc.inputs.iter().enumerate().all(|(s, &(_, n))| m.inv[s] >= n);
                if !ready {
                    Self::set_status(m, if m.timer > 0 { status::WORKING } else { status::NO_INPUT });
                } else if m.out as u16 + out_n as u16 > out_n as u16 * 2 {
                    Self::set_status(m, status::BLOCKED);
                } else {
                    Self::demand(&mut self.nets, m);
                    let speed = m.speed as u32 * sat / FULL;
                    if speed == 0 {
                        Self::set_status(m, status::NO_POWER);
                    } else {
                        m.timer = m.timer.saturating_add(speed as u16);
                        if m.timer as u32 >= rc.ticks as u32 * SPEED_ONE {
                            m.timer = 0;
                            for (s, &(x, n)) in rc.inputs.iter().enumerate() {
                                m.inv[s] -= n;
                                self.consumed[x as usize] += n as u32;
                            }
                            m.out += out_n;
                            self.produced[out_item as usize] += out_n as u32;
                        }
                        Self::set_status(m, status::WORKING);
                    }
                }
                if m.out > 0 {
                    return Some(Emit { item: out_item, tile: m.tile, faces: front, n: 1 });
                }
                // Idle and empty on Auto: free to pick another recipe on the next input.
                if m.flags & flag::LOCKED == 0 && def.recipes.len() > 1 && m.timer == 0 && m.inv == [0; 3] {
                    m.recipe = NO_RECIPE;
                }
                None
            }
            Class::Terraformer => {
                let rc = &def.recipes[0];
                if rc.inputs.iter().enumerate().all(|(s, &(_, n))| m.inv[s] >= n) {
                    Self::demand(&mut self.nets, m);
                    let speed = m.speed as u32 * sat / FULL;
                    if speed == 0 {
                        Self::set_status(m, status::NO_POWER);
                    } else {
                        m.timer = m.timer.saturating_add(speed as u16);
                        if m.timer as u32 >= rc.ticks as u32 * SPEED_ONE {
                            m.timer = 0;
                            for (s, &(x, n)) in rc.inputs.iter().enumerate() {
                                m.inv[s] -= n;
                                self.consumed[x as usize] += n as u32;
                            }
                            let mult = if m.kind == bk::HEATER { self.heater_mult } else { 1 };
                            let points =
                                def.points as u64 * mult as u64 * amp as u64 * self.terra_pct as u64 / 100;
                            self.meter_gain[def.meter as usize] += points;
                            if m.kind == bk::HATCHERY {
                                self.meter_gain[meter::OXYGEN as usize] += points / 4;
                            }
                        }
                        Self::set_status(m, status::WORKING);
                    }
                } else {
                    Self::set_status(m, if m.timer > 0 { status::WORKING } else { status::NO_INPUT });
                }
                None
            }
            Class::Splitter => {
                if m.held == it::NONE {
                    return None;
                }
                // Try front, left, right starting from the round-robin cursor.
                let order = [m.dir, dir::ccw(m.dir), dir::cw(m.dir)];
                let s = m.rr as usize;
                Some(Emit {
                    item: m.held,
                    tile: m.tile,
                    faces: [order[s % 3], order[(s + 1) % 3], order[(s + 2) % 3]],
                    n: 3,
                })
            }
            Class::Sorter => {
                if m.held == it::NONE {
                    return None;
                }
                if m.filter == it::NONE || m.held == m.filter {
                    return Some(Emit { item: m.held, tile: m.tile, faces: front, n: 1 });
                }
                let sides = [dir::ccw(m.dir), dir::cw(m.dir)];
                let s = m.rr as usize & 1;
                Some(Emit { item: m.held, tile: m.tile, faces: [sides[s], sides[s ^ 1], 0], n: 2 })
            }
            Class::Storage => {
                if m.aux == 0 || m.aux == NONE {
                    return None;
                }
                Some(Emit { item: m.held, tile: m.tile, faces: front, n: 1 })
            }
            Class::Tunnel => {
                if m.rr != ENTRANCE || m.aux == NONE || m.queue == NO_QUEUE {
                    return None;
                }
                let q = &self.queues[m.queue as usize];
                if q.len == 0 || (q.ready[q.head as usize] as i32).wrapping_sub(now as i32) > 0 {
                    return None;
                }
                Some(Emit { item: q.items[q.head as usize], tile: m.aux, faces: front, n: 1 })
            }
            Class::Recycler => {
                if m.out > 0 {
                    Self::demand(&mut self.nets, m);
                    let speed = m.speed as u32 * sat / FULL;
                    m.timer = m.timer.saturating_add(speed as u16);
                    if m.timer as u32 >= RECYCLE_TICKS * SPEED_ONE {
                        m.timer = 0;
                        m.out -= 1;
                    }
                    Self::set_status(m, if speed == 0 { status::NO_POWER } else { status::WORKING });
                } else {
                    Self::set_status(m, status::IDLE);
                }
                None
            }
            Class::Generator => {
                if m.net == NO_NET {
                    Self::set_status(m, status::UNLINKED);
                    return None;
                }
                let fuel = def.fuel;
                let net = &mut self.nets[m.net as usize];
                if fuel.0 == it::NONE {
                    // Solar.
                    let out = m.draw * self.daylight / FULL;
                    net.supply += out;
                    Self::set_status(m, if out > 0 { status::WORKING } else { status::IDLE });
                    return None;
                }
                if (m.aux == 0 || m.aux == NONE) && m.inv[0] > 0 {
                    m.inv[0] -= 1;
                    m.aux = fuel.1.saturating_mul(60);
                    self.consumed[fuel.0 as usize] += 1;
                }
                if m.aux != 0 && m.aux != NONE {
                    net.supply += m.draw;
                    let burn = (m.draw as u64 * net.load as u64 / FULL as u64) as u32;
                    m.aux = m.aux.saturating_sub(burn.max((net.load > 0) as u32));
                    if m.aux == 0 && m.inv[0] == 0 {
                        m.aux = NONE;
                    }
                    Self::set_status(m, status::WORKING);
                } else {
                    m.aux = NONE;
                    Self::set_status(m, status::NO_FUEL);
                }
                None
            }
            Class::DronePort => {
                if m.aux == NONE {
                    Self::set_status(m, status::UNLINKED);
                }
                if m.queue == NO_QUEUE {
                    return None;
                }
                let port = &self.ports[m.queue as usize];
                port.outbox.front().map(|item| Emit { item, tile: m.tile, faces: front, n: 1 })
            }
            _ => None,
        }
    }

    /// The output from `process` was accepted through `faces[k]`.
    pub fn emitted(&mut self, i: usize, k: u8) {
        let m = &mut self.list[i];
        match m.class() {
            Class::Drill | Class::Pump | Class::Crafter => m.out -= 1,
            Class::Splitter => {
                m.held = it::NONE;
                m.rr = (m.rr + k + 1) % 3;
            }
            Class::Sorter => {
                if m.filter != it::NONE && m.held != m.filter {
                    m.rr = (m.rr + k + 1) & 1;
                }
                m.held = it::NONE;
            }
            Class::Storage => {
                m.aux -= 1;
                if m.aux == 0 {
                    m.held = it::NONE;
                }
            }
            Class::Tunnel => {
                let q = &mut self.queues[m.queue as usize];
                q.head = ((q.head as usize + 1) % QUEUE_CAP) as u8;
                q.len -= 1;
            }
            Class::DronePort => {
                self.ports[m.queue as usize].outbox.pop();
            }
            _ => {}
        }
    }

    // ---- Power ------------------------------------------------------------------------------

    /// Settles every network after the machine pass: satisfaction for consumers, load for
    /// generators, and battery charge or discharge. `core_net` gets the Core's own supply.
    pub fn settle_power(&mut self, core_net: u16, core_supply: u32) {
        for (n, net) in self.nets.iter_mut().enumerate() {
            if n as u16 == core_net {
                net.supply += core_supply;
            }
            let (d, s) = (net.demand, net.supply);
            let flow: i64;
            if s >= d {
                net.sat = FULL;
                net.load = if s == 0 { 0 } else { ((d as u64 * FULL as u64).div_ceil(s as u64)) as u32 };
                let room = net.capacity - net.stored;
                let charge = ((s - d).min(net.rate) as u64).min(room);
                net.stored += charge;
                flow = charge as i64;
                // Charging also draws on the generators.
                if s > 0 && charge > 0 {
                    net.load =
                        (((d as u64 + charge) * FULL as u64).div_ceil(s as u64)).min(FULL as u64) as u32;
                }
            } else {
                let discharge = ((d - s).min(net.rate) as u64).min(net.stored);
                net.stored -= discharge;
                flow = -(discharge as i64);
                net.sat = ((s as u64 + discharge) * FULL as u64 / d as u64) as u32;
                net.load = FULL;
            }
            net.last_demand = d;
            net.last_supply = s;
            net.last_flow = flow;
            net.demand = 0;
            net.supply = 0;
        }
    }
}
