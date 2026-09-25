//! Per-frame instance buffer: one 16-byte record per visible sprite, written straight into
//! linear memory. JS uploads it with a single `bufferSubData` and draws it with a single
//! instanced draw call. Record order is draw order: belts, machines, the Core, items,
//! drones and meteors, weather, night lights, then placement previews. Items come early
//! among the small sprites so that, if the buffer ever fills up, what gets dropped is
//! decoration rather than the factory.

use crate::content::{BUILDINGS, Class, STAGES, bk, stage};
use crate::machines::{EXIT, Machine, NO_RECIPE, status};
use crate::sky::{AFTERGLOW, FALL_TICKS};
use crate::types::{NONE, TICKS_PER_SEC, dir};
use crate::world::{CORE_SIZE, World, stat};

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct Instance {
    /// Sprite center in tile units.
    pub x: f32,
    pub y: f32,
    /// See [`sprite`].
    pub sprite: u8,
    /// Quarter turns clockwise (0 = facing east) in the low two bits; sprite-specific above.
    pub rot: u8,
    /// Edge length in 1/16 tile.
    pub size: u8,
    /// Sprite-specific: crafting progress, stage progress, ...
    pub param: u8,
    /// RGBA8 tint (little-endian 0xAABBGGRR). White = untinted. Lights use alpha 0, which
    /// makes them additive under premultiplied blending.
    pub color: u32,
}

pub const INSTANCE_CAP: usize = 1 << 16;
/// Slots kept free for overlays drawn after items.
const OVERLAY_RESERVE: usize = 4096 + 512;
const _: () = assert!(core::mem::size_of::<Instance>() == 16);

/// Sprite ids. Atlas cells (16x8): items at their id (1..=63), buildings at 64 + kind,
/// a few extra cells at the end. Ids from 128 up are shapes drawn by the shader.
pub mod sprite {
    pub const BUILDING: u8 = 64;
    pub const DRONE: u8 = 126;
    pub const TUNNEL_EXIT: u8 = 127;
    /// A straight belt; with a param, a stub of belt running under a building (see `STUB`).
    pub const BELT: u8 = 128;
    pub const BELT_RIGHT: u8 = 129;
    pub const BELT_LEFT: u8 = 130;
    /// rot bits 2..4: Ark phase, bits 5..6: trim. color: launch progress in red.
    pub const CORE: u8 = 131;
    pub const DELETE: u8 = 132;
    /// Additive glow; param = strength.
    pub const GLOW: u8 = 133;
    /// Problem badge; param = machine status.
    pub const STATUS: u8 = 134;
    /// Falling meteor; param = fall progress.
    pub const METEOR: u8 = 135;
    /// Impact flash; param = age.
    pub const FLASH: u8 = 136;
    pub const RAIN: u8 = 137;
    pub const SNOW: u8 = 138;
    /// param = wing phase.
    pub const BIRD: u8 = 139;
    pub const SHADOW: u8 = 140;
    /// Tinted by the instance colour; param = wing phase.
    pub const BUTTERFLY: u8 = 141;
}

const WHITE: u32 = 0xffff_ffff;
const GHOST_OK: u32 = 0xa0ff_ffff;
const GHOST_BAD: u32 = 0xa05a_5aff;
const UNPAIRED: u32 = 0xff90_90ff;
const SHADOW: u32 = 0x5a00_0000;
const TILE: u8 = 16;
const ITEM: u8 = 8;
/// A machine must be stuck this long (ticks) before it gets a problem badge.
const STALL_BADGE: u8 = 90;
/// Belts fill their tiles, but buildings sit inset in theirs; where a belt feeds a building
/// or leaves one, a stub of belt this long (1/255 tile) runs under it so the two touch.
/// The Core's corners are rounder, so its stubs reach further.
const STUB: u8 = 46;
const CORE_STUB: u8 = 77;
/// Stub flags above the rotation bits: the building is behind the stub rather than ahead,
/// and (machine to machine) there is a building at both ends.
const STUB_BEHIND: u8 = 4;
const STUB_BOTH: u8 = 8;

#[derive(Clone, Copy, Default, Debug)]
pub struct Cursor {
    pub x: i32,
    pub y: i32,
    pub kind: u8,
    pub dir: u8,
    pub flags: u8,
}

pub mod cursor {
    pub const VISIBLE: u8 = 1;
    pub const DELETE: u8 = 2;
    /// Paste the active blueprint; `dir` is its rotation.
    pub const PASTE: u8 = 4;
}

/// Level-of-detail and overlay flags for [`World::render`].
pub mod lod {
    pub const ITEMS: u32 = 1;
    pub const STRUCTURES: u32 = 2;
    /// Badges on machines that are stuck (no power, no input, blocked).
    pub const STATUS: u32 = 4;
    /// Night-time lights.
    pub const LIGHTS: u32 = 8;
    /// Rain, snow and birds.
    pub const WEATHER: u32 = 16;
    pub const ALL: u32 = ITEMS | STRUCTURES;
}

/// Glow colours by build category (little-endian RGBA, alpha 0 = additive).
const LIGHT: [u32; 5] = [0x00b4_c8e0, 0x0040_a0ff, 0x00ff_c070, 0x0060_e8ff, 0x0090_ffa0];

#[inline(always)]
fn push(out: &mut Vec<Instance>, inst: Instance) {
    // Never grow: JS holds a view onto this buffer.
    if out.len() < out.capacity() {
        out.push(inst);
    }
}

#[inline(always)]
fn sprite_at(x: f32, y: f32, sprite: u8, rot: u8, size: u8, param: u8, color: u32) -> Instance {
    Instance { x, y, sprite, rot, size, param, color }
}

fn hash(a: u32, b: u32) -> u32 {
    let mut h = a.wrapping_mul(0x27d4_eb2d) ^ b.wrapping_mul(0x1656_67b1);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^ (h >> 12)
}

fn unit(h: u32) -> f32 {
    (h & 0xffff) as f32 / 65536.0
}

/// Whether machine `m` takes items in through its side facing direction `face`.
fn takes_from(m: &Machine, face: u8) -> bool {
    let behind = face == dir::opposite(m.dir);
    match m.class() {
        Class::Crafter | Class::Storage | Class::DronePort | Class::Recycler => face != m.dir,
        Class::Terraformer | Class::Incinerator => true,
        Class::Generator => BUILDINGS[m.kind as usize].fuel.0 != 0,
        Class::Splitter | Class::Sorter => behind,
        Class::Tunnel => behind && m.rr != EXIT,
        _ => false,
    }
}

/// The sides (a bit per direction) machine `m` puts items out through.
fn out_faces(m: &Machine) -> u8 {
    let front = 1 << m.dir;
    match m.class() {
        Class::Drill | Class::Pump | Class::Crafter | Class::Storage | Class::DronePort => front,
        Class::Tunnel if m.rr == EXIT => front,
        Class::Splitter | Class::Sorter => front | 1 << dir::ccw(m.dir) | 1 << dir::cw(m.dir),
        _ => 0,
    }
}

impl World {
    /// Fills the instance buffer for the view rectangle (tile units) and returns the count.
    /// `alpha` in [0, 1) interpolates moving things between the last two ticks. `flags`
    /// picks detail and overlays (see [`lod`]): far out, belts and machines come from the
    /// ground pass's tile map instead, so zoomed-out frames cost the same for any factory.
    pub fn render(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, alpha: f32, flags: u32) -> u32 {
        self.ensure_built();
        let mut out = core::mem::take(&mut self.instances);
        out.clear();
        let g = &self.grid;
        let tx0 = (x0.floor() as i32 - 1).max(0);
        let ty0 = (y0.floor() as i32 - 1).max(0);
        let tx1 = (x1.ceil() as i32 + 1).min(g.w);
        let ty1 = (y1.ceil() as i32 + 1).min(g.h);
        let dark = 1000u32.saturating_sub(self.stats[stat::DAYLIGHT]);
        let lights = flags & lod::LIGHTS != 0 && dark > 150;
        let mut glows = core::mem::take(&mut self.glows);
        glows.clear();

        if flags & lod::STRUCTURES != 0 {
            // One scan of the visible window: belts go straight out, machines are collected
            // so they draw on top. Cost scales with the screen, not with the factory.
            let mut visible = core::mem::take(&mut self.visible_machines);
            visible.clear();
            for y in ty0..ty1 {
                let row = y * g.w;
                for x in tx0..tx1 {
                    let t = (row + x) as usize;
                    let k = g.kind[t];
                    if k == bk::BELT {
                        let (out_d, in_d) = (g.dir[t], self.belts.in_dir[t]);
                        let (spr, rot) = if in_d == out_d {
                            (sprite::BELT, out_d)
                        } else if out_d == (in_d + 1) & 3 {
                            (sprite::BELT_RIGHT, in_d)
                        } else {
                            (sprite::BELT_LEFT, in_d)
                        };
                        push(&mut out, sprite_at(x as f32 + 0.5, y as f32 + 0.5, spr, rot, TILE, 0, WHITE));
                        // A belt feeding a building runs a little way under it.
                        let n = g.step(t as u32, out_d);
                        let len = match if n == NONE { bk::EMPTY } else { g.kind[n as usize] } {
                            bk::CORE => CORE_STUB,
                            nk if nk > bk::CORE
                                && takes_from(
                                    &self.machines.list[g.ent[n as usize] as usize],
                                    dir::opposite(out_d),
                                ) =>
                            {
                                STUB
                            }
                            _ => 0,
                        };
                        if len > 0 {
                            let (sx, sy) = (
                                x as f32 + 0.5 + 0.5 * dir::dx(out_d) as f32,
                                y as f32 + 0.5 + 0.5 * dir::dy(out_d) as f32,
                            );
                            push(&mut out, sprite_at(sx, sy, sprite::BELT, out_d, TILE, len, WHITE));
                        }
                    } else if k > bk::CORE && g.seen[t] != 0 {
                        visible.push(g.ent[t]);
                    }
                }
            }

            let w = g.w as u32;
            let ms = &self.machines;
            // Outputs run a stub under the building they leave too (and, when a machine
            // hands items straight to another, under both), drawn before the machines.
            for &i in &visible {
                let m = &ms.list[i as usize];
                let faces = out_faces(m);
                if faces == 0 {
                    continue;
                }
                let (cx, cy) = ((m.tile % w) as f32 + 0.5, (m.tile / w) as f32 + 0.5);
                for d in 0..4u8 {
                    let n = g.step(m.tile, d);
                    if faces & (1 << d) == 0 || n == NONE {
                        continue;
                    }
                    let flags = match g.kind[n as usize] {
                        bk::BELT if g.dir[n as usize] != dir::opposite(d) => STUB_BEHIND,
                        bk::CORE => STUB_BEHIND | STUB_BOTH,
                        nk if nk > bk::CORE
                            && takes_from(&ms.list[g.ent[n as usize] as usize], dir::opposite(d)) =>
                        {
                            STUB_BEHIND | STUB_BOTH
                        }
                        _ => continue,
                    };
                    let (sx, sy) = (cx + 0.5 * dir::dx(d) as f32, cy + 0.5 * dir::dy(d) as f32);
                    push(&mut out, sprite_at(sx, sy, sprite::BELT, d | flags, TILE, STUB, WHITE));
                }
            }

            // Machines, with a badge showing what each one makes.
            for &i in &visible {
                let m = &ms.list[i as usize];
                let (cx, cy) = ((m.tile % w) as f32 + 0.5, (m.tile / w) as f32 + 0.5);
                let def = &BUILDINGS[m.kind as usize];
                let (spr, tint) = match def.class {
                    Class::Tunnel if m.rr == EXIT => (sprite::TUNNEL_EXIT, WHITE),
                    Class::Tunnel if m.aux == NONE => (sprite::BUILDING + m.kind, UNPAIRED),
                    _ => (sprite::BUILDING + m.kind, WHITE),
                };
                // Bits 2..3 of rot carry shards, bit 4 the amplifier (shader adds pips).
                let extra = (m.shards.min(3) << 2) | ((m.amplified() as u8) << 4);
                // Wrecks are drawn larger so they stand out as places to visit.
                let size = if def.class == Class::Wreck { TILE + TILE / 2 } else { TILE };
                push(&mut out, sprite_at(cx, cy, spr, m.dir | extra, size, ms.progress(m), tint));
                let badge = match def.class {
                    Class::Drill => m.aux as u8,
                    Class::Crafter if m.recipe != NO_RECIPE => def.recipes[m.recipe as usize].output.0,
                    Class::Sorter | Class::Storage if m.held != 0 || m.filter != 0 => {
                        if def.class == Class::Sorter { m.filter } else { m.held }
                    }
                    _ => 0,
                };
                if badge != 0 {
                    push(&mut out, sprite_at(cx + 0.25, cy - 0.25, badge, 0, 7, 0, WHITE));
                }
                if flags & lod::STATUS != 0 && m.stall >= STALL_BADGE {
                    let s = m.status;
                    let show =
                        matches!(s, status::NO_POWER | status::BLOCKED | status::NO_FUEL | status::UNLINKED)
                            || (s == status::NO_INPUT
                                && m.recipe != NO_RECIPE
                                && def.class != Class::Terraformer);
                    if show {
                        push(&mut out, sprite_at(cx - 0.28, cy - 0.28, sprite::STATUS, 0, 7, s, WHITE));
                    }
                }
                if lights && m.status == status::WORKING && def.category < 5 {
                    glows.push((cx, cy, LIGHT[def.category as usize], 36));
                }
            }
            self.visible_machines = visible;
        }

        // Core and Ark, ringed by progress towards the next terraforming stage.
        let c = &self.core;
        let half = CORE_SIZE as f32 * 0.5;
        let (hx, hy) = (c.x as f32 + half, c.y as f32 + half);
        if hx + half + 1.0 >= x0 && hx - half - 1.0 <= x1 && hy + half + 1.0 >= y0 && hy - half - 1.0 <= y1 {
            let s = c.stage as usize;
            let progress = match STAGES.get(s + 1) {
                Some(next) => {
                    let lo = STAGES[s].ti;
                    ((c.ti - lo) * 255 / (next.ti - lo).max(1)).min(255) as u8
                }
                None => 255,
            };
            let rot = (c.ark_phase.min(5) << 2) | (c.trim.min(3) << 5);
            let launch = self.launch as u32;
            let size = (TILE as f32 * (CORE_SIZE as f32 + 1.0)) as u8;
            push(&mut out, sprite_at(hx, hy, sprite::CORE, rot, size, progress, 0xff00_0000 | launch));
            if lights {
                glows.push((hx, hy, 0x0060_c0ff, 110));
            }
        }

        // Items: only segments whose bounds touch the view are walked.
        if flags & lod::ITEMS != 0 {
            let view = [x0 - 0.5, y0 - 0.5, x1 + 0.5, y1 + 0.5];
            let limit = out.capacity() - OVERLAY_RESERVE;
            self.belts.for_each_visible_item(g, view, alpha, |x, y, it| {
                if out.len() < limit {
                    out.push(sprite_at(x, y, it, 0, ITEM, 0, WHITE));
                }
            });
        }

        // Drones in flight, with their shadows.
        for p in 0..self.machines.ports.len() {
            if let Some((x, y, h)) = self.drone_pos(p, alpha) {
                if x < x0 - 2.0 || x > x1 + 2.0 || y < y0 - 2.0 || y > y1 + 2.0 {
                    continue;
                }
                push(&mut out, sprite_at(x + 0.3 * h, y + 0.45 * h, sprite::SHADOW, 0, 9, 0, SHADOW));
                let size = (10.0 + 4.0 * h) as u8;
                let spin = ((self.tick as f32 + alpha) * 1.7) as u32 as u8;
                push(&mut out, sprite_at(x, y, sprite::DRONE, 0, size, spin, WHITE));
                if lights {
                    glows.push((x, y, 0x0040_ffff, 20));
                }
            }
        }

        // Meteors: a streak falling from the upper left, then a flash.
        let now = self.tick as f32 + alpha;
        for m in &self.sky.meteors {
            let (tx, ty) = self.grid.xy(m.tile);
            let (tx, ty) = (tx as f32 + 0.5, ty as f32 + 0.5);
            let t = now - (m.impact as f32 - FALL_TICKS as f32);
            if t < FALL_TICKS as f32 {
                let f = (t / FALL_TICKS as f32).clamp(0.0, 1.0);
                let (x, y) = (tx - 14.0 * (1.0 - f), ty - 22.0 * (1.0 - f));
                if x > x0 - 4.0 && x < x1 + 4.0 && y > y0 - 4.0 && y < y1 + 4.0 {
                    push(&mut out, sprite_at(x, y, sprite::METEOR, 0, 40, (f * 255.0) as u8, WHITE));
                }
            } else {
                let age = ((t - FALL_TICKS as f32) / AFTERGLOW as f32).clamp(0.0, 1.0);
                if tx > x0 - 3.0 && tx < x1 + 3.0 && ty > y0 - 3.0 && ty < y1 + 3.0 {
                    push(&mut out, sprite_at(tx, ty, sprite::FLASH, 0, 64, (age * 255.0) as u8, 0x0000_0000));
                }
            }
        }

        if flags & lod::WEATHER != 0 {
            self.weather(&mut out, [x0, y0, x1, y1], now);
        }

        for &(x, y, color, size) in &glows {
            push(
                &mut out,
                sprite_at(x, y, sprite::GLOW, 0, size, (dark.min(1000) * 255 / 1000) as u8, color),
            );
        }
        self.glows = glows;

        // Placement preview: a single building, a blueprint, or the delete marker.
        let cur = self.cursor;
        let g = &self.grid;
        if cur.flags & cursor::VISIBLE != 0 {
            let (cx, cy) = (cur.x as f32 + 0.5, cur.y as f32 + 0.5);
            if cur.flags & cursor::DELETE != 0 {
                push(&mut out, sprite_at(cx, cy, sprite::DELETE, 0, TILE, 0, WHITE));
            } else if cur.flags & cursor::PASTE != 0 {
                for (x, y, kind, d, _) in self.bp_layout(cur.x, cur.y, cur.dir) {
                    let tint = if self.can_place(x, y, kind) { GHOST_OK } else { GHOST_BAD };
                    let spr = if kind == bk::BELT { sprite::BELT } else { sprite::BUILDING + kind };
                    push(&mut out, sprite_at(x as f32 + 0.5, y as f32 + 0.5, spr, d, TILE, 0, tint));
                }
            } else if cur.kind == bk::BELT || (cur.kind > bk::CORE && (cur.kind as usize) < BUILDINGS.len()) {
                let tint = if self.can_place(cur.x, cur.y, cur.kind) { GHOST_OK } else { GHOST_BAD };
                let spr = if cur.kind == bk::BELT { sprite::BELT } else { sprite::BUILDING + cur.kind };
                push(&mut out, sprite_at(cx, cy, spr, cur.dir, TILE, 0, tint));
                let t = g.index(cur.x, cur.y);
                if BUILDINGS[cur.kind as usize].class == Class::Drill && t != NONE && g.res[t as usize] != 0 {
                    push(&mut out, sprite_at(cx + 0.25, cy - 0.25, g.res[t as usize], 0, 7, 0, tint));
                }
            }
        }

        let n = out.len() as u32;
        self.stats[stat::INSTANCES] = n;
        self.instances = out;
        n
    }

    /// Rain, snow and birds, by terraforming stage. Deterministic in simulation time, so
    /// pausing freezes them and nothing needs saving.
    fn weather(&self, out: &mut Vec<Instance>, view: [f32; 4], now: f32) {
        let stage = self.core.stage;
        let secs = now / TICKS_PER_SEC as f32;
        let (w, h) = (view[2] - view[0], view[3] - view[1]);
        let area = w * h;
        // Precipitation comes and goes: snow while the air is thin, rain once there's water.
        let (kind, period, wet) = match stage {
            s if s >= stage::WATER => (sprite::RAIN, 540.0, 0.22),
            s if s >= stage::THIN_AIR => (sprite::SNOW, 420.0, 0.3),
            _ => (0, 1.0, 0.0),
        };
        let phase = (secs / period).fract();
        if kind != 0 && phase < wet {
            let fade = (phase / 0.05).min((wet - phase) / 0.05).min(1.0);
            let n = ((area * 0.12 * fade) as usize).min(320);
            let (speed, drift) = if kind == sprite::RAIN { (14.0, 3.0) } else { (1.6, 0.8) };
            for i in 0..n {
                let hsh = hash(i as u32, 0x51ed);
                let ox = unit(hsh);
                let oy = unit(hsh >> 7 ^ 0x2f1);
                let life = h + 2.0;
                let y = view[1] - 1.0 + (oy * life + secs * speed).rem_euclid(life);
                let sway = if kind == sprite::SNOW { (secs * 1.3 + i as f32).sin() * 0.3 } else { 0.0 };
                let x = view[0] + (ox * (w + 2.0) + secs * drift + sway).rem_euclid(w + 2.0) - 1.0;
                push(out, sprite_at(x, y, kind, 0, if kind == sprite::RAIN { 10 } else { 4 }, 0, WHITE));
            }
        }
        // Butterflies drift over the view once flowers bloom.
        if stage >= stage::BLOOMING {
            let n = ((area * 0.012) as usize).clamp(4, 28);
            const WINGS: [u32; 4] = [0xff5c_a8f4, 0xff3c_c8fb, 0xffe8_e8ff, 0xfff4_8cc4];
            for i in 0..n as u32 {
                let hsh = hash(i, 0xb077);
                let (ox, oy) = (unit(hsh), unit(hsh >> 8));
                let speed = 0.25 + unit(hsh >> 16) * 0.3;
                let a = secs * speed + i as f32;
                let x = view[0] + (ox * w + a.sin() * 3.0 + secs * 0.2).rem_euclid(w);
                let y = view[1] + (oy * h + (a * 1.3).cos() * 2.0).rem_euclid(h);
                let flap = ((secs * 14.0 + i as f32).sin() * 127.0 + 128.0) as u8;
                push(out, sprite_at(x, y, sprite::BUTTERFLY, 0, 5, flap, WINGS[i as usize % WINGS.len()]));
            }
        }
        // Birds cross the sky once there is grassland to nest in; flocks, once wildlife thrives.
        if stage >= stage::GRASSLAND {
            let flocks = if stage >= stage::WILDLIFE { 3 } else { 1 };
            for f in 0..flocks {
                let flight = 70.0;
                let lap = (secs / flight + f as f32 * 0.37).floor();
                let t = (secs / flight + f as f32 * 0.37).fract();
                let hsh = hash(lap as u32, f + 11);
                let ang = unit(hsh) * core::f32::consts::TAU;
                let (dx, dy) = (ang.cos(), ang.sin());
                let (cx, cy) = (view[0] + w * 0.5, view[1] + h * 0.5);
                let span = w.max(h) * 0.7 + 10.0;
                let off = (unit(hsh >> 9) - 0.5) * w.min(h) * 0.8;
                let (bx, by) = (
                    cx - dx * span + dx * 2.0 * span * t - dy * off,
                    cy - dy * span + dy * 2.0 * span * t + dx * off,
                );
                for b in 0..5u32 {
                    let side = if b % 2 == 0 { 1.0 } else { -1.0 };
                    let rank = b.div_ceil(2) as f32;
                    let (x, y) = (
                        bx - dx * rank * 0.9 - dy * side * rank * 0.7,
                        by - dy * rank * 0.9 + dx * side * rank * 0.7,
                    );
                    let rot = ((ang / core::f32::consts::FRAC_PI_2).round() as i32 & 3) as u8;
                    let flap = ((secs * 7.0 + b as f32 * 1.3).sin() * 127.0 + 128.0) as u8;
                    push(out, sprite_at(x + 0.8, y + 1.1, sprite::SHADOW, 0, 5, 0, 0x3000_0000));
                    push(out, sprite_at(x, y, sprite::BIRD, rot, 9, flap, WHITE));
                }
            }
        }
    }
}
