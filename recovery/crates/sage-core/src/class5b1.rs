//! Wave 44: large-owner-class dword/vec/flag setters (`module NULL`, band
//! `0x005b1`). All 25 rows are `__thiscall` accessors of one big class whose
//! fields span `this + 0x124 ..= 0x220`. The field mix — scattered dword
//! config cells, two vec3 stores, a pair store, a bit-flag dword (`0x208`)
//! with four single-bit set/clear rows, an adjacent byte cell (`0x20b`), and
//! a row that ORs a caller value into the flag dword's top byte — reads like
//! a per-instance FX settings/template block (band tag evidence is FX).
//!
//! Uniform shape: plain stores through `this`; no dirty flag, no guard, no
//! return value. As in `drawdata`/`fxflags`, the caller (owner struct)
//! resolves each `this + OFF` and hands the referenced cell in by mutable
//! reference; the vec/pair stores take the game's source pointer as a
//! fixed-size slice (`*param_2`, `param_2[1]`, … read in place).
//!
//! Field-offset map per row:
//! - single dword into `this`: `0x005b1460` (+0x1c0), `0x005b1470` (+0x1c4),
//!   `0x005b1480` (+0x124), `0x005b14a0` (+0x128), `0x005b14c0` (+0x12c,
//!   decompiler prints `300`), `0x005b14e0` (+0x138), `0x005b1500` (+0x13c),
//!   `0x005b1520` (+0x140), `0x005b15a0` (+0x164), `0x005b15b0` (+0x1a8),
//!   `0x005b15d0` (+0x1ac), `0x005b15f0` (+0x1e0), `0x005b16c0` (+0x20c),
//!   `0x005b16d0` (+0x210), `0x005b16f0` (+0x214), `0x005b1710` (+0x218)
//! - three dwords from the source pointer into consecutive cells:
//!   `0x005b1540` (+0x144/0x148/0x14c), `0x005b1570` (+0x150/0x154/0x158)
//! - two dwords from the source pointer: `0x005b1730` (+0x21c/0x220)
//! - set/clear one bit of the flag dword at `this + 0x208`:
//!   `0x005b1640` (bit 0), `0x005b1660` (bit 1), `0x005b1680` (bit 2),
//!   `0x005b16a0` (bit 3)
//! - byte zero at `this + 0x20b` then `flags |= value << 24`: `0x005b1750`
//! - skipped (call chain): `0x005b1610` — body is
//!   `FUN_005a45e3(param_2, this + 0x1b0)`, and `FUN_005a45e3` itself calls
//!   `FUN_005a441f` before unpacking a dword into a byte bitmap at
//!   `this + 0x1b0`; not a simple accessor, so no mirror here.

/// `this + 0x1c0 = v`
///
/// Row: `0x005b1460`.
pub fn set_1c0(cell: &mut u32, v: u32) {
    *cell = v;
}

/// `this + 0x1c4 = v`
///
/// Row: `0x005b1470`.
pub fn set_1c4(cell: &mut u32, v: u32) {
    *cell = v;
}

/// `this + 0x124 = v`
///
/// Row: `0x005b1480`.
pub fn set_124(cell: &mut u32, v: u32) {
    *cell = v;
}

/// `this + 0x128 = v`
///
/// Row: `0x005b14a0`.
pub fn set_128(cell: &mut u32, v: u32) {
    *cell = v;
}

/// `this + 0x12c = v` (the decompiler prints the offset as decimal `300`).
///
/// Row: `0x005b14c0`.
pub fn set_12c(cell: &mut u32, v: u32) {
    *cell = v;
}

/// `this + 0x138 = v`
///
/// Row: `0x005b14e0`.
pub fn set_138(cell: &mut u32, v: u32) {
    *cell = v;
}

/// `this + 0x13c = v`
///
/// Row: `0x005b1500`.
pub fn set_13c(cell: &mut u32, v: u32) {
    *cell = v;
}

/// `this + 0x140 = v`
///
/// Row: `0x005b1520`.
pub fn set_140(cell: &mut u32, v: u32) {
    *cell = v;
}

/// `this + 0x144 = *src; this + 0x148 = src[1]; this + 0x14c = src[2]` —
/// the game copies three dwords straight from the source pointer.
///
/// Row: `0x005b1540`.
pub fn set_144_vec3(a: &mut u32, b: &mut u32, c: &mut u32, src: &[u32; 3]) {
    *a = src[0];
    *b = src[1];
    *c = src[2];
}

/// `this + 0x150 = *src; this + 0x154 = src[1]; this + 0x158 = src[2]` —
/// the second three-dword source-pointer copy.
///
/// Row: `0x005b1570`.
pub fn set_150_vec3(a: &mut u32, b: &mut u32, c: &mut u32, src: &[u32; 3]) {
    *a = src[0];
    *b = src[1];
    *c = src[2];
}

/// `this + 0x164 = v`
///
/// Row: `0x005b15a0`.
pub fn set_164(cell: &mut u32, v: u32) {
    *cell = v;
}

/// `this + 0x1a8 = v`
///
/// Row: `0x005b15b0`.
pub fn set_1a8(cell: &mut u32, v: u32) {
    *cell = v;
}

/// `this + 0x1ac = v`
///
/// Row: `0x005b15d0`.
pub fn set_1ac(cell: &mut u32, v: u32) {
    *cell = v;
}

/// `this + 0x1e0 = v`
///
/// Row: `0x005b15f0`.
pub fn set_1e0(cell: &mut u32, v: u32) {
    *cell = v;
}

/// Set/clear bit 0 of the flag dword at `this + 0x208`: nonzero arg → `|= 1`,
/// zero arg → `&= 0xfffffffe`. The game tests `param_2 == 0` on a full int,
/// so any nonzero value sets.
///
/// Row: `0x005b1640`.
pub fn set_bit0(flags: &mut u32, on: bool) {
    if on {
        *flags |= 1;
    } else {
        *flags &= 0xffff_fffe;
    }
}

/// Set/clear bit 1 of the flag dword at `this + 0x208` (`|= 2` / `&=
/// 0xfffffffd`).
///
/// Row: `0x005b1660`.
pub fn set_bit1(flags: &mut u32, on: bool) {
    if on {
        *flags |= 2;
    } else {
        *flags &= 0xffff_fffd;
    }
}

/// Set/clear bit 2 of the flag dword at `this + 0x208` (`|= 4` / `&=
/// 0xfffffffb`).
///
/// Row: `0x005b1680`.
pub fn set_bit2(flags: &mut u32, on: bool) {
    if on {
        *flags |= 4;
    } else {
        *flags &= 0xffff_fffb;
    }
}

/// Set/clear bit 3 of the flag dword at `this + 0x208` (`|= 8` / `&=
/// 0xfffffff7`).
///
/// Row: `0x005b16a0`.
pub fn set_bit3(flags: &mut u32, on: bool) {
    if on {
        *flags |= 8;
    } else {
        *flags &= 0xffff_fff7;
    }
}

/// `this + 0x20c = v`
///
/// Row: `0x005b16c0`.
pub fn set_20c(cell: &mut u32, v: u32) {
    *cell = v;
}

/// `this + 0x210 = v`
///
/// Row: `0x005b16d0`.
pub fn set_210(cell: &mut u32, v: u32) {
    *cell = v;
}

/// `this + 0x214 = v`
///
/// Row: `0x005b16f0`.
pub fn set_214(cell: &mut u32, v: u32) {
    *cell = v;
}

/// `this + 0x218 = v`
///
/// Row: `0x005b1710`.
pub fn set_218(cell: &mut u32, v: u32) {
    *cell = v;
}

/// `this + 0x21c = *src; this + 0x220 = src[1]` — the two-dword
/// source-pointer copy at the end of the field block.
///
/// Row: `0x005b1730`.
pub fn set_21c_pair(a: &mut u32, b: &mut u32, src: &[u32; 2]) {
    *a = src[0];
    *b = src[1];
}

/// Byte store first (`this + 0x20b = 0`), then `this + 0x208 |= value << 24`
/// — the game ORs the caller value into the flag dword's top byte, dropping
/// any bits shifted out of the 32-bit cell (the decompiler shows an `int`
/// left shift; the mirror takes `u32`, high bits simply fall off).
///
/// Row: `0x005b1750`.
pub fn zero_20b_or_top_byte(byte: &mut u8, flags: &mut u32, value: u32) {
    *byte = 0;
    *flags |= value << 24;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_bits_are_independent() {
        // Setting and clearing single bits leaves the others untouched.
        let mut f = 0u32;
        set_bit0(&mut f, true);
        set_bit2(&mut f, true);
        assert_eq!(f, 0b0101);
        set_bit0(&mut f, false);
        assert_eq!(f, 0b0100);
        // Clearing an unset bit is a no-op, never sets anything.
        set_bit3(&mut f, false);
        assert_eq!(f, 0b0100);
    }

    #[test]
    fn bit_setters_trigger_on_any_nonzero() {
        // The game only tests `param_2 == 0`; the `on: bool` boundary keeps
        // the same behavior for every nonzero value.
        let mut f = 0u32;
        set_bit1(&mut f, true);
        assert_eq!(f, 2);
    }

    #[test]
    fn top_byte_or_keeps_low_bytes_and_drops_overflow() {
        let mut byte = 0xa5u8;
        let mut f = 0x0000_00f0u32;
        zero_20b_or_top_byte(&mut byte, &mut f, 0x5a);
        assert_eq!(byte, 0); // the byte at 0x20b is zeroed first
        assert_eq!(f, 0x5a00_00f0);
        // Bits above byte 3 fall off the u32 shift exactly as in the game.
        zero_20b_or_top_byte(&mut byte, &mut f, 0xdead_beef);
        assert_eq!(f, 0x5a00_00f0 | (0xbeef << 24));
    }

    #[test]
    fn vec_and_pair_copies_read_the_source_in_order() {
        let src3 = [1u32, 2, 3];
        let (mut a, mut b, mut c) = (9, 9, 9);
        set_144_vec3(&mut a, &mut b, &mut c, &src3);
        assert_eq!((a, b, c), (1, 2, 3));
        set_150_vec3(&mut a, &mut b, &mut c, &[7, 8, 9]);
        assert_eq!((a, b, c), (7, 8, 9));
        let (mut x, mut y) = (0, 0);
        set_21c_pair(&mut x, &mut y, &[0xaa, 0xbb]);
        assert_eq!((x, y), (0xaa, 0xbb));
    }

    #[test]
    fn single_cell_setters_are_plain_stores() {
        let mut cell = 0u32;
        set_12c(&mut cell, 300); // 0x005b14c0 writes decimal 300 in the dump
        assert_eq!(cell, 300);
        set_1c0(&mut cell, u32::MAX);
        assert_eq!(cell, u32::MAX);
    }
}
