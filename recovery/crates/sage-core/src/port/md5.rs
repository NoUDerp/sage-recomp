//! Port of `Common/MD5.cpp` from openbfme_core.
//!
//! Upstream sources:
//!   - `engine/src/Common/MD5.cpp`
//!   - `engine/src/Common/MD5.h`
//!
//! Safe, deterministic, dependency-free: `core` and `std` only, no `unsafe`,
//! no I/O, no clock, no randomness, no global mutable state. The verifier
//! compiles this file on its own, so it may not reference other modules.

#![allow(unused_variables)]

pub fn md5_hex(data: &[u8]) -> String {
    unimplemented!("port Common/MD5.cpp: md5_hex")
}

pub fn md5_stream_hex(first: &[u8], second: &[u8]) -> String {
    unimplemented!("port Common/MD5.cpp: md5_stream_hex")
}
