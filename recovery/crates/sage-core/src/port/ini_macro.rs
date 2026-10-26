//! Port of `Common/INI/INIMacro.cpp` from openbfme_core.
//!
//! Upstream sources:
//!   - `engine/src/Common/INI/INIMacro.cpp`
//!   - `engine/src/Common/INI.h`
//!   - `engine/src/Common/AsciiString.cpp`
//!   - `engine/src/Common/AsciiString.h`
//!
//! Safe, deterministic, dependency-free: `core` and `std` only, no `unsafe`,
//! no I/O, no clock, no randomness, no global mutable state. The verifier
//! compiles this file on its own, so it may not reference other modules.

#![allow(unused_variables)]

pub fn macro_hash(name: &str) -> u32 {
    unimplemented!("port Common/INI/INIMacro.cpp: macro_hash")
}

pub fn macro_value(defines: &str, name: &str) -> Option<String> {
    unimplemented!("port Common/INI/INIMacro.cpp: macro_value")
}
