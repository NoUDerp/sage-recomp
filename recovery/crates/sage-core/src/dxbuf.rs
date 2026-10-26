//! Wave 45: W3D-Draw **DX buffer-size math and buffer plumbing** (`module
//! W3D-Draw`, band `0x00595bc0`–`0x005969c0`, rows `12 <= size <= 140`).
//!
//! Identity evidence (per repo spec §8, cited at each row):
//! - `0x00595bc0` = `((d0x1c - d0x18) + 1) * d8 * 4 + 0x20` — vertex buffer
//!   bytes for a `[start, end]` inclusive range with per-vertex stride `d8`.
//! - `0x00595be0` zeroes dwords 0–2/5 and sets the pair at `+0x0c/+0x10` to
//!   `0xffffffff` — exactly the cells both index formulas read (empty range).
//! - `0x00595cc0` = `(((d0x10 - d0x0c) + 8) div 8) + 0x18` via the
//!   `x + (x >> 0x1f & 7) >> 3` sign-preserving divide-by-8 trick — index
//!   buffer header size (mirror keeps the trick literal).
//! - `0x00595dc0` = `d0x10 * d0x0c * 4 + 0x1c` — index buffer bytes
//!   (count × index size × 4 + header).
//! - The 0x00596x family is dynamic int/float vectors (`find`/`erase`/`==`)
//!   plus a slot table of indirection cells and a sorted-key cursor
//!   (`0x00595ef0`) whose entries carry an `0x8000_0000` flag bit.
//!
//! Skip ledger (rule 4 of the wave contract — body is a call chain / `code**`
//! dispatch / global-`DAT_` machinery / Interlocked):
//!
//! | row          | reason                                                                                   |
//! |--------------|------------------------------------------------------------------------------------------|
//! | `0x00595f60` | ctor zeroes `this[0..6]` then lazy-fills the **global** sine table `DAT_00db6818` (loop to `0xdb6bd8`, `fsin` over `DAT_00bce3f8/bbb8d8/bbb8d0`, gate flag `DAT_00dfcec0`) — table writes not owned by the struct param |
//! | `0x005960c0` | body = `FUN_0042fd80(*(this + 0x14))` (delete via `(*DAT_00de03fc)(p, 2)`) + null store — call chain |
//! | `0x005960e0` | same delete wrapper on `this + 0x18` — call chain                                          |
//! | `0x00596150` | vtable-pointer stores: `+0x8` ← `&PTR_FUN_00bd5cac` then overwritten by `&PTR_FUN_00bcef94`, `+0x0` ← `&PTR_FUN_00bc650c` — vtable machinery, no data semantics |
//! | `0x00596170` | `FUN_0042fd80(*this)` release + `FUN_00a19880(*param_2)` (strdup: strlen + `FUN_00ac4c30(len, "str")` + copy) — call chain |
//! | `0x00596280` | refcount-0 legs dispatch destructors: `(**(code **) *puVar2)()` and `(*(code *) **(undefined4 **)(iVar3 + 0x18))()` |
//! | `0x005962b0` | same two `code**` dispatch legs (+ `DAT_00bbb8d8` fill re-init)                             |
//! | `0x00596310` | owner swap: the decrement-to-0 leg calls `(*(code *) **(undefined4 **)(old + 0x18))()`      |
//! | `0x00596370` | list-wipe loop: re-reads `**(this + 4)` and calls `FUN_00430830` (delete via `(*DAT_00de03fc)(p, 3)`) per node — call chain |
//! | `0x00596470` | body starts with `FUN_0046577f()` = Interlocked spinlock acquire (`LOCK(); *cell |= 1; …`) on an ecx-borne lock cell whose offset the decomp never shows — Interlocked/OS guard |
//! | `0x005964a0` | ctor stores vtable `&PTR_FUN_00bd5e58` at `+0x0` and conditionally allocates via `FUN_0042fde0(count * 4)` (new via `(*DAT_00de0404)(size, 2, 0)`) |
//! | `0x00596560` | virtual resize `(**(code **)(*this + 8))(count, 0)` mutates the buffer, then the refill loop calls `FUN_00560b60` per entry (a `_strcmpi` scan over a `0x58`-stride name table) — dispatch + call chain |
//! | `0x005967d0` | gate call `FUN_005796a0(count, src)` rebinds the same object (mutates `this[1]/[2]/[3]` + byte `+0xd`; itself `code**` dispatch when `count == 0` + `FUN_0042fde0` alloc) — callee mutates this object's state, so the row is not data-faithful without modeling the callee |
//! | `0x00596840` | frees a vector with the MSVC CRT `_eh_vector_destructor_iterator_(ptr, 8, count, FUN_00442ff8)` + `FUN_0042fd80(buf - 4)` — CRT/OS call chain |
//! | `0x00596880` | append: the grow path is gated by virtual `(**(code **)(*this + 8))(mincap + cur, 0)` whose result decides the branch — `code**` dispatch in control flow |
//!
//! Mirrored rows keep the game's i32 wrapping (MSVC x86 two's complement):
//! all pointer/integer arithmetic uses `wrapping_*`; out-of-range slice
//! indexing models the game's unchecked `base + idx * 4` derefs as panics
//! (faithful-quirk policy, same convention as `drawdata`/`flagname`).
//! Pointer hops are caller-resolved: cells arrive by reference
//! (`drawdata`/`fxflags` precedent), nullable hops arrive as
//! `Option<&mut i32>` (`fxstore2`/`audiohandle` precedent).

/// Slot-table hop helper: the game computes `(this + 4)[idx]` with **no**
/// bounds or sign check (out-of-range/negative = read anywhere = UB); the
/// mirror converts to `usize` and lets the slice index panic.
fn slot(index: i32) -> usize {
    index as usize
}

/// `((this.d0x1c - this.d0x18) + 1) * this.d8 * 4 + 0x20` — DX **vertex
/// buffer** byte size: inclusive index range `[d0x18, d0x1c]` × per-vertex
/// dword stride `d8` × 4, plus a 0x20 header. Signed wrapping chain, exactly
/// as MSVC emits it.
///
/// Row: `0x00595bc0` — cells: `this + 0x18` range start, `this + 0x1c` range
/// end, `this + 8` dword stride (`(*(int *)(this + 0x1c) - *(int *)(this +
/// 0x18) + 1) * *(int *)(this + 8) * 4 + 0x20`).
pub fn dx_vertex_buffer_bytes(start: i32, end: i32, stride: i32) -> i32 {
    let count = end.wrapping_sub(start).wrapping_add(1);
    count.wrapping_mul(stride).wrapping_mul(4).wrapping_add(0x20)
}

/// Empty-range init of the 6-dword index-buffer block: dwords `+0x0/+0x4/+0x8
/// = 0`, `+0x14 = 0`, and `+0x0c/+0x10 = 0xffffffff` — the exact cells read by
/// `0x00595cc0` and `0x00595dc0` (empty range ⇒ header `0x19` / buffer
/// `0x20`); pairing with those formulas is [INFERENCE] from the offsets.
///
/// Row: `0x00595be0`.
pub fn init_range_sentinel_block(block: &mut [u32; 6]) {
    block[0] = 0;
    block[1] = 0;
    block[2] = 0;
    block[3] = 0xffff_ffff;
    block[4] = 0xffff_ffff;
    block[5] = 0;
}

/// `(((this.d0x10 - this.d0x0c) + 8) div 8) + 0x18` — DX **index buffer**
/// header size. The body computes `v = (d0x10 - d0x0c) + 8` then divides by
/// 8 with the sign trick `(v + (v >> 0x1f & 7)) >> 3` (add 7 only for
/// negatives ⇒ truncation toward zero); the mirror keeps every step literal
/// (`v >> 31` is the arithmetic sign mask, `wrapping_add` mirrors the
/// unchecked int add).
///
/// Row: `0x00595cc0`.
pub fn dx_index_header_bytes(low: i32, high: i32) -> i32 {
    let v = high.wrapping_sub(low).wrapping_add(8);
    (v.wrapping_add((v >> 31) & 7) >> 3).wrapping_add(0x18)
}

/// 7-dword zero init (offsets `+0x0`–`+0x18`; covers the `+0x0c/+0x10` pair
/// of `0x00595cc0`/`0x00595dc0` — zero range yields `0x19` / `0x1c`).
///
/// Row: `0x00595ce0`.
pub fn zero_block7(block: &mut [u32; 7]) {
    for cell in block.iter_mut() {
        *cell = 0;
    }
}

/// `this.d0x10 * this.d0x0c * 4 + 0x1c` — DX **index buffer** byte size:
/// count × per-index cells × 4 bytes + 0x1c header, wrapping like the game.
///
/// Row: `0x00595dc0` (cells at `this + 0x10` and `this + 0x0c`).
pub fn dx_index_buffer_bytes(a: i32, b: i32) -> i32 {
    a.wrapping_mul(b).wrapping_mul(4).wrapping_add(0x1c)
}

/// 6-dword zero init (offsets `+0x0`–`+0x14`; shape-named — the owning
/// buffer class is not pinned by the body itself).
///
/// Row: `0x00595e00`.
pub fn zero_block6(block: &mut [u32; 6]) {
    for cell in block.iter_mut() {
        *cell = 0;
    }
}

/// Sorted-key cursor over the dword table at `this + 0x14` (count
/// `this + 0x0c`, cursor `this + 0x10`): entries are `& 0x7fffffff` masked
/// values carrying an `0x8000_0000` flag bit. The body (1) if the value at
/// the current cursor is `<= target`, starts one slot right; (2) walks right
/// while `target >= value`, stopping at the first `target < value` or at
/// `count`; (3) steps back one (clamping negatives to 0), stores the cursor
/// back, and returns whether the entry there has the `0x8000_0000` flag.
///
/// Every table read is unchecked in the game (`base + idx * 4` deref = UB
/// out of range); the mirror indexes and panics.
///
/// Row: `0x00595ef0`.
pub fn sorted_key_cursor_advance(entries: &[u32], count: i32, cursor: &mut i32, target: i32) -> bool {
    let c = *cursor;
    let mut i: i32 = 0;
    if ((entries[slot(c)] & 0x7fff_ffff) as i32) <= target {
        i = c.wrapping_add(1);
    }
    if i < count {
        let mut p = slot(i);
        loop {
            if target < (entries[p] & 0x7fff_ffff) as i32 {
                break;
            }
            i += 1;
            p += 1;
            if i >= count {
                break;
            }
        }
    }
    let mut i = i.wrapping_sub(1);
    if i < 0 {
        i = 0;
    }
    *cursor = i;
    entries[slot(i)] & 0x8000_0000 != 0
}

/// `this + 4 = v`
///
/// Row: `0x00596100`.
pub fn set_4(cell: &mut u32, v: u32) {
    *cell = v;
}

/// `this + 8 = v`
///
/// Row: `0x00596110`.
pub fn set_8(cell: &mut u32, v: u32) {
    *cell = v;
}

/// Ref-handle block shared by rows `0x005961b0` (init) and `0x005961e0`
/// (assign); the skipped rows `0x00596280`/`0x005962b0`/`0x00596310` release
/// the same shape. Field offsets (per the decompiled field accesses):
///
/// - `+0x00` `obj`   — object pointer; its refcount lives at `obj + 0x04`
/// - `+0x04` `a`     — payload dword, copied verbatim by `0x005961e0`
/// - `+0x08` `b`     — payload dword; `0x005961b0` seeds it with
///   `DAT_00bbb8d8` (pinned `1.0f` from the image in `math.rs` line 17)
/// - `+0x0c` `owner` — owner pointer; its refcount lives at `owner + 0x1c`
///
/// The byte cell at `+0x10` is modeled separately: `0x005961b0`/`0x005961e0`
/// *store* one byte there, while `0x005961e0` *reads* a whole dword at the
/// source's `+0x10` and truncates to char — so the dst cell is `u8` and the
/// src side is a raw `u32`.
pub struct RefHandle {
    /// `+0x00`
    pub obj: u32,
    /// `+0x04`
    pub a: u32,
    /// `+0x08`
    pub b: u32,
    /// `+0x0c`
    pub owner: u32,
}

/// Default init: `+0x04 = 0; +0x00 = 0; +0x08 = DAT_00bbb8d8; +0x0c = 0;
/// byte at +0x10 = param_2`. The `fill` parameter is the caller-resolved
/// `DAT_00bbb8d8` dword (raw bits `0x3f80_0000` = 1.0f).
///
/// Row: `0x005961b0`.
pub fn init_ref_handle_default(h: &mut RefHandle, flag_cell: &mut u8, fill: u32, flag: u8) {
    h.a = 0;
    h.obj = 0;
    h.b = fill;
    h.owner = 0;
    *flag_cell = flag;
}

/// Handle assignment with addref:
/// `if (*src) ++*(LONG*)(*src + 4); dst[0] = *src; if (src[3]) ++*(LONG*)(src[3]
/// + 0x1c); dst[3] = src[3]; *(char*)(dst + 0x10) = (char)src[4]; dst[1] =
/// src[1]; dst[2] = src[2]`. Plain increments (the game's unchecked `INC`;
/// single-threaded deterministic core, same note as `audiohandle::try_acquire`).
///
/// The two refcount cells are the caller-resolved hops (`obj + 4`,
/// `owner + 0x1c`); `None` models the null pointer leg exactly as the game's
/// guard skips the increment.
///
/// Row: `0x005961e0`.
pub fn assign_ref_handle(
    dst: &mut RefHandle,
    dst_flag_byte: &mut u8,
    src: &RefHandle,
    src_flag_dword: u32,
    obj_refcount: Option<&mut i32>,
    owner_refcount: Option<&mut i32>,
) {
    debug_assert_eq!(src.obj == 0, obj_refcount.is_none(), "NULL obj ↔ no cell");
    debug_assert_eq!(src.owner == 0, owner_refcount.is_none(), "NULL owner ↔ no cell");
    if let Some(cell) = obj_refcount {
        *cell += 1;
    }
    dst.obj = src.obj;
    if let Some(cell) = owner_refcount {
        *cell += 1;
    }
    dst.owner = src.owner;
    *dst_flag_byte = (src_flag_dword & 0xff) as u8;
    dst.a = src.a;
    dst.b = src.b;
}

/// Gated linear int search: buffer at `this + 4`, count `this + 8`, key
/// `*param_2`; returns the first index whose dword equals the key, `-1` on
/// miss. Quirk pinned: when the gate **byte** at `this + 0xc` is zero the
/// function returns `0` (not `-1`) — the outer guard short-circuits before
/// the miss assignment.
///
/// Row: `0x005964f0`.
pub fn find_index_gated(gate: u8, data: &[u32], count: i32, key: u32) -> i32 {
    if gate != 0 {
        if 0 < count {
            let mut i: i32 = 0;
            let mut p: usize = 0;
            loop {
                if data[p] == key {
                    return i;
                }
                i += 1;
                p += 1;
                if i >= count {
                    break;
                }
            }
        }
        return -1;
    }
    0
}

/// `removeAt`-style erase: guard `count <= index` → 0 (signed); else
/// decrement the count cell at `this + 0x10` and shift `data[index + 1 ..
/// count]` one slot left in the buffer at `this + 4` (the game's per-dword
/// `*(buf + k*4) = *(buf + 4 + k*4)` loop; the vacated last slot is left
/// stale below `count`), returning 1. A negative `index` passes the signed
/// guard and walks **before** the buffer in the game (UB) — the mirror
/// panics.
///
/// Row: `0x00596520`.
pub fn erase_at_shift(data: &mut [u32], count: &mut i32, index: i32) -> u32 {
    if *count <= index {
        return 0;
    }
    let n = count.wrapping_sub(1);
    *count = n;
    if index < n {
        let mut k = index;
        loop {
            data[slot(k)] = data[slot(k.wrapping_add(1))];
            k += 1;
            if k >= n {
                break;
            }
        }
    }
    1
}

/// Slot-table addref: the game computes `entry = **(int **)(*(this + 4) +
/// idx * 4)` — two hops: slot cell `P = (this + 4)[idx]`, then `entry = *P`
/// — and when `entry != 0` increments the refcount at `entry + 4`. The slot
/// block arrives as `table[idx]` = the dword block at `P`; the `entry + 4`
/// cell is caller-resolved (`None` models the null-entry guard, pairing
/// asserted as in `audiohandle::set_handle`).
///
/// Row: `0x00596670`.
pub fn addref_slot_entry(table: &[&[u32]], index: i32, entry_refcount: Option<&mut i32>) {
    let entry = table[slot(index)][0];
    debug_assert_eq!(entry == 0, entry_refcount.is_none(), "NULL entry ↔ no cell");
    if let Some(cell) = entry_refcount {
        *cell += 1;
    }
}

/// Slot-table read `**(undefined4 **)(*(this + 4) + idx * 4)`: the dword at
/// `P + 0x0` where `P = (this + 4)[idx]`.
///
/// Row: `0x00596690`.
pub fn slot_entry_cell(table: &[&[u32]], index: i32) -> u32 {
    table[slot(index)][0]
}

/// Slot-table float read `*(float *)(P + 4)`, `P = (this + 4)[idx]` (the
/// `float10` return type is just the x87 register widening; value preserved).
///
/// Row: `0x005966c0`.
pub fn slot_float_at_4(table: &[&[u32]], index: i32) -> f32 {
    f32::from_bits(table[slot(index)][1])
}

/// Slot-table float read `*(float *)(P + 8)`.
///
/// Row: `0x005966f0`.
pub fn slot_float_at_8(table: &[&[u32]], index: i32) -> f32 {
    f32::from_bits(table[slot(index)][2])
}

/// Slot-table owner fetch with addref: `Q = *(int *)(P + 0xc)` where
/// `P = (this + 4)[idx]`; when `Q != 0` the game increments the refcount at
/// `Q + 0x1c` (caller-resolved cell, `None` = null guard), then returns `Q`.
///
/// Row: `0x00596740`.
pub fn slot_owner_cell_addref(table: &[&[u32]], index: i32, owner_refcount: Option<&mut i32>) -> u32 {
    let q = table[slot(index)][3];
    debug_assert_eq!(q == 0, owner_refcount.is_none(), "NULL owner ↔ no cell");
    if let Some(cell) = owner_refcount {
        *cell += 1;
    }
    q
}

/// Slot-table owner read, no refcount touch: `*(undefined4 *)(P + 0xc)`.
///
/// Row: `0x00596760`.
pub fn slot_owner_cell(table: &[&[u32]], index: i32) -> u32 {
    table[slot(index)][3]
}

/// Linear float search: buffer at `this + 4` (count `this + 0x10`), exact
/// `==` compare against `*param_2` (so a NaN key matches nothing); returns
/// the first index or `-1`; a non-positive count skips the scan entirely.
///
/// Row: `0x00596800`.
pub fn find_float_index(data: &[f32], count: i32, key: f32) -> i32 {
    if 0 < count {
        let mut i: i32 = 0;
        let mut p: usize = 0;
        loop {
            if data[p] == key {
                return i;
            }
            i += 1;
            p += 1;
            if i >= count {
                break;
            }
        }
    }
    -1
}

/// Vector `operator==`: counts (both `+0x08`) must match, then every dword
/// of the two buffers (`+0x04`) compares equal. The body walks with the
/// pointer-diff trick `iVar3 = other_base - this_base;
/// *(int *)(iVar3 + (int)this_ptr)` — semantically just parallel element
/// access; counts `<= 0` skip the loop (empty ⇒ equal).
///
/// Rows: `0x00596920`.
pub fn int_buffers_equal(a: &[u32], a_count: i32, b: &[u32], b_count: i32) -> bool {
    if a_count != b_count {
        return false;
    }
    if 0 < a_count {
        let mut i: i32 = 0;
        let mut p: usize = 0;
        loop {
            if a[p] != b[p] {
                return false;
            }
            i += 1;
            p += 1;
            if i >= a_count {
                break;
            }
        }
    }
    true
}

/// Ungated linear int search: buffer at `this + 4`, count `this + 0x10`;
/// first index equal to `*param_2` or `-1`. Same shape as `find_index_gated`
/// (`0x005964f0`) without the gate-byte quirk — kept as its own mirror
/// because the count cell lives at a different offset (`0x10` vs `8`).
///
/// Row: `0x005969c0`.
pub fn find_int_index(data: &[u32], count: i32, key: u32) -> i32 {
    if 0 < count {
        let mut i: i32 = 0;
        let mut p: usize = 0;
        loop {
            if data[p] == key {
                return i;
            }
            i += 1;
            p += 1;
            if i >= count {
                break;
            }
        }
    }
    -1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertex_buffer_size_formula() {
        // ctor sentinel state (stride 0, range -1..-1) → bare header.
        assert_eq!(dx_vertex_buffer_bytes(-1, -1, 0), 0x20);
        // 10 indices × stride 3 × 4 + 0x20.
        assert_eq!(dx_vertex_buffer_bytes(0, 9, 3), 10 * 3 * 4 + 0x20);
        // Stride × 4 wraps into the negative half exactly like the game.
        assert_eq!(dx_vertex_buffer_bytes(0, 0, 0x2000_0000), i32::MIN + 0x20);
        // The range diff wraps too: (MIN - 1) = MAX, +1 wraps back to MIN;
        // MIN * 4 wraps to 0, leaving the bare 0x20 header.
        assert_eq!(dx_vertex_buffer_bytes(1, i32::MIN, 1), 0x20);
    }

    #[test]
    fn range_sentinel_init_feeds_index_formulas() {
        let mut b = [0xa5a5_a5a5u32; 6];
        init_range_sentinel_block(&mut b);
        assert_eq!(b, [0, 0, 0, 0xffff_ffff, 0xffff_ffff, 0]);
        // the sentinel pair sits at +0x0c/+0x10 (indices 3/4) — exactly the
        // cells 0x00595cc0 and 0x00595dc0 read: empty range ⇒ 0x19/0x20
        assert_eq!(dx_index_header_bytes(b[3] as i32, b[4] as i32), 0x19);
        assert_eq!(dx_index_buffer_bytes(b[3] as i32, b[4] as i32), 0x20);
    }

    #[test]
    fn index_header_sign_trick() {
        // empty: v = 8 → 1 dword-group + 0x18.
        assert_eq!(dx_index_header_bytes(0, 0), 0x19);
        // v = -5 (high-low = -13): (-5 + 7) >> 3 = 0 → truncation toward zero.
        assert_eq!(dx_index_header_bytes(13, 0), 0x18);
        // v = -13 (high-low = -21): (-13 + 7) >> 3 = -1 → 0x18 - 1.
        assert_eq!(dx_index_header_bytes(21, 0), 0x17);
        // diff 7 ⇒ v = 15; (15 + 0) >> 3 = 1 (still one group + 0x18).
        assert_eq!(dx_index_header_bytes(0, 7), 0x19);
        // wrapping diff: MIN - 1 wraps to MAX, +8 wraps to MIN+7 = -2147483641;
        // the sign trick adds 7 (negative), arithmetic >> 3 gives -268435455,
        // +0x18 ⇒ -268435431 (0xf000_0019)
        assert_eq!(dx_index_header_bytes(1, i32::MIN), -268435431);
    }

    #[test]
    fn index_buffers_init_and_size() {
        let mut h = [9u32; 7];
        zero_block7(&mut h);
        assert_eq!(h, [0; 7]);
        assert_eq!(dx_index_header_bytes(0, 0), 0x19);

        let mut b = [7u32; 6];
        zero_block6(&mut b);
        assert_eq!(b, [0; 6]);
        assert_eq!(dx_index_buffer_bytes(0, 0), 0x1c);
        assert_eq!(dx_index_buffer_bytes(2, 3), 2 * 3 * 4 + 0x1c);
        // wrapping chain: 0x4000_0000 * 2 = MIN, MIN * 4 = 0 → bare header
        assert_eq!(dx_index_buffer_bytes(0x4000_0000, 2), 0x1c);
        // negative count mirrors the signed product
        assert_eq!(dx_index_buffer_bytes(-2, 3), -24 + 0x1c);
    }

    #[test]
    fn sorted_key_cursor_walks_and_flags() {
        // ascending masked values, target 6 stops after 5 (i=2) then steps
        // back to 1; entry 5 has no flag bit.
        let mut cur = 0i32;
        assert!(!sorted_key_cursor_advance(&[3, 5, 7, 9], 4, &mut cur, 6));
        assert_eq!(cur, 1);
        // target below everything: first read masks 3 > -1? no start move;
        // loop breaks immediately; i = -1 → clamp 0.
        let mut cur = 0i32;
        assert!(!sorted_key_cursor_advance(&[3, 5, 7, 9], 4, &mut cur, -1));
        assert_eq!(cur, 0);
        // flag bit survives the mask: entry 0 = 0x8000_0003 (masked 3 <= 3
        // ⇒ start at 1; 3 < 5 break; back to 0) → flag true.
        let mut cur = 0i32;
        assert!(sorted_key_cursor_advance(&[0x8000_0003, 5], 2, &mut cur, 3));
        assert_eq!(cur, 0);
        // target past the end: cursor ends count-1 after the step-back.
        let mut cur = 0i32;
        assert!(!sorted_key_cursor_advance(&[1, 2], 2, &mut cur, 100));
        assert_eq!(cur, 1);
    }

    #[test]
    #[should_panic]
    fn sorted_key_cursor_oob_panics() {
        let mut cur = 5i32; // first read is unchecked in the game (UB)
        sorted_key_cursor_advance(&[1, 2], 2, &mut cur, 0);
    }

    #[test]
    fn plain_setters() {
        let mut c = 1u32;
        set_4(&mut c, 0xbeef);
        assert_eq!(c, 0xbeef);
        set_8(&mut c, 0xdead);
        assert_eq!(c, 0xdead);
    }

    #[test]
    fn ref_handle_default_init() {
        let mut h = RefHandle { obj: 9, a: 7, b: 6, owner: 5 };
        let mut flag = 0u8;
        init_ref_handle_default(&mut h, &mut flag, 1.0f32.to_bits(), 3);
        assert_eq!((h.obj, h.a, h.b, h.owner), (0, 0, 1.0f32.to_bits(), 0));
        assert_eq!(flag, 3); // param_2 byte lands in the +0x10 cell
    }

    #[test]
    fn ref_handle_assign_bumps_both_refs_and_truncates_flag() {
        let src = RefHandle { obj: 0x1234, a: 0xabcd_1111, b: 0x2222_2222, owner: 0x9999 };
        let mut dst = RefHandle { obj: 1, a: 2, b: 3, owner: 4 };
        let mut dst_flag = 0u8;
        let mut obj_rc = 1i32;
        let mut owner_rc = 7i32;
        assign_ref_handle(
            &mut dst,
            &mut dst_flag,
            &src,
            0x1234_abcd,
            Some(&mut obj_rc),
            Some(&mut owner_rc),
        );
        assert_eq!((dst.obj, dst.a, dst.b, dst.owner), (0x1234, 0xabcd_1111, 0x2222_2222, 0x9999));
        // (char) truncation keeps only the low byte
        assert_eq!(dst_flag, 0xcd);
        assert_eq!((obj_rc, owner_rc), (2, 8));
        // NULL legs: no cells, no panic
        let nsrc = RefHandle { obj: 0, a: 0, b: 0, owner: 0 };
        assign_ref_handle(&mut dst, &mut dst_flag, &nsrc, 0, None, None);
        assert_eq!((dst.obj, dst.owner), (0, 0));
    }

    #[test]
    fn gated_find_quirk_returns_zero_when_gate_clear() {
        let data = [10u32, 20, 30];
        // gate clear: returns 0 even though the key is absent (game quirk)
        assert_eq!(find_index_gated(0, &data, 3, 99), 0);
        // gate set: normal miss
        assert_eq!(find_index_gated(1, &data, 3, 99), -1);
        assert_eq!(find_index_gated(1, &data, 3, 20), 1);
        // count <= 0 never scans, miss even if data would match
        assert_eq!(find_index_gated(1, &data, 0, 10), -1);
        assert_eq!(find_index_gated(1, &data, -5, 10), -1);
        // count smaller than the slice: only the first `count` entries search
        assert_eq!(find_index_gated(1, &data, 1, 20), -1);
    }

    #[test]
    fn erase_at_shift_semantics() {
        let mut data = [1u32, 2, 3, 4];
        let mut count = 4i32;
        assert_eq!(erase_at_shift(&mut data, &mut count, 1), 1);
        assert_eq!(data, [1, 3, 4, 4]); // vacated last slot stays stale
        assert_eq!(count, 3);
        // index == count - 1: count drops, nothing shifts
        assert_eq!(erase_at_shift(&mut data, &mut count, 2), 1);
        assert_eq!((data, count), ([1, 3, 4, 4], 2));
        // signed guard
        assert_eq!(erase_at_shift(&mut data, &mut count, 2), 0);
        assert_eq!((data, count), ([1, 3, 4, 4], 2));
        // count == 0: guard `0 <= index` fires for index >= 0
        let mut zero = 0i32;
        assert_eq!(erase_at_shift(&mut [0u32; 1], &mut zero, 0), 0);
    }

    #[test]
    #[should_panic]
    fn erase_at_negative_index_panics() {
        // game walks before the buffer (UB); mirror panics (faithful-quirk)
        let mut data = [1u32, 2];
        let mut count = 2i32;
        erase_at_shift(&mut data, &mut count, -1);
    }

    #[test]
    fn slot_table_family_reads_and_addrefs() {
        let a = [0x0000_1111u32, 0x3f80_0000, 0xc000_0000, 0xffff_0000];
        let b = [0u32, 2, 3, 0]; // zero entry/owner cells feed the `None` addref guards
        let table = [&a[..], &b[..]];
        // 0x00596690: dword at P + 0x0 (double hop resolved via inner slice)
        assert_eq!(slot_entry_cell(&table, 0), 0x0000_1111);
        // 0x005966c0 / 0x005966f0: floats at P + 4 / P + 8
        assert_eq!(slot_float_at_4(&table, 0), 1.0f32);
        assert_eq!(slot_float_at_8(&table, 0), -2.0f32);
        // 0x00596760: owner dword at P + 0xc, no side effects
        assert_eq!(slot_owner_cell(&table, 0), 0xffff_0000);
        // 0x00596740: owner fetch bumps rc at Q + 0x1c
        let mut q_rc = 5i32;
        assert_eq!(slot_owner_cell_addref(&table, 0, Some(&mut q_rc)), 0xffff_0000);
        assert_eq!(q_rc, 6);
        // null owner leg: no cell, returns 0
        assert_eq!(slot_owner_cell_addref(&table, 1, None), 0);
        // 0x00596670: entry = *P (= P[0]); nonzero → bump entry + 4
        let mut e_rc = 0i32;
        addref_slot_entry(&table, 0, Some(&mut e_rc));
        assert_eq!(e_rc, 1);
        // P[0] == 0 for slot 1: guard skips the increment
        addref_slot_entry(&table, 1, None);
    }

    #[test]
    #[should_panic]
    fn slot_index_out_of_range_panics() {
        let a = [1u32, 2, 3, 4];
        let table = [&a[..]];
        slot_entry_cell(&table, -1); // unchecked base + idx*4 in the game (UB)
    }

    #[test]
    fn float_find_exact_equality_and_nan() {
        let data = [1.5f32, -0.0, 4.0];
        assert_eq!(find_float_index(&data, 3, 4.0), 2);
        assert_eq!(find_float_index(&data, 3, 0.0), 1); // -0.0 == 0.0 like C
        assert_eq!(find_float_index(&data, 3, f32::NAN), -1); // NaN matches nothing
        assert_eq!(find_float_index(&data, 2, 4.0), -1); // count bounds the scan
        assert_eq!(find_float_index(&data, 0, 1.5), -1);
        assert_eq!(find_float_index(&data, -3, 1.5), -1);
    }

    #[test]
    fn int_buffers_equal_mirrors_operator_eq() {
        let a = [1u32, 2, 3, 4];
        let b = [1u32, 2, 3, 4];
        let c = [1u32, 2, 3, 5];
        assert!(int_buffers_equal(&a, 4, &b, 4));
        assert!(!int_buffers_equal(&a, 4, &c, 4));
        assert!(!int_buffers_equal(&a, 4, &b, 3)); // count cell compare first
        // counts <= 0 skip the loop: equal without touching buffers
        assert!(int_buffers_equal(&a, 0, &c, 0));
        assert!(int_buffers_equal(&a, -7, &c, -7));
    }

    #[test]
    fn ungated_find_mirrors_gated_minus_one_side() {
        let data = [7u32, 8, 9];
        assert_eq!(find_int_index(&data, 3, 9), 2);
        assert_eq!(find_int_index(&data, 3, 6), -1);
        assert_eq!(find_int_index(&data, 2, 9), -1);
        assert_eq!(find_int_index(&data, 0, 7), -1);
    }
}
