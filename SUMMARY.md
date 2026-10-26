# SUMMARY — port-0014 (Common/CommandPoints.cpp)

Task: `port-0014` (issue #18), policy 18, toolchain `sha256:32a34a3e20be…`.
Code output: `recovery/crates/sage-core/src/port/command_points.rs`
(`SUMMARY.md` is included per maintainer instruction and is the only other
file in the branch).

## What was ported

Upstream spec: `engine/src/Common/CommandPoints.cpp` and
`Common/CommandPoints.h` at pinned commit
`f45ea04a575677eaaf497f307548b60853528d4e`.

- All arithmetic follows `CpMath`: 32-bit wrapping `add`/`imul` on two's
  complement bits with **signed** compares afterwards (the binary's
  `cmovg`/`jle`), i.e. Rust `wrapping_add` / `wrapping_mul` + signed `>`/`<=`.
- `cp_limit` = `getLimit` (RW 0x6A7B9F) for records whose filter handle is
  unset: `!ObjectFilterMatch::isValid(null)` is true, so **every** record
  counts; total = `bonus + base` then each record value (u32 bit pattern
  reinterpreted as the signed `int value`), then `total > cap ? cap : total`.
- `cp_can_afford` = `canAfford` (RW 0x6A7F79): zero template cost → true;
  else signed compare of the wrapped `usage + templateCommandPoints` against
  the limit; `armyOfDead` (KindOf bit 151) forces true.
- `cp_init_cap` = cap half of `init` (RW 0x6A7C86 .. 0x6A7E07): store source
  cap unless a script set one and it is larger; then lobby percent applies
  `imul` (wrapping) followed by `cdq; idiv 100` — truncation toward zero,
  which matches Rust's `/` on `i32`.
- `cp_init_base` = base half of `init`: base = `start + territoryGoodTerm +
  territoryEvilTerm` (RW 0x6A7ECA products pre-summed by the caller), stored
  when no script set one or when it is larger.

No ambiguous upstream behaviour: integers only, defined record order, no
floats (`COMMAND_POINT_BONUS` attribute-modifier floats are upstream stop
S-253 and are not part of these entries). Module is `core`/`std` only, no
`unsafe`, no I/O, no clock/random/global mutable state.

## Evidence

- `recomp task verify`: 8/8 case-group digests (256 cases/function), `pass: true`.
- Independent cross-check: 1600 randomized/edge cases against a separate
  Python transcription of the pinned C++ (full i32 range, INT_MIN/INT_MAX/0
  edges, 0..6 record values, signed wrap comparisons, negative lobby
  products): 1600/1600 identical. No golden contents were read.

## Notes

- `preflight` will fail identically to #21/#22 and any PR here: the pinned
  action rev requires `target_sha256` in task records, which this no-target
  project's policy-18 records omit. Root cause and repro in issue #23; not
  bypassed.
- The task-record arg kinds are followed: `cp_limit` takes the record values
  as the declared `&[u32]` (bit-reinterpreted as the C++ `int value`).
