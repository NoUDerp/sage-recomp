//! GUI-region 2D geometry — GUI band, `0x0070b135..0x0070c934`.
//!
//! `Region2` mirrors the struct the bodies fully reveal: a Vector2
//! vector (`+0/+4/+8` = begin/end/cap) plus six frame floats
//! (`+0x0c..+0x20` = dwords 3..8) and a byte flag at `+0x24`
//! (`*(char *)(param_1 + 9)` in the decompile). translate (0x0070b135)
//! bumps points + frame corners but only fields 3..6; scale
//! (0x0070b1a7) hits all six only while the flag is clear; transform
//! (0x0070b232) is the 2D affine over the 8-float matrix
//! (`x' = m0*x + m1*y + m3`, `y' = m4*x + m5*y + m7` — the `m2*0.0`/
//! `m6*0.0` z terms are literal in the disasm) and *sets* the flag,
//! after which the frame floats stop tracking.
//!
//! Swaps: 0x0070b31a swaps two 4-float LineSegment2D; 0x0070b76f is
//! the owner-object swap (head via 0x00967ecd — dwords [0],[1] swap
//! plus the `+8` PooledString pair delegated to 0x00974b2d, mirrored
//! as `PooledString::swap_with` — then the segment, two u32s and the
//! flag byte). Field getters 0x0070c890..0x0070c934 read the owner's
//! texture fields `+0x78..+0x98` (0x0070c890 truncates a float like
//! the CRT cast; `0x0070c95c` is a refcounting string-setter wrapper
//! — NOT mirrored).

use crate::pooledstring::PooledString;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Region2 {
    pub points: Vec<[f32; 2]>,
    /// frame floats, dwords 3..8
    pub frame: [f32; 6],
    /// +0x24: set once a matrix transform has been applied
    pub transformed: bool,
}

impl Region2 {
    /// 0x0070b135 — translate points and (while untransformed) the
    /// four corner entries of `frame`.
    pub fn translate(&mut self, d: [f32; 2]) {
        for p in self.points.iter_mut() {
            p[0] += d[0];
            p[1] += d[1];
        }
        if !self.transformed {
            self.frame[0] += d[0];
            self.frame[1] += d[1];
            self.frame[2] += d[0];
            self.frame[3] += d[1];
        }
    }

    /// 0x0070b1a7 — scale points and (while untransformed) all six
    /// frame floats.
    pub fn scale(&mut self, s: f32) {
        for p in self.points.iter_mut() {
            p[0] *= s;
            p[1] *= s;
        }
        if !self.transformed {
            for f in self.frame.iter_mut() {
                *f *= s;
            }
        }
    }

    /// 0x0070b232 — apply the affine rows of `m` (8 floats) to every
    /// point; frame floats are NOT touched and the flag is set.
    pub fn transform(&mut self, m: &[f32; 8]) {
        for p in self.points.iter_mut() {
            let [x, y] = *p;
            p[0] = m[0] * x + m[1] * y + m[2] * 0.0 + m[3];
            p[1] = m[4] * x + m[5] * y + m[6] * 0.0 + m[7];
        }
        self.transformed = true;
    }
}

/// 0x0070b31a — LineSegment2D swap (4 floats).
pub fn swap_segment(a: &mut [f32; 4], b: &mut [f32; 4]) {
    std::mem::swap(a, b);
}

/// Owner object of the swap pair 0x0070b76f/0x00967ecd. The `+8`
/// slot is a PooledString (proved by the 0x00974b2d delegation), so
/// the head triple is points-begin/end + string, not a vector cap.
#[derive(Clone, Default)]
pub struct RegionOwner {
    pub points: Vec<[f32; 2]>, // +0x00/+0x04
    pub pooled: PooledString,  // +0x08
    pub segment: [f32; 4],     // +0x0c
    pub f1c: u32,
    pub f20: u32,
    pub flag: u8, // +0x24
}

/// 0x00967ecd — swap the head dwords [0],[1] then delegate the +0x08
/// PooledString pair to 0x00974b2d.
pub fn swap_region_head(a: &mut RegionOwner, b: &mut RegionOwner) {
    std::mem::swap(&mut a.points, &mut b.points);
    a.pooled.swap_with(&mut b.pooled);
}

/// 0x0070b76f — full owner swap: head, segment, two u32s, flag byte.
pub fn swap_owner(a: &mut RegionOwner, b: &mut RegionOwner) {
    swap_region_head(a, b);
    swap_segment(&mut a.segment, &mut b.segment);
    std::mem::swap(&mut a.f1c, &mut b.f1c);
    std::mem::swap(&mut a.f20, &mut b.f20);
    std::mem::swap(&mut a.flag, &mut b.flag);
}

/// Texture-field accessors 0x0070c890..0x0070c934 on the same owner
/// class (fields +0x78..+0x98; only these fields are revealed by the
/// getter bodies, so the model is intentionally partial).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TexFields {
    pub f78: f32,
    pub u7c: u32, // 0x0070c896
    pub u80: u32, // 0x0070c8a7
    pub u88: u32, // 0x0070c8cc
    pub u8c: u32, // 0x0070c8e6
    pub u90: u32, // 0x0070c900
    pub u94: u32, // 0x0070c91a
    pub u98: u32, // 0x0070c934
}

impl TexFields {
    /// 0x0070c890 — `(int)*(float *)(this + 0x78)`: C truncation.
    pub fn i78(&self) -> i32 {
        self.f78 as i32
    }
    pub fn v7c(&self) -> u32 {
        self.u7c
    } // 0x0070c896
    pub fn v80(&self) -> u32 {
        self.u80
    } // 0x0070c8a7
    pub fn v88(&self) -> u32 {
        self.u88
    } // 0x0070c8cc
    pub fn v8c(&self) -> u32 {
        self.u8c
    } // 0x0070c8e6
    pub fn v90(&self) -> u32 {
        self.u90
    } // 0x0070c900
    pub fn v94(&self) -> u32 {
        self.u94
    } // 0x0070c91a
    pub fn v98(&self) -> u32 {
        self.u98
    } // 0x0070c934
}

#[cfg(test)]
mod tests {
    use super::*;

    fn region() -> Region2 {
        Region2 {
            points: vec![[0.0, 0.0], [2.0, 4.0]],
            frame: [1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
            transformed: false,
        }
    }

    #[test]
    fn translate_touches_only_corner_frame() {
        let mut r = region();
        r.translate([1.5, -1.0]);
        assert_eq!(r.points, vec![[1.5, -1.0], [3.5, 3.0]]);
        // fields 3..6 shift; the last two untouched
        assert_eq!(r.frame, [2.5, 1.0, 4.5, 3.0, 5.0, 6.0]);
    }

    #[test]
    fn scale_all_then_flag_freezes_frame() {
        let mut r = region();
        r.scale(2.0);
        assert_eq!(r.points, vec![[0.0, 0.0], [4.0, 8.0]]);
        assert_eq!(r.frame, [2.0, 4.0, 6.0, 8.0, 10.0, 12.0]);
        r.transform(&[1.0, 0.0, 0.0, 10.0, 0.0, 1.0, 0.0, 20.0]);
        assert!(r.transformed);
        assert_eq!(r.points, vec![[10.0, 20.0], [14.0, 28.0]]);
        r.scale(3.0); // frame untouched now
        assert_eq!(r.frame, [2.0, 4.0, 6.0, 8.0, 10.0, 12.0]);
        assert_eq!(r.points[1], [42.0, 84.0]);
    }

    #[test]
    fn transform_is_affine_rows() {
        let mut r = region();
        // x' = 2x + 3y + 7 ; y' = 5x + 6y + 9 (m2/m6 z-terms multiply 0)
        r.transform(&[2.0, 3.0, 99.0, 7.0, 5.0, 6.0, 99.0, 9.0]);
        assert_eq!(
            r.points[1],
            [2.0 * 2.0 + 3.0 * 4.0 + 7.0, 5.0 * 2.0 + 6.0 * 4.0 + 9.0]
        );
    }

    #[test]
    fn owner_swap_includes_pooled_and_segment() {
        let mut pool = crate::pooledstring::StringPool::new();
        let mut a = RegionOwner {
            points: vec![[1.0, 1.0]],
            pooled: pool.set_string("a"),
            segment: [1.0, 0.0, 0.0, 0.0],
            f1c: 1,
            f20: 2,
            flag: 3,
        };
        let mut b = RegionOwner {
            points: vec![[9.0, 9.0], [8.0, 8.0]],
            pooled: pool.set_string("b"),
            segment: [0.0, 2.0, 0.0, 0.0],
            f1c: 7,
            f20: 8,
            flag: 9,
        };
        swap_owner(&mut a, &mut b);
        assert_eq!(a.points, vec![[9.0, 9.0], [8.0, 8.0]]);
        assert_eq!(a.pooled.text(), "b");
        assert_eq!(b.pooled.text(), "a");
        assert_eq!(a.segment, [0.0, 2.0, 0.0, 0.0]);
        assert_eq!((a.f1c, a.f20, a.flag), (7, 8, 9));
        assert_eq!((b.f1c, b.f20, b.flag), (1, 2, 3));
    }

    #[test]
    fn field_getters_truncate_like_c() {
        let t = TexFields {
            f78: -2.7,
            ..Default::default()
        };
        assert_eq!(t.i78(), -2); // C cast truncates toward zero
        let t = TexFields {
            f78: 5.9,
            u88: 0xabcd,
            ..Default::default()
        };
        assert_eq!(t.i78(), 5);
        assert_eq!(t.v88(), 0xabcd);
    }
}
