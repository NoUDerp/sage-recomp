//! Port of `Common/Thing/KindOfTokens.cpp` from openbfme_core.
//!
//! Upstream sources:
//!   - `engine/src/Common/Thing/KindOfTokens.cpp`
//!   - `engine/src/Common/Thing/KindOfTokens.h`
//!   - `engine/src/GameLogic/BitFlagNames.cpp`
//!
//! Safe, deterministic, dependency-free: `core` and `std` only, no `unsafe`,
//! no I/O, no clock, no randomness, no global mutable state. The verifier
//! compiles this file on its own, so it may not reference other modules.

#![allow(unused_variables)]

pub fn kind_of_name(index: u32) -> Option<String> {
    unimplemented!("port Common/Thing/KindOfTokens.cpp: kind_of_name")
}

pub fn kind_of_canonical_index(index: u32) -> i32 {
    unimplemented!("port Common/Thing/KindOfTokens.cpp: kind_of_canonical_index")
}

pub fn kind_of_bit_mask(index: u32) -> Vec<u32> {
    unimplemented!("port Common/Thing/KindOfTokens.cpp: kind_of_bit_mask")
}
