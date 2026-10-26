//! Port of `Common/CommandPoints.cpp` from openbfme_core.
//!
//! Upstream sources:
//!   - `engine/src/Common/CommandPoints.cpp`
//!   - `engine/src/Common/CommandPoints.h`
//!
//! Safe, deterministic, dependency-free: `core` and `std` only, no `unsafe`,
//! no I/O, no clock, no randomness, no global mutable state. The verifier
//! compiles this file on its own, so it may not reference other modules.

#![allow(unused_variables)]

// CpMath (Common/CommandPoints.h): the binary's 32-bit add/sub/imul on
// two's-complement bit patterns, with SIGNED compares afterwards.

/// CommandPoints::getLimit (RW 0x6A7B9F) with records whose filters are unset
/// (`!isValid` -> the record always counts): total = bonus + base + every
/// record value (wrapping 32-bit adds), then `total > cap ? cap : total`
/// (cmovg, signed).
pub fn cp_limit(base: i32, bonus: i32, cap: i32, record_values: &[u32]) -> i32 {
    let mut total = bonus.wrapping_add(base);
    for &value in record_values {
        total = total.wrapping_add(value as i32);
    }
    if total > cap {
        cap
    } else {
        total
    }
}

/// CommandPoints::canAfford (RW 0x6A7F79): a zero template cost is always
/// affordable; otherwise the wrapped `usage + cp` is compared signed with the
/// limit; an ARMY_OF_DEAD template is always affordable.
pub fn cp_can_afford(usage: i32, template_command_points: i32, army_of_dead: bool, limit: i32) -> bool {
    if template_command_points == 0 {
        return true;
    }
    let ok = usage.wrapping_add(template_command_points) <= limit;
    if army_of_dead {
        true
    } else {
        ok
    }
}

/// CommandPoints::init (RW 0x6A7C86), the cap half: the cap is stored when no
/// script set one or when the source cap is larger; then the lobby percentage
/// applies `imul` (wrapping) and `cdq; idiv 100` (truncation toward zero).
pub fn cp_init_cap(script_cap: i32, script_set: bool, source_cap: i32, lobby_percent: i32, apply_lobby_percent: bool) -> i32 {
    let mut cap = script_cap;
    if !script_set || source_cap > cap {
        cap = source_cap;
    }
    if apply_lobby_percent {
        cap = cap.wrapping_mul(lobby_percent) / 100;
    }
    cap
}

/// CommandPoints::init (RW 0x6A7C86), the base half: the base is
/// `start + territoryGoodTerm + territoryEvilTerm` (RW 0x6A7ECA, wrapping);
/// stored when no script set one or when it is larger.
pub fn cp_init_base(script_base: i32, script_set: bool, start: i32, territory_good_term: i32, territory_evil_term: i32) -> i32 {
    let base = start.wrapping_add(territory_good_term).wrapping_add(territory_evil_term);
    if !script_set || base > script_base {
        base
    } else {
        script_base
    }
}
