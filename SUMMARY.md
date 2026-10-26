# SUMMARY — port-0003 (Common/System/XferCRC.cpp)

Task: `port-0003` (issue #7), policy 18, toolchain `sha256:32a34a3e20be…`.
Sole code output: `recovery/crates/sage-core/src/port/xfer_crc.rs`.
(Per maintainer instruction this PR also carries `SUMMARY.md`, the only file
outside the declared output path.)

## What was ported

Upstream spec: `engine/src/Common/System/XferCRC.cpp`, `Common/XferCRC.h`,
`Common/StateHash.h` at pinned commit `f45ea04a575677eaaf497f307548b60853528d4e`.

- `StateHasher` starts at 0 and `addU32(v)` is `crc = rotl32(crc, 1) + v`
  mod 2^32 (RW 0xA211DF step).
- `XferCRC::xferImplementation`: whole little-endian 4-byte words first, then
  one rotate-add per trailing byte; word grouping restarts at every transfer.
- `beginBlock`/`endBlock`/`skipBlock` add nothing, so `xfer_crc_chunks` feeds
  `first` and `second` as two transfers into one hasher.
- `xfer_crc_words` applies `addU32` directly per element; that equals the
  little-endian byte path of the same words on the (little-endian) reference
  build, so no ambiguity.

No ambiguous upstream behaviour was encountered: no floats, no platform
formatting, no uninitialised reads. The module is `core`/`std` only, no
`unsafe`, no I/O, no clock/random/global mutable state; results are bit-exact
on every platform.

## Evidence

- `recomp task verify`: 8/8 case-group digests match (256 cases/function).
- Extra cross-check: 1500 randomized cases against an independent
  reimplementation of the pinned C++ semantics (byte lengths 0..200, two-chunk
  transfers 0..48 bytes each, 0..12 full-range u32 words): 1500/1500 identical.

## Process notes for the coordinator

- `recomp task submit` in this lane failed twice on host config, not task
  content: missing git identity (commit failed after it had already committed
  `47f746c`) and missing git push credentials (fixed with
  `gh auth setup-git`). After the second failure I completed submit's exact
  remaining steps by hand: pushed `recomp/port-0003-7` and opened PR #22 with
  the byte-identical body the CLI generates (one `Closes #7`, unchanged
  `Recomp-*` markers). No marker was hand-edited; the branch was not
  force-pushed or rewritten.
- `preflight` fails on this PR (and failed on merged #21) for a repo-wide
  defect: the pinned action rev `936de745…` requires `target_sha256` in task
  records, which policy-18 records of this no-target project omit. Root cause
  reproduced locally with the pinned action code. See issue #23; no bypass was
  attempted.
