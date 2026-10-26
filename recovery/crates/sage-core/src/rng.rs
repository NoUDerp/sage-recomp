//! SAGE random number generator + `GameClientRandomVariable`.
//!
//! Mirrors the decompiled/disassembled machine code in the RNG translation
//! unit (0x00633e00..0x00634420). The three global generator instances are
//! `0x00dba3d0` (logic, logging variants), `0x00dba3a0` (game client), and
//! `0x00dba3b8` (misc float variant); `0x00633f82` seeds all three from one
//! value and caches it at `0x00dfe734`.
//!
//! Semantics recovered from instruction stream (Ghidra's C for `next()`
//! dropped the `EAX` result): `next` advances a 192-bit coupled-counter state
//! (6 u32 words) and returns the new word0.

/// 0x00633f3a: `seed(word) -> state` constants (additive chain, wrapping u32).
const SEED_K0: u32 = 0xf22d0e56;
const SEED_K1: u32 = 0x69fbe76d;
const SEED_K2: u32 = 0x3df3b646;
const SEED_K3: u32 = 0x40dde76d;
const SEED_K4: u32 = 0x68cd851f;
const SEED_K5: u32 = 0xd1a9fbe7;

/// f32 at `0x00bc26ec`: signed→unsigned float offset.
const NEG_ADJUST: f32 = 4294967296.0;
/// `0x00dfe738`, computed at init (0x00badbff) as `1/(pow(2,32)-1)` →
/// rounds to exactly 2^-32 in f32.
const RANDOM_SCALE: f32 = 2.3283064e-10; // 1.0f32 / 4294967295.0f32

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RandomGenerator {
    /// words at +0x00..0x14 of the game instance (e.g. 0x00dba3d0).
    pub state: [u32; 6],
}

impl RandomGenerator {
    /// 0x00633f3a `seed`
    pub fn seed(seed: u32) -> Self {
        let s0 = seed.wrapping_add(SEED_K0);
        let s1 = s0.wrapping_sub(SEED_K1);
        let s2 = s1.wrapping_add(SEED_K2);
        let s3 = s2.wrapping_add(SEED_K3);
        let s4 = s3.wrapping_sub(SEED_K4);
        let s5 = s4.wrapping_add(SEED_K5);
        RandomGenerator {
            state: [s0, s1, s2, s3, s4, s5],
        }
    }

    /// 0x00633ec3 `next` — returns the new state word 0 (signed in callers).
    pub fn next(&mut self) -> u32 {
        let s = &mut self.state;
        let s5 = s[5];
        let u = s[4].wrapping_add(s5);
        let c1 = if u < s5 || u < s[4] { 1 } else { 0 };
        s[4] = u;
        let u2 = u.wrapping_add(c1).wrapping_add(s[3]);
        let c2 = if u2 < s[3] { 1 } else { 0 };
        s[3] = u2;
        let u3 = u2.wrapping_add(c2).wrapping_add(s[2]);
        let c3 = if u3 < s[2] { 1 } else { 0 };
        s[2] = u3;
        let u4 = u3.wrapping_add(c3).wrapping_add(s[1]);
        let c4 = if u4 < s[1] { 1 } else { 0 };
        s[1] = u4;
        s[0] = s[0].wrapping_add(u4).wrapping_add(c4); // 633f10 add eax,edx; 633f12
        s[5] = s5.wrapping_add(1);
        if s[5] == 0 {
            s[4] = s[4].wrapping_add(1);
            if s[4] == 0 {
                s[3] = s[3].wrapping_add(1);
                if s[3] == 0 {
                    s[2] = s[2].wrapping_add(1);
                    if s[2] == 0 {
                        s[1] = s[1].wrapping_add(1);
                        if s[1] == 0 {
                            s[0] = s[0].wrapping_add(1);
                        }
                    }
                }
            }
        }
        s[0]
    }
}

/// 0x0063404a `RandomInt(min,max)` on the client instance (`0x00dba3a0`);
/// 0x00633ff4 is the same algorithm on the logic instance with a debug log
/// (`s_logicrandom` via the optional debug file at `0x00dfeff0`, omitted —
/// never active outside debug builds).
pub fn random_int(rng: &mut RandomGenerator, min: i32, max: i32) -> i32 {
    let span = (max.wrapping_sub(min) as u32).wrapping_add(1);
    if span == 0 {
        return max;
    }
    let r = rng.next();
    (r % span).wrapping_add(min as u32) as i32
}
pub fn random_float(rng: &mut RandomGenerator, min: f32, max: f32) -> f32 {
    let span = max - min;
    // comiss semantics: generator runs unless 0 >= span (NaN span runs it too).
    if span <= 0.0 {
        return max;
    }
    let r = rng.next();
    let rs = r as i64 - ((r as i64) >> 31 & 0x1_0000_0000); // signed int value
    let mut fr = rs as f64;
    if rs < 0 {
        fr += NEG_ADJUST as f64;
    }
    ((fr * RANDOM_SCALE as f64) * span as f64 + min as f64) as f32
}

/// `GameClientRandomVariable` — {dist, min, max} at +0/+4/+8.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GameClientRandomVariable {
    pub distribution: DistributionType,
    pub min: f32,
    pub max: f32,
}

/// BFME2 `GameClientRandomVariable::DistributionType` — name list
/// `[UNIFORM, GAUSSIAN, TRIANGULAR, LOW_BIAS, HIGH_BIAS]`
/// (@0x00db9bec / strings 0x00be8404/3f8/3ec/3e0/3d4). `getValue`
/// (0x006341a1) switches: 0 → uniform body, 1 → same random body
/// (helper 0x00634111, debug args line 0x18a RandomValue.cpp),
/// everything else (incl. 2..4 and negatives) → `DAT_00bbaeac` = 0.0f.
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DistributionType {
    Uniform = 0,
    Gaussian = 1,
    Triangular = 2,
    LowBias = 3,
    HighBias = 4,
    /// Any other stored value — `getValue` returns 0.0f.
    Invalid = -1,
}

impl GameClientRandomVariable {
    /// 0x004b4979 default ctor (dist 0, min 0, max 0).
    pub fn new() -> Self {
        GameClientRandomVariable {
            distribution: DistributionType::Uniform,
            min: 0.0,
            max: 0.0,
        }
    }

    /// 0x006341e7 `setRange(min, max, dist)`
    pub fn set_range(&mut self, min: f32, max: f32, dist: DistributionType) {
        self.min = min;
        self.max = max;
        self.distribution = dist;
    }

    /// 0x00852d82 / 0x008f5f92 / 0x00821795 accessors
    pub fn get_minimum_value(&self) -> f32 {
        self.min
    }
    pub fn get_maximum_value(&self) -> f32 {
        self.max
    }
    pub fn get_distribution_type(&self) -> DistributionType {
        self.distribution
    }

    /// 0x006341a1 `getValue` on the client instance: dist 0 with
    /// min == max → min (unordered pair falls through to the random
    /// body too); dist 0 or 1 → uniform float body (0x00634111);
    /// dist ≥ 2 (TRIANGULAR/LOW_BIAS/HIGH_BIAS) or negative →
    /// `DAT_00bbaeac` (0.0f).
    pub fn get_value(&mut self, rng: &mut RandomGenerator) -> f32 {
        match self.distribution {
            DistributionType::Uniform if self.min == self.max => self.min,
            DistributionType::Uniform | DistributionType::Gaussian => {
                random_float(rng, self.min, self.max)
            }
            _ => 0.0,
        }
    }
}

impl Default for GameClientRandomVariable {
    fn default() -> Self {
        Self::new()
    }
}

/// 0x00706183 — `xfer(GameClientRandomVariable&)` composite (FX/Behavior
/// stores). `Version1` gate, then min/max as `'real'` and the distribution
/// as `'enu4'` (label `GameClientRandomVariable::DistributionType`,
/// `FUN_00705f92`). On the light-CRC pass the game transfers a *local copy*
/// and never calls `setRange` — fields untouched; the normal pass transfers
/// zero-initialised locals and applies `setRange(min,max,(DistributionType)
/// enum-int)`; ints outside {0,1} land on `Invalid` (game stores the raw
/// int; `getValue` yields 0.0 for it either way).
pub fn xfer_game_client_random_variable<X: sage_xfer::Xfer>(
    x: &mut X,
    v: &mut GameClientRandomVariable,
) {
    x.version1();
    let light = x.is_light_crc();
    let (mut lo, mut hi) = (0.0f32, 0.0f32);
    let mut dist_i: i32 = 0;
    if light {
        lo = v.min;
        hi = v.max;
        dist_i = match v.distribution {
            DistributionType::Uniform => 0,
            DistributionType::Gaussian => 1,
            DistributionType::Triangular => 2,
            DistributionType::LowBias => 3,
            DistributionType::HighBias => 4,
            DistributionType::Invalid => -1,
        };
    }
    x.xfer_f32(&mut lo);
    x.xfer_f32(&mut hi);
    let mut db = dist_i.to_le_bytes();
    x.xfer_enum_named("GameClientRandomVariable::DistributionType", &mut db);
    if !light {
        let d = match i32::from_le_bytes(db) {
            0 => DistributionType::Uniform,
            1 => DistributionType::Gaussian,
            2 => DistributionType::Triangular,
            3 => DistributionType::LowBias,
            4 => DistributionType::HighBias,
            _ => DistributionType::Invalid,
        };
        v.set_range(lo, hi, d);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() <= eps
    }

    #[test]
    fn generator_matches_golden_vectors() {
        // Golden vectors simulated from the instruction stream at 0x00633ec3
        // (ground truth = machine code, not this file).
        let cases: [(u32, [u32; 6]); 3] = [
            (
                0,
                [
                    0x559a51ed, 0x274ea7f5, 0xe827f7e0, 0x76b65632, 0x7f1b3feb, 0xeae8d5ee,
                ],
            ),
            (
                12345,
                [
                    0x559b7343, 0x27529ca2, 0xe8328458, 0x76ce1240, 0x7f4ab807, 0xeb3fdccc,
                ],
            ),
            (
                0xdeadbeef,
                [
                    0x8daccb87, 0x6b8f5191, 0x9e29bc2a, 0x103a4fd5, 0xb223332d, 0xc877693a,
                ],
            ),
        ];
        for (seed, expect) in cases {
            let mut g = RandomGenerator::seed(seed);
            for e in expect {
                assert_eq!(g.next(), e, "seed {seed}");
            }
        }
        // 300th draw off seed 0xC0FFEE exercises the carry chain across
        // wraps; simulated value 0xd3632570.
        let mut g = RandomGenerator::seed(0xC0FFEE);
        for _ in 0..299 {
            g.next();
        }
        assert_eq!(g.next(), 0xd363_2570);
    }

    #[test]
    fn float_matches_golden_vectors() {
        let mut g = RandomGenerator::seed(12345);
        let v = random_float(&mut g, -1.0, 1.0);
        assert!(close(v, -0.331_193_54, 1e-6), "{v}");
        let mut g = RandomGenerator::seed(12345);
        let v = random_float(&mut g, 0.0, 100.0);
        assert!(close(v, 33.440_323, 1e-5), "{v}");
    }

    #[test]
    fn int_range_vectors_and_bounds() {
        let mut g = RandomGenerator::seed(999);
        let got = [(); 5].map(|_| random_int(&mut g, 1, 10));
        assert_eq!(got, [2, 9, 7, 1, 6]);
        // span overflow: (max-min)+1 == 0 → returns max
        let mut g = RandomGenerator::seed(1);
        assert_eq!(random_int(&mut g, i32::MIN, i32::MAX), i32::MAX);
        // in range over many draws
        let mut g = RandomGenerator::seed(7);
        for _ in 0..1000 {
            let v = random_int(&mut g, -5, 5);
            assert!((-5..=5).contains(&v));
        }
    }

    #[test]
    fn float_edge_cases() {
        let mut g = RandomGenerator::seed(3);
        // degenerate span → returns max, generator NOT advanced
        let g2 = g;
        assert_eq!(random_float(&mut g, 5.0, 5.0), 5.0);
        assert_eq!(g2.state, g.state);
        // descending span → max
        assert_eq!(random_float(&mut g, 4.0, 2.0), 2.0);
        // span always inside [min,max]
        for _ in 0..1000 {
            let v = random_float(&mut g, -2.0, 3.0);
            assert!((-2.0..=3.0).contains(&v));
        }
    }

    #[test]
    fn game_client_random_variable_semantics() {
        let mut g = RandomGenerator::seed(0);
        let mut v = GameClientRandomVariable::new();
        assert_eq!(v.min, 0.0);
        assert_eq!(v.max, 0.0);
        assert_eq!(v.get_distribution_type(), DistributionType::Uniform);
        // degenerate uniform → min without advancing rng
        v.set_range(4.0, 4.0, DistributionType::Uniform);
        let mut g0 = g;
        assert_eq!(v.get_value(&mut g0), 4.0);
        assert_eq!(g0.state, g.state);
        // dist 1 = GAUSSIAN: shipped helper body is the plain uniform
        // float draw (0x00634111); dist >= 2 would return 0.0 instead
        v.set_range(0.0, 1.0, DistributionType::Gaussian);
        let t = v.get_value(&mut g);
        assert!((0.0..=1.0).contains(&t));
        // invalid dist → 0.0
        v.distribution = DistributionType::Invalid;
        assert_eq!(v.get_value(&mut g), 0.0);
    }

    #[test]
    fn game_client_random_variable_equality() {
        // 0x00405568 operator== compares {dist int, min, max} field-wise;
        // 0x0040559b operator!= is `1 - (a==b)` — folded into derive.
        let a = GameClientRandomVariable {
            distribution: DistributionType::Uniform,
            min: 1.0,
            max: 2.0,
        };
        let b = GameClientRandomVariable {
            distribution: DistributionType::Uniform,
            min: 1.0,
            max: 2.0,
        };
        assert_eq!(a, b);
        assert_ne!(
            a,
            GameClientRandomVariable {
                distribution: DistributionType::Gaussian,
                min: 1.0,
                max: 2.0
            }
        );
        assert_ne!(
            a,
            GameClientRandomVariable {
                distribution: DistributionType::Uniform,
                min: f32::NAN,
                max: 2.0
            }
        );
        // NaN field: game's float == is unordered-false, derive matches
        assert_ne!(
            GameClientRandomVariable {
                distribution: DistributionType::Uniform,
                min: f32::NAN,
                max: 0.0
            },
            GameClientRandomVariable {
                distribution: DistributionType::Uniform,
                min: f32::NAN,
                max: 0.0
            }
        );
    }

    // 0x00706183 xfer composite
    struct Rec {
        ev: Vec<(u32, Vec<u8>)>,
        light: bool,
        stream: Vec<Vec<u8>>, // per-call inbound bytes (load semantics)
    }
    impl sage_xfer::Xfer for Rec {
        fn xfer_internal(&mut self, tag: u32, value: &mut [u8]) {
            let inb = self.stream.remove(0);
            self.ev.push((tag, value.to_vec())); // what the caller pushed
            value.copy_from_slice(&inb);
        }
        fn is_light_crc(&self) -> bool {
            self.light
        }
    }

    #[test]
    fn gcrv_xfer_composite() {
        use sage_xfer::{TAG_ENU, TAG_REAL, TAG_VER};
        let mut v = GameClientRandomVariable::new();
        let mut r = Rec {
            ev: vec![],
            light: false,
            stream: vec![
                vec![1], // Version.current = 1 (u8)
                3.5f32.to_le_bytes().to_vec(),
                9.0f32.to_le_bytes().to_vec(),
                1i32.to_le_bytes().to_vec(),
            ],
        };
        xfer_game_client_random_variable(&mut r, &mut v);
        let tags: Vec<u32> = r.ev.iter().map(|e| e.0).collect();
        assert_eq!(tags, vec![TAG_VER, TAG_REAL, TAG_REAL, TAG_ENU + 4]);
        assert_eq!(v.min, 3.5);
        assert_eq!(v.max, 9.0);
        assert_eq!(v.distribution, DistributionType::Gaussian);

        // out-of-range dist int -> Invalid (game: raw store, getValue 0.0)
        let mut v2 = GameClientRandomVariable::new();
        let mut r2 = Rec {
            ev: vec![],
            light: false,
            stream: vec![
                vec![1],
                0.0f32.to_le_bytes().to_vec(),
                1.0f32.to_le_bytes().to_vec(),
                7i32.to_le_bytes().to_vec(),
            ],
        };
        xfer_game_client_random_variable(&mut r2, &mut v2);
        assert_eq!(v2.distribution, DistributionType::Invalid);
        assert_eq!(v2.get_value(&mut RandomGenerator::seed(1)), 0.0);

        // light-CRC pass: transfers current values, fields NOT written back
        let mut v3 = GameClientRandomVariable {
            distribution: DistributionType::Uniform,
            min: 2.0,
            max: 4.0,
        };
        let mut r3 = Rec {
            ev: vec![],
            light: true,
            stream: vec![
                vec![1],
                8.0f32.to_le_bytes().to_vec(),
                9.0f32.to_le_bytes().to_vec(),
                3i32.to_le_bytes().to_vec(),
            ],
        };
        xfer_game_client_random_variable(&mut r3, &mut v3);
        assert_eq!((v3.min, v3.max), (2.0, 4.0));
        let transferred: Vec<Vec<u8>> = r3.ev.iter().skip(1).map(|e| e.1.clone()).collect();
        assert_eq!(transferred[0], 2.0f32.to_le_bytes().to_vec());
    }
}
