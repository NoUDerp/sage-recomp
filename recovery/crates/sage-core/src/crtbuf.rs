//! Wave 46: statically-linked C++ stream-buffer machinery in the `0x00a02670`–
//! `0x00a02df0` band (ledger tag `CRT-LIB`, the statically-linked CRT/cpp
//! region below the `0x00a20000` skip boundary — `crtmath.rs` precedent:
//! mirror, don't skip).
//!
//! The band is the buffer-cursor core of the legacy Dinkumware-style
//! `std::streambuf` family (the game links `msvcp71.dll`, but these stream
//! members were pulled into the image statically): every object exposes
//! *buffer-cursor records*, three raw dwords laid out exactly like the C
//! runtime `FILE` prefix `{_ptr, _cnt, _base}` — a read/write cursor, a
//! remaining byte count, and the record's anchor. Rows read those records
//! through pointer slots (`*(int **)(this + slot)` then `[0]/[1]/[2]`), never
//! through the slots' addresses, so the mirror borrows a [`BufCursor`] per
//! record and leaves slot addressing to the caller.
//!
//! Rows → functions (all module `CRT-LIB`):
//!
//! | row | fn | semantics |
//! |---|---|---|
//! | `0x00a02670` | `advance_get_cursor` | `*p += n; p[1] -= n` — advance the cursor at `this+0x08`, shrink the remainder |
//! | `0x00a026e0` | SKIP | `newbuf`: call through the alloc-callback pointer at `this+0x54`, else CRT heap alloc `FUN_0042fde0` (`(*DAT_00de0404)(size,2,0)` heap jump) |
//! | `0x00a02710` | SKIP | `delbuf`: call through the dealloc-callback pointer at `this+0x58`, else `FUN_0042fd80` (`(*DAT_00de03fc)(p,2)` heap free — contract skip-list delete) |
//! | `0x00a02740` | `set_buffer_areas` | `setbuf`-style: `strlen` when `count == 0`, `0x7fffffff` when `count < 0`, then program the two records |
//! | `0x00a027b0` | `show_base_mark_64_0c` | flag byte `+0x64`: `|= 2` when bit0 clear; return record `[2]` via pointer slot `+0x0c` |
//! | `0x00a027d0` | `set_bit1_60` | flag byte `+0x60`: `f ^= ((p << 1) ^ f) & 2` when bit0 set (bit1 ← param bit0) |
//! | `0x00a027f0` | `show_base_mark_60_08` | flag byte `+0x60`; record via pointer slot `+0x08` |
//! | `0x00a02810` | `consumed_count_0c` | `_Ptr ? _Ptr - _Base : 0` (`outq`/`pcount` shape) via slot `+0x0c` |
//! | `0x00a02830` | `set_bit1_68` | same bit setter for flag byte `+0x68` |
//! | `0x00a02850` | `consumed_count_14` | same `ptr - base` shape via slot `+0x14` |
//! | `0x00a02a30` | SKIP | constructor: vptr stores (`PTR_FUN_00bbbc30` → `PTR_FUN_00c7a878`), lock helper `FUN_00407100` (`(**(code **)(*DAT_00ddeb18 + 4))()` vtable-dispatched critical section), then calls `FUN_00a02740` — vtable + lock + FUN-chain |
//! | `0x00a02b50` | SKIP | destructor: SEH `ExceptionList` prologue/epilogue, vptr store, `FUN_0042fd80` heap free or `(+0x58)` callback dispatch |
//! | `0x00a02bd0` | `peek_byte_eof` | `sgetc`-style peek: on exhausted record `A`, borrow count up to record `B`'s cursor; EOF sentinel `0xffffffff` |
//! | `0x00a02df0` | SKIP | deleting-destructor shell: SEH, vptr stores on the `-0x68` base subobject, calls `FUN_00a02b50` |
//!
//! Object shapes (evidence from the skipped constructor/destructor rows):
//! the constructors (`0x00a028a0/0x00a02970/0x00a02a30/0x00a02aa0`, all
//! 4-byte-scaled `undefined4 *this`) self-point: `this[1] = this + 3`
//! (slot `+0x04` → record embedded at `+0x0c..+0x14`) and `this[2] =
//! this + 0xb` (slot `+0x08` → record embedded at `+0x2c..+0x34`); `+0x54`
//! and `+0x58` are alloc/free callback pointers written by the two-callback
//! constructor `0x00a02970` (`this[0x15]`/`this[0x16]` = `+0x54`/`+0x58` —
//! the same slots `0x00a026e0`/`0x00a02710` and the destructor `0x00a02b50`
//! read, the latter also gating the buffer free on flag byte `+0x5c` bit0
//! set and bit1 clear); the outer composite embeds this object at `+0x68`
//! (`0x00a02df0` base adjust `-0x68`) and `+0x6c` (neighbour `0x00a02eb0`
//! base adjust `-0x6c`). The accessor rows here instead read *pointer
//! slots* at `+0x08`/`+0x0c`/`+0x14` and byte fields at `+0x60`/`+0x64`/
//! `+0x68` — a related (derived/base) shape; because every row dereferences
//! the slot value and only touches record dwords `[0]/[1]/[2]`, the mirror's
//! [`BufCursor`] borrow is byte-exact for either shape.
//!
//! Faithful-quirk notes pinned by tests:
//! - All cursor math is raw 32-bit: `wrapping_add`/`wrapping_sub` everywhere,
//!   and the two `0x00a02bd0` range tests cast to **unsigned** (`(uint)` in
//!   the decompilation), so a negative-looking `+0x80000000` cursor extends
//!   a record to `cnt = 0x80000000`.
//! - `0x00a02740`'s strlen scan reads past the modelled window when no NUL
//!   exists (the game has no bound — UB); the mirror panics.
//! - `0x00a02bd0` reads the current byte *without advancing* (pure peek),
//!   and the decompiler's `piVar[2] = piVar[2]; *piVar = iVar;` self-moves
//!   are register artifacts — only `[1]` (the count) is stored.
//! - `0x00a026e0`/`0x00a02710` treat `+0x54`/`+0x58` as function pointers
//!   (`(**(code **)x)(a)` is Ghidra's call-through-pointer idiom; the
//!   destructor `0x00a02b50` calls `this[0x16]` = `+0x58` directly).

/// One buffer-cursor record: three raw dwords — `[0]` read/write cursor,
/// `[1]` remaining byte count from the cursor, `[2]` anchor (base or end
/// depending on the record's role). All three hold image addresses or
/// byte counts, kept as raw `i32` so subtraction and the game's `(uint)`
/// casts reproduce exactly.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BufCursor {
    /// `[0]` — current read/write cursor (image address; `0` = no buffer).
    pub ptr: i32,
    /// `[1]` — remaining bytes from `ptr`.
    pub cnt: i32,
    /// `[2]` — record anchor (`_Base`-style dword).
    pub base: i32,
}

/// `0x00a02670` — advance the read cursor of the record at `this + 0x08`:
/// `*p += n; p[1] -= n`. Both updates wrap like the x86 `add`/`sub` they
/// compile to; no bounds test exists in the game.
pub fn advance_get_cursor(rec: &mut BufCursor, n: i32) {
    rec.ptr = rec.ptr.wrapping_add(n);
    rec.cnt = rec.cnt.wrapping_sub(n);
}

/// `0x00a02740` — `setbuf`-style programming of the two record pointers
/// (`this + 0x04` → `area_a`, `this + 0x08` → `area_b`).
///
/// Game semantics, in order:
/// - `buf_addr == 0` (`param_2 != 0` guard): nothing happens.
/// - `count < 1`: `count == 0` takes the buffer length by scanning `buf_mem`
///   (the byte window starting at `buf_addr`) for the NUL — the game computes
///   `(int)scan_end - (int)(buf + 1)`, i.e. the NUL index; a window with no
///   NUL is the game's unbounded `strlen` past the buffer (UB) → panic.
///   `count < 0` becomes `0x7fffffff` (`INT_MAX`).
/// - `end != 0`: `area_a = {buf, end - buf, buf}` and
///   `area_b = {end, count, end}`; `area_a`'s count is the pointer delta
///   `end - buf` (wrapping like the game's dword subtract).
/// - `end == 0`: only `area_a` is programmed, as `{buf, count, buf}`;
///   `area_b` is left untouched.
///
/// `buf_mem` is only read on the `count == 0` scan path; callers pass `&[]`
/// otherwise.
pub fn set_buffer_areas(
    area_a: &mut BufCursor,
    area_b: &mut BufCursor,
    buf_addr: i32,
    end: i32,
    count: i32,
    buf_mem: &[u8],
) {
    if buf_addr == 0 {
        return;
    }
    let mut n = count;
    if n < 1 {
        if n == 0 {
            let nul = buf_mem.iter().position(|&b| b == 0).unwrap_or_else(|| {
                panic!("0x00a02740: strlen scan ran past the modelled window (the game reads past the buffer until a NUL: UB)")
            });
            n = nul as i32;
        } else {
            n = 0x7fff_ffff;
        }
    }
    if end != 0 {
        area_a.base = buf_addr;
        area_a.ptr = buf_addr;
        area_a.cnt = end.wrapping_sub(buf_addr);
        area_b.base = end;
        area_b.ptr = end;
        area_b.cnt = n;
    } else {
        area_a.base = buf_addr;
        area_a.ptr = buf_addr;
        area_a.cnt = n;
    }
}

/// `0x00a027f0` — touch-and-anchor pair for flag byte `+0x60` and the record
/// held at slot `+0x08`: if bit0 of the flag byte is clear it sets bit1
/// (`|= 2`, byte store); either way it returns the record's `[2]` dword
/// (`(*(int *)(this + 8))[2]` — the anchor). The game null-checks nothing:
/// a zero slot would dereference address `8` (UB) — the borrow makes the
/// record's existence the caller's obligation.
pub fn show_base_mark_60_08(flag_60: &mut u8, rec: &BufCursor) -> i32 {
    if (*flag_60 & 1) == 0 {
        *flag_60 |= 2;
    }
    rec.base
}

/// `0x00a027b0` — same shape as [`show_base_mark_60_08`] for flag byte
/// `+0x64` (written as `param_1 + 100`) and the record held at slot
/// `+0x0c`.
pub fn show_base_mark_64_0c(flag_64: &mut u8, rec: &BufCursor) -> i32 {
    if (*flag_64 & 1) == 0 {
        *flag_64 |= 2;
    }
    rec.base
}

/// `0x00a027d0` — conditional bit set on flag byte `+0x60`: the game stores
/// `(param_2 << 1 ^ f) & 2 ^ f`, i.e. `f ^ (((p << 1) ^ f) & 2)` — after the
/// `& 2` mask only bit 1 of the shifted param survives, so the effect is
/// *bit1 ← bit0 of the (sign-extended `char`) param*, with all other bits
/// kept. The whole update only happens when bit0 of the flag byte is set.
/// The expression is kept verbatim (the shift is an immediate `<< 1`, so
/// `wrapping_shl` masks like the `shl` does).
pub fn set_bit1_60(flag_60: &mut u8, param_2: i8) {
    let f = *flag_60 as i32;
    if (f & 1) != 0 {
        *flag_60 = ((f ^ (((param_2 as i32).wrapping_shl(1) ^ f) & 2)) ) as u8;
    }
}

/// `0x00a02830` — identical to [`set_bit1_60`] but for flag byte `+0x68`.
pub fn set_bit1_68(flag_68: &mut u8, param_2: i8) {
    let f = *flag_68 as i32;
    if (f & 1) != 0 {
        *flag_68 = ((f ^ (((param_2 as i32).wrapping_shl(1) ^ f) & 2)) ) as u8;
    }
}

/// `0x00a02810` — `outq`/`pcount`-shaped count through slot `+0x0c`:
/// `rec[0] == 0 → 0`, else `rec[0] - rec[2]` (bytes between cursor and
/// anchor; the dword subtraction wraps, and a cursor below the anchor
/// returns the negative delta exactly like the game's pointer difference).
pub fn consumed_count_0c(rec: &BufCursor) -> i32 {
    if rec.ptr == 0 {
        return 0;
    }
    rec.ptr.wrapping_sub(rec.base)
}

/// `0x00a02850` — identical to [`consumed_count_0c`] through slot `+0x14`.
pub fn consumed_count_14(rec: &BufCursor) -> i32 {
    if rec.ptr == 0 {
        return 0;
    }
    rec.ptr.wrapping_sub(rec.base)
}

/// `0x00a02bd0` — `sgetc`-style peek over record `A` (slot `+0x04`) with a
/// cross-record count extension from record `B` (slot `+0x08`), returning
/// the byte at `A.ptr` **without advancing the cursor**, or the EOF
/// sentinel `0xffffffff`.
///
/// Game flow (all tests verbatim from the body):
/// - "exhausted" is written as the self-comparison `ptr == cnt + ptr`
///   (wrapping dword add; equivalent to `cnt ≡ 0 (mod 2³²)` but kept literal).
/// - When `A` is exhausted, `B.ptr != 0`, and the *unsigned* test
///   `(uint)(A.cnt + A.ptr) < (uint)B.ptr` holds, the game stores
///   `A.cnt = B.ptr - A.ptr` (wrapping — a `0x80000000`-looking `B.ptr`
///   gives `cnt = 0x80000000`, pinned by tests). The decompiler's
///   `A[2] = A[2]; A[0] = iVar;` lines are register self-moves (no-ops).
/// - Then, after reloading the slot, the exhausted test runs again: exhausted
///   → `0xffffffff`; otherwise `(uint)*(byte *)A.ptr` — the byte at the
///   cursor. `mem` is the byte window starting at image address `mem_base`;
///   a cursor outside it is the game's wild read (UB) → panic.
pub fn peek_byte_eof(rec_a: &mut BufCursor, rec_b: &BufCursor, mem: &[u8], mem_base: i32) -> u32 {
    if rec_a.ptr == rec_a.cnt.wrapping_add(rec_a.ptr) {
        let b = rec_b.ptr;
        if b != 0
            && (rec_a.cnt.wrapping_add(rec_a.ptr) as u32) < (b as u32)
        {
            rec_a.cnt = b.wrapping_sub(rec_a.ptr);
        }
    }
    if rec_a.ptr == rec_a.cnt.wrapping_add(rec_a.ptr) {
        return 0xffff_ffff;
    }
    let off = (rec_a.ptr as u32).wrapping_sub(mem_base as u32);
    match mem.get(off as usize) {
        Some(&b) => b as u32,
        None => panic!("0x00a02bd0: byte read at {:#010x} outside the modelled window (game wild read: UB)", rec_a.ptr as u32),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advance_cursor_exact() {
        let mut r = BufCursor { ptr: 0x1000, cnt: 500, base: 0x1000 };
        advance_get_cursor(&mut r, 100);
        assert_eq!(r, BufCursor { ptr: 0x1064, cnt: 400, base: 0x1000 });
        // negative n widens cnt and pulls the cursor back
        advance_get_cursor(&mut r, -100);
        assert_eq!(r, BufCursor { ptr: 0x1000, cnt: 500, base: 0x1000 });
    }

    #[test]
    fn advance_cursor_wraps() {
        let mut r = BufCursor { ptr: 0x7fff_ffff, cnt: 0, base: 0 };
        advance_get_cursor(&mut r, 1);
        assert_eq!(r.ptr, i32::MIN); // 0x80000000 wraps signed
        assert_eq!(r.cnt, -1); // 0 - 1 wraps
        let mut s = BufCursor { ptr: 0, cnt: 0, base: 0 };
        advance_get_cursor(&mut s, i32::MIN);
        assert_eq!(s.ptr, i32::MIN);
        assert_eq!(s.cnt, i32::MIN); // 0 - 0x80000000 == 0x80000000
    }

    #[test]
    fn set_buffer_null_noop() {
        let mut a = BufCursor { ptr: 0xdead, cnt: 0xbeef, base: 1 };
        let mut b = BufCursor { ptr: 1, cnt: 2, base: 3 };
        set_buffer_areas(&mut a, &mut b, 0, 0x2000, 7, &[]);
        assert_eq!(a, BufCursor { ptr: 0xdead, cnt: 0xbeef, base: 1 });
        assert_eq!(b, BufCursor { ptr: 1, cnt: 2, base: 3 });
    }

    #[test]
    fn set_buffer_negative_count_int_max() {
        let mut a = BufCursor::default();
        let mut b = BufCursor::default();
        set_buffer_areas(&mut a, &mut b, 0x401000, 0x402000, -5, &[]);
        assert_eq!(a, BufCursor { ptr: 0x401000, cnt: 0x1000, base: 0x401000 });
        assert_eq!(b, BufCursor { ptr: 0x402000, cnt: 0x7fff_ffff, base: 0x402000 });
    }

    #[test]
    fn set_buffer_zero_count_strlen() {
        let mut a = BufCursor::default();
        let mut b = BufCursor::default();
        let mem = *b"hello\0tail";
        // count == 0 with end: a gets the pointer delta, b gets the strlen (5)
        set_buffer_areas(&mut a, &mut b, 0x401000, 0x402000, 0, &mem);
        assert_eq!(a, BufCursor { ptr: 0x401000, cnt: 0x1000, base: 0x401000 });
        assert_eq!(b, BufCursor { ptr: 0x402000, cnt: 5, base: 0x402000 });
        // count == 0 with end == 0: only area_a is written, cnt = strlen
        let mut a2 = BufCursor::default();
        let mut b2 = BufCursor { ptr: 9, cnt: 9, base: 9 };
        set_buffer_areas(&mut a2, &mut b2, 0x401000, 0, 0, &mem);
        assert_eq!(a2, BufCursor { ptr: 0x401000, cnt: 5, base: 0x401000 });
        assert_eq!(b2, BufCursor { ptr: 9, cnt: 9, base: 9 });
    }

    #[test]
    fn set_buffer_plain_count_end_zero_leaves_b() {
        let mut a = BufCursor::default();
        let mut b = BufCursor { ptr: 7, cnt: 7, base: 7 };
        set_buffer_areas(&mut a, &mut b, 0x8000, 0, 7, &[]);
        assert_eq!(a, BufCursor { ptr: 0x8000, cnt: 7, base: 0x8000 });
        assert_eq!(b, BufCursor { ptr: 7, cnt: 7, base: 7 });
    }

    #[test]
    #[should_panic(expected = "strlen scan ran past")]
    fn set_buffer_strlen_unterminated_is_ub() {
        let mut a = BufCursor::default();
        let mut b = BufCursor::default();
        let mem = *b"no nul here";
        set_buffer_areas(&mut a, &mut b, 0x100, 0, 0, &mem);
    }

    #[test]
    fn show_base_mark_pair() {
        // bit0 clear -> bit1 set, anchor returned; slot +0x60 variant
        let mut f = 0b00u8;
        let r = BufCursor { ptr: 0x100, cnt: 4, base: 0x2000 };
        assert_eq!(show_base_mark_60_08(&mut f, &r), 0x2000);
        assert_eq!(f, 0b10);
        // bit0 set -> flag untouched
        let mut g = 0b01u8;
        assert_eq!(show_base_mark_60_08(&mut g, &r), 0x2000);
        assert_eq!(g, 0b01);
        // higher bits survive; slot +0x64 variant guards on its own bit0
        let mut h = 0b0100u8;
        assert_eq!(show_base_mark_64_0c(&mut h, &r), 0x2000);
        assert_eq!(h, 0b0110);
        let mut i = 0b0011u8;
        assert_eq!(show_base_mark_64_0c(&mut i, &r), 0x2000);
        assert_eq!(i, 0b0011);
    }

    #[test]
    fn bit_setters_exact_expression() {
        // bit1 <- param bit0, only when flag bit0 set
        let mut f = 0b01u8;
        set_bit1_60(&mut f, 1);
        assert_eq!(f, 0b11);
        set_bit1_60(&mut f, 0);
        assert_eq!(f, 0b01);
        // sign-extended char: -2 (bit0 clear) leaves bit1 at 0...
        let mut g = 0b101u8;
        set_bit1_60(&mut g, -2);
        assert_eq!(g, 0b101);
        // ...-1 (bit0 set) sets it
        set_bit1_60(&mut g, -1);
        assert_eq!(g, 0b111);
        // flag bit0 clear: guard skips entirely even with bit=1
        let mut h = 0b10u8;
        set_bit1_60(&mut h, 1);
        assert_eq!(h, 0b10);
        // upper five bits survive the &2 mask
        let mut j = 0xfdu8; // 1111_1101
        set_bit1_60(&mut j, 1);
        assert_eq!(j, 0xff);
        set_bit1_60(&mut j, 0);
        assert_eq!(j, 0xfd);
        // +0x68 twin behaves identically
        let mut k = 0b01u8;
        set_bit1_68(&mut k, 3); // bit0 of 3 is set
        assert_eq!(k, 0b11);
        set_bit1_68(&mut k, 2); // bit0 of 2 is clear
        assert_eq!(k, 0b01);
    }

    #[test]
    fn consumed_count_shape() {
        // null cursor guards to 0 even with a set anchor
        assert_eq!(consumed_count_0c(&BufCursor { ptr: 0, cnt: 4, base: 0x1234 }), 0);
        assert_eq!(consumed_count_14(&BufCursor { ptr: 0, cnt: 4, base: 0x1234 }), 0);
        let r = BufCursor { ptr: 0x401064, cnt: -100, base: 0x401000 };
        assert_eq!(consumed_count_0c(&r), 100);
        assert_eq!(consumed_count_14(&r), 100);
        // cursor below anchor: negative pointer delta (cnt is ignored entirely)
        let r2 = BufCursor { ptr: 0x3ff000, cnt: 999, base: 0x401000 };
        assert_eq!(consumed_count_0c(&r2), 0x3ff000i32.wrapping_sub(0x401000));
        assert_eq!(consumed_count_0c(&r2), -0x2000);
    }

    #[test]
    fn peek_not_exhausted_reads_without_advance() {
        let mem = [0x41u8, 0x42, 0x43, 0x44, 0x45];
        let mut a = BufCursor { ptr: 4, cnt: 3, base: 2 };
        let b = BufCursor { ptr: 100, cnt: 100, base: 100 };
        assert_eq!(peek_byte_eof(&mut a, &b, &mem, 2), 0x43); // byte at 4 = mem[2]
        // pure peek: the record is untouched, and B was not consulted
        assert_eq!(a, BufCursor { ptr: 4, cnt: 3, base: 2 });
        assert_eq!(b, BufCursor { ptr: 100, cnt: 100, base: 100 });
    }

    #[test]
    fn peek_exhausted_null_b_eof() {
        let mem = [0u8; 16];
        let mut a = BufCursor { ptr: 4, cnt: 0, base: 0 };
        let b = BufCursor { ptr: 0, cnt: 7, base: 7 };
        assert_eq!(peek_byte_eof(&mut a, &b, &mem, 0), 0xffff_ffff);
        assert_eq!(a.cnt, 0); // no extension when B.cursor is null
    }

    #[test]
    fn peek_exhausted_extends_from_b() {
        let mem = [0x10u8, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16];
        let mut a = BufCursor { ptr: 4, cnt: 0, base: 99 };
        let b = BufCursor { ptr: 6, cnt: 0, base: 0 };
        // (cnt+ptr)=4 < 6 (unsigned): cnt := 6 - 4 = 2, then byte at 4
        assert_eq!(peek_byte_eof(&mut a, &b, &mem, 0), 0x14);
        assert_eq!(a, BufCursor { ptr: 4, cnt: 2, base: 99 });
    }

    #[test]
    fn peek_extend_boundaries() {
        let mem = [0u8; 16];
        // strict <: B.cursor == A end -> no extension, EOF
        let mut a = BufCursor { ptr: 10, cnt: 0, base: 0 };
        let b = BufCursor { ptr: 10, cnt: 0, base: 0 };
        assert_eq!(peek_byte_eof(&mut a, &b, &mem, 0), 0xffff_ffff);
        assert_eq!(a.cnt, 0);
        // unsigned compare: A end 0xfffffffe is NOT < B.cursor 2
        let mut c = BufCursor { ptr: -2, cnt: 0, base: 0 };
        let d = BufCursor { ptr: 2, cnt: 0, base: 0 };
        assert_eq!(peek_byte_eof(&mut c, &d, &mem, 0), 0xffff_ffff);
        assert_eq!(c.cnt, 0);
        // 0x80000000-looking B.cursor extends to the wrapped count (pinned),
        // and the byte read still happens at A.ptr
        let mut e = BufCursor { ptr: 0, cnt: 0, base: 0 };
        let f = BufCursor { ptr: i32::MIN, cnt: 0, base: 0 };
        assert_eq!(peek_byte_eof(&mut e, &f, &mem, 0), 0x00);
        assert_eq!(e.cnt, i32::MIN); // 0x80000000 - 0 wraps to 0x80000000
    }

    #[test]
    #[should_panic(expected = "outside the modelled window")]
    fn peek_wild_read_is_ub() {
        let mem = [0u8; 4];
        let mut a = BufCursor { ptr: 40, cnt: 3, base: 0 };
        let b = BufCursor { ptr: 0, cnt: 0, base: 0 };
        peek_byte_eof(&mut a, &b, &mem, 10); // 40 is outside [10, 14)
    }
}
