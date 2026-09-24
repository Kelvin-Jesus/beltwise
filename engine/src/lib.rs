//! Beltwise engine: a data-oriented factory simulation compiled to WebAssembly.
//!
//! The JS boundary is a flat C ABI (no wasm-bindgen): plain numbers in, plain numbers out,
//! and pointers into linear memory for bulk data (instance buffer, maps, stats, saves).
//! JS reads those regions through typed-array views, so nothing is copied or allocated per
//! frame on either side.

pub mod belts;
pub mod blueprint;
pub mod content;
pub mod core;
pub mod drones;
pub mod explore;
pub mod machines;
pub mod power;
pub mod progress;
pub mod render;
pub mod save;
pub mod sky;
pub mod types;
pub mod world;
pub mod worldgen;

#[cfg(test)]
mod tests;

use ::core::cell::UnsafeCell;
use content::{BUILDINGS, Class, ITEM_COUNT, TECH, TECH_COUNT};
use machines::{FULL, NO_NET, SPEED_ONE, flight};
use types::NONE;
use world::World;

/// Bumped whenever an export's signature or a shared memory layout changes.
pub const ABI_VERSION: u32 = 3;
pub const INFO_LEN: usize = 24;

/// Everything the exports touch. Single-threaded Wasm has exactly one instance.
struct State {
    world: World,
    content: String,
    save: Vec<u8>,
    load: Vec<u8>,
    blueprint: Vec<u8>,
    info: [u32; INFO_LEN],
    net: [u32; 8],
}

struct Slot(UnsafeCell<Option<State>>);

// SAFETY: wasm32-unknown-unknown (without the atomics feature) is single threaded, and no
// export re-enters another, so at most one `&mut State` exists at a time.
unsafe impl Sync for Slot {}

static STATE: Slot = Slot(UnsafeCell::new(None));

#[inline(always)]
fn state() -> &'static mut State {
    // SAFETY: see `Slot`. `fx_init` runs first; JS guarantees it.
    unsafe { (*STATE.0.get()).as_mut().unwrap_unchecked() }
}

#[inline(always)]
fn game() -> &'static mut World {
    &mut state().world
}

#[unsafe(no_mangle)]
pub extern "C" fn fx_abi() -> u32 {
    ABI_VERSION
}

#[unsafe(no_mangle)]
pub extern "C" fn fx_init(width: u32, height: u32, seed: u32) {
    let s = State {
        world: World::new(width as i32, height as i32, seed),
        content: content::content_json(),
        save: Vec::new(),
        load: Vec::new(),
        blueprint: Vec::new(),
        info: [0; INFO_LEN],
        net: [0; 8],
    };
    // SAFETY: see `Slot`.
    unsafe { *STATE.0.get() = Some(s) };
}

/// Runs `n` fixed simulation steps.
#[unsafe(no_mangle)]
pub extern "C" fn fx_tick(n: u32) {
    let g = game();
    for _ in 0..n {
        g.tick();
    }
}

/// Builds the instance buffer for a view rectangle in tile units; returns the count.
/// `flags`: see `render::lod`.
#[unsafe(no_mangle)]
pub extern "C" fn fx_render(x0: f32, y0: f32, x1: f32, y1: f32, alpha: f32, flags: u32) -> u32 {
    game().render(x0, y0, x1, y1, alpha, flags)
}

// ---- Shared memory ------------------------------------------------------------------------------

/// One byte per tile: the building kind (for the far-zoom tile map).
#[unsafe(no_mangle)]
pub extern "C" fn fx_kinds() -> *const u8 {
    game().grid.kind.as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn fx_instances() -> *const render::Instance {
    game().instances.as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn fx_instance_cap() -> u32 {
    render::INSTANCE_CAP as u32
}

#[unsafe(no_mangle)]
pub extern "C" fn fx_stats() -> *const u32 {
    game().stats.as_ptr()
}

/// Core storage: one `u32` per item id.
#[unsafe(no_mangle)]
pub extern "C" fn fx_storage() -> *const u32 {
    game().core.stored.as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn fx_item_count() -> u32 {
    ITEM_COUNT as u32
}

/// Per minute: produced, consumed, delivered (`ITEM_COUNT` each).
#[unsafe(no_mangle)]
pub extern "C" fn fx_rates() -> *const u32 {
    game().rates.per_min.as_ptr()
}

/// One byte per tile: the deposit (an item id) under it.
#[unsafe(no_mangle)]
pub extern "C" fn fx_resources() -> *const u8 {
    game().grid.res.as_ptr()
}

/// One byte per tile: deposit purity (0 impure, 1 normal, 2 pure).
#[unsafe(no_mangle)]
pub extern "C" fn fx_purity() -> *const u8 {
    game().grid.purity.as_ptr()
}

/// One byte per tile: 255 where explored.
#[unsafe(no_mangle)]
pub extern "C" fn fx_fog() -> *const u8 {
    game().grid.seen.as_ptr()
}

/// JS uploaded the fog rectangle reported in the stats; start a new one.
#[unsafe(no_mangle)]
pub extern "C" fn fx_fog_ack() {
    game().fog_ack();
}

/// One byte per tile: 1 where some power network reaches.
#[unsafe(no_mangle)]
pub extern "C" fn fx_power_map() -> *const u8 {
    let g = game();
    g.ensure_built();
    g.power_map.as_ptr()
}

/// RGBA per tile: elevation, moisture, detail. Static.
#[unsafe(no_mangle)]
pub extern "C" fn fx_terrain() -> *const u8 {
    game().terrain.as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn fx_width() -> u32 {
    game().grid.w as u32
}

#[unsafe(no_mangle)]
pub extern "C" fn fx_height() -> u32 {
    game().grid.h as u32
}

/// Current belt speed in tiles per tick (stripes scroll in sync with items).
#[unsafe(no_mangle)]
pub extern "C" fn fx_belt_speed() -> f32 {
    game().belts.speed as f32 / types::SUB as f32
}

/// Static game content as JSON.
#[unsafe(no_mangle)]
pub extern "C" fn fx_content() -> *const u8 {
    state().content.as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn fx_content_len() -> u32 {
    state().content.len() as u32
}

/// Result words of the last salvage or offline catch-up (`World::report`).
#[unsafe(no_mangle)]
pub extern "C" fn fx_report() -> *const u32 {
    game().report.as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn fx_report_len() -> u32 {
    game().report.len() as u32
}

// ---- Building ------------------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn fx_place(x: i32, y: i32, kind: u32, dir: u32) -> u32 {
    game().place(x, y, kind as u8, dir as u8) as u32
}

/// 0 if `kind` can be placed at (x, y), otherwise a `world::reason` code.
#[unsafe(no_mangle)]
pub extern "C" fn fx_check_place(x: i32, y: i32, kind: u32) -> u32 {
    game().check_place(x, y, kind as u8) as u32
}

#[unsafe(no_mangle)]
pub extern "C" fn fx_remove(x: i32, y: i32) -> u32 {
    game().remove(x, y) as u32
}

/// Recipe (255 = Auto) and sorter filter for buildings placed from now on.
#[unsafe(no_mangle)]
pub extern "C" fn fx_set_place(recipe: u32, filter: u32) {
    let g = game();
    g.place_recipe = recipe as u8;
    g.place_filter = filter as u8;
}

/// Packed tile info: `kind | dir << 8 | deposit << 16 | purity << 24`, or `u32::MAX` off-map.
/// Bit 31 is set on unexplored tiles, bit 30 on water.
#[unsafe(no_mangle)]
pub extern "C" fn fx_tile(x: i32, y: i32) -> u32 {
    let w = game();
    let g = &w.grid;
    let t = g.index(x, y);
    if t == NONE {
        return u32::MAX;
    }
    let ti = t as usize;
    let fog = if g.seen[ti] == 0 { 1 << 31 } else { 0 };
    let water = if w.is_water(t) { 1 << 30 } else { 0 };
    g.kind[ti] as u32
        | (g.dir[ti] as u32) << 8
        | (g.res[ti] as u32) << 16
        | (g.purity[ti] as u32 & 3) << 24
        | fog
        | water
}

/// Placement preview. `flags`: see `render::cursor`; with PASTE, `dir` is the rotation.
#[unsafe(no_mangle)]
pub extern "C" fn fx_set_cursor(x: i32, y: i32, kind: u32, dir: u32, flags: u32) {
    game().cursor = render::Cursor { x, y, kind: kind as u8, dir: dir as u8, flags: flags as u8 };
}

/// Starts a new undo group; call at the beginning of each build/erase gesture.
#[unsafe(no_mangle)]
pub extern "C" fn fx_edit_begin() {
    game().edit_begin();
}

#[unsafe(no_mangle)]
pub extern "C" fn fx_undo() -> u32 {
    game().undo() as u32
}

// ---- Machine settings ------------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn fx_set_recipe(x: i32, y: i32, r: u32) -> u32 {
    game().set_recipe(x, y, r as u8) as u32
}

#[unsafe(no_mangle)]
pub extern "C" fn fx_set_filter(x: i32, y: i32, item: u32) -> u32 {
    game().set_filter(x, y, item as u8) as u32
}

#[unsafe(no_mangle)]
pub extern "C" fn fx_set_shards(x: i32, y: i32, n: u32) -> u32 {
    game().set_shards(x, y, n.min(255) as u8) as u32
}

#[unsafe(no_mangle)]
pub extern "C" fn fx_set_amp(x: i32, y: i32, on: u32) -> u32 {
    game().set_amplifier(x, y, on != 0) as u32
}

/// Links drone port (x, y) to the port at (tx, ty); a target off the map unlinks it.
#[unsafe(no_mangle)]
pub extern "C" fn fx_link(x: i32, y: i32, tx: i32, ty: i32) -> u32 {
    game().link_port(x, y, tx, ty) as u32
}

/// Details of the building at (x, y) for the inspector, or null if there is none.
/// Layout (`INFO_LEN` words): kind, dir, recipe, status, progress, inv0, inv1, inv2, out,
/// held, aux, queue/inbox, role/outbox, x, y, flags, shards, purity, filter, network,
/// power draw (kW), effective speed (%), network satisfaction (x1000), drone state or
/// generator fuel left (%).
#[unsafe(no_mangle)]
pub extern "C" fn fx_inspect(x: i32, y: i32) -> *const u32 {
    let s = state();
    let w = &s.world;
    let t = w.grid.index(x, y);
    if t == NONE || w.grid.ent[t as usize] == NONE {
        return ::core::ptr::null();
    }
    let i = w.grid.ent[t as usize] as usize;
    let m = &w.machines.list[i];
    let def = &BUILDINGS[m.kind as usize];
    let sat = if m.draw == 0 || def.class == Class::Generator {
        FULL
    } else if m.net == NO_NET {
        0
    } else {
        w.machines.nets[m.net as usize].sat
    };
    let (a, b, extra) = match def.class {
        Class::Tunnel => (w.machines.queue_len(i) as u32, m.rr as u32, 0),
        Class::DronePort => {
            let p = &w.machines.ports[m.queue as usize];
            let state = if p.state == flight::HOME { 0 } else { p.state as u32 };
            (p.inbox.len as u32, p.outbox.len as u32, state)
        }
        Class::Generator if def.fuel.1 > 0 => {
            let left = if m.aux == NONE { 0 } else { m.aux as u64 * 100 / (def.fuel.1 as u64 * 60) };
            (0, 0, left as u32)
        }
        _ => (0, 0, 0),
    };
    s.info = [
        m.kind as u32,
        m.dir as u32,
        m.recipe as u32,
        m.status as u32,
        w.machines.progress(m) as u32,
        m.inv[0] as u32,
        m.inv[1] as u32,
        m.inv[2] as u32,
        m.out as u32,
        m.held as u32,
        m.aux,
        a,
        b,
        x as u32,
        y as u32,
        m.flags as u32,
        m.shards as u32,
        m.purity as u32,
        m.filter as u32,
        if m.net == NO_NET { NONE } else { m.net as u32 },
        m.draw,
        m.speed as u32 * 100 / SPEED_ONE,
        sat * 1000 / FULL,
        extra,
    ];
    s.info.as_ptr()
}

/// Power network `net`: demand, supply (kW), satisfaction (x1000), battery charge and
/// capacity (MJ), battery flow (kW, +charging), batteries, has Core.
#[unsafe(no_mangle)]
pub extern "C" fn fx_net_info(net: u32) -> *const u32 {
    let s = state();
    let Some(n) = s.world.machines.nets.get(net as usize) else { return ::core::ptr::null() };
    s.net = [
        n.last_demand,
        n.last_supply,
        n.sat * 1000 / FULL,
        (n.stored / 60_000) as u32,
        (n.capacity / 60_000) as u32,
        n.last_flow as i32 as u32,
        n.batteries,
        n.has_core as u32,
    ];
    s.net.as_ptr()
}

// ---- Research, the Ark, exploration, shop ---------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn fx_research(t: u32) -> u32 {
    game().research(t as u8) as u32
}

/// 0 = locked, 1 = available, 2 = researched (repeatables stay 1 while available).
#[unsafe(no_mangle)]
pub extern "C" fn fx_tech_state(t: u32) -> u32 {
    let c = &game().core;
    if t as usize >= TECH_COUNT {
        0
    } else if c.is_available(t as u8) {
        1
    } else if c.researched[t as usize] {
        2
    } else {
        0
    }
}

/// Current level of a repeatable node.
#[unsafe(no_mangle)]
pub extern "C" fn fx_tech_level(t: u32) -> u32 {
    if t as usize >= TECH_COUNT { 0 } else { game().core.level(t as u8) as u32 }
}

/// Amount of the `k`-th cost entry of node `t` right now (repeatables grow per level).
#[unsafe(no_mangle)]
pub extern "C" fn fx_tech_cost(t: u32, k: u32) -> u32 {
    if t as usize >= TECH_COUNT || k as usize >= TECH[t as usize].cost.len() {
        return 0;
    }
    game().core.tech_cost(t as u8).nth(k as usize).map_or(0, |(_, n)| n)
}

/// Sends Core storage into the current Ark phase. Returns 1 if that completed the phase.
#[unsafe(no_mangle)]
pub extern "C" fn fx_ark() -> u32 {
    game().ark_contribute() as u32
}

#[unsafe(no_mangle)]
pub extern "C" fn fx_salvage(x: i32, y: i32) -> u32 {
    game().salvage(x, y)
}

/// The alternates a data probe offers, packed a | b << 8 | c << 16 (255 = none).
#[unsafe(no_mangle)]
pub extern "C" fn fx_probe_options() -> u32 {
    let o = game().probe_options();
    o[0] as u32 | (o[1] as u32) << 8 | (o[2] as u32) << 16
}

#[unsafe(no_mangle)]
pub extern "C" fn fx_choose_alt(k: u32) -> u32 {
    game().choose_alt(k.min(255) as u8) as u32
}

#[unsafe(no_mangle)]
pub extern "C" fn fx_buy(i: u32) -> u32 {
    game().buy(i as usize) as u32
}

#[unsafe(no_mangle)]
pub extern "C" fn fx_cosmetic(bit: u32) -> u32 {
    game().set_cosmetic(bit.min(255) as u8) as u32
}

#[unsafe(no_mangle)]
pub extern "C" fn fx_can_launch(planet: u32) -> u32 {
    game().can_launch_to(planet.min(255) as u8) as u32
}

/// Launches the Ark: the world is replaced by a new planet. Returns 1 on success.
#[unsafe(no_mangle)]
pub extern "C" fn fx_launch(planet: u32) -> u32 {
    let s = state();
    match s.world.launch(planet.min(255) as u8) {
        Some(w) => {
            s.world = w;
            1
        }
        None => 0,
    }
}

/// Launch animation progress (0..=255) for the Ark sprite.
#[unsafe(no_mangle)]
pub extern "C" fn fx_set_launch(t: u32) {
    game().launch = t.min(255) as u8;
}

/// Credits production for `secs` of absence. Returns the report length.
#[unsafe(no_mangle)]
pub extern "C" fn fx_offline(secs: u32) -> u32 {
    game().apply_offline(secs)
}

// ---- Blueprints ----------------------------------------------------------------------------------

/// Copies the rectangle into the active blueprint; returns the cell count.
#[unsafe(no_mangle)]
pub extern "C" fn fx_bp_capture(x0: i32, y0: i32, x1: i32, y1: i32) -> u32 {
    game().bp_capture(x0, y0, x1, y1)
}

/// Serializes the active blueprint; the bytes are at `fx_bp_ptr()`. Returns the length.
#[unsafe(no_mangle)]
pub extern "C" fn fx_bp_write() -> u32 {
    let s = state();
    s.world.bp_write(&mut s.blueprint);
    s.blueprint.len() as u32
}

#[unsafe(no_mangle)]
pub extern "C" fn fx_bp_ptr() -> *mut u8 {
    state().blueprint.as_mut_ptr()
}

/// A buffer of `len` bytes for JS to copy a blueprint into before `fx_bp_read`.
#[unsafe(no_mangle)]
pub extern "C" fn fx_bp_buffer(len: u32) -> *mut u8 {
    let s = state();
    s.blueprint.clear();
    s.blueprint.resize(len as usize, 0);
    s.blueprint.as_mut_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn fx_bp_read() -> u32 {
    let s = state();
    s.world.bp_read(&s.blueprint)
}

#[unsafe(no_mangle)]
pub extern "C" fn fx_bp_paste(x: i32, y: i32, rot: u32) -> u32 {
    game().bp_paste(x, y, rot as u8)
}

// ---- Saves ---------------------------------------------------------------------------------------

/// Serializes the game; the bytes are at `fx_save_ptr()`. Returns the length. `now` is the
/// wall-clock time in seconds, for offline progress.
#[unsafe(no_mangle)]
pub extern "C" fn fx_save(now: f64) -> u32 {
    let s = state();
    s.world.save(&mut s.save, now);
    s.save.len() as u32
}

#[unsafe(no_mangle)]
pub extern "C" fn fx_save_ptr() -> *const u8 {
    state().save.as_ptr()
}

/// Returns a buffer of `len` bytes for JS to copy a save into, then call `fx_load`.
#[unsafe(no_mangle)]
pub extern "C" fn fx_load_buffer(len: u32) -> *mut u8 {
    let s = state();
    s.load.clear();
    s.load.resize(len as usize, 0);
    s.load.as_mut_ptr()
}

/// Wall-clock time stored in the save in the load buffer (0 if none).
#[unsafe(no_mangle)]
pub extern "C" fn fx_load_saved_at() -> f64 {
    World::saved_at(&state().load)
}

/// Replaces the world with the save in the load buffer. Returns 1 on success; on failure
/// the current world is untouched.
#[unsafe(no_mangle)]
pub extern "C" fn fx_load() -> u32 {
    let s = state();
    match World::load(&s.load) {
        Some(w) => {
            s.world = w;
            s.load = Vec::new();
            1
        }
        None => 0,
    }
}

// ---- Testing aids --------------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn fx_bench(loops: u32) {
    game().build_benchmark(loops);
}

/// Sandbox mode: everything unlocked, free and explored (benchmarks, testing).
#[unsafe(no_mangle)]
pub extern "C" fn fx_sandbox(on: u32) {
    let g = game();
    g.sandbox = on != 0;
    g.machines.free_power = on != 0;
    g.retune();
}

/// Sandbox only: adds terraforming points directly (previewing stages).
#[unsafe(no_mangle)]
pub extern "C" fn fx_sandbox_meters(heat: f64, pressure: f64, oxygen: f64, biomass: f64) {
    let w = game();
    if w.sandbox {
        w.core.add_meters(&[heat as u64, pressure as u64, oxygen as u64, biomass as u64]);
        w.update_stats();
    }
}

/// Sandbox only: gives the Core `n` of every item, completes Ark phases up to `ark`, and
/// reveals the map (for testing late-game content).
#[unsafe(no_mangle)]
pub extern "C" fn fx_sandbox_grant(n: u32, ark: u32) {
    let w = game();
    if !w.sandbox {
        return;
    }
    for s in w.core.stored.iter_mut().skip(1) {
        *s = s.saturating_add(n);
    }
    w.core.ark_phase = w.core.ark_phase.max(ark.min(content::ARK.len() as u32) as u8);
    w.core.credits += n as u64;
    w.core.revision += 1;
    w.grid.seen.fill(255);
    w.fog_dirty = [0, 0, w.grid.w, w.grid.h];
    w.fog_rev += 1;
    w.update_stats();
}
