//! SAGE integer/float geometric regions: `ICoord2D`/`ICoord3D`,
//! `IRegion2D`/`IRegion3D`, `Region2D`/`Region3D`.
//!
//! Mirrors 0x00404c28..0x004064fc (plus copy ctors at 0x0047e523,
//! 0x009cc49a, 0x0049ac04, 0x006e7fda, 0x009f178f, helper 0x004057f1).

use crate::math::{Coord2D, Coord3D};

/// 0x00404cad/0x00404cc9/0x00404ce5/0x00404cf8/0x00404d0b/0x00404d24/
/// 0x00404d3d, ctor 0x0047e523.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ICoord2D {
    pub x: i32,
    pub y: i32,
}

impl ICoord2D {
    pub fn new(x: i32, y: i32) -> Self {
        ICoord2D { x, y }
    }
    /// 0x00404ce5 (the shared `operator!=` at 0x00404cc9 is `==` negated;
    /// Rust derives it via `Eq`).
    pub fn add_assign(&mut self, o: &ICoord2D) -> &mut Self {
        self.x = self.x.wrapping_add(o.x);
        self.y = self.y.wrapping_add(o.y);
        self
    }
    pub fn sub_assign(&mut self, o: &ICoord2D) -> &mut Self {
        self.x = self.x.wrapping_sub(o.x);
        self.y = self.y.wrapping_sub(o.y);
        self
    }
    pub fn mul_assign(&mut self, s: i32) -> &mut Self {
        self.x = self.x.wrapping_mul(s);
        self.y = self.y.wrapping_mul(s);
        self
    }
    /// 0x00404d24 — MSVC C division truncates toward zero, like Rust `/`;
    /// `wrapping_div` mirrors the wrap on `MIN / -1`.
    pub fn div_assign(&mut self, s: i32) -> &mut Self {
        self.x = self.x.wrapping_div(s);
        self.y = self.y.wrapping_div(s);
        self
    }
    /// 0x00404d3d
    pub fn move_by(&mut self, d: i32) -> &mut Self {
        self.x = self.x.wrapping_add(d);
        self.y = self.y.wrapping_add(d);
        self
    }
}

/// ctor 0x009cc49a; 0x00404dcf; 0x00404d95.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ICoord3D {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl ICoord3D {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        ICoord3D { x, y, z }
    }
    pub fn zero(&mut self) {
        *self = ICoord3D::default();
    }
    /// 0x00404d95 — sum of squares computed in wrapping int32 (C wraps),
    /// f64 sqrt, then `__ftol` truncation; NaN (negative wrapped sum) →
    /// `INT_MIN` as the x87 invalid-int conversion does.
    pub fn length(&self) -> i32 {
        let s = self
            .x
            .wrapping_mul(self.x)
            .wrapping_add(self.y.wrapping_mul(self.y))
            .wrapping_add(self.z.wrapping_mul(self.z));
        if s < 0 {
            i32::MIN
        } else {
            (s as f64).sqrt() as i32
        }
    }
}

/// 0x004057f1: expand a (min, max) pair by a point component. Note the game
/// never reorders min/max — callers keep the invariant.
fn expand_pair(a: &mut i32, p: i32, c: &mut i32) {
    if p < *a {
        *a = p;
    } else if *c < p {
        *c = p;
    }
}

/// Layout: min at +0/+4, max at +8/+0xc.
/// width 0x00404d4b, height 0x00404d51, `==` 0x00404d58 (via Eq),
/// `!=` 0x00404d84 (via Ne), `=` 0x006e7fda (Copy), expandBy 0x00406469.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IRegion2D {
    pub min_x: i32,
    pub min_y: i32,
    pub max_x: i32,
    pub max_y: i32,
}

impl IRegion2D {
    pub fn width(&self) -> i32 {
        self.max_x.wrapping_sub(self.min_x)
    }
    pub fn height(&self) -> i32 {
        self.max_y.wrapping_sub(self.min_y)
    }
    pub fn expand_by(&mut self, p: &ICoord2D) {
        expand_pair(&mut self.min_x, p.x, &mut self.max_x);
        expand_pair(&mut self.min_y, p.y, &mut self.max_y);
    }
}

/// width 0x00404e93, height 0x00404e99, depth 0x00404ea0, `=` 0x009f178f,
/// expandBy 0x004064fc.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IRegion3D {
    pub min_x: i32,
    pub min_y: i32,
    pub min_z: i32,
    pub max_x: i32,
    pub max_y: i32,
    pub max_z: i32,
}

impl IRegion3D {
    pub fn width(&self) -> i32 {
        self.max_x.wrapping_sub(self.min_x)
    }
    pub fn height(&self) -> i32 {
        self.max_y.wrapping_sub(self.min_y)
    }
    pub fn depth(&self) -> i32 {
        self.max_z.wrapping_sub(self.min_z)
    }
    pub fn expand_by(&mut self, p: &ICoord3D) {
        expand_pair(&mut self.min_x, p.x, &mut self.max_x);
        expand_pair(&mut self.min_y, p.y, &mut self.max_y);
        expand_pair(&mut self.min_z, p.z, &mut self.max_z);
    }
}

/// Float region, min at +0/+4, max at +8/+0xc.
/// width 0x00404c28, height 0x0044259b, `IsExactlyEqualTo` 0x00404c65,
/// `isInside` 0x00404c2e, `expandBy` 0x00406423,
/// `lineSegmentIntersects` 0x004062fd.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Region2D {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}
/// `LineSegment2D` — `{start @+0, end @+8}` two Coord2D.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LineSegment2D {
    pub start: Coord2D,
    pub end: Coord2D,
}

impl LineSegment2D {
    /// 0x00404c06 — field-by-field copy from the two Coord2D refs, verbatim.
    pub fn new(a: &Coord2D, b: &Coord2D) -> Self {
        LineSegment2D { start: *a, end: *b }
    }
}

/// The two ctor rows are FOLDED across `LineSegment2D` AND float `Region2D`:
/// 0x00404c06 `(const Coord2D&, const Coord2D&)`, 0x0044254e copy-ctor
/// (mirrored by `Clone`/`Copy` on both).
impl Region2D {
    /// 0x00404c06 — four dword copies from the two Coord2D refs.
    pub fn new(lo: &Coord2D, hi: &Coord2D) -> Self {
        Region2D {
            min_x: lo.x,
            min_y: lo.y,
            max_x: hi.x,
            max_y: hi.y,
        }
    }
}

/// 0x00ab3100 `ClosestPointOnLineSegment(seg, point)` — free function.
/// Exact x87 flow (helpers `FUN_005040ae` sub / `FUN_0047dec8` add inlined):
/// `dot = (point−start)·(end−start)`; **only `dot < 0` returns `start` — NaN
/// falls through both guards verbatim**; `dot >= len2` (ordered) clamps to
/// `end`; else `start + d·(dot/len2)` (`FUN_0047dec8` adds to `start`).
pub fn closest_point_on_line_segment(seg: &LineSegment2D, point: &Coord2D) -> Coord2D {
    let dx = seg.end.x - seg.start.x;
    let dy = seg.end.y - seg.start.y;
    let dot = (point.x - seg.start.x) * dx + (point.y - seg.start.y) * dy;
    if dot < 0.0 {
        // only strict negatives return here; NaN continues (0x00ab3172)
        return seg.start;
    }
    let len2 = dx * dx + dy * dy;
    if dot >= len2 {
        // ordered `dot >= len2` clamps to end; unordered falls through (0x00ab319f)
        return seg.end;
    }
    let t = dot / len2;
    Coord2D {
        x: seg.start.x + dx * t,
        y: seg.start.y + dy * t,
    }
}

/// x87 slab-method epsilon pair at `0x00bbb9b0`/`0x00bbb9b4`.
const SLAB_EPS: f32 = 1e-4;
const SLAB_NEG_EPS: f32 = -1e-4;

impl Region2D {
    pub fn width(&self) -> f32 {
        self.max_x - self.min_x
    }
    pub fn height(&self) -> f32 {
        self.max_y - self.min_y
    }
    /// 0x00404c65 — bitwise-style equality per the four component `==`s.
    pub fn is_exactly_equal_to(&self, o: &Region2D) -> bool {
        self.min_x == o.min_x
            && self.min_y == o.min_y
            && self.max_x == o.max_x
            && self.max_y == o.max_y
    }
    /// 0x00404c2e — strictly inside; every boundary touch is outside.
    /// NaN inputs compare all-false, yielding `true` — mirrored.
    pub fn is_inside(&self, p: &Coord2D) -> bool {
        if (p.x < self.min_x || p.x == self.min_x)
            || (self.max_x < p.x || self.max_x == p.x)
            || (p.y < self.min_y || p.y == self.min_y)
            || (self.max_y < p.y || self.max_y == p.y)
        {
            false
        } else {
            true
        }
    }
    /// 0x00406423 — verbatim nested-compare form (NaN falls to the else
    /// branches exactly as the game's `fcomip` flags do).
    pub fn expand_by(&mut self, p: &Coord2D) {
        let x = p.x;
        if self.min_x <= x {
            if self.max_x <= x && x != self.max_x {
                self.max_x = x;
            }
        } else {
            self.min_x = x;
        }
        let y = p.y;
        if self.min_y <= y {
            if self.max_y <= y && y != self.max_y {
                self.max_y = y;
            }
        } else {
            self.min_y = y;
        }
    }

    /// 0x004062fd — slab test with reciprocal parametrization; the
    /// axis-parallel cases test the endpoint against the slab instead.
    pub fn line_segment_intersects(&self, p1: &Coord2D, p2: &Coord2D) -> bool {
        // X slab (0x004062fd first block, `dx = p2.x − p1.x`).
        let dx = p2.x - p1.x;
        if SLAB_EPS <= dx || dx <= SLAB_NEG_EPS {
            let r = 1.0 / dx;
            let t0 = (self.min_x - p1.x) * r;
            let t1 = (self.max_x - p1.x) * r;
            let lo = if t1 <= t0 { t1 } else { t0 };
            let hi = if t0 <= t1 { t1 } else { t0 };
            if 1.0 < lo {
                return false;
            }
            if hi <= 0.0 && hi != 0.0 {
                return false;
            }
        } else {
            if p1.x <= self.min_x && self.min_x != p1.x {
                return false;
            }
            if self.max_x <= p1.x && p1.x != self.max_x {
                return false;
            }
        }
        // Y slab (0x0040630d continuation).
        let dy = p2.y - p1.y;
        if SLAB_EPS <= dy || dy <= SLAB_NEG_EPS {
            let r = 1.0 / dy;
            let t0 = (self.min_y - p1.y) * r;
            let t1 = (self.max_y - p1.y) * r;
            let lo = if t1 <= t0 { t1 } else { t0 };
            let hi = if t0 <= t1 { t1 } else { t0 };
            if lo <= 1.0 && (0.0 < hi || hi == 0.0) {
                return true;
            }
        } else if (self.min_y < p1.y || self.min_y == p1.y)
            && (p1.y < self.max_y || p1.y == self.max_y)
        {
            return true;
        }
        false
    }
}

/// min at +0/+4/+8, max at +0xc/+0x10/+0x14.
/// width 0x00404dda, height 0x00404de0, depth 0x00404de7, `zero` 0x00404dee,
/// copy ctor 0x0049ac04 (Copy), `expandBy` 0x00406495,
/// `isInRegionNoZ` 0x00404e0f, `isInRegionWithZ` 0x00404e46.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Region3D {
    pub min: Coord3D,
    pub max: Coord3D,
}

impl Region3D {
    pub fn width(&self) -> f32 {
        self.max.x - self.min.x
    }
    pub fn height(&self) -> f32 {
        self.max.y - self.min.y
    }
    pub fn depth(&self) -> f32 {
        self.max.z - self.min.z
    }
    pub fn zero(&mut self) {
        self.min = Coord3D::default();
        self.max = Coord3D::default();
    }
    pub fn expand_by(&mut self, p: &Coord3D) {
        let x = p.x;
        if self.min.x <= x {
            if self.max.x <= x && x != self.max.x {
                self.max.x = x;
            }
        } else {
            self.min.x = x;
        }
        let y = p.y;
        if self.min.y <= y {
            if self.max.y <= y && y != self.max.y {
                self.max.y = y;
            }
        } else {
            self.min.y = y;
        }
        let z = p.z;
        if self.min.z <= z {
            if self.max.z <= z && z != self.max.z {
                self.max.z = z;
            }
        } else {
            self.min.z = z;
        }
    }
    /// 0x00404e0f — strictly inside in x/y (z ignored); NaN → true.
    pub fn is_in_region_no_z(&self, p: &Coord3D) -> bool {
        if (p.x < self.min.x || p.x == self.min.x)
            || (self.max.x < p.x || self.max.x == p.x)
            || (p.y < self.min.y || p.y == self.min.y)
            || (self.max.y < p.y || self.max.y == p.y)
        {
            false
        } else {
            true
        }
    }
    /// 0x00404e46 — same with z.
    pub fn is_in_region_with_z(&self, p: &Coord3D) -> bool {
        if (p.x < self.min.x || p.x == self.min.x)
            || (self.max.x < p.x || self.max.x == p.x)
            || (p.y < self.min.y || p.y == self.min.y)
            || (self.max.y < p.y || self.max.y == p.y)
            || (p.z < self.min.z || p.z == self.min.z)
            || (self.max.z < p.z || self.max.z == p.z)
        {
            false
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icoord_operators_wrap_and_div_truncate() {
        let mut a = ICoord2D::new(3, -7);
        a.add_assign(&ICoord2D::new(-5, 2));
        assert_eq!((a.x, a.y), (-2, -5));
        a.sub_assign(&ICoord2D::new(-2, -5));
        assert_eq!((a.x, a.y), (0, 0));
        let mut b = ICoord2D::new(7, -7);
        b.div_assign(2);
        assert_eq!((b.x, b.y), (3, -3)); // truncation toward zero
        b.mul_assign(-1);
        assert_eq!((b.x, b.y), (-3, 3));
        b.move_by(4);
        assert_eq!((b.x, b.y), (1, 7));
        assert!(ICoord2D::new(1, 2) != ICoord2D::new(1, 3));
        let c = ICoord3D::new(2, 3, 6);
        assert_eq!(c.length(), (49.0f64).sqrt() as i32);
        // wrapped-negative sum → INT_MIN (x87 invalid conversion)
        let big = ICoord3D::new(50000, 50000, 50000);
        assert_eq!(big.length(), i32::MIN);
    }

    #[test]
    fn iregion_expand_and_dims() {
        let mut r = IRegion2D {
            min_x: 0,
            min_y: 0,
            max_x: 4,
            max_y: 4,
        };
        assert_eq!((r.width(), r.height()), (4, 4));
        r.expand_by(&ICoord2D::new(-3, 9));
        assert_eq!((r.min_x, r.min_y, r.max_x, r.max_y), (-3, 0, 4, 9));
        r.expand_by(&ICoord2D::new(1, 1)); // inside → no change
        assert_eq!((r.min_x, r.max_x), (-3, 4));
        let mut r3 = IRegion3D {
            min_z: -1,
            max_z: 2,
            ..Default::default()
        };
        r3.expand_by(&ICoord3D::new(5, 5, 9));
        assert_eq!((r3.width(), r3.depth()), (5, 10));
    }

    #[test]
    fn region2d_expansion_is_verbatim() {
        let mut r = Region2D {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 1.0,
            max_y: 1.0,
        };
        r.expand_by(&Coord2D::new(2.0, f32::NAN));
        // x: max grows; y: NaN <= comparisons all false → min_y = NaN
        assert_eq!(r.max_x, 2.0);
        assert!(r.min_y.is_nan());
        assert!(r.is_inside(&Coord2D::new(0.5, 0.5)));
        assert!(!r.is_inside(&Coord2D::new(0.0, 0.5))); // boundary = outside
        assert!(!r.is_exactly_equal_to(&Region2D::default()));
    }

    #[test]
    fn line_segment_intersects_slab_cases() {
        let r = Region2D {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 10.0,
            max_y: 10.0,
        };
        let p = |x, y| Coord2D::new(x, y);
        // crossing the slab in both axes
        assert!(r.line_segment_intersects(&p(-5.0, 5.0), &p(15.0, 5.0)));
        // entirely to the left, parallel x (dx ~ 0)
        assert!(!r.line_segment_intersects(&p(-5.0, 0.0), &p(-5.0, 20.0)));
        // above the region, parallel x
        assert!(!r.line_segment_intersects(&p(-5.0, 15.0), &p(15.0, 15.0)));
        // degenerate zero-length segment inside → both slabs parallel branch
        assert!(r.line_segment_intersects(&p(5.0, 5.0), &p(5.0, 5.0)));
    }

    #[test]
    fn region3d_containment_and_expand() {
        let mut r = Region3D {
            min: Coord3D::new(-1.0, -1.0, 0.0),
            max: Coord3D::new(1.0, 1.0, 10.0),
        };
        assert!(r.is_in_region_no_z(&Coord3D::new(0.0, 0.0, 999.0)));
        assert!(!r.is_in_region_with_z(&Coord3D::new(0.0, 0.0, 999.0)));
        r.expand_by(&Coord3D::new(0.0, 2.0, -5.0));
        assert_eq!(r.max.y, 2.0);
        assert_eq!(r.min.z, -5.0);
        r.zero();
        assert_eq!(r.width(), 0.0);
    }

    #[test]
    fn folded_segment_and_region_ctors() {
        let a = Coord2D { x: 1.0, y: 2.0 };
        let b = Coord2D { x: 3.0, y: 4.0 };
        // 0x00404c06 is folded across both types — same four dword copies;
        // 0x0044254e copy-ctor mirrored by Copy.
        assert_eq!(
            LineSegment2D::new(&a, &b),
            LineSegment2D { start: a, end: b }
        );
        assert_eq!(
            Region2D::new(&a, &b),
            Region2D {
                min_x: 1.0,
                min_y: 2.0,
                max_x: 3.0,
                max_y: 4.0
            }
        );
        assert_eq!(Region2D::new(&a, &b), Region2D::new(&a, &b).clone());
    }

    #[test]
    fn closest_point_on_line_segment_quirks() {
        let seg = LineSegment2D::new(&Coord2D { x: 0.0, y: 0.0 }, &Coord2D { x: 10.0, y: 0.0 });
        assert_eq!(
            closest_point_on_line_segment(&seg, &Coord2D { x: 5.0, y: 5.0 }),
            Coord2D { x: 5.0, y: 0.0 }
        );
        // far side clamps to end
        assert_eq!(
            closest_point_on_line_segment(&seg, &Coord2D { x: 12.0, y: 1.0 }),
            Coord2D { x: 10.0, y: 0.0 }
        );
        // strictly behind the segment → start
        assert_eq!(
            closest_point_on_line_segment(&seg, &Coord2D { x: -3.0, y: 2.0 }),
            Coord2D { x: 0.0, y: 0.0 }
        );
        // degenerate segment: dot=0, dot>=len2(=0) → end (same point)
        let pt = LineSegment2D::new(&Coord2D { x: 1.0, y: 2.0 }, &Coord2D { x: 1.0, y: 2.0 });
        assert_eq!(
            closest_point_on_line_segment(&pt, &Coord2D { x: 4.0, y: 6.0 }),
            Coord2D { x: 1.0, y: 2.0 }
        );
    }
}
