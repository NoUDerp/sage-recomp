//! `Xfer` base class — SAGE serialization dispatcher.
//!
//! Ground model from decomp (33 ledger rows, cluster `Xfer::`): every
//! type forwarder in the base class funnels through the pure-virtual
//! hook at vtable slot +0x98 (`Xfer::XferInternal(tag, value, size)`;
//! base slot = `0x0043b810` purecall shim: `push 1; call 0x00438790`).
//! Base vtable `0x00bbb910` (from ctor 0x004053de):
//!
//! ||slot||off||identity||
//! ||---||---||---||
//! ||[1..4]||+0x04..+0x10|| `IsCRC`/`IsLightCRC`/`IsLoading`/`IsStoring`
//! — all four share the base stub `0x0087a699` (`return false`); the
//! forwarder bodies branch on slot **+0x08** (true ⇒ store scheme,
//! false ⇒ load scheme), which identifies *that* slot functionally as
//! `IsStoring`; the remaining three slot names are order-uncertain but
//! behaviorally inert in the base.||
//! ||[8]||+0x20|| `SkipBadBlock` stub `0x0090b238` (no-op)||
//! ||[9]||+0x24|| `XferRawBytes` `0x00a0b886`||
//! ||[10]||+0x28|| `xfer(Version&)` `0x00a0b8d9`||
//! ||[12..24,26,27,29..37]|| ||per-type forwarders (this module)||
//! ||[25]||+0x64|| `0x00a0c0a0` — EH prolog shell (calls `strlen` on a
//! `char* const&` member): unmirrored until /GX repair (ledger task #5)||
//! ||[38]||+0x98|| pure hook `0x0043b810`||
//!
//! Slots [5],[6],[7],[11] are also purecall (`0x0043b810`) — four
//! further pure virtuals whose types are not in the mirrored surface.

use std::fmt;

pub const TAG_BYTE: u32 = 0x6279_7465; // "byte" (as inline LE chars)
pub const TAG_UBYTE: u32 = 0x7562_7974; // "ubyte"
pub const TAG_BOOL: u32 = 0x626f_6f6c; // "bool"
pub const TAG_SHORT: u32 = 0x7368_7274; // "shrt"
pub const TAG_USHORT: u32 = 0x7573_6874; // "ushrt" 0x75736874
pub const TAG_INT: u32 = 0x0069_6e74; // "int"
pub const TAG_UINT: u32 = 0x7569_6e74; // "uint"
pub const TAG_REAL: u32 = 0x7265_616c; // "real"
pub const TAG_I64: u32 = 0x0069_3634; // "i64"
pub const TAG_RAW: u32 = 0x0072_6177; // "raw"
pub const TAG_VER: u32 = 0x0076_6572; // "ver"
pub const TAG_ASTR: u32 = 0x6173_7472; // "astr"
pub const TAG_USTR: u32 = 0x7573_7472; // "ustr"
pub const TAG_C3D: u32 = 0x0063_3364; // "c3d"
pub const TAG_IC3D: u32 = 0x6963_3364; // "ic3d"
pub const TAG_R3D: u32 = 0x0072_3364; // "r3d"
pub const TAG_IR3D: u32 = 0x6972_3364; // "ir3d"
pub const TAG_C2D_PTR: u32 = 0x0063_3264; // "c2d" (Coord2D passes &LAB_00633264, the constant's address)
pub const TAG_IC2D: u32 = 0x6963_3264; // "ic2d"
pub const TAG_R2D: u32 = 0x0072_3264; // "r2d"
pub const TAG_IR2D: u32 = 0x6972_3264; // "ir2d"
pub const TAG_RANGE: u32 = 0x726e_6765; // "rnge"
pub const TAG_RGB: u32 = 0x0072_6762; // "rgb"
pub const TAG_RGBR: u32 = 0x7267_6272; // "rgbr"
pub const TAG_RGBI: u32 = 0x7267_6269; // "rgbi"
pub const TAG_ENU: u32 = 0x656e_7530; // "enu1" = TAG_ENU + n

/// Thrown by version-range violations (base `xfer(Version&)` builds a
/// message at 0x00a0c36e then `_CxxThrowException` with ThrowInfo
/// `0x00cffd18`; format strings `0x00c7ae8c`/`0x00c7ae54`).
#[derive(Debug, Clone, PartialEq)]
pub struct VersionMismatch {
    pub fmt: &'static str,
    pub a: u8,
    pub b: u8,
}

impl fmt::Display for VersionMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.fmt {
            s if s.starts_with("Found version") => {
                write!(
                    f,
                    "Found version {} but the earliest supported is {}",
                    self.a, self.b
                )
            }
            _ => write!(
                f,
                "Don't know how to load version {} (most recent is {})",
                self.a, self.b
            ),
        }
    }
}

pub const FMT_OLD: &str = "Found version %i but the earliest supported is %i"; // 0x00c7ae8c
pub const FMT_NEW: &str = "Don't know how to load version %i (most recent is %i)"; // 0x00c7ae54

/// `Xfer::Version` (2 bytes: [0] = minimum acceptable version,
/// [1] = version being/currently transferred).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Version {
    pub minimum: u8,
    pub current: u8,
}

/// The pure hook + mode predicates. `xfer_internal(tag, value)` matches
/// the `(tag, value, size)` C signature; Rust passes the LE bytes as a
/// mutable slice so a load-mode backend can write through exactly like
/// the game hook.
pub trait Xfer {
    /// vtable +0x98, pure (`0x0043b810`).
    fn xfer_internal(&mut self, tag: u32, value: &mut [u8]);
    /// vtable +0x08: true ⇒ store scheme (length known from object).
    /// Base stub `0x0087a699` returns false.
    fn is_storing(&self) -> bool {
        false
    }
    /// vtable +0x04/+0x0c/+0x10: the other three `Is*` predicates share
    /// the same base stub; slot order among them is unconfirmed but
    /// they are inert in the base forwarders (only +0x08 branches).
    fn is_crc(&self) -> bool {
        false
    }
    fn is_light_crc(&self) -> bool {
        false
    }
    fn is_loading(&self) -> bool {
        false
    }

    /// `Xfer::SkipBadBlock` (0x0090b238): base stub no-op.
    fn skip_bad_block(&mut self, _snapshot: &mut dyn XferSnapshot, _size: u32) {}

    /// `Xfer::XferRawBytes` (0x00a0b886): the guard is
    /// `(size == 0) || (ptr != null)` — an empty transfer still emits
    /// both hook calls; only a null pointer with nonzero size is
    /// skipped (impossible for a Rust slice).
    fn xfer_raw_bytes(&mut self, value: &mut [u8]) {
        let mut size = (value.len() as u32).to_le_bytes();
        self.xfer_internal(TAG_RAW, &mut size);
        let size = u32::from_le_bytes(size);
        debug_assert_eq!(size as usize, value.len());
        self.xfer_internal(0, &mut value[..size as usize]);
    }

    /// `Xfer::XferEnum(label, void* value, uint size)` (0x00a0bbd5):
    /// tag `"enu" + size`, size ∈ 1..=4. Outside range the game calls
    /// the report/throw chain at 0x00a0bc34 (`0x00a0c36e` →
    /// `_CxxThrowException`) — mirrored as panic (format lost: the
    /// reporter body is an EH shell, ledger task #5).
    fn xfer_enum(&mut self, value: &mut [u8]) {
        let n = value.len();
        if n == 0 || n > 4 {
            panic!("XferEnum: invalid enum size {n} (game throws at 0x00a0bc34)");
        }
        self.xfer_internal(TAG_ENU + n as u32, value);
    }

    /// Label-carrying `XferEnum` call sites (`FUN_00706052`/`00706022`/
    /// `0070603a`/`00705f92` push the keyword-table label as param_1). The
    /// base class ignores it — 0x00a0bbd5 never touches param_1 — concrete
    /// INI backends use it for keyword lookup and override here.
    fn xfer_enum_named(&mut self, _label: &str, value: &mut [u8]) {
        self.xfer_enum(value);
    }

    // ---- primitive forwarders (vtable slots [28..36]) ----
    // tag immediates are the 4-char LE codes the game pushes; value is
    // marshalled LE, exactly as the C hook receives `void* + size`.

    /// 0x00405468 slot [35]: `xfer(char&)`, tag 'byte', size 1.
    fn xfer_i8(&mut self, v: &mut i8) {
        let mut b = [*v as u8];
        self.xfer_internal(TAG_BYTE, &mut b);
        *v = b[0] as i8;
    }
    /// 0x00405488 [34]: `xfer(unsigned char&)`, tag 'ubyte'.
    fn xfer_u8(&mut self, v: &mut u8) {
        let mut b = [*v];
        self.xfer_internal(TAG_UBYTE, &mut b);
        *v = b[0];
    }
    /// 0x004054a4 [37]: `xfer(bool&)`, tag 'bool', size 1.
    fn xfer_bool(&mut self, v: &mut bool) {
        let mut b = [*v as u8];
        self.xfer_internal(TAG_BOOL, &mut b);
        *v = b[0] != 0;
    }
    /// 0x00405514 [33]: `xfer(short&)`, tag 'shrt', size 2.
    fn xfer_i16(&mut self, v: &mut i16) {
        let mut b = v.to_le_bytes();
        self.xfer_internal(TAG_SHORT, &mut b);
        *v = i16::from_le_bytes(b);
    }
    /// 0x00405530 [32]: `xfer(unsigned short&)`, tag 'ushrt'.
    fn xfer_u16(&mut self, v: &mut u16) {
        let mut b = v.to_le_bytes();
        self.xfer_internal(TAG_USHORT, &mut b);
        *v = u16::from_le_bytes(b);
    }
    /// 0x004054c0 [31]: `xfer(int&)`, tag 'int', size 4.
    fn xfer_i32(&mut self, v: &mut i32) {
        let mut b = v.to_le_bytes();
        self.xfer_internal(TAG_INT, &mut b);
        *v = i32::from_le_bytes(b);
    }
    /// 0x004054f8 [30]: `xfer(unsigned int&)`, tag 'uint'.
    fn xfer_u32(&mut self, v: &mut u32) {
        let mut b = v.to_le_bytes();
        self.xfer_internal(TAG_UINT, &mut b);
        *v = u32::from_le_bytes(b);
    }
    /// 0x0040554c [28]: `xfer(float&)`, tag 'real', size 4.
    fn xfer_f32(&mut self, v: &mut f32) {
        let mut b = v.to_le_bytes();
        self.xfer_internal(TAG_REAL, &mut b);
        *v = f32::from_le_bytes(b);
    }
    /// 0x004054dc [29]: `xfer(__int64&)`, tag 'i64', size 8.
    fn xfer_i64(&mut self, v: &mut i64) {
        let mut b = v.to_le_bytes();
        self.xfer_internal(TAG_I64, &mut b);
        *v = i64::from_le_bytes(b);
    }

    /// `xfer(Version&)` (0x00a0b8d9). Store mode: version byte only.
    /// Load mode: hook the byte, then range-check against the object's
    /// `minimum` / `current`:
    /// `v[1] < v[0]` ⇒ throw "Found version %i but the earliest
    /// supported is %i"; `v[1] > pre-hook value` ⇒ "Don't know how to
    /// load version %i (most recent is %i)". (Both report chains end in
    /// `_CxxThrowException` 0x00a29094 — mirrored as panics.)
    fn xfer_version(&mut self, v: &mut Version) {
        if self.is_storing() {
            let mut b = [v.current];
            self.xfer_internal(TAG_VER, &mut b);
            v.current = b[0];
            return;
        }
        let pre = v.current;
        let mut b = [v.current];
        self.xfer_internal(TAG_VER, &mut b);
        v.current = b[0];
        if v.current < v.minimum {
            panic_mismatch(FMT_OLD, v.current, v.minimum);
        } else if v.current > pre {
            panic_mismatch(FMT_NEW, v.current, pre);
        }
    }

    /// `Xfer::Version1` (0x004053ee): local `Version{1,1}` through the
    /// virtual Version forwarder (slot [10]).
    fn version1(&mut self) {
        let mut v = Version {
            minimum: 1,
            current: 1,
        };
        self.xfer_version(&mut v);
    }

    /// `xfer(Snapshot&)` (0x00a0b8c6): `snapshot.xfer_with(this)` —
    /// the C code calls the Snapshot vtable slot +0xc.
    fn xfer_snapshot(&mut self, s: &mut dyn XferSnapshot)
    where
        Self: Sized,
    {
        s.xfer_with(self);
    }

    // ---- struct forwarders: tag-announce (size 0) + raw blob ----

    /// 0x00a0b971: tag 'c3d' + 12-byte blob.
    fn xfer_coord3d(&mut self, c: &mut Coord3D) {
        self.xfer_internal(TAG_C3D, &mut []);
        let mut b = c.to_bytes();
        self.xfer_internal(0, &mut b);
        *c = Coord3D::from_bytes(&b);
    }
    /// 0x00a0b99d: 'ic3d' + 12.
    fn xfer_icoord3d(&mut self, c: &mut ICoord3D) {
        self.xfer_internal(TAG_IC3D, &mut []);
        let mut b = c.to_bytes();
        self.xfer_internal(0, &mut b);
        *c = ICoord3D::from_bytes(&b);
    }
    /// 0x00a0b9c9: 'r3d' + two 12-byte blobs (min, max).
    fn xfer_region3d(&mut self, r: &mut Region3D) {
        self.xfer_internal(TAG_R3D, &mut []);
        let mut b = r.min.to_bytes();
        self.xfer_internal(0, &mut b);
        r.min = Coord3D::from_bytes(&b);
        let mut b = r.max.to_bytes();
        self.xfer_internal(0, &mut b);
        r.max = Coord3D::from_bytes(&b);
    }
    /// 0x00a0ba0a: 'ir3d' + two 12-byte int blobs.
    fn xfer_iregion3d(&mut self, r: &mut IRegion3D) {
        self.xfer_internal(TAG_IR3D, &mut []);
        let mut b = r.min.to_bytes();
        self.xfer_internal(0, &mut b);
        r.min = ICoord3D::from_bytes(&b);
        let mut b = r.max.to_bytes();
        self.xfer_internal(0, &mut b);
        r.max = ICoord3D::from_bytes(&b);
    }
    /// 0x00a0ba4b: tag passed by pointer `&LAB_00633264` ("c2d") + 8.
    fn xfer_coord2d(&mut self, c: &mut Coord2D) {
        self.xfer_internal(TAG_C2D_PTR, &mut []);
        let mut b = c.to_bytes();
        self.xfer_internal(0, &mut b);
        *c = Coord2D::from_bytes(&b);
    }
    /// 0x00a0ba77: 'ic2d' + 8.
    fn xfer_icoord2d(&mut self, c: &mut ICoord2D) {
        self.xfer_internal(TAG_IC2D, &mut []);
        let mut b = c.to_bytes();
        self.xfer_internal(0, &mut b);
        *c = ICoord2D::from_bytes(&b);
    }
    /// 0x00a0baa3: 'r2d' + two 8-byte blobs.
    fn xfer_region2d(&mut self, r: &mut Region2D) {
        self.xfer_internal(TAG_R2D, &mut []);
        let mut b = r.min.to_bytes();
        self.xfer_internal(0, &mut b);
        r.min = Coord2D::from_bytes(&b);
        let mut b = r.max.to_bytes();
        self.xfer_internal(0, &mut b);
        r.max = Coord2D::from_bytes(&b);
    }
    /// 0x00a0bae4: 'ir2d' + two 8-byte int blobs.
    fn xfer_iregion2d(&mut self, r: &mut IRegion2D) {
        self.xfer_internal(TAG_IR2D, &mut []);
        let mut b = r.min.to_bytes();
        self.xfer_internal(0, &mut b);
        r.min = ICoord2D::from_bytes(&b);
        let mut b = r.max.to_bytes();
        self.xfer_internal(0, &mut b);
        r.max = ICoord2D::from_bytes(&b);
    }
    /// 0x00a0bb25: 'rnge' + 8 (min,max f32).
    fn xfer_real_range(&mut self, r: &mut RealRange) {
        self.xfer_internal(TAG_RANGE, &mut []);
        let mut b = r.to_bytes();
        self.xfer_internal(0, &mut b);
        *r = RealRange::from_bytes(&b);
    }
    /// 0x00a0bb51: 'rgb' + 12.
    fn xfer_rgb_color(&mut self, c: &mut RGBColor) {
        self.xfer_internal(TAG_RGB, &mut []);
        let mut b = c.to_bytes();
        self.xfer_internal(0, &mut b);
        *c = RGBColor::from_bytes(&b);
    }
    /// 0x00a0bb7d: 'rgbr' + 16.
    fn xfer_rgba_color_real(&mut self, c: &mut RGBAColorReal) {
        self.xfer_internal(TAG_RGBR, &mut []);
        let mut b = c.to_bytes();
        self.xfer_internal(0, &mut b);
        *c = RGBAColorReal::from_bytes(&b);
    }
    /// 0x00a0bba9: 'rgbi' + 16.
    fn xfer_rgba_color_int(&mut self, c: &mut RGBAColorInt) {
        self.xfer_internal(TAG_RGBI, &mut []);
        let mut b = c.to_bytes();
        self.xfer_internal(0, &mut b);
        *c = RGBAColorInt::from_bytes(&b);
    }
}

fn panic_mismatch(fmt: &'static str, a: u8, b: u8) -> ! {
    panic!(
        "{}",
        VersionMismatch {
            fmt: if fmt.starts_with("Found") {
                FMT_OLD
            } else {
                FMT_NEW
            },
            a,
            b
        }
    );
}

pub trait XferSnapshot {
    /// Snapshot vtable slot +0xc: receive the xfer dispatcher.
    fn xfer_with(&mut self, x: &mut dyn Xfer);
}

// ---- POD value types transferred as raw blobs ----

macro_rules! pod {
    ($name:ident { $($field:ident : $ty:ty),* $(,)? } = $size:expr) => {
        #[derive(Clone, Copy, PartialEq, Debug, Default)]
        pub struct $name { $(pub $field: $ty),* }
        impl $name {
            pub fn to_bytes(&self) -> [u8; $size] {
                let mut b = [0u8; $size];
                let mut o = 0usize;
                $(
                    let bytes = Enc::enc(&self.$field);
                    b[o..o + bytes.len()].copy_from_slice(&bytes);
                    o += bytes.len();
                )*
                let _ = o;
                b
            }
            pub fn from_bytes(b: &[u8]) -> Self {
                let mut s = Self::default();
                let mut rest = b;
                $(
                    let n = std::mem::size_of::<$ty>();
                    let (head, tail) = rest.split_at(n);
                    s.$field = <$ty as Enc>::dec(head);
                    rest = tail;
                )*
                let _ = rest;
                s
            }
        }
        impl Enc for $name {
            fn enc(&self) -> Vec<u8> {
                self.to_bytes().to_vec()
            }
            fn dec(b: &[u8]) -> Self {
                Self::from_bytes(b)
            }
        }
    };
}

pub trait Enc: Copy {
    fn enc(&self) -> Vec<u8>;
    fn dec(b: &[u8]) -> Self;
}
macro_rules! scalar_enc {
    ($t:ty) => {
        impl Enc for $t {
            fn enc(&self) -> Vec<u8> {
                self.to_le_bytes().to_vec()
            }
            fn dec(b: &[u8]) -> Self {
                <$t>::from_le_bytes(b.try_into().unwrap())
            }
        }
    };
}
scalar_enc!(f32);
scalar_enc!(i32);
scalar_enc!(u32);

pod!(
    Coord3D {
        x: f32,
        y: f32,
        z: f32
    } = 12
);
pod!(
    ICoord3D {
        x: i32,
        y: i32,
        z: i32
    } = 12
);
pod!(Coord2D { x: f32, y: f32 } = 8);
pod!(ICoord2D { x: i32, y: i32 } = 8);
pod!(RealRange { min: f32, max: f32 } = 8);
pod!(
    RGBColor {
        red: f32,
        green: f32,
        blue: f32
    } = 12
);
pod!(
    RGBAColorReal {
        red: f32,
        green: f32,
        blue: f32,
        alpha: f32
    } = 16
);
pod!(
    RGBAColorInt {
        red: u32,
        green: u32,
        blue: u32,
        alpha: u32
    } = 16
);
pod!(
    Region3D {
        min: Coord3D,
        max: Coord3D
    } = 24
);
pod!(
    IRegion3D {
        min: ICoord3D,
        max: ICoord3D
    } = 24
);
pod!(
    Region2D {
        min: Coord2D,
        max: Coord2D
    } = 16
);
pod!(
    IRegion2D {
        min: ICoord2D,
        max: ICoord2D
    } = 16
);

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::AssertUnwindSafe;

    struct Recorder {
        ev: Vec<(u32, Vec<u8>)>,
    }
    impl Xfer for Recorder {
        fn xfer_internal(&mut self, tag: u32, value: &mut [u8]) {
            self.ev.push((tag, value.to_vec()));
        }
        fn is_storing(&self) -> bool {
            true
        }
    }

    #[test]
    fn forwarder_tag_size_sequences() {
        let mut r = Recorder { ev: vec![] };
        r.xfer_i32(&mut -7);
        r.xfer_f32(&mut 1.5);
        r.xfer_bool(&mut true);
        r.xfer_i8(&mut -1);
        assert_eq!(r.ev[0], (TAG_INT, (-7i32).to_le_bytes().to_vec()));
        assert_eq!(r.ev[1].1, 1.5f32.to_le_bytes().to_vec());
        assert_eq!(r.ev[2], (TAG_BOOL, vec![1]));
        assert_eq!(r.ev[3], (TAG_BYTE, vec![0xff]));
        r.xfer_coord3d(&mut Coord3D {
            x: 1.,
            y: 2.,
            z: 3.,
        });
        assert_eq!(r.ev[4], (TAG_C3D, vec![])); // tag announce, size 0
        assert_eq!(r.ev[5].0, 0);
        assert_eq!(r.ev[5].1.len(), 12);
        r.xfer_iregion3d(&mut IRegion3D::default());
        assert_eq!(r.ev[6].0, TAG_IR3D);
        assert_eq!(r.ev[7].1.len(), 12); // min blob
        assert_eq!(r.ev[8].1.len(), 12); // max blob
    }

    #[test]
    fn enum_tag_arithmetic() {
        let mut r = Recorder { ev: vec![] };
        r.xfer_enum(&mut [1u8]);
        let mut v4 = [0u8; 4];
        r.xfer_enum(&mut v4);
        assert_eq!(r.ev[0].0, TAG_ENU + 1); // "enu1"
        assert_eq!(r.ev[1].0, 0x656e_7534); // "enu4"
    }

    #[test]
    fn enum_invalid_size_throws_like_game() {
        let mut r = Recorder { ev: vec![] };
        let p = std::panic::catch_unwind(AssertUnwindSafe(|| r.xfer_enum(&mut [0u8; 5])));
        assert!(p.is_err()); // game: report+throw chain at 0x00a0bc34
    }

    #[test]
    fn version_load_range_checks() {
        struct Loader {
            incoming: u8,
        }
        impl Xfer for Loader {
            fn xfer_internal(&mut self, tag: u32, value: &mut [u8]) {
                if tag == TAG_VER {
                    value[0] = self.incoming;
                }
            }
        }
        // accepted: minimum <= stream <= pre
        let mut v = Version {
            minimum: 1,
            current: 3,
        };
        Loader { incoming: 2 }.xfer_version(&mut v);
        assert_eq!(v.current, 2);
        // older than minimum ⇒ "Found version" throw
        let mut v = Version {
            minimum: 2,
            current: 3,
        };
        let p = std::panic::catch_unwind(AssertUnwindSafe(|| {
            Loader { incoming: 1 }.xfer_version(&mut v)
        }));
        assert!(p.is_err());
        // newer than pre ⇒ "Don't know how to load version" throw
        let mut v = Version {
            minimum: 1,
            current: 2,
        };
        let p = std::panic::catch_unwind(AssertUnwindSafe(|| {
            Loader { incoming: 5 }.xfer_version(&mut v)
        }));
        assert!(p.is_err());
        // store mode: no range logic, byte just written
        let mut r = Recorder { ev: vec![] };
        r.xfer_version(&mut Version {
            minimum: 9,
            current: 4,
        });
        assert_eq!(r.ev, vec![(TAG_VER, vec![4])]);
    }

    #[test]
    fn raw_bytes_prefix_and_empty_guard() {
        let mut r = Recorder { ev: vec![] };
        r.xfer_raw_bytes(&mut [7u8, 8]);
        assert_eq!(r.ev[0], (TAG_RAW, 2u32.to_le_bytes().to_vec()));
        assert_eq!(r.ev[1], (0, vec![7, 8]));
        // size==0 passes the guard: both hooks still fire
        r.xfer_raw_bytes(&mut []);
        assert_eq!(r.ev[2], (TAG_RAW, 0u32.to_le_bytes().to_vec()));
        assert_eq!(r.ev[3], (0, vec![]));
    }

    #[test]
    fn pod_blob_roundtrip_le() {
        let c = RGBAColorInt {
            red: 1,
            green: 2,
            blue: 3,
            alpha: 4,
        };
        assert_eq!(RGBAColorInt::from_bytes(&c.to_bytes()), c);
        assert_eq!(Enc::enc(&c).as_slice(), &c.to_bytes()[..]);
        let r = Region2D {
            min: Coord2D { x: 0., y: -1. },
            max: Coord2D { x: 1., y: 1. },
        };
        assert_eq!(Region2D::from_bytes(&r.to_bytes()), r);
        // little-endian spot check
        assert_eq!(ICoord2D { x: 1, y: 2 }.to_bytes(), [1, 0, 0, 0, 2, 0, 0, 0]);
    }
}
