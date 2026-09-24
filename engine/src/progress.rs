//! Long-term progress: the shop, cosmetics, launching the Ark to a new planet, and the
//! production that piles up while the game is closed.

use crate::content::{ARK, ITEM_COUNT, Offer, PLANETS, SHOP, cosmetic, meter};
use crate::world::World;

/// Offline production is credited for at most this long.
pub const OFFLINE_CAP_SECS: u32 = 8 * 3600;

impl World {
    /// Buys shop entry `i` with credits.
    pub fn buy(&mut self, i: usize) -> bool {
        let Some(d) = SHOP.get(i) else { return false };
        let c = &mut self.core;
        if c.credits < d.price as u64 {
            return false;
        }
        match d.offer {
            Offer::Item(item, n) => c.stored[item as usize] += n as u32,
            Offer::Cosmetic(bit) => {
                if c.cosmetics & (1 << bit) != 0 {
                    return false;
                }
                c.cosmetics |= 1 << bit;
            }
        }
        c.credits -= d.price as u64;
        c.revision += 1;
        self.update_stats();
        true
    }

    /// Switches to an owned cosmetic: a belt colour or a Core trim.
    pub fn set_cosmetic(&mut self, bit: u8) -> bool {
        let c = &mut self.core;
        if bit >= 32 || c.cosmetics & (1 << bit) == 0 {
            return false;
        }
        if bit < cosmetic::BELT_FIRST + cosmetic::BELT_COUNT {
            c.belt_color = bit - cosmetic::BELT_FIRST;
        } else if (cosmetic::TRIM_FIRST..cosmetic::TRIM_FIRST + cosmetic::TRIM_COUNT).contains(&bit) {
            c.trim = bit - cosmetic::TRIM_FIRST;
        } else {
            return false;
        }
        self.update_stats();
        true
    }

    /// Whether `planet` can be the next destination.
    pub fn can_launch_to(&self, planet: u8) -> bool {
        self.core.ark_phase as usize >= ARK.len()
            && PLANETS.get(planet as usize).is_some_and(|p| p.unlock <= self.core.legacy + 1)
    }

    /// Launches the colony: a fresh world on `planet` that keeps credits, cosmetics,
    /// achievements and a permanent production bonus.
    pub fn launch(&self, planet: u8) -> Option<World> {
        if !self.can_launch_to(planet) {
            return None;
        }
        let legacy = self.core.legacy + 1;
        let seed = self.seed.wrapping_mul(0x2c1b_3c6d).wrapping_add(legacy.wrapping_mul(7919)) | 1;
        let mut w = World::new_planet(self.grid.w, self.grid.h, seed, planet, legacy);
        let (old, c) = (&self.core, &mut w.core);
        c.credits = old.credits;
        c.credits_recycled = old.credits_recycled;
        c.achievements = old.achievements;
        c.flags = old.flags;
        c.cosmetics = old.cosmetics;
        c.belt_color = old.belt_color;
        c.trim = old.trim;
        w.retune();
        w.update_stats();
        Some(w)
    }

    /// Records the rates offline progress will use (called when saving).
    /// Right after loading the windows are empty, so the loaded rates are kept.
    pub fn snapshot_rates(&mut self) {
        if self.rates.has_data() {
            self.offline_items.copy_from_slice(&self.rates.per_min[ITEM_COUNT * 2..]);
        }
        if self.core.has_rates() {
            for m in 0..meter::COUNT {
                self.offline_meters[m] = self.core.meter_rate_x100(m);
            }
        }
    }

    /// Credits `secs` of absence at the rates recorded in the save: deliveries to the Core
    /// and terraforming. `report` gets [seconds, Ti gained lo, hi, item, count, ...].
    pub fn apply_offline(&mut self, secs: u32) -> u32 {
        self.report.clear();
        let secs = secs.min(OFFLINE_CAP_SECS);
        if secs < 60 {
            return 0;
        }
        let ti_before = self.core.ti;
        let mut gain = [0u64; meter::COUNT];
        for (m, g) in gain.iter_mut().enumerate() {
            *g = self.offline_meters[m] * secs as u64 / 100;
        }
        self.core.add_meters(&gain);
        let ti = self.core.ti - ti_before;
        self.report.extend_from_slice(&[secs, ti as u32, (ti >> 32) as u32]);
        for item in 1..ITEM_COUNT {
            let n = (self.offline_items[item] as u64 * secs as u64 / 60).min(u32::MAX as u64 / 2) as u32;
            if n == 0 {
                continue;
            }
            let c = &mut self.core;
            c.stored[item] = c.stored[item].saturating_add(n);
            c.delivered[item] = c.delivered[item].saturating_add(n);
            c.total_delivered += n as u64;
            self.report.extend_from_slice(&[item as u32, n]);
        }
        self.core.revision += 1;
        self.update_stats();
        self.report.len() as u32
    }
}
