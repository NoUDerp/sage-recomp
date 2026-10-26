//! Wave 45: the `0x00983726`–`0x00983ce0` run — bounds-checked predicates,
//! stores and STL copy machinery over two `std::vector` shapes owned by one
//! container class (ledger band tag `FX`, propagated by cluster; see the
//! identity note under *Band identity* below — the evidence points at
//! `GameLogic/Object/Contain/HordeContain`, not FX).
//!
//! Both vectors use the same three cursor cells of the owner: `+0x08` begin,
//! `+0x0c` end, `+0x14` capacity (raw dwords, i.e. pointers in the image).
//! The mirror borrows the storage — element `i` of the borrowed slice is the
//! game's `begin + i * stride` slot — and takes the cursor cells by value so
//! the count arithmetic stays exactly as the game computes it.
//!
//! Row -> function map:
//!
//! | Row          | Function                  | Game shape                                                |
//! |--------------|---------------------------|-----------------------------------------------------------|
//! | `0x00983726` | `dword_is_two`            | `vector<dword>`: guarded `*(begin + i*4) == 2`            |
//! | `0x00983750` | `set_dword_two`           | guarded `*(begin + i*4) = 2`                              |
//! | `0x00983772` | `set_dword_three`         | guarded `*(begin + i*4) = 3`                              |
//! | `0x00983a1e` | `assign_item`             | element `operator=`: 6 dwords + **byte** at `+0x10`       |
//! | `0x00983a4f` | `copy_items_forward`      | `_Copy` loop, 7 dwords per element, cursor-driven         |
//! | `0x00983aa9` | `copy_items_backward`     | `_Copy_backward`: count from cursors, descending copy     |
//! | `0x00983be6` | `item_state_is_one`       | `vector<0x1c>`: guarded `*(begin + i*0x1c) == 1`          |
//! | `0x00983c1a` | `item_state_is_two`       | guarded `*(begin + i*0x1c) == 2`                          |
//! | `0x00983c4e` | `item_f14_exceeds_global` | guarded `DAT_00dfe78c + 0x40 < *(elem + 0x14)` (unsigned) |
//! | `0x00983c88` | `set_item_state_two`      | guarded `*(begin + i*0x1c) = 2`                           |
//! | `0x00983cb4` | `set_item_state_three`    | guarded `*(begin + i*0x1c) = 3`                           |
//! | `0x00983ce0` | `set_item_flag`           | guarded **byte** store `*(elem + 0x10) = 1`               |
//!
//! Skipped rows (call chains / vtable dispatch / SEH — see the contract rule):
//! `0x00983794`, `0x009838b5`, `0x009838dc`, `0x00983902`, `0x00983933`,
//! `0x009839a7`, `0x009839f5`, `0x00983a72`, `0x00983a84`, `0x00983b11`,
//! `0x00983b37`, `0x00983b54`, `0x00983b6f`.
//! Of those, the `0x00983a1e`–`0x00983b54` run is MSVC STL machinery for the
//! same element type: `0x00983a72` = `_Constructor` (`if (src) *dst = *src`
//! over `assign_item`), `0x00983a84` / `0x00983b11` = the
//! `_Uninitialized_copy` cursor loops around it, `0x00983b37` / `0x00983b54` =
//! parameter-permuting thunks into `0x00983aa9` / `0x00983a84`. Their bodies
//! are only calls, so they are skipped; the copy width and stride they imply
//! are already pinned by `copy_items_forward` / `copy_items_backward`.
//!
//! Quirks pinned by the game and mirrored exactly:
//! - **Two different count formulas.** The stride-4 rows compute
//!   `(end - begin) >> 2` (arithmetic shift = floor), the stride-`0x1c` rows
//!   compute `(end - begin) / 0x1c` (signed division, truncates toward zero).
//!   For a corrupt `end < begin` the first yields a huge unsigned count
//!   (`0xfffffffe` for a `-8` delta) while the second yields `0`.
//! - **Out-of-range predicates do not return `false`.** The three
//!   `0x1c`-stride predicates return `in_EAX & 0xffffff00`: the *low byte* is
//!   guaranteed `0`, but the value is non-zero whenever the element count is
//!   `>= 0x100`, so callers that test truthiness see "true" on an
//!   out-of-range index (same masked-sentinel family as `flagname`'s
//!   `set_flag_by_name`). On the `(int)index < 0` path the game never
//!   assigns `in_EAX` at all, so everything above bit 7 is undefined there;
//!   the mirror returns the count-masked form and the tests pin only the
//!   guaranteed low byte for that path.
//! - **Stale upper bytes on the success path.** `0x00983be6` / `0x00983c1a`
//!   return Ghidra `CONCAT31((int3)((uint)*(int*)(this+8) >> 8), test)`, i.e.
//!   the low byte carries the boolean and bits 8..31 carry the `begin` dword
//!   with its low byte cleared. `0x00983c4e` has no such artifact — the
//!   compiler zeroed `EAX`, so that row returns a clean `0`/`1`.
//! - **Element copy is not a memcpy.** `assign_item` (`0x00983a1e`) writes the
//!   `+0x10` member as a **single byte**, so destination bytes `+0x11..+0x13`
//!   survive; the `0x00983a4f` / `0x00983aa9` block copies move all seven
//!   dwords, padding included. Tests pin the difference.
//! - `0x00983a4f` loops on **cursor inequality** (`param_1 != param_2`), so a
//!   byte distance that is not a whole multiple of `0x1c` (or is negative)
//!   walks the cursor past the end forever = UB; the mirror panics instead.
//!   `0x00983aa9` divides, so a non-multiple distance truncates silently —
//!   that truncation is real game behaviour and is pinned.
//! - `0x00983ce0`'s second game parameter is dead: the body never reads it.
//!
//! Band identity: the neighbouring row `0x009837fa` compares a name against
//! `s_HordeMeleeHoldGround_00c6fc14` (string `"HordeMeleeHoldGround"` at
//! `0x00c6fc14`); the data neighbourhood of this run also holds
//! `"HordeMeleeSwarm"` (`0x00c6fcc0`), `"HordeMeleeFormation"` (`0x00c6fdd0`),
//! `"std::deque"` (`0x00c6fd84`), `"Deque must be empty on load"`
//! (`0x00c6fd68`) and the debug path at `0x00c6ff80` =
//! `C:\projects\bfme2patch103\bfme2\Code\GameEngine\Source\GameLogic\Object\Contain\HordeContain\HordeMeleeAmoeba.cpp`.
//! So the ledger's `FX` tag on `0x00983726`–`0x009839f5` is cluster
//! propagation, not identity: the containers here are horde-member vectors
//! (`state` cell tested against `1`/`2`, written `2`/`3`, plus a `0x1c`
//! deadline-ish dword compared against the engine global `DAT_00dfe78c + 0x40`).

/// Element stride of the wide vector (`0x1c` = 7 dwords).
const ITEM_STRIDE: u32 = 0x1c;

/// The owner's three cursor cells, exactly as stored:
/// `+0x08` begin, `+0x0c` end, `+0x14` capacity (raw dwords — pointers in
/// the game image). No mirrored row in this run reads capacity; the
/// call-chain rows `0x009839a7` / `0x009839f5` pass it to the allocator.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VecHead {
    /// `this + 0x08` — first element (game pointer).
    pub begin: u32,
    /// `this + 0x0c` — one past the last element (game pointer).
    pub end: u32,
    /// `this + 0x14` — one past the end of the allocation (game pointer).
    pub cap: u32,
}

impl VecHead {
    /// Stride-4 element count as the game computes it:
    /// `(uint)((int)(end - begin) >> 2)` — wrapping dword subtraction, then an
    /// **arithmetic** shift, then the unsigned reinterpretation. A corrupt
    /// `end < begin` therefore yields a huge count, not a negative one.
    pub fn dword_count(&self) -> u32 {
        (self.end.wrapping_sub(self.begin) as i32 >> 2) as u32
    }

    /// Stride-`0x1c` element count as the game computes it:
    /// `(uint)((int)(end - begin) / 0x1c)` — signed division truncating
    /// toward zero, then the unsigned reinterpretation.
    pub fn item_count(&self) -> u32 {
        self.item_count_signed() as u32
    }

    /// The same division **before** the unsigned cast — the rows that gate on
    /// `0 < count` (the backward copy) work on the signed value.
    pub fn item_count_signed(&self) -> i32 {
        self.end.wrapping_sub(self.begin) as i32 / ITEM_STRIDE as i32
    }
}

/// One element of the wide vector: `0x1c` bytes / 7 dwords, laid out as the
/// bodies address it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HordeItem {
    /// `+0x00` state cell: tested `== 1` (`0x00983be6`) and `== 2`
    /// (`0x00983c1a`), written `2` (`0x00983c88`) and `3` (`0x00983cb4`).
    pub state: u32,
    /// `+0x04` payload (untouched by this run).
    pub f04: u32,
    /// `+0x08` payload (untouched by this run).
    pub f08: u32,
    /// `+0x0c` payload (untouched by this run).
    pub f0c: u32,
    /// `+0x10` flag byte — `0x00983ce0` stores `1` here as a **byte**.
    pub flag: u8,
    /// `+0x11..+0x13` alignment padding: copied by the block copies, never by
    /// `assign_item`.
    pub pad: [u8; 3],
    /// `+0x14` unsigned dword compared against the engine global
    /// `DAT_00dfe78c + 0x40` by `0x00983c4e` (deadline-shaped, unproven).
    pub f14: u32,
    /// `+0x18` payload (untouched by this run).
    pub f18: u32,
}

/// `0x00983726` — bounds-guarded `vector<dword>` element test.
///
/// The game rejects `(int)idx < 0` and `(uint)((end - begin) >> 2) <= idx`
/// with `false`; otherwise it reads `begin[idx]` and compares against `2`.
/// An index that passes the guard but falls outside the caller's slice is the
/// game's wild read (UB) — the mirror panics.
pub fn dword_is_two(head: &VecHead, data: &[u32], index: u32) -> bool {
    if (index as i32) < 0 || head.dword_count() <= index {
        return false;
    }
    data[index as usize] == 2
}

/// `0x00983750` — guarded `begin[idx] = 2`. Out-of-range or negative index
/// stores nothing (both guards are `&&`-joined in the game).
pub fn set_dword_two(head: &VecHead, data: &mut [u32], index: u32) {
    if -1 < (index as i32) && index < head.dword_count() {
        data[index as usize] = 2;
    }
}

/// `0x00983772` — guarded `begin[idx] = 3`; identical guard to `0x00983750`.
pub fn set_dword_three(head: &VecHead, data: &mut [u32], index: u32) {
    if -1 < (index as i32) && index < head.dword_count() {
        data[index as usize] = 3;
    }
}

/// `0x00983be6` — guarded wide-element state test against `1`.
///
/// In range: Ghidra `CONCAT31((int3)((uint)*(int*)(this+8) >> 8), test)` —
/// the `begin` dword's upper 24 bits OR'd with the boolean. Out of range:
/// `in_EAX & 0xffffff00` where `in_EAX` is the element count (or, on the
/// negative-index short circuit, an unassigned register — the mirror uses the
/// count form and callers can only ever rely on the zero low byte).
pub fn item_state_is_one(head: &VecHead, items: &[HordeItem], index: u32) -> u32 {
    let count = head.item_count();
    if (index as i32) < 0 || count <= index {
        return count & 0xffffff00;
    }
    (head.begin & 0xffffff00) | (items[index as usize].state == 1) as u32
}

/// `0x00983c1a` — guarded wide-element state test against `2`, same return
/// shape as `item_state_is_one` (`CONCAT31` upper bytes, masked sentinel).
pub fn item_state_is_two(head: &VecHead, items: &[HordeItem], index: u32) -> u32 {
    let count = head.item_count();
    if (index as i32) < 0 || count <= index {
        return count & 0xffffff00;
    }
    (head.begin & 0xffffff00) | (items[index as usize].state == 2) as u32
}

/// `0x00983c4e` — guarded `global < element.f14`, **unsigned**.
///
/// `global` is the dword at `DAT_00dfe78c + 0x40` (engine singleton block,
/// resolved by the caller — same block as the `fxflags` byte setters). Unlike
/// the two state tests this row returns a clean `0`/`1` in range: the compiler
/// zeroed `EAX`, so no stale `begin` bits leak out. Out of range the masked
/// sentinel still applies.
pub fn item_f14_exceeds_global(
    head: &VecHead,
    items: &[HordeItem],
    index: u32,
    global_dfe78c_40: u32,
) -> u32 {
    let count = head.item_count();
    if (index as i32) < 0 || count <= index {
        return count & 0xffffff00;
    }
    (global_dfe78c_40 < items[index as usize].f14) as u32
}

/// `0x00983c88` — guarded state store `begin[idx] = 2` (nested guards, as in
/// the game: the sign test wraps the range test).
pub fn set_item_state_two(head: &VecHead, items: &mut [HordeItem], index: u32) {
    if -1 < (index as i32) {
        if index < head.item_count() {
            items[index as usize].state = 2;
        }
    }
}

/// `0x00983cb4` — guarded state store `begin[idx] = 3`.
pub fn set_item_state_three(head: &VecHead, items: &mut [HordeItem], index: u32) {
    if -1 < (index as i32) {
        if index < head.item_count() {
            items[index as usize].state = 3;
        }
    }
}

/// `0x00983ce0` — guarded **byte** store `*(elem + 0x10) = 1`; the element's
/// padding bytes `+0x11..+0x13` are untouched. The game's second parameter is
/// never read, so the mirror omits it.
pub fn set_item_flag(head: &VecHead, items: &mut [HordeItem], index: u32) {
    if -1 < (index as i32) {
        if index < head.item_count() {
            items[index as usize].flag = 1;
        }
    }
}

/// `0x00983a1e` — element assignment (`__thiscall`, `param_1` = `this` =
/// destination). Six dwords move as dwords; the `+0x10` member moves as a
/// single **byte**, so the destination padding survives — observable against
/// the whole-element block copies.
pub fn assign_item(dst: &mut HordeItem, src: &HordeItem) {
    dst.state = src.state;
    dst.f04 = src.f04;
    dst.f08 = src.f08;
    dst.f0c = src.f0c;
    dst.flag = src.flag;
    dst.f14 = src.f14;
    dst.f18 = src.f18;
}

/// `0x00983a4f` — forward element copy (`std::_Copy` for this element type).
///
/// The game loops `while (param_1 != param_2)` moving 7 dwords per element, so
/// the element count is implied by the cursor pair, not by a count parameter.
/// A byte distance that is negative or not a multiple of `0x1c` makes the
/// cursor step past `param_2` and the loop run off into memory (UB); the
/// mirror panics on that cursor pair. Buffer slots the cursors point at but
/// that the caller's slices do not cover are the game's wild read/write and
/// panic here.
pub fn copy_items_forward(head: &VecHead, dst: &mut [HordeItem], src: &[HordeItem]) {
    let delta = head.end.wrapping_sub(head.begin) as i32;
    if delta < 0 || delta % ITEM_STRIDE as i32 != 0 {
        panic!(
            "0x00983a4f: cursor pair {:#010x}..{:#010x} is not a whole number of {ITEM_STRIDE:#x}-byte elements (game spins)",
            head.begin, head.end
        );
    }
    let count = (delta / ITEM_STRIDE as i32) as usize;
    let mut i = 0usize;
    while i < count {
        // Whole-element move: seven dwords, padding included.
        dst[i] = src[i];
        i += 1;
    }
}

/// `0x00983aa9` — backward element copy (`std::_Copy_backward`, used by the
/// vector's in-place shifts).
///
/// `count = ((int)param_2 - param_1) / 0x1c` is a **signed division** of the
/// cursor delta: a non-multiple delta truncates (pinned), a negative delta
/// gives `count < 0` so the `0 < count` gate skips the copy entirely. Elements
/// then move **descending** (`param_2` and `param_3` are pre-decremented),
/// which is what makes an overlapping downward shift safe — the mirror keeps
/// the descending order even though its borrows cannot alias.
///
/// `src_end` is the element index of the game's `param_3` cursor (one past the
/// last source element); the return is the game's post-loop `param_3`, also as
/// an element index (`src_end - count`, wrapping like the pointer arithmetic),
/// i.e. the start of the source range actually copied.
pub fn copy_items_backward(
    head: &VecHead,
    dst: &mut [HordeItem],
    src: &[HordeItem],
    src_end: usize,
) -> usize {
    let count = head.item_count_signed();
    if count > 0 {
        let mut k = count;
        loop {
            k -= 1;
            let j = k as usize;
            dst[j] = src[src_end - count as usize + j];
            if k == 0 {
                break;
            }
        }
    }
    src_end.wrapping_sub(count.max(0) as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Three dwords live at `begin .. begin+0xc`.
    fn three_dwords() -> VecHead {
        VecHead { begin: 0x1000, end: 0x100c, cap: 0x1010 }
    }

    #[test]
    fn dword_count_uses_arithmetic_shift_not_division() {
        // well-formed: 12 bytes -> 3 elements
        assert_eq!(three_dwords().dword_count(), 3);
        // corrupt container: -8 bytes -> -2 (shift floors) -> 0xfffffffe,
        // while the 0x1c family's division truncates to 0.
        let corrupt = VecHead { begin: 8, end: 0, cap: 0 };
        assert_eq!(corrupt.dword_count(), 0xffff_fffe);
        assert_eq!(corrupt.item_count(), 0);
        // a non-multiple delta truncates toward zero for the signed division
        let ragged = VecHead { begin: 0, end: 2 * ITEM_STRIDE + 5, cap: 0 };
        assert_eq!(ragged.item_count_signed(), 2);
    }

    #[test]
    fn dword_is_two_guards_negative_and_end_boundary() {
        let data = [2u32, 3, 7];
        assert!(dword_is_two(&three_dwords(), &data, 0));
        assert!(!dword_is_two(&three_dwords(), &data, 1)); // 3 != 2
        assert!(!dword_is_two(&three_dwords(), &data, 2)); // 7 != 2
        assert!(!dword_is_two(&three_dwords(), &data, 3)); // idx == count
        assert!(!dword_is_two(&three_dwords(), &data, u32::MAX)); // (int)-1
        // empty vector: every index, including 0, is rejected
        let empty = VecHead { begin: 0x1000, end: 0x1000, cap: 0x1010 };
        assert!(!dword_is_two(&empty, &data, 0));
    }

    #[test]
    fn dword_stores_respect_the_same_guard() {
        let mut data = [0u32; 3];
        set_dword_two(&three_dwords(), &mut data, 2);
        set_dword_three(&three_dwords(), &mut data, 0);
        set_dword_two(&three_dwords(), &mut data, 3); // rejected
        set_dword_three(&three_dwords(), &mut data, 0x8000_0000); // (int) < 0
        assert_eq!(data, [3, 0, 2]);
    }

    #[test]
    fn state_tests_reproduce_the_concat31_and_sentinel_bit_patterns() {
        // begin has a non-zero low byte: the stale upper bits must show the
        // begin dword with that low byte cleared.
        let head = VecHead { begin: 0x0040_d004, end: 0x0040_d004 + 2 * ITEM_STRIDE, cap: 0 };
        let items = [
            HordeItem { state: 1, ..HordeItem::default() },
            HordeItem { state: 2, ..HordeItem::default() },
        ];
        assert_eq!(item_state_is_one(&head, &items, 0), 0x0040_d001);
        assert_eq!(item_state_is_one(&head, &items, 1), 0x0040_d000); // bit clear, upper stale
        assert_eq!(item_state_is_two(&head, &items, 1), 0x0040_d001);
        assert_eq!(item_state_is_two(&head, &items, 0), 0x0040_d000);

        // out-of-range sentinel == count & 0xffffff00, so a >= 0x100-element
        // container reports *non-zero* (truthy) for an out-of-range index.
        let wide = VecHead { begin: 0x0040_d000, end: 0x0040_d000 + 0x101 * ITEM_STRIDE, cap: 0 };
        assert_eq!(item_state_is_one(&wide, &[], 0x101), 0x0000_0100);
        assert_eq!(item_state_is_two(&wide, &[], 0x102), 0x0000_0100);
        // count < 0x100 masks the sentinel down to exactly 0
        assert_eq!(item_state_is_one(&head, &items, 2), 0);

        // negative index: in_EAX is never assigned in the game, only the
        // cleared low byte is guaranteed.
        assert_eq!(item_state_is_one(&head, &items, u32::MAX) & 0xff, 0);
        assert_eq!(item_state_is_two(&wide, &[], 0x8000_0000) & 0xff, 0);
    }

    #[test]
    fn f14_compare_is_strictly_unsigned_and_returns_clean_bits() {
        let head = VecHead { begin: 0x0040_d004, end: 0x0040_d004 + 2 * ITEM_STRIDE, cap: 0 };
        let items = [
            HordeItem { f14: 10, ..HordeItem::default() },
            HordeItem { f14: 0xffff_fff0, ..HordeItem::default() },
        ];
        assert_eq!(item_f14_exceeds_global(&head, &items, 0, 9), 1);
        assert_eq!(item_f14_exceeds_global(&head, &items, 0, 10), 0); // strict <
        // unsigned: 0xffffffff is *not* below 0xffff_fff0
        assert_eq!(item_f14_exceeds_global(&head, &items, 1, 0xffff_ffff), 0);
        assert_eq!(item_f14_exceeds_global(&head, &items, 1, 10), 1);
        // in range there is no `CONCAT31` artifact: bits 8..31 stay clear,
        // unlike `item_state_is_one`/`item_state_is_two` on the same head
        assert_eq!(item_f14_exceeds_global(&head, &items, 0, 9) >> 8, 0);
        // out of range keeps the masked sentinel, negative keeps low byte 0
        assert_eq!(item_f14_exceeds_global(&wide_head(), &[], 0x101, 1), 0x100);
        assert_eq!(item_f14_exceeds_global(&head, &items, u32::MAX, 1) & 0xff, 0);
    }

    fn wide_head() -> VecHead {
        VecHead { begin: 0x0040_d000, end: 0x0040_d000 + 0x101 * ITEM_STRIDE, cap: 0 }
    }

    #[test]
    fn item_stores_and_flag_respect_guard_and_byte_width() {
        let head = VecHead { begin: 0, end: 2 * ITEM_STRIDE, cap: 3 * ITEM_STRIDE };
        let mut items = [HordeItem::default(); 2];
        items[1].pad = [7, 8, 9];
        set_item_state_two(&head, &mut items, 0);
        set_item_state_three(&head, &mut items, 1);
        set_item_flag(&head, &mut items, 1);
        set_item_state_two(&head, &mut items, 2); // idx == count: rejected
        set_item_state_three(&head, &mut items, 0x8000_0000); // negative
        set_item_flag(&head, &mut items, u32::MAX); // negative
        assert_eq!(items[0].state, 2);
        assert_eq!(items[1].state, 3);
        assert_eq!(items[1].flag, 1);
        assert_eq!(items[1].pad, [7, 8, 9]); // byte store, padding intact
        assert_eq!(items[0].flag, 0);
    }

    #[test]
    fn assign_item_moves_the_flag_as_a_byte_not_a_dword() {
        let src = HordeItem {
            state: 0xabcd_ef01,
            f04: 4,
            f08: 8,
            f0c: 0xc,
            flag: 0x11,
            pad: [1, 2, 3],
            f14: 0x14,
            f18: 0x18,
        };
        let mut dst = HordeItem {
            state: 0xdead_beef,
            flag: 0x99,
            pad: [7, 8, 9],
            f18: 0x77,
            ..HordeItem::default()
        };
        assign_item(&mut dst, &src);
        assert_eq!(dst.state, 0xabcd_ef01);
        assert_eq!((dst.f04, dst.f08, dst.f0c), (4, 8, 0xc));
        assert_eq!((dst.flag, dst.f14, dst.f18), (0x11, 0x14, 0x18));
        // the game stores `(undefined1)(param_1 + 4)`: dst padding survives
        assert_eq!(dst.pad, [7, 8, 9]);
    }

    #[test]
    fn forward_copy_moves_whole_elements_padding_included() {
        let head = VecHead { begin: 0x1c00, end: 0x1c00 + 2 * ITEM_STRIDE, cap: 0x1c60 };
        let src = [
            HordeItem { state: 1, flag: 0x11, pad: [1, 2, 3], f18: 0xaa, ..HordeItem::default() },
            HordeItem { state: 2, flag: 0x22, pad: [4, 5, 6], ..HordeItem::default() },
            HordeItem { state: 9, flag: 0x99, pad: [7, 7, 7], ..HordeItem::default() },
        ];
        let mut dst = [HordeItem { pad: [0xf0; 3], ..HordeItem::default() }; 3];
        copy_items_forward(&head, &mut dst, &src);
        assert_eq!(dst[0], src[0]);
        assert_eq!(dst[1], src[1]);
        // only two elements (cursor pair) — the third slot is untouched
        assert_eq!(dst[2], HordeItem { pad: [0xf0; 3], ..HordeItem::default() });
    }

    #[test]
    #[should_panic(expected = "0x00983a4f")]
    fn forward_copy_rejects_a_ragged_cursor_pair() {
        let head = VecHead { begin: 0, end: ITEM_STRIDE + 1, cap: 0 };
        let mut dst = [HordeItem::default(); 4];
        copy_items_forward(&head, &mut dst, &[]);
    }

    #[test]
    fn backward_copy_descends_truncates_and_returns_the_source_cursor() {
        // count = 2 from a well-formed pair, taken from the tail of `src`
        let head = VecHead { begin: 0x2000, end: 0x2000 + 2 * ITEM_STRIDE, cap: 0 };
        let src = [
            HordeItem { state: 0, ..HordeItem::default() },
            HordeItem { state: 1, ..HordeItem::default() },
            HordeItem { state: 2, ..HordeItem::default() },
        ];
        let mut dst = [HordeItem::default(); 3];
        assert_eq!(copy_items_backward(&head, &mut dst, &src, 3), 1);
        assert_eq!(dst[0].state, 1);
        assert_eq!(dst[1].state, 2);
        assert_eq!(dst[2].state, 0); // untouched

        // ragged cursor delta truncates (real game behaviour, no panic)
        let ragged = VecHead { begin: 0x2000, end: 0x2000 + 2 * ITEM_STRIDE + 5, cap: 0 };
        let mut dst2 = [HordeItem::default(); 2];
        assert_eq!(copy_items_backward(&ragged, &mut dst2, &src, 3), 1);
        assert_eq!((dst2[0].state, dst2[1].state), (1, 2));

        // negative cursor delta: `0 < count` gate skips everything and the
        // game's returned cursor is the incoming one
        let corrupt = VecHead { begin: 0x2040, end: 0x2000, cap: 0 };
        let mut dst3 = [HordeItem { state: 5, ..HordeItem::default() }; 2];
        assert_eq!(copy_items_backward(&corrupt, &mut dst3, &src, 3), 3);
        assert_eq!((dst3[0].state, dst3[1].state), (5, 5));
    }
}
