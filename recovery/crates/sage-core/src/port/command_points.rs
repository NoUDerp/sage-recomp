//! Port of `Common/CommandPoints.cpp` from openbfme_core.
//!
//! Upstream sources:
//!   - `engine/src/Common/CommandPoints.cpp`
//!   - `engine/src/Common/CommandPoints.h`
//!
//! Safe, deterministic, dependency-free: `core` and `std` only, no `unsafe`,
//! no I/O, no clock, no randomness, no global mutable state. The verifier
//! compiles this file on its own, so it may not reference other modules.

#![allow(unused_variables)]

pub fn cp_limit(base: i32, bonus: i32, cap: i32, record_values: &[u32]) -> i32 {
    unimplemented!("port Common/CommandPoints.cpp: cp_limit")
}

pub fn cp_can_afford(usage: i32, template_command_points: i32, army_of_dead: bool, limit: i32) -> bool {
    unimplemented!("port Common/CommandPoints.cpp: cp_can_afford")
}

pub fn cp_init_cap(script_cap: i32, script_set: bool, source_cap: i32, lobby_percent: i32, apply_lobby_percent: bool) -> i32 {
    unimplemented!("port Common/CommandPoints.cpp: cp_init_cap")
}

pub fn cp_init_base(script_base: i32, script_set: bool, start: i32, territory_good_term: i32, territory_evil_term: i32) -> i32 {
    unimplemented!("port Common/CommandPoints.cpp: cp_init_base")
}
