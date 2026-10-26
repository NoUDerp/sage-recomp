//! Wave 44: holder flag-byte and dword accessors — 18 rows from the
//! `0x0046ce71`–`0x0046d000` run (module tag unassigned in the ledger).
//!
//! One mutable class carries a flag dword at `this + 0x10`, three flag bytes
//! at `this + 0x11/0x12/0x13`, plain dword cells at `this + 0x48/0x80/0x84`,
//! and a dword block starting at `this + 0x90` indexed by a runtime index
//! (`this + 0x90 + idx * 4`). As in `drawdata`/`fxstore`, every mirror takes
//! the *referenced cells themselves* (`&mut` / value); the caller resolves
//! field offsets and pointer hops.
//!
//! Row -> function map:
//!
//! | Row          | Function              | Game shape (field offsets)                                   |
//! |--------------|-----------------------|--------------------------------------------------------------|
//! | `0x0046ce71` | `set_indexed`         | `this + 0x90 + idx * 4 = value` (unchecked index)            |
//! | `0x0046ce87` | `float_at_indexed`    | `float` read at `this + 0x90 + idx * 4` (unchecked index)    |
//! | `0x0046ce9a` | `set_cell`            | `this + 0x84 = value`                                        |
//! | `0x0046ceb2` | `set_cell`            | `this + 0x80 = value`                                        |
//! | `0x0046cebf` | `set_cell`            | `this + 0x48 = value`                                        |
//! | `0x0046cecd` | — skipped             | guarded **vtable call** `(**(code **)(*this + 0x188))()`     |
//! | `0x0046ceee` | `bits_6000_set`       | `(this + 0x10 & 0x6000) == 0x6000`                           |
//! | `0x0046cf00` | `bit_clear_at` (13)   | `~(this + 0x10 >> 0xd) & 1`                                  |
//! | `0x0046cf0c` | `set_bit_if_zero`     | `v == 0` -> byte `this + 0x11 \|= 0x20`, else `&= 0xdf`      |
//! | `0x0046cf20` | `bit_clear_at` (14)   | `~(this + 0x10 >> 0xe) & 1`                                  |
//! | `0x0046cf2c` | `set_bit_if_zero`     | `v == 0` -> byte `this + 0x11 \|= 0x40`, else `&= 0xbf`      |
//! | `0x0046cf52` | `set_bit_if_nonzero`  | `this + 0x13`, mask `0x10`                                   |
//! | `0x0046cf6f` | `set_bit_if_nonzero`  | `this + 0x12`, mask `0x04`                                   |
//! | `0x0046cf8c` | `set_bit_if_nonzero`  | `this + 0x12`, mask `0x40`                                   |
//! | `0x0046cfa9` | `set_bit_if_nonzero`  | `this + 0x13`, mask `0x02`                                   |
//! | `0x0046cfc6` | `set_bit_if_nonzero`  | `this + 0x13`, mask `0x04`                                   |
//! | `0x0046cfe3` | `set_bit_if_nonzero`  | `this + 0x13`, mask `0x08`                                   |
//! | `0x0046d000` | `set_bit_if_nonzero`  | `this + 0x13`, mask `0x01`                                   |
//!
//! Quirks pinned by the game and mirrored exactly:
//! - The two byte-bit setters on `this + 0x11` (`0x0046cf0c`, `0x0046cf2c`)
//!   have **inverted polarity**: a zero value *sets* the bit and a non-zero
//!   value *clears* it; the seven later rows use the obvious polarity.
//! - The indexed block store/load has no bounds check (out-of-range writes
//!   and reads scribble memory = UB); the mirrors index the slice and panic.
//! - The block is written as raw dwords but read back as `f32` — same
//!   memory, two views, so the mirrors take `&mut [u32]` / `&[f32]`.
//! - `0x0046cecd` calls through the owner's vtable slot `+0x188` behind a
//!   `(this + 0x10 & 0x6000) == 0x6000` guard — not a pure accessor, skipped.

/// Plain dword store into the owner block (the value parameter is stored
/// verbatim, no read-modify-write).
///
/// Row: `0x0046ce9a` — field offset `+0x84`
/// Row: `0x0046ceb2` — field offset `+0x80`
/// Row: `0x0046cebf` — field offset `+0x48`
pub fn set_cell(cell: &mut u32, value: u32) {
    *cell = value;
}

/// Indexed dword store into the block the game places at `this + 0x90`
/// (address arithmetic `this + 0x90 + idx * 4`, no bounds check — an
/// out-of-range write is UB in the game; the mirror panics instead).
///
/// Row: `0x0046ce71` — block base `+0x90`
pub fn set_indexed(block: &mut [u32], index: u32, value: u32) {
    block[index as usize] = value;
}

/// Indexed read of the same `this + 0x90` block as an `f32` (the store rows
/// above write it as raw dwords; unchecked index, same UB as `set_indexed`).
///
/// Row: `0x0046ce87` — block base `+0x90`
pub fn float_at_indexed(block: &[f32], index: u32) -> f32 {
    block[index as usize]
}

/// Flag-dword test: do BOTH bits `0x4000|0x2000` read set?
///
/// Row: `0x0046ceee` — flag dword `+0x10`
pub fn bits_6000_set(flags: u32) -> bool {
    flags & 0x6000 == 0x6000
}

/// Flag-dword single-bit "is clear" test — the game computes
/// `~(flags >> bit) & 1`, i.e. the result is `1` exactly when the bit is
/// clear. The shift count is a constant per row (`0xd` / `0xe`).
///
/// Row: `0x0046cf00` — flag dword `+0x10`, bit `13`
/// Row: `0x0046cf20` — flag dword `+0x10`, bit `14`
pub fn bit_clear_at(flags: u32, bit: u32) -> bool {
    (flags >> bit) & 1 == 0
}

/// Byte-bit setter with **inverted** polarity: a zero value *sets* the mask
/// bit, a non-zero value clears it (mask leaves the other seven bits alone).
///
/// Row: `0x0046cf0c` — byte `+0x11`, mask `0x20`
/// Row: `0x0046cf2c` — byte `+0x11`, mask `0x40`
pub fn set_bit_if_zero(byte: &mut u8, value: u32, mask: u8) {
    if value == 0 {
        *byte |= mask;
    } else {
        *byte &= !mask;
    }
}

/// Byte-bit setter with the obvious polarity: non-zero sets the mask bit,
/// zero clears it.
///
/// Row: `0x0046cf52` — byte `+0x13`, mask `0x10`
/// Row: `0x0046cf6f` — byte `+0x12`, mask `0x04`
/// Row: `0x0046cf8c` — byte `+0x12`, mask `0x40`
/// Row: `0x0046cfa9` — byte `+0x13`, mask `0x02`
/// Row: `0x0046cfc6` — byte `+0x13`, mask `0x04`
/// Row: `0x0046cfe3` — byte `+0x13`, mask `0x08`
/// Row: `0x0046d000` — byte `+0x13`, mask `0x01`
pub fn set_bit_if_nonzero(byte: &mut u8, value: u32, mask: u8) {
    if value == 0 {
        *byte &= !mask;
    } else {
        *byte |= mask;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexed_store_and_float_read_hit_the_right_slot() {
        let mut block = [0u32; 4];
        set_indexed(&mut block, 2, 0x4148_0000);
        assert_eq!(block, [0, 0, 0x4148_0000, 0]);
        let floats = [1.5, -2.25, f32::from_bits(0x4148_0000), 0.0];
        assert_eq!(float_at_indexed(&floats, 2), 12.5);
    }

    #[test]
    #[should_panic]
    fn indexed_out_of_range_panics_instead_of_scribbling() {
        // Game UB: unchecked `this + 0x90 + idx * 4`; the read row has the
        // same shape.
        let mut block = [0u32; 4];
        set_indexed(&mut block, 4, 1);
    }

    #[test]
    fn flag_testers_read_the_dword() {
        assert!(bits_6000_set(0x6000));
        assert!(bits_6000_set(0x7fff));
        assert!(!bits_6000_set(0x4000));
        assert!(!bits_6000_set(0x2000));
        // `~(x >> bit) & 1`: true exactly when the bit is CLEAR.
        assert!(!bit_clear_at(0x2000, 13));
        assert!(bit_clear_at(0x1fff, 13));
        assert!(!bit_clear_at(0x4000, 14));
        assert!(bit_clear_at(0x3fff, 14));
    }

    #[test]
    fn inverted_setters_set_on_zero_value() {
        // 0x0046cf0c (+0x11, 0x20): v == 0 SETS, v != 0 clears.
        let mut b = 0x11;
        set_bit_if_zero(&mut b, 0, 0x20);
        assert_eq!(b, 0x31);
        set_bit_if_zero(&mut b, 1, 0x20);
        assert_eq!(b, 0x11);
        // 0x0046cf2c (+0x11, 0x40): other bits untouched.
        let mut c = 0x0f;
        set_bit_if_zero(&mut c, 7, 0x40);
        assert_eq!(c, 0x0f);
    }

    #[test]
    fn plain_setters_set_on_nonzero_value() {
        // 0x0046d000 (+0x13, 0x01) and 0x0046cf8c (+0x12, 0x40).
        let mut b = 0x80;
        set_bit_if_nonzero(&mut b, 1, 0x40);
        assert_eq!(b, 0xc0);
        set_bit_if_nonzero(&mut b, 0, 0x40);
        assert_eq!(b, 0x80);
        let mut d = 0b1010;
        set_bit_if_nonzero(&mut d, 0xffff_ffff, 0x01);
        assert_eq!(d, 0b1011);
    }
}
