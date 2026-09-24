//! Exploration: fog of war, radars, and the wrecks of the first expedition (supply caches,
//! power shards, amplifiers, story logs and data probes that unlock alternate recipes).

use crate::content::{ALT_COUNT, LOGS, bk, it, wreck};
use crate::machines::{FULL, NO_NET, SPEED_ONE, status};
use crate::types::NONE;
use crate::world::World;

/// Radars reveal up to this radius, one tile every half second.
pub const RADAR_RANGE: u32 = 70;
const RADAR_STEP_TICKS: u32 = 30;
/// Credits paid for a probe when every alternate is already known.
const SPARE_PROBE_CREDITS: u32 = 2000;

/// What `salvage` reports (first word of `World::report`).
pub mod found {
    pub const NOTHING: u32 = 0;
    pub const CACHE: u32 = 1;
    pub const PROBE: u32 = 2;
    pub const LOG: u32 = 3;
    pub const SHARDS: u32 = 4;
    pub const AMPLIFIER: u32 = 5;
    pub const CREDITS: u32 = 6;
}

impl World {
    /// Clears the fog in a disc. Tracks the changed area so JS only re-uploads that part.
    pub fn reveal(&mut self, cx: i32, cy: i32, r: i32) {
        let (w, h) = (self.grid.w, self.grid.h);
        let r2 = r * r + r;
        let (x0, y0, x1, y1) = ((cx - r).max(0), (cy - r).max(0), (cx + r).min(w - 1), (cy + r).min(h - 1));
        let mut changed = false;
        for y in y0..=y1 {
            let row = (y * w) as usize;
            for x in x0..=x1 {
                let (dx, dy) = (x - cx, y - cy);
                if dx * dx + dy * dy <= r2 && self.grid.seen[row + x as usize] == 0 {
                    self.grid.seen[row + x as usize] = 255;
                    changed = true;
                }
            }
        }
        if changed {
            self.fog_rev = self.fog_rev.wrapping_add(1);
            let d = &mut self.fog_dirty;
            *d = [d[0].min(x0), d[1].min(y0), d[2].max(x1 + 1), d[3].max(y1 + 1)];
        }
    }

    /// Radars draw power while they scan and widen their revealed circle over time.
    pub(crate) fn tick_radars(&mut self) {
        let mut reveals: Vec<(i32, i32, i32)> = Vec::new();
        for k in 0..self.radars.len() {
            let t = self.radars[k];
            let i = self.grid.ent[t as usize] as usize;
            let sat = self.machines.sat_of(&self.machines.list[i]);
            let ms = &mut self.machines;
            let m = &mut ms.list[i];
            if m.aux >= RADAR_RANGE {
                m.status = status::IDLE;
                m.stall = 0;
                continue;
            }
            if m.net != NO_NET {
                ms.nets[m.net as usize].demand += m.draw;
            }
            if sat == 0 {
                m.status = status::NO_POWER;
                m.stall = m.stall.saturating_add(1);
                continue;
            }
            m.timer = m.timer.saturating_add((m.speed as u32 * sat / FULL) as u16);
            if m.timer as u32 >= RADAR_STEP_TICKS * SPEED_ONE {
                m.timer = 0;
                m.aux += 1;
                let (x, y) = self.grid.xy(t);
                reveals.push((x, y, m.aux as i32));
            }
            m.status = status::WORKING;
            m.stall = 0;
        }
        for (x, y, r) in reveals {
            self.reveal(x, y, r);
        }
    }

    /// Salvages the wreck at (x, y). Returns what was found (see [`found`]); details land
    /// in `report`: the kind, then item/count pairs or a log index.
    pub fn salvage(&mut self, x: i32, y: i32) -> u32 {
        let t = self.grid.index(x, y);
        if t == NONE || self.grid.kind[t as usize] != bk::WRECK || self.grid.seen[t as usize] == 0 {
            return found::NOTHING;
        }
        let i = self.grid.ent[t as usize] as usize;
        let contents = (self.machines.list[i].aux >> 8) as u8;
        if let Some(moved) = self.machines.remove(i) {
            self.grid.ent[moved as usize] = i as u32;
        }
        self.grid.ent[t as usize] = NONE;
        self.grid.kind[t as usize] = bk::EMPTY;
        self.map_rev = self.map_rev.wrapping_add(1);
        self.reveal(x, y, 5);
        let c = &mut self.core;
        c.salvaged += 1;
        c.revision += 1;
        self.report.clear();
        let (cx, cy) = (c.x + c.size / 2, c.y + c.size / 2);
        let dist = (((x - cx) * (x - cx) + (y - cy) * (y - cy)) as f32).sqrt();
        let mut kind = contents;
        if kind == wreck::LOG && c.logs as usize >= LOGS.len() {
            kind = wreck::CACHE;
        }
        let result = match kind {
            wreck::PROBE => {
                if self.probe_options()[0] == u8::MAX {
                    self.core.credits += SPARE_PROBE_CREDITS as u64;
                    self.report.extend_from_slice(&[found::CREDITS, SPARE_PROBE_CREDITS]);
                    found::CREDITS
                } else {
                    self.core.probes += 1;
                    self.report.push(found::PROBE);
                    found::PROBE
                }
            }
            wreck::LOG => {
                let c = &mut self.core;
                self.report.extend_from_slice(&[found::LOG, c.logs as u32]);
                c.logs += 1;
                found::LOG
            }
            wreck::SHARDS => {
                self.core.stored[it::POWER_SHARD as usize] += 2;
                self.report.extend_from_slice(&[found::SHARDS, it::POWER_SHARD as u32, 2]);
                found::SHARDS
            }
            wreck::AMPLIFIER => {
                self.core.stored[it::AMPLIFIER as usize] += 1;
                self.report.extend_from_slice(&[found::AMPLIFIER, it::AMPLIFIER as u32, 1]);
                found::AMPLIFIER
            }
            _ => {
                // Supply caches hold better goods the further out they lie.
                let loot: &[(u8, u32)] = if dist < 60.0 {
                    &[(it::IRON_PLATE, 60), (it::COPPER_WIRE, 60), (it::GEAR, 20)]
                } else if dist < 120.0 {
                    &[(it::STEEL, 60), (it::CIRCUIT, 40), (it::MOTOR, 15)]
                } else {
                    &[(it::COMPUTER, 12), (it::REINFORCED_PLATE, 40), (it::BATTERY, 20)]
                };
                self.report.push(found::CACHE);
                for &(item, n) in loot {
                    self.core.stored[item as usize] += n;
                    self.report.extend_from_slice(&[item as u32, n]);
                }
                found::CACHE
            }
        };
        self.update_stats();
        result
    }

    /// The three alternates the next data probe offers (`u8::MAX` = none left).
    pub fn probe_options(&self) -> [u8; 3] {
        let mut locked: Vec<u8> = (0..ALT_COUNT as u8).filter(|&k| self.core.alts & (1 << k) == 0).collect();
        let mut rng = self.seed ^ (self.core.probes_used as u32 + 1).wrapping_mul(0x9e37_79b9);
        let mut out = [u8::MAX; 3];
        for slot in &mut out {
            if locked.is_empty() {
                break;
            }
            rng ^= rng << 13;
            rng ^= rng >> 17;
            rng ^= rng << 5;
            *slot = locked.swap_remove(rng as usize % locked.len());
        }
        out
    }

    /// Analyses a data probe, unlocking alternate recipe `k` (one of `probe_options`).
    pub fn choose_alt(&mut self, k: u8) -> bool {
        if self.core.probes == 0 || k == u8::MAX || !self.probe_options().contains(&k) {
            return false;
        }
        let c = &mut self.core;
        c.alts |= 1 << k;
        c.probes -= 1;
        c.probes_used += 1;
        c.revision += 1;
        self.retune();
        self.update_stats();
        true
    }
}
