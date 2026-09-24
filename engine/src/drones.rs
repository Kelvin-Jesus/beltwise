//! Drone ports. A port linked to another loads everything fed into it onto its drone,
//! which flies the load to the other port's outbox and comes back. Ports need power to
//! launch; a drone in the air always finishes its trip.

use crate::content::{Class, bk};
use crate::core::event;
use crate::machines::{DRONE_CARGO, NO_NET, flight, status};
use crate::types::NONE;
use crate::world::World;

/// Cruise speed: 10 tiles per second.
const TICKS_PER_TILE: f32 = 6.0;
/// Take-off and landing, each.
pub const TAKEOFF: u16 = 30;
/// A partial load leaves after waiting this long.
const PATIENCE: u16 = 180;

fn flight_ticks(a: (i32, i32), b: (i32, i32)) -> u16 {
    let (dx, dy) = ((b.0 - a.0) as f32, (b.1 - a.1) as f32);
    ((dx * dx + dy * dy).sqrt() * TICKS_PER_TILE) as u16 + 2 * TAKEOFF
}

impl World {
    /// Points the drone port at (x, y) to the port at (tx, ty); a target off the map unlinks.
    pub fn link_port(&mut self, x: i32, y: i32, tx: i32, ty: i32) -> bool {
        let from = self.grid.index(x, y);
        if from == NONE || self.grid.kind[from as usize] != bk::DRONE_PORT {
            return false;
        }
        let to = self.grid.index(tx, ty);
        let i = self.grid.ent[from as usize] as usize;
        if to == NONE {
            self.machines.list[i].aux = NONE;
            return true;
        }
        if to == from || self.grid.kind[to as usize] != bk::DRONE_PORT {
            return false;
        }
        self.machines.list[i].aux = to;
        self.core.flags |= event::DRONE_LINK;
        true
    }

    /// Forgets every link that points at `tile` (its port is being removed).
    pub(crate) fn unlink_port(&mut self, tile: u32) {
        for m in &mut self.machines.list {
            if m.class() == Class::DronePort && m.aux == tile {
                m.aux = NONE;
            }
        }
    }

    pub(crate) fn tick_drones(&mut self) {
        for p in 0..self.machines.ports.len() {
            let mut port = self.machines.ports[p];
            if port.tile == NONE {
                continue;
            }
            let mi = self.grid.ent[port.tile as usize] as usize;
            let m = self.machines.list[mi];
            let sat = self.machines.sat_of(&m);
            match port.state {
                flight::HOME => {
                    if m.aux == NONE || port.inbox.len == 0 {
                        port.wait = 0;
                    } else {
                        port.wait = port.wait.saturating_add(1);
                        if sat > 0 && (port.inbox.len as usize >= DRONE_CARGO || port.wait >= PATIENCE) {
                            while port.cargo.free() > 0 {
                                match port.inbox.pop() {
                                    Some(item) => {
                                        port.cargo.push(item);
                                    }
                                    None => break,
                                }
                            }
                            port.state = flight::OUTBOUND;
                            port.t = 0;
                            port.dest = m.aux;
                            port.dur = flight_ticks(self.grid.xy(port.tile), self.grid.xy(m.aux));
                            port.wait = 0;
                        }
                    }
                }
                flight::OUTBOUND => {
                    port.t += 1;
                    if port.t >= port.dur {
                        port.state = flight::UNLOADING;
                    }
                }
                flight::UNLOADING => {
                    let dest = port.dest;
                    if dest as usize >= self.grid.kind.len()
                        || self.grid.kind[dest as usize] != bk::DRONE_PORT
                    {
                        port.state = flight::RETURNING;
                        port.t = 0;
                    } else {
                        let q = self.machines.list[self.grid.ent[dest as usize] as usize].queue as usize;
                        let target = &mut self.machines.ports[q];
                        while let Some(item) = port.cargo.front() {
                            if !target.outbox.push(item) {
                                break;
                            }
                            port.cargo.pop();
                        }
                        if port.cargo.len == 0 {
                            port.state = flight::RETURNING;
                            port.t = 0;
                        }
                    }
                }
                _ => {
                    port.t += 1;
                    if port.t >= port.dur {
                        port.state = flight::HOME;
                        // Cargo for a port that vanished goes back into the inbox.
                        while let Some(item) = port.cargo.pop() {
                            if !port.inbox.push(item) {
                                break;
                            }
                        }
                    }
                }
            }
            let flying = port.state != flight::HOME;
            if flying && m.net != NO_NET {
                self.machines.nets[m.net as usize].demand += m.draw;
            }
            let s = if m.aux == NONE {
                status::UNLINKED
            } else if flying {
                status::WORKING
            } else if port.inbox.len > 0 && sat == 0 {
                status::NO_POWER
            } else {
                status::IDLE
            };
            let mm = &mut self.machines.list[mi];
            mm.status = s;
            mm.stall =
                if s == status::NO_POWER || s == status::UNLINKED { mm.stall.saturating_add(1) } else { 0 };
            self.machines.ports[p] = port;
        }
    }

    /// Where port `p`'s drone is, interpolated `alpha` into the next tick: (x, y, height),
    /// height 0 on the ground and 1 at cruise altitude. None while it is parked.
    pub fn drone_pos(&self, p: usize, alpha: f32) -> Option<(f32, f32, f32)> {
        let port = &self.machines.ports[p];
        if port.tile == NONE || port.state == flight::HOME || port.dest == NONE {
            return None;
        }
        let (hx, hy) = self.grid.xy(port.tile);
        let (dx, dy) = self.grid.xy(port.dest);
        let (home, dest) = ((hx as f32 + 0.5, hy as f32 + 0.5), (dx as f32 + 0.5, dy as f32 + 0.5));
        let dur = port.dur.max(1) as f32;
        let t = (port.t as f32 + alpha).min(dur);
        let lift = |t: f32| (t / TAKEOFF as f32).min((dur - t) / TAKEOFF as f32).clamp(0.0, 1.0);
        let lerp = |a: (f32, f32), b: (f32, f32), f: f32| (a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f);
        let ease = |f: f32| f * f * (3.0 - 2.0 * f);
        Some(match port.state {
            flight::OUTBOUND => {
                let (x, y) = lerp(home, dest, ease(t / dur));
                (x, y, lift(t))
            }
            flight::UNLOADING => (dest.0, dest.1, 0.3),
            _ => {
                let (x, y) = lerp(dest, home, ease(t / dur));
                (x, y, lift(t))
            }
        })
    }
}
