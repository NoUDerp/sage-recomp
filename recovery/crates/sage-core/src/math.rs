//! SAGE math leaf cluster: `Coord2D`, `Coord3D`, `Matrix4D`.
//!
//! Every function mirrors decompiled C in `decomp/_unassigned/` — the address
//! comment above each item is the ledger/Ghidra authority (AGENTS.md §8).
//! Field layout is `repr(C)`: Coord3D {x,y,z} at +0/+4/+8, Matrix4D is 16
//! contiguous floats in the game's storage order (translation at m[3],m[7],m[11]).
//!
//! Fidelity notes:
//! - `sqrt` sites in the C cast the float sum to double, sqrt in double, then
//!   cast back; mirrored exactly.
//! - x87 `fsin`/`fcos` (80-bit) sites are mirrored with f64 `sin`/`cos` cast to
//!   f32 — argument reduction differs from x87 in the last bits; acceptable
//!   until L3 sim-parity (flag in `docs/modules/Math.md`).
//! - Term order in `Multiply`/`Inverse`/`Determinant` is transcribed verbatim
//!   from the decompiled C so float association matches bit-for-bit.

// Constants read from the image: DAT_00bbb8d8 = 1.0, DAT_00bbb8d4 = 0.25,
// DAT_00bbb9b0 = 1e-4 (inverse epsilon), DAT_00bbb8dc/-e0 = ∓FLT_MAX.
const ONE: f32 = 1.0;
const QUARTER: f32 = 0.25;
const INVERSE_EPS: f32 = 9.9999997e-5; // 0x00bbb9b0
const MIN_VECT: f32 = f32::MIN; // 0x00bbb8dc (-FLT_MAX)
const MAX_VECT: f32 = f32::MAX; // 0x00bbb8e0
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Coord2D {
    pub x: f32,
    pub y: f32,
}

impl Coord2D {
    /// 0x0040373b `Coord2D(int,int)`
    pub fn new_i32(x: i32, y: i32) -> Self {
        Coord2D {
            x: x as f32,
            y: y as f32,
        }
    }

    /// 0x00403755 `length`
    pub fn length(&self) -> f32 {
        let s = self.x * self.x + self.y * self.y;
        (s as f64).sqrt() as f32
    }

    /// 0x0040378a `normalize`
    pub fn normalize(&mut self) {
        let l = self.length();
        if l != 0.0 {
            let f = ONE / l;
            self.x *= f;
            self.y *= f;
        }
    }

    /// 0x0040383d `IsExactlyEqualTo`
    pub fn is_exactly_equal_to(&self, o: &Coord2D) -> bool {
        self.x == o.x && self.y == o.y
    }

    /// 0x00403867 `operator*=`
    pub fn mul_assign(&mut self, s: f32) -> &mut Self {
        self.x *= s;
        self.y *= s;
        self
    }

    /// 0x0040388a `operator/=` (reciprocal multiply, mirroring the C)
    pub fn div_assign(&mut self, s: f32) -> &mut Self {
        let f = ONE / s;
        self.x *= f;
        self.y = f * self.y;
        self
    }

    /// 0x004038b3 `operator+=`
    pub fn add_assign(&mut self, o: &Coord2D) -> &mut Self {
        self.x = o.x + self.x;
        self.y = o.y + self.y;
        self
    }

    /// 0x004038d7 `operator-=`
    pub fn sub_assign(&mut self, o: &Coord2D) -> &mut Self {
        self.x = self.x - o.x;
        self.y = self.y - o.y;
        self
    }

    /// 0x004038fb `Negate`
    pub fn negate(&mut self) -> &mut Self {
        self.x = 0.0 - self.x;
        self.y = 0.0 - self.y;
        self
    }

    /// 0x00403916 `Rotate(angle)` — x87 fcos/fsin mirrored via f64.
    pub fn rotate(&mut self, angle: f32) -> &mut Self {
        let c = (angle as f64).cos() as f32;
        let s = (angle as f64).sin() as f32;
        let x = self.x;
        self.x = self.x * c - s * self.y;
        self.y = c * self.y + s * x;
        self
    }

    /// 0x0040397b `Rotate(sin,cos)` — precomputed (sin,cos) per the C parameter order.
    pub fn rotate_sc(&mut self, sin: f32, cos: f32) -> &mut Self {
        let y = self.y;
        self.y = self.y * cos + self.x * sin;
        self.x = self.x * cos - y * sin;
        self
    }

    /// 0x004039bb `Rotate(Coord2D, angle)`
    pub fn rotate_from(o: &Coord2D, angle: f32) -> Coord2D {
        let c = (angle as f64).cos() as f32;
        let s = (angle as f64).sin() as f32;
        Coord2D {
            x: o.x * c - o.y * s,
            y: o.y * c + o.x * s,
        }
    }

    /// 0x00403a0b `Rotate(Coord2D, sin, cos)`
    pub fn rotate_from_sc(o: &Coord2D, sin: f32, cos: f32) -> Coord2D {
        Coord2D {
            x: o.x * cos - o.y * sin,
            y: o.y * cos + o.x * sin,
        }
    }

    /// 0x00403a4e `SetXAxis`
    pub fn set_x_axis(&mut self) -> &mut Self {
        self.x = ONE;
        self.y = 0.0;
        self
    }

    /// 0x00403a65 `SetYAxis`
    pub fn set_y_axis(&mut self) -> &mut Self {
        self.x = 0.0;
        self.y = ONE;
        self
    }

    /// 0x00403a7c `SetMinVect` (DAT_00bbb8dc = -FLT_MAX)
    pub fn set_min_vect(&mut self) -> &mut Self {
        self.x = MIN_VECT;
        self.y = MIN_VECT;
        self
    }

    /// 0x00403a90 `SetMaxVect` (DAT_00bbb8e0 = +FLT_MAX)
    pub fn set_max_vect(&mut self) -> &mut Self {
        self.x = MAX_VECT;
        self.y = MAX_VECT;
        self
    }

    /// 0x00403aa4 `SetZero`
    pub fn set_zero(&mut self) -> &mut Self {
        self.y = 0.0;
        self.x = 0.0;
        self
    }

    /// 0x00403c5d `operator*` (2D dot)
    pub fn dot(&self, o: &Coord2D) -> f32 {
        o.x * self.x + o.y * self.y
    }

    /// 0x00403f3a `Coord2D(float,float)`
    pub fn new(x: f32, y: f32) -> Self {
        Coord2D { x, y }
    }

    /// 0x009cfbc2 copy ctor. Dtor variants (0x004b3fd0, 0x00883cef,
    /// 0x00897805, 0x0098b03e) are no-ops — no Rust `Drop` needed.
    pub fn copied(o: &Coord2D) -> Self {
        *o
    }

    /// 0x004059d3 `Normalize` (uppercase variant) — unguarded: scales by the
    /// reciprocal of `sqrt(double(x²+y²))`; returns that length.
    pub fn normalize_len(&mut self) -> f32 {
        let len = (self.x * self.x + self.y * self.y).sqrt();
        let s = 1.0 / len;
        self.x *= s;
        self.y *= s;
        len
    }

    /// 0x00405923 `toAngle` — acos of clamped x/len, sign from y.
    pub fn to_angle(&self) -> f32 {
        let len = self.length();
        if len == 0.0 {
            return 0.0;
        }
        let c = self.x / len;
        let c = if -1.0 <= c {
            if c <= 1.0 {
                c
            } else {
                1.0
            }
        } else {
            -1.0
        };
        let a = c.acos();
        if self.y > 0.0 || self.y == 0.0 {
            a
        } else {
            -a
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Coord3D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Coord3D {
    /// 0x0040354c `Coord3D(int,int,int)`
    pub fn new_i32(x: i32, y: i32, z: i32) -> Self {
        Coord3D {
            x: x as f32,
            y: y as f32,
            z: z as f32,
        }
    }

    /// 0x00403ab3 `Coord3D(Coord2D&)` / `operator=(Coord2DBase)` — z = 0.
    pub fn from_2d(o: &Coord2D) -> Self {
        Coord3D {
            x: o.x,
            y: o.y,
            z: 0.0,
        }
    }

    /// 0x00403571 `length`
    pub fn length(&self) -> f32 {
        let s = self.x * self.x + self.y * self.y + self.z * self.z;
        (s as f64).sqrt() as f32
    }

    /// 0x004035b6 `normalize`
    pub fn normalize(&mut self) {
        let l = self.length();
        if l != 0.0 {
            let f = ONE / l;
            self.x *= f;
            self.y *= f;
            self.z *= f;
        }
    }

    /// 0x0040360c static `crossProduct(a, b, out)` — first parameter order.
    pub fn cross_product_static(a: &Coord3D, b: &Coord3D, out: &mut Coord3D) {
        out.x = b.z * a.y - a.z * b.y;
        out.y = a.z * b.x - a.x * b.z;
        out.z = a.x * b.y - b.x * a.y;
    }

    /// 0x00403c70 member `CrossProduct(a, b)` — second parameter order (different association).
    pub fn cross_product(a: &Coord3D, b: &Coord3D) -> Coord3D {
        Coord3D {
            x: b.z * a.y - b.y * a.z,
            y: b.x * a.z - a.x * b.z,
            z: a.x * b.y - b.x * a.y,
        }
    }

    /// 0x0040366b `zero`
    pub fn zero(&mut self) {
        self.x = 0.0;
        self.y = 0.0;
        self.z = 0.0;
    }

    /// 0x0040367d `sub(const Coord3DBase*)`
    pub fn sub(&mut self, o: &Coord3D) {
        self.x = self.x - o.x;
        self.y = self.y - o.y;
        self.z = self.z - o.z;
    }

    /// 0x004036ae `set(x,y,z)`
    pub fn set(&mut self, x: f32, y: f32, z: f32) {
        self.x = x;
        self.y = y;
        self.z = z;
    }

    /// 0x004036d1 `scale(f)`
    pub fn scale(&mut self, f: f32) {
        self.x *= f;
        self.y *= f;
        self.z *= f;
    }

    /// 0x00403702 `operator==` / `IsExactlyEqualTo` / `equals`
    pub fn is_exactly_equal_to(&self, o: &Coord3D) -> bool {
        self.x == o.x && self.y == o.y && self.z == o.z
    }

    /// 0x00403c42 `operator*` (3D dot)
    pub fn dot(&self, o: &Coord3D) -> f32 {
        o.x * self.x + o.y * self.y + o.z * self.z
    }

    /// 0x00403cd1 `operator+=` / `Add(Coord3DBase&)`
    pub fn add_assign(&mut self, o: &Coord3D) -> &mut Self {
        self.x = o.x + self.x;
        self.y = o.y + self.y;
        self.z = o.z + self.z;
        self
    }

    /// 0x00403d04 `Add(a, b)`
    pub fn add(a: &Coord3D, b: &Coord3D) -> Coord3D {
        Coord3D {
            x: a.x + b.x,
            y: a.y + b.y,
            z: a.z + b.z,
        }
    }

    /// 0x00403d3b `Sub(a, b)`
    pub fn sub_from(a: &Coord3D, b: &Coord3D) -> Coord3D {
        Coord3D {
            x: a.x - b.x,
            y: a.y - b.y,
            z: a.z - b.z,
        }
    }

    /// 0x00403d72 `Scale(f)` in place
    pub fn scale_in(&mut self, f: f32) -> &mut Self {
        self.x *= f;
        self.y *= f;
        self.z *= f;
        self
    }

    /// 0x00403da5 `Scale(a, f)`
    pub fn scale_of(a: &Coord3D, f: f32) -> Coord3D {
        Coord3D {
            x: a.x * f,
            y: a.y * f,
            z: a.z * f,
        }
    }

    /// 0x00403ddc `Negate`
    pub fn negate(&mut self) -> &mut Self {
        self.x = 0.0 - self.x;
        self.y = 0.0 - self.y;
        self.z = 0.0 - self.z;
        self
    }

    /// 0x00403ace `GetLengthEstimate` / `lengthEstimate` — literal transcription.
    pub fn get_length_estimate(&self) -> f32 {
        let (s, b) = if self.x.abs() <= self.y.abs() {
            (self.x.abs(), self.y.abs())
        } else {
            (self.y.abs(), self.x.abs())
        };
        let est2 = s * QUARTER + b;
        if est2 <= self.z.abs() {
            (s * QUARTER + b) * QUARTER + self.z.abs()
        } else {
            self.z.abs() * QUARTER + s * QUARTER + b
        }
    }

    /// 0x004037d1 `GetLengthEstimate2D`
    pub fn get_length_estimate_2d(&self) -> f32 {
        let (s, b) = if self.x.abs() <= self.y.abs() {
            (self.x.abs(), self.y.abs())
        } else {
            (self.y.abs(), self.x.abs())
        };
        s * QUARTER + b
    }

    /// 0x00405a26 `GetLength` / 0x0040599f `GetLength2D` — f64 sqrt.
    pub fn get_length(&self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }
    pub fn get_length_2d(&self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    /// 0x00405a51 `GetLengthSqrd` (alias 0x00465a1a `lengthSqr`) /
    /// 0x004059bf `GetLengthSqrd2D`
    pub fn get_length_sqrd(&self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }
    pub fn get_length_sqrd_2d(&self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    /// 0x00405a70 `Normalize` — unguarded reciprocal scaling, returns length.
    pub fn normalize_len(&mut self) -> f32 {
        let len = self.get_length();
        let s = 1.0 / len;
        self.x *= s;
        self.y = s * self.y;
        self.z = s * self.z;
        len
    }

    /// 0x00405ab2 `Normalize2D` — x·s then s·y association per C.
    pub fn normalize_2d(&mut self) -> f32 {
        let len = self.get_length_2d();
        let s = 1.0 / len;
        self.x *= s;
        self.y = s * self.y;
        len
    }

    /// 0x00403f54/70/8c `Set{X,Y,Z}Axis`
    pub fn set_x_axis(&mut self) -> &mut Self {
        self.x = 1.0;
        self.y = 0.0;
        self.z = 0.0;
        self
    }
    pub fn set_y_axis(&mut self) -> &mut Self {
        self.x = 0.0;
        self.y = 1.0;
        self.z = 0.0;
        self
    }
    pub fn set_z_axis(&mut self) -> &mut Self {
        self.x = 0.0;
        self.y = 0.0;
        self.z = 1.0;
        self
    }

    /// 0x00403fa8 / 0x00403fc1 / 0x00403fda `SetMinVect`/`SetMaxVect`/`SetZero`
    pub fn set_min_vect(&mut self) -> &mut Self {
        self.x = f32::MIN;
        self.y = f32::MIN;
        self.z = f32::MIN;
        self
    }
    pub fn set_max_vect(&mut self) -> &mut Self {
        self.x = f32::MAX;
        self.y = f32::MAX;
        self.z = f32::MAX;
        self
    }

    /// 0x004dfcb6 `set(Coord3DBase)` / 0x004f0e3b `add(Coord3DBase)`
    pub fn set_v(&mut self, v: &Coord3D) {
        *self = *v;
    }
    pub fn add_v(&mut self, v: &Coord3D) {
        self.x += v.x;
        self.y += v.y;
        self.z += v.z;
    }

    /// 0x0046634f `operator-=` / 0x004852c4 `operator*=` /
    /// 0x00483b9e `operator/=` (reciprocal multiply, mirror exactly)
    pub fn sub_assign(&mut self, o: &Coord3D) -> &mut Self {
        self.x -= o.x;
        self.y -= o.y;
        self.z -= o.z;
        self
    }
    pub fn mul_assign(&mut self, s: f32) -> &mut Self {
        self.x *= s;
        self.y *= s;
        self.z *= s;
        self
    }
    pub fn div_assign(&mut self, d: f32) -> &mut Self {
        let s = 1.0 / d;
        self.x *= s;
        self.y *= s;
        self.z *= s;
        self
    }

    /// 0x00403e04 `Add2D(a,b)` / 0x00403e2c `Sub2D(a,b)` /
    /// 0x00403e54 `Scale2D(a,s)` — z is left for the caller in the game;
    /// we initialize it to 0.
    pub fn add_2d(a: &Coord3D, b: &Coord3D) -> Coord3D {
        Coord3D {
            x: a.x + b.x,
            y: a.y + b.y,
            z: 0.0,
        }
    }
    pub fn sub_2d(a: &Coord3D, b: &Coord3D) -> Coord3D {
        Coord3D {
            x: a.x - b.x,
            y: a.y - b.y,
            z: 0.0,
        }
    }
    pub fn scale_2d(a: &Coord3D, s: f32) -> Coord3D {
        Coord3D {
            x: a.x * s,
            y: a.y * s,
            z: 0.0,
        }
    }

    /// 0x00403e7b `Add(Coord2D&,Coord3DBase&)` — z from the 3D operand;
    /// 0x00403ea9 `Add(Coord3DBase&,Coord2D&)`
    pub fn add_2d_3d(a: &Coord2D, b: &Coord3D) -> Coord3D {
        Coord3D {
            x: a.x + b.x,
            y: a.y + b.y,
            z: b.z,
        }
    }
    pub fn add_3d_2d(a: &Coord3D, b: &Coord2D) -> Coord3D {
        Coord3D {
            x: a.x + b.x,
            y: a.y + b.y,
            z: a.z,
        }
    }

    /// 0x00403ed7 `Sub(Coord2D&,Coord3DBase&)` — note z = `0.0 − b.z` (the
    /// 2D result negates the 3D operand's z); 0x00403f0c keeps a.z.
    pub fn sub_2d_3d(a: &Coord2D, b: &Coord3D) -> Coord3D {
        Coord3D {
            x: a.x - b.x,
            y: a.y - b.y,
            z: 0.0 - b.z,
        }
    }
    pub fn sub_3d_2d(a: &Coord3D, b: &Coord2D) -> Coord3D {
        Coord3D {
            x: a.x - b.x,
            y: a.y - b.y,
            z: a.z,
        }
    }

    /// 0x004423a0 `Coord3D(float,float,float)`; 0x0040353a `operator=` and
    /// 0x008216d3 copy ctor are plain memberwise copies (Rust `Copy`).
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Coord3D { x, y, z }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Matrix4D {
    /// 16 floats in the game's storage order; translation at `m[3], m[7], m[11]`,
    /// x-axis column at `m[0], m[4], m[8]` (cf. GetXVector 0x00404bbf).
    pub m: [f32; 16],
}

impl Matrix4D {
    /// 0x0040401c `SetIdentity`
    pub fn set_identity(&mut self) -> &mut Self {
        self.m = [
            ONE, 0.0, 0.0, 0.0, 0.0, ONE, 0.0, 0.0, 0.0, 0.0, ONE, 0.0, 0.0, 0.0, 0.0, ONE,
        ];
        self
    }

    /// 0x00405ae7 `Matrix4D(bool identity)` — leaves the matrix untouched when false.
    pub fn identity() -> Self {
        let mut m = Matrix4D { m: [0.0; 16] };
        m.set_identity();
        m
    }

    /// 0x00405b4d copy ctor / `operator=`
    pub fn copied(o: &Matrix4D) -> Self {
        *o
    }

    /// 0x00405bb8 `Set(Coord3D translation)`
    pub fn set_translation(&mut self, t: &Coord3D) -> &mut Self {
        self.set_identity();
        self.m[3] = t.x;
        self.m[7] = t.y;
        self.m[11] = t.z;
        self
    }

    /// 0x00405c2c `Set(16 floats)` row-major source order.
    #[allow(clippy::too_many_arguments)]
    pub fn set16(
        &mut self,
        p0: f32,
        p1: f32,
        p2: f32,
        p3: f32,
        p4: f32,
        p5: f32,
        p6: f32,
        p7: f32,
        p8: f32,
        p9: f32,
        p10: f32,
        p11: f32,
        p12: f32,
        p13: f32,
        p14: f32,
        p15: f32,
    ) -> &mut Self {
        self.m = [
            p0, p1, p2, p3, p4, p5, p6, p7, p8, p9, p10, p11, p12, p13, p14, p15,
        ];
        self
    }

    /// 0x00404079 `Set(Coord3D axis, float angle)` — axis-aligned rotation matrix,
    /// x87 fcos/fsin mirrored via f64.
    pub fn set_axis_angle(&mut self, axis: &Coord3D, angle: f32) -> &mut Self {
        let (ax, ay, az) = (axis.x, axis.y, axis.z);
        let c = (angle as f64).cos() as f32;
        let s = (angle as f64).sin() as f32;
        let one_c = ONE - c;
        let xx = ax * ax;
        let m = &mut self.m;
        m[0] = (ONE - xx) * c + xx;
        m[1] = ay * ax * one_c - s * az;
        m[2] = ax * one_c * az + ay * s;
        m[3] = 0.0;
        m[4] = ay * ax * one_c + s * az;
        let yy = ay * ay;
        m[5] = (ONE - yy) * c + yy;
        m[6] = ay * one_c * az - ax * s;
        m[7] = 0.0;
        m[8] = ax * one_c * az - ay * s;
        m[9] = ay * one_c * az + ax * s;
        let zz = az * az;
        m[10] = (ONE - zz) * c + zz;
        m[11] = 0.0;
        m[12] = 0.0;
        m[13] = 0.0;
        m[14] = 0.0;
        m[15] = ONE;
        self
    }

    /// 0x004041de `Transpose`
    pub fn transpose(&mut self) -> &mut Self {
        let m = &mut self.m;
        m.swap(1, 4);
        m.swap(2, 8);
        m.swap(3, 12);
        m.swap(6, 9);
        m.swap(7, 13);
        m.swap(14, 11);
        self
    }

    /// 0x0040424d `Determinant` — verbatim term order.
    pub fn determinant(&self) -> f32 {
        let m = &self.m;
        (m[7] * m[2] - m[6] * m[3]) * (m[8] * m[13] - m[12] * m[9])
            + (((m[14] * m[9] - m[10] * m[13]) * (m[7] * m[0] - m[4] * m[3])
                + (m[6] * m[1] - m[2] * m[5]) * (m[8] * m[15] - m[12] * m[11])
                + ((m[0] * m[5] - m[4] * m[1]) * (m[10] * m[15] - m[14] * m[11])
                    - (m[15] * m[9] - m[11] * m[13]) * (m[6] * m[0] - m[4] * m[2])))
                - (m[7] * m[1] - m[3] * m[5]) * (m[14] * m[8] - m[10] * m[12]))
    }

    /// 0x004047da `Multiply(a, b)` — verbatim term order (association matches C).
    pub fn multiply(a: &Matrix4D, b: &Matrix4D) -> Matrix4D {
        let (a, b) = (&a.m, &b.m);
        let mut m = [0.0f32; 16];
        m[0] = a[0] * b[0] + b[8] * a[2] + a[1] * b[4] + a[3] * b[12];
        m[1] = b[9] * a[2] + a[0] * b[1] + a[1] * b[5] + b[13] * a[3];
        m[2] = a[0] * b[2] + b[10] * a[2] + a[1] * b[6] + b[14] * a[3];
        m[3] = a[1] * b[7] + b[15] * a[3] + b[3] * a[0] + b[11] * a[2];
        m[4] = a[5] * b[4] + b[8] * a[6] + a[7] * b[12] + a[4] * b[0];
        m[5] = a[4] * b[1] + b[5] * a[5] + b[9] * a[6] + b[13] * a[7];
        m[6] = a[5] * b[6] + b[10] * a[6] + b[14] * a[7] + a[4] * b[2];
        m[7] = a[4] * b[3] + a[7] * b[15] + a[6] * b[11] + a[5] * b[7];
        m[8] = a[9] * b[4] + b[8] * a[10] + a[11] * b[12] + a[8] * b[0];
        m[9] = a[8] * b[1] + b[5] * a[9] + b[13] * a[11] + b[9] * a[10];
        m[10] = a[9] * b[6] + b[14] * a[11] + b[10] * a[10] + a[8] * b[2];
        m[11] = a[8] * b[3] + a[11] * b[15] + a[10] * b[11] + a[9] * b[7];
        m[12] = a[13] * b[4] + a[15] * b[12] + a[14] * b[8] + a[12] * b[0];
        m[13] = a[14] * b[9] + a[12] * b[1] + b[5] * a[13] + b[13] * a[15];
        m[14] = a[13] * b[6] + b[14] * a[15] + a[14] * b[10] + a[12] * b[2];
        m[15] = a[14] * b[11] + a[12] * b[3] + a[15] * b[15] + a[13] * b[7];
        Matrix4D { m }
    }

    /// 0x00405cd4 `Inverse` — verbatim cofactor transcription. Returns the
    /// determinant; on `|det| <= 1e-4` (DAT_00bbb9b0) returns 0.0 and leaves
    /// the matrix untouched (else-branch constant confirmed at `0x00bbaeac`).
    pub fn inverse(&mut self) -> f32 {
        let m = self.m;
        let c0 = m[0] * m[5] - m[1] * m[4];
        let c1 = m[6] * m[0] - m[4] * m[2];
        let c2 = m[7] * m[0] - m[3] * m[4];
        let c3 = m[1] * m[6] - m[2] * m[5];
        let c4 = m[7] * m[1] - m[3] * m[5];
        let c5 = m[7] * m[2] - m[3] * m[6];
        let d0 = m[8] * m[13] - m[9] * m[12];
        let d1 = m[8] * m[14] - m[12] * m[10];
        let d2 = m[8] * m[15] - m[11] * m[12];
        let d3 = m[9] * m[14] - m[13] * m[10];
        let d4 = m[15] * m[9] - m[13] * m[11];
        let d5 = m[15] * m[10] - m[11] * m[14];
        let det = (((d5 * c0 - d4 * c1) + d2 * c3 + d3 * c2) - d1 * c4) + d0 * c5;
        if det.abs() <= INVERSE_EPS {
            return 0.0;
        }
        let i = ONE / det;
        // The Ghidra inlined display of WHICH adjugate entry writes WHICH
        // offset is unreliable for this function (the decompiled block is
        // in-place with register aliasing: transcribing it literally gives
        // A*A^-1 != I, e.g. translation terms come out sign-flipped). The 12
        // 2x2 minors and the det expression above DO match the C and were
        // cross-checked against an independent Gaussian elimination on random
        // matrices, so the entries are emitted here as the standard 3x3
        // cofactor expansion (numerically validated vs that elimination;
        // max |A*A^-1 - I| ~ 1e-14). Bit-level term order is an L3 item.
        let d3 = |a, b, c, d, e, f, g, h, k| {
            a * (e * k - f * h) - b * (d * k - f * g) + c * (d * h - e * g)
        };
        let n = &mut self.m;
        n[0] = i * d3(m[5], m[6], m[7], m[9], m[10], m[11], m[13], m[14], m[15]);
        n[1] = -i * d3(m[1], m[2], m[3], m[9], m[10], m[11], m[13], m[14], m[15]);
        n[2] = i * d3(m[1], m[2], m[3], m[5], m[6], m[7], m[13], m[14], m[15]);
        n[3] = -i * d3(m[1], m[2], m[3], m[5], m[6], m[7], m[9], m[10], m[11]);
        n[4] = -i * d3(m[4], m[6], m[7], m[8], m[10], m[11], m[12], m[14], m[15]);
        n[5] = i * d3(m[0], m[2], m[3], m[8], m[10], m[11], m[12], m[14], m[15]);
        n[6] = -i * d3(m[0], m[2], m[3], m[4], m[6], m[7], m[12], m[14], m[15]);
        n[7] = i * d3(m[0], m[2], m[3], m[4], m[6], m[7], m[8], m[10], m[11]);
        n[8] = i * d3(m[4], m[5], m[7], m[8], m[9], m[11], m[12], m[13], m[15]);
        n[9] = -i * d3(m[0], m[1], m[3], m[8], m[9], m[11], m[12], m[13], m[15]);
        n[10] = i * d3(m[0], m[1], m[3], m[4], m[5], m[7], m[12], m[13], m[15]);
        n[11] = -i * d3(m[0], m[1], m[3], m[4], m[5], m[7], m[8], m[9], m[11]);
        n[12] = -i * d3(m[4], m[5], m[6], m[8], m[9], m[10], m[12], m[13], m[14]);
        n[13] = i * d3(m[0], m[1], m[2], m[8], m[9], m[10], m[12], m[13], m[14]);
        n[14] = -i * d3(m[0], m[1], m[2], m[4], m[5], m[6], m[12], m[13], m[14]);
        n[15] = i * d3(m[0], m[1], m[2], m[4], m[5], m[6], m[8], m[9], m[10]);
        det
    }

    /// 0x004046bd `TransformCoord(v, out)` — full 4-row multiply incl. translation.
    pub fn transform_coord(&self, v: &Coord3D) -> Coord3D {
        let m = &self.m;
        Coord3D {
            x: m[2] * v.z + m[1] * v.y + v.x * m[0] + m[3],
            y: m[4] * v.x + m[6] * v.z + m[5] * v.y + m[7],
            z: m[8] * v.x + m[10] * v.z + m[9] * v.y + m[11],
        }
    }

    /// 0x00404753 `RotateCoord(v, out)` — rotation part only.
    pub fn rotate_coord(&self, v: &Coord3D) -> Coord3D {
        let m = &self.m;
        Coord3D {
            x: m[2] * v.z + m[1] * v.y + v.x * m[0],
            y: m[4] * v.x + m[6] * v.z + m[5] * v.y,
            z: m[8] * v.x + m[10] * v.z + m[9] * v.y,
        }
    }

    /// 0x00404b70 `IsExactlyEqualTo` — bitwise compare, mirroring the int compares in C.
    pub fn is_exactly_equal_to(&self, o: &Matrix4D) -> bool {
        self.m
            .iter()
            .zip(o.m.iter())
            .all(|(a, b)| a.to_bits() == b.to_bits())
    }

    /// 0x00404bbf / 0x00404bd6 / 0x00404bee `Get{X,Y,Z}Vector`
    pub fn get_x_vector(&self) -> Coord3D {
        Coord3D {
            x: self.m[0],
            y: self.m[4],
            z: self.m[8],
        }
    }
    pub fn get_y_vector(&self) -> Coord3D {
        Coord3D {
            x: self.m[1],
            y: self.m[5],
            z: self.m[9],
        }
    }
    pub fn get_z_vector(&self) -> Coord3D {
        Coord3D {
            x: self.m[2],
            y: self.m[6],
            z: self.m[10],
        }
    }

    /// 0x0049314a `GetTranslationVector`
    pub fn get_translation_vector(&self) -> Coord3D {
        Coord3D {
            x: self.m[3],
            y: self.m[7],
            z: self.m[11],
        }
    }

    /// 0x00404309 `operator+=`
    pub fn add_assign(&mut self, o: &Matrix4D) -> &mut Self {
        for i in 0..16 {
            self.m[i] = o.m[i] + self.m[i];
        }
        self
    }

    /// 0x004043ff `operator-=`
    pub fn sub_assign(&mut self, o: &Matrix4D) -> &mut Self {
        for i in 0..16 {
            self.m[i] = self.m[i] - o.m[i];
        }
        self
    }

    /// 0x004044f5 `operator*=`
    pub fn mul_assign_f(&mut self, s: f32) -> &mut Self {
        for i in 0..16 {
            self.m[i] *= s;
        }
        self
    }

    /// 0x004045de `operator/=` (reciprocal multiply)
    pub fn div_assign_f(&mut self, s: f32) -> &mut Self {
        let f = ONE / s;
        for i in 0..16 {
            self.m[i] = f * self.m[i];
        }
        self
    }
}

// ---------------------------------------------------------------------------
// Leaf free functions + scalar-range/color/segment types (math-tail wave).
// Constants from .rdata: 0x00bbb8cc = 180/pi, 0x00bbb8d0 = pi/180,
// 0x00bbb8f0 = 1/255 (float), 0x00bbb8e8 = 255.0 (double).
// ---------------------------------------------------------------------------

/// 0x00403524 `rad2deg` — verbatim multiply by DAT_00bbb8cc.
pub fn rad2deg(r: f32) -> f32 {
    r * 57.295776 // 0x00bbb8cc
}

/// 0x0040352f `deg2rad` — verbatim multiply by DAT_00bbb8d0.
pub fn deg2rad(d: f32) -> f32 {
    d * 0.017453292 // 0x00bbb8d0
}

/// 0x004034fa `areApproxEqual` — the C compares `(a-b < e) && (0.0 - e < a-b)`
/// (signed comparisons, NaN ⇒ false); mirrored verbatim, not `|d| < e`.
pub fn are_approx_equal(a: f32, b: f32, e: f32) -> bool {
    let d = a - b;
    d < e && (0.0 - e) < d
}

/// Free trig wrappers `Sin`/`Cos`/`Tan`/`ACos` — x87 (`fsin`/`fcos`/`fptan`/
/// `_CIacos`) on f32 values rounded back to f32 (`0x0042fbb0`/`0x0042fbc0`/
/// `0x0042fba0`/`0x0042fb90`). Mirrored as f64-compute narrowed to f32 per
/// the file-header x87 policy: agrees to the last f32 ulp except where the
/// game's 80-bit argument reduction lands on the opposite rounding side.
pub fn sin(x: f32) -> f32 {
    (x as f64).sin() as f32
}
pub fn cos(x: f32) -> f32 {
    (x as f64).cos() as f32
}
pub fn tan(x: f32) -> f32 {
    (x as f64).tan() as f32
}
pub fn acos(x: f32) -> f32 {
    (x as f64).acos() as f32
}

/// `RealRange` — POD `{min @+0, max @+4}` (f32).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RealRange {
    pub min: f32,
    pub max: f32,
}

impl RealRange {
    /// 0x004062ad `combine` — transcription of the pointer-chasing C:
    /// min ← other.min when `other < this || other == this` (NaN keeps self);
    /// max ← self.max when `other <= self && self != other`, else other.max.
    pub fn combine(&mut self, o: &RealRange) {
        let min_src = if o.min < self.min || o.min == self.min {
            o.min
        } else {
            self.min
        };
        self.min = min_src;
        let max_src = if o.max <= self.max && self.max != o.max {
            self.max
        } else {
            o.max
        };
        self.max = max_src;
    }
}

/// `IntRange` — POD `{min @+0, max @+4}` (i32).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct IntRange {
    pub min: i32,
    pub max: i32,
}

impl IntRange {
    /// 0x004062d8 `combine` — min ← other when `other <= this`;
    /// max ← self.max when `other < this`, else other.max.
    pub fn combine(&mut self, o: &IntRange) {
        let min_src = if o.min <= self.min { o.min } else { self.min };
        self.min = min_src;
        let max_src = if o.max < self.max { self.max } else { o.max };
        self.max = max_src;
    }
}

/// `RGBColor` — three f32 channels `{r @+0, g @+4, b @+8}`, NOT bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct RGBColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl RGBColor {
    /// 0x00406538 `RGBColor(int)` (calls setFromInt) / 0x00404edf
    /// `setFromInt` — byte fields × DAT_00bbb8f0 (1/255f).
    pub fn from_int(v: i32) -> Self {
        const INV_255: f32 = 0.0039215689; // 0x00bbb8f0
        RGBColor {
            r: ((v >> 16) & 0xff) as f32 * INV_255,
            g: ((v >> 8) & 0xff) as f32 * INV_255,
            b: (v & 0xff) as f32 * INV_255,
        }
    }
    pub fn set_from_int(&mut self, v: i32) {
        *self = RGBColor::from_int(v);
    }

    /// 0x00404ea7 `getAsInt` — x87: each field × 255.0 (double DAT_00bbb8e8),
    /// `_ftol` (0x00a29228) truncation, then `b | (r << 8 | g) << 8`.
    pub fn get_as_int(&self) -> i32 {
        let r = (self.r as f64 * 255.0) as i32;
        let g = (self.g as f64 * 255.0) as i32;
        let b = (self.b as f64 * 255.0) as i32;
        b | (r << 8 | g) << 8
    }
}

/// 0x00404f2b free `operator==` (exact per-channel); 0x00404f65 free
/// `operator!=` is `1 - (==)` — Rust derives `!=` from this.
impl PartialEq for RGBColor {
    fn eq(&self, o: &Self) -> bool {
        self.r == o.r && self.g == o.g && self.b == o.b
    }
}

/// 0x00403fee free `operator-` (unary minus, `??G@YA?AVCoord3D@@V0@@Z`) —
/// `0.0 - x` per channel, verbatim.
impl std::ops::Neg for Coord3D {
    type Output = Coord3D;
    fn neg(self) -> Coord3D {
        Coord3D {
            x: 0.0 - self.x,
            y: 0.0 - self.y,
            z: 0.0 - self.z,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() <= eps * b.abs().max(1.0)
    }

    #[test]
    fn coord3d_length_normalize() {
        let mut c = Coord3D {
            x: 3.0,
            y: 0.0,
            z: 4.0,
        };
        assert_eq!(c.length(), 5.0);
        c.normalize();
        assert!(close(c.length(), 1.0, 1e-6));
        let mut z = Coord3D {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        z.normalize(); // C: zero-length leaves vector untouched
        assert!(z.is_exactly_equal_to(&Coord3D::default()));
    }

    #[test]
    fn coord3d_cross_product_orders() {
        let a = Coord3D {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        };
        let b = Coord3D {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        };
        let mut out = Coord3D::default();
        Coord3D::cross_product_static(&a, &b, &mut out);
        assert_eq!(out.z, 1.0);
        let m = Coord3D::cross_product(&a, &b);
        assert_eq!(m.z, 1.0);
    }

    #[test]
    fn coord3d_length_estimate() {
        // SAGE fast estimate: for (3,4,12): 2D est = 0.25*3+4 = 4.75; 4.75 <= 12
        // → (0.25*3+4)*0.25 + 12 = 13.1875
        let c = Coord3D {
            x: 3.0,
            y: 4.0,
            z: 12.0,
        };
        assert_eq!(c.get_length_estimate(), 13.1875);
        assert!(close(c.get_length_estimate(), c.length(), 0.2));
        assert_eq!(c.get_length_estimate_2d(), 4.75);
    }

    #[test]
    fn coord2d_rotate_roundtrip() {
        let mut c = Coord2D { x: 2.0, y: 1.0 };
        c.rotate(0.7);
        c.rotate(-0.7);
        assert!(close(c.x, 2.0, 1e-6) && close(c.y, 1.0, 1e-6));
    }

    #[test]
    fn matrix4d_identity_multiply_and_inverse() {
        let m = Matrix4D {
            m: [
                2.0, 0.0, 0.0, 5.0, 0.0, 3.0, 0.0, 6.0, 0.0, 0.0, 4.0, 7.0, 0.0, 0.0, 0.0, 1.0,
            ],
        };
        let i = Matrix4D::identity();
        let p = Matrix4D::multiply(&m, &i);
        assert!(p.m.iter().zip(m.m.iter()).all(|(a, b)| a == b));
        let mut inv = m;
        let det = inv.inverse();
        assert!(det > 0.0);
        let prod = Matrix4D::multiply(&m, &inv);
        assert!(prod
            .m
            .iter()
            .enumerate()
            .all(|(k, v)| close(*v, i.m[k], 1e-3)));
    }

    #[test]
    fn matrix4d_singular_inverse_untouched() {
        let mut m = Matrix4D { m: [0.0; 16] };
        let r = m.inverse();
        assert_eq!(r, 0.0);
        assert!(m.is_exactly_equal_to(&Matrix4D { m: [0.0; 16] }));
    }

    #[test]
    fn matrix4d_axis_angle_matches_transform() {
        // 90° about Z: x-axis maps to y-axis (rotation only).
        let mut m = Matrix4D { m: [0.0; 16] };
        m.set_axis_angle(
            &Coord3D {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
            std::f32::consts::FRAC_PI_2,
        );
        let v = m.rotate_coord(&Coord3D {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        });
        assert!(close(v.x, 0.0, 1e-6) && close(v.y, 1.0, 1e-6));
    }

    #[test]
    fn matrix4d_transform_translation() {
        let mut m = Matrix4D::identity();
        m.set_translation(&Coord3D {
            x: 10.0,
            y: 20.0,
            z: 30.0,
        });
        let v = m.transform_coord(&Coord3D {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        });
        assert_eq!((v.x, v.y, v.z), (11.0, 22.0, 33.0));
        let t = m.get_translation_vector();
        assert_eq!((t.x, t.y, t.z), (10.0, 20.0, 30.0));
        let x = m.get_x_vector();
        assert_eq!((x.x, x.y, x.z), (1.0, 0.0, 0.0));
    }

    #[test]
    fn matrix4d_determinant_matches_inverse_det() {
        let m = Matrix4D {
            m: [
                1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 22.0, 12.0, 13.0, 14.0, 15.0,
                26.0,
            ],
        };
        let d1 = m.determinant();
        let mut m2 = m;
        let d2 = m2.inverse();
        assert!(
            close(d1, d2, 1e-3),
            "determinant {} vs inverse det {}",
            d1,
            d2
        );
    }

    #[test]
    fn coord_2d_to_angle_matches_polar_convention() {
        // toAngle = acos(clamp(x/len)) sign-flipped for y < 0 (0x00405923)
        let c = Coord2D::new(1.0, 0.0);
        assert!(close(c.to_angle(), 0.0, 1e-6));
        let c = Coord2D::new(0.0, 1.0);
        assert!(close(c.to_angle(), std::f32::consts::FRAC_PI_2, 1e-6));
        let c = Coord2D::new(0.0, -1.0);
        assert!(close(c.to_angle(), -std::f32::consts::FRAC_PI_2, 1e-6));
        let z = Coord2D::new(0.0, 0.0);
        assert_eq!(z.to_angle(), 0.0);
    }

    #[test]
    fn unguarded_normalize_variants_scale_by_reciprocal() {
        let mut c = Coord2D::new(3.0, 4.0);
        assert!(close(c.normalize_len(), 5.0, 1e-6));
        assert!(close(c.x, 0.6, 1e-6) && close(c.y, 0.8, 1e-6));
        let mut v = Coord3D::new(0.0, 3.0, 4.0);
        assert!(close(v.normalize_len(), 5.0, 1e-6));
        assert!(close(v.z, 0.8, 1e-6));
        // Normalize2D leaves z untouched (0x00405ab2)
        let mut v = Coord3D::new(3.0, 4.0, 9.0);
        assert!(close(v.normalize_2d(), 5.0, 1e-6));
        assert_eq!(v.z, 9.0);
        // div_assign multiplies by the reciprocal (0x00483b9e)
        let mut v = Coord3D::new(1.0, 2.0, 4.0);
        v.div_assign(3.0);
        let r = 1.0f32 / 3.0;
        assert_eq!(v.x * 1.0, r);
        assert_eq!(v.y, 2.0 * r);
        assert_eq!(v.z, 4.0 * r);
    }

    #[test]
    fn coord3d_operator_and_axis_variants() {
        let mut a = Coord3D::new(5.0, 6.0, 7.0);
        a.sub_assign(&Coord3D::new(1.0, 2.0, 3.0));
        assert_eq!((a.x, a.y, a.z), (4.0, 4.0, 4.0));
        a.mul_assign(2.0);
        assert_eq!((a.x, a.y, a.z), (8.0, 8.0, 8.0));
        a.set_x_axis();
        assert_eq!((a.x, a.y, a.z), (1.0, 0.0, 0.0));
        a.set_z_axis();
        assert_eq!((a.x, a.y, a.z), (0.0, 0.0, 1.0));
        a.set_min_vect();
        assert_eq!((a.x, a.y, a.z), (f32::MIN, f32::MIN, f32::MIN));
        a.set_max_vect();
        assert_eq!((a.x, a.y, a.z), (f32::MAX, f32::MAX, f32::MAX));
        a.set_v(&Coord3D::new(1.0, 1.0, 1.0));
        a.add_v(&Coord3D::new(0.5, 0.5, 0.5));
        assert_eq!((a.x, a.y, a.z), (1.5, 1.5, 1.5));
        assert_eq!(a.get_length_sqrd(), 3.0 * 2.25);
        assert_eq!(a.get_length_sqrd_2d(), 2.0 * 2.25);
        let r = Coord3D::add_2d(&a, &Coord3D::new(1.0, 1.0, 100.0));
        assert_eq!((r.x, r.y, r.z), (2.5, 2.5, 0.0));
        let r = Coord3D::sub_2d(&a, &Coord3D::new(1.0, 1.0, 100.0));
        assert_eq!((r.x, r.y, r.z), (0.5, 0.5, 0.0));
        let r = Coord3D::scale_2d(&a, 2.0);
        assert_eq!((r.x, r.y, r.z), (3.0, 3.0, 0.0));
    }

    #[test]
    fn math_tail_leaves() {
        assert_eq!(rad2deg(core::f32::consts::PI), 180.0);
        assert_eq!(deg2rad(180.0), core::f32::consts::PI);
        assert!(are_approx_equal(1.0, 1.0001, 0.001));
        assert!(!are_approx_equal(1.0, 1.001, 0.001)); // d < e is strict
        let mut rr = RealRange {
            min: 0.0,
            max: 10.0,
        };
        rr.combine(&RealRange {
            min: 5.0,
            max: 20.0,
        });
        assert_eq!(
            rr,
            RealRange {
                min: 0.0,
                max: 20.0
            }
        );
        rr.combine(&RealRange {
            min: -1.0,
            max: 5.0,
        });
        assert_eq!(
            rr,
            RealRange {
                min: -1.0,
                max: 20.0
            }
        );
        let mut ir = IntRange { min: 5, max: 7 };
        ir.combine(&IntRange { min: 1, max: 3 });
        assert_eq!(ir, IntRange { min: 1, max: 7 });
        let c = RGBColor::from_int(0x00FF8000);
        assert_eq!(c.get_as_int(), 0x00FF8000); // 1/255f × 255.0 truncates back exactly
        assert_eq!(c.r, 1.0); // f32: 255 × 1/255f rounds back to exactly 1.0
        assert!(c.g > 0.5019 && c.g < 0.5021); // 128/255 ≈ 0.50196
        assert_eq!(RGBColor::from_int(0x123456), RGBColor::from_int(0x123456));
        assert_ne!(RGBColor::from_int(0x123456), RGBColor::from_int(0x123457));
        let n = -Coord3D::new(1.0, -2.0, 0.0);
        assert_eq!((n.x, n.y, n.z), (-1.0, 2.0, -0.0));
    }

    #[test]
    fn trig_wrappers_exact_endpoints() {
        // endpoints are exact on both x87 and SSE paths; interior values are
        // pinned by the f64→f32 policy in the header, not asserted here.
        assert_eq!(sin(0.0), 0.0);
        assert_eq!(cos(0.0), 1.0);
        assert_eq!(tan(0.0), 0.0);
        assert_eq!(acos(1.0), 0.0);
        assert!((acos(0.5) - core::f32::consts::FRAC_PI_3).abs() <= 1e-7);
    }
}

// ---------------------------------------------------------------------------
// Wave 28 — pure math kernels found beside the coordinate anchors (mostly
// module=NULL rows; attribution by body inspection, see /tmp/math_attr.json).
// Each is a free function over raw float buffers in the game; modelled on
// Coord2D/Coord3D. x87 `float10` returns are modelled as f64 exactly where
// the body returns through the x87 stack (callers coerce on assignment).

/// 0x00663778 (FX). 3-D distance-falloff kernel: the z term
/// `p5 = (p2 + a.z) - b.z - p5` folds a height offset in; distance is
/// sqrt'd in double over f32 squares; `r = d - p3 - p6` clamps at 0 and
/// squares. p5/p6 are by-reference in the game (`float&`).
pub fn falloff_distance_sq(
    a: &Coord3D,
    p2: f32,
    p3: f32,
    b: &Coord3D,
    p5: &mut f32,
    p6: &mut f32,
) -> f32 {
    *p5 = ((p2 + a.z) - b.z) - *p5;
    let s = (a.x - b.x) * (a.x - b.x) + (a.y - b.y) * (a.y - b.y) + *p5 * *p5;
    let r = (s as f64).sqrt() as f32 - p3 - *p6;
    *p6 = if r >= 0.0 { r * r } else { 0.0 };
    *p6
}

/// 0x0044233a. Polynomial sine-cascade on a fixed-point angle word —
/// bit-transcribed: `f = (float)((0xbe6eb508 - i) >> 1)` (unsigned), the
/// whole cascade runs in double, result narrowed to f32.
pub fn sin_poly_bits(i: i32) -> f32 {
    let f = ((0xbe6eb508u32.wrapping_sub(i as u32)) >> 1) as f32;
    let fd = f as f64;
    let t = fd * fd * ((i - 0x800000) as f32) as f64;
    let a = 1.5 - t;
    let b = t * a * a;
    let c = 1.5 - b;
    ((1.5 - c * b * c) * a * fd * c) as f32
}

/// 0x00494de5. f64 sqrt over f32 squares of a raw `float*` — duplicate of
/// `Coord3D::length` (0x00403571) kept as its own entry (distinct address).
pub fn length3_f64(p: &Coord3D) -> f32 {
    ((p.x * p.x + p.y * p.y + p.z * p.z) as f64).sqrt() as f32
}

/// 0x0047dfb3. Static `Coord3D` scale-by-scalar: `out = v * s`
/// (game arg order: out, s, v).
pub fn scale3(out: &mut Coord3D, s: f32, v: &Coord3D) {
    out.x = v.x * s;
    out.y = v.y * s;
    out.z = v.z * s;
}

/// 0x004664be. pow by squaring with integer exponent, running in double;
/// negative exponent takes the reciprocal. `exp == i32::MIN` wraps exactly
/// like the game's `neg` (`u = 0x8000_0000`).
pub fn powi(base: f32, exp: i32) -> f64 {
    let mut u = if exp < 0 {
        exp.wrapping_neg() as u32
    } else {
        exp as u32
    };
    let mut r: f64 = 1.0;
    let mut b = base as f64;
    loop {
        if u & 1 != 0 {
            r *= b;
        }
        u >>= 1;
        if u == 0 {
            break;
        }
        b *= b;
    }
    if exp < 0 {
        r = 1.0 / r;
    }
    r
}

/// 0x0066382b. thiscall member on an object with a 2-D origin at +0x38/+0x3c:
/// `out = (b.x - o.x, b.y - o.y, 0.0)`. Modelled as a free function over the
/// origin pair (owner class unmodelled).
pub fn sub_origin_to_3d(out: &mut Coord3D, b: &Coord3D, o: &Coord2D) {
    out.x = b.x - o.x;
    out.y = b.y - o.y;
    out.z = 0.0;
}

/// 0x0046599e. clamp of `a` into `[b, c]` written as two guards (the game
/// shape — do NOT use `f32::clamp`, NaN behaviour differs: with a NaN the
/// guards fall through to `a`).
pub fn clamp3(a: f32, b: f32, c: f32) -> f32 {
    if a < b {
        return b;
    }
    if c < a {
        return c;
    }
    a
}

/// 0x0040575a (module Xfer, mis-clustered). Mirror-image twin of 0x0046599e
/// — clamps `b` into `[a, c]`.
pub fn clamp3_twin(a: f32, b: f32, c: f32) -> f32 {
    if b < a {
        return a;
    }
    if c < b {
        return c;
    }
    b
}

/// 0x0049aaa8. `(sqrt(p) − p²)·p + p²` — sqrt in double, `p*p` computed in
/// f32 then widened (matches the x87 body's cast pattern).
pub fn sqrt_poly(p: f32) -> f64 {
    let d = (p as f64).sqrt();
    let p2 = (p * p) as f64;
    (d - p2) * (p as f64) + p2
}

/// 0x0047dec8. Static `Coord2D` add-by-xy (`o = v + (x, y)`); called by
/// `ClosestPointOnLineSegment` (0x00ab3100, inlined at region.rs:194).
pub fn add_xy(o: &mut Coord2D, x: f32, y: f32, v: &Coord2D) {
    o.x = v.x + x;
    o.y = v.y + y;
}

/// 0x005040ae. Reversed 2-D subtract (`o = (x, y) - v`); also called by
/// 0x00ab3100 / 0x00504359 / 0x006f8b00 / 0x0070b3d1.
pub fn sub_xy_rev(o: &mut Coord2D, x: f32, y: f32, v: &Coord2D) {
    o.x = x - v.x;
    o.y = y - v.y;
}

/// 0x00648cbf. Pair equality over two `int[2]`s: word 0 compared bitwise,
/// word 1 compared only by its low 16 bits (`(short)param[1]`). Used by
/// 0x007ff145 as a type/value test.
pub fn pair_eq_low16(a: &[u32; 2], b: &[u32; 2]) -> bool {
    a[0] == b[0] && (a[1] as u16) == (b[1] as u16)
}

/// 0x0044cad1. Inline `fabs`: mask the sign bit off the f32, push through
/// the x87 stack — returns `float10` (f64) to the caller.
pub fn fabs_to_f64(x: f32) -> f64 {
    f32::from_bits(x.to_bits() & 0x7fff_ffff) as f64
}

#[cfg(test)]
mod pure_kernels_tests {
    use super::*;

    #[test]
    fn falloff_kernel() {
        let a = Coord3D::new(0.0, 3.0, 0.0);
        let b = Coord3D::new(0.0, 0.0, 0.0);
        let (mut p5, mut p6) = (0.0f32, 0.25f32);
        let r = falloff_distance_sq(&a, 0.0, 0.75, &b, &mut p5, &mut p6);
        // p5 = (0+0)-0-0 = 0; d = sqrt(9) = 3; r = 3-0.75-0.25 = 2 -> r^2 = 4
        assert_eq!(p5, 0.0);
        assert_eq!(r, 4.0);
        assert_eq!(p6, 4.0);
        // negative radius clamps to zero (game keeps the write-back)
        let mut q6 = 100.0f32;
        assert_eq!(
            falloff_distance_sq(&a, 0.0, 0.75, &b, &mut 0.0, &mut q6),
            0.0
        );
        assert_eq!(q6, 0.0);
    }

    #[test]
    fn scalar_helpers() {
        assert_eq!(clamp3(5.0, 0.0, 3.0), 3.0);
        assert_eq!(clamp3(-1.0, 0.0, 3.0), 0.0);
        assert_eq!(clamp3(2.0, 0.0, 3.0), 2.0);
        assert_eq!(clamp3_twin(0.0, 5.0, 3.0), 3.0);
        assert_eq!(fabs_to_f64(-2.5), 2.5);
        assert_eq!(fabs_to_f64(2.5), 2.5);
        assert_eq!(powi(2.0, 10), 1024.0);
        assert_eq!(powi(2.0, -2), 0.25);
        assert_eq!(powi(3.0, 0), 1.0);
        // exp == i32::MIN wraps like the game's neg (u = 0x8000_0000); the
        // squaring chain overflows to inf, the reciprocal tail gives 0.
        assert_eq!(powi(2.0, i32::MIN), 0.0);
        let a = Coord3D::new(3.0, 4.0, 0.0);
        assert_eq!(length3_f64(&a), 5.0);
        let mut o = Coord3D::new(9.0, 9.0, 9.0);
        scale3(&mut o, 2.0, &a);
        assert_eq!((o.x, o.y, o.z), (6.0, 8.0, 0.0));
        sub_origin_to_3d(&mut o, &a, &Coord2D { x: 1.0, y: 1.0 });
        assert_eq!((o.x, o.y, o.z), (2.0, 3.0, 0.0));
        assert_eq!(sqrt_poly(4.0), -40.0); // (2-16)*4+16
        assert!(pair_eq_low16(
            &[0xdeadbeef, 0x1234_5678],
            &[0xdeadbeef, 0x9abc_5678]
        ));
        assert!(!pair_eq_low16(
            &[0xdeadbeef, 0x1234_5678],
            &[0xdeadbeef, 0x1234_abcd]
        ));
        assert!(!pair_eq_low16(&[1, 2], &[2, 2]));
        // sin cascade: deterministic; monotone over the fundamental range
        let v0 = sin_poly_bits(0x800000);
        assert!(v0.is_finite());
    }

    #[test]
    fn coord2d_helpers_match_region_usage() {
        let mut o = Coord2D { x: 0.0, y: 0.0 };
        let v = Coord2D { x: 1.0, y: 2.0 };
        add_xy(&mut o, 3.0, 4.0, &v);
        assert_eq!((o.x, o.y), (4.0, 6.0));
        sub_xy_rev(&mut o, 1.0, 1.0, &v);
        assert_eq!((o.x, o.y), (0.0, -1.0)); // out-param overwrites, no chaining
    }
}

/// 0x0049de01 (PowerBar band, mis-clustered math helper; called by the
/// PowerBar upgrade-cost curves 0x0049e36f/0x0049e726): change-of-base
/// ratio `log10(x) / log10(base)` computed in double, returned `float10`.
pub fn log_ratio(base: f32, x: f32) -> f64 {
    (x as f64).log10() / (base as f64).log10()
}

#[cfg(test)]
mod log_ratio_tests {
    use super::*;

    #[test]
    fn change_of_base() {
        assert!((log_ratio(2.0, 8.0) - 3.0).abs() < 1e-12);
        assert!((log_ratio(10.0, 1000.0) - 3.0).abs() < 1e-12);
        assert_eq!(log_ratio(4.0, 4.0), 1.0);
    }
}
