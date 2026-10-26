//! Wave 43: Miles-audio **handle / refcount plumbing** — the audio-manager
//! band `0x00450d6c`–`0x004535a6` (band identity: row `0x00451017` calls
//! `_AIL_set_sample_loop_count_8` on the sample handle stored at `+8` (2D) or
//! `+4` (3D), gated by the byte at `+2`, which pins the whole band as the Miles
//! Sound System (AIL) wrapper layer over the game's own sound objects).
//!
//! Uniform shape: every game function takes `this` (`param_1`) and walks field
//! offsets / pointer hops internally; the mirror takes the **referenced cells**
//! by value or `&mut` and lets the caller resolve the offsets — the
//! `drawdata::set_dirty_cell(dirty, cell, value)` precedent. Pointer-valued
//! cells stay `u32` with `0` = NULL (exactly the game's encoding); the cell a
//! pointer points at arrives as a separate borrowed cell.
//!
//! | rows | function | what it touches |
//! |------|----------|-----------------|
//! | `0x00450d6c` | `state_any_set` | state quad `this + 0x49..0x4c` |
//! | `0x00450d86` | `state_any_unmasked` | same quad + `param_2` mask bits 1/2/4/8 |
//! | `0x00450e1c` | `handle_identity_eq` | identity dword `+8` of two handles |
//! | `0x00450efa` | `try_acquire` | refcount dword `+4` |
//! | `0x004514eb` | `dec_if_nonzero` | count dword `this + 0x684` |
//! | `0x004515b3` | `clear_and_store` | byte `this + 0x6ac`, dword `this + 0x6b0` |
//! | `0x00451914` + `0x00451ada` | `set_handle` | handle slot dword + target refcount `+4` |
//! | `0x004520c6` | `decay_matrix` | signed `6 × 2 × 4` matrix at `this + 0x188` |
//! | `0x00452662` | `read_sample_handle` | tag `obj + 0x14`, record table at `this + 0xbd4` |
//! | `0x004535a6` | `store_ready_byte` | byte `obj + 0x4b`, mode cells `+0x1c→+0x30` / `this + 0x678` |
//!
//! The band's FFI surface is **intentionally excluded** and deferred to the
//! client/audio-backend layer (nothing deterministic to recompile): 46 rows in
//! the band call `_AIL_*` directly — `0x00451017` (set sample loop count, 2D vs
//! 3D), `0x004517f1` (`_AIL_get_preference`/`_AIL_set_preference` then
//! `_AIL_serve`), `0x00451846` (preference product `0x22 * 0x2a`) — two rows
//! drive Win32 critical sections (`0x00453281` = `EnterCriticalSection(this +
//! 0xb90 → +0x40)`), and the pacing rows call `Sleep` (`0x0045d73e`,
//! `0x0045eff3`). None of them are mirrored here.
//!
//! Quirks pinned by the game and mirrored exactly:
//! - `InterlockedIncrement`/`InterlockedDecrement` become plain `+1`/`-1`: the
//!   core is single-threaded and deterministic, the atomics only mattered
//!   because the AIL serve thread exists (see the excluded `0x004517f1`).
//! - `0x00450efa` only succeeds when the count cell was **already ≥ 1**: the
//!   first reference is planted unguarded by `set_handle`, so the try-acquire
//!   is a "copy an already-owned handle" test, not a lock-from-zero.
//! - `0x00450e1c` never compares the handle *addresses* of two live objects —
//!   only their `+8` dwords, so two distinct handles with equal identity
//!   compare equal; `NULL == NULL` is `true`, `NULL` vs live is `false`.
//! - The `+0x49..0x4c` quad is per-byte truthy for these two rows, yet other
//!   callers treat the *same* bytes as a bitmask (e.g. `0x0045b021` tests
//!   `(+0x49) & 6`) — the mirrors keep both readings.
//! - `0x004520c6` guards with `0 < (signed char)cell`, so `0` stays `0` and
//!   **negative cells are never touched** (a latch at zero, not a ramp to it);
//!   `0x004514eb` guards with `!= 0`, so negative counts keep counting down.
//! - `0x00452662`'s record index is multiplied by the `0x48` record stride with
//!   no bounds check (wild `imul` + pointer add = UB); the mirror panics.
//! - The `CONCAT44(hi, lo)` 64-bit return of `0x00452662` becomes `(i32, i32)`
//!   `(hi, lo)`: `hi` is the status half (`0` on the tag-2/tag-3 paths, else
//!   `tag - 4`), `lo` the raw value bits.

/// Stride of one record in the manager's table at `this + 0xbd4` (the game's
/// `*(int *)(… + 8) * 0x48` element indexing, row `0x00452662`).
pub const SAMPLE_RECORD_STRIDE: usize = 0x48;

/// Any byte of the four-byte state quad at `this + 0x49..0x4c` non-zero.
/// The game tests each through `*(char *)`, i.e. sign-ignoring truthiness.
///
/// Row: `0x00450d6c` — quad at `this + 0x49`/`0x4a`/`0x4b`/`0x4c`.
pub fn state_any_set(state: [u8; 4]) -> bool {
    state[0] != 0 || state[1] != 0 || state[2] != 0 || state[3] != 0
}

/// Mask-gated form: `true` when some non-zero quad byte has **its own mask bit
/// clear** — bit `1` → `+0x49`, `2` → `+0x4a`, `4` → `+0x4b`, `8` → `+0x4c`.
/// The game's condition for `0` is
/// `(b0 == 0 || mask & 1) && (b1 == 0 || mask & 2) && (b2 == 0 || mask & 4) && (b3 == 0 || mask & 8)`.
///
/// Row: `0x00450d86` — quad at `this + 0x49..0x4c`, mask = `param_2` (byte).
pub fn state_any_unmasked(state: [u8; 4], mask: u8) -> bool {
    (state[0] != 0 && mask & 1 == 0)
        || (state[1] != 0 && mask & 2 == 0)
        || (state[2] != 0 && mask & 4 == 0)
        || (state[3] != 0 && mask & 8 == 0)
}

/// Null-aware identity compare of two handles. `None` is the NULL pointer; the
/// payload is each object's identity dword at `+8`. Both NULL → `true`; one
/// NULL → `false`; both live → compare the identity dwords only (the handle
/// addresses themselves are never compared — a real game quirk).
///
/// Row: `0x00450e1c` — identity dwords `param_1 + 8` / `param_2 + 8`.
pub fn handle_identity_eq(lhs: Option<i32>, rhs: Option<i32>) -> bool {
    match (lhs, rhs) {
        (None, None) => true,
        (None, Some(_)) | (Some(_), None) => false,
        (Some(a), Some(b)) => a == b,
    }
}

/// Try to take a second reference through the refcount cell: plain
/// increment/rollback arithmetic (single-threaded deterministic core — the
/// game used `InterlockedIncrement`/`InterlockedDecrement` at `this + 4`).
/// The cell is left exactly as found on failure. Success requires the cell to
/// have been `>= 1` already; it stays incremented on success.
///
/// Row: `0x00450efa` — refcount cell at `this + 4`.
pub fn try_acquire(refcount: &mut i32) -> bool {
    // The cell never comes near `i32::MAX` in the game (unchecked `INC` = UB
    // there); the mirror keeps the plain arithmetic and may panic on overflow.
    *refcount += 1;
    if *refcount < 2 {
        *refcount -= 1;
        return false;
    }
    true
}

/// Guarded decrement: subtract one only when the cell is non-zero, so the busy
/// counter is never taken twice past zero. The guard is `!= 0`, not `> 0` — a
/// negative cell keeps decreasing (faithful quirk).
///
/// Row: `0x004514eb` — count cell at `this + 0x684`.
pub fn dec_if_nonzero(cell: &mut i32) {
    if *cell != 0 {
        *cell -= 1;
    }
}

/// Clear the manager's pending byte and overwrite the paired value cell —
/// unconditionally, in the game's write order (`+0x6ac` first, then `+0x6b0`).
///
/// Row: `0x004515b3` — byte at `this + 0x6ac`, dword at `this + 0x6b0`.
pub fn clear_and_store(flag: &mut u8, cell: &mut u32, value: u32) {
    *flag = 0;
    *cell = value;
}

/// Handle assignment plus refcount bump: `*this = handle; if (handle)
/// ++*(LONG *)(handle + 4);`. Both rows collapse to this one function —
/// `0x00451914` receives the handle **by value** (`int param_2`, a raw object
/// pointer) while `0x00451ada` reads it from another handle slot
/// (`iVar1 = *param_2`, its own copy happens before the destination write, so
/// self-assignment is a no-op); the observable effect on the cells is identical.
/// The game returns `param_1` (the `this` slot, for chaining), which the mirror
/// cannot observe, so it returns nothing.
///
/// `handle == 0` must agree with `refcount.is_none()` — the mirror asserts the
/// pairing; in the game a mismatch is a write through a dangling pointer.
///
/// Rows: `0x00451914` (handle by value, refcount at `handle + 4`),
/// `0x00451ada` (handle = `*param_2`, refcount at `*param_2 + 4`; slot `param_1`).
pub fn set_handle(slot: &mut u32, handle: u32, refcount: Option<&mut i32>) {
    debug_assert_eq!(handle == 0, refcount.is_none(), "NULL handle ↔ no cell");
    *slot = handle;
    if let Some(cell) = refcount {
        // Plain increment — see the single-threaded note on `try_acquire`.
        *cell += 1;
    }
}

/// The manager's `6 × 2 × 4` signed byte matrix at `this + 0x188` (48
/// contiguous bytes, `0x188..0x1b8`): every strictly positive cell decays one
/// step per call; `0` stays put and negatives are left alone, so a cell latches
/// at zero instead of ramping into the negative range. The three nested loops
/// (`6` groups → `2` → `4` bytes, base advancing `+4`) walk the block
/// contiguously, which the nesting below preserves.
///
/// Row: `0x004520c6` — matrix at `this + 0x188`, guard `'\0' < (char)cell`.
pub fn decay_matrix(matrix: &mut [i8; 48]) {
    for group in 0..6usize {
        for pair in 0..2usize {
            for lane in 0..4usize {
                let cell = &mut matrix[(group * 2 + pair) * 4 + lane];
                if *cell > 0 {
                    *cell -= 1;
                }
            }
        }
    }
}

/// Tagged sample-handle reader. The object's tag cell at `obj + 0x14` selects
/// the resolution:
/// - `2` → the value dword `obj + 8` itself (`hi = 0`);
/// - `3` → `obj + 8` is a **record index**: `slot = index * 0x48 + table`, the
///   gate byte `slot + 2` must be non-zero, and the value is the raw dword at
///   `slot + 4`; a clear gate yields `(0, 0)`, and the index is used unchecked
///   (wild `imul` + pointer add = UB in the game) so it panics here;
/// - anything else → `(tag - 4, 0)`: the tag itself, shifted by four, is the
///   status half of the return and no value is produced.
///
/// The 64-bit `CONCAT44(iVar3, uVar1)` return becomes `(hi, lo)`; `lo` carries
/// raw dword bits (callers reinterpret as needed).
///
/// Row: `0x00452662` — tag `obj + 0x14`, value/index `obj + 8`, record table at
/// `this + 0xbd4` (`SAMPLE_RECORD_STRIDE`-byte records, gate byte `+2`, value `+4`).
pub fn read_sample_handle(tag: i32, obj_field8: i32, table: &[u8]) -> (i32, i32) {
    match tag {
        2 => (0, obj_field8),
        3 => {
            let start = usize::try_from(obj_field8 as i64 * SAMPLE_RECORD_STRIDE as i64)
                .expect("negative record index: unchecked in the game (UB), panic here");
            let record = &table[start..start + SAMPLE_RECORD_STRIDE];
            if record[2] != 0 {
                (0, i32::from_le_bytes([record[4], record[5], record[6], record[7]]))
            } else {
                (0, 0)
            }
        }
        other => (other.wrapping_sub(4), 0),
    }
}

/// Conditional write of the state quad's `+0x4b` byte — the same cell read by
/// `state_any_set`/`state_any_unmasked` (element 2 of the quad). It is cleared
/// when the object's sub-object mode is `2` **or** equals the manager's mode
/// cell, otherwise set; the store always happens (never skipped).
///
/// Row: `0x004535a6` — target byte `obj + 0x4b`, mode `*(*(obj + 0x1c) + 0x30)`,
/// manager mode cell `this + 0x678`.
pub fn store_ready_byte(target: &mut u8, mode: i32, owner_mode: i32) {
    *target = if mode == 2 || mode == owner_mode { 0 } else { 1 };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_acquire_requires_an_existing_reference() {
        let mut cell = 0;
        assert!(!try_acquire(&mut cell));
        assert_eq!(cell, 0, "failed acquire rolls the increment back");

        let mut cell = 1;
        assert!(try_acquire(&mut cell));
        assert_eq!(cell, 2, "success keeps the increment");

        let mut cell = 7;
        assert!(try_acquire(&mut cell));
        assert_eq!(cell, 8);

        let mut cell = -1;
        assert!(!try_acquire(&mut cell));
        assert_eq!(cell, -1, "increment to 0 still fails the `< 2` gate");
    }

    #[test]
    fn guarded_decrement_guards_on_nonzero_not_positive() {
        let mut cell = 3;
        dec_if_nonzero(&mut cell);
        assert_eq!(cell, 2);

        let mut cell = 1;
        dec_if_nonzero(&mut cell);
        assert_eq!(cell, 0);

        let mut cell = 0;
        dec_if_nonzero(&mut cell);
        assert_eq!(cell, 0, "0 is the latch");

        let mut cell = -2;
        dec_if_nonzero(&mut cell);
        assert_eq!(cell, -3, "game guard is != 0, so negatives keep falling");
    }

    #[test]
    fn decay_matrix_latches_at_zero_and_ignores_negatives() {
        let mut matrix = [0i8; 48];
        matrix[0] = 1;
        matrix[3] = 1;
        matrix[4] = 2;
        matrix[8] = i8::MAX;
        matrix[12] = -5;
        matrix[47] = i8::MIN;
        decay_matrix(&mut matrix);
        assert_eq!((matrix[0], matrix[3], matrix[4], matrix[8]), (0, 0, 1, i8::MAX - 1));
        assert_eq!((matrix[12], matrix[47]), (-5, i8::MIN), "0 < (char)cell guard");
        assert_eq!(matrix[1], 0, "untouched cells stay put");
    }

    #[test]
    fn state_quad_predicate_and_mask_gate() {
        assert!(!state_any_set([0, 0, 0, 0]));
        assert!(state_any_set([0, 0, 0, 8]));

        // Only +0x4a is set: masked by bit 2 it stops counting, unmasked it does.
        assert!(state_any_unmasked([0, 9, 0, 0], 0b0101));
        assert!(!state_any_unmasked([0, 9, 0, 0], 2));
        assert!(!state_any_unmasked([0, 9, 0, 0], 0xff));

        assert!(state_any_unmasked([1, 2, 4, 8], 0), "every byte counts");
        assert!(!state_any_unmasked([1, 2, 4, 8], 0xf), "all four masked out");
        assert!(state_any_unmasked([1, 2, 4, 8], 0xd), "only +0x4b left unmasked");

        // Identity compare is null-aware and never looks at handle addresses.
        assert!(handle_identity_eq(None, None), "NULL == NULL");
        assert!(!handle_identity_eq(Some(0x100), None));
        assert!(!handle_identity_eq(None, Some(0x100)));
        assert!(handle_identity_eq(Some(5), Some(5)));
        assert!(!handle_identity_eq(Some(5), Some(6)));
    }

    #[test]
    fn tagged_reader_dispatches_on_the_tag_cell() {
        let mut table = vec![0u8; SAMPLE_RECORD_STRIDE * 3];
        let slot = SAMPLE_RECORD_STRIDE * 2;
        table[slot + 2] = 1;
        table[slot + 4..slot + 8].copy_from_slice(&0x2200_1100u32.to_le_bytes());

        assert_eq!(read_sample_handle(2, 0x77, &table), (0, 0x77), "direct obj + 8");
        assert_eq!(read_sample_handle(3, 2, &table), (0, 0x2200_1100i32), "indexed record");
        assert_eq!(read_sample_handle(3, 0, &table), (0, 0), "gate byte clear");
        assert_eq!(read_sample_handle(9, 0, &table), (5, 0), "status = tag - 4");
        assert_eq!(read_sample_handle(0, 0, &table), (-4, 0), "wrapping tag - 4");
        let negative = std::panic::catch_unwind(|| read_sample_handle(3, -1, &table));
        assert!(negative.is_err(), "unchecked index: UB in the game, panic here");
    }

    #[test]
    fn handle_assignment_bumps_only_live_targets() {
        let mut slot = 0u32;
        let mut refcount = 0i32;
        set_handle(&mut slot, 0x0045_1914, Some(&mut refcount));
        assert_eq!(slot, 0x0045_1914);
        assert_eq!(refcount, 1);

        // 0x00451ada path: the source slot is read out first, then written back.
        let copied = slot;
        set_handle(&mut slot, copied, Some(&mut refcount));
        assert_eq!(refcount, 2, "copy bumps the target's refcount");

        let mut null_slot = 0xdead_beef;
        set_handle(&mut null_slot, 0, None);
        assert_eq!(null_slot, 0, "NULL is stored, nothing is incremented");
    }

    #[test]
    fn pending_store_and_conditional_state_byte() {
        let mut flag = 1u8;
        let mut cell = 7u32;
        clear_and_store(&mut flag, &mut cell, 0x55);
        assert_eq!((flag, cell), (0, 0x55));

        let mut ready = 0u8;
        store_ready_byte(&mut ready, 2, 9);
        assert_eq!(ready, 0, "mode == 2 clears");
        store_ready_byte(&mut ready, 9, 9);
        assert_eq!(ready, 0, "mode == manager mode clears");
        store_ready_byte(&mut ready, 3, 9);
        assert_eq!(ready, 1, "anything else sets");
    }
}
