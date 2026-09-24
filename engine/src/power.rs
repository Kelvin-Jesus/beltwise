//! Power networks: which poles connect, what each network covers, and which network every
//! machine belongs to. Rebuilt lazily after edits (never per tick): a union-find over the
//! Core and the poles, then one coverage fill.

use crate::content::{BUILDINGS, CORE_RADIUS, Class, POLE_LINK, POLE_RADIUS};
use crate::machines::{NO_NET, Net};
use crate::types::NONE;
use crate::world::World;

fn find(parent: &mut [u32], mut i: u32) -> u32 {
    while parent[i as usize] != i {
        let p = parent[parent[i as usize] as usize];
        parent[i as usize] = p;
        i = p;
    }
    i
}

fn union(parent: &mut [u32], a: u32, b: u32) {
    let (ra, rb) = (find(parent, a), find(parent, b));
    if ra != rb {
        parent[ra.max(rb) as usize] = ra.min(rb);
    }
}

impl World {
    /// Hands every network's battery charge back to its batteries, so the charge survives
    /// networks merging or splitting (and is saved per battery).
    pub fn sync_batteries(&mut self) {
        let nets = &self.machines.nets;
        let mut left: Vec<(u64, u32)> = nets.iter().map(|n| (n.stored, n.batteries)).collect();
        for m in &mut self.machines.list {
            if m.class() != Class::Battery {
                continue;
            }
            if m.net == NO_NET || m.net as usize >= left.len() {
                continue; // keeps its own charge
            }
            let (stored, count) = &mut left[m.net as usize];
            let share = if *count <= 1 { *stored } else { *stored / *count as u64 };
            m.aux = share.min(u32::MAX as u64) as u32;
            *stored -= share;
            *count = count.saturating_sub(1);
        }
    }

    pub fn rebuild_power(&mut self) {
        self.sync_batteries();
        let (w, h) = (self.grid.w, self.grid.h);
        let (ccx, ccy) = (self.core.x + self.core.size / 2, self.core.y + self.core.size / 2);

        // Nodes: 0 is the Core, then every pole.
        let mut pos: Vec<(i32, i32)> = vec![(ccx, ccy)];
        for m in &self.machines.list {
            if m.class() == Class::Pole {
                pos.push(self.grid.xy(m.tile));
            }
        }
        let n = pos.len();
        let mut parent: Vec<u32> = (0..n as u32).collect();

        // Poles inside the Core's field join the Core; poles within reach join each other.
        // A coarse bucket grid keeps this near-linear for long pole lines.
        let cell = POLE_LINK.max(1);
        let (bw, bh) = ((w + cell - 1) / cell, (h + cell - 1) / cell);
        let mut buckets: Vec<Vec<u32>> = vec![Vec::new(); (bw * bh) as usize];
        for (i, &(x, y)) in pos.iter().enumerate().skip(1) {
            if (x - ccx).abs() <= CORE_RADIUS && (y - ccy).abs() <= CORE_RADIUS {
                union(&mut parent, 0, i as u32);
            }
            let (cx, cy) = (x / cell, y / cell);
            for by in (cy - 1).max(0)..=(cy + 1).min(bh - 1) {
                for bx in (cx - 1).max(0)..=(cx + 1).min(bw - 1) {
                    for &j in &buckets[(by * bw + bx) as usize] {
                        let (ox, oy) = pos[j as usize];
                        let (dx, dy) = (ox - x, oy - y);
                        if dx * dx + dy * dy <= POLE_LINK * POLE_LINK {
                            union(&mut parent, i as u32, j);
                        }
                    }
                }
            }
            buckets[(cy * bw + cx) as usize].push(i as u32);
        }

        // Compact network ids, Core's network first.
        let mut id = vec![NO_NET; n];
        let mut count = 0u16;
        for i in 0..n {
            let r = find(&mut parent, i as u32) as usize;
            if id[r] == NO_NET {
                id[r] = count;
                count += 1;
            }
            id[i] = id[r];
        }

        // Coverage: the Core's field, then each pole's square (first come, first served).
        self.power_cov.fill(NO_NET);
        let paint = |cov: &mut Vec<u16>, map: &mut Vec<u8>, cx: i32, cy: i32, r: i32, net: u16| {
            for y in (cy - r).max(0)..=(cy + r).min(h - 1) {
                let row = (y * w) as usize;
                for x in (cx - r).max(0)..=(cx + r).min(w - 1) {
                    let t = row + x as usize;
                    if cov[t] == NO_NET {
                        cov[t] = net;
                        map[t] = 1;
                    }
                }
            }
        };
        self.power_map.fill(0);
        paint(&mut self.power_cov, &mut self.power_map, ccx, ccy, CORE_RADIUS, id[0]);
        for (i, &(x, y)) in pos.iter().enumerate().skip(1) {
            paint(&mut self.power_cov, &mut self.power_map, x, y, POLE_RADIUS, id[i]);
        }

        // Networks and membership.
        let mut nets = vec![Net { sat: 1024, ..Default::default() }; count as usize];
        nets[id[0] as usize].has_core = true;
        for m in &mut self.machines.list {
            m.net = if m.tile == NONE { NO_NET } else { self.power_cov[m.tile as usize] };
            if m.class() == Class::Battery && m.net != NO_NET {
                let def = &BUILDINGS[m.kind as usize];
                let net = &mut nets[m.net as usize];
                net.batteries += 1;
                net.capacity += def.store as u64 * 60;
                net.rate += def.power;
                net.stored += if m.aux == NONE { 0 } else { m.aux as u64 };
            }
        }
        for net in &mut nets {
            net.stored = net.stored.min(net.capacity);
        }
        // Keep last tick's satisfaction where a network survives, so machines don't stutter.
        let old = core::mem::take(&mut self.machines.nets);
        for (k, net) in nets.iter_mut().enumerate() {
            if let Some(o) = old.get(k) {
                net.sat = o.sat;
                net.load = o.load;
            }
        }
        self.machines.nets = nets;
        self.core_net = id[0];
        self.power_dirty = false;
        self.power_rev = self.power_rev.wrapping_add(1);
    }
}
