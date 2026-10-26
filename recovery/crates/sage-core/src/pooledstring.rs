//! `PooledString` — SAGE's interned identifier string.
//!
//! Decomp ground model: one field, a pointer into a global intern table
//! (`m_hash` bucket array of 11131 slots at `0x00e0c8a0`, `hash % 0x2b7b`
//! verified at 0x00acb86e `mov ecx, 0x2b7b; div ecx`). Table entries live
//! in 0x40000-byte bump blocks (`FUN_004307f0(0x40000,'str')`), layout
//! `{next +0, nocase_rep +4, text +8}` (`setString` 0x00acb7c0). `m_null`
//! is the empty-string entry: `isEmpty`/`isNotEmpty` compare identity
//! against it (0x00a0b7ef / 0x00a0b7fb), and `setString(null | "")` maps
//! to it.
//!
//! Rust model: `Rc<StringEntry>` = entry pointer; `None` = `m_null`.
//! Observable identity (`isEqual` 0x005f4325 pointer compare) and the
//! case-insensitive class representative (`getIdNoCase` 0x00a0b820:
//! `cmovne` ⇒ `nocase_rep ?: entry`) are exact. The bump allocator and
//! bucket chains are reproduced structurally (insertion order, block
//! condition) but memory layout is not modeled. Critical section
//! (0x00e176a8, bypassed when byte 0x00e176c0 ≠ 0) and one-time
//! ctor/atexit hooks (`FUN_004411c1`, `FUN_00bb9c20`) are L0 no-ops in
//! the single-threaded core.

use std::collections::HashMap;
use std::rc::Rc;

pub const HASH_BUCKETS: usize = 0x2b7b; // 11131, verified 0x00acb86e
const BUF_BLOCK: usize = 0x40000; // m_buf block size

/// `StringEntry`: interned record (+0 next lives in the table chain).
pub struct StringEntry {
    /// +4: case-insensitive class representative (first-inserted entry
    /// of the fold-equal class, fixed at insertion).
    pub nocase_rep: Option<Rc<StringEntry>>,
    /// +8: NUL-terminated text.
    pub text: String,
}

/// Interned handle. `None` == `&m_null` (the empty string).
#[derive(Clone)]
pub struct PooledString {
    entry: Option<Rc<StringEntry>>,
}

/// Case-insensitive class id (`getIdNoCase` ⇒ `nocase_rep ?: entry`).
pub struct ClassId(Option<Rc<StringEntry>>);

// Identity (pointer) equality — the game compares entry addresses.
impl PartialEq for ClassId {
    fn eq(&self, o: &Self) -> bool {
        match (&self.0, &o.0) {
            (None, None) => true,
            (Some(a), Some(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }
}
impl Eq for ClassId {}

impl PooledString {
    /// Default ctor 0x00a0b7c4; copy ctor 0x0067ea56 is `Clone`.
    pub fn new() -> Self {
        PooledString { entry: None }
    }

    /// 0x00974b2d — swap: the game copy-constructs a temp (`PooledString
    /// pPVar1 = param_1`), stores `*param_2` into the first slot, then
    /// the temp into the second. Rc-handle exchange; identity-preserving
    /// for pool references.
    pub fn swap_with(&mut self, other: &mut Self) {
        std::mem::swap(&mut self.entry, &mut other.entry);
    }

    /// `isEmpty` 0x00a0b7ef: identity with `m_null`.
    pub fn is_empty(&self) -> bool {
        self.entry.is_none()
    }

    /// `isNotEmpty` 0x00a0b7fb.
    pub fn is_not_empty(&self) -> bool {
        self.entry.is_some()
    }

    /// `getLength` 0x00a0b7e2 (`strlen(entry + 8)`).
    pub fn length(&self) -> usize {
        self.entry.as_ref().map_or(0, |e| e.text.len())
    }

    /// Interned text (`m_null` ⇒ "").
    pub fn text(&self) -> &str {
        self.entry.as_ref().map_or("", |e| e.text.as_str())
    }

    /// `isEqual` 0x005f4325: entry pointer compare.
    pub fn is_equal(&self, other: &PooledString) -> bool {
        match (&self.entry, &other.entry) {
            (None, None) => true,
            (Some(a), Some(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }

    /// `getIdNoCase` 0x00a0b820 (`m_null` ⇒ `m_null` itself ⇒ `None`).
    pub fn id_no_case(&self) -> ClassId {
        match &self.entry {
            None => ClassId(None),
            Some(e) => ClassId(e.nocase_rep.clone().or_else(|| Some(e.clone()))),
        }
    }

    /// `isEqualNoCase` 0x00a0b82b: class representative identity.
    pub fn is_equal_no_case(&self, other: &PooledString) -> bool {
        self.id_no_case() == other.id_no_case()
    }

    /// `compare(const char*, n)` 0x00a0bcab → `FUN_00405816` (memcmp of
    /// the shorter prefix, tie ⇒ length difference) against
    /// `strlen(entry + 8)`. Wrappers: cstr 0x00a0bd47, PooledString
    /// 0x00a0bd6c, AsciiString 0x00a0be2b.
    pub fn compare(&self, other: &[u8]) -> i32 {
        let a = self.text().as_bytes();
        let m = a.len().min(other.len());
        for i in 0..m {
            if a[i] != other[i] {
                return if a[i] < other[i] { -1 } else { 1 };
            }
        }
        a.len() as i32 - other.len() as i32
    }

    /// `compareNoCase(const char*, n)` 0x00a0bce2 → `FUN_00405841`
    /// (`_memicmp`, C-locale ASCII fold). Wrappers 0x00a0bd8e (cstr),
    /// 0x00a0bdb3 (PooledString), 0x00a0be55 (AsciiString).
    pub fn compare_no_case(&self, other: &[u8]) -> i32 {
        let a = self.text().as_bytes();
        let m = a.len().min(other.len());
        for i in 0..m {
            let (fa, fb) = (c_fold(a[i]), c_fold(other[i]));
            if fa != fb {
                return if fa < fb { -1 } else { 1 };
            }
        }
        a.len() as i32 - other.len() as i32
    }
}

impl Default for PooledString {
    fn default() -> Self {
        Self::new()
    }
}

fn c_fold(c: u8) -> u32 {
    if (b'A'..=b'Z').contains(&c) {
        (c + 32) as u32
    } else {
        c as u32
    }
}

/// `PooledString::getHash` 0x00acb780 (no state used):
/// `h = 0; for c in s[..n] { h = tolower(c) ^ (h rol 4) }`.
pub fn get_hash(s: &[u8]) -> u32 {
    let mut h: u32 = 0;
    for &c in s {
        h = c_fold(c) ^ (h >> 28 | h << 4);
    }
    h
}

/// The intern table (`m_hash` + `m_buf` + `m_bufUsed`).
pub struct StringPool {
    buckets: Vec<Vec<Rc<StringEntry>>>,
    buf_used: usize,
    entries: Vec<Rc<StringEntry>>,
    /// fold-key → class representative (the game propagates this via
    /// entry+4 at insertion; chain order makes it the first-inserted).
    nocase: HashMap<Vec<u8>, Rc<StringEntry>>,
}

impl Default for StringPool {
    fn default() -> Self {
        Self::new()
    }
}

impl StringPool {
    pub fn new() -> Self {
        StringPool {
            buckets: vec![Vec::new(); HASH_BUCKETS],
            buf_used: 0,
            entries: Vec::new(),
            nocase: HashMap::new(),
        }
    }

    /// `setString` 0x00acb7c0 and the entry points that funnel into it:
    /// ctor 0x00a0b7cd (char*), ctor 0x00a0be03 (AsciiString&),
    /// `operator=` 0x00a0b80e (char*), 0x00a0be7f (AsciiString&).
    /// Null/empty ⇒ `m_null` (`setString` first branch).
    pub fn set_string<S: AsRef<str>>(&mut self, s: S) -> PooledString {
        let s = s.as_ref();
        if s.is_empty() {
            return PooledString::new();
        }
        let bytes = s.as_bytes();
        let h = get_hash(bytes);
        let bucket = (h % HASH_BUCKETS as u32) as usize;
        // bump-block condition 0x00acb8d3: block switches when
        // m_bufUsed + strlen + 12 exceeds 0x40000; fresh entry costs
        // strlen + 1 + 8.
        if BUF_BLOCK < self.buf_used + bytes.len() + 12 {
            self.buf_used = 0;
        }
        // case-sensitive chain walk (0x00acb890): chain is prepend-ordered.
        for e in self.buckets[bucket].iter() {
            if e.text.as_str() == s {
                return PooledString {
                    entry: Some(e.clone()),
                };
            }
        }
        // fresh entry: nocase rep = class rep if the class exists
        // (`_strcmpi` chain loop 0x00acb8e5, then rep-of-match +4).
        let key: Vec<u8> = bytes.iter().map(|&c| c_fold(c) as u8).collect();
        let rep = self.nocase.get(&key).cloned();
        let entry = Rc::new(StringEntry {
            nocase_rep: rep.clone(),
            text: s.to_string(),
        });
        if rep.is_none() {
            self.nocase.insert(key, entry.clone());
        }
        self.buckets[bucket].insert(0, entry.clone()); // prepend
        self.buf_used += bytes.len() + 9;
        self.entries.push(entry);
        PooledString {
            entry: self.buckets[bucket][0].clone().into(),
        }
    }

    /// `PooledString::clear` 0x00a0b807: assign `m_null`.
    pub fn clear(&self, s: &mut PooledString) {
        s.entry = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_rol4_fold() {
        assert_eq!(get_hash(b""), 0);
        // h = fold('A') ^ 0 = 0x61; then h = 'b' ^ rol4(0x61) =
        // 0x62 ^ 0x610 = 0x672
        assert_eq!(get_hash(b"Ab"), 0x672);
        assert_eq!(get_hash(b"ab"), get_hash(b"AB"), "fold-invariant");
    }

    #[test]
    fn interning_identity() {
        let mut p = StringPool::new();
        let a = p.set_string("Widget");
        let b = p.set_string("Widget");
        assert!(a.is_equal(&b)); // same entry pointer
        let c = p.set_string("widget");
        assert!(!a.is_equal(&c)); // case-sensitive: distinct entries
        let d = p.set_string("Widget");
        assert!(a.is_equal(&d));
        assert_eq!(a.text(), "Widget");
    }

    #[test]
    fn null_entry_semantics() {
        let mut p = StringPool::new();
        let e1 = p.set_string("");
        let e2 = p.set_string("   "); // not empty: interned
        assert!(e1.is_empty() && e1.length() == 0);
        assert!(!e2.is_empty());
        // `clear` ⇒ m_null
        let mut x = p.set_string("hi");
        p.clear(&mut x);
        assert!(x.is_empty() && x.is_equal(&e1));
    }

    #[test]
    fn nocase_class_rep() {
        let mut p = StringPool::new();
        let first = p.set_string("Alpha"); // class rep = itself
        let second = p.set_string("ALPHA"); // rep points at first
        let third = p.set_string("alpha"); // same class
        assert!(first.id_no_case() == second.id_no_case());
        assert!(first.id_no_case() == third.id_no_case());
        assert!(first.is_equal_no_case(&second) && second.is_equal_no_case(&third));
        let other = p.set_string("Beta");
        assert!(!first.is_equal_no_case(&other));
        // rep is the FIRST-inserted entry: second's entry pointer differs
        assert!(!second.is_equal(&first));
    }

    #[test]
    fn compare_helpers() {
        let mut p = StringPool::new();
        let w = p.set_string("Hero");
        assert_eq!(w.compare(b"Hero"), 0);
        assert_eq!(w.compare(b"Her"), 1); // prefix tie ⇒ len diff
        assert_eq!(w.compare(b"HeroX"), -1);
        assert_eq!(w.compare(b"hero"), -1); // 'H'(0x48) < 'h' unsigned memcmp
        assert_eq!(w.compare_no_case(b"HERO"), 0);
        assert_eq!(w.compare_no_case(b"her"), 1);
    }
}
