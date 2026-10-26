//! Port of `Common/NameKeyGenerator.cpp` from openbfme_core.
//!
//! Upstream sources:
//!   - `engine/src/Common/NameKeyGenerator.cpp`
//!   - `engine/src/Common/NameKeyGenerator.h`
//!
//! Safe, deterministic, dependency-free: `core` and `std` only, no `unsafe`,
//! no I/O, no clock, no randomness, no global mutable state. The verifier
//! compiles this file on its own, so it may not reference other modules.

#![allow(unused_variables)]

pub fn name_key_hash(name: &str) -> u32 {
    unimplemented!("port Common/NameKeyGenerator.cpp: name_key_hash")
}

pub fn name_key_socket(name: &str) -> u32 {
    unimplemented!("port Common/NameKeyGenerator.cpp: name_key_socket")
}

pub fn name_keys(names: &str) -> Vec<u32> {
    unimplemented!("port Common/NameKeyGenerator.cpp: name_keys")
}
