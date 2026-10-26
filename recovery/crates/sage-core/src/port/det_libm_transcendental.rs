//! Port of `Common/System/DeterministicLibm.cpp` from openbfme_core.
//!
//! Upstream sources:
//!   - `engine/src/Common/System/DeterministicLibm.cpp`
//!   - `engine/src/Common/System/DeterministicLibmTables.inc`
//!   - `engine/src/Common/DeterministicLibm.h`
//!
//! Safe, deterministic, dependency-free: `core` and `std` only, no `unsafe`,
//! no I/O, no clock, no randomness, no global mutable state. The verifier
//! compiles this file on its own, so it may not reference other modules.

#![allow(unused_variables)]

pub fn det_sin(x: f64) -> f64 {
    unimplemented!("port Common/System/DeterministicLibm.cpp: det_sin")
}

pub fn det_cos(x: f64) -> f64 {
    unimplemented!("port Common/System/DeterministicLibm.cpp: det_cos")
}

pub fn det_atan2(y: f64, x: f64) -> f64 {
    unimplemented!("port Common/System/DeterministicLibm.cpp: det_atan2")
}

pub fn det_exp(x: f64) -> f64 {
    unimplemented!("port Common/System/DeterministicLibm.cpp: det_exp")
}
