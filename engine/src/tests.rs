use crate::content::{
    ACHIEVEMENTS, ALT_BASE, ALT_COUNT, ARK, BUILDINGS, Class, FREE, Goal, ITEM_COUNT, OBJECTIVES, SHOP, TECH,
    TECH_COUNT, bk, it, meter, tech,
};
use crate::machines::{ENTRANCE, EXIT, FULL, NO_RECIPE, status};
use crate::render::lod;
use crate::types::{MIN_GAP, NONE, SUB, dir};
use crate::world::{World, reason};

/// Removes the generated wrecks so tests control every tile.
fn clear_wrecks(w: &mut World) {
    for t in 0..w.grid.kind.len() {
        if w.grid.kind[t] == bk::WRECK {
            let i = w.grid.ent[t] as usize;
            if let Some(moved) = w.machines.remove(i) {
                w.grid.ent[moved as usize] = i as u32;
            }
            w.grid.kind[t] = bk::EMPTY;
            w.grid.ent[t] = NONE;
        }
    }
}

/// Sandbox world: no deposits or wrecks unless a test adds them, everything free, unlocked,
/// explored and powered.
fn world() -> World {
    let mut w = World::new(96, 96, 7);
    w.grid.res.fill(0);
    w.grid.purity.fill(1);
    clear_wrecks(&mut w);
    w.sandbox = true;
    w.machines.free_power = true;
    w.retune();
    w
}

/// Campaign rules: costs and research apply; the map is explored and powered.
fn campaign() -> World {
    let mut w = World::new(96, 96, 7);
    w.grid.res.fill(0);
    w.grid.purity.fill(1);
    w.grid.seen.fill(255);
    clear_wrecks(&mut w);
    w.machines.free_power = true;
    w
}

/// Campaign rules with real power: machines need a network.
fn powered() -> World {
    let mut w = campaign();
    w.machines.free_power = false;
    w.core.stored = [5000; ITEM_COUNT];
    w
}

fn belt_line(w: &mut World, x0: i32, y0: i32, len: i32, d: u8) {
    for i in 0..len {
        assert!(w.place(x0 + dir::dx(d) * i, y0 + dir::dy(d) * i, bk::BELT, d));
    }
}

fn deposit(w: &mut World, x: i32, y: i32, res: u8) {
    let t = w.grid.index(x, y);
    w.grid.res[t as usize] = res;
}

/// Offers an item to a belt tile as if it arrived travelling in direction `d`.
fn feed(w: &mut World, x: i32, y: i32, item: u8, d: u8) -> bool {
    w.ensure_built();
    let t = w.grid.index(x, y);
    w.belts.insert_from(&w.grid, t, item, d)
}

fn run(w: &mut World, ticks: u32) {
    for _ in 0..ticks {
        w.tick();
    }
}

fn machine_at(w: &World, x: i32, y: i32) -> u32 {
    w.grid.ent[w.grid.index(x, y) as usize]
}

fn items_on_belts(w: &World) -> u32 {
    let counted: u32 = w.belts.segs.iter().map(|s| s.count).sum();
    assert_eq!(counted, w.belts.item_count, "item_count bookkeeping drifted");
    counted
}

fn items_on_tiles(w: &World, tiles: &[(i32, i32)]) -> u32 {
    let mut snap = Vec::new();
    w.belts.snapshot(&mut snap);
    let ids: Vec<u32> = tiles.iter().map(|&(x, y)| w.grid.index(x, y)).collect();
    snap.iter().filter(|(t, _, _)| ids.contains(t)).count() as u32
}

fn item_positions(w: &World) -> Vec<(i32, i32, u8)> {
    let mut v = Vec::new();
    w.belts.for_each_visible_item(&w.grid, [-1e9, -1e9, 1e9, 1e9], 1.0, |x, y, it| {
        v.push(((x * 64.0).round() as i32, (y * 64.0).round() as i32, it));
    });
    v.sort();
    v
}

/// A drill on `res` six tiles west of the Core, belted into it. Returns the drill tile.
fn mine_into_core(w: &mut World, res: u8) -> (i32, i32) {
    let (cx, cy) = (w.core.x, w.core.y);
    deposit(w, cx - 6, cy, res);
    assert!(w.place(cx - 6, cy, bk::DRILL, dir::E));
    belt_line(w, cx - 5, cy, 5, dir::E);
    (cx - 6, cy)
}

// ---- Belts -----------------------------------------------------------------------------------

#[test]
fn item_travels_the_belt_and_waits_at_a_dead_end() {
    let mut w = world();
    belt_line(&mut w, 2, 2, 10, dir::E);
    assert!(feed(&mut w, 2, 2, it::IRON_ORE, dir::E));
    let ticks = 10 * SUB / w.belts.speed;
    run(&mut w, ticks - 1);
    assert_eq!(w.belts.front_ready(0), 0, "arrived too early");
    run(&mut w, 1);
    assert_eq!(w.belts.front_ready(0), it::IRON_ORE);
    run(&mut w, 100);
    assert_eq!(items_on_belts(&w), 1);
    assert_eq!(w.belts.front_ready(0), it::IRON_ORE, "blocked item must stay at the end");
}

#[test]
fn blocked_belt_compresses_to_full_density() {
    let mut w = world();
    belt_line(&mut w, 2, 2, 10, dir::E);
    for _ in 0..2000 {
        feed(&mut w, 2, 2, it::STONE, dir::E);
        w.tick();
    }
    assert_eq!(items_on_belts(&w), 10 * SUB / MIN_GAP + 1);
    let seg = w.belts.segs[0];
    assert_eq!(seg.first_moving, seg.count, "fully compressed segment should be skipped");
}

#[test]
fn belt_throughput_is_225_per_minute() {
    let mut w = world();
    let (cx, cy) = (w.core.x, w.core.y);
    belt_line(&mut w, cx - 8, cy + 1, 8, dir::E);
    for _ in 0..4000 {
        feed(&mut w, cx - 8, cy + 1, it::STONE, dir::E);
        w.tick();
    }
    let before = w.core.total_delivered;
    for _ in 0..3600 {
        feed(&mut w, cx - 8, cy + 1, it::STONE, dir::E);
        w.tick();
    }
    let per_min = w.core.total_delivered - before;
    assert!((224..=226).contains(&per_min), "got {per_min}/min");
    assert!((220..=230).contains(&w.core.rate_per_min()));
}

#[test]
fn side_load_merges_without_losing_items() {
    let mut w = world();
    let (cx, cy) = (w.core.x, w.core.y);
    belt_line(&mut w, cx - 12, cy + 2, 12, dir::E);
    belt_line(&mut w, cx - 6, cy - 4, 6, dir::S);
    w.ensure_built();
    assert_eq!(w.belts.segs.len(), 2);
    let mut fed = 0u64;
    for tick in 0..3000 {
        if tick % 40 == 0 {
            fed += feed(&mut w, cx - 12, cy + 2, it::IRON_ORE, dir::E) as u64;
        }
        fed += feed(&mut w, cx - 6, cy - 4, it::COPPER_ORE, dir::S) as u64;
        w.tick();
    }
    assert_eq!(fed, w.core.total_delivered + items_on_belts(&w) as u64, "items were lost");
    assert!(w.core.delivered[it::IRON_ORE as usize] > 0 && w.core.delivered[it::COPPER_ORE as usize] > 0);
}

#[test]
fn t_junction_accepts_both_side_feeders() {
    let mut w = world();
    let (cx, cy) = (w.core.x, w.core.y);
    belt_line(&mut w, cx + 1, cy + 6, 3, dir::N);
    assert!(w.place(cx, cy + 6, bk::BELT, dir::E));
    assert!(w.place(cx + 2, cy + 6, bk::BELT, dir::W));
    w.ensure_built();
    assert_eq!(w.belts.segs.len(), 3, "a T-junction head has no chain predecessor");
    let (mut a, mut b) = (0u64, 0u64);
    for _ in 0..3600 {
        a += feed(&mut w, cx, cy + 6, it::IRON_ORE, dir::E) as u64;
        b += feed(&mut w, cx + 2, cy + 6, it::COPPER_ORE, dir::W) as u64;
        w.tick();
    }
    assert_eq!(a + b, w.core.total_delivered + items_on_belts(&w) as u64);
    assert!(w.core.total_delivered > 200, "merged line should run near capacity");
    assert!(a.abs_diff(b) <= 3, "zipper merge should alternate fairly ({a} vs {b})");
}

#[test]
fn corner_belts_form_one_segment() {
    let mut w = world();
    belt_line(&mut w, 2, 2, 5, dir::E);
    belt_line(&mut w, 7, 2, 5, dir::S);
    w.ensure_built();
    assert_eq!(w.belts.segs.len(), 1);
    let corner = w.grid.index(7, 2) as usize;
    assert_eq!(w.belts.in_dir[corner], dir::E);
    assert_eq!(w.grid.dir[corner], dir::S);
    assert!(feed(&mut w, 2, 2, it::COAL, dir::E));
    let ticks = 10 * SUB / w.belts.speed;
    run(&mut w, ticks);
    assert_eq!(w.belts.front_ready(0), it::COAL);
}

#[test]
fn closed_loop_circulates_forever() {
    let mut w = world();
    belt_line(&mut w, 2, 2, 7, dir::E);
    belt_line(&mut w, 9, 2, 2, dir::S);
    belt_line(&mut w, 9, 4, 7, dir::W);
    belt_line(&mut w, 2, 4, 2, dir::N);
    w.ensure_built();
    assert_eq!(w.belts.segs.len(), 1);
    w.belts.fill(0, it::GEAR);
    let n = items_on_belts(&w);
    assert!(n > 20);
    let before = item_positions(&w);
    run(&mut w, 10_000);
    assert_eq!(items_on_belts(&w), n, "loop lost or duplicated items");
    w.tick();
    assert!(w.belts.segs[0].moved_from < n, "loop stalled");
    assert_ne!(before, item_positions(&w));
}

#[test]
fn long_chain_is_split_but_keeps_full_throughput() {
    let mut w = World::new(700, 16, 3);
    clear_wrecks(&mut w);
    w.sandbox = true;
    w.machines.free_power = true;
    let (cx, cy) = (w.core.x, w.core.y);
    belt_line(&mut w, 0, cy + 1, cx, dir::E);
    w.ensure_built();
    assert_eq!(w.belts.segs.len() as i32, (cx + 255) / 256);
    for _ in 0..(cx as u32 * SUB / w.belts.speed + 600) {
        feed(&mut w, 0, cy + 1, it::STONE, dir::E);
        w.tick();
    }
    let before = w.core.total_delivered;
    for _ in 0..3600 {
        feed(&mut w, 0, cy + 1, it::STONE, dir::E);
        w.tick();
    }
    let per_min = w.core.total_delivered - before;
    assert!((224..=226).contains(&per_min), "split chain delivered {per_min}/min");
}

#[test]
fn rebuild_preserves_item_positions() {
    let mut w = world();
    belt_line(&mut w, 2, 2, 10, dir::E);
    for _ in 0..200 {
        feed(&mut w, 2, 2, it::STONE, dir::E);
        w.tick();
    }
    let before = item_positions(&w);
    assert!(before.len() > 5);
    assert!(w.place(12, 2, bk::BELT, dir::E));
    w.ensure_built();
    assert_eq!(item_positions(&w), before);
}

#[test]
fn removing_a_belt_drops_only_its_items() {
    let mut w = world();
    belt_line(&mut w, 2, 2, 10, dir::E);
    for _ in 0..2000 {
        feed(&mut w, 2, 2, it::STONE, dir::E);
        w.tick();
    }
    assert_eq!(items_on_belts(&w), 21);
    assert!(w.remove(7, 2));
    w.ensure_built();
    assert_eq!(w.belts.segs.len(), 2);
    assert_eq!(items_on_belts(&w), 19, "exactly the two items on the removed tile vanish");
}

#[test]
fn head_on_belts_do_not_pass_items() {
    let mut w = world();
    belt_line(&mut w, 2, 2, 4, dir::E);
    belt_line(&mut w, 7, 2, 2, dir::W);
    assert!(feed(&mut w, 2, 2, it::STONE, dir::E));
    run(&mut w, 1000);
    let s = w.belts.tile_seg[w.grid.index(5, 2) as usize];
    assert_eq!(w.belts.front_ready(s), it::STONE, "item waits at the head-on boundary");
}

// ---- Production ------------------------------------------------------------------------------

#[test]
fn drill_feeds_core_at_its_rate() {
    let mut w = world();
    mine_into_core(&mut w, it::IRON_ORE);
    run(&mut w, 3600 * 2);
    let rate = w.core.rate_per_min();
    assert!((58..=62).contains(&rate), "drill rate {rate}/min");
}

#[test]
fn purity_scales_drill_speed() {
    for (purity, lo, hi) in [(0u8, 28, 32), (2, 118, 122)] {
        let mut w = world();
        let (cx, cy) = (w.core.x, w.core.y);
        let t = w.grid.index(cx - 6, cy) as usize;
        w.grid.purity[t] = purity;
        mine_into_core(&mut w, it::IRON_ORE);
        run(&mut w, 3600 * 2);
        let rate = w.core.rate_per_min();
        assert!((lo..=hi).contains(&rate), "purity {purity}: {rate}/min");
    }
}

#[test]
fn smelting_line_delivers_ingots() {
    let mut w = world();
    let (cx, cy) = (w.core.x, w.core.y);
    deposit(&mut w, cx - 8, cy, it::IRON_ORE);
    assert!(w.place(cx - 8, cy, bk::DRILL, dir::E));
    assert!(w.place(cx - 7, cy, bk::BELT, dir::E));
    assert!(w.place(cx - 6, cy, bk::SMELTER, dir::E));
    belt_line(&mut w, cx - 5, cy, 5, dir::E);
    run(&mut w, 1800);
    assert!(w.core.delivered[it::IRON_INGOT as usize] >= 20);
    assert_eq!(w.core.delivered[it::IRON_ORE as usize], 0, "all ore is smelted");
    assert!(w.rates.per_min[it::IRON_INGOT as usize] > 30, "production is measured per item");
}

#[test]
fn smelter_switches_recipe_when_idle() {
    let mut w = world();
    let (cx, cy) = (w.core.x, w.core.y);
    assert!(w.place(cx - 2, cy, bk::SMELTER, dir::E));
    assert!(w.place(cx - 1, cy, bk::BELT, dir::E));
    let m = machine_at(&w, cx - 2, cy);
    assert!(w.machines.accept(m, it::IRON_ORE, dir::E));
    assert!(!w.machines.accept(m, it::COPPER_ORE, dir::E), "busy with iron");
    run(&mut w, 200);
    assert!(w.machines.accept(m, it::COPPER_ORE, dir::E), "idle smelters take any ore");
    run(&mut w, 200);
    assert_eq!(w.core.delivered[it::IRON_INGOT as usize], 1);
    assert_eq!(w.core.delivered[it::COPPER_INGOT as usize], 1);
}

#[test]
fn chosen_recipe_is_kept_and_refunds_inputs_when_changed() {
    let mut w = world();
    let (cx, cy) = (w.core.x, w.core.y);
    w.place_recipe = 3; // reinforced plates
    assert!(w.place(cx - 2, cy, bk::ASSEMBLER, dir::E));
    w.place_recipe = NO_RECIPE;
    let m = machine_at(&w, cx - 2, cy);
    assert!(!w.machines.accept(m, it::COPPER_WIRE, dir::N), "locked to reinforced plates");
    assert!(w.machines.accept(m, it::STEEL, dir::N));
    let steel = w.core.stored[it::STEEL as usize];
    assert!(w.set_recipe(cx - 2, cy, 0));
    assert_eq!(w.core.stored[it::STEEL as usize], steel + 1, "buffered input returned");
    assert!(w.set_recipe(cx - 2, cy, NO_RECIPE), "back to Auto");
    assert!(w.machines.accept(m, it::COPPER_WIRE, dir::N));
}

#[test]
fn assembler_combines_inputs() {
    let mut w = world();
    let (cx, cy) = (w.core.x, w.core.y);
    assert!(w.place(cx - 2, cy, bk::ASSEMBLER, dir::E));
    assert!(w.place(cx - 1, cy, bk::BELT, dir::E));
    let m = machine_at(&w, cx - 2, cy);
    assert!(w.machines.accept(m, it::IRON_PLATE, dir::N));
    run(&mut w, 200);
    assert_eq!(w.core.delivered[it::GEAR as usize], 0, "a gear needs two plates");
    assert!(w.machines.accept(m, it::IRON_PLATE, dir::S));
    run(&mut w, 200);
    assert_eq!(w.core.delivered[it::GEAR as usize], 1);
    for item in [it::COPPER_WIRE, it::GEAR, it::COPPER_WIRE] {
        assert!(w.machines.accept(m, item, dir::E));
    }
    assert!(!w.machines.accept(m, it::IRON_PLATE, dir::E), "committed to motors");
    run(&mut w, 300);
    assert_eq!(w.core.delivered[it::MOTOR as usize], 1);
}

#[test]
fn heaters_raise_heat_and_stages() {
    let mut w = world();
    assert!(w.place(10, 10, bk::HEATER, dir::E));
    let m = machine_at(&w, 10, 10);
    assert!(w.machines.accept(m, it::COAL, dir::W));
    assert!(w.machines.accept(m, it::COAL, dir::N), "terraformers take input from any side");
    run(&mut w, 480);
    assert_eq!(w.core.meters[meter::HEAT as usize], 40);
    assert_eq!(w.core.ti, 40);
    assert_eq!(w.core.stage, 0);
    w.core.add_meters(&[2_500, 0, 0, 0]);
    assert_eq!(w.core.stage, 1);
    let credits = w.core.credits;
    w.core.add_meters(&[0, 130_000, 0, 0]);
    assert_eq!(w.core.stage, crate::content::stage::WATER, "stages can be skipped in one step");
    let paid: u64 = (2..=4).map(|s| crate::content::STAGES[s].credits as u64).sum();
    assert_eq!(w.core.credits, credits + paid, "every stage passed pays its reward");
}

#[test]
fn incinerator_destroys_items() {
    let mut w = world();
    belt_line(&mut w, 2, 2, 3, dir::E);
    assert!(w.place(5, 2, bk::INCINERATOR, dir::E));
    for _ in 0..600 {
        feed(&mut w, 2, 2, it::STONE, dir::E);
        w.tick();
    }
    assert!(w.machines.incinerated > 30);
    assert!(items_on_belts(&w) <= 7);
}

#[test]
fn shards_overclock_and_amplifiers_double_output() {
    let mut base = world();
    mine_into_core(&mut base, it::STONE);
    run(&mut base, 3600);
    let normal = base.core.delivered[it::STONE as usize];

    let mut w = world();
    w.core.stored[it::POWER_SHARD as usize] = 3;
    w.core.stored[it::AMPLIFIER as usize] = 1;
    let (x, y) = mine_into_core(&mut w, it::STONE);
    assert!(w.set_shards(x, y, 2));
    assert_eq!(w.core.stored[it::POWER_SHARD as usize], 1, "shards come from the Core");
    run(&mut w, 3600);
    let fast = w.core.delivered[it::STONE as usize];
    assert!(fast * 10 >= normal * 19 && fast * 10 <= normal * 21, "200% clock: {fast} vs {normal}");
    let i = machine_at(&w, x, y) as usize;
    assert!(w.machines.list[i].draw > BUILDINGS[bk::DRILL as usize].power * 2, "overclocking costs power");

    assert!(w.set_shards(x, y, 0));
    assert_eq!(w.core.stored[it::POWER_SHARD as usize], 3, "shards are returned");
    assert!(w.set_amplifier(x, y, true));
    assert_eq!(w.core.stored[it::AMPLIFIER as usize], 0);
    let before = w.core.delivered[it::STONE as usize];
    run(&mut w, 3600);
    let amped = w.core.delivered[it::STONE as usize] - before;
    assert!(amped * 10 >= normal * 19, "amplified: {amped} vs {normal}");
    assert!(w.remove(x, y));
    assert_eq!(w.core.stored[it::AMPLIFIER as usize], 1, "removing refunds the amplifier");
}

// ---- Power -----------------------------------------------------------------------------------

#[test]
fn machines_need_power_and_poles_extend_the_core_field() {
    let mut w = powered();
    assert!(w.research(tech::PRESSING) && w.research(tech::POWER));
    let (cx, cy) = (w.core.x, w.core.y);
    deposit(&mut w, cx - 6, cy, it::STONE);
    assert!(w.place(cx - 6, cy, bk::DRILL, dir::E));
    let (fx, fy) = (cx - 40, cy);
    deposit(&mut w, fx, fy, it::STONE);
    assert!(w.place(fx, fy, bk::DRILL, dir::E));
    run(&mut w, 200);
    let near = machine_at(&w, cx - 6, cy) as usize;
    let far = machine_at(&w, fx, fy) as usize;
    assert_eq!(w.machines.list[near].status, status::BLOCKED, "mined and waiting for a belt");
    assert_eq!(w.machines.list[far].status, status::NO_POWER);
    let mut x = cx - 16;
    while x > fx - 6 {
        assert!(w.place(x, cy + 2, bk::POLE, 0));
        x -= 9;
    }
    run(&mut w, 200);
    let far = machine_at(&w, fx, fy) as usize;
    assert_ne!(w.machines.list[far].status, status::NO_POWER, "pole chain carries the Core's power");
    assert_eq!(w.machines.nets.len(), 1, "one connected network");
}

#[test]
fn generators_burn_fuel_only_as_needed_and_brownouts_slow_machines() {
    let mut w = powered();
    for t in [tech::PRESSING, tech::POWER, tech::ASSEMBLY, tech::INCINERATION] {
        assert!(w.research(t));
    }
    let (bx, by) = (10, 10);
    assert!(w.place(bx, by, bk::POLE, 0));
    for k in 0..8 {
        assert!(w.place(bx - 4 + k, by + 2, bk::ASSEMBLER, dir::S));
        assert!(w.place(bx - 4 + k, by + 3, bk::INCINERATOR, dir::S));
    }
    // Each assembler has plates, so all want to work: 80 MW of demand.
    let feed_all = |w: &mut World| {
        for k in 0..8 {
            let m = machine_at(w, bx - 4 + k, by + 2);
            while w.machines.accept(m, it::IRON_PLATE, dir::S) {}
        }
    };
    feed_all(&mut w);
    run(&mut w, 3);
    let net = w.machines.list[machine_at(&w, bx, by) as usize].net as usize;
    assert_eq!(w.machines.nets[net].sat, 0, "no generator: no power");
    assert!(w.place(bx + 2, by, bk::COAL_GEN, 0));
    let g = machine_at(&w, bx + 2, by);
    for _ in 0..10 {
        assert!(w.machines.accept(g, it::COAL, dir::E));
    }
    for _ in 0..10 {
        feed_all(&mut w);
        run(&mut w, 60);
    }
    let net = w.machines.list[g as usize].net as usize;
    let sat = w.machines.nets[net].sat * 100 / FULL;
    assert!((70..=80).contains(&sat), "60 of 80 MW: {sat}%");
    let left = w.machines.list[g as usize].inv[0];
    assert!((7..=8).contains(&left), "about two coal burned in 10 s: {left} left");
}

#[test]
fn batteries_store_surplus_and_cover_the_night() {
    let mut w = powered();
    w.sandbox = true;
    w.retune();
    let (bx, by) = (10, 10);
    assert!(w.place(bx, by, bk::POLE, 0));
    for k in 0..4 {
        assert!(w.place(bx - 2 + k, by - 2, bk::SOLAR, 0));
    }
    assert!(w.place(bx + 2, by + 2, bk::BATTERY_BANK, 0));
    w.tick = crate::sky::DAY_TICKS / 2 - crate::sky::DAY_TICKS * 28 / 100; // noon
    run(&mut w, 1200);
    let net = w.machines.list[machine_at(&w, bx, by) as usize].net as usize;
    let charged = w.machines.nets[net].stored;
    assert!(charged > 0, "the sun charges the battery");
    // At night the panels give nothing, and the battery carries a small load.
    w.tick = crate::sky::DAY_TICKS - crate::sky::DAY_TICKS * 28 / 100 - 600;
    assert!(w.place(bx - 1, by + 2, bk::CRUSHER, 0));
    let c = machine_at(&w, bx - 1, by + 2);
    for _ in 0..2 {
        w.machines.accept(c, it::STONE, dir::N);
    }
    run(&mut w, 30);
    assert_eq!(crate::sky::daylight(w.tick), 0);
    let net = w.machines.list[c as usize].net as usize;
    assert_eq!(w.machines.nets[net].sat, FULL, "battery covers the crusher");
    assert!(w.machines.nets[net].stored < charged);
}

// ---- Logistics --------------------------------------------------------------------------------

#[test]
fn splitter_deals_evenly_and_skips_missing_outputs() {
    let mut w = world();
    belt_line(&mut w, 10, 20, 3, dir::E);
    assert!(w.place(13, 20, bk::SPLITTER, dir::E));
    belt_line(&mut w, 14, 20, 12, dir::E);
    belt_line(&mut w, 13, 19, 12, dir::N);
    belt_line(&mut w, 13, 21, 12, dir::S);
    for _ in 0..900 {
        feed(&mut w, 10, 20, it::STONE, dir::E);
        w.tick();
    }
    let front: Vec<_> = (14..26).map(|x| (x, 20)).collect();
    let left: Vec<_> = (8..20).map(|y| (13, y)).collect();
    let right: Vec<_> = (21..33).map(|y| (13, y)).collect();
    let (f, l, r) = (items_on_tiles(&w, &front), items_on_tiles(&w, &left), items_on_tiles(&w, &right));
    assert!(f > 10 && f.abs_diff(l) <= 1 && f.abs_diff(r) <= 1, "uneven split: {f} {l} {r}");
}

#[test]
fn sorter_pulls_its_item_out_of_a_mixed_belt() {
    let mut w = world();
    belt_line(&mut w, 10, 20, 3, dir::E);
    w.place_filter = it::COPPER_ORE;
    assert!(w.place(13, 20, bk::SORTER, dir::E));
    w.place_filter = 0;
    belt_line(&mut w, 14, 20, 12, dir::E);
    belt_line(&mut w, 13, 19, 12, dir::N);
    belt_line(&mut w, 13, 21, 12, dir::S);
    let mut copper = true;
    for _ in 0..900 {
        let item = if copper { it::COPPER_ORE } else { it::IRON_ORE };
        if feed(&mut w, 10, 20, item, dir::E) {
            copper = !copper;
        }
        w.tick();
    }
    let mut snap = Vec::new();
    w.belts.snapshot(&mut snap);
    let at = |x: i32, y: i32| w.grid.index(x, y);
    let front: Vec<u8> =
        snap.iter().filter(|(t, _, _)| (14..26).any(|x| at(x, 20) == *t)).map(|&(_, _, i)| i).collect();
    let sides: Vec<u8> = snap
        .iter()
        .filter(|(t, _, _)| (8..20).any(|y| at(13, y) == *t) || (21..33).any(|y| at(13, y) == *t))
        .map(|&(_, _, i)| i)
        .collect();
    assert!(front.len() > 5 && front.iter().all(|&i| i == it::COPPER_ORE), "front: {front:?}");
    assert!(sides.len() > 5 && sides.iter().all(|&i| i == it::IRON_ORE));
}

#[test]
fn storage_buffers_one_item_and_refunds_on_removal() {
    let mut w = world();
    assert!(w.place(20, 20, bk::STORAGE, dir::E));
    let s = machine_at(&w, 20, 20);
    for _ in 0..30 {
        assert!(w.machines.accept(s, it::GEAR, dir::E));
    }
    assert!(!w.machines.accept(s, it::STEEL, dir::E), "one kind at a time");
    assert!(!w.machines.accept(s, it::GEAR, dir::W), "not through its output");
    belt_line(&mut w, 21, 20, 3, dir::E);
    run(&mut w, 60);
    let left = w.machines.list[s as usize].aux;
    assert!(left < 30, "releases forward");
    let gears = w.core.stored[it::GEAR as usize];
    assert!(w.remove(20, 20));
    assert_eq!(w.core.stored[it::GEAR as usize], gears + left, "contents go to the Core");
}

#[test]
fn tunnel_carries_items_under_a_crossing_belt() {
    let mut w = world();
    belt_line(&mut w, 27, 30, 3, dir::E);
    assert!(w.place(30, 30, bk::TUNNEL, dir::E));
    assert!(w.place(34, 30, bk::TUNNEL, dir::E));
    belt_line(&mut w, 35, 30, 6, dir::E);
    belt_line(&mut w, 32, 26, 8, dir::S);
    let (a, b) = (machine_at(&w, 30, 30) as usize, machine_at(&w, 34, 30) as usize);
    assert_eq!((w.machines.list[a].rr, w.machines.list[b].rr), (ENTRANCE, EXIT));
    let mut fed = 0u64;
    for _ in 0..900 {
        fed += feed(&mut w, 27, 30, it::IRON_ORE, dir::E) as u64;
        feed(&mut w, 32, 26, it::COAL, dir::S);
        w.tick();
    }
    let out: Vec<_> = (35..41).map(|x| (x, 30)).collect();
    let crossing: Vec<_> = (26..34).map(|y| (32, y)).collect();
    assert!(items_on_tiles(&w, &out) >= 10, "items should emerge from the exit");
    assert!(items_on_tiles(&w, &crossing) >= 10, "the crossing belt is unaffected");
    assert!(fed >= items_on_tiles(&w, &out) as u64 + w.machines.queue_len(a) as u64);
}

#[test]
fn tunnels_pair_in_either_order_and_unpair_on_removal() {
    let mut w = world();
    assert!(w.place(34, 40, bk::TUNNEL, dir::E));
    let far = machine_at(&w, 34, 40) as usize;
    assert_eq!(w.machines.list[far].aux, NONE);
    assert!(!w.machines.accept(far as u32, it::STONE, dir::E), "unpaired tunnels take nothing");
    assert!(w.place(30, 40, bk::TUNNEL, dir::E));
    let near = machine_at(&w, 30, 40) as usize;
    let far = machine_at(&w, 34, 40) as usize;
    assert_eq!((w.machines.list[near].rr, w.machines.list[far].rr), (ENTRANCE, EXIT));
    assert!(w.remove(34, 40));
    let near = machine_at(&w, 30, 40) as usize;
    assert_eq!(w.machines.list[near].aux, NONE);
    assert!(w.place(37, 40, bk::TUNNEL, dir::E), "7 tiles away: out of range");
    let near = machine_at(&w, 30, 40) as usize;
    assert_eq!(w.machines.list[near].aux, NONE);
}

#[test]
fn drones_fly_items_between_linked_ports() {
    let mut w = world();
    assert!(w.place(10, 10, bk::DRONE_PORT, dir::E));
    assert!(w.place(70, 60, bk::DRONE_PORT, dir::E));
    belt_line(&mut w, 71, 60, 4, dir::E);
    let a = machine_at(&w, 10, 10);
    assert!(!w.machines.accept(a, it::GEAR, dir::E), "unlinked ports take nothing");
    assert!(w.link_port(10, 10, 70, 60));
    assert!(!w.link_port(10, 10, 10, 10), "not to itself");
    for _ in 0..30 {
        assert!(w.machines.accept(a, it::GEAR, dir::E));
    }
    run(&mut w, 60);
    let port = w.machines.list[a as usize].queue as usize;
    assert!(w.drone_pos(port, 0.0).is_some(), "a full load takes off at once");
    run(&mut w, 800);
    let arrived = items_on_tiles(&w, &[(71, 60), (72, 60), (73, 60), (74, 60)]);
    assert!(arrived >= 7, "cargo reached the far belt: {arrived}");
    assert!(w.remove(70, 60));
    let a = machine_at(&w, 10, 10) as usize;
    assert_eq!(w.machines.list[a].aux, NONE, "link cleared when the target goes");
}

// ---- Economy, research, objectives ------------------------------------------------------

#[test]
fn buildings_cost_materials_and_refund_on_removal() {
    let mut w = campaign();
    deposit(&mut w, 10, 10, it::IRON_ORE);
    let (fe, cu) = (w.core.stored[it::IRON_INGOT as usize], w.core.stored[it::COPPER_INGOT as usize]);
    assert!(w.place(10, 10, bk::DRILL, dir::E));
    assert_eq!(w.core.stored[it::IRON_INGOT as usize], fe - 5);
    assert_eq!(w.core.stored[it::COPPER_INGOT as usize], cu - 2);
    assert!(w.remove(10, 10));
    assert_eq!(w.core.stored[it::IRON_INGOT as usize], fe);
    assert_eq!(w.core.stored[it::COPPER_INGOT as usize], cu);
    w.core.stored = [0; ITEM_COUNT];
    assert_eq!(w.check_place(10, 10, bk::DRILL), reason::COST);
    assert_eq!(w.check_place(11, 10, bk::DRILL), reason::NO_DEPOSIT);
    assert!(w.place(11, 10, bk::BELT, dir::E), "belts are free");
    deposit(&mut w, 12, 12, it::CRUDE_OIL);
    assert_eq!(w.check_place(12, 12, bk::DRILL), reason::WRONG_DEPOSIT, "oil needs an oil pump");
}

#[test]
fn fog_blocks_building_until_explored() {
    let mut w = World::new(128, 128, 5);
    clear_wrecks(&mut w);
    let (cx, cy) = (w.core.x, w.core.y);
    assert_eq!(w.check_place(cx - 30, cy, bk::BELT), reason::FOG);
    // Belts reveal a little ahead, so a line can be pushed out into the dark.
    let mut x = cx - 5;
    while x > 2 && w.check_place(x, cy, bk::BELT) == reason::OK {
        assert!(w.place(x, cy, bk::BELT, dir::W));
        x -= 1;
    }
    assert!(x <= 2, "the line kept revealing: stopped {} tiles out", cx - x);
    assert!(w.fog_dirty[0] < w.fog_dirty[2], "revealed area is reported to JS");
}

#[test]
fn research_unlocks_buildings_in_order_and_tiers_need_the_ark() {
    let mut w = campaign();
    assert_eq!(w.check_place(10, 10, bk::PRESS), reason::LOCKED);
    assert!(!w.research(tech::ASSEMBLY), "needs Pressing first");
    assert!(w.research(tech::PRESSING));
    assert!(!w.research(tech::PRESSING), "only once");
    assert_eq!(w.core.stored[it::IRON_INGOT as usize], 20, "40 start - 20");
    assert!(w.place(10, 10, bk::PRESS, dir::E));
    w.core.stored = [1000; ITEM_COUNT];
    for t in [tech::ASSEMBLY, tech::STEELMAKING, tech::CRUSHING, tech::ELECTRONICS] {
        assert!(w.research(t));
    }
    assert!(!w.research(tech::MELTING), "needs the Warming stage");
    w.core.add_meters(&[2_500, 0, 0, 0]);
    assert!(w.research(tech::MELTING));
    assert!(!w.research(tech::SILICA), "tier 2 needs the Ark's Foundation");
    w.core.stored = [0; ITEM_COUNT];
    for &(item, n) in ARK[0].cost {
        w.core.stored[item as usize] = n / 2;
    }
    assert!(!w.ark_contribute(), "half paid");
    assert!(w.core.ark_paid[..ARK[0].cost.len()].iter().all(|&p| p > 0), "partial payments count");
    for &(item, n) in ARK[0].cost {
        w.core.stored[item as usize] = n;
    }
    assert!(w.ark_contribute(), "phase complete");
    assert_eq!(w.core.ark_phase, 1);
    assert!(w.core.stored.iter().any(|&n| n > 0), "only what was needed was taken");
    w.core.stored = [1000; ITEM_COUNT];
    assert!(w.research(tech::SILICA));
    deposit(&mut w, 20, 20, it::TITANIUM_ORE);
    assert_eq!(w.check_place(20, 20, bk::DRILL), reason::DEPOSIT_LOCKED);
}

#[test]
fn research_unlocks_recipes_and_repeatables_scale() {
    let mut w = campaign();
    w.core.stored = [100_000; ITEM_COUNT];
    for t in [tech::PRESSING, tech::ASSEMBLY, tech::STEELMAKING] {
        assert!(w.research(t));
    }
    let (cx, cy) = (w.core.x, w.core.y);
    assert!(w.place(cx - 2, cy, bk::FOUNDRY, dir::E));
    let f = machine_at(&w, cx - 2, cy);
    assert!(!w.machines.accept(f, it::SAND, dir::N), "silicon is locked");
    w.core.ark_phase = 3;
    for t in [tech::CRUSHING, tech::ELECTRONICS, tech::SILICA, tech::DRILLS2] {
        assert!(w.research(t));
    }
    assert!(w.machines.accept(f, it::SAND, dir::N), "silicon unlocked");
    let first = w.core.tech_cost(tech::R_MINING).next().unwrap().1;
    assert!(w.research(tech::R_MINING));
    let second = w.core.tech_cost(tech::R_MINING).next().unwrap().1;
    assert_eq!(second, first * 3 / 2);
    assert!(w.research(tech::R_MINING));
    assert_eq!(w.core.level(tech::R_MINING), 2);
    assert_eq!(w.machines.drill_pct, 150 * 120 / 100, "tier 2 drills + 20%");
}

#[test]
fn belt_research_raises_throughput() {
    let mut w = world();
    w.core.stored = [1000; ITEM_COUNT];
    for t in [tech::PRESSING, tech::ASSEMBLY, tech::STEELMAKING, tech::BELTS2] {
        assert!(w.research(t));
    }
    assert_eq!(w.belts.speed, 4);
    let (cx, cy) = (w.core.x, w.core.y);
    belt_line(&mut w, cx - 8, cy + 1, 8, dir::E);
    for _ in 0..3000 {
        feed(&mut w, cx - 8, cy + 1, it::STONE, dir::E);
        w.tick();
    }
    let before = w.core.total_delivered;
    for _ in 0..3600 {
        feed(&mut w, cx - 8, cy + 1, it::STONE, dir::E);
        w.tick();
    }
    let per_min = w.core.total_delivered - before;
    assert!((299..=301).contains(&per_min), "tier 2 belts delivered {per_min}/min");
}

#[test]
fn objectives_advance_and_pay_rewards() {
    let mut w = campaign();
    deposit(&mut w, 10, 10, it::IRON_ORE);
    assert_eq!(w.core.objective, 0);
    let fe = w.core.stored[it::IRON_INGOT as usize];
    assert!(w.place(10, 10, bk::DRILL, dir::E));
    run(&mut w, 16);
    assert_eq!(w.core.objective, 1);
    assert_eq!(w.core.stored[it::IRON_INGOT as usize], fe - 5 + 10, "reward paid");
    w.core.objective = OBJECTIVES.len() as u32 + 57;
    w.core.begin_goal();
    let (have, need) = w.core.goal_progress(&w.machines.counts);
    assert!(need > have, "endless goals never run out");
}

#[test]
fn achievements_pay_credits_once() {
    let mut w = world();
    mine_into_core(&mut w, it::STONE);
    run(&mut w, 420);
    assert!(w.core.achievements & 1 != 0, "first delivery");
    assert_eq!(w.core.credits, ACHIEVEMENTS[0].credits as u64);
    run(&mut w, 240);
    assert_eq!(w.core.credits, ACHIEVEMENTS[0].credits as u64, "paid once");
}

#[test]
fn shop_sells_items_and_cosmetics_for_credits() {
    let mut w = world();
    assert!(!w.buy(0), "no credits yet");
    w.core.credits = 10_000;
    assert!(w.buy(0));
    assert_eq!(w.core.stored[it::POWER_SHARD as usize], 1);
    assert_eq!(w.core.credits, 10_000 - SHOP[0].price as u64);
    let belt = SHOP.iter().position(|s| s.key == "belt_teal").unwrap();
    assert!(!w.set_cosmetic(2), "not owned");
    assert!(w.buy(belt));
    assert!(!w.buy(belt), "owned already");
    assert!(w.set_cosmetic(2));
    assert_eq!(w.core.belt_color, 2);
}

#[test]
fn machines_replace_belts_and_rotate_in_place() {
    let mut w = world();
    assert!(w.place(5, 5, bk::BELT, dir::E));
    assert!(w.place(5, 5, bk::SMELTER, dir::S));
    assert_eq!(w.grid.kind[w.grid.index(5, 5) as usize], bk::SMELTER);
    assert!(w.belts.list.is_empty());
    assert!(w.place(5, 5, bk::SMELTER, dir::N));
    assert_eq!(w.machines.list.len(), 1);
    assert_eq!(w.machines.list[0].dir, dir::N);
    assert_eq!(w.check_place(5, 5, bk::PRESS), reason::OCCUPIED);
}

#[test]
fn undo_reverts_whole_gestures_with_refunds() {
    let mut w = campaign();
    deposit(&mut w, 10, 10, it::IRON_ORE);
    deposit(&mut w, 11, 10, it::IRON_ORE);
    let start = w.core.stored;
    w.edit_begin();
    assert!(w.place(10, 10, bk::DRILL, dir::E));
    assert!(w.place(11, 10, bk::DRILL, dir::E));
    w.edit_begin();
    belt_line(&mut w, 12, 10, 3, dir::E);
    assert_eq!(w.undo_depth(), 2);
    assert!(w.undo());
    assert!(w.belts.list.is_empty());
    assert_eq!(w.machines.list.len(), 2);
    assert!(w.undo());
    assert!(w.machines.list.is_empty());
    assert_eq!(w.core.stored, start, "costs refunded");
    assert!(!w.undo());
}

// ---- Exploration -------------------------------------------------------------------------------

#[test]
fn wrecks_are_salvaged_for_logs_probes_and_caches() {
    let mut w = World::new(512, 512, 11);
    let wrecks: Vec<u32> =
        (0..w.grid.kind.len() as u32).filter(|&t| w.grid.kind[t as usize] == bk::WRECK).collect();
    assert!(wrecks.len() >= 20, "{} wrecks", wrecks.len());
    let far = *wrecks
        .iter()
        .max_by_key(|&&t| {
            let (x, y) = w.grid.xy(t);
            (x - w.core.x).abs() + (y - w.core.y).abs()
        })
        .unwrap();
    let (x, y) = w.grid.xy(far);
    assert_eq!(w.salvage(x, y), crate::explore::found::NOTHING, "hidden in the fog");
    w.grid.seen.fill(255);
    for &t in &wrecks {
        let (x, y) = w.grid.xy(t);
        assert_ne!(w.salvage(x, y), crate::explore::found::NOTHING);
    }
    assert_eq!(w.core.salvaged as usize, wrecks.len());
    assert_eq!(w.core.logs, 10, "every log recovered");
    assert!(w.core.probes >= 5);
    assert!(w.core.stored[it::AMPLIFIER as usize] >= 2);
    assert_eq!(w.machine_count(), 0, "wrecks are gone");
    let offers = w.probe_options();
    assert!(offers.iter().all(|&k| (k as usize) < ALT_COUNT));
    assert!(w.choose_alt(offers[1]));
    assert!(w.core.alts & (1 << offers[1]) != 0);
    assert!(!w.choose_alt(200));
}

#[test]
fn alternate_recipes_need_a_probe_then_run() {
    let mut w = campaign();
    w.core.stored = [1000; ITEM_COUNT];
    let (cx, cy) = (w.core.x, w.core.y);
    assert!(w.place(cx - 2, cy, bk::SMELTER, dir::E));
    assert!(w.place(cx - 1, cy, bk::BELT, dir::E));
    let recipes = BUILDINGS[bk::SMELTER as usize].recipes;
    let pure_iron = recipes.iter().position(|r| r.name == "Pure Iron").unwrap() as u8;
    assert!(!w.set_recipe(cx - 2, cy, pure_iron), "locked alternate");
    let k = recipes[pure_iron as usize].unlock - ALT_BASE;
    w.core.alts |= 1 << k;
    w.retune();
    assert!(w.set_recipe(cx - 2, cy, pure_iron));
    let m = machine_at(&w, cx - 2, cy);
    let ingots = w.core.delivered[it::IRON_INGOT as usize];
    assert!(w.machines.accept(m, it::IRON_ORE, dir::N));
    assert!(w.machines.accept(m, it::WATER, dir::N));
    run(&mut w, 200);
    assert_eq!(w.core.delivered[it::IRON_INGOT as usize] - ingots, 2, "two ingots from one ore");
}

#[test]
fn radar_reveals_a_widening_circle() {
    let mut w = world();
    w.grid.seen.fill(0);
    assert!(w.place(40, 40, bk::RADAR, 0));
    let t = w.grid.index(40 + 30, 40) as usize;
    assert_eq!(w.grid.seen[t], 0);
    run(&mut w, 30 * 30);
    assert_eq!(w.grid.seen[t], 255, "30 tiles out after 15 s");
}

#[test]
fn meteor_showers_leave_meteorite_deposits() {
    let mut w = world();
    w.core.ark_phase = 2;
    run(&mut w, 3 * 60 * 60 + 45 * 60);
    assert!(w.sky.landed > 5, "{} meteorites", w.sky.landed);
    let meteorites = w.grid.res.iter().filter(|&&r| r == it::METEORITE).count();
    assert!(meteorites > 5);
    assert!(w.res_rev > 0);
}

// ---- Blueprints ----------------------------------------------------------------------------------

#[test]
fn blueprints_copy_and_paste_rotated() {
    let mut w = world();
    w.place_recipe = 1; // wire
    assert!(w.place(10, 10, bk::PRESS, dir::E));
    w.place_recipe = NO_RECIPE;
    belt_line(&mut w, 11, 10, 3, dir::E);
    assert_eq!(w.bp_capture(10, 10, 13, 10), 4);
    let mut bytes = Vec::new();
    w.bp_write(&mut bytes);
    assert_eq!(bytes.len(), 4 * crate::blueprint::BP_CELL_BYTES);
    assert_eq!(w.bp_read(&bytes), 4);
    assert_eq!(w.bp_paste(40, 60, 1), 4, "rotated a quarter turn clockwise");
    let press: Vec<u32> =
        (0..w.grid.kind.len() as u32).filter(|&t| w.grid.kind[t as usize] == bk::PRESS).collect();
    assert_eq!(press.len(), 2);
    let p = press.iter().copied().find(|&t| w.grid.xy(t) != (10, 10)).unwrap();
    assert_eq!(w.grid.dir[p as usize], dir::S);
    let m = &w.machines.list[w.grid.ent[p as usize] as usize];
    assert_eq!(m.recipe, 1, "the chosen recipe travels with the blueprint");
    let (px, py) = w.grid.xy(p);
    for k in 1..4 {
        assert_eq!(w.grid.kind[w.grid.index(px, py + k) as usize], bk::BELT);
    }
}

// ---- Progress -------------------------------------------------------------------------------------

#[test]
fn launching_starts_a_new_planet_with_a_bonus() {
    let mut w = world();
    w.core.credits = 777;
    w.core.achievements = 5;
    assert!(w.launch(0).is_none(), "the Ark isn't finished");
    w.core.ark_phase = ARK.len() as u8;
    assert!(w.launch(2).is_none(), "Thalassa needs two launches");
    let mut n = w.launch(1).expect("launch to Ember");
    assert_eq!(n.core.legacy, 1);
    assert_eq!(n.planet, 1);
    assert_eq!(n.core.credits, 777);
    assert_eq!(n.core.achievements, 5);
    assert_eq!(n.core.ark_phase, 0);
    assert_eq!(n.machines.craft_pct, 110, "+10% production per launch");
    assert!(n.core.stage >= 1, "Ember starts warm");
    n.grid.seen.fill(255);
    if let Some(t) =
        (0..n.grid.kind.len() as u32).find(|&t| n.is_lava(t) && n.grid.kind[t as usize] == bk::EMPTY)
    {
        let (x, y) = n.grid.xy(t);
        assert_eq!(n.check_place(x, y, bk::BELT), reason::LAVA);
    }
}

#[test]
fn offline_progress_credits_recorded_rates() {
    let mut w = world();
    mine_into_core(&mut w, it::STONE);
    run(&mut w, 3600);
    let mut bytes = Vec::new();
    w.save(&mut bytes, 1000.0);
    assert_eq!(World::saved_at(&bytes), 1000.0);
    let mut l = World::load(&bytes).unwrap();
    let stone = l.core.stored[it::STONE as usize];
    let len = l.apply_offline(600);
    assert!(len > 3);
    let gained = l.core.stored[it::STONE as usize] - stone;
    assert!((550..=650).contains(&gained), "ten minutes at 60/min: {gained}");
    assert_eq!(l.apply_offline(30), 0, "short absences don't count");
}

// ---- Saves -------------------------------------------------------------------------------------

#[test]
fn save_and_load_round_trip_deterministically() {
    let mut w = world();
    let (cx, cy) = (w.core.x, w.core.y);
    deposit(&mut w, cx - 12, cy, it::IRON_ORE);
    assert!(w.place(cx - 12, cy, bk::DRILL, dir::E));
    belt_line(&mut w, cx - 11, cy, 2, dir::E);
    assert!(w.place(cx - 9, cy, bk::SMELTER, dir::E));
    assert!(w.place(cx - 8, cy, bk::TUNNEL, dir::E));
    assert!(w.place(cx - 4, cy, bk::TUNNEL, dir::E));
    belt_line(&mut w, cx - 3, cy, 3, dir::E);
    deposit(&mut w, cx - 12, cy + 2, it::COAL);
    assert!(w.place(cx - 12, cy + 2, bk::DRILL, dir::E));
    belt_line(&mut w, cx - 11, cy + 2, 4, dir::E);
    assert!(w.place(cx - 7, cy + 2, bk::HEATER, dir::E));
    assert!(w.place(cx - 12, cy + 6, bk::DRONE_PORT, dir::E));
    assert!(w.place(cx + 12, cy + 6, bk::DRONE_PORT, dir::E));
    assert!(w.link_port(cx - 12, cy + 6, cx + 12, cy + 6));
    let port = machine_at(&w, cx - 12, cy + 6);
    for _ in 0..10 {
        w.machines.accept(port, it::GEAR, dir::E);
    }
    assert!(w.place(cx - 14, cy + 8, bk::STORAGE, dir::E));
    let st = machine_at(&w, cx - 14, cy + 8);
    w.machines.accept(st, it::STEEL, dir::W);
    assert!(w.place(cx + 3, cy - 8, bk::BATTERY_BANK, 0));
    assert!(w.place(cx - 10, cy - 6, bk::COAL_GEN, 0));
    w.machines.free_power = false;
    w.core.stored[it::IRON_INGOT as usize] += 100;
    assert!(w.research(tech::PRESSING));
    w.grid.seen.fill(0);
    w.reveal(cx, cy, 30);
    run(&mut w, 900);
    assert!(w.core.delivered[it::IRON_INGOT as usize] > 0);
    assert!(w.core.meters[meter::HEAT as usize] > 0);

    let mut bytes = Vec::new();
    w.save(&mut bytes, 5.0);
    let mut l = World::load(&bytes).expect("save should load");
    l.sandbox = true;
    let mut again = Vec::new();
    l.save(&mut again, 5.0);
    assert_eq!(bytes, again, "load then save must reproduce the same bytes");

    run(&mut w, 1200);
    run(&mut l, 1200);
    w.clear_rate_history();
    l.clear_rate_history();
    let (mut a, mut b) = (Vec::new(), Vec::new());
    w.save(&mut a, 5.0);
    l.save(&mut b, 5.0);
    assert_eq!(a, b, "the loaded world must evolve identically");
    assert!(World::load(&bytes[..bytes.len() - 3]).is_none(), "truncated saves are rejected");
    assert!(World::load(b"nonsense").is_none());
}

/// Writes a save in the version 1 format (before power, tiers and exploration).
fn v1_save(seed: u32, drill: (i32, i32)) -> Vec<u8> {
    let (w, h) = (96u16, 96u16);
    let mut out = Vec::new();
    out.extend_from_slice(b"BWSV");
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&w.to_le_bytes());
    out.extend_from_slice(&h.to_le_bytes());
    out.extend_from_slice(&seed.to_le_bytes());
    out.extend_from_slice(&500u32.to_le_bytes()); // tick
    out.extend_from_slice(&3u32.to_le_bytes()); // objective
    out.extend_from_slice(&0u32.to_le_bytes());
    for m in [3_000u64, 0, 0, 0] {
        out.extend_from_slice(&m.to_le_bytes());
    }
    out.extend_from_slice(&77u64.to_le_bytes());
    out.extend_from_slice(&1u32.to_le_bytes()); // Pressing researched
    for i in 0..25u32 {
        // v1 item 8 = iron ingot, 12 = iron plate.
        let stored: u32 = match i {
            8 => 111,
            12 => 222,
            _ => 0,
        };
        out.extend_from_slice(&stored.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
    }
    let (x, y) = drill;
    let t = |x: i32, y: i32| (y * w as i32 + x) as u32;
    out.extend_from_slice(&3u32.to_le_bytes());
    for (tile, kind) in [(t(x, y), 3u8), (t(x + 1, y), 1), (t(x + 2, y), 1)] {
        out.extend_from_slice(&tile.to_le_bytes());
        out.extend_from_slice(&[kind, 0]);
    }
    // Machines: the drill, holding a finished stone.
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&t(x, y).to_le_bytes());
    out.extend_from_slice(&[255, 0, 0, 0, 0, 0, 3]);
    out.extend_from_slice(&10u16.to_le_bytes());
    out.extend_from_slice(&3u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // tunnels
    // One item (v1 id 12, iron plate) on the first belt.
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&t(x + 1, y).to_le_bytes());
    out.extend_from_slice(&[10, 12]);
    out
}

#[test]
fn version_1_saves_still_load() {
    let probe = World::new(96, 96, 9);
    let (cx, cy) = (probe.core.x, probe.core.y);
    let bytes = v1_save(9, (cx - 10, cy + 1));
    let w = World::load(&bytes).expect("v1 save loads");
    assert_eq!(w.core.stored[it::IRON_INGOT as usize], 111, "item ids remapped");
    assert_eq!(w.core.stored[it::IRON_PLATE as usize], 222);
    assert!(w.core.researched[tech::PRESSING as usize]);
    assert_eq!(w.core.stage, 1);
    let d = machine_at(&w, cx - 10, cy + 1) as usize;
    assert_eq!(w.machines.list[d].class(), Class::Drill);
    assert_eq!(w.machines.list[d].out, 1, "held ore became output");
    assert_eq!(w.machines.list[d].timer, 10 * 64);
    assert_eq!(w.belts.item_count, 1);
    let mut snap = Vec::new();
    w.belts.snapshot(&mut snap);
    assert_eq!(snap[0].2, it::IRON_PLATE);
    assert!(w.grid.seen[w.grid.index(cx - 10, cy + 1) as usize] != 0, "the old factory is explored");
    assert!(w.core.objective > 0, "finished goals are skipped");
}

// ---- Rendering & content ----------------------------------------------------------------------

#[test]
fn render_culls_to_the_view() {
    let mut w = world();
    belt_line(&mut w, 2, 2, 10, dir::E);
    belt_line(&mut w, 60, 60, 10, dir::E);
    assert_eq!(w.render(0.0, 0.0, 20.0, 20.0, 0.0, lod::ALL), 10);
    assert_eq!(w.render(0.0, 0.0, 96.0, 96.0, 0.0, lod::ALL), 20 + 1, "all belts plus the core");
    assert_eq!(
        w.render(0.0, 0.0, 96.0, 96.0, 0.0, lod::ITEMS),
        1,
        "far zoom: structures come from the tile map"
    );
}

#[test]
fn content_tables_are_consistent() {
    // Research: valid, acyclic prerequisites; tiers 1..=5.
    let mut done = [false; TECH_COUNT];
    for _ in 0..TECH_COUNT {
        for (i, t) in TECH.iter().enumerate() {
            if t.requires.iter().all(|&r| done[r as usize]) {
                done[i] = true;
            }
        }
    }
    assert!(done.iter().all(|&d| d), "research has a cycle or a missing prerequisite");
    assert!(TECH.iter().all(|t| (1..=5).contains(&t.tier)));
    for (k, b) in BUILDINGS.iter().enumerate().skip(3) {
        assert!(b.research == FREE || (b.research as usize) < TECH_COUNT, "{}", b.key);
        assert!(b.recipes.len() <= 32 && b.recipes.iter().all(|r| r.inputs.len() <= 3), "{}", b.key);
        if b.class == Class::Drill {
            assert!(!b.deposits.is_empty(), "{}", b.key);
        }
        if b.category == crate::content::cat::HIDDEN {
            assert_eq!(k as u8, bk::WRECK, "{} is hidden", b.key);
        }
    }
    let mut alts = [0; ALT_COUNT];
    for b in &BUILDINGS {
        for r in b.recipes {
            match r.unlock {
                FREE => {}
                u if u >= ALT_BASE => {
                    alts[(u - ALT_BASE) as usize] += 1;
                    assert!(!r.name.is_empty());
                }
                u => assert!((u as usize) < TECH_COUNT),
            }
        }
    }
    assert!(alts.iter().all(|&n| n == 1), "{alts:?}");
    let stages = crate::content::STAGES.len();
    assert!(TECH.iter().all(|t| (t.stage as usize) < stages));
    for o in &OBJECTIVES {
        match o.goal {
            Goal::ReachStage(s) => assert!((s as usize) < stages),
            Goal::Build(k, _) => assert!((k as usize) < BUILDINGS.len()),
            Goal::Deliver(item, _) => assert!((item as usize) < ITEM_COUNT),
            Goal::Research(t) => assert!((t as usize) < TECH_COUNT),
            _ => {}
        }
    }
    let json = crate::content::content_json();
    let depth = json.chars().fold(0i32, |d, c| match c {
        '{' | '[' => d + 1,
        '}' | ']' => d - 1,
        _ => d,
    });
    assert_eq!(depth, 0);
    assert!(
        json.contains("\"thermal\"") && json.contains("\"Living Planet\"") && json.contains("\"Launch\"")
    );
}

#[test]
#[ignore = "benchmark: cargo test --release -- --ignored --nocapture"]
fn bench_tick_50_loops() {
    let mut w = World::new(512, 512, 1);
    w.build_benchmark(50);
    run(&mut w, 120);
    let (belts, items, machines) = (w.belts.list.len(), w.belts.item_count, w.machines.list.len());
    let start = std::time::Instant::now();
    let ticks = 600;
    run(&mut w, ticks);
    let dt = start.elapsed().as_secs_f64();
    let t0 = std::time::Instant::now();
    let n = w.render(0.0, 0.0, 120.0, 70.0, 0.5, lod::ALL | lod::STATUS);
    let render_us = t0.elapsed().as_secs_f64() * 1e6;
    println!(
        "{belts} belts, {items} items, {machines} machines, {} segments, {} nets: {:.1} us/tick, render {n} instances in {render_us:.0} us",
        w.belts.segs.len(),
        w.machines.nets.len(),
        dt * 1e6 / ticks as f64
    );
    assert!(machines > 3000);
    assert!(w.machines.incinerated > 0, "production lines should be flowing");
    let unpowered = w.machines.list.iter().filter(|m| m.status == status::NO_POWER).count();
    assert_eq!(unpowered, 0, "every benchmark machine is powered");
}
