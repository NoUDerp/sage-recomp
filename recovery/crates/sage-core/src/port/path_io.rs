//! Port of `Common/System/PathIO.cpp` from openbfme_core.
//!
//! Upstream sources:
//!   - `engine/src/Common/System/PathIO.cpp`
//!   - `engine/src/Common/PathIO.h`
//!
//! Safe, deterministic, dependency-free: `core` and `std` only, no `unsafe`,
//! no I/O, no clock, no randomness, no global mutable state. The verifier
//! compiles this file on its own, so it may not reference other modules.

#![allow(unused_variables)]

pub fn check_install_path(path: &str) -> Option<String> {
    unimplemented!("port Common/System/PathIO.cpp: check_install_path")
}
