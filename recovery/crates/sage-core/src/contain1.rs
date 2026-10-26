//! Wave 47 `Contain` module — 16 rows in the `0x00864b*`–`0x00878c*` band
//! plus `0x00986083` (BFME2-patch image; the reference units quote retail
//! `0x004xxxxx` addresses = ours minus `0x400000`).
//!
//! All rows are Contain modules of `GameLogic/Object/Contain`: the
//! `GarrisonContain` placement walk, `TransportContain::update` /
//! `isSpecificRiderFreeToExit`, the `HordeContain` `+0x11C` interface
//! (compiled with the `this - 0x11C` folding, so raw struct offsets below
//! quote the *object/module/member* offsets the bodies actually read),
//! `HordeContain::onRemoving` (`+0x20` interface slot 23), the INI
//! `SplitResult` parser, and `HordeMeleeFormation::startMeleeAttack`.
//!
//! | addr | size | their symbol | one-line role |
//! |---|---|---|---|
//! | `0x0087894c` | 399 | `?addValidObjectsToGarrisonPoints@GarrisonContain@@QAEXXZ` | place each valid garrison occupant/member at victim, victim position, or own position |
//! | `0x00868749` | 392 | `?update@TransportContain@@UAE?AW4UpdateSleepTime@@XZ` | payload frame gate, health regen to contained, cond-89 sync to the `+0x10C` flag byte |
//! | `0x0086df9a` | 377 | `?rva0046DF9A@HordeContain@@UAE_NPAUCoord3D@@@Z` | `+0x11C` slot 139: mean *Drawable* position of contained + `+0x170`-keyed objects |
//! | `0x00864b96` | 364 | `?rva00464B96@OpenContain@@UAEXPAVObject@@@Z` | scatter a rider to a random polar point around the container |
//! | `0x0086bc30` | 320 | `?rva0046BC30@HordeContain@@UAEXXZ` | `+0x11C` slot 50: move live members to goal + rotated formation offsets while `+0x2A0` unset |
//! | `0x0086e113` | 320 | `?rva0046E113@HordeContain@@UAE_NPAUCoord3D@@@Z` | `+0x11C` slot 140: mean *Object* position of contained + keyed objects |
//! | `0x00869e03` | 300 | `?onRemoving@HordeContain@@UAEXPAVObject@@@Z` | `+0x20` slot 23: status/condition cleanup and the member-decal tail for a leaving object |
//! | `0x008739b4` | 299 | `?rva004739B4@HordeContain@@UAEXXZ` | `+0x11C` slot 49: level every contained/keyed object once, record experience in the `+0x258` map |
//! | `0x0086db6d` | 293 | `?rva0046DB6D@HordeContain@@UAEXABVAsciiString@@PAVRva2225E0Filter@@@Z` | `+0x11C` slot 118: add a named attribute modifier to every accepted member pool unless the store category is missing or index 6 |
//! | `0x00873799` | 285 | `?rva00473799@HordeContain@@UAEXABV?$list@PAVObject@@...` | `+0x11C` slot 30: assign spots, record experience, hand the maximum to the horde tracker |
//! | `0x0086c20b` | 284 | `?rva0046C20B@HordeContain@@UAEXXZ` | `+0x11C` slot 57: simple-horde-decal switch — self decal or per-member decals |
//! | `0x00986083` | 284 | `?startMeleeAttack@HordeMeleeFormation@@QAEXPAVObject@@@Z` | arm melee slots: record member positions into the `0x54`-stride attack vector |
//! | `0x0086c4c0` | 279 | `?rva0046C4C0@HordeContain@@UAEXXZ` | `+0x11C` slot 55: store the AI query at `+0x19C`, clear cond 61 for idle movers, hand off to primary slot 32 |
//! | `0x0086f288` | 277 | `parseHordeContainSplitResult` | INI `SplitResult : UnitType : RankNumber` entry parse (no file I/O — token stream supplied) |
//! | `0x0086ecde` | 273 | `?rva0046ECDE@HordeContain@@UAEXPBUCoord3D@@0HW4PathfindLayerEnum@@@Z` | `+0x11C` slot 111: send every member through a portal with the averaged-dest micropath |
//! | `0x008671ac` | 267 | `?isSpecificRiderFreeToExit@TransportContain@@MAE_NPAVObject@@@Z` | rider exit gate: AI free-to-exit, airborne override, terrain/land checks |
//!
//! Shared object-layout facts (from the decompiled bodies; the reference
//! `TransportContain.cpp` Object view pins the same words):
//! - `Object::m_id` @ `+0x74`; position `+0x38/0x3c/0x40`; orientation
//!   `+0x44`; bounding-circle radius `+0xb8`.
//! - model-condition words base `+0x10C` (`BitFlags<591>`, 19 dwords):
//!   condition 61 = dword `+0x110` bit 29 (clear = byte `+0x113 & 0xdf`),
//!   condition 89 = dword `+0x114` bit 25 (clear = byte `+0x117 & 0xfd`),
//!   conditions 254/255 = dword `+0x128` bits 30/31 (clear = byte
//!   `+0x12B` masks `0xbf`/`0x7f`). The change notifier is `FUN_0068ae6D`
//!   (`Object::rva0028AE6D`) and fires only when a bit actually flips.
//! - `Object::m_contain` @ `+0x250`, `m_body` @ `+0x254`,
//!   `m_ai` (AIUpdateInterface) @ `+0x258` (the bodies write `+600`);
//!   `m_264` experience tracker (current XP float @ `+0x10`);
//!   `+0x438` bit 0 = effectively-dead; `ThingTemplate` ptr @ `+0x4`
//!   with kind-of bit `+0x11f:0x80` (BF), status masks `+0x109:0x8`,
//!   `+0x113:0x24`.
//! - `HordeContain` `+0x11C` iface fields fold to `this-0x11C`:
//!   contained-list pair helper `FUN_0086247d`; `+0x170` keyed set at
//!   `+0x54` (resolved through `FUN_00449dc5` = `TheGameLogic::findObjectByID`);
//!   experience map `m_258` (key = `ThingTemplate::m_5D8` `u16`,
//!   `FUN_00870041` lookup); `m_188` formation-offset records, stride
//!   `0x1c`, offsets x/y at record `+0x04`/`+0x08`; `m_2A0` gate at
//!   iface `+0x184`; `m_19C` at iface `+0x80`; `m_198` byte at iface
//!   `+0x7c`; slot index lookup `FUN_0086acf6(id)` = `rva0046ACF6`.
//! - singleton globals: `DAT_00dfe78c` = TheGameLogic (logic frame @
//!   `+0x40`), `DAT_00dfeee8` = ThePlayerList (local player @ `+0x10`),
//!   `DAT_00dfe758 + 0x9a6` = `m_useSimpleHordeDecals` (**set when the
//!   byte is non-zero**; every body tests `== 0` for the *simple* skip),
//!   `DAT_00dfec50` = TheTerrainLogic (height slot `+0x1c`,
//!   isUnderwater slot `+0x4c`).
//! - float constants: `DAT_00bbb8d8` = 1.0f (count step / numerator),
//!   `DAT_00bc26f0` = 0.5f (portal dest average), `DAT_00bcf628` =
//!   0.01f (health-regen percent), `DAT_00bc746c` = 6.2831855f,
//!   `DAT_00bc8980` = 1.5f (scatter radius scale), `DAT_00dba4f8` =
//!   seconds per logic frame (runtime value, passed in).
//!
//! Faithful-quirk notes pinned by the tests:
//! - The x87 bodies hold register-only arithmetic chains in extended
//!   precision with a single rounding at the final store; the mirrors
//!   model those chains in `f64` (exact for products of `f32` values)
//!   and cast once (transport heal amount, scatter target, slot-50
//!   rotation, slot-30 angle). Loops that store each iteration (mean
//!   positions) stay `f32` per step.
//! - Mean centers (slots 139/140) divide *once* — `scale = 1.0f /
//!   count` — then multiply each component; not three divisions.
//! - Slots 30/49: retail advances the list walk **only inside** the
//!   non-null / has-AI branch, so a null-AI (30) or null-object (49)
//!   entry is a live infinite loop in the game; the mirrors panic
//!   (the `crtbuf.rs` UB precedent).
//! - `0x0086bc30` keeps dead CRT `sin`/`cos` calls before the retained
//!   FSINCOS pair; only the fsincos results are used. Its index
//!   (`rva0046ACF6`) is **not** bounds-checked against `m_188` — the
//!   game dereferences wild; the mirror panics.
//! - `0x00986083` dereferences the member AI for `resetAttack` even when
//!   the earlier guard passed an AI-less member (only reachable when the
//!   status `0x1C` is set); the mirror panics there too.
//! - The garrison row tests the slot-`0xB0` mask bit twice on the
//!   member path (two temporaries, same receiver and argument → same
//!   value); modelled as one flag.
//! - The parse row allocates the 12-byte entry first, so both error
//!   paths leak it in the game; the Rust version returns `Err` with the
//!   exception kind instead of throwing (no file I/O, no INI object).

/// Three `f32` components — `Coord3D`. Object position fields live at
/// `+0x38` (x), `+0x3c` (y), `+0x40` (z).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Coord3D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

// ---------------------------------------------------------------------------
// 0x0087894c — GarrisonContain::addValidObjectsToGarrisonPoints
// ---------------------------------------------------------------------------

/// Where `putObjectAtBestGarrisonPoint` (`0x008785a2`) is asked to place an
/// object: at the current victim (`FUN_00668d71` non-null → victim arg,
/// position arg null), else at the victim position (`FUN_00664e93` non-null),
/// else at the object's own position — unless the `+0x20` contain interface's
/// slot-`0xB0` 16-byte mask has bit 61 (`+4` dword bit 29) set, which skips
/// the placement entirely.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GarrisonTarget {
    Victim(u32),
    VictimPosition(Coord3D),
    OwnPosition,
}

/// The per-object placement facts (victim, victim position, own position
/// `+0x38`, and the slot-`0xB0` mask bit 61). The mask bit is only
/// *consulted* when both victim lookups fail; callers may precompute.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlacementFacts {
    /// `FUN_00668d71` — current victim's ObjectID (`None` = null).
    pub current_victim: Option<u32>,
    /// `FUN_00664e93` — victim position (`None` = null).
    pub victim_position: Option<Coord3D>,
    /// Object position `+0x38`.
    pub own_position: Coord3D,
    /// bit 61 of the slot-`0xB0` mask (tested twice on the member path).
    pub contain_mask_bit61: bool,
}

/// One second-level member of an occupant's contain-module list (the
/// interface answer at vtable slot `+0x7C`, list filled through slot
/// `+0x108`). Guards: current weapon (`FUN_0068aebd(0)`), weapon template
/// byte getter zero (`FUN_006c9400` through weapon `+4`), AI at `+0x258`.
#[derive(Clone, Debug, PartialEq)]
pub struct GarrisonMember {
    pub id: u32,
    pub has_current_weapon: bool,
    pub weapon_template_getter_zero: bool,
    pub has_ai: bool,
    pub facts: PlacementFacts,
}

/// One occupant of the garrison contain list.
#[derive(Clone, Debug, PartialEq)]
pub struct GarrisonOccupant {
    pub id: u32,
    /// `testStatus(0x25)` (`FUN_0044e536`) — set occupants are skipped.
    pub garrisoned_status: bool,
    /// AI present at `+0x258` — required on the no-contain-module path.
    pub has_ai: bool,
    /// contain module non-null at `+0x250`.
    pub has_contain_module: bool,
    /// slot `+0x7C` answer: `None` = null interface (occupant skipped),
    /// `Some(members)` = the second-level list (only read when
    /// `has_contain_module`).
    pub contain_interface_members: Option<Vec<GarrisonMember>>,
    pub facts: PlacementFacts,
}

/// One placement the walk would hand to `putObjectAtBestGarrisonPoint`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GarrisonPlacementRecord {
    pub object_id: u32,
    pub target: GarrisonTarget,
}

fn garrison_target(facts: &PlacementFacts) -> Option<GarrisonTarget> {
    if let Some(v) = facts.current_victim {
        return Some(GarrisonTarget::Victim(v));
    }
    if let Some(p) = facts.victim_position {
        return Some(GarrisonTarget::VictimPosition(p));
    }
    if facts.contain_mask_bit61 {
        // double test on the member path; identical temporaries → same bit.
        return None;
    }
    Some(GarrisonTarget::OwnPosition)
}

/// `0x0087894c` — walk the garrison contain list (pair helper
/// `FUN_0086247d`); skip occupants with status `0x25`; an occupant *with*
/// a contain module at `+0x250` is never placed itself — only its
/// interface-list members that pass the weapon/getter/AI guard are; an
/// occupant *without* a contain module is placed itself when it has an AI.
pub fn garrison_contain_add_valid_objects_to_garrison_points(
    occupants: &[GarrisonOccupant],
) -> Vec<GarrisonPlacementRecord> {
    let mut out = Vec::new();
    for occ in occupants {
        if occ.garrisoned_status {
            continue;
        }
        if !occ.has_contain_module {
            if occ.has_ai {
                if let Some(t) = garrison_target(&occ.facts) {
                    out.push(GarrisonPlacementRecord {
                        object_id: occ.id,
                        target: t,
                    });
                }
            }
        } else if let Some(members) = &occ.contain_interface_members {
            for m in members {
                if m.has_current_weapon
                    && m.weapon_template_getter_zero
                    && m.has_ai
                {
                    if let Some(t) = garrison_target(&m.facts) {
                        out.push(GarrisonPlacementRecord {
                            object_id: m.id,
                            target: t,
                        });
                    }
                }
            }
        }
        // `contain_interface_members == None` with a module present models
        // the null slot-`0x7C` answer: whole occupant skipped.
    }
    out
}

// ---------------------------------------------------------------------------
// 0x00868749 — TransportContain::update
// ---------------------------------------------------------------------------

/// One member of the body-contain list (`m_contain` `+0x250`, interface
/// slot `+0x118`). `present = false` models a null `Object*` entry: the
/// regen walk dereferences it (game crash → panic), the cond-89 walk
/// checks for it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TransportContainedMember {
    pub present: bool,
    /// `m_body(+0x254)` slot `+0x10` — max health (body assumed present).
    pub max_health: f32,
    /// `m_body(+0x254)` slot `+0x18` — current health.
    pub cur_health: f32,
    /// model condition 89: dword `+0x114` bit 25.
    pub condition_89: bool,
}

/// What one `update` pass did. `healed` holds the amount per member the
/// `attemptHealing` call would carry (list order); `condition89_transitions`
/// holds `true` for set / `false` for cleared, members that changed only.
#[derive(Clone, Debug, PartialEq)]
pub struct TransportUpdateOutcome {
    /// primary slot 29 (`createPayload`) fired.
    pub create_payload: bool,
    pub healed: Vec<f32>,
    /// `UpdateUpgradeCreationTriggers` (`0x00868045`) ran (always).
    pub upgrade_triggers: bool,
    /// new value of the module byte at `+0x10C` (`param +0xfc`).
    pub condition_flag_10c: bool,
    pub condition89_transitions: Vec<bool>,
    /// primary slot 30 ran (always, after `OpenContain::update`).
    pub open_contain_slot30: bool,
    /// the `UpdateSleepTime` returned by `OpenContain::update` (`0x008640be`).
    pub sleep_time: u32,
}

/// `0x00868749` — the transport update slot.
///
/// Order exactly as the body: (1) payload gate — module-data byte `+0x150`
/// set **and** module frame word (`+0xf8` through the `+0x10` interface)
/// `<=` TheGameLogic frame (`+0x40`), unsigned compare; (2) regen —
/// module-data float `+0xa8` non-zero **and** the object's own body
/// (`+0x250`… the body interface read at `+0x250` through the Object base)
/// non-null, then per member `cur < max` heals
/// `(float)(max * regen * seconds_per_logic_frame * 0.01f)` — one extended
/// -precision chain, single final cast; (3) `UpdateUpgradeCreationTriggers`;
/// (4) the cond-61-vs-flag byte sync — on a *transition only*, the flag
/// follows bit 29 of self `+0x110` and every member's cond 89 follows it
/// (set-if-clear / clear-if-set, notifier on every flip); (5) return
/// `OpenContain::update()` and run primary slot 30.
#[allow(clippy::too_many_arguments)]
pub fn transport_contain_update(
    md_creates_payload: bool,
    module_frame_f8: u32,
    logic_frame: u32,
    health_regen: f32,
    own_body_present: bool,
    members: &mut [TransportContainedMember],
    own_condition_61: bool,
    condition_flag_10c: bool,
    seconds_per_logic_frame: f32,
    open_contain_update_result: u32,
) -> TransportUpdateOutcome {
    let mut out = TransportUpdateOutcome {
        create_payload: false,
        healed: Vec::new(),
        upgrade_triggers: true,
        condition_flag_10c,
        condition89_transitions: Vec::new(),
        open_contain_slot30: true,
        sleep_time: open_contain_update_result,
    };

    if md_creates_payload && module_frame_f8 <= logic_frame {
        out.create_payload = true;
    }

    if health_regen != 0.0 && own_body_present {
        for m in members.iter_mut() {
            if !m.present {
                // body-slot load through null Object*: the game faults.
                panic!("contained list holds a null Object* (regen walk dereferences it)");
            }
            if m.cur_health < m.max_health {
                let amount = (m.max_health as f64
                    * health_regen as f64
                    * seconds_per_logic_frame as f64
                    * f64::from(0.01f32)) as f32;
                out.healed.push(amount);
            }
        }
    }

    let flag_set = condition_flag_10c;
    if own_condition_61 != flag_set {
        let new_flag = own_condition_61;
        out.condition_flag_10c = new_flag;
        for m in members.iter_mut() {
            if !m.present {
                continue;
            }
            if new_flag {
                if !m.condition_89 {
                    m.condition_89 = true;
                    out.condition89_transitions.push(true);
                }
            } else if m.condition_89 {
                m.condition_89 = false;
                out.condition89_transitions.push(false);
            }
        }
    }

    out
}

// ---------------------------------------------------------------------------
// 0x0086df9a / 0x0086e113 — HordeContain mean centers (iface slots 139/140)
// ---------------------------------------------------------------------------

/// Mean over a pair of lists — the shared accumulation of slots 139/140.
/// `contained` is the contained list, `keyed` the `+0x170` set entries
/// resolved through `findObjectByID` (`None` = dead id). `None` per entry
/// models "no object" (slot 140) or "no Drawable" (slot 139).
///
/// Accumulation is per-component `f32` (each add stores the running
/// total like the body's `*param_2 = *param_2 + x`); on a non-zero count
/// the division happens *once* — `scale = 1.0f / count` — and each
/// component is scaled by it. `None` result ⇔ `count == 0.0` (the out
/// param stays all-zero — nothing accumulated). `count` is a literal
/// 1.0 tally and can never be NaN, so only the position components can
/// carry NaN/inf through the running sum and the scaling multiplies.
fn mean_center(contained: &[Option<Coord3D>], keyed: &[Option<Coord3D>]) -> Option<Coord3D> {
    let mut c = Coord3D::default();
    let mut count = 0.0f32;
    for slot in contained.iter().chain(keyed.iter()) {
        if let Some(p) = slot {
            c.x = c.x + p.x;
            c.y = c.y + p.y;
            c.z = c.z + p.z;
            count = count + 1.0;
        }
    }
    if count == 0.0 {
        return None;
    }
    // `DAT_00bbb8d8 / count` into a float local, then three multiplies.
    let scale = 1.0f32 / count;
    c.x = scale * c.x;
    c.y = scale * c.y;
    c.z = c.z * scale;
    Some(c)
}

/// `0x0086df9a` — `+0x11C` slot 139: mean of the *Drawable* positions
/// (`FUN_009508e2` getDrawable, `FUN_00676470` `rva00276470` position) of
/// contained and keyed objects; `None` (zeroed center) when none have one.
pub fn horde_contain_mean_drawable_center(
    contained: &[Option<Coord3D>],
    keyed: &[Option<Coord3D>],
) -> Option<Coord3D> {
    mean_center(contained, keyed)
}

/// `0x0086e113` — `+0x11C` slot 140: mean of the *Object* positions
/// (`+0x38`) of contained and keyed objects; `None` when both lists are
/// empty of live entries.
pub fn horde_contain_mean_center(
    contained: &[Option<Coord3D>],
    keyed: &[Option<Coord3D>],
) -> Option<Coord3D> {
    mean_center(contained, keyed)
}

// ---------------------------------------------------------------------------
// 0x00864b96 — OpenContain::scatter
// ---------------------------------------------------------------------------

/// What the scatter does to the rider once the random point is known.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScatterAction {
    /// rider has no AI (`+0x258` null): `rider->setPosition(&target)`.
    RiderMovedToTarget,
    /// rider has AI: `ignoreObstacle(container)` then
    /// `aiMoveToPosition(&target, CMD_FROM_AI)`. With status mask bit 61
    /// set, the rider is first snapped to the container position —
    /// `setPosition`, `rva0023D3AF(TheGameLogic frame)`, `setPosition`
    /// again — hence `snapped` also carries the container position.
    RiderOrdered {
        snapped_to_container: bool,
        container_position: Coord3D,
    },
}

/// `0x00864b96` — scatter one rider away from the container.
///
/// Body flow (all `float10` = x87 extended, single rounding at each float
/// store — modelled with `f64` chains):
/// - angle draw = `GetGameLogicRandomValueReal(0.0f, 6.2831855f)`
///   (`FUN_00634092`, line-992 literal); unless the `+0x20` status slot
///   `0xB0` mask bit 61 is set, the angle instead becomes
///   `(float)(GetRelativeAngle(rider_pos) + container.orientation(+0x44))`.
/// - radius = container `+0xb8`; distance draw =
///   `GetGameLogicRandomValueReal(radius, radius * 1.5f)` — draws are
///   supplied pre-drawn so the mirror stays pure.
/// - `target.x = (float)(Cos(angle) * distance + pos.x)`, same `y` with
///   `Sin`; `target.z = TheTerrainLogic->height(target.x, target.y,
///   container layer, 0, 1)` (supplied, evaluated at the target xy).
/// - rider AI `+0x258` null → `setPosition(&target)`; else (bit61) snap
///   to the container position three calls, then `ignoreObstacle` and
///   the `CMD_FROM_AI = 2` move order.
pub fn open_contain_scatter(
    container_position: Coord3D,
    container_orientation: f32,
    container_radius: f32,
    rider_position: Coord3D, // read by the game's GetRelativeAngle call, not by the math
    rider_has_ai: bool,
    status_mask_bit61: bool,
    random_angle: f32,
    random_distance: f32,
    relative_angle: f32,
    terrain_height: f32,
) -> (Coord3D, ScatterAction) {
    // `Cos`/`Sin` game wrappers return rounded floats; the products then
    // live in x87 until the `local_1c/local_18` stores.
    let angle = if status_mask_bit61 {
        random_angle
    } else {
        ((relative_angle as f64) + f64::from(container_orientation)) as f32
    };
    let cos = angle.cos();
    let sin = angle.sin();
    let _ = container_radius; // radius only parameterizes the game's draw
    let _ = rider_position;
    let tx = ((f64::from(cos) * f64::from(random_distance)) + f64::from(container_position.x)) as f32;
    let ty = ((f64::from(sin) * f64::from(random_distance)) + f64::from(container_position.y)) as f32;
    let target = Coord3D {
        x: tx,
        y: ty,
        z: terrain_height,
    };
    let action = if rider_has_ai {
        ScatterAction::RiderOrdered {
            snapped_to_container: status_mask_bit61,
            container_position,
        }
    } else {
        ScatterAction::RiderMovedToTarget
    };
    (target, action)
}

// ---------------------------------------------------------------------------
// 0x0086bc30 — HordeContain slot 50: rotated formation offsets
// ---------------------------------------------------------------------------

/// One live member for the slot-50 move: `id` for the `m_188`-index lookup
/// (`FUN_0086acf6`), and the effectively-dead bit (`+0x438` bit 0). The
/// list is *not* null-checked by the body (`+0x438` is read off the entry
/// directly), so entries are assumed live objects.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FormationMoveMember {
    pub id: u32,
    pub effectively_dead: bool,
}

/// One `m_188` formation record: x/y at record `+0x04`/`+0x08`
/// (records are `0x1c`-stride).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FormationOffset2D {
    pub x: f32,
    pub y: f32,
}

/// `0x0086bc30` — `+0x11C` slot 50.
///
/// Gate: `m_2A0 != 0` → nothing at all; then `GetGoalPosition`
/// (`FUN_0068ad42`) must succeed (goal `local_2c/28/24`), and the goal
/// angle (`FUN_0068ad56`) is read. For every member: skip
/// effectively-dead; `index = rva0046ACF6(member.id)`; the offset record
/// `m_188Begin[index]` is read **without bounds checking** (negative or
/// oversized → the game's wild read → panic here). Rotation keeps only
/// the FSINCOS results (the two CRT calls before them are dead code kept
/// by codegen): `c = (float)cos(angle)`, `s = (float)sin(angle)` rounded
/// first, then the extended-precision chain
/// `new_x = c*ox - s*oy`, `new_y = (c*oy) + (s*ox)` (separate member
/// operations — `y = cos*y` then `+= sin*x`), and the final goal adds
/// `goal.x + new_x`, `goal.y + new_y`, `goal.z` straight through; each
/// result rounds once at its store. `FUN_0068acee` =
/// `Object::rva0028ACEE(&pos, goalLayer)` — the goal layer rides along.
pub fn horde_contain_move_members_to_goal(
    gate_2a0_set: bool,
    goal: Option<Coord3D>,
    goal_angle: f32,
    goal_layer: i32,
    members: &[FormationMoveMember],
    offsets: &[FormationOffset2D],
    slot_index_of: &dyn Fn(u32) -> i32,
) -> Vec<(u32, Coord3D, i32)> {
    let mut out = Vec::new();
    if gate_2a0_set {
        return out;
    }
    let goal = match goal {
        Some(g) => g,
        None => return out,
    };
    let c = goal_angle.cos();
    let s = goal_angle.sin();
    for m in members {
        if m.effectively_dead {
            continue;
        }
        let index = slot_index_of(m.id);
        if index < 0 || (index as usize) >= offsets.len() {
            panic!("m_188 index {} out of range — retail wild pointer read", index);
        }
        let off = offsets[index as usize];
        let nx = (f64::from(c) * f64::from(off.x) - f64::from(s) * f64::from(off.y)) as f32;
        let ny = ((f64::from(c) * f64::from(off.y)) + (f64::from(s) * f64::from(off.x))) as f32;
        let pos = Coord3D {
            x: (f64::from(goal.x) + f64::from(nx)) as f32,
            y: (f64::from(goal.y) + f64::from(ny)) as f32,
            z: goal.z,
        };
        out.push((m.id, pos, goal_layer));
    }
    out
}

// ---------------------------------------------------------------------------
// 0x00869e03 — HordeContain::onRemoving (+0x20 interface slot 23)
// ---------------------------------------------------------------------------

/// The leaving `Object`'s in-image state the body touches. `status_26`
/// models the status-bit word holding status `0x26` as a bool (the
/// clear-through `FUN_0063db0e` is an external call recorded in the
/// outcome). Condition bools are the `+0x128` dword bits 30/31.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RemovingObject {
    /// `ThingTemplate` `+0x109` bit 3.
    pub template_109_bit3: bool,
    /// `ThingTemplate` `+0x113` bits `0x24` (5 and 2).
    pub template_113_mask24: bool,
    /// status `0x26` currently set (the body clears it, unconditionally
    /// on the template hit — recorded regardless of prior state).
    pub status_26_set: bool,
    /// model condition 254 (`+0x128` bit 30).
    pub condition_254: bool,
    /// model condition 255 (`+0x128` bit 31).
    pub condition_255: bool,
}

/// Tail of the member-decal handling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecalTailAction {
    /// no owner (or owner has no active `+0x43C` drawable): tail skipped.
    None,
    /// simple-horde-decals global set: `ownerDraw->rva00272945(slot96(0))`.
    OwnerSimpleDecal,
    /// member drawable present with `+0x43C` set: early `return`.
    MemberDecalActiveSkip,
    /// member drawable inactive but the related object
    /// (`FUN_006931f5(0)`) has an active drawable: early `return`.
    RelatedDecalActiveSkip,
    /// the `draw->rva00272BE7()` cleanup fired.
    ClearMemberDecals,
}

/// `0x00869e03` — `HordeContain::onRemoving` minus the base call:
/// `OpenContain::onRemoving(object)` (`0x00864d02`) runs first and is the
/// caller's obligation. Then: template flags `+0x109:8 | +0x113:0x24` →
/// `setStatus(0x26, false)`; conditions 254 and 255 each cleared with the
/// notifier (`FUN_0068ae6d`) only when set; body slot `+0x34` with 5 and
/// `rva0029130c(2)` unconditionally (the body pointer at `+0x254` is
/// dereferenced unchecked — assumed present); module-data `+0x234 != -1`
/// and the object's AI → `slot142(0)`; then the decal tail keyed on the
/// owner (`m_object`, this `-0x18`) having a drawable with `+0x43C` set.
/// `m_use_simple_horde_decals` is `*(DAT_00dfe758 + 0x9a6) != 0`.
pub fn horde_contain_on_removing(
    object: &mut RemovingObject,
    module_data_234: Option<i32>,
    object_has_ai: bool,
    owner_active_drawable: bool,
    m_use_simple_horde_decals: bool,
    member_drawable_active: Option<bool>,
    related_drawable_active: Option<bool>,
) -> OnRemovingOutcome {
    let mut outcome = OnRemovingOutcome {
        status_26_cleared: false,
        condition_254_cleared: false,
        condition_255_cleared: false,
        body_slot13_5: true,
        rva0029130c_2: true,
        ai_slot142_0: false,
        decal_tail: DecalTailAction::None,
    };

    if object.template_109_bit3 || object.template_113_mask24 {
        object.status_26_set = false;
        outcome.status_26_cleared = true;
    }

    if object.condition_254 {
        object.condition_254 = false;
        outcome.condition_254_cleared = true;
    }
    if object.condition_255 {
        object.condition_255 = false;
        outcome.condition_255_cleared = true;
    }

    if let Some(v) = module_data_234 {
        if v != -1 && object_has_ai {
            outcome.ai_slot142_0 = true;
        }
    }

    if owner_active_drawable {
        if m_use_simple_horde_decals {
            outcome.decal_tail = DecalTailAction::OwnerSimpleDecal;
        } else {
            match member_drawable_active {
                // null drawable → active-by-default → return.
                None => outcome.decal_tail = DecalTailAction::MemberDecalActiveSkip,
                Some(true) => outcome.decal_tail = DecalTailAction::MemberDecalActiveSkip,
                Some(false) => {
                    if related_drawable_active == Some(true) {
                        outcome.decal_tail = DecalTailAction::RelatedDecalActiveSkip;
                    } else {
                        outcome.decal_tail = DecalTailAction::ClearMemberDecals;
                    }
                }
            }
        }
    }

    outcome
}

/// Everything the `onRemoving` tail decided (see [`horde_contain_on_removing`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OnRemovingOutcome {
    pub status_26_cleared: bool,
    pub condition_254_cleared: bool,
    pub condition_255_cleared: bool,
    /// body slot `+0x34(5)` and `rva0029130c(2)` fire on every call (the
    /// body pointer at `+0x254` must exist — retail UB otherwise).
    pub body_slot13_5: bool,
    pub rva0029130c_2: bool,
    pub ai_slot142_0: bool,
    pub decal_tail: DecalTailAction,
}

// ---------------------------------------------------------------------------
// 0x008739b4 — HordeContain slot 49: level-up + experience bookkeeping
// ---------------------------------------------------------------------------

/// One member for the experience walks (slots 49 and 30 share it).
/// `present = false` models a null `Object*` list entry;
/// `template_key` is `ThingTemplate::m_5D8` (`u16`, read at template
/// `+0x5d8`); `experience` is the tracker float at `m_264 + 0x10`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExperienceMember {
    pub id: u32,
    pub present: bool,
    pub template_key: u16,
    pub experience: f32,
}

/// Assign-or-update one entry of the `m_258` experience map
/// (`std::map<unsigned short, float>`, `operator[]` then assign).
fn map_assign(map: &mut Vec<(u16, f32)>, key: u16, value: f32) {
    for e in map.iter_mut() {
        if e.0 == key {
            e.1 = value;
            return;
        }
    }
    map.push((key, value));
}

/// `0x008739b4` — `+0x11C` slot 49. Early return when the `+0x170` set is
/// empty *and* the contained list is empty (the decompiled list-count loop
/// plus the dead `iVar3 == 0` retest are the inlined `list::size()`).
/// Every contained entry then levels once (`ExperienceTracker::rva0039B4EC
/// (1, false, false)`) and records its XP under its template key; a **null
/// entry does not advance the walk — retail hangs**, modelled by panic.
/// Keyed objects (`findObjectByID` resolved, `None` = dead) get the same
/// treatment with no quirk. Finally `FUN_0086e740(0)` fires.
pub fn horde_contain_level_and_record_experience(
    keyed_count: usize,
    contained: &[ExperienceMember],
    keyed: &[Option<ExperienceMember>],
    map: &mut Vec<(u16, f32)>,
) -> ExperienceWalkOutcome {
    let mut out = ExperienceWalkOutcome {
        leveled_ids: Vec::new(),
        finalize_86e740_0: true,
    };
    if keyed_count == 0 && contained.is_empty() {
        out.finalize_86e740_0 = false;
        return out;
    }
    for m in contained {
        if !m.present {
            panic!("null contained entry: retail advances only past non-null nodes");
        }
        out.leveled_ids.push(m.id);
        map_assign(map, m.template_key, m.experience);
    }
    for slot in keyed {
        if let Some(m) = slot {
            out.leveled_ids.push(m.id);
            map_assign(map, m.template_key, m.experience);
        }
    }
    out
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExperienceWalkOutcome {
    /// ids walked, in body order (contained list first, then keyed set).
    pub leveled_ids: Vec<u32>,
    pub finalize_86e740_0: bool,
}

// ---------------------------------------------------------------------------
// 0x0086db6d — HordeContain slot 118: attribute-modifier pool fan-out
// ---------------------------------------------------------------------------

/// One pool target (contained or keyed member); only the id and the
/// filter-relevant identity matter here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModifierMember {
    pub id: u32,
}

/// `0x0086db6d` — `+0x11C` slot 118. Unless the named modifier's store
/// category is missing (`category_index = None`) or has `m_index`
/// (`+0xc`) equal 6, add the modifier (name key `param_2`, arg `a3`) to
/// the pool of every contained object the filter accepts (all of them
/// with no filter — `FUN_00762437(obj, self-player)` is then never
/// called), then the same over the `+0x170`-keyed objects resolved
/// through `findObjectByID`. Independently of that, when the horde
/// object's own pool update (`FUN_0068bdd7`) exists the modifier is added
/// to it (`FUN_00803e72`).
pub fn horde_contain_add_modifier_to_pools(
    category_index: Option<i32>,
    contained: &[ModifierMember],
    keyed: &[Option<ModifierMember>],
    filter: Option<&dyn Fn(&ModifierMember) -> bool>,
    own_pool_present: bool,
) -> ModifierPoolOutcome {
    let mut out = ModifierPoolOutcome {
        pool_adds: Vec::new(),
        self_pool_add: false,
    };
    if let Some(idx) = category_index {
        if idx != 6 {
            for m in contained {
                match filter {
                    None => out.pool_adds.push(m.id),
                    Some(f) => {
                        if f(m) {
                            out.pool_adds.push(m.id);
                        }
                    }
                }
            }
            for slot in keyed {
                if let Some(m) = slot {
                    match filter {
                        None => out.pool_adds.push(m.id),
                        Some(f) => {
                            if f(m) {
                                out.pool_adds.push(m.id);
                            }
                        }
                    }
                }
            }
        }
    }
    if own_pool_present {
        out.self_pool_add = true;
    }
    out
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModifierPoolOutcome {
    pub pool_adds: Vec<u32>,
    pub self_pool_add: bool,
}

// ---------------------------------------------------------------------------
// 0x00873799 — HordeContain slot 30: spot assignment + experience max
// ---------------------------------------------------------------------------

/// One member of the slot-30 argument list. `ai_present` gates the whole
/// body; `has_ai == false` is the retail hang (walk does not advance).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AssignMember {
    pub id: u32,
    pub ai_present: bool,
    pub template_key: u16,
    pub experience: f32,
}

/// `0x00873799` — `+0x11C` slot 30 over a caller-supplied list (the
/// argument list, not the usual contained list). When `m_198` (iface
/// `+0x7c`) is clear, primary slot 33 fires with 1 first. Each entry with
/// an AI: `rva0028AD32`, slot 11 (assign spot), `m_258[template key] =
/// xp`, running max `best` (initial `1.0f` — `DAT_00bbb8d8`, strict `<`
/// update so a NaN xp never wins), `FUN_0086a893`, and — when the module
/// data exists and its `+0x234 != -1` — AI slot 142 with that value. A
/// no-AI entry **hangs the retail walk**; modelled by panic. The list is
/// *not* null-checked (AI read off the entry: game fault on null).
/// Afterwards the horde tracker gets `rva0039B3D1(best, true)`, then
/// `FUN_0086e740(0)`, slot 121, slot 4 (0) — all recorded as fired.
pub fn horde_contain_assign_spots_and_track_experience(
    m_198_set: bool,
    module_data_234: Option<i32>,
    members: &[AssignMember],
    map: &mut Vec<(u16, f32)>,
) -> AssignSpotsOutcome {
    let mut out = AssignSpotsOutcome {
        primary_slot33_1: !m_198_set,
        assigned_ids: Vec::new(),
        ad32_ids: Vec::new(),
        a893_ids: Vec::new(),
        slot142_ids: Vec::new(),
        best: 1.0f32,
        tracker_best: 0.0f32,
        finalize: true,
        slot121: true,
        slot4_0: true,
    };
    for m in members {
        if !m.ai_present {
            panic!("entry without AI: retail advances the walk only past AI entries");
        }
        out.ad32_ids.push(m.id);
        out.assigned_ids.push(m.id);
        map_assign(map, m.template_key, m.experience);
        if out.best < m.experience {
            out.best = m.experience;
        }
        out.a893_ids.push(m.id);
        if let Some(v) = module_data_234 {
            if v != -1 {
                out.slot142_ids.push(m.id);
            }
        }
    }
    out.tracker_best = out.best;
    out
}

#[derive(Clone, Debug, PartialEq)]
pub struct AssignSpotsOutcome {
    pub primary_slot33_1: bool,
    pub assigned_ids: Vec<u32>,
    pub ad32_ids: Vec<u32>,
    pub a893_ids: Vec<u32>,
    pub slot142_ids: Vec<u32>,
    pub best: f32,
    /// value handed to the horde tracker `rva0039B3D1(_, true)`.
    pub tracker_best: f32,
    pub finalize: bool,
    pub slot121: bool,
    pub slot4_0: bool,
}

// ---------------------------------------------------------------------------
// 0x0086c20b — HordeContain slot 57: horde decal switch
// ---------------------------------------------------------------------------

/// One member/keyed object for the decal walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecalMember {
    pub id: u32,
    /// `getDrawable()` (`FUN_009508e2`) non-null.
    pub has_drawable: bool,
}

/// `0x0086c20b` — `+0x11C` slot 57. When the horde object exists and its
/// controlling player (`FUN_0068afa9`) is the local player
/// (`ThePlayerList +0x10`): with simple-horde-decals set
/// (`*(DAT_00dfe758 + 0x9a6) != 0`) and a self Drawable, fire
/// `rva00272BAB(slot96(0), slot101(self))`; otherwise set the
/// `use_member_decals` switch, which then gates the per-drawable
/// `rva00272BAB(1, slot101(obj))` calls over the contained list and the
/// resolved `+0x170` keys (walks advance past null entries here).
pub fn horde_contain_set_member_decal_state(
    self_present: bool,
    self_locally_owned: bool,
    m_use_simple_horde_decals: bool,
    self_has_drawable: bool,
    contained: &[DecalMember],
    keyed: &[Option<DecalMember>],
) -> DecalUpdateOutcome {
    let mut out = DecalUpdateOutcome {
        use_member_decals: false,
        self_decal: false,
        member_decal_ids: Vec::new(),
    };
    if self_present && self_locally_owned {
        if m_use_simple_horde_decals {
            if self_has_drawable {
                out.self_decal = true;
            }
        } else {
            out.use_member_decals = true;
        }
    }
    if out.use_member_decals {
        for m in contained {
            if m.has_drawable {
                out.member_decal_ids.push(m.id);
            }
        }
        for slot in keyed {
            if let Some(m) = slot {
                if m.has_drawable {
                    out.member_decal_ids.push(m.id);
                }
            }
        }
    }
    out
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecalUpdateOutcome {
    pub use_member_decals: bool,
    pub self_decal: bool,
    pub member_decal_ids: Vec<u32>,
}

// ---------------------------------------------------------------------------
// 0x00986083 — HordeMeleeFormation::startMeleeAttack
// ---------------------------------------------------------------------------

/// One attack slot in the `0x54`-stride melee vector (`state` dword at
/// record `+0x00`, position at `+0x04/08/0c`, `needs_position` byte at
/// `+0x1c`, path deque cleared through `FUN_0098592d`).
#[derive(Clone, Debug, PartialEq)]
pub struct FormationAttackEntry {
    pub position: Coord3D,
    pub needs_position: bool,
    pub path: Vec<Coord3D>,
}

/// One listed unit. `ai` fields ride the AIUpdateInterface: mode dword
/// `+0x1fc` and the engagement pointer `+0x140`; `idle_blocked` is AI
/// slot `+0x1b8`; `formation_status_1c` is `testStatus(0x1c)`.
#[derive(Clone, Debug, PartialEq)]
pub struct MeleeMember {
    pub id: u32,
    pub ai_present: bool,
    pub ai_mode_1fc: i32,
    pub ai_engaged_140: bool,
    pub formation_status_1c: bool,
    pub idle_blocked: bool,
    pub position: Coord3D,
}

/// One member that got an attack slot armed: the entry index and the two
/// follow-through externals that fired (`rva0028ACEE(self position,
/// layer)` via `FUN_0068b511`, and AI slot `+0x220` `resetAttack`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeleeAttackAssignment {
    pub object_id: u32,
    pub entry_index: usize,
    pub position: Coord3D,
}

/// `0x00986083` — set the held-object flag byte (`object +0x120`), walk
/// the formation list (the `+0x20`-installed fill through slot `+0x118`),
/// and for each unit: skip when `ai && ai+0x1fc == 4 && ai+0x140`
/// (already melee-engaged); look up the attack-slot index by object id
/// (`FUN_0086acf6`); skip when `index < 0 || index >= count` (count =
/// `(end - begin) / 0x54`, the C division truncates); when status `0x1C`
/// is clear: no AI → skip the whole unit, else fire the idle command
/// (`FUN_005e8a38(2)`) unless the AI reports idle-blocked. Then record:
/// `entry.position = unit.position`, `needsPosition = true`, path cleared
/// — and the AI-slot-`+0x220` reset dereference happens **even for an
/// AI-less unit** once the status gate passed: the game faults, the
/// mirror panics.
pub fn horde_melee_formation_start_melee_attack(
    held_active: &mut bool,
    attacks: &mut [FormationAttackEntry],
    members: &[MeleeMember],
    slot_index_of: &dyn Fn(u32) -> i32,
) -> MeleeAttackOutcome {
    *held_active = true;
    let mut out = MeleeAttackOutcome {
        idle_commands: Vec::new(),
        assignments: Vec::new(),
    };
    for m in members {
        let engaged = m.ai_present && m.ai_mode_1fc == 4 && m.ai_engaged_140;
        if engaged {
            continue;
        }
        let index = slot_index_of(m.id);
        let count = attacks.len();
        if index < 0 || (index as usize) >= count {
            continue;
        }
        if !m.formation_status_1c {
            if !m.ai_present {
                continue;
            }
            if !m.idle_blocked {
                out.idle_commands.push(m.id);
            }
        }
        let e = &mut attacks[index as usize];
        e.position = m.position;
        e.needs_position = true;
        e.path.clear();
        if !m.ai_present {
            // resetAttack slot +0x220 on a null AI: retail fault.
            panic!("resetAttack through null AI (status 0x1C set, member has no AI)");
        }
        out.assignments.push(MeleeAttackAssignment {
            object_id: m.id,
            entry_index: index as usize,
            position: m.position,
        });
    }
    out
}

#[derive(Clone, Debug, PartialEq)]
pub struct MeleeAttackOutcome {
    /// units that got `aiIdle(FORMATION_COMMAND)` (`FUN_005e8a38(2)`).
    pub idle_commands: Vec<u32>,
    pub assignments: Vec<MeleeAttackAssignment>,
}

// ---------------------------------------------------------------------------
// 0x0086c4c0 — HordeContain slot 55
// ---------------------------------------------------------------------------

/// One member for the slot-55 walk. `accepts_290fbb` is the object
/// `rva00290FBB` gate; `ai_is_moving` is `FUN_00664688`;
/// `condition_61` is the `+0x110` bit-29 model condition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slot55Member {
    pub id: u32,
    pub present: bool,
    pub accepts_290fbb: bool,
    pub ai_is_moving: bool,
    pub condition_61: bool,
}

/// `0x0086c4c0` — `+0x11C` slot 55. First store our AI's
/// `rva00260DED` result (`FUN_00660ded`) at iface `+0x80` (field
/// `+0x19C`). Then, over the contained list and the keyed tree: present
/// objects the `rva00290FBB` query accepts get — when the AI is *not*
/// moving and model condition 61 is set — condition 61 cleared with the
/// notifier, and are handed to primary slot 32. Walks advance past every
/// entry; keyed objects are null-checked.
pub fn horde_contain_clear_condition61_and_hand_off(
    ai_query_260ded: i32,
    contained: &mut [Slot55Member],
    keyed: &mut [Option<Slot55Member>],
) -> Slot55Outcome {
    let mut out = Slot55Outcome {
        m_19c: ai_query_260ded,
        cleared_ids: Vec::new(),
        hand_off_ids: Vec::new(),
    };
    for slot in contained.iter_mut() {
        if !slot.present {
            continue;
        }
        if slot.accepts_290fbb {
            if !slot.ai_is_moving && slot.condition_61 {
                slot.condition_61 = false;
                out.cleared_ids.push(slot.id);
            }
            out.hand_off_ids.push(slot.id);
        }
    }
    for slot in keyed.iter_mut() {
        if let Some(m) = slot {
            if m.accepts_290fbb {
                if !m.ai_is_moving && m.condition_61 {
                    m.condition_61 = false;
                    out.cleared_ids.push(m.id);
                }
                out.hand_off_ids.push(m.id);
            }
        }
    }
    out
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Slot55Outcome {
    pub m_19c: i32,
    pub cleared_ids: Vec<u32>,
    pub hand_off_ids: Vec<u32>,
}

// ---------------------------------------------------------------------------
// 0x0086f288 — parseHordeContainSplitResult
// ---------------------------------------------------------------------------

/// The 12-byte INI entry (`new`-allocated, then pushed onto the module
/// data's `vector<const ModuleData *>` at `store`; the allocation itself
/// is dropped here): two `StringBase<char>` fields and the rank int.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HordeContainSplitEntry {
    pub split_result: String,
    pub unit_type: String,
    pub rank_number: i32,
}

/// Which `INIException(3, …)` the game would throw:
/// `"'Target' expected"` (`0x00c44bd8`) or `"'Result' expected"`
/// (`0x00c44bec`). Both throw through `DAT_00cfe2fc` ThrowInfo and leak
/// the 12-byte entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitParseError {
    TargetExpected,
    ResultExpected,
}

/// C `atoi`: optional leading whitespace, optional sign, digit run
/// (stop at the first non-digit); no digits → 0. Accumulation uses a
/// saturating `i64` then truncates like MSVC's `strtol`→`int` path.
fn c_atoi(s: &str) -> i32 {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() && (bytes[i] == b' ' || (bytes[i] >= 0x09 && bytes[i] <= 0x0d)) {
        i += 1;
    }
    let neg = if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        bytes[i] == b'-'
    } else {
        false
    };
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }
    // accumulate magnitude, saturating at 0x80000000
    let mut acc: i64 = 0;
    let mut any = false;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        let d = i64::from(bytes[i] - b'0');
        acc = (acc.saturating_mul(10) + d).min(0x8000_0000);
        any = true;
        i += 1;
    }
    if !any {
        return 0;
    }
    if acc == 0x8000_0000 {
        // strtol ERANGE: LONG_MAX / LONG_MIN on 32-bit long
        return if neg { i32::MIN } else { i32::MAX };
    }
    if neg {
        -(acc as i32)
    } else {
        acc as i32
    }
}

/// `0x0086f288` — the `SplitResult` INI callback, pure-bookkeeping half:
/// the colon-separator token stream (`getNextTokenOrNull` /
/// `getNextToken` over the INI at `+0x420`) is supplied by the caller so
/// the parse itself has no file I/O. Grammar:
/// `SplitResult : <split result> : UnitType : <unit type> :
/// [RankNumber : <int>]` — the `RankNumber` pair is optional and its
/// *absence* (or any token mismatch) stores 0. The entry is pushed to the
/// store list only on success; the two failure paths throw
/// `INIException(3, …)` — modelled as `Err`.
pub fn parse_horde_contain_split_result(
    next_token_or_null: &mut dyn FnMut() -> Option<String>,
    next_token: &mut dyn FnMut() -> String,
) -> Result<HordeContainSplitEntry, SplitParseError> {
    match next_token_or_null() {
        None => return Err(SplitParseError::TargetExpected),
        Some(tok) => {
            if tok != "SplitResult" {
                return Err(SplitParseError::TargetExpected);
            }
        }
    }
    let split_result = next_token();
    match next_token_or_null() {
        None => return Err(SplitParseError::ResultExpected),
        Some(tok) => {
            if tok != "UnitType" {
                return Err(SplitParseError::ResultExpected);
            }
        }
    }
    let unit_type = next_token();
    let rank_number = match next_token_or_null() {
        Some(tok) if tok == "RankNumber" => c_atoi(&next_token()),
        _ => 0,
    };
    Ok(HordeContainSplitEntry {
        split_result,
        unit_type,
        rank_number,
    })
}

// ---------------------------------------------------------------------------
// 0x0086ecde — HordeContain slot 111: portal transit
// ---------------------------------------------------------------------------

/// One member for the portal walk; the slot-7 result (`FUN_0086e6ea`
/// fix-up spot and angle) is supplied per member.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PortalMember {
    pub id: u32,
    pub present: bool,
    /// `(*param_1 + 0x1c)` — slot 7 — position and angle for the member.
    pub slot7_position: Coord3D,
    pub slot7_angle: f32,
}

/// One member's two transit calls: the `0x0086e6ea` row fix-up against
/// our object, then `AIUpdateInterface::moveThroughPortalWithMicropath
/// (a1, a2, a3, layer, &dest, &slot7 pos)`.
#[derive(Clone, Debug, PartialEq)]
pub struct PortalTransit {
    pub object_id: u32,
    pub fixup_position: Coord3D,
    pub fixup_angle: f32,
    pub destination: Coord3D,
}

/// `0x0086ecde` — `+0x11C` slot 111: send every contained member through
/// a portal. The walk reads the member pointer, skips null entries,
/// takes the slot-7 spot/angle, then computes the destination from our
/// object position: on `layer != LAYER_GROUND` (`param_5 != 1`) the
/// destination is `((a2 + self) * 0.5f)` per component (`DAT_00bc26f0` =
/// 0.5f — the decompiled `(*param_3 + local_2c) * DAT_00bc26f0`, f32 per
/// step), otherwise the self position verbatim. The advance (`*local_8`)
/// sits outside the null check here — walks past nulls.
pub fn horde_contain_send_members_through_portal(
    self_position: Coord3D,
    a2: Coord3D,
    layer: i32,
    members: &[PortalMember],
) -> Vec<PortalTransit> {
    let mut out = Vec::new();
    for m in members {
        if !m.present {
            continue;
        }
        let dest = if layer != 1 {
            Coord3D {
                x: (a2.x + self_position.x) * 0.5,
                y: (a2.y + self_position.y) * 0.5,
                z: (a2.z + self_position.z) * 0.5,
            }
        } else {
            self_position
        };
        out.push(PortalTransit {
            object_id: m.id,
            fixup_position: m.slot7_position,
            fixup_angle: m.slot7_angle,
            destination: dest,
        });
    }
    out
}

// ---------------------------------------------------------------------------
// 0x008671ac — TransportContain::isSpecificRiderFreeToExit
// ---------------------------------------------------------------------------

/// `0x008671ac` — TRUE (1) when the transport may force the rider out.
///
/// Body, exactly:
/// - null rider → TRUE ("in general, I can exit people").
/// - transport AI (`+0x258`) present **and** its slot-`+0x1a8`
///   free-to-exit query returns non-zero (not `FREE_TO_EXIT = 0`) → FALSE.
/// - airborne locomotor in use (`FUN_0068b81e`) → TRUE.
/// - rider AI (`+0x258`) and its current locomotor (AI `+0x1f0`) must
///   both exist → otherwise FALSE.
/// - transport KindOf BF (template `+0x11f` bit 7) **and** the
///   underwater query (TheTerrainLogic slot `+0x4c`, x/y only) →
///   `getClosestPointOnLand` (`FUN_006ee7ee`); TRUE on success.
/// - otherwise (not BF, or not underwater): pathfinder
///   `IsValidMovementTerrain(layer, locomotor, &myPosition)`
///   (`FUN_006e7296`); TRUE on success.
/// - fall through → FALSE.
///
/// The three world queries are supplied as callbacks so the mirror fires
/// exactly the calls the game makes (short-circuit discipline visible in
/// tests). `ai_free_to_exit` is only consulted when the AI exists.
pub fn transport_contain_is_specific_rider_free_to_exit(
    rider_present: bool,
    transport_ai_present: bool,
    ai_free_to_exit: u32,
    using_airborne_locomotor: bool,
    rider_has_ai_with_locomotor: bool,
    transport_kindof_bf: bool,
    is_underwater: &dyn Fn() -> bool,
    closest_point_on_land: &dyn Fn() -> bool,
    valid_movement_terrain: &dyn Fn() -> bool,
) -> bool {
    if !rider_present {
        return true;
    }
    if transport_ai_present && ai_free_to_exit != 0 {
        return false;
    }
    if using_airborne_locomotor {
        return true;
    }
    if !rider_has_ai_with_locomotor {
        return false;
    }
    if transport_kindof_bf && is_underwater() {
        return closest_point_on_land();
    }
    valid_movement_terrain()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(x: f32, y: f32, z: f32) -> Coord3D {
        Coord3D { x, y, z }
    }

    // -- 0x0087894c ---------------------------------------------------------

    #[test]
    fn garrison_placement_order_and_gates() {
        let facts = PlacementFacts {
            current_victim: Some(7),
            victim_position: Some(c(1.0, 2.0, 3.0)),
            own_position: c(9.0, 9.0, 9.0),
            contain_mask_bit61: false,
        };
        let occ = GarrisonOccupant {
            id: 1,
            garrisoned_status: false,
            has_ai: true,
            has_contain_module: false,
            contain_interface_members: None,
            facts,
        };
        // victim wins over victim position wins over own position:
        assert_eq!(
            garrison_contain_add_valid_objects_to_garrison_points(&[occ.clone()]),
            vec![GarrisonPlacementRecord {
                object_id: 1,
                target: GarrisonTarget::Victim(7)
            }]
        );
        let mut o2 = occ.clone();
        o2.facts.current_victim = None;
        assert_eq!(
            garrison_contain_add_valid_objects_to_garrison_points(&[o2])[0].target,
            GarrisonTarget::VictimPosition(c(1.0, 2.0, 3.0))
        );
        let mut o3 = occ.clone();
        o3.facts.current_victim = None;
        o3.facts.victim_position = None;
        assert_eq!(
            garrison_contain_add_valid_objects_to_garrison_points(&[o3.clone()])[0].target,
            GarrisonTarget::OwnPosition
        );
        // mask bit 61 skips the own-position fallback entirely:
        let mut o4 = o3.clone();
        o4.facts.contain_mask_bit61 = true;
        assert!(garrison_contain_add_valid_objects_to_garrison_points(&[o4]).is_empty());
        // status 0x25 occupants never placed:
        let mut o5 = occ;
        o5.garrisoned_status = true;
        assert!(garrison_contain_add_valid_objects_to_garrison_points(&[o5]).is_empty());
        // no-AI occupants without a contain module skipped:
        let mut o6 = o3;
        o6.has_ai = false;
        assert!(garrison_contain_add_valid_objects_to_garrison_points(&[o6]).is_empty());
    }

    #[test]
    fn garrison_module_branch_places_only_members() {
        let facts = PlacementFacts {
            current_victim: None,
            victim_position: None,
            own_position: c(0.0, 0.0, 0.0),
            contain_mask_bit61: false,
        };
        let member = |id: u32, weapon: bool, getter_zero: bool, ai: bool| GarrisonMember {
            id,
            has_current_weapon: weapon,
            weapon_template_getter_zero: getter_zero,
            has_ai: ai,
            facts: facts.clone(),
        };
        let occ = GarrisonOccupant {
            id: 10,
            garrisoned_status: false,
            has_ai: true,
            has_contain_module: true,
            contain_interface_members: Some(vec![
                member(11, true, true, true),
                member(12, false, true, true),
                member(13, true, false, true),
                member(14, true, true, false),
            ]),
            facts: facts.clone(),
        };
        // the occupant itself is NOT placed; only members passing all
        // three guards:
        assert_eq!(
            garrison_contain_add_valid_objects_to_garrison_points(&[occ.clone()]),
            vec![GarrisonPlacementRecord {
                object_id: 11,
                target: GarrisonTarget::OwnPosition
            }]
        );
        // null slot-0x7C answer (None) skips the whole occupant:
        let occ2 = GarrisonOccupant {
            contain_interface_members: None,
            ..occ
        };
        assert!(garrison_contain_add_valid_objects_to_garrison_points(&[occ2]).is_empty());
    }

    // -- 0x00868749 ---------------------------------------------------------

    fn tm(max: f32, cur: f32, c89: bool) -> TransportContainedMember {
        TransportContainedMember {
            present: true,
            max_health: max,
            cur_health: cur,
            condition_89: c89,
        }
    }

    #[test]
    fn transport_payload_gate_is_unsigned() {
        let mut members = vec![];
        let o = transport_contain_update(
            true, 100, 100, 0.0, true, &mut members, false, false, 0.046, 1,
        );
        assert!(o.create_payload); // <= fires on equality
        let o = transport_contain_update(
            true, 101, 100, 0.0, true, &mut members, false, false, 0.046, 1,
        );
        assert!(!o.create_payload);
        let o = transport_contain_update(
            false, 0, u32::MAX, 0.0, true, &mut members, false, false, 0.046, 2,
        );
        assert!(!o.create_payload); // byte +0x150 clear
        assert_eq!(o.sleep_time, 2);
    }

    #[test]
    fn transport_heal_is_one_x87_chain() {
        let mut members = vec![tm(100.0, 50.0, false), tm(100.0, 100.0, false)];
        let o = transport_contain_update(
            false, 0, 0, 20.0, true, &mut members, false, false, 0.125, 1,
        );
        // full-precision chain then one cast: 100*20*0.125*0.01 = 2.5
        assert_eq!(o.healed, vec![2.5]);
        // cur == max is NOT hurt: the body's strict `<` gate.
        assert_eq!(members[1].cur_health, 100.0);
    }

    #[test]
    fn transport_cond89_sync_only_on_transition() {
        let mut members = vec![tm(1.0, 1.0, false), tm(1.0, 1.0, true)];
        // no transition: flag and bit agree → members untouched.
        let o = transport_contain_update(
            false, 0, 0, 0.0, false, &mut members, true, true, 0.0, 1,
        );
        assert!(o.condition89_transitions.is_empty());
        // transition 0→1: set members that lack 89.
        let o = transport_contain_update(
            false, 0, 0, 0.0, false, &mut members, true, false, 0.0, 1,
        );
        assert!(o.condition_flag_10c);
        assert_eq!(o.condition89_transitions, vec![true]);
        assert_eq!(members[0].condition_89, true);
        // transition 1→0: clear members that hold 89.
        let o = transport_contain_update(
            false, 0, 0, 0.0, false, &mut members, false, true, 0.0, 1,
        );
        assert!(!o.condition_flag_10c);
        assert_eq!(o.condition89_transitions, vec![false, false]); // members 0 and 1 flipped
        assert_eq!(members[0].condition_89, false);
    }

    #[test]
    #[should_panic]
    fn transport_null_entry_faults_the_regen_walk() {
        let mut members = vec![TransportContainedMember {
            present: false,
            max_health: 0.0,
            cur_health: 0.0,
            condition_89: false,
        }];
        transport_contain_update(true, 0, 0, 1.0, true, &mut members, false, false, 0.0, 1);
    }

    // -- 0x0086df9a / 0x0086e113 -------------------------------------------

    #[test]
    fn mean_center_empty_is_none_with_zero() {
        assert_eq!(horde_contain_mean_center(&[], &[]), None);
        assert_eq!(horde_contain_mean_center(&[None], &[None, None]), None);
    }

    #[test]
    fn mean_center_scales_by_reciprocal() {
        // three entries spread over both lists
        let got = horde_contain_mean_center(
            &[Some(c(1.0, 0.0, 0.0)), Some(c(2.0, 0.0, 0.0))],
            &[Some(c(3.0, 6.0, 9.0)), None],
        )
        .unwrap();
        assert_eq!(got, c(2.0, 2.0, 3.0));
        // the reciprocal path is visible with a non-power-of-2 count:
        // (1.0/3.0 rounds, then each component multiplies) — pin it:
        let got = horde_contain_mean_center(
            &[Some(c(1.0, 0.0, 0.0)), Some(c(0.0, 1.0, 0.0)), Some(c(0.0, 0.0, 1.0))],
            &[],
        )
        .unwrap();
        let scale = 1.0f32 / 3.0f32;
        assert_eq!(got.x, scale * 1.0);
        assert_eq!(got.x.to_bits(), scale.to_bits());
    }

    #[test]
    fn mean_center_nan_and_inf_edges() {
        // count==0 test is `==`, never NaN; sums propagate NaN exactly.
        let got = horde_contain_mean_center(&[Some(c(f32::NAN, 1.0, 0.0))], &[]).unwrap();
        assert!(got.x.is_nan());
        assert_eq!(got.y, 1.0);
        let got = horde_contain_mean_center(
            &[Some(c(f32::INFINITY, 0.0, 0.0))],
            &[Some(c(f32::INFINITY, 0.0, 0.0))],
        )
        .unwrap();
        // inf + inf = inf; scale = 0.5 keeps inf (single multiply).
        assert!(got.x.is_infinite() && got.x > 0.0);
    }

    #[test]
    fn mean_drawable_center_shares_the_shape() {
        assert_eq!(
            horde_contain_mean_drawable_center(&[None], &[Some(c(4.0, -2.0, 1.0))]),
            Some(c(4.0, -2.0, 1.0))
        );
    }

    // -- 0x00864b96 ---------------------------------------------------------

    #[test]
    fn scatter_random_point_and_actions() {
        // angle 0: cos = 1, sin = 0 exactly → target straight +x.
        let (t, a) = open_contain_scatter(
            c(10.0, 20.0, 5.0),
            1.234,
            3.0,
            c(0.0, 0.0, 0.0),
            false,
            true, // bit61: random angle used verbatim
            0.0,
            7.0,
            9.999, // relative angle must be IGNORED with bit61
            -4.5,
        );
        assert_eq!(t, c(17.0, 20.0, -4.5));
        assert_eq!(a, ScatterAction::RiderMovedToTarget);

        // rider with AI, no bit61: orientation + relative angle, no snap.
        let (t, a) = open_contain_scatter(
            c(0.0, 0.0, 0.0),
            std::f32::consts::FRAC_PI_2,
            1.0,
            c(0.0, 0.0, 0.0),
            true,
            false,
            99.0, // random draw ignored on the non-bit61 path
            2.0,
            0.0,
            0.0,
        );
        // cos(pi/2)*2 ≈ 0 (1e-8-level residue), y = 2
        assert!(t.x.abs() < 1e-6);
        assert!((t.y - 2.0).abs() < 1e-6);
        assert_eq!(
            a,
            ScatterAction::RiderOrdered {
                snapped_to_container: false,
                container_position: c(0.0, 0.0, 0.0)
            }
        );

        // bit61 + AI → the three-call container snap rides along.
        let (_, a) = open_contain_scatter(
            c(5.0, 5.0, 1.0),
            0.0,
            1.0,
            c(1.0, 1.0, 1.0),
            true,
            true,
            0.0,
            1.0,
            0.0,
            0.0,
        );
        assert_eq!(
            a,
            ScatterAction::RiderOrdered {
                snapped_to_container: true,
                container_position: c(5.0, 5.0, 1.0)
            }
        );
    }

    // -- 0x0086bc30 ---------------------------------------------------------

    #[test]
    fn slot50_gate_and_dead_skips() {
        let m = |id: u32, dead: bool| FormationMoveMember {
            id,
            effectively_dead: dead,
        };
        assert!(horde_contain_move_members_to_goal(
            true,
            Some(c(0.0, 0.0, 0.0)),
            0.0,
            0,
            &[m(1, false)],
            &[FormationOffset2D { x: 1.0, y: 1.0 }],
            &|_| 0
        )
        .is_empty());
        assert!(horde_contain_move_members_to_goal(
            false,
            None,
            0.0,
            0,
            &[m(1, false)],
            &[FormationOffset2D { x: 1.0, y: 1.0 }],
            &|_| 0
        )
        .is_empty());
        assert!(horde_contain_move_members_to_goal(
            false,
            Some(c(0.0, 0.0, 0.0)),
            0.0,
            0,
            &[m(1, true)],
            &[FormationOffset2D { x: 1.0, y: 1.0 }],
            &|_| 0
        )
        .is_empty());
    }

    #[test]
    fn slot50_rotation_arithmetic() {
        // angle 0: c = 1, s = 0 exactly → identity rotation.
        let got = horde_contain_move_members_to_goal(
            false,
            Some(c(10.0, 20.0, 30.0)),
            0.0,
            2,
            &[FormationMoveMember {
                id: 7,
                effectively_dead: false,
            }],
            &[FormationOffset2D { x: 3.0, y: 4.0 }],
            &|_| 0,
        );
        assert_eq!(got, vec![(7, c(13.0, 24.0, 30.0), 2)]);
        // angle pi/2: c ≈ 0 (6e-8 residue), s = 1 → new_x = -y, new_y ≈ x.
        let got = horde_contain_move_members_to_goal(
            false,
            Some(c(0.0, 0.0, 0.0)),
            std::f32::consts::FRAC_PI_2,
            1,
            &[FormationMoveMember {
                id: 1,
                effectively_dead: false,
            }],
            &[FormationOffset2D { x: 3.0, y: 4.0 }],
            &|_| 0,
        );
        let (id, p, _) = got[0];
        assert_eq!(id, 1);
        assert!((p.x + 4.0).abs() < 1e-4);
        assert!((p.y - 3.0).abs() < 1e-4);
    }

    #[test]
    #[should_panic]
    fn slot50_negative_index_faults() {
        horde_contain_move_members_to_goal(
            false,
            Some(c(0.0, 0.0, 0.0)),
            0.0,
            0,
            &[FormationMoveMember {
                id: 5,
                effectively_dead: false,
            }],
            &[FormationOffset2D::default()],
            &|_| -1,
        );
    }

    // -- 0x00869e03 ---------------------------------------------------------

    fn robj(t109: bool, t113: bool, c254: bool, c255: bool) -> RemovingObject {
        RemovingObject {
            template_109_bit3: t109,
            template_113_mask24: t113,
            status_26_set: true,
            condition_254: c254,
            condition_255: c255,
        }
    }

    #[test]
    fn on_removing_condition_clears_are_gated() {
        let mut o = robj(false, false, true, false);
        let out = horde_contain_on_removing(&mut o, None, false, false, false, None, None);
        assert!(!out.status_26_cleared);
        assert!(out.condition_254_cleared && !out.condition_255_cleared);
        assert!(!o.condition_254 && o.status_26_set);
        assert!(out.body_slot13_5 && out.rva0029130c_2);
    }

    #[test]
    fn on_removing_template_gate_and_ai_slot() {
        let mut o = robj(false, true, false, false);
        let out = horde_contain_on_removing(&mut o, Some(-1), true, false, false, None, None);
        assert!(out.status_26_cleared && !o.status_26_set);
        assert!(!out.ai_slot142_0); // m_234 == -1
        let mut o2 = robj(true, false, false, false);
        let out = horde_contain_on_removing(&mut o2, Some(3), true, false, false, None, None);
        assert!(out.ai_slot142_0);
        let out = horde_contain_on_removing(&mut o2, Some(3), false, false, false, None, None);
        assert!(!out.ai_slot142_0); // no AI on the leaving object
    }

    #[test]
    fn on_removing_decal_tail_matrix() {
        // owner inactive → tail None regardless of the rest:
        let mut o = robj(false, false, false, false);
        let out = horde_contain_on_removing(&mut o, None, false, false, true, Some(false), None);
        assert_eq!(out.decal_tail, DecalTailAction::None);
        // owner active + simple decals → owner decal:
        let out = horde_contain_on_removing(&mut o, None, false, true, true, Some(false), None);
        assert_eq!(out.decal_tail, DecalTailAction::OwnerSimpleDecal);
        // not simple: member drawable missing or active → skip;
        for md in [None, Some(true)] {
            let out = horde_contain_on_removing(&mut o, None, false, true, false, md, Some(true));
            assert_eq!(out.decal_tail, DecalTailAction::MemberDecalActiveSkip);
        }
        // inactive member drawable + active related drawable → skip:
        let out = horde_contain_on_removing(&mut o, None, false, true, false, Some(false), Some(true));
        assert_eq!(out.decal_tail, DecalTailAction::RelatedDecalActiveSkip);
        // inactive member, no/quiet related → clear.
        for rel in [None, Some(false)] {
            let out = horde_contain_on_removing(&mut o, None, false, true, false, Some(false), rel);
            assert_eq!(out.decal_tail, DecalTailAction::ClearMemberDecals);
        }
    }

    // -- 0x008739b4 ---------------------------------------------------------

    fn em(id: u32, present: bool, key: u16, xp: f32) -> ExperienceMember {
        ExperienceMember {
            id,
            present,
            template_key: key,
            experience: xp,
        }
    }

    #[test]
    fn slot49_both_empty_guard() {
        let mut map = vec![];
        let out = horde_contain_level_and_record_experience(0, &[], &[], &mut map);
        assert!(!out.finalize_86e740_0);
        assert!(out.leveled_ids.is_empty());
        // keyed non-empty alone passes the guard with an empty list:
        let out = horde_contain_level_and_record_experience(
            1,
            &[],
            &[Some(em(9, true, 4, 5.0))],
            &mut map,
        );
        assert!(out.finalize_86e740_0);
        assert_eq!(map, vec![(4u16, 5.0f32)]);
        assert_eq!(out.leveled_ids, vec![9]);
    }

    #[test]
    fn slot49_map_assign_overwrites_and_keeps_order() {
        let mut map = vec![];
        let contained = [em(1, true, 7, 10.0), em(2, true, 7, 12.5)];
        let keyed = [None, Some(em(3, true, 9, 1.0))];
        let out = horde_contain_level_and_record_experience(3, &contained, &keyed, &mut map);
        assert_eq!(out.leveled_ids, vec![1, 2, 3]);
        assert_eq!(map, vec![(7u16, 12.5f32), (9u16, 1.0f32)]);
    }

    #[test]
    #[should_panic]
    fn slot49_null_entry_hangs_the_retail_walk() {
        let mut map = vec![];
        horde_contain_level_and_record_experience(1, &[em(1, false, 0, 0.0)], &[], &mut map);
    }

    // -- 0x0086db6d ---------------------------------------------------------

    #[test]
    fn slot118_category_gate_and_filter() {
        let contained = [ModifierMember { id: 1 }, ModifierMember { id: 2 }];
        let keyed = [Some(ModifierMember { id: 3 }), None];

        // category missing → only the self pool add (pool present):
        let out = horde_contain_add_modifier_to_pools(None, &contained, &keyed, None, true);
        assert!(out.pool_adds.is_empty() && out.self_pool_add);
        // category index 6 → same skip:
        let out = horde_contain_add_modifier_to_pools(Some(6), &contained, &keyed, None, false);
        assert!(out.pool_adds.is_empty() && !out.self_pool_add);
        // category fine, no filter → everything incl. keyed dead-id skip:
        let out = horde_contain_add_modifier_to_pools(Some(2), &contained, &keyed, None, false);
        assert_eq!(out.pool_adds, vec![1, 2, 3]);
        // filter drops members:
        let out = horde_contain_add_modifier_to_pools(
            Some(2),
            &contained,
            &keyed,
            Some(&|m: &ModifierMember| m.id != 2),
            false,
        );
        assert_eq!(out.pool_adds, vec![1, 3]);
    }

    // -- 0x00873799 ---------------------------------------------------------

    #[test]
    fn slot30_best_starts_at_one_and_is_strict() {
        let mut map = vec![];
        let ms = [
            AssignMember {
                id: 1,
                ai_present: true,
                template_key: 3,
                experience: 0.5,
            },
            AssignMember {
                id: 2,
                ai_present: true,
                template_key: 4,
                experience: 1.0, // == best → not replaced (strict <)
            },
            AssignMember {
                id: 3,
                ai_present: true,
                template_key: 5,
                experience: 7.25,
            },
        ];
        let out = horde_contain_assign_spots_and_track_experience(true, Some(11), &ms, &mut map);
        assert!(!out.primary_slot33_1);
        assert_eq!(out.best, 7.25);
        assert_eq!(out.tracker_best, 7.25);
        assert_eq!(out.slot142_ids, vec![1, 2, 3]);
        assert_eq!(map, vec![(3u16, 0.5f32), (4u16, 1.0f32), (5u16, 7.25f32)]);
    }

    #[test]
    fn slot30_nan_xp_never_wins_and_guards_fire() {
        let mut map = vec![];
        let ms = [AssignMember {
            id: 1,
            ai_present: true,
            template_key: 1,
            experience: f32::NAN,
        }];
        // best < NaN is false → best stays 1.0:
        let out = horde_contain_assign_spots_and_track_experience(false, None, &ms, &mut map);
        assert!(out.primary_slot33_1);
        assert_eq!(out.best, 1.0);
        assert!(out.slot142_ids.is_empty()); // null module data
        assert!(out.finalize && out.slot121 && out.slot4_0);
    }

    #[test]
    #[should_panic]
    fn slot30_no_ai_entry_hangs_the_walk() {
        let mut map = vec![];
        horde_contain_assign_spots_and_track_experience(
            true,
            Some(0),
            &[AssignMember {
                id: 1,
                ai_present: false,
                template_key: 0,
                experience: 0.0,
            }],
            &mut map,
        );
    }

    // -- 0x0086c20b ---------------------------------------------------------

    #[test]
    fn slot57_decal_switch() {
        let contained = [
            DecalMember {
                id: 1,
                has_drawable: true,
            },
            DecalMember {
                id: 2,
                has_drawable: false,
            },
        ];
        let keyed = [Some(DecalMember {
            id: 3,
            has_drawable: true,
        })];

        // local owner + simple decals + drawable → self decal only:
        let out = horde_contain_set_member_decal_state(true, true, true, true, &contained, &keyed);
        assert!(out.self_decal && !out.use_member_decals && out.member_decal_ids.is_empty());
        // simple decals but no drawable → nothing:
        let out = horde_contain_set_member_decal_state(true, true, true, false, &contained, &keyed);
        assert!(!out.self_decal && out.member_decal_ids.is_empty());
        // not simple → member decals over both walks, drawables only:
        let out = horde_contain_set_member_decal_state(true, true, false, true, &contained, &keyed);
        assert_eq!(out.member_decal_ids, vec![1, 3]);
        // not locally owned → neither switch nor member walk:
        let out = horde_contain_set_member_decal_state(true, false, false, true, &contained, &keyed);
        assert!(!out.use_member_decals && out.member_decal_ids.is_empty());
    }

    // -- 0x00986083 ---------------------------------------------------------

    fn mm(
        id: u32,
        ai: bool,
        mode: i32,
        engaged: bool,
        status1c: bool,
        blocked: bool,
        pos: Coord3D,
    ) -> MeleeMember {
        MeleeMember {
            id,
            ai_present: ai,
            ai_mode_1fc: mode,
            ai_engaged_140: engaged,
            formation_status_1c: status1c,
            idle_blocked: blocked,
            position: pos,
        }
    }

    #[test]
    fn melee_armed_slots_and_bounds() {
        let mut active = false;
        let mut attacks = vec![
            FormationAttackEntry {
                position: Coord3D::default(),
                needs_position: false,
                path: vec![c(1.0, 1.0, 1.0)],
            },
            FormationAttackEntry {
                position: Coord3D::default(),
                needs_position: false,
                path: vec![],
            },
        ];
        let members = [
            mm(1, true, 4, true, false, false, c(1.0, 0.0, 0.0)), // engaged → skip
            mm(2, true, 3, false, false, true, c(2.0, 0.0, 0.0)), // idle-blocked: no cmd
            mm(3, true, 0, false, false, false, c(3.0, 0.0, 0.0)), // idle cmd fires
            mm(4, true, 0, false, false, false, c(4.0, 0.0, 0.0)), // index out of range
        ];
        let out = horde_melee_formation_start_melee_attack(&mut active, &mut attacks, &members, &|id| match id {
            1 | 2 => 0,
            3 => 1,
            _ => 2,
        });
        assert!(active);
        assert_eq!(out.idle_commands, vec![3]);
        assert_eq!(
            out.assignments,
            vec![
                MeleeAttackAssignment {
                    object_id: 2,
                    entry_index: 0,
                    position: c(2.0, 0.0, 0.0)
                },
                MeleeAttackAssignment {
                    object_id: 3,
                    entry_index: 1,
                    position: c(3.0, 0.0, 0.0)
                }
            ]
        );
        assert!(attacks[0].needs_position);
        assert!(attacks[0].path.is_empty());
        assert_eq!(attacks[0].position, c(2.0, 0.0, 0.0));
        // index -1 also out of range (the `uVar3 < 0` test):
        let out = horde_melee_formation_start_melee_attack(
            &mut active,
            &mut attacks,
            &[mm(9, true, 0, false, true, false, c(0.0, 0.0, 0.0))],
            &|_| -1,
        );
        assert!(out.assignments.is_empty());
    }

    #[test]
    fn melee_ailess_member_with_status_skips_then_panics_on_reset() {
        // status clear + no AI → whole member skipped, no panic:
        let mut active = false;
        let mut attacks = vec![FormationAttackEntry {
            position: Coord3D::default(),
            needs_position: false,
            path: vec![],
        }];
        let out = horde_melee_formation_start_melee_attack(
            &mut active,
            &mut attacks,
            &[mm(1, false, 0, false, false, false, c(0.0, 0.0, 0.0))],
            &|_| 0,
        );
        assert!(out.assignments.is_empty());
    }

    #[test]
    #[should_panic]
    fn melee_status_set_with_no_ai_faults_on_reset_attack() {
        let mut active = false;
        let mut attacks = vec![FormationAttackEntry {
            position: Coord3D::default(),
            needs_position: false,
            path: vec![],
        }];
        horde_melee_formation_start_melee_attack(
            &mut active,
            &mut attacks,
            &[mm(1, false, 0, false, true, false, c(0.0, 0.0, 0.0))],
            &|_| 0,
        );
    }

    // -- 0x0086c4c0 ---------------------------------------------------------

    #[test]
    fn slot55_stores_query_and_clears_only_movers_idle() {
        let mut contained = [Slot55Member {
            id: 1,
            present: true,
            accepts_290fbb: true,
            ai_is_moving: false,
            condition_61: true,
        }];
        let mut keyed = [
            None,
            Some(Slot55Member {
                id: 2,
                present: true,
                accepts_290fbb: true,
                ai_is_moving: true,
                condition_61: true,
            }),
            Some(Slot55Member {
                id: 3,
                present: true,
                accepts_290fbb: false,
                ai_is_moving: false,
                condition_61: true,
            }),
        ];
        let out = horde_contain_clear_condition61_and_hand_off(-12345, &mut contained, &mut keyed);
        assert_eq!(out.m_19c, -12345);
        assert_eq!(out.cleared_ids, vec![1]);
        assert_eq!(out.hand_off_ids, vec![1, 2]); // id 3 not accepted
        assert!(!contained[0].condition_61);
        // moving member keeps the flag but is still handed off:
        assert!(keyed[1].as_ref().unwrap().condition_61);
    }

    #[test]
    fn slot55_null_entries_advance_past() {
        let mut contained = [Slot55Member {
            id: 0,
            present: false,
            accepts_290fbb: true,
            ai_is_moving: false,
            condition_61: true,
        }];
        let out = horde_contain_clear_condition61_and_hand_off(0, &mut contained, &mut []);
        assert!(out.cleared_ids.is_empty() && out.hand_off_ids.is_empty());
    }

    /// Feed a fixed token stream to the parser: both `getNextTokenOrNull`
    /// and `getNextToken` pull the next entry (shared `Cell` cursor).
    fn parse_with(
        tokens: &[&str],
    ) -> Result<HordeContainSplitEntry, SplitParseError> {
        let i = std::cell::Cell::new(0usize);
        let mut a = || -> Option<String> {
            let k = i.get();
            i.set(k + 1);
            tokens.get(k).map(|s| s.to_string())
        };
        let mut b = || -> String {
            let k = i.get();
            i.set(k + 1);
            tokens.get(k).copied().unwrap_or("").to_string()
        };
        parse_horde_contain_split_result(&mut a, &mut b)
    }
    // -- 0x0086f288 ---------------------------------------------------------


    #[test]
    fn split_parse_full_entry() {
        // stream: label value label value label value
        let e = parse_with(&["SplitResult", "RESULT_A", "UnitType", "UNIT_B", "RankNumber", "42"])
            .unwrap();
        assert_eq!(
            e,
            HordeContainSplitEntry {
                split_result: "RESULT_A".into(),
                unit_type: "UNIT_B".into(),
                rank_number: 42,
            }
        );
    }

    #[test]
    fn split_parse_optional_rank_and_errors() {
        // a trailing label that is not "RankNumber" → rank 0 (the peeked
        // token is consumed, the value never read):
        let e = parse_with(&["SplitResult", "R", "UnitType", "U", "", "0"]).unwrap();
        assert_eq!(e.rank_number, 0); // token "" != "RankNumber"

        // missing first token → TargetExpected:
        assert_eq!(parse_with(&[]), Err(SplitParseError::TargetExpected));
        // wrong first token → TargetExpected:
        assert_eq!(parse_with(&["Target"]), Err(SplitParseError::TargetExpected));
        // wrong second label → ResultExpected:
        assert_eq!(
            parse_with(&["SplitResult", "R", "Result", "U"]),
            Err(SplitParseError::ResultExpected)
        );
        // RankNumber pair present but non-numeric value → atoi = 0:
        let e = parse_with(&["SplitResult", "R", "UnitType", "U", "RankNumber", "x"]).unwrap();
        assert_eq!(e.rank_number, 0);
    }

    #[test]
    fn atoi_semantics_pinned() {
        assert_eq!(c_atoi("42"), 42);
        assert_eq!(c_atoi("  -17"), -17);
        assert_eq!(c_atoi("+3abc"), 3); // stops at first non-digit
        assert_eq!(c_atoi("abc"), 0); // no digits
        assert_eq!(c_atoi(""), 0);
        assert_eq!(c_atoi("9999999999999999"), i64::from(i32::MAX) as i32); // saturates then truncates
        assert_eq!(c_atoi(" \t\n8"), 8);
    }

    // -- 0x0086ecde ---------------------------------------------------------

    #[test]
    fn portal_destination_averaging_is_layer_gated() {
        let self_pos = c(10.0, 20.0, 30.0);
        let a2 = c(1.0, 2.0, 3.0);
        let ms = [PortalMember {
            id: 5,
            present: true,
            slot7_position: c(0.5, 0.5, 0.5),
            slot7_angle: 0.25,
        }];
        // layer == LAYER_GROUND (1): destination is the self position.
        let got = horde_contain_send_members_through_portal(self_pos, a2, 1, &ms);
        assert_eq!(got[0].destination, self_pos);
        // any other layer: (a2 + self) * 0.5 per component.
        let got = horde_contain_send_members_through_portal(self_pos, a2, 0, &ms);
        assert_eq!(got[0].destination, c(5.5, 11.0, 16.5));
        assert_eq!(got[0].fixup_position, c(0.5, 0.5, 0.5));
        assert_eq!(got[0].fixup_angle, 0.25);
        // null members skipped but walk continues:
        let ms2 = [
            PortalMember {
                id: 0,
                present: false,
                slot7_position: Coord3D::default(),
                slot7_angle: 0.0,
            },
            ms[0],
        ];
        let got = horde_contain_send_members_through_portal(self_pos, a2, 2, &ms2);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].object_id, 5);
    }

    // -- 0x008671ac ---------------------------------------------------------

    #[test]
    fn rider_exit_gate_matrix() {
        // null rider → TRUE without touching any query:
        assert!(transport_contain_is_specific_rider_free_to_exit(
            false,
            true,
            7,
            false,
            false,
            false,
            &|| panic!("no queries for a null rider"),
            &|| panic!(),
            &|| panic!(),
        ));
        // AI says not free (non-zero) → FALSE, airborne not asked:
        assert!(!transport_contain_is_specific_rider_free_to_exit(
            true,
            true,
            1,
            true,
            true,
            false,
            &|| panic!("airborne query never reached"),
            &|| panic!(),
            &|| panic!(),
        ));
        // AI absent → the gate passes (only consulted when present):
        assert!(transport_contain_is_specific_rider_free_to_exit(
            true, false, 0, true, true, false,
            &|| true, &|| false, &|| false,
        ));
        // airborne → TRUE without any locomotor/terrain work:
        assert!(transport_contain_is_specific_rider_free_to_exit(
            true, true, 0, true, false, false,
            &|| panic!("terrain never consulted when airborne"),
            &|| panic!(), &|| panic!(),
        ));
        // rider without AI/locomotor → FALSE:
        assert!(!transport_contain_is_specific_rider_free_to_exit(
            true, false, 0, false, false, true,
            &|| panic!("not BF-branch when rider lacks a locomotor"),
            &|| panic!(), &|| panic!(),
        ));
        // BF + underwater → closest-land decides; underwater query fires,
        // valid-movement never:
        assert!(transport_contain_is_specific_rider_free_to_exit(
            true, false, 0, false, true, true,
            &|| true, &|| true, &|| panic!("valid-terrain not consulted underwater"),
        ));
        assert!(!transport_contain_is_specific_rider_free_to_exit(
            true, false, 0, false, true, true,
            &|| true, &|| false, &|| panic!(),
        ));
        // BF but NOT underwater → valid-terrain decides:
        assert!(transport_contain_is_specific_rider_free_to_exit(
            true, false, 0, false, true, true,
            &|| false, &|| panic!(), &|| true,
        ));
        // not BF → valid-terrain directly:
        assert!(!transport_contain_is_specific_rider_free_to_exit(
            true, false, 0, false, true, false,
            &|| panic!(), &|| panic!(), &|| false,
        ));
    }
}
