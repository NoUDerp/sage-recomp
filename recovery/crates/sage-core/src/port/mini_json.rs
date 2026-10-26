//! Port of `Common/MiniJson.cpp` from openbfme_core.
//!
//! Upstream sources:
//!   - `engine/src/Common/MiniJson.cpp`
//!   - `engine/src/Common/MiniJson.h`
//!
//! Safe, deterministic, dependency-free: `core` and `std` only, no `unsafe`,
//! no I/O, no clock, no randomness, no global mutable state. The verifier
//! compiles this file on its own, so it may not reference other modules.

#![allow(unused_variables)]

pub fn canonicalise(text: &str) -> Option<String> {
    unimplemented!("port Common/MiniJson.cpp: canonicalise")
}

pub fn parse_error(text: &str) -> Option<String> {
    unimplemented!("port Common/MiniJson.cpp: parse_error")
}
