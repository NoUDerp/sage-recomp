//! Game-options slot cluster (`OptSlot` entry + `OptSet` container)
//! — the 0x007ff000 window: map/multiplayer player-slot options.
//! GUI band (rows already tagged `Math`/`Network` keep their tag).
//!
//! `OptSet` (container): +0x08 slot bitmask, +0x10 d10 (byte +0x10
//! tested by 0x007ff2d6/312; bytes +0x10/+0x11 written by
//! 0x007ff268/296), +0x18..+0x34 eight slot pointers, +0x38/+0x3C
//! pair, +0xcc byte[16]. `OptSlot` (entry): +0x04 kind
//! {0,1,2..5,6}, +0x08/+0x09 byte flags, +0x0C/+0x10 key dwords
//! (+0x10 double-used as bytes on the *container* side), +0x18
//! selection (-2 sentinel = unset), +0x1C selection value,
//! +0x24/28/2C fallback chain (-1/-2 sentinels), +0x30 UnicodeString,
//! +0x38/3C pair, +0x50..+0x5C classification quad, +0x60 byte,
//! +0x64 mask, +0x1A4 active byte, +0x1A8 AsciiString label.
//! Full defaults live in 0x007ff516/0x007fff99 (both BLOCKED on /GX
//! skip shells — see Wave-38 docs note).
//!
//! The game dispatches "current slot index" through the container
//! vtable slot +0x34 (`getCurrentSlot`, used by 0x007ff7b7/3b5/3e3/
//! 9e2); modelled here as the `cur_slot` field — every observed
//! override returns a stored index.
//!
//! Ghidra register folding runs throughout this cluster: `this`/ECX
//! leaks through `extraout_ECX` and the `uVar>>0x20` artifacts, and
//! `param_1 == -0x18` is a folded null-owner check (`slotptr−0x18 ==
//! −0x18`). Two rows read the caller's entry EAX on a guard branch
//! (0x007ff1da, 0x007ff312) — caller-dependent UB, modelled as
//! `false` with the quirk noted. 0x007ff987/9e2/3b5 deref slots
//! without null checks (game invariant: slots populated on these
//! paths) — the Rust mirror skips `None` with the same outcome under
//! the invariant.

use crate::math::pair_eq_low16;
use crate::string::{Ascii, StringBase, StringUtf16};

/// One option/slot entry (`OptSlot`). Fields limited to what this
/// cluster touches; layout offsets in the doc-comment.
#[derive(Clone, Debug, Default)]
pub struct OptSlot {
    /// +0x04 kind: {0,1,2..5,6}; 6 = locked/"closed" entry
    pub kind: i32,
    /// +0x08/+0x09 byte flags
    pub f8: u8,
    pub f9: u8,
    /// +0x0C key (lookups 0x007ff34a)
    pub key_c: i32,
    /// +0x10 key dword (lookup 0x007ff374; sweeps set −1)
    pub w10: i32,
    /// +0x18 selection; −2 sentinel = unset (0x007ff165)
    pub sel18: i32,
    /// +0x1C selection value
    pub val1c: i32,
    /// +0x24/+0x28/+0x2C fallback-chain ints (−1/−2 sentinels)
    pub fb24: i32,
    pub fb28: i32,
    pub fb2c: i32,
    /// +0x30 display name (Unicode)
    pub name_u: StringUtf16,
    /// +0x38/+0x3C pair (matched via `pair_eq_low16`)
    pub pair38: [u32; 2],
    /// +0x50 classification kind (0 = unset)
    pub cls50: u32,
    /// +0x54 classification operand
    pub cls54: u32,
    /// +0x58 classification operand
    pub cls58: i32,
    /// +0x5C selected id
    pub sel5c: i32,
    /// +0x60 byte gate, +0x64 mask (0x007ff457/496)
    pub b60: u8,
    pub mask64: i32,
    /// +0x1A4 active byte
    pub active1a4: u8,
    /// +0x1A8 ASCII label
    pub label1a8: StringBase<Ascii>,
}

impl OptSlot {
    /// 0x007ff0f1 — `kind == 6`.
    pub fn is_type6(&self) -> bool {
        self.kind == 6
    }

    /// 0x007ff127 — `kind` in {2,3,4,5}.
    pub fn is_type2345(&self) -> bool {
        matches!(self.kind, 2..=5)
    }

    /// 0x007ff0fb — kind in {2..6} AND active byte +0x1A4 set.
    pub fn is_active(&self) -> bool {
        (self.kind == 6 || matches!(self.kind, 2..=5)) && self.active1a4 != 0
    }

    /// 0x007ff165 — unset sentinel at +0x18.
    pub fn is_unset(&self) -> bool {
        self.sel18 == -2
    }

    /// 0x007ff0e7 — set selected id (+0x5C).
    pub fn set_sel5c(&mut self, v: i32) {
        self.sel5c = v;
    }

    /// 0x007ff328 — set classification kind (+0x50).
    pub fn set_cls50(&mut self, v: u32) {
        self.cls50 = v;
    }

    /// 0x007ff895 — clear +0x08 iff kind == 6.
    pub fn clear8_if6(&mut self) {
        if self.kind == 6 {
            self.f8 = 0;
        }
    }

    /// 0x007ff8a0 — set +0x09 iff kind == 6.
    pub fn set9_if6(&mut self, v: u8) {
        if self.kind == 6 {
            self.f9 = v;
        }
    }

    /// 0x007ff5f2 — label setter: `set` + `trim` + `toUpper` on
    /// +0x1A8.
    pub fn set_name(&mut self, s: &StringBase<Ascii>) {
        self.label1a8.set(s.str());
        self.label1a8.trim();
        self.label1a8.to_upper();
    }

    /// 0x007fff62 — kind 6 AND case-insensitive equality of +0x30
    /// against `other`; the game releases the ARGUMENT's buffer
    /// before returning (the param is a by-reference temp) — kept
    /// faithfully.
    pub fn name_eq6(&self, other: &mut StringUtf16) -> bool {
        let r = self.kind == 6 && self.name_u.compare_no_case_string_base(other) == 0;
        other.release_buffer();
        r
    }

    /// 0x007ffeff — same test against `key` translated to Unicode
    /// (0x00acb5f0 chain); both temps released at the tail (again
    /// including the caller's argument).
    pub fn name_eq6_translated(&self, key: &mut StringBase<Ascii>) -> bool {
        let mut t = StringUtf16::new();
        t.translate_ascii(key.str());
        let r = self.kind == 6 && self.name_u.compare_no_case_string_base(&t) == 0;
        t.release_buffer();
        key.release_buffer();
        r
    }

    /// 0x007ff145 — kind 6 AND `pair_eq_low16(self+0x38,
    /// optset+0x38)` (FUN_00648cbf mirrored in math.rs; the pair
    /// pointer arg arrives EDX-folded at call sites 0x007ff2d6/312).
    pub fn pair_test(&self, optset_pair: &[u32; 2]) -> bool {
        self.kind == 6 && pair_eq_low16(&self.pair38, optset_pair)
    }

    /// 0x007ff16f — classification compute over the +0x50 quad:
    /// kind 0 → `cls50 & ~0xFF`; kind 1 → 1; kind 2 →
    /// `side_id(1, sel5c, 0, 0)`; kind 3 → `side_id(cls54 with its
    /// low byte +2 (wrapping, no carry into the upper three bytes —
    /// Ghidra CONCAT31), cls58, 0, 0)`; any other kind → mask
    /// passthrough.
    pub fn compute_class(&self) -> i32 {
        match self.cls50 {
            0 => (self.cls50 & 0xffffff00) as i32,
            1 => 1,
            2 => side_id(1, self.sel5c, 0, 0),
            3 => {
                let a = (self.cls54 & 0xffffff00) | (self.cls54.wrapping_add(2) & 0xff);
                side_id(a as i32, self.cls58, 0, 0)
            }
            _ => (self.cls50 & 0xffffff00) as i32,
        }
    }

    /// 0x007ff870 — resolved value: the game consults the global
    /// options singleton `DAT_00e022f4` (non-null AND byte +0x1E) —
    /// passed here as `gate1e`; when the slot is not current/paired
    /// the fallback +0x28 wins, else the plain +0x10.
    pub fn value_b(&self, gate1e: bool, optset: &OptSet<'_>) -> i32 {
        if gate1e && !optset.is_current_or_paired(self) {
            self.fb28
        } else {
            self.w10
        }
    }
}

/// 0x0096bd91 — pure side/id arithmetic: `16*(a−c) − d + b`
/// (i32 wrapping; the game's `'\x10'` display artifact is `imul 16`).
pub fn side_id(a: i32, b: i32, c: i32, d: i32) -> i32 {
    (a.wrapping_sub(c))
        .wrapping_mul(16)
        .wrapping_sub(d)
        .wrapping_add(b)
}

/// Options container with its eight slot pointers (`OptSet`).
#[derive(Debug)]
pub struct OptSet<'a> {
    /// +0x08 slot bitmask
    pub mask8: u32,
    /// +0x10 dword; byte +0x10 gates the match scans, bytes
    /// +0x10/+0x11 written by 0x007ff268/296
    pub d10: i32,
    /// +0x38/+0x3C pair matched against slots
    pub pair38: [u32; 2],
    /// +0xcc byte[16] (memset by the blocked reset 0x007fff99)
    pub cc: [u8; 16],
    /// vtable+0x34 `getCurrentSlot`, modelled as a field
    pub cur_slot: i32,
    /// +0x18..+0x34 slot pointers
    pub slots: [Option<&'a mut OptSlot>; 8],
}

impl<'a> OptSet<'a> {
    /// index of `other` among the slots, −1 if absent (loop skeleton
    /// of 0x007ff7b7).
    fn index_of(&self, other: &OptSlot) -> i32 {
        for (i, s) in self.slots.iter().enumerate() {
            if let Some(s) = s {
                if std::ptr::eq(&**s, other) {
                    return i as i32;
                }
            }
        }
        -1
    }

    /// 0x007ff2be — guarded slot read: `idx<0 || 7<idx` → null.
    pub fn get_slot(&self, idx: i32) -> Option<&OptSlot> {
        if idx < 0 || idx > 7 {
            return None;
        }
        self.slots[idx as usize].as_ref().map(|s| &**s)
    }

    /// 0x007ff29f — same with the (Ghidra-folded) null-owner guard.
    pub fn get_slot_owner(set: Option<&'a OptSet<'a>>, idx: i32) -> Option<&'a OptSlot> {
        match set {
            None => None,
            Some(_) if idx < 0 || idx > 7 => None,
            Some(s) => s.slots[idx as usize].as_ref().map(|sl| &**sl),
        }
    }

    /// 0x007ff332 — guarded slot store (`-1 < idx < 8`).
    pub fn set_slot(&mut self, idx: i32, slot: Option<&'a mut OptSlot>) {
        if idx > -1 && idx < 8 {
            self.slots[idx as usize] = slot;
        }
    }

    /// 0x007ff1fe — enable slot bit in the +0x08 mask
    /// (`1u32 << (i & 31)` — the &31 is a faithful quirk; guard
    /// `-1 < i < 8` makes it moot).
    pub fn enable_slot_bit(&mut self, i: i32) {
        if i > -1 && i < 8 {
            self.mask8 |= 1u32 << (i as u8 & 0x1f);
        }
    }

    /// 0x007ff1da — test slot bit. Quirk: the guard branch returns
    /// the caller's entry EAX with its low byte cleared
    /// (`in_EAX & 0xffffff00`) — caller-dependent, modelled `false`.
    pub fn slot_bit_test(&self, i: i32) -> bool {
        if i < 0 || i > 7 {
            return false;
        }
        (1u32 << (i as u8 & 0x1f) & self.mask8) != 0
    }

    /// 0x007ff1c2 — any byte of +0xcc..+0xdb set? (the `CONCAT31`
    /// return shape makes this a plain 1/0).
    pub fn any_scan_cc(&self) -> bool {
        self.cc.iter().any(|&b| b != 0)
    }

    /// 0x007ff268 — byte +0x10 = 1, +0x11 = 0 (dword view:
    /// low half := 0x0001).
    pub fn flag10_set(&mut self) {
        self.d10 = (self.d10 & !0xffff) | 0x0001;
    }

    /// 0x007ff296 — byte +0x10 = 0, +0x11 = 0.
    pub fn flag10_clear(&mut self) {
        self.d10 &= !0xffff;
    }

    /// byte +0x10 of the container (tested by 0x007ff2d6/312).
    pub fn flag10(&self) -> u8 {
        (self.d10 & 0xff) as u8
    }

    /// 0x007ff218 — count of populated-and-active slots.
    pub fn count_active(&self) -> i32 {
        let mut n = 0;
        for i in 0..8 {
            if let Some(s) = self.get_slot(i) {
                if s.is_active() {
                    n += 1;
                }
            }
        }
        n
    }

    /// 0x007ff2d6 — index of the first populated slot pair-matching
    /// the container pair, −1 (requires byte +0x10 set).
    pub fn first_match(&self) -> i32 {
        if self.flag10() == 0 {
            return -1;
        }
        for i in 0..8 {
            if let Some(s) = self.get_slot(i) {
                if s.pair_test(&self.pair38) {
                    return i;
                }
            }
        }
        -1
    }

    /// 0x007ff312 — byte +0x10 gate, then pair-test of SLOT 0:
    /// disasm `lea eax,[ecx+0x38]; mov ecx,[ecx+0x18]; call
    /// 0x007ff145` (Ghidra folded the slot load into the register
    /// chain). Guard path is `xor al,al` — keeps caller EAX's upper
    /// bytes (caller-dependent, modelled `false`). Null slot 0 →
    /// the game would pair-test a null receiver reading +4/+0x38 —
    /// invariant slots populated; modelled `false`.
    pub fn any_match(&self) -> bool {
        if self.flag10() == 0 {
            return false; // game: caller-entry EAX & ~0xff (UB)
        }
        match self.get_slot(0) {
            Some(s) => s.pair_test(&self.pair38),
            None => false,
        }
    }

    /// 0x007ff34a — any slot (except index `exclude`) with +0x0C ==
    /// `key`.
    pub fn has_key_c(&self, key: i32, exclude: i32) -> bool {
        for i in 0..8 {
            if let Some(s) = self.get_slot(i) {
                if s.key_c == key && i != exclude {
                    return true;
                }
            }
        }
        false
    }

    /// 0x007ff374 — twin of 0x007ff34a on +0x10.
    pub fn has_key_10(&self, key: i32, exclude: i32) -> bool {
        for i in 0..8 {
            if let Some(s) = self.get_slot(i) {
                if s.w10 == key && i != exclude {
                    return true;
                }
            }
        }
        false
    }

    /// 0x007ff39e — sweep: slot +0x10 := −1 for all non-null slots
    /// (owner-guarded getter 0x007ff29f; game iterates 0..8 with the
    /// index riding the getter-chain register artifact).
    pub fn clear_w10(&mut self) {
        for i in 0..8 {
            if let Some(s) = &mut self.slots[i] {
                s.w10 = -1;
            }
        }
    }

    /// 0x007ff457 — true unless some slot is active with
    /// classification kind set AND its +0x64 mask covered by the
    /// +0x60 byte gate (`mask64 & (b60!=0 ? −1 : 0) == 0`); null
    /// owner (folded guard) → true.
    pub fn no_classified_active(set: Option<&OptSet<'_>>) -> bool {
        let Some(s) = set else { return true };
        for i in 0..8 {
            if let Some(sl) = s.get_slot(i) {
                let cover = if sl.b60 != 0 { !0i32 } else { 0 };
                if sl.is_active() && sl.cls50 != 0 && (sl.mask64 & cover) == 0 {
                    return false;
                }
            }
        }
        true
    }

    /// 0x007ff496 — single-index twin: true when the slot is NOT a
    /// "classified active with covered mask" entry. Deviation: the
    /// game skips the negative-index guard (reads below the array —
    /// UB); the mirror treats out-of-range as true like the
    /// `7 < idx` branch.
    pub fn classified_ok(set: Option<&OptSet<'_>>, idx: i32) -> bool {
        let Some(s) = set else { return true };
        if idx > 7 {
            return true;
        }
        let Some(sl) = s.slots.get(idx as usize).and_then(|x| x.as_ref()) else {
            return true;
        };
        if !sl.is_active() || sl.cls50 == 0 {
            return true;
        }
        let cover = if sl.b60 != 0 { !0i32 } else { 0 };
        (sl.mask64 & cover) != 0
    }

    /// 0x007ff987 — count slots that are kind 0 or active (game
    /// derefs null slots — invariant says non-null on this path;
    /// `None` counted as the kind-0 read would be anyway? No: the
    /// game would fault. Counted as NOT-counted; noted).
    pub fn count_inactive_or_type0(&self) -> i32 {
        let mut n = 0;
        for i in 0..8 {
            if let Some(sl) = self.get_slot(i) {
                if sl.kind == 0 || sl.is_active() {
                    n += 1;
                }
            }
        }
        n
    }

    /// 0x007ff9b4 — sweep: slot[0].+0x08 := 1 unconditionally (if
    /// populated); slots 1..8 with kind 6 get +0x08 := 0.
    pub fn sweep8(&mut self) {
        if let Some(s0) = &mut self.slots[0] {
            s0.f8 = 1;
        }
        for i in 1..8 {
            if let Some(s) = &mut self.slots[i] {
                if s.kind == 6 {
                    s.f8 = 0;
                }
            }
        }
    }

    /// 0x007ff7b7 — is `other` the current slot, or does it share
    /// the current slot's +0x1C selection value (>= −1), or is the
    /// current slot's fallback +0x2C the −2 sentinel?
    pub fn is_current_or_paired(&self, other: &OptSlot) -> bool {
        let idx = self.index_of(other);
        let cur = self.cur_slot;
        let Some(s) = self.get_slot(cur) else {
            return false;
        };
        if idx < 0 {
            return false;
        }
        if idx == cur || (other.val1c == s.val1c && other.val1c > -1) {
            return true;
        }
        s.fb2c == -2
    }

    /// 0x007ff3b5 — some non-current slot has kind 6.
    pub fn unique_selection(&self) -> bool {
        for i in 0..8 {
            if i == self.cur_slot {
                continue;
            }
            if let Some(s) = self.get_slot(i) {
                if s.kind == 6 {
                    return true;
                }
            }
        }
        false
    }

    /// 0x007ff3e3 — every other active slot shares the base
    /// selection value: base = current slot's +0x1C when `cur >= 0`,
    /// else the FIRST active slot's +0x1C (−1 when none); mismatch or
    /// negative selection → false.
    pub fn consistent_selection(&self) -> bool {
        let cur = self.cur_slot;
        let base: i32 = if cur < 0 {
            let mut b = -1;
            for i in 0..8 {
                if let Some(s) = self.get_slot(i) {
                    if s.is_active() {
                        b = s.val1c;
                        break;
                    }
                }
            }
            b
        } else {
            match self.get_slot(cur) {
                Some(s) => s.val1c,
                None => -1,
            }
        };
        for i in 0..8 {
            if i as i32 == cur {
                continue;
            }
            if let Some(s) = self.get_slot(i) {
                if s.is_active() && (s.val1c < 0 || s.val1c != base) {
                    return false;
                }
            }
        }
        true
    }

    /// 0x007ff9e2 — any non-current slot kind 6 ⇒ 0; otherwise 1 if
    /// some non-current type-2..5 slot is NOT current-or-paired;
    /// 0 otherwise. (Game derefs null slots; invariant non-null —
    /// `None` skipped.)
    pub fn some_difficulty_unpaired(&self) -> u8 {
        let mut u: u8 = 0;
        for i in 0..8 {
            if i as i32 == self.cur_slot {
                continue;
            }
            if let Some(s) = self.get_slot(i) {
                if s.kind == 6 {
                    return 0;
                }
                if s.is_type2345() && !self.is_current_or_paired(s) {
                    u = 1;
                }
            }
        }
        u
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::string::{Ascii, StringBase, Utf16};

    fn slot(kind: i32) -> OptSlot {
        OptSlot {
            kind,
            active1a4: 1,
            ..Default::default()
        }
    }

    #[test]
    fn predicates_and_sentinels() {
        let mut s = slot(6);
        assert!(s.is_type6() && s.is_active() && !s.is_type2345());
        s.active1a4 = 0;
        assert!(!s.is_active()); // kind 6 but inactive
        s.sel18 = -2;
        assert!(s.is_unset());
        s.set_sel5c(3);
        s.set_cls50(2);
        assert_eq!((s.sel5c, s.cls50), (3, 2));
        s.clear8_if6();
        s.set9_if6(1);
        assert_eq!((s.f8, s.f9), (0, 1));
    }

    #[test]
    fn classification_math() {
        assert_eq!(side_id(1, 5, 0, 0), 16 + 5);
        assert_eq!(side_id(0, 0, -1, -2), 16 + 2);
        let mut s = slot(3);
        s.cls50 = 3;
        s.cls54 = 0x00abcd_ff; // low byte 0xff +2 wraps to 0x01, no carry
        s.cls58 = 7;
        assert_eq!(s.compute_class(), (0x00abcd_01) as i32 * 16 + 7);
        s.cls50 = 4;
        assert_eq!(s.compute_class(), 0); // 4 & ~0xFF = 0
        s.cls50 = 0x204;
        assert_eq!(s.compute_class(), 0x200); // mask keeps bytes ≥1
    }

    #[test]
    fn pair_test_uses_low16() {
        let mut s = slot(6);
        s.pair38 = [0xdead_beef, 0x1234_5678];
        let p = [0xdead_beef, 0x9abc_5678]; // word0 exact, word1 low16
        assert!(s.pair_test(&p));
        s.kind = 2;
        assert!(!s.pair_test(&p));
    }

    #[test]
    fn container_guarded_access() {
        let mut a = slot(2);
        let mut b = slot(6);
        let mut set = OptSet {
            mask8: 0,
            d10: 0x1122_3344,
            pair38: [0; 2],
            cc: [0; 16],
            cur_slot: 0,
            slots: [
                Some(&mut a),
                Some(&mut b),
                None,
                None,
                None,
                None,
                None,
                None,
            ],
        };
        assert_eq!(set.get_slot(-1).map(|s| s.kind), None);
        assert_eq!(set.get_slot(8).map(|s| s.kind), None);
        assert_eq!(set.get_slot(1).map(|s| s.kind), Some(6));
        set.set_slot(1, None);
        assert!(set.slots[1].is_none());
        set.set_slot(8, None); // guard swallows

        set.enable_slot_bit(3);
        assert!(set.slot_bit_test(3) && !set.slot_bit_test(4));
        assert_eq!(set.flag10(), 0x44);
        set.flag10_clear();
        assert_eq!(set.d10, 0x1122_0000);
        set.flag10_set();
        assert_eq!(set.d10, 0x1122_0001);

        set.cc[7] = 1;
        assert!(set.any_scan_cc());

        // sweeps
        set.clear_w10();
        assert_eq!(set.get_slot(0).unwrap().w10, -1);
        set.sweep8();
        assert_eq!(set.get_slot(0).unwrap().f8, 1); // slot0 unconditional

        assert_eq!(set.count_active(), 1); // a active kind2; b None now
    }

    #[test]
    fn match_scans() {
        let mut a = slot(6);
        a.pair38 = [0x1111_2222, 0x3333_4444];
        let mut set = OptSet {
            mask8: 0,
            d10: 1, // byte +0x10 set
            pair38: [0x1111_2222, 0x7777_4444],
            cc: [0; 16],
            cur_slot: 0,
            slots: [Some(&mut a), None, None, None, None, None, None, None],
        };
        assert_eq!(set.first_match(), 0);
        assert!(set.any_match());
        set.d10 = 0;
        assert_eq!(set.first_match(), -1);
        assert!(!set.any_match());

        set.slots[0].as_mut().unwrap().key_c = 5;
        set.slots[0].as_mut().unwrap().w10 = 9;
        assert!(set.has_key_c(5, 1) && !set.has_key_c(5, 0));
        assert!(set.has_key_10(9, -1) && !set.has_key_10(8, -1));
    }

    #[test]
    fn current_pairing_and_selection() {
        let mut cur = slot(3);
        cur.val1c = 4;
        let mut other = slot(3);
        other.val1c = 4;
        let mut locked = slot(6);
        locked.val1c = 4; // keeps consistent_selection base intact
        let mut set = OptSet {
            mask8: 0,
            d10: 0,
            pair38: [0; 2],
            cc: [0; 16],
            cur_slot: 0,
            slots: [
                Some(&mut cur),
                Some(&mut other),
                Some(&mut locked),
                None,
                None,
                None,
                None,
                None,
            ],
        };
        assert!(set.is_current_or_paired(set.get_slot(0).unwrap())); // idx == cur
        assert!(set.is_current_or_paired(set.get_slot(1).unwrap())); // same val1c
        assert!(set.unique_selection()); // slot 2 kind6 non-current
        assert!(set.consistent_selection()); // other active val1c==base
        assert_eq!(set.some_difficulty_unpaired(), 0); // kind6 present

        // classified twins
        let mut cls = slot(2);
        cls.cls50 = 1;
        cls.mask64 = 0; // uncovered → the "classified active" trigger
        cls.b60 = 1;
        set.slots[3] = Some(&mut cls);
        assert!(!OptSet::no_classified_active(Some(&set)));
        assert!(!OptSet::classified_ok(Some(&set), 3));
        let mut cov = slot(2);
        cov.cls50 = 1;
        cov.mask64 = 0b101; // covered by the +0x60 gate
        cov.b60 = 1;
        cov.active1a4 = 1;
        set.slots[3] = Some(&mut cov);
        assert!(OptSet::classified_ok(Some(&set), 3));
        assert!(OptSet::no_classified_active(Some(&set)));
    }

    #[test]
    fn name_ops_round_trip() {
        let mut s = slot(6);

        // build "  hero  " via Ascii bytes
        let raw = *b"  hero  ";
        let mut arg = StringBase::new();
        arg.set(&raw.iter().copied().map(Ascii).collect::<Vec<_>>());
        s.set_name(&arg);
        let want: Vec<Ascii> = b"HERO".as_slice().iter().copied().map(Ascii).collect();
        assert_eq!(s.label1a8.str(), want.as_slice()); // trimmed + upper

        let u: Vec<Utf16> = "HERO".encode_utf16().map(Utf16).collect();
        s.name_u.set(&u); // +0x30 display name
        let mut q = StringUtf16::new();
        q.set(&u);
        assert!(s.name_eq6(&mut q)); // releases q's buffer (quirk)

        let mut key = StringBase::new();
        key.set(&raw[2..6].iter().copied().map(Ascii).collect::<Vec<_>>()); // "hero"
        assert!(s.name_eq6_translated(&mut key));
        assert!(key.is_empty()); // caller's arg released too
    }
}
