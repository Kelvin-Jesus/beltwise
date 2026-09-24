//! Engine-wide constants and direction helpers. Game content lives in `content.rs`.

/// Sentinel for "no index" in `u32` index arrays.
pub const NONE: u32 = u32::MAX;

/// Fixed simulation rate. Rendering runs at any rate and interpolates between ticks.
pub const TICKS_PER_SEC: u32 = 60;

// ---- Belt fixed-point geometry ------------------------------------------------------------

/// Sub-units per tile along a belt path. All belt positions are integers.
pub const SUB: u32 = 96;
/// Minimum spacing between two consecutive items (two items per tile when compressed).
/// Every belt speed divides it exactly, so hand-overs never lose a tick to rounding.
pub const MIN_GAP: u32 = 48;
/// Chains longer than this are split into several segments. Keeps every gap within `u16`
/// and bounds the cost of a mid-segment (side-load) insert.
pub const MAX_SEG_TILES: u32 = 256;

/// Tunnels reach up to this many tiles from entrance to exit.
pub const TUNNEL_RANGE: i32 = 6;

/// Directions, clockwise in a y-down world: E=0, S=1, W=2, N=3.
pub mod dir {
    pub const E: u8 = 0;
    pub const S: u8 = 1;
    pub const W: u8 = 2;
    pub const N: u8 = 3;

    #[inline(always)]
    pub const fn dx(d: u8) -> i32 {
        match d & 3 {
            0 => 1,
            2 => -1,
            _ => 0,
        }
    }
    #[inline(always)]
    pub const fn dy(d: u8) -> i32 {
        match d & 3 {
            1 => 1,
            3 => -1,
            _ => 0,
        }
    }
    #[inline(always)]
    pub const fn opposite(d: u8) -> u8 {
        (d + 2) & 3
    }
    #[inline(always)]
    pub const fn cw(d: u8) -> u8 {
        (d + 1) & 3
    }
    #[inline(always)]
    pub const fn ccw(d: u8) -> u8 {
        (d + 3) & 3
    }
}
