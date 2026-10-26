//! Port of `Common/System/Dict.cpp` from openbfme_core.
//!
//! Upstream sources:
//!   - `engine/src/Common/System/Dict.cpp`
//!   - `engine/src/Common/Dict.h`
//!
//! Safe, deterministic, dependency-free: `core` and `std` only, no `unsafe`,
//! no I/O, no clock, no randomness, no global mutable state. The verifier
//! compiles this file on its own, so it may not reference other modules.

#![allow(unused_variables)]

pub fn dict_dump(program: &[u8]) -> String {
    unimplemented!("port Common/System/Dict.cpp: dict_dump")
}

pub fn dict_lookup(program: &[u8], key: u32) -> String {
    unimplemented!("port Common/System/Dict.cpp: dict_lookup")
}
