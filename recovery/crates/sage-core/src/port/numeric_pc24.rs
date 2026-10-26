//! Port of `Common/System/NumericState.cpp` from openbfme_core.
//!
//! Upstream sources:
//!   - `engine/src/Common/System/NumericState.cpp`
//!   - `engine/src/Common/NumericState.h`
//!
//! Safe, deterministic, dependency-free: `core` and `std` only, no `unsafe`,
//! no I/O, no clock, no randomness, no global mutable state. The verifier
//! compiles this file on its own, so it may not reference other modules.

#![allow(unused_variables)]

pub fn pc24_add_w(a: f64, b: f64) -> f64 {
    unimplemented!("port Common/System/NumericState.cpp: pc24_add_w")
}

pub fn pc24_mul_w(a: f64, b: f64) -> f64 {
    unimplemented!("port Common/System/NumericState.cpp: pc24_mul_w")
}

pub fn pc24_div_w(a: f64, b: f64) -> f64 {
    unimplemented!("port Common/System/NumericState.cpp: pc24_div_w")
}

pub fn fstp_dword_bits(x: f64) -> u32 {
    unimplemented!("port Common/System/NumericState.cpp: fstp_dword_bits")
}
