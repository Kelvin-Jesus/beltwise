// Mirrors of engine ABI constants (engine/src/lib.rs, world.rs, render.rs, machines.rs).
// Game content (items, buildings, research...) is NOT duplicated here: it comes from the
// engine as JSON at start-up (see content.ts).

export const ABI_VERSION = 3;

export const WORLD_SIZE = 512;
export const TICK_MS = 1000 / 60;
export const TICKS_PER_SEC = 60;
/** Max fixed steps per frame before we drop time (prevents a spiral of death after stalls). */
export const MAX_STEPS_PER_FRAME = 8;

/** Building kinds that the UI treats specially. */
export const Kind = {
  Empty: 0,
  Belt: 1,
  Core: 2,
  Drill: 3,
  Tunnel: 19,
  Pole: 21,
  Sorter: 27,
  Storage: 28,
  DronePort: 29,
  Radar: 30,
  WaterPump: 33,
  Hatchery: 36,
  Wreck: 37,
} as const;

/** E, S, W, N — clockwise in the y-down world. */
export const DX = [1, 0, -1, 0] as const;
export const DY = [0, 1, 0, -1] as const;

export const CursorFlag = { Visible: 1, Delete: 2, Paste: 4 } as const;

/** Offsets into the u32 stats block (engine/src/world.rs `stat`). */
export const Stat = {
  Tick: 0,
  Delivered: 1,
  Rate: 2,
  Stage: 3,
  TiLo: 4,
  TiHi: 5,
  TiRateX100: 6,
  Meters: 7,
  MeterRates: 15,
  Objective: 19,
  GoalHaveLo: 20,
  GoalHaveHi: 21,
  GoalNeedLo: 22,
  GoalNeedHi: 23,
  Items: 24,
  Belts: 25,
  Segments: 26,
  Machines: 27,
  Instances: 28,
  BeltSpeed: 29,
  CoreX: 30,
  CoreY: 31,
  CoreSize: 32,
  ResearchedLo: 33,
  ResearchedHi: 34,
  Revision: 35,
  UndoDepth: 36,
  GoalKind: 37,
  GoalA: 38,
  MapRev: 39,
  ArkPhase: 40,
  CreditsLo: 41,
  CreditsHi: 42,
  PowerSupply: 43,
  PowerDemand: 44,
  PowerSat: 45,
  BatteryMj: 46,
  BatteryCapMj: 47,
  PowerRev: 48,
  FogRev: 49,
  FogDirty: 50,
  ResRev: 54,
  Daylight: 55,
  ShowerIn: 56,
  ShowerActive: 57,
  AchLo: 58,
  AchHi: 59,
  Probes: 60,
  Alts: 61,
  Logs: 62,
  Salvaged: 63,
  Legacy: 64,
  Planet: 65,
  Cosmetics: 66,
  BeltColor: 67,
  Trim: 68,
  Levels: 69,
  Meteors: 73,
  Nets: 74,
  DronesFlying: 75,
  WaterX1000: 76,
  ArkPaid: 77,
  LastAch: 81,
  BpCells: 82,
} as const;
export const STATS_LEN = 96;

/** Goal kinds in the stats block. */
export const GoalKind = { Build: 0, Deliver: 1, Research: 2, Ti: 3, Stage: 4, Ark: 5, Salvage: 6 } as const;

/** Render level-of-detail and overlay flags (engine/src/render.rs `lod`). */
export const Lod = { Items: 1, Structures: 2, Status: 4, Lights: 8, Weather: 16 } as const;

/** Tool ids besides building kinds. */
export const TOOL_MOVE = -1;
export const TOOL_DELETE = -2;
export const TOOL_COPY = -3;
export const TOOL_PASTE = -4;

/** Why a placement was refused (engine/src/world.rs `reason`). */
export const Reason = {
  Ok: 0,
  Bounds: 1,
  Locked: 2,
  Occupied: 3,
  NoDeposit: 4,
  DepositLocked: 5,
  Cost: 6,
  Fog: 7,
  Lava: 8,
  NoWater: 9,
  WrongDeposit: 10,
} as const;

/** Machine status (engine/src/machines.rs `status`). */
export const Status = { Idle: 0, Working: 1, NoInput: 2, Blocked: 3, NoPower: 4, NoFuel: 5, Unlinked: 6 } as const;

/** What a salvaged wreck held (engine/src/explore.rs `found`). */
export const Found = { Nothing: 0, Cache: 1, Probe: 2, Log: 3, Shards: 4, Amplifier: 5, Credits: 6 } as const;

/**
 * Sprite ids (engine/src/render.rs `sprite`). Atlas (16x8 cells): items at their id,
 * buildings at 64 + kind, then the drone and the tunnel exit. 128 and up are shader shapes.
 */
export const Sprite = {
  Building: 64,
  Drone: 126,
  TunnelExit: 127,
  Belt: 128,
  BeltRight: 129,
  BeltLeft: 130,
  Core: 131,
  Delete: 132,
  Glow: 133,
  Status: 134,
  Meteor: 135,
  Flash: 136,
  Rain: 137,
  Snow: 138,
  Bird: 139,
  Shadow: 140,
} as const;
export const ATLAS_COLS = 16;
export const ATLAS_ROWS = 8;
export const ATLAS_CELL = 128;

/** Bytes per instance record (engine/src/render.rs `Instance`). */
export const INSTANCE_BYTES = 16;

/** Inspector block layout (engine/src/lib.rs `fx_inspect`). */
export const Info = {
  Kind: 0,
  Dir: 1,
  Recipe: 2,
  Status: 3,
  Progress: 4,
  Inv: 5,
  Out: 8,
  Held: 9,
  Aux: 10,
  A: 11,
  B: 12,
  X: 13,
  Y: 14,
  Flags: 15,
  Shards: 16,
  Purity: 17,
  Filter: 18,
  Net: 19,
  Draw: 20,
  SpeedPct: 21,
  Sat: 22,
  Extra: 23,
} as const;
export const INFO_LEN = 24;
export const NO_RECIPE = 255;
export const NONE = 0xffffffff;
/** `Machine::flags` bits. */
export const MachineFlag = { Locked: 1, Amp: 2 } as const;
/** Bytes per blueprint cell (engine/src/blueprint.rs). */
export const BP_CELL_BYTES = 8;
