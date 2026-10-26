//! Wave 48 (`ai1`): the GameLogic/AI + SkirmishAI band — 8 rows (module tag
//! `AI`). Cross-module callees are modelled as closures or a `dyn` host (the
//! `pathfinder1.rs` hook idiom). Offsets and quirks are quoted from the
//! decompiled bodies; callee identities, enum spellings and member names are
//! reference-tree facts (facts only), noted `rowed <name>`.
//!
//! | addr | size | their symbol | one-line role |
//! |---|---|---|---|
//! | `0x00741c54` | 200 | `?allow@Rva00341C54Filter@@UAE_NPAVObject@@@Z` | partition-filter `allow`: dead/peer-bit/relationship/physics-pair rejects, else the pathfinder engagement-spot search on the candidate position |
//! | `0x0081f369` | 200 | `?rva0041F369@ArmyDefinition@@QAEXXZ` | rescale the three weight columns (`+0x04/+0x08/+0x0c`) of the army entries so each column sums to `K` |
//! | `0x0075487e` | 199 | `?update@AIFaceState@@UAE?AW4StateReturnType@@XZ` | face-toward state update: `fabs(relative angle) < tol` → success, else turn through the AI/aim virtuals |
//! | `0x00763df9` | 199 | `?PeekPastClimbPortal@Path@@QAEPAUCoord3D@@PAU2@@Z` | copy up to three nodes past the climb-portal node into the out `Coord3D` |
//! | `0x008eacf5` | 197 | `?rva004EACF5@@YAMPBVCoord2D@@00@Z` | `(a-c)·(b-c)` strictly inside `|b-c|²` (AI wall corner test) |
//! | `0x008d8dea` | 194 | `?updateTurretAI@TurretAI@@QAE?AW4UpdateSleepTime@@XZ` | turret-AI wake bookkeeping, state-id-4 fire gate, sleep re-arm |
//! | `0x007705c2` | 190 | `?cohereAttackState@AIGroup@@QAEXXZ` | pick the first member's current victim, then order every able/idle member without a victim to attack that position |
//! | `0x0074afcd` | 189 | `?update@AIAttackPositionFireWeaponState@@UAE?AW4StateReturnType@@XZ` | weapon-status gate, then `fireCurrentWeapon` / `preFireCurrentWeapon` |
//!
//! Faithful-quirk notes pinned by tests:
//! - `0x00741c54` returns the Ghidra `uVar2 & 0xffffff00` on every reject: the
//!   mangled `_N` ABI reads only `AL`, so *all* reject paths observe `false`.
//!   `FUN_0068d156` (`rowed Object::getRelationship`) is skipped by the first
//!   two byte tests, and `FUN_00790533` (`rowed` physics flag-pair check) is
//!   reached only when the candidate's `+0x25c` slot is non-null. The log gate
//!   (`TheGameLogic + 0x1b4 > 0 && DAT_00dfeff0 != 0`) prints only `x`/`y` of
//!   the copied position; the `z` dword is copied but never logged.
//! - `0x0081f369` sums all three columns in one pass (`+0x08`, `+0x04`,
//!   `+0x0c` per element), re-reads the `K` global for the `+0x04` divisor but
//!   uses the entry-time copy for the other two, and skips a column whose sum
//!   is not `> 0.0` — so `0.0`, a negative sum or a NaN sum leaves that column
//!   untouched (no divide-by-zero, no scaling).
//! - `0x0075487e` resolves `machine → owner → ai` unconditionally at entry
//!   (the game's null chain would be a wild read; the caller supplies it). The
//!   angle is the x87 `long double` result of `rowed Object::GetRelativeAngle`
//!   (`0x004b4542`) compared as a double against `(double)tol`; `NaN` fails the
//!   `<` and takes the turning branch. The `mode == 2` aim-interface getter
//!   `0x0069439d` is *not* called when `mode != 2` (the `&&` short-circuit).
//! - `0x00763df9` copies `+0x0c/+0x10/+0x14` and stores `z` last through the
//!   shared `LAB`; the third node past the portal is used only when its
//!   `+0x20` dword differs from `0x7fffffff`.
//! - `0x008eacf5` keeps both comparisons strict, so `a == b` (dot == `|b-c|²`)
//!   and any perpendicular/NaN case are false; the mangled `YAM` return makes
//!   the two bare decompiler `return;` statements the `true`/`false` arms.
//! - `0x008d8dea` returns the *unsigned* frames remaining while asleep
//!   (`sleep_until - frame`); the state-id-4 gate skips the whole firing block
//!   (so `m_didFire` is not cleared) and sleeps `UPDATE_SLEEP_FOREVER`; the
//!   BFME-2 delta re-reads the machine slot after `updateStateMachine` and
//!   returns `UPDATE_SLEEP_NONE` without re-arming when it was deleted.
//! - `0x007705c2` calls `rowed AIUpdateInterface::getCurrentVictim` (`0x00668d71`)
//!   *twice* per candidate — the `if` tests the first call, the assignment
//!   stores the second — and phase 2 restarts the walk from the list head.
//! - `0x0074afcd` orders the tests as current-weapon, then the `+0x438` bit0
//!   check, then `rowed Object::testStatus(0x52)`; `4` is `PRE_ATTACK`
//!   (keep waiting → `STATE_CONTINUE`), `0` is `READY_TO_FIRE`.

/// `Coord3D` / `Coord2D` scalar records (IEEE f32 throughout).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AiVec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AiVec2 {
    pub x: f32,
    pub y: f32,
}

/// `StateReturnType` (`rowed` spelling: `STATE_CONTINUE = 0`,
/// `STATE_SUCCESS = -1`, `STATE_FAILURE = -2`).
pub const STATE_CONTINUE: i32 = 0;
pub const STATE_SUCCESS: i32 = -1;
pub const STATE_FAILURE: i32 = -2;

/// `UpdateSleepTime` (`UPDATE_SLEEP_INVALID = 0`, `UPDATE_SLEEP_NONE = 1`,
/// `UPDATE_SLEEP_FOREVER = 0x3fffffff`).
pub const UPDATE_SLEEP_INVALID: u32 = 0;
pub const UPDATE_SLEEP_NONE: u32 = 1;
pub const UPDATE_SLEEP_FOREVER: u32 = 0x3fff_ffff;

// ---------------------------------------------------------------------------
// 0x00741c54 — Rva00341C54Filter::allow(Object *)
// ---------------------------------------------------------------------------

/// The candidate `Object` as this filter reads it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AiFilterCandidate {
    /// `Object::m_privateStatus @ +0x438` (bit0 = effectively dead).
    pub status_438: u8,
    /// `Object::+0x25c` pointer slot (`0` = clear; non-null enables the
    /// physics flag-pair test).
    pub slot_25c: i32,
    /// `Object::m_pos @ +0x38..+0x44` — the stack triple the body copies.
    pub pos: AiVec3,
}

/// The filter object itself (`this + 0x08` / `this + 0x0c`).
#[derive(Clone, Copy, Debug, Default)]
pub struct Rva00341C54Filter {
    /// `filter+0x08` — the reference `Object *` handed to the search.
    pub object: i32,
    /// `filter+0x08` object's status byte (`*(object + 0x438)`).
    pub object_status_438: u8,
    /// `filter+0x0c` — forwarded raw dword.
    pub arg_0c: u32,
    /// `*(object + 0x258)` — the object's `AIUpdateInterface` slot; the body
    /// forms `ai + 0x1cc` unchecked, so a null slot gives the wild `0x1cc`.
    pub object_ai_258: i32,
}

/// `0x00741c54` — `PartitionFilter` `allow` over one candidate.
///
/// Rejects (in the body's order) when the candidate's `+0x438` bit0 is set,
/// when bit3 differs from the filter object's own `+0x438`, when
/// `FUN_0068d156` (`rowed Object::getRelationship`) is non-zero, or when the
/// `+0x25c` slot is non-null *and* `FUN_00790533` (`rowed` physics
/// flag-pair check) returns non-zero. The reject arm returns
/// `uVar2 & 0xffffff00` — `AL == 0` — so the observable result is `false`.
/// Otherwise the copied position is optionally logged (`desync_log_gate`
/// models `TheGameLogic + 0x1b4 > 0 && DAT_00dfeff0 != 0`; only `x`/`y` are
/// passed as the two doubles) and the truth value comes from `FUN_006f23a6`
/// (`rowed` pathfinder melee engagement-spot search) called with
/// `(object, filter+0x0c, ai + 0x1cc, &pos, candidate)`.
pub fn rva00341c54_filter_allow(
    filter: &Rva00341C54Filter,
    candidate: &AiFilterCandidate,
    desync_log_gate: bool,
    mut relationship: impl FnMut() -> u32, // 0x0068d156
    mut physics_pair: impl FnMut() -> i8,  // 0x00790533
    mut desync_log: impl FnMut(f64, f64),  // 0x006cec42 (+ s_FUCK_OFF_DESYNC…)
    mut engagement_spot: impl FnMut(i32, u32, i32, AiVec3, &AiFilterCandidate) -> u32, // 0x006f23a6
) -> bool {
    let b = candidate.status_438;
    // `uVar2 = CONCAT31((int3)(in_EAX >> 8), bVar1)` is an uninitialised-
    // register artifact; the reject arm masks the low byte off anyway.
    if (b & 1) != 0
        || ((filter.object_status_438 ^ b) & 8) != 0
        || relationship() != 0
        || (candidate.slot_25c != 0 && physics_pair() != 0)
    {
        return false;
    }
    if desync_log_gate {
        desync_log(f64::from(candidate.pos.x), f64::from(candidate.pos.y));
    }
    engagement_spot(
        filter.object,
        filter.arg_0c,
        filter.object_ai_258.wrapping_add(0x1cc),
        candidate.pos,
        candidate,
    ) != 0
}

// ---------------------------------------------------------------------------
// 0x0081f369 — ArmyDefinition weight-column rescale
// ---------------------------------------------------------------------------

/// One army-entry record as the body walks the `vector` slots
/// (`ArmyDefinition` `+0x04` begin / `+0x08` end).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ArmyWeightRecord {
    /// @ `+0x04` first weight column.
    pub w04: f32,
    /// @ `+0x08` second weight column.
    pub w08: f32,
    /// @ `+0x0c` third weight column.
    pub w0c: f32,
}

/// `0x0081f369` — normalise the three weight columns of every entry so each
/// column sums to `K`.
///
/// One pass sums `+0x08` into `s8`, `+0x04` into `s4` and `+0x0c` into `sc`
/// (the body's per-element order, each column accumulated in list order).
/// Then, only for columns whose sum is `> 0.0`, every element is multiplied by
/// `K / sum`: the `+0x04` scale is computed once from a *re-read* `K`
/// (`_DAT_00bc292c`), while the `+0x08`/`+0x0c` arms keep using the copy taken
/// at entry (`fVar2`) and re-form the quotient per element. `K` is an image
/// constant not recoverable from the decompilation, so it is a parameter (the
/// `scriptw47_1.rs` precedent). An empty vector (`begin == end`) does nothing.
pub fn army_definition_scale_weights(entries: &mut [ArmyWeightRecord], k: f32) {
    if entries.is_empty() {
        return;
    }
    let mut s4 = 0.0f32;
    let mut s8 = 0.0f32;
    let mut sc = 0.0f32;
    for e in entries.iter() {
        s8 = s8 + e.w08;
        s4 = e.w04 + s4;
        sc = e.w0c + sc;
    }
    if s4 > 0.0 {
        let scale = k / s4;
        for e in entries.iter_mut() {
            e.w04 = e.w04 * scale;
        }
    }
    if s8 > 0.0 {
        for e in entries.iter_mut() {
            e.w08 = e.w08 * (k / s8);
        }
    }
    if sc > 0.0 {
        for e in entries.iter_mut() {
            e.w0c = e.w0c * (k / sc);
        }
    }
}

// ---------------------------------------------------------------------------
// 0x0075487e — AIFaceState::update() -> StateReturnType
// ---------------------------------------------------------------------------

/// The face state as the body reads it (`State` base + the family's
/// `m_28`/`m_2C` pair).
#[derive(Clone, Copy, Debug, Default)]
pub struct AiFaceState {
    /// `machine -> m_owner(+0x14) -> m_ai(+0x258)`, resolved by the caller:
    /// the body loads that chain unconditionally at entry (a null machine or
    /// owner is the game's wild read).
    pub ai_258: i32,
    /// @ `+0x28` face mode: `0` needs no goal object, `2` takes the
    /// aim-interface arm and defers to `rowed AIIdleState::update`.
    pub mode_28: i32,
    /// @ `+0x2c` flag byte selecting which virtual turns the unit.
    pub flag_2c: u8,
}

/// `0x0075487e` — face-toward state update.
///
/// `mode_28 == 0` **or** a non-null goal object (`0x008d7726`, `rowed
/// getGoalObject`) enters the body; otherwise `STATE_FAILURE`. The body then
/// compares `fabs(relative angle) < (double)tol` — success (`STATE_SUCCESS`)
/// when the unit is already facing. Otherwise it turns: flag `+0x2c` clear →
/// the AI's vtable slot `0x210`; else `mode_28 == 2` and the `0x0069439d`
/// (`rowed` aim-interface getter) target is non-null → that target's slot
/// `0x210`; otherwise the AI's slot `0x21c`. Finally `mode_28 == 2` returns
/// `0x0075389e` (`rowed AIIdleState::update`), any other mode returns
/// `STATE_CONTINUE`.
///
/// The angle is the x87 `long double` result of `0x004b4542` (`rowed
/// Object::GetRelativeAngle`) — modelled as `f64`, the widening the body
/// performs before `fabs`; `NaN` fails the compare.
pub fn ai_face_state_update(
    st: &AiFaceState,
    tol: f32, // DAT_00c13b5c
    mut goal_object: impl FnMut() -> i32, // 0x008d7726
    mut relative_angle: impl FnMut() -> f64, // 0x004b4542
    mut ai_turn_to_goal: impl FnMut(i32), // ai vtable +0x210
    mut aim_target: impl FnMut() -> i32, // 0x0069439d
    mut aim_turn_to_goal: impl FnMut(i32), // aim vtable +0x210
    mut ai_turn_to_pos: impl FnMut(i32), // ai vtable +0x21c
    mut idle_state_update: impl FnMut() -> i32, // 0x0075389e
) -> i32 {
    if st.mode_28 != 0 && goal_object() == 0 {
        return STATE_FAILURE;
    }
    let angle = relative_angle().abs();
    if angle < f64::from(tol) {
        return STATE_SUCCESS;
    }
    if st.flag_2c == 0 {
        ai_turn_to_goal(st.ai_258);
    } else if st.mode_28 == 2 {
        let t = aim_target();
        if t != 0 {
            aim_turn_to_goal(t);
        } else {
            ai_turn_to_pos(st.ai_258);
        }
    } else {
        ai_turn_to_pos(st.ai_258);
    }
    if st.mode_28 == 2 {
        idle_state_update()
    } else {
        STATE_CONTINUE
    }
}

// ---------------------------------------------------------------------------
// 0x00763df9 — Path::PeekPastClimbPortal(Coord3D *out)
// ---------------------------------------------------------------------------

/// A path node as this row touches it: `+0x08` link, the `Coord3D` at
/// `+0x0c..+0x14`, and the raw `+0x20` dword compared against the
/// `0x7fffffff` sentinel. Nodes live in the game's pooled arena; the mirror
/// borrows them.
#[derive(Clone, Copy, Debug)]
pub struct PathNodeRef<'a> {
    /// `+0x08` next node.
    pub next: Option<&'a PathNodeRef<'a>>,
    /// `+0x0c..+0x14` position.
    pub pos: AiVec3,
    /// `+0x20` raw dword (`0x7fffffff` = "unset" sentinel).
    pub link_20: i32,
}

/// `0x00763df9` — `Path::PeekPastClimbPortal`.
///
/// Body order: `this+0x04` null → the out triple is zeroed (`x = y = 0`, then
/// `z` through the shared tail store). Otherwise `this+0x10` (the portal
/// node): null → the `+0x04` node's position. Otherwise walk `+0x08` links:
/// the portal's next node, then *its* next (overwriting the triple), then —
/// only when that third node exists and its `+0x20` differs from
/// `0x7fffffff` — the third node's position. A portal node with no successor
/// supplies its own position. `z` is always the last component stored
/// (the body's `LAB_00763eb7` tail). The mangled `PAUCoord3D` return is the
/// out pointer, handed back here.
pub fn path_peek_past_climb_portal<'a>(
    head_04: Option<&PathNodeRef<'_>>,
    portal_10: Option<&PathNodeRef<'_>>,
    out: &'a mut AiVec3,
) -> &'a mut AiVec3 {
    let head = match head_04 {
        None => {
            out.x = 0.0;
            out.y = 0.0;
            out.z = 0.0;
            return out;
        }
        Some(h) => h,
    };
    match portal_10 {
        None => {
            out.x = head.pos.x;
            out.y = head.pos.y;
            out.z = head.pos.z;
            out
        }
        Some(portal) => {
            let n1 = match portal.next {
                None => {
                    out.x = portal.pos.x;
                    out.y = portal.pos.y;
                    out.z = portal.pos.z;
                    return out;
                }
                Some(n) => n,
            };
            let mut pos = n1.pos;
            if let Some(n2) = n1.next {
                pos = n2.pos;
                if let Some(n3) = n2.next {
                    if n3.link_20 != 0x7fff_ffff {
                        pos = n3.pos;
                    }
                }
            }
            out.x = pos.x;
            out.y = pos.y;
            out.z = pos.z;
            out
        }
    }
}

// ---------------------------------------------------------------------------
// 0x008eacf5 — AI wall `(a-c)·(b-c)` open-interval test
// ---------------------------------------------------------------------------

/// `0x008eacf5` — `bool (const Coord2D *, const Coord2D *, const Coord2D *)`
/// from the AI wall builder.
///
/// `dot = (a.y-c.y)*(b.y-c.y) + (a.x-c.x)*(b.x-c.x)`; true only when
/// `0.0 < dot` **and** `dot < (b.x-c.x)² + (b.y-c.y)²` — the projection of
/// `a-c` on the `c→b` ray is strictly inside the `c..b` span (the `c`
/// endpoint, the `b` endpoint, right angles and every NaN case are false).
/// The mangled `YAM` return type makes the decompiler's two bare `return;`
/// statements the `true`/`false` arms.
pub fn ai_wall_projection_in_open_span(a: &AiVec2, b: &AiVec2, c: &AiVec2) -> bool {
    let dot = (a.y - c.y) * (b.y - c.y) + (a.x - c.x) * (b.x - c.x);
    if 0.0 < dot
        && dot < (b.x - c.x) * (b.x - c.x) + (b.y - c.y) * (b.y - c.y)
    {
        true
    } else {
        false
    }
}

// ---------------------------------------------------------------------------
// 0x008d8dea — TurretAI::updateTurretAI() -> UpdateSleepTime
// ---------------------------------------------------------------------------

/// The `TurretAI` fields this row reads and writes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TurretAiState {
    /// `m_enableSweepUntil @ +0x24` (written here, never read).
    pub enable_sweep_until_24: u32,
    /// `m_continuousFireExpirationFrame @ +0x30` (unsigned compare).
    pub continuous_fire_expiration_30: u32,
    /// `m_sleepUntil @ +0x34` — the wake frame; `0` means "not asleep".
    pub sleep_until_34: u32,
    /// `m_playRotSound @ +0x38`.
    pub play_rot_sound_38: u8,
    /// `m_playPitchSound @ +0x39`.
    pub play_pitch_sound_39: u8,
    /// `m_didFire @ +0x3b` (cleared before, re-set by the machine update).
    pub did_fire_3b: u8,
    /// `m_enabled @ +0x3c`.
    pub enabled_3c: u8,
    /// `m_firesWhileTurning @ +0x3d`.
    pub fires_while_turning_3d: u8,
}

/// `TurretAI::m_turretStateMachine @ +0x14` as a raw slot the hooks resolve;
/// a self-deleting machine clears `value` inside `state_machine_update`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TurretMachineSlot {
    pub value: i32,
}

/// `0x008d8dea` — the turret-AI update/tick.
///
/// Still-asleep gate: `sleep_until_34 != 0 && frame < sleep_until_34` returns
/// the unsigned frames remaining. Otherwise the sleep starts at
/// `UPDATE_SLEEP_FOREVER`; when `m_firesWhileTurning` is clear **or** the
/// continuous-fire expiry is `<= frame` (unsigned) both sound flags are
/// cleared. When `m_enabled` is clear, the machine's current state
/// (`slot → +0x04`, its `StateID` at `+0x04`) must be id `4` or the whole
/// firing block is skipped (`m_didFire` untouched) and the forever sleep is
/// armed. In the firing block: clear `m_didFire`, dispatch `+0x10`
/// (`updateStateMachine`), re-read the slot — the BFME-2 delta returns
/// `UPDATE_SLEEP_NONE` with **no** re-arm if the machine is gone — re-stamp
/// `+0x24`/`+0x30` with `frame + 3` when the update re-set `m_didFire`, run
/// `0x008d82f6` (`rowed stopRotOrPitchSound`) when both sound flags are clear
/// else `0x008d8b50` (`rowed startRotOrPitchSound`), then clamp the update's
/// sleep (`< 1 → 1`, `< 0x3fffffff → itself`, else keep forever) and store
/// `sleep_until_34 = sleep + frame` (wrapping).
///
/// A null machine slot on either dereference path is the game's wild
/// read/call at `0x4`/`0x10` → panic.
pub fn turret_ai_update_turret_ai(
    st: &mut TurretAiState,
    slot: &mut TurretMachineSlot,
    frame: u32, // TheGameLogic::m_frame (DAT_00dfe78c + 0x40)
    mut current_state_id: impl FnMut(i32) -> i32, // (slot → +0x04 state) → +0x04 id
    mut state_machine_update: impl FnMut(&mut TurretMachineSlot, &mut u8) -> i32, // vtable +0x10
    stop_rot_or_pitch: impl FnOnce(),  // 0x008d82f6
    start_rot_or_pitch: impl FnOnce(), // 0x008d8b50
) -> u32 {
    let sleep_until = st.sleep_until_34;
    if sleep_until != 0 && frame < sleep_until {
        return sleep_until - frame;
    }
    let mut sleep: i32 = 0x3fff_ffff;
    if st.fires_while_turning_3d == 0 || st.continuous_fire_expiration_30 <= frame {
        st.play_rot_sound_38 = 0;
        st.play_pitch_sound_39 = 0;
    }
    if st.enabled_3c == 0 {
        if slot.value == 0 {
            panic!("0x008d8dea: null m_turretStateMachine read at 0x4 (game UB)");
        }
        if current_state_id(slot.value) != 4 {
            st.sleep_until_34 = (sleep as u32).wrapping_add(frame);
            return sleep as u32;
        }
    }
    st.did_fire_3b = 0;
    if slot.value == 0 {
        panic!("0x008d8dea: null m_turretStateMachine dispatched at 0x10 (game UB)");
    }
    let upd = state_machine_update(slot, &mut st.did_fire_3b);
    if slot.value == 0 {
        return UPDATE_SLEEP_NONE;
    }
    if st.did_fire_3b != 0 {
        st.enable_sweep_until_24 = frame.wrapping_add(3);
        st.continuous_fire_expiration_30 = frame.wrapping_add(3);
    }
    if st.play_rot_sound_38 == 0 && st.play_pitch_sound_39 == 0 {
        stop_rot_or_pitch();
    } else {
        start_rot_or_pitch();
    }
    if upd < 1 {
        sleep = 1;
    } else if upd < 0x3fff_ffff {
        sleep = upd;
    }
    st.sleep_until_34 = (sleep as u32).wrapping_add(frame);
    sleep as u32
}

// ---------------------------------------------------------------------------
// 0x007705c2 — AIGroup::cohereAttackState()
// ---------------------------------------------------------------------------

/// One list member (`AIGroup::m_members @ +4`, the `ListNode` idiom; the
/// node's `+0x08` payload is an `Object *`) as the row loads it.
#[derive(Clone, Copy, Debug, Default)]
pub struct AiGroupMember {
    /// `Object::m_contain @ +0x274` (`0` = member is not contained).
    pub contain_274: i32,
    /// `Object::m_ai @ +0x258` (`0` = no AI update interface).
    pub ai_258: i32,
}

/// The victim `Object`, seen through its `m_pos @ +0x38` (the body passes the
/// raw address `victim + 0x38` to the position command).
#[derive(Clone, Copy, Debug, Default)]
pub struct AiVictim {
    pub position: AiVec3,
}

/// `0x007705c2` — cohere the group's attack state.
///
/// Phase 1 walks the member list (in list order) and keeps the first member
/// that is uncontained (`+0x274 == 0`), has an AI (`+0x258 != 0`) and whose
/// `0x00668d71` (`rowed AIUpdateInterface::getCurrentVictim`) is non-null —
/// the quirk is that the getter is called **twice**: the `if` tests the first
/// result and the assignment stores the second, so a side-effecting getter
/// that clears the victim leaves the search running. Found → the walk stops
/// (the body's `goto` lands on phase 2 through the loop's re-entry test).
///
/// Phase 2 restarts from the list head and, for every uncontained member that
/// passes `0x00690b73` (`rowed Object::isAbleToAttack`), has an AI, has *no*
/// current victim and passes the AI's vtable slot `0x1b8` (`rowed
/// AIUpdateInterface::isIdle`), runs the `0x00695a0f` position command
/// (`rowed` position-command: victim position, `0x7fffffff` max shots,
/// `CMD_FROM_AI == 2`).
pub fn ai_group_cohere_attack_state<'a>(
    members: &[AiGroupMember],
    mut current_victim: impl FnMut(&AiGroupMember) -> Option<&'a AiVictim>, // 0x00668d71
    mut able_to_attack: impl FnMut(&AiGroupMember) -> i8, // 0x00690b73
    mut ai_is_idle: impl FnMut(i32) -> i8, // ai vtable +0x1b8
    mut attack_position: impl FnMut(AiVec3, i32, i32), // 0x00695a0f
) {
    let mut found: Option<&'a AiVictim> = None;
    for m in members.iter() {
        if found.is_some() {
            break;
        }
        if m.contain_274 == 0
            && m.ai_258 != 0
            && current_victim(m).is_some()
        {
            found = current_victim(m);
        }
    }
    let found = match found {
        Some(v) => v,
        None => return,
    };
    for m in members.iter() {
        if m.contain_274 == 0
            && able_to_attack(m) != 0
            && m.ai_258 != 0
            && current_victim(m).is_none()
            && ai_is_idle(m.ai_258) != 0
        {
            attack_position(found.position, 0x7fff_ffff, 2);
        }
    }
}

// ---------------------------------------------------------------------------
// 0x0074afcd — AIAttackPositionFireWeaponState::update() -> StateReturnType
// ---------------------------------------------------------------------------

/// The state's own slots plus the machine owner's status byte.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AiAttackPositionFireState {
    /// `machine(+0x18) → m_owner(+0x14) → Object::m_privateStatus @ +0x438`
    /// (bit0 = effectively dead), resolved by the caller.
    pub owner_status_438: u8,
    /// @ `+0x24` "already firing" flag (cleared on the pre-fire arm).
    pub firing_24: u8,
    /// @ `+0x20` raw pointer slot (null-checked before any dispatch).
    pub holder_20: i32,
}

/// The row's callees; `WeaponStatus` is the `rowed` spelling
/// (`READY_TO_FIRE = 0, OUT_OF_AMMO, BETWEEN_FIRING_SHOTS, RELOADING_CLIP,
/// PRE_ATTACK = 4`).
pub trait FireWeaponStateHost {
    /// `0x0068aebd` `rowed Object::getCurrentWeapon(WeaponSlotType *slot)` on
    /// the owner; the body passes the address of the stack slot holding
    /// `this` (hence the state view). `0` = no current weapon.
    fn current_weapon(&mut self, state: &AiAttackPositionFireState) -> i32;
    /// `0x0044e536` `rowed Object::testStatus(0x52)` on the owner.
    fn test_status_52(&mut self) -> bool;
    /// `0x006ccfc7` `rowed Weapon::getStatus()`.
    fn weapon_status(&mut self) -> i32;
    /// `0x0068fc8f` `rowed Object::setFiringConditionForCurrentWeapon()`.
    fn set_firing_condition(&mut self);
    /// `0x00691916` `rowed Object::fireCurrentWeapon(const Coord3D *)`, the
    /// argument being the machine's `+0x24` position address.
    fn fire_current_weapon(&mut self, goal_pos_addr: i32);
    /// `0x0063db0e` `rowed Object::setStatus(status, value)`.
    fn set_status(&mut self, status: i32, value: i32);
    /// vtable slot `0x08` of the object behind `holder_20`, arg `this`.
    fn holder_slot_08(&mut self, state: &AiAttackPositionFireState) -> bool;
    /// vtable slot `0x00` of that same object.
    fn holder_slot_00(&mut self);
    /// `0x008d7726` `rowed getGoalObject`.
    fn goal_object(&mut self) -> i32;
    /// `0x006919a7` `rowed Object::preFireCurrentWeapon(const Object *,
    /// const Coord3D *)`.
    fn pre_fire_current_weapon(&mut self, goal: i32, goal_pos_addr: i32);
}

/// `0x0074afcd` — the attack-position fire-weapon state update.
///
/// No current weapon, or the owner's `+0x438` bit0 set, or
/// `testStatus(0x52)` → `STATE_FAILURE` (in that order, short-circuited).
/// With the `+0x24` firing flag clear: a `PRE_ATTACK` (4) weapon status falls
/// through to `STATE_CONTINUE`; any status other than `READY_TO_FIRE` (0) is
/// `STATE_FAILURE`; at `READY_TO_FIRE` a null `+0x20` holder **or** the
/// holder's slot-`0x08` virtual returning true fires
/// (`setFiringConditionForCurrentWeapon`, `fireCurrentWeapon(machine + 0x24)`,
/// `setStatus(0x1b, 0)`, holder slot `0x00`) and returns `STATE_SUCCESS` —
/// otherwise `STATE_FAILURE`. With the flag set the body clears it,
/// `setStatus(0xd, 1)` and calls `preFireCurrentWeapon(goalObject,
/// machine + 0x24)`, then returns `STATE_CONTINUE`. The body overwrites its
/// owner handle with the status (`iVar4` reuse) without re-reading it.
pub fn ai_attack_position_fire_weapon_state_update(
    st: &mut AiAttackPositionFireState,
    machine: i32, // State::m_machine @ +0x18 (+0x24 = the goal Coord3D)
    host: &mut dyn FireWeaponStateHost,
) -> i32 {
    if host.current_weapon(st) == 0
        || (st.owner_status_438 & 1) != 0
        || host.test_status_52()
    {
        return STATE_FAILURE;
    }
    if st.firing_24 == 0 {
        let status = host.weapon_status();
        if status != 4 {
            if status != 0 {
                return STATE_FAILURE;
            }
            if st.holder_20 == 0 || host.holder_slot_08(st) {
                host.set_firing_condition();
                host.fire_current_weapon(machine.wrapping_add(0x24));
                host.set_status(0x1b, 0);
                host.holder_slot_00();
                return STATE_SUCCESS;
            }
            return STATE_FAILURE;
        }
    } else {
        st.firing_24 = 0;
        host.set_status(0xd, 1);
        let goal = host.goal_object();
        host.pre_fire_current_weapon(goal, machine.wrapping_add(0x24));
    }
    STATE_CONTINUE
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::cell::{Cell, RefCell};

    fn v3(x: f32, y: f32, z: f32) -> AiVec3 {
        AiVec3 { x, y, z }
    }

    fn v2(x: f32, y: f32) -> AiVec2 {
        AiVec2 { x, y }
    }

    // -- 0x00741c54 ----------------------------------------------------------

    fn filter() -> Rva00341C54Filter {
        Rva00341C54Filter {
            object: 0x1234,
            object_status_438: 0,
            arg_0c: 9,
            object_ai_258: 0x2580,
        }
    }

    #[test]
    fn filter_rejects_effectively_dead_without_any_callee() {
        let f = filter();
        let c = AiFilterCandidate {
            status_438: 1,
            slot_25c: 7,
            pos: v3(1.0, 2.0, 3.0),
        };
        let n = Cell::new(0i32);
        let r = rva00341c54_filter_allow(
            &f,
            &c,
            true,
            || {
                n.set(n.get() + 1);
                3
            },
            || {
                n.set(n.get() + 1);
                1
            },
            |_, _| n.set(n.get() + 10),
            |_, _, _, _, _| {
                n.set(n.get() + 100);
                1
            },
        );
        assert!(!r);
        assert_eq!(n.get(), 0, "the byte tests short-circuit every callee");
    }

    #[test]
    fn filter_rejects_on_differing_bit3_only() {
        let f = filter();
        let c = AiFilterCandidate {
            status_438: 8,
            ..Default::default()
        };
        let n = Cell::new(0i32);
        assert!(!rva00341c54_filter_allow(
            &f,
            &c,
            false,
            || {
                n.set(n.get() + 1);
                0
            },
            || {
                n.set(n.get() + 1);
                1
            },
            |_, _| n.set(n.get() + 10),
            |_, _, _, _, _| {
                n.set(n.get() + 100);
                1
            }
        ));
        assert_eq!(n.get(), 0);
    }

    #[test]
    fn filter_rejects_on_relationship_and_skips_physics_when_slot_clear() {
        let f = filter();
        let c = AiFilterCandidate {
            status_438: 2,
            slot_25c: 0,
            ..Default::default()
        };
        let mut rel = 0;
        let mut phys = 0;
        assert!(!rva00341c54_filter_allow(
            &f,
            &c,
            false,
            || {
                rel += 1;
                0xffff_ffff
            },
            || {
                phys += 1;
                1
            },
            |_, _| {},
            |_, _, _, _, _| {
                unreachable!();
            }
        ));
        assert_eq!(rel, 1);
        assert_eq!(phys, 0, "+0x25c == 0 blocks the flag-pair check");
    }

    #[test]
    fn filter_rejects_on_physics_pair_only_when_slot_set() {
        let f = filter();
        let c = AiFilterCandidate {
            status_438: 0,
            slot_25c: -1,
            ..Default::default()
        };
        let mut phys = 0;
        assert!(!rva00341c54_filter_allow(
            &f,
            &c,
            false,
            || 0,
            || {
                phys += 1;
                -1
            },
            |_, _| {},
            |_, _, _, _, _| {
                unreachable!();
            }
        ));
        assert_eq!(phys, 1);
    }

    #[test]
    fn filter_accept_forwards_ai_plus_0x1cc_and_logs_only_xy() {
        let f = filter();
        let c = AiFilterCandidate {
            status_438: 0x10, // bit4 set: bit0 clear, bit3 clear
            slot_25c: 0x900,
            pos: v3(-4.5, 6.25, f32::NAN),
        };
        let mut logged = Vec::new();
        let mut got: Option<(i32, u32, i32, AiVec3)> = None;
        let r = rva00341c54_filter_allow(
            &f,
            &c,
            true,
            || 0,
            || 0,
            |x, y| logged.push((x, y)),
            |obj, arg0c, ai_seed, pos, cand| {
                assert_eq!(cand.status_438, 0x10);
                got = Some((obj, arg0c, ai_seed, pos));
                0x100
            },
        );
        assert!(r);
        let g = got.expect("the engagement search ran");
        assert_eq!((g.0, g.1, g.2), (0x1234, 9, 0x2580 + 0x1cc));
        assert_eq!((g.3.x, g.3.y), (-4.5, 6.25));
        assert!(g.3.z.is_nan(), "the copied z dword rides along");
        assert_eq!(logged, vec![(-4.5f64, 6.25f64)]);
    }

    #[test]
    fn filter_log_gate_off_and_null_ai_seed_wraps() {
        let f = Rva00341C54Filter {
            object: 0,
            object_status_438: 0,
            arg_0c: 0,
            object_ai_258: 0,
        };
        let c = AiFilterCandidate::default();
        let mut logged = 0;
        let mut seed = -1;
        let r = rva00341c54_filter_allow(
            &f,
            &c,
            false,
            || 0,
            || 0,
            |_, _| logged += 1,
            |_, _, ai_seed, _, _| {
                seed = ai_seed;
                0
            },
        );
        assert!(!r);
        assert_eq!(logged, 0);
        // the body never null-checks the AI slot: 0 + 0x1cc is the wild seed
        assert_eq!(seed, 0x1cc);
    }

    // -- 0x0081f369 ---------------------------------------------------------

    #[test]
    fn army_scales_each_column_to_k() {
        let mut e = vec![
            ArmyWeightRecord {
                w04: 1.0,
                w08: 2.0,
                w0c: 4.0,
            },
            ArmyWeightRecord {
                w04: 3.0,
                w08: 2.0,
                w0c: 0.0,
            },
        ];
        army_definition_scale_weights(&mut e, 100.0);
        assert_eq!(
            e[0],
            ArmyWeightRecord {
                w04: 25.0,
                w08: 50.0,
                w0c: 100.0
            }
        );
        assert_eq!(
            e[1],
            ArmyWeightRecord {
                w04: 75.0,
                w08: 50.0,
                w0c: 0.0
            }
        );
    }

    #[test]
    fn army_skips_columns_that_are_not_positive() {
        // column 1 sums negative, column 2 sums to 0.0, column 3 sums NaN:
        // nothing scales, and no divide-by-zero happens.
        let mut e = vec![
            ArmyWeightRecord {
                w04: 5.0,
                w08: 3.0,
                w0c: f32::NAN,
            },
            ArmyWeightRecord {
                w04: -6.0,
                w08: -3.0,
                w0c: 1.0,
            },
        ];
        army_definition_scale_weights(&mut e, 100.0);
        assert_eq!(
            (e[0].w04, e[1].w04, e[0].w08, e[1].w08),
            (5.0, -6.0, 3.0, -3.0)
        );
        assert_eq!(e[1].w0c, 1.0);
        assert!(e[0].w0c.is_nan(), "NaN sum leaves the column alone");
    }

    #[test]
    fn army_empty_and_single_entry() {
        let mut empty: Vec<ArmyWeightRecord> = Vec::new();
        army_definition_scale_weights(&mut empty, 100.0);
        let mut one = vec![ArmyWeightRecord {
            w04: 2.0,
            w08: 8.0,
            w0c: 0.5,
        }];
        army_definition_scale_weights(&mut one, 7.0);
        assert_eq!(
            one[0],
            ArmyWeightRecord {
                w04: 7.0,
                w08: 7.0,
                w0c: 7.0,
            }
        );
    }

    // -- 0x0075487e ---------------------------------------------------------

    #[test]
    fn face_state_success_on_small_angle_without_turning() {
        let st = AiFaceState {
            ai_258: 0x2580,
            mode_28: 2,
            flag_2c: 1,
        };
        let turns = Cell::new(0i32);
        let idle = Cell::new(0i32);
        assert_eq!(
            ai_face_state_update(
                &st,
                0.5,
                || 0x111,
                || -0.4999,
                |_| turns.set(turns.get() + 1),
                || {
                    turns.set(turns.get() + 1);
                    0x222
                },
                |_| turns.set(turns.get() + 1),
                |_| turns.set(turns.get() + 1),
                || {
                    idle.set(idle.get() + 1);
                    STATE_CONTINUE
                },
            ),
            STATE_SUCCESS
        );
        assert_eq!((turns.get(), idle.get()), (0, 0));
    }

    #[test]
    fn face_state_failure_needs_goal_object_only_outside_mode_zero() {
        let st = AiFaceState {
            ai_258: 0,
            mode_28: 1,
            flag_2c: 0,
        };
        let mut angle = 0;
        assert_eq!(
            ai_face_state_update(
                &st,
                0.5,
                || 0,
                || {
                    angle += 1;
                    9.0
                },
                |_| {},
                || 0,
                |_| {},
                |_| {},
                || STATE_CONTINUE,
            ),
            STATE_FAILURE
        );
        assert_eq!(angle, 0, "the angle is never taken without a goal");
        // mode 0 skips the goal lookup entirely (short-circuit)
        let st0 = AiFaceState {
            ai_258: 0,
            mode_28: 0,
            flag_2c: 0,
        };
        assert_eq!(
            ai_face_state_update(
                &st0,
                0.5,
                || {
                    unreachable!();
                },
                || 0.0,
                |_| {},
                || 0,
                |_| {},
                |_| {},
                || STATE_CONTINUE,
            ),
            STATE_SUCCESS
        );
    }

    #[test]
    fn face_state_turn_dispatch_and_idle_tail() {
        // flag clear -> the AI's slot 0x210; mode 2 defers to idle update
        let st = AiFaceState {
            ai_258: 0x2580,
            mode_28: 2,
            flag_2c: 0,
        };
        let seq: RefCell<Vec<&str>> = RefCell::new(Vec::new());
        let r = ai_face_state_update(
            &st,
            0.5,
            || 0x111,
            || 1.0,
            |_| seq.borrow_mut().push("ai210"),
            || {
                seq.borrow_mut().push("aim_get");
                0x222
            },
            |_| seq.borrow_mut().push("aim210"),
            |_| seq.borrow_mut().push("ai21c"),
            || {
                seq.borrow_mut().push("idle");
                7
            },
        );
        assert_eq!(r, 7);
        assert_eq!(*seq.borrow(), vec!["ai210", "idle"]);

        // flag set, mode 2, null aim target -> falls back to slot 0x21c
        let st2 = AiFaceState {
            ai_258: 0x2580,
            mode_28: 2,
            flag_2c: 1,
        };
        let seq2: RefCell<Vec<&str>> = RefCell::new(Vec::new());
        assert_eq!(
            ai_face_state_update(
                &st2,
                0.5,
                || 0x111,
                || f64::NAN,
                |_| seq2.borrow_mut().push("ai210"),
                || {
                    seq2.borrow_mut().push("aim_get");
                    0
                },
                |_| seq2.borrow_mut().push("aim210"),
                |_| seq2.borrow_mut().push("ai21c"),
                || {
                    seq2.borrow_mut().push("idle");
                    STATE_CONTINUE
                },
            ),
            STATE_CONTINUE
        );
        assert_eq!(*seq2.borrow(), vec!["aim_get", "ai21c", "idle"]);

        // flag set, mode 3: the aim getter is never reached (&& short-circuit)
        let st3 = AiFaceState {
            ai_258: 0x2580,
            mode_28: 3,
            flag_2c: 1,
        };
        let seq3: RefCell<Vec<&str>> = RefCell::new(Vec::new());
        assert_eq!(
            ai_face_state_update(
                &st3,
                0.5,
                || 0x111,
                || 0.5,
                |_| seq3.borrow_mut().push("ai210"),
                || {
                    seq3.borrow_mut().push("aim_get");
                    0x222
                },
                |_| seq3.borrow_mut().push("aim210"),
                |_| seq3.borrow_mut().push("ai21c"),
                || {
                    seq3.borrow_mut().push("idle");
                    1
                },
            ),
            STATE_CONTINUE
        );
        assert_eq!(*seq3.borrow(), vec!["ai21c"]);
    }

    #[test]
    fn face_state_tolerance_boundary_is_strict_and_nan_turns() {
        let st = AiFaceState {
            ai_258: 1,
            mode_28: 0,
            flag_2c: 0,
        };
        // |angle| == tol is *not* below tol -> turning branch
        let mut turned = 0;
        assert_eq!(
            ai_face_state_update(
                &st,
                0.5,
                || 0,
                || 0.5f64,
                |_| turned += 1,
                || 0,
                |_| {},
                |_| {},
                || STATE_CONTINUE,
            ),
            STATE_CONTINUE
        );
        assert_eq!(turned, 1);
        // NaN fabs never satisfies `<`
        assert_eq!(
            ai_face_state_update(
                &st,
                0.5,
                || 0,
                || f64::NAN,
                |_| turned += 1,
                || 0,
                |_| {},
                |_| {},
                || STATE_CONTINUE,
            ),
            STATE_CONTINUE
        );
        assert_eq!(turned, 2);
    }

    // -- 0x00763df9 ---------------------------------------------------------

    #[test]
    fn peek_null_head_zeroes_out() {
        let mut out = v3(9.0, 9.0, 9.0);
        let r = path_peek_past_climb_portal(None, None, &mut out);
        assert_eq!(*r, v3(0.0, 0.0, 0.0));
        r.z = 7.0;
        assert_eq!(out.z, 7.0, "the mangled return is the out pointer");
    }

    #[test]
    fn peek_without_portal_uses_head_position() {
        let head = PathNodeRef {
            next: None,
            pos: v3(1.5, 2.5, 3.5),
            link_20: 0x7fff_ffff,
        };
        let mut out = AiVec3::default();
        path_peek_past_climb_portal(Some(&head), None, &mut out);
        assert_eq!(out, v3(1.5, 2.5, 3.5));
    }

    #[test]
    fn peek_portal_chain_depths_and_sentinel() {
        let head = PathNodeRef {
            next: None,
            pos: v3(0.0, 0.0, 0.0),
            link_20: 0,
        };
        // portal with no successor: its own position
        let p0 = PathNodeRef {
            next: None,
            pos: v3(10.0, 11.0, 12.0),
            link_20: 0,
        };
        let mut out = AiVec3::default();
        path_peek_past_climb_portal(Some(&head), Some(&p0), &mut out);
        assert_eq!(out, v3(10.0, 11.0, 12.0));

        // one node past the portal
        let n1a = PathNodeRef {
            next: None,
            pos: v3(20.0, 21.0, 22.0),
            link_20: 0,
        };
        let pa = PathNodeRef {
            next: Some(&n1a),
            pos: v3(10.0, 11.0, 12.0),
            link_20: 0,
        };
        path_peek_past_climb_portal(Some(&head), Some(&pa), &mut out);
        assert_eq!(out, v3(20.0, 21.0, 22.0));

        // two nodes past the portal
        let n2b = PathNodeRef {
            next: None,
            pos: v3(30.0, 31.0, 32.0),
            link_20: 0,
        };
        let n1b = PathNodeRef {
            next: Some(&n2b),
            pos: v3(20.0, 21.0, 22.0),
            link_20: 0,
        };
        let pb = PathNodeRef {
            next: Some(&n1b),
            pos: v3(10.0, 11.0, 12.0),
            link_20: 0,
        };
        path_peek_past_climb_portal(Some(&head), Some(&pb), &mut out);
        assert_eq!(out, v3(30.0, 31.0, 32.0));

        // three nodes past the portal (its +0x20 is not the sentinel)
        let n3c = PathNodeRef {
            next: None,
            pos: v3(40.0, 41.0, 42.0),
            link_20: 0x7ffffffe,
        };
        let n2c = PathNodeRef {
            next: Some(&n3c),
            pos: v3(30.0, 31.0, 32.0),
            link_20: 0,
        };
        let n1c = PathNodeRef {
            next: Some(&n2c),
            pos: v3(20.0, 21.0, 22.0),
            link_20: 0,
        };
        let pc = PathNodeRef {
            next: Some(&n1c),
            pos: v3(10.0, 11.0, 12.0),
            link_20: 0,
        };
        path_peek_past_climb_portal(Some(&head), Some(&pc), &mut out);
        assert_eq!(out, v3(40.0, 41.0, 42.0));

        // the sentinel suppresses the third node: the second one wins
        let n3d = PathNodeRef {
            next: None,
            pos: v3(40.0, 41.0, 42.0),
            link_20: 0x7fff_ffff,
        };
        let n2d = PathNodeRef {
            next: Some(&n3d),
            pos: v3(33.0, 34.0, 35.0),
            link_20: 0,
        };
        let n1d = PathNodeRef {
            next: Some(&n2d),
            pos: v3(20.0, 21.0, 22.0),
            link_20: 0,
        };
        let pd = PathNodeRef {
            next: Some(&n1d),
            pos: v3(10.0, 11.0, 12.0),
            link_20: 0,
        };
        path_peek_past_climb_portal(Some(&head), Some(&pd), &mut out);
        assert_eq!(out, v3(33.0, 34.0, 35.0));
    }

    // -- 0x008eacf5 ---------------------------------------------------------

    #[test]
    fn wall_projection_open_interval() {
        let c = v2(0.0, 0.0);
        let b = v2(10.0, 0.0);
        // strictly inside the c..b span on the ray
        assert!(ai_wall_projection_in_open_span(&v2(4.0, 3.0), &b, &c));
        // a == b: dot == |b-c|^2 -> strict `<` fails
        assert!(!ai_wall_projection_in_open_span(&b, &b, &c));
        // behind c (obtuse) -> dot <= 0
        assert!(!ai_wall_projection_in_open_span(&v2(-1.0, 0.0), &b, &c));
        // exactly perpendicular -> dot == 0
        assert!(!ai_wall_projection_in_open_span(&v2(0.0, 5.0), &b, &c));
        // beyond b -> dot > |b-c|^2
        assert!(!ai_wall_projection_in_open_span(&v2(11.0, 0.0), &b, &c));
        // degenerate segment b == c -> |b-c|^2 == 0 and dot == 0
        assert!(!ai_wall_projection_in_open_span(&v2(3.0, 3.0), &c, &c));
        // NaN in either comparison -> false
        assert!(!ai_wall_projection_in_open_span(
            &v2(f32::NAN, 0.0),
            &b,
            &c
        ));
        assert!(!ai_wall_projection_in_open_span(
            &v2(4.0, 0.0),
            &v2(f32::NAN, 0.0),
            &c
        ));
        // the y-term is folded first; a pure-y case still passes
        assert!(ai_wall_projection_in_open_span(&v2(1.0, 6.0), &v2(0.0, 10.0), &c));
    }

    // -- 0x008d8dea ---------------------------------------------------------

    #[test]
    fn turret_sleep_gate_short_circuits_everything() {
        let mut st = TurretAiState {
            sleep_until_34: 10,
            ..Default::default()
        };
        let mut slot = TurretMachineSlot { value: 0x500 };
        let got = turret_ai_update_turret_ai(
            &mut st,
            &mut slot,
            4,
            |_| unreachable!(),
            |_, _| unreachable!(),
            || unreachable!(),
            || unreachable!(),
        );
        assert_eq!(got, 6);
        assert_eq!(st.sleep_until_34, 10, "the wake frame is left alone");
        // sleep_until == frame is not "asleep" (strict `<`)
        let got2 = turret_ai_update_turret_ai(
            &mut st,
            &mut slot,
            10,
            |_| 4,
            |_, _| 0,
            || {},
            || {},
        );
        assert_eq!(got2, 1);
        assert_eq!(st.sleep_until_34, 11);
    }

    #[test]
    fn turret_state_id_gate_skips_the_firing_block() {
        let mut st = TurretAiState {
            enabled_3c: 0,
            did_fire_3b: 1,
            fires_while_turning_3d: 0,
            play_rot_sound_38: 1,
            play_pitch_sound_39: 1,
            ..Default::default()
        };
        let mut slot = TurretMachineSlot { value: 0x500 };
        let mut ids = 0;
        let mut dispatches = 0;
        let got = turret_ai_update_turret_ai(
            &mut st,
            &mut slot,
            0x1000,
            |s| {
                ids += 1;
                assert_eq!(s, 0x500);
                3
            },
            |_, _| {
                dispatches += 1;
                0
            },
            || unreachable!(),
            || unreachable!(),
        );
        assert_eq!(got, UPDATE_SLEEP_FOREVER);
        assert_eq!(ids, 1);
        assert_eq!(dispatches, 0);
        assert_eq!(st.did_fire_3b, 1, "m_didFire survives the skipped block");
        assert_eq!(st.sleep_until_34, 0x1000u32.wrapping_add(0x3fff_ffff));
        // the sound flags were cleared earlier (firesWhileTurning clear)
        assert_eq!((st.play_rot_sound_38, st.play_pitch_sound_39), (0, 0));
    }

    #[test]
    fn turret_firing_block_clamps_sleep_and_rearms() {
        let mut st = TurretAiState {
            enabled_3c: 1,
            fires_while_turning_3d: 1,
            continuous_fire_expiration_30: 200,
            play_rot_sound_38: 1,
            play_pitch_sound_39: 1,
            ..Default::default()
        };
        let mut slot = TurretMachineSlot { value: 0x500 };
        let mut starts = 0;
        let mut stops = 0;
        let got = turret_ai_update_turret_ai(
            &mut st,
            &mut slot,
            100,
            |_| unreachable!(), // m_enabled set: the state id is never read
            |s, did_fire| {
                assert_eq!(s.value, 0x500);
                *did_fire = 1; // the machine reports a shot
                7
            },
            || stops += 1,
            || starts += 1,
        );
        assert_eq!(got, 7);
        assert_eq!((stops, starts), (0, 1), "both sound flags still set");
        assert_eq!(st.enable_sweep_until_24, 103);
        assert_eq!(st.continuous_fire_expiration_30, 103);
        assert_eq!(st.sleep_until_34, 107);

        // clamps: <= 0 becomes 1; >= UPDATE_SLEEP_FOREVER stays forever
        let mut st2 = TurretAiState::default();
        let mut slot2 = TurretMachineSlot { value: 0x500 };
        assert_eq!(
            turret_ai_update_turret_ai(
                &mut st2,
                &mut slot2,
                20,
                |_| 4,
                |_, _| -5,
                || {},
                || {},
            ),
            UPDATE_SLEEP_NONE
        );
        assert_eq!(st2.sleep_until_34, 21);
        assert_eq!(
            turret_ai_update_turret_ai(
                &mut st2,
                &mut slot2,
                21,
                |_| 4,
                |_, _| 0x3fff_ffff,
                || {},
                || {},
            ),
            UPDATE_SLEEP_FOREVER
        );
        assert_eq!(st2.sleep_until_34, 21 + 0x3fff_ffff);
    }

    #[test]
    fn turret_deleted_machine_returns_none_without_rearming() {
        let mut st = TurretAiState {
            enabled_3c: 1,
            sleep_until_34: 0,
            ..Default::default()
        };
        let mut slot = TurretMachineSlot { value: 0x500 };
        let routines = Cell::new(0i32);
        let got = turret_ai_update_turret_ai(
            &mut st,
            &mut slot,
            50,
            |_| unreachable!(),
            |s, _| {
                s.value = 0; // the state machine deletes itself
                1
            },
            || routines.set(routines.get() + 1),
            || routines.set(routines.get() + 1),
        );
        assert_eq!(got, UPDATE_SLEEP_NONE);
        assert_eq!(
            routines.get(),
            0,
            "the BFME-2 early return skips both sound arms"
        );
        assert_eq!(st.sleep_until_34, 0, "no re-arm");
    }

    #[test]
    #[should_panic(expected = "null m_turretStateMachine read at 0x4")]
    fn turret_null_machine_slot_state_id_is_a_wild_read() {
        let mut st = TurretAiState::default();
        let mut slot = TurretMachineSlot { value: 0 };
        turret_ai_update_turret_ai(
            &mut st,
            &mut slot,
            1,
            |_| 4,
            |_, _| 1,
            || {},
            || {},
        );
    }

    // -- 0x007705c2 ---------------------------------------------------------

    #[test]
    fn group_cohere_orders_idle_members_to_the_found_victim() {
        let members = vec![
            AiGroupMember {
                contain_274: 9,
                ai_258: 0xB0,
            },
            AiGroupMember {
                contain_274: 0,
                ai_258: 0xB1,
            },
            AiGroupMember {
                contain_274: 0,
                ai_258: 0xB2,
            },
            AiGroupMember {
                contain_274: 0,
                ai_258: 0xB3,
            },
            AiGroupMember {
                contain_274: 0,
                ai_258: 0xB4,
            },
        ];
        let victim = AiVictim {
            position: v3(1.0, 2.0, 3.0),
        };
        let mut victims = 0;
        let mut able = 0;
        let mut idle = 0;
        let mut cmds: Vec<(AiVec3, i32, i32)> = Vec::new();
        ai_group_cohere_attack_state(
            &members,
            |m| {
                victims += 1;
                if m.ai_258 == 0xB1 {
                    Some(&victim)
                } else {
                    None
                }
            },
            |m| {
                able += 1;
                if m.ai_258 == 0xB4 {
                    0
                } else {
                    1
                }
            },
            |ai| {
                idle += 1;
                if ai == 0xB3 {
                    0
                } else {
                    1
                }
            },
            |p, shots, src| cmds.push((p, shots, src)),
        );
        // phase 1: m1 pays two getCurrentVictim calls, then the walk stops
        // (m0 is contained, so phase 1 sees only m1);
        // phase 2 restarts: m1 again (has a victim), m2 and m3 (no victim),
        // m4 is blocked by isAbleToAttack before the victim lookup.
        assert_eq!(victims, 5);
        assert_eq!(able, 4);
        assert_eq!(idle, 2);
        assert_eq!(cmds, vec![(v3(1.0, 2.0, 3.0), 0x7fff_ffff, 2)]);
    }

    #[test]
    fn group_cohere_second_victim_call_can_clear_the_find() {
        let members = vec![
            AiGroupMember {
                contain_274: 0,
                ai_258: 0xB1,
            },
            AiGroupMember {
                contain_274: 0,
                ai_258: 0xB2,
            },
        ];
        let v_a = AiVictim {
            position: v3(1.0, 1.0, 1.0),
        };
        let v_b = AiVictim {
            position: v3(2.0, 2.0, 2.0),
        };
        let mut calls = 0;
        let mut cmds: Vec<(AiVec3, i32, i32)> = Vec::new();
        ai_group_cohere_attack_state(
            &members,
            |m| {
                calls += 1;
                match (m.ai_258, calls) {
                    (0xB1, 1) => Some(&v_a), // the `if` sees a victim …
                    (0xB1, _) => None,       // … the assignment stores none
                    _ => Some(&v_b),
                }
            },
            |_| 1,
            |_| 1,
            |p, shots, src| cmds.push((p, shots, src)),
        );
        // 1+2 (m1's pair), 3+4 (m2's pair, kept), phase 2: m1 none -> ordered,
        // m2 has a victim -> not ordered
        assert_eq!(calls, 6);
        assert_eq!(cmds, vec![(v3(2.0, 2.0, 2.0), 0x7fff_ffff, 2)]);
    }

    #[test]
    fn group_cohere_without_any_victim_never_reaches_phase_two() {
        let members = vec![AiGroupMember {
            contain_274: 0,
            ai_258: 0xB1,
        }];
        let mut calls = 0;
        ai_group_cohere_attack_state(
            &members,
            |_| {
                calls += 1;
                None
            },
            |_| unreachable!(),
            |_| unreachable!(),
            |_, _, _| unreachable!(),
        );
        assert_eq!(calls, 1, "the second call needs a non-null first result");
    }

    // -- 0x0074afcd ---------------------------------------------------------

    #[derive(Default)]
    struct FireHost {
        seq: Vec<&'static str>,
        weapon: i32,
        status_52: bool,
        wstatus: i32,
        slot08: bool,
        goal: i32,
        statuses: Vec<(i32, i32)>,
        fired: Vec<i32>,
        pre: Vec<(i32, i32)>,
    }

    impl FireWeaponStateHost for FireHost {
        fn current_weapon(&mut self, _st: &AiAttackPositionFireState) -> i32 {
            self.seq.push("weapon");
            self.weapon
        }
        fn test_status_52(&mut self) -> bool {
            self.seq.push("status52");
            self.status_52
        }
        fn weapon_status(&mut self) -> i32 {
            self.seq.push("wstatus");
            self.wstatus
        }
        fn set_firing_condition(&mut self) {
            self.seq.push("cond");
        }
        fn fire_current_weapon(&mut self, goal_pos_addr: i32) {
            self.seq.push("fire");
            self.fired.push(goal_pos_addr);
        }
        fn set_status(&mut self, status: i32, value: i32) {
            self.seq.push("status");
            self.statuses.push((status, value));
        }
        fn holder_slot_08(&mut self, _st: &AiAttackPositionFireState) -> bool {
            self.seq.push("slot08");
            self.slot08
        }
        fn holder_slot_00(&mut self) {
            self.seq.push("slot00");
        }
        fn goal_object(&mut self) -> i32 {
            self.seq.push("goal");
            self.goal
        }
        fn pre_fire_current_weapon(&mut self, goal: i32, goal_pos_addr: i32) {
            self.seq.push("pre");
            self.pre.push((goal, goal_pos_addr));
        }
    }

    #[test]
    fn fire_state_gate_order_short_circuits() {
        let mut st = AiAttackPositionFireState {
            owner_status_438: 0,
            firing_24: 0,
            holder_20: 0x300,
        };
        let mut h = FireHost::default();
        assert_eq!(
            ai_attack_position_fire_weapon_state_update(&mut st, 0x100, &mut h),
            STATE_FAILURE
        );
        assert_eq!(h.seq, vec!["weapon"]);

        let mut st2 = st;
        st2.owner_status_438 = 1;
        let mut h2 = FireHost {
            weapon: 0x77,
            ..Default::default()
        };
        assert_eq!(
            ai_attack_position_fire_weapon_state_update(&mut st2, 0x100, &mut h2),
            STATE_FAILURE
        );
        assert_eq!(h2.seq, vec!["weapon"], "the status byte skips testStatus");

        let mut h3 = FireHost {
            weapon: 0x77,
            status_52: true,
            ..Default::default()
        };
        assert_eq!(
            ai_attack_position_fire_weapon_state_update(&mut st, 0x100, &mut h3),
            STATE_FAILURE
        );
        assert_eq!(h3.seq, vec!["weapon", "status52"]);
    }

    #[test]
    fn fire_state_weapon_status_arms() {
        // PRE_ATTACK (4) keeps waiting
        let mut st = AiAttackPositionFireState::default();
        let mut h = FireHost {
            weapon: 0x77,
            wstatus: 4,
            ..Default::default()
        };
        assert_eq!(
            ai_attack_position_fire_weapon_state_update(&mut st, 0x100, &mut h),
            STATE_CONTINUE
        );
        assert_eq!(h.seq, vec!["weapon", "status52", "wstatus"]);

        // any other status than READY_TO_FIRE (0) fails
        let mut h2 = FireHost {
            weapon: 0x77,
            wstatus: 2,
            ..Default::default()
        };
        assert_eq!(
            ai_attack_position_fire_weapon_state_update(&mut st, 0x100, &mut h2),
            STATE_FAILURE
        );
        assert_eq!(h2.seq, vec!["weapon", "status52", "wstatus"]);
    }

    #[test]
    fn fire_state_ready_to_fire_arms_and_holder_gate() {
        // null holder slot: fires without consulting the virtual
        let mut st = AiAttackPositionFireState {
            holder_20: 0,
            ..Default::default()
        };
        let mut h = FireHost {
            weapon: 0x77,
            wstatus: 0,
            ..Default::default()
        };
        assert_eq!(
            ai_attack_position_fire_weapon_state_update(&mut st, 0x100, &mut h),
            STATE_SUCCESS
        );
        assert_eq!(
            h.seq,
            vec!["weapon", "status52", "wstatus", "cond", "fire", "status", "slot00"]
        );
        assert_eq!(h.fired, vec![0x100 + 0x24]);
        assert_eq!(h.statuses, vec![(0x1b, 0)]);

        // holder present, slot 0x08 true: same fire sequence after the probe
        let mut h2 = FireHost {
            weapon: 0x77,
            wstatus: 0,
            slot08: true,
            ..Default::default()
        };
        let mut st2 = AiAttackPositionFireState {
            holder_20: 0x300,
            ..Default::default()
        };
        assert_eq!(
            ai_attack_position_fire_weapon_state_update(&mut st2, 0x100, &mut h2),
            STATE_SUCCESS
        );
        assert_eq!(h2.seq.last(), Some(&"slot00"));
        assert_eq!(h2.seq[3], "slot08");

        // holder present, slot 0x08 false: failure, nothing fired
        let mut h3 = FireHost {
            weapon: 0x77,
            wstatus: 0,
            slot08: false,
            ..Default::default()
        };
        assert_eq!(
            ai_attack_position_fire_weapon_state_update(&mut st2, 0x100, &mut h3),
            STATE_FAILURE
        );
        assert!(h3.fired.is_empty());
    }

    #[test]
    fn fire_state_firing_flag_takes_the_pre_fire_arm() {
        let mut st = AiAttackPositionFireState {
            firing_24: 1,
            holder_20: 0x300,
            ..Default::default()
        };
        let mut h = FireHost {
            weapon: 0x77,
            wstatus: 4, // never consulted on this arm
            goal: 0x99,
            ..Default::default()
        };
        assert_eq!(
            ai_attack_position_fire_weapon_state_update(&mut st, -0x20, &mut h),
            STATE_CONTINUE
        );
        assert_eq!(st.firing_24, 0);
        assert_eq!(h.statuses, vec![(0xd, 1)]);
        assert_eq!(h.pre, vec![(0x99, (-0x20i32).wrapping_add(0x24))]);
        assert_eq!(
            h.seq,
            vec!["weapon", "status52", "status", "goal", "pre"]
        );
    }
}

