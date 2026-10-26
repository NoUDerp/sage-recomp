//! Port of `Common/System/PathIO.cpp` from openbfme_core.
//!
//! Upstream sources:
//!   - `engine/src/Common/System/PathIO.cpp`
//!   - `engine/src/Common/PathIO.h`
//!
//! Safe, deterministic, dependency-free: `core` and `std` only, no `unsafe`,
//! no I/O, no clock, no randomness, no global mutable state. The verifier
//! compiles this file on its own, so it may not reference other modules.

#![allow(unused_variables)]

pub fn check_install_path(path: &str) -> Option<String> {
    if path.is_empty() || path.bytes().any(|byte| matches!(byte, b'\r' | b'\n' | 0)) {
        return Some(format!(
            "folder cannot be stored in install-paths.cfg (empty, NUL or line break): {}",
            path
        ));
    }
    None
}
