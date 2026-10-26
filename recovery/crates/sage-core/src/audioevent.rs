//! AudioEventInfo POD plumbing + voice/zoom config guards — Audio
//! band. Data model only (playback itself is Miles middleware, left
//! as platform).
//!
//! `AudioEventInfo` is 0x34 bytes: dwords 0..7, three explicit bytes
//! at +0x20/+0x21/+0x22 (the assignment 0x005dd0a0 copies them
//! individually and therefore PRESERVES byte +0x23 — faithful quirk),
//! then dwords 9..0xc. Mirror of 0x005dd0a0 (assign), 0x005dd14f
//! (13-dword block copy — full dword, +0x23 included), 0x005dd2dc
//! (guarded single assign), 0x005dd2ee (range copy loop), 0x005dcf82
//! (defaults from constants: `DAT_00bbe358` = 1000.0f (`0x447a0000`),
//! `DAT_00dba4e4` = 5 as raw dword read from .data init — so
//! `*3`/`*7`/`*0x1e` = 15/35/150 ints, 2000, `DAT_00bdc380` = 600.0f
//! (`0x44160000`)). The default setter 0x005de6e1 is NOT mirrored:
//! it touches string members via FUN_0073c432/FUN_00611e58.
//!
//! The 0x005ded5e/0x005ddc2c/0x005de5d2/0x005de402 vector insert/grow
//! machinery is NOT mirrored: the growth path tail-jumps through more
//! unmodelled thunks and the allocator pair. The default-event INI
//! registrars (0x005da98a..) and the Xfer member trampolines
//! (0x0097557f..) are BLOCKED: their 10-byte stub callees
//! (FUN_005da770 → 0xb69751, FUN_009cd3aa/43d/4d0/563 → 0xba1559) are
//! all `/GX` EH prolog shells (`mov eax,<wrapper>; call 0x00a29188`)
//! — see the EH-shell trap, ledger repair task #5.
//!
//! Guards: 0x00441403 NaN-clamps the voice/zoom config triple at
//! offsets 0/+18/+36 floats (stride 0x12 floats = 0x48 B; fields
//! s_MicrophonePreferredFractionCamera_00bc1e50 family); 0x004416ec
//! clamps then squares the same three entries. EAX/ESI register args
//! (the byte offset) pinned from disassembly.

/// 0x34-byte AudioEventInfo element.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AudioEventInfo {
    pub d: [u32; 8], // dwords 0..7
    /// +0x20..+0x23; `assign` copies only the first three bytes
    pub bytes8: [u8; 4],
    pub d9: u32,
    pub d10: u32,
    pub d11: u32,
    pub d12: u32,
}

impl Default for AudioEventInfo {
    fn default() -> Self {
        Self {
            d: [0; 8],
            bytes8: [0; 4],
            d9: 0,
            d10: 0,
            d11: 0,
            d12: 0,
        }
    }
}

/// 0x005dd0a0 — field-wise assign; byte +0x23 is deliberately not
/// copied (the game copies +0x20,+0x21,+0x22 as singles).
pub fn assign(dst: &mut AudioEventInfo, src: &AudioEventInfo) {
    dst.d = src.d;
    dst.bytes8[..3].copy_from_slice(&src.bytes8[..3]);
    dst.d9 = src.d9;
    dst.d10 = src.d10;
    dst.d11 = src.d11;
    dst.d12 = src.d12;
}

/// 0x005dd14f — copy `n = (src_end − src_begin)/0x34` elements as
/// full 13-dword blocks (element-wise `Copy`, unlike `assign`) from
/// `src` into `dst`; returns the destination advance count.
pub fn block_copy(dst: &mut [AudioEventInfo], src: &[AudioEventInfo]) -> usize {
    let n = src.len();
    for i in 0..n {
        dst[i] = src[i];
    }
    n
}

/// 0x005dd2dc — guarded assign thunk (`if (param_1 != 0)`).
pub fn assign_opt(dst: Option<&mut AudioEventInfo>, src: &AudioEventInfo) {
    if let Some(d) = dst {
        assign(d, src);
    }
}

/// 0x005dd2ee — range copy: element-assign from each src element in
/// `src` into consecutive `dst` slots; returns the advance count.
pub fn copy_range(dst: &mut [AudioEventInfo], src: &[AudioEventInfo]) -> usize {
    let n = src.len();
    for i in 0..n {
        assign_opt(Some(&mut dst[i]), &src[i]);
    }
    n
}

/// 0x005dcf82 — the game's alternate default fill: fields 0/5 hold
/// float bits (1000.0 / 600.0), fields 1..4 the ints 15/35/150/2000
/// (DAT_00dba4e4 = 5). Fields 6..12 are untouched (game does not
/// zero them).
pub fn set_defaults(e: &mut AudioEventInfo) {
    e.d[0] = 1000.0f32.to_bits();
    e.d[1] = 15; // DAT_00dba4e4 * 3
    e.d[2] = 35; // DAT_00dba4e4 * 7
    e.d[3] = 150; // DAT_00dba4e4 * 0x1e
    e.d[4] = 2000;
    e.d[5] = 600.0f32.to_bits();
}

/// 0x00441403 — NaN guard on the triple at float stride 18 starting
/// at `o`: first entry NaN → 0.0; the other two fall back to the
/// (already clamped) first entry.
pub fn nan_clamp(buf: &mut [f32], o: usize) {
    if buf[o].is_nan() {
        buf[o] = 0.0;
    }
    if buf[o + 18].is_nan() {
        buf[o + 18] = buf[o];
    }
    if buf[o + 36].is_nan() {
        buf[o + 36] = buf[o];
    }
}

/// 0x004416ec — clamp, then square the same three entries.
pub fn clamp_and_square(buf: &mut [f32], o: usize) {
    nan_clamp(buf, o);
    buf[o] *= buf[o];
    buf[o + 18] *= buf[o + 18];
    buf[o + 36] *= buf[o + 36];
}

#[cfg(test)]
mod tests {
    use super::*;

    fn elem() -> AudioEventInfo {
        AudioEventInfo {
            d: [1, 2, 3, 4, 5, 6, 7, 8],
            bytes8: [0x11, 0x22, 0x33, 0x44],
            d9: 9,
            d10: 10,
            d11: 11,
            d12: 12,
        }
    }

    #[test]
    fn assign_preserves_byte_0x23() {
        let mut dst = AudioEventInfo::default();
        assign(&mut dst, &elem());
        assert_eq!(dst.d[7], 8);
        assert_eq!(&dst.bytes8[..3], &[0x11, 0x22, 0x33]);
        assert_eq!(dst.bytes8[3], 0); // NOT copied (game quirk)
    }

    #[test]
    fn block_copy_copies_full_dwords() {
        let src = [elem()];
        let mut dst = [AudioEventInfo::default()];
        assert_eq!(block_copy(&mut dst, &src), 1);
        assert_eq!(dst[0].bytes8, [0x11, 0x22, 0x33, 0x44]); // +0x23 copied
    }

    #[test]
    fn range_copy_and_defaults() {
        let src = [elem(), elem()];
        let mut dst = [AudioEventInfo::default(); 2];
        assert_eq!(copy_range(&mut dst, &src), 2);
        assert_eq!(dst[1].d12, 12);
        assert_eq!(dst[1].bytes8[3], 0); // via assign semantics

        let mut e = AudioEventInfo::default();
        set_defaults(&mut e);
        assert_eq!(f32::from_bits(e.d[0]), 1000.0);
        assert_eq!((e.d[1], e.d[2], e.d[3], e.d[4]), (15, 35, 150, 2000));
        assert_eq!(f32::from_bits(e.d[5]), 600.0);
        assert_eq!((e.d[6], e.d[7]), (0, 0)); // untouched
    }

    #[test]
    fn nan_guards() {
        let mut buf = vec![0.0f32; 41];
        buf[4] = f32::NAN;
        buf[4 + 18] = f32::NAN;
        buf[4 + 36] = f32::NAN;
        nan_clamp(&mut buf, 4);
        assert_eq!(buf[4], 0.0);
        assert_eq!(buf[4 + 18], 0.0);
        assert_eq!(buf[4 + 36], 0.0);

        buf[4 + 18] = f32::NAN;
        buf[4] = 2.0;
        nan_clamp(&mut buf, 4);
        assert_eq!(buf[4 + 18], 2.0); // falls back to first entry

        let mut buf = vec![0.0f32; 40];
        buf[0] = 3.0;
        buf[18] = 4.0;
        buf[36] = -5.0;
        clamp_and_square(&mut buf, 0);
        assert_eq!((buf[0], buf[18], buf[36]), (9.0, 16.0, 25.0));
    }
}
