//! Wave 42: W3D drawable dirty-flag setters (`module W3D-Draw`, band `0x0053c7xx–0x0053ce70`).
//!
//! Uniform shape: mark the owner dirty (`this + 0x68 = 1`) **first**, then store
//! through/into the sub-object at `*(this + 8)` or directly into `this` fields.
//! The dirty byte and each target cell are taken by mutable reference — the caller
//! (owner struct) resolves the pointer hop, mirroring the `fxflags` precedent.
//!
//! Field-offset map per row:
//! - vec3 into sub-object: `0x0053cb10` (+0x10/0x14/0x18), `0x0053cbb0` (+0x0/0x4/0x8),
//!   `0x0053cc40` (+0x20/0x24/0x28), `0x0053cce0` (+0x30/0x34/0x38)
//! - single cell into sub-object: `0x0053cd30` (+0x40), `0x0053cd60` (+0xc)
//! - tri-state mode into `this`: `0x0053cd80` (+0x10), `0x0053cdb0` (+0x14), `0x0053cde0` (+0x18)
//! - tri-state getters (no dirty write): `0x0053ce10` (+0x10), `0x0053ce30` (+0x14), `0x0053ce50` (+0x18)
//! - single-dword fold bit set/clear into `this + 0xc`: `0x0053c780`
//! - indexed dword store into the block at `this + 0x40`: `0x0053ce70`
//!   (index unchecked in the game — an out-of-range write is UB; the mirror panics)

/// `this + 0x68 = 1` precedes every store in this family — observable as ordering
/// when a setter panics mid-way in debug tooling, so the mirrors set it first.
fn mark(dirty: &mut u8) {
    *dirty = 1;
}

/// Tri-state clamp setter: `1 → 1`, `2 → 2`, everything else → `0`, plus dirty.
///
/// Rows: `0x0053cd80`, `0x0053cdb0`, `0x0053cde0`.
pub fn set_dirty_mode(dirty: &mut u8, cell: &mut u32, value: i32) {
    mark(dirty);
    *cell = match value {
        1 => 1,
        2 => 2,
        _ => 0,
    };
}

/// Tri-state read (rows write nothing): `0x0053ce10`, `0x0053ce30`, `0x0053ce50`.
pub fn mode_of(cell: u32) -> u32 {
    match cell {
        1 => 1,
        2 => 2,
        _ => 0,
    }
}

/// Three-dword store into the sub-object block, plus dirty.
///
/// Rows: `0x0053cb10`, `0x0053cbb0`, `0x0053cc40`, `0x0053cce0`.
pub fn set_dirty_vec3(
    dirty: &mut u8,
    a: &mut u32,
    b: &mut u32,
    c: &mut u32,
    x: u32,
    y: u32,
    z: u32,
) {
    mark(dirty);
    *a = x;
    *b = y;
    *c = z;
}

/// Single-dword store into the sub-object block, plus dirty.
///
/// Rows: `0x0053cd30`, `0x0053cd60`.
pub fn set_dirty_cell(dirty: &mut u8, cell: &mut u32, value: u32) {
    mark(dirty);
    *cell = value;
}

/// Set/clear one bit of the owner's single-dword flag cell (`this + 0xc`), plus
/// dirty. The bit index folds `& 0x1f` (full-u32 index, no dword addressing).
///
/// Row: `0x0053c780`.
pub fn set_dirty_bit(dirty: &mut u8, flags_cell: &mut u32, bit_index: u32, on: bool) {
    mark(dirty);
    let mask = 1u32 << (bit_index & 0x1f);
    if !on {
        *flags_cell &= !mask;
    } else {
        *flags_cell |= mask;
    }
}

/// Indexed dword store into the owner block at `this + 0x40`, plus dirty.
/// The game computes `this + 0x40 + idx * 4` with no bounds check (out-of-range
/// writes scribble memory = UB); the mirror indexes the block and panics instead.
///
/// Row: `0x0053ce70`.
pub fn set_dirty_indexed(dirty: &mut u8, block: &mut [u32], index: u32, value: u32) {
    mark(dirty);
    block[index as usize] = value;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_clamps_everything_but_one_and_two() {
        for (input, want) in [(1i32, 1u32), (2, 2), (0, 0), (3, 0), (-1, 0), (i32::MIN, 0)] {
            let mut dirty = 0u8;
            let mut cell = 0xdead_beef;
            set_dirty_mode(&mut dirty, &mut cell, input);
            assert_eq!(cell, want, "input {input}");
            assert_eq!(dirty, 1);
        }
    }

    #[test]
    fn mode_read_clamps_too() {
        assert_eq!(mode_of(1), 1);
        assert_eq!(mode_of(2), 2);
        assert_eq!(mode_of(7), 0);
    }

    #[test]
    fn vec3_and_cell_stores_mark_dirty() {
        let mut dirty = 0u8;
        let (mut a, mut b, mut c) = (0u32, 0u32, 0u32);
        set_dirty_vec3(&mut dirty, &mut a, &mut b, &mut c, 1, 2, 3);
        assert_eq!((a, b, c), (1, 2, 3));
        assert_eq!(dirty, 1);

        let mut cell = 0u32;
        let mut d2 = 0u8;
        set_dirty_cell(&mut d2, &mut cell, 0xbeef);
        assert_eq!(cell, 0xbeef);
        assert_eq!(d2, 1);
    }

    #[test]
    fn bit_set_clear_folds_index_mod_32() {
        let mut dirty = 0u8;
        let mut flags = 0u32;
        set_dirty_bit(&mut dirty, &mut flags, 5, true);
        assert_eq!(flags, 1 << 5);
        set_dirty_bit(&mut dirty, &mut flags, 5 + 32, false); // folds onto bit 5
        assert_eq!(flags, 0);
        assert_eq!(dirty, 1);
    }

    #[test]
    fn indexed_store_writes_and_marks() {
        let mut dirty = 0u8;
        let mut block = [0u32; 4];
        set_dirty_indexed(&mut dirty, &mut block, 2, 9);
        assert_eq!(block, [0, 0, 9, 0]);
        assert_eq!(dirty, 1);
    }
}
