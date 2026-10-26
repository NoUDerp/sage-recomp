//! Port of `Common/Audio/Mp3Tables.cpp` from openbfme_core.
//!
//! Upstream sources:
//!   - `engine/src/Common/Audio/Mp3Tables.cpp`
//!   - `engine/src/Common/Audio/Mp3Tables.h`
//!
//! Safe, deterministic, dependency-free: `core` and `std` only, no `unsafe`,
//! no I/O, no clock, no randomness, no global mutable state. The verifier
//! compiles this file on its own, so it may not reference other modules.

#![allow(unused_variables)]

pub fn mp3_huffman_shape(table: u32) -> Vec<u32> {
    unimplemented!("port Common/Audio/Mp3Tables.cpp: mp3_huffman_shape")
}

pub fn mp3_huffman_len_code(table: u32, index: u32) -> Vec<u32> {
    unimplemented!("port Common/Audio/Mp3Tables.cpp: mp3_huffman_len_code")
}

pub fn mp3_quad_len_code(select: u32, index: u32) -> Vec<u32> {
    unimplemented!("port Common/Audio/Mp3Tables.cpp: mp3_quad_len_code")
}

pub fn mp3_synth_window(index: u32) -> Option<i32> {
    unimplemented!("port Common/Audio/Mp3Tables.cpp: mp3_synth_window")
}
