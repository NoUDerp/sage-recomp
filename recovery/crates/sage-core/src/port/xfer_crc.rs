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

/// StateHasher::addU32 (Common/StateHash.h, RW 0xA211DF):
/// `crc = rotl32(crc, 1) + v` mod 2^32.
#[inline]
fn add_u32(crc: &mut u32, v: u32) {
    *crc = crc.rotate_left(1).wrapping_add(v);
}

/// XferCRC::xferImplementation (Common/System/XferCRC.cpp): whole little-endian
/// words first, then one rotate-add per trailing byte. Word grouping restarts
/// at every transfer.
fn feed(crc: &mut u32, data: &[u8]) {
    let mut chunks = data.chunks_exact(4);
    for word in &mut chunks {
        add_u32(crc, u32::from_le_bytes([word[0], word[1], word[2], word[3]]));
    }
    for &byte in chunks.remainder() {
        add_u32(crc, byte as u32);
    }
}

pub fn xfer_crc_bytes(data: &[u8]) -> u32 {
    let mut crc = 0u32;
    feed(&mut crc, data);
    crc
}

pub fn xfer_crc_chunks(first: &[u8], second: &[u8]) -> u32 {
    // beginBlock/endBlock add nothing; each transfer restarts its word grouping.
    let mut crc = 0u32;
    feed(&mut crc, first);
    feed(&mut crc, second);
    crc
}

pub fn xfer_crc_words(values: &[u32]) -> u32 {
    // StateHasher::addU32 per word; equals the little-endian byte path of the
    // same words (the reference build is little-endian).
    let mut crc = 0u32;
    for &value in values {
        add_u32(&mut crc, value);
    }
    crc
}
