//! Port of `Common/Sha256.cpp` from openbfme_core.
//!
//! Upstream sources:
//!   - `engine/src/Common/Sha256.cpp`
//!   - `engine/src/Common/Sha256.h`
//!
//! Safe, deterministic, dependency-free: `core` and `std` only, no `unsafe`,
//! no I/O, no clock, no randomness, no global mutable state. The verifier
//! compiles this file on its own, so it may not reference other modules.

#![allow(unused_variables)]

pub fn sha256_hex(data: &[u8]) -> String {
    unimplemented!("port Common/Sha256.cpp: sha256_hex")
}

pub fn sha256_stream_hex(first: &[u8], second: &[u8]) -> String {
    unimplemented!("port Common/Sha256.cpp: sha256_stream_hex")
}

pub fn hmac_sha256_hex(key: &[u8], data: &[u8]) -> String {
    unimplemented!("port Common/Sha256.cpp: hmac_sha256_hex")
}
