//! MD5 (RFC 1321) — the GameSpy stats-client hash, Network band.
//!
//! Mirror of the contiguous MD5 stack at `0x00a06220..0x00a06360..
//! 0x00a06d00`, used by the GameSpy DirectGame stats client
//! (`s_gamestats_gamespy_com_00dd51f0`, query-template obfuscation).
//! Ground truth: `MD5Init` stores the RFC IV (`0x67452301/0xefcdab89/
//! 0x98badcfe/0x10325476`); the transform body carries the full RFC K
//! table (`0xd76aa478 … 0xeb86d391`); padding `DAT_00dd5158` =
//! `0x80, 0…` (verified raw at file off `0x9d3d58`); digest→hex uses
//! `"%02x"` (`DAT_00bbe8ec`). Ghidra folds the transform's block-pointer
//! argument in the `MD5Update` body (known folding artifact); the
//! byte-cursor walk is RFC-identical and pinned by the chunking test.

/// Context layout matches the game struct: buf@+0x00, bit-count
/// lo/hi@+0x10/+0x14, input buffer@+0x18.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Md5Ctx {
    pub buf: [u32; 4],
    pub bits_lo: u32,
    pub bits_hi: u32,
    pub in_: [u8; 64],
}

impl Default for Md5Ctx {
    fn default() -> Self {
        Self {
            buf: [0; 4],
            bits_lo: 0,
            bits_hi: 0,
            in_: [0u8; 64],
        }
    }
}

/// `DAT_00dd5158` PADDING[64]: `0x80` then zeros.
pub const MD5_PADDING: [u8; 64] = {
    let mut p = [0u8; 64];
    p[0] = 0x80;
    p
};

/// 0x00a06220 — `MD5Init`: IV + zero bit-count. The game does NOT clear
/// the input buffer here; mirrored as-is.
pub fn md5_init(ctx: &mut Md5Ctx) {
    ctx.buf = [0x6745_2301, 0xefcd_ab89, 0x98ba_dcfe, 0x1032_5476];
    ctx.bits_lo = 0;
    ctx.bits_hi = 0;
}

/// 0x00a06360 — `MD5Transform`: RFC 1321 §3.4 compression over `ctx.in_`
/// (16 little-endian words), updating `ctx.buf`.
pub fn md5_transform(ctx: &mut Md5Ctx) {
    let block = ctx.in_;
    md5_transform_block(ctx, &block);
}

/// Same compression with an explicit block source: the game's update
/// loop feeds full blocks straight from the caller buffer (Ghidra
/// folds that pointer argument), so `ctx.in_` only ever holds the
/// carried tail.
pub fn md5_transform_block(ctx: &mut Md5Ctx, block: &[u8]) {
    let k: [u32; 64] = [
        0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613,
        0xfd469501, 0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193,
        0xa679438e, 0x49b40821, 0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d,
        0x02441453, 0xd8a1e681, 0xe7d3fbc8, 0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed,
        0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a, 0xfffa3942, 0x8771f681, 0x6d9d6122,
        0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70, 0x289b7ec6, 0xeaa127fa,
        0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665, 0xf4292244,
        0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
        0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb,
        0xeb86d391,
    ];
    const S: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5,
        9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10,
        15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];
    let mut m = [0u32; 16];
    for (i, w) in m.iter_mut().enumerate() {
        *w = u32::from_le_bytes(block[i * 4..i * 4 + 4].try_into().unwrap());
    }
    let [mut a, mut b, mut c, mut d] = ctx.buf;
    for i in 0..64 {
        let (f, g) = match i {
            0..=15 => ((b & c) | ((!b) & d), i),
            16..=31 => ((d & b) | ((!d) & c), (5 * i + 1) & 0xf),
            32..=47 => (b ^ c ^ d, (3 * i + 5) & 0xf),
            _ => (c ^ (b | (!d)), (7 * i) & 0xf),
        };
        let f = f.wrapping_add(a).wrapping_add(k[i]).wrapping_add(m[g]);
        a = d;
        d = c;
        c = b;
        b = b.wrapping_add((f << S[i]) | (f >> (32 - S[i])));
    }
    ctx.buf[0] = ctx.buf[0].wrapping_add(a);
    ctx.buf[1] = ctx.buf[1].wrapping_add(b);
    ctx.buf[2] = ctx.buf[2].wrapping_add(c);
    ctx.buf[3] = ctx.buf[3].wrapping_add(d);
}

/// 0x00a06c40 — `MD5Update`: bit-count bookkeeping (64-bit split at
/// +0x10/+0x14), partial-buffer fill, full-block transforms, tail carry.
pub fn md5_update(ctx: &mut Md5Ctx, data: &[u8]) {
    let len = data.len() as u32;
    let bits = len.wrapping_shl(3);
    let count = ctx.bits_lo.wrapping_add(bits);
    let part = (ctx.bits_lo >> 3) & 0x3f;
    ctx.bits_lo = count;
    if count < bits {
        ctx.bits_hi = ctx.bits_hi.wrapping_add(1);
    }
    let mut avail = 0x40u32.wrapping_sub(part);
    ctx.bits_hi = ctx.bits_hi.wrapping_add(len >> 29);

    let (dst_idx, src_idx);
    if len < avail {
        dst_idx = part as usize;
        src_idx = 0usize;
        avail = 0; // game sets the tail-length base to 0 here
    } else {
        ctx.in_[part as usize..].copy_from_slice(&data[..avail as usize]);
        md5_transform(ctx);
        // full blocks straight from `data`, RFC cursor walk
        let mut consumed = avail;
        while consumed + 63 < len {
            md5_transform_block(ctx, &data[consumed as usize..consumed as usize + 64]);
            consumed += 0x40;
        }
        dst_idx = 0;
        src_idx = consumed as usize;
        avail = consumed;
    }
    let rest = (len - avail) as usize;
    ctx.in_[dst_idx..dst_idx + rest].copy_from_slice(&data[src_idx..src_idx + rest]);
}

/// 0x00a06d00 — `MD5Final(digest_out, ctx)`: append `0x80`-padding to
/// the length boundary, then the 8-byte LE bit-count; hand out the LE
/// digest; wipe the first 22 context bytes (RFC codebase's zeroing —
/// `bits_hi` keeps its upper 16 bits, faithful quirk).
pub fn md5_final(digest_out: &mut [u8; 16], ctx: &mut Md5Ctx) {
    let mut count8 = [0u8; 8];
    count8[..4].copy_from_slice(&ctx.bits_lo.to_le_bytes());
    count8[4..].copy_from_slice(&ctx.bits_hi.to_le_bytes());
    let pad = (ctx.bits_lo >> 3) & 0x3f;
    let padlen: u32 = if pad < 0x38 { 0x38 - pad } else { 0x78 - pad };
    md5_update(ctx, &MD5_PADDING[..padlen as usize]);
    md5_update(ctx, &count8);
    for (i, w) in ctx.buf.iter().enumerate() {
        digest_out[i * 4..i * 4 + 4].copy_from_slice(&w.to_le_bytes());
    }
    // 0x16-byte zero loop from ctx base: buf + bits_lo + low half of bits_hi.
    ctx.buf = [0; 4];
    ctx.bits_lo = 0;
    ctx.bits_hi &= 0xFFFF_0000;
}

/// 0x00a06320 — digest → 32 lowercase hex chars (`"%02x"`,
/// `DAT_00bbe8ec`; game writes in-place, NUL at [32]).
pub fn md5_digest_hex(digest: &[u8; 16]) -> [u8; 33] {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = [0u8; 33];
    for (i, b) in digest.iter().enumerate() {
        out[i * 2] = HEX[(b >> 4) as usize];
        out[i * 2 + 1] = HEX[(b & 0xf) as usize];
    }
    out[32] = 0;
    out
}

/// 0x00a06db0 — one-shot: stack ctx, init/update/final, hex format.
pub fn md5_hex(input: &[u8]) -> [u8; 33] {
    let mut ctx = Md5Ctx::default();
    md5_init(&mut ctx);
    md5_update(&mut ctx, input);
    let mut digest = [0u8; 16];
    md5_final(&mut digest, &mut ctx);
    md5_digest_hex(&digest)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex_str(h: &[u8; 33]) -> &str {
        std::str::from_utf8(&h[..32]).unwrap()
    }

    #[test]
    fn rfc_1321_test_vectors() {
        assert_eq!(hex_str(&md5_hex(b"")), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(hex_str(&md5_hex(b"a")), "0cc175b9c0f1b6a831c399e269772661");
        assert_eq!(
            hex_str(&md5_hex(b"abc")),
            "900150983cd24fb0d6963f7d28e17f72"
        );
        assert_eq!(
            hex_str(&md5_hex(b"message digest")),
            "f96b697d7cb7938d525a2f31aaf161d0"
        );
        assert_eq!(
            hex_str(&md5_hex(b"The quick brown fox jumps over the lazy dog")),
            "9e107d9d372bb6826bd81d3542a419d6"
        );
        assert_eq!(
            hex_str(&md5_hex(
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
            )),
            "d174ab98d277d9f5a5611c2c9f419d9f"
        );
    }

    #[test]
    fn chunked_update_matches_one_shot() {
        // exercises part/avail/tail cursor and the bit-count bookkeeping
        let data: Vec<u8> = (0..300u32).map(|i| (i * 7 + 13) as u8).collect();
        let mut ctx = Md5Ctx::default();
        md5_init(&mut ctx);
        let mut i = 0;
        for chunk in [1usize, 7, 64, 63, 65, 3, 57, 100] {
            if i >= data.len() {
                break;
            }
            md5_update(&mut ctx, &data[i..i + chunk.min(data.len() - i)]);
            i += chunk;
        }
        md5_update(&mut ctx, &data[i.min(data.len())..]);
        let mut digest = [0u8; 16];
        md5_final(&mut digest, &mut ctx);
        let mut want = [0u8; 16];
        let mut c2 = Md5Ctx::default();
        md5_init(&mut c2);
        md5_update(&mut c2, &data);
        md5_final(&mut want, &mut c2);
        assert_eq!(digest, want);
        // final-wipe quirk: context zeroed (bits_hi high half kept)
        assert_eq!(ctx.buf, [0; 4]);
        assert_eq!(ctx.bits_lo, 0);
    }

    #[test]
    fn padding_boundary_exact_block() {
        // 56-byte input forces the 120-pad branch (pad >= 0x38)
        let data = vec![0x41u8; 56];
        assert_eq!(hex_str(&md5_hex(&data)), "a2f3e2024931bd470555002aa5ccc010");
    }
}
