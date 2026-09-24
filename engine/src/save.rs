//! Save games: a compact little-endian binary snapshot of everything that isn't derived.
//! Terrain, deposits and wrecks are regenerated from the seed (then patched: meteorites
//! added, salvaged wrecks removed); belt and power topology are rebuilt from the grid;
//! research tuning is re-derived. Version 1 saves (before power, tiers and exploration)
//! still load: item ids are remapped and the land around the factory is revealed.

use crate::content::{
    BUILDING_COUNT, BUILDINGS, Class, ITEM_COUNT, OBJECTIVES, RESOURCE_COUNT, TECH_COUNT, bk, it, meter,
};
use crate::machines::{ENTRANCE, ItemRing, Machine, NO_QUEUE, NO_RECIPE, SPEED_ONE};
use crate::types::NONE;
use crate::world::World;

const MAGIC: &[u8; 4] = b"BWSV";
const VERSION: u16 = 2;
/// Items in version 1 saves: ids from 8 up moved by two when crude oil and meteorite joined
/// the raw resources.
const V1_ITEMS: usize = 25;

struct Writer(Vec<u8>);

impl Writer {
    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    fn u16(&mut self, v: u16) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn u64(&mut self, v: u64) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn ring<const N: usize>(&mut self, r: &ItemRing<N>) {
        self.u8(r.len);
        for item in r.iter() {
            self.u8(item);
        }
    }
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl Reader<'_> {
    fn take<const N: usize>(&mut self) -> Option<[u8; N]> {
        let bytes = self.data.get(self.pos..self.pos + N)?;
        self.pos += N;
        bytes.try_into().ok()
    }
    fn u8(&mut self) -> Option<u8> {
        Some(self.take::<1>()?[0])
    }
    fn u16(&mut self) -> Option<u16> {
        Some(u16::from_le_bytes(self.take()?))
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take()?))
    }
    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.take()?))
    }
    fn item(&mut self) -> Option<u8> {
        let v = self.u8()?;
        ((v as usize) < ITEM_COUNT).then_some(v)
    }
    fn ring<const N: usize>(&mut self, r: &mut ItemRing<N>) -> Option<()> {
        let n = self.u8()?;
        if n as usize > N {
            return None;
        }
        *r = ItemRing::default();
        for _ in 0..n {
            let item = self.item()?;
            r.push(item);
        }
        Some(())
    }
}

/// Version 1 item id to the current one.
fn v1_item(v: u8) -> u8 {
    if v >= 8 { v + 2 } else { v }
}

impl World {
    /// Serializes the world. `now` is the wall-clock time (seconds) for offline progress.
    pub fn save(&mut self, out: &mut Vec<u8>, now: f64) {
        self.sync_batteries();
        self.snapshot_rates();
        let mut w = Writer(core::mem::take(out));
        w.0.clear();
        w.0.extend_from_slice(MAGIC);
        w.u16(VERSION);
        w.u16(self.grid.w as u16);
        w.u16(self.grid.h as u16);
        w.u32(self.seed);
        w.u32(self.tick);
        w.u8(self.planet);
        w.u64(now.to_bits());

        let c = &self.core;
        w.u32(c.legacy);
        w.u32(c.objective);
        w.u32(c.obj_base);
        for m in c.meters {
            w.u64(m);
        }
        w.u64(c.total_delivered);
        let researched = c.researched.iter().enumerate().fold(0u64, |acc, (i, &r)| acc | ((r as u64) << i));
        w.u64(researched);
        for l in c.levels {
            w.u16(l);
        }
        for i in 0..ITEM_COUNT {
            w.u32(c.stored[i]);
            w.u32(c.delivered[i]);
        }
        w.u8(c.ark_phase);
        for p in c.ark_paid {
            w.u32(p);
        }
        w.u64(c.credits);
        w.u64(c.credits_recycled);
        w.u64(c.achievements);
        w.u32(c.flags);
        w.u8(c.probes);
        w.u8(c.probes_used);
        w.u32(c.alts);
        w.u8(c.logs);
        w.u32(c.salvaged);
        w.u32(c.cosmetics);
        w.u8(c.belt_color);
        w.u8(c.trim);
        for &r in &self.offline_items {
            w.u32(r);
        }
        for &r in &self.offline_meters {
            w.u64(r);
        }
        w.u32(self.sky.next_shower);
        w.u32(self.sky.landed);

        // Fog as alternating runs of unexplored / explored tiles.
        let mut runs: Vec<u32> = Vec::new();
        let (mut cur, mut len) = (0u8, 0u32);
        for &s in &self.grid.seen {
            let v = (s != 0) as u8;
            if v == cur {
                len += 1;
            } else {
                runs.push(len);
                cur = v;
                len = 1;
            }
        }
        runs.push(len);
        w.u32(runs.len() as u32);
        for r in runs {
            w.u32(r);
        }

        // Meteorite deposits (everything else regenerates from the seed).
        let meteorites: Vec<u32> =
            (0..self.grid.res.len() as u32).filter(|&t| self.grid.res[t as usize] == it::METEORITE).collect();
        w.u32(meteorites.len() as u32);
        for t in meteorites {
            w.u32(t);
        }
        // Wrecks not yet salvaged.
        let wrecks: Vec<u32> =
            (0..self.grid.kind.len() as u32).filter(|&t| self.grid.kind[t as usize] == bk::WRECK).collect();
        w.u32(wrecks.len() as u32);
        for t in wrecks {
            w.u32(t);
        }

        // Buildings (belts and machines).
        let placed: Vec<u32> = (0..self.grid.kind.len() as u32)
            .filter(|&t| !matches!(self.grid.kind[t as usize], bk::EMPTY | bk::CORE | bk::WRECK))
            .collect();
        w.u32(placed.len() as u32);
        for &t in &placed {
            w.u32(t);
            w.u8(self.grid.kind[t as usize]);
            w.u8(self.grid.dir[t as usize]);
        }

        // Machine state, keyed by tile, in tile order so equal worlds give equal bytes.
        let machines: Vec<usize> = placed
            .iter()
            .filter(|&&t| self.grid.ent[t as usize] != NONE)
            .map(|&t| self.grid.ent[t as usize] as usize)
            .collect();
        w.u32(machines.len() as u32);
        for &i in &machines {
            let m = &self.machines.list[i];
            w.u32(m.tile);
            w.u8(m.recipe);
            w.u8(m.rr);
            for v in m.inv {
                w.u8(v);
            }
            w.u8(m.out);
            w.u8(m.held);
            w.u16(m.timer);
            w.u32(m.aux);
            w.u8(m.flags);
            w.u8(m.shards);
            w.u8(m.filter);
        }

        // Items underground.
        let tunnels: Vec<usize> =
            machines.iter().copied().filter(|&i| self.machines.queue_len(i) > 0).collect();
        w.u32(tunnels.len() as u32);
        for &i in &tunnels {
            let m = &self.machines.list[i];
            let q = &self.machines.queues[m.queue as usize];
            w.u32(m.tile);
            w.u16(self.tick.wrapping_sub(q.last_in).min(u16::MAX as u32) as u16);
            w.u8(q.len);
            for k in 0..q.len as usize {
                let slot = (q.head as usize + k) % q.items.len();
                w.u8(q.items[slot]);
                w.u16(q.ready[slot].wrapping_sub(self.tick).min(u16::MAX as u32) as u16);
            }
        }

        // Drone ports: buffers and the drone.
        let ports: Vec<usize> =
            machines.iter().copied().filter(|&i| self.machines.list[i].class() == Class::DronePort).collect();
        w.u32(ports.len() as u32);
        for &i in &ports {
            let m = &self.machines.list[i];
            let p = &self.machines.ports[m.queue as usize];
            w.u32(m.tile);
            w.ring(&p.inbox);
            w.ring(&p.outbox);
            w.ring(&p.cargo);
            w.u8(p.state);
            w.u16(p.t);
            w.u16(p.dur);
            w.u16(p.wait);
            w.u32(p.dest);
        }

        // Items on belts.
        let mut items = Vec::new();
        self.belts.snapshot(&mut items);
        items.sort_unstable();
        w.u32(items.len() as u32);
        for (tile, off, item) in items {
            w.u32(tile);
            w.u8(off);
            w.u8(item);
        }
        *out = w.0;
    }

    /// Wall-clock time (seconds) stored in a save, for offline progress; 0 if unknown.
    pub fn saved_at(data: &[u8]) -> f64 {
        let mut r = Reader { data, pos: 0 };
        let ok = r.take::<4>().is_some_and(|m| &m == MAGIC) && r.u16() == Some(VERSION);
        if !ok {
            return 0.0;
        }
        r.pos += 2 + 2 + 4 + 4 + 1;
        r.u64().map(f64::from_bits).unwrap_or(0.0)
    }

    /// Builds a world from a save. Returns `None` for anything malformed.
    pub fn load(data: &[u8]) -> Option<World> {
        let mut r = Reader { data, pos: 0 };
        if &r.take::<4>()? != MAGIC {
            return None;
        }
        match r.u16()? {
            1 => Self::load_v1(&mut r),
            VERSION => Self::load_v2(&mut r),
            _ => None,
        }
    }

    fn load_v2(r: &mut Reader) -> Option<World> {
        let (w, h) = (r.u16()? as i32, r.u16()? as i32);
        if !(16..=4096).contains(&w) || !(16..=4096).contains(&h) {
            return None;
        }
        let (seed, tick, planet) = (r.u32()?, r.u32()?, r.u8()?);
        let _saved_at = r.u64()?;
        let legacy = r.u32()?;
        let mut world = World::new_planet(w, h, seed, planet, legacy);
        world.tick = tick;
        let tiles = (w * h) as u32;

        let c = &mut world.core;
        c.objective = r.u32()?;
        c.obj_base = r.u32()?;
        let mut meters = [0u64; meter::COUNT];
        for m in &mut meters {
            *m = r.u64()?;
        }
        c.set_meters(meters);
        c.total_delivered = r.u64()?;
        let researched = r.u64()?;
        for t in 0..TECH_COUNT {
            c.researched[t] = researched & (1 << t) != 0;
        }
        for l in &mut c.levels {
            *l = r.u16()?;
        }
        for i in 0..ITEM_COUNT {
            c.stored[i] = r.u32()?;
            c.delivered[i] = r.u32()?;
        }
        c.ark_phase = r.u8()?.min(crate::content::ARK.len() as u8);
        for p in &mut c.ark_paid {
            *p = r.u32()?;
        }
        c.credits = r.u64()?;
        c.credits_recycled = r.u64()?;
        c.achievements = r.u64()?;
        c.flags = r.u32()?;
        c.probes = r.u8()?;
        c.probes_used = r.u8()?;
        c.alts = r.u32()?;
        c.logs = r.u8()?;
        c.salvaged = r.u32()?;
        c.cosmetics = r.u32()?;
        c.belt_color = r.u8()?;
        c.trim = r.u8()?;
        c.revision += 1;
        for i in 0..ITEM_COUNT {
            world.offline_items[i] = r.u32()?;
        }
        for m in 0..meter::COUNT {
            world.offline_meters[m] = r.u64()?;
        }
        world.sky.next_shower = r.u32()?;
        world.sky.landed = r.u32()?;

        let runs = r.u32()?;
        let mut t = 0usize;
        for k in 0..runs {
            let len = r.u32()? as usize;
            if t + len > tiles as usize {
                return None;
            }
            world.grid.seen[t..t + len].fill(if k % 2 == 1 { 255 } else { 0 });
            t += len;
        }
        world.fog_dirty = [0, 0, w, h];
        world.fog_rev = world.fog_rev.wrapping_add(1);

        for _ in 0..r.u32()? {
            let t = r.u32()?;
            if t >= tiles {
                return None;
            }
            world.grid.res[t as usize] = it::METEORITE;
            world.grid.purity[t as usize] = 2;
        }
        let mut keep = vec![false; tiles as usize];
        for _ in 0..r.u32()? {
            let t = r.u32()?;
            if t >= tiles {
                return None;
            }
            keep[t as usize] = true;
        }
        world.remove_wrecks_except(&keep);

        world.read_buildings(r, false)?;
        world.finish_load();
        Some(world)
    }

    fn load_v1(r: &mut Reader) -> Option<World> {
        let (w, h) = (r.u16()? as i32, r.u16()? as i32);
        if !(16..=4096).contains(&w) || !(16..=4096).contains(&h) {
            return None;
        }
        let seed = r.u32()?;
        let mut world = World::new(w, h, seed);
        world.tick = r.u32()?;
        let c = &mut world.core;
        let _objective = r.u32()?;
        let _obj_base = r.u32()?;
        let mut meters = [0u64; meter::COUNT];
        for m in &mut meters {
            *m = r.u64()?;
        }
        c.set_meters(meters);
        c.total_delivered = r.u64()?;
        let researched = r.u32()?;
        for t in 0..24 {
            c.researched[t] = researched & (1 << t) != 0;
        }
        c.stored = [0; ITEM_COUNT];
        for i in 0..V1_ITEMS {
            let j = v1_item(i as u8) as usize;
            c.stored[j] = r.u32()?;
            c.delivered[j] = r.u32()?;
        }
        c.revision += 1;
        // The chain changed: start it over, skipping what is already done (no rewards).
        c.objective = 0;
        world.read_buildings(r, true)?;
        // Explore the land around the existing factory.
        let (cx, cy) = (world.core.x + 2, world.core.y + 2);
        let tiles: Vec<u32> = (0..world.grid.kind.len() as u32)
            .filter(|&t| !matches!(world.grid.kind[t as usize], bk::EMPTY | bk::CORE | bk::WRECK))
            .collect();
        for t in tiles {
            let (x, y) = world.grid.xy(t);
            world.reveal(x, y, 8);
        }
        world.reveal(cx, cy, 30);
        world.finish_load();
        world.core.begin_goal();
        world.core.fast_forward(&world.machines.counts);
        world.update_stats();
        Some(world)
    }

    /// Buildings, machine state, tunnel queues, drone ports (v2) and belt items.
    fn read_buildings(&mut self, r: &mut Reader, v1: bool) -> Option<()> {
        let tiles = self.grid.kind.len() as u32;
        let placed = r.u32()?;
        for _ in 0..placed {
            let (t, k, d) = (r.u32()?, r.u8()?, r.u8()?);
            if t >= tiles || k as usize >= BUILDING_COUNT || matches!(k, bk::EMPTY | bk::CORE | bk::WRECK) {
                return None;
            }
            let ti = t as usize;
            if v1 && self.grid.kind[ti] == bk::WRECK {
                // Wrecks are newer than this save; don't let one sit under the factory.
                let mut keep = vec![true; self.grid.kind.len()];
                keep[ti] = false;
                self.remove_wrecks_except(&keep);
            }
            if self.grid.kind[ti] != bk::EMPTY {
                return None;
            }
            self.grid.kind[ti] = k;
            self.grid.dir[ti] = d & 3;
            if k == bk::BELT {
                self.belts.add_tile(t);
            } else {
                let mut m = Machine::new(k, d & 3, t);
                match BUILDINGS[k as usize].class {
                    Class::Drill => {
                        m.aux = self.grid.res[ti] as u32;
                        m.purity = self.grid.purity[ti];
                    }
                    Class::Radar => self.radars.push(t),
                    _ => {}
                }
                let i = self.machines.add(m);
                self.grid.ent[ti] = i;
            }
        }

        let machines = r.u32()?;
        for _ in 0..machines {
            let t = r.u32()?;
            let (recipe, rr) = (r.u8()?, r.u8()?);
            let mut inv = [r.u8()?, r.u8()?, r.u8()?];
            let (mut out, mut held, mut timer, aux) = (r.u8()?, r.u8()?, r.u16()?, r.u32()?);
            let (flags, shards, filter) = if v1 { (0, 0, 0) } else { (r.u8()?, r.u8()?, r.u8()?) };
            if t >= tiles || self.grid.ent[t as usize] == NONE {
                return None;
            }
            let i = self.grid.ent[t as usize] as usize;
            let def = &BUILDINGS[self.machines.list[i].kind as usize];
            let recipe = if def.recipes.is_empty() { NO_RECIPE } else { recipe };
            if recipe != NO_RECIPE && recipe as usize >= def.recipes.len() {
                return None;
            }
            if v1 {
                held = if held == 0 { 0 } else { v1_item(held) };
                timer = timer.saturating_mul(SPEED_ONE as u16);
                if def.class == Class::Drill && held != 0 {
                    out = 1; // v1 drills held one finished ore
                    held = 0;
                }
                if def.class == Class::Tunnel {
                    inv[1] = 0;
                }
            }
            if held as usize >= ITEM_COUNT || filter as usize >= ITEM_COUNT || shards > 3 {
                return None;
            }
            let m = &mut self.machines.list[i];
            (m.recipe, m.rr, m.inv, m.out, m.held, m.timer) = (recipe, rr, inv, out, held, timer);
            (m.flags, m.shards, m.filter) = (flags, shards, filter);
            match def.class {
                Class::Drill => {
                    // Trust the save over regenerated terrain for what each drill mines.
                    let aux = if v1 && aux != NONE { v1_item(aux as u8) as u32 } else { aux };
                    if aux == 0 || aux > RESOURCE_COUNT as u32 {
                        return None;
                    }
                    m.aux = aux;
                    self.grid.res[t as usize] = aux as u8;
                }
                Class::Tunnel => {
                    if aux != NONE && (aux >= tiles || self.grid.kind[aux as usize] != bk::TUNNEL) {
                        return None;
                    }
                    m.aux = aux;
                    if m.rr == ENTRANCE && aux != NONE {
                        self.machines.alloc_queue(i);
                    }
                }
                Class::DronePort => {
                    if aux != NONE && aux >= tiles {
                        return None;
                    }
                    m.aux = aux;
                }
                _ => m.aux = aux,
            }
        }

        let tunnels = r.u32()?;
        for _ in 0..tunnels {
            let (t, since, len) = (r.u32()?, r.u16()?, r.u8()?);
            if t >= tiles || self.grid.ent[t as usize] == NONE {
                return None;
            }
            let i = self.grid.ent[t as usize] as usize;
            let q = self.machines.list[i].queue;
            if self.machines.list[i].class() != Class::Tunnel {
                return None;
            }
            if q != NO_QUEUE {
                self.machines.queues[q as usize].last_in = self.tick.wrapping_sub(since as u32);
            }
            for _ in 0..len {
                let (item, remaining) = (r.u8()?, r.u16()?);
                let item = if v1 { v1_item(item) } else { item };
                if q == NO_QUEUE || item as usize >= ITEM_COUNT {
                    return None;
                }
                let q = &mut self.machines.queues[q as usize];
                if q.len as usize >= q.items.len() {
                    return None;
                }
                let slot = (q.head as usize + q.len as usize) % q.items.len();
                q.items[slot] = item;
                q.ready[slot] = self.tick.wrapping_add(remaining as u32);
                q.len += 1;
            }
        }

        if !v1 {
            for _ in 0..r.u32()? {
                let t = r.u32()?;
                if t >= tiles || self.grid.kind[t as usize] != bk::DRONE_PORT {
                    return None;
                }
                let q = self.machines.list[self.grid.ent[t as usize] as usize].queue as usize;
                let mut p = self.machines.ports[q];
                r.ring(&mut p.inbox)?;
                r.ring(&mut p.outbox)?;
                r.ring(&mut p.cargo)?;
                (p.state, p.t, p.dur, p.wait, p.dest) =
                    (r.u8()?.min(3), r.u16()?, r.u16()?, r.u16()?, r.u32()?);
                if p.dest != NONE && p.dest >= tiles {
                    return None;
                }
                self.machines.ports[q] = p;
            }
        }

        let n = r.u32()?;
        let mut items = Vec::with_capacity(n.min(1 << 20) as usize);
        for _ in 0..n {
            let (t, off, item) = (r.u32()?, r.u8()?, r.u8()?);
            let item = if v1 { v1_item(item) } else { item };
            if t >= tiles || item as usize >= ITEM_COUNT || item == 0 {
                return None;
            }
            items.push((t, off, item));
        }
        if r.pos != r.data.len() {
            return None; // trailing garbage: not a save we wrote
        }
        self.belts.restore(&self.grid, &items);
        Some(())
    }

    /// Removes every generated wreck whose tile isn't marked in `keep`.
    fn remove_wrecks_except(&mut self, keep: &[bool]) {
        for (t, &kept) in keep.iter().enumerate() {
            if self.grid.kind[t] == bk::WRECK && !kept {
                let i = self.grid.ent[t] as usize;
                if let Some(moved) = self.machines.remove(i) {
                    self.grid.ent[moved as usize] = i as u32;
                }
                self.grid.kind[t] = bk::EMPTY;
                self.grid.ent[t] = NONE;
            }
        }
    }

    fn finish_load(&mut self) {
        self.retune();
        self.power_dirty = true;
        self.ensure_built();
        if self.core.objective as usize > OBJECTIVES.len() + 10_000 {
            self.core.objective = OBJECTIVES.len() as u32;
        }
        self.update_stats();
    }
}
