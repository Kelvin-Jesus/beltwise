//! The Core: storage (the currency for buildings, research and the Ark), research state,
//! terraforming meters and stages, throughput tracking, the objective chain and the
//! colony's long-term progress (credits, achievements, discoveries, launches).

use crate::content::{
    ARK, Effect, FREE, Goal, ITEM_COUNT, Item, OBJECTIVES, REPEAT_COUNT, REPEAT_FIRST, STAGES, START_STOCK,
    TECH, TECH_COUNT, it, meter, repeat_cost, tech,
};
use crate::types::TICKS_PER_SEC;

const WINDOW_SECS: usize = 60;
const TI_WINDOW: usize = 10;

/// One-off events that achievements look for (`Core::flags`).
pub mod event {
    pub const CLOCK250: u32 = 1;
    pub const AMPLIFIED: u32 = 2;
    pub const BLUEPRINT: u32 = 4;
    pub const DRONE_LINK: u32 = 8;
}

pub struct Core {
    pub x: i32,
    pub y: i32,
    pub size: i32,
    pub stored: [u32; ITEM_COUNT],
    /// Lifetime deliveries per item (objective tracking).
    pub delivered: [u32; ITEM_COUNT],
    /// Deliveries this second (drained into the rate window).
    pub delivered_now: [u32; ITEM_COUNT],
    pub total_delivered: u64,
    pub meters: [u64; meter::COUNT],
    pub ti: u64,
    pub stage: u8,
    pub researched: [bool; TECH_COUNT],
    /// Levels of the repeatable nodes.
    pub levels: [u16; REPEAT_COUNT],
    pub objective: u32,
    /// Baseline for `Goal::Deliver` progress (deliveries of the item when the goal began).
    pub obj_base: u32,
    /// Bumped whenever research, stage, objective or discoveries change (UI refresh).
    pub revision: u32,
    /// Completed Ark phases, and what has been paid into the current one.
    pub ark_phase: u8,
    pub ark_paid: [u32; 4],
    pub credits: u64,
    /// Credits earned by recycling (the Recycler achievement).
    pub credits_recycled: u64,
    pub achievements: u64,
    pub last_achievement: u32,
    pub flags: u32,
    /// Data probes found but not yet analysed, and how many were analysed so far.
    pub probes: u8,
    pub probes_used: u8,
    /// Alternate recipes unlocked (bit per alternate).
    pub alts: u32,
    /// Expedition logs recovered (they are read in order).
    pub logs: u8,
    pub salvaged: u32,
    /// Cosmetics owned (bits, see `content::cosmetic`) and in use.
    pub cosmetics: u32,
    pub belt_color: u8,
    pub trim: u8,
    /// Completed launches (each adds `LEGACY_PCT` production).
    pub legacy: u32,
    pub planet: u8,
    // Items delivered per second over a sliding minute.
    buckets: [u32; WINDOW_SECS],
    bucket: usize,
    window_sum: u32,
    ticks_in_bucket: u32,
    secs: u32,
    // Meter gains per second over a sliding window, for rates.
    gain_hist: [[u64; meter::COUNT]; TI_WINDOW],
    gain_now: [u64; meter::COUNT],
    hist_len: usize,
}

impl Core {
    pub fn new(x: i32, y: i32, size: i32) -> Self {
        let mut c = Core {
            x,
            y,
            size,
            stored: [0; ITEM_COUNT],
            delivered: [0; ITEM_COUNT],
            delivered_now: [0; ITEM_COUNT],
            total_delivered: 0,
            meters: [0; meter::COUNT],
            ti: 0,
            stage: 0,
            researched: [false; TECH_COUNT],
            levels: [0; REPEAT_COUNT],
            objective: 0,
            obj_base: 0,
            revision: 0,
            ark_phase: 0,
            ark_paid: [0; 4],
            credits: 0,
            credits_recycled: 0,
            achievements: 0,
            last_achievement: u32::MAX,
            flags: 0,
            probes: 0,
            probes_used: 0,
            alts: 0,
            logs: 0,
            salvaged: 0,
            cosmetics: 1 | 1 << 8,
            belt_color: 0,
            trim: 0,
            legacy: 0,
            planet: 0,
            buckets: [0; WINDOW_SECS],
            bucket: 0,
            window_sum: 0,
            ticks_in_bucket: 0,
            secs: 0,
            gain_hist: [[0; meter::COUNT]; TI_WINDOW],
            gain_now: [0; meter::COUNT],
            hist_len: 0,
        };
        for &(item, n) in START_STOCK {
            c.stored[item as usize] += n as u32;
        }
        c
    }

    pub fn accept(&mut self, item: Item) -> bool {
        self.stored[item as usize] += 1;
        self.delivered[item as usize] += 1;
        self.delivered_now[item as usize] += 1;
        self.total_delivered += 1;
        self.buckets[self.bucket] += 1;
        self.window_sum += 1;
        true
    }

    // ---- Costs ----------------------------------------------------------------------------

    pub fn can_afford(&self, cost: &[(Item, u16)]) -> bool {
        cost.iter().all(|&(item, n)| self.stored[item as usize] >= n as u32)
    }

    pub fn pay(&mut self, cost: &[(Item, u16)]) -> bool {
        if !self.can_afford(cost) {
            return false;
        }
        for &(item, n) in cost {
            self.stored[item as usize] -= n as u32;
        }
        true
    }

    pub fn refund(&mut self, cost: &[(Item, u16)]) {
        for &(item, n) in cost {
            self.stored[item as usize] += n as u32;
        }
    }

    // ---- Research ---------------------------------------------------------------------------

    pub fn is_unlocked(&self, research: u8) -> bool {
        research == FREE || self.researched[research as usize]
    }

    pub fn can_mine(&self, item: Item) -> bool {
        match item {
            it::TITANIUM_ORE => self.researched[tech::TITANIUM as usize],
            it::URANIUM_ORE => self.researched[tech::NUCLEAR as usize],
            it::METEORITE => self.researched[tech::XENO as usize],
            it::CRUDE_OIL => self.researched[tech::OIL as usize],
            it::NONE => false,
            _ => true,
        }
    }

    /// Current level of repeatable node `t` (0 for the others).
    pub fn level(&self, t: u8) -> u16 {
        if TECH[t as usize].repeat { self.levels[(t - REPEAT_FIRST) as usize] } else { 0 }
    }

    /// Whether node `t` could be researched right now, ignoring cost.
    pub fn is_available(&self, t: u8) -> bool {
        let def = &TECH[t as usize];
        (def.repeat || !self.researched[t as usize])
            && self.stage >= def.stage
            && self.ark_phase + 1 >= def.tier
            && def.requires.iter().all(|&r| self.researched[r as usize])
    }

    /// What node `t` costs now (repeatables grow 1.5x per level).
    pub fn tech_cost(&self, t: u8) -> impl Iterator<Item = (Item, u32)> + '_ {
        let level = self.level(t);
        TECH[t as usize].cost.iter().map(move |&(item, n)| (item, repeat_cost(n, level)))
    }

    /// Pays for and completes node `t`. The world re-derives tuning afterwards.
    pub fn research(&mut self, t: u8) -> bool {
        if (t as usize) >= TECH_COUNT || !self.is_available(t) {
            return false;
        }
        if !self.tech_cost(t).all(|(item, n)| self.stored[item as usize] >= n) {
            return false;
        }
        let cost: [(Item, u32); 4] = {
            let mut c = [(0, 0); 4];
            for (k, x) in self.tech_cost(t).enumerate().take(4) {
                c[k] = x;
            }
            c
        };
        for (item, n) in cost {
            self.stored[item as usize] -= n;
        }
        self.researched[t as usize] = true;
        if TECH[t as usize].repeat {
            self.levels[(t - REPEAT_FIRST) as usize] += 1;
        }
        self.revision += 1;
        true
    }

    /// The effects of every researched node (for re-deriving tuning).
    pub fn effects(&self) -> impl Iterator<Item = &'static Effect> + '_ {
        TECH.iter().enumerate().filter(|(t, _)| self.researched[*t]).flat_map(|(_, d)| d.effects.iter())
    }

    // ---- The Ark ----------------------------------------------------------------------------

    /// Moves everything the current phase still needs out of storage. Returns true when the
    /// phase completes.
    pub fn ark_contribute(&mut self) -> bool {
        let p = self.ark_phase as usize;
        if p >= ARK.len() {
            return false;
        }
        let mut done = true;
        let mut moved = false;
        for (k, &(item, n)) in ARK[p].cost.iter().enumerate() {
            let give = (n - self.ark_paid[k]).min(self.stored[item as usize]);
            self.stored[item as usize] -= give;
            self.ark_paid[k] += give;
            moved |= give > 0;
            done &= self.ark_paid[k] >= n;
        }
        if done {
            self.ark_phase += 1;
            self.ark_paid = [0; 4];
        }
        if moved || done {
            self.revision += 1;
        }
        done
    }

    // ---- Terraforming -----------------------------------------------------------------------

    pub fn add_meters(&mut self, gain: &[u64; meter::COUNT]) {
        for (m, g) in gain.iter().enumerate() {
            self.meters[m] += g;
            self.gain_now[m] += g;
        }
        self.ti = self.meters.iter().sum();
        while (self.stage as usize) + 1 < STAGES.len() && self.ti >= STAGES[self.stage as usize + 1].ti {
            self.stage += 1;
            let reward = &STAGES[self.stage as usize];
            self.credits += reward.credits as u64;
            self.stored[it::POWER_SHARD as usize] += reward.shards as u32;
            self.revision += 1;
        }
    }

    /// Restores meters from a save without counting them as fresh growth.
    pub fn set_meters(&mut self, meters: [u64; meter::COUNT]) {
        self.meters = meters;
        self.ti = meters.iter().sum();
        self.stage = STAGES.iter().rposition(|s| self.ti >= s.ti).unwrap_or(0) as u8;
    }

    /// Meter growth per second (x100 fixed point), averaged over the last seconds.
    pub fn meter_rate_x100(&self, m: usize) -> u64 {
        if self.hist_len == 0 {
            return 0;
        }
        let sum: u64 = self.gain_hist[..self.hist_len].iter().map(|h| h[m]).sum();
        sum * 100 / self.hist_len as u64
    }

    // ---- Clock --------------------------------------------------------------------------------

    pub fn tick(&mut self) {
        self.ticks_in_bucket += 1;
        if self.ticks_in_bucket == TICKS_PER_SEC {
            self.ticks_in_bucket = 0;
            self.secs += 1;
            self.bucket = (self.bucket + 1) % WINDOW_SECS;
            self.window_sum -= self.buckets[self.bucket];
            self.buckets[self.bucket] = 0;
            self.gain_hist.copy_within(0..TI_WINDOW - 1, 1);
            self.gain_hist[0] = self.gain_now;
            self.gain_now = [0; meter::COUNT];
            self.hist_len = (self.hist_len + 1).min(TI_WINDOW);
        }
    }

    /// Whether the meter rate window has any data (it starts empty after loading).
    pub fn has_rates(&self) -> bool {
        self.hist_len > 0
    }

    pub fn clear_rate_history(&mut self) {
        self.gain_hist = [[0; meter::COUNT]; TI_WINDOW];
        self.gain_now = [0; meter::COUNT];
        self.hist_len = 0;
    }

    /// Items per minute delivered over the last minute (extrapolated while it fills).
    pub fn rate_per_min(&self) -> u32 {
        let full = self.secs.min(WINDOW_SECS as u32 - 1);
        let window_ticks = (full * TICKS_PER_SEC + self.ticks_in_bucket).max(5 * TICKS_PER_SEC);
        (self.window_sum as u64 * 60 * TICKS_PER_SEC as u64 / window_ticks as u64) as u32
    }

    // ---- Objectives -----------------------------------------------------------------------------

    /// The current goal. After the scripted chain, goals keep coming: every other one asks
    /// for twice the Terraform Index, the rest for growing deliveries of advanced parts.
    pub fn goal(&self) -> (Goal, &'static [(Item, u16)]) {
        let i = self.objective as usize;
        if let Some(o) = OBJECTIVES.get(i) {
            return (o.goal, o.reward);
        }
        const REWARD: &[(Item, u16)] = &[(it::POWER_SHARD, 1), (it::COMPUTER, 20)];
        (endless_goal((i - OBJECTIVES.len()) as u64), REWARD)
    }

    /// Progress towards the current goal as (have, need). `built` counts placed buildings.
    pub fn goal_progress(&self, built: &[u16]) -> (u64, u64) {
        match self.goal().0 {
            Goal::Build(kind, n) => (built[kind as usize] as u64, n as u64),
            Goal::Deliver(item, n) => {
                (self.delivered[item as usize].saturating_sub(self.obj_base) as u64, n as u64)
            }
            Goal::Research(t) => (self.researched[t as usize] as u64, 1),
            Goal::ReachTi(n) => (self.ti, n),
            Goal::ReachStage(s) => ((self.stage >= s) as u64, 1),
            Goal::Ark(n) => (self.ark_phase.min(n) as u64, n as u64),
            Goal::Salvage(n) => (self.salvaged.min(n as u32) as u64, n as u64),
        }
    }

    /// Completes the current goal if it is met: pays the reward and moves on.
    pub fn check_goal(&mut self, built: &[u16]) -> bool {
        let (have, need) = self.goal_progress(built);
        if have < need {
            return false;
        }
        let (_, reward) = self.goal();
        self.refund(reward);
        self.objective += 1;
        self.begin_goal();
        self.revision += 1;
        true
    }

    /// Skips goals that are already met, without rewards (for saves from older versions).
    pub fn fast_forward(&mut self, built: &[u16]) {
        for _ in 0..OBJECTIVES.len() {
            let (have, need) = self.goal_progress(built);
            if have < need || self.objective as usize >= OBJECTIVES.len() {
                break;
            }
            self.objective += 1;
            self.begin_goal();
        }
    }

    /// Sets the baseline for the (new) current goal.
    pub fn begin_goal(&mut self) {
        self.obj_base = match self.goal().0 {
            Goal::Deliver(item, _) => self.delivered[item as usize],
            _ => 0,
        };
    }
}

/// Endless goals after the scripted chain.
fn endless_goal(k: u64) -> Goal {
    const PARTS: [u8; 6] =
        [it::COMPUTER, it::FRAME, it::QUANTUM_CORE, it::ALIEN_ALLOY, it::FUEL_ROD, it::REINFORCED_PLATE];
    if k.is_multiple_of(2) {
        Goal::ReachTi(STAGES[STAGES.len() - 1].ti << (k / 2 + 1).min(30))
    } else {
        Goal::Deliver(PARTS[(k / 2) as usize % PARTS.len()], 100 * (k as u32 / 2 + 1))
    }
}
