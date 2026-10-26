//! Reimplement the original x86 functions listed below.
//! Inputs use raw ABI bits: f32 in low 32 bits, f64 in all 64 bits.
//! Read reference/decomp/<address>.c; do not copy target bytes.

use super::Observed;

// 0x00928264 ?rva00528264@Rva00528264@@QAEXH@Z
// signature: {"args": ["i32"], "kind": "simple", "ret": "void", "thiscall": true, "want_float": false}
// reference: reference/decomp/0x00928264.c
// 0x009468ab ?rva005468AB@SynchronizeGroupOrder@@UAE_NH@Z
// signature: {"args": ["i32"], "kind": "simple", "ret": "bool", "thiscall": true, "want_float": false}
// reference: reference/decomp/0x009468ab.c
// 0x008ac135 ?rva004AC135@RainOfFireUpdate@@UAEXM@Z
// signature: {"args": ["f32"], "kind": "simple", "ret": "void", "thiscall": true, "want_float": false}
// reference: reference/decomp/0x008ac135.c
// 0x008bd77c ?multiply@Rva004BD77CFloatField@@QAEXM@Z
// signature: {"args": ["f32"], "kind": "simple", "ret": "void", "thiscall": true, "want_float": false}
// reference: reference/decomp/0x008bd77c.c
// 0x007f7133 ?v03@Rva003F71D4Metric@@UAE_NHH@Z
// signature: {"args": ["i32", "i32"], "kind": "simple", "ret": "bool", "thiscall": true, "want_float": false}
// reference: reference/decomp/0x007f7133.c
// 0x007f718d ?found@Rva003F7177Callback@@UAE_NHH@Z
// signature: {"args": ["i32", "i32"], "kind": "simple", "ret": "bool", "thiscall": true, "want_float": false}
// reference: reference/decomp/0x007f718d.c
// 0x00a198d0 ?Greatest_Common_Divisor@@YAIII@Z
// signature: {"args": ["u32", "u32"], "kind": "simple", "ret": "u32", "thiscall": false, "want_float": false}
// reference: reference/decomp/0x00a198d0.c
// 0x0046eded ?Set_Fog_Range@SceneClass@@UAEXMM@Z
// signature: {"args": ["f32", "f32"], "kind": "simple", "ret": "void", "thiscall": true, "want_float": false}
// reference: reference/decomp/0x0046eded.c
// 0x00924a2d ?rva00524A2D@Rva00524A2D@@QAEXHHHH@Z
// signature: {"args": ["i32", "i32", "i32", "i32"], "kind": "simple", "ret": "void", "thiscall": true, "want_float": false}
// reference: reference/decomp/0x00924a2d.c

pub fn call(address: u32, args: &[u64], scratch: &mut [u8; 64])
    -> Option<Observed>
{
    let _ = (address, args, scratch);
    None
}
