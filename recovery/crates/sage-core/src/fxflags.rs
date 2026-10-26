//! Engine-singleton flag-byte setters — 10 rows (0x007bc*/0x007bb*
//! run, FX band). Each body is a single byte store
//! `*(DAT + off) = v` through a runtime-populated singleton
//! pointer slot in `.data` (the slots hold garbage in the image
//! file; the game initialises the pointers before any setter can
//! run).
//!
//! Faithful model: callers resolve the singleton and hand the
//! target byte in as `&mut u8`; the setter itself only performs
//! the store, exactly as the game does. Offsets are quoted from
//! the decompiled bodies.

/// DAT_00dfe78c block.

/// 0x007bb700 — `*(DAT_00dfe78c + 0x98) = v`
pub fn set_dfe78c_98(flag: &mut u8, v: u8) {
    *flag = v;
}

/// 0x007bca68 — `*(DAT_00dfe78c + 0x99) = v`
pub fn set_dfe78c_99(flag: &mut u8, v: u8) {
    *flag = v;
}

/// 0x007bca7b — `*(DAT_00dfe78c + 0x9a) = v`
pub fn set_dfe78c_9a(flag: &mut u8, v: u8) {
    *flag = v;
}

/// 0x007bca8e — `*(DAT_00dfe78c + 0x9b) = v`
pub fn set_dfe78c_9b(flag: &mut u8, v: u8) {
    *flag = v;
}

/// 0x007bc8f0 — `*(DAT_00dfe78c + 0x9f) = v`
pub fn set_dfe78c_9f(flag: &mut u8, v: u8) {
    *flag = v;
}

/// DAT_00dfe77c block.

/// 0x007bcfd4 — `*(DAT_00dfe77c + 0xc0) = v`
pub fn set_dfe77c_c0(flag: &mut u8, v: u8) {
    *flag = v;
}

/// 0x007bd01f — `*(DAT_00dfe77c + 0xc1) = v`
pub fn set_dfe77c_c1(flag: &mut u8, v: u8) {
    *flag = v;
}

/// DAT_00dfe16c block.

/// 0x007bcc6e — `*(DAT_00dfe16c + 0x1a4d6) = v` (deep field of a
/// large singleton block)
pub fn set_dfe16c_1a4d6(flag: &mut u8, v: u8) {
    *flag = v;
}

/// DAT_00dfe758 block.

/// 0x007bcc81 — `*(DAT_00dfe758 + 0xd34) = v`
pub fn set_dfe758_d34(flag: &mut u8, v: u8) {
    *flag = v;
}

/// DAT_00dfec50 block (the vtable-bearing singleton).
/// 0x007bc392 — guarded store: `if (DAT_00dfec50 != 0)
/// *(DAT_00dfec50 + 0x1914) = v`. `None` models the null
/// singleton (store skipped, no crash — the game's guard).
pub fn set_dfec50_1914(flag: Option<&mut u8>, v: u8) {
    if let Some(p) = flag {
        *p = v;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_are_plain_byte_writes() {
        let mut f = 0u8;
        set_dfe78c_98(&mut f, 0x11);
        assert_eq!(f, 0x11);
        set_dfe78c_9f(&mut f, 0xff);
        assert_eq!(f, 0xff);
        set_dfec50_1914(Some(&mut f), 3);
        assert_eq!(f, 3);
        // guarded setter with null singleton: no write, no panic
        set_dfec50_1914(None, 7);
        assert_eq!(f, 3);
    }
}
