//! Port of `Common/RandomValue.cpp` from openbfme_core.
//!
//! Upstream sources:
//!   - `engine/src/Common/RandomValue.cpp`
//!   - `engine/src/Common/RandomValue.h`
//!
//! Safe, deterministic, dependency-free: `core` and `std` only, no `unsafe`,
//! no I/O, no clock, no randomness, no global mutable state. The verifier
//! compiles this file on its own, so it may not reference other modules.

#![allow(unused_variables)]

pub fn zh_next_u32(seed: u32, draws: u32) -> u32 {
    unimplemented!("port Common/RandomValue.cpp: zh_next_u32")
}

pub fn lcg_next_u32(seed: u32, draws: u32) -> u32 {
    unimplemented!("port Common/RandomValue.cpp: lcg_next_u32")
}

pub fn zh_value_in_range(seed: u32, draws: u32, lo: i32, hi: i32) -> i32 {
    unimplemented!("port Common/RandomValue.cpp: zh_value_in_range")
}

pub fn lcg_value_in_range(seed: u32, draws: u32, lo: i32, hi: i32) -> i32 {
    unimplemented!("port Common/RandomValue.cpp: lcg_value_in_range")
}
