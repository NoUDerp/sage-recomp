//! Big-endian cursor (de)serializer primitives — Network band.
//!
//! Mirror of the `0x0084780f..0x0084799a` family used by the session
//! settings packer 0x00847cb3 (`memset` + Write-chain over fields
//! +0x44..+0x88) and the peer-list parser 0x0084842d (10 ×
//! `FUN_0084793f` monotonic-range checks). All ops are bounds-checked
//! against an optional `cap` (a one-past-end index; `None` = the game's
//! NULL `cap`, which disables the check and lets blob writes run
//! unbounded, matching the game's `param_4 == 0` bypass). Cursors are
//! byte indices into the caller's buffer; a refused op returns the
//! cursor unchanged — the game's exact semantics (`cur <= cap &&
//! cur + n <= cap` guard at 0x0084780f/0x00847909).

/// 0x0084780f — bounded blob write; cursor advances only when the whole
/// blob fits.
pub fn write_blob(buf: &mut [u8], cur: usize, src: &[u8], cap: Option<usize>) -> usize {
    match cap {
        None => {
            buf[cur..cur + src.len()].copy_from_slice(src);
            cur + src.len()
        }
        Some(c) if cur <= c && cur + src.len() <= c => {
            buf[cur..cur + src.len()].copy_from_slice(src);
            cur + src.len()
        }
        _ => cur,
    }
}

/// 0x00847909 — bounded blob read into `dst`; same guard.
pub fn read_blob(buf: &[u8], cur: usize, dst: &mut [u8], cap: Option<usize>) -> usize {
    match cap {
        None => {
            dst.copy_from_slice(&buf[cur..cur + dst.len()]);
            cur + dst.len()
        }
        Some(c) if cur <= c && cur + dst.len() <= c => {
            dst.copy_from_slice(&buf[cur..cur + dst.len()]);
            cur + dst.len()
        }
        _ => cur,
    }
}

/// 0x0084786b — `htonl` + 4-byte blob write (void: cursor change is
/// invisible to the caller; mirrored as returning it anyway).
pub fn write_u32be(buf: &mut [u8], cur: usize, v: u32, cap: Option<usize>) -> usize {
    let local = v.to_be_bytes(); // htonl into a stack dword == BE bytes
    write_blob(buf, cur, &local, cap)
}

/// 0x00847845 — `htons` + 2-byte blob write.
pub fn write_u16be(buf: &mut [u8], cur: usize, v: u16, cap: Option<usize>) -> usize {
    let local = v.to_be_bytes();
    write_blob(buf, cur, &local, cap)
}

/// 0x00847891 — 1-byte blob write.
pub fn write_u8(buf: &mut [u8], cur: usize, v: u8, cap: Option<usize>) -> usize {
    write_blob(buf, cur, &[v], cap)
}

/// 0x0084793f — 4-byte read + byte-swap into the out value (the game's
/// `htonl` on load == `ntohl`). Out-of-bounds: the game hands back an
/// uninitialized stack dword (UB); the Rust mirror says `None`.
pub fn read_u32be(buf: &[u8], cur: usize, cap: Option<usize>) -> Option<(u32, usize)> {
    let mut local = [0u8; 4];
    let next = read_blob(buf, cur, &mut local, cap);
    if next == cur {
        None
    } else {
        Some((u32::from_be_bytes(local), next))
    }
}

/// 0x0084796c — 2-byte read + `htons` swap.
pub fn read_u16be(buf: &[u8], cur: usize, cap: Option<usize>) -> Option<(u16, usize)> {
    let mut local = [0u8; 2];
    let next = read_blob(buf, cur, &mut local, cap);
    if next == cur {
        None
    } else {
        Some((u16::from_be_bytes(local), next))
    }
}

/// 0x0084799a — 1-byte read.
pub fn read_u8(buf: &[u8], cur: usize, cap: Option<usize>) -> Option<(u8, usize)> {
    let mut local = [0u8; 1];
    let next = read_blob(buf, cur, &mut local, cap);
    if next == cur {
        None
    } else {
        Some((local[0], next))
    }
}

/// 0x008478a9 — bounded string write. `src` is NUL-terminated; `cap`
/// NULL takes the `_mbscpy` unbounded branch. Bounded branch is
/// `strncpy(cur, src, cap-cur)` (no NUL appended when truncated) —
/// then if the string reached or passed `cap`, force `cap[-1] = 0` and
/// park the cursor at `cap`; otherwise cursor = `cur + strlen + 1`
/// (past the copied NUL).
pub fn write_str(buf: &mut [u8], cur: usize, src: &[u8], cap: Option<usize>) -> usize {
    let len = src.iter().position(|b| *b == 0).unwrap_or(src.len());
    match cap {
        None => {
            buf[cur..cur + len + 1].copy_from_slice(&src[..len + 1]);
            cur + len + 1
        }
        Some(c) if c <= cur => cur,
        Some(c) => {
            let n = c - cur;
            let k = len.min(n);
            buf[cur..cur + k].copy_from_slice(&src[..k]);
            if len >= n {
                buf[c - 1] = 0;
                c
            } else {
                cur + len + 1
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_bounds_are_inclusive_of_cap() {
        let mut buf = vec![0u8; 8];
        // fits exactly: cur=4 n=4 cap=8 -> advances to 8
        assert_eq!(write_blob(&mut buf, 4, &[1, 2, 3, 4], Some(8)), 8);
        // one past cap: refused, cursor unchanged
        assert_eq!(write_blob(&mut buf, 8, &[9], Some(8)), 8);
        // cap NULL: unbounded write
        assert_eq!(write_blob(&mut buf, 0, &[7; 8], None), 8);
    }

    #[test]
    fn be_roundtrip_matches_htonl() {
        let mut buf = vec![0u8; 16];
        let c = write_u32be(&mut buf, 0, 0x1122_3344, Some(16));
        assert_eq!(c, 4);
        assert_eq!(&buf[..4], &[0x11, 0x22, 0x33, 0x44]);
        assert_eq!(read_u32be(&buf, 0, Some(16)), Some((0x1122_3344, 4)));
        let c = write_u16be(&mut buf, 4, 0xAABB, Some(16));
        assert_eq!(c, 6);
        assert_eq!(read_u16be(&buf, 4, Some(16)), Some((0xAABB, 6)));
        assert_eq!(write_u8(&mut buf, 6, 0x5A, Some(16)), 7);
        assert_eq!(read_u8(&buf, 6, Some(16)), Some((0x5A, 7)));
        // OOB reads yield None and never fabricate a value
        assert_eq!(read_u32be(&buf, 14, Some(16)), None);
    }

    #[test]
    fn write_str_strncpy_semantics() {
        let mut buf = vec![0u8; 10];
        // fits: cursor lands past the NUL
        let c = write_str(&mut buf, 0, b"hi\0", Some(10));
        assert_eq!(c, 3);
        assert_eq!(&buf[..4], b"hi\0\0");
        // truncation: strncpy copies cap-cur chars with NO NUL,
        // then the force-NUL at cap-1 parks the cursor at cap
        let mut buf = vec![0u8; 10];
        let c = write_str(&mut buf, 0, b"hello\0", Some(4));
        assert_eq!(c, 4);
        assert_eq!(&buf[..4], b"hel\0");
        // empty room: refused
        assert_eq!(write_str(&mut buf, 5, b"x\0", Some(5)), 5);
    }
}
