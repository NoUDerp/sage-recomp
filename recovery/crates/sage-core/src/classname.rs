//! Xfer / RTTI class-name thunks — 9 rows. Each is a 6-byte
//! `mov eax, <ptr to .rdata string>; ret` — the virtual
//! `getClassName()`-style accessor a storage/manager class hands
//! to the Xfer registration table. Mirrored as functions returning
//! the static class name.
//!
//! The addresses are scattered across the module pages (each class
//! compiled in its own TU), so they carry mixed module tags; the
//! `module` string the class registers under is what matters, not
//! the page it landed on.
//!
//! Note on `0x007b0e57`: the game returns `&PTR_s_...`, i.e. the
//! address of a .rdata cell that itself holds the string pointer
//! (a reassignable name slot) — dereferenced once it is the static
//! name, mirrored here as the string with that indirection noted.

/// 0x007b3662 — `Script` (`.rdata 0x00c1f434`).
pub fn class_name_script() -> &'static str {
    "Script"
}

/// 0x007b40fa — `ScriptGroup` (`.rdata 0x00c1f480`).
pub fn class_name_script_group() -> &'static str {
    "ScriptGroup"
}

/// 0x007b7745 — `ScriptList` (`.rdata 0x00c1faa0`).
pub fn class_name_script_list() -> &'static str {
    "ScriptList"
}

/// 0x007b00d0 — `ParticleSystemStorageModule` (`.rdata 0x00c1d930`).
pub fn class_name_particle_system_storage_module() -> &'static str {
    "ParticleSystemStorageModule"
}

/// 0x007b014c — `CPUParticleSystemStorageModule` (`.rdata 0x00c1d990`).
pub fn class_name_cpu_particle_system_storage_module() -> &'static str {
    "CPUParticleSystemStorageModule"
}

/// 0x007b03ed — `GPUParticleSystemStorageModule` (`.rdata 0x00c1d9f0`).
pub fn class_name_gpu_particle_system_storage_module() -> &'static str {
    "GPUParticleSystemStorageModule"
}

/// 0x007b040c — `TerrainFireSystemStorageModule` (`.rdata 0x00c1da50`).
pub fn class_name_terrain_fire_system_storage_module() -> &'static str {
    "TerrainFireSystemStorageModule"
}

/// 0x007b0e57 — `IntrinsicSciencePurchasePoints`; the game returns
/// a PTR-to-PTR cell (`0x00c1db18` → string at `0x00c1daf8`).
pub fn class_name_intrinsic_science_purchase_points() -> &'static str {
    "IntrinsicSciencePurchasePoints"
}

/// 0x007b9124 — `LivingWorldCampaignManager` (`.rdata 0x00c1fb3c`).
pub fn class_name_living_world_campaign_manager() -> &'static str {
    "LivingWorldCampaignManager"
}

