//! `StringBase<T>` — SAGE's refcounted copy-on-write string (both
//! instantiations: `StringBase<char>` and `StringBase<unsigned_short>`).
//!
//! Ground model (decomp): a `StringBase` is a single pointer to a
//! refcounted buffer: header `{i32 rc, u16 len, u16 cap}` at +0/+4/+6 and
//! element data at +8 (`0x004364a0`/`0x00436410`). An empty *string* is a
//! null pointer (`str()` → static NUL, `getLength()` → 0). Copy ctor
//! (`0x004365f0`) and `set(StringBase&)` (`0x004366f0`) share the pointer
//! under the engine critical section and bump `rc`; mutators detach first
//! (`ensureUniqueBufferOfSize` 0x004364a0 char / 0x00436f00 u16).
//!
//! `Rc::strong_count` reproduces the header `rc` exactly: the game mutates
//! in place iff `rc == 1`, which is precisely `Rc::get_mut` succeeding.
//! The critical section (engine +8, bypassed when engine byte +0x20 is
//! set, `FUN_00435c90`) is a no-op in the single-threaded core; the game's
//! pool allocator (`FUN_004307f0(size,'str')` / `FUN_00430830`) is
//! modeled by `Rc`/`Vec`.
//!
//! Buffer sizing is reproduced exactly (capacity is observable):
//! - `char` (0x004364a0): alloc = `(n + 12) & !3` bytes, cap bytes =
//!   alloc − 8; throws (modeled as panic) when `n + 9 > 0x7fff`.
//! - `u16` (0x00436f00): alloc = `(2n + 13) & !3` bytes, cap in **units**
//!   = (alloc − 8) / 2; panics when `2n + 10 > 0x7fff`.
//! - append path (`concat` → ensure with a trailing source) grows to
//!   `max(n, cap + (cap >> 1) - 1)` before allocating.
//!
//! Case-insensitive operations mirror the CRT *C-locale* folding
//! (`_memicmp` byte-wise, `towlower` per u16 unit — ASCII A–Z/a–z only;
//! non-ASCII case pairs compare unequal, matching the game binary).
//! `format`/`format_va` (0x00437d90 / 0x00436d40 / 0x00437920 and the u16
//! counterparts 0x00437ff0 / 0x004377e0 / 0x00437a30) are NOT mirrored:
//! they call MSVC71 `sprintf` and need a formatter-parity pass at L2.

use std::rc::Rc;

/// Element type of a `StringBase<T>` instantiation.
pub trait SbChar: Copy + PartialEq {
    /// The NUL element (`TheNullChr` in the decomp).
    const NUL: Self;
    /// Bytes per element (1 for char, 2 for unsigned_short).
    const WIDTH: usize;
    /// Overflow guard from the grow paths: panic iff
    /// `needed * WIDTH + OFFSET > 0x7fff` (OFFSET 9 char / 10 u16).
    fn guard(needed: usize) -> bool;
    /// alloc bytes for `needed` elements, matching the game's rounding.
    fn alloc_bytes(needed: usize) -> usize;
    /// Unsigned comparison key for `memcmp`-style compares.
    fn key(self) -> u32;
    /// Element from an ASCII literal byte (for `"None"` etc.).
    fn lit(c: u8) -> Self;
    /// Element from its comparison key (xfer blob decode).
    fn from_key(k: u32) -> Self;
    /// Length-header tag for the `Xfer` string forwarders
    /// (`'astr'` = 0x61737472 / `'ustr'` = 0x75737472).
    const XFER_TAG: u32;
    /// Case folding used by the `*NoCase` family (C-locale ASCII).
    fn fold(self) -> u32 {
        let k = self.key();
        if (0x41..=0x5a).contains(&k) {
            k + 32
        } else {
            k
        }
    }
    /// `isspace((int)c)` in the C locale.
    fn is_space(self) -> bool {
        matches!(self.key(), 0x09..=0x0d | 0x20)
    }
    /// `tolower`/`towlower` (C locale: ASCII only; everything else
    /// unchanged, as the CRT behaves for the game's default locale).
    fn to_lower(self) -> Self;
    fn to_upper(self) -> Self;
}

/// `StringBase<char>` element.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Ascii(pub u8);

impl SbChar for Ascii {
    const NUL: Self = Ascii(0);
    const WIDTH: usize = 1;
    fn guard(n: usize) -> bool {
        n + 9 > 0x7fff
    }
    fn alloc_bytes(n: usize) -> usize {
        (n + 12) >> 2 << 2
    }
    fn key(self) -> u32 {
        self.0 as u32
    }
    fn lit(c: u8) -> Self {
        Ascii(c)
    }
    fn from_key(k: u32) -> Self {
        Ascii(k as u8)
    }
    const XFER_TAG: u32 = sage_xfer::TAG_ASTR;
    fn to_lower(self) -> Self {
        if (0x41..=0x5a).contains(&self.0) {
            Ascii(self.0 + 32)
        } else {
            self
        }
    }
    fn to_upper(self) -> Self {
        if (0x61..=0x7a).contains(&self.0) {
            Ascii(self.0 - 32)
        } else {
            self
        }
    }
    fn is_space(self) -> bool {
        self.0 < 0x80 && matches!(self.0, 0x09..=0x0d | 0x20)
    }
}

/// `StringBase<unsigned_short>` element (UTF-16 code units, the game's
/// native unicode strings).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Utf16(pub u16);

impl SbChar for Utf16 {
    const NUL: Self = Utf16(0);
    const WIDTH: usize = 2;
    fn guard(n: usize) -> bool {
        n * 2 + 10 > 0x7fff
    }
    fn alloc_bytes(n: usize) -> usize {
        (n * 2 + 13) >> 2 << 2
    }
    fn key(self) -> u32 {
        self.0 as u32
    }
    fn lit(c: u8) -> Self {
        Utf16(c as u16)
    }
    fn from_key(k: u32) -> Self {
        Utf16(k as u16)
    }
    const XFER_TAG: u32 = sage_xfer::TAG_USTR;
    fn to_lower(self) -> Self {
        if (0x41..=0x5a).contains(&self.0) {
            Utf16(self.0 + 32)
        } else {
            self
        }
    }
    fn to_upper(self) -> Self {
        if (0x61..=0x7a).contains(&self.0) {
            Utf16(self.0 - 32)
        } else {
            self
        }
    }
}

struct Buf<T: SbChar> {
    len: u16,
    /// capacity in elements (header +6: bytes for char, units for u16).
    cap: u16,
    /// `cap`-sized data area (game: buffer + 8). `data[len]` is kept
    /// NUL-terminated by every mutator while `len < cap` (the invariant
    /// every game size computation enforces).
    data: Vec<T>,
}

/// `StringBase<char>` (the `AsciiString` substrate).
pub type StringAscii = StringBase<Ascii>;
/// `StringBase<unsigned_short>` (the `UnicodeString` substrate).
pub type StringUtf16 = StringBase<Utf16>;

/// Mirrors `StringBase<T>`: one field — a pointer to a shared buffer.
pub struct StringBase<T: SbChar> {
    buf: Option<Rc<Buf<T>>>,
}

impl<T: SbChar> Clone for StringBase<T> {
    /// Copy ctor 0x004365f0 / 0x00437050: share pointer, `rc += 1`.
    fn clone(&self) -> Self {
        StringBase {
            buf: self.buf.clone(),
        }
    }
}

impl<T: SbChar> Default for StringBase<T> {
    /// Default ctors 0x00437bf0 / 0x00437e70 / 0x00726be6: null pointer.
    fn default() -> Self {
        StringBase { buf: None }
    }
}

impl<T: SbChar> PartialEq for StringBase<T> {
    /// Content equality over the length-trimmed view (`str()`).
    fn eq(&self, other: &Self) -> bool {
        self.str() == other.str()
    }
}

impl<T: SbChar + core::fmt::Debug> core::fmt::Debug for StringBase<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(self.str().iter()).finish()
    }
}

impl PartialEq<u8> for Ascii {
    /// Lets tests compare `str()` against byte literals (`b"text"`).
    fn eq(&self, other: &u8) -> bool {
        self.0 == *other
    }
}

impl<T: SbChar> StringBase<T> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Header `rc` (`**buf`), observable through the shared buffer.
    pub fn ref_count(&self) -> u32 {
        self.buf.as_ref().map_or(0, |b| Rc::strong_count(b) as u32)
    }

    /// `getLength` 0x00405808 / 0x00435740.
    pub fn length(&self) -> usize {
        self.buf.as_ref().map_or(0, |b| b.len as usize)
    }

    /// `isEmpty` 0x00401e2f / 0x00435740.
    pub fn is_empty(&self) -> bool {
        match &self.buf {
            None => true,
            Some(b) => b.len == 0,
        }
    }

    /// `isNotEmpty` 0x00807101 / 0x004358e0.
    pub fn is_not_empty(&self) -> bool {
        !self.is_empty()
    }

    /// `str()`: contents (null buffer ⇒ the static NUL ⇒ empty view).
    pub fn str(&self) -> &[T] {
        self.buf
            .as_ref()
            .map_or(&[] as &[T], |b| &b.data[..b.len as usize])
    }

    /// `peek` 0x0088dc01: C returns `buf + 8` *unconditionally* (deref is
    /// UB when the string is null); Rust surfaces the null as `None`.
    pub fn peek(&self) -> Option<&[T]> {
        self.buf.as_ref().map(|b| &b.data[..])
    }

    /// Buffer capacity in elements (header +6 semantics).
    pub fn capacity(&self) -> usize {
        self.buf.as_ref().map_or(0, |b| b.cap as usize)
    }

    /// `isNone` 0x00437de0 / 0x00438040: case-insensitive compare against
    /// the literal at `DAT_00bbdbc8` = `"None"`.
    pub fn is_none(&self) -> bool {
        let none: Vec<T> = b"None".iter().map(|&c| T::lit(c)).collect();
        self.compare_no_case_cstr(&none) == 0
    }

    /// `isNotNone` 0x00438050 / 0x00438070.
    pub fn is_not_none(&self) -> bool {
        !self.is_none()
    }

    /// `getCharAt` 0x00435720 / 0x00435760: NUL when the buffer is null;
    /// C reads `data[i]` with no bound check (caller's contract; Rust
    /// panics — the game's OOB read is UB we do not reproduce).
    pub fn char_at(&self, i: usize) -> T {
        match &self.buf {
            None => T::NUL,
            Some(b) => b.data[i],
        }
    }

    /// `swap` 0x00435910: pointer exchange only.
    pub fn swap(&mut self, other: &mut Self) {
        std::mem::swap(&mut self.buf, &mut other.buf);
    }

    /// `operator=` 0x00436e50 → `set(StringBase&)` 0x004366f0: self-guard,
    /// release, share, `rc++` (Rc clone).
    pub fn assign(&mut self, other: &Self) {
        if let (Some(a), Some(b)) = (&self.buf, &other.buf) {
            if Rc::ptr_eq(a, b) {
                return;
            }
        }
        self.release_buffer();
        self.buf = other.buf.clone();
    }

    /// `releaseBuffer` 0x00436410 / 0x00436e70 (dtors 0x0088ba39 /
    /// 0x009b804e identical): `--rc`, free at 0, pointer := null. Rc drop
    /// mirrors the decrement + pool free.
    pub fn release_buffer(&mut self) {
        self.buf = None;
    }

    /// `debugIgnoreLeaks` 0x004907bc / 0x00a9e440 / 0x00acc4d0: no-ops.
    pub fn debug_ignore_leaks(&self) {}

    // ------------------------------------------------------------------
    // internal buffer management
    // ------------------------------------------------------------------

    /// `ensureUniqueBufferOfSize` 0x004364a0 (char) / 0x00436f00 (u16),
    /// with the two source shapes the game actually uses: `preserve`
    /// replays the copy-old-data branch (`param_2`), `appending` selects
    /// the `src4 != 0` 1.5× growth bump (only ever true on `concat`
    /// paths). On return the buffer is unique with `cap > needed` and
    /// contents preserved iff `preserve`.
    fn ensure_unique(&mut self, needed: usize, preserve: bool, appending: bool) {
        let mut n = needed;
        let inplace = match self.buf.as_ref() {
            Some(b) => {
                if (needed as u16) < b.cap && Rc::strong_count(b) == 1 {
                    true
                } else {
                    if (needed as u16) >= b.cap && appending {
                        let bump = (b.cap as usize) + (b.cap as usize >> 1) - 1;
                        if n < bump {
                            n = bump;
                        }
                    }
                    false
                }
            }
            None => false,
        };
        if inplace {
            return;
        }
        if T::guard(n) {
            // `_CxxThrowException(&1, &DAT_00cfe2c8)`: std::length_error.
            panic!("StringBase: buffer size overflow (length_error)");
        }
        let alloc = T::alloc_bytes(n);
        let cap = (alloc - 8) / T::WIDTH;
        let mut nb = Buf {
            len: 0,
            cap: cap as u16,
            data: vec![T::NUL; cap],
        };
        if preserve {
            if let Some(old) = self.buf.as_ref() {
                nb.len = old.len;
                nb.data[..old.len as usize].copy_from_slice(&old.data[..old.len as usize]);
            }
        }
        nb.data[nb.len as usize] = T::NUL;
        // game: releaseBuffer(this); attach new — Rc drop = release.
        self.buf = Some(Rc::new(nb));
    }

    /// Shared tail of every set/concat: the `src->write(dest)` vtable call
    /// + length update + `data[len] = 0`.
    fn write_at(&mut self, at: usize, src: &[T]) {
        let b = Rc::get_mut(self.buf.as_mut().unwrap()).expect("unique after ensure_unique");
        b.data[at..at + src.len()].copy_from_slice(src);
        b.len = (at + src.len()) as u16;
        b.data[b.len as usize] = T::NUL;
    }

    // ------------------------------------------------------------------
    // set family
    // ------------------------------------------------------------------

    /// `set(const T*, n)` 0x00436780 / 0x004371e0; the cstr/char thunks
    /// 0x004055f5 / 0x00551243 / 0x0040561a (char) and 0x0040565d /
    /// 0x0080fb15 / 0x00405683 (u16) are strlen/single-char wrappers.
    /// Empty source (null pointer or `n == 0`) takes the `releaseBuffer`
    /// branch (released state, `ref_count() == 0`). `is_none()` is a
    /// *literal* test against `"None"`, not a null-buffer test.
    pub fn set(&mut self, src: &[T]) {
        if src.is_empty() {
            self.release_buffer();
            return;
        }
        self.ensure_unique(src.len(), false, false);
        self.write_at(0, src);
    }

    /// `set(StringBase&)` 0x004366f0 / 0x00437150.
    pub fn set_string_base(&mut self, other: &Self) {
        self.assign(other);
    }

    /// `set(const T*, offset, count)` 0x00437c10 / 0x00437e90 — clamping
    /// transcribed verbatim.
    pub fn set_sub_cstr(&mut self, src: &[T], offset: i32, count: i32) {
        let len = src.len() as i32;
        let total = offset + count;
        if total < 0 || offset >= len {
            self.release_buffer();
            return;
        }
        let mut off = offset;
        let mut cnt = count;
        if off < 0 {
            off = 0;
            cnt = total;
        }
        if len <= off + cnt {
            cnt = len - off;
        }
        self.set(&src[off as usize..off as usize + cnt.max(0) as usize]);
    }

    /// `set(const StringBase&, offset, count)` 0x00437c70 / 0x00437ef0.
    pub fn set_sub_string_base(&mut self, other: &Self, offset: i32, count: i32) {
        let total = offset + count;
        if total >= 0 {
            let len = other.length() as i32;
            if offset < len {
                let mut off = offset;
                let mut cnt = count;
                if off < 0 {
                    off = 0;
                    cnt = total;
                }
                if len <= off + cnt {
                    cnt = len - off;
                }
                let s = other.str();
                let take = s[off as usize..off as usize + cnt.max(0) as usize].to_vec();
                self.set(&take);
                return;
            }
        }
        self.release_buffer();
    }

    // ------------------------------------------------------------------
    // concat family
    // ------------------------------------------------------------------

    /// `concat(const T*, n)` 0x004369a0 / 0x00437410; thunks 0x00405629 /
    /// 0x0040564e / 0x00405692 / 0x004056b8 and `concat(StringBase&)`
    /// 0x00406987 / 0x00406a2a pass `(str, len)`. `n == 0`: no-op.
    pub fn concat(&mut self, src: &[T]) {
        if src.is_empty() {
            return;
        }
        if self.buf.is_none() {
            self.set(src);
            return;
        }
        let len = self.length();
        self.ensure_unique(len + src.len(), true, true);
        self.write_at(len, src);
    }

    // ------------------------------------------------------------------
    // compare family
    // ------------------------------------------------------------------

    /// `compare(char)` 0x00435f60 / 0x00436150: byte loop against a
    /// 1-element string; all-equal ⇒ `len − 1` (empty string ⇒ −1).
    pub fn compare_elem(&self, c: T) -> i32 {
        let s = self.str();
        for &b in s {
            if b != c {
                return if b.key() < c.key() { -1 } else { 1 };
            }
        }
        s.len() as i32 - 1
    }

    /// `compare(const T*, n)` (0x00406733 → `FUN_00405816`): unsigned
    /// memcmp of the shorter prefix, tie ⇒ `len − n`. The
    /// `compare(const T*)` strlen wrappers are 0x004069b1 / (u16
    /// 0x00406a54) and `compare(StringBase&)` 0x004069d6 / 0x00406a7a.
    pub fn compare_cstr(&self, other: &[T]) -> i32 {
        let a = self.str();
        let m = a.len().min(other.len());
        for i in 0..m {
            let (x, y) = (a[i].key(), other[i].key());
            if x != y {
                return if x < y { -1 } else { 1 };
            }
        }
        a.len() as i32 - other.len() as i32
    }

    /// `compare(StringBase&)` 0x004069d6 / 0x00406a7a.
    pub fn compare_string_base(&self, other: &Self) -> i32 {
        self.compare_cstr(other.str())
    }

    /// `compareNoCase(const T*, n)` — 0x00437980 / 0x00406772 (char,
    /// `_memicmp`) and 0x00437a90 / 0x004067f0 (u16 → `FUN_00405344`
    /// `towlower` loop): folded compare of the shorter prefix, tie ⇒
    /// length difference. `compareNoCase(StringBase&)` 0x00406a00 /
    /// 0x00406aa4 forwards `(str, len)`.
    pub fn compare_no_case_cstr(&self, other: &[T]) -> i32 {
        let a = self.str();
        let m = a.len().min(other.len());
        for i in 0..m {
            let (fa, fb) = (a[i].fold(), other[i].fold());
            if fa != fb {
                return if fa < fb { -1 } else { 1 };
            }
        }
        a.len() as i32 - other.len() as i32
    }

    /// `compareNoCase(char)` 0x004379e0 / 0x00437af0: folds against the
    /// first element (size 0 when empty ⇒ result `len − 1`).
    pub fn compare_no_case_elem(&self, c: T) -> i32 {
        let s = self.str();
        if !s.is_empty() {
            let (fa, fb) = (s[0].fold(), c.fold());
            if fa != fb {
                return if fa < fb { -1 } else { 1 };
            }
        }
        s.len() as i32 - 1
    }

    /// `compareNoCase(StringBase&)` 0x00406a00 / 0x00406aa4.
    pub fn compare_no_case_string_base(&self, other: &Self) -> i32 {
        self.compare_no_case_cstr(other.str())
    }

    // ------------------------------------------------------------------
    // startsWith / endsWith
    // ------------------------------------------------------------------

    /// `startsWith(const T*, n)` 0x00435970 / 0x00436190 (u16 wrapper
    /// doing wcslen); empty prefix ⇒ true (checked before the length
    /// gate), `len < n` ⇒ false, then element compare. `startsWith(cstr)`
    /// 0x0042be9c and `startsWith(StringBase&)` 0x00435fb0 / 0x004361f0
    /// share these semantics (empty/null other ⇒ true via the same checks).
    pub fn starts_with(&self, prefix: &[T]) -> bool {
        if prefix.is_empty() {
            return true;
        }
        let s = self.str();
        s.len() >= prefix.len() && s[..prefix.len()] == *prefix
    }

    /// `startsWith(StringBase&)` 0x00435fb0 / 0x004361f0.
    pub fn starts_with_string_base(&self, prefix: &Self) -> bool {
        self.starts_with(prefix.str())
    }

    /// `startsWithNoCase(...)` — `(...,n)` 0x004359b0 / 0x00436250,
    /// cstr 0x00436000 / 0x004362b0, `StringBase&` 0x0042c42f /
    /// 0x00435b80. (The C strlen wrappers deref a null cstr — UB; Rust
    /// treats the empty slice as the empty prefix.)
    pub fn starts_with_no_case(&self, prefix: &[T]) -> bool {
        if prefix.is_empty() {
            return true;
        }
        let s = self.str();
        if s.len() < prefix.len() {
            return false;
        }
        s[..prefix.len()]
            .iter()
            .zip(prefix)
            .all(|(a, b)| a.fold() == b.fold())
    }

    /// `endsWith(const T*, n)` 0x00435a00 / 0x00435bd0; cstr thunks
    /// 0x0042bec1 / 0x00436310.
    pub fn ends_with(&self, suffix: &[T]) -> bool {
        if suffix.is_empty() {
            return true;
        }
        let s = self.str();
        s.len() >= suffix.len() && s[s.len() - suffix.len()..] == *suffix
    }

    /// `endsWith(StringBase&)` 0x00436060 / 0x00436350.
    pub fn ends_with_string_base(&self, suffix: &Self) -> bool {
        self.ends_with(suffix.str())
    }

    /// `endsWithNoCase(...)` — `(...,n)` 0x00435a50 / 0x00436390, cstr
    /// 0x004360d0 / 0x004363d0, `StringBase&` 0x00436110 / 0x00435c30.
    pub fn ends_with_no_case(&self, suffix: &[T]) -> bool {
        if suffix.is_empty() {
            return true;
        }
        let s = self.str();
        if s.len() < suffix.len() {
            return false;
        }
        let start = s.len() - suffix.len();
        s[start..]
            .iter()
            .zip(suffix)
            .all(|(a, b)| a.fold() == b.fold())
    }

    // ------------------------------------------------------------------
    // find
    // ------------------------------------------------------------------

    /// `find(char)` 0x0042c459 / 0x00435ab0: index of first occurrence
    /// (C returns a pointer; callers use `ptr − str`).
    pub fn find(&self, c: T) -> Option<usize> {
        self.str().iter().position(|&b| b == c)
    }

    /// `reverseFind(char)` 0x00435930 / 0x00435af0: last occurrence.
    pub fn reverse_find(&self, c: T) -> Option<usize> {
        self.str().iter().rposition(|&b| b == c)
    }

    // ------------------------------------------------------------------
    // mutators
    // ------------------------------------------------------------------

    /// `toLower` 0x00436a70 / `toUpper` 0x00436b60 (u16 0x004374e0 /
    /// 0x004375e0): reseat unique (re-alloc sized from the current
    /// length when shared or `len >= cap`), NUL-terminate, fold every
    /// element in place. C-locale ASCII only.
    pub fn to_lower(&mut self) {
        self.to_case(true);
    }
    pub fn to_upper(&mut self) {
        self.to_case(false);
    }
    fn to_case(&mut self, lower: bool) {
        if self.buf.is_none() {
            return;
        }
        let len = self.length();
        self.ensure_unique(len, true, false);
        let b = Rc::get_mut(self.buf.as_mut().unwrap()).unwrap();
        b.data[len] = T::NUL;
        for x in b.data[..len].iter_mut() {
            *x = if lower { x.to_lower() } else { x.to_upper() };
        }
    }

    /// `removeLastChar` 0x00436c50 / 0x004376e0: unique-reseat, then
    /// `len − 1`; reaching length 0 releases the buffer entirely (string
    /// becomes NONE, and further calls are no-ops).
    pub fn remove_last_char(&mut self) {
        if self.buf.is_none() {
            return;
        }
        let len = self.length();
        self.ensure_unique(len, true, false);
        let new_len = self.length() as i32 - 1;
        if new_len > 0 {
            let b = Rc::get_mut(self.buf.as_mut().unwrap()).unwrap();
            b.len = new_len as u16;
            b.data[new_len as usize] = T::NUL;
        } else {
            self.release_buffer();
        }
    }

    /// `trim` 0x00437cf0 / 0x00437f70: drop the leading `isspace` run (a
    /// `set` of the remainder), then the trailing run (via
    /// `removeLastChar`, which nulls the string when all is trimmed).
    pub fn trim(&mut self) {
        if self.buf.is_none() {
            return;
        }
        let lead = {
            let s = self.str();
            s.iter().position(|&c| !c.is_space()).unwrap_or(s.len())
        };
        if lead > 0 {
            let rest = self.str()[lead..].to_vec();
            self.set(&rest);
        }
        while let Some(b) = self.buf.as_ref() {
            let len = b.len as usize;
            if len == 0 || !b.data[len - 1].is_space() {
                break;
            }
            self.remove_last_char();
        }
    }

    /// `getBufferForRead(n)` 0x00436640 / 0x004370a0: hand out a writable
    /// buffer sized for `n` — reseating when null/shared/undersized — set
    /// `len = n` and NUL-terminate (contents stale until the caller
    /// writes them).
    pub fn get_buffer_for_read(&mut self, n: usize) {
        let reseat = match self.buf.as_ref() {
            None => true,
            Some(b) => (b.cap as usize) <= n || Rc::strong_count(b) != 1,
        };
        if reseat {
            if T::guard(n) {
                panic!("StringBase: buffer size overflow (length_error)");
            }
            let alloc = T::alloc_bytes(n);
            let cap = (alloc - 8) / T::WIDTH;
            let nb = Buf {
                len: 0,
                cap: cap as u16,
                data: vec![T::NUL; cap],
            };
            self.release_buffer();
            self.buf = Some(Rc::new(nb));
        } else {
            let b = Rc::get_mut(self.buf.as_mut().unwrap()).unwrap();
            b.data[b.len as usize] = T::NUL;
        }
        let b = Rc::get_mut(self.buf.as_mut().unwrap()).unwrap();
        b.len = n as u16;
        b.data[n] = T::NUL;
    }

    /// `nextToken(StringBase* out, const T* delims)` 0x00436d90 /
    /// 0x00437830. `self` is the remainder cursor, `out` receives the
    /// token. Returns false (clearing **both** strings, per the
    /// `releaseBuffer(this); releaseBuffer(param_1)` branch) when no
    /// token remains. Empty `delims` ⇒ the whole string is one token,
    /// remainder NONE.
    pub fn next_token(&mut self, out: &mut Self, delims: &[T]) -> bool {
        if self.is_empty() {
            return false;
        }
        let (start, end) = {
            let s = self.str();
            let is_delim = |c: &T| delims.contains(c);
            let start = s.iter().position(|c| !is_delim(c)).unwrap_or(s.len());
            let end = s[start..]
                .iter()
                .position(is_delim)
                .map_or(s.len(), |p| start + p);
            (start, end)
        };
        if end <= start {
            self.release_buffer();
            out.release_buffer();
            return false;
        }
        let token = self.str()[start..end].to_vec();
        let rest = self.str()[end..].to_vec();
        out.set(&token);
        // set with empty rest ⇒ release: remainder becomes NONE, as in C.
        self.set(&rest);
        true
    }

    /// `format_va(char*/ushort*, va_list)` 0x00436d40 / 0x004377e0 (and via
    /// the varargs wrappers `format` 0x00437d90 / 0x00437ff0):
    /// `_vsnprintf(buf, 0x2000, fmt, ap)` / `vswprintf(buf, 0x2000, fmt, ap)`
    /// into a 0x2000-element stack buffer, then `set(this, buf, n)`. On
    /// truncation MSVC returns -1 and the game passes `(size_t)-1` to `set`,
    /// which trips the grow guard's `length_error` — mirrored by the panic.
    pub fn format_va(&mut self, fmt: &[T], args: &[FmtArg<'_, T>]) {
        let (buf, n) = vsnprintf_like(fmt, args);
        self.set_len(&buf, if n < 0 { usize::MAX } else { n as usize });
    }

    /// The `format_va(StringBase&, va_list)` overloads 0x00437920 /
    /// 0x00437a30 (and `AsciiString::format(AsciiString&)` 0x004381e0): a
    /// null fmt pointer formats against `TheNullChr` (empty fmt), so Rust
    /// models the parameter as `Option`.
    pub fn format_va_sb(&mut self, fmt: Option<&Self>, args: &[FmtArg<'_, T>]) {
        match fmt {
            Some(f) => {
                let fs = f.str().to_vec();
                self.format_va(&fs, args);
            }
            None => self.format_va(&[T::NUL], args),
        }
    }

    /// Varargs wrappers 0x00437d90 / 0x00437ff0 / 0x00438150: identical tail.
    pub fn format(&mut self, fmt: &[T], args: &[FmtArg<'_, T>]) {
        self.format_va(fmt, args);
    }

    /// Core `set(const T*, size_t len)` (the tail the strlen/single-char
    /// wrappers 0x004055f5 / 0x0040561a funnel into) with the game's exact
    /// `len` honoured: `len != src.len()` only happens on the format
    /// truncation path where `len` is `(size_t)-1`.
    fn set_len(&mut self, src: &[T], len: usize) {
        if len == 0 {
            self.release_buffer();
            return;
        }
        if len != src.len() && T::guard(len) {
            panic!("StringBase: buffer size overflow (length_error)");
        }
        self.set(src);
    }
}

/// Argument list for `format`/`format_va` (the C `...` list, values already
/// default-promoted as the CRT sees them: `float` arrives as `f64`).
#[derive(Clone, Copy, Debug)]
pub enum FmtArg<'a, T: SbChar> {
    Str(&'a [T]), // %s
    Ch(T),        // %c
    I32(i32),     // %d %i (also the promoted char for %c in C callers)
    U32(u32),     // %u %x %X %o
    F64(f64),     // %f %F %e %E %g %G
}

fn pad_spaces<T: SbChar>(out: &mut Vec<T>, trunc: &mut bool, cap: usize, n: usize) {
    for _ in 0..n {
        if out.len() < cap {
            out.push(T::lit(b' '));
        } else {
            *trunc = true;
        }
    }
}

fn pad_zeroes<T: SbChar>(out: &mut Vec<T>, trunc: &mut bool, cap: usize, n: usize) {
    for _ in 0..n {
        if out.len() < cap {
            out.push(T::lit(b'0'));
        } else {
            *trunc = true;
        }
    }
}

/// MSVC `_vsnprintf` / `vswprintf` over a `0x2000`-element buffer: returns
/// `(written, n)` where `n` = element count on success (NUL excluded); MSVC
/// fills the buffer with no NUL and returns -1 exactly when the output does
/// not fit (`out.len() >= 0x2000`). Supports the directive grammar the game
/// uses: flags `[-+ 0#]`, decimal width (`*` consumes a promoted `I32`),
/// precision (`.*` likewise), length modifiers parsed and ignored (already
/// promoted), conversions `d i u x X o c s f F e E g G %`; any other
/// specifier is echoed verbatim as MSVC does.
fn vsnprintf_like<T: SbChar>(fmt: &[T], args: &[FmtArg<'_, T>]) -> (Vec<T>, i32) {
    const CAP: usize = 0x2000;
    // C-string semantics: the fmt pointer stops at its NUL terminator.
    let flen = fmt.iter().position(|c| c.key() == 0).unwrap_or(fmt.len());
    let fmt = &fmt[..flen];
    let mut out: Vec<T> = Vec::new();
    let mut trunc = false;
    let mut ai = 0usize;
    let mut i = 0usize;
    macro_rules! emit {
        ($e:expr) => {{
            let e = $e;
            if out.len() < CAP {
                out.push(e);
            } else {
                trunc = true;
            }
        }};
    }
    macro_rules! emit_asc {
        ($b:expr) => {{
            for &b in $b {
                emit!(T::lit(b));
            }
        }};
    }
    fn digits<T: SbChar>(fmt: &[T], i: &mut usize) -> usize {
        let mut w: usize = 0;
        while *i < fmt.len() {
            let k = fmt[*i].key();
            if (0x30..=0x39).contains(&k) {
                w = w.saturating_mul(10).saturating_add((k - 0x30) as usize);
                *i += 1;
            } else {
                break;
            }
        }
        w
    }
    while i < fmt.len() {
        if fmt[i].key() != 0x25 {
            emit!(fmt[i]);
            i += 1;
            continue;
        }
        i += 1;
        let (mut left, mut plus, mut space, mut zero, mut alt) =
            (false, false, false, false, false);
        while i < fmt.len() {
            match fmt[i].key() {
                0x2d => left = true,
                0x2b => plus = true,
                0x20 => space = true,
                0x30 => zero = true,
                0x23 => alt = true,
                _ => break,
            }
            i += 1;
        }
        let mut width = digits(fmt, &mut i);
        if i < fmt.len() && fmt[i].key() == 0x2a {
            i += 1;
            if let Some(FmtArg::I32(v)) = args.get(ai) {
                if *v < 0 {
                    left = true;
                    width = (*v as i64).unsigned_abs() as usize;
                } else {
                    width = *v as usize;
                }
                ai += 1;
            }
        }
        let mut prec: Option<usize> = None;
        if i < fmt.len() && fmt[i].key() == 0x2e {
            i += 1;
            if i < fmt.len() && fmt[i].key() == 0x2a {
                i += 1;
                if let Some(FmtArg::I32(v)) = args.get(ai) {
                    if *v >= 0 {
                        prec = Some(*v as usize);
                    }
                    ai += 1;
                }
            } else {
                prec = Some(digits(fmt, &mut i));
            }
        }
        while i < fmt.len() && matches!(fmt[i].key(), 0x68 | 0x6c | 0x4c | 0x4a | 0x5a) {
            i += 1; // length modifier: already promoted in the Rust API
        }
        if i >= fmt.len() {
            emit!(T::lit(b'%'));
            break;
        }
        let conv = fmt[i].key();
        i += 1;
        match conv {
            0x25 => emit!(T::lit(b'%')), // %%
            0x64 | 0x69 => {
                // d/i
                let v = match args.get(ai) {
                    Some(FmtArg::I32(v)) => *v as i64,
                    Some(FmtArg::U32(v)) => *v as i32 as i64,
                    Some(FmtArg::Ch(c)) => c.key() as i8 as i64,
                    _ => 0,
                };
                ai += 1;
                let neg = v < 0;
                let mut core = (v.unsigned_abs()).to_string().into_bytes();
                if let Some(p) = prec {
                    if core.len() < p {
                        core.splice(0..0, std::iter::repeat(b'0').take(p - core.len()));
                    }
                    if p == 0 && core == b"0" {
                        core.clear();
                    }
                }
                let sign = if neg {
                    b'-'
                } else if plus {
                    b'+'
                } else if space {
                    b' '
                } else {
                    0
                };
                let sgn = if sign != 0 { 1 } else { 0 };
                let pad = width.saturating_sub(core.len() + sgn);
                if left {
                    if sign != 0 {
                        emit!(T::lit(sign));
                    }
                    emit_asc!(&core);
                    pad_spaces(&mut out, &mut trunc, CAP, pad);
                } else if zero {
                    if sign != 0 {
                        emit!(T::lit(sign));
                    }
                    pad_zeroes(&mut out, &mut trunc, CAP, pad);
                    emit_asc!(&core);
                } else {
                    pad_spaces(&mut out, &mut trunc, CAP, pad);
                    if sign != 0 {
                        emit!(T::lit(sign));
                    }
                    emit_asc!(&core);
                }
            }
            0x75 | 0x78 | 0x58 | 0x6f => {
                // u x X o
                let v = match args.get(ai) {
                    Some(FmtArg::U32(v)) => *v,
                    Some(FmtArg::I32(v)) => *v as u32,
                    Some(FmtArg::Ch(c)) => c.key() as u32,
                    _ => 0,
                };
                ai += 1;
                let radix: u32 = match conv {
                    0x75 => 10,
                    0x6f => 8,
                    _ => 16,
                };
                let mut core: Vec<u8> = Vec::new();
                let mut n = v;
                loop {
                    let d = (n % radix) as u8;
                    core.push(if d < 10 { b'0' + d } else { b'a' + d - 10 });
                    n /= radix;
                    if n == 0 {
                        break;
                    }
                }
                core.reverse();
                if conv == 0x58 {
                    for b in core.iter_mut() {
                        *b = b.to_ascii_uppercase();
                    }
                }
                if let Some(p) = prec {
                    while core.len() < p {
                        core.insert(0, b'0');
                    }
                    if p == 0 && v == 0 {
                        core.clear();
                    }
                }
                let prefix: &[u8] = if alt && v != 0 && radix == 16 {
                    if conv == 0x58 {
                        b"0X"
                    } else {
                        b"0x"
                    }
                } else if alt && v != 0 && radix == 8 {
                    b"0"
                } else {
                    b""
                };
                let pad = width.saturating_sub(core.len() + prefix.len());
                if left {
                    emit_asc!(prefix);
                    emit_asc!(&core);
                    pad_spaces(&mut out, &mut trunc, CAP, pad);
                } else if zero {
                    emit_asc!(prefix);
                    pad_zeroes(&mut out, &mut trunc, CAP, pad);
                    emit_asc!(&core);
                } else {
                    pad_spaces(&mut out, &mut trunc, CAP, pad);
                    emit_asc!(prefix);
                    emit_asc!(&core);
                }
            }
            0x63 => {
                // c
                let c = match args.get(ai) {
                    Some(FmtArg::Ch(c)) => *c,
                    Some(FmtArg::I32(v)) => T::from_key(*v as u8 as u32),
                    _ => T::NUL,
                };
                ai += 1;
                if left {
                    emit!(c);
                    pad_spaces(&mut out, &mut trunc, CAP, width.saturating_sub(1));
                } else {
                    pad_spaces(&mut out, &mut trunc, CAP, width.saturating_sub(1));
                    emit!(c);
                }
            }
            0x73 => {
                // s: elements up to NUL, precision caps the run
                let s: &[T] = match args.get(ai) {
                    Some(FmtArg::Str(s)) => *s,
                    _ => &[],
                };
                ai += 1;
                let end = s.iter().position(|c| c.key() == 0).unwrap_or(s.len());
                let take = prec.map_or(end, |p| end.min(p));
                let pad = width.saturating_sub(take);
                if left {
                    for &e in &s[..take] {
                        emit!(e);
                    }
                    pad_spaces(&mut out, &mut trunc, CAP, pad);
                } else {
                    pad_spaces(&mut out, &mut trunc, CAP, pad);
                    for &e in &s[..take] {
                        emit!(e);
                    }
                }
            }
            kf @ (0x66 | 0x46 | 0x65 | 0x45 | 0x67 | 0x47) => {
                // f F e E g G
                let v = match args.get(ai) {
                    Some(FmtArg::F64(v)) => *v,
                    Some(FmtArg::I32(v)) => *v as f64,
                    Some(FmtArg::U32(v)) => *v as f64,
                    _ => 0.0,
                };
                ai += 1;
                let asc = float_field(v, kf as u8, prec, alt);
                let sign = asc.first().copied().filter(|c| matches!(c, b'-' | b'+'));
                let body: &[u8] = if sign.is_some() { &asc[1..] } else { &asc };
                let pad = width.saturating_sub(asc.len());
                if left {
                    if let Some(sg) = sign {
                        emit!(T::lit(sg));
                    }
                    emit_asc!(body);
                    pad_spaces(&mut out, &mut trunc, CAP, pad);
                } else if zero {
                    if let Some(sg) = sign {
                        emit!(T::lit(sg));
                    }
                    pad_zeroes(&mut out, &mut trunc, CAP, pad);
                    emit_asc!(body);
                } else {
                    pad_spaces(&mut out, &mut trunc, CAP, pad);
                    emit_asc!(&asc);
                }
            }
            other => {
                // MSVC echoes unknown specifiers verbatim.
                emit!(T::lit(b'%'));
                emit!(T::from_key(other));
            }
        }
    }
    if trunc || out.len() == CAP {
        (out, -1)
    } else {
        let n = out.len() as i32;
        (out, n)
    }
}

/// f/e/g rendering in UCRT style: Rust core float formatting is correctly
/// rounded like UCRT; only the exponent spelling (`e+05`) differs.
fn float_field(v: f64, conv: u8, prec: Option<usize>, alt: bool) -> Vec<u8> {
    let up = matches!(conv, 0x46 | 0x45 | 0x47);
    let mut s = if v.is_nan() {
        if alt {
            "NAN".to_string()
        } else {
            "nan".to_string()
        }
    } else if v.is_infinite() {
        if alt {
            (if v < 0.0 { "-INF" } else { "INF" }).to_string()
        } else {
            (if v < 0.0 { "-inf" } else { "inf" }).to_string()
        }
    } else {
        let neg = v < 0.0 || (v == 0.0 && v.is_sign_negative());
        let a = v.abs();
        let rexp = |s: String| -> String {
            if let Some(x) = s.find('e') {
                let (m, e) = s.split_at(x);
                let e = &e[1..];
                let (esign, edig) = match e.strip_prefix('-') {
                    Some(d) => ('-', d),
                    None => ('+', e.strip_prefix('+').unwrap_or(e)),
                };
                let edig = edig.trim_start_matches('0');
                let edig = if edig.is_empty() { "0" } else { edig };
                let edig = format!("{edig:0>2}");
                format!("{m}e{esign}{edig}")
            } else {
                s
            }
        };
        let body = match conv {
            0x66 | 0x46 => format!("{:.*}", prec.unwrap_or(6), a),
            0x65 | 0x45 => rexp(format!("{:.*e}", prec.unwrap_or(6), a)),
            _ => {
                // g/G: significant digits (default 6); e-style when the
                // exponent is outside [-4, precision).
                let p = prec.unwrap_or(6).max(1);
                let exp = if a == 0.0 {
                    0
                } else {
                    a.log10().floor() as i32
                };
                if exp < -4 || exp >= p as i32 {
                    rexp(format!("{:.*e}", p - 1, a))
                } else {
                    let mut s = format!("{:.*}", (p as i32 - 1 - exp).max(0) as usize, a);
                    if !alt {
                        while s.ends_with('0') {
                            s.pop();
                        }
                        if s.ends_with('.') {
                            s.pop();
                        }
                    }
                    s
                }
            }
        };
        format!("{}{}", if neg { "-" } else { "" }, body)
    };
    if up {
        s = s.to_ascii_uppercase();
    }
    s.into_bytes()
}

/// 0x00a0bea4 `Xfer::operator&(AsciiString&)` — tag 'str' (0x61737472)
/// length framing:
/// LOAD (`!IsStoring`): 1-byte length transferred (local starts 0); if it
/// comes back 0xff a tag-0 *dword* length follows; len 0 →
/// `releaseBuffer`; else `getBufferForRead(len)` + tag-0 raw `len` bytes +
/// NUL terminator (modeled by `set`, which reproduces length/capacity
/// state; NUL is implicit in the game header model).
/// STORE: length byte = buffer length (sentinel 0xff + tag-0 dword when
/// >= 0xff), then tag-0 raw `len` bytes of buffer data (transferred
/// in place by the game — mirrored via scratch + write-back `set`).
pub fn xfer_ascii_string<X: sage_xfer::Xfer>(x: &mut X, s: &mut AsciiString) {
    const TAG_STR: u32 = 0x6173_7472; // "str"
    let mut len: u32;
    if x.is_storing() {
        len = s.length() as u32;
        if len < 0xff {
            let mut b = [len as u8];
            x.xfer_internal(TAG_STR, &mut b);
            len = b[0] as u32;
        } else {
            x.xfer_internal(TAG_STR, &mut [0xffu8]);
            let mut b = len.to_le_bytes();
            x.xfer_internal(0, &mut b);
            len = u32::from_le_bytes(b);
        }
        let mut data: Vec<u8> = s.str().iter().map(|c| c.0).collect();
        data.resize(len as usize, 0);
        x.xfer_internal(0, &mut data);
        if len != 0 {
            // game wrote back into the buffer at +8; visible length stays
            // the transferred framing length.
            s.set(&data.iter().map(|b| Ascii(*b)).collect::<Vec<_>>());
        }
    } else {
        let mut b = [0u8];
        x.xfer_internal(TAG_STR, &mut b);
        len = b[0] as u32;
        if len == 0xff {
            let mut d = 0u32.to_le_bytes();
            x.xfer_internal(0, &mut d);
            len = u32::from_le_bytes(d);
        }
        if len == 0 {
            s.release_buffer();
        } else {
            let mut data = vec![0u8; len as usize];
            x.xfer_internal(0, &mut data);
            s.set(&data.iter().map(|c| Ascii(*c)).collect::<Vec<_>>());
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn a(s: &str) -> Vec<Ascii> {
        s.bytes().map(Ascii).collect()
    }
    fn u(s: &str) -> Vec<Utf16> {
        s.encode_utf16().map(Utf16).collect()
    }

    #[test]
    fn set_reproduces_game_capacity() {
        let mut s = StringAscii::new();
        assert_eq!(s.capacity(), 0);
        s.set(&a("hi")); // alloc (2+12)&!3 = 12 ⇒ cap 4
        assert_eq!((s.length(), s.capacity()), (2, 4));
        assert_eq!(s.ref_count(), 1);
        s.set(&[]); // n == 0 releases: string becomes NONE
        assert!(s.is_empty() && s.ref_count() == 0 && s.capacity() == 0);
    }

    #[test]
    fn cow_shares_until_mutation() {
        let mut s = StringAscii::new();
        s.set(&a("hello")); // cap (5+12)&!3−8 = 8
        let mut t = s.clone();
        assert_eq!(s.ref_count(), 2);
        assert_eq!(t.str(), s.str());
        t.concat(&a("!")); // rc != 1 ⇒ fresh buffer, contents preserved
        assert_eq!(t.str(), a("hello!").as_slice());
        assert_eq!(s.str(), a("hello").as_slice());
        assert_eq!(s.ref_count(), 1);
        assert_eq!(t.ref_count(), 1);
        // in-place path: unique + len+1 < cap keeps capacity
        t.concat(&a("?")); // 7 >= 8? no: 7 < 8 ⇒ in-place
        assert_eq!(t.capacity(), 8);
    }

    #[test]
    fn concat_growth_bump_exact() {
        let mut s = StringAscii::new();
        s.set(&a("ab")); // cap 4
        s.concat(&a("cd")); // needed 4 !< cap 4 ⇒ bump max(4, 4+2−1)=5 ⇒ cap 8
        assert_eq!(s.str(), a("abcd").as_slice());
        assert_eq!(s.capacity(), 8);
        let mut w = StringUtf16::new();
        w.set(&u("Hi")); // alloc (4+13)&!3 = 16 ⇒ cap 4 units
        assert_eq!(w.capacity(), 4);
        w.concat(&u("!")); // 3 < 4 ⇒ in-place
        assert_eq!(w.capacity(), 4);
        w.concat(&u("!")); // 4 >= 4 ⇒ bump 5 ⇒ (10+13)&!3 = 20 ⇒ cap 6
        assert_eq!(w.capacity(), 6);
        assert_eq!(w.str(), u("Hi!!").as_slice());
    }

    #[test]
    fn set_sub_clamping_verbatim() {
        let src = a("hello");
        let cases: [(i32, i32); 8] = [
            (2, 100), // ⇒ "llo"
            (-1, 3),  // ⇒ "he"
            (10, 5),  // ⇒ NONE
            (-5, 5),  // total 0 ⇒ set(...,0) ⇒ NONE
            (0, 5),   // ⇒ "hello"
            (4, 1),   // ⇒ "o"
            (-3, 2),  // total −1 ⇒ NONE
            (5, 0),   // offset >= len ⇒ NONE
        ];
        let want = ["llo", "he", "NONE", "NONE", "hello", "o", "NONE", "NONE"];
        for ((off, cnt), want) in cases.into_iter().zip(want) {
            let mut s = StringAscii::new();
            s.set_sub_cstr(&src, off, cnt);
            let got: String = s.str().iter().map(|&x| x.0 as char).collect();
            if want == "NONE" {
                assert!(s.ref_count() == 0, "({off},{cnt}) ⇒ {got:?}");
            } else {
                assert_eq!(got, want, "({off},{cnt})");
            }
        }
        let mut s = StringAscii::new();
        let other = {
            let mut o = StringAscii::new();
            o.set(&a("hello"));
            o
        };
        s.set_sub_string_base(&other, 1, 1000);
        assert_eq!(s.str(), a("ello").as_slice());
        s.set_sub_string_base(&other, 7, 2);
        assert!(s.ref_count() == 0 && s.is_empty());
        s.set_sub_string_base(&other, -2, 4);
        assert_eq!(s.str(), a("he").as_slice());
    }

    #[test]
    fn compare_semantics() {
        let mut s = StringAscii::new();
        s.set(&a("AB"));
        assert_eq!(s.compare_elem(Ascii(b'A')), 1); // all-equal prefix, len−1
        s.set(&a("A"));
        assert_eq!(s.compare_elem(Ascii(b'A')), 0);
        assert_eq!(s.compare_elem(Ascii(b'B')), -1);
        let empty = StringAscii::new();
        assert_eq!(empty.compare_elem(Ascii(b'A')), -1);
        s.set(&a("abc"));
        assert_eq!(s.compare_cstr(&a("abd")), -1);
        assert_eq!(s.compare_cstr(&a("ab")), 1); // tie ⇒ len diff
        assert_eq!(s.compare_cstr(&a("AB")), 1); // unsigned memcmp
        assert_eq!(s.compare_no_case_cstr(&a("AbC")), 0);
        assert_eq!(s.compare_no_case_cstr(&a("ABC")), 0);
        assert_eq!(s.compare_no_case_elem(Ascii(b'A')), 2); // folds ONE elem, ⇒ len−1
        assert_eq!(s.compare_no_case_elem(Ascii(b'B')), -1);
        assert_eq!(empty.compare_no_case_elem(Ascii(b'A')), -1);
        // C-locale folding: non-ASCII pairs stay distinct (0xC4 ≠ 0xE4)
        s.set(&[Ascii(0xc4)]);
        assert_eq!(s.compare_no_case_cstr(&[Ascii(0xe4)]), -1);
    }

    #[test]
    fn prefix_suffix_and_find() {
        let mut s = StringAscii::new();
        s.set(&a("Hello"));
        assert!(s.starts_with(&a("He")));
        assert!(!s.starts_with(&a("Heello")));
        assert!(s.starts_with(&[]));
        assert!(s.starts_with_no_case(&a("hEL")));
        assert!(!s.starts_with_no_case(&a("helLox"))); // longer than string
        assert!(s.ends_with(&a("lo")));
        assert!(s.ends_with_no_case(&a("LO")));
        assert!(s.ends_with(&[]));
        assert_eq!(s.find(Ascii(b'l')), Some(2));
        assert_eq!(s.reverse_find(Ascii(b'l')), Some(3));
        assert_eq!(s.find(Ascii(b'z')), None);
        assert_eq!(s.char_at(1), Ascii(b'e'));
        let none = StringAscii::new();
        assert_eq!(none.char_at(0), Ascii(0));
        assert!(!none.starts_with(&a("x"))); // empty string, non-empty prefix
    }

    #[test]
    fn none_literal() {
        // DAT_00bbdbc8 = "None"
        for lit in ["None", "noNe", "NONE"] {
            let mut s = StringAscii::new();
            s.set(&a(lit));
            assert!(s.is_none() && s.is_not_none() == false && s.is_not_empty());
        }
        for lit in ["NoneX", "", "non"] {
            let mut s = StringAscii::new();
            s.set(&a(lit));
            assert!(!s.is_none());
        }
        assert!(StringAscii::new().is_none() == false);
    }

    #[test]
    fn mutators_trim_remove_case() {
        let mut s = StringAscii::new();
        s.set(&a("\t a\tb \r \n"));
        s.trim();
        assert_eq!(s.str(), a("a\tb").as_slice());
        s.set(&a("   "));
        s.trim();
        assert!(s.ref_count() == 0); // everything trimmed ⇒ released
        s.set(&a("ab"));
        s.remove_last_char();
        assert_eq!(s.str(), a("a").as_slice());
        s.remove_last_char(); // len reaches 0 ⇒ buffer released ⇒ NONE
        assert!(s.ref_count() == 0); // len hit 0 ⇒ buffer released
        s.remove_last_char(); // NONE: no-op
        assert_eq!(s.capacity(), 0);
        s.set(&[Ascii(b'A'), Ascii(b'b'), Ascii(b'C'), Ascii(0xff)]);
        s.to_lower();
        assert_eq!(
            s.str(),
            [Ascii(b'a'), Ascii(b'b'), Ascii(b'c'), Ascii(0xff)]
        );
        s.to_upper();
        assert_eq!(
            s.str(),
            [Ascii(b'A'), Ascii(b'B'), Ascii(b'C'), Ascii(0xff)]
        );
        let mut w = StringUtf16::new();
        w.set(vec![Utf16(0x41), Utf16(0xc4)].as_slice());
        w.to_lower();
        assert_eq!(w.str(), [Utf16(0x61), Utf16(0xc4)]); // C locale: Ä unchanged
    }

    #[test]
    fn next_token_sequence() {
        let mut s = StringAscii::new();
        s.set(&a(",abc,,de"));
        let mut out = StringAscii::new();
        assert!(s.next_token(&mut out, &a(",")));
        assert_eq!(out.str(), a("abc").as_slice());
        assert_eq!(s.str(), a(",,de").as_slice());
        assert!(s.next_token(&mut out, &a(",")));
        assert_eq!(out.str(), a("de").as_slice());
        assert_eq!(s.ref_count(), 0); // empty remainder releases
        assert!(!s.next_token(&mut out, &a(",")));
        s.set(&a(",,,"));
        assert!(!s.next_token(&mut out, &a(","))); // clears BOTH strings
        assert_eq!(out.ref_count(), 0);
        s.set(&a("x y"));
        assert!(s.next_token(&mut out, &[])); // empty delims ⇒ whole token
        assert_eq!(out.str(), a("x y").as_slice());
        assert_eq!(s.ref_count(), 0);
    }

    #[test]
    fn buffer_for_read_and_swap() {
        let mut s = StringAscii::new();
        s.get_buffer_for_read(3);
        assert_eq!((s.length(), s.str() == [Ascii(0); 3]), (3, true));
        s.get_buffer_for_read(0);
        assert!(s.is_empty() && s.capacity() > 0); // len := n, buffer alive
        let mut t = s.clone();
        s.get_buffer_for_read(1); // shared ⇒ reseat
        assert_eq!(s.ref_count(), 1);
        assert_eq!(t.ref_count(), 1);
        s.swap(&mut t);
        assert_eq!(s.length(), 0);
        assert_eq!(t.length(), 1);
        let mut n = StringAscii::new();
        s.swap(&mut n);
        assert!(n.length() == 0 && s.length() == 0);
    }

    #[test]
    fn length_guards_panic_like_game_throw() {
        // char: throw iff n + 9 > 0x7fff (std::length_error modeled panic)
        let ok = a(&"x".repeat(0x7fff - 9));
        let mut s = StringAscii::new();
        s.set(&ok);
        assert_eq!(s.length(), 0x7fff - 9);
        let boom = a(&"x".repeat(0x7fff - 8));
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut s = StringAscii::new();
            s.set(&boom);
        }));
        assert!(r.is_err());
        // u16: throw iff 2n + 10 > 0x7fff ⇒ 16378 ok, 16379 panics
        let w: Vec<Utf16> = std::iter::repeat(Utf16(b'y' as u16)).take(16378).collect();
        let mut w1 = StringUtf16::new();
        w1.set(&w);
        assert_eq!(w1.capacity(), 16380);
        let w: Vec<Utf16> = std::iter::repeat(Utf16(b'y' as u16)).take(16379).collect();
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut w2 = StringUtf16::new();
            w2.set(&w);
        }));
        assert!(r.is_err());
    }

    #[test]
    fn utf16_none_and_nocase() {
        let mut w = StringUtf16::new();
        w.set(&u("None"));
        assert!(w.is_none());
        w.set(&u("HeLlO"));
        assert_eq!(w.compare_no_case_cstr(&u("hello")), 0);
        assert_ne!(
            w.compare_no_case_cstr(&[Utf16(0xc4)]),
            0,
            "C-locale towlower: non-ASCII never folds"
        );
        w.set(&u("Hi"));
        assert_eq!(w.compare_cstr(&u("Ha")), 1); // 'i'(0x69) > 'a'(0x61)
        assert!(w.starts_with_no_case(&u("hi")));
    }

    #[test]
    fn xfer_ascii_string_framing() {
        // load: short length, 0xff-extended length, and zero-release paths
        struct Rx {
            ev: Vec<(u32, Vec<u8>)>,
            stream: std::collections::VecDeque<Vec<u8>>,
            storing: bool,
        }
        impl sage_xfer::Xfer for Rx {
            fn xfer_internal(&mut self, tag: u32, v: &mut [u8]) {
                let inb = self.stream.pop_front().unwrap_or_else(|| v.to_vec());
                self.ev.push((tag, v.to_vec()));
                if inb.len() == v.len() {
                    v.copy_from_slice(&inb);
                }
            }
            fn is_storing(&self) -> bool {
                self.storing
            }
        }
        let mut rx = Rx {
            ev: vec![],
            stream: vec![vec![3u8], b"abc".to_vec()].into(),
            storing: false,
        };
        let mut s = AsciiString::new();
        xfer_ascii_string(&mut rx, &mut s);
        assert_eq!(rx.ev[0].0, 0x6173_7472);
        assert_eq!(rx.ev[1].0, 0);
        assert_eq!(s.str(), b"abc");
        // 0xff sentinel path
        let big = "x".repeat(300);
        let mut rx = Rx {
            ev: vec![],
            stream: vec![
                vec![0xffu8],
                300u32.to_le_bytes().to_vec(),
                big.as_bytes().to_vec(),
            ]
            .into(),
            storing: false,
        };
        let mut s = AsciiString::new();
        xfer_ascii_string(&mut rx, &mut s);
        assert_eq!(s.length(), 300);
        // zero length releases
        let mut s2 = AsciiString::new();
        s2.set(&a("hi"));
        let mut rx = Rx {
            ev: vec![],
            stream: vec![vec![0u8]].into(),
            storing: false,
        };
        xfer_ascii_string(&mut rx, &mut s2);
        assert_eq!(s2.length(), 0); // len-0 load → releaseBuffer
                                    // store path: 2 events for a short string (framing + raw)
        let mut s3 = AsciiString::new();
        s3.set(&a("hello"));
        let mut rx = Rx {
            ev: vec![],
            stream: vec![].into(),
            storing: true,
        };
        xfer_ascii_string(&mut rx, &mut s3);
        assert_eq!(
            rx.ev,
            vec![(0x6173_7472, vec![5u8]), (0, b"hello".to_vec())]
        );
    }
}

// ---------------------------------------------------------------------
// AsciiString / UnicodeString (thin named wrappers around the template)
// ---------------------------------------------------------------------

/// `AsciiString` — `StringBase<char>` plus translate/`+=` sugar. Ctor
/// thunks 0x0040654a (cstr) / 0x0040655c (cstr+n) / 0x00406572 (char) /
/// 0x00406584 (self,off,cnt) / 0x0040659e (cstr,off,cnt) / 0x005d8f56
/// (copy) forward to the mirrored StringBase ctors; `operator=`
/// 0x00401733 / 0x004065b8 / 0x004065ca → `set*`; `operator+=`
/// 0x004065e8 / 0x004065fa / 0x00406d12 → `concat`. `format`
/// (0x00438150 / 0x004381e0) unmirrored (sprintf parity, L2).
pub type AsciiString = StringBase<Ascii>;

/// `UnicodeString` — `StringBase<unsigned_short>`. Ctors 0x00406618
/// (copy) / 0x0040662a (u16*) / 0x0040663c (u16*,n) / 0x00406652 (u16) /
/// 0x00406664 (self,off,cnt) / 0x00acb6d0 (AsciiString& via translate);
/// `operator=` 0x00405394 / 0x0040667e / 0x00406690 → `set*`;
/// `operator+=` 0x004066ae / 0x004066c0 / 0x00406d24 → `concat`.
/// `format` (0x00acb5d0 / 0x00acb660) unmirrored.
pub type UnicodeString = StringBase<Utf16>;

impl StringAscii {
    /// `AsciiString::translate(const ushort*)` 0x00438170; wrapper
    /// 0x00438220 (UnicodeString&), ctor 0x00438250, `operator=`
    /// 0x004382b0. Conversion via `WideToMultiByte` (0x0042fb30 =
    /// `WideCharToMultiByte(0xfde9 = CP_UTF8, …, cchWide = -1)`):
    /// size probe → `ensureUniqueBufferOfSize(nbytes−1)` → write →
    /// `len = ret − 1`. Null/empty source or conversion failure
    /// (unpaired surrogates: `WideCharToMultiByte` returns 0) ⇒
    /// `releaseBuffer`.
    pub fn translate_utf16(&mut self, src: &[Utf16]) {
        if src.is_empty() || src[0].0 == 0 {
            self.release_buffer();
            return;
        }
        let units: Vec<u16> = src.iter().map(|u| u.0).take_while(|&u| u != 0).collect();
        let mut out: Vec<u8> = Vec::new();
        for r in std::char::decode_utf16(units.iter().copied()) {
            match r {
                Ok(c) => {
                    let mut b = [0u8; 4];
                    out.extend_from_slice(c.encode_utf8(&mut b).as_bytes());
                }
                Err(_) => {
                    self.release_buffer(); // WideCharToMultiByte ⇒ 0 ⇒ release
                    return;
                }
            }
        }
        self.ensure_unique(out.len(), false, false);
        let src_bytes: Vec<Ascii> = out.into_iter().map(Ascii).collect();
        self.write_at(0, &src_bytes);
    }

    /// `AsciiString::operator+=(const PooledString&)` 0x00a0bd19:
    /// `concat(entry->text)` — `m_null` entry text is "" ⇒ no-op.
    pub fn concat_pooled(&mut self, s: &crate::pooledstring::PooledString) {
        let v: Vec<Ascii> = s.text().bytes().map(Ascii).collect();
        self.concat(&v);
    }
}

impl StringUtf16 {
    /// `UnicodeString::translate(const char*)` 0x00acb5f0; wrapper
    /// 0x00acb6a0 (AsciiString&), ctor 0x00acb6d0. Conversion via
    /// `FUN_0042fb60` = `MultiByteToWideChar(0xfde9 = CP_UTF8, …,
    /// cbMultiByte = -1)`: probe → ensure(units) → write →
    /// `len = ret − 1`; null/empty/invalid-UTF-8 ⇒ `releaseBuffer`.
    pub fn translate_ascii(&mut self, src: &[Ascii]) {
        if src.is_empty() || src[0].0 == 0 {
            self.release_buffer();
            return;
        }
        let bytes: Vec<u8> = src.iter().map(|b| b.0).take_while(|&b| b != 0).collect();
        let units: Vec<Utf16> = match std::str::from_utf8(&bytes) {
            Ok(s) => s.encode_utf16().map(Utf16).collect(),
            Err(_) => {
                self.release_buffer(); // MultiByteToWideChar ⇒ 0 ⇒ release
                return;
            }
        };
        self.ensure_unique(units.len(), false, false);
        self.write_at(0, &units);
    }
}

#[cfg(test)]
mod wrapper_tests {
    use super::*;
    use crate::pooledstring::StringPool;

    fn u(s: &str) -> Vec<Utf16> {
        s.encode_utf16().map(Utf16).collect()
    }
    fn a(s: &str) -> Vec<Ascii> {
        s.bytes().map(Ascii).collect()
    }

    #[test]
    fn utf8_codepage_roundtrip() {
        // 0x0042fb30: WideCharToMultiByte(CP_UTF8 = 0xfde9). "café"
        // é U+00E9 → 2 UTF-8 bytes ⇒ ascii len 5? c,a,f,0xC3,0xA9 = 5.
        let mut s = AsciiString::new();
        s.translate_utf16(&u("café"));
        assert_eq!(s.length(), 5);
        assert_eq!(s.str()[3], Ascii(0xc3));
        assert_eq!(s.str()[4], Ascii(0xa9));
        let mut w = UnicodeString::new();
        w.translate_ascii(&s.str().to_vec());
        assert_eq!(w.str(), u("café").as_slice());
        // astral char: surrogate pair ⇒ 2 units back, 4 UTF-8 bytes
        let mut s2 = AsciiString::new();
        s2.translate_utf16(&u("\u{1F600}"));
        assert_eq!(s2.length(), 4);
        let mut w2 = UnicodeString::new();
        w2.translate_ascii(&s2.str().to_vec());
        assert_eq!(w2.str(), u("\u{1F600}").as_slice());
    }

    #[test]
    fn translate_failures_release_buffer() {
        let mut s = AsciiString::new();
        s.set(&a("keep"));
        s.translate_utf16(&[Utf16(0xd800)]); // unpaired surrogate ⇒ 0 ⇒ release
        assert_eq!(s.ref_count(), 0);
        let mut w = UnicodeString::new();
        w.set(&u("keep"));
        w.translate_ascii(&[Ascii(0xe9), Ascii(0x00)]); // invalid UTF-8 ⇒ release
        assert_eq!(w.ref_count(), 0);
        // empty source ⇒ release (0x00438170 first branch)
        s.set(&a("x"));
        s.translate_utf16(&[]);
        assert_eq!(s.ref_count(), 0);
    }

    #[test]
    fn pooled_plus_eq() {
        let mut pool = StringPool::new();
        let id = pool.set_string("Tree");
        let mut s = AsciiString::new();
        s.set(&a("Olog"));
        s.concat_pooled(&id); // operator+= PooledString 0x00a0bd19
        assert_eq!(s.str(), a("OlogTree").as_slice());
        let null = pool.set_string("");
        s.concat_pooled(&null); // m_null text "" ⇒ concat no-op
        assert_eq!(s.str(), a("OlogTree").as_slice());
    }
}

// ---------------------------------------------------------------------
// Xfer integration (Xfer::operator==(AsciiString&) 0x00a0bea4,
// Xfer::operator==(UnicodeString&) 0x00a0bf9e)
// ---------------------------------------------------------------------

impl<T: SbChar> StringBase<T> {
    /// Forward this string through an `Xfer` dispatcher.
    ///
    /// **Store** (`x.is_storing()` — vtable slot +0x08): length header —
    /// `('astr'|'ustr', &len_byte, 1)` when `len < 0xff`, else
    /// `('astr'|'ustr', [0xff], 1)` + `(0, &len_u32, 4)` — then the
    /// blob `(0, data, len * WIDTH)`. A released buffer stores length 0
    /// and the game's static `TheNullChr` (`0x00bbb5c4`) as a
    /// zero-size blob.
    ///
    /// **Load**: probe `(tag, &byte, 1)`; `0xff` ⇒ read u32 length;
    /// length 0 ⇒ `releaseBuffer`; else `getBufferForRead(len)`
    /// (unique, sets length, NUL-terminates), blob hook over the raw
    /// buffer bytes (`2 * len` for u16 — verified via the `add ecx,ecx`
    /// at 0x00a0c064/0x00a0c01a), then the game's second
    /// `getBufferForRead(len)` — idempotent here.
    pub fn xfer<X: sage_xfer::Xfer + ?Sized>(&mut self, x: &mut X) {
        if x.is_storing() {
            let len = self.length();
            if len < 0xff {
                let mut b = [len as u8];
                x.xfer_internal(T::XFER_TAG, &mut b);
            } else {
                let mut b = [0xffu8];
                x.xfer_internal(T::XFER_TAG, &mut b);
                let mut lb = (len as u32).to_le_bytes();
                x.xfer_internal(0, &mut lb);
            }
            let mut raw: Vec<u8> = Vec::with_capacity(len * T::WIDTH);
            for e in self.str() {
                let k = e.key();
                if T::WIDTH == 1 {
                    raw.push(k as u8);
                } else {
                    raw.extend_from_slice(&(k as u16).to_le_bytes());
                }
            }
            x.xfer_internal(0, &mut raw[..len * T::WIDTH]);
        } else {
            let mut b = [0u8];
            x.xfer_internal(T::XFER_TAG, &mut b);
            let mut len = b[0] as usize;
            if len == 0xff {
                let mut lb = [0u8; 4];
                x.xfer_internal(0, &mut lb);
                len = u32::from_le_bytes(lb) as usize;
            }
            if len == 0 {
                self.release_buffer();
                return;
            }
            self.get_buffer_for_read(len);
            let mut raw = vec![0u8; len * T::WIDTH];
            x.xfer_internal(0, &mut raw);
            let units: Vec<T> = if T::WIDTH == 1 {
                raw.iter().map(|&c| T::from_key(c as u32)).collect()
            } else {
                raw.chunks_exact(2)
                    .map(|c| T::from_key(u16::from_le_bytes([c[0], c[1]]) as u32))
                    .collect()
            };
            self.write_at(0, &units);
            // game calls getBufferForRead(len) again here; identical effect
        }
    }
}

#[cfg(test)]
mod xfer_tests {
    use super::*;
    use sage_xfer::Xfer;

    struct Store {
        out: Vec<u8>,
    }
    impl Xfer for Store {
        fn xfer_internal(&mut self, _tag: u32, value: &mut [u8]) {
            self.out.extend_from_slice(value);
        }
        fn is_storing(&self) -> bool {
            true
        }
    }
    struct Load {
        inb: std::collections::VecDeque<u8>,
    }
    impl Xfer for Load {
        fn xfer_internal(&mut self, _tag: u32, value: &mut [u8]) {
            for v in value.iter_mut() {
                *v = self.inb.pop_front().unwrap();
            }
        }
    }

    #[test]
    fn ascii_store_load_stream() {
        let mut s = AsciiString::new();
        s.set(&"Tree".bytes().map(Ascii).collect::<Vec<_>>());
        let mut st = Store { out: vec![] };
        s.xfer(&mut st);
        // header 'astr' 4 + blob 'Tree'
        assert_eq!(st.out, b"\x04Tree");
        // released buffer stores len 0
        let mut n = AsciiString::new();
        let mut st2 = Store { out: vec![] };
        n.xfer(&mut st2);
        assert_eq!(st2.out, vec![0]);
        // load: length byte + bytes
        let mut l = Load {
            inb: b"\x03cat".iter().copied().collect(),
        };
        let mut d = AsciiString::new();
        d.xfer(&mut l);
        assert_eq!(
            d.str(),
            "cat".bytes().map(Ascii).collect::<Vec<_>>().as_slice()
        );
        // length 0 ⇒ release
        let mut l0 = Load {
            inb: [0].iter().copied().collect(),
        };
        d.set(&"keep".bytes().map(Ascii).collect::<Vec<_>>());
        d.xfer(&mut l0);
        assert_eq!(d.ref_count(), 0);
    }

    #[test]
    fn extended_len_and_utf16_stream() {
        // 0xff escape path (load side; 255+ length is guard-tested cheaply)
        let bytes: Vec<u8> = std::iter::once(0xffu8)
            .chain(3u32.to_le_bytes())
            .chain(b"abc".iter().copied())
            .collect();
        let mut l = Load {
            inb: bytes.into_iter().collect(),
        };
        let mut d = AsciiString::new();
        d.xfer(&mut l);
        assert_eq!(d.length(), 3);
        // u16 blob width 2
        let mut w = UnicodeString::new();
        w.set(&[Utf16(0x48), Utf16(0x00e9)]);
        let mut st = Store { out: vec![] };
        w.xfer(&mut st);
        assert_eq!(st.out, [2, 0x48, 0x00, 0xe9, 0x00]);
        let mut l2 = Load {
            inb: st.out.into_iter().collect(),
        };
        let mut w2 = UnicodeString::new();
        w2.xfer(&mut l2);
        assert_eq!(w2.str(), w.str());
    }
}
#[cfg(test)]
mod format_tests {
    use super::*;

    fn a(s: &str) -> Vec<Ascii> {
        s.bytes().map(Ascii).collect()
    }
    fn fmtc(fmt: &str, args: &[FmtArg<'_, Ascii>]) -> String {
        let f: Vec<Ascii> = fmt.bytes().map(Ascii).collect();
        let (buf, n) = vsnprintf_like(&f, args);
        assert!(n >= 0);
        String::from_utf8(buf.iter().map(|a| a.0).collect()).unwrap()
    }

    #[test]
    fn directives_and_padding() {
        assert_eq!(
            fmtc(
                "%s=%d (%.2f)",
                &[FmtArg::Str(&a("hp")), FmtArg::I32(-7), FmtArg::F64(1.5)]
            ),
            "hp=-7 (1.50)"
        );
        assert_eq!(
            fmtc(
                "%5d|%-5d|%05d| %d",
                &[
                    FmtArg::I32(42),
                    FmtArg::I32(42),
                    FmtArg::I32(42),
                    FmtArg::I32(42)
                ]
            ),
            "   42|42   |00042| 42"
        );
        assert_eq!(
            fmtc(
                "%x %#X %o %%.%%",
                &[FmtArg::U32(0xab), FmtArg::U32(0xab), FmtArg::U32(8)]
            ),
            "ab 0XAB 10 %.%"
        );
        assert_eq!(
            fmtc(
                "%8s|%-4.2s|%c",
                &[
                    FmtArg::Str(&a("wide")),
                    FmtArg::Str(&a("wide")),
                    FmtArg::Ch(Ascii(b'z'))
                ]
            ),
            "    wide|wi  |z"
        );
        assert_eq!(
            fmtc(
                "%.0f %g %e",
                &[FmtArg::F64(2.5), FmtArg::F64(0.0001), FmtArg::F64(0.00001)]
            ),
            "2 0.0001 1.000000e-05"
        );
        // MSVC echoes unknown specifiers.
        assert_eq!(fmtc("%q", &[]), "%q");
    }

    #[test]
    fn truncation_matches_msvc_semantics() {
        // 0x2000-element buffer: exactly-full output truncates (returns -1,
        // no NUL), one-short fits.
        let big = vec![Ascii(b'a'); 0x2000];
        let f: Vec<Ascii> = b"%s".iter().map(|b| Ascii(*b)).collect();
        let (buf, n) = vsnprintf_like(&f, &[FmtArg::Str(&big)]);
        assert_eq!((buf.len(), n), (0x2000, -1));
        let ok = vec![Ascii(b'a'); 0x2000 - 1];
        let (buf, n) = vsnprintf_like(&f, &[FmtArg::Str(&ok)]);
        assert_eq!((buf.len(), n), (0x1FFF, 0x1FFF));
        // Game tail on truncation: set(this, buf, (size_t)-1) trips the
        // length_error guard.
        let mut s = AsciiString::new();
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            s.format_va(&f, &[FmtArg::Str(&big)])
        }));
        assert!(r.is_err());
    }

    #[test]
    fn format_sets_and_string_fmt() {
        let mut s = AsciiString::new();
        s.format(&a("n=%d"), &[FmtArg::I32(9)]);
        assert_eq!(
            String::from_utf8(s.str().iter().map(|x| x.0).collect()).unwrap(),
            "n=9"
        );
        s.format_va_sb(None, &[]); // null fmt => empty
        assert_eq!(s.str().len(), 0);
        // u16 flavor round-trips through the same engine.
        let fmt16: Vec<Utf16> = "%d".bytes().map(|b| Utf16(b as u16)).collect();
        let mut u = UnicodeString::new();
        u.format_va(&fmt16, &[FmtArg::I32(7)]);
        assert_eq!(u.str(), &[Utf16(b'7' as u16)]);
    }
}

// ---------------------------------------------------------------------------
// Wave 25: compare/scan primitives (CRT-glue rows behind the StringBase
// compare family). `short` elements are read sign-extended: the game
// computes mismatches as `(uint)(short)a - (uint)(short)b`, so elements
// >= 0x8000 differ from their raw key difference — mirrored exactly.
// ---------------------------------------------------------------------------

/// 0x004052f8 `wmemcmp` core: element-wise compare of `n` u16 elements;
/// first mismatch returns the `(uint)` difference of the sign-extended
/// `short` values, equal prefix returns 0.
pub fn u16_compare_n(a: &[Utf16], b: &[Utf16], n: usize) -> u32 {
    for i in 0..n {
        let x = a[i].0 as i16 as u32;
        let y = b[i].0 as i16 as u32;
        if x != y {
            return x.wrapping_sub(y);
        }
    }
    0
}

/// 0x00405344 `wmemicmp` core: `towlower` each element (C locale:
/// ASCII A-Z only) then compare as in 0x004052f8.
pub fn u16_compare_ci_n(a: &[Utf16], b: &[Utf16], n: usize) -> u32 {
    let low = |v: u16| -> u32 {
        let s = v as i16 as u32;
        if (0x41..=0x5a).contains(&v) {
            s.wrapping_add(32)
        } else {
            s
        }
    };
    for i in 0..n {
        let x = low(a[i].0);
        let y = low(b[i].0);
        if x != y {
            return x.wrapping_sub(y);
        }
    }
    0
}

/// 0x00405816: length-pair compare over char buffers: memcmp over
/// `min(la, lb)` bytes (unsigned byte difference), ties fall back to
/// `la - lb`.
pub fn char_len_compare(a: &[Ascii], la: i32, b: &[Ascii], lb: i32) -> i32 {
    let n = core::cmp::min(la, lb) as usize;
    for i in 0..n {
        let x = a[i].0 as u32;
        let y = b[i].0 as u32;
        if x != y {
            return (x.wrapping_sub(y)) as i32;
        }
    }
    la - lb
}

/// 0x00405841: same with `_memicmp` (per-byte `tolower`, C locale ASCII).
pub fn char_len_compare_ci(a: &[Ascii], la: i32, b: &[Ascii], lb: i32) -> i32 {
    let n = core::cmp::min(la, lb) as usize;
    for i in 0..n {
        let x = if (b'A'..=b'Z').contains(&a[i].0) {
            (a[i].0 + 32) as u32
        } else {
            a[i].0 as u32
        };
        let y = if (b'A'..=b'Z').contains(&b[i].0) {
            (b[i].0 + 32) as u32
        } else {
            b[i].0 as u32
        };
        if x != y {
            return (x.wrapping_sub(y)) as i32;
        }
    }
    la - lb
}

/// 0x0040586d: length-pair compare over u16 buffers via 0x004052f8.
pub fn u16_len_compare(a: &[Utf16], la: i32, b: &[Utf16], lb: i32) -> i32 {
    let n = core::cmp::min(la, lb) as usize;
    let d = u16_compare_n(a, b, n);
    if d == 0 {
        la - lb
    } else {
        d as i32
    }
}

/// 0x00405898: same via the 0x00405344 case-insensitive core.
pub fn u16_len_compare_ci(a: &[Utf16], la: i32, b: &[Utf16], lb: i32) -> i32 {
    let n = core::cmp::min(la, lb) as usize;
    let d = u16_compare_ci_n(a, b, n);
    if d == 0 {
        la - lb
    } else {
        d as i32
    }
}

/// 0x00435800: `iswspace` prefix skip (C locale: 0x09..=0x0d, 0x20);
/// returns the index of the first non-space element.
pub fn u16_skip_space(s: &[Utf16]) -> usize {
    let mut i = 0;
    while i < s.len() && matches!(s[i].0 as u32, 0x09..=0x0d | 0x20) {
        i += 1;
    }
    i
}

/// 0x00435d70 `strspn` core: index of the first char of `s` NOT in
/// `set` (empty set ⇒ 0).
pub fn ansi_strspn(s: &[Ascii], set: &[Ascii]) -> usize {
    for (i, c) in s.iter().enumerate() {
        if set.is_empty() || !set.contains(c) {
            return i;
        }
    }
    s.len()
}

/// 0x00435db0 `strcspn` core: index of the first char of `s` IN `set`
/// (empty set ⇒ end of `s`).
pub fn ansi_strcspn(s: &[Ascii], set: &[Ascii]) -> usize {
    for (i, c) in s.iter().enumerate() {
        if !set.is_empty() && set.contains(c) {
            return i;
        }
    }
    s.len()
}

/// 0x00435ed0: u16 `strspn` (same walk over `short` elements).
pub fn u16_strspn(s: &[Utf16], set: &[Utf16]) -> usize {
    ansi_walk_spn(s, set, true)
}

/// 0x00435f20: u16 `strcspn`.
pub fn u16_strcspn(s: &[Utf16], set: &[Utf16]) -> usize {
    ansi_walk_spn(s, set, false)
}

fn ansi_walk_spn<T: PartialEq>(s: &[T], set: &[T], spn: bool) -> usize {
    for (i, c) in s.iter().enumerate() {
        let in_set = !set.is_empty() && set.contains(c);
        if in_set != spn {
            return i;
        }
    }
    s.len()
}

#[cfg(test)]
mod cmp_tests {
    use super::*;
    fn a(s: &str) -> Vec<Ascii> {
        s.bytes().map(Ascii).collect()
    }
    fn u(s: &str) -> Vec<Utf16> {
        s.bytes().map(|b| Utf16(b as u16)).collect()
    }

    #[test]
    fn u16_compare_cores() {
        assert_eq!(
            u16_compare_n(&u("abc"), &u("abd"), 3),
            (b'c' as u32).wrapping_sub(b'd' as u32)
        );
        assert_eq!(u16_compare_n(&u("abc"), &u("abc"), 3), 0);
        assert_eq!(u16_compare_ci_n(&u("AbC"), &u("abc"), 3), 0);
        // sign-extension truth: 0x8000 reads as i16 -32768 vs 1 ⇒ diff
        // 0x8000 - 1 = 0x7FFF (raw), ext: 0xFFFF8000 - 1 = 0xFFFF7FFF.
        assert_eq!(u16_compare_n(&[Utf16(0x8000)], &[Utf16(1)], 1), 0xffff_7fff);
        // n bounds: only the compared prefix matters
        assert_eq!(u16_compare_n(&u("abc"), &u("axc"), 1), 0);
    }

    #[test]
    fn length_pair_wrappers() {
        assert_eq!(char_len_compare(&a("abc"), 3, &a("abd"), 3), -1);
        assert_eq!(char_len_compare(&a("ab"), 2, &a("abc"), 3), -1); // memcmp tie ⇒ la-lb
        assert_eq!(char_len_compare_ci(&a("ABC"), 3, &a("abc"), 3), 0);
        assert_eq!(char_len_compare_ci(&a("AB"), 2, &a("ab"), 3), -1);
        assert_eq!(u16_len_compare(&u("ab"), 2, &u("ab"), 2), 0);
        assert_eq!(
            u16_len_compare(&u("AB"), 2, &u("ab"), 2),
            ('A' as i32) - ('a' as i32)
        );
        assert_eq!(u16_len_compare_ci(&u("AB"), 2, &u("ab"), 3), -1);
    }

    #[test]
    fn scan_primitives() {
        assert_eq!(u16_skip_space(&u("  \t\n x")), 5);
        assert_eq!(u16_skip_space(&u("")), 0);
        assert_eq!(ansi_strspn(&a("aabx"), &a("ab")), 3);
        assert_eq!(ansi_strspn(&a("x"), &[]), 0); // empty set ⇒ first char is "not in set"
        assert_eq!(ansi_strcspn(&a("aabx"), &a("bx")), 2);
        assert_eq!(ansi_strcspn(&a("aa"), &[]), 2); // empty set ⇒ end of s
        assert_eq!(u16_strspn(&u("aabx"), &u("ab")), 3);
        assert_eq!(u16_strcspn(&u("aabx"), &u("bx")), 2);
    }
}
