//! Port of `Common/NameKeyGenerator.cpp` from openbfme_core.
//!
//! Upstream sources:
//!   - `engine/src/Common/NameKeyGenerator.cpp`
//!   - `engine/src/Common/NameKeyGenerator.h`
//!
//! Safe, deterministic, dependency-free: `core` and `std` only, no `unsafe`,
//! no I/O, no clock, no randomness, no global mutable state. The verifier
//! compiles this file on its own, so it may not reference other modules.
//!
//! Every upstream entry point takes a `const char *`, so a name ends at its
//! first NUL byte: hashing, socket selection and the `strcmp` chain walk all
//! stop there, even though the generator stores the whole `std::string`.

/// `NameKeyGenerator::hash` (RW 0x548538): `h = h * 0x21 + (signed char)c`.
///
/// The multiply is a 32-bit `imul` and each byte is sign-extended by `movsx`
/// before the add, so a byte >= 0x80 subtracts; both operations wrap.
pub fn name_key_hash(name: &str) -> u32 {
    let mut h: u32 = 0;
    for &byte in c_str(name) {
        let signed = byte as i8 as i32 as u32;
        h = h.wrapping_mul(0x21).wrapping_add(signed);
    }
    h
}

/// `NameKeyGenerator::socketOf` (RW 0x548822): `hash(name) % SOCKET_COUNT`.
pub fn name_key_socket(name: &str) -> u32 {
    name_key_hash(name) % SOCKET_COUNT
}

/// Feed `names`, split on `';'`, through one freshly `init()`ed generator.
///
/// `init()` (RW 0x5486DF) starts `m_nextID` at 1, so 0 stays `NAMEKEY_INVALID`.
/// Keys are handed out sequentially on first sight; a repeated name returns the
/// key the chain already holds. Identity is the `strcmp` view of the name — the
/// bytes before the first NUL — so `"a\0b"` and `"a"` share one key.
pub fn name_keys(names: &str) -> Vec<u32> {
    let mut table: Vec<(&[u8], u32)> = Vec::new();
    let mut next_id: u32 = 1;
    let mut keys = Vec::new();
    for part in names.split(';') {
        let name = c_str(part);
        let existing = table.iter().find(|(known, _)| *known == name).map(|(_, id)| *id);
        let key = match existing {
            Some(id) => id,
            None => {
                let id = next_id;
                next_id = next_id.wrapping_add(1);
                table.push((name, id));
                id
            }
        };
        keys.push(key);
    }
    keys
}

/// `SOCKET_COUNT` (RW 0x548822: `mov ecx, 0xAFCF`).
const SOCKET_COUNT: u32 = 45007;

/// The `const char *` view of a name: everything before the first NUL byte.
fn c_str(name: &str) -> &[u8] {
    let bytes = name.as_bytes();
    match bytes.iter().position(|&byte| byte == 0) {
        Some(end) => &bytes[..end],
        None => bytes,
    }
}
