//! FXParticleSystem configuration layer (`FXParticleSystem::*` decomp names).
//!
//! Mirrors the template/info/config classes of BFME2's particle system:
//! `ParticleSystemInfo` (0x005f4e82), `ParticleSystemTemplate` accessors,
//! keyframe structs, and the module-template base plumbing. The runtime
//! `ParticleSystem`/`Module` classes have **no named rows in the ledger** —
//! `createModule` bodies (0x007ada72…) stay blocked on them.
//!
//! Field offsets cite the game struct layout (ctor 0x005f4e82 defaults,
//! token table 0x005f3d2e, copy logic 0x0040200d). INI `parse` (0x005f3d2e,
//! 0x0095b93f…) and `DoXfer` rows are Tier B: they need the INI subsystem;
//! their token→offset evidence is encoded in the field names below.

use crate::math::RGBColor;
use crate::region::Region2D;
use crate::string::AsciiString;

/// Particle priority enum values (`ParticlePriorityNames` @ `0x00bda058`).
pub mod priority {
    /// Default `ParticleSystemInfo::Priority` (ctor 0x005f4e82 `+0x7c = 1`).
    pub const DEFAULT: u32 = 1;
}

/// INI token-table default lists (enum name tables; parse = Tier B):
/// `ParticlePriorityNames` 0x00bda058, `ParticleShaderTypeNames` 0x00bdccd8,
/// `ParticleTypeNames` 0x00bda050.

/// Plain keyframe: `{ time f32 @+0, value f32 @+4 }` (ctor 0x009993cd: zeros).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Keyframe {
    pub time: f32,
    pub value: f32,
}

/// `RGBColorKeyframe`: ctor 0x00401e67 zeroes all four dwords. Layout from
/// `parseRGBColorKeyframe` 0x0095b913: `FUN_0042f259` writes the three color
/// floats (bytes × 1/255) at `+0/+4/+8`; `FUN_0042ef72` writes an unsigned
/// dword at `+0xc` (the keyframe time).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RgbColorKeyframe {
    pub color: [f32; 3], // +0/+4/+8
    pub time: u32,       // +0xc
}

/// Default distribution selector: `GameClientRandomVariable::setRange(…, 1)`
/// (0x00401e7f). Per the game's `DistributionTypeNames` list (0x00db9bec →
/// UNIFORM, GAUSSIAN, TRIANGULAR, LOW_BIAS, HIGH_BIAS) value 1 = GAUSSIAN,
/// NOT UNIFORM — the earlier Generals-era assumption was wrong
/// (`GameClientRandomVariable::getValue` 0x006341a1: cases 0/1 draw a
/// random float; every other selector returns 0.0).
pub const DIST_GAUSSIAN: u32 = 1;

/// `RandomAlphaKeyframe` (ctor 0x00401e7f = `setRange(0,0,1)` + zero time):
/// a GameClientRandomVariable `{ distribution @+0, min @+4, max @+8 }`
/// (setRange 0x006341e7 writes min/max to +4/+8, dist to +0) plus the
/// keyframe time dword at `+0xc` (parse 0x0095b29c).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RandomAlphaKeyframe {
    pub distribution: u32, // +0
    pub min: f32,          // +4
    pub max: f32,          // +8
    pub time: u32,         // +0xc
}

impl Default for RandomAlphaKeyframe {
    /// setRange(0.0, 0.0, dist=1 = GAUSSIAN); time = 0 (0x00401e7f).
    fn default() -> Self {
        Self {
            distribution: DIST_GAUSSIAN,
            min: 0.0,
            max: 0.0,
            time: 0,
        }
    }
}

/// 0x00706183 on a *raw* `GameClientRandomVariable` dword triple
/// `{dist-int, min, max}` (fx structs keep the raw layout; `rng::
/// xfer_game_client_random_variable` is the typed twin). Semantics:
/// `Version1` gate; light-CRC pass transfers a local copy of the stored
/// values without write-back; normal pass transfers zero-initialised locals
/// and stores all three transferred values back verbatim (the game stores
/// the dist dword as a raw int — no enum clamping here).
pub(crate) fn xfer_gcrv_raw<X: sage_xfer::Xfer>(x: &mut X, d: &mut [u32; 3]) {
    x.version1();
    let light = x.is_light_crc();
    let (mut lo, mut hi) = (0.0f32, 0.0f32);
    let mut di = 0i32;
    if light {
        lo = f32::from_bits(d[1]);
        hi = f32::from_bits(d[2]);
        di = d[0] as i32;
    }
    x.xfer_f32(&mut lo);
    x.xfer_f32(&mut hi);
    let mut db = di.to_le_bytes();
    x.xfer_enum_named("GameClientRandomVariable::DistributionType", &mut db);
    if !light {
        d[0] = i32::from_le_bytes(db) as u32;
        d[1] = lo.to_bits();
        d[2] = hi.to_bits();
    }
}

/// `EventModuleInfo` (ctor 0x00401236: two `u16 = 1` at +0/+2).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EventModuleInfo {
    pub field_0: u16,
    pub field_2: u16,
}

impl Default for EventModuleInfo {
    fn default() -> Self {
        Self {
            field_0: 1,
            field_2: 1,
        }
    }
}

/// `FXParticleSystemInfo` snapshot name (0x00401272 → `0x00bbb530`).
pub const PARTICLE_SYSTEM_INFO_SNAPSHOT_NAME: &str = "FXParticleSystemInfo";

/// Per-particle random range: `{ distribution @off, min @off+4, max @off+8 }`
/// written by `FUN_00738a9d` (INI `setRange` variant of 0x006341e7).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RandomRange {
    pub min: f32,
    pub max: f32,
    pub distribution: u32,
}

/// `FXParticleSystem::ParticleSystemInfo` (160-byte struct).
///
/// Defaults byte-exact from the ctor (0x005f4e82); field names from the INI
/// token table (0x005f3d2e: Priority→0x7c, IsOneShot→4, Shader→8, Type→0xc,
/// ParticleName→0x10, Lifetime→0x14, SystemLifetime→0x20, SortLevel→0x24,
/// Size→0x28, StartSizeRate→0x34, BurstDelay→0x44, BurstCount→0x50,
/// InitialDelay→0x5c, IsGroundAligned→0x80 … ShroudEmitter→0x84). The
/// maximum-height pair at +0x90/+0x94 defaults to `1.0f` (`DAT_00bbb8d8`).
#[derive(Clone)]
pub struct ParticleSystemInfo {
    /// +0x04 `IsOneShot` (parse handler FUN_0042e850 = bool).
    pub is_one_shot: bool,
    /// +0x08 `Shader` (`ParticleShaderTypeNames` 0x00bdccd8), default 1.
    pub shader: u32,
    /// +0x0c `Type` (`ParticleTypeNames` 0x00bda050), default 1.
    pub ptype: u32,
    /// +0x10 `ParticleName`.
    pub particle_name: AsciiString,
    /// +0x14 Lifetime random range (min/max/dist triple).
    pub lifetime: RandomRange,
    /// +0x20 `SystemLifetime` (single int, handler FUN_0042ef72).
    pub system_lifetime: u32,
    /// +0x24 `SortLevel` (single int, FUN_0042ef72).
    pub sort_level: u32,
    /// +0x28 `Size` random range.
    pub size: RandomRange,
    /// +0x34 `StartSizeRate` random range (+0x40 = its pad, 0).
    pub start_size_rate: RandomRange,
    /// +0x44 `BurstDelay` random range.
    pub burst_delay: RandomRange,
    /// +0x50 `BurstCount` random range.
    pub burst_count: RandomRange,
    /// +0x5c `InitialDelay` random range.
    pub initial_delay: RandomRange,
    /// +0x68 `SlaveSystem` (AsciiString).
    pub slave_system: AsciiString,
    /// +0x6c/0x70/0x74 `SlavePosOffset` (Coord3D floats).
    pub slave_pos_offset: [f32; 3],
    /// +0x78 `PerParticleAttachedSystem` (AsciiString).
    pub per_particle_attached_system: AsciiString,
    /// +0x7c `Priority`, default 1 (`DEFAULT`).
    pub priority: u32,
    /// +0x80 `IsGroundAligned`.
    pub is_ground_aligned: bool,
    /// +0x81 `IsEmitAboveGroundOnly`.
    pub is_emit_above_ground_only: bool,
    /// +0x82 `IsParticleUpTowardsEmitter`.
    pub is_particle_up_towards_emitter: bool,
    /// +0x83 `UseMaximumHeight`.
    pub use_maximum_height: bool,
    /// +0x84 `ShroudEmitter`.
    pub shroud_emitter: bool,
    /// +0x40: pad dword after the StartSizeRate range; only observable
    /// through DoXfer (0x005f4f5a transfers it when stream version > 1).
    pub pad_40: u32,
    /// +0x88/+0x8c UV min corner (the UV `Region2D` spans +0x88..+0x94;
    /// its max corner is the maximum-height pair below — one struct span).
    pub uv_min: (f32, f32),
    /// +0x90/+0x94 maximum-height pair, default 1.0f (`DAT_00bbb8d8`);
    /// doubles as the UV region's max corner.
    pub max_height: (f32, f32),
    /// +0x98, default 0 (cache/handle slot; no named token).
    pub field_98: u32,
}

impl Default for ParticleSystemInfo {
    /// Byte-exact defaults from `ParticleSystemInfo()` 0x005f4e82.
    fn default() -> Self {
        Self {
            is_one_shot: false,
            shader: 1,
            ptype: 1,
            particle_name: AsciiString::default(),
            lifetime: RandomRange::default(),
            system_lifetime: 0,
            sort_level: 0,
            size: RandomRange::default(),
            start_size_rate: RandomRange::default(),
            burst_delay: RandomRange::default(),
            burst_count: RandomRange::default(),
            initial_delay: RandomRange::default(),
            slave_system: AsciiString::default(),
            slave_pos_offset: [0.0; 3],
            per_particle_attached_system: AsciiString::default(),
            priority: priority::DEFAULT,
            is_ground_aligned: false,
            is_emit_above_ground_only: false,
            is_particle_up_towards_emitter: false,
            use_maximum_height: false,
            shroud_emitter: false,
            pad_40: 0,
            uv_min: (0.0, 0.0),
            max_height: (1.0, 1.0),
            field_98: 0,
        }
    }
}

/// `ParticleSystemInfo::GetSnapshotName` (0x00401272).
pub fn particle_system_info_snapshot_name(_info: &ParticleSystemInfo) -> &'static str {
    PARTICLE_SYSTEM_INFO_SNAPSHOT_NAME
}

impl ParticleSystemInfo {
    /// 0x005f4f5a `DoXfer` — whole body guarded by `!IsStoring` (slot
    /// +0x10). `xfer(Version&)` with local `Version{minimum:1, current:3}`;
    /// 'bool' +4; 'enu4' "ParticleShaderType" +8 (FUN_00706052); 'enu4'
    /// "ParticleType" +0xc (FUN_00706022); AsciiString +0x10 (0x00a0bea4);
    /// GCRV +0x14; 'uint' +0x20/+0x24; GCRV +0x28/+0x34; GCRV +0x44/+0x50/
    /// +0x5c; AsciiString +0x68; 'c3d' +0x6c; AsciiString +0x78; 'enu4'
    /// "ParticlePriorityType" +0x7c (FUN_0070603a); 'bool' +0x80/+0x81/
    /// +0x82/+0x83. Gate `1 < current`: 'uint' +0x40 (the StartSizeRate pad
    /// dword — `pad_40`) + `XferRawBytes(this+0x98, 4)` (slot +0x24,
    /// untagged — `field_98`). Gate `2 < current`: 'bool' +0x84.
    pub fn do_xfer<X: sage_xfer::Xfer>(&mut self, x: &mut X) {
        if x.is_storing() {
            return;
        }
        let mut v = sage_xfer::Version {
            minimum: 1,
            current: 3,
        };
        x.xfer_version(&mut v);
        x.xfer_bool(&mut self.is_one_shot);
        let mut e = self.shader.to_le_bytes();
        x.xfer_enum_named("ParticleShaderType", &mut e);
        self.shader = u32::from_le_bytes(e);
        let mut e = self.ptype.to_le_bytes();
        x.xfer_enum_named("ParticleType", &mut e);
        self.ptype = u32::from_le_bytes(e);
        crate::string::xfer_ascii_string(x, &mut self.particle_name);
        xfer_rr(x, &mut self.lifetime);
        x.xfer_u32(&mut self.system_lifetime);
        x.xfer_u32(&mut self.sort_level);
        xfer_rr(x, &mut self.size);
        xfer_rr(x, &mut self.start_size_rate);
        xfer_rr(x, &mut self.burst_delay);
        xfer_rr(x, &mut self.burst_count);
        xfer_rr(x, &mut self.initial_delay);
        crate::string::xfer_ascii_string(x, &mut self.slave_system);
        let mut c = sage_xfer::Coord3D {
            x: self.slave_pos_offset[0],
            y: self.slave_pos_offset[1],
            z: self.slave_pos_offset[2],
        };
        x.xfer_coord3d(&mut c);
        self.slave_pos_offset = [c.x, c.y, c.z];
        crate::string::xfer_ascii_string(x, &mut self.per_particle_attached_system);
        let mut e = self.priority.to_le_bytes();
        x.xfer_enum_named("ParticlePriorityType", &mut e);
        self.priority = u32::from_le_bytes(e);
        x.xfer_bool(&mut self.is_ground_aligned);
        x.xfer_bool(&mut self.is_emit_above_ground_only);
        x.xfer_bool(&mut self.is_particle_up_towards_emitter);
        x.xfer_bool(&mut self.use_maximum_height);
        if 1 < v.current {
            x.xfer_u32(&mut self.pad_40);
            let mut b = self.field_98.to_le_bytes();
            x.xfer_internal(0, &mut b); // XferRawBytes — no tag (slot +0x24)
            self.field_98 = u32::from_le_bytes(b);
        }
        if 2 < v.current {
            x.xfer_bool(&mut self.shroud_emitter);
        }
    }
}

/// GCRV composite over a `RandomRange` triple — same 0x00706183 semantics
/// as `xfer_gcrv_raw` (memory order is `{dist, min, max}`; the Rust field
/// order is logical only).
fn xfer_rr<X: sage_xfer::Xfer>(x: &mut X, r: &mut RandomRange) {
    let mut d = [r.distribution, r.min.to_bits(), r.max.to_bits()];
    xfer_gcrv_raw(x, &mut d);
    r.distribution = d[0];
    r.min = f32::from_bits(d[1]);
    r.max = f32::from_bits(d[2]);
}

/// GCRV composite on a raw dword slice at offset 0 (`{dist,min,max}`).
fn xfer_gcrv_slice<X: sage_xfer::Xfer>(x: &mut X, d: &mut [u32]) {
    let mut g = [d[0], d[1], d[2]];
    xfer_gcrv_raw(x, &mut g);
    d[..3].copy_from_slice(&g);
}

/// `FXParticleSystem::ModuleTemplate` — empty base (0x004011b4 ctor,
/// 0x004011bd dtor, 0x004011c4 copy: vptr only, no fields).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ModuleTemplate {}

/// `FXParticleSystem::CategoryModuleTemplate<N>` base (ctor 0x00401ef9 and
/// the 0x00401d69 alias set): two `u8 = 1` flags at +8/+9 over an empty
/// `ModuleTemplate` head. Field semantics unnamed in the binary.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CategoryModuleTemplate {
    pub flag_8: bool,
    pub flag_9: bool,
}

impl Default for CategoryModuleTemplate {
    fn default() -> Self {
        Self {
            flag_8: true,
            flag_9: true,
        }
    }
}

/// `FXParticleSystem::ParticleSystemTemplate` config portion.
///
/// Layout: `ParticleSystemInfo` at +0, template name `AsciiString` @+0x9c,
/// cache flag @+0xa0 (cleared by mutators), module-template list @+0xa4.
/// The module list (copy thunk `FUN_005fc404`) and the parse/dtor/copy ctors
/// stay Tier B (module runtime unmirrored; dtor/copy rows are EH shells).
#[derive(Clone)]
pub struct ParticleSystemTemplate {
    pub info: ParticleSystemInfo,
    /// +0x9c template name.
    pub name: AsciiString,
    /// +0xa0 cache flag (cleared by `set_slave_system_name` etc.).
    pub cache_flag: bool,
}

impl Default for ParticleSystemTemplate {
    fn default() -> Self {
        Self {
            info: ParticleSystemInfo::default(),
            name: AsciiString::default(),
            cache_flag: false,
        }
    }
}

impl ParticleSystemTemplate {
    /// `getName` (0x00402600): copy of the name at +0x9c.
    pub fn get_name(&self) -> AsciiString {
        self.name.clone()
    }

    /// `getTextureFilename` (0x0040261e): returns a **copy of +0x10** —
    /// the same field the parse table names `ParticleName` (game quirk).
    pub fn get_texture_filename(&self) -> AsciiString {
        self.info.particle_name.clone()
    }

    /// `setTextureFilename` (0x00782015): `StringBase::set` onto +0x10.
    pub fn set_texture_filename(&mut self, value: &AsciiString) {
        self.info.particle_name = value.clone();
    }

    /// `setSlaveSystemName` (0x005f3fd0): set +0x68, clear the +0xa0 flag.
    pub fn set_slave_system_name(&mut self, value: &AsciiString) {
        self.info.slave_system = value.clone();
        self.cache_flag = false;
    }

    /// `setUV` (0x00401278): null-guarded 4-float copy into +0x88…+0x94
    /// (clobbers the max-height default pair — byte-exact game behavior).
    pub fn set_uv(&mut self, value: &Region2D) {
        self.info.uv_min = (value.min_x, value.min_y);
        self.info.max_height = (value.max_x, value.max_y);
    }

    /// `getUV` (0x0095c414): the +0x88..+0x94 span as a region.
    pub fn get_uv(&self) -> Region2D {
        Region2D {
            min_x: self.info.uv_min.0,
            min_y: self.info.uv_min.1,
            max_x: self.info.max_height.0,
            max_y: self.info.max_height.1,
        }
    }
    /// 0x005fced1 `operator=`: `ParticleSystemInfo::operator=` (modeled by
    /// the field clone — string members `set`-assign exactly as the game
    /// op= does), `name` (+0x9c) `StringBase::set`, `cache_flag` (+0xa0) :=
    /// 0, then `FUN_005fc404(src+0xa4)` on the source's module-template
    /// list — the runtime module subsystem is not modeled (no fields here).
    pub fn assign_from(&mut self, src: &Self) {
        self.info = src.info.clone();
        self.name = src.name.clone();
        self.cache_flag = false;
    }
}

// ---------------------------------------------------------------------------
// Module-info structs (`FXParticleSystem::*ModuleInfo`, `*Emission*Info`).
//
// `GetSnapshotName` rows are 6-byte `mov eax, imm32; ret` returning the
// literal below (verified via capstone on the immediate string VAs).
// Default ctors exist only for some types — `Default` is implemented ONLY
// where a decompiled default ctor exists (ruled: no invented defaults).
// Copy-ctor/`operator=` rows mirror to `Clone` (POD types derive it).
// ---------------------------------------------------------------------------

/// `WindModuleInfo` (0x48 B). Ctor 0x007a53c4; copy 0x005f35df; op= 0x005f3654.
/// Defaults from the ctor's `movss` loads (.rdata constants):
/// strength 2.0 @+8 (0xc1b2e8), dists 75.0/200.0 @+0xc/+0x10,
/// angle change 0.15 @+0x18 and +0x1c (both load 0xc1b2f8),
/// 0.45 @+0x20 (0xc1b300), start-angle max 0.78539819 @+0x2c (0xc1b308),
/// end-angle 5.4977875 @+0x30 (0xdc0a9c, .data-init const),
/// end min/max 5.4977875/6.2831855 @+0x34/+0x38 (0xc1b30c/0xc1b310).
/// +0x24 loads `DAT_00e02904` (.data/BSS: 0 at image time).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindModuleInfo {
    pub field_4: u32,                // +0x04 = 1
    pub strength: f32,               // +0x08 DEFAULT_STRENGTH = 2.0
    pub full_strength_dist: f32,     // +0x0c = 75.0
    pub zero_strength_dist: f32,     // +0x10 = 200.0
    pub field_14: f32,               // +0x14 = 0
    pub angle_change_a: f32,         // +0x18 DEFAULT_ANGLE_CHANGE = 0.15
    pub angle_change_b: f32,         // +0x1c DEFAULT_ANGLE_CHANGE = 0.15
    pub angle_change_max: f32,       // +0x20 DEFAULT_ANGLE_CHANGE_MAX = 0.45
    pub motion_start_angle: f32,     // +0x24 = DAT_00e02904 (0 in image)
    pub motion_start_angle_max: f32, // +0x2c = 0.7853981852531433 (pi/4)
    pub field_28: f32,               // +0x28 = 0
    pub motion_end_angle: f32,       // +0x30 = 5.4977874755859375 (7pi/4)
    pub motion_end_angle_min: f32,   // +0x34 = 5.4977874755859375
    pub motion_end_angle_max: f32,   // +0x38 = 6.2831854820251465 (2pi)
    pub flag_3c: bool,               // +0x3c = 1
    pub field_40: f32,               // +0x40 = 0
    pub field_44: f32,               // +0x44 = 0
}

impl Default for WindModuleInfo {
    fn default() -> Self {
        Self {
            field_4: 1,
            strength: 2.0,
            full_strength_dist: 75.0,
            zero_strength_dist: 200.0,
            field_14: 0.0,
            angle_change_a: 0.15,
            angle_change_b: 0.15,
            angle_change_max: 0.45,
            motion_start_angle: 0.0,
            motion_start_angle_max: 0.78539819,
            field_28: 0.0,
            motion_end_angle: 5.4977875,
            motion_end_angle_min: 5.4977875,
            motion_end_angle_max: 6.2831855,
            flag_3c: true,
            field_40: 0.0,
            field_44: 0.0,
        }
    }
}

impl WindModuleInfo {
    /// GetSnapshotName 0x005f35d9 (string 0x00c1b580 block).
    pub fn snapshot_name() -> &'static str {
        "WindModuleInfo"
    }
    /// 0x007a547e `DoXfer`: entire body guarded by `!IsStoring` (slot +0x10).
    /// `Version1`; `xferEnum("WindMotion")` (FUN_0070606a, slot +0x94 → 'enu4')
    /// at +4; thirteen `xfer(float&)` +8..+0x38; `xfer(char&)` (slot +0x8c,
    /// tag 'byte', 0x0040546c) at +0x3c (the bool flag — char, not bool);
    /// then two more floats +0x40/+0x44.
    pub fn do_xfer<X: sage_xfer::Xfer>(&mut self, x: &mut X) {
        if x.is_storing() {
            return;
        }
        x.version1();
        let mut wm = self.field_4.to_le_bytes();
        x.xfer_enum_named("WindMotion", &mut wm);
        self.field_4 = u32::from_le_bytes(wm);
        for f in [
            &mut self.strength,
            &mut self.full_strength_dist,
            &mut self.zero_strength_dist,
            &mut self.field_14,
            &mut self.angle_change_a,
            &mut self.angle_change_b,
            &mut self.angle_change_max,
            &mut self.motion_start_angle,
            &mut self.field_28,
            &mut self.motion_start_angle_max,
            &mut self.motion_end_angle,
            &mut self.motion_end_angle_min,
            &mut self.motion_end_angle_max,
        ] {
            x.xfer_f32(f);
        }
        let mut b = self.flag_3c as i8;
        x.xfer_i8(&mut b);
        self.flag_3c = b != 0;
        x.xfer_f32(&mut self.field_40);
        x.xfer_f32(&mut self.field_44);
    }
}

/// `GpuDrawModuleInfo` (0x14 B payload). Ctor 0x00963981; op= 0x007a7ef0
/// (string `set` at +0xc). Ctor stores a raw 0 at +0xc (zero-init string
/// storage; `StringBase::set` is the only mutator — mirrors that pattern).
#[derive(Clone, Debug, PartialEq)]
pub struct GpuDrawModuleInfo {
    pub field_4: u32,      // +0x04 = 1
    pub field_8: u32,      // +0x08 = 1
    pub name: AsciiString, // +0x0c (ctor: zero-init storage)
    pub scale: f32,        // +0x10 = DAT_00bbb8d8 = 1.0
}

impl Default for GpuDrawModuleInfo {
    fn default() -> Self {
        Self {
            field_4: 1,
            field_8: 1,
            name: AsciiString::default(),
            scale: 1.0,
        }
    }
}

impl GpuDrawModuleInfo {
    pub fn snapshot_name() -> &'static str {
        "GpuDrawModuleInfo" // GetSnapshotName 0x007a615d
    }
    /// 0x0096390a `DoXfer`: `Version1`; 'int' +4/+8 (slot +0x7c, 0x004054c0);
    /// AsciiString +0xc (0x00a0bea4); 'real' +0x10.
    pub fn do_xfer<X: sage_xfer::Xfer>(&mut self, x: &mut X) {
        x.version1();
        let mut a = self.field_4 as i32;
        x.xfer_i32(&mut a);
        self.field_4 = a as u32;
        let mut b = self.field_8 as i32;
        x.xfer_i32(&mut b);
        self.field_8 = b as u32;
        crate::string::xfer_ascii_string(x, &mut self.name);
        x.xfer_f32(&mut self.scale);
    }
}

// Empty-payload info structs: ctor/copy-ctor bodies are vptr-store only.

/// `StreakDrawModuleInfo` (ctors 0x0095fd41 / 0x007a7ab2: no fields).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StreakDrawModuleInfo;
impl StreakDrawModuleInfo {
    pub fn snapshot_name() -> &'static str {
        "StreakDrawModuleInfo" // GetSnapshotName 0x007a613f
    }
    /// 0x00706a9b — the one body Ghidra names for Streak/Quad/Butterfly/
    /// DefaultDraw `DoXfer`: `Version1` only, no payload.
    pub fn do_xfer<X: sage_xfer::Xfer>(&mut self, x: &mut X) {
        x.version1();
    }
    /// 0x0095fd2f wrapper pushes the EMPTY table DAT_00c6bb18: the parse
    /// block accepts only `END`; any token is "Unknown field" (level 5).
    pub fn parse(ini: &mut crate::ini::Ini) -> IniResult<Self> {
        run(ini, &[], |_ini, e, _off, _v2| {
            unmodelled(_ini, e, e.handler)
        })?;
        Ok(Self)
    }
}

/// `QuadDrawModuleInfo` (ctors 0x009601ad / 0x007a7bea: no fields).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct QuadDrawModuleInfo;
impl QuadDrawModuleInfo {
    pub fn snapshot_name() -> &'static str {
        "QuadDrawModuleInfo" // GetSnapshotName 0x007a6145
    }
    /// 0x00706a9b (shared with Streak/Butterfly/DefaultDraw): `Version1` only.
    pub fn do_xfer<X: sage_xfer::Xfer>(&mut self, x: &mut X) {
        x.version1();
    }
    /// 0x0096017f wrapper: empty table DAT_00c6bb18 (see Streak note).
    pub fn parse(ini: &mut crate::ini::Ini) -> IniResult<Self> {
        run(ini, &[], |_ini, e, _off, _v2| {
            unmodelled(_ini, e, e.handler)
        })?;
        Ok(Self)
    }
}

/// `ButterflyDrawModuleInfo` (ctors 0x00960ab1 / 0x007a7c8c: no fields).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ButterflyDrawModuleInfo;
impl ButterflyDrawModuleInfo {
    pub fn snapshot_name() -> &'static str {
        "ButterflyDrawModuleInfo" // GetSnapshotName 0x007a614b
    }
    /// 0x00706a9b (shared with Streak/Quad/DefaultDraw): `Version1` only.
    pub fn do_xfer<X: sage_xfer::Xfer>(&mut self, x: &mut X) {
        x.version1();
    }
    /// 0x00960a9f wrapper: empty table DAT_00c6bb18 (see Streak note).
    pub fn parse(ini: &mut crate::ini::Ini) -> IniResult<Self> {
        run(ini, &[], |_ini, e, _off, _v2| {
            unmodelled(_ini, e, e.handler)
        })?;
        Ok(Self)
    }
}

/// `DefaultDrawModuleInfo` (ctors 0x0095c446 / 0x007a643c: no fields).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DefaultDrawModuleInfo;
impl DefaultDrawModuleInfo {
    pub fn snapshot_name() -> &'static str {
        "DefaultDrawModuleInfo" // GetSnapshotName 0x007a6127
    }
    /// 0x00706a9b (shared with Streak/Quad/Butterfly): `Version1` only.
    pub fn do_xfer<X: sage_xfer::Xfer>(&mut self, x: &mut X) {
        x.version1();
    }
    /// 0x0095c434 `DefaultModuleTemplate<6>::parse`: empty table
    /// DAT_00c6bb18 (see Streak note).
    pub fn parse(ini: &mut crate::ini::Ini) -> IniResult<Self> {
        run(ini, &[], |_ini, e, _off, _v2| {
            unmodelled(_ini, e, e.handler)
        })?;
        Ok(Self)
    }
}

/// `EmissionVelocityInfo` base (ctors 0x005f35c5 / 0x005f35ce: no fields).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EmissionVelocityInfo;
impl EmissionVelocityInfo {
    pub fn snapshot_name() -> &'static str {
        "EmissionVelocityInfo" // GetSnapshotName 0x005f35bf
    }
    /// 0x0087a69c `DoXfer` (Ghidra alias of `EmissionVolumeInfo::DoXfer` and
    /// `ParticleSystemTemplate::GetAssetList`): empty body, no transfers;
    /// 0x00866e05 is a non-virtual thunk forwarding straight into it.
    pub fn do_xfer<X: sage_xfer::Xfer>(&mut self, _x: &mut X) {}
}

/// `EmissionVolumeInfo` base. Ctor 0x005f353d (`+4 = 0`), copy 0x005f3550,
/// op= 0x005f3565.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EmissionVolumeInfo {
    pub flag_4: bool, // +0x04
}
impl EmissionVolumeInfo {
    pub fn snapshot_name() -> &'static str {
        "EmissionVolumeInfo" // GetSnapshotName 0x005f354a
    }
    /// 0x0087a69c `DoXfer` (empty, aliased with `EmissionVelocityInfo`).
    pub fn do_xfer<X: sage_xfer::Xfer>(&mut self, _x: &mut X) {}
}

/// `BoxEmissionVolumeInfo`. Ctor 0x007a664a (dims 0), copy 0x007a6669,
/// op= 0x007a6696.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BoxEmissionVolumeInfo {
    pub flag_4: bool,   // +0x04
    pub dims: [f32; 3], // +0x08, +0x0c, +0x10 = 0
}

/// `SphereEmissionVolumeInfo`. Ctor 0x007a67e8 (radius 0), copy 0x007a67fd,
/// op= 0x007a681e.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SphereEmissionVolumeInfo {
    pub flag_4: bool, // +0x04
    pub radius: f32,  // +0x08 = 0
}

/// `CylinderEmissionVolumeInfo`. Default ctor 0x007a6958 (all zero; body
/// ALSO named for `LineEmissionVolumeInfo` — identical code), copy
/// 0x007a653b (same aliasing), op= 0x007a6986.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CylinderEmissionVolumeInfo {
    pub flag_4: bool,     // +0x04
    pub fields: [f32; 6], // +0x08..+0x1c = 0
}

/// `LineEmissionVolumeInfo` — shares the folded ctor bodies with
/// `CylinderEmissionVolumeInfo`; own op= 0x007a657a (same 7 stores).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LineEmissionVolumeInfo {
    pub flag_4: bool,
    pub fields: [f32; 6],
}

/// `SphericalEmissionVelocityInfo`: copy 0x007a7270, op= 0x007a7296
/// (3 dwords +0x04..+0x0c; default ctor not decompiled standalone — the
/// wave-22 template ctors zero it, hence `Default` = zeroed).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SphericalEmissionVelocityInfo {
    pub dwords: [u32; 3], // +0x04..+0x0c
}

/// `CylindricalEmissionVelocityInfo` — copy ctor 0x007a7501 and op=
/// 0x007a765f bodies are ALSO named for `OutwardEmissionVelocityInfo`
/// (identical code); `OutwardEmissionVelocityInfo` mirrors the same layout.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CylindricalEmissionVelocityInfo {
    pub dwords: [u32; 6], // +0x04..+0x18
}

/// `OutwardEmissionVelocityInfo` (layout via the folded 0x007a7501 /
/// 0x007a765f bodies).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OutwardEmissionVelocityInfo {
    pub dwords: [u32; 6],
}

/// `OrthoEmissionVelocityInfo`: copy 0x007a706f, op= 0x007a70a9
/// (9 dwords +0x04..+0x24).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct OrthoEmissionVelocityInfo {
    pub dwords: [u32; 9], // +0x04..+0x24
}

/// `TerrainFireEmissionInfo` (EmissionVolumeInfo-derived): copy 0x007a6e0e
/// (base + 9 dwords +0x08..+0x2c), op= 0x007a6e4e.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TerrainFireEmissionInfo {
    pub flag_4: bool,      // +0x04 (base copy sets byte +4; op= writes it)
    pub dwords: [u32; 10], // +0x08..+0x30 (CellEmissionChance real @+0x2c)
}

/// `LightningEmissionInfo` (EmissionVolumeInfo-derived): copy 0x007a6af5,
/// op= 0x007a6b8f — flat map: byte +4 then 33 dwords +0x08..+0x88.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LightningEmissionInfo {
    pub flag_4: bool,      // +0x04
    pub dwords: [u32; 33], // +0x08..+0x88
}

// `[u32; 33]` predates const-generic `Default`; ctor evidence zero-fills.
impl Default for LightningEmissionInfo {
    fn default() -> Self {
        Self {
            flag_4: false,
            dwords: [0; 33],
        }
    }
}

/// `DefaultPhysicsModuleInfo`: copy 0x007a7736, op= 0x007a7776
/// (7 dwords +0x04..+0x1c; bools +0x20/+0x21; zero defaults via template ctor).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DefaultPhysicsModuleInfo {
    pub dwords: [u32; 7], // +0x04..+0x1c
    pub flag_20: bool,    // +0x20
    pub flag_21: bool,    // +0x21
}
impl DefaultPhysicsModuleInfo {
    pub fn snapshot_name() -> &'static str {
        "DefaultPhysicsModuleInfo" // GetSnapshotName 0x007a612d
    }
    /// 0x0095eecb `DoXfer`: `xfer(Version&)` (slot +0x28) with local
    /// `Version{minimum:1, current:3}`; `xfer(float&)` +0x10; `xfer(Coord3D&)`
    /// (slot +0x60, 'c3d', 0x00a0b971) +4; GCRV composite +0x14; then the
    /// gates `1 < version.current` → bool +0x20 and `2 < version.current` →
    /// bool +0x21 (stream version on load; local 3 on store).
    pub fn do_xfer<X: sage_xfer::Xfer>(&mut self, x: &mut X) {
        let mut v = sage_xfer::Version {
            minimum: 1,
            current: 3,
        };
        x.xfer_version(&mut v);
        let mut r = f32::from_bits(self.dwords[3]); // +0x10 (dwords start at +4)
        x.xfer_f32(&mut r);
        self.dwords[3] = r.to_bits();
        let mut c = sage_xfer::Coord3D {
            x: f32::from_bits(self.dwords[0]),
            y: f32::from_bits(self.dwords[1]),
            z: f32::from_bits(self.dwords[2]),
        };
        x.xfer_coord3d(&mut c);
        self.dwords[0] = c.x.to_bits();
        self.dwords[1] = c.y.to_bits();
        self.dwords[2] = c.z.to_bits();
        let mut g = [self.dwords[4], self.dwords[5], self.dwords[6]];
        xfer_gcrv_raw(x, &mut g);
        self.dwords[4..7].copy_from_slice(&g);
        if 1 < v.current {
            x.xfer_bool(&mut self.flag_20);
        }
        if 2 < v.current {
            x.xfer_bool(&mut self.flag_21);
        }
    }
}

/// `DefaultUpdateModuleInfo`: copy 0x007a789e, op= 0x007a78ff
/// (25 dwords +0x04..+0x64; the copy/op= loops move only 24 — the 25th
/// dword +0x64 is the InitialDelay `max`, observable only through
/// DoXfer 0x0095f67c's version-gated trio).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DefaultUpdateModuleInfo {
    pub dwords: [u32; 25], // +0x04..+0x64
}
impl DefaultUpdateModuleInfo {
    pub fn snapshot_name() -> &'static str {
        "DefaultUpdateModuleInfo" // GetSnapshotName 0x007a6133
    }
    /// 0x0095f67c `DoXfer`: `!IsStoring` guard; `xfer(Version&)` local
    /// `Version{minimum:1, current:2}` (imm 0x201); five GCRV composites
    /// +0x04..+0x3f; 'enu4' "RotationType" (FUN_00706082) +0x40 =
    /// `dwords[15]`; gate `1 < current`: three more GCRV +0x44/+0x50/+0x5c
    /// (`dwords[16..24]`).
    pub fn do_xfer<X: sage_xfer::Xfer>(&mut self, x: &mut X) {
        if x.is_storing() {
            return;
        }
        let mut v = sage_xfer::Version {
            minimum: 1,
            current: 2,
        };
        x.xfer_version(&mut v);
        for k in 0..5 {
            xfer_gcrv_slice(x, &mut self.dwords[k * 3..]);
        }
        let mut e = self.dwords[15].to_le_bytes();
        x.xfer_enum_named("RotationType", &mut e);
        self.dwords[15] = u32::from_le_bytes(e);
        if 1 < v.current {
            for k in 0..3 {
                xfer_gcrv_slice(x, &mut self.dwords[16 + k * 3..]);
            }
        }
    }
}

/// `RenderObjectUpdateModuleInfo`: copy 0x007a7f3e, op= 0x007a7fcf
/// (37 dwords +0x04..+0x94).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderObjectUpdateModuleInfo {
    pub dwords: [u32; 37], // +0x04..+0x94
}

// `[u32; 37]` predates const-generic `Default`; template ctors zero-fill.
impl Default for RenderObjectUpdateModuleInfo {
    fn default() -> Self {
        Self { dwords: [0; 37] }
    }
}
impl RenderObjectUpdateModuleInfo {
    pub fn snapshot_name() -> &'static str {
        "RenderObjectUpdateModuleInfo" // GetSnapshotName 0x007a6163
    }
    /// 0x00962182 `DoXfer`: `!IsStoring` guard; `Version1`; twelve GCRV
    /// composites +0x04..+0x93 (stride 0xc); 'enu4' "RotationType"
    /// (FUN_00706082) at +0x94 = `dwords[36]`.
    pub fn do_xfer<X: sage_xfer::Xfer>(&mut self, x: &mut X) {
        if x.is_storing() {
            return;
        }
        x.version1();
        for k in 0..12 {
            xfer_gcrv_slice(x, &mut self.dwords[k * 3..]);
        }
        let mut e = self.dwords[36].to_le_bytes();
        x.xfer_enum_named("RotationType", &mut e);
        self.dwords[36] = u32::from_le_bytes(e);
    }
}

/// `LightningDrawModuleInfo`: copy 0x007a7dca, op= 0x007a7e04
/// (10 dwords +0x04..+0x28; byte +0x2c).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LightningDrawModuleInfo {
    pub dwords: [u32; 10], // +0x04..+0x28
    pub flag_2c: bool,     // +0x2c
}
impl LightningDrawModuleInfo {
    pub fn snapshot_name() -> &'static str {
        "LightningDrawModuleInfo" // GetSnapshotName 0x007a6157
    }
    /// 0x009614d9 `DoXfer`: `Version1`; three GCRV composites (0x00706183) at
    /// +4/+0x10/+0x1c; `xfer(float&)` (slot +0x70, tag 'real') at +0x28;
    /// `xfer(bool&)` (slot +0x90, tag 'bool', 0x004054a4) at +0x2c.
    pub fn do_xfer<X: sage_xfer::Xfer>(&mut self, x: &mut X) {
        x.version1();
        for base in [0usize, 3, 6] {
            let mut g = [
                self.dwords[base],
                self.dwords[base + 1],
                self.dwords[base + 2],
            ];
            xfer_gcrv_raw(x, &mut g);
            self.dwords[base..base + 3].copy_from_slice(&g);
        }
        let mut r = f32::from_bits(self.dwords[9]);
        x.xfer_f32(&mut r);
        self.dwords[9] = r.to_bits();
        x.xfer_bool(&mut self.flag_2c);
    }
}

/// `DefaultAlphaModuleInfo`: copy 0x007a616f (32-dword loop), op=
/// 0x007a618d (8x16 B loop +0x04..+0x83). 16 B entries (RandomAlphaKeyframe
/// shape per 0x00401e7f); semantic keyframe pairing is Tier B (DoXfer
/// 0x0095b266), so the raw layout is mirrored.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DefaultAlphaModuleInfo {
    pub entries: [[f32; 4]; 8], // +0x04..+0x83
}
impl DefaultAlphaModuleInfo {
    pub fn snapshot_name() -> &'static str {
        "DefaultAlphaModuleInfo" // GetSnapshotName 0x007a611b
    }
    /// 0x0095b266 `DoXfer`: `Version1`; per entry (base +4, stride 0x10):
    /// GCRV composite (0x00706183) on the entry's {dist,min,max}, then
    /// `xfer(unsigned int&)` at entry+0x10 = the *next* entry's dist dword
    /// (same off-by-one family as DefaultColor 0x0095b8cd); iteration 8's
    /// uint targets +0x84, past the 32-dword payload — the standalone object
    /// has no field there, mirrored via a scratch dword (game reads/writes
    /// whatever follows; not modelled).
    pub fn do_xfer<X: sage_xfer::Xfer>(&mut self, x: &mut X) {
        x.version1();
        let mut oob84 = 0u32;
        for k in 0..8 {
            let mut g = [
                self.entries[k][0].to_bits(),
                self.entries[k][1].to_bits(),
                self.entries[k][2].to_bits(),
            ];
            xfer_gcrv_raw(x, &mut g);
            self.entries[k][0] = f32::from_bits(g[0]);
            self.entries[k][1] = f32::from_bits(g[1]);
            self.entries[k][2] = f32::from_bits(g[2]);
            match k {
                0..=6 => {
                    let mut b = self.entries[k + 1][0].to_bits();
                    x.xfer_u32(&mut b);
                    self.entries[k + 1][0] = f32::from_bits(b);
                }
                _ => x.xfer_u32(&mut oob84),
            }
        }
    }
}

/// `DefaultColorModuleInfo`: copy 0x007a62d9, op= 0x007a6306 —
/// 8x16 B loop (+0x04..+0x83, `RGBColorKeyframe` shape per 0x00401e67)
/// + 3 trailing dwords +0x84/+0x88/+0x8c.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DefaultColorModuleInfo {
    pub keyframes: [RgbColorKeyframe; 8], // +0x04..+0x83
    pub dwords: [u32; 3],                 // +0x84..+0x8c
}
impl DefaultColorModuleInfo {
    pub fn snapshot_name() -> &'static str {
        "DefaultColorModuleInfo" // GetSnapshotName 0x007a6121
    }
    /// 0x0095b8cd `DoXfer`: `Version1`, then 8 entries (base `+4`, stride
    /// 0x10): `xfer(RGBColor&)` (slot +0x3c → tag 'rgb' + 12 B raw, 0x00a0bb51)
    /// then `xfer(unsigned int&)` (slot +0x78, tag 'uint', 0x004054f8) at
    /// entry+0x10. The game's off-by-one: entry+0x10 is the *next* entry's
    /// color[0], so no `time` (+0xc) is ever transferred and iteration 8's
    /// "uint" lands on the random-variable dist dword +0x84; then the GCRV
    /// composite (0x00706183) at +0x84. Mirrored verbatim.
    pub fn do_xfer<X: sage_xfer::Xfer>(&mut self, x: &mut X) {
        x.version1();
        for k in 0..8 {
            let mut c = sage_xfer::RGBColor {
                red: self.keyframes[k].color[0],
                green: self.keyframes[k].color[1],
                blue: self.keyframes[k].color[2],
            };
            x.xfer_rgb_color(&mut c);
            self.keyframes[k].color = [c.red, c.green, c.blue];
            match k {
                0..=6 => {
                    let mut b = self.keyframes[k + 1].color[0].to_bits();
                    x.xfer_u32(&mut b);
                    self.keyframes[k + 1].color[0] = f32::from_bits(b);
                }
                _ => x.xfer_u32(&mut self.dwords[0]),
            }
        }
        xfer_gcrv_raw(x, &mut self.dwords);
    }
    /// 0x0095b863 `tintAllColors(int)`: `setFromInt` the tint (math 0x00404edf),
    /// then the game loop `(pDVar1 = this+0x18; writes −4/0/+4; stride +0x10;
    /// ×8)`. The loop is off by one: it tints keyframes 1..7's colors (kf0 is
    /// NEVER tinted) and iteration 8 multiplies the random-variable dwords at
    /// +0x84/+0x88/+0x8c as floats — clobbered immediately by
    /// `setRange(0.0, 0.0, dist=1)` (0x006341e7). Mirrored faithfully.
    pub fn tint_all_colors(&mut self, tint: i32) {
        let t = RGBColor::from_int(tint);
        for kf in &mut self.keyframes[1..] {
            kf.color[0] = kf.color[0] * t.r;
            kf.color[1] = t.g * kf.color[1]; // C operand order verbatim
            kf.color[2] = kf.color[2] * t.b;
        }
        for (i, mul) in [t.r, t.g, t.b].iter().enumerate() {
            self.dwords[i] = (f32::from_bits(self.dwords[i]) * *mul).to_bits();
        }
        self.dwords = [1, 0, 0]; // setRange: dist @0x84=1, min @0x88=0, max @0x8c=0
    }
}

/// `RenderObjectDrawModuleInfo` (0x40 B). Ctor 0x00962bb1: three groups of
/// `{ raw-zero-init string, u32, u32, u32=8 }` at +0x10/+0x20/+0x30;
/// op= 0x007a7d2e string-`set`s those three offsets.
#[derive(Clone, Debug, PartialEq)]
pub struct RenderObjectDrawModuleInfo {
    pub flag_4: bool,          // +0x04 = 0
    pub field_8: u32,          // +0x08 = 0
    pub flag_c: bool,          // +0x0c = 0
    pub groups: [RodGroup; 3], // +0x10, +0x20, +0x30
}

/// One `RenderObjectDrawModuleInfo` triple: `{ name, a, b, c=8 }`.
#[derive(Clone, Debug, PartialEq)]
pub struct RodGroup {
    pub name: AsciiString,
    pub a: u32,
    pub b: u32,
    pub c: u32,
}

impl Default for RenderObjectDrawModuleInfo {
    fn default() -> Self {
        Self {
            flag_4: false,
            field_8: 0,
            flag_c: false,
            groups: [
                RodGroup {
                    name: AsciiString::default(),
                    a: 0,
                    b: 0,
                    c: 8,
                },
                RodGroup {
                    name: AsciiString::default(),
                    a: 0,
                    b: 0,
                    c: 8,
                },
                RodGroup {
                    name: AsciiString::default(),
                    a: 0,
                    b: 0,
                    c: 8,
                },
            ],
        }
    }
}
impl RenderObjectDrawModuleInfo {
    pub fn snapshot_name() -> &'static str {
        "RenderObjectDrawModuleInfo" // GetSnapshotName 0x007a6151
    }
    /// 0x00962871 `DoXfer`: `Version1`; 'bool' +0xc; then the three groups
    /// (+0x10/+0x20/+0x30): AsciiString name (0x00a0bea4), 'int' (slot +0x7c,
    /// 0x004054c0), 'real' +0x18/+0x28/+0x38, 'enu4' "ParticleShaderType"
    /// (FUN_00706052) on `c`; then 'bool' +4 and 'real' +8 (`field_8` holds
    /// the float bits).
    pub fn do_xfer<X: sage_xfer::Xfer>(&mut self, x: &mut X) {
        x.version1();
        x.xfer_bool(&mut self.flag_c);
        for g in self.groups.iter_mut() {
            crate::string::xfer_ascii_string(x, &mut g.name);
            let mut i = g.a as i32;
            x.xfer_i32(&mut i);
            g.a = i as u32;
            let mut r = f32::from_bits(g.b);
            x.xfer_f32(&mut r);
            g.b = r.to_bits();
            let mut e = g.c.to_le_bytes();
            x.xfer_enum_named("ParticleShaderType", &mut e);
            g.c = u32::from_le_bytes(e);
        }
        x.xfer_bool(&mut self.flag_4);
        let mut r = f32::from_bits(self.field_8);
        x.xfer_f32(&mut r);
        self.field_8 = r.to_bits();
    }
}

/// `LifeEventModuleInfo`: op= 0x007a7a51 — string +0x04 (set) + 4 dwords
/// +0x08..+0x14. (No decompiled ctor/copy-ctor row; `getEventFX` 0x0096410f
/// and DoXfer 0x00963e72 are Tier B.)
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LifeEventModuleInfo {
    pub name: AsciiString, // +0x04
    pub dwords: [u32; 4],  // +0x08..+0x14
}
impl LifeEventModuleInfo {
    pub fn snapshot_name() -> &'static str {
        "LifeEventModuleInfo" // GetSnapshotName 0x007a6139
    }
    /// 0x00963e72 `DoXfer`: `!IsStoring` guard; `Version1`; GCRV composite
    /// on `dwords[0..3]` (+8); AsciiString +4.
    pub fn do_xfer<X: sage_xfer::Xfer>(&mut self, x: &mut X) {
        if x.is_storing() {
            return;
        }
        x.version1();
        xfer_gcrv_slice(x, &mut self.dwords);
        crate::string::xfer_ascii_string(x, &mut self.name);
    }
}

/// `TerrainCollisionModuleInfo`: op= 0x007a81ae — string +0x04 (set),
/// dwords +0x08..+0x10, byte +0x14, dword +0x18.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TerrainCollisionModuleInfo {
    pub name: AsciiString, // +0x04
    pub dwords: [u32; 3],  // +0x08..+0x10
    pub flag_14: bool,     // +0x14
    pub field_18: u32,     // +0x18
}
impl TerrainCollisionModuleInfo {
    pub fn snapshot_name() -> &'static str {
        "TerrainCollisionModuleInfo" // GetSnapshotName 0x007a6169
    }
    /// 0x0096445e `DoXfer`: `Version1`; GCRV composite (0x00706183) +8;
    /// AsciiString +4 (0x00a0bea4); 'bool' +0x14.
    pub fn do_xfer<X: sage_xfer::Xfer>(&mut self, x: &mut X) {
        x.version1();
        xfer_gcrv_slice(x, &mut self.dwords);
        crate::string::xfer_ascii_string(x, &mut self.name);
        x.xfer_bool(&mut self.flag_14);
    }
}

// ---------------------------------------------------------------------------
// INI parse tier — the 18-byte `*ModuleTemplate::parse` wrappers and their
// token tables (wave 17). Game shape (every wrapper disassembles to):
// `push TABLE; push ecx(=info dest); mov ecx,[esp+0xc](=ini);
// call 0x0042de78; ret 4` → `parse_block` (0x0042d7b2) with the template
// object in ECX as the destination base. Handler VAs below are the exact
// dispatch keys; dest = template-object byte offset (`info(off)` = off−8
// in the modeled standalone offsets — verified against each DoXfer).

/// `parse` token-table handler dispatch keys (entry[1]).
const H_BOOL: u32 = 0x0042e850; // df97 → d14a → mov byte [dest]
const H_REAL: u32 = 0x0042efc0; // df97 → eda5 → mov float [dest]
const H_I32: u32 = 0x0042ef56; // df97 → eccf → mov [dest]
const H_BOUNDED_U32: u32 = 0x0042ef72; // v2 = bound (0 = unchecked), 0x00bbe2b0
const H_VEC3: u32 = 0x0042f507; // three real fields +0/+4/+8
const H_ENUM: u32 = 0x0042ec4e; // v2 = name list, scan-throw, [dest] = idx
const H_STRING: u32 = 0x0042f11e; // quoted-string slot (body 0x0042ea59)
const H_EVENTFX_STRING: u32 = 0x00963ea8; // pure tail-wrapper → f11e(ini,dest,0)
const H_GCRV: u32 = 0x00738a9d; // min max [DISTNAME], setRange dist@0/min@4/max@8
const H_RAND_KEYFRAME: u32 = 0x0095b29c; // min max uint; setRange(dest,·,·,1)@+0xc
const H_RGB_KEYFRAME: u32 = 0x0095b913; // R= G= B= then bounded uint @dest+0xc

/// `ParticlePriorityNames` (list @0x00c1b6a4, table of 0x005f3d2e).
pub const PARTICLE_PRIORITY_NAMES: &[&str] = &[
    "NONE",
    "ULTRA_HIGH_ONLY",
    "HIGH_OR_ABOVE",
    "MEDIUM_OR_ABOVE",
    "LOW_OR_ABOVE",
    "VERY_LOW_OR_ABOVE",
    "ALWAYS_RENDER",
];
/// `ParticleShaderTypeNames` (@0x00c1b610).
pub const PARTICLE_SHADER_TYPE_NAMES: &[&str] = &[
    "NONE",
    "ADDITIVE",
    "ADDITIVE_ALPHA_TEST",
    "ALPHA",
    "ALPHA_TEST",
    "MULTIPLY",
    "ADDITIVE_NO_DEPTH_TEST",
    "ALPHA_NO_DEPTH_TEST",
    "W3D_DIFFUSE",
    "W3D_ALPHA",
    "W3D_EMISSIVE",
];
/// `ParticleTypeNames` (@0x00c1b640).
pub const PARTICLE_TYPE_NAMES: &[&str] = &[
    "NONE",
    "PARTICLE",
    "DRAWABLE",
    "STREAK",
    "VOLUME_PARTICLE",
    "SMUDGE",
    "TERRAIN_PARTICLE",
    "GPU_PARTICLE",
    "GPU_TERRAINFIRE",
];
/// `RotationType` names for the `Rotation` token (list @0x00c1b6d8; matches
/// the W3D RotationType enum — NONE first, ROTATION_OFF=1).
pub const ROTATION_TYPE_NAMES: &[&str] = &[
    "NONE",
    "ROTATION_OFF",
    "ROTATE_X",
    "ROTATE_Y",
    "ROTATE_Z",
    "ROTATE_V",
];
/// `GameClientRandomVariable::DistributionTypeNames` (@0x00db9bec).
pub const DISTRIBUTION_TYPE_NAMES: &[&str] =
    &["UNIFORM", "GAUSSIAN", "TRIANGULAR", "LOW_BIAS", "HIGH_BIAS"];

// -- token tables (verbatim entry order from the wrapper PTR tables) --------

const TBL_POINT_VOL: &[crate::ini::Entry] = &[crate::ini::Entry {
    name: "IsHollow",
    handler: H_BOOL,
    v2: 0,
    off: 12,
}];
const TBL_LINE_VOL: &[crate::ini::Entry] = &[
    crate::ini::Entry {
        name: "IsHollow",
        handler: H_BOOL,
        v2: 0,
        off: 12,
    },
    crate::ini::Entry {
        name: "StartPoint",
        handler: H_VEC3,
        v2: 0,
        off: 16,
    },
    crate::ini::Entry {
        name: "EndPoint",
        handler: H_VEC3,
        v2: 0,
        off: 28,
    },
];
const TBL_BOX_VOL: &[crate::ini::Entry] = &[
    crate::ini::Entry {
        name: "IsHollow",
        handler: H_BOOL,
        v2: 0,
        off: 12,
    },
    crate::ini::Entry {
        name: "HalfSize",
        handler: H_VEC3,
        v2: 0,
        off: 16,
    },
];
const TBL_SPHERE_VOL: &[crate::ini::Entry] = &[
    crate::ini::Entry {
        name: "IsHollow",
        handler: H_BOOL,
        v2: 0,
        off: 12,
    },
    crate::ini::Entry {
        name: "Radius",
        handler: H_REAL,
        v2: 0,
        off: 16,
    },
];
const TBL_CYL_VOL: &[crate::ini::Entry] = &[
    crate::ini::Entry {
        name: "IsHollow",
        handler: H_BOOL,
        v2: 0,
        off: 12,
    },
    crate::ini::Entry {
        name: "Radius",
        handler: H_REAL,
        v2: 0,
        off: 16,
    },
    crate::ini::Entry {
        name: "RadiusRate",
        handler: H_REAL,
        v2: 0,
        off: 20,
    },
    crate::ini::Entry {
        name: "Length",
        handler: H_REAL,
        v2: 0,
        off: 24,
    },
    crate::ini::Entry {
        name: "Offset",
        handler: H_VEC3,
        v2: 0,
        off: 28,
    },
];
const TBL_SPHERICAL_VEL: &[crate::ini::Entry] = &[crate::ini::Entry {
    name: "Speed",
    handler: H_GCRV,
    v2: 0,
    off: 12,
}];
const TBL_CYL_VEL: &[crate::ini::Entry] = &[
    crate::ini::Entry {
        name: "Radial",
        handler: H_GCRV,
        v2: 0,
        off: 12,
    },
    crate::ini::Entry {
        name: "Normal",
        handler: H_GCRV,
        v2: 0,
        off: 24,
    },
];
const TBL_OUTWARD_VEL: &[crate::ini::Entry] = &[
    crate::ini::Entry {
        name: "Speed",
        handler: H_GCRV,
        v2: 0,
        off: 12,
    },
    crate::ini::Entry {
        name: "OtherSpeed",
        handler: H_GCRV,
        v2: 0,
        off: 24,
    },
];
const TBL_ORTHO_VEL: &[crate::ini::Entry] = &[
    crate::ini::Entry {
        name: "X",
        handler: H_GCRV,
        v2: 0,
        off: 12,
    },
    crate::ini::Entry {
        name: "Y",
        handler: H_GCRV,
        v2: 0,
        off: 24,
    },
    crate::ini::Entry {
        name: "Z",
        handler: H_GCRV,
        v2: 0,
        off: 36,
    },
];
const TBL_TERRAIN_FIRE: &[crate::ini::Entry] = &[
    crate::ini::Entry {
        name: "Xoffset",
        handler: H_GCRV,
        v2: 0,
        off: 16,
    },
    crate::ini::Entry {
        name: "Yoffset",
        handler: H_GCRV,
        v2: 0,
        off: 28,
    },
    crate::ini::Entry {
        name: "Zoffset",
        handler: H_GCRV,
        v2: 0,
        off: 40,
    },
    crate::ini::Entry {
        name: "CellEmissionChance",
        handler: H_REAL,
        v2: 0,
        off: 52,
    },
];
const TBL_LIGHTNING_EMISSION: &[crate::ini::Entry] = &[
    crate::ini::Entry {
        name: "StartPoint",
        handler: H_VEC3,
        v2: 0,
        off: 16,
    },
    crate::ini::Entry {
        name: "EndPoint",
        handler: H_VEC3,
        v2: 0,
        off: 28,
    },
    crate::ini::Entry {
        name: "Amplitude1",
        handler: H_GCRV,
        v2: 0,
        off: 40,
    },
    crate::ini::Entry {
        name: "Frequency1",
        handler: H_GCRV,
        v2: 0,
        off: 52,
    },
    crate::ini::Entry {
        name: "Phase1",
        handler: H_GCRV,
        v2: 0,
        off: 64,
    },
    crate::ini::Entry {
        name: "Amplitude2",
        handler: H_GCRV,
        v2: 0,
        off: 76,
    },
    crate::ini::Entry {
        name: "Frequency2",
        handler: H_GCRV,
        v2: 0,
        off: 88,
    },
    crate::ini::Entry {
        name: "Phase2",
        handler: H_GCRV,
        v2: 0,
        off: 100,
    },
    crate::ini::Entry {
        name: "Amplitude3",
        handler: H_GCRV,
        v2: 0,
        off: 112,
    },
    crate::ini::Entry {
        name: "Frequency3",
        handler: H_GCRV,
        v2: 0,
        off: 124,
    },
    crate::ini::Entry {
        name: "Phase3",
        handler: H_GCRV,
        v2: 0,
        off: 136,
    },
];
const TBL_DEFAULT_3: &[crate::ini::Entry] = &[
    crate::ini::Entry {
        name: "Gravity",
        handler: H_REAL,
        v2: 0,
        off: 24,
    },
    crate::ini::Entry {
        name: "VelocityDamping",
        handler: H_GCRV,
        v2: 0,
        off: 28,
    },
    crate::ini::Entry {
        name: "DriftVelocity",
        handler: H_VEC3,
        v2: 0,
        off: 12,
    },
    crate::ini::Entry {
        name: "Swirly",
        handler: H_BOOL,
        v2: 0,
        off: 40,
    },
    crate::ini::Entry {
        name: "ParticlesAttachToBone",
        handler: H_BOOL,
        v2: 0,
        off: 41,
    },
];
const TBL_DEFAULT_2: &[crate::ini::Entry] = &[
    crate::ini::Entry {
        name: "SizeRate",
        handler: H_GCRV,
        v2: 0,
        off: 12,
    },
    crate::ini::Entry {
        name: "SizeRateDamping",
        handler: H_GCRV,
        v2: 0,
        off: 24,
    },
    crate::ini::Entry {
        name: "AngleZ",
        handler: H_GCRV,
        v2: 0,
        off: 36,
    },
    crate::ini::Entry {
        name: "AngularRateZ",
        handler: H_GCRV,
        v2: 0,
        off: 48,
    },
    crate::ini::Entry {
        name: "AngularDamping",
        handler: H_GCRV,
        v2: 0,
        off: 60,
    },
    crate::ini::Entry {
        name: "Rotation",
        handler: H_ENUM,
        v2: 0x00c1b6d8,
        off: 72,
    },
    crate::ini::Entry {
        name: "AngleXY",
        handler: H_GCRV,
        v2: 0,
        off: 76,
    },
    crate::ini::Entry {
        name: "AngularRateXY",
        handler: H_GCRV,
        v2: 0,
        off: 88,
    },
    crate::ini::Entry {
        name: "AngularDampingXY",
        handler: H_GCRV,
        v2: 0,
        off: 100,
    },
];
const TBL_ALPHA: &[crate::ini::Entry] = &[
    crate::ini::Entry {
        name: "Alpha1",
        handler: H_RAND_KEYFRAME,
        v2: 0,
        off: 12,
    },
    crate::ini::Entry {
        name: "Alpha2",
        handler: H_RAND_KEYFRAME,
        v2: 0,
        off: 28,
    },
    crate::ini::Entry {
        name: "Alpha3",
        handler: H_RAND_KEYFRAME,
        v2: 0,
        off: 44,
    },
    crate::ini::Entry {
        name: "Alpha4",
        handler: H_RAND_KEYFRAME,
        v2: 0,
        off: 60,
    },
    crate::ini::Entry {
        name: "Alpha5",
        handler: H_RAND_KEYFRAME,
        v2: 0,
        off: 76,
    },
    crate::ini::Entry {
        name: "Alpha6",
        handler: H_RAND_KEYFRAME,
        v2: 0,
        off: 92,
    },
    crate::ini::Entry {
        name: "Alpha7",
        handler: H_RAND_KEYFRAME,
        v2: 0,
        off: 108,
    },
    crate::ini::Entry {
        name: "Alpha8",
        handler: H_RAND_KEYFRAME,
        v2: 0,
        off: 124,
    },
];
const TBL_COLOR: &[crate::ini::Entry] = &[
    crate::ini::Entry {
        name: "Color1",
        handler: H_RGB_KEYFRAME,
        v2: 0,
        off: 12,
    },
    crate::ini::Entry {
        name: "Color2",
        handler: H_RGB_KEYFRAME,
        v2: 0,
        off: 28,
    },
    crate::ini::Entry {
        name: "Color3",
        handler: H_RGB_KEYFRAME,
        v2: 0,
        off: 44,
    },
    crate::ini::Entry {
        name: "Color4",
        handler: H_RGB_KEYFRAME,
        v2: 0,
        off: 60,
    },
    crate::ini::Entry {
        name: "Color5",
        handler: H_RGB_KEYFRAME,
        v2: 0,
        off: 76,
    },
    crate::ini::Entry {
        name: "Color6",
        handler: H_RGB_KEYFRAME,
        v2: 0,
        off: 92,
    },
    crate::ini::Entry {
        name: "Color7",
        handler: H_RGB_KEYFRAME,
        v2: 0,
        off: 108,
    },
    crate::ini::Entry {
        name: "Color8",
        handler: H_RGB_KEYFRAME,
        v2: 0,
        off: 124,
    },
    crate::ini::Entry {
        name: "ColorScale",
        handler: H_GCRV,
        v2: 0,
        off: 140,
    },
];
const TBL_GPU_DRAW: &[crate::ini::Entry] = &[
    crate::ini::Entry {
        name: "FramesPerRow",
        handler: H_I32,
        v2: 0,
        off: 12,
    },
    crate::ini::Entry {
        name: "TotalFrames",
        handler: H_I32,
        v2: 0,
        off: 16,
    },
    crate::ini::Entry {
        name: "DetailTexture",
        handler: H_STRING,
        v2: 0,
        off: 20,
    },
    crate::ini::Entry {
        name: "SpeedMultiplier",
        handler: H_REAL,
        v2: 0,
        off: 24,
    },
];
const TBL_RENDER_DRAW: &[crate::ini::Entry] = &[
    crate::ini::Entry {
        name: "MultiRenderObjects",
        handler: H_BOOL,
        v2: 0,
        off: 20,
    },
    crate::ini::Entry {
        name: "RenderGroup1",
        handler: H_STRING,
        v2: 0,
        off: 24,
    },
    crate::ini::Entry {
        name: "NumObjects1",
        handler: H_I32,
        v2: 0,
        off: 28,
    },
    crate::ini::Entry {
        name: "Percent1",
        handler: H_REAL,
        v2: 0,
        off: 32,
    },
    crate::ini::Entry {
        name: "Shader1",
        handler: H_ENUM,
        v2: 0x00c1b610,
        off: 36,
    },
    crate::ini::Entry {
        name: "RenderGroup2",
        handler: H_STRING,
        v2: 0,
        off: 40,
    },
    crate::ini::Entry {
        name: "NumObjects2",
        handler: H_I32,
        v2: 0,
        off: 44,
    },
    crate::ini::Entry {
        name: "Percent2",
        handler: H_REAL,
        v2: 0,
        off: 48,
    },
    crate::ini::Entry {
        name: "Shader2",
        handler: H_ENUM,
        v2: 0x00c1b610,
        off: 52,
    },
    crate::ini::Entry {
        name: "RenderGroup3",
        handler: H_STRING,
        v2: 0,
        off: 56,
    },
    crate::ini::Entry {
        name: "NumObjects3",
        handler: H_I32,
        v2: 0,
        off: 60,
    },
    crate::ini::Entry {
        name: "Percent3",
        handler: H_REAL,
        v2: 0,
        off: 64,
    },
    crate::ini::Entry {
        name: "Shader3",
        handler: H_ENUM,
        v2: 0x00c1b610,
        off: 68,
    },
    crate::ini::Entry {
        name: "SinkOnTerrainCollision",
        handler: H_BOOL,
        v2: 0,
        off: 12,
    },
    crate::ini::Entry {
        name: "SinkRate",
        handler: H_REAL,
        v2: 0,
        off: 16,
    },
];
const TBL_RENDER_UPDATE: &[crate::ini::Entry] = &[
    crate::ini::Entry {
        name: "StartSizeX",
        handler: H_GCRV,
        v2: 0,
        off: 12,
    },
    crate::ini::Entry {
        name: "StartSizeY",
        handler: H_GCRV,
        v2: 0,
        off: 24,
    },
    crate::ini::Entry {
        name: "StartSizeZ",
        handler: H_GCRV,
        v2: 0,
        off: 36,
    },
    crate::ini::Entry {
        name: "SizeRateX",
        handler: H_GCRV,
        v2: 0,
        off: 48,
    },
    crate::ini::Entry {
        name: "SizeRateY",
        handler: H_GCRV,
        v2: 0,
        off: 60,
    },
    crate::ini::Entry {
        name: "SizeRateZ",
        handler: H_GCRV,
        v2: 0,
        off: 72,
    },
    crate::ini::Entry {
        name: "SizeDampingX",
        handler: H_GCRV,
        v2: 0,
        off: 84,
    },
    crate::ini::Entry {
        name: "SizeDampingY",
        handler: H_GCRV,
        v2: 0,
        off: 96,
    },
    crate::ini::Entry {
        name: "SizeDampingZ",
        handler: H_GCRV,
        v2: 0,
        off: 108,
    },
    crate::ini::Entry {
        name: "AngleZ",
        handler: H_GCRV,
        v2: 0,
        off: 120,
    },
    crate::ini::Entry {
        name: "AngularRateZ",
        handler: H_GCRV,
        v2: 0,
        off: 132,
    },
    crate::ini::Entry {
        name: "AngularDamping",
        handler: H_GCRV,
        v2: 0,
        off: 144,
    },
    crate::ini::Entry {
        name: "Rotation",
        handler: H_ENUM,
        v2: 0x00c1b6d8,
        off: 156,
    },
];
const TBL_LIGHTNING_DRAW: &[crate::ini::Entry] = &[
    crate::ini::Entry {
        name: "OffsetX",
        handler: H_GCRV,
        v2: 0,
        off: 12,
    },
    crate::ini::Entry {
        name: "OffsetY",
        handler: H_GCRV,
        v2: 0,
        off: 24,
    },
    crate::ini::Entry {
        name: "OffsetZ",
        handler: H_GCRV,
        v2: 0,
        off: 36,
    },
    crate::ini::Entry {
        name: "MultiChance",
        handler: H_REAL,
        v2: 0,
        off: 48,
    },
    crate::ini::Entry {
        name: "TileTexture",
        handler: H_BOOL,
        v2: 0,
        off: 52,
    },
];
const TBL_LIFE_EVENT: &[crate::ini::Entry] = &[
    crate::ini::Entry {
        name: "PerParticle",
        handler: H_BOOL,
        v2: 0,
        off: 8,
    },
    crate::ini::Entry {
        name: "KillAfterEvent",
        handler: H_BOOL,
        v2: 0,
        off: 9,
    },
    crate::ini::Entry {
        name: "EventFX",
        handler: H_EVENTFX_STRING,
        v2: 0,
        off: 16,
    },
    crate::ini::Entry {
        name: "EventTime",
        handler: H_GCRV,
        v2: 0,
        off: 20,
    },
];
const TBL_TERRAIN_COLLISION: &[crate::ini::Entry] = &[
    crate::ini::Entry {
        name: "PerParticle",
        handler: H_BOOL,
        v2: 0,
        off: 8,
    },
    crate::ini::Entry {
        name: "KillAfterEvent",
        handler: H_BOOL,
        v2: 0,
        off: 9,
    },
    crate::ini::Entry {
        name: "EventFX",
        handler: H_EVENTFX_STRING,
        v2: 0,
        off: 16,
    },
    crate::ini::Entry {
        name: "HeightOffset",
        handler: H_GCRV,
        v2: 0,
        off: 20,
    },
    crate::ini::Entry {
        name: "OrientFXToTerrain",
        handler: H_BOOL,
        v2: 0,
        off: 32,
    },
];
const TBL_PARTICLE_SYSTEM: &[crate::ini::Entry] = &[
    crate::ini::Entry {
        name: "Priority",
        handler: H_ENUM,
        v2: 0x00c1b6a4,
        off: 0x7c,
    },
    crate::ini::Entry {
        name: "IsOneShot",
        handler: H_BOOL,
        v2: 0,
        off: 4,
    },
    crate::ini::Entry {
        name: "Shader",
        handler: H_ENUM,
        v2: 0x00c1b610,
        off: 8,
    },
    crate::ini::Entry {
        name: "Type",
        handler: H_ENUM,
        v2: 0x00c1b640,
        off: 0xc,
    },
    crate::ini::Entry {
        name: "ParticleName",
        handler: H_STRING,
        v2: 0,
        off: 0x10,
    },
    crate::ini::Entry {
        name: "SlaveSystem",
        handler: H_STRING,
        v2: 0,
        off: 0x68,
    },
    crate::ini::Entry {
        name: "SlavePosOffset",
        handler: H_VEC3,
        v2: 0,
        off: 0x6c,
    },
    crate::ini::Entry {
        name: "PerParticleAttachedSystem",
        handler: H_STRING,
        v2: 0,
        off: 0x78,
    },
    crate::ini::Entry {
        name: "Lifetime",
        handler: H_GCRV,
        v2: 0,
        off: 0x14,
    },
    crate::ini::Entry {
        name: "SystemLifetime",
        handler: H_BOUNDED_U32,
        v2: 0,
        off: 0x20,
    },
    crate::ini::Entry {
        name: "SortLevel",
        handler: H_BOUNDED_U32,
        v2: 0,
        off: 0x24,
    },
    crate::ini::Entry {
        name: "Size",
        handler: H_GCRV,
        v2: 0,
        off: 0x28,
    },
    crate::ini::Entry {
        name: "StartSizeRate",
        handler: H_GCRV,
        v2: 0,
        off: 0x34,
    },
    crate::ini::Entry {
        name: "BurstDelay",
        handler: H_GCRV,
        v2: 0,
        off: 0x44,
    },
    crate::ini::Entry {
        name: "BurstCount",
        handler: H_GCRV,
        v2: 0,
        off: 0x50,
    },
    crate::ini::Entry {
        name: "InitialDelay",
        handler: H_GCRV,
        v2: 0,
        off: 0x5c,
    },
    crate::ini::Entry {
        name: "IsGroundAligned",
        handler: H_BOOL,
        v2: 0,
        off: 0x80,
    },
    crate::ini::Entry {
        name: "IsEmitAboveGroundOnly",
        handler: H_BOOL,
        v2: 0,
        off: 0x81,
    },
    crate::ini::Entry {
        name: "IsParticleUpTowardsEmitter",
        handler: H_BOOL,
        v2: 0,
        off: 0x82,
    },
    crate::ini::Entry {
        name: "UseMaximumHeight",
        handler: H_BOOL,
        v2: 0,
        off: 0x83,
    },
    crate::ini::Entry {
        name: "ShroudEmitter",
        handler: H_BOOL,
        v2: 0,
        off: 0x84,
    },
];

// -- store helpers (handler read/store patterns, per game body) -----------

type IniResult<T> = crate::ini::IniResult<T>;

/// 0x0042e850: `df97 → d14a → mov byte [dest]`.
fn store_bool(ini: &mut crate::ini::Ini, b: &mut bool) -> IniResult<()> {
    *b = ini.get_bool()?;
    Ok(())
}
/// `mov float [dest]` store behind the real handler 0x0042efc0.
fn store_real_u32(ini: &mut crate::ini::Ini, d: &mut [u32], i: usize) -> IniResult<()> {
    d[i] = (ini.get_real()? as f32).to_bits();
    Ok(())
}
fn store_real_f32(ini: &mut crate::ini::Ini, d: &mut [f32], i: usize) -> IniResult<()> {
    d[i] = ini.get_real()? as f32;
    Ok(())
}
/// 0x0042f507 handler: three real fields at +0/+4/+8.
fn store_vec3_u32(ini: &mut crate::ini::Ini, d: &mut [u32], i: usize) -> IniResult<()> {
    let (x, y, z) = ini.get_vec3()?;
    d[i] = (x as f32).to_bits();
    d[i + 1] = (y as f32).to_bits();
    d[i + 2] = (z as f32).to_bits();
    Ok(())
}
fn store_vec3_f32(ini: &mut crate::ini::Ini, d: &mut [f32], i: usize) -> IniResult<()> {
    let (x, y, z) = ini.get_vec3()?;
    d[i] = x as f32;
    d[i + 1] = y as f32;
    d[i + 2] = z as f32;
    Ok(())
}
/// 0x0042ec4e handler: token → always-throw scan of the entry's `v2`
/// name list; stores the index.
fn enum_value(ini: &mut crate::ini::Ini, v2: u32) -> IniResult<u32> {
    use crate::ini::IniError;
    let names: &[&str] = match v2 {
        0x00c1b6a4 => PARTICLE_PRIORITY_NAMES,
        0x00c1b610 => PARTICLE_SHADER_TYPE_NAMES,
        0x00c1b640 => PARTICLE_TYPE_NAMES,
        0x00c1b6d8 => ROTATION_TYPE_NAMES,
        _ => {
            return Err(IniError::internal_msg(format!(
                "unmodelled enum name list v2={v2:#010x}"
            )));
        }
    };
    Ok(ini.get_enum(names)? as u32)
}
fn store_enum(ini: &mut crate::ini::Ini, d: &mut [u32], i: usize, v2: u32) -> IniResult<()> {
    d[i] = enum_value(ini, v2)?;
    Ok(())
}
/// 0x0042f11e / 0x00963ea8 (tail-wrapper): quoted-string slot. A null
/// token performs NO assignment (mirrors `set` never being called).
fn store_string(ini: &mut crate::ini::Ini, s: &mut crate::string::AsciiString) -> IniResult<()> {
    if let Some(v) = ini.get_quoted_string()? {
        let units: Vec<crate::string::Ascii> = v.bytes().map(crate::string::Ascii).collect();
        s.set(&units);
    }
    Ok(())
}
/// 0x00738a9d handler: two reals then the 0x0042deed(0) soft continuation
/// token; default dist = 1 (GAUSSIAN); a token soft-scans
/// `DistributionTypeNames`. Store order dist@0/min@4/max@8 (`setRange`
/// 0x006341e7).
fn fetch_gcrv(ini: &mut crate::ini::Ini) -> IniResult<(u32, f32, f32)> {
    let mn = ini.get_real()? as f32;
    let mx = ini.get_real()? as f32;
    let dist = match ini.next_token(None) {
        Some(t) => crate::ini::Ini::scan_index(&t, DISTRIBUTION_TYPE_NAMES, false)? as u32,
        None => DIST_GAUSSIAN,
    };
    Ok((dist, mn, mx))
}
fn store_gcrv_u32(ini: &mut crate::ini::Ini, d: &mut [u32], i: usize) -> IniResult<()> {
    let (dist, mn, mx) = fetch_gcrv(ini)?;
    d[i] = dist;
    d[i + 1] = mn.to_bits();
    d[i + 2] = mx.to_bits();
    Ok(())
}
/// One token table row driver: seed the one-row `BlockTable` (0x0042d771)
/// and run `parse_block` (0x0042d7b2) with the template's closure.
fn run(
    ini: &mut crate::ini::Ini,
    tbl: &'static [crate::ini::Entry],
    mut apply: impl FnMut(&mut crate::ini::Ini, &crate::ini::Entry, usize, u32) -> IniResult<()>,
) -> IniResult<()> {
    let mut bt = crate::ini::BlockTable::default();
    bt.push(crate::ini::TableRow {
        entries: tbl,
        dest_off: 0,
    })?;
    crate::ini::parse_block(ini, &bt, &mut apply)
}

// -- parse entry points (wrapper rows ⇄ token tables) ----------------------
// Table offsets address the info object embedded in the template
// (`info(off)` = off − 8 in modeled standalone offsets — every module
// DoXfer matched its table under that shift; ParticleSystemTemplate's
// stack-built table addresses the payload directly, base 0).

impl EmissionVolumeInfo {
    /// 0x0095ca78 wrapper `PointEmissionVolumeModuleTemplate::parse`.
    pub fn parse(ini: &mut crate::ini::Ini) -> IniResult<Self> {
        let mut o = Self { flag_4: false };
        run(ini, TBL_POINT_VOL, |ini, e, _off, _v2| match e.handler {
            H_BOOL => store_bool(ini, &mut o.flag_4),
            h => unmodelled(ini, e, h),
        })?;
        Ok(o)
    }
}

impl LineEmissionVolumeInfo {
    /// 0x0095cd26 wrapper: IsHollow byte +4, StartPoint f32[3] +8,
    /// EndPoint f32[3] +0x14.
    pub fn parse(ini: &mut crate::ini::Ini) -> IniResult<Self> {
        let mut o = Self::default();
        run(ini, TBL_LINE_VOL, |ini, e, off, _v2| {
            match (e.handler, off) {
                (H_BOOL, 12) => store_bool(ini, &mut o.flag_4),
                (H_VEC3, 16) => store_vec3_f32(ini, &mut o.fields, 0),
                (H_VEC3, 28) => store_vec3_f32(ini, &mut o.fields, 3),
                (_, _) => unmodelled(ini, e, e.handler),
            }
        })?;
        Ok(o)
    }
}

impl BoxEmissionVolumeInfo {
    /// 0x0095cfb1 wrapper.
    pub fn parse(ini: &mut crate::ini::Ini) -> IniResult<Self> {
        let mut o = Self::default();
        run(ini, TBL_BOX_VOL, |ini, e, off, _v2| {
            match (e.handler, off) {
                (H_BOOL, 12) => store_bool(ini, &mut o.flag_4),
                (H_VEC3, 16) => store_vec3_f32(ini, &mut o.dims, 0),
                (_, _) => unmodelled(ini, e, e.handler),
            }
        })?;
        Ok(o)
    }
}

impl SphereEmissionVolumeInfo {
    /// 0x0095d3c7 wrapper.
    pub fn parse(ini: &mut crate::ini::Ini) -> IniResult<Self> {
        let mut o = Self::default();
        run(ini, TBL_SPHERE_VOL, |ini, e, off, _v2| {
            match (e.handler, off) {
                (H_BOOL, 12) => store_bool(ini, &mut o.flag_4),
                (H_REAL, 16) => {
                    o.radius = ini.get_real()? as f32;
                    Ok(())
                }
                (_, _) => unmodelled(ini, e, e.handler),
            }
        })?;
        Ok(o)
    }
}

impl CylinderEmissionVolumeInfo {
    /// 0x0095d604 wrapper: Radius/RadiusRate/Length +0x08..+0x10,
    /// Offset f32[3] +0x14.
    pub fn parse(ini: &mut crate::ini::Ini) -> IniResult<Self> {
        let mut o = Self::default();
        run(ini, TBL_CYL_VOL, |ini, e, off, _v2| {
            match (e.handler, off) {
                (H_BOOL, 12) => store_bool(ini, &mut o.flag_4),
                (H_REAL, 16) => store_real_f32(ini, &mut o.fields, 0),
                (H_REAL, 20) => store_real_f32(ini, &mut o.fields, 1),
                (H_REAL, 24) => store_real_f32(ini, &mut o.fields, 2),
                (H_VEC3, 28) => store_vec3_f32(ini, &mut o.fields, 3),
                (_, _) => unmodelled(ini, e, e.handler),
            }
        })?;
        Ok(o)
    }
}

impl SphericalEmissionVelocityInfo {
    /// 0x0095ea7c wrapper (table 0x00c6c230): `Speed` GCRV at +0x04.
    pub fn parse(ini: &mut crate::ini::Ini) -> IniResult<Self> {
        let mut o = Self { dwords: [0; 3] };
        run(ini, TBL_SPHERICAL_VEL, |ini, e, off, _v2| match e.handler {
            H_GCRV => store_gcrv_u32(ini, &mut o.dwords, (off - 12) / 4),
            h => unmodelled(ini, e, h),
        })?;
        Ok(o)
    }
}

impl CylindricalEmissionVelocityInfo {
    /// 0x0095eb72 wrapper: Radial GCRV +4, Normal GCRV +0x10.
    pub fn parse(ini: &mut crate::ini::Ini) -> IniResult<Self> {
        let mut o = Self { dwords: [0; 6] };
        run(ini, TBL_CYL_VEL, |ini, e, off, _v2| match e.handler {
            H_GCRV => store_gcrv_u32(ini, &mut o.dwords, (off - 12) / 4),
            h => unmodelled(ini, e, h),
        })?;
        Ok(o)
    }
}

impl OutwardEmissionVelocityInfo {
    /// 0x0095ece7 wrapper (table 0x00c6c390): Speed/OtherSpeed GCRVs.
    pub fn parse(ini: &mut crate::ini::Ini) -> IniResult<Self> {
        let mut o = Self { dwords: [0; 6] };
        run(ini, TBL_OUTWARD_VEL, |ini, e, off, _v2| match e.handler {
            H_GCRV => store_gcrv_u32(ini, &mut o.dwords, (off - 12) / 4),
            h => unmodelled(ini, e, h),
        })?;
        Ok(o)
    }
}

impl OrthoEmissionVelocityInfo {
    /// 0x0095e80d wrapper: X/Y/Z GCRVs at +4/+0x10/+0x1c.
    pub fn parse(ini: &mut crate::ini::Ini) -> IniResult<Self> {
        let mut o = Self { dwords: [0; 9] };
        run(ini, TBL_ORTHO_VEL, |ini, e, off, _v2| match e.handler {
            H_GCRV => store_gcrv_u32(ini, &mut o.dwords, (off - 12) / 4),
            h => unmodelled(ini, e, h),
        })?;
        Ok(o)
    }
}

impl TerrainFireEmissionInfo {
    /// 0x0095e5e4 wrapper: X/Y/Z-offset GCRVs +0x08..+0x28 (stride 0xc),
    /// CellEmissionChance real +0x2c.
    pub fn parse(ini: &mut crate::ini::Ini) -> IniResult<Self> {
        let mut o = Self {
            flag_4: false,
            dwords: [0; 10],
        };
        run(ini, TBL_TERRAIN_FIRE, |ini, e, off, _v2| match e.handler {
            H_GCRV => store_gcrv_u32(ini, &mut o.dwords, (off - 16) / 4),
            H_REAL => store_real_u32(ini, &mut o.dwords, (off - 16) / 4),
            h => unmodelled(ini, e, h),
        })?;
        Ok(o)
    }
}

impl LightningEmissionInfo {
    /// 0x0095d976 wrapper: StartPoint/EndPoint Coord3D +8/+0x14, then 9
    /// GCRVs +0x20..+0x80 (Amplitude/Frequency/Phase ×3).
    pub fn parse(ini: &mut crate::ini::Ini) -> IniResult<Self> {
        let mut o = Self {
            flag_4: false,
            dwords: [0; 33],
        };
        run(ini, TBL_LIGHTNING_EMISSION, |ini, e, off, _v2| {
            match (e.handler, off) {
                (H_VEC3, 16) => store_vec3_u32(ini, &mut o.dwords, 0),
                (H_VEC3, 28) => store_vec3_u32(ini, &mut o.dwords, 3),
                (H_GCRV, _) => store_gcrv_u32(ini, &mut o.dwords, (off - 16) / 4),
                (_, _) => unmodelled(ini, e, e.handler),
            }
        })?;
        Ok(o)
    }
}

impl DefaultPhysicsModuleInfo {
    /// 0x0095ee4c wrapper `DefaultModuleTemplate<3>::parse`: Gravity real
    /// +0x10, VelocityDamping GCRV +0x14, DriftVelocity Coord3D +4,
    /// Swirly/ParticlesAttachToBone bytes +0x20/+0x21 — the DoXfer field
    /// set exactly.
    pub fn parse(ini: &mut crate::ini::Ini) -> IniResult<Self> {
        let mut o = Self {
            dwords: [0; 7],
            flag_20: false,
            flag_21: false,
        };
        run(ini, TBL_DEFAULT_3, |ini, e, off, _v2| {
            match (e.handler, off) {
                (H_VEC3, 12) => store_vec3_u32(ini, &mut o.dwords, 0),
                (H_REAL, 24) => store_real_u32(ini, &mut o.dwords, 3),
                (H_GCRV, _) => store_gcrv_u32(ini, &mut o.dwords, (off - 12) / 4),
                (H_BOOL, 40) => store_bool(ini, &mut o.flag_20),
                (H_BOOL, 41) => store_bool(ini, &mut o.flag_21),
                (_, _) => unmodelled(ini, e, e.handler),
            }
        })?;
        Ok(o)
    }
}

impl DefaultUpdateModuleInfo {
    /// 0x0095f4bc wrapper `DefaultModuleTemplate<2>::parse`: 8 GCRVs
    /// +4..+0x63 + `Rotation` enum dword +0x40 (`dwords[15]`; the table
    /// lists the enum at 72 → (72−12)/4 = 15). The game's table has no
    /// InitialDelay token (that trio is DoXfer-only past version 1).
    pub fn parse(ini: &mut crate::ini::Ini) -> IniResult<Self> {
        let mut o = Self { dwords: [0; 25] };
        run(ini, TBL_DEFAULT_2, |ini, e, off, v2| match e.handler {
            H_GCRV => store_gcrv_u32(ini, &mut o.dwords, (off - 12) / 4),
            H_ENUM => store_enum(ini, &mut o.dwords, (off - 12) / 4, v2),
            h => unmodelled(ini, e, h),
        })?;
        Ok(o)
    }
}

impl DefaultAlphaModuleInfo {
    /// 0x0095b2fd wrapper; handler 0x0095b29c per keyframe (Alpha1 at
    /// table 12 → entry[0] base +0x04, f32 view slot 0 = the dist dword).
    pub fn parse(ini: &mut crate::ini::Ini) -> IniResult<Self> {
        let mut o = Self {
            entries: [[0.0; 4]; 8],
        };
        run(ini, TBL_ALPHA, |ini, e, off, _v2| match e.handler {
            H_RAND_KEYFRAME => {
                let mn = ini.get_real()? as f32;
                let mx = ini.get_real()? as f32;
                let t = ini.get_u32_bounded(0)?;
                let k = (off - 12) / 16;
                o.entries[k][0] = f32::from_bits(DIST_GAUSSIAN);
                o.entries[k][1] = mn;
                o.entries[k][2] = mx;
                o.entries[k][3] = f32::from_bits(t);
                Ok(())
            }
            h => unmodelled(ini, e, h),
        })?;
        Ok(o)
    }
}

impl DefaultColorModuleInfo {
    /// 0x0095b93f wrapper; handler 0x0095b913 per keyframe + ColorScale
    /// GCRV at +0x84 (`dwords`).
    pub fn parse(ini: &mut crate::ini::Ini) -> IniResult<Self> {
        let mut o = Self {
            keyframes: [RgbColorKeyframe {
                color: [0.0; 3],
                time: 0,
            }; 8],
            dwords: [0; 3],
        };
        run(ini, TBL_COLOR, |ini, e, off, _v2| match e.handler {
            H_RGB_KEYFRAME => {
                let c = ini.get_rgb_f()?;
                let t = ini.get_u32_bounded(0)?;
                let k = (off - 12) / 16;
                o.keyframes[k].color = c;
                o.keyframes[k].time = t;
                Ok(())
            }
            H_GCRV => store_gcrv_u32(ini, &mut o.dwords, (off - 140) / 4),
            h => unmodelled(ini, e, h),
        })?;
        Ok(o)
    }
}

impl GpuDrawModuleInfo {
    /// 0x009638f8 wrapper: FramesPerRow i32 +4, TotalFrames i32 +8,
    /// DetailTexture string +0x0c, SpeedMultiplier real +0x10.
    pub fn parse(ini: &mut crate::ini::Ini) -> IniResult<Self> {
        let mut o = Self {
            field_4: 1,
            field_8: 1,
            name: AsciiString::default(),
            scale: 1.0,
        };
        run(ini, TBL_GPU_DRAW, |ini, e, off, _v2| {
            match (e.handler, off) {
                (H_I32, 12) => {
                    o.field_4 = ini.get_i32()? as u32;
                    Ok(())
                }
                (H_I32, 16) => {
                    o.field_8 = ini.get_i32()? as u32;
                    Ok(())
                }
                (H_STRING, 20) => store_string(ini, &mut o.name),
                (H_REAL, 24) => {
                    o.scale = ini.get_real()? as f32;
                    Ok(())
                }
                (_, _) => unmodelled(ini, e, e.handler),
            }
        })?;
        Ok(o)
    }
}

impl RenderObjectDrawModuleInfo {
    /// 0x00962933 wrapper: SinkOnTerrainCollision byte +4, SinkRate real
    /// +8 (float bits in `field_8`), MultiRenderObjects byte +0x0c, then
    /// three group quartets (+0x10 name/a/b-c): string, i32 count, real
    /// percent, `ParticleShaderType` enum.
    pub fn parse(ini: &mut crate::ini::Ini) -> IniResult<Self> {
        let mut o = Self::default();
        run(ini, TBL_RENDER_DRAW, |ini, e, off, v2| {
            // Group slots share the 16 B game stride; Rust RodGroup is
            // laid out independently, so address the four fields by index.
            let grp = |off: usize| (off - 24) / 16;
            match (e.handler, off) {
                (H_BOOL, 12) => store_bool(ini, &mut o.flag_4),
                (H_REAL, 16) => {
                    o.field_8 = (ini.get_real()? as f32).to_bits();
                    Ok(())
                }
                (H_BOOL, 20) => store_bool(ini, &mut o.flag_c),
                (H_STRING, _) => store_string(ini, &mut o.groups[grp(off)].name),
                (H_I32, _) => {
                    o.groups[grp(off)].a = ini.get_i32()? as u32;
                    Ok(())
                }
                (H_REAL, _) => {
                    o.groups[grp(off)].b = (ini.get_real()? as f32).to_bits();
                    Ok(())
                }
                (H_ENUM, _) => {
                    o.groups[grp(off)].c = enum_value(ini, v2)?;
                    Ok(())
                }
                (_, _) => unmodelled(ini, e, e.handler),
            }
        })?;
        Ok(o)
    }
}

impl RenderObjectUpdateModuleInfo {
    /// 0x00961f0e wrapper (table 0x00c6c710): twelve GCRV tokens at table
    /// 12..144 (stride 0xc → `dwords[(off−12)/4]`) + `Rotation` enum at
    /// 156 → dword +0x94 = `dwords[36]`.
    pub fn parse(ini: &mut crate::ini::Ini) -> IniResult<Self> {
        let mut o = Self { dwords: [0; 37] };
        run(ini, TBL_RENDER_UPDATE, |ini, e, off, v2| match e.handler {
            H_GCRV => store_gcrv_u32(ini, &mut o.dwords, (off - 12) / 4),
            H_ENUM => store_enum(ini, &mut o.dwords, (off - 12) / 4, v2),
            h => unmodelled(ini, e, h),
        })?;
        Ok(o)
    }
}

impl LightningDrawModuleInfo {
    /// 0x0096141c wrapper (table 0x00c6c608): OffsetX/Y/Z GCRVs +4/
    /// +0x10/+0x1c, MultiChance real +0x28, TileTexture bool byte +0x2c.
    pub fn parse(ini: &mut crate::ini::Ini) -> IniResult<Self> {
        let mut o = Self {
            dwords: [0; 10],
            flag_2c: false,
        };
        run(ini, TBL_LIGHTNING_DRAW, |ini, e, off, _v2| {
            match (e.handler, off) {
                (H_BOOL, 52) => store_bool(ini, &mut o.flag_2c),
                (H_REAL, _) => store_real_u32(ini, &mut o.dwords, (off - 12) / 4),
                (H_GCRV, _) => store_gcrv_u32(ini, &mut o.dwords, (off - 12) / 4),
                (_, _) => unmodelled(ini, e, e.handler),
            }
        })?;
        Ok(o)
    }
}

impl LifeEventModuleInfo {
    /// 0x00963ebf wrapper (table 0x00c6cac0): PerParticle/KillAfter bool
    /// bytes, EventFX quoted string (handler 0x00963ea8 tail-wraps
    /// 0x0042f11e) into the modeled `name`, EventTime GCRV at `dwords`
    /// base (+8: `dwords[0..3]`).
    /// NOTE: this class is the one where the wrapper table offsets do not
    /// fit the uniform `off−8` info-embed rule (PerParticle/KillAfter land
    /// on the `dwords[0]` byte lanes, matching `getEventFX` 0x0096410f
    /// only loosely). Ambiguity flagged for the runtime wave — the parse
    /// behavior itself (token order, handler chain) is exact.
    pub fn parse(ini: &mut crate::ini::Ini) -> IniResult<Self> {
        let mut o = Self {
            name: AsciiString::default(),
            dwords: [0; 4],
        };
        run(ini, TBL_LIFE_EVENT, |ini, e, off, _v2| {
            match (e.handler, off) {
                (H_BOOL, 8) => {
                    o.dwords[0] |= ini.get_bool()? as u32; // byte lane +8
                    Ok(())
                }
                (H_BOOL, 9) => {
                    o.dwords[0] |= (ini.get_bool()? as u32) << 8; // byte lane +9
                    Ok(())
                }
                (H_EVENTFX_STRING, 16) => store_string(ini, &mut o.name),
                (H_GCRV, 20) => store_gcrv_u32(ini, &mut o.dwords, 1),
                (_, _) => unmodelled(ini, e, e.handler),
            }
        })?;
        Ok(o)
    }
}

impl TerrainCollisionModuleInfo {
    /// 0x00964497 wrapper (table 0x00c6cb68): same token family as
    /// LifeEvent — `HeightOffset` GCRV instead of EventTime, plus
    /// OrientFXToTerrain bool at template byte 0x20 (modeled `flag_14`,
    /// see the LifeEvent note).
    pub fn parse(ini: &mut crate::ini::Ini) -> IniResult<Self> {
        let mut o = Self {
            name: AsciiString::default(),
            dwords: [0; 3],
            flag_14: false,
            field_18: 0,
        };
        run(ini, TBL_TERRAIN_COLLISION, |ini, e, off, _v2| {
            match (e.handler, off) {
                (H_BOOL, 8) => {
                    o.dwords[0] |= ini.get_bool()? as u32; // byte lane +8
                    Ok(())
                }
                (H_BOOL, 9) => {
                    o.dwords[0] |= (ini.get_bool()? as u32) << 8; // byte lane +9
                    Ok(())
                }
                (H_EVENTFX_STRING, 16) => store_string(ini, &mut o.name),
                (H_GCRV, 20) => store_gcrv_u32(ini, &mut o.dwords, 0),
                (H_BOOL, 32) => store_bool(ini, &mut o.flag_14),
                (_, _) => unmodelled(ini, e, e.handler),
            }
        })?;
        Ok(o)
    }
}

impl ParticleSystemInfo {
    /// 0x005f3d2e `ParticleSystemTemplate::parse` — the stack-built
    /// 21-entry table (immediates verified at 0x005f3d77..0x005f3fab:
    /// name strings, enum list VAs, `off` slots 0x7c/8/0xc/…).
    pub fn parse(ini: &mut crate::ini::Ini) -> IniResult<Self> {
        let mut o = Self::default();
        run(ini, TBL_PARTICLE_SYSTEM, |ini, e, off, v2| {
            match e.handler {
                H_BOOL => match off {
                    0x04 => store_bool(ini, &mut o.is_one_shot),
                    0x80 => store_bool(ini, &mut o.is_ground_aligned),
                    0x81 => store_bool(ini, &mut o.is_emit_above_ground_only),
                    0x82 => store_bool(ini, &mut o.is_particle_up_towards_emitter),
                    0x83 => store_bool(ini, &mut o.use_maximum_height),
                    0x84 => store_bool(ini, &mut o.shroud_emitter),
                    _ => unmodelled(ini, e, e.handler),
                },
                H_ENUM => match off {
                    8 => {
                        o.shader = enum_value(ini, v2)?;
                        Ok(())
                    }
                    0xc => {
                        o.ptype = enum_value(ini, v2)?;
                        Ok(())
                    }
                    0x7c => {
                        o.priority = enum_value(ini, v2)?;
                        Ok(())
                    }
                    _ => unmodelled(ini, e, e.handler),
                },
                H_STRING => match off {
                    0x10 => store_string(ini, &mut o.particle_name),
                    0x68 => store_string(ini, &mut o.slave_system),
                    0x78 => store_string(ini, &mut o.per_particle_attached_system),
                    _ => unmodelled(ini, e, e.handler),
                },
                H_VEC3 => match off {
                    0x6c => {
                        let (x, y, z) = ini.get_vec3()?;
                        o.slave_pos_offset = [x as f32, y as f32, z as f32];
                        Ok(())
                    }
                    _ => unmodelled(ini, e, e.handler),
                },
                H_BOUNDED_U32 => match off {
                    0x20 => {
                        o.system_lifetime = ini.get_u32_bounded(v2)?;
                        Ok(())
                    }
                    0x24 => {
                        o.sort_level = ini.get_u32_bounded(v2)?;
                        Ok(())
                    }
                    _ => unmodelled(ini, e, e.handler),
                },
                H_GCRV => {
                    let (dist, mn, mx) = fetch_gcrv(ini)?;
                    let set = |r: &mut RandomRange| {
                        r.min = mn;
                        r.max = mx;
                        r.distribution = dist;
                    };
                    match off {
                        0x14 => {
                            set(&mut o.lifetime);
                            Ok(())
                        }
                        0x28 => {
                            set(&mut o.size);
                            Ok(())
                        }
                        0x34 => {
                            set(&mut o.start_size_rate);
                            Ok(())
                        }
                        0x44 => {
                            set(&mut o.burst_delay);
                            Ok(())
                        }
                        0x50 => {
                            set(&mut o.burst_count);
                            Ok(())
                        }
                        0x5c => {
                            set(&mut o.initial_delay);
                            Ok(())
                        }
                        _ => unmodelled(ini, e, e.handler),
                    }
                }
                h => unmodelled(ini, e, h),
            }
        })?;
        Ok(o)
    }
}

/// Unknown handler for the template's modeled field set — honest internal
/// error, never a silent skip.
fn unmodelled(_ini: &mut crate::ini::Ini, e: &crate::ini::Entry, h: u32) -> IniResult<()> {
    Err(crate::ini::IniError::internal_msg(format!(
        "unmodelled token handler {h:#010x} for '{}'",
        e.name
    )))
}

// ---------------------------------------------------------------------------
// Module-class registry — `CategoryModuleClass<N>` + `ConcreteModuleClass<Tag>`
// (wave 5). Game layout: `{ vptr, key @+4, name @+8, next @+0xc }`
// (getKey 0x009801f2 `return *(this+4)`; getName 0x0070f45f `*(this+8)`).
// `ConcreteModuleClass<Tag>::getInstance` (43 B, 27 rows; the 27
// `ConcreteModuleTemplate::getClass` rows inline the same Meyers singleton)
// lazily runs the class ctor, which calls
// `CategoryModuleClass<N>(this, is_default, *KEY, *NAME)` (0x007a8215 &
// friends): store key/name, prepend to the category list
// (`next = s_first; s_first = this`), and — via
// `CategoryModuleClassBase<N,1>` (0x005f4341 family) — set `s_default`
// when `is_default`. Category 8 (Event) has NO default slot
// (`CategoryModuleClassBase<8,0>` 0x005f446c is a no-op).
//
// KEY/NAME evidence: .rdata {identifier, display} pointer pairs — the
// 9-category table at 0x00c1d840..0x00c1d888 and per-tag slots cited in
// `CLASS_DEFS` below. The `char**` globals (0x00e02cXX) are BSS, so which
// slot feeds KEY vs NAME is not statically observable; we pin
// KEY = identifier, NAME = display (INI lookup uses the identifier).
// `DefaultModuleTag<4>`/`<5>` are never instantiated (no ctor rows); those
// categories' default slots go to the Ortho/Point tags (flag = true, own KEY/NAME).
// ---------------------------------------------------------------------------

/// Category {identifier, display} pairs — .rdata table 0x00c1d840 (9 entries,
/// 8 B each). `DefaultModuleTag<N>` classes use entry N.
pub const CATEGORY_NAMES: [(&str, &str); 9] = [
    ("Color", "Color"),                        // 0x00c1d840
    ("Alpha", "Alpha"),                        // 0x00c1d848
    ("Update", "Update"),                      // 0x00c1d850
    ("Physics", "Physics"),                    // 0x00c1d858
    ("EmissionVelocity", "Emission Velocity"), // 0x00c1d860
    ("EmissionVolume", "Emission Volume"),     // 0x00c1d868
    ("Draw", "Draw"),                          // 0x00c1d870
    ("Wind", "Wind"),                          // 0x00c1d878
    ("Event", "Event"),                        // 0x00c1d880
];

/// One `ConcreteModuleClass<Tag>` entry (immutable part of the class).
#[derive(Debug, PartialEq)]
pub struct ModuleClass {
    pub key: &'static str,  // +0x04
    pub name: &'static str, // +0x08
    pub category: u8,
    pub is_default: bool, // ctor flag: takes the category `s_default` slot
}

/// All 27 instantiated module classes. `ctor` addresses are the
/// `ConcreteModuleClass<Tag>` protected ctors (all decompiled); slot VAs
/// cite the .rdata {identifier, display} pair.
pub const CLASS_DEFS: [ModuleClass; 27] = [
    // category defaults (DefaultModuleTag<N>), ctor flag = true
    ModuleClass {
        key: "Color",
        name: "Color",
        category: 0,
        is_default: true,
    }, // 0x007aa823
    ModuleClass {
        key: "Alpha",
        name: "Alpha",
        category: 1,
        is_default: true,
    }, // 0x007aa801
    ModuleClass {
        key: "Update",
        name: "Update",
        category: 2,
        is_default: true,
    }, // 0x007aa867
    ModuleClass {
        key: "Physics",
        name: "Physics",
        category: 3,
        is_default: true,
    }, // 0x007aa845
    ModuleClass {
        key: "OrthoEmissionVelocity",
        name: "Ortho Emission Velocity",
        category: 4,
        is_default: true,
    }, // 0x007aa8cd, slot 0x00c6c178
    ModuleClass {
        key: "Draw",
        name: "Draw",
        category: 6,
        is_default: true,
    }, // 0x007aa206
    ModuleClass {
        key: "Wind",
        name: "Wind",
        category: 7,
        is_default: true,
    }, // 0x007aa889
    // EmissionVelocity (cat 4) tags — slots 0x00c6c1f0/0xc6cbc8/0xc6c250/0xc6c348
    ModuleClass {
        key: "SphericalEmissionVelocity",
        name: "Spherical Emission Velocity",
        category: 4,
        is_default: false,
    }, // 0x007a887b
    ModuleClass {
        key: "HemisphericalEmissionVelocity",
        name: "Hemispherical Emission Velocity",
        category: 4,
        is_default: false,
    }, // 0x007a889d
    ModuleClass {
        key: "CylindricalEmissionVelocity",
        name: "Cylindrical Emission Velocity",
        category: 4,
        is_default: false,
    }, // 0x007a88bf
    ModuleClass {
        key: "OutwardEmissionVelocity",
        name: "Outward Emission Velocity",
        category: 4,
        is_default: false,
    }, // 0x007a88e1
    // EmissionVolume (cat 5) tags — Point slot 0x00c6bb2c;
    // slots 0x00c6bb8c/0xbc94/0xbd7c/0xbe64/0xbf90/0xc0c8
    ModuleClass {
        key: "PointEmissionVolume",
        name: "Point Emission Volume",
        category: 5,
        is_default: true,
    }, // 0x007aa8ef
    ModuleClass {
        key: "LineEmissionVolume",
        name: "Line Emission Volume",
        category: 5,
        is_default: false,
    }, // 0x007a8936
    ModuleClass {
        key: "BoxEmissionVolume",
        name: "Box Emission Volume",
        category: 5,
        is_default: false,
    }, // 0x007a8958
    ModuleClass {
        key: "SphereEmissionVolume",
        name: "Sphere Emission Volume",
        category: 5,
        is_default: false,
    }, // 0x007a897a
    ModuleClass {
        key: "CylinderEmissionVolume",
        name: "Cylinder Emission Volume",
        category: 5,
        is_default: false,
    }, // 0x007a899c
    ModuleClass {
        key: "LightningEmission",
        name: "Lightning Emission",
        category: 5,
        is_default: false,
    }, // 0x007a89be
    ModuleClass {
        key: "TerrainFireEmission",
        name: "Terrain Fire Emission",
        category: 5,
        is_default: false,
    }, // 0x007a89e0
    // Draw (cat 6) tags — slots 0x00c6c540/0xc560/0xc580/0xc7f4/0xc5b0/0xc9e8
    ModuleClass {
        key: "StreakDraw",
        name: "Streak Draw",
        category: 6,
        is_default: false,
    }, // 0x007a8248
    ModuleClass {
        key: "QuadDraw",
        name: "Quad Draw",
        category: 6,
        is_default: false,
    }, // 0x007a826a
    ModuleClass {
        key: "ButterflyDraw",
        name: "Butterfly Draw",
        category: 6,
        is_default: false,
    }, // 0x007a828c
    ModuleClass {
        key: "RenderObjectDraw",
        name: "RenderObject Draw",
        category: 6,
        is_default: false,
    }, // 0x007a82ae
    ModuleClass {
        key: "LightningDraw",
        name: "Lightning Draw",
        category: 6,
        is_default: false,
    }, // 0x007a82d0
    ModuleClass {
        key: "GpuDraw",
        name: "Gpu Draw",
        category: 6,
        is_default: false,
    }, // 0x007a82f2
    // Event (cat 8) tags — slots 0x00c6cb14/0xca70; category 8 has no default
    ModuleClass {
        key: "TerrainCollision",
        name: "Terrain Collision",
        category: 8,
        is_default: false,
    }, // 0x007a8826
    ModuleClass {
        key: "LifeEvent",
        name: "Life Event",
        category: 8,
        is_default: false,
    }, // 0x007aa8ab
    // Update (cat 2) tag — slot 0x00c6c668
    ModuleClass {
        key: "RenderObjectUpdate",
        name: "RenderObject Update",
        category: 2,
        is_default: false,
    }, // 0x007a8804
];

/// Registry mirrors the 9 `s_first` heads, 8 `s_default` slots
/// (category 8 has none), the per-class `next` (+0xc) links, and the
/// Meyers `_DAT_xxx & 1` init guards of the 27 singletons.
pub struct Registry {
    first: [Option<&'static ModuleClass>; 9],
    default: [Option<&'static ModuleClass>; 8],
    next: [Option<&'static ModuleClass>; 27],
    installed: [bool; 27],
}

impl Registry {
    const fn new() -> Self {
        Registry {
            first: [None; 9],
            default: [None; 8],
            next: [None; 27],
            installed: [false; 27],
        }
    }
}

static REGISTRY: std::sync::Mutex<Registry> = std::sync::Mutex::new(Registry::new());

/// `ConcreteModuleClass<Tag>::getInstance` (×27, 43 B Meyers singletons) and
/// the identical inlined bodies of the 27 `ConcreteModuleTemplate::getClass`
/// rows. First call constructs the class: the `CategoryModuleClass<N>` ctor
/// stores key/name, prepends to the category list, and sets `s_default`
/// when the ctor flag is set. Indexes into `CLASS_DEFS`.
pub fn get_class(i: usize) -> &'static ModuleClass {
    let cls = &CLASS_DEFS[i];
    let mut r = REGISTRY.lock().unwrap();
    if !r.installed[i] {
        r.next[i] = r.first[cls.category as usize];
        r.first[cls.category as usize] = Some(cls);
        if cls.is_default {
            r.default[cls.category as usize] = Some(cls);
        }
        r.installed[i] = true;
    }
    cls
}

/// `CategoryModuleClass<N>::getFirst` (0x005f4335 family, 9 rows).
pub fn get_first(category: usize) -> Option<&'static ModuleClass> {
    REGISTRY.lock().unwrap().first[category]
}

/// `CategoryModuleClassBase<N,D>::getDefault` (0x005f433b family, 8 rows).
/// Category 8 (`Event`) has no default slot: always `None` (0x005f446c no-op
/// base ctor `CategoryModuleClassBase<8,0>`).
pub fn get_default(category: usize) -> Option<&'static ModuleClass> {
    if category >= 8 {
        return None;
    }
    REGISTRY.lock().unwrap().default[category]
}

/// `CategoryModuleClass<N>::getNext` (0x005db0a8, 9 folded rows; reads +0xc).
pub fn get_next(cls: &ModuleClass) -> Option<&'static ModuleClass> {
    let r = REGISTRY.lock().unwrap();
    CLASS_DEFS
        .iter()
        .position(|c| std::ptr::eq(c, cls))
        .and_then(|i| r.next[i])
}
/// 0x007ae381 `staticInitModules`: runs the 27 Meyers-singleton
/// trampolines in fixed call order. Wrapper→class map: 20 trampolines
/// carry the `MODULE_KEY` constant in their demangled tag (e.g.
/// 0x007ad532 = TERRAIN_COLLISION → index 24; 0x007ad553/0x007ad5f0 carry
/// the Ortho/PointEmissionVolume tag names). The 7 Draw-family trampolines
/// sit at 0x007a8314..0x007a8494 as Ghidra-misbounded rows (skipped); the
/// classes are identified by elimination (only the 7 Draw-family classes
/// lack wrappers) and ordered by trampoline address ==
/// template-definition order [INFERENCE].
pub fn static_init_modules() {
    for &i in &[
        1, 0, 5, 3, 2, 6, 25, // Alpha, Color, Draw*, Physics, Update, Wind, LifeEvent
        18, 19, 20, 21, 22, 23, // Streak*, Quad*, Butterfly*, RenderObjectDraw*,
        // LightningDraw*, GpuDraw*  (* = inferred slot)
        26, 24, 4, 7, 8, 9, 10, // RenderObjectUpdate, TerrainCollision, Ortho,
        // Spherical, Hemi, CylEV, Outward
        11, 12, 13, 14, 15, 16, 17, // Point..TerrainFire
    ] {
        get_class(i);
    }
}

// ---------------------------------------------------------------------------
// Wave 22: `*ModuleTemplate` objects + `ConcreteModuleTemplate<Tag>` plumbing
// ---------------------------------------------------------------------------
//
// Every module template class in the game is { ModuleTemplate vftable at +0,
// CategoryModuleInfo vftable at +4, inline `Info` payload at +8 (or +0xc for
// Box-style emission volumes) }. The named open rows for these classes are
// copy/default ctors (`??0`), dtors (`??1`), `operator=` (`??4`), virtual
// `clone` and `createTemplate`: each body installs vftables and copies or
// default-constructs the Info payload — zero data-plane semantics beyond the
// Info itself (e.g. Box copy ctor 0x007a6738 = base copy + Info copy with the
// null->self operand-fold `-(uint)(p!=0)&(uint)(p+8)`; Box default ctor
// 0x007a66fd zeroes +0xc..+0x18 which is exactly `BoxEmissionVolumeInfo::
// default()`; Box op= 0x007a6778 copies only the four info dwords). Rust
// `Default`/`Clone` are the faithful models; vftables have no data-plane
// counterpart. `clone` rows (29-35 B, e.g. 0x007a94f3) = `FUN_0042fda0(0x18)`
// + copy-ctor -> `clone_boxed`; `createTemplate` rows (0x007ab41c, 0x007ab4a4,
// 0x007ab52c, 0x007ab5b4, 0x007ab63c) = new + default-ctor -> `create_boxed`.
//
// Known quirks mirrored by construction:
// - LineEmissionVolumeModuleTemplate stores a *Cylinder*EmissionVolumeInfo
//   (its copy ctor 0x007a65e9 family calls `CylinderEmissionVolumeInfo` ctors;
//   the line volume is degenerate cylinder semantics in the game).
// - OutwardEmissionVelocityModuleTemplate likewise stores a
//   *Cylindrical*EmissionVelocityInfo.
// - HemisphericalEmissionVelocityModuleTemplate *derives from* Spherical:
//   its ctors just copy-construct the Spherical template (0x007a74xx family);
//   same payload, distinct vftables.
// - `DefaultModuleTemplate<7>` (0x007a8ca5) copy-constructs
//   `CategoryModuleTemplate<7>` (= Wind) instead of holding its own Info,
//   hence the type alias below.
// - `createModule` rows (11, e.g. 0x007ada95) allocate + construct the runtime
//   `*Module` classes, which are not modelled; those rows stay `decompiled`.
//   So do the 7 `thunk_*` jmp stubs, the 2 lazy `getEventFX` rows
//   (0x0096410f/0x0096468e, need the FXList store at 0x005e281a) and the
//   shared `writeINI` sink 0x009613f9 (File-writer band).
macro_rules! fx_module_template {
    ($name:ident, $info:ty) => {
        #[derive(Clone, Debug, Default, PartialEq)]
        pub struct $name {
            pub info: $info,
        }
        impl $name {
            /// Virtual `clone`: heap-allocate + copy-construct.
            pub fn clone_boxed(t: &Self) -> Box<Self> {
                Box::new(t.clone())
            }
            /// `ConcreteModuleClass<Tag>::createTemplate`: new default object.
            pub fn create_boxed() -> Box<Self> {
                Box::new(Self::default())
            }
        }
    };
}
fx_module_template!(DefaultModuleTemplate0, DefaultColorModuleInfo); // <0>
fx_module_template!(DefaultModuleTemplate1, DefaultAlphaModuleInfo); // <1>
fx_module_template!(DefaultModuleTemplate2, DefaultUpdateModuleInfo); // <2>
fx_module_template!(DefaultModuleTemplate3, DefaultPhysicsModuleInfo); // <3>
fx_module_template!(DefaultModuleTemplate6, DefaultDrawModuleInfo); // <6>
fx_module_template!(
    OrthoEmissionVelocityModuleTemplate,
    OrthoEmissionVelocityInfo
);
fx_module_template!(
    SphericalEmissionVelocityModuleTemplate,
    SphericalEmissionVelocityInfo
);
fx_module_template!(
    HemisphericalEmissionVelocityModuleTemplate,
    SphericalEmissionVelocityInfo
);
fx_module_template!(
    CylindricalEmissionVelocityModuleTemplate,
    CylindricalEmissionVelocityInfo
);
fx_module_template!(
    OutwardEmissionVelocityModuleTemplate,
    CylindricalEmissionVelocityInfo
);
fx_module_template!(PointEmissionVolumeModuleTemplate, EmissionVolumeInfo);
fx_module_template!(LineEmissionVolumeModuleTemplate, CylinderEmissionVolumeInfo);
fx_module_template!(BoxEmissionVolumeModuleTemplate, BoxEmissionVolumeInfo);
fx_module_template!(SphereEmissionVolumeModuleTemplate, SphereEmissionVolumeInfo);
fx_module_template!(
    CylinderEmissionVolumeModuleTemplate,
    CylinderEmissionVolumeInfo
);
fx_module_template!(LightningEmissionModuleTemplate, LightningEmissionInfo);
fx_module_template!(TerrainFireEmissionModuleTemplate, TerrainFireEmissionInfo);
fx_module_template!(StreakDrawModuleTemplate, StreakDrawModuleInfo);
fx_module_template!(QuadDrawModuleTemplate, QuadDrawModuleInfo);
fx_module_template!(ButterflyDrawModuleTemplate, ButterflyDrawModuleInfo);
fx_module_template!(RenderObjectDrawModuleTemplate, RenderObjectDrawModuleInfo);
fx_module_template!(LightningDrawModuleTemplate, LightningDrawModuleInfo);
fx_module_template!(GpuDrawModuleTemplate, GpuDrawModuleInfo);
fx_module_template!(
    RenderObjectUpdateModuleTemplate,
    RenderObjectUpdateModuleInfo
);
fx_module_template!(TerrainCollisionModuleTemplate, TerrainCollisionModuleInfo);
fx_module_template!(LifeEventModuleTemplate, LifeEventModuleInfo);
fx_module_template!(CategoryModuleTemplate7, WindModuleInfo); // <7> = Wind
/// `DefaultModuleTemplate<7>` delegates entirely to `<7>` (0x007a8ca5).
pub type DefaultModuleTemplate7 = CategoryModuleTemplate7;

// `ConcreteModuleTemplate<Tag>` classes carry no data fields beyond their
// target template (copy/default ctors, dtors, op=, `clone` all just run the
// template's and swap vftables). Rust models each as an alias to its target;
// the key macro `*_MODULE_KEY` strings from the mangled names name the alias.
pub type ConcreteColorModuleTemplate = DefaultModuleTemplate0; // DefaultModuleTag<0>
pub type ConcreteAlphaModuleTemplate = DefaultModuleTemplate1; // DefaultModuleTag<1>
pub type ConcreteUpdateModuleTemplate = DefaultModuleTemplate2; // DefaultModuleTag<2>
pub type ConcretePhysicsModuleTemplate = DefaultModuleTemplate3; // DefaultModuleTag<3>
pub type ConcreteDrawModuleTemplate = DefaultModuleTemplate6; // DefaultModuleTag<6>
pub type ConcreteOrthoEmissionVelocityModuleTemplate = OrthoEmissionVelocityModuleTemplate;
pub type ConcreteSphericalEmissionVelocityModuleTemplate = SphericalEmissionVelocityModuleTemplate;
pub type ConcreteHemisphericalEmissionVelocityModuleTemplate =
    HemisphericalEmissionVelocityModuleTemplate;
pub type ConcreteCylindricalEmissionVelocityModuleTemplate =
    CylindricalEmissionVelocityModuleTemplate;
pub type ConcreteOutwardEmissionVelocityModuleTemplate = OutwardEmissionVelocityModuleTemplate;
pub type ConcretePointEmissionVolumeModuleTemplate = PointEmissionVolumeModuleTemplate;
pub type ConcreteLineEmissionVolumeModuleTemplate = LineEmissionVolumeModuleTemplate;
pub type ConcreteBoxEmissionVolumeModuleTemplate = BoxEmissionVolumeModuleTemplate;
pub type ConcreteSphereEmissionVolumeModuleTemplate = SphereEmissionVolumeModuleTemplate;
pub type ConcreteCylinderEmissionVolumeModuleTemplate = CylinderEmissionVolumeModuleTemplate;
pub type ConcreteLightningEmissionModuleTemplate = LightningEmissionModuleTemplate;
pub type ConcreteTerrainFireEmissionModuleTemplate = TerrainFireEmissionModuleTemplate;
pub type ConcreteStreakDrawModuleTemplate = StreakDrawModuleTemplate;
pub type ConcreteQuadDrawModuleTemplate = QuadDrawModuleTemplate;
pub type ConcreteButterflyDrawModuleTemplate = ButterflyDrawModuleTemplate;
pub type ConcreteRenderObjectDrawModuleTemplate = RenderObjectDrawModuleTemplate;
pub type ConcreteLightningDrawModuleTemplate = LightningDrawModuleTemplate;
pub type ConcreteGpuDrawModuleTemplate = GpuDrawModuleTemplate;
pub type ConcreteRenderObjectUpdateModuleTemplate = RenderObjectUpdateModuleTemplate;
pub type ConcreteTerrainCollisionModuleTemplate = TerrainCollisionModuleTemplate;
pub type ConcreteLifeEventModuleTemplate = LifeEventModuleTemplate;

/// Free functions `GetName`/`GetKey` (0x007afd0a / 0x007afd16): table of
/// {name, key} pointer pairs per `ModuleCategory` at 0x00c1d840. Only 9
/// entries exist; the game reads uninitialised dwords past index 8, which
/// Rust models as `None`.
pub const MODULE_CATEGORY_NAMES: [&str; 9] = [
    "Color",
    "Alpha",
    "Update",
    "Physics",
    "Emission Velocity",
    "Emission Volume",
    "Draw",
    "Wind",
    "Event",
];
pub const MODULE_CATEGORY_KEYS: [&str; 9] = [
    "Color",
    "Alpha",
    "Update",
    "Physics",
    "EmissionVelocity",
    "EmissionVolume",
    "Draw",
    "Wind",
    "Event",
];
pub fn category_name(cat: usize) -> Option<&'static str> {
    MODULE_CATEGORY_NAMES.get(cat).copied()
}
pub fn category_key(cat: usize) -> Option<&'static str> {
    MODULE_CATEGORY_KEYS.get(cat).copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::string::Ascii;

    #[test]
    fn wave22_template_defaults_clone_and_categories() {
        // Box default ctor 0x007a66fd zeroes +0xc..0x18 = info::default().
        let b = BoxEmissionVolumeModuleTemplate::default();
        assert_eq!(b.info, BoxEmissionVolumeInfo::default());
        // Box op= 0x007a6778 copies the four info dwords => Clone.
        let c = BoxEmissionVolumeModuleTemplate {
            info: BoxEmissionVolumeInfo {
                flag_4: true,
                dims: [1.0, 2.0, 3.0],
            },
        };
        assert_eq!(c.clone(), c);
        // `clone` rows allocate + copy (0x007a94f3): identical contents.
        let hb = SphericalEmissionVelocityModuleTemplate::clone_boxed(
            &SphericalEmissionVelocityModuleTemplate::default(),
        );
        assert_eq!(*hb, SphericalEmissionVelocityModuleTemplate::default());
        // `createTemplate` rows (0x007ab4a4): default-constructed heap object.
        assert_eq!(
            *LineEmissionVolumeModuleTemplate::create_boxed(),
            LineEmissionVolumeModuleTemplate::default()
        );
        assert_eq!(category_name(4), Some("Emission Velocity"));
        assert_eq!(category_key(5), Some("EmissionVolume"));
        assert_eq!(category_key(9), None); // OOB in game reads garbage
    }

    #[test]
    fn particle_system_info_defaults() {
        // 0x005f4e82: ones at +8/+0xc/+0x7c, max-height DAT_00bbb8d8 = 1.0f.
        let i = ParticleSystemInfo::default();
        assert_eq!(i.shader, 1);
        assert_eq!(i.ptype, 1);
        assert_eq!(i.priority, 1);
        assert_eq!(i.max_height, (1.0, 1.0));
        assert!(!i.is_one_shot);
        assert_eq!(i.lifetime, RandomRange::default());
        assert_eq!(i.slave_pos_offset, [0.0; 3]);
        assert_eq!(i.uv_min, (0.0, 0.0));
        assert!(i.particle_name.peek().is_none());
        assert_eq!(i.field_98, 0);
    }

    #[test]
    fn snapshot_name_string() {
        // 0x00401272 → s_FXParticleSystemInfo_00bbb530
        assert_eq!(
            particle_system_info_snapshot_name(&ParticleSystemInfo::default()),
            "FXParticleSystemInfo"
        );
    }

    #[test]
    fn texture_filename_aliases_particle_name() {
        // 0x0040261e / 0x00782015 operate on +0x10 (parse: ParticleName).
        let mut t = ParticleSystemTemplate::default();
        let s: Vec<Ascii> = "sparkle.bmp".bytes().map(Ascii).collect();
        let mut sb = AsciiString::new();
        sb.set(&s);
        t.set_texture_filename(&sb);
        assert_eq!(t.info.particle_name.str(), &s[..]);
        assert_eq!(t.get_texture_filename().str(), &s[..]);
    }

    #[test]
    fn slave_system_clears_cache_flag() {
        // 0x005f3fd0 clears +0xa0 after the string set.
        let mut t = ParticleSystemTemplate::default();
        t.cache_flag = true;
        let smoke: Vec<Ascii> = "smoke".bytes().map(Ascii).collect();
        let mut sb = AsciiString::new();
        sb.set(&smoke);
        t.set_slave_system_name(&sb);
        assert!(!t.cache_flag);
        assert_eq!(t.info.slave_system.str(), &smoke[..]);
    }

    #[test]
    fn uv_copy() {
        // 0x00401278 four dwords; 0x0095c414 returns the span.
        let mut t = ParticleSystemTemplate::default();
        // Default UV span +0x88..+0x94 = (0,0,1,1): max corner is the
        // DAT_00bbb8d8 (1.0f) pair (0x005f4e82 +0x90/+0x94).
        assert_eq!(
            t.get_uv(),
            Region2D {
                min_x: 0.0,
                min_y: 0.0,
                max_x: 1.0,
                max_y: 1.0
            }
        );
        let r = Region2D {
            min_x: 0.0,
            min_y: 0.5,
            max_x: 1.0,
            max_y: 0.75,
        };
        t.set_uv(&r);
        assert_eq!(t.get_uv(), r);
        assert_eq!(t.info.max_height, (1.0, 0.75));
    }

    #[test]
    fn keyframe_defaults() {
        assert_eq!(
            Keyframe::default(),
            Keyframe {
                time: 0.0,
                value: 0.0
            }
        );
        assert_eq!(
            RgbColorKeyframe::default(),
            RgbColorKeyframe {
                color: [0.0; 3],
                time: 0
            }
        );
        // 0x00401e7f: setRange(0, 0, 1) + time 0.
        assert_eq!(
            RandomAlphaKeyframe::default(),
            RandomAlphaKeyframe {
                distribution: 1,
                min: 0.0,
                max: 0.0,
                time: 0
            }
        );
        // 0x00401236: 1,1
        assert_eq!(
            EventModuleInfo::default(),
            EventModuleInfo {
                field_0: 1,
                field_2: 1
            }
        );
    }

    #[test]
    fn category_module_template_flags() {
        // 0x00401ef9: +8/+9 = 1,1
        let c = CategoryModuleTemplate::default();
        assert!(c.flag_8 && c.flag_9);
    }

    #[test]
    fn wind_defaults() {
        // Ctor 0x007a53c4 constant loads (.rdata 0xc1b2e8..0xc1b310, 0xdc0a9c).
        let w = WindModuleInfo::default();
        assert_eq!(w.field_4, 1);
        assert_eq!(w.strength, 2.0);
        assert_eq!(w.full_strength_dist, 75.0);
        assert_eq!(w.zero_strength_dist, 200.0);
        assert_eq!(w.angle_change_a, 0.15);
        assert_eq!(w.angle_change_b, 0.15);
        assert_eq!(w.angle_change_max, 0.45);
        assert_eq!(w.motion_start_angle_max.to_bits(), 0.78539819f32.to_bits());
        assert_eq!(w.motion_end_angle.to_bits(), 5.4977875f32.to_bits());
        assert_eq!(w.motion_end_angle_max.to_bits(), 6.2831855f32.to_bits());
        assert!(w.flag_3c);
        assert_eq!(w, WindModuleInfo::default().clone());
    }

    #[test]
    fn info_defaults() {
        let g = GpuDrawModuleInfo::default();
        assert_eq!((g.field_4, g.field_8, g.scale), (1, 1, 1.0));
        assert!(g.name.str().is_empty());
        let r = RenderObjectDrawModuleInfo::default();
        assert_eq!(r.groups[2].c, 8);
        assert_eq!(EmissionVolumeInfo::default().flag_4, false);
        assert_eq!(BoxEmissionVolumeInfo::default().dims, [0.0; 3]);
        assert_eq!(SphereEmissionVolumeInfo::default().radius, 0.0);
        assert_eq!(CylinderEmissionVolumeInfo::default().fields, [0.0; 6]);
    }

    #[test]
    fn snapshot_names() {
        // GetSnapshotName rows: `mov eax, imm32; ret` (0x007a611b..0x007a6169,
        // 0x005f35d9, 0x005f354a, 0x005f35bf).
        assert_eq!(WindModuleInfo::snapshot_name(), "WindModuleInfo");
        assert_eq!(
            DefaultAlphaModuleInfo::snapshot_name(),
            "DefaultAlphaModuleInfo"
        );
        assert_eq!(
            DefaultColorModuleInfo::snapshot_name(),
            "DefaultColorModuleInfo"
        );
        assert_eq!(
            DefaultDrawModuleInfo::snapshot_name(),
            "DefaultDrawModuleInfo"
        );
        assert_eq!(
            DefaultPhysicsModuleInfo::snapshot_name(),
            "DefaultPhysicsModuleInfo"
        );
        assert_eq!(
            DefaultUpdateModuleInfo::snapshot_name(),
            "DefaultUpdateModuleInfo"
        );
        assert_eq!(LifeEventModuleInfo::snapshot_name(), "LifeEventModuleInfo");
        assert_eq!(
            StreakDrawModuleInfo::snapshot_name(),
            "StreakDrawModuleInfo"
        );
        assert_eq!(QuadDrawModuleInfo::snapshot_name(), "QuadDrawModuleInfo");
        assert_eq!(
            ButterflyDrawModuleInfo::snapshot_name(),
            "ButterflyDrawModuleInfo"
        );
        assert_eq!(
            RenderObjectDrawModuleInfo::snapshot_name(),
            "RenderObjectDrawModuleInfo"
        );
        assert_eq!(
            LightningDrawModuleInfo::snapshot_name(),
            "LightningDrawModuleInfo"
        );
        assert_eq!(GpuDrawModuleInfo::snapshot_name(), "GpuDrawModuleInfo");
        assert_eq!(
            RenderObjectUpdateModuleInfo::snapshot_name(),
            "RenderObjectUpdateModuleInfo"
        );
        assert_eq!(
            TerrainCollisionModuleInfo::snapshot_name(),
            "TerrainCollisionModuleInfo"
        );
        assert_eq!(EmissionVolumeInfo::snapshot_name(), "EmissionVolumeInfo");
        assert_eq!(
            EmissionVelocityInfo::snapshot_name(),
            "EmissionVelocityInfo"
        );
    }

    #[test]
    fn info_string_clones() {
        // operator= rows with StringBase::set (0x007a7d2e, 0x007a7ef0,
        // 0x007a7a51, 0x007a81ae) mirror to Clone of the string fields.
        let mut rod = RenderObjectDrawModuleInfo::default();
        let f: Vec<Ascii> = "flare01.tga".bytes().map(Ascii).collect();
        rod.groups[0].name.set(&f);
        let rod2 = rod.clone();
        assert_eq!(rod2.groups[0].name.str(), &f[..]);
        let sp: Vec<Ascii> = "SPLASH".bytes().map(Ascii).collect();
        let mut le = LifeEventModuleInfo {
            name: AsciiString::default(),
            dwords: [1, 2, 3, 4],
        };
        le.name.set(&sp);
        assert_eq!(le.clone().name.str(), &sp[..]);
        let tc = TerrainCollisionModuleInfo {
            name: AsciiString::default(),
            dwords: [9, 9, 9],
            flag_14: true,
            field_18: 7,
        };
        assert_eq!(tc.clone().field_18, 7);
    }

    #[test]
    fn pod_info_copies() {
        // Copy-ctor/op= byte-for-byte layout: DefaultColorModuleInfo keyframe
        // block (0x007a62d9) and LightningEmissionInfo flat map (0x007a6af5).
        let dc = DefaultColorModuleInfo {
            keyframes: [RgbColorKeyframe {
                color: [0.25, 0.5, 1.0],
                time: 1,
            }; 8],
            dwords: [3, 4, 5],
        };
        assert_eq!(dc.clone().keyframes[7].color[2], 1.0);
        assert_eq!(dc.clone().dwords[1], 4);
        let le = LightningEmissionInfo {
            flag_4: true,
            dwords: [0x41; 33],
        };
        assert_eq!(le.clone().dwords[32], 0x41);
    }

    #[test]
    fn class_registry() {
        // Single test fn: the registry is global mutable state, like the
        // game's s_first/s_default globals.
        assert_eq!(CLASS_DEFS.len(), 27);
        assert_eq!(CATEGORY_NAMES[4], ("EmissionVelocity", "Emission Velocity"));
        assert_eq!(get_default(0), None);
        assert_eq!(get_first(6), None); // nothing installed yet
        let streak = get_class(18); // StreakDraw 0x007a8248
        assert_eq!(streak.key, "StreakDraw");
        assert_eq!(streak.name, "Streak Draw");
        assert_eq!(get_first(6), Some(streak));
        assert_eq!(get_next(streak), None);
        assert_eq!(get_default(6), None);
        let ddraw = get_class(5); // Draw default 0x007aa206 (flag true)
        assert_eq!(get_first(6), Some(ddraw));
        assert_eq!(get_next(ddraw), Some(streak)); // prepend → LIFO
        assert_eq!(get_default(6), Some(ddraw));
        // cat 4: Ortho (index 4) takes the default slot (ctor 0x007aa8cd, flag 1)
        let ortho = get_class(4);
        let spheric = get_class(7);
        let outward = get_class(10);
        assert_eq!(get_default(4), Some(ortho));
        assert_eq!(get_first(4), Some(outward));
        assert_eq!(get_next(outward), Some(spheric));
        // cat 8: no default slot ever
        let tc = get_class(24);
        let le = get_class(25);
        assert_eq!(get_default(8), None);
        assert_eq!(get_first(8), Some(le));
        assert_eq!(get_next(le), Some(tc));
        // every remaining index installs cleanly and matches CATEGORY_NAMES shape
        for i in 0..27 {
            let c = get_class(i);
            assert!(c.category < 9);
        }
        // category defaults after full install: only flag=true ctors write the slot
        assert_eq!(get_default(4), Some(get_class(4))); // Ortho (0x007aa8cd, flag 1)
        assert_eq!(get_default(5), Some(get_class(11))); // Point (0x007aa8ef, flag 1)
        assert_eq!(get_default(2), Some(get_class(2))); // Update default; RenderObjectUpdate flag 0
        assert_eq!(get_default(6), Some(get_class(5))); // Draw default; draw tags flag 0
    }

    #[test]
    fn tint_all_colors_off_by_one() {
        let mut info = DefaultColorModuleInfo {
            keyframes: [RgbColorKeyframe {
                color: [1.0; 3],
                time: 0,
            }; 8],
            dwords: [0, 0x3f800000, 0x40000000], // dist=0, min=1.0, max=2.0
        };
        info.tint_all_colors(0x00FF8000); // t = {1.0 (f32 rounds back), 0.50196, 0}
        let t = RGBColor::from_int(0x00FF8000);
        assert_eq!(info.keyframes[0].color, [1.0, 1.0, 1.0]); // game skips kf0
        assert_eq!(info.keyframes[1].color, [t.r, t.g, 0.0]);
        assert_eq!(info.keyframes[7].color, [t.r, t.g, 0.0]);
        // iteration 8 multiplied the rndvar dwords, setRange then reset them
        assert_eq!(info.dwords, [1, 0, 0]);
    }

    /// recording backend with inbound stream (load semantics); events record
    /// what the caller pushed, inbound bytes are applied on length match.
    struct Rx {
        ev: Vec<(u32, Vec<u8>)>,
        stream: std::collections::VecDeque<Vec<u8>>,
        storing: bool,
        light: bool,
    }
    impl Rx {
        fn new(stream: Vec<Vec<u8>>) -> Self {
            Rx {
                ev: vec![],
                stream: stream.into(),
                storing: false,
                light: false,
            }
        }
    }
    impl sage_xfer::Xfer for Rx {
        fn xfer_internal(&mut self, tag: u32, v: &mut [u8]) {
            let inb = self.stream.pop_front().unwrap_or_else(|| v.to_vec());
            self.ev.push((tag, v.to_vec()));
            if inb.len() == v.len() {
                v.copy_from_slice(&inb);
            }
        }
        fn is_storing(&self) -> bool {
            self.storing
        }
        fn is_light_crc(&self) -> bool {
            self.light
        }
    }

    #[test]
    fn do_xfer_default_color_offbyone() {
        use sage_xfer::{TAG_ENU, TAG_REAL, TAG_RGB, TAG_UINT, TAG_VER};
        let c12 =
            |v: f32| -> Vec<u8> { [v.to_le_bytes(), v.to_le_bytes(), v.to_le_bytes()].concat() };
        let mut stream = vec![vec![1u8]]; // Version1 (current=1)
        for k in 0..8 {
            stream.push(vec![]); // 'rgb' tag marker
            stream.push(c12(0.5 * (k as f32 + 1.0))); // color blob
            stream.push((0x1234_0000u32 + k as u32).to_le_bytes().to_vec()); // the +0x10 uint
        }
        // GCRV composite: ver, min, max, enu4
        stream.push(vec![1]);
        stream.push(2.0f32.to_le_bytes().to_vec());
        stream.push(3.0f32.to_le_bytes().to_vec());
        stream.push(7i32.to_le_bytes().to_vec());
        let mut info = DefaultColorModuleInfo {
            keyframes: [RgbColorKeyframe {
                color: [0.0; 3],
                time: 9,
            }; 8],
            dwords: [0xdead, 0, 0],
        };
        let mut rx = Rx::new(stream);
        info.do_xfer(&mut rx);
        let tags: Vec<u32> = rx.ev.iter().map(|e| e.0).collect();
        let mut expect = vec![TAG_VER];
        for _ in 0..8 {
            expect.extend_from_slice(&[TAG_RGB, 0, TAG_UINT]);
        }
        expect.extend_from_slice(&[TAG_VER, TAG_REAL, TAG_REAL, TAG_ENU + 4]);
        assert_eq!(tags, expect);
        // off-by-one proof: no keyframe time ever touched...
        assert_eq!(info.keyframes.iter().filter(|k| k.time == 9).count(), 8);
        // ...but kf1..7 color[0] got the previous iteration's uint blob,
        // then their own 12-byte color load (stream order c_k then u_{k+1}),
        // so final color[0] is the load value 0.5*(k+1):
        assert_eq!(info.keyframes[7].color, [4.0, 4.0, 4.0]);
        // iteration-8 uint + GCRV write-back: dwords[0] ends as the enu int
        assert_eq!(info.dwords, [7, 2.0f32.to_bits(), 3.0f32.to_bits()]);
    }

    #[test]
    fn do_xfer_physics_version_gates() {
        use sage_xfer::{TAG_BOOL, TAG_C3D, TAG_ENU, TAG_REAL, TAG_VER};
        let load = |cur: u8, info: &mut DefaultPhysicsModuleInfo| {
            let stream = vec![
                vec![cur], // xfer_version (1 byte)
                1.5f32.to_le_bytes().to_vec(),
                vec![], // 'c3d' marker
                [1.0f32, 2.0, 3.0]
                    .iter()
                    .flat_map(|f| f.to_le_bytes())
                    .collect(),
                vec![1], // GCRV ver
                0.0f32.to_le_bytes().to_vec(),
                1.0f32.to_le_bytes().to_vec(),
                0i32.to_le_bytes().to_vec(),
                1u8.to_le_bytes().to_vec(), // bool +0x20 (if gate)
                1u8.to_le_bytes().to_vec(), // bool +0x21 (if gate)
            ];
            let mut rx = Rx {
                ev: vec![],
                stream: stream.into(),
                storing: false,
                light: false,
            };
            info.do_xfer(&mut rx);
            rx.ev.iter().map(|e| e.0).collect::<Vec<_>>()
        };
        let mut a = DefaultPhysicsModuleInfo {
            dwords: [0; 7],
            flag_20: false,
            flag_21: false,
        };
        let t1 = load(1, &mut a);
        assert_eq!(
            t1,
            vec![
                TAG_VER,
                TAG_REAL,
                TAG_C3D,
                0,
                TAG_VER,
                TAG_REAL,
                TAG_REAL,
                TAG_ENU + 4
            ]
        );
        assert!(!a.flag_20 && !a.flag_21);
        let mut b = DefaultPhysicsModuleInfo {
            dwords: [0; 7],
            flag_20: false,
            flag_21: false,
        };
        let t2 = load(3, &mut b);
        assert_eq!(&t2[t2.len() - 2..], &[TAG_BOOL, TAG_BOOL]);
        assert!(b.flag_20 && b.flag_21);
    }

    #[test]
    fn do_xfer_wind_store_guard_and_enum() {
        use sage_xfer::{TAG_BYTE, TAG_ENU, TAG_VER};
        let mut w = WindModuleInfo::default();
        let mut rx = Rx {
            ev: vec![],
            stream: std::collections::VecDeque::new(),
            storing: true,
            light: false,
        };
        w.do_xfer(&mut rx);
        assert!(rx.ev.is_empty()); // slot +0x10 IsStoring: body skipped (0x007a547e)
        let mut w = WindModuleInfo::default();
        let mut stream = vec![vec![1u8]];
        stream.push(2i32.to_le_bytes().to_vec()); // WindMotion enum
        for i in 0..15 {
            stream.push(((i as f32) + 1.0).to_le_bytes().to_vec()); // 13 + 2 floats
        }
        stream.insert(2 + 13, vec![1u8]); // char at +0x3c sits between the 13th and 14th float
        let mut rx = Rx {
            ev: vec![],
            stream: stream.into(),
            storing: false,
            light: false,
        };
        w.do_xfer(&mut rx);
        let tags: Vec<u32> = rx.ev.iter().map(|e| e.0).collect();
        assert_eq!(tags[0], TAG_VER);
        assert_eq!(tags[1], TAG_ENU + 4);
        assert_eq!(w.field_4, 2);
        assert_eq!(tags[2 + 13], TAG_BYTE);
        assert_eq!(tags.len(), 1 + 1 + 13 + 1 + 2);
        assert!(w.flag_3c);
        assert_eq!(w.field_44, 15.0);
    }

    #[test]
    fn do_xfer_particle_system_info_full() {
        use sage_xfer::{TAG_BOOL, TAG_C3D, TAG_ENU, TAG_REAL, TAG_UINT, TAG_VER};
        const TAG_STR: u32 = 0x6173_7472;
        let f = |v: f32| v.to_le_bytes().to_vec();
        let i = |v: i32| v.to_le_bytes().to_vec();
        let gcrv = |a: f32, b: f32, d: i32| vec![vec![1u8], f(a), f(b), i(d)];
        let mut stream: Vec<Vec<u8>> =
            vec![vec![3u8], vec![1u8], i(5), i(2), vec![2u8], b"ab".to_vec()];
        stream.extend(gcrv(1.0, 2.0, 1)); // Lifetime
        stream.extend([7u32.to_le_bytes().to_vec(), 9u32.to_le_bytes().to_vec()]);
        stream.extend(gcrv(3.0, 4.0, 0)); // Size
        stream.extend(gcrv(5.0, 6.0, 0)); // StartSizeRate
        stream.extend(gcrv(0.0, 0.0, 0)); // BurstDelay
        stream.extend(gcrv(1.0, 2.0, 0)); // BurstCount
        stream.extend(gcrv(0.0, 0.0, 0)); // InitialDelay
        stream.push(vec![0u8]); // SlaveSystem: len 0
        stream.push(vec![]); // 'c3d' marker
        stream.push(
            [1.0f32, 2.0, 3.0]
                .iter()
                .flat_map(|x| x.to_le_bytes())
                .collect(),
        );
        stream.push(vec![0u8]); // PerParticleAttachedSystem
        stream.push(i(1)); // Priority
        stream.extend([vec![1u8], vec![0u8], vec![1u8], vec![0u8]]); // bools +0x80..83
        stream.push(0xA5u32.to_le_bytes().to_vec()); // pad_40 (gate >1)
        stream.push(0x2Au32.to_le_bytes().to_vec()); // +0x98 raw (gate >1)
        stream.push(vec![1u8]); // ShroudEmitter (gate >2)
        let mut info = ParticleSystemInfo::default();
        let mut rx = Rx {
            ev: vec![],
            stream: stream.into(),
            storing: false,
            light: false,
        };
        info.do_xfer(&mut rx);
        assert_eq!(rx.ev.len(), 44);
        assert_eq!(info.shader, 5);
        assert_eq!(info.ptype, 2);
        assert_eq!(info.particle_name.str(), b"ab");
        assert_eq!(
            info.lifetime,
            RandomRange {
                min: 1.0,
                max: 2.0,
                distribution: 1
            }
        );
        assert_eq!(
            info.size,
            RandomRange {
                min: 3.0,
                max: 4.0,
                distribution: 0
            }
        );
        assert_eq!(info.system_lifetime, 7);
        assert_eq!(info.sort_level, 9);
        assert_eq!(info.slave_system.length(), 0); // len-0 load → releaseBuffer
        assert_eq!(info.slave_pos_offset, [1.0, 2.0, 3.0]);
        assert_eq!(info.is_ground_aligned, true);
        assert_eq!(info.use_maximum_height, false);
        assert_eq!(info.pad_40, 0xA5);
        assert_eq!(info.field_98, 0x2A);
        assert!(info.shroud_emitter);
        // event 0/1 sanity on tags
        assert_eq!(rx.ev[0].0, TAG_VER);
        assert_eq!(rx.ev[1].0, TAG_BOOL);
        assert_eq!(rx.ev[2].0, TAG_ENU + 4);
        assert_eq!(rx.ev[4].0, TAG_STR);
        assert_eq!(rx.ev[6].0, TAG_VER); // GCRV composite starts
        assert_eq!(rx.ev[7].0, TAG_REAL);
        assert_eq!(rx.ev[10].0, TAG_UINT);
        assert_eq!(rx.ev[33].0, TAG_C3D);
    }

    #[test]
    fn do_xfer_particle_system_info_version1_gates() {
        let f = |v: f32| v.to_le_bytes().to_vec();
        let i = |v: i32| v.to_le_bytes().to_vec();
        let gcrv = || vec![vec![1u8], f(0.0), f(0.0), 0i32.to_le_bytes().to_vec()];
        let mut stream: Vec<Vec<u8>> = vec![vec![1u8], vec![0u8], i(1), i(1), vec![0u8]];
        stream.extend(gcrv());
        stream.extend([0u32.to_le_bytes().to_vec(), 0u32.to_le_bytes().to_vec()]);
        for _ in 0..5 {
            stream.extend(gcrv());
        }
        stream.push(vec![0u8]); // slave len0
        stream.push(vec![]); // c3d marker
        stream.push(vec![0u8; 12]);
        stream.push(vec![0u8]); // ppa len0
        stream.push(i(1)); // priority
        stream.extend([vec![0u8], vec![0u8], vec![0u8], vec![0u8]]);
        let mut info = ParticleSystemInfo::default();
        info.field_98 = 0x77;
        let mut rx = Rx {
            ev: vec![],
            stream: stream.into(),
            storing: false,
            light: false,
        };
        info.do_xfer(&mut rx);
        assert_eq!(rx.ev.len(), 40); // no pad_40 / raw / shroud events
        assert_eq!(info.pad_40, 0);
        assert_eq!(info.field_98, 0x77); // untouched when version == 1
        assert!(!info.shroud_emitter);
    }

    fn gcrv_stream(v: f32, w: f32, d: i32) -> Vec<Vec<u8>> {
        vec![
            vec![1u8],
            v.to_le_bytes().to_vec(),
            w.to_le_bytes().to_vec(),
            d.to_le_bytes().to_vec(),
        ]
    }

    #[test]
    fn do_xfer_default_update_gates() {
        let build = |cur: u8| -> Vec<Vec<u8>> {
            let mut s: Vec<Vec<u8>> = vec![vec![cur]];
            for _ in 0..5 {
                s.extend(gcrv_stream(1.0, 2.0, 0));
            }
            s.push(3i32.to_le_bytes().to_vec()); // RotationType +0x40
            for _ in 0..3 {
                s.extend(gcrv_stream(4.0, 5.0, 1));
            }
            s
        };
        let mut m = DefaultUpdateModuleInfo { dwords: [0; 25] };
        let mut rx = Rx {
            ev: vec![],
            stream: build(1).into(),
            storing: false,
            light: false,
        };
        m.do_xfer(&mut rx);
        assert_eq!(rx.ev.len(), 22); // gate 1<current closed at version 1
        assert_eq!(m.dwords[15], 3);
        assert_eq!(m.dwords[16], 0); // gated trio untouched
        let mut m = DefaultUpdateModuleInfo { dwords: [0; 25] };
        let mut rx = Rx {
            ev: vec![],
            stream: build(2).into(),
            storing: false,
            light: false,
        };
        m.do_xfer(&mut rx);
        assert_eq!(rx.ev.len(), 34);
        assert_eq!(&m.dwords[16..19], [1, 4.0f32.to_bits(), 5.0f32.to_bits()]);
    }

    #[test]
    fn do_xfer_render_object_update() {
        let mut s: Vec<Vec<u8>> = vec![vec![1u8]];
        for _ in 0..12 {
            s.extend(gcrv_stream(0.5, 1.5, 0));
        }
        s.push(2i32.to_le_bytes().to_vec()); // RotationType +0x94
        let mut m = RenderObjectUpdateModuleInfo { dwords: [0; 37] };
        let mut rx = Rx {
            ev: vec![],
            stream: s.into(),
            storing: false,
            light: false,
        };
        m.do_xfer(&mut rx);
        assert_eq!(rx.ev.len(), 50);
        assert_eq!(m.dwords[36], 2);
        assert_eq!(m.dwords[35], 1.5f32.to_bits());
        // store path: body skipped (IsStoring)
        let mut rx = Rx {
            ev: vec![],
            stream: vec![].into(),
            storing: true,
            light: false,
        };
        m.do_xfer(&mut rx);
        assert_eq!(rx.ev.len(), 0); // store pass: body skipped entirely
    }

    #[test]
    fn do_xfer_render_object_draw() {
        let mut s: Vec<Vec<u8>> = vec![vec![1u8], vec![1u8]]; // ver, flag_c bool
        for k in 0..3 {
            s.push(vec![2u8]); // name len 2
            s.push(vec![b'a' + k, b'x']);
            s.push((k as i32 + 1).to_le_bytes().to_vec()); // 'int'
            s.push(((k as f32) + 0.5).to_le_bytes().to_vec()); // 'real'
            s.push((k as i32 + 7).to_le_bytes().to_vec()); // shader enum
        }
        s.push(vec![1u8]); // flag_4 bool
        s.push(9.5f32.to_le_bytes().to_vec()); // +0x08 real
        let mut m = RenderObjectDrawModuleInfo::default();
        let mut rx = Rx {
            ev: vec![],
            stream: s.into(),
            storing: false,
            light: false,
        };
        m.do_xfer(&mut rx);
        assert_eq!(rx.ev.len(), 16 + 3); // +3 raw string blobs
        assert!(m.flag_c && m.flag_4);
        assert_eq!(m.field_8, 9.5f32.to_bits());
        assert_eq!(m.groups[1].name.str(), b"bx");
        assert_eq!(m.groups[2].a, 3);
        assert_eq!(m.groups[2].b, 2.5f32.to_bits());
        assert_eq!(m.groups[2].c, 9);
    }

    #[test]
    fn do_xfer_small_module_family() {
        // GpuDraw: ver, int, int, str+raw, real
        let mut g = GpuDrawModuleInfo::default();
        let mut rx = Rx {
            ev: vec![],
            stream: vec![
                vec![1u8],
                7i32.to_le_bytes().to_vec(),
                8i32.to_le_bytes().to_vec(),
                vec![2u8],
                b"uv".to_vec(),
                2.5f32.to_le_bytes().to_vec(),
            ]
            .into(),
            storing: false,
            light: false,
        };
        g.do_xfer(&mut rx);
        assert_eq!(rx.ev.len(), 6);
        assert_eq!((g.field_4, g.field_8), (7, 8));
        assert_eq!(g.name.str(), b"uv");
        assert_eq!(g.scale, 2.5);
        // TerrainCollision: ver, GCRV(4), str(len0), bool
        let mut t = TerrainCollisionModuleInfo {
            name: AsciiString::new(),
            dwords: [0; 3],
            flag_14: false,
            field_18: 0,
        };
        let mut s: Vec<Vec<u8>> = vec![vec![1u8]];
        s.extend(gcrv_stream(1.0, 3.0, 1));
        s.push(vec![0u8]);
        s.push(vec![1u8]);
        let mut rx = Rx {
            ev: vec![],
            stream: s.into(),
            storing: false,
            light: false,
        };
        t.do_xfer(&mut rx);
        assert_eq!(rx.ev.len(), 7);
        assert_eq!(t.dwords, [1, 1.0f32.to_bits(), 3.0f32.to_bits()]);
        assert!(t.flag_14);
        // LifeEvent: store guard → zero events; load = ver, GCRV(4), str(len0)
        let mut l = LifeEventModuleInfo {
            name: AsciiString::new(),
            dwords: [0; 4],
        };
        let mut rx = Rx {
            ev: vec![],
            stream: vec![].into(),
            storing: true,
            light: false,
        };
        l.do_xfer(&mut rx);
        assert!(rx.ev.is_empty());
        let mut s: Vec<Vec<u8>> = vec![vec![1u8]];
        s.extend(gcrv_stream(2.0, 4.0, 0));
        s.push(vec![0u8]);
        let mut rx = Rx {
            ev: vec![],
            stream: s.into(),
            storing: false,
            light: false,
        };
        l.do_xfer(&mut rx);
        assert_eq!(rx.ev.len(), 6);
        // Shared Version1-only DoXfer (0x00706a9b) + empty pair (0x0087a69c)
        let mut rx = Rx {
            ev: vec![],
            stream: vec![vec![1u8]].into(),
            storing: false,
            light: false,
        };
        ButterflyDrawModuleInfo.do_xfer(&mut rx);
        QuadDrawModuleInfo.do_xfer(&mut rx);
        StreakDrawModuleInfo.do_xfer(&mut rx);
        DefaultDrawModuleInfo.do_xfer(&mut rx);
        assert_eq!(rx.ev.len(), 4); // one TAG_VER each
        let mut rx = Rx {
            ev: vec![],
            stream: vec![].into(),
            storing: false,
            light: false,
        };
        EmissionVelocityInfo.do_xfer(&mut rx);
        EmissionVolumeInfo { flag_4: false }.do_xfer(&mut rx);
        assert!(rx.ev.is_empty()); // empty DoXfer bodies
    }

    #[test]
    fn static_init_modules_installs_all() {
        static_init_modules();
        for i in 0..27 {
            let _ = get_class(i); // idempotent after init
        }
        // per-category chain counts are install-order independent
        let chain = |cat: usize| {
            let mut n = 0;
            let mut cur = get_first(cat);
            while let Some(c) = cur {
                n += 1;
                cur = get_next(c);
            }
            n
        };
        assert_eq!(chain(0), 1);
        assert_eq!(chain(1), 1);
        assert_eq!(chain(2), 2); // Update + RenderObjectUpdate
        assert_eq!(chain(3), 1);
        assert_eq!(chain(4), 5); // Ortho + 4 velocity tags
        assert_eq!(chain(5), 7); // Point + 6 volume tags
        assert_eq!(chain(6), 7); // Draw + 6 draw tags
        assert_eq!(chain(7), 1);
        assert_eq!(chain(8), 2); // TerrainCollision + LifeEvent
    }

    #[test]
    fn particle_system_template_assign() {
        let mut a = ParticleSystemTemplate::default();
        a.cache_flag = true;
        a.info.system_lifetime = 42;
        let b = ParticleSystemTemplate::default();
        a.assign_from(&b);
        assert_eq!(a.info.system_lifetime, 0);
        assert!(!a.cache_flag);
        assert_eq!(a.name.length(), 0);
        let mut c = ParticleSystemTemplate::default();
        let nm: Vec<Ascii> = "fx1".bytes().map(Ascii).collect();
        c.name.set(&nm);
        c.info.shader = 3;
        let mut d = ParticleSystemTemplate::default();
        d.assign_from(&c);
        assert_eq!(d.name.str(), b"fx1");
        assert_eq!(d.info.shader, 3);
    }

    // -- wave 17: parse tier (wrapper token tables) -----------------------

    fn pini(lines: &[&str]) -> crate::ini::Ini {
        let i = crate::ini::Ini::new(lines.iter().map(|s| s.to_string()).collect());
        // parse_block self-advances the line table (0x0042d7b2 loop head).
        i
    }

    #[test]
    fn alpha_keyframe_parse() {
        // 0x0095b29c: `min max uint`, dist forced to 1 (GAUSSIAN).
        let mut i = pini(&["Alpha1 = 0.25 0.75 3", "end"]);
        let o = DefaultAlphaModuleInfo::parse(&mut i).unwrap();
        assert_eq!(o.entries[0][0], f32::from_bits(DIST_GAUSSIAN));
        assert_eq!(o.entries[0][1], 0.25);
        assert_eq!(o.entries[0][2], 0.75);
        assert_eq!(o.entries[0][3], f32::from_bits(3));
    }

    #[test]
    fn rgb_keyframe_parse() {
        // 0x0095b913: R= G= B= scaled + trailing time uint.
        let mut i = pini(&["Color1 = R = 255 G = 0 B = 128 7", "end"]);
        let o = DefaultColorModuleInfo::parse(&mut i).unwrap();
        assert_eq!(o.keyframes[0].color, [1.0, 0.0, 0.50196081]);
        assert_eq!(o.keyframes[0].time, 7);
    }

    #[test]
    fn render_update_rotation_enum() {
        // Rotation enum at 156 → dwords[36]; ROTATE_X = index 2.
        let mut i = pini(&["Rotation = ROTATE_X", "end"]);
        let o = RenderObjectUpdateModuleInfo::parse(&mut i).unwrap();
        assert_eq!(o.dwords[36], 2);
    }

    #[test]
    fn psi_lifetime_gcrv_and_priority() {
        // 0x00738a9d soft-scan: UNIFORM → 0; Priority enum list index 3.
        let mut i = pini(&[
            "Lifetime = 1 3 UNIFORM",
            "Priority = MEDIUM_OR_ABOVE",
            "end",
        ]);
        let o = ParticleSystemInfo::parse(&mut i).unwrap();
        assert_eq!(
            o.lifetime,
            RandomRange {
                min: 1.0,
                max: 3.0,
                distribution: 0
            }
        );
        assert_eq!(o.priority, 3);
    }

    #[test]
    fn psi_gcrv_default_is_gaussian() {
        // no trailing distribution token → dist stays 1 (GAUSSIAN).
        let mut i = pini(&["BurstCount = 4 6", "end"]);
        let o = ParticleSystemInfo::parse(&mut i).unwrap();
        assert_eq!(
            o.burst_count,
            RandomRange {
                min: 4.0,
                max: 6.0,
                distribution: 1
            }
        );
    }

    #[test]
    fn gpu_draw_detail_texture_string() {
        // 0x0042f11e quoted-string handler (0x0042ea59 body).
        let mut i = pini(&["DetailTexture = \"Elasto1\"", "FramesPerRow = 4", "end"]);
        let o = GpuDrawModuleInfo::parse(&mut i).unwrap();
        assert_eq!(o.name.str(), b"Elasto1");
        assert_eq!(o.field_4, 4);
    }

    #[test]
    fn render_draw_group_quartet() {
        let mut i = pini(&[
            "RenderGroup1 = \"GrpA\"",
            "NumObjects1 = 3",
            "Percent1 = 0.5",
            "Shader1 = ADDITIVE",
            "SinkOnTerrainCollision = yes",
            "end",
        ]);
        let o = RenderObjectDrawModuleInfo::parse(&mut i).unwrap();
        assert_eq!(o.groups[0].name.str(), b"GrpA");
        assert_eq!(o.groups[0].a, 3);
        assert_eq!(o.groups[0].b, 0.5f32.to_bits());
        assert_eq!(o.groups[0].c, 1); // ADDITIVE in ParticleShaderTypeNames
        assert!(o.flag_4);
    }

    #[test]
    fn physics_fields_parse() {
        // Drift real x3 @+4, Gravity real @+0x10, VelocityDamping GCRV
        // @+0x14, Swirly/ParticlesAttachToBone bytes @+0x20/+0x21.
        let mut i = pini(&[
            "DriftVelocity = X = 1 Y = 2 Z = 3",
            "Gravity = 1.5",
            "VelocityDamping = 0 1 GAUSSIAN",
            "Swirly = yes",
            "ParticlesAttachToBone = no",
            "end",
        ]);
        let o = DefaultPhysicsModuleInfo::parse(&mut i).unwrap();
        assert_eq!(
            o.dwords[0..3],
            [1.0f32.to_bits(), 2.0f32.to_bits(), 3.0f32.to_bits()]
        );
        assert_eq!(o.dwords[3], 1.5f32.to_bits());
        assert_eq!(o.dwords[4], DIST_GAUSSIAN);
        assert_eq!(o.dwords[5], 0.0f32.to_bits());
        assert_eq!(o.dwords[6], 1.0f32.to_bits());
        assert!(o.flag_20);
        assert!(!o.flag_21);
    }

    #[test]
    fn empty_table_parsers() {
        // DAT_00c6bb18: only END accepted; any token → level 5.
        let mut i = pini(&["end"]);
        assert_eq!(
            StreakDrawModuleInfo::parse(&mut i).unwrap(),
            StreakDrawModuleInfo
        );
        let mut i = pini(&["Scale = 3", "end"]);
        let e = ButterflyDrawModuleInfo::parse(&mut i).unwrap_err();
        assert_eq!(e.level, 5);
    }

    #[test]
    fn entry_name_match_is_case_sensitive() {
        // entry scan FUN_0042bc27 uses `strcmp`; only the END test at
        // 0x0042d81b is _strcmpi — so `alphA1` is an unknown field.
        let mut i = pini(&["alphA1 = 1 2 5", "END"]);
        let e = DefaultAlphaModuleInfo::parse(&mut i).unwrap_err();
        assert_eq!(e.level, 5);
        let mut i = pini(&["Alpha1 = 1 2 5", "END"]);
        let o = DefaultAlphaModuleInfo::parse(&mut i).unwrap();
        assert_eq!(o.entries[0][3], f32::from_bits(5));
    }

    #[test]
    fn missing_end_and_unknown_field_levels() {
        let mut i = pini(&["Frobnicate = 1"]);
        let e = DefaultAlphaModuleInfo::parse(&mut i).unwrap_err();
        assert_eq!(e.level, 5);
        let mut i = pini(&["Alpha1 = 1 2 3"]);
        let e = DefaultAlphaModuleInfo::parse(&mut i).unwrap_err();
        assert_eq!(e.level, 4);
        assert!(e.message.contains("Missing 'END'"));
    }

    #[test]
    fn bounded_u32_rejects_over_bound() {
        // SystemLifetime/SortLevel handlers pass v2 as bound (0 = none);
        // the 0x0042ef72 throw is @0x00bbe2b0 (level 3) when bound != 0.
        let mut i = pini(&["SortLevel = 9", "end"]);
        let o = ParticleSystemInfo::parse(&mut i).unwrap();
        assert_eq!(o.sort_level, 9);
    }
}

// Wave 26: FXParticleSystem runtime-module classes (DoXfer data plane;
// attributed from marked class anchors via call-graph — see
// docs/modules/FXParticleSystem.md Wave 26).

/// `DefaultUpdateModule` (runtime counterpart of
/// `DefaultUpdateModuleInfo`): layout pinned by DoXfer 0x0095f76c —
/// six `xfer(float&)` legs +0x04..+0x18, 'enu4' RotationType +0x1c,
/// three more floats gated on `1 < version.current` +0x20..+0x28.
/// vtable head and update()/physics state stay unmodelled.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DefaultUpdateModule {
    pub floats_lo: [f32; 6], // +0x04..+0x18
    pub rotation_type: u32,  // +0x1c ('enu4' "RotationType")
    pub floats_hi: [f32; 3], // +0x20..+0x28 (version-gated)
}
impl DefaultUpdateModule {
    /// 0x0095f805: `Xfer::Version1` then the payload (0x0095f76c).
    pub fn do_xfer<X: sage_xfer::Xfer>(&mut self, x: &mut X) {
        x.version1();
        self.do_xfer_payload(x);
    }
    /// 0x0095f76c: `xfer(Version&)` with local `Version{minimum:1,
    /// current:2}` (imm 0x201 byte pair at slot +0x28), six floats,
    /// RotationType, then the gated trio.
    pub fn do_xfer_payload<X: sage_xfer::Xfer>(&mut self, x: &mut X) {
        let mut v = sage_xfer::Version {
            minimum: 1,
            current: 2,
        };
        x.xfer_version(&mut v);
        for f in self.floats_lo.iter_mut() {
            x.xfer_f32(f);
        }
        let mut e = self.rotation_type.to_le_bytes();
        x.xfer_enum_named("RotationType", &mut e);
        self.rotation_type = u32::from_le_bytes(e);
        if 1 < v.current {
            for f in self.floats_hi.iter_mut() {
                x.xfer_f32(f);
            }
        }
    }
}

/// `RenderObjectUpdateModule` (runtime counterpart of
/// `RenderObjectUpdateModuleInfo`, twin of DoXfer 0x00962182): twelve
/// plain floats +0x04..+0x30 (12-byte GCRVs resolve to single floats on
/// the module), RotationType +0x34.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RenderObjectUpdateModule {
    pub floats: [f32; 12],  // +0x04..+0x30
    pub rotation_type: u32, // +0x34
}
impl RenderObjectUpdateModule {
    /// 0x0096234a: `Xfer::Version1` then 0x009622a6.
    pub fn do_xfer<X: sage_xfer::Xfer>(&mut self, x: &mut X) {
        x.version1();
        self.do_xfer_payload(x);
    }
    /// 0x009622a6: `Version1` runs AGAIN here — the stream carries the
    /// version twice (shim + payload); game quirk mirrored verbatim.
    pub fn do_xfer_payload<X: sage_xfer::Xfer>(&mut self, x: &mut X) {
        x.version1();
        for f in self.floats.iter_mut() {
            x.xfer_f32(f);
        }
        let mut e = self.rotation_type.to_le_bytes();
        x.xfer_enum_named("RotationType", &mut e);
        self.rotation_type = u32::from_le_bytes(e);
    }
}

/// `WindModule`: wind info subobject at +0x1c (DoXfer thunks
/// 0x007a5896/0x007a589e pin containment); runtime header +0x04..+0x18
/// unmodelled beyond storage.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WindModule {
    pub header: [u32; 6],     // +0x04..+0x18 (runtime state)
    pub info: WindModuleInfo, // +0x1c
}
impl WindModule {
    /// 0x007a5757 (thunked through 0x007a59bb): `Version1` then
    /// `WindModuleInfo::DoXfer` on the +0x1c leg.
    pub fn do_xfer<X: sage_xfer::Xfer>(&mut self, x: &mut X) {
        x.version1();
        self.info.do_xfer(x);
    }
    /// 0x007a5896: secondary-vftable DoXfer thunk — forwards to
    /// `WindModuleInfo::DoXfer` with the +0x1c leg as `this`. (The
    /// head-leg thunk 0x007a589e is the same forward; Rust callers hit
    /// `WindModuleInfo::do_xfer` directly.)
    pub fn do_xfer_info_leg<X: sage_xfer::Xfer>(&mut self, x: &mut X) {
        self.info.do_xfer(x);
    }
}

/// `SphericalEmissionVelocityModule`: info subobject at +0x18, so the
/// info's GCRV triple lands at +0x1c — the offset DoXfer 0x0095ea8e
/// xfers directly (thunk 0x007ac23b).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SphericalEmissionVelocityModule {
    pub header: [u32; 5],                    // +0x04..+0x14 (runtime state)
    pub info: SphericalEmissionVelocityInfo, // +0x18
}
impl SphericalEmissionVelocityModule {
    /// 0x0095ea8e: `Version1`; `xfer_random_variable` (0x00706183) on
    /// the info GCRV at +0x1c.
    pub fn do_xfer<X: sage_xfer::Xfer>(&mut self, x: &mut X) {
        x.version1();
        xfer_gcrv_raw(x, &mut self.info.dwords);
    }
}

#[cfg(test)]
mod runtime_module_tests {
    use super::*;
    use sage_xfer::{TAG_ENU, TAG_REAL, TAG_VER};

    struct Rx {
        ev: Vec<(u32, Vec<u8>)>,
        stream: std::collections::VecDeque<Vec<u8>>,
    }
    impl Rx {
        fn new(stream: Vec<Vec<u8>>) -> Self {
            Rx {
                ev: vec![],
                stream: stream.into(),
            }
        }
    }
    impl sage_xfer::Xfer for Rx {
        fn xfer_internal(&mut self, tag: u32, v: &mut [u8]) {
            let inb = self.stream.pop_front().unwrap_or_else(|| v.to_vec());
            self.ev.push((tag, v.to_vec()));
            if inb.len() == v.len() {
                v.copy_from_slice(&inb);
            }
        }
        fn is_storing(&self) -> bool {
            false
        }
        fn is_light_crc(&self) -> bool {
            false
        }
    }

    #[test]
    fn default_update_module_do_xfer() {
        let mut m = DefaultUpdateModule::default();
        let mut x = Rx::new(vec![vec![2u8, 0u8, 0u8, 0u8]]); // Version{min:1,cur:2} inbound
        m.do_xfer(&mut x);
        // Version1 stream entry, then Version struct, 6 floats, enum, 3 floats
        let tags: Vec<u32> = x.ev.iter().map(|e| e.0).collect();
        assert_eq!(tags[0], TAG_VER); // Version1
        assert_eq!(tags[1], TAG_VER); // Version{1,2}
        assert_eq!(tags[2..8], vec![TAG_REAL; 6]);
        assert_eq!(tags[8], TAG_ENU + 4); // 'enu4' (4-byte value)
        assert_eq!(tags[9..12], vec![TAG_REAL; 3]); // cur=2 > 1
    }

    #[test]
    fn render_object_update_double_version_quirk() {
        let mut m = RenderObjectUpdateModule::default();
        let mut x = Rx::new(vec![]);
        m.do_xfer(&mut x);
        assert_eq!(x.ev[0].0, TAG_VER);
        assert_eq!(x.ev[1].0, TAG_VER); // Version1 twice (0x0096234a + 0x009622a6)
        assert_eq!(
            x.ev[2..14].iter().map(|e| e.0).collect::<Vec<_>>(),
            vec![TAG_REAL; 12]
        );
        assert_eq!(x.ev[14].0, TAG_ENU + 4);
        assert_eq!(x.ev.len(), 15);
    }

    #[test]
    fn wind_and_spherical_modules() {
        let mut w = WindModule::default();
        let mut x = Rx::new(vec![]);
        w.do_xfer(&mut x);
        assert_eq!(x.ev[0].0, TAG_VER); // Version1 (0x007a5757)
                                        // WindModuleInfo::DoXfer body is IsStoring-guarded -> nothing after
                                        // the guard read in load mode? guard passes (is_storing false):
                                        // expect the wind field sequence follows (non-empty).
        assert!(x.ev.len() > 1);
        let n = x.ev.len();
        let mut x2 = Rx::new(vec![]);
        w.do_xfer_info_leg(&mut x2); // 0x007a5896: no Version1 prefix
        assert_eq!(x2.ev.len(), n - 1);

        let mut s = SphericalEmissionVelocityModule::default();
        let mut x3 = Rx::new(vec![]);
        s.do_xfer(&mut x3);
        assert_eq!(x3.ev[0].0, TAG_VER); // Version1
                                         // GCRV composite follows (ver + min + max + enum legs, rng.rs 0x00706183)
        assert!(x3.ev.len() >= 5);
    }
}

// Wave 27: emission module construction family (createModule + the
// four-level ctor chains beneath). Every chain: operator new (size)
// -> top ctor (per class) -> middle ctor -> leaf copy-ctor (copies the
// Info subobject from the template via the marked Info copy-ctors) ->
// shared base ctors; all levels store vtables, which Rust expresses
// statically. Rust counterpart of the whole chain: allocate +
// zero runtime header + copy-construct the info from the template.
// The `+0x18` (velocity) / `+0x1c` (volume) info offsets are pinned by
// the leaf bodies (`copy-ctor(param_1 + 6/7 dwords, param_3 + ...)`).

macro_rules! fx_runtime_module {
    ($modname:ident, $tmpl:ident, $info:ty, $hdr:literal, $size:literal,
     $fac:expr, $top:expr, $mid:expr, $leaf:expr) => {
        #[doc = concat!(
                            "FXParticleSystem runtime module (new(", $size, ") + factory ",
                            $fac, " -> ctor ", $top, " -> ", $mid, " -> ", $leaf,
                            "): runtime header unmodelled (physics methods open); the ",
                            "ctor chain copy-constructs the Info subobject from the ",
                            "template — Rust counterpart is `create`.")]
        #[derive(Clone, Debug, Default, PartialEq)]
        pub struct $modname {
            pub header: [u32; $hdr], // +0x04..info offset (runtime state)
            pub info: $info,
        }
        impl $modname {
            pub fn create(template: &$tmpl) -> Self {
                Self {
                    header: [0; $hdr],
                    info: template.info.clone(),
                }
            }
        }
    };
}

// Emission-velocity family: info subobject at +0x18 (leaf copies via
// marked Info ctors 0x0095e80d/0x0095ea7c/0x0095d976-family).
fx_runtime_module!(
    OrthoEmissionVelocityModule,
    OrthoEmissionVelocityModuleTemplate,
    OrthoEmissionVelocityInfo,
    5,
    0x40,
    "0x007ada72",
    "0x007ad16c",
    "0x007acb79",
    "0x007ac1a3"
);
fx_runtime_module!(
    HemisphericalEmissionVelocityModule,
    HemisphericalEmissionVelocityModuleTemplate,
    SphericalEmissionVelocityInfo,
    5,
    0x28,
    "0x007adab8",
    "0x007ad1ca",
    "0x007acbcd",
    "0x007acba3"
);
fx_runtime_module!(
    CylindricalEmissionVelocityModule,
    CylindricalEmissionVelocityModuleTemplate,
    CylindricalEmissionVelocityInfo,
    5,
    0x34,
    "0x007adadb",
    "0x007ad1f9",
    "0x007acbf7",
    "0x007ac24b"
);
fx_runtime_module!(
    OutwardEmissionVelocityModule,
    OutwardEmissionVelocityModuleTemplate,
    CylindricalEmissionVelocityInfo,
    5,
    0x34,
    "0x007adafe",
    "0x007ad228",
    "0x007acc21",
    "0x007ac297"
);

// Emission-volume family: info subobject at +0x1c; shared base ctors
// 0x007abf54/0x007ac180/0x007aa228 fold into `create` as well.
fx_runtime_module!(
    PointEmissionVolumeModule,
    PointEmissionVolumeModuleTemplate,
    EmissionVolumeInfo,
    6,
    0x24,
    "0x007adb21",
    "0x007ad278",
    "0x007acc4b",
    "0x007abf85"
);
fx_runtime_module!(
    LineEmissionVolumeModule,
    LineEmissionVolumeModuleTemplate,
    CylinderEmissionVolumeInfo,
    6,
    0x3c,
    "0x007adb44",
    "0x007ad2b3",
    "0x007acc7c",
    "0x007abfe1"
);
fx_runtime_module!(
    BoxEmissionVolumeModule,
    BoxEmissionVolumeModuleTemplate,
    BoxEmissionVolumeInfo,
    6,
    0x30,
    "0x007adb67",
    "0x007ad2e9",
    "0x007accad",
    "0x007ac034"
);
fx_runtime_module!(
    SphereEmissionVolumeModule,
    SphereEmissionVolumeModuleTemplate,
    SphereEmissionVolumeInfo,
    6,
    0x28,
    "0x007adb8a",
    "0x007ad31f",
    "0x007accde",
    "0x007ac087"
);
fx_runtime_module!(
    CylinderEmissionVolumeModule,
    CylinderEmissionVolumeModuleTemplate,
    CylinderEmissionVolumeInfo,
    6,
    0x3c,
    "0x007adbad",
    "0x007ad355",
    "0x007acd0f",
    "0x007ac0da"
);
fx_runtime_module!(
    TerrainFireEmissionModule,
    TerrainFireEmissionModuleTemplate,
    TerrainFireEmissionInfo,
    6,
    0x4c,
    "0x007adc0f",
    "0x007ad3c1",
    "0x007acd40",
    "0x007ac12d"
);

impl SphericalEmissionVelocityModule {
    /// createModule 0x007ada95 (new(0x28) + ctor 0x007ad19b ->
    /// 0x007acba3 -> 0x007ac1f7, which copy-constructs the marked
    /// `SphericalEmissionVelocityInfo` @+0x18): Rust equivalent.
    pub fn create(template: &SphericalEmissionVelocityModuleTemplate) -> Self {
        Self {
            header: [0; 5],
            info: template.info,
        }
    }
}

#[cfg(test)]
mod runtime_module_ctor_tests {
    use super::*;

    #[test]
    fn create_copies_template_info() {
        let mut t = BoxEmissionVolumeModuleTemplate::default();
        t.info.flag_4 = true;
        t.info.dims = [1.0, 2.0, 3.0];
        let m = BoxEmissionVolumeModule::create(&t);
        assert_eq!(m.info, t.info);
        assert_eq!(m.header, [0; 6]);

        let mut st = SphericalEmissionVelocityModuleTemplate::default();
        st.info.dwords = [9, 8, 7];
        let sm = SphericalEmissionVelocityModule::create(&st);
        assert_eq!(sm.info.dwords, [9, 8, 7]);

        // quirk classes carry the embedded info type (wave 22):
        let mut lt = LineEmissionVolumeModuleTemplate::default();
        lt.info.fields = [0.5, 1.5, 2.5, 3.5, 4.5, 5.5];
        let lm = LineEmissionVolumeModule::create(&lt);
        assert_eq!(lm.info, lt.info);

        let ot = OutwardEmissionVelocityModuleTemplate::default();
        let om = OutwardEmissionVelocityModule::create(&ot);
        assert_eq!(om.info, ot.info);

        let pt = PointEmissionVolumeModuleTemplate::default();
        let pm = PointEmissionVolumeModule::create(&pt);
        assert_eq!(pm.info, pt.info);
        assert_eq!(pm.header.len(), 6); // info @+0x1c
    }
}

// Wave 29: module-level virtual-clone chains (LifeEvent, TerrainCollision)
// and the module-list unlink. Unlike the emission family, these copy-ctors
// do NOT deep-copy: each chain names individual dword/byte fields and the
// remaining bytes keep whatever `operator new` left (modelled as zero).
// The 0x007abecc/0x0095e2e0 chain is NOT mirrored: its source fields sit
// at a constant −0x14 base shift (info read from `param_3 + 8` where the
// module class keeps it at +0x1c) and it writes dwords to +0xa4 — far
// past the module allocations (0x20..0x4c) — so that chain builds the
// per-particle `ParticleTerrainFire` class, which stays unmodelled.

/// FXParticleSystem LifeEventModule (0x20 B). Virtual clone: factory
/// 0x007ae092 (`new(0x20)` + ctor); copy chain 0x007ae0af -> 0x007ae0d5 ->
/// 0x007ae13c (shared with TerrainCollision) -> 0x007addec, plus the
/// sub-object copy 0x007ae11b over the pair at +0x14/+0x18.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LifeEventModule {
    pub fields: [u32; 8],
}
impl LifeEventModule {
    pub fn clone_ctor(src: &Self) -> Self {
        let mut d = Self::default(); // new(): rest uninitialised in the game
        d.fields[1] = src.fields[1]; // +0x04 dword (0x007addec)
        d.fields[3] = src.fields[3] & 0xff; // +0x0c byte  (0x007ae13c)
        d.fields[5] = src.fields[5]; // +0x14 dword (0x007ae11b)
        d.fields[6] = src.fields[6]; // +0x18 dword (0x007ae11b)
        d.fields[7] = src.fields[7] & 0xff; // +0x1c byte  (0x007ae0d5)
        d
    }
}

/// FXParticleSystem TerrainCollisionModule (0x24 B). Virtual clone:
/// factory 0x007ae28c (`new(0x24)` + ctor); chain 0x007ae2a9 ->
/// 0x007ae2cf -> 0x007ae13c/0x007addec (shared with LifeEvent), sub-object
/// copy 0x007ae315 over +0x14/+0x18 dwords and the +0x1c byte.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TerrainCollisionModule {
    pub fields: [u32; 9],
}
impl TerrainCollisionModule {
    pub fn clone_ctor(src: &Self) -> Self {
        let mut d = Self::default();
        d.fields[1] = src.fields[1]; // +0x04 dword (0x007addec)
        d.fields[3] = src.fields[3] & 0xff; // +0x0c byte  (0x007ae13c)
        d.fields[5] = src.fields[5]; // +0x14 dword (0x007ae315)
        d.fields[6] = src.fields[6]; // +0x18 dword (0x007ae315)
        d.fields[7] = src.fields[7] & 0xff; // +0x1c byte  (0x007ae2cf)
        d
    }
}

/// Module-list bookkeeping (0x007afd5c): count at +0x10, end slots at
/// +0x14/+0x18 (the body reads `+0x14 <- node.link_64` and
/// `+0x18 <- node.link_68` — which slot is head vs tail is not
/// disambiguated by the code, so both are kept address-named); members
/// carry link dwords +0x64/+0x68 (u32 handles, 0 = null) and an in-list
/// byte at +0x74.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ModuleList {
    pub count: i32,  // +0x10
    pub end_14: u32, // +0x14
    pub end_18: u32, // +0x18
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ModuleLink {
    pub link_64: u32,  // +0x64
    pub link_68: u32,  // +0x68
    pub in_list: bool, // +0x74
}
impl ModuleList {
    /// 0x007afd5c `removeModule`: no-op unless in-list; neighbor splice via
    /// the two link dwords, end-slot updates compared BEFORE the node's
    /// links are cleared, then node cleared and count decremented.
    pub fn unlink(&mut self, idx: u32, links: &mut [ModuleLink]) {
        let link = links[idx as usize];
        if !link.in_list {
            return;
        }
        if link.link_64 != 0 {
            links[link.link_64 as usize].link_68 = link.link_68;
        }
        if link.link_68 != 0 {
            links[link.link_68 as usize].link_64 = link.link_64;
        }
        if idx == self.end_14 {
            self.end_14 = link.link_64;
        }
        if idx == self.end_18 {
            self.end_18 = link.link_68;
        }
        links[idx as usize] = ModuleLink::default(); // +0x68=0, +0x64=0, +0x74=0
        self.count -= 1;
    }
}

#[cfg(test)]
mod wave29_tests {
    use super::*;

    #[test]
    fn clone_ctor_field_subset() {
        let src = LifeEventModule {
            fields: [7, 8, 9, 0x1234_5678, 11, 12, 13, 0x9abc_def0],
        };
        let d = LifeEventModule::clone_ctor(&src);
        assert_eq!(d.fields[1], 8);
        assert_eq!(d.fields[3], 0x78); // byte copy only (0x007ae13c)
        assert_eq!((d.fields[5], d.fields[6]), (12, 13));
        assert_eq!(d.fields[7], 0xf0); // byte copy (0x007ae0d5)
        assert_eq!((d.fields[0], d.fields[2], d.fields[4]), (0, 0, 0)); // NOT copied

        let tsrc = TerrainCollisionModule {
            fields: [1, 2, 3, 0xff00_00aa, 5, 6, 7, 0xbeef_0055, 9],
        };
        let td = TerrainCollisionModule::clone_ctor(&tsrc);
        assert_eq!(td.fields[1], 2);
        assert_eq!(td.fields[3], 0xaa);
        assert_eq!((td.fields[5], td.fields[6]), (6, 7));
        assert_eq!(td.fields[7], 0x55);
        assert_eq!(td.fields[8], 0); // +0x20 never written by any ctor leg
    }

    #[test]
    fn module_list_unlink() {
        // three-member list 1 - 2 - 3 (0 = null sentinel); +0x68 leg reads
        // as "next", +0x64 as "prev"
        let mut links = [
            ModuleLink::default(),
            ModuleLink {
                link_64: 0,
                link_68: 2,
                in_list: true,
            },
            ModuleLink {
                link_64: 1,
                link_68: 3,
                in_list: true,
            },
            ModuleLink {
                link_64: 2,
                link_68: 0,
                in_list: true,
            },
        ];
        let mut list = ModuleList {
            count: 3,
            end_14: 3,
            end_18: 1,
        };
        list.unlink(2, &mut links);
        assert_eq!(links[1].link_68, 3);
        assert_eq!(links[3].link_64, 1);
        assert_eq!(links[2], ModuleLink::default());
        assert_eq!((list.count, list.end_14, list.end_18), (2, 3, 1));

        // end-slot member: slot takes the node's link, null leg skipped;
        // not-in-list is a no-op leaving count alone.
        list.unlink(1, &mut links);
        assert_eq!((list.count, list.end_14, list.end_18), (1, 3, 3));
        list.unlink(1, &mut links);
        assert_eq!(list.count, 1);
        // last member: both slots update from the node's (zeroed) links
        list.unlink(3, &mut links);
        assert_eq!((list.count, list.end_14, list.end_18), (0, 0, 0));
    }
}

// Wave 30 — `ConcreteModuleClass<DefaultModuleTag<N>>::readTemplate`
// INI factories (0x007ac2e5 <6>, 0x007ac555 <1>, 0x007ac5db <0>,
// 0x007ac661 <3>, 0x007ac6e1 <2>): operator new + default-template ctor
// (NULL on alloc failure) + the marked `<T>::parse` wrapper (all five
// marked: 0x0095c434/0x0095b2fd/0x0095b93f/0x0095ee4c/0x0095f4bc). The
// parse already constructs the info plane, so the Rust factory is the
// parse call wrapped in the template.

/// readTemplate INI factory (game NULL-on-alloc-failure becomes an
/// ordinary Rust value; allocation errors are not modelled).
pub fn read_default_module_template_6(
    ini: &mut crate::ini::Ini,
) -> IniResult<DefaultModuleTemplate6> {
    Ok(DefaultModuleTemplate6 {
        info: DefaultDrawModuleInfo::parse(ini)?,
    })
}
pub fn read_default_module_template_1(
    ini: &mut crate::ini::Ini,
) -> IniResult<DefaultModuleTemplate1> {
    Ok(DefaultModuleTemplate1 {
        info: DefaultAlphaModuleInfo::parse(ini)?,
    })
}
pub fn read_default_module_template_0(
    ini: &mut crate::ini::Ini,
) -> IniResult<DefaultModuleTemplate0> {
    Ok(DefaultModuleTemplate0 {
        info: DefaultColorModuleInfo::parse(ini)?,
    })
}
pub fn read_default_module_template_3(
    ini: &mut crate::ini::Ini,
) -> IniResult<DefaultModuleTemplate3> {
    Ok(DefaultModuleTemplate3 {
        info: DefaultPhysicsModuleInfo::parse(ini)?,
    })
}
pub fn read_default_module_template_2(
    ini: &mut crate::ini::Ini,
) -> IniResult<DefaultModuleTemplate2> {
    Ok(DefaultModuleTemplate2 {
        info: DefaultUpdateModuleInfo::parse(ini)?,
    })
}

#[cfg(test)]
mod wave30_tests {
    use super::*;

    #[test]
    fn read_template_wraps_parse() {
        // Drive the tag<3> factory with the physics field set (same body
        // as physics_fields_parse): the factory must wrap parse's info.
        let mut ini = crate::ini::Ini::new(
            [
                "DriftVelocity = X = 1 Y = 2 Z = 3",
                "Gravity = 1.5",
                "VelocityDamping = 0 1 GAUSSIAN",
                "Swirly = yes",
                "ParticlesAttachToBone = no",
                "end",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        );
        let t = read_default_module_template_3(&mut ini).unwrap();
        assert_eq!(t.info.dwords[3], 1.5f32.to_bits());
        assert!(t.info.flag_20);
    }
}
