//! Wave 44: statically-linked CRT multi-precision (bignum) primitives in the
//! `0x00a1b740–0x00a1c640` band — the carry/borrow chains behind the CRT math
//! library (long-double `pow`/`strtold` support; region neighbors the `printf`
//! core at `0x00a2b16a`, adjudicated `CRT-LIB`).
//!
//! Every row is a faithful mirror of the decompiled loop, including the game's
//! carry-chain bookkeeping: several routines thread the carry byte through the
//! low byte of a `u32` whose high 24 bits are a shift-out tail (`CONCAT31` in
//! the decompilation) and return the raw mixed value, not a clean carry. The
//! tests pin those exact return bit patterns.
//!
//! Rows → functions (all module `CRT-LIB`):
//!
//! | row | fn | semantics |
//! |---|---|---|
//! | `0x00a1b740` | `elem_before_is_neg` | `arr[idx-1] < 0` (heap-parent style test) |
//! | `0x00a1b9a0` | `sign_extend_copy_backward` | reversed `src` copy + sign fill of the rest |
//! | `0x00a1bd10` | `shl_one` | shift-left-by-1 over words with carry-in byte chain |
//! | `0x00a1bd50` | `bit_not_n` | two's-complement `~` over first `count` words |
//! | `0x00a1bd70` | `set_word_zero_rest` | zero `count` words then `words[0] = v` |
//! | `0x00a1bd90` | `highest_bit_field` | last non-zero word → 1-based MSB position `(i+1)·32 − lz` |
//! | `0x00a1be30` | `compare_words_desc` | unsigned word compare from high word → −1/0/1 |
//! | `0x00a1be70` | `add_carry_words` | word add with carry-in byte chain |
//! | `0x00a1bed0` | `add_carry_scalar` | scalar add (first word only) with carry chain |
//! | `0x00a1bf30` | `sub_short_borrow` | 16-bit array subtract with borrow (2·N iters) |
//! | `0x00a1bf80` | `sub_short_scalar_borrow` | 16-bit scalar subtract with borrow |
//! | `0x00a1bfd0` | `mul_short_inline` | in-place 16-bit multiply-accumulate by scalar |
//! | `0x00a1c0c0` | `mul_short_acc` | `dst += a * scalar` (16-bit) with final carry add |
//! | `0x00a1c120` | `mul_short_matrix` | schoolbook 16-bit multiply over dword `b` |
//! | `0x00a1c2b0` | `reverse_bytes` | `std::reverse` over a byte range |
//! | `0x00a1c2e0` | `compare_int` | three-way `int` compare (−1/0/1) |
//! | `0x00a1c5e0` | `load_length_record` | type-2 length-prefixed record loader |
//! | `0x00a1c640` | `negate` | `~x + 1` over `count` words (carry-propagated) |
//!
//! Not mirrored (documented, deliberately): `0x00a1b7d0` and `0x00a1ba50` are
//! side-effect-free scans (no observable behavior — mirroring would be pure
//! tautology); `0x00a1c220`, `0x00a1c300`, `0x00a1c360` are CRT-internal math
//! constant-table consumers (`DAT_00e095xx`, bsearch table `DAT_00dd6350` —
//! CRT data substrate decision, deferred).

/// `*(int *)(this + -4 + idx*4) < 0` — test of the word *before* `idx`.
/// `idx = 0` under-reads in the image (UB); the mirror's `idx - 1` panics.
///
/// Row: `0x00a1b740`.
pub fn elem_before_is_neg(arr: &[i32], idx: usize) -> bool {
    arr[idx - 1] < 0
}

/// Fill the top `words*4 − src.len()` bytes with the sign byte of `src[0]`
/// (`0xFF` if `src[0] & 0x80` else `0`), then write `src` **reversed** into the
/// low `src.len()` bytes. The game writes both phases bottom-ward; with distinct
/// buffers only the final content is observable.
///
/// Row: `0x00a1b9a0` (called from `0x00a1c5e0`).
pub fn sign_extend_copy_backward(dst: &mut [u8], src: &[u8], words: usize) {
    let total = words * 4;
    let sign = if src[0] & 0x80 != 0 { 0xff } else { 0 };
    if total > src.len() {
        for i in src.len()..total {
            dst[i] = sign;
        }
    }
    for (j, b) in src.iter().enumerate() {
        dst[src.len() - 1 - j] = *b;
    }
}

/// Carry-byte bookkeeping identical to the game: the carry lives in the low
/// byte of a running `u32` whose high 24 bits are the incoming `(count >> 8)`
/// tail shifted per iteration; the raw value is returned (`0` count returns the
/// initial argument unchanged). `(char)carry != 0` gates the +1.
///
/// Row: `0x00a1bd10`.
pub fn shl_one(words: &mut [u32], count: u32, carry_in: u32) -> u32 {
    if count == 0 {
        return carry_in;
    }
    let mut c: u32 = (((count >> 8) << 8) & 0xffff_ffff) | (carry_in & 0xff);
    let mut i = 0usize;
    let mut n = count;
    while n != 0 {
        let w = words[i];
        let s = w.wrapping_mul(2);
        words[i] = s;
        if (c as u8) != 0 {
            words[i] = s.wrapping_add(1);
        }
        i += 1;
        n -= 1;
        c = ((c >> 8) << 8) | ((w >> 31) & 1);
    }
    c
}

/// Two's-complement ones': `~words[i]` for `0 < count` (signed guard: `count <=
/// 0` does nothing).
///
/// Row: `0x00a1bd50`.
pub fn bit_not_n(words: &mut [u32], count: i32) {
    if count > 0 {
        for i in 0..count as usize {
            words[i] = !words[i];
        }
    }
}

/// Zero `count` words, then `words[0] = value` (the trailing store lands after
/// the zero-fill in the game; order preserved).
///
/// Row: `0x00a1bd70`.
pub fn set_word_zero_rest(words: &mut [u32], value: u32, count: u32) {
    let mut p = 0usize;
    let mut n = count;
    while n != 0 {
        words[p] = 0;
        p += 1;
        n -= 1;
    }
    words[0] = value;
}

/// Scan words backwards from `count − 1` for the last non-zero word `w` at index
/// `i`, then return the **1-based** most-significant-set-bit position
/// `(i + 1) * 32 − leading_zeros(w)` (the game shifts `param_2 = (i+1)` left by
/// 5 and counts down per leading zero); `0` if all words are zero.
///
/// Row: `0x00a1bd90`.
pub fn highest_bit_field(words: &[u32], count: usize) -> i32 {
    let mut i = count;
    while i != 0 {
        i -= 1;
        let w = words[i];
        if w != 0 {
            let mut pos = ((i + 1) as i32) << 5;
            if (w as i32) >= 0 {
                let mut mask = 0x8000_0000u32;
                while mask & w == 0 {
                    mask >>= 1;
                    pos -= 1;
                }
            }
            return pos;
        }
    }
    0
}

/// Unsigned three-way compare of two word arrays taken from the HIGH word
/// (`count − 1`) down: `−1` / `0` / `1`.
///
/// Row: `0x00a1be30`.
pub fn compare_words_desc(a: &[u32], b: &[u32], count: usize) -> i32 {
    let mut i = count;
    while i != 0 {
        i -= 1;
        let (x, y) = (a[i], b[i]);
        if x < y {
            return -1;
        }
        if y < x {
            return 1;
        }
    }
    0
}

/// `dst[i] = a[i] + b[i] + carry` with the game's exact carry predicate:
/// `(sum_no_carry < a)` OR `(carry != 0 && total == 0)`. The running `u32`
/// carry byte + `(count >> 8)` tail is returned raw (see `shl_one`).
///
/// Row: `0x00a1be70`.
pub fn add_carry_words(dst: &mut [u32], a: &[u32], b: &[u32], count: u32, carry_in: u32) -> u32 {
    if count == 0 {
        return carry_in;
    }
    let mut c: u32 = (((count >> 8) << 8) & 0xffff_ffff) | (carry_in & 0xff);
    let mut i = 0usize;
    let mut n = count;
    while n != 0 {
        let bv = a[i];
        let sum0 = bv.wrapping_add(b[i]);
        let total = (c & 0xff).wrapping_add(sum0);
        let hi = (c >> 8) << 8;
        let carry_out = (sum0 < bv) || ((c as u8) != 0 && total == 0);
        c = if carry_out { hi | 1 } else { hi };
        dst[i] = total;
        i += 1;
        n -= 1;
    }
    c
}

/// Same chain, but the addend is a scalar folded into the FIRST word only
/// (the game zeroes `param_3` inside the loop after the first iteration).
///
/// Row: `0x00a1bed0`.
pub fn add_carry_scalar(dst: &mut [u32], a: &[u32], scalar: u32, count: u32, carry_in: u32) -> u32 {
    if count == 0 {
        return carry_in;
    }
    let mut c: u32 = (((count >> 8) << 8) & 0xffff_ffff) | (carry_in & 0xff);
    let mut i = 0usize;
    let mut n = count;
    let mut s = scalar;
    while n != 0 {
        let av = a[i];
        let sum0 = av.wrapping_add(s);
        let total = (c & 0xff).wrapping_add(sum0);
        let hi = (c >> 8) << 8;
        let carry_out = (sum0 < av) || ((c as u8) != 0 && total == 0);
        c = if carry_out { hi | 1 } else { hi };
        dst[i] = total;
        i += 1;
        n -= 1;
        s = 0;
    }
    c
}

/// 16-bit array subtract with borrow. Iteration count is the game's
/// `param_5 * 2` (skipped when it wraps to `0`); borrow-out is bit 16 of the
/// 32-bit difference. `dst` is the game's `this` array (in-place allowed).
///
/// Row: `0x00a1bf30`.
pub fn sub_short_borrow(dst: &mut [u16], a: &[u16], b: &[u16], param_5: u32, borrow_in: u32) {
    let iters = param_5.wrapping_mul(2);
    if iters == 0 {
        return;
    }
    let mut borrow = borrow_in;
    for i in 0..iters as usize {
        let d = (a[i] as u32).wrapping_sub(b[i] as u32).wrapping_sub(borrow);
        dst[i] = d as u16;
        borrow = (d >> 16) as u32 & 1;
    }
}

/// 16-bit scalar subtract with borrow: the scalar (`param_3 & 0xffff`) is used
/// only in the first iteration, then zero. Iterations again `param_5 * 2`.
///
/// Row: `0x00a1bf80`.
pub fn sub_short_scalar_borrow(dst: &mut [u16], a: &[u16], scalar: u16, param_5: u32, borrow_in: u32) {
    let iters = param_5.wrapping_mul(2);
    if iters == 0 {
        return;
    }
    let mut borrow = borrow_in;
    let mut s = scalar as u32;
    for i in 0..iters as usize {
        let d = (a[i] as u32).wrapping_sub(s).wrapping_sub(borrow);
        dst[i] = d as u16;
        borrow = (d >> 16) as u32 & 1;
        s = 0;
    }
}

/// In-place 16-bit multiply-accumulate by a scalar over `param_4 * 2` words:
/// `acc = a[i] * scalar + acc; a[i] = acc; acc >>= 16`. Returns nothing (the
/// game's `return 0` is dead for a void ABI).
///
/// Row: `0x00a1bfd0`.
pub fn mul_short_inline(a: &mut [u16], scalar: i32, param_4: i32) {
    let iters = param_4.wrapping_mul(2);
    let mut acc: u32 = 0;
    if iters > 0 {
        for i in 0..iters as usize {
            acc = (a[i] as u32).wrapping_mul(scalar as u32).wrapping_add(acc);
            a[i] = acc as u16;
            acc >>= 16;
        }
    }
}

/// `dst[i] += a[i] * scalar` (16-bit), carrying the top bits upward; after the
/// loop the final carry word is **added** to `dst[n]` (one past — the game
/// reads/writes the word after the range; the mirror indexes it, so `dst` must
/// cover `n + 1` — same UB envelope). `n < 1` is a no-op (the game's
/// self-assignment).
///
/// Row: `0x00a1c0c0`.
pub fn mul_short_acc(dst: &mut [u16], a: &[u16], scalar: u16, n: i32) {
    if n < 1 {
        let _ = dst[0];
        return;
    }
    let mut carry: u32 = 0;
    let mut last: u32 = 0;
    for i in 0..n as usize {
        last = (a[i] as u32).wrapping_mul(scalar as u32).wrapping_add(dst[i] as u32).wrapping_add(carry);
        dst[i] = last as u16;
        carry = last >> 16;
    }
    dst[n as usize] = dst[n as usize].wrapping_add((last >> 16) as u16);
}

/// Schoolbook 16-bit multiply: zero `2n` dwords (`4n` u16 words — the game
/// writes `2n` dwords), then for each of `n` **dword** entries of `b` (low 16
/// bits used) run `mul_short_acc` at word offset `2j` (game byte offset `4j`)
/// over `n` words. The tail-add at word `2j + n` stays inside the zeroed
/// region. Non-positive `n` mirrors the game's nonsensical wrap (panics).
///
/// Row: `0x00a1c120` (composes `0x00a1c0c0`).
pub fn mul_short_matrix(dst: &mut [u16], a: &[u16], b_bytes: &[u8], n: i32) {
    let zwords = (n.wrapping_mul(2) as usize).wrapping_mul(2);
    for i in 0..zwords {
        dst[i] = 0;
    }
    if n > 0 {
        let mut off = 0usize;
        for j in 0..n as usize {
            let scalar = u16::from_le_bytes([b_bytes[off], b_bytes[off + 1]]);
            mul_short_acc(&mut dst[2 * j..], a, scalar, n);
            off += 4;
        }
    }
}

/// `std::reverse` over `n` bytes; empty/single ranges untouched (`first < last`
/// guard taken from pointers, faithful).
///
/// Row: `0x00a1c2b0`.
pub fn reverse_bytes(buf: &mut [u8]) {
    if buf.len() < 2 {
        return;
    }
    let mut lo = 0usize;
    let mut hi = buf.len() - 1;
    while lo < hi {
        let t = buf[lo];
        buf[lo] = buf[hi];
        buf[hi] = t;
        lo += 1;
        hi -= 1;
    }
}

/// Three-way signed `int` compare: `−1` if less, else `1` if greater else `0`.
/// (Used as the bsearch comparator by `0x00a1c300`.)
///
/// Row: `0x00a1c2e0`.
pub fn compare_int(a: &i32, b: &i32) -> i32 {
    if *a < *b {
        -1
    } else if *b < *a {
        1
    } else {
        0
    }
}

/// Type-`2` length-prefixed record loader over `record`:
/// `record[0]` must be `2`, else nothing happens. Length = `record[1]`; if its
/// high bit is set: marker = `len & 0x7f`, and `marker > 2` aborts; the length
/// is the next byte, except when `marker != 1 && (record[1] & 0x7f) != 0` the
/// length is the big-endian pair `record[2]:record[3]` (Ghidra `CONCAT11`).
/// Payload (`len` bytes from the following offset) goes through
/// `sign_extend_copy_backward` when `len <= words * 4`.
///
/// Row: `0x00a1c5e0` (composes `0x00a1b9a0`).
pub fn load_length_record(record: &[u8], dst: &mut [u8], words: usize) {
    if record[0] != 2 {
        return;
    }
    let b1 = record[1];
    let mut len = b1 as usize;
    let mut off = 2usize;
    if (b1 as i8) < 0 {
        let marker = len & 0x7f;
        if marker > 2 {
            return;
        }
        len = record[2] as usize;
        off = 3;
        if marker != 1 && (b1 & 0x7f) != 0 {
            len = ((record[2] as u16) << 8 | record[3] as u16) as usize;
            off = 4;
        }
    }
    if len <= words * 4 {
        sign_extend_copy_backward(dst, &record[off..off + len], words);
    }
}

/// Full two's-complement negate: `~` over `count` words (signed guard) then a
/// 1-with-carry ripple. Quirk pinned: with `count == 0` the `~` phase is
/// skipped but the ripple still increments `words[0]` exactly once.
///
/// Row: `0x00a1c640`.
pub fn negate(words: &mut [u32], count: i32) {
    if count > 0 {
        for i in 0..count as usize {
            words[i] = !words[i];
        }
    }
    let mut i = 0usize;
    let mut n = count;
    loop {
        words[i] = words[i].wrapping_add(1);
        if words[i] != 0 {
            break;
        }
        i += 1;
        n -= 1;
        if n == 0 {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shl_one_carry_chain() {
        let mut w = [1u32, 0x8000_0000, 0];
        // carry-in bit lands in the low word; MSB of word1 carries into word2
        let r = shl_one(&mut w, 2, 1);
        assert_eq!(w, [3, 0, 0]); // carry out of word 1 is NOT applied (count = 2)
        assert_eq!(r, 1); // raw return: low byte = last MSB out, tail (count>>8)<<8 = 0
        // explicit carry-in zero path
        let mut w2 = [0x8000_0000];
        assert_eq!(shl_one(&mut w2, 1, 0), 1); // carry out 1 in low byte
        assert_eq!(w2, [0]);
    }

    #[test]
    fn add_carry_words_chain_and_raw_return() {
        let mut dst = [0u32; 2];
        let r = add_carry_words(
            &mut dst,
            &[0xffff_ffff, 0xffff_ffff],
            &[1, 0],
            2,
            0,
        );
        assert_eq!(dst, [0, 0]); // word 1: 1 + 0xffff_ffff wraps to 0, carry still out
        assert_eq!(r & 0xff, 1); // carry out of word 0 -> word 1 got 1+0xffff_ffff
        // raw return keeps the (count>>8)<<8 tail
        assert_eq!(r, 1);
        // count == 0: returns the initial argument untouched
        assert_eq!(add_carry_words(&mut [], &[], &[], 0, 7), 7);
    }

    #[test]
    fn negate_roundtrip_and_zero_count_quirk() {
        let mut w = [0u32, 0, 1]; // 2^64 as three words
        negate(&mut w, 3);
        assert_eq!(w, [0, 0, 0xffff_ffff]); // -2^64
        negate(&mut w, 3);
        assert_eq!(w, [0, 0, 1]);
        // count == 0: no ~, but words[0] still +1 exactly once
        let mut w2 = [5u32, 9];
        negate(&mut w2, 0);
        assert_eq!(w2, [6, 9]);
    }

    #[test]
    fn highest_bit_field_matches_lz_formula() {
        assert_eq!(highest_bit_field(&[0, 0x0000_0001], 2), 33); // (1+1)*32 − 31
        assert_eq!(highest_bit_field(&[0x8000_0000], 1), 32); // top bit of word 0
        assert_eq!(highest_bit_field(&[0, 0x4000_0000], 2), 63);
        assert_eq!(highest_bit_field(&[0, 0], 2), 0);
    }

    #[test]
    fn compare_paths() {
        assert_eq!(compare_words_desc(&[0, 1], &[0, 2], 2), -1);
        assert_eq!(compare_words_desc(&[5, 2], &[9, 2], 2), -1); // equal at high word, then 5 < 9
        assert_eq!(compare_words_desc(&[9, 2], &[5, 2], 2), 1);
        assert_eq!(compare_words_desc(&[5, 2], &[5, 2], 2), 0);
        assert_eq!(compare_int(&-5, &3), -1);
        assert_eq!(compare_int(&3, &-5), 1);
        assert_eq!(compare_int(&3, &3), 0);
    }

    #[test]
    fn sign_extend_reversed_copy() {
        let mut dst = [0u8; 8]; // words = 2 -> 8 bytes
        sign_extend_copy_backward(&mut dst, &[0x81, 0x02, 0x03], 2);
        // src reversed into low 3 bytes: [0x03,0x02,0x81], top 5 bytes sign=0xff
        assert_eq!(dst, [0x03, 0x02, 0x81, 0xff, 0xff, 0xff, 0xff, 0xff]);
        let mut dst2 = [0u8; 8];
        sign_extend_copy_backward(&mut dst2, &[0x7f], 2);
        assert_eq!(&dst2[1..], &[0; 7]);
    }

    #[test]
    fn length_record_decode_variants() {
        // short form: len=2
        let mut dst = [0u8; 4];
        load_length_record(&[2, 2, 0x81, 0x00], &mut dst, 1);
        assert_eq!(dst, [0x00, 0x81, 0xff, 0xff]); // reversed copy + 0xff sign fill
        // long form marker=1: single length byte (0x81 has bit7, marker 1)
        let mut d2 = [0u8; 8];
        load_length_record(&[2, 0x81, 0x01, 0x40, 0x00, 0x00, 0x00, 0x00], &mut d2, 2);
        assert_eq!(d2[0], 0x40);
        // marker=2: big-endian pair length 0x0140 -> 320 > 8 words? 320 > 8 -> skip
        let d3 = [0u8; 8];
        let before = d3;
        let mut d3 = before;
        load_length_record(&[2, 0x82, 0x01, 0x40, 0x00, 0x00, 0x00, 0x00], &mut d3, 2);
        assert_eq!(d3, [0; 8]);
        // marker > 2 aborts
        let mut d4 = [0u8; 8];
        load_length_record(&[2, 0x83, 0x01, 0x40, 0x00, 0x00, 0x00, 0x00], &mut d4, 2);
        assert_eq!(d4, [0; 8]);
    }

    #[test]
    fn sub_short_borrow_chains() {
        let a = [0x0000u16, 0x0001, 0x0002, 0x0003];
        let b = [0x0001u16, 0x0000, 0x0000, 0x0000];
        let mut dst = [0u16; 4];
        sub_short_borrow(&mut dst, &a, &b, 2, 0); // param_5=2 -> 4 iters
        assert_eq!(dst, [0xffff, 0x0000, 0x0002, 0x0003]);
        sub_short_borrow(&mut dst, &a, &b, 0, 1); // wraps*2=0: untouched
        assert_eq!(dst, [0xffff, 0x0000, 0x0002, 0x0003]);
    }

    #[test]
    fn mul_short_acc_tail_carry_add() {
        let mut dst = [0xffffu16, 0x0000, 0x0000];
        let a = [0xffffu16, 0x0000];
        mul_short_acc(&mut dst, &a, 2, 2);
        // 0xffff*2 + 0xffff = 0x2fffd -> word0=0xfffd carry 2; 0*2 + 0 + 2 = 2
        assert_eq!(dst, [0xfffd, 2, 0]);
    }

    #[test]
    fn mul_short_matrix_small_fingerprint() {
        // a = [1,2] (n=2), b dword0 = 3, dword1 = 0
        let mut dst = [0xffffu16; 8]; // 4n = 8 words get zeroed
        let a = [1u16, 2];
        let b = [3u8, 0, 0, 0, 0, 0, 0, 0];
        mul_short_matrix(&mut dst, &a, &b, 2);
        assert_eq!(dst, [3, 6, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn reverse_and_scalar_paths() {
        let mut buf = [1u8, 2, 3, 4];
        reverse_bytes(&mut buf);
        assert_eq!(buf, [4, 3, 2, 1]);
        let mut w = [1u32; 3];
        set_word_zero_rest(&mut w, 7, 3);
        assert_eq!(w, [7, 0, 0]);
        let mut arr = [5u16, 6];
        mul_short_inline(&mut arr, 3, 1); // iters = 2
        assert_eq!(arr, [15, (1 * 6 + 0) * 3]); // acc chain: word0 5*3=15 acc=0; word1 6*3=18
        let mut d2 = [0u16; 3];
        sub_short_scalar_borrow(&mut d2, &[0, 0, 0], 1, 1, 0); // 2 iters
        assert_eq!(d2, [0xffff, 0xffff & !0, 0]); // second iter: 0-0-1 = -1
    }

    #[test]
    fn elem_before_neg() {
        assert!(elem_before_is_neg(&[-3, 7], 1));
        assert!(!elem_before_is_neg(&[3, 7], 1));
    }

    #[test]
    fn bit_not_signed_guard() {
        let mut w = [0u32, 1];
        bit_not_n(&mut w, 0); // no-op
        assert_eq!(w, [0, 1]);
        bit_not_n(&mut w, -5); // no-op
        assert_eq!(w, [0, 1]);
        bit_not_n(&mut w, 1);
        assert_eq!(w, [!0, 1]);
    }
}
