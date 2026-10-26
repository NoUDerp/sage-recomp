//! Port of `Common/System/DeterministicLibm.cpp` from openbfme_core.
//!
//! Upstream sources:
//!   - `engine/src/Common/System/DeterministicLibm.cpp`
//!   - `engine/src/Common/DeterministicLibm.h`
//!
//! Safe, deterministic, dependency-free: `core` and `std` only, no `unsafe`,
//! no I/O, no clock, no randomness, no global mutable state. The verifier
//! compiles this file on its own, so it may not reference other modules.

#![allow(unused_variables)]

pub fn det_floor(x: f64) -> f64 {
    unimplemented!("port Common/System/DeterministicLibm.cpp: det_floor")
}

pub fn det_ceil(x: f64) -> f64 {
    unimplemented!("port Common/System/DeterministicLibm.cpp: det_ceil")
}

pub fn det_fmod(x: f64, y: f64) -> f64 {
    unimplemented!("port Common/System/DeterministicLibm.cpp: det_fmod")
}

pub fn det_sqrt(x: f64) -> f64 {
    unimplemented!("port Common/System/DeterministicLibm.cpp: det_sqrt")
}
