//! Wave 45: D3D device band `0x0051ee20`–`0x0051f960` (module W3D-Draw).
//!
//! Four families live in this band:
//!
//! 1. **Render-state enum -> name lookup** — 13 rows `0x0051ee20`..`0x0051f160`,
//!    each a flat if/switch chain over one Direct3D state enum returning a `char*`
//!    into the `.rdata` string pool. Every name below was read out of the golden
//!    image at the quoted VA (`game/game.dat`; `.rdata` VA `0x00bba000` -> file
//!    offset `0x7b9400`, section table verified in `docs/startup-path.md`). The
//!    strings are *packed inline* in the pool (e.g. the `D3DZB` triple sits at
//!    `0x00bd08b0`/`0x00bd08bc`/`0x00bd08c8`, stride `0x0c`), but every body loads
//!    an absolute immediate per case, so the mirror keeps one `&'static str` const
//!    per referenced pointer instead of a stride table.
//! 2. **PIX/D3DPERF wide-string hooks** — `0x0051f340` (`DAT_00dedbdc`),
//!    `0x0051f3c0` (`DAT_00dedbe0`, skipped) and `0x0051f3d0` (`_DAT_00dedbe4`).
//!    The three slots are filled by `0x00525620` lines 118–120 from
//!    `GetProcAddress(hmod, "D3DPERF_BeginEvent")` (string `0x00bd11f4`),
//!    `"D3DPERF_EndEvent"` (`0x00bd11e0`) and `"D3DPERF_SetMarker"` (`0x00bd11cc`) —
//!    verified golden bytes. `0x0051f340`/`0x0051f3d0` widen a 1-byte-per-char name
//!    into a 256-`short` stack buffer and call the hook `(color, wide_name)`.
//! 3. **Refcounted render-state object slot** — `0x0051f4b0` stores a pointer into
//!    the global dword array `&DAT_00dee5e0[index]`, `AddRef`s the incoming object
//!    (`*(short*)(ptr + 4) += 1`), releases the old one through the global release
//!    helper `0x00a1ed10`, and raises one dirty bit of the state cache mask
//!    `DAT_00dec4f4`.
//! 4. **DirectX thread-ownership predicates + record index helpers** —
//!    `0x0051f500`/`0x0051f600` read the lock globals `DAT_00deda88` (owner TID) and
//!    `DAT_00deda8c` (recursion count) alongside `GetCurrentThreadId()`; the mirror
//!    lifts the TID to a parameter (the OS call is the caller's boundary).
//!    `0x0051f910`/`0x0051f960` are the two `__thiscall` helpers of one
//!    fixed-stride record table (`RecordTable` below).
//!
//! Skipped rows (shared-contract rule 4), with the body evidence that disqualifies
//! them:
//! - `0x0051f3c0` — null-guarded tail call through the global hook slot
//!   `DAT_00dedbe0`; the slot is the `GetProcAddress` result for
//!   `"D3DPERF_EndEvent"` (string 0x00bd11e0, assigned at 0x00525620:119).
//!   Guard + indirect call only, no computed semantics.
//! - `0x0051f450` — call chain (`FUN_00439660(0x44584552, …)`, `FUN_00438790(1)`)
//!   followed by three chained vtable dispatches through global `DAT_00de0880`
//!   (slots `+0x60`/`+0x6c`, then `+0x38`/`+0x4c` on the returned interfaces)
//!   logging `"Direct3D error "` (string 0x00bd0d22, used from `+2`).
//! - `0x0051f520` — OS-synchronisation body: `WaitForSingleObject(DAT_00dec598,
//!   20000)` + `Enter`/`LeaveCriticalSection(&DAT_00dec540)` + owner/count stores,
//!   with a `FUN_004387c0`/vtable error chain on `"A thread held onto DirectX for
//!   more than 20000msec."` (string 0x00bd0d34).
//! - `0x0051f5b0` — the same body with a caller-supplied timeout:
//!   `WaitForSingleObject` + `Enter`/`LeaveCriticalSection` + owner/count stores,
//!   returning 0 on `0x102` (WAIT_TIMEOUT) else 1.
//! - `0x0051f660` — pure vtable dispatch: loads `param_1[2]`, then calls slot
//!   `+0xc` and slot `+0x8` with that value and `0`.
//! - `0x0051f6b0` — MSVC vector deleting-destructor thunk: scalar destructor
//!   `FUN_00a10a40` / `FUN_0042fd60` / `FUN_0042fd80` calls and
//!   `_eh_vector_destructor_iterator_`; its only own logic is the `param_2 & 2`
//!   array flag and the `-4` allocation-cookie adjustment.
//! - `0x0051f710` — global vtable-pointer store (`*param_1 = PTR_DAT_00dd528c`),
//!   a `FUN_00a10b00(0, 0)` call, then a copy of global byte `DAT_00e08c18` into
//!   `**param_1`.
//!
//! Faithful-quirk notes (all pinned by tests):
//! - Every lookup's miss value is `s_>UNKNOWN_00bcfe07 + 1`: the golden bytes at
//!   `0x00bcfe07` are `">UNKNOWN\0"`, so the game hands back the `U` at
//!   `0x00bcfe08` — the string `"UNKNOWN"`, never NULL.
//! - `0x0051f4b0`: `(byte)index & 0x1f` (byte truncation is a no-op under `& 0x1f`)
//!   and `0x40 << n` are 32-bit x86 shifts, so `n >= 26` sets **no** bit; the mirror
//!   uses `wrapping_shl` to keep that.
//! - `0x0051f680` clamps in the game's *test order*: with `lo > hi` the low bound
//!   wins, and NaN comparisons fall through exactly like MSVC's `cmp`/`branch`.
//! - `0x0051f910` returns `0` (a hit on slot 0) when the table's `+0xc` valid byte
//!   is clear — not `-1`. Out-of-range `count` reads past the array in the game
//!   (UB); the mirror indexes the slice and panics.

// ---------------------------------------------------------------------------
// Golden `.rdata` string pool — one const per pointer the bodies load.
// ---------------------------------------------------------------------------

/// Miss value of every lookup below: `s_>UNKNOWN_00bcfe07 + 1`.
const UNKNOWN: &str = "UNKNOWN";

// `0x0051ee20` (`D3DZBUFFERTYPE`): packed run at stride `0x0c`.
const D3DZB_FALSE: &str = "D3DZB_FALSE"; // 0x00bd08b0
const D3DZB_TRUE: &str = "D3DZB_TRUE"; // 0x00bd08bc
const D3DZB_USEW: &str = "D3DZB_USEW"; // 0x00bd08c8

// `0x0051ee50` (`D3DFILLMODE`).
const D3DFILL_POINT: &str = "D3DFILL_POINT"; // 0x00bd08d4
const D3DFILL_WIREFRAME: &str = "D3DFILL_WIREFRAME"; // 0x00bd08e4
const D3DFILL_SOLID: &str = "D3DFILL_SOLID"; // 0x00bd08f8

// `0x0051ee80` (`D3DSHADEMODE`).
const D3DSHADE_FLAT: &str = "D3DSHADE_FLAT"; // 0x00bd0908
const D3DSHADE_GOURAUD: &str = "D3DSHADE_GOURAUD"; // 0x00bd0918
const D3DSHADE_PHONG: &str = "D3DSHADE_PHONG"; // 0x00bd092c

// `0x0051eeb0` (`D3DBLEND`); the pool packs them descending, cases ascend.
const D3DBLEND_ZERO: &str = "D3DBLEND_ZERO"; // 0x00bd0a48
const D3DBLEND_ONE: &str = "D3DBLEND_ONE"; // 0x00bd0a38
const D3DBLEND_SRCCOLOR: &str = "D3DBLEND_SRCCOLOR"; // 0x00bd0a24
const D3DBLEND_INVSRCCOLOR: &str = "D3DBLEND_INVSRCCOLOR"; // 0x00bd0a0c
const D3DBLEND_SRCALPHA: &str = "D3DBLEND_SRCALPHA"; // 0x00bd09f8
const D3DBLEND_INVSRCALPHA: &str = "D3DBLEND_INVSRCALPHA"; // 0x00bd09e0
const D3DBLEND_DESTALPHA: &str = "D3DBLEND_DESTALPHA"; // 0x00bd09cc
const D3DBLEND_INVDESTALPHA: &str = "D3DBLEND_INVDESTALPHA"; // 0x00bd09b4
const D3DBLEND_DESTCOLOR: &str = "D3DBLEND_DESTCOLOR"; // 0x00bd09a0
const D3DBLEND_INVDESTCOLOR: &str = "D3DBLEND_INVDESTCOLOR"; // 0x00bd0988
const D3DBLEND_SRCALPHASAT: &str = "D3DBLEND_SRCALPHASAT"; // 0x00bd0970
const D3DBLEND_BOTHSRCALPHA: &str = "D3DBLEND_BOTHSRCALPHA"; // 0x00bd0958
const D3DBLEND_BOTHINVSRCALPHA: &str = "D3DBLEND_BOTHINVSRCALPHA"; // 0x00bd093c

// `0x0051ef50` (`D3DCULL`).
const D3DCULL_NONE: &str = "D3DCULL_NONE"; // 0x00bd0a58
const D3DCULL_CW: &str = "D3DCULL_CW"; // 0x00bd0a68
const D3DCULL_CCW: &str = "D3DCULL_CCW"; // 0x00bd0a74

// `0x0051ef80` (`D3DCMPFUNC`).
const D3DCMP_NEVER: &str = "D3DCMP_NEVER"; // 0x00bd0af4
const D3DCMP_LESS: &str = "D3DCMP_LESS"; // 0x00bd0ae8
const D3DCMP_EQUAL: &str = "D3DCMP_EQUAL"; // 0x00bd0ad8
const D3DCMP_LESSEQUAL: &str = "D3DCMP_LESSEQUAL"; // 0x00bd0ac4
const D3DCMP_GREATER: &str = "D3DCMP_GREATER"; // 0x00bd0ab4
const D3DCMP_NOTEQUAL: &str = "D3DCMP_NOTEQUAL"; // 0x00bd0aa4
const D3DCMP_GREATEREQUAL: &str = "D3DCMP_GREATEREQUAL"; // 0x00bd0a90
const D3DCMP_ALWAYS: &str = "D3DCMP_ALWAYS"; // 0x00bd0a80

// `0x0051eff0` (`D3DFOGMODE`).
const D3DFOG_NONE: &str = "D3DFOG_NONE"; // 0x00bd0b2c
const D3DFOG_EXP: &str = "D3DFOG_EXP"; // 0x00bd0b20
const D3DFOG_EXP2: &str = "D3DFOG_EXP2"; // 0x00bd0b14
const D3DFOG_LINEAR: &str = "D3DFOG_LINEAR"; // 0x00bd0b04

// `0x0051f030` (`D3DSTENCILOP`).
const D3DSTENCILOP_KEEP: &str = "D3DSTENCILOP_KEEP"; // 0x00bd0bd0
const D3DSTENCILOP_ZERO: &str = "D3DSTENCILOP_ZERO"; // 0x00bd0bbc
const D3DSTENCILOP_REPLACE: &str = "D3DSTENCILOP_REPLACE"; // 0x00bd0ba4
const D3DSTENCILOP_INCRSAT: &str = "D3DSTENCILOP_INCRSAT"; // 0x00bd0b8c
const D3DSTENCILOP_DECRSAT: &str = "D3DSTENCILOP_DECRSAT"; // 0x00bd0b74
const D3DSTENCILOP_INVERT: &str = "D3DSTENCILOP_INVERT"; // 0x00bd0b60
const D3DSTENCILOP_INCR: &str = "D3DSTENCILOP_INCR"; // 0x00bd0b4c
const D3DSTENCILOP_DECR: &str = "D3DSTENCILOP_DECR"; // 0x00bd0b38

// `0x0051f0a0` (`D3DMATERIALCOLORSOURCE`).
const D3DMCS_MATERIAL: &str = "D3DMCS_MATERIAL"; // 0x00bd0be4
const D3DMCS_COLOR1: &str = "D3DMCS_COLOR1"; // 0x00bd0bf4
const D3DMCS_COLOR2: &str = "D3DMCS_COLOR2"; // 0x00bd0c04

// `0x0051f0d0` (`D3DVBF` vertex-blend modes; the two sentinels are far from 0..3).
const D3DVBF_3WEIGHTS: &str = "D3DVBF_3WEIGHTS"; // 0x00bd0c34
const D3DVBF_DISABLE: &str = "D3DVBF_DISABLE"; // 0x00bd0c44
const D3DVBF_1WEIGHTS: &str = "D3DVBF_1WEIGHTS"; // 0x00bd0c54
const D3DVBF_2WEIGHTS: &str = "D3DVBF_2WEIGHTS"; // 0x00bd0c64
const D3DVBF_TWEENING: &str = "D3DVBF_TWEENING"; // 0x00bd0c14
const D3DVBF_0WEIGHTS: &str = "D3DVBF_0WEIGHTS"; // 0x00bd0c24

// `0x0051f120` (`D3DPATCHEDGESTYLE`).
const D3DPATCHEDGE_DISCRETE: &str = "D3DPATCHEDGE_DISCRETE"; // 0x00bd0c74
const D3DPATCHEDGE_CONTINUOUS: &str = "D3DPATCHEDGE_CONTINUOUS"; // 0x00bd0c8c

// `0x0051f140` (`D3DDISABLEDMATERIALMODE`).
const D3DDMT_ENABLE: &str = "D3DDMT_ENABLE"; // 0x00bd0ca4
const D3DDMT_DISABLE: &str = "D3DDMT_DISABLE"; // 0x00bd0cb4

// `0x0051f160` (`D3DBLENDOP`).
const D3DBLENDOP_ADD: &str = "D3DBLENDOP_ADD"; // 0x00bd0d10
const D3DBLENDOP_SUBTRACT: &str = "D3DBLENDOP_SUBTRACT"; // 0x00bd0cfc
const D3DBLENDOP_REVSUBTRACT: &str = "D3DBLENDOP_REVSUBTRACT"; // 0x00bd0ce4
const D3DBLENDOP_MIN: &str = "D3DBLENDOP_MIN"; // 0x00bd0cd4
const D3DBLENDOP_MAX: &str = "D3DBLENDOP_MAX"; // 0x00bd0cc4

// ---------------------------------------------------------------------------
// 1. Render-state enum -> state-name lookup
// ---------------------------------------------------------------------------

/// Row: `0x0051ee20` — z-buffer state name (`int`, golden run 0x00bd08b0 stride 0x0c).
pub fn zbuffer_type_name(state: i32) -> &'static str {
    match state {
        0 => D3DZB_FALSE,
        1 => D3DZB_TRUE,
        2 => D3DZB_USEW,
        _ => UNKNOWN,
    }
}

/// Row: `0x0051ee50` — fill-mode name (`int`). Note `0` is *not* mapped — the game
/// answers `UNKNOWN` for `D3DFILL_POINT`'s wire value 0.
pub fn fill_mode_name(state: i32) -> &'static str {
    match state {
        1 => D3DFILL_POINT,
        2 => D3DFILL_WIREFRAME,
        3 => D3DFILL_SOLID,
        _ => UNKNOWN,
    }
}

/// Row: `0x0051ee80` — shade-mode name (`int`).
pub fn shade_mode_name(state: i32) -> &'static str {
    match state {
        1 => D3DSHADE_FLAT,
        2 => D3DSHADE_GOURAUD,
        3 => D3DSHADE_PHONG,
        _ => UNKNOWN,
    }
}

/// Row: `0x0051eeb0` — blend-factor name (`undefined4` -> `u32`, 13 cases).
pub fn blend_func_name(state: u32) -> &'static str {
    match state {
        1 => D3DBLEND_ZERO,
        2 => D3DBLEND_ONE,
        3 => D3DBLEND_SRCCOLOR,
        4 => D3DBLEND_INVSRCCOLOR,
        5 => D3DBLEND_SRCALPHA,
        6 => D3DBLEND_INVSRCALPHA,
        7 => D3DBLEND_DESTALPHA,
        8 => D3DBLEND_INVDESTALPHA,
        9 => D3DBLEND_DESTCOLOR,
        10 => D3DBLEND_INVDESTCOLOR,
        0xb => D3DBLEND_SRCALPHASAT,
        0xc => D3DBLEND_BOTHSRCALPHA,
        0xd => D3DBLEND_BOTHINVSRCALPHA,
        _ => UNKNOWN,
    }
}

/// Row: `0x0051ef50` — cull-mode name (`int`).
pub fn cull_mode_name(state: i32) -> &'static str {
    match state {
        1 => D3DCULL_NONE,
        2 => D3DCULL_CW,
        3 => D3DCULL_CCW,
        _ => UNKNOWN,
    }
}

/// Row: `0x0051ef80` — compare-function name (`undefined4` -> `u32`).
pub fn compare_func_name(state: u32) -> &'static str {
    match state {
        1 => D3DCMP_NEVER,
        2 => D3DCMP_LESS,
        3 => D3DCMP_EQUAL,
        4 => D3DCMP_LESSEQUAL,
        5 => D3DCMP_GREATER,
        6 => D3DCMP_NOTEQUAL,
        7 => D3DCMP_GREATEREQUAL,
        8 => D3DCMP_ALWAYS,
        _ => UNKNOWN,
    }
}

/// Row: `0x0051eff0` — fog-mode name (`undefined4` -> `u32`); the only lookup in the
/// family whose case list starts at 0 and whose order is *not* the enum order
/// (`0` -> `D3DFOG_NONE`, `3` -> `D3DFOG_LINEAR`).
pub fn fog_mode_name(state: u32) -> &'static str {
    match state {
        0 => D3DFOG_NONE,
        1 => D3DFOG_EXP,
        2 => D3DFOG_EXP2,
        3 => D3DFOG_LINEAR,
        _ => UNKNOWN,
    }
}

/// Row: `0x0051f030` — stencil-op name (`undefined4` -> `u32`).
pub fn stencil_op_name(state: u32) -> &'static str {
    match state {
        1 => D3DSTENCILOP_KEEP,
        2 => D3DSTENCILOP_ZERO,
        3 => D3DSTENCILOP_REPLACE,
        4 => D3DSTENCILOP_INCRSAT,
        5 => D3DSTENCILOP_DECRSAT,
        6 => D3DSTENCILOP_INVERT,
        7 => D3DSTENCILOP_INCR,
        8 => D3DSTENCILOP_DECR,
        _ => UNKNOWN,
    }
}

/// Row: `0x0051f0a0` — material-colour-source name (`int`).
pub fn material_color_source_name(state: i32) -> &'static str {
    match state {
        0 => D3DMCS_MATERIAL,
        1 => D3DMCS_COLOR1,
        2 => D3DMCS_COLOR2,
        _ => UNKNOWN,
    }
}

/// Row: `0x0051f0d0` — vertex-blend-mode name (`uint` -> `u32`, unsigned compare).
/// The game tests `state < 4` and then re-tests the four low values, else it
/// special-cases only `0xff`/`0x100`; the six arms are disjoint, so the flat match
/// is exactly equivalent. Anything `>= 4` that is not a sentinel (including
/// negative callers, which wrap into the `>= 4` branch) answers `UNKNOWN`.
pub fn vertex_blend_mode_name(state: u32) -> &'static str {
    match state {
        0 => D3DVBF_DISABLE,
        1 => D3DVBF_1WEIGHTS,
        2 => D3DVBF_2WEIGHTS,
        3 => D3DVBF_3WEIGHTS,
        0xff => D3DVBF_TWEENING,
        0x100 => D3DVBF_0WEIGHTS,
        _ => UNKNOWN,
    }
}

/// Row: `0x0051f120` — patch-edge-style name (`int`).
pub fn patch_edge_style_name(state: i32) -> &'static str {
    match state {
        0 => D3DPATCHEDGE_DISCRETE,
        1 => D3DPATCHEDGE_CONTINUOUS,
        _ => UNKNOWN,
    }
}

/// Row: `0x0051f140` — disabled-material-mode name (`int`).
pub fn disabled_material_mode_name(state: i32) -> &'static str {
    match state {
        0 => D3DDMT_ENABLE,
        1 => D3DDMT_DISABLE,
        _ => UNKNOWN,
    }
}

/// Row: `0x0051f160` — blend-op name (`undefined4` -> `u32`).
pub fn blend_op_name(state: u32) -> &'static str {
    match state {
        1 => D3DBLENDOP_ADD,
        2 => D3DBLENDOP_SUBTRACT,
        3 => D3DBLENDOP_REVSUBTRACT,
        4 => D3DBLENDOP_MIN,
        5 => D3DBLENDOP_MAX,
        _ => UNKNOWN,
    }
}

// ---------------------------------------------------------------------------
// 2. D3DPERF (PIX event) wide-string hooks
// ---------------------------------------------------------------------------

/// Width of both rows' stack buffer (`short local_200[256]` / `short asStack_200[256]`).
pub const WIDE_NAME_UNITS: usize = 256;

/// The unrolled 4-at-a-time ANSI -> `short` widening shared by 0x0051f340 and
/// 0x0051f3d0. The game sign-extends each `char` into a `short` and stops at the
/// first zero (the zero *is* written, nothing after it); if no zero appears it
/// stops after unit `0xff`, leaving the 256-unit buffer unterminated.
///
/// The C reads at most 256 source bytes; a `src` shorter than that with no zero
/// byte reads past the caller's buffer (UB) — the Rust index panics instead.
fn widen_name(src: &[u8], dst: &mut [i16; WIDE_NAME_UNITS]) {
    let mut unit = 0usize;
    loop {
        for k in 0..4 {
            let wide = (src[unit + k] as i8) as i16;
            dst[unit + k] = wide;
            if wide == 0 {
                return;
            }
        }
        unit += 4;
        if unit > 0xff {
            return;
        }
    }
}

/// Row: `0x0051f340` — `D3DPERF_BeginEvent(color, wide_name)` wrapper.
/// `hook` is the global slot `DAT_00dedbdc` (installed at 0x00525620:118 from
/// `"D3DPERF_BeginEvent"`, string 0x00bd11f4); `None` models the NULL slot — the
/// guard skips *both* the conversion and the call. `name` is the `char*`
/// (`param_1`), `color` the pass-through first argument (`param_2`).
///
/// Units past the terminator stay uninitialised stack in the game; the mirror
/// leaves them zero, which no callee can observe because it stops at the
/// terminator.
pub fn begin_event(hook: Option<impl FnOnce(u32, &mut [i16; WIDE_NAME_UNITS])>, color: u32, name: &[u8]) {
    if let Some(hook) = hook {
        let mut buf = [0i16; WIDE_NAME_UNITS];
        widen_name(name, &mut buf);
        hook(color, &mut buf);
    }
}

/// Row: `0x0051f3d0` — `D3DPERF_SetMarker(color, wide_name)` wrapper — identical body
/// to 0x0051f340 through the second hook slot. The slot reads as `_DAT_00dedbe4`
/// in the decompile (Ghidra's "overlapping smaller symbol" warning); the value
/// stored at 0x00525620:120 is the 4-byte `GetProcAddress` result for
/// `"D3DPERF_SetMarker"` (string 0x00bd11cc), so the mirror models one hook
/// pointer exactly like `DAT_00dedbdc`.
pub fn set_marker(hook: Option<impl FnOnce(u32, &mut [i16; WIDE_NAME_UNITS])>, color: u32, name: &[u8]) {
    if let Some(hook) = hook {
        let mut buf = [0i16; WIDE_NAME_UNITS];
        widen_name(name, &mut buf);
        hook(color, &mut buf);
    }
}

// ---------------------------------------------------------------------------
// 3. Refcounted render-state object slot
// ---------------------------------------------------------------------------

/// Row: `0x0051f4b0` — store a refcounted object pointer into state slot
/// `(&DAT_00dee5e0)[index]` (callers pass stage indices 0..3 — e.g. 0x0049582b:147–150).
///
/// Exact order and guards of the game body:
/// 1. nothing at all when `incoming == *slot`;
/// 2. `if (incoming != 0) *(short *)(incoming + 4) += 1;` — the incoming object's
///    refcount cell (field offset +4) bumps with 16-bit wrap; `None` models the
///    NULL pointer (no cell, no bump);
/// 3. `if (*slot != 0) FUN_00a1ed10(old);` — the global release helper (decrements
///    the old object's +4 refcount and, on zero, runs its vtable+0x20 delete);
///    modelled as the caller-supplied `release_old`, called exactly when the old
///    slot was non-NULL and before the store;
/// 4. store, then raise the cache dirty bit `DAT_00dec4f4 |= 0x40 << ((byte)index & 0x1f)`;
///    x86 32-bit shifting means indices 26..31 (and the `0xff`-truncated negative
///    forms) set no bit at all.
pub fn set_state_object(
    slot: &mut i32,
    dirty: &mut u32,
    index: i32,
    incoming: i32,
    incoming_refcount: Option<&mut i16>,
    release_old: impl FnOnce(),
) {
    let old = *slot;
    if incoming == old {
        return;
    }
    if let Some(cell) = incoming_refcount {
        *cell = cell.wrapping_add(1);
    }
    if old != 0 {
        release_old();
    }
    *slot = incoming;
    *dirty |= 0x40u32.wrapping_shl(((index as u8) & 0x1f) as u32);
}

// ---------------------------------------------------------------------------
// 4. DirectX lock ownership predicates
// ---------------------------------------------------------------------------

/// Row: `0x0051f500` — "no wait needed" — the calling thread already owns the DirectX
/// lock (`DAT_00deda88`) **or** the lock recursion count (`DAT_00deda8c`) is zero.
/// `current_tid` stands in for `GetCurrentThreadId()`; the two globals are passed
/// by value, matching the `fxflags`/`drawdata` "caller resolves the global" rule.
pub fn lock_may_enter(current_tid: u32, owner_tid: u32, lock_count: u32) -> bool {
    current_tid == owner_tid || lock_count == 0
}

/// Row: `0x0051f600` — "this thread holds it" — owner match **and** non-zero
/// recursion count. (The owner/count writes themselves live in the skipped
/// OS-call rows 0x0051f520/0x0051f5b0.)
pub fn lock_held_by(current_tid: u32, owner_tid: u32, lock_count: u32) -> bool {
    current_tid == owner_tid && lock_count != 0
}

// ---------------------------------------------------------------------------
// 5. Float clamp
// ---------------------------------------------------------------------------

/// Row: `0x0051f680` — three-way clamp in the game's test order —
/// `if (v < lo) return lo; if (hi < v) return hi; return v;`.
/// Because the low-bound test runs first and returns unconditionally, `lo > hi`
/// yields `lo` (no re-clamp); NaN operands make both tests false and return `v`,
/// exactly like MSVC's `cmp`/`jb` sequence. Ghidra's `float10` return type is the
/// x87 register artifact; the value is the incoming 32-bit float.
pub fn clamped(value: f32, low: f32, high: f32) -> f32 {
    if value < low {
        low
    } else if high < value {
        high
    } else {
        value
    }
}

// ---------------------------------------------------------------------------
// 6. Fixed-stride record table helpers
// ---------------------------------------------------------------------------

/// Shared owner (`this`) layout of rows 0x0051f910 / 0x0051f960:
///
/// ```text
/// +0x00  header / vtable (untouched by both rows)
/// +0x04  int*  data    record array, 16-byte (4-dword) records
/// +0x08  int   count   record count (signed; <= 0 means "empty")
/// +0x0c  byte  valid   table-initialised flag
/// ```
pub struct RecordTable<'a> {
    pub data: &'a [[i32; 4]],
    pub count: i32,
    pub valid: u8,
}

/// Row: `0x0051f910` — linear record search. Compares record dwords 0, 1 and 2 with
/// `key`; dword 3 is never compared. Returns the record index on the first hit,
/// `-1` when the table is valid but holds no match (including `count <= 0`), and
/// **`0`** when the `+0xc` valid byte is clear — the game's guard falls through to
/// `return iVar1` with `iVar1` still 0, which callers read as "slot 0 matched".
///
/// The loop is the game's `do { … } while (++i < count)`; a `count` larger than
/// `data.len()` reads past the array in the game (UB) and panics here.
pub fn find_record(table: &RecordTable<'_>, key: &[i32; 3]) -> i32 {
    if table.valid == 0 {
        return 0;
    }
    if table.count > 0 {
        let mut index = 0i32;
        loop {
            let record = table.data[index as usize];
            if record[0] == key[0] && record[1] == key[1] && record[2] == key[2] {
                return index;
            }
            index += 1;
            if index >= table.count {
                break;
            }
        }
    }
    -1
}

/// Row: `0x0051f960` — record slot addressed by `byte_delta` = `param_2 - *(this + 4)`
/// (pointer subtraction done in the caller). The game casts the difference to
/// `uint` and shifts `>> 4` — an unsigned divide by 16 with no negative check, so
/// an address below the array base yields a huge slot index (`0xFFFFFFF_0` for a
/// delta of `-16`). `0` when the `+0xc` valid byte is clear.
pub fn record_slot(table: &RecordTable<'_>, byte_delta: i32) -> u32 {
    if table.valid == 0 {
        return 0;
    }
    (byte_delta as u32) >> 4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zbuffer_and_two_entry_tables_map_exactly() {
        assert_eq!(zbuffer_type_name(0), "D3DZB_FALSE");
        assert_eq!(zbuffer_type_name(1), "D3DZB_TRUE");
        assert_eq!(zbuffer_type_name(2), "D3DZB_USEW");
        assert_eq!(zbuffer_type_name(3), "UNKNOWN");
        assert_eq!(zbuffer_type_name(-1), "UNKNOWN");

        // 0x0051f120 / 0x0051f140: 0 and 1 only.
        assert_eq!(patch_edge_style_name(0), "D3DPATCHEDGE_DISCRETE");
        assert_eq!(patch_edge_style_name(1), "D3DPATCHEDGE_CONTINUOUS");
        assert_eq!(patch_edge_style_name(2), "UNKNOWN");
        assert_eq!(disabled_material_mode_name(0), "D3DDMT_ENABLE");
        assert_eq!(disabled_material_mode_name(1), "D3DDMT_DISABLE");
        assert_eq!(disabled_material_mode_name(-1), "UNKNOWN");
    }

    #[test]
    fn three_entry_tables_leave_zero_unmapped() {
        // 0x0051ee50 / 0x0051ee80 / 0x0051ef50 / 0x0051f0a0 start at 1 except MCS.
        for state in [1i32, 2, 3] {
            let fill = fill_mode_name(state);
            let shade = shade_mode_name(state);
            let cull = cull_mode_name(state);
            match state {
                1 => assert_eq!((fill, shade, cull), ("D3DFILL_POINT", "D3DSHADE_FLAT", "D3DCULL_NONE")),
                2 => assert_eq!(
                    (fill, shade, cull),
                    ("D3DFILL_WIREFRAME", "D3DSHADE_GOURAUD", "D3DCULL_CW")
                ),
                _ => assert_eq!(
                    (fill, shade, cull),
                    ("D3DFILL_SOLID", "D3DSHADE_PHONG", "D3DCULL_CCW")
                ),
            }
        }
        assert_eq!(fill_mode_name(0), "UNKNOWN");
        assert_eq!(shade_mode_name(0), "UNKNOWN");
        assert_eq!(cull_mode_name(0), "UNKNOWN");
        assert_eq!(cull_mode_name(4), "UNKNOWN");

        assert_eq!(material_color_source_name(0), "D3DMCS_MATERIAL");
        assert_eq!(material_color_source_name(1), "D3DMCS_COLOR1");
        assert_eq!(material_color_source_name(2), "D3DMCS_COLOR2");
        assert_eq!(material_color_source_name(3), "UNKNOWN");
    }

    #[test]
    fn wide_switch_tables_cover_every_case_and_miss() {
        let blend: [&str; 13] = [
            "D3DBLEND_ZERO",
            "D3DBLEND_ONE",
            "D3DBLEND_SRCCOLOR",
            "D3DBLEND_INVSRCCOLOR",
            "D3DBLEND_SRCALPHA",
            "D3DBLEND_INVSRCALPHA",
            "D3DBLEND_DESTALPHA",
            "D3DBLEND_INVDESTALPHA",
            "D3DBLEND_DESTCOLOR",
            "D3DBLEND_INVDESTCOLOR",
            "D3DBLEND_SRCALPHASAT",
            "D3DBLEND_BOTHSRCALPHA",
            "D3DBLEND_BOTHINVSRCALPHA",
        ];
        for (case, name) in blend.iter().enumerate() {
            assert_eq!(blend_func_name(case as u32 + 1), *name, "case {case}");
        }
        assert_eq!(blend_func_name(0), "UNKNOWN");
        assert_eq!(blend_func_name(14), "UNKNOWN");
        assert_eq!(blend_func_name(u32::MAX), "UNKNOWN"); // a negative caller

        let cmp: [&str; 8] = [
            "D3DCMP_NEVER",
            "D3DCMP_LESS",
            "D3DCMP_EQUAL",
            "D3DCMP_LESSEQUAL",
            "D3DCMP_GREATER",
            "D3DCMP_NOTEQUAL",
            "D3DCMP_GREATEREQUAL",
            "D3DCMP_ALWAYS",
        ];
        for (case, name) in cmp.iter().enumerate() {
            assert_eq!(compare_func_name(case as u32 + 1), *name, "case {case}");
        }
        assert_eq!(compare_func_name(9), "UNKNOWN");

        let stencil: [&str; 8] = [
            "D3DSTENCILOP_KEEP",
            "D3DSTENCILOP_ZERO",
            "D3DSTENCILOP_REPLACE",
            "D3DSTENCILOP_INCRSAT",
            "D3DSTENCILOP_DECRSAT",
            "D3DSTENCILOP_INVERT",
            "D3DSTENCILOP_INCR",
            "D3DSTENCILOP_DECR",
        ];
        for (case, name) in stencil.iter().enumerate() {
            assert_eq!(stencil_op_name(case as u32 + 1), *name, "case {case}");
        }
        assert_eq!(stencil_op_name(0), "UNKNOWN");
        assert_eq!(stencil_op_name(0xff), "UNKNOWN");

        assert_eq!(fog_mode_name(0), "D3DFOG_NONE");
        assert_eq!(fog_mode_name(1), "D3DFOG_EXP");
        assert_eq!(fog_mode_name(2), "D3DFOG_EXP2");
        assert_eq!(fog_mode_name(3), "D3DFOG_LINEAR");
        assert_eq!(fog_mode_name(4), "UNKNOWN");

        let blendop: [&str; 5] = [
            "D3DBLENDOP_ADD",
            "D3DBLENDOP_SUBTRACT",
            "D3DBLENDOP_REVSUBTRACT",
            "D3DBLENDOP_MIN",
            "D3DBLENDOP_MAX",
        ];
        for (case, name) in blendop.iter().enumerate() {
            assert_eq!(blend_op_name(case as u32 + 1), *name, "case {case}");
        }
        assert_eq!(blend_op_name(6), "UNKNOWN");
    }

    #[test]
    fn vertex_blend_sentinels_are_unsigned() {
        assert_eq!(vertex_blend_mode_name(0), "D3DVBF_DISABLE");
        assert_eq!(vertex_blend_mode_name(1), "D3DVBF_1WEIGHTS");
        assert_eq!(vertex_blend_mode_name(2), "D3DVBF_2WEIGHTS");
        assert_eq!(vertex_blend_mode_name(3), "D3DVBF_3WEIGHTS");
        assert_eq!(vertex_blend_mode_name(4), "UNKNOWN");
        assert_eq!(vertex_blend_mode_name(0xfe), "UNKNOWN");
        assert_eq!(vertex_blend_mode_name(0xff), "D3DVBF_TWEENING");
        assert_eq!(vertex_blend_mode_name(0x100), "D3DVBF_0WEIGHTS");
        assert_eq!(vertex_blend_mode_name(0x101), "UNKNOWN");
        // (uint)-1 == 0xffffffff fails both `== 0xff` and `== 0x100`.
        assert_eq!(vertex_blend_mode_name(-1i32 as u32), "UNKNOWN");
    }

    #[test]
    fn perf_hooks_widen_until_the_terminator() {
        let mut seen: Option<(u32, [i16; WIDE_NAME_UNITS])> = None;
        begin_event(
            Some(|color: u32, buf: &mut [i16; WIDE_NAME_UNITS]| {
                seen = Some((color, *buf));
            }),
            0x1122_3344,
            b"AB\0tail",
        );
        let (color, buf) = seen.expect("hook must run");
        assert_eq!(color, 0x1122_3344);
        assert_eq!(&buf[0..3], &[b'A' as i16, b'B' as i16, 0]);
        assert_eq!(buf[3], 0, "nothing after the terminator is written");

        // 0x0051f3d0 goes through its own hook slot.
        let mut marker_units = 0usize;
        set_marker(
            Some(|_color: u32, buf: &mut [i16; WIDE_NAME_UNITS]| {
                marker_units = buf.iter().take_while(|u| **u != 0).count();
                assert_eq!(&buf[0..4], &[1, 2, 3, 4]);
            }),
            0,
            &[1u8, 2, 3, 4, 0],
        );
        assert_eq!(marker_units, 4);
    }

    #[test]
    fn perf_hook_sign_extends_and_caps_at_256_units() {
        let mut buf = [0i16; WIDE_NAME_UNITS];
        // 0x80 widens to (short)0x80 = -128: the game sign-extends `char`.
        widen_name(&[0x80, 0x7f, 0x00], &mut buf);
        assert_eq!(&buf[0..3], &[-128i16, 127, 0]);

        // No terminator: the unrolled loop stops after unit 0xff, buffer full.
        let mut long = vec![b'x'; 400];
        long.push(0);
        let mut full = [0i16; WIDE_NAME_UNITS];
        widen_name(&long, &mut full);
        assert!(full.iter().all(|u| *u == b'x' as i16), "256 units written, unterminated");
    }

    #[test]
    fn null_perf_hook_skips_conversion_and_call() {
        // Rows 0x0051f340 / 0x0051f3d0: the guard runs first, so a NULL slot reads
        // nothing (widening an empty buffer would index it) and calls nothing.
        begin_event(None::<fn(u32, &mut [i16; WIDE_NAME_UNITS])>, 7, &[]);
        set_marker(None::<fn(u32, &mut [i16; WIDE_NAME_UNITS])>, 7, &[]);
        let mut calls = 0usize;
        // A live hook still runs for an empty C string (a single NUL byte).
        set_marker(
            Some(|_c: u32, _b: &mut [i16; WIDE_NAME_UNITS]| calls += 1),
            7,
            b"\0",
        );
        assert_eq!(calls, 1);
    }

    #[test]
    fn state_object_store_orders_refcount_release_and_dirty_bit() {
        let mut slot = 0i32;
        let mut dirty = 0u32;
        let mut refs = 0i16;
        let mut released: Vec<i32> = Vec::new();

        // NULL slot -> first store: no release, one AddRef of the incoming object.
        set_state_object(&mut slot, &mut dirty, 0, 0x1000, Some(&mut refs), || {
            released.push(-1)
        });
        assert_eq!((slot, refs, dirty), (0x1000, 1, 0x40));
        assert!(released.is_empty());

        // Same pointer: whole body skipped (no refcount, no dirty change).
        set_state_object(&mut slot, &mut dirty, 1, 0x1000, Some(&mut refs), || {
            released.push(1)
        });
        assert_eq!((slot, refs, dirty), (0x1000, 1, 0x40));
        assert!(released.is_empty());

        // Replace: the OLD object is released exactly once (the game runs the
        // release before the store), then dirty bit 1 is raised.
        let old = slot;
        set_state_object(&mut slot, &mut dirty, 1, 0x2000, Some(&mut refs), || released.push(old));
        assert_eq!(released, vec![0x1000]);
        assert_eq!((slot, refs, dirty), (0x2000, 2, 0x40 | 0x80));

        // Incoming NULL: no refcount cell to bump, old still released.
        let old = slot;
        set_state_object(&mut slot, &mut dirty, 2, 0, None, || released.push(old));
        assert_eq!(released, vec![0x1000, 0x2000]);
        assert_eq!((slot, refs, dirty), (0, 2, 0x40 | 0x80 | 0x100));

        // Clearing an already-NULL slot changes nothing.
        set_state_object(&mut slot, &mut dirty, 3, 0, None, || released.push(0));
        assert_eq!(released.len(), 2);
    }

    #[test]
    fn state_object_dirty_shift_wraps_like_x86() {
        let mut slot = 0i32;
        let mut dirty = 0u32;
        set_state_object(&mut slot, &mut dirty, 25, 1, None, || {});
        assert_eq!(dirty, 0x8000_0000);
        set_state_object(&mut slot, &mut dirty, 26, 2, None, || {});
        assert_eq!(dirty, 0x8000_0000, "0x40 << 26 overflows the 32-bit shift");

        let mut dirty = 0u32;
        let mut slot = 0i32;
        // (byte)-1 == 0xff, & 0x1f -> 31: shift overflows, no bit raised.
        set_state_object(&mut slot, &mut dirty, -1, 5, None, || {});
        assert_eq!((slot, dirty), (5, 0));
        // index 32 folds to the same bit as index 0.
        let mut dirty = 0u32;
        let mut slot = 0i32;
        set_state_object(&mut slot, &mut dirty, 32, 6, None, || {});
        assert_eq!(dirty, 0x40);
    }

    #[test]
    fn lock_predicates_are_the_exact_boolean_shapes() {
        // 0x0051f500: owner OR unheld.
        assert!(lock_may_enter(7, 7, 3));
        assert!(lock_may_enter(7, 7, 0));
        assert!(lock_may_enter(1, 2, 0));
        assert!(!lock_may_enter(1, 2, 1));
        assert!(!lock_may_enter(1, 2, u32::MAX));

        // 0x0051f600: owner AND held.
        assert!(lock_held_by(7, 7, 1));
        assert!(!lock_held_by(7, 7, 0));
        assert!(!lock_held_by(1, 2, 4));
        assert!(!lock_held_by(0, 0, 0));
    }

    #[test]
    fn clamp_uses_the_games_test_order() {
        assert_eq!(clamped(5.0, 0.0, 10.0), 5.0);
        assert_eq!(clamped(-3.0, 0.0, 10.0), 0.0);
        assert_eq!(clamped(30.0, 0.0, 10.0), 10.0);
        assert_eq!(clamped(0.0, 0.0, 10.0), 0.0);
        assert_eq!(clamped(10.0, 0.0, 10.0), 10.0);
        // Inverted bounds: the low test wins because it returns first.
        assert_eq!(clamped(5.0, 20.0, 10.0), 20.0);
        assert_eq!(clamped(50.0, 20.0, 10.0), 10.0);
        // NaN comparisons are false in C and in Rust: both fall through to value.
        assert!(clamped(f32::NAN, 0.0, 10.0).is_nan());
        assert_eq!(clamped(5.0, f32::NAN, f32::NAN), 5.0);
        assert_eq!(clamped(1.0f32 / 3.0, 0.0, 1.0).to_bits(), (1.0f32 / 3.0).to_bits());
    }

    #[test]
    fn record_search_compares_only_three_dwords() {
        let table = RecordTable {
            data: &[
                [1, 2, 3, 0x7fff_ffff],
                [4, 5, 6, 0],
                [7, 8, 9, 0x1234_5678],
            ],
            count: 3,
            valid: 1,
        };
        assert_eq!(find_record(&table, &[1, 2, 3]), 0);
        assert_eq!(find_record(&table, &[4, 5, 6]), 1);
        assert_eq!(find_record(&table, &[7, 8, 9]), 2);
        // dword 3 of the record is never compared -> still a hit.
        assert_eq!(find_record(&table, &[7, 8, 9]), 2);
        assert_eq!(find_record(&table, &[7, 8, 10]), -1);
        assert_eq!(find_record(&table, &[0, 0, 0]), -1);
    }

    #[test]
    fn record_search_guards_valid_and_count() {
        let data = [[1, 2, 3, 4]; 3];
        let invalid = RecordTable { data: &data, count: 3, valid: 0 };
        assert_eq!(find_record(&invalid, &[9, 9, 9]), 0, "guard falls through with index 0");
        assert_eq!(find_record(&invalid, &[1, 2, 3]), 0);

        for count in [0i32, -1, i32::MIN] {
            let empty = RecordTable { data: &data, count, valid: 1 };
            assert_eq!(find_record(&empty, &[1, 2, 3]), -1, "count {count}");
        }

        // count == 1 stops the scan at record 0 even though the array is longer.
        let short = RecordTable { data: &data, count: 1, valid: 1 };
        assert_eq!(find_record(&short, &[4, 5, 6]), -1);
        assert_eq!(find_record(&short, &[1, 2, 3]), 0);
    }

    #[test]
    #[should_panic]
    fn record_search_count_past_array_end_panics() {
        // The game reads past the record array here (UB); the mirror panics.
        let data = [[1, 2, 3, 4]; 2];
        let table = RecordTable { data: &data, count: 5, valid: 1 };
        let _ = find_record(&table, &[9, 9, 9]);
    }

    #[test]
    fn record_slot_is_an_unsigned_divide_by_16() {
        let table = RecordTable { data: &[], count: 0, valid: 1 };
        assert_eq!(record_slot(&table, 0), 0);
        assert_eq!(record_slot(&table, 15), 0);
        assert_eq!(record_slot(&table, 16), 1);
        assert_eq!(record_slot(&table, 16 * 7), 7);
        assert_eq!(record_slot(&table, 0x1_0000), 0x1000);
        // Negative delta: cast to uint first, then the logical shift.
        assert_eq!(record_slot(&table, -16), (-16i32 as u32) >> 4);
        assert_eq!(record_slot(&table, -1), 0x0fff_ffff);

        let invalid = RecordTable { data: &[], count: 9, valid: 0 };
        assert_eq!(record_slot(&invalid, 16 * 7), 0);
    }
}
