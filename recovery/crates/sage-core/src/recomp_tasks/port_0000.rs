//! Reimplement the original x86 functions listed below.
//! Inputs use raw ABI bits: f32 in low 32 bits, f64 in all 64 bits.
//! Read reference/decomp/<address>.c; do not copy target bytes.

use super::Observed;

// 0x006e6cfe ?rva002E6CFE@Rva002E6CFE@@QAEXHHH@Z
// signature: {"args": ["i32", "i32", "i32"], "kind": "simple", "ret": "void", "thiscall": true, "want_float": false}
// reference: reference/decomp/0x006e6cfe.c
// 0x00949237 ?SetSquadWidth@FormationSquad@@QAEXH@Z
// signature: {"args": ["i32"], "kind": "simple", "ret": "void", "thiscall": true, "want_float": false}
// reference: reference/decomp/0x00949237.c
// 0x004d0bfd ?Translate@Matrix3D@@QAEXMMM@Z
// signature: {"args": ["f32", "f32", "f32"], "kind": "simple", "ret": "void", "thiscall": true, "want_float": false}
// reference: reference/decomp/0x004d0bfd.c
// 0x005e3699 ?Set@Matrix3D@@QAEXMMMMMMMMMMMM@Z
// signature: {"args": ["f32", "f32", "f32", "f32", "f32", "f32", "f32", "f32", "f32", "f32", "f32", "f32"], "kind": "simple", "ret": "void", "thiscall": true, "want_float": false}
// reference: reference/decomp/0x005e3699.c
// 0x0085829e ?setMotion@BridgeScaffoldBehavior@@UAEXW4ScaffoldTargetMotion@@@Z
// signature: {"args": ["enum4"], "kind": "simple", "ret": "void", "thiscall": true, "want_float": false}
// reference: reference/decomp/0x0085829e.c
// 0x00430d10 ?rva00030D10@Rva00030D10@@QAEXI@Z
// signature: {"args": ["u32"], "kind": "simple", "ret": "void", "thiscall": true, "want_float": false}
// reference: reference/decomp/0x00430d10.c
// 0x008f5391 ?rva004F5391@TunnelTracker@@UAEX_N@Z
// signature: {"args": ["bool"], "kind": "simple", "ret": "void", "thiscall": true, "want_float": false}
// reference: reference/decomp/0x008f5391.c
// 0x0086261b ?rva0046261B@SlaughterHordeContain@@UAEX_N@Z
// signature: {"args": ["bool"], "kind": "simple", "ret": "void", "thiscall": true, "want_float": false}
// reference: reference/decomp/0x0086261b.c
// 0x005e3679 ?Rva001E3679@@YA_NH@Z
// signature: {"args": ["i32"], "kind": "simple", "ret": "bool", "thiscall": false, "want_float": false}
// reference: reference/decomp/0x005e3679.c
// 0x006c94f5 ?cacheStatus@Weapon@@QBEXW4WeaponStatus@@@Z
// signature: {"args": ["enum4"], "kind": "simple", "ret": "void", "thiscall": true, "want_float": false}
// reference: reference/decomp/0x006c94f5.c
// 0x006e6e6c ?isGroundOrBridgeLayer@@YAHH@Z
// signature: {"args": ["i32"], "kind": "simple", "ret": "i32", "thiscall": false, "want_float": false}
// reference: reference/decomp/0x006e6e6c.c
// 0x008381b0 ?rva004381B0@Rva004381B0@@QAEXM@Z
// signature: {"args": ["f32"], "kind": "simple", "ret": "void", "thiscall": true, "want_float": false}
// reference: reference/decomp/0x008381b0.c

pub fn call(address: u32, args: &[u64], scratch: &mut [u8; 64])
    -> Option<Observed>
{
    let _ = (address, args, scratch);
    None
}
