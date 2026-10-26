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

// MpGameRules.h: enum GameRuleId, NUM_GAME_RULES = 10.
const NUM_GAME_RULES: usize = 10;

// MpGameRules.cpp anonymous namespace: the per-mode combo tables. A combo is
// { id, choices (label + value), count, defaultIndex }; count == number of
// choices, so the choice slices below carry it.
struct RuleCombo {
    id: usize,
    choices: &'static [i32],
    default_index: usize,
}

// RW 0xDB77BC: mode 0 = skirmish/LAN/online custom game.
const INITIAL_RESOURCES: RuleCombo = RuleCombo {
    id: 4, // RULE_INITIAL_RESOURCES
    choices: &[500, 750, 1000, 1200, 1400, 1600, 1800, 2000, 2500, 3000, 4000],
    default_index: 2,
};
const COMMAND_POINT_FACTOR: RuleCombo = RuleCombo {
    id: 3, // RULE_COMMAND_POINT_FACTOR
    choices: &[33, 50, 100, 200, 400, 800, 10000],
    default_index: 2,
};

// RW 0xDB77D0: mode 1 = War of the Ring (RotWK's fifth row is VictoryType).
const STRATEGIC_PHASE_TIMER: RuleCombo = RuleCombo {
    id: 6, // RULE_STRATEGIC_PHASE_TIMER
    choices: &[0, 5, 10, 30, 45, 60, 120, 180, 500],
    default_index: 0,
};
const BATTLE_TYPE: RuleCombo = RuleCombo {
    id: 7, // RULE_BATTLE_TYPE
    choices: &[0, 1, 2],
    default_index: 0,
};
const BATTLE_CHOICE: RuleCombo = RuleCombo {
    id: 8, // RULE_BATTLE_CHOICE
    choices: &[0, 1],
    default_index: 0,
};
const AUTO_RESOLVE_TYPE: RuleCombo = RuleCombo {
    id: 9, // RULE_AUTO_RESOLVE_TYPE
    choices: &[0, 1],
    default_index: 0,
};
const VICTORY_TYPE: RuleCombo = RuleCombo {
    id: 5, // RULE_VICTORY_TYPE
    choices: &[0, 1, 2, 3, 4, 5, 6],
    default_index: 0,
};

// RW 0x960A10: comboBoxes(mode); null (empty here) for any other mode.
fn combo_table(mode: i32) -> &'static [&'static RuleCombo] {
    match mode {
        0 => &[&INITIAL_RESOURCES, &COMMAND_POINT_FACTOR],
        1 => &[
            &STRATEGIC_PHASE_TIMER,
            &BATTLE_TYPE,
            &BATTLE_CHOICE,
            &AUTO_RESOLVE_TYPE,
            &VICTORY_TYPE,
        ],
        _ => &[],
    }
}

// RW 0xDB77C4 kChecks, listed in table order per mode (RW 0x960A2B/0x960A5D):
// mode 0 uses all three entries, mode 1 the first two. Entry = (id, byte).
const MODE0_CHECKS: &[(usize, i32)] = &[(0, 1), (2, 1), (1, 0)];
const MODE1_CHECKS: &[(usize, i32)] = &[(0, 1), (2, 1)];

fn check_table(mode: i32) -> &'static [(usize, i32)] {
    match mode {
        0 => MODE0_CHECKS,
        1 => MODE1_CHECKS,
        _ => &[],
    }
}

// RW 0xDB7794: the rule label tag table.
const RULE_LABELS: [&str; NUM_GAME_RULES] = [
    "RULE:AllowCustomHeroes",
    "RULE:ClanGame",
    "RULE:AllowRingHeroes",
    "RULE:CommandPointFactor",
    "RULE:InitialResources",
    "RULE:VictoryType",
    "RULE:StrategicPhaseTimer",
    "RULE:BattleType",
    "RULE:BattleChoice",
    "RULE:AutoResolveType",
];

/// `MpGameRules::defaults` (RW 0x960A74): every rule starts -1; mode -1 stops
/// there; otherwise each combo contributes its default choice value, then
/// each check its byte value. Finally a demo mode (`demoMode & 3`, MpGameRules
/// kDemoMode is GlobalData + 0x9D4) forces rule 0 (`RULE_ALLOW_CUSTOM_HEROES`)
/// to 0 when it is set. Returned as the 32-bit patterns of the C++ ints.
pub fn mp_rules_defaults(mode: i32, demo_mode: i32) -> Vec<u32> {
    let mut r = [0xFFFF_FFFFu32; NUM_GAME_RULES]; // memset 0xFF
    if mode == -1 {
        return r.to_vec();
    }
    for combo in combo_table(mode) {
        r[combo.id] = combo.choices[combo.default_index] as u32;
    }
    for (id, value) in check_table(mode) {
        r[*id] = (*value != 0) as u32; // movzx of the byte
    }
    // demoRestrictsRules(demoMode) && r[RULE_ALLOW_CUSTOM_HEROES] != -1
    if (demo_mode & 3) != 0 && r[0] != 0xFFFF_FFFF {
        r[0] = 0;
    }
    r.to_vec()
}

/// `MpGameRules::ruleLabel` (RW 0x960B14): unsigned compare with 10, so rule
/// >= 10 (and negatives) have no label; retail prints "Invalid Rule" then.
pub fn mp_rules_label(rule: i32) -> Option<String> {
    if (rule as u32) < NUM_GAME_RULES as u32 {
        Some(RULE_LABELS[rule as usize].to_string())
    } else {
        None
    }
}

/// `MpGameRules::findCombo` plus the selected choice's value: the combo of
/// `rule` in `mode` (a mode without that combo, or an index past the choice
/// count, has no value).
pub fn mp_rules_choice_value(mode: i32, rule: i32, choice: u32) -> Option<i32> {
    for combo in combo_table(mode) {
        if combo.id == rule as usize {
            if (choice as usize) < combo.choices.len() {
                return Some(combo.choices[choice as usize]);
            }
            return None;
        }
    }
    None
}

/// `LegendaryHeroes::damageDealtFactor`: the literals 1.16666663f,
/// 1.33333337f, 1.5f, 1.0f (f32 by upstream; widened to the declared f64
/// exactly as C++ promotes the float at the call boundary), any other factor
/// (including < 1 and out of range) reads as off = 1.
pub fn mp_legendary_damage_factor(factor: i32) -> f64 {
    match factor {
        2 => 1.16666663f32 as f64,
        3 => 1.33333337f32 as f64,
        4 => 1.5f32 as f64,
        _ => 1.0f32 as f64,
    }
}
