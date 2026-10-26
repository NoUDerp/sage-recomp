//! Port of `Common/MpGameRules.cpp` from openbfme_core.
//!
//! Upstream sources:
//!   - `engine/src/Common/MpGameRules.cpp`
//!   - `engine/src/Common/MpGameRules.h`
//!
//! Safe, deterministic, dependency-free: `core` and `std` only, no `unsafe`,
//! no I/O, no clock, no randomness, no global mutable state. The verifier
//! compiles this file on its own, so it may not reference other modules.

#![allow(unused_variables)]

pub fn mp_rules_defaults(mode: i32, demo_mode: i32) -> Vec<u32> {
    unimplemented!("port Common/MpGameRules.cpp: mp_rules_defaults")
}

pub fn mp_rules_label(rule: i32) -> Option<String> {
    unimplemented!("port Common/MpGameRules.cpp: mp_rules_label")
}

pub fn mp_rules_choice_value(mode: i32, rule: i32, choice: u32) -> Option<i32> {
    unimplemented!("port Common/MpGameRules.cpp: mp_rules_choice_value")
}

pub fn mp_legendary_damage_factor(factor: i32) -> f64 {
    unimplemented!("port Common/MpGameRules.cpp: mp_legendary_damage_factor")
}
