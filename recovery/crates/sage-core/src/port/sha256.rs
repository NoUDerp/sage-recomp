//! Port of `Common/Sha256.cpp` from openbfme_core.
//!
//! Upstream sources:
//!   - `engine/src/Common/Sha256.cpp`
//!   - `engine/src/Common/Sha256.h`
//!
//! Safe, deterministic, dependency-free: `core` and `std` only, no `unsafe`,
//! no I/O, no clock, no randomness, no global mutable state. The verifier
//! compiles this file on its own, so it may not reference other modules.

#![allow(unused_variables)]

// FIPS 180-4 SHA-256 and RFC 2104 HMAC-SHA-256, as the upstream
// Common/Sha256 class computes them: big-endian words, 64-byte block,
// 64-bit big-endian bit length in the final padding block.

const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

const BLOCK: usize = 64;

struct Sha256 {
    h: [u32; 8],
    buf: [u8; BLOCK],
    buf_len: usize,
    len_bits: u64,
}

impl Sha256 {
    fn new() -> Self {
        Sha256 {
            h: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
                0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
            ],
            buf: [0u8; BLOCK],
            buf_len: 0,
            len_bits: 0,
        }
    }

    fn compress(&mut self, block: &[u8; BLOCK]) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                block[4 * i],
                block[4 * i + 1],
                block[4 * i + 2],
                block[4 * i + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let mut a = self.h[0];
        let mut b = self.h[1];
        let mut c = self.h[2];
        let mut d = self.h[3];
        let mut e = self.h[4];
        let mut f = self.h[5];
        let mut g = self.h[6];
        let mut hh = self.h[7];
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        self.h[0] = self.h[0].wrapping_add(a);
        self.h[1] = self.h[1].wrapping_add(b);
        self.h[2] = self.h[2].wrapping_add(c);
        self.h[3] = self.h[3].wrapping_add(d);
        self.h[4] = self.h[4].wrapping_add(e);
        self.h[5] = self.h[5].wrapping_add(f);
        self.h[6] = self.h[6].wrapping_add(g);
        self.h[7] = self.h[7].wrapping_add(hh);
    }

    fn update(&mut self, data: &[u8]) {
        self.len_bits = self.len_bits.wrapping_add((data.len() as u64) << 3);
        self.absorb(data);
    }

    // Feed bytes through the block buffer without touching the bit counter
    // (finalize uses this for the padding, which is not part of the length).
    fn absorb(&mut self, mut data: &[u8]) {
        if self.buf_len > 0 {
            let take = core::cmp::min(BLOCK - self.buf_len, data.len());
            self.buf[self.buf_len..self.buf_len + take].copy_from_slice(&data[..take]);
            self.buf_len += take;
            data = &data[take..];
            if self.buf_len == BLOCK {
                let block = self.buf;
                self.compress(&block);
                self.buf_len = 0;
            }
        }
        while data.len() >= BLOCK {
            let mut block = [0u8; BLOCK];
            block.copy_from_slice(&data[..BLOCK]);
            self.compress(&block);
            data = &data[BLOCK..];
        }
        if !data.is_empty() {
            self.buf[..data.len()].copy_from_slice(data);
            self.buf_len = data.len();
        }
    }

    fn finalize(mut self) -> [u8; 32] {
        let len_bits = self.len_bits;
        // 0x80 then zeros until 56 bytes are staged in the block; when the
        // buffer already holds 56..63 bytes, the padding crosses into a
        // fresh block instead.
        let need = if self.buf_len <= 55 {
            56 - self.buf_len
        } else {
            BLOCK + 56 - self.buf_len
        };
        let mut pad = [0u8; BLOCK];
        pad[0] = 0x80;
        self.absorb(&pad[..need]);
        self.absorb(&len_bits.to_be_bytes());
        let mut out = [0u8; 32];
        for i in 0..8 {
            out[4 * i..4 * i + 4].copy_from_slice(&self.h[i].to_be_bytes());
        }
        out
    }
}

fn hex_lower(digest: &[u8; 32]) -> String {
    let mut s = String::with_capacity(64);
    for &byte in digest {
        s.push(char::from(b"0123456789abcdef"[byte as usize >> 4]));
        s.push(char::from(b"0123456789abcdef"[byte as usize & 15]));
    }
    s
}

fn sha256(digest_input: &[u8]) -> [u8; 32] {
    let mut ctx = Sha256::new();
    ctx.update(digest_input);
    ctx.finalize()
}

pub fn sha256_hex(data: &[u8]) -> String {
    hex_lower(&sha256(data))
}

pub fn sha256_stream_hex(first: &[u8], second: &[u8]) -> String {
    let mut ctx = Sha256::new();
    ctx.update(first);
    ctx.update(second);
    hex_lower(&ctx.finalize())
}

pub fn hmac_sha256_hex(key: &[u8], data: &[u8]) -> String {
    // RFC 2104: keys longer than the block are hashed first, then padded
    // with zeros to 64 bytes.
    let mut block_key = [0u8; BLOCK];
    if key.len() > BLOCK {
        let hashed = sha256(key);
        block_key[..32].copy_from_slice(&hashed);
    } else {
        block_key[..key.len()].copy_from_slice(key);
    }
    let mut ipad = [0u8; BLOCK];
    let mut opad = [0u8; BLOCK];
    for i in 0..BLOCK {
        ipad[i] = block_key[i] ^ 0x36;
        opad[i] = block_key[i] ^ 0x5c;
    }
    let mut inner = Sha256::new();
    inner.update(&ipad);
    inner.update(data);
    let inner_digest = inner.finalize();
    let mut outer = Sha256::new();
    outer.update(&opad);
    outer.update(&inner_digest);
    hex_lower(&outer.finalize())
}
