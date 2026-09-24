//! The sky: the day/night cycle (solar power follows it) and meteor showers, which leave
//! meteorite deposits behind.

use crate::content::{PLANETS, it};
use crate::machines::FULL;
use crate::types::{NONE, TICKS_PER_SEC};
use crate::world::World;

/// One day lasts eight minutes.
pub const DAY_TICKS: u32 = 8 * 60 * TICKS_PER_SEC;
/// A new game starts in the early morning.
const DAY_OFFSET: u32 = DAY_TICKS * 28 / 100;
/// Meteors take this long from first glimpse to impact.
pub const FALL_TICKS: u32 = 90;
/// Impacts stay visible (flash and smoke) for this long.
pub const AFTERGLOW: u32 = 50;
const SHOWER_TICKS: u32 = 40 * TICKS_PER_SEC;
const WARNING_TICKS: u32 = 30 * TICKS_PER_SEC;
/// Meteors land within this distance of the Core.
const SHOWER_RADIUS: f32 = 90.0;

/// Time of day in 0..1 (0 = midnight, 0.5 = noon).
pub fn time_of_day(tick: u32) -> f32 {
    ((tick.wrapping_add(DAY_OFFSET)) % DAY_TICKS) as f32 / DAY_TICKS as f32
}

/// Sunlight in 0..=FULL: full for most of the day, dark for about a third of it.
pub fn daylight(tick: u32) -> u32 {
    let t = time_of_day(tick);
    let sun = (core::f32::consts::TAU * (t - 0.25)).sin();
    ((sun * 1.6 + 0.4).clamp(0.0, 1.0) * FULL as f32) as u32
}

#[derive(Clone, Copy, Debug)]
pub struct Meteor {
    pub tile: u32,
    pub impact: u32,
}

pub struct Sky {
    /// When the next shower starts (0 = not scheduled yet).
    pub next_shower: u32,
    /// End of the shower in progress (0 = none).
    pub shower_end: u32,
    next_meteor: u32,
    pub meteors: Vec<Meteor>,
    /// Meteorites landed so far.
    pub landed: u32,
    rng: u32,
}

impl Sky {
    pub fn new(seed: u32) -> Sky {
        Sky {
            next_shower: 0,
            shower_end: 0,
            next_meteor: 0,
            meteors: Vec::new(),
            landed: 0,
            rng: seed.wrapping_mul(0x9e37_79b9) | 1,
        }
    }

    fn next(&mut self) -> u32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        self.rng
    }

    fn unit(&mut self) -> f32 {
        (self.next() & 0xffff) as f32 / 65536.0
    }

    /// (ticks until the next shower while it is announced, shower in progress).
    pub fn announce(&self, tick: u32) -> (u32, bool) {
        let active = self.shower_end != 0;
        if !active && self.next_shower > tick && self.next_shower - tick <= WARNING_TICKS {
            (self.next_shower - tick, false)
        } else {
            (0, active)
        }
    }
}

impl World {
    /// Showers begin once the Ark's hull is done (tier 3), when meteorites start to matter.
    fn showers_enabled(&self) -> bool {
        self.core.ark_phase >= 2
    }

    pub(crate) fn tick_sky(&mut self) {
        let now = self.tick;
        self.sky.meteors.retain(|m| now <= m.impact + AFTERGLOW);
        if !self.showers_enabled() {
            return;
        }
        let interval = PLANETS[self.planet as usize].meteor_minutes * 60 * TICKS_PER_SEC;
        if self.sky.next_shower == 0 {
            self.sky.next_shower = now + 3 * 60 * TICKS_PER_SEC;
        }
        if self.sky.shower_end == 0 && now >= self.sky.next_shower {
            self.sky.shower_end = now + SHOWER_TICKS;
            self.sky.next_meteor = now;
        }
        if self.sky.shower_end != 0 {
            if now >= self.sky.shower_end {
                self.sky.shower_end = 0;
                let jitter = self.sky.next() % (interval / 3).max(1);
                self.sky.next_shower = now + interval * 5 / 6 + jitter;
            } else if now >= self.sky.next_meteor {
                self.spawn_meteor();
                self.sky.next_meteor = now + 60 + self.sky.next() % 90;
            }
        }
        for k in 0..self.sky.meteors.len() {
            if self.sky.meteors[k].impact == now {
                let t = self.sky.meteors[k].tile;
                self.land_meteor(t);
            }
        }
    }

    fn spawn_meteor(&mut self) {
        let (cx, cy) = (self.core.x + self.core.size / 2, self.core.y + self.core.size / 2);
        for _ in 0..8 {
            let a = self.sky.unit() * core::f32::consts::TAU;
            let d = 8.0 + self.sky.unit() * SHOWER_RADIUS;
            let t = self.grid.index(cx + (a.cos() * d) as i32, cy + (a.sin() * d) as i32);
            if t != NONE && !self.is_lava(t) {
                self.sky.meteors.push(Meteor { tile: t, impact: self.tick + FALL_TICKS });
                return;
            }
        }
    }

    /// A small meteorite deposit forms around the impact, on bare ground only.
    fn land_meteor(&mut self, t: u32) {
        let (x, y) = self.grid.xy(t);
        let r = 1.0 + self.sky.unit() * 0.9;
        let mut formed = false;
        for yy in y - 2..=y + 2 {
            for xx in x - 2..=x + 2 {
                let n = self.grid.index(xx, yy);
                if n == NONE {
                    continue;
                }
                let d = (((xx - x) * (xx - x) + (yy - y) * (yy - y)) as f32).sqrt();
                let n = n as usize;
                if d <= r && self.grid.res[n] == 0 && self.grid.kind[n] == crate::content::bk::EMPTY {
                    self.grid.res[n] = it::METEORITE;
                    self.grid.purity[n] = 2;
                    formed = true;
                }
            }
        }
        if formed {
            self.sky.landed += 1;
            self.res_rev = self.res_rev.wrapping_add(1);
        }
    }
}
