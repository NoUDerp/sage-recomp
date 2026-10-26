//! Wave 46: band `0x005540b0–0x00554520` (13 rows, 712 B).
//!
//! Orientation assigned the tag `CRT-LIB` (statically-linked CRT x87 math
//! wrappers), but the decompiled bodies show the band is **mixed**: two rows
//! are genuine CRT helper code, the other eleven are a game/MFC-style GDI
//! device-context and view-state wrapper cluster (evidence: Win32 GDI imports
//! `SelectObject`/`DeleteObject`/`DeleteDC` and Ghidra types `HDC`/`HGDIOBJ`
//! in `0x005543c0`/`0x005543e0`/`0x005544b0`/`0x00554520`; `__thiscall`
//! member convention with `this + 4` old-object / `this + 8` created-object /
//! `this + 0x10` DC fields — the textbook memory-DC RAII layout;
//! `0x005544f0`/`0x00554520` call the CRT delete thunks `FUN_0042fd60` /
//! `FUN_0042fd80`). See the parent-facing row report for the suggested tag
//! split.
//!
//! Rows → functions (mirrored rows only):
//!
//! | row | fn | semantics |
//! |---|---|---|
//! | `0x005540b0` | `floor_f32` | CRT float-precision `floor`: widen to double, `floor`, return as `float` |
//! | `0x005540d0` | `offset_rect_by_point` | `{l,t,r,b} += {x,y}` on a 4-float rect |
//! | `0x005542a0` | `is_private_use_range` | Unicode PUA band test `U+E01..U+E3A ∪ U+E3F..U+E5B` |
//! | `0x00554320` | `set_pair_at_0b4_if_changed` | compare/store float pair at `this+0xb4`/`this+0xb8`, `1` on change |
//! | `0x00554370` | `set_pair_at_0bc_if_changed` | same for the pair at `this+0xbc`/`this+0xc0` |
//!
//! Not mirrored (skipped per the wave skip rules; addresses and evidence):
//!
//! - `0x005541e0` — constructor whose core is an allocation:
//!   `FUN_0042fde0(param_2 * 2)`; `FUN_0042fde0` itself is a thin thunk through
//!   the global function pointer `DAT_00de0404` (`(*DAT_00de0404)(size, 2, 0)`
//!   — operator-new-style dispatch). The remainder stores the buffer at
//!   `*this`, the count at `this[1]`, zeroes `this[2]`, then calloc-style
//!   zeroes exactly `2*param_2` bytes (`(param_2 & 0x7fffffff) >> 1` dwords
//!   plus a `(2*param_2) & 3` byte tail) if the allocation succeeded.
//!   malloc/new + global-pointer dispatch → skip.
//! - `0x00554230` — every path dispatches through
//!   `(**(code **)(*DAT_00deda34 + 0x78))` (vtable `code**` through a global
//!   `DAT_`) and bumps the global counter `DAT_00deda98` → skip.
//! - `0x005542d0` — calls `(**(code **)*param_1)()` and
//!   `(**(code **)*puVar2)()` (vtable `code**` destroy/release), refcount
//!   churn at `param_2 + 4` and `this[0x13][1]` → skip.
//! - `0x005543c0` — calls `SelectObject((HDC)param_2, param_3)` (Win32 GDI OS
//!   call) into a `{HDC, old GDI object}` pair → skip.
//! - `0x005543e0` — one-shot `SelectObject(*this, this[1])` restore (OS call)
//!   → skip.
//! - `0x005544b0` — memory-DC teardown: `SelectObject(this[0x10], this[4])`,
//!   `DeleteObject(this[8])`, `DeleteDC(this[0x10])` — three GDI OS calls →
//!   skip.
//! - `0x005544f0` — destructor calling the CRT delete thunks `FUN_0042fd80`
//!   (`delete` array) on `*this` and, when `param_2 & 1`, `FUN_0042fd60`
//!   (`delete`) on `this` — explicitly listed skip helpers → skip.
//! - `0x00554520` — `0x005544b0` body plus conditional `FUN_0042fd60(this)`
//!   and return of `this` → skip.

/// CRT float-precision `floor` wrapper: the body widens the `float` argument
/// to `double`, calls the CRT double `floor`, and returns through the x87
/// register as `float`. Ghidra renders the return as `float10` (`(float10)
/// dVar1`) — an artifact of the `fld` ST(0) return ABI; the caller-visible
/// value is the floored `float`.
///
/// Exactness: `floor` of a `float` widened to `double` is an exactly
/// representable `float` (integers need no more mantissa than the input), so
/// the double round-trip is bit-exact and `f32::floor` is a faithful mirror:
/// round toward −∞, `floor(+0.0) = +0.0`, `floor(−0.0) = −0.0`, NaN is
/// returned unchanged (x87 `floor` leaves NaN in ST(0); any signaling NaN
/// arriving through the `float → double` widening would have been quieted
/// before this function's `floor`, same as Rust's conversion quieting —
/// [INFERENCE] on the SNaN quieting path), ±∞ pass through.
///
/// Row: `0x005540b0`.
pub fn floor_f32(x: f32) -> f32 {
    x.floor()
}

/// `void __thiscall` member of a 4-float rectangle class (`this` = `{left,
/// top, right, bottom}` at `+0x00/+0x04/+0x08/+0x0C`); `param_2` points at a
/// 2-float point `{x, y}`. The body adds `x` to `this[0]` and `this[2]` and
/// `y` to `this[1]` and `this[3]` — i.e. translate the rect by the point
/// (left/right by `x`, top/bottom by `y`); the operand order in each store is
/// commutative under IEEE. `float` adds are 32-bit (no x87 widening appears
/// in the body).
///
/// Row: `0x005540d0`.
pub fn offset_rect_by_point(rect: &mut [f32; 4], delta: &[f32; 2]) {
    rect[0] += delta[0];
    rect[1] += delta[1];
    rect[2] += delta[0];
    rect[3] += delta[1];
}

/// Predicate over a code point (the game's parameter is `ushort` — a
/// 16-bit `wchar`-style value): returns `1` exactly when the value lies in
/// `U+0E01..=U+0E3A` or `U+0E3F..=U+0E5B`, else `0`. These are Unicode
/// Private Use Area sub-bands, the shape used by CRT MBCS/codepage vendor
/// character validation; the body is the raw double-range guard:
/// `if ((c < 0xE01 || 0xE3A < c) && (c < 0xE3F || 0xE5B < c)) return 0;
/// return 1;`.
///
/// Row: `0x005542a0`.
pub fn is_private_use_range(c: u16) -> bool {
    !((c < 0xE01 || 0xE3A < c) && (c < 0xE3F || 0xE5B < c))
}

/// Receiver fields for the `0x00554320`/`0x00554370` setter pair, modeled at
/// the raw offsets the decompiled bodies touch (one object per row; sharing
/// this struct assumes the two receivers are the same class — plausible next
/// neighbors, not proven). The band context (memory-DC/GDI rows above) hints
/// window/viewport-extent-style pairs; the field names stay neutral.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ExtentsHolder {
    /// `this + 0xB4` (float)
    pub a_lo: f32,
    /// `this + 0xB8` (float)
    pub a_hi: f32,
    /// `this + 0xBC` (float)
    pub b_lo: f32,
    /// `this + 0xC0` (float)
    pub b_hi: f32,
}

/// `undefined1 __thiscall`: short-circuit `&&` compare of the stored pair
/// with the arguments; if both are equal return `0` (no store), else write
/// **both** floats and return `1`. A partial mismatch rewrites the whole pair
/// (even the field that already matched). IEEE equality quirks are faithful:
/// a stored NaN never compares equal (`1` + store), while `+0.0` vs `−0.0`
/// compare equal (`0`, the stored sign of zero is kept).
///
/// Row: `0x00554320`.
pub fn set_pair_at_0b4_if_changed(obj: &mut ExtentsHolder, v0: f32, v1: f32) -> bool {
    if obj.a_lo == v0 && obj.a_hi == v1 {
        return false;
    }
    obj.a_lo = v0;
    obj.a_hi = v1;
    true
}

/// Same member shape as [`set_pair_at_0b4_if_changed`] for the pair at
/// `this + 0xBC` / `this + 0xC0`.
///
/// Row: `0x00554370`.
pub fn set_pair_at_0bc_if_changed(obj: &mut ExtentsHolder, v0: f32, v1: f32) -> bool {
    if obj.b_lo == v0 && obj.b_hi == v1 {
        return false;
    }
    obj.b_lo = v0;
    obj.b_hi = v1;
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floor_pins_bit_patterns() {
        assert_eq!(floor_f32(2.5).to_bits(), 2.0f32.to_bits());
        assert_eq!(floor_f32(-2.5).to_bits(), (-3.0f32).to_bits());
        assert_eq!(floor_f32(-0.5).to_bits(), (-1.0f32).to_bits());
        assert_eq!(floor_f32(0.5).to_bits(), 0.0f32.to_bits());
        assert_eq!(floor_f32(-0.0).to_bits(), 0x8000_0000); // −0 stays −0
        assert_eq!(floor_f32(0.0).to_bits(), 0x0000_0000);
        assert_eq!(floor_f32(3.0).to_bits(), 3.0f32.to_bits());
        assert_eq!(floor_f32(-3.0).to_bits(), (-3.0f32).to_bits());
        // nextafter(2^24) → 2^24 (integers ≥ 2^23 are all fixed points)
        assert_eq!(floor_f32(16_777_217.0).to_bits(), 16_777_216.0f32.to_bits());
        assert_eq!(floor_f32(f32::MAX).to_bits(), f32::MAX.to_bits());
        assert_eq!(floor_f32(-f32::MAX).to_bits(), (-f32::MAX).to_bits());
        assert_eq!(floor_f32(f32::INFINITY).to_bits(), f32::INFINITY.to_bits());
        assert_eq!(floor_f32(f32::NEG_INFINITY).to_bits(), f32::NEG_INFINITY.to_bits());
        assert!(floor_f32(f32::NAN).is_nan());
        assert!(floor_f32(-f32::NAN).is_nan());
    }

    #[test]
    fn rect_offset_adds_x_to_horizontal_edges_and_y_to_vertical() {
        let mut rect = [1.0, 2.0, 4.0, 6.0];
        offset_rect_by_point(&mut rect, &[0.5, -2.0]);
        assert_eq!(rect, [1.5, 0.0, 4.5, 4.0]);

        // exact f32 add semantics (delta components are reused, not consumed)
        let mut r2 = [0.1, 0.1, 0.1, 0.1];
        offset_rect_by_point(&mut r2, &[0.2, 0.2]);
        assert_eq!(r2, [0.3, 0.3, 0.3, 0.3]); // 0.1f32+0.2f32 rounds to the same f32 as the 0.3 literal

        // −0 + 0 → +0 (IEEE), NaN propagates
        let mut r3 = [-0.0, f32::NAN, 1.0, f32::INFINITY];
        offset_rect_by_point(&mut r3, &[0.0, 1.0]);
        assert_eq!(r3[0], 0.0);
        assert_eq!(r3[0].to_bits(), 0x0000_0000);
        assert!(r3[1].is_nan());
        assert_eq!(r3[3], f32::INFINITY);
    }

    #[test]
    fn private_use_range_boundaries() {
        for c in [0xE01u16, 0xE3A, 0xE3F, 0xE5B] {
            assert!(is_private_use_range(c), "{c:#06X} must be in range");
        }
        for c in [0u16, 0xE00, 0xE3B, 0xE3C, 0xE3E, 0xE5C, 0xFFF0, 0xFFFF] {
            assert!(!is_private_use_range(c), "{c:#06X} must be out of range");
        }
    }

    #[test]
    fn pair_setters_store_only_on_change() {
        let mut o = ExtentsHolder { a_lo: 1.0, a_hi: 2.0, b_lo: 3.0, b_hi: 4.0, ..ExtentsHolder::default() };

        // equal pair: no store, 0
        assert!(!set_pair_at_0b4_if_changed(&mut o, 1.0, 2.0));
        assert_eq!((o.a_lo, o.a_hi), (1.0, 2.0));

        // partial mismatch rewrites BOTH fields
        assert!(set_pair_at_0b4_if_changed(&mut o, 9.0, 2.0));
        assert_eq!((o.a_lo, o.a_hi), (9.0, 2.0));

        // a stored NaN never compares equal (C `==` is false): the game
        // STORES and returns 1 — including the NaN-vs-NaN case
        let mut n = ExtentsHolder { a_lo: 9.0, a_hi: f32::NAN, ..ExtentsHolder::default() };
        assert!(set_pair_at_0b4_if_changed(&mut n, 9.0, f32::NAN));
        assert!(n.a_hi.is_nan());
        n = ExtentsHolder { a_lo: f32::NAN, a_hi: f32::NAN, ..ExtentsHolder::default() };
        assert!(set_pair_at_0b4_if_changed(&mut n, 1.0, 2.0)); // NaN slot overwritten
        assert_eq!((n.a_lo, n.a_hi), (1.0, 2.0));

        // +0.0 == −0.0 in IEEE: reports "unchanged", stored sign survives
        let mut z = ExtentsHolder { a_lo: 0.0, a_hi: -0.0, ..ExtentsHolder::default() };
        assert!(!set_pair_at_0b4_if_changed(&mut z, -0.0, 0.0));
        assert_eq!((z.a_lo.to_bits(), z.a_hi.to_bits()), (0x0000_0000, 0x8000_0000));
        // but a real change rewrites the sign of zero
        assert!(set_pair_at_0b4_if_changed(&mut z, -0.0, -1.0));
        assert_eq!((z.a_lo.to_bits(), z.a_hi.to_bits()), (0x8000_0000, 0xBF80_0000));

        // the 0xBC/0xC0 pair is independent of the 0xB4/0xB8 pair
        assert!(!set_pair_at_0bc_if_changed(&mut o, 3.0, 4.0));
        assert!(set_pair_at_0bc_if_changed(&mut o, -1.5, 0.0));
        assert_eq!((o.b_lo, o.b_hi), (-1.5, 0.0));
        assert_eq!((o.a_lo, o.a_hi), (9.0, 2.0)); // untouched by the 0xBC pair member
    }
}
