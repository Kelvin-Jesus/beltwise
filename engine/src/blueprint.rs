//! Blueprints: copy a rectangle of buildings (with their chosen recipes and sorter
//! filters) and paste it anywhere, rotated. JS keeps the library and share codes; the
//! engine holds the active blueprint, draws its ghost and places it as one undo step.

use crate::content::{BUILDINGS, Class, bk};
use crate::core::event;
use crate::machines::{NO_RECIPE, flag};
use crate::types::NONE;
use crate::world::World;

/// Serialized size of one cell: dx, dy (i16 each), kind, dir, recipe, filter.
pub const BP_CELL_BYTES: usize = 8;
/// Largest blueprint, in cells (and 64x64 tiles).
pub const BP_MAX: usize = 4096;
pub const BP_SIDE: i32 = 64;

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct BpCell {
    pub dx: i16,
    pub dy: i16,
    pub kind: u8,
    pub dir: u8,
    pub recipe: u8,
    pub filter: u8,
}

impl World {
    /// Copies the buildings inside the rectangle (inclusive corners) into the active
    /// blueprint. Returns the number of cells.
    pub fn bp_capture(&mut self, ax: i32, ay: i32, bx: i32, by: i32) -> u32 {
        let (x0, x1) = (ax.min(bx).max(0), ax.max(bx).min(self.grid.w - 1));
        let (y0, y1) = (ay.min(by).max(0), ay.max(by).min(self.grid.h - 1));
        let (x1, y1) = (x1.min(x0 + BP_SIDE - 1), y1.min(y0 + BP_SIDE - 1));
        self.blueprint.clear();
        for y in y0..=y1 {
            for x in x0..=x1 {
                let t = self.grid.index(x, y) as usize;
                let kind = self.grid.kind[t];
                if matches!(kind, bk::EMPTY | bk::CORE | bk::WRECK) {
                    continue;
                }
                let (mut recipe, mut filter) = (NO_RECIPE, 0);
                let e = self.grid.ent[t];
                if e != NONE {
                    let m = &self.machines.list[e as usize];
                    if m.flags & flag::LOCKED != 0 {
                        recipe = m.recipe;
                    }
                    filter = m.filter;
                }
                self.blueprint.push(BpCell {
                    dx: (x - x0) as i16,
                    dy: (y - y0) as i16,
                    kind,
                    dir: self.grid.dir[t],
                    recipe,
                    filter,
                });
            }
        }
        self.blueprint.len() as u32
    }

    pub fn bp_write(&self, out: &mut Vec<u8>) {
        out.clear();
        for c in &self.blueprint {
            out.extend_from_slice(&c.dx.to_le_bytes());
            out.extend_from_slice(&c.dy.to_le_bytes());
            out.extend_from_slice(&[c.kind, c.dir, c.recipe, c.filter]);
        }
    }

    /// Loads a blueprint serialized by `bp_write`. Invalid cells are dropped.
    pub fn bp_read(&mut self, data: &[u8]) -> u32 {
        self.blueprint.clear();
        for c in data.chunks_exact(BP_CELL_BYTES).take(BP_MAX) {
            let cell = BpCell {
                dx: i16::from_le_bytes([c[0], c[1]]),
                dy: i16::from_le_bytes([c[2], c[3]]),
                kind: c[4],
                dir: c[5] & 3,
                recipe: c[6],
                filter: c[7],
            };
            let valid = (cell.kind as usize) < BUILDINGS.len()
                && !matches!(cell.kind, bk::EMPTY | bk::CORE | bk::WRECK)
                && (0..BP_SIDE as i16).contains(&cell.dx)
                && (0..BP_SIDE as i16).contains(&cell.dy);
            if valid {
                self.blueprint.push(cell);
            }
        }
        self.blueprint.len() as u32
    }

    /// Where each cell lands when the blueprint is pasted centred on (x, y) after `rot`
    /// clockwise quarter turns: (x, y, kind, dir, cell index).
    pub fn bp_layout(&self, x: i32, y: i32, rot: u8) -> impl Iterator<Item = (i32, i32, u8, u8, usize)> + '_ {
        let rot = rot & 3;
        let turn = move |dx: i32, dy: i32| -> (i32, i32) {
            let (mut a, mut b) = (dx, dy);
            for _ in 0..rot {
                (a, b) = (-b, a);
            }
            (a, b)
        };
        let (mut minx, mut miny, mut maxx, mut maxy) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
        for c in &self.blueprint {
            let (a, b) = turn(c.dx as i32, c.dy as i32);
            (minx, miny, maxx, maxy) = (minx.min(a), miny.min(b), maxx.max(a), maxy.max(b));
        }
        let (ox, oy) = (x - (minx + maxx).div_euclid(2), y - (miny + maxy).div_euclid(2));
        self.blueprint.iter().enumerate().map(move |(k, c)| {
            let (a, b) = turn(c.dx as i32, c.dy as i32);
            (ox + a, oy + b, c.kind, (c.dir + rot) & 3, k)
        })
    }

    /// Places the blueprint (one undo step if the caller began an edit). Buildings that
    /// can't be placed (blocked, locked or unaffordable) are skipped. Returns how many
    /// were placed.
    pub fn bp_paste(&mut self, x: i32, y: i32, rot: u8) -> u32 {
        let cells: Vec<(i32, i32, u8, u8, usize)> = self.bp_layout(x, y, rot).collect();
        let (recipe, filter) = (self.place_recipe, self.place_filter);
        let mut placed = 0;
        // Machines first, so belts that feed them can orient against them.
        for pass in 0..2 {
            for &(cx, cy, kind, d, k) in &cells {
                if (kind == bk::BELT) != (pass == 1) {
                    continue;
                }
                let cell = self.blueprint[k];
                self.place_recipe = cell.recipe;
                self.place_filter =
                    if BUILDINGS[kind as usize].class == Class::Sorter { cell.filter } else { 0 };
                if self.place(cx, cy, kind, d) {
                    placed += 1;
                }
            }
        }
        self.place_recipe = recipe;
        self.place_filter = filter;
        if placed > 0 {
            self.core.flags |= event::BLUEPRINT;
        }
        placed
    }
}
