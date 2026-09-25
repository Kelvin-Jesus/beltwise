//! Demo worlds for the in-game guide. The UI runs a second engine instance on a small,
//! flat map with sandbox rules and builds scripted scenes on it (a drill feeding a smelter,
//! a splitter dealing ore...), so every item and building can be shown working, rendered
//! live by the game itself. Nothing here touches the player's world.

use crate::content::{ITEM_COUNT, RESOURCE_COUNT, STORAGE_CAP, bk, meter, wreck};
use crate::machines::{Machine, Machines, flag};
use crate::types::NONE;
use crate::world::{CORE_SIZE, World};

/// Elevation of demo ground: dry land, clear of ice caps, lakes and lava on every planet.
pub const GROUND: u8 = 117;

/// `World::new_demo` options.
pub mod option {
    /// Machines outside a power network run anyway.
    pub const FREE_POWER: u32 = 1;
    /// Start unexplored (radar scenes).
    pub const FOG: u32 = 2;
}

impl World {
    /// A blank demo map: flat ground, no deposits, wrecks or buildings, everything unlocked
    /// and free, at noon. The Core sits at (core_x, core_y), or nowhere if that is off the
    /// map. `meters` set the terraforming look (and whether lakes hold water).
    pub fn new_demo(
        w: i32,
        h: i32,
        planet: u8,
        core: (i32, i32),
        options: u32,
        meters: [u64; meter::COUNT],
    ) -> World {
        let mut world = World::new_planet(w, h, 0x5eed, planet, 0);
        let g = &mut world.grid;
        g.kind.fill(bk::EMPTY);
        g.dir.fill(0);
        g.res.fill(0);
        g.purity.fill(1);
        g.ent.fill(NONE);
        g.seen.fill(if options & option::FOG != 0 { 0 } else { 255 });
        for t in world.terrain.chunks_exact_mut(4) {
            t[0] = GROUND;
        }
        world.machines = Machines::default();
        world.radars.clear();
        let (cx, cy) = core;
        let on_map = cx >= 0 && cy >= 0 && cx + CORE_SIZE <= w && cy + CORE_SIZE <= h;
        // Far off the map, the Core neither draws, powers nor accepts anything.
        let (cx, cy) = if on_map { (cx, cy) } else { (-1000, -1000) };
        world.core.x = cx;
        world.core.y = cy;
        if on_map {
            for y in cy..cy + CORE_SIZE {
                for x in cx..cx + CORE_SIZE {
                    let t = world.grid.index(x, y) as usize;
                    world.grid.kind[t] = bk::CORE;
                }
            }
        }
        world.core.set_meters(meters);
        world.sandbox = true;
        world.machines.free_power = options & option::FREE_POWER != 0;
        world.tick = crate::sky::NOON;
        world.fog_dirty = [0, 0, w, h];
        world.fog_rev = world.fog_rev.wrapping_add(1);
        world.map_rev = world.map_rev.wrapping_add(1);
        world.res_rev = world.res_rev.wrapping_add(1);
        world.power_dirty = true;
        world.retune();
        world.update_stats();
        world
    }

    /// Paints a deposit (demo worlds only).
    pub fn demo_ore(&mut self, x: i32, y: i32, item: u8, purity: u8) -> bool {
        let t = self.grid.index(x, y);
        if !self.sandbox || t == NONE || item > RESOURCE_COUNT {
            return false;
        }
        self.grid.res[t as usize] = item;
        self.grid.purity[t as usize] = purity.min(2);
        self.res_rev = self.res_rev.wrapping_add(1);
        true
    }

    /// Sets the ground elevation of a tile (0..=255): low ground holds lakes once the
    /// planet has water (demo worlds only).
    pub fn demo_ground(&mut self, x: i32, y: i32, elevation: u8) -> bool {
        let t = self.grid.index(x, y);
        if !self.sandbox || t == NONE {
            return false;
        }
        self.terrain[t as usize * 4] = elevation;
        true
    }

    /// Turns the Storage at (x, y) into a bottomless supply of `item` (demo worlds only).
    pub fn demo_source(&mut self, x: i32, y: i32, item: u8) -> bool {
        let Some(i) = self.machine_at(x, y) else { return false };
        let m = &mut self.machines.list[i];
        if !self.sandbox || m.kind != bk::STORAGE || item == 0 || item as usize >= ITEM_COUNT {
            return false;
        }
        m.held = item;
        m.aux = STORAGE_CAP;
        m.flags |= flag::SOURCE;
        true
    }

    /// Runs every machine `pct` percent as fast (demo worlds only).
    pub fn demo_speed(&mut self, pct: u32) {
        if self.sandbox {
            self.machines.demo_pct = pct.clamp(10, 1000);
            self.machines.configure_all();
        }
    }

    /// Drops a wreck on an empty tile (demo worlds only).
    pub fn demo_wreck(&mut self, x: i32, y: i32) -> bool {
        let t = self.grid.index(x, y);
        if !self.sandbox || t == NONE || self.grid.kind[t as usize] != bk::EMPTY {
            return false;
        }
        let mut m = Machine::new(bk::WRECK, 0, t);
        m.aux = (wreck::LOG as u32) << 8;
        self.grid.kind[t as usize] = bk::WRECK;
        let i = self.machines.add(m);
        self.grid.ent[t as usize] = i;
        self.map_rev = self.map_rev.wrapping_add(1);
        true
    }
}
