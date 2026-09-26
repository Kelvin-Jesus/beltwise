//! Game content as data: items, buildings and their recipes, research, the Ark megaproject,
//! terraforming stages, objectives, achievements, lore, the shop and planet types. The
//! simulation reads these tables directly; the UI receives the same tables as JSON
//! (`content_json`), so names, costs and numbers have one source of truth.

use core::fmt::Write;

pub type Item = u8;

/// Item ids. Raw resources share their id with the deposit (`Grid::res`) they come from.
pub mod it {
    pub const NONE: u8 = 0;
    pub const IRON_ORE: u8 = 1;
    pub const COPPER_ORE: u8 = 2;
    pub const STONE: u8 = 3;
    pub const COAL: u8 = 4;
    pub const ICE: u8 = 5;
    pub const TITANIUM_ORE: u8 = 6;
    pub const URANIUM_ORE: u8 = 7;
    pub const CRUDE_OIL: u8 = 8;
    pub const METEORITE: u8 = 9;
    pub const IRON_INGOT: u8 = 10;
    pub const COPPER_INGOT: u8 = 11;
    pub const SAND: u8 = 12;
    pub const GLASS: u8 = 13;
    pub const IRON_PLATE: u8 = 14;
    pub const COPPER_WIRE: u8 = 15;
    pub const GEAR: u8 = 16;
    pub const STEEL: u8 = 17;
    pub const CIRCUIT: u8 = 18;
    pub const MOTOR: u8 = 19;
    pub const WATER: u8 = 20;
    pub const TITANIUM_INGOT: u8 = 21;
    pub const TITANIUM_PLATE: u8 = 22;
    pub const ALGAE: u8 = 23;
    pub const FERTILIZER: u8 = 24;
    pub const FUEL_ROD: u8 = 25;
    pub const FRAME: u8 = 26;
    pub const SILICON: u8 = 27;
    pub const PLASTIC: u8 = 28;
    pub const FUEL: u8 = 29;
    pub const BATTERY: u8 = 30;
    pub const COMPUTER: u8 = 31;
    pub const REINFORCED_PLATE: u8 = 32;
    pub const ALIEN_ALLOY: u8 = 33;
    pub const QUANTUM_CORE: u8 = 34;
    pub const SEEDS: u8 = 35;
    pub const POWER_SHARD: u8 = 36;
    pub const AMPLIFIER: u8 = 37;
}
pub const ITEM_COUNT: usize = 38;
/// Deposits are items 1..=9.
pub const RESOURCE_COUNT: u8 = 9;

pub struct ItemDef {
    pub key: &'static str,
    pub name: &'static str,
    /// Credits a Recycler pays for one.
    pub value: u32,
}

const fn item(key: &'static str, name: &'static str, value: u32) -> ItemDef {
    ItemDef { key, name, value }
}

pub const ITEMS: [ItemDef; ITEM_COUNT] = [
    item("none", "Nothing", 0),
    item("iron_ore", "Iron Ore", 1),
    item("copper_ore", "Copper Ore", 1),
    item("stone", "Stone", 1),
    item("coal", "Coal", 1),
    item("ice", "Ice", 1),
    item("titanium_ore", "Titanium Ore", 4),
    item("uranium_ore", "Uranium Ore", 6),
    item("crude_oil", "Crude Oil", 2),
    item("meteorite", "Meteorite", 10),
    item("iron_ingot", "Iron Ingot", 2),
    item("copper_ingot", "Copper Ingot", 2),
    item("sand", "Sand", 1),
    item("glass", "Glass", 3),
    item("iron_plate", "Iron Plate", 3),
    item("copper_wire", "Copper Wire", 1),
    item("gear", "Gear", 7),
    item("steel", "Steel", 6),
    item("circuit", "Circuit", 15),
    item("motor", "Motor", 35),
    item("water", "Water", 2),
    item("titanium_ingot", "Titanium Ingot", 8),
    item("titanium_plate", "Titanium Plate", 10),
    item("algae", "Algae", 4),
    item("fertilizer", "Fertilizer", 12),
    item("fuel_rod", "Fuel Rod", 120),
    item("frame", "Frame", 60),
    item("silicon", "Silicon", 10),
    item("plastic", "Plastic", 6),
    item("fuel", "Fuel", 8),
    item("battery", "Battery", 30),
    item("computer", "Computer", 120),
    item("reinforced_plate", "Reinforced Plate", 25),
    item("alien_alloy", "Alien Alloy", 150),
    item("quantum_core", "Quantum Core", 2000),
    item("seeds", "Seeds", 40),
    item("power_shard", "Power Shard", 500),
    item("amplifier", "Amplifier", 5000),
];

/// Building kinds (`Grid::kind`). Ids 0..=20 are unchanged from save version 1.
pub mod bk {
    pub const EMPTY: u8 = 0;
    pub const BELT: u8 = 1;
    pub const CORE: u8 = 2;
    pub const DRILL: u8 = 3;
    pub const SMELTER: u8 = 4;
    pub const CRUSHER: u8 = 5;
    pub const PRESS: u8 = 6;
    pub const ASSEMBLER: u8 = 7;
    pub const FOUNDRY: u8 = 8;
    pub const ELECTRONICS: u8 = 9;
    pub const MELTER: u8 = 10;
    pub const BIOLAB: u8 = 11;
    pub const ENRICHER: u8 = 12;
    pub const HEATER: u8 = 13;
    pub const VAPORIZER: u8 = 14;
    pub const OXYGENATOR: u8 = 15;
    pub const GREENHOUSE: u8 = 16;
    pub const THERMAL: u8 = 17;
    pub const SPLITTER: u8 = 18;
    pub const TUNNEL: u8 = 19;
    pub const INCINERATOR: u8 = 20;
    pub const POLE: u8 = 21;
    pub const COAL_GEN: u8 = 22;
    pub const SOLAR: u8 = 23;
    pub const FUEL_GEN: u8 = 24;
    pub const REACTOR: u8 = 25;
    pub const BATTERY_BANK: u8 = 26;
    pub const SORTER: u8 = 27;
    pub const STORAGE: u8 = 28;
    pub const DRONE_PORT: u8 = 29;
    pub const RADAR: u8 = 30;
    pub const RECYCLER: u8 = 31;
    pub const OIL_PUMP: u8 = 32;
    pub const WATER_PUMP: u8 = 33;
    pub const REFINERY: u8 = 34;
    pub const HIVE: u8 = 35;
    pub const HATCHERY: u8 = 36;
    pub const WRECK: u8 = 37;
}
pub const BUILDING_COUNT: usize = 38;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Class {
    None,
    Belt,
    Core,
    /// Mines the deposit under it.
    Drill,
    /// Consumes a recipe's inputs and emits its output through the front face.
    Crafter,
    /// Consumes a recipe's inputs and raises a terraforming meter.
    Terraformer,
    Splitter,
    Tunnel,
    Incinerator,
    /// Extends power coverage and links networks.
    Pole,
    /// Burns fuel (or catches sunlight) for power.
    Generator,
    /// Stores surplus power and releases it when generators fall short.
    Battery,
    /// Sends one item type straight on and everything else to the sides.
    Sorter,
    /// Buffers up to `STORAGE_CAP` items of one type.
    Storage,
    /// Flies items to a linked port.
    DronePort,
    /// Reveals a wide area over time.
    Radar,
    /// Turns items into credits.
    Recycler,
    /// Pumps water from a lake (needs liquid water under it).
    Pump,
    /// Crash site left by the first expedition; salvaged, never built.
    Wreck,
}

impl Class {
    const fn key(self) -> &'static str {
        match self {
            Class::None => "none",
            Class::Belt => "belt",
            Class::Core => "core",
            Class::Drill => "drill",
            Class::Crafter => "crafter",
            Class::Terraformer => "terraformer",
            Class::Splitter => "splitter",
            Class::Tunnel => "tunnel",
            Class::Incinerator => "incinerator",
            Class::Pole => "pole",
            Class::Generator => "generator",
            Class::Battery => "battery",
            Class::Sorter => "sorter",
            Class::Storage => "storage",
            Class::DronePort => "drone",
            Class::Radar => "radar",
            Class::Recycler => "recycler",
            Class::Pump => "pump",
            Class::Wreck => "wreck",
        }
    }
}

/// `Recipe::unlock` and `BuildingDef::research` value for things available from the start.
pub const FREE: u8 = 255;
/// `Recipe::unlock` values from here on are alternate recipes (`ALT_BASE + index`), unlocked
/// by analysing data probes found in wrecks.
pub const ALT_BASE: u8 = 128;

pub struct Recipe {
    pub inputs: &'static [(Item, u8)],
    /// `(NONE, 0)` for terraformers.
    pub output: (Item, u8),
    pub ticks: u16,
    /// `FREE`, a tech id, or `ALT_BASE + alternate index`.
    pub unlock: u8,
    /// Alternates have names; standard recipes are named after their output.
    pub name: &'static str,
}

const fn r(inputs: &'static [(Item, u8)], output: (Item, u8), ticks: u16) -> Recipe {
    Recipe { inputs, output, ticks, unlock: FREE, name: "" }
}
const fn rt(inputs: &'static [(Item, u8)], output: (Item, u8), ticks: u16, tech: u8) -> Recipe {
    Recipe { inputs, output, ticks, unlock: tech, name: "" }
}
const fn alt(
    name: &'static str,
    inputs: &'static [(Item, u8)],
    output: (Item, u8),
    ticks: u16,
    k: u8,
) -> Recipe {
    Recipe { inputs, output, ticks, unlock: ALT_BASE + k, name }
}

/// Build-menu categories.
pub mod cat {
    pub const LOGISTICS: u8 = 0;
    pub const EXTRACTION: u8 = 1;
    pub const MANUFACTURING: u8 = 2;
    pub const POWER: u8 = 3;
    pub const TERRAFORMING: u8 = 4;
    pub const HIDDEN: u8 = 255;
}

/// Terraforming meters.
pub mod meter {
    pub const HEAT: u8 = 0;
    pub const PRESSURE: u8 = 1;
    pub const OXYGEN: u8 = 2;
    pub const BIOMASS: u8 = 3;
    pub const COUNT: usize = 4;
    pub const NONE: u8 = 255;
}

pub struct BuildingDef {
    pub key: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
    pub class: Class,
    pub category: u8,
    pub cost: &'static [(Item, u16)],
    pub recipes: &'static [Recipe],
    pub meter: u8,
    /// Terraformers: meter points per completed cycle.
    pub points: u32,
    /// Tech node that unlocks it, or `FREE`.
    pub research: u8,
    /// Consumers: power drawn while working (kW). Generators: output (kW). Batteries: the
    /// fastest they charge or discharge (kW).
    pub power: u32,
    /// Generators: the fuel they burn and the energy one item holds (kJ).
    pub fuel: (Item, u32),
    /// Batteries: capacity (kJ).
    pub store: u32,
    /// Drills: the deposits they can mine.
    pub deposits: &'static [Item],
}

use it::*;

const NO_RECIPES: &[Recipe] = &[];
const B: BuildingDef = BuildingDef {
    key: "",
    name: "",
    desc: "",
    class: Class::None,
    category: cat::HIDDEN,
    cost: &[],
    recipes: NO_RECIPES,
    meter: meter::NONE,
    points: 0,
    research: FREE,
    power: 0,
    fuel: (NONE, 0),
    store: 0,
    deposits: &[],
};

const MW: u32 = 1000;
const SOLID_DEPOSITS: &[Item] =
    &[IRON_ORE, COPPER_ORE, STONE, COAL, ICE, TITANIUM_ORE, URANIUM_ORE, METEORITE];

pub const BUILDINGS: [BuildingDef; BUILDING_COUNT] = [
    BuildingDef { key: "empty", name: "Empty", ..B },
    BuildingDef {
        key: "belt",
        name: "Belt",
        desc: "Moves items. Drag to lay a line; corners are automatic. Free.",
        class: Class::Belt,
        category: cat::LOGISTICS,
        ..B
    },
    BuildingDef {
        key: "core",
        name: "Core",
        desc: "Stores everything delivered and powers nearby buildings. Materials here pay for buildings, research and the Ark.",
        class: Class::Core,
        ..B
    },
    BuildingDef {
        key: "drill",
        name: "Drill",
        desc: "Mines the deposit under it and outputs forward. Pure deposits mine twice as fast.",
        class: Class::Drill,
        category: cat::EXTRACTION,
        cost: &[(IRON_INGOT, 5), (COPPER_INGOT, 2)],
        power: 3 * MW,
        deposits: SOLID_DEPOSITS,
        ..B
    },
    BuildingDef {
        key: "smelter",
        name: "Smelter",
        desc: "Smelts ore into ingots and sand into glass.",
        class: Class::Crafter,
        category: cat::EXTRACTION,
        cost: &[(STONE, 10), (IRON_INGOT, 4)],
        recipes: &[
            r(&[(IRON_ORE, 1)], (IRON_INGOT, 1), 60),
            r(&[(COPPER_ORE, 1)], (COPPER_INGOT, 1), 60),
            r(&[(SAND, 1)], (GLASS, 1), 60),
            rt(&[(TITANIUM_ORE, 1)], (TITANIUM_INGOT, 1), 90, tech::TITANIUM),
            alt("Pure Iron", &[(IRON_ORE, 1), (WATER, 1)], (IRON_INGOT, 2), 90, 0),
            alt("Pure Copper", &[(COPPER_ORE, 1), (WATER, 1)], (COPPER_INGOT, 2), 90, 1),
        ],
        power: 4 * MW,
        ..B
    },
    BuildingDef {
        key: "crusher",
        name: "Crusher",
        desc: "Crushes stone into sand.",
        class: Class::Crafter,
        category: cat::EXTRACTION,
        cost: &[(IRON_INGOT, 8), (STONE, 5)],
        recipes: &[r(&[(STONE, 1)], (SAND, 2), 60)],
        research: tech::CRUSHING,
        power: 4 * MW,
        ..B
    },
    BuildingDef {
        key: "press",
        name: "Press",
        desc: "Presses ingots into plates, and copper into wire.",
        class: Class::Crafter,
        category: cat::MANUFACTURING,
        cost: &[(IRON_INGOT, 10), (COPPER_INGOT, 5)],
        recipes: &[
            r(&[(IRON_INGOT, 1)], (IRON_PLATE, 1), 45),
            r(&[(COPPER_INGOT, 1)], (COPPER_WIRE, 2), 45),
            rt(&[(TITANIUM_INGOT, 1)], (TITANIUM_PLATE, 1), 60, tech::TITANIUM),
            alt("Fine Wire", &[(COPPER_INGOT, 1)], (COPPER_WIRE, 3), 60, 2),
            alt("Steel Plates", &[(STEEL, 1)], (IRON_PLATE, 3), 60, 3),
        ],
        research: tech::PRESSING,
        power: 5 * MW,
        ..B
    },
    BuildingDef {
        key: "assembler",
        name: "Assembler",
        desc: "Builds gears, motors, plates, batteries and frames from parts.",
        class: Class::Crafter,
        category: cat::MANUFACTURING,
        cost: &[(IRON_PLATE, 10), (COPPER_WIRE, 6)],
        recipes: &[
            r(&[(IRON_PLATE, 2)], (GEAR, 1), 90),
            r(&[(GEAR, 1), (COPPER_WIRE, 2)], (MOTOR, 1), 150),
            rt(&[(TITANIUM_PLATE, 2), (STEEL, 1)], (FRAME, 1), 180, tech::TITANIUM),
            rt(&[(STEEL, 2), (IRON_PLATE, 2)], (REINFORCED_PLATE, 1), 150, tech::REINFORCED),
            rt(&[(COPPER_INGOT, 2), (PLASTIC, 1)], (BATTERY, 1), 120, tech::BATTERIES),
            alt("Steel Gear", &[(STEEL, 1)], (GEAR, 2), 90, 4),
            alt("Quick Motor", &[(GEAR, 2), (CIRCUIT, 1)], (MOTOR, 2), 180, 5),
            alt("Bolted Frame", &[(REINFORCED_PLATE, 1), (TITANIUM_PLATE, 2)], (FRAME, 2), 240, 6),
            alt("Dense Battery", &[(PLASTIC, 2), (SILICON, 1)], (BATTERY, 2), 150, 11),
        ],
        research: tech::ASSEMBLY,
        power: 10 * MW,
        ..B
    },
    BuildingDef {
        key: "foundry",
        name: "Foundry",
        desc: "Alloys iron with coal into steel, sand into silicon, and meteorite into alien alloy.",
        class: Class::Crafter,
        category: cat::MANUFACTURING,
        cost: &[(STONE, 20), (IRON_PLATE, 10)],
        recipes: &[
            r(&[(IRON_INGOT, 1), (COAL, 1)], (STEEL, 1), 120),
            rt(&[(SAND, 2), (COAL, 1)], (SILICON, 1), 120, tech::SILICA),
            rt(&[(METEORITE, 1), (TITANIUM_INGOT, 1)], (ALIEN_ALLOY, 1), 240, tech::XENO),
            alt("Coke Steel", &[(IRON_ORE, 1), (COAL, 2)], (STEEL, 1), 120, 7),
        ],
        research: tech::STEELMAKING,
        power: 12 * MW,
        ..B
    },
    BuildingDef {
        key: "electronics",
        name: "Electronics Lab",
        desc: "Etches circuits, and builds computers from circuits, plastic and silicon.",
        class: Class::Crafter,
        category: cat::MANUFACTURING,
        cost: &[(STEEL, 10), (COPPER_WIRE, 10), (GLASS, 5)],
        recipes: &[
            r(&[(COPPER_WIRE, 2), (IRON_PLATE, 1)], (CIRCUIT, 1), 120),
            rt(&[(CIRCUIT, 2), (PLASTIC, 2), (SILICON, 1)], (COMPUTER, 1), 300, tech::COMPUTING),
            alt("Silicon Circuit", &[(SILICON, 1), (COPPER_WIRE, 2)], (CIRCUIT, 2), 150, 8),
        ],
        research: tech::ELECTRONICS,
        power: 20 * MW,
        ..B
    },
    BuildingDef {
        key: "melter",
        name: "Melter",
        desc: "Melts ice into water.",
        class: Class::Crafter,
        category: cat::MANUFACTURING,
        cost: &[(IRON_PLATE, 10), (GLASS, 5)],
        recipes: &[r(&[(ICE, 1)], (WATER, 1), 60)],
        research: tech::MELTING,
        power: 6 * MW,
        ..B
    },
    BuildingDef {
        key: "biolab",
        name: "Bio Lab",
        desc: "Grows algae, brews fertilizer and germinates seeds.",
        class: Class::Crafter,
        category: cat::MANUFACTURING,
        cost: &[(STEEL, 10), (GLASS, 8), (CIRCUIT, 5)],
        recipes: &[
            r(&[(WATER, 1), (SAND, 1)], (ALGAE, 1), 120),
            r(&[(ALGAE, 2), (COAL, 1)], (FERTILIZER, 1), 120),
            rt(&[(ALGAE, 1), (FERTILIZER, 1)], (SEEDS, 1), 180, tech::POLLINATORS),
            alt("Quick Algae", &[(WATER, 2)], (ALGAE, 1), 120, 9),
        ],
        research: tech::BIOLOGY,
        power: 15 * MW,
        ..B
    },
    BuildingDef {
        key: "enricher",
        name: "Enricher",
        desc: "Packs uranium into fuel rods, and fuses alien alloy into quantum cores.",
        class: Class::Crafter,
        category: cat::MANUFACTURING,
        cost: &[(TITANIUM_PLATE, 10), (CIRCUIT, 10), (MOTOR, 5)],
        recipes: &[
            r(&[(URANIUM_ORE, 2), (STEEL, 1)], (FUEL_ROD, 1), 240),
            rt(&[(ALIEN_ALLOY, 1), (COMPUTER, 1)], (QUANTUM_CORE, 1), 600, tech::QUANTUM),
        ],
        research: tech::NUCLEAR,
        power: 40 * MW,
        ..B
    },
    BuildingDef {
        key: "heater",
        name: "Heater",
        desc: "Burns coal to warm the planet. Raises Heat. Needs no power.",
        class: Class::Terraformer,
        category: cat::TERRAFORMING,
        cost: &[(IRON_PLATE, 8), (COPPER_WIRE, 4)],
        recipes: &[r(&[(COAL, 1)], (NONE, 0), 240)],
        meter: meter::HEAT,
        points: 20,
        research: tech::HEATING,
        ..B
    },
    BuildingDef {
        key: "vaporizer",
        name: "Vaporizer",
        desc: "Boils ice into the sky. Raises Pressure.",
        class: Class::Terraformer,
        category: cat::TERRAFORMING,
        cost: &[(STEEL, 10), (CIRCUIT, 5)],
        recipes: &[r(&[(ICE, 1)], (NONE, 0), 120)],
        meter: meter::PRESSURE,
        points: 30,
        research: tech::VAPOR,
        power: 20 * MW,
        ..B
    },
    BuildingDef {
        key: "oxygenator",
        name: "Oxygenator",
        desc: "Releases oxygen from algae. Raises Oxygen.",
        class: Class::Terraformer,
        category: cat::TERRAFORMING,
        cost: &[(GLASS, 8), (CIRCUIT, 6), (MOTOR, 4)],
        recipes: &[r(&[(ALGAE, 1)], (NONE, 0), 120)],
        meter: meter::OXYGEN,
        points: 100,
        research: tech::OXYGEN,
        power: 25 * MW,
        ..B
    },
    BuildingDef {
        key: "greenhouse",
        name: "Greenhouse",
        desc: "Grows plants from water and fertilizer. Raises Biomass.",
        class: Class::Terraformer,
        category: cat::TERRAFORMING,
        cost: &[(GLASS, 12), (STEEL, 6), (CIRCUIT, 4)],
        recipes: &[r(&[(WATER, 1), (FERTILIZER, 1)], (NONE, 0), 180)],
        meter: meter::BIOMASS,
        points: 450,
        research: tech::GREENHOUSES,
        power: 30 * MW,
        ..B
    },
    BuildingDef {
        key: "thermal",
        name: "Thermal Core",
        desc: "A small star in a box. Burns fuel rods for enormous Heat. Needs no power.",
        class: Class::Terraformer,
        category: cat::TERRAFORMING,
        cost: &[(TITANIUM_PLATE, 10), (FRAME, 6), (CIRCUIT, 8)],
        recipes: &[r(&[(FUEL_ROD, 1)], (NONE, 0), 600)],
        meter: meter::HEAT,
        points: 5000,
        research: tech::THERMAL,
        ..B
    },
    BuildingDef {
        key: "splitter",
        name: "Splitter",
        desc: "Takes items from behind and deals them to its front, left and right in turn.",
        class: Class::Splitter,
        category: cat::LOGISTICS,
        cost: &[(IRON_PLATE, 3)],
        research: tech::SPLITTERS,
        ..B
    },
    BuildingDef {
        key: "tunnel",
        name: "Tunnel",
        desc: "Place an entrance, then an exit up to 5 tiles ahead. Items travel underneath.",
        class: Class::Tunnel,
        category: cat::LOGISTICS,
        cost: &[(IRON_PLATE, 4), (COPPER_WIRE, 2)],
        research: tech::TUNNELS,
        ..B
    },
    BuildingDef {
        key: "incinerator",
        name: "Incinerator",
        desc: "Destroys anything fed into it.",
        class: Class::Incinerator,
        category: cat::LOGISTICS,
        cost: &[(IRON_INGOT, 5), (STONE, 5)],
        research: tech::INCINERATION,
        ..B
    },
    BuildingDef {
        key: "pole",
        name: "Power Pole",
        desc: "Powers buildings within 5 tiles and links to poles within 10. Reveals the land around it.",
        class: Class::Pole,
        category: cat::POWER,
        cost: &[(IRON_PLATE, 2), (COPPER_WIRE, 3)],
        research: tech::POWER,
        ..B
    },
    BuildingDef {
        key: "coal_gen",
        name: "Coal Generator",
        desc: "Burns coal for 60 MW. Uses fuel only as fast as the network draws power.",
        class: Class::Generator,
        category: cat::POWER,
        cost: &[(IRON_PLATE, 20), (COPPER_WIRE, 15), (STONE, 20)],
        research: tech::POWER,
        power: 60 * MW,
        fuel: (COAL, 300_000),
        ..B
    },
    BuildingDef {
        key: "solar",
        name: "Solar Panel",
        desc: "8 MW in full sun, nothing at night. Pair with batteries.",
        class: Class::Generator,
        category: cat::POWER,
        cost: &[(SILICON, 10), (GLASS, 10), (STEEL, 5)],
        research: tech::SOLAR,
        power: 8 * MW,
        ..B
    },
    BuildingDef {
        key: "fuel_gen",
        name: "Fuel Generator",
        desc: "Burns fuel for 150 MW.",
        class: Class::Generator,
        category: cat::POWER,
        cost: &[(REINFORCED_PLATE, 20), (MOTOR, 10), (CIRCUIT, 20)],
        research: tech::FUEL_POWER,
        power: 150 * MW,
        fuel: (FUEL, 900_000),
        ..B
    },
    BuildingDef {
        key: "reactor",
        name: "Reactor",
        desc: "Splits fuel rods for 1.2 GW.",
        class: Class::Generator,
        category: cat::POWER,
        cost: &[(FRAME, 20), (COMPUTER, 20), (REINFORCED_PLATE, 50)],
        research: tech::REACTOR,
        power: 1200 * MW,
        fuel: (FUEL_ROD, 60_000_000),
        ..B
    },
    BuildingDef {
        key: "battery_bank",
        name: "Battery Bank",
        desc: "Stores 3 GJ of surplus power and releases it (up to 150 MW) when generators fall short.",
        class: Class::Battery,
        category: cat::POWER,
        cost: &[(BATTERY, 10), (STEEL, 20)],
        research: tech::BATTERIES,
        power: 150 * MW,
        store: 3_000_000,
        ..B
    },
    BuildingDef {
        key: "sorter",
        name: "Sorter",
        desc: "Sends its chosen item straight on and everything else to the sides. Pick the item in the inspector.",
        class: Class::Sorter,
        category: cat::LOGISTICS,
        cost: &[(IRON_PLATE, 5), (CIRCUIT, 2)],
        research: tech::SORTING,
        ..B
    },
    BuildingDef {
        key: "storage",
        name: "Storage",
        desc: "Buffers up to 500 items of one kind and releases them forward.",
        class: Class::Storage,
        category: cat::LOGISTICS,
        cost: &[(IRON_PLATE, 20), (STEEL, 10)],
        research: tech::STORAGE,
        ..B
    },
    BuildingDef {
        key: "drone_port",
        name: "Drone Port",
        desc: "Link it to another port and its drone flies everything fed in across the map.",
        class: Class::DronePort,
        category: cat::LOGISTICS,
        cost: &[(COMPUTER, 5), (BATTERY, 10), (MOTOR, 10)],
        research: tech::DRONES,
        power: 10 * MW,
        ..B
    },
    BuildingDef {
        key: "radar",
        name: "Radar",
        desc: "Slowly reveals the land up to 70 tiles away, wrecks included.",
        class: Class::Radar,
        category: cat::LOGISTICS,
        cost: &[(COMPUTER, 10), (REINFORCED_PLATE, 20), (MOTOR, 10)],
        research: tech::RADAR,
        power: 20 * MW,
        ..B
    },
    BuildingDef {
        key: "recycler",
        name: "Recycler",
        desc: "Breaks down anything fed into it for credits to spend in the shop.",
        class: Class::Recycler,
        category: cat::LOGISTICS,
        cost: &[(REINFORCED_PLATE, 10), (MOTOR, 10), (CIRCUIT, 10)],
        research: tech::RECYCLING,
        power: 20 * MW,
        ..B
    },
    BuildingDef {
        key: "oil_pump",
        name: "Oil Pump",
        desc: "Pumps crude oil from an oil seep.",
        class: Class::Drill,
        category: cat::EXTRACTION,
        cost: &[(REINFORCED_PLATE, 10), (MOTOR, 10), (STEEL, 20)],
        research: tech::OIL,
        power: 15 * MW,
        deposits: &[CRUDE_OIL],
        ..B
    },
    BuildingDef {
        key: "water_pump",
        name: "Water Pump",
        desc: "Pumps water from a lake. Build it on liquid water.",
        class: Class::Pump,
        category: cat::EXTRACTION,
        cost: &[(REINFORCED_PLATE, 5), (MOTOR, 5), (STEEL, 10)],
        research: tech::PUMPING,
        power: 7 * MW,
        ..B
    },
    BuildingDef {
        key: "refinery",
        name: "Refinery",
        desc: "Refines crude oil into plastic or fuel.",
        class: Class::Crafter,
        category: cat::MANUFACTURING,
        cost: &[(REINFORCED_PLATE, 15), (MOTOR, 10), (CIRCUIT, 10)],
        recipes: &[
            r(&[(CRUDE_OIL, 1)], (PLASTIC, 1), 90),
            r(&[(CRUDE_OIL, 2)], (FUEL, 1), 120),
            alt("Heavy Oil Fuel", &[(CRUDE_OIL, 2), (COAL, 1)], (FUEL, 2), 150, 10),
        ],
        research: tech::OIL,
        power: 25 * MW,
        ..B
    },
    BuildingDef {
        key: "hive",
        name: "Pollinator Hive",
        desc: "Releases pollinators that spread plant life. Raises Biomass enormously.",
        class: Class::Terraformer,
        category: cat::TERRAFORMING,
        cost: &[(REINFORCED_PLATE, 10), (GLASS, 20), (COMPUTER, 2)],
        recipes: &[r(&[(SEEDS, 1), (WATER, 1)], (NONE, 0), 240)],
        meter: meter::BIOMASS,
        points: 3000,
        research: tech::POLLINATORS,
        power: 20 * MW,
        ..B
    },
    BuildingDef {
        key: "hatchery",
        name: "Hatchery",
        desc: "Stocks the lakes with fish. Must touch liquid water. Raises Biomass and Oxygen.",
        class: Class::Terraformer,
        category: cat::TERRAFORMING,
        cost: &[(FRAME, 5), (GLASS, 20), (COMPUTER, 4)],
        recipes: &[r(&[(ALGAE, 2), (FERTILIZER, 1)], (NONE, 0), 300)],
        meter: meter::BIOMASS,
        points: 8000,
        research: tech::HATCHERY,
        power: 30 * MW,
        ..B
    },
    BuildingDef {
        key: "wreck",
        name: "Wreck",
        desc: "A crashed pod from the first expedition. Tap it to salvage what's left.",
        class: Class::Wreck,
        ..B
    },
];

/// Items a Storage holds.
pub const STORAGE_CAP: u32 = 500;
/// Power poles: coverage (Chebyshev radius) and link distance.
pub const POLE_RADIUS: i32 = 5;
pub const POLE_LINK: i32 = 10;
/// The Core's own supply and coverage radius around its center.
pub const CORE_POWER: u32 = 40 * MW;
pub const CORE_RADIUS: i32 = 20;

// ---- Research -------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Effect {
    Unlock(u8),
    /// Allows drills on this deposit.
    Ore(Item),
    BeltTier(u8),
    DrillTier(u8),
    CraftTier(u8),
    HeaterBoost,
    /// Repeatable bonuses, percent per level.
    DrillBonus(u8),
    FactoryBonus(u8),
    TerraBonus(u8),
    PowerBonus(u8),
}

pub struct TechDef {
    pub key: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
    pub cost: &'static [(Item, u16)],
    pub requires: &'static [u8],
    /// Minimum terraforming stage.
    pub stage: u8,
    /// Tier (1..=5): tier n needs n - 1 completed Ark phases.
    pub tier: u8,
    pub effects: &'static [Effect],
    /// Can be researched again and again; each level costs 1.5x the last.
    pub repeat: bool,
}

pub mod tech {
    pub const PRESSING: u8 = 0;
    pub const CRUSHING: u8 = 1;
    pub const SPLITTERS: u8 = 2;
    pub const TUNNELS: u8 = 3;
    pub const HEATING: u8 = 4;
    pub const ASSEMBLY: u8 = 5;
    pub const STEELMAKING: u8 = 6;
    pub const ELECTRONICS: u8 = 7;
    pub const BELTS2: u8 = 8;
    pub const DRILLS2: u8 = 9;
    pub const MELTING: u8 = 10;
    pub const VAPOR: u8 = 11;
    pub const HEATERS2: u8 = 12;
    pub const INCINERATION: u8 = 13;
    pub const BIOLOGY: u8 = 14;
    pub const OXYGEN: u8 = 15;
    pub const TITANIUM: u8 = 16;
    pub const GREENHOUSES: u8 = 17;
    pub const CRAFT2: u8 = 18;
    pub const BELTS3: u8 = 19;
    pub const DRILLS3: u8 = 20;
    pub const NUCLEAR: u8 = 21;
    pub const THERMAL: u8 = 22;
    pub const CRAFT3: u8 = 23;
    pub const POWER: u8 = 24;
    pub const STORAGE: u8 = 25;
    pub const SORTING: u8 = 26;
    pub const SILICA: u8 = 27;
    pub const REINFORCED: u8 = 28;
    pub const SOLAR: u8 = 29;
    pub const PUMPING: u8 = 30;
    pub const OIL: u8 = 31;
    pub const FUEL_POWER: u8 = 32;
    pub const COMPUTING: u8 = 33;
    pub const BATTERIES: u8 = 34;
    pub const RADAR: u8 = 35;
    pub const DRONES: u8 = 36;
    pub const RECYCLING: u8 = 37;
    pub const POLLINATORS: u8 = 38;
    pub const HATCHERY: u8 = 39;
    pub const REACTOR: u8 = 40;
    pub const XENO: u8 = 41;
    pub const QUANTUM: u8 = 42;
    pub const R_MINING: u8 = 43;
    pub const R_FACTORY: u8 = 44;
    pub const R_TERRA: u8 = 45;
    pub const R_POWER: u8 = 46;
}
pub const TECH_COUNT: usize = 47;
/// Repeatable nodes are the last ones; their levels are stored in this order.
pub const REPEAT_FIRST: u8 = tech::R_MINING;
pub const REPEAT_COUNT: usize = 4;

use tech::*;

const T: TechDef = TechDef {
    key: "",
    name: "",
    desc: "",
    cost: &[],
    requires: &[],
    stage: 0,
    tier: 1,
    effects: &[],
    repeat: false,
};

pub const TECH: [TechDef; TECH_COUNT] = [
    TechDef {
        key: "pressing",
        name: "Pressing",
        desc: "Unlocks the Press: iron plates and copper wire.",
        cost: &[(IRON_INGOT, 20)],
        effects: &[Effect::Unlock(bk::PRESS)],
        ..T
    },
    TechDef {
        key: "crushing",
        name: "Crushing",
        desc: "Unlocks the Crusher: stone into sand, sand into glass.",
        cost: &[(STONE, 30), (IRON_INGOT, 10)],
        effects: &[Effect::Unlock(bk::CRUSHER)],
        ..T
    },
    TechDef {
        key: "splitters",
        name: "Splitters",
        desc: "Unlocks the Splitter to divide a belt into several.",
        cost: &[(IRON_PLATE, 10)],
        requires: &[PRESSING],
        effects: &[Effect::Unlock(bk::SPLITTER)],
        ..T
    },
    TechDef {
        key: "tunnels",
        name: "Tunnels",
        desc: "Unlocks Tunnels so belts can cross.",
        cost: &[(IRON_PLATE, 15), (COPPER_WIRE, 10)],
        requires: &[SPLITTERS],
        effects: &[Effect::Unlock(bk::TUNNEL)],
        ..T
    },
    TechDef {
        key: "heating",
        name: "Heating",
        desc: "Unlocks the Heater, your first terraformer.",
        cost: &[(IRON_PLATE, 15), (COPPER_WIRE, 10)],
        requires: &[PRESSING],
        effects: &[Effect::Unlock(bk::HEATER)],
        ..T
    },
    TechDef {
        key: "assembly",
        name: "Assembly",
        desc: "Unlocks the Assembler: gears and motors.",
        cost: &[(IRON_PLATE, 30), (COPPER_WIRE, 20)],
        requires: &[PRESSING],
        effects: &[Effect::Unlock(bk::ASSEMBLER)],
        ..T
    },
    TechDef {
        key: "steel",
        name: "Steelmaking",
        desc: "Unlocks the Foundry: iron and coal into steel.",
        cost: &[(GEAR, 20), (IRON_INGOT, 30)],
        requires: &[ASSEMBLY],
        effects: &[Effect::Unlock(bk::FOUNDRY)],
        ..T
    },
    TechDef {
        key: "electronics",
        name: "Electronics",
        desc: "Unlocks the Electronics Lab: circuits.",
        cost: &[(STEEL, 20), (COPPER_WIRE, 40), (GLASS, 10)],
        requires: &[STEELMAKING, CRUSHING],
        effects: &[Effect::Unlock(bk::ELECTRONICS)],
        ..T
    },
    TechDef {
        key: "belts2",
        name: "Fast Belts",
        desc: "All belts move a third faster: 300 items/min.",
        cost: &[(GEAR, 40), (STEEL, 20)],
        requires: &[STEELMAKING],
        effects: &[Effect::BeltTier(2)],
        ..T
    },
    TechDef {
        key: "drills2",
        name: "Better Drills",
        desc: "All drills mine 50% faster.",
        cost: &[(STEEL, 30), (CIRCUIT, 20)],
        requires: &[ELECTRONICS],
        tier: 2,
        effects: &[Effect::DrillTier(2)],
        ..T
    },
    TechDef {
        key: "melting",
        name: "Ice Melting",
        desc: "Unlocks the Melter: ice into water.",
        cost: &[(STEEL, 20), (GLASS, 15)],
        requires: &[STEELMAKING],
        stage: stage::WARMING,
        effects: &[Effect::Unlock(bk::MELTER)],
        ..T
    },
    TechDef {
        key: "vapor",
        name: "Vaporization",
        desc: "Unlocks the Vaporizer, which thickens the atmosphere.",
        cost: &[(CIRCUIT, 20), (STEEL, 20)],
        requires: &[ELECTRONICS],
        stage: stage::WARMING,
        tier: 2,
        effects: &[Effect::Unlock(bk::VAPORIZER)],
        ..T
    },
    TechDef {
        key: "heaters2",
        name: "Efficient Heaters",
        desc: "Heaters produce twice the heat from the same coal.",
        cost: &[(CIRCUIT, 30), (GEAR, 40)],
        requires: &[HEATING, ELECTRONICS],
        effects: &[Effect::HeaterBoost],
        ..T
    },
    TechDef {
        key: "incineration",
        name: "Incineration",
        desc: "Unlocks the Incinerator to get rid of surplus items.",
        cost: &[(IRON_PLATE, 10), (STONE, 20)],
        requires: &[PRESSING],
        effects: &[Effect::Unlock(bk::INCINERATOR)],
        ..T
    },
    TechDef {
        key: "biology",
        name: "Biology",
        desc: "Unlocks the Bio Lab: algae and fertilizer.",
        cost: &[(CIRCUIT, 30), (GLASS, 30), (WATER, 20)],
        requires: &[MELTING],
        stage: stage::THIN_AIR,
        tier: 2,
        effects: &[Effect::Unlock(bk::BIOLAB)],
        ..T
    },
    TechDef {
        key: "oxygen",
        name: "Oxygenation",
        desc: "Unlocks the Oxygenator.",
        cost: &[(ALGAE, 20), (MOTOR, 20)],
        requires: &[BIOLOGY],
        stage: stage::THIN_AIR,
        tier: 2,
        effects: &[Effect::Unlock(bk::OXYGENATOR)],
        ..T
    },
    TechDef {
        key: "titanium",
        name: "Titanium",
        desc: "Drills can mine titanium. Smelt and press it into plates, then assemble frames.",
        cost: &[(COMPUTER, 20), (STEEL, 100)],
        requires: &[ELECTRONICS],
        tier: 4,
        effects: &[Effect::Ore(TITANIUM_ORE)],
        ..T
    },
    TechDef {
        key: "greenhouses",
        name: "Greenhouses",
        desc: "Unlocks the Greenhouse, the biggest step towards life.",
        cost: &[(FERTILIZER, 30), (MOTOR, 20), (GLASS, 30)],
        requires: &[OXYGEN],
        stage: stage::WATER,
        tier: 3,
        effects: &[Effect::Unlock(bk::GREENHOUSE)],
        ..T
    },
    TechDef {
        key: "craft2",
        name: "Fast Assembly",
        desc: "All factories work 30% faster.",
        cost: &[(CIRCUIT, 60), (MOTOR, 30)],
        requires: &[ELECTRONICS, ASSEMBLY],
        tier: 2,
        effects: &[Effect::CraftTier(2)],
        ..T
    },
    TechDef {
        key: "belts3",
        name: "Express Belts",
        desc: "All belts move twice as fast as the original: 450 items/min.",
        cost: &[(MOTOR, 50), (TITANIUM_PLATE, 40)],
        requires: &[BELTS2, TITANIUM],
        tier: 4,
        effects: &[Effect::BeltTier(3)],
        ..T
    },
    TechDef {
        key: "drills3",
        name: "Deep Drills",
        desc: "All drills mine twice as fast as the original.",
        cost: &[(MOTOR, 50), (COMPUTER, 30), (TITANIUM_PLATE, 30)],
        requires: &[DRILLS2, TITANIUM],
        tier: 4,
        effects: &[Effect::DrillTier(3)],
        ..T
    },
    TechDef {
        key: "nuclear",
        name: "Nuclear",
        desc: "Drills can mine uranium. Unlocks the Enricher for fuel rods.",
        cost: &[(TITANIUM_PLATE, 80), (COMPUTER, 40), (MOTOR, 40)],
        requires: &[TITANIUM],
        tier: 5,
        effects: &[Effect::Ore(URANIUM_ORE), Effect::Unlock(bk::ENRICHER)],
        ..T
    },
    TechDef {
        key: "thermal",
        name: "Thermal Core",
        desc: "Unlocks the Thermal Core, a fuel-rod furnace for massive Heat.",
        cost: &[(FUEL_ROD, 30), (FRAME, 30)],
        requires: &[NUCLEAR],
        stage: stage::GRASSLAND,
        tier: 5,
        effects: &[Effect::Unlock(bk::THERMAL)],
        ..T
    },
    TechDef {
        key: "craft3",
        name: "Robotic Assembly",
        desc: "All factories work 70% faster than the original.",
        cost: &[(FRAME, 60), (COMPUTER, 40)],
        requires: &[CRAFT2, TITANIUM],
        tier: 4,
        effects: &[Effect::CraftTier(3)],
        ..T
    },
    TechDef {
        key: "power",
        name: "Electricity",
        desc: "Power Poles carry power beyond the Core's field; Coal Generators add more.",
        cost: &[(IRON_PLATE, 20), (COPPER_WIRE, 20)],
        requires: &[PRESSING],
        effects: &[Effect::Unlock(bk::POLE), Effect::Unlock(bk::COAL_GEN)],
        ..T
    },
    TechDef {
        key: "storage",
        name: "Storage",
        desc: "Unlocks Storage to buffer production.",
        cost: &[(IRON_PLATE, 30), (STEEL, 10)],
        requires: &[STEELMAKING],
        effects: &[Effect::Unlock(bk::STORAGE)],
        ..T
    },
    TechDef {
        key: "sorting",
        name: "Sorting",
        desc: "Unlocks the Sorter to pull one item out of a mixed belt.",
        cost: &[(CIRCUIT, 20), (IRON_PLATE, 40)],
        requires: &[SPLITTERS, ELECTRONICS],
        effects: &[Effect::Unlock(bk::SORTER)],
        ..T
    },
    TechDef {
        key: "silicon",
        name: "Silicon",
        desc: "Foundries can bake sand and coal into silicon.",
        cost: &[(STEEL, 50), (CIRCUIT, 30)],
        requires: &[ELECTRONICS],
        tier: 2,
        ..T
    },
    TechDef {
        key: "reinforced",
        name: "Reinforced Plates",
        desc: "Assemblers can bolt steel and iron into reinforced plates.",
        cost: &[(STEEL, 60), (GEAR, 60)],
        requires: &[STEELMAKING],
        tier: 2,
        ..T
    },
    TechDef {
        key: "solar",
        name: "Solar Power",
        desc: "Unlocks the Solar Panel: free power while the sun is up.",
        cost: &[(SILICON, 40), (GLASS, 40), (STEEL, 20)],
        requires: &[SILICA, POWER],
        tier: 2,
        effects: &[Effect::Unlock(bk::SOLAR)],
        ..T
    },
    TechDef {
        key: "pumping",
        name: "Pumping",
        desc: "Unlocks the Water Pump for the new lakes.",
        cost: &[(REINFORCED_PLATE, 20), (MOTOR, 20)],
        requires: &[REINFORCED, MELTING],
        stage: stage::WATER,
        tier: 2,
        effects: &[Effect::Unlock(bk::WATER_PUMP)],
        ..T
    },
    TechDef {
        key: "oil",
        name: "Oil Processing",
        desc: "Oil Pumps tap oil seeps; Refineries turn crude into plastic and fuel.",
        cost: &[(REINFORCED_PLATE, 50), (MOTOR, 40), (CIRCUIT, 60)],
        requires: &[REINFORCED],
        tier: 3,
        effects: &[Effect::Unlock(bk::OIL_PUMP), Effect::Unlock(bk::REFINERY)],
        ..T
    },
    TechDef {
        key: "fuel_power",
        name: "Fuel Power",
        desc: "Unlocks the Fuel Generator: 150 MW.",
        cost: &[(PLASTIC, 50), (REINFORCED_PLATE, 40)],
        requires: &[OIL, POWER],
        tier: 3,
        effects: &[Effect::Unlock(bk::FUEL_GEN)],
        ..T
    },
    TechDef {
        key: "computing",
        name: "Computing",
        desc: "Electronics Labs can build computers.",
        cost: &[(PLASTIC, 60), (SILICON, 60), (CIRCUIT, 80)],
        requires: &[OIL, SILICA],
        tier: 3,
        ..T
    },
    TechDef {
        key: "batteries",
        name: "Batteries",
        desc: "Assemblers can make batteries; unlocks the Battery Bank.",
        cost: &[(PLASTIC, 60), (COPPER_INGOT, 100)],
        requires: &[OIL],
        tier: 3,
        effects: &[Effect::Unlock(bk::BATTERY_BANK)],
        ..T
    },
    TechDef {
        key: "radar",
        name: "Radar",
        desc: "Unlocks the Radar to reveal the land far away.",
        cost: &[(COMPUTER, 10), (REINFORCED_PLATE, 30)],
        requires: &[COMPUTING],
        tier: 3,
        effects: &[Effect::Unlock(bk::RADAR)],
        ..T
    },
    TechDef {
        key: "drones",
        name: "Drones",
        desc: "Unlocks Drone Ports: link two and items fly between them.",
        cost: &[(COMPUTER, 20), (BATTERY, 30), (MOTOR, 40)],
        requires: &[COMPUTING, BATTERIES],
        tier: 3,
        effects: &[Effect::Unlock(bk::DRONE_PORT)],
        ..T
    },
    TechDef {
        key: "recycling",
        name: "Recycling",
        desc: "Unlocks the Recycler: surplus into credits for the shop.",
        cost: &[(CIRCUIT, 50), (REINFORCED_PLATE, 30)],
        requires: &[REINFORCED],
        tier: 3,
        effects: &[Effect::Unlock(bk::RECYCLER)],
        ..T
    },
    TechDef {
        key: "seeds",
        name: "Pollinators",
        desc: "Bio Labs can germinate seeds; unlocks the Pollinator Hive.",
        cost: &[(FERTILIZER, 60), (COMPUTER, 20), (FRAME, 10)],
        requires: &[GREENHOUSES, TITANIUM],
        stage: stage::MOSS,
        tier: 4,
        effects: &[Effect::Unlock(bk::HIVE)],
        ..T
    },
    TechDef {
        key: "hatchery",
        name: "Aquaculture",
        desc: "Unlocks the Hatchery to bring the lakes to life.",
        cost: &[(SEEDS, 60), (FRAME, 30), (COMPUTER, 30)],
        requires: &[POLLINATORS],
        stage: stage::GRASSLAND,
        tier: 4,
        effects: &[Effect::Unlock(bk::HATCHERY)],
        ..T
    },
    TechDef {
        key: "reactor",
        name: "Fission Power",
        desc: "Unlocks the Reactor: 1.2 GW from fuel rods.",
        cost: &[(FUEL_ROD, 20), (FRAME, 40)],
        requires: &[NUCLEAR, FUEL_POWER],
        tier: 5,
        effects: &[Effect::Unlock(bk::REACTOR)],
        ..T
    },
    TechDef {
        key: "xeno",
        name: "Xenometallurgy",
        desc: "Drills can mine meteorites; Foundries alloy them with titanium.",
        cost: &[(COMPUTER, 50), (TITANIUM_PLATE, 100)],
        requires: &[TITANIUM],
        tier: 5,
        effects: &[Effect::Ore(METEORITE)],
        ..T
    },
    TechDef {
        key: "quantum",
        name: "Quantum Cores",
        desc: "Enrichers can fuse alien alloy and computers into quantum cores.",
        cost: &[(ALIEN_ALLOY, 30), (COMPUTER, 50)],
        requires: &[XENO, NUCLEAR],
        tier: 5,
        ..T
    },
    TechDef {
        key: "r_mining",
        name: "Deep Core Mining",
        desc: "Drills and pumps work 10% faster. Repeatable.",
        cost: &[(COMPUTER, 20), (STEEL, 100)],
        requires: &[DRILLS2],
        tier: 4,
        effects: &[Effect::DrillBonus(10)],
        repeat: true,
        ..T
    },
    TechDef {
        key: "r_factory",
        name: "Overdrive",
        desc: "Factories work 10% faster. Repeatable.",
        cost: &[(COMPUTER, 20), (MOTOR, 60)],
        requires: &[CRAFT2],
        tier: 4,
        effects: &[Effect::FactoryBonus(10)],
        repeat: true,
        ..T
    },
    TechDef {
        key: "r_terra",
        name: "Terraforming Efficiency",
        desc: "Terraformers add 15% more. Repeatable.",
        cost: &[(FERTILIZER, 50), (COMPUTER, 10)],
        requires: &[GREENHOUSES],
        tier: 4,
        effects: &[Effect::TerraBonus(15)],
        repeat: true,
        ..T
    },
    TechDef {
        key: "r_power",
        name: "Grid Optimization",
        desc: "Generators produce 10% more. Repeatable.",
        cost: &[(BATTERY, 30), (COMPUTER, 10)],
        requires: &[BATTERIES],
        tier: 4,
        effects: &[Effect::PowerBonus(10)],
        repeat: true,
        ..T
    },
];

/// Cost of the next level of a repeatable node: 1.5x per level.
pub fn repeat_cost(n: u16, level: u16) -> u32 {
    let mut c = n as u64;
    for _ in 0..level.min(40) {
        c = c * 3 / 2;
    }
    c.min(u32::MAX as u64) as u32
}

// ---- The Ark ------------------------------------------------------------------------------------

pub struct ArkPhase {
    pub name: &'static str,
    pub desc: &'static str,
    pub cost: &'static [(Item, u32)],
}

/// The megaproject. Completing phase n unlocks tier n + 1; the fifth launches the colony
/// to a new planet.
pub const ARK: [ArkPhase; 5] = [
    ArkPhase {
        name: "Foundation",
        desc: "A launch pad and scaffold around the Core. Unlocks tier 2: silicon, solar power and reinforced plates.",
        cost: &[(STEEL, 150), (CIRCUIT, 80), (MOTOR, 40)],
    },
    ArkPhase {
        name: "Hull",
        desc: "The Ark's hull takes shape. Unlocks tier 3: oil, computers, batteries, drones and radar.",
        cost: &[(REINFORCED_PLATE, 150), (GLASS, 400), (SILICON, 100)],
    },
    ArkPhase {
        name: "Systems",
        desc: "Navigation and life support. Unlocks tier 4: titanium, frames, pollinators and express belts.",
        cost: &[(COMPUTER, 80), (BATTERY, 120), (PLASTIC, 300)],
    },
    ArkPhase {
        name: "Habitat",
        desc: "Greenhouse decks for the voyage. Unlocks tier 5: nuclear power, meteorites and quantum cores.",
        cost: &[(FRAME, 120), (SEEDS, 150), (TITANIUM_PLATE, 300)],
    },
    ArkPhase {
        name: "Launch",
        desc: "Fuel it, wake the quantum drive, and send the colony to a new world.",
        cost: &[(QUANTUM_CORE, 20), (ALIEN_ALLOY, 150), (FUEL_ROD, 60), (FUEL, 600)],
    },
];

// ---- Terraforming -----------------------------------------------------------------------------

pub struct StageDef {
    pub name: &'static str,
    pub desc: &'static str,
    /// Terraform Index needed to reach this stage.
    pub ti: u64,
    /// Paid once when the stage is reached.
    pub credits: u32,
    pub shards: u8,
}

/// Stage indices.
pub mod stage {
    pub const BARREN: u8 = 0;
    pub const WARMING: u8 = 1;
    pub const THIN_AIR: u8 = 2;
    pub const CLOUDS: u8 = 3;
    pub const WATER: u8 = 4;
    pub const LICHEN: u8 = 5;
    pub const MOSS: u8 = 6;
    pub const GRASSLAND: u8 = 7;
    pub const BLOOMING: u8 = 8;
    pub const FOREST: u8 = 9;
    pub const WILDLIFE: u8 = 10;
    pub const LIVING: u8 = 11;
}

const fn st(name: &'static str, desc: &'static str, ti: u64, credits: u32, shards: u8) -> StageDef {
    StageDef { name, desc, ti, credits, shards }
}

pub const STAGES: [StageDef; 12] = [
    st("Barren", "A frozen rock under a thin, dusty sky.", 0, 0, 0),
    st("Warming", "The ice caps begin to retreat.", 2_500, 100, 0),
    st("Thin Air", "A real atmosphere starts to hold. Snow drifts down.", 25_000, 200, 0),
    st("Clouds", "Clouds gather and drift over the land.", 60_000, 300, 0),
    st("Liquid Water", "Meltwater pools in the lowlands, and the first rain falls.", 125_000, 500, 1),
    st("Lichen", "Hardy lichen crusts the bare rock.", 250_000, 700, 0),
    st("Moss", "The first green spreads across damp ground.", 500_000, 1_000, 1),
    st("Grassland", "Plains turn green, and the first birds arrive.", 2_000_000, 2_000, 1),
    st("Blooming", "Flowers open, and butterflies drift between them.", 4_000_000, 3_000, 1),
    st("Forest", "Forests take root.", 7_500_000, 5_000, 1),
    st("Wildlife", "Flocks fill the sky and fish crowd the lakes.", 14_000_000, 8_000, 2),
    st("Living Planet", "A breathing world. What will you build next?", 25_000_000, 20_000, 3),
];

pub const METER_NAMES: [&str; meter::COUNT] = ["Heat", "Pressure", "Oxygen", "Biomass"];
/// Meter value at which its visual effect is complete (log-scaled).
pub const METER_FULL: [u64; meter::COUNT] = [8_000_000, 4_000_000, 4_000_000, 10_000_000];

// ---- Objectives --------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Goal {
    Build(u8, u16),
    Deliver(Item, u32),
    Research(u8),
    ReachTi(u64),
    ReachStage(u8),
    /// Complete this many Ark phases.
    Ark(u8),
    /// Salvage this many wrecks.
    Salvage(u8),
}

pub struct ObjectiveDef {
    pub text: &'static str,
    pub goal: Goal,
    pub reward: &'static [(Item, u16)],
}

const fn o(text: &'static str, goal: Goal, reward: &'static [(Item, u16)]) -> ObjectiveDef {
    ObjectiveDef { text, goal, reward }
}

pub const OBJECTIVES: [ObjectiveDef; 53] = [
    o("Build a Drill on an iron deposit", Goal::Build(bk::DRILL, 1), &[(IRON_INGOT, 10)]),
    o("Build a Smelter and belt the ore into it", Goal::Build(bk::SMELTER, 1), &[(STONE, 10)]),
    o(
        "Deliver iron ingots to the Core",
        Goal::Deliver(IRON_INGOT, 20),
        &[(IRON_INGOT, 15), (COPPER_INGOT, 10)],
    ),
    o("Research Pressing", Goal::Research(PRESSING), &[(IRON_INGOT, 20)]),
    o("Deliver iron plates", Goal::Deliver(IRON_PLATE, 30), &[(COPPER_INGOT, 20)]),
    o("Deliver copper wire", Goal::Deliver(COPPER_WIRE, 30), &[(IRON_PLATE, 20)]),
    o("Research Electricity", Goal::Research(POWER), &[(IRON_PLATE, 10), (COPPER_WIRE, 10)]),
    o("Build a Coal Generator and feed it coal", Goal::Build(bk::COAL_GEN, 1), &[(IRON_PLATE, 20)]),
    o("Research Heating", Goal::Research(HEATING), &[(IRON_PLATE, 10)]),
    o("Feed a Heater with coal: reach 250 Ti", Goal::ReachTi(250), &[(COPPER_WIRE, 20)]),
    o("Research Assembly", Goal::Research(ASSEMBLY), &[(IRON_PLATE, 30)]),
    o("Deliver gears", Goal::Deliver(GEAR, 40), &[(IRON_INGOT, 50)]),
    o("Research Steelmaking", Goal::Research(STEELMAKING), &[(STONE, 50)]),
    o("Deliver steel", Goal::Deliver(STEEL, 40), &[(GEAR, 20)]),
    o("Reach the Warming stage", Goal::ReachStage(stage::WARMING), &[(STEEL, 20)]),
    o("Research Electronics", Goal::Research(ELECTRONICS), &[(COPPER_WIRE, 50)]),
    o("Deliver circuits", Goal::Deliver(CIRCUIT, 50), &[(STEEL, 30)]),
    o("Deliver motors", Goal::Deliver(MOTOR, 30), &[(CIRCUIT, 20)]),
    o("Complete the Ark's Foundation", Goal::Ark(1), &[(POWER_SHARD, 1), (STEEL, 50)]),
    o("Research Reinforced Plates", Goal::Research(REINFORCED), &[(STEEL, 40)]),
    o("Deliver reinforced plates", Goal::Deliver(REINFORCED_PLATE, 40), &[(MOTOR, 10)]),
    o("Research Vaporization", Goal::Research(VAPOR), &[(CIRCUIT, 20)]),
    o("Reach the Thin Air stage", Goal::ReachStage(stage::THIN_AIR), &[(CIRCUIT, 30), (STEEL, 30)]),
    o("Research Silicon", Goal::Research(SILICON), &[(SAND, 60)]),
    o("Build 4 Solar Panels", Goal::Build(bk::SOLAR, 4), &[(SILICON, 20)]),
    o("Research Biology", Goal::Research(BIOLOGY), &[(GLASS, 30)]),
    o("Deliver algae", Goal::Deliver(ALGAE, 40), &[(MOTOR, 10)]),
    o("Research Oxygenation", Goal::Research(OXYGEN), &[(CIRCUIT, 40)]),
    o("Reach the Liquid Water stage", Goal::ReachStage(stage::WATER), &[(STEEL, 60)]),
    o("Complete the Ark's Hull", Goal::Ark(2), &[(POWER_SHARD, 1), (REINFORCED_PLATE, 40)]),
    o("Research Oil Processing", Goal::Research(OIL), &[(REINFORCED_PLATE, 20)]),
    o("Deliver plastic", Goal::Deliver(PLASTIC, 60), &[(CIRCUIT, 40)]),
    o("Research Computing", Goal::Research(COMPUTING), &[(SILICON, 30)]),
    o("Deliver computers", Goal::Deliver(COMPUTER, 20), &[(PLASTIC, 40)]),
    o("Research Radar and build one", Goal::Build(bk::RADAR, 1), &[(REINFORCED_PLATE, 20)]),
    o("Salvage a wreck", Goal::Salvage(1), &[(COMPUTER, 5)]),
    o("Research Greenhouses", Goal::Research(GREENHOUSES), &[(FERTILIZER, 20)]),
    o("Reach the Moss stage", Goal::ReachStage(stage::MOSS), &[(COMPUTER, 10)]),
    o("Complete the Ark's Systems", Goal::Ark(3), &[(AMPLIFIER, 1), (COMPUTER, 20)]),
    o("Research Titanium", Goal::Research(TITANIUM), &[(MOTOR, 20)]),
    o("Deliver titanium plates", Goal::Deliver(TITANIUM_PLATE, 60), &[(COMPUTER, 10)]),
    o("Deliver frames", Goal::Deliver(FRAME, 30), &[(TITANIUM_PLATE, 40)]),
    o("Research Pollinators and build a Hive", Goal::Build(bk::HIVE, 1), &[(SEEDS, 20)]),
    o("Reach the Grassland stage", Goal::ReachStage(stage::GRASSLAND), &[(FRAME, 20)]),
    o("Complete the Ark's Habitat", Goal::Ark(4), &[(POWER_SHARD, 2), (FRAME, 20)]),
    o("Research Nuclear", Goal::Research(NUCLEAR), &[(STEEL, 100)]),
    o("Deliver fuel rods", Goal::Deliver(FUEL_ROD, 20), &[(FRAME, 10)]),
    o("Build a Reactor", Goal::Build(bk::REACTOR, 1), &[(FUEL_ROD, 10)]),
    o("Research Xenometallurgy", Goal::Research(XENO), &[(COMPUTER, 20)]),
    o("Deliver alien alloy", Goal::Deliver(ALIEN_ALLOY, 20), &[(FUEL_ROD, 10)]),
    o("Reach the Forest stage", Goal::ReachStage(stage::FOREST), &[(FRAME, 40)]),
    o("Deliver quantum cores", Goal::Deliver(QUANTUM_CORE, 5), &[(ALIEN_ALLOY, 20)]),
    o("Launch the Ark", Goal::Ark(5), &[]),
];

/// Starting Core stock: enough for a first mining and smelting line.
pub const START_STOCK: &[(Item, u16)] = &[(IRON_INGOT, 40), (COPPER_INGOT, 20), (STONE, 30)];

/// Belt speed (sub-units per tick) per belt tier (index = tier - 1): 225, 300 and 450
/// items/min. Each divides `MIN_GAP` so compressed belts run at exactly this rate.
pub const BELT_SPEEDS: [u32; 3] = [3, 4, 6];
/// Drill period (ticks) at normal purity and tier 1.
pub const DRILL_TICKS: u16 = 60;
/// Drill speed (percent) per drill tier.
pub const DRILL_PCT: [u32; 3] = [100, 150, 200];
/// Factory speed (percent) per craft tier.
pub const CRAFT_PCT: [u32; 3] = [100, 130, 170];
/// Water pump period (ticks).
pub const PUMP_TICKS: u16 = 30;
/// Deposit purity (`Grid::purity`): impure, normal, pure. Speed in percent.
pub const PURITY_PCT: [u32; 3] = [50, 100, 200];
pub const PURITY_NAMES: [&str; 3] = ["Impure", "Normal", "Pure"];
/// Overclock: speed (percent) per power shards installed (0..=3), and the power it costs.
pub const CLOCK_PCT: [u32; 4] = [100, 150, 200, 250];
pub const CLOCK_POWER_PCT: [u32; 4] = [100, 169, 246, 329];
/// An amplifier doubles every output at four times the power.
pub const AMP_POWER_MULT: u32 = 4;

// ---- Achievements --------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cond {
    Delivered(u64),
    Machines(u32),
    Belts(u32),
    Rate(u32),
    Stage(u8),
    Researched(u32),
    AllResearch,
    Built(u8, u16),
    PowerMw(u32),
    Wrecks(u32),
    Logs(u32),
    Ark(u8),
    Credits(u64),
    Stored(u64),
    Clock250,
    Amplified,
    Blueprint,
    DroneLink,
    Launches(u32),
    BatteryMj(u32),
}

pub struct AchDef {
    pub key: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
    pub cond: Cond,
    pub credits: u32,
    pub shards: u8,
}

const fn a(
    key: &'static str,
    name: &'static str,
    desc: &'static str,
    cond: Cond,
    credits: u32,
    shards: u8,
) -> AchDef {
    AchDef { key, name, desc, cond, credits, shards }
}

pub const ACHIEVEMENTS: [AchDef; 33] = [
    a("first", "First Delivery", "Deliver an item to the Core.", Cond::Delivered(1), 50, 0),
    a("line", "Assembly Line", "Run 10 machines.", Cond::Machines(10), 100, 0),
    a("industry", "Industrialist", "Run 100 machines.", Cond::Machines(100), 500, 0),
    a("mega", "Megafactory", "Run 1,000 machines.", Cond::Machines(1000), 5000, 1),
    a("belts", "Belt Enthusiast", "Lay 500 belts.", Cond::Belts(500), 200, 0),
    a("spaghetti", "Spaghetti Chef", "Lay 5,000 belts.", Cond::Belts(5000), 2000, 0),
    a("rate1", "Full Throttle", "Deliver 450 items per minute.", Cond::Rate(450), 500, 0),
    a("rate2", "Torrent", "Deliver 2,000 items per minute.", Cond::Rate(2000), 3000, 1),
    a("million", "Millionaire", "Deliver a million items in total.", Cond::Delivered(1_000_000), 10000, 1),
    a("warm", "Warm Welcome", "Reach the Warming stage.", Cond::Stage(stage::WARMING), 200, 0),
    a("rain", "Rainmaker", "Reach the Liquid Water stage.", Cond::Stage(stage::WATER), 1000, 0),
    a("green", "Green Thumb", "Reach the Grassland stage.", Cond::Stage(stage::GRASSLAND), 3000, 1),
    a("bloom", "Butterfly Effect", "Reach the Blooming stage.", Cond::Stage(stage::BLOOMING), 4000, 0),
    a("gaia", "Gaia", "Bring the planet to life.", Cond::Stage(stage::LIVING), 20000, 2),
    a("scholar", "Scholar", "Research 10 technologies.", Cond::Researched(10), 300, 0),
    a("polymath", "Polymath", "Research every technology at least once.", Cond::AllResearch, 10000, 1),
    a("power", "Power Up", "Build a Coal Generator.", Cond::Built(bk::COAL_GEN, 1), 100, 0),
    a("gigawatt", "Gigawatt", "Produce 1 GW of power at once.", Cond::PowerMw(1000), 5000, 1),
    a("sun", "Sun Worshipper", "Build 20 Solar Panels.", Cond::Built(bk::SOLAR, 20), 1000, 0),
    a("battery", "Charged Up", "Hold 10 GJ in batteries.", Cond::BatteryMj(10_000), 1500, 0),
    a("explorer", "Explorer", "Salvage a wreck.", Cond::Wrecks(1), 200, 0),
    a("archaeologist", "Archaeologist", "Salvage 10 wrecks.", Cond::Wrecks(10), 2000, 1),
    a("story", "Storyteller", "Recover every expedition log.", Cond::Logs(10), 3000, 1),
    a("airmail", "Airmail", "Link two Drone Ports.", Cond::DroneLink, 500, 0),
    a("overclock", "Redline", "Overclock a machine to 250%.", Cond::Clock250, 500, 0),
    a("amplified", "Amplified", "Install an Amplifier.", Cond::Amplified, 500, 0),
    a("blueprint", "Architect", "Paste a blueprint.", Cond::Blueprint, 300, 0),
    a("ark1", "Groundbreaking", "Complete the Ark's Foundation.", Cond::Ark(1), 500, 0),
    a("ark3", "Systems Go", "Complete the Ark's Systems.", Cond::Ark(3), 3000, 0),
    a("launch", "Liftoff", "Launch the Ark.", Cond::Ark(5), 20000, 3),
    a("credits", "Recycler", "Earn 10,000 credits.", Cond::Credits(10_000), 1000, 0),
    a("hoard", "Hoarder", "Keep 100,000 items in the Core.", Cond::Stored(100_000), 2000, 0),
    a("voyager", "Voyager", "Settle a second planet.", Cond::Launches(1), 5000, 1),
];

// ---- Wrecks, lore and alternate recipes ---------------------------------------------------------------

pub struct LogDef {
    pub title: &'static str,
    pub text: &'static str,
}

pub const LOGS: [LogDef; 10] = [
    LogDef {
        title: "Landing log, day 1",
        text: "Meridian Pod 4 came down hard in the northern ice. Hull breach, but the Core survived. We drill, we smelt, and we wait for the other pods to call in.",
    },
    LogDef {
        title: "Survey note",
        text: "Rich iron to the west, copper glinting under the frost. Someone scratched a message on this crate: \"Belts before breakfast.\" Morale is holding.",
    },
    LogDef {
        title: "Engineer's confession",
        text: "The splitter jammed again. I rebuilt it three times before I noticed the belt was running backwards. Do not tell the captain.",
    },
    LogDef {
        title: "Weather report",
        text: "Wind at ninety knots, visibility zero. But the heaters work: this morning the ice at the crater rim was wet. Wet! On this rock.",
    },
    LogDef {
        title: "Medical log",
        text: "The cold gets into everything: machines, food, people. We burn more coal to keep warm than to make steel. It cannot go on.",
    },
    LogDef {
        title: "Seed vault inventory",
        text: "Moss, lichen, forty-two grasses, nine conifers, three flowering shrubs and one very stubborn apple tree. Keep the vault sealed until there is air to breathe.",
    },
    LogDef {
        title: "Captain's log, day 211",
        text: "Pods 2 and 6 have gone quiet. We are too few to finish the work. The Core still holds the Ark design, so whoever lands next can finish it.",
    },
    LogDef {
        title: "Reactor incident",
        text: "Thermal Core 1 is gone, and its crater is already filling with meltwater. The planet took a step forward that day, whether we liked it or not.",
    },
    LogDef {
        title: "Unsent letter",
        text: "When the grass comes, and it will, lie in it for me. Watch the clouds. Then get back to work: the belts don't run themselves.",
    },
    LogDef {
        title: "Final transmission",
        text: "To whoever lands next: the Ark will carry our seeds to the stars. Build it. Launch it. And don't stop at one world.",
    },
];

/// Number of alternate recipes (`Recipe::unlock == ALT_BASE + k`).
pub const ALT_COUNT: usize = 12;

/// What a wreck holds (`Machine::aux` of a wreck: kind << 8 | index).
pub mod wreck {
    pub const CACHE: u8 = 0;
    pub const PROBE: u8 = 1;
    pub const LOG: u8 = 2;
    pub const SHARDS: u8 = 3;
    pub const AMPLIFIER: u8 = 4;
}

// ---- Shop and cosmetics --------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Offer {
    Item(Item, u16),
    /// Cosmetic bit (see `cosmetic`).
    Cosmetic(u8),
}

pub struct ShopDef {
    pub key: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
    pub price: u32,
    pub offer: Offer,
}

/// Cosmetic bits: belt colours 0..=4 (0 is the default and always owned), Core trims 8..=10.
pub mod cosmetic {
    pub const BELT_FIRST: u8 = 0;
    pub const BELT_COUNT: u8 = 5;
    pub const TRIM_FIRST: u8 = 8;
    pub const TRIM_COUNT: u8 = 3;
}

pub const SHOP: [ShopDef; 9] = [
    ShopDef {
        key: "shard",
        name: "Power Shard",
        desc: "Install in a machine to overclock it by 50%.",
        price: 1500,
        offer: Offer::Item(POWER_SHARD, 1),
    },
    ShopDef {
        key: "amplifier",
        name: "Amplifier",
        desc: "Doubles a machine's output at four times the power.",
        price: 20000,
        offer: Offer::Item(AMPLIFIER, 1),
    },
    ShopDef {
        key: "belt_amber",
        name: "Amber belts",
        desc: "Warm amber belt stripes.",
        price: 2000,
        offer: Offer::Cosmetic(1),
    },
    ShopDef {
        key: "belt_teal",
        name: "Teal belts",
        desc: "Cool teal belt stripes.",
        price: 2000,
        offer: Offer::Cosmetic(2),
    },
    ShopDef {
        key: "belt_violet",
        name: "Violet belts",
        desc: "Electric violet belt stripes.",
        price: 2000,
        offer: Offer::Cosmetic(3),
    },
    ShopDef {
        key: "belt_crimson",
        name: "Crimson belts",
        desc: "Racing-red belt stripes.",
        price: 2000,
        offer: Offer::Cosmetic(4),
    },
    ShopDef {
        key: "trim_gold",
        name: "Gold trim",
        desc: "The classic Core finish.",
        price: 0,
        offer: Offer::Cosmetic(8),
    },
    ShopDef {
        key: "trim_chrome",
        name: "Chrome trim",
        desc: "A mirror-bright Core and Ark.",
        price: 5000,
        offer: Offer::Cosmetic(9),
    },
    ShopDef {
        key: "trim_aurora",
        name: "Aurora trim",
        desc: "A Core that shimmers like the northern lights.",
        price: 8000,
        offer: Offer::Cosmetic(10),
    },
];

// ---- Planets -------------------------------------------------------------------------------------

pub struct PlanetDef {
    pub key: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
    /// Shift applied to terrain elevation (negative: more basins, so more future lakes).
    pub elevation: f32,
    /// Relative weights for common deposits: iron, copper, stone, coal, ice, oil.
    pub weights: [u8; 6],
    /// Heat already present on arrival.
    pub start_heat: u64,
    /// Minutes between meteor showers.
    pub meteor_minutes: u32,
    /// Launches needed before this planet can be chosen.
    pub unlock: u32,
}

pub const PLANETS: [PlanetDef; 3] = [
    PlanetDef {
        key: "glacial",
        name: "Glacia",
        desc: "Frozen and quiet. Plenty of ice, and a good place to learn.",
        elevation: 0.0,
        weights: [5, 4, 3, 3, 4, 2],
        start_heat: 0,
        meteor_minutes: 12,
        unlock: 0,
    },
    PlanetDef {
        key: "volcanic",
        name: "Ember",
        desc: "Warm from the start, but ice is scarce and meteors fall twice as often.",
        elevation: 0.06,
        weights: [5, 4, 4, 5, 1, 3],
        start_heat: 3_000,
        meteor_minutes: 6,
        unlock: 1,
    },
    PlanetDef {
        key: "oceanic",
        name: "Thalassa",
        desc: "A frozen ocean with scattered islands. The seas return early; drones matter.",
        elevation: -0.16,
        weights: [4, 4, 3, 3, 6, 3],
        start_heat: 0,
        meteor_minutes: 12,
        unlock: 2,
    },
];

/// Production bonus (percent) per completed launch.
pub const LEGACY_PCT: u32 = 10;

// ---- JSON export for the UI ----------------------------------------------------------------------

fn pairs<N: core::fmt::Display + Copy>(s: &mut String, list: &[(Item, N)]) {
    s.push('[');
    for (i, (item, n)) in list.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        let _ = write!(s, "[{item},{n}]");
    }
    s.push(']');
}

fn list<T>(s: &mut String, items: &[T], mut f: impl FnMut(&mut String, usize, &T)) {
    s.push('[');
    for (i, x) in items.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        f(s, i, x);
    }
    s.push(']');
}

/// Every static table the UI needs, as one JSON document built once at start-up.
/// A string written as JSON string contents: quotes, backslashes and control characters
/// are escaped, so content text can contain anything.
pub struct J<'a>(pub &'a str);

impl core::fmt::Display for J<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for c in self.0.chars() {
            match c {
                '"' => f.write_str("\\\"")?,
                '\\' => f.write_str("\\\\")?,
                '\n' => f.write_str("\\n")?,
                c if (c as u32) < 0x20 => write!(f, "\\u{:04x}", c as u32)?,
                c => f.write_char(c)?,
            }
        }
        Ok(())
    }
}

pub fn content_json() -> String {
    let mut s = String::with_capacity(64 * 1024);
    s.push_str("{\"items\":");
    list(&mut s, &ITEMS, |s, _, d| {
        let _ = write!(s, "{{\"key\":\"{}\",\"name\":\"{}\",\"value\":{}}}", J(d.key), J(d.name), d.value);
    });
    s.push_str(",\"buildings\":");
    list(&mut s, &BUILDINGS, |s, _, d| {
        let _ = write!(
            s,
            "{{\"key\":\"{}\",\"name\":\"{}\",\"desc\":\"{}\",\"class\":\"{}\",\"category\":{},\"meter\":{},\"points\":{},\"research\":{},\"power\":{},\"fuel\":[{},{}],\"store\":{},\"cost\":",
            J(d.key),
            J(d.name),
            J(d.desc),
            d.class.key(),
            d.category,
            d.meter,
            d.points,
            d.research,
            d.power,
            d.fuel.0,
            d.fuel.1,
            d.store
        );
        pairs(s, d.cost);
        s.push_str(",\"deposits\":");
        list(s, d.deposits, |s, _, x| {
            let _ = write!(s, "{x}");
        });
        s.push_str(",\"recipes\":");
        list(s, d.recipes, |s, _, rc| {
            s.push_str("{\"inputs\":");
            let inputs: Vec<(Item, u8)> = rc.inputs.to_vec();
            pairs(s, &inputs);
            let _ = write!(
                s,
                ",\"output\":[{},{}],\"ticks\":{},\"unlock\":{},\"name\":\"{}\"}}",
                rc.output.0,
                rc.output.1,
                rc.ticks,
                rc.unlock,
                J(rc.name)
            );
        });
        s.push('}');
    });
    s.push_str(",\"tech\":");
    list(&mut s, &TECH, |s, t, d| {
        let _ = write!(
            s,
            "{{\"key\":\"{}\",\"name\":\"{}\",\"desc\":\"{}\",\"stage\":{},\"tier\":{},\"repeat\":{},\"cost\":",
            J(d.key),
            J(d.name),
            J(d.desc),
            d.stage,
            d.tier,
            d.repeat
        );
        pairs(s, d.cost);
        s.push_str(",\"requires\":");
        list(s, d.requires, |s, _, r| {
            let _ = write!(s, "{r}");
        });
        s.push_str(",\"unlocks\":[");
        let mut first = true;
        for e in d.effects {
            if let Effect::Unlock(kind) = e {
                if !first {
                    s.push(',');
                }
                first = false;
                let _ = write!(s, "{kind}");
            }
        }
        // Recipes this node unlocks, as [building, recipe index].
        s.push_str("],\"recipes\":[");
        let mut first = true;
        for (k, b) in BUILDINGS.iter().enumerate() {
            for (j, rc) in b.recipes.iter().enumerate() {
                if rc.unlock as usize == t {
                    if !first {
                        s.push(',');
                    }
                    first = false;
                    let _ = write!(s, "[{k},{j}]");
                }
            }
        }
        s.push_str("]}");
    });
    s.push_str(",\"ark\":");
    list(&mut s, &ARK, |s, _, d| {
        let _ = write!(s, "{{\"name\":\"{}\",\"desc\":\"{}\",\"cost\":", J(d.name), J(d.desc));
        pairs(s, d.cost);
        s.push('}');
    });
    s.push_str(",\"stages\":");
    list(&mut s, &STAGES, |s, _, d| {
        let _ = write!(
            s,
            "{{\"name\":\"{}\",\"desc\":\"{}\",\"ti\":{},\"credits\":{},\"shards\":{}}}",
            J(d.name),
            J(d.desc),
            d.ti,
            d.credits,
            d.shards
        );
    });
    s.push_str(",\"meters\":");
    list(&mut s, &METER_NAMES, |s, i, name| {
        let _ = write!(s, "{{\"name\":\"{}\",\"full\":{}}}", J(name), METER_FULL[i]);
    });
    s.push_str(",\"objectives\":");
    list(&mut s, &OBJECTIVES, |s, _, d| {
        let (kind, a, b) = goal_parts(d.goal);
        let _ = write!(
            s,
            "{{\"text\":\"{}\",\"kind\":\"{}\",\"a\":{},\"b\":{},\"reward\":",
            J(d.text),
            kind,
            a,
            b
        );
        pairs(s, d.reward);
        s.push('}');
    });
    s.push_str(",\"achievements\":");
    list(&mut s, &ACHIEVEMENTS, |s, _, d| {
        let _ = write!(
            s,
            "{{\"key\":\"{}\",\"name\":\"{}\",\"desc\":\"{}\",\"credits\":{},\"shards\":{}}}",
            J(d.key),
            J(d.name),
            J(d.desc),
            d.credits,
            d.shards
        );
    });
    s.push_str(",\"logs\":");
    list(&mut s, &LOGS, |s, _, d| {
        let _ = write!(s, "{{\"title\":\"{}\",\"text\":\"{}\"}}", J(d.title), J(d.text));
    });
    s.push_str(",\"alternates\":[");
    let mut first = true;
    for k in 0..ALT_COUNT {
        for (b, def) in BUILDINGS.iter().enumerate() {
            for (j, rc) in def.recipes.iter().enumerate() {
                if rc.unlock as usize == ALT_BASE as usize + k {
                    if !first {
                        s.push(',');
                    }
                    first = false;
                    let _ = write!(s, "[{b},{j}]");
                }
            }
        }
    }
    s.push_str("],\"shop\":");
    list(&mut s, &SHOP, |s, _, d| {
        let (kind, a, b) = match d.offer {
            Offer::Item(item, n) => ("item", item as u32, n as u32),
            Offer::Cosmetic(bit) => ("cosmetic", bit as u32, 1),
        };
        let _ = write!(
            s,
            "{{\"key\":\"{}\",\"name\":\"{}\",\"desc\":\"{}\",\"price\":{},\"kind\":\"{}\",\"a\":{},\"b\":{}}}",
            J(d.key),
            J(d.name),
            J(d.desc),
            d.price,
            kind,
            a,
            b
        );
    });
    s.push_str(",\"planets\":");
    list(&mut s, &PLANETS, |s, _, d| {
        let _ = write!(
            s,
            "{{\"key\":\"{}\",\"name\":\"{}\",\"desc\":\"{}\",\"unlock\":{}}}",
            J(d.key),
            J(d.name),
            J(d.desc),
            d.unlock
        );
    });
    let _ = write!(
        s,
        ",\"purity\":[\"{}\",\"{}\",\"{}\"],\"beltSpeeds\":[{},{},{}],\"storageCap\":{},\"poleRadius\":{},\"poleLink\":{},\"coreRadius\":{},\"corePower\":{},\"clockPct\":[{},{},{},{}],\"legacyPct\":{}}}",
        J(PURITY_NAMES[0]),
        J(PURITY_NAMES[1]),
        J(PURITY_NAMES[2]),
        BELT_SPEEDS[0],
        BELT_SPEEDS[1],
        BELT_SPEEDS[2],
        STORAGE_CAP,
        POLE_RADIUS,
        POLE_LINK,
        CORE_RADIUS,
        CORE_POWER,
        CLOCK_PCT[0],
        CLOCK_PCT[1],
        CLOCK_PCT[2],
        CLOCK_PCT[3],
        LEGACY_PCT
    );
    s
}

/// Goal as (kind, a, b) for the UI and the stats block.
pub fn goal_parts(g: Goal) -> (&'static str, u64, u64) {
    match g {
        Goal::Build(k, n) => ("build", k as u64, n as u64),
        Goal::Deliver(item, n) => ("deliver", item as u64, n as u64),
        Goal::Research(t) => ("research", t as u64, 1),
        Goal::ReachTi(n) => ("ti", 0, n),
        Goal::ReachStage(st) => ("stage", st as u64, 1),
        Goal::Ark(n) => ("ark", n as u64, n as u64),
        Goal::Salvage(n) => ("salvage", 0, n as u64),
    }
}
