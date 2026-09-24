//! Segment-based belt transport ("transport lines").
//!
//! Consecutive belts are merged into *segments*: chains of tiles that items travel through
//! along one fixed-point path. Items on a segment are stored front-to-back as *gaps*:
//! `gap[0]` is the front item's distance to the segment end, `gap[i]` the distance from item
//! `i` to item `i - 1`. Advancing a segment only shrinks the first gap that still has slack;
//! every item behind it moves implicitly. A tick therefore costs O(segments), not O(items),
//! and a fully compressed (blocked) segment costs O(1) thanks to the cached `first_moving`.
//!
//! Items live in one flat arena (a ring buffer per segment) so the update loop walks memory
//! linearly. Topology is rebuilt from the grid whenever belts change, preserving every
//! surviving item's position: O(belts + items), and never on a frame without edits.

use crate::content::{BELT_SPEEDS, bk};
use crate::types::{MAX_SEG_TILES, MIN_GAP, NONE, SUB, dir};
use crate::world::Grid;

#[derive(Clone, Copy, Default, Debug)]
pub struct Segment {
    /// Range of this segment's tiles (start -> end) in `Belts::seg_tiles`.
    pub tiles_start: u32,
    pub len: u32,
    /// Ring buffer region in the item arena.
    pub arena_start: u32,
    pub cap: u32,
    pub head: u32,
    pub count: u32,
    /// Distance from the segment end to the back-most item (sum of all gaps).
    pub gap_sum: u32,
    /// Items `[0, first_moving)` are known to be compressed and are skipped by `advance`.
    pub first_moving: u32,
    /// Render interpolation: items `>= moved_from` moved `moved_amt` sub-units last tick.
    pub moved_from: u32,
    pub moved_amt: u32,
    /// Tile the last belt points into and the direction items travel when handed over.
    pub target_tile: u32,
    pub target_dir: u8,
    /// Ticks the front item has been waiting for a hand-over (merge arbitration).
    pub wait: u32,
    /// Tile-space bounds (min x, min y, max x, max y) for view culling.
    pub bbox: [f32; 4],
}

#[derive(Clone, Copy)]
struct Saved {
    key: u64,
    item: u8,
}

pub struct Belts {
    pub segs: Vec<Segment>,
    /// Update order: downstream segments first so hand-overs see this tick's free space.
    pub order: Vec<u32>,
    pub seg_tiles: Vec<u32>,
    pub gaps: Vec<u16>,
    pub items: Vec<u8>,
    /// Per tile: owning segment, index along it, and the direction items enter it from.
    pub tile_seg: Vec<u32>,
    pub tile_idx: Vec<u32>,
    pub in_dir: Vec<u8>,
    /// Every belt tile, for O(belts) rebuilds.
    pub list: Vec<u32>,
    list_pos: Vec<u32>,
    pub dirty: bool,
    pub item_count: u32,
    pub rebuilds: u32,
    /// Sub-units every item moves per tick (raised by belt research).
    pub speed: u32,
    // Scratch buffers reused across rebuilds.
    saved: Vec<Saved>,
    state: Vec<u8>,
    stack: Vec<u32>,
}

/// The belt feeding `t` along its chain: the belt behind it, else a lone side feeder (a curve).
/// Two side feeders and no rear belt make a T-junction: both side-load instead.
fn pred(g: &Grid, t: u32) -> u32 {
    let d = g.dir[t as usize];
    let r = g.step(t, dir::opposite(d));
    if r != NONE && g.kind[r as usize] == bk::BELT && g.dir[r as usize] == d {
        return r;
    }
    let mut found = NONE;
    for sd in [dir::cw(d), dir::ccw(d)] {
        let s = g.step(t, sd);
        if s != NONE && g.kind[s as usize] == bk::BELT && g.dir[s as usize] == dir::opposite(sd) {
            if found != NONE {
                return NONE;
            }
            found = s;
        }
    }
    found
}

/// The next belt along `t`'s chain, if `t` is that belt's chain predecessor.
fn succ(g: &Grid, t: u32) -> u32 {
    let n = g.step(t, g.dir[t as usize]);
    if n != NONE && g.kind[n as usize] == bk::BELT && pred(g, n) == t { n } else { NONE }
}

#[inline(always)]
fn slot(s: &Segment, i: u32) -> usize {
    let mut r = s.head + i;
    if r >= s.cap {
        r -= s.cap;
    }
    (s.arena_start + r) as usize
}

impl Belts {
    pub fn new(tiles: usize) -> Self {
        Belts {
            segs: Vec::new(),
            order: Vec::new(),
            seg_tiles: Vec::new(),
            gaps: Vec::new(),
            items: Vec::new(),
            tile_seg: vec![NONE; tiles],
            tile_idx: vec![0; tiles],
            in_dir: vec![0; tiles],
            list: Vec::new(),
            list_pos: vec![NONE; tiles],
            dirty: false,
            item_count: 0,
            rebuilds: 0,
            speed: BELT_SPEEDS[0],
            saved: Vec::new(),
            state: Vec::new(),
            stack: Vec::new(),
        }
    }

    /// Registers a new or re-oriented belt tile. Topology is rebuilt lazily.
    pub fn add_tile(&mut self, t: u32) {
        if self.list_pos[t as usize] == NONE {
            self.list_pos[t as usize] = self.list.len() as u32;
            self.list.push(t);
        }
        self.dirty = true;
    }

    pub fn remove_tile(&mut self, t: u32) {
        let p = self.list_pos[t as usize];
        if p == NONE {
            return;
        }
        let last = self.list.pop().unwrap_or(t);
        if last != t {
            self.list[p as usize] = last;
            self.list_pos[last as usize] = p;
        }
        self.list_pos[t as usize] = NONE;
        self.dirty = true;
    }

    // ---- Simulation -----------------------------------------------------------------------

    /// Moves every item on segment `s` by up to `speed`, respecting spacing.
    #[inline]
    pub fn advance(&mut self, s: u32) {
        let seg = &mut self.segs[s as usize];
        seg.moved_from = seg.count;
        seg.moved_amt = 0;
        let mut i = seg.first_moving;
        let mut rem = self.speed;
        while i < seg.count {
            let k = slot(seg, i);
            let g = self.gaps[k] as u32;
            let min = if i == 0 { 0 } else { MIN_GAP };
            if g > min {
                let take = rem.min(g - min);
                self.gaps[k] = (g - take) as u16;
                seg.gap_sum -= take;
                if seg.moved_from == seg.count {
                    seg.moved_from = i;
                    seg.moved_amt = take;
                }
                rem -= take;
                if rem == 0 {
                    break;
                }
            }
            // Item `i` is compressed now; items behind it keep the leftover movement.
            i += 1;
        }
        seg.first_moving = i;
    }

    /// The front item if it has reached the end of its segment and waits for a hand-over.
    #[inline]
    pub fn front_ready(&self, s: u32) -> u8 {
        let seg = &self.segs[s as usize];
        if seg.count == 0 {
            return 0;
        }
        let k = slot(seg, 0);
        if self.gaps[k] == 0 { self.items[k] } else { 0 }
    }

    pub fn pop_front(&mut self, s: u32) -> u8 {
        let seg = &mut self.segs[s as usize];
        let k = slot(seg, 0);
        let (g0, it) = (self.gaps[k], self.items[k]);
        seg.head += 1;
        if seg.head == seg.cap {
            seg.head = 0;
        }
        seg.count -= 1;
        if seg.count > 0 {
            // The new front's gap becomes relative to the segment end.
            let k1 = slot(seg, 0);
            self.gaps[k1] += g0;
        } else {
            seg.gap_sum = 0;
        }
        seg.first_moving = 0;
        self.item_count -= 1;
        it
    }

    /// Offers `it`, travelling in direction `md`, to belt tile `t`.
    pub fn insert_from(&mut self, g: &Grid, t: u32, it: u8, md: u8) -> bool {
        let ti = t as usize;
        if g.dir[ti] == dir::opposite(md) {
            return false; // the belt runs straight back into the source
        }
        let s = self.tile_seg[ti];
        if s == NONE {
            return false;
        }
        let k = self.tile_idx[ti];
        // Entering a chain head from behind starts at its entry edge; anything else is a
        // side-load onto the tile center.
        let x = if k == 0 && self.in_dir[ti] == md { 0 } else { k * SUB + SUB / 2 };
        self.insert_at(s, x, it)
    }

    /// Inserts an item at path position `x` (sub-units from the segment start) if the
    /// spacing to both neighbours allows it.
    pub fn insert_at(&mut self, s: u32, x: u32, it: u8) -> bool {
        let mut seg = self.segs[s as usize];
        let l = seg.len * SUB;
        if x > l || seg.count >= seg.cap {
            return false;
        }
        let d = l - x; // distance from the segment end
        if seg.count == 0 || d >= seg.gap_sum {
            // Append behind the current back item (the common case).
            let gap = if seg.count == 0 { d } else { d - seg.gap_sum };
            if seg.count > 0 && gap < MIN_GAP {
                return false;
            }
            let k = slot(&seg, seg.count);
            self.gaps[k] = gap as u16;
            self.items[k] = it;
            seg.count += 1;
            seg.gap_sum = d;
        } else {
            // Mid-segment insert: find the first item behind the insertion point.
            let mut acc = 0;
            let mut i = 0;
            loop {
                let di = acc + self.gaps[slot(&seg, i)] as u32;
                if di > d {
                    break;
                }
                acc = di;
                i += 1;
            }
            let behind = acc + self.gaps[slot(&seg, i)] as u32;
            if (i > 0 && d - acc < MIN_GAP) || behind - d < MIN_GAP {
                return false;
            }
            let mut j = seg.count;
            while j > i {
                let (dst, src) = (slot(&seg, j), slot(&seg, j - 1));
                self.gaps[dst] = self.gaps[src];
                self.items[dst] = self.items[src];
                j -= 1;
            }
            let k = slot(&seg, i);
            self.gaps[k] = (d - acc) as u16;
            self.items[k] = it;
            self.gaps[slot(&seg, i + 1)] = (behind - d) as u16;
            seg.count += 1;
            seg.first_moving = seg.first_moving.min(i);
        }
        self.segs[s as usize] = seg;
        self.item_count += 1;
        true
    }

    // ---- Topology -------------------------------------------------------------------------

    pub fn rebuild(&mut self, g: &Grid) {
        self.rebuilds += 1;
        self.save_items();
        for &t in &self.seg_tiles {
            self.tile_seg[t as usize] = NONE;
        }
        self.segs.clear();
        self.seg_tiles.clear();

        // Chains start at belts without a chain predecessor...
        for li in 0..self.list.len() {
            let t = self.list[li];
            if pred(g, t) == NONE {
                self.walk_chain(g, t);
            }
        }
        // ...and everything left over sits on a closed loop.
        for li in 0..self.list.len() {
            let t = self.list[li];
            if self.tile_seg[t as usize] == NONE {
                self.walk_chain(g, t);
            }
        }

        let gw = g.w as u32;
        for seg in &mut self.segs {
            let tiles = &self.seg_tiles[seg.tiles_start as usize..(seg.tiles_start + seg.len) as usize];
            let end = tiles[tiles.len() - 1];
            seg.target_dir = g.dir[end as usize];
            seg.target_tile = g.step(end, seg.target_dir);
            let mut b = [f32::MAX, f32::MAX, f32::MIN, f32::MIN];
            for &t in tiles {
                let (x, y) = ((t % gw) as f32, (t / gw) as f32);
                b = [b[0].min(x), b[1].min(y), b[2].max(x + 1.0), b[3].max(y + 1.0)];
            }
            seg.bbox = b;
        }

        self.compute_order(g);
        self.layout_arena();
        self.restore_items(g);
        self.dirty = false;
    }

    fn walk_chain(&mut self, g: &Grid, head: u32) {
        let mut t = head;
        let mut open = true;
        loop {
            if open {
                self.segs.push(Segment {
                    tiles_start: self.seg_tiles.len() as u32,
                    target_tile: NONE,
                    ..Default::default()
                });
                open = false;
            }
            let si = self.segs.len() as u32 - 1;
            let seg = &mut self.segs[si as usize];
            let p = pred(g, t);
            let ti = t as usize;
            self.tile_seg[ti] = si;
            self.tile_idx[ti] = seg.len;
            self.in_dir[ti] = if p == NONE { g.dir[ti] } else { g.dir[p as usize] };
            self.seg_tiles.push(t);
            seg.len += 1;
            if seg.len == MAX_SEG_TILES {
                open = true;
            }
            let n = succ(g, t);
            if n == NONE || self.tile_seg[n as usize] != NONE {
                break; // chain end, or a loop closed on itself
            }
            t = n;
        }
    }

    /// Orders segments downstream-first. Each segment has at most one target segment, so the
    /// graph is functional: follow targets until a visited node, then emit in reverse.
    fn compute_order(&mut self, g: &Grid) {
        let n = self.segs.len();
        self.state.clear();
        self.state.resize(n, 0);
        self.order.clear();
        for s in 0..n as u32 {
            let mut cur = s;
            self.stack.clear();
            while cur != NONE && self.state[cur as usize] == 0 {
                self.state[cur as usize] = 1;
                self.stack.push(cur);
                let tt = self.segs[cur as usize].target_tile;
                cur = if tt != NONE && g.kind[tt as usize] == bk::BELT {
                    self.tile_seg[tt as usize]
                } else {
                    NONE
                };
            }
            while let Some(x) = self.stack.pop() {
                self.state[x as usize] = 2;
                self.order.push(x);
            }
        }
    }

    fn layout_arena(&mut self) {
        let mut total = 0;
        for seg in &mut self.segs {
            seg.cap = seg.len * SUB / MIN_GAP + 2;
            seg.arena_start = total;
            total += seg.cap;
        }
        self.gaps.clear();
        self.gaps.resize(total as usize, 0);
        self.items.clear();
        self.items.resize(total as usize, 0);
    }

    /// Records each item as (tile, offset within tile) before the topology changes.
    fn save_items(&mut self) {
        self.saved.clear();
        for seg in &self.segs {
            let l = seg.len * SUB;
            let mut d = 0;
            for i in 0..seg.count {
                let k = slot(seg, i);
                d += self.gaps[k] as u32;
                let x = l - d.min(l);
                let ti = (x / SUB).min(seg.len - 1);
                let tile = self.seg_tiles[(seg.tiles_start + ti) as usize];
                let off = x - ti * SUB;
                self.saved.push(Saved { key: ((tile as u64) << 32) | off as u64, item: self.items[k] });
            }
        }
    }

    /// Re-inserts saved items into the new segments; items on removed belts are dropped and
    /// items that no longer fit (e.g. two chains merged nose to tail) are pushed back or dropped.
    fn restore_items(&mut self, g: &Grid) {
        let mut saved = core::mem::take(&mut self.saved);
        for sv in &mut saved {
            let tile = (sv.key >> 32) as u32;
            let off = (sv.key & 0xffff_ffff) as u32;
            let s = if g.kind[tile as usize] == bk::BELT { self.tile_seg[tile as usize] } else { NONE };
            sv.key = if s == NONE {
                u64::MAX
            } else {
                let seg = &self.segs[s as usize];
                let d = (seg.len * SUB).saturating_sub(self.tile_idx[tile as usize] * SUB + off);
                ((s as u64) << 32) | d as u64
            };
        }
        // Front-most first within each segment.
        saved.sort_unstable_by_key(|sv| sv.key);
        self.item_count = 0;
        for sv in &saved {
            if sv.key == u64::MAX {
                break;
            }
            let s = (sv.key >> 32) as usize;
            let seg = &mut self.segs[s];
            let (prev, min) = if seg.count == 0 { (0, 0) } else { (seg.gap_sum, MIN_GAP) };
            let d = ((sv.key & 0xffff_ffff) as u32).max(prev + min);
            if d > seg.len * SUB || seg.count >= seg.cap {
                continue;
            }
            let k = slot(seg, seg.count);
            self.gaps[k] = (d - prev) as u16;
            self.items[k] = sv.item;
            seg.count += 1;
            seg.gap_sum = d;
            self.item_count += 1;
        }
        saved.clear();
        self.saved = saved;
    }

    /// Every item as (tile, offset within the tile in sub-units, item), for save games.
    pub fn snapshot(&self, out: &mut Vec<(u32, u8, u8)>) {
        for seg in &self.segs {
            let l = seg.len * SUB;
            let mut d = 0;
            for i in 0..seg.count {
                let k = slot(seg, i);
                d += self.gaps[k] as u32;
                let x = l - d.min(l);
                let ti = (x / SUB).min(seg.len - 1);
                let tile = self.seg_tiles[(seg.tiles_start + ti) as usize];
                out.push((tile, (x - ti * SUB) as u8, self.items[k]));
            }
        }
    }

    /// Puts snapshot items back onto the current belts (after the grid has been restored).
    pub fn restore(&mut self, g: &Grid, items: &[(u32, u8, u8)]) {
        if self.dirty {
            self.rebuild(g);
        }
        self.saved.clear();
        for seg in &mut self.segs {
            seg.count = 0;
            seg.head = 0;
            seg.gap_sum = 0;
            seg.first_moving = 0;
        }
        for &(tile, off, item) in items {
            if (tile as usize) < g.kind.len() {
                self.saved.push(Saved { key: ((tile as u64) << 32) | off as u64, item });
            }
        }
        self.restore_items(g);
    }

    /// Packs a segment with `it` at maximum density, leaving one slot free at each end so
    /// that closed loops keep circulating (benchmarks and tests). Inserts back-to-front so
    /// every insert is an O(1) append.
    pub fn fill(&mut self, s: u32, it: u8) {
        let l = self.segs[s as usize].len * SUB;
        let mut x = l.saturating_sub(MIN_GAP);
        while x >= MIN_GAP {
            self.insert_at(s, x, it);
            x -= MIN_GAP;
        }
    }

    /// World-space position of an item `f` (0..1) of the way along tile `t`'s path.
    #[inline]
    pub fn path_point(&self, g: &Grid, t: u32, f: f32) -> (f32, f32) {
        let w = g.w as u32;
        let (cx, cy) = ((t % w) as f32 + 0.5, (t / w) as f32 + 0.5);
        let (ind, outd) = (self.in_dir[t as usize], g.dir[t as usize]);
        let (ax, ay) = (dir::dx(ind) as f32, dir::dy(ind) as f32);
        if ind == outd {
            return (cx + ax * (f - 0.5), cy + ay * (f - 0.5));
        }
        // Corner: quadratic Bezier from the entry edge through the center to the exit edge.
        let (bx, by) = (dir::dx(outd) as f32, dir::dy(outd) as f32);
        let (ex, ey) = (cx - 0.5 * ax, cy - 0.5 * ay);
        let (ox, oy) = (cx + 0.5 * bx, cy + 0.5 * by);
        let u = 1.0 - f;
        let (a, b, c) = (u * u, 2.0 * u * f, f * f);
        (a * ex + b * cx + c * ox, a * ey + b * cy + c * oy)
    }

    /// Calls `emit(x, y, item)` for every item inside `view` (min x, min y, max x, max y),
    /// interpolated `alpha` of the way from the previous tick to the current one.
    #[inline]
    pub fn for_each_visible_item(
        &self,
        g: &Grid,
        view: [f32; 4],
        alpha: f32,
        mut emit: impl FnMut(f32, f32, u8),
    ) {
        let back = 1.0 - alpha;
        let fsub = SUB as f32;
        for seg in &self.segs {
            if seg.count == 0
                || seg.bbox[0] > view[2]
                || seg.bbox[2] < view[0]
                || seg.bbox[1] > view[3]
                || seg.bbox[3] < view[1]
            {
                continue;
            }
            let l = (seg.len * SUB) as f32;
            let lag = back * seg.moved_amt as f32;
            let mut d = 0u32;
            for i in 0..seg.count {
                let k = slot(seg, i);
                d += self.gaps[k] as u32;
                let dist = if i >= seg.moved_from { d as f32 + lag } else { d as f32 };
                let x = (l - dist).clamp(0.0, l);
                let ti = ((x / fsub) as u32).min(seg.len - 1);
                let f = (x - ti as f32 * fsub) / fsub;
                let t = self.seg_tiles[(seg.tiles_start + ti) as usize];
                let (px, py) = self.path_point(g, t, f);
                if px >= view[0] && px <= view[2] && py >= view[1] && py <= view[3] {
                    emit(px, py, self.items[k]);
                }
            }
        }
    }
}
