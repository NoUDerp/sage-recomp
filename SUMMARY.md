# SUMMARY — port-0002 (Common/Sha256.cpp)

Task: `port-0002` (issue #6), policy 18, toolchain `sha256:32a34a3e20be…`.
Code output: `recovery/crates/sage-core/src/port/sha256.rs`
(`SUMMARY.md` is included per maintainer instruction and is the only other
file in the branch).

## What was ported

Upstream spec: `engine/src/Common/Sha256.cpp` and `Common/Sha256.h` at
pinned commit `f45ea04a575677eaaf497f307548b60853528d4e`.

- `Sha256` state: 8-word chaining value, 64-byte block buffer, byte count.
  `update` accumulates the 64-bit bit length **before** `finish()` captures
  it (upstream line 89), then feeds `0x80`, zeros until 56 bytes are staged,
  and the big-endian 64-bit length — so when 56..63 bytes are staged, the
  padding crosses into a fresh block. Rust `finalize` uses a length-blind
  `absorb` for the padding, which reproduces `finish()` exactly while
  keeping the bit counter data-only.
- `compress` (`block`): big-endian message schedule, `σ0/σ1` expansion,
  standard rounds, wrapping adds throughout. The `kK` table is transcribed
  byte-for-byte from the pinned file (it is the FIPS 180-4 table; each of
  the 64 words diffed against upstream).
- `hex_lower`: lowercase hex, matching upstream's digest formatting helpers.
- `sha256_hex` = `Sha256::hash` one-shot; `sha256_stream_hex` = `update`
  twice then `finish` (exercises buffer carry across calls);
  `hmac_sha256_hex` = upstream `HmacSha256` exactly: key > 64 → hashed to
  32 bytes first, key < 64 → zero-padded (empty key → all-zero block key,
  no special case), then `ipad`/`0x36`, `opad`/`0x5c`, inner/outer.

No ambiguous upstream behaviour: integers and bytes only, no floats, no
timestamps, no locale. Module is `core`/`std` only, no `unsafe`, no I/O,
no clock/random/global mutable state.

## Evidence

- `recomp task verify`: 8/8 case-group digests (256 cases/function),
  `pass: true`. No golden contents read, no expected values hard-coded.
- Independent cross-check: 1025 cases against a separate Python
  transcription of the pinned C++ — single-shot lengths
  0,1,55,56,57,63,64,65,111,112,119,120,127,128,129,199,200 + random,
  two-chunk stream splits (70/70 max), HMAC keys of length 0,1,20,32,63,64,
  65,96 + random to 96: 1025/1025 identical. The transcription itself was
  validated equal to Python `hashlib`/`hmac` and the official FIPS/RFC 2104
  vectors, which independently confirms the pinned `kK` table is the
  standard one.

## Notes

- `preflight` will fail identically to #21/#22/#24 and any PR here: the
  pinned action rev requires `target_sha256` in task records, which this
  no-target project's policy-18 records omit. Root cause and repro in
  issue #23; not bypassed.
- Two constant-transcription typos caught by the cross-check diff (K[11],
  K[38]) are a reminder to diff constant tables against the pinned file
  rather than memory; the committed table is a byte-for-byte match.
