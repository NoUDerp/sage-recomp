//! Wave 44: W3D-Draw vector-pair out-param accessors — 22 rows from the
//! `0x0048bb0e`–`0x0048bd36` run (band tag W3D-Draw-dominant; module tag
//! unassigned in the ledger).
//!
//! One mutable class exposes scattered cells as C-style out-parameter
//! getters (`*param_2 = *(this + off); param_2[1] = ...`), plain dword/byte
//! stores, and an index pair at `this + 0x8c/0x90` whose `0xffffffff` (-1)
//! sentinel means "unset". The out-param readers are mirrored as functions
//! that *return* the tuple/array; the stores take the referenced cells by
//! `&mut` exactly as in `drawdata`/`fxstore` — the caller resolves field
//! offsets.
//!
//! Row -> function map:
//!
//! | Row          | Function           | Game shape (field offsets)                              |
//! |--------------|--------------------|---------------------------------------------------------|
//! | `0x0048bb0e` | `pair_of`          | out-pair <- `this + 0x20`, `this + 0x24`                |
//! | `0x0048bb27` | `vec3_of`          | out-dwords <- `this + 0x0c/0x10/0x14`                   |
//! | `0x0048bb4a` | `set_cell`         | `this + 0x54 = value`                                   |
//! | `0x0048bb5c` | `set_cell`         | `this + 0x50 = value`                                   |
//! | `0x0048bb6f` | `set_cell`         | `this + 0x68 = value`                                   |
//! | `0x0048bb93` | `set_cell`         | `this + 0x70 = value`                                   |
//! | `0x0048bba1` | `set_pair`         | `this + 0x8c = x`, `this + 0x90 = y`                    |
//! | `0x0048bbb8` | `reset_pair`       | `this + 0x8c = -1`, `this + 0x90 = -1`                  |
//! | `0x0048bbc7` | `pair_valid`       | `this + 0x8c != -1 && this + 0x90 != -1`                |
//! | `0x0048bbe0` | `vec3_of`          | out-dwords <- `this + 0x8c/0x90/0x94`                   |
//! | `0x0048bbf6` | — skipped          | scaled accumulate through global `DAT_00bbb9ac`         |
//! | `0x0048bc2b` | `clear_cell`       | `this + 0x98 = 0`                                       |
//! | `0x0048bc3e` | `set_pair`         | `this + 0xac = x`, `this + 0xb0 = y`                    |
//! | `0x0048bc6b` | `set_cell`         | `this + 0x9c = value`                                   |
//! | `0x0048bc7c` | `set_cell`         | `this + 0xa4 = value`                                   |
//! | `0x0048bca9` | `set_cell`         | `this + 0x23d4 = value`                                 |
//! | `0x0048bcb6` | `store_pair_from`  | `this + 0x78/0x7c` <- `*param_2`, `param_2[1]`          |
//! | `0x0048bcd3` | `set_byte`         | `this + 0x2490 = value` (byte store)                    |
//! | `0x0048bce7` | `set_byte`         | `this + 0x249d = value` (byte store)                    |
//! | `0x0048bcfb` | `set_byte`         | `this + 0x249e = value` (byte store)                    |
//! | `0x0048bd13` | `set_byte`         | `this + 0x2448 = value` (byte store)                    |
//! | `0x0048bd36` | `set_cell`         | `this + 0x23cc = value`                                 |
//!
//! Quirks pinned by the game and mirrored exactly:
//! - The "valid" test (`0x0048bbc7`) compares the pair cells **signed** with
//!   `-1` and returns `0` if EITHER cell is the sentinel; `reset_pair`
//!   (`0x0048bbb8`) writes that sentinel back, so mirror the constant as
//!   `u32::MAX`.
//! - `store_pair_from` (`0x0048bcb6`) *reads* two consecutive dwords through
//!   the caller's pointer — the mirror takes a fixed 2-dword slice.
//! - `0x0048bbf6` scales a negative delta by the engine global
//!   `DAT_00bbb9ac` — not a pure accessor, skipped.

/// Pair out-param getter, mirrored as a tuple — the game writes the two
/// dwords into successive out-param slots, so tuple order is the C order.
///
/// Row: `0x0048bb0e` — cells `+0x20`, `+0x24`
pub fn pair_of(a: u32, b: u32) -> (u32, u32) {
    (a, b)
}

/// Three-dword out-param getter, mirrored as an array in C order
/// (`param_2[0..3]`).
///
/// Row: `0x0048bb27` — cells `+0x0c`, `+0x10`, `+0x14`
/// Row: `0x0048bbe0` — cells `+0x8c`, `+0x90`, `+0x94`
pub fn vec3_of(a: u32, b: u32, c: u32) -> [u32; 3] {
    [a, b, c]
}

/// Plain dword store, value written verbatim.
///
/// Row: `0x0048bb4a` — field offset `+0x54`
/// Row: `0x0048bb5c` — field offset `+0x50`
/// Row: `0x0048bb6f` — field offset `+0x68`
/// Row: `0x0048bb93` — field offset `+0x70`
/// Row: `0x0048bc6b` — field offset `+0x9c`
/// Row: `0x0048bc7c` — field offset `+0xa4`
/// Row: `0x0048bca9` — field offset `+0x23d4`
/// Row: `0x0048bd36` — field offset `+0x23cc`
pub fn set_cell(cell: &mut u32, value: u32) {
    *cell = value;
}

/// Byte store (the game stores `undefined1`; only the low byte exists).
///
/// Row: `0x0048bcd3` — field offset `+0x2490`
/// Row: `0x0048bce7` — field offset `+0x249d`
/// Row: `0x0048bcfb` — field offset `+0x249e`
/// Row: `0x0048bd13` — field offset `+0x2448`
pub fn set_byte(cell: &mut u8, value: u8) {
    *cell = value;
}

/// Constant zero store (no value parameter in the game).
///
/// Row: `0x0048bc2b` — field offset `+0x98`
pub fn clear_cell(cell: &mut u32) {
    *cell = 0;
}

/// Two-dword store: successive adjacent cells, values in C parameter order.
///
/// Row: `0x0048bba1` — cells `+0x8c`, `+0x90`
/// Row: `0x0048bc3e` — cells `+0xac`, `+0xb0`
pub fn set_pair(a: &mut u32, b: &mut u32, x: u32, y: u32) {
    *a = x;
    *b = y;
}

/// Reset the index pair to the game's `-1` "unset" sentinel in both cells.
///
/// Row: `0x0048bbb8` — cells `+0x8c`, `+0x90`
pub fn reset_pair(a: &mut u32, b: &mut u32) {
    *a = u32::MAX;
    *b = u32::MAX;
}

/// Pair-sentinel guard: the game returns `1` only when NEITHER cell
/// (compared signed) equals `-1`; either sentinel alone makes the pair
/// invalid (`||` of the two tests).
///
/// Row: `0x0048bbc7` — cells `+0x8c`, `+0x90`
pub fn pair_valid(a: u32, b: u32) -> bool {
    a != u32::MAX && b != u32::MAX
}

/// Copy two consecutive dwords from the caller's source block into the pair
/// of adjacent cells (`this + 0x78`, `this + 0x7c`); the game reads them
/// through an unchecked pointer — the mirror takes a fixed 2-dword slice.
///
/// Row: `0x0048bcb6` — cells `+0x78`, `+0x7c`
pub fn store_pair_from(src: &[u32; 2], a: &mut u32, b: &mut u32) {
    *a = src[0];
    *b = src[1];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn out_param_readers_preserve_c_order() {
        assert_eq!(pair_of(0xaaaa, 0xbbbb), (0xaaaa, 0xbbbb));
        assert_eq!(vec3_of(1, 2, 3), [1, 2, 3]);
        // 0x0048bbe0 reads three ADJACENT cells but hands them out in the
        // same order as 0x0048bb27 — one shared shape.
        let pair = vec3_of(0x8c, 0x90, 0x94);
        assert_eq!((pair[0], pair[2]), (0x8c, 0x94));
    }

    #[test]
    fn either_sentinel_invalidates_pair() {
        // The game ORs the two -1 tests: one sentinel is enough.
        assert!(!pair_valid(u32::MAX, 5));
        assert!(!pair_valid(5, u32::MAX));
        assert!(!pair_valid(u32::MAX, u32::MAX));
        assert!(pair_valid(5, 6));
        // 0xfffffffe is NOT the sentinel.
        assert!(pair_valid(0xffff_fffe, 0));
    }

    #[test]
    fn reset_then_set_pair_round_trips_validity() {
        let (mut a, mut b) = (7u32, 9u32);
        assert!(pair_valid(a, b));
        reset_pair(&mut a, &mut b);
        assert_eq!((a, b), (u32::MAX, u32::MAX));
        assert!(!pair_valid(a, b));
        set_pair(&mut a, &mut b, 1, u32::MAX);
        assert_eq!((a, b), (1, u32::MAX));
        assert!(!pair_valid(a, b));
    }

    #[test]
    fn store_pair_from_copies_in_order_and_leaves_source() {
        let src = [0x1111u32, 0x2222];
        let (mut a, mut b) = (0u32, 0u32);
        store_pair_from(&src, &mut a, &mut b);
        assert_eq!((a, b), (0x1111, 0x2222));
        assert_eq!(src, [0x1111, 0x2222]);
    }

    #[test]
    fn plain_stores_write_exactly_their_cell() {
        let mut cell = 0xdead_beef;
        set_cell(&mut cell, 3);
        assert_eq!(cell, 3);
        clear_cell(&mut cell);
        assert_eq!(cell, 0);
        let mut byte = 0u8;
        set_byte(&mut byte, 0xa5);
        assert_eq!(byte, 0xa5);
    }
}
