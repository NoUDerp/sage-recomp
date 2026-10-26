//! Wave 43: big-holder **FX accessor one-liners** — 16 rows from band
//! `0x006a98d1`–`0x006aa32f`, all members of one large mutable class (cells run
//! to at least `this + 0x7c4`; module tag context FX). The same band also hosts
//! the KindOf name/bit machinery mirrored in `flagname` (`0x006aa2cf`,
//! `0x006aa2f9`), so several of these accessors sit on flag-mask blocks.
//!
//! As in `drawdata`, every mirror takes the *referenced cells themselves*
//! (`&mut` / value); the caller resolves field offsets and pointer hops.
//!
//! Row -> function map:
//!
//! | Row          | Function           | Game shape (field offsets)                                          |
//! |--------------|--------------------|---------------------------------------------------------------------|
//! | `0x006a98d1` | `vector_element`   | 0x60-byte element pointer, vector `this + 0x7c4`, count `this + 0x7c0` |
//! | `0x006a9cca` | `set_cell_via_ptr` | store via holder: `*(*(this + 0x2dc) + 0x1c)` iff holder non-NULL   |
//! | `0x006a9d98` | `set_byte`         | `this + 0x33c = value`                                              |
//! | `0x006a9e25` | `set_cell`         | `this + 0x314 = value`                                              |
//! | `0x006a9efb` | `sub_clamp_zero`   | `this + 0x31c -= amount`, negatives clamped to `0`                  |
//! | `0x006aa031` | `find_node`        | chain walk from `*(this + 0x9c)`, key node `+4`, next node `+0xc`   |
//! | `0x006aa08e` | `gated_positive`   | gate byte `+0xa8`, mask `+0xa4`, signed count `+0xa0`               |
//! | `0x006aa0cd` | `set_cell`         | `this + 0x6f8 = value`                                              |
//! | `0x006aa123` | `select_cell`      | selector -> `+0xac` / `+0xb0` / `+0xb4`, else `0`                   |
//! | `0x006aa176` | `indexed_or_zero`  | bounded read `this + 0x708 + idx * 4`, out-of-range -> `0`          |
//! | `0x006aa201` | `byte_at`          | unbounded byte read `this + 0x340 + offset` (offset is the FIRST arg) |
//! | `0x006aa231` | `both_bytes_zero`  | `this + 0x33a == 0 && this + 0x734 == 0`                            |
//! | `0x006aa245` | `byte_via_ptr`     | byte at `*(this + 0x34) + 0x151`, NULL holder -> `0`                |
//! | `0x006aa257` | `set_cell`         | `this + 0x750 = value`                                              |
//! | `0x006aa292` | `masks_covered`    | 32-slot subset test `(cells[i] & masks[i]) == masks[i]`             |
//! | `0x006aa32f` | `equal`            | STL `equal`: range `[first, last)` vs sequence, bool                |
//!
//! Quirks pinned by the game and mirrored exactly:
//! - The element getter rejects out-of-range with NULL (Rust `None`), but the
//!   indexed dword getter rejects with the *value* `0` — a stored `0` is
//!   indistinguishable from the rejection; the mirror returns `0` identically.
//! - `0x006aa292` decompiles to page-truncated pointer arithmetic as the return
//!   (`masks_ptr & 0xffffff00`, low byte `1` on full coverage); every caller
//!   (`0x0080c804`, `0x008b4130`, `0x008808f2`) tests only the low byte, so the
//!   mirror returns `bool` — that bit is the entire observable semantics.
//! - `0x006aa031` decompiles as `void`, yet callers store its return
//!   (`0x006adac3`: `iVar2 = FUN_006aa031(param_2)`): a typed-away node
//!   pointer; the mirror returns the hit node (or `None` for the NULL end).
//! - Game indexing without checks (`byte_at`, and any block borrowed shorter
//!   than the code-baked bound or the `count` cell) is UB in the game; the
//!   mirrors use plain Rust indexing and panic instead (faithful-quirk policy).
//! - Subtraction in `0x006a9efb` wraps like C `int` *before* the clamp; the
//!   mirror uses `wrapping_sub` then clamps, so overflow scenarios match bit
//!   for bit. Single-threaded deterministic core: no interlocked variants here.

/// Element size of the `0x006a98d1` vector: `0x60` bytes = 24 dwords.
const ELEMENT_DWORDS: usize = 0x60 / 4;

/// Bounded element lookup in the owner's fixed-stride vector: the game returns
/// NULL for `idx < 0` or `idx >= *(this + 0x7c0)`, else `this + 0x7c4 + idx * 0x60`.
/// The caller borrows the vector payload (the block starting at `this + 0x7c4`);
/// a `count` cell larger than the block is UB in the game, so the mirror panics
/// on the slice range (faithful-quirk policy).
///
/// Row: `0x006a98d1` (count at `this + 0x7c0`, payload at `this + 0x7c4`).
pub fn vector_element<'a>(count: i32, block: &'a mut [u32], index: i32) -> Option<&'a mut [u32]> {
    if index < 0 || count <= index {
        return None;
    }
    let start = (index as usize) * ELEMENT_DWORDS;
    Some(&mut block[start..start + ELEMENT_DWORDS])
}

/// Store through a possibly-NULL holder: the game checks `*(this + 0x2dc) != 0`
/// first and only then writes the cell at holder `+ 0x1c`. The caller resolves
/// the hop; `None` models the NULL holder (write skipped, exactly like the guard).
///
/// Row: `0x006a9cca` (holder at `this + 0x2dc`, target at holder `+ 0x1c`).
pub fn set_cell_via_ptr(cell: Option<&mut u32>, value: u32) {
    if let Some(cell) = cell {
        *cell = value;
    }
}

/// Plain byte store into an owner cell (game takes `undefined1`).
///
/// Row: `0x006a9d98` (cell at `this + 0x33c`).
pub fn set_byte(cell: &mut u8, value: u8) {
    *cell = value;
}

/// Plain dword store into an owner cell; three rows in this band are identical
/// one-liners on different offsets, so they share this mirror (each cited).
///
/// Rows: `0x006a9e25` (cell at `this + 0x314`), `0x006aa0cd` (cell at
/// `this + 0x6f8`), `0x006aa257` (cell at `this + 0x750`).
pub fn set_cell(cell: &mut u32, value: u32) {
    *cell = value;
}

/// Signed countdown cell: `*cell -= amount` in wrapping C-`int` arithmetic, then
/// any negative result is clamped to `0`. The clamp happens *after* the wrap, so
/// e.g. subtracting `i32::MIN` wraps negative and lands on `0`, not on a huge
/// positive — the mirror reproduces that bit-for-bit.
///
/// Row: `0x006a9efb` (cell at `this + 0x31c`).
pub fn sub_clamp_zero(cell: &mut i32, amount: i32) {
    let value = cell.wrapping_sub(amount);
    *cell = if value < 0 { 0 } else { value };
}

/// Node shape for the singly-linked chain scanned by the game: key cell at node
/// `+ 4`, next pointer at node `+ 0xc`. Implemented by the caller's node type.
pub trait LinkedNode {
    /// Key stored at node `+ 4`.
    fn node_key(&self) -> i32;
    /// Next node from node `+ 0xc`; `None` is the game's NULL terminator.
    fn node_next(&self) -> Option<&Self>;
}

/// Chain walk: start at the head (`*(this + 0x9c)` in the game; `None` = NULL),
/// stop at the first node whose key equals `key`, `None` at the NULL end. The
/// game body decompiles as `void` but callers store the walked-out pointer — a
/// typed-away node return (see module header quirk list).
///
/// Row: `0x006aa031` (head at `this + 0x9c`).
pub fn find_node<'a, N: LinkedNode>(head: Option<&'a N>, key: i32) -> Option<&'a N> {
    let mut node = head?;
    loop {
        if node.node_key() == key {
            return Some(node);
        }
        node = node.node_next()?;
    }
}

/// Three-cell predicate: the signed count at `+ 0xa0` must be `> 0`, *unless* the
/// gate byte at `+ 0xa8` is set AND the mask at `+ 0xa4` is clear — that combo
/// short-circuits to `false` regardless of the count (body: `gate == 0 || mask !=
/// 0` selects the count test; the else branch returns `false`).
///
/// Row: `0x006aa08e` (cells at `this + 0xa8` / `this + 0xa4` / `this + 0xa0`).
pub fn gated_positive(gate_byte: u8, mask_cell: u32, count_cell: i32) -> bool {
    (gate_byte == 0 || mask_cell != 0) && count_cell > 0
}

/// Selector read over three adjacent cells (`+ 0xac`, `+ 0xb0`, `+ 0xb4`):
/// selector `1`/`2`/`3` picks the cell; *everything* else returns `0`. The
/// default is indistinguishable from a cell that legitimately stores `0`.
///
/// Row: `0x006aa123` (cells at `this + 0xac` / `this + 0xb0` / `this + 0xb4`).
pub fn select_cell(selector: i32, first: u32, second: u32, third: u32) -> u32 {
    match selector {
        1 => first,
        2 => second,
        3 => third,
        _ => 0,
    }
}

/// Bounded indexed read from the owner's 10-dword table at `this + 0x708`: the
/// bound is baked in the code (`idx < 0 || 9 < idx` rejects), and the rejection
/// value is `0`, not NULL — a stored `0` cannot be told apart. A borrowed block
/// shorter than 10 dwords is caller error (the game reads on blindly = UB); the
/// mirror panics (faithful-quirk policy).
///
/// Row: `0x006aa176` (table at `this + 0x708`, indices `0..=9`).
pub fn indexed_or_zero(block: &[u32], index: i32) -> u32 {
    if index < 0 || index > 9 {
        0
    } else {
        block[index as usize]
    }
}

/// Unchecked byte read: the game computes `owner + 0x340 + offset` with NO
/// bounds or sign check (negative offsets walk before the block — UB); the
/// mirror panics via Rust indexing. Argument-order quirk: the game's FIRST
/// parameter is the offset and the SECOND is the object.
///
/// Row: `0x006aa201` (block base at `this + 0x340`).
pub fn byte_at(block: &[u8], offset: i32) -> u8 {
    block[offset as usize]
}

/// Emptiness predicate over two gate bytes: true iff BOTH are zero (byte
/// truthiness, not `== 1`).
///
/// Row: `0x006aa231` (bytes at `this + 0x33a` and `this + 0x734`).
pub fn both_bytes_zero(first: u8, second: u8) -> bool {
    first == 0 && second == 0
}

/// Byte read through a possibly-NULL sub-object: the game returns `0` when
/// `*(this + 0x34)` is NULL, else the byte at sub `+ 0x151`. `None` models the
/// NULL holder and yields the same `0`.
///
/// Row: `0x006aa245` (holder at `this + 0x34`, byte at sub `+ 0x151`).
pub fn byte_via_ptr(cell: Option<&u8>) -> u8 {
    match cell {
        Some(cell) => *cell,
        None => 0,
    }
}

/// 32-slot mask-coverage test over two parallel dword blocks: true iff every
/// slot satisfies `(cells[i] & masks[i]) == masks[i]` (i.e. each mask's set bits
/// are a subset of the owner's block — a "has all these flags" query over the
/// wide bitset that the `flagname` KindOf rows index). The game's return value
/// is pointer arithmetic (`masks_ptr & 0xffffff00 | ok`, see module header);
/// callers test only the low byte, which is exactly this `bool`.
///
/// Row: `0x006aa292` (owner block walked dword-by-dword, mask cursor parallel).
pub fn masks_covered(cells: &[u32; 32], masks: &[u32; 32]) -> bool {
    (0..32).all(|i| (cells[i] & masks[i]) == masks[i])
}

/// STL `equal` over a half-open range `[first, last)` against a sequence:
/// `true` iff the leading `range.len()` sequence elements compare equal. Empty
/// range -> `true` immediately (the game checks `first != last` before the loop).
/// Sequence elements past the range end are ignored; a sequence shorter than the
/// range is UB in the game (reads past it) and panics here. The body is typed
/// `float*` (a `std::equal<float>` instantiation), but the template shape is
/// generic, so the mirror is too — with the game's float semantics preserved:
/// `NaN` never compares equal, so any `NaN` pair fails the test.
///
/// Row: `0x006aa32f` (caller `0x006aac9e` passes `begin, end, other.begin`).
pub fn equal<T: PartialEq>(range: &[T], seq: &[T]) -> bool {
    (0..range.len()).all(|i| range[i] == seq[i])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vector_element_bounds_and_stride() {
        // 4 elements of 24 dwords (0x60 bytes) each.
        let mut block = [0u32; 4 * 24];
        assert!(vector_element(4, &mut block, 4).is_none()); // idx == count -> NULL
        assert!(vector_element(4, &mut block, -1).is_none()); // negative -> NULL
        assert!(vector_element(0, &mut block, 0).is_none()); // empty vector
        let e = vector_element(4, &mut block, 3).expect("idx == count-1 is valid");
        assert_eq!(e.len(), ELEMENT_DWORDS);
        e[23] = 0xabcd;
        // element 3 starts at dword 3 * 24: write landed in the last slot of the block
        assert_eq!(block[4 * 24 - 1], 0xabcd);
    }

    #[test]
    fn indexed_and_selector_defaults_are_zero() {
        let block = [10u32, 11, 12, 13, 14, 15, 16, 17, 18, 0];
        assert_eq!(indexed_or_zero(&block, 9), 0); // last valid slot; stored 0 == rejection sentinel
        assert_eq!(indexed_or_zero(&block, 8), 18);
        assert_eq!(indexed_or_zero(&block, 10), 0); // bound is inclusive 9
        assert_eq!(indexed_or_zero(&block, -1), 0);
        assert_eq!(select_cell(1, 7, 8, 9), 7);
        assert_eq!(select_cell(2, 7, 8, 9), 8);
        assert_eq!(select_cell(3, 7, 8, 9), 9);
        assert_eq!(select_cell(0, 7, 8, 9), 0);
        assert_eq!(select_cell(4, 7, 8, 9), 0);
        assert_eq!(select_cell(-1, 7, 8, 9), 0);
    }

    #[test]
    fn equal_empty_true_mismatch_false() {
        let empty: [f32; 0] = [];
        assert!(equal(&empty, &[1.0f32, 2.0])); // first == last -> true, seq irrelevant
        assert!(equal(&[1.0f32, 2.0, 3.0], &[1.0, 2.0, 3.0, 9.0])); // seq tail past the range is ignored
        assert!(!equal(&[1.0f32, 2.0], &[1.0, 3.0]));
        assert!(!equal(&[f32::NAN], &[f32::NAN])); // NaN != NaN in the game's float compare
        assert!(equal(&[7u32, 8], &[7u32, 8])); // dword instantiation of the same template
    }

    #[test]
    fn masks_covered_requires_subset_in_every_slot() {
        let all_on = [0xffff_ffffu32; 32];
        let none = [0u32; 32];
        assert!(masks_covered(&all_on, &none)); // empty mask set trivially covered
        assert!(masks_covered(&[1u32, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 1 << 31], &{
            let mut m = [0u32; 32];
            m[0] = 1;
            m[31] = 1 << 31;
            m
        }));
        // a miss in the LAST slot still fails: the game checks all 32 slots (uVar1 < 0x20)
        let mut tail = [0u32; 32];
        tail[31] = 0x8000_0001;
        assert!(!masks_covered(&[0xffff_ffe0u32; 32], &tail));
    }

    #[test]
    fn find_node_walks_until_match_or_null() {
        struct Node {
            key: i32,
            next: Option<Box<Node>>,
        }
        impl LinkedNode for Node {
            fn node_key(&self) -> i32 {
                self.key
            }
            fn node_next(&self) -> Option<&Node> {
                self.next.as_deref()
            }
        }
        let chain = Node {
            key: 1,
            next: Some(Box::new(Node {
                key: 2,
                next: Some(Box::new(Node { key: 3, next: None })),
            })),
        };
        assert_eq!(find_node(Some(&chain), 2).map(Node::node_key), Some(2));
        assert_eq!(find_node(Some(&chain), 3).map(Node::node_key), Some(3));
        assert!(find_node(Some(&chain), 4).is_none()); // walked off the NULL end
        assert!(find_node::<Node>(None, 1).is_none()); // NULL head (this + 0x9c == 0)
    }

    #[test]
    fn predicates_pin_the_gate_tables() {
        // 0x006aa08e: gate clear -> the count decides...
        assert!(gated_positive(0, 0, 1));
        assert!(!gated_positive(0, 0, 0));
        assert!(!gated_positive(0, 0, -1)); // signed compare, not truthiness
        // ...gate set: mask non-zero -> count decides; mask clear -> false regardless
        assert!(gated_positive(1, 0xff, 2));
        assert!(!gated_positive(1, 0, 2));
        // 0x006aa231: both bytes must be zero; 0x80 is just as non-zero as 1
        assert!(both_bytes_zero(0, 0));
        assert!(!both_bytes_zero(1, 0));
        assert!(!both_bytes_zero(0, 0x80));
        // 0x006aa245: NULL holder and a real byte
        assert_eq!(byte_via_ptr(None), 0);
        let sub = 0x11u8;
        assert_eq!(byte_via_ptr(Some(&sub)), 0x11);
    }

    #[test]
    fn stores_guards_and_clamp() {
        let mut byte_cell = 0u8;
        set_byte(&mut byte_cell, 0x80);
        assert_eq!(byte_cell, 0x80);
        let mut dword = 0u32;
        set_cell(&mut dword, 0xdead_beef);
        assert_eq!(dword, 0xdead_beef);
        // 0x006a9cca: NULL holder skips the store entirely
        let mut via = 7u32;
        set_cell_via_ptr(Some(&mut via), 9);
        assert_eq!(via, 9);
        set_cell_via_ptr(None, 11);
        assert_eq!(via, 9);
        // 0x006aa201: plain block read at the requested offset (arg order quirk lives in the docs)
        let bytes = [1u8, 2, 3, 4];
        assert_eq!(byte_at(&bytes, 0), 1);
        assert_eq!(byte_at(&bytes, 3), 4);
        // 0x006a9efb: clamp after wrapping subtract
        let mut countdown = 10i32;
        sub_clamp_zero(&mut countdown, 4);
        assert_eq!(countdown, 6);
        sub_clamp_zero(&mut countdown, 100);
        assert_eq!(countdown, 0);
        let mut wrapped = 5i32;
        sub_clamp_zero(&mut wrapped, i32::MIN); // 5 - MIN wraps negative BEFORE the clamp
        assert_eq!(wrapped, 0);
        let mut already_negative = -5i32; // a cell dirtied elsewhere still clamps
        sub_clamp_zero(&mut already_negative, 3);
        assert_eq!(already_negative, 0);
    }
}
