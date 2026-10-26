//! Port of `Common/System/XferCRC.cpp` from openbfme_core.
//!
//! Upstream sources:
//!   - `engine/src/Common/System/XferCRC.cpp`
//!   - `engine/src/Common/XferCRC.h`
//!   - `engine/src/Common/StateHash.h`
//!
//! Safe, deterministic, dependency-free: `core` and `std` only, no `unsafe`,
//! no I/O, no clock, no randomness, no global mutable state. The verifier
//! compiles this file on its own, so it may not reference other modules.

#![allow(unused_variables)]

pub fn xfer_crc_bytes(data: &[u8]) -> u32 {
    unimplemented!("port Common/System/XferCRC.cpp: xfer_crc_bytes")
}

pub fn xfer_crc_chunks(first: &[u8], second: &[u8]) -> u32 {
    unimplemented!("port Common/System/XferCRC.cpp: xfer_crc_chunks")
}

pub fn xfer_crc_words(values: &[u32]) -> u32 {
    unimplemented!("port Common/System/XferCRC.cpp: xfer_crc_words")
}
