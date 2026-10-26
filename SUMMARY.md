# SUMMARY — port-0015 (Common/MpGameRules.cpp)

Task: `port-0015` (issue #19), policy 18, toolchain `sha256:32a34a3e20be…`.
Code output: `recovery/crates/sage-core/src/port/mp_game_rules.rs`
(`SUMMARY.md` is included per maintainer instruction and is the only other
file in the branch).

## What was ported

Upstream spec: `engine/src/Common/MpGameRules.cpp` and `Common/MpGameRules.h`
at pinned commit `f45ea04a575677eaaf497f307548b60853528d4e`.

- `mp_rules_defaults` = `MpGameRules::defaults` (RW 0x960A74): ten rules
  start at `-1` (memset 0xFF) — returned as the u32 bit patterns of the C++
  ints — mode `-1` returns immediately; otherwise the mode's combo defaults
  (RW 0x960A10/0x960A46 tables, mode 0: InitialResources→1000,
  CommandPointFactor→100; mode 1: the five WotR rows, all default 0), then
  the check bytes in table order (mode 0: rules 0,2 → 1 and rule 1 → 0;
  mode 1: first two only). A demo mode (`demoMode & 3`, GlobalData + 0x9D4)
  forces rule 0 to 0 only when it is set — the early return for mode `-1`
  and untouched modes `-2/2` skip that step. Any mode other than -1/0/1 has
  no combo/check table (nullptr + count 0), i.e. all `-1`.
- `mp_rules_label` = `ruleLabel` (RW 0x960B14): **unsigned** compare with 10,
  so negatives and ≥10 have no label (`None`; retail prints "Invalid Rule"
  from RW 0xC8292C, which the declared `Option<String>` models as absent).
- `mp_rules_choice_value` = `findCombo` + selection: combo of `rule` in
  `mode` with `choice < count` yields the choice value, otherwise `None`.
  The declared rule range 3..9 covers combos only, so the check rules
  (0,1,2) never enter this entry.
- `mp_legendary_damage_factor` = `LegendaryHeroes::damageDealtFactor`:
  literal f32 constants, "no arithmetic" per upstream — `1.16666663f`
  (0x3F955555), `1.33333337f` (0x3FAAAAAB), `1.5f`, default `1.0f`; widened
  to the declared f64 exactly as C++ promotes at the call boundary (e.g.
  factor 2 → 1.1666666269302368, *not* 7.0/6.0 = 1.1666666666666667).

All constant tables (choices, counts, default indexes, check entries, label
tags) were diffed word-for-word against the pinned `.cpp`, not transcribed
from memory. No ambiguous behaviour: integers and fixed tables only, the one
float handled as literals per the upstream comment. Module is `core`/`std`
only, no `unsafe`, no I/O, no clock/random/global mutable state.

## Evidence

- `recomp task verify`: 8/8 case-group digests (256 cases), `pass: true`.
  No golden contents read, nothing hard-coded.
- Independent cross-check: the declared input space is small, so it was
  checked **exhaustively** — 437/437: all 25 (mode,demo) defaults, rules
  −3..12 labels, all 5×7×11 (mode,rule,choice) selections, factors −5..5
  compared as exact f64 bit patterns — against a separate Python
  transcription of the pinned C++. The transcription's f32 rounding was
  sanity-checked against the upstream header's own bit comments
  (0x1.2aaaaap+0, 0x1.555556p+0).
- Standalone `rustc --edition 2021 --crate-type lib -D warnings` is clean.

## Notes

- `preflight` will fail identically to #21/#22/#24/#25 and any PR here: the
  pinned action rev requires `target_sha256` in task records, which this
  no-target project's policy-18 records omit. Root cause and repro in
  issue #23; not bypassed.
