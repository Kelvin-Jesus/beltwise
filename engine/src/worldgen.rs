//! Seeded world generation: terrain fields for the terraforming visuals, resource deposits
//! (with purity) and the wrecks of the first expedition. A fixed starter ring keeps the
//! first objectives reachable; richer and rarer deposits sit further out to pull the
//! factory outwards.

use crate::content::{PLANETS, bk, it, wreck};
use crate::types::NONE;
use crate::world::Grid;

/// Ember: terrain above this elevation is lava.
pub const LAVA_LINE: f32 = 0.74;
const WRECKS: usize = 24;

struct Rng(u32);

impl Rng {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
    fn unit(&mut self) -> f32 {
        (self.next() & 0xffff) as f32 / 65536.0
    }
}

fn hash(x: i32, y: i32, seed: u32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x27d4_eb2d)
        ^ (y as u32).wrapping_mul(0x1656_67b1)
        ^ seed.wrapping_mul(0x9e37_79b9);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297a_2d39);
    h ^ (h >> 15)
}

fn hash01(x: i32, y: i32, seed: u32) -> f32 {
    (hash(x, y, seed) & 0xffff) as f32 / 65536.0
}

/// Smooth value noise with lattice spacing `scale` tiles.
fn value_noise(x: f32, y: f32, scale: f32, seed: u32) -> f32 {
    let (fx, fy) = (x / scale, y / scale);
    let (ix, iy) = (fx.floor() as i32, fy.floor() as i32);
    let (tx, ty) = (fx - ix as f32, fy - iy as f32);
    let (sx, sy) = (tx * tx * (3.0 - 2.0 * tx), ty * ty * (3.0 - 2.0 * ty));
    let a = hash01(ix, iy, seed);
    let b = hash01(ix + 1, iy, seed);
    let c = hash01(ix, iy + 1, seed);
    let d = hash01(ix + 1, iy + 1, seed);
    let top = a + (b - a) * sx;
    let bottom = c + (d - c) * sx;
    top + (bottom - top) * sy
}

fn fbm(x: f32, y: f32, base: f32, octaves: u32, seed: u32) -> f32 {
    let (mut sum, mut amp, mut norm, mut scale) = (0.0, 1.0, 0.0, base);
    for o in 0..octaves {
        sum += value_noise(x, y, scale, seed.wrapping_add(o * 1013)) * amp;
        norm += amp;
        amp *= 0.5;
        scale *= 0.5;
    }
    sum / norm
}

struct Gen<'a> {
    g: &'a mut Grid,
    terrain: &'a [u8],
    lava: bool,
}

impl Gen<'_> {
    fn buildable(&self, t: u32) -> bool {
        !(self.lava && self.terrain[t as usize * 4] as f32 / 255.0 > LAVA_LINE)
    }

    fn blob(&mut self, cx: i32, cy: i32, r: f32, res: u8, purity: u8, seed: u32) {
        let ri = r.ceil() as i32 + 1;
        for y in cy - ri..=cy + ri {
            for x in cx - ri..=cx + ri {
                let t = self.g.index(x, y);
                if t == NONE || self.g.kind[t as usize] != bk::EMPTY || !self.buildable(t) {
                    continue;
                }
                let (dx, dy) = ((x - cx) as f32, (y - cy) as f32);
                if (dx * dx + dy * dy).sqrt() + hash01(x, y, seed) * 1.6 < r + 0.8 {
                    self.g.res[t as usize] = res;
                    self.g.purity[t as usize] = purity;
                }
            }
        }
    }
}

/// Purity for a deposit at distance `d` from the landing site: richer further out.
fn purity(rng: &mut Rng, d: f32) -> u8 {
    let roll = rng.unit();
    let pure = (d / 400.0).clamp(0.05, 0.45);
    let impure = (0.3 - d / 800.0).clamp(0.08, 0.3);
    if roll < pure {
        2
    } else if roll < pure + impure {
        0
    } else {
        1
    }
}

/// Fills `terrain` (RGBA per tile: elevation, moisture, detail, 255) and the deposits, and
/// returns the wrecks to place: (tile, contents as kind << 8).
pub fn generate(
    g: &mut Grid,
    terrain: &mut [u8],
    seed: u32,
    planet: u8,
    core_x: i32,
    core_y: i32,
    core: i32,
) -> Vec<(u32, u16)> {
    let def = &PLANETS[planet as usize];
    let (cx, cy) = (core_x + core / 2, core_y + core / 2);

    for y in 0..g.h {
        for x in 0..g.w {
            let (fx, fy) = (x as f32, y as f32);
            // fBm bunches up around 0.5; stretch it so there are real basins (future lakes)
            // and highlands (ice caps that retreat as the planet warms).
            let mut e = ((fbm(fx, fy, 64.0, 5, seed) - 0.5) * 2.4 + 0.5 + def.elevation).clamp(0.0, 1.0);
            // Keep the landing site on middle ground: it should neither flood nor burn.
            let (dx, dy) = (fx - cx as f32, fy - cy as f32);
            let near = (1.0 - (dx * dx + dy * dy).sqrt() / 26.0).clamp(0.0, 1.0);
            e += (0.52 - e) * near * near * (3.0 - 2.0 * near);
            let m = ((fbm(fx + 1000.0, fy - 700.0, 40.0, 4, seed ^ 0x5bd1_e995) - 0.5) * 2.0 + 0.5)
                .clamp(0.0, 1.0);
            let d = fbm(fx, fy, 6.0, 2, seed ^ 0x68e3_1da4);
            let i = ((y * g.w + x) * 4) as usize;
            terrain[i] = (e.clamp(0.0, 1.0) * 255.0) as u8;
            terrain[i + 1] = (m.clamp(0.0, 1.0) * 255.0) as u8;
            terrain[i + 2] = (d.clamp(0.0, 1.0) * 255.0) as u8;
            terrain[i + 3] = 255;
        }
    }

    let mut gn = Gen { g, terrain, lava: planet == 1 };

    // Starter ring: everything the first objectives need within ~16 tiles.
    let starters: [(i32, i32, f32, u8); 7] = [
        (-9, -4, 2.6, it::IRON_ORE),
        (8, -7, 2.4, it::COPPER_ORE),
        (-7, 8, 2.4, it::STONE),
        (10, 6, 2.2, it::COAL),
        (15, -2, 2.0, it::IRON_ORE),
        (-15, 2, 2.0, it::COAL),
        (2, -16, 2.2, it::ICE),
    ];
    for (i, &(dx, dy, r, res)) in starters.iter().enumerate() {
        gn.blob(cx + dx, cy + dy, r, res, 1, seed.wrapping_add(i as u32));
    }

    // Deposits further out: common ores everywhere, oil and titanium mid-range, uranium
    // far out. Purity comes from its own random stream so Glacia keeps the exact layout
    // older saves were made on.
    let mut rng = Rng(seed | 1);
    let mut prng = Rng((seed ^ 0x7f4a_7c15) | 1);
    let place =
        |gn: &mut Gen, rng: &mut Rng, prng: &mut Rng, res: u8, dist: (f32, f32), radius: (f32, f32)| {
            for _ in 0..40 {
                let ang = rng.unit() * core::f32::consts::TAU;
                let d = dist.0 + rng.unit() * (dist.1 - dist.0);
                let (x, y) = (cx + (ang.cos() * d) as i32, cy + (ang.sin() * d) as i32);
                if x > 3 && y > 3 && x < gn.g.w - 4 && y < gn.g.h - 4 && gn.buildable(gn.g.index(x, y)) {
                    let r = radius.0 + rng.unit() * (radius.1 - radius.0);
                    let s = rng.next();
                    let p = purity(prng, d);
                    gn.blob(x, y, r, res, p, s);
                    return;
                }
            }
        };
    let area = (gn.g.w * gn.g.h) as f32 / (512.0 * 512.0);
    let common = [it::IRON_ORE, it::COPPER_ORE, it::STONE, it::COAL, it::ICE];
    const GLACIA_POOL: [u8; 8] =
        [it::IRON_ORE, it::COPPER_ORE, it::STONE, it::COAL, it::ICE, it::IRON_ORE, it::COPPER_ORE, it::ICE];
    let total: u32 = def.weights[..5].iter().map(|&w| w as u32).sum();
    for _ in 0..(70.0 * area).max(6.0) as u32 {
        let res = if planet == 0 {
            GLACIA_POOL[(rng.next() % GLACIA_POOL.len() as u32) as usize]
        } else {
            let mut pick = rng.next() % total.max(1);
            let mut res = common[0];
            for (k, &w) in def.weights[..5].iter().enumerate() {
                if pick < w as u32 {
                    res = common[k];
                    break;
                }
                pick -= w as u32;
            }
            res
        };
        place(&mut gn, &mut rng, &mut prng, res, (22.0, 240.0), (1.8, 3.6));
    }
    for _ in 0..(10.0 * area).max(3.0) as u32 {
        place(&mut gn, &mut rng, &mut prng, it::TITANIUM_ORE, (32.0, 110.0), (1.8, 3.0));
    }
    for _ in 0..(8.0 * area).max(3.0) as u32 {
        place(&mut gn, &mut rng, &mut prng, it::URANIUM_ORE, (70.0, 200.0), (1.6, 2.6));
    }
    for _ in 0..(def.weights[5] as f32 * 3.0 * area).max(3.0) as u32 {
        place(&mut gn, &mut rng, &mut prng, it::CRUDE_OIL, (30.0, 150.0), (1.4, 2.4));
    }

    // Wrecks of the first expedition, on bare ground. Contents are spread so logs turn up
    // steadily, probes early, and the rare amplifiers far out.
    let mut spots: Vec<(f32, u32)> = Vec::new();
    let mut tries = 0;
    while spots.len() < WRECKS && tries < 4000 {
        tries += 1;
        let ang = rng.unit() * core::f32::consts::TAU;
        let d = 18.0 + rng.unit().powf(0.8) * 215.0;
        let (x, y) = (cx + (ang.cos() * d) as i32, cy + (ang.sin() * d) as i32);
        let t = gn.g.index(x, y);
        if t == NONE || x < 2 || y < 2 || x >= gn.g.w - 2 || y >= gn.g.h - 2 {
            continue;
        }
        let ti = t as usize;
        let crowded = spots.iter().any(|&(_, o)| {
            let (ox, oy) = ((o % gn.g.w as u32) as i32, (o / gn.g.w as u32) as i32);
            (ox - x).abs() < 12 && (oy - y).abs() < 12
        });
        if gn.g.kind[ti] == bk::EMPTY && gn.g.res[ti] == 0 && gn.buildable(t) && !crowded {
            spots.push((d, t));
        }
    }
    spots.sort_by(|a, b| a.0.total_cmp(&b.0));
    const PATTERN: [u8; WRECKS] = [
        wreck::LOG,
        wreck::CACHE,
        wreck::PROBE,
        wreck::LOG,
        wreck::CACHE,
        wreck::PROBE,
        wreck::LOG,
        wreck::SHARDS,
        wreck::LOG,
        wreck::PROBE,
        wreck::CACHE,
        wreck::LOG,
        wreck::PROBE,
        wreck::LOG,
        wreck::SHARDS,
        wreck::LOG,
        wreck::CACHE,
        wreck::PROBE,
        wreck::LOG,
        wreck::AMPLIFIER,
        wreck::LOG,
        wreck::PROBE,
        wreck::LOG,
        wreck::AMPLIFIER,
    ];
    spots.iter().enumerate().map(|(k, &(_, t))| (t, (PATTERN[k] as u16) << 8)).collect()
}
