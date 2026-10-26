//! Wave 47: GameLogic/Pathfinder band, module tag `Pathfinder` — 16 rows
//! (`0x006e7…0x00932`). Cross-module callees that live in other ledger
//! rows are modelled as traits / closures (the `dxstate.rs` hook idiom);
//! every address, offset and quirk below is quoted from our decompiled
//! bodies. Reference-tree facts are limited to member offsets, constants
//! and enum values, marked `// <Class>::<member> @ +0xNN`.
//!
//! | addr | size | their symbol | one-line role |
//! |---|---|---|---|
//! | `0x006f3f7d` | 362 | PathfinderMoveAllies cellCallback | occupant scan: command an idle/overlapping ally AI to move away |
//! | `0x006ecc0d` | 342 | Pathfinder slot (pathfind query) | footprint fit probe: every cell exists, mover accepts, then movement + cost checks |
//! | `0x00860cba` | 288 | DynamicPortalBehaviour footprint/range query | axis-unit vector → max-axis-normalised module offset, object-transformed |
//! | `0x006ec9e1` | 286 | Pathfinder::GetOverlapGoalUnits | collect up to 16 occupant IDs over the unit's footprint (1–2 layers) |
//! | `0x006ecaff` | 270 | Pathfinder::CountOverlapHordeGoalUnits | score horde-goal occupants over footprint: same container +1, other +2 |
//! | `0x00766cea` | 258 | PathfindLayer::ConnectsZones | scan layer cells, map ground-cell zones through the manager, need both zones seen |
//! | `0x00932708` | 251 | PathfindZoneManager::CreateEquivalencySet | reset one union-find, MakeSet every zone, link adjacent zone pairs passing the masks |
//! | `0x006fa202` | 218 | Pathfinder::AdjustHordeMeleeDestination | spiral-search an acceptable cell around dest, else the functor's fallback cell |
//! | `0x006edcf7` | 178 | Pathfinder::AdjustToNearestGroundCell | ground-cell circular search; rewrite pos to found cell centre + terrain height |
//! | `0x006edda9` | 178 | Pathfinder::AdjustToNearestValidCell | twin of the above with the valid-cell scan / budget `+0x1208` |
//! | `0x006e7e26` | 176 | Pathfinder rva query (`0x002E7E26` visitor) | convex-poly/rect cell visitor: keep nearest clear cell to `source` |
//! | `0x00765df0` | 168 | Path::SpliceAndPrepend | advance to tail, reclaim nodes, splice the supplied path in front, delete it |
//! | `0x006e7875` | 162 | WorldToCell | world → cell coords (`floor(x*0.1 [+0.5])` + x87 `fistp`) |
//! | `0x006e7a62` | 160 | ComputeNormalRadialOffset | offset a point off an object's radius along the segment normal |
//! | `0x006e7296` | 158 | Pathfinder::IsValidMovementTerrain | cell-type / locomotor-surface validity, `CONCAT31` byte+tail result |
//! | `0x006ed0cc` | 142 | line-contexts init (`0x002ED15AInfo::init`) | fill the 0x5C-byte line-walk context and mirror its 16B query block |
//!
//! Faithful-quirk notes pinned by tests:
//! - `0x006e7875` computes in f32 (`DAT_00bc2424 = 0.1f`,
//!   `DAT_00bc26f0 = 0.5f`), widens the f32 product/sum to double only for
//!   `floor`, then `(int)ROUND((float)d)` — an x87 `fistp` store: round to
//!   nearest-even, out-of-range/NaN writes `0x80000000`.
//! - `0x006e7e26` computes `delta.z` as the literal self-subtraction
//!   `source.z - source.z`: a NaN (or infinite) `source.z` makes every
//!   distance NaN, so *no* cell ever updates the best record.
//! - `0x006e7296` returns the full Ghidra `CONCAT31((int3)(w >> 8), bit)`
//!   dword, not just the byte: the upper bits come from the cell flags
//!   (raw or `& 0x3f0`) or from the locomotor surface mask.
//! - `0x006ecc0d`'s movement record is built from the two dwords at
//!   `ai + 0x1cc`; with `ai == 0` the game forms the wild address `0x1cc`
//!   (UB) — the mirror requires the caller to supply the seed dwords.
//! - `0x00765df0` calls the pooled-node free (`0x00665488`) and the
//!   `Path` destroy pair (`0x00764a89` + `0x0042fd60`) through hooks; the
//!   `m_selected` dereference and arena lookups mirror the raw loads
//!   (missing handle = the game's wild read → panic).

/// `Coord3D` (three f32, IEEE throughout).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PfVec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// `ICoord2D` (two ints).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PfCellPt {
    pub x: i32,
    pub y: i32,
}

/// `DAT_00bc2424` — `PATHFINDER_CELL_INV` = 1/10 (float; the cell is 10
/// world units wide, per the reference's `PATHFINDING_CELL_WIDTH = 10.0f`).
pub const PF_CELL_INV: f32 = 0.1;

/// `DAT_00bc26f0` — the `+0.5f` cell-centre bias.
pub const PF_CELL_HALF: f32 = 0.5;

/// `DAT_00bbb8d8` — `1.0f` (pinned from the image in `math.rs`).
pub const PF_ONE: f32 = 1.0;

/// The `(int)ROUND((float)d)` idiom: an x87 `fistp` integer store.
/// Rounds half-to-even; NaN or anything outside `[-2^31, 2^31)` stores
/// the x87 invalid-integer value `0x80000000` (`i32::MIN`). The decomp's
/// inputs are always `floor` results, so the rounding is normally exact;
/// the edge behaviour is pinned by tests anyway.
pub fn x87_fistp_i32(v: f32) -> i32 {
    let v = v as f64;
    let f = v.floor();
    let d = v - f;
    let r = if d < 0.5 {
        f
    } else if d > 0.5 {
        f + 1.0
    } else if f % 2.0 == 0.0 {
        f
    } else {
        f + 1.0
    };
    if r >= -2147483648.0 && r < 2147483648.0 {
        r as i32
    } else {
        // x87 integer conversion error stores 0x80000000 (also NaN)
        i32::MIN
    }
}

// ---------------------------------------------------------------------------
// 0x006e7875 — WorldToCell
// ---------------------------------------------------------------------------

/// `0x006e7875` — `WorldToCell(ICoord2D *out, char center, const Coord3D *pos)`.
///
/// Body (verbatim order): when `center == 0` each axis is
/// `floor(x * 0.1f + 0.5f)` (cell-centre bias), otherwise plain
/// `floor(x * 0.1f)`. The multiply/add run in f32, the widening to double
/// happens only for `floor`, and the int conversion is `(int)ROUND((float)d)`
/// (x87 `fistp`, see [`x87_fistp_i32`]). The game computes the Y cast first
/// (`local_8`), stores X then Y; the game returns `out` in EAX.
pub fn world_to_cell<'a>(out: &'a mut PfCellPt, center: bool, pos: &PfVec3) -> &'a mut PfCellPt {
    let (tx, ty) = if !center {
        (
            f64::from(pos.x * PF_CELL_INV + PF_CELL_HALF),
            f64::from(pos.y * PF_CELL_INV + PF_CELL_HALF),
        )
    } else {
        (f64::from(pos.x * PF_CELL_INV), f64::from(pos.y * PF_CELL_INV))
    };
    // The game `floor`s the widened value before casting, so the
    // `(int)ROUND((float)d)` round-trip is exact. The Y cast (`local_8`)
    // is evaluated before the X store.
    let iy = x87_fistp_i32(ty.floor() as f32);
    out.x = x87_fistp_i32(tx.floor() as f32);
    out.y = iy;
    out
}

// ---------------------------------------------------------------------------
// 0x006e7296 — Pathfinder::IsValidMovementTerrain
// ---------------------------------------------------------------------------

/// A pathfind cell as seen by the rows here: only `m_flags @ +0x0C`
/// (`type = flags & 0xf`, `layer = (flags >> 4) & 0x3f`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PfCellFlags {
    pub flags: u32, // +0x0C
}

/// `0x006e7296` — `Pathfinder::IsValidMovementTerrain(layer, locomotor, pos)`.
///
/// Cell coords use the plain floor conversion (no `+0.5`). A null cell
/// returns `0`; cell type 4 (`CELL_OBSTACLE`) or 5 (`CELL_IMPASSABLE`)
/// return true; a non-ground cell (`(flags & 0x3f0) != 0x10`, LAYER_GROUND
/// = 1) of type 0 (`CELL_CLEAR`) returns true; otherwise the locomotor's
/// legal-surface mask (`locomotor->m_moduleData @ +0x14`) is ANDed with the
/// per-type table `DAT_00dbd33c[type]`.
///
/// The result is the full Ghidra `CONCAT31((int3)(w >> 8), bit)` dword:
/// `w` is the raw flags (types 4/5), the masked `flags & 0x3f0` (layer
/// path), or the legal-surfaces dword (table path) — only the low byte is
/// the bool; the upper bits are live garbage tails, mirrored verbatim.
pub fn pathfinder_is_valid_movement_terrain<'a>(
    layer: u32,
    pos: &PfVec3,
    legal_surfaces: u32, // Locomotor module data `m_legalSurfaces` @ +0x14
    surfaces_by_type: &[u32; 16], // DAT_00dbd33c (index = cell type 0..15)
    get_cell: impl FnOnce(u32, i32, i32) -> Option<&'a PfCellFlags>, // 0x006e6d62
) -> u32 {
    let x = x87_fistp_i32(f64::from(pos.x * PF_CELL_INV) as f32);
    let y = x87_fistp_i32(f64::from(pos.y * PF_CELL_INV) as f32);
    let cell = match get_cell(layer, x, y) {
        Some(c) => c,
        None => return 0,
    };
    let flags = cell.flags;
    let kind = flags & 0xf;
    if kind == 4 || kind == 5 {
        return ((flags >> 8) << 8) | 1;
    }
    let w3 = flags & 0x3f0;
    if w3 != 0x10 && kind == 0 {
        return ((w3 >> 8) << 8) | 1;
    }
    let legal = legal_surfaces;
    let bit = (surfaces_by_type[kind as usize] & legal) != 0;
    ((legal >> 8) << 8) | bit as u32
}

// ---------------------------------------------------------------------------
// 0x006e7a62 — ComputeNormalRadialOffset
// ---------------------------------------------------------------------------

/// The object view the helper reads: `position @ +0x38`
/// (`radius @ +0xB8` is folded into the `radius` argument by the callers).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PfObjectPos {
    pub position: PfVec3, // Object::m_pos @ +0x38
}

/// `0x006e7a62` — `ComputeNormalRadialOffset(from, insert, to, obj, radius)`.
///
/// Our body keeps the deltas as `local_c = from.x - to.x`,
/// `local_10 = from.y - to.y` (the negated segment) and tests
/// `(obj.y - to.y) * local_c - (obj.x - to.x) * local_10 <= 0.0` — the
/// segment/object cross product negated (with `to` as apex; identical sign
/// to `(to - from) x (obj - from)`). `<= 0.0` is false for NaN, which
/// selects the else branch (pinned by test). Negating `local_10` gives
/// normal `(dy, -dx)` on a positive cross, `( -dy, dx)` on a negative one.
/// The normal `(x, y, 0)` is normalised (`Coord3D::normalize`: skip when
/// `len == 0.0`; NaN len still scales — game behaviour), then
/// `insert = obj.position + normal * radius` (Z comes from the object).
pub fn compute_normal_radial_offset(
    from: &PfVec3,
    insert: &mut PfVec3,
    to: &PfVec3,
    obj: &PfObjectPos,
    radius: f32,
) {
    let mut nx = from.x - to.x; // local_c
    let mut ny = from.y - to.y; // local_10
    let test = (obj.position.y - to.y) * nx - (obj.position.x - to.x) * ny;
    if test <= 0.0 {
        ny = -ny;
    } else {
        nx = -nx;
    }
    let nz = 0.0f32; // local_8
    let len = (nx * nx + ny * ny + nz * nz).sqrt();
    if len != 0.0 {
        let scale = 1.0 / len;
        nx *= scale;
        ny *= scale;
    }
    insert.x = nx * radius + obj.position.x;
    insert.y = ny * radius + obj.position.y;
    insert.z = obj.position.z;
}

// ---------------------------------------------------------------------------
// 0x006e7e26 — Pathfinder rva query (Rva002E7E26 cell visitor)
// ---------------------------------------------------------------------------

/// The visitor context (`0x002E7E26` class): owner handle, the source
/// point, the best cell found so far, its squared cost, and the found flag.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BestCellVisitor {
    pub owner: i32,      // +0x00
    pub source: PfVec3,  // +0x04..+0x0F (z @ +0x0C)
    pub closest: PfVec3, // +0x10..+0x1B
    pub cost: f32,       // +0x1C initial/best squared cost
    pub found: bool,     // +0x20
}

/// `0x006e7e26` — the rectangle/convex-poly visitor
/// `rva002E7E26::rva002E7E26(Object *cellWord, int x, int y)`.
///
/// `kind_word` is the cell dword at `cellWord + 0xC` (`LandWordView::packed`);
/// only kind (`& 0xf`) matters: 7 (`LandSolid`), 1 (`LandWater`) and any
/// other non-zero kind are rejected (tests evaluated in that literal
/// order). The candidate world point is `(x*10, y*10, source.z)` with the
/// int multiply wrapping first. The squared delta keeps the body's literal
/// `z - z` self-subtraction and the `(dz² + dy²) + dx²` add order. On
/// `dist < cost` the best record is rewritten; the callback returns the
/// constant `0` (the game's `AL`, continuation signal), mirrored as false.
pub fn pathfinder_best_cell_visit(
    v: &mut BestCellVisitor,
    kind_word: u32,
    x: i32,
    y: i32,
) -> bool {
    let kind = kind_word & 0xf;
    if kind != 7 && kind != 1 && kind == 0 {
        let z = v.source.z;
        let px = (x.wrapping_mul(10)) as f32;
        let py = (y.wrapping_mul(10)) as f32;
        let dx = v.source.x - px;
        let dy = v.source.y - py;
        let dz = z - z; // literal `(fVar1 - fVar1)` — NaN source.z poisons dist
        let dist = dz * dz + dy * dy + dx * dx;
        if dist < v.cost {
            v.closest.x = px;
            v.closest.y = py;
            v.closest.z = z;
            v.cost = dist;
            v.found = true;
        }
    }
    false
}

// ---------------------------------------------------------------------------
// 0x006f3f7d — PathfinderMoveAllies cellCallback
// ---------------------------------------------------------------------------

/// `Relationship` enum (reference: ENEMIES 0, NEUTRAL 1, ALLIES 2).
pub const RELATIONSHIP_ALLIES: i32 = 2;

/// An `Object` as the callback observes it: `m_ID @ +0x74`,
/// `m_radius @ +0xB8`, `m_containedBy @ +0x274`, `m_ai @ +0x258`.
#[derive(Debug)]
pub struct MoveAlliesUnit<'a> {
    pub id: i32,
    pub radius: f32,
    pub contained_by: Option<&'a MoveAlliesUnit<'a>>,
    pub ai: Option<&'a MoveAlliesAi>,
}

/// `AIUpdateInterface` view: the body reads the state machine at
/// `ai + 0x30` and inlines its `m_goal @ +0x24` (null machine = the game's
/// wild read — the caller must supply a real goal). The two predicate rows
/// (`0x00664688` `isMoving`, `0x00662dd3(mover)`) are precomputed booleans
/// (fxflags-style resolved flags).
#[derive(Debug)]
pub struct MoveAlliesAi {
    pub is_moving: bool,            // 0x00664688()
    pub goal_toward_mover: bool,    // 0x00662dd3(m_obj) != 0
    pub goal: PfVec3,               // *(ai + 0x30) machine -> goal @ +0x24
}

/// Singly linked collision-list node: `next @ +0x00`, `object @ +0x08`.
#[derive(Debug)]
pub struct OccupantNode<'a, T> {
    pub next: Option<&'a OccupantNode<'a, T>>,
    pub object: Option<&'a T>,
}

/// The visited `PathfindCell`: `m_info @ +0x00`; the occupants list head is
/// `info + 0x20` (null `CellInfo` → immediate return, exactly as the body).
#[derive(Debug)]
pub struct AlliesScanCell<'a> {
    pub info: Option<&'a AlliesCellInfo<'a>>,
}

#[derive(Debug)]
pub struct AlliesCellInfo<'a> {
    pub occupants: Option<&'a OccupantNode<'a, MoveAlliesUnit<'a>>>, // CellInfo::m_occupants @ +0x20
}

/// The cross-module queries the callback performs per candidate, in the
/// body's literal evaluation order.
pub trait MoveAlliesScan {
    /// `0x0068d156` `Object::getRelationship(other)` on the mover;
    /// `RELATIONSHIP_ALLIES` passes.
    fn relationship_to_mover(&self, candidate: &MoveAlliesUnit) -> i32;
    /// The horde-exclusion chain: `0x00449dc5` `findObjectByID(ignore_id)`,
    /// template flag byte `+0x115 & 0x20`, contain pointer `+0x250`,
    /// `getHordeContain` (contain vtable `+0x7C`) and `contains(other)`
    /// (horde-view vtable `+0x18`). True = the chain fully resolves and the
    /// horde contains `other` → candidate skipped.
    fn horde_excluded(&self, candidate: &MoveAlliesUnit) -> bool;
    /// `0x006931ba` `Object::rva002931BA()` — "resolve through the container".
    fn flag_931ba(&self, candidate: &MoveAlliesUnit) -> bool;
    /// `0x0066c411` — `AICommandInterface::aiMoveAwayFromUnit(mover,
    /// destination, CMD_FROM_AI = 2)` on `ai + 0x20`.
    fn ai_move_away_from_unit(
        &mut self,
        ai: &MoveAlliesAi,
        mover: &MoveAlliesUnit,
        destination: &PfVec3,
    );
}

/// `0x006f3f7d` — the move-allies cell callback
/// (`cellCallback(previous, current, x, y)`; only `current` is read).
///
/// Null cell info returns at once. For every occupant: skip self
/// (pointer identity), the `ignore_id` match on `Object::m_ID @ +0x74`
/// (a null candidate is the game's wild `*(0 + 0x74)` read → panic here),
/// non-ALLIES relationship, and the horde-exclusion chain. Then a
/// `rva002931BA`-flagged unit is replaced by its `m_containedBy @ +0x274`;
/// with a resolved unit *and* its AI (`+0x258`) alive: a unit that is not
/// moving — or is moving but `rva00262dd3(mover)` is false and its AI goal
/// sits within the mover's `m_radius @ +0xB8` of the destination
/// (literal `(dz² + dy²) + dx² < r²`) — gets `aiMoveAwayFromUnit` and the
/// scan stops. The game's return is the constant 0.
pub fn pathfinder_move_allies_cell_callback<S: MoveAlliesScan + ?Sized>(
    mover: &MoveAlliesUnit,
    ignore_id: i32,
    destination: &PfVec3,
    cell: Option<&AlliesScanCell>,
    scan: &mut S,
) {
    let cell = match cell {
        Some(c) => c,
        None => return, // `*param_3 == 0` -> null list
    };
    let info = match cell.info {
        Some(i) => i,
        None => return,
    };
    let mut node = info.occupants;
    loop {
        let node_ref = match node {
            Some(n) => n,
            None => return,
        };
        let mut candidate = match node_ref.object {
            Some(o) => o,
            None => panic!("move-allies callback: null occupant object (game wild-reads *(0 + 0x74))"),
        };
        if candidate.id != ignore_id
            && scan.relationship_to_mover(candidate) == RELATIONSHIP_ALLIES
            && !scan.horde_excluded(candidate)
        {
            if scan.flag_931ba(candidate) {
                if let Some(parent) = candidate.contained_by {
                    candidate = parent;
                }
            }
            if let Some(ai) = candidate.ai {
                let mut moving = ai.is_moving;
                if moving {
                    let goal = &ai.goal;
                    let dx = goal.x - destination.x;
                    let dy = goal.y - destination.y;
                    let dz = goal.z - destination.z;
                    if !ai.goal_toward_mover
                        && dz * dz + dy * dy + dx * dx < mover.radius * mover.radius
                    {
                        moving = false;
                    }
                }
                if !moving {
                    scan.ai_move_away_from_unit(ai, mover, destination);
                    return;
                }
            }
        }
        node = node_ref.next;
    }
}

// ---------------------------------------------------------------------------
// 0x006ecc0d — Pathfinder slot (pathfind query / position fit)
// ---------------------------------------------------------------------------

/// A cell as this row sees it: it is only null-checked and handed to the
/// mover-acceptance row `0x006e6dc4`.
#[derive(Clone, Copy, Debug)]
pub struct FitCell;

/// The 8-byte `Rva002E8BCF` movement probe built by `0x006e8bcf`
/// (the `TCheckMovementInfo::m_1C` block). The two seed dwords are the
/// raw pair at `ai + 0x1cc`; with `ai == 0` the game forms address
/// `0x1cc` itself (UB) — supplying a seed is the caller's obligation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MovementProbe {
    pub seed0: i32,              // *(ai + 0x1CC)
    pub seed1: i32,              // *(ai + 0x1D0)
    pub ignore_template_flag: bool, // (template byte +0x634 == 0)
    pub priority_minus_1: i32,      // template +0x56C - 1
    pub afbb: bool,                 // 0x0068afbb()
}

/// The on-stack `TCheckMovementInfo` this row assembles before the two
/// big callees: `m_00` cell @ +0x00, layer @ +0x08, radius @ +0x0C,
/// `centerInCell` @ +0x10 (byte; the `m_11 = false` store is the other
/// half of the body's 2-byte `local_40` rewrite), option bits @ +0x14,
/// ignore object id @ +0x18. The row `0x006e6fdf` constructor ran first
/// (`Default` models it).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CheckMovementQuery {
    pub cell: PfCellPt,
    pub layer: i32,
    pub radius: i32,
    pub center_in_cell: bool, // +0x10
    pub m_11: bool,           // +0x11 — forced false by this row
    pub m_14: u32,            // +0x14
    pub m_18: i32,            // +0x18
}

/// `0x006ecc0d` — `Pathfinder::rva002ECC0D(obj, pos)` (the "does this
/// footprint fit here" pathfind query).
///
/// `radius`/`center_in_cell` come from `0x006ebca7` (unit split), `layer`
/// from `0x0068b511`, `priority` from `template + 0x56C`, `flag_634` from
/// the template byte `+0x634`, `afbb` from `0x0068afbb`,
/// `kind_bit_110_26` from `*(template + 0x110) & 0x4000000` (the
/// `testKindWord` probe: set -> `m_14 = 1`, clear -> `16`; the body spells
/// it `(-(bit) & 0xfffffff1) + 0x10`), and `ai_object_id` from
/// `0x0046e009` with `0` when `ai (obj + 0x258) == 0`.
///
/// The footprint grid walks `i, j in [-radius, radius)` (a non-positive
/// radius simply runs zero iterations — the body's `-radius < radius`
/// guard); every cell must exist (null -> false) and pass the mover probe
/// `0x006e6dc4` (false -> false). Then `0x006ea34a`
/// (`checkForMovement`) and `0x006ec0a2` (`CalcCollisionFreeExtraCosts`,
/// full C argument list; the result passes when it compares `== 0`).
#[allow(clippy::too_many_arguments)]
pub fn pathfinder_position_fit_check(
    obj: i32,
    pos: &PfVec3,
    center_in_cell: bool,
    radius: i32,
    layer: u32,
    priority: i32,
    flag_634: u8,
    afbb: bool,
    kind_bit_110_26: bool,
    ai_object_id: i32,
    movement_seed: [i32; 2],
    get_cell: impl FnMut(u32, i32, i32) -> Option<FitCell>, // 0x006e6d62
    movement_ok: impl FnMut(&MovementProbe, &FitCell) -> bool, // 0x006e6dc4
    check_for_movement: impl FnOnce(i32, &CheckMovementQuery) -> bool, // 0x006ea34a
    extra_costs: impl FnOnce(i32, i32, &CheckMovementQuery, i32, i32, i32, i32) -> i32, // 0x006ec0a2
) -> bool {
    let mut cell = PfCellPt::default();
    world_to_cell(&mut cell, center_in_cell, pos);
    let probe = MovementProbe {
        seed0: movement_seed[0],
        seed1: movement_seed[1],
        ignore_template_flag: flag_634 == 0,
        priority_minus_1: priority - 1,
        afbb,
    };
    {
        let mut get_cell = get_cell;
        let mut movement_ok = movement_ok;
        for i in -radius..radius {
            for j in -radius..radius {
                let c = match get_cell(layer, cell.x + i, cell.y + j) {
                    Some(c) => c,
                    None => return false,
                };
                if !movement_ok(&probe, &c) {
                    return false;
                }
            }
        }
    }
    let mut info = CheckMovementQuery {
        cell,
        layer: layer as i32,
        radius,
        center_in_cell,
        m_11: false,
        m_14: if kind_bit_110_26 { 1 } else { 16 },
        m_18: ai_object_id,
    };
    if !check_for_movement(obj, &info) {
        return false;
    }
    let center_plus = if info.center_in_cell { 1 } else { 0 };
    let query_layer = info.layer; // by-value arg: the game reads it pre-call
    extra_costs(obj, 0, &mut info, query_layer, radius, radius + center_plus, 0) == 0
}

// ---------------------------------------------------------------------------
// 0x006ec9e1 — Pathfinder::GetOverlapGoalUnits
// ---------------------------------------------------------------------------

/// A goal-list occupant: the id (`Object::m_ID @ +0x74`) is copied to the
/// output; the flag is the precomputed predicate `0x006e6b89` — a set flag
/// skips the occupant.
#[derive(Debug)]
pub struct GoalUnit {
    pub id: i32,
    pub skip: bool, // 0x006e6b89()
}

/// The collision cell this row walks: null cell or null `m_info` skips the
/// slot; the goal-node list head is `info + 0x14`.
#[derive(Debug)]
pub struct GoalCell<'a> {
    pub info: Option<&'a GoalCellInfo<'a>>,
}

#[derive(Debug)]
pub struct GoalCellInfo<'a> {
    pub occupants: Option<&'a OccupantNode<'a, GoalUnit>>, // info + 0x14
}

/// `0x006ec9e1` — `Pathfinder::GetOverlapGoalUnits(object, position, out)`.
///
/// The cell base comes from `world_to_cell` with the unit's odd-parity
/// centre flag (`0x006ebc14`); the half/rest split of the footprint
/// diameter (`0x006ebcd6`) is `half = floor(d / 2)` (the body's
/// `cdq/sub/sar` pair — mirrored as the arithmetic shift `d >> 1`, exact
/// for negatives too) and `rest = d - half`. The layer list is
/// `[layer, 1]` with `1` (LAYER_GROUND) appended only when the layer is
/// not ground-or-bridge (`0x006e6e6c`) *and* the terrain bridge query
/// (the `DAT_00dfec50` vtable slot `+0xB0` call) passes. The scan counter
/// `DAT_00dfecd0` is incremented, the mover's own `0x006e6b89` predicate
/// is invoked and discarded (kept literally), and the x-outer/y-outer/k-
/// inner loops collect `GoalUnit::id`s, returning `16` as soon as the
/// output is full.
pub fn pathfinder_get_overlap_goal_units<'a, F, G>(
    object_layer: u32,       // 0x0068b511
    center_flag: bool,       // 0x006ebbfb IsOdd
    bridge_interacts: bool,  // DAT_00dfec50 vtable +0xB0 chain
    diameter: i32,           // 0x006e9b31 (diameter in cells)
    position: &PfVec3,
    scan_counter: &mut u32,  // DAT_00dfecd0
    mover_skip: bool,        // 0x006e6b89(mover) — read and discarded
    is_ground_or_bridge: F,  // 0x006e6e6c
    get_cell: G,             // 0x006e6d62
    out: &mut [i32; 16],
) -> i32
where
    F: FnOnce(u32) -> bool,
    G: FnMut(u32, i32, i32) -> Option<&'a GoalCell<'a>>,
{
    let mut cell = PfCellPt::default();
    world_to_cell(&mut cell, center_flag, position);
    let half = diameter >> 1;
    let rest = diameter - half;

    let mut layers = [object_layer, 0];
    let mut num_layers = 1usize;
    if !is_ground_or_bridge(object_layer) && bridge_interacts {
        layers[1] = 1; // LAYER_GROUND
        num_layers = 2;
    }
    let _ = mover_skip; // the body calls 0x006e6b89 on the mover, result unused

    let mut get_cell = get_cell;
    *scan_counter = scan_counter.wrapping_add(1);
    let mut count = 0i32;
    for x in (cell.x - half)..(cell.x + rest) {
        for y in (cell.y - half)..(cell.y + rest) {
            for k in 0..num_layers {
                let path_cell = match get_cell(layers[k], x, y) {
                    Some(c) => c,
                    None => continue,
                };
                let info = match path_cell.info {
                    Some(i) => i,
                    None => continue,
                };
                let mut node = info.occupants;
                while let Some(n) = node {
                    if let Some(unit) = n.object {
                        if !unit.skip {
                            out[count as usize] = unit.id;
                            count += 1;
                            if count == 16 {
                                return 16;
                            }
                        }
                    }
                    node = n.next;
                }
            }
        }
    }
    count
}

// ---------------------------------------------------------------------------
// 0x006ecaff — Pathfinder::CountOverlapHordeGoalUnits
// ---------------------------------------------------------------------------

/// A horde-goal occupant: `handle` is the object pointer identity,
/// `container` the `Object::m_container @ +0x274` pointer identity (the
/// game compares raw pointers; handles model that equality).
#[derive(Debug)]
pub struct HordeUnit {
    pub handle: i32,
    pub container: i32,
}

/// The collision cell view for this row; the horde-goal node list head is
/// `info + 0x18`.
#[derive(Debug)]
pub struct HordeCell<'a> {
    pub info: Option<&'a HordeCellInfo<'a>>,
}

#[derive(Debug)]
pub struct HordeCellInfo<'a> {
    pub horde_nodes: Option<&'a OccupantNode<'a, HordeUnit>>, // info + 0x18
}

/// `0x006ecaff` — `Pathfinder::CountOverlapHordeGoalUnits(object, position)`.
///
/// With a null container (`+0x274`) the body returns 0 without even
/// building the context. Otherwise the same cell-base + half/rest split
/// and layer-list logic as [`pathfinder_get_overlap_goal_units`], but the
/// `info + 0x18` horde list is scored: the mover itself is skipped, a
/// unit in the same container adds 1, any other adds 2.
pub fn pathfinder_count_overlap_horde_goal_units<'a, F, G>(
    mover_handle: i32,
    mover_container: i32, // 0 (null) short-circuits
    object_layer: u32,
    center_flag: bool,
    bridge_interacts: bool,
    diameter: i32,
    position: &PfVec3,
    is_ground_or_bridge: F,
    get_cell: G,
) -> i32
where
    F: FnOnce(u32) -> bool,
    G: FnMut(u32, i32, i32) -> Option<&'a HordeCell<'a>>,
{
    if mover_container == 0 {
        return 0;
    }
    let mut cell = PfCellPt::default();
    world_to_cell(&mut cell, center_flag, position);
    let half = diameter >> 1;
    let rest = diameter - half;

    let mut layers = [object_layer, 0];
    let mut num_layers = 1usize;
    if !is_ground_or_bridge(object_layer) && bridge_interacts {
        layers[1] = 1;
        num_layers = 2;
    }

    let mut get_cell = get_cell;
    let mut count = 0i32;
    for x in (cell.x - half)..(cell.x + rest) {
        for y in (cell.y - half)..(cell.y + rest) {
            for k in 0..num_layers {
                let path_cell = match get_cell(layers[k], x, y) {
                    Some(c) => c,
                    None => continue,
                };
                let info = match path_cell.info {
                    Some(i) => i,
                    None => continue,
                };
                let mut node = info.horde_nodes;
                while let Some(n) = node {
                    if let Some(unit) = n.object {
                        if unit.handle != mover_handle {
                            if unit.container == mover_container {
                                count += 1;
                            } else {
                                count += 2;
                            }
                        }
                    }
                    node = n.next;
                }
            }
        }
    }
    count
}

// ---------------------------------------------------------------------------
// 0x00766cea — PathfindLayer::ConnectsZones
// ---------------------------------------------------------------------------

/// The layer fields the body reads: `m_destroyed @ +0x30`, `m_width @ +0x08`,
/// `m_height @ +0x0C`, `m_xOrigin @ +0x10`, `m_yOrigin @ +0x14`.
#[derive(Clone, Copy, Debug, Default)]
pub struct PathfindLayerView {
    pub destroyed: bool,
    pub width: i32,
    pub height: i32,
    pub x_origin: i32,
    pub y_origin: i32,
}

/// A bridge-layer cell as read here: `m_flags @ +0x0C` (16-byte cells).
#[derive(Clone, Copy, Debug, Default)]
pub struct BridgeLayerCell {
    pub flags: u32, // +0x0C; connect layer = (flags >> 10) & 0x3f
}

/// A ground cell: `m_zone @ +0x08` (ushort — `0x0093241f` takes the raw
/// `*(ushort *)(cell + 8)`).
#[derive(Clone, Copy, Debug, Default)]
pub struct ZoneGroundCell {
    pub zone: u16, // +0x08
}

/// `LocomotorSet` view: `m_surfaces @ +0x10`, `m_15 @ +0x15`.
#[derive(Clone, Copy, Debug, Default)]
pub struct LocomotorSetView {
    pub surfaces: i32, // +0x10
    pub flag_15: u8,   // +0x15
}

/// The 13-byte `PathfindMovementProfile` the body stacks: surfaces, two
/// false byte flags (crusher/terrain-only), layer `-1`, and the mover
/// byte — built verbatim from the [`LocomotorSetView`] fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ZoneProfile {
    pub surfaces: i32,      // +0x00
    pub crusher: u8,        // +0x04 (false)
    pub terrain_only: u8,   // +0x05 (false)
    pub layer: i32,         // +0x08 (-1)
    pub mover_flag: u8,     // +0x0C
}

pub trait ZoneConnectScan {
    /// `0x006e6e6c` `isGroundOrBridgeLayer(layer)`.
    fn is_ground_or_bridge(&self, layer: u32) -> bool;
    /// `0x006e6d62` `Pathfinder::getCell(LAYER_GROUND = 1, x, y)`.
    fn get_cell(&self, layer: u32, x: i32, y: i32) -> Option<&ZoneGroundCell>;
    /// `0x0093241f` (zone-manager first mapping; returns the ushort zone).
    fn zone_step(&mut self, profile: &mut ZoneProfile, zone: u16) -> u16;
    /// `0x00931fd4` (zone-manager second mapping).
    fn zone_resolve(&mut self, profile: &mut ZoneProfile, zone: u16) -> u16;
}

/// `0x00766cea` — `PathfindLayer::ConnectsZones(zoneManager, locomotorSet,
/// zone1, zone2)`.
///
/// A layer that is not `m_destroyed` cannot connect anything (the body's
/// `in_EAX & 0xffffff00`), and a non-positive width skips the scan. The
/// full double loop walks every cell (no early exit): cells whose layer
/// `((flags >> 10) & 0x3f)` is ground-or-bridge consult
/// `getCell(1, x + xOrigin, y + yOrigin)`; the found ground cell's zone is
/// pushed through the two manager maps with the on-stack profile, and each
/// matching low-16 value sets `found1`/`found2`. The result is
/// `found1 && found2` *after* the whole scan (the game's post-loop test).
pub fn pathfind_layer_connects_zones<S: ZoneConnectScan + ?Sized>(
    layer: &PathfindLayerView,
    cells: &[&[BridgeLayerCell]], // m_layerCells[x][y]
    locomotor: &LocomotorSetView,
    zone1: u16,
    zone2: u16,
    scan: &mut S,
) -> bool {
    if !layer.destroyed {
        return false;
    }
    if layer.width <= 0 {
        return false;
    }
    let mut found1 = false;
    let mut found2 = false;
    for x in 0..layer.width {
        if layer.height <= 0 {
            continue;
        }
        let profile = ZoneProfile {
            surfaces: locomotor.surfaces,
            crusher: 0,
            terrain_only: 0,
            layer: -1,
            mover_flag: locomotor.flag_15,
        };
        for y in 0..layer.height {
            let cell = &cells[x as usize][y as usize];
            let connect_layer = (cell.flags >> 10) & 0x3f;
            if !scan.is_ground_or_bridge(connect_layer) {
                continue;
            }
            let ground = match scan.get_cell(1, layer.x_origin + x, layer.y_origin + y) {
                Some(g) => g,
                None => continue,
            };
            let mut profile = profile;
            let zone = scan.zone_step(&mut profile, ground.zone);
            let zone = scan.zone_resolve(&mut profile, zone);
            if (zone & 0xffff) == zone1 {
                found1 = true;
            }
            if (zone & 0xffff) == zone2 {
                found2 = true;
            }
        }
    }
    found1 && found2
}

// ---------------------------------------------------------------------------
// 0x00932708 — PathfindZoneManager::CreateEquivalencySet
// ---------------------------------------------------------------------------

/// `ZoneNeighborRecord` — the 4-byte adjacency record
/// (`{unsigned short zone; unsigned short count;}`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ZoneNeighborRecord {
    pub zone: u16,
    pub count: u16,
}

pub trait EquivalencySetOps {
    /// `0x00931abb` `MakeSet(i)` on the selected union-find
    /// (`unions[variant][equivalent]`).
    fn make_set(&mut self, zone: u16);
    /// `0x00931720(variant, zone)` — the used-mask predicate.
    fn mask_used(&self, variant: u32, zone: u32) -> bool;
    /// `0x00931757(equivalent, zone)` — `CouldBeInEquivSet`.
    fn could_be_in_equiv_set(&self, equivalent: u32, zone: u32) -> bool;
    /// `0x009317d7(equivalent, first, second)` — `InEquivSet`.
    fn in_equiv_set(&self, equivalent: u32, first: u32, second: u32) -> bool;
    /// `0x00932431` `LinkSets(first, second)` on the selected union-find.
    fn link_sets(&mut self, first: u16, second: u16);
}

/// `0x00932708` — `PathfindZoneManager::CreateEquivalencySet(variant,
/// equivalent)`.
///
/// `union_first` is the union record's `first` ushort (the 20-byte record
/// at `unions[variant][equivalent] @ manager + 0x1B594`, field @ +0x02) —
/// the body's first store zeroes it. With `manager_end` (`+0x06`, ushort)
/// non-zero, every zone `0..end` gets `MakeSet`. Then for every zone with
/// `mask_used(variant, zone)` and `could_be_in_equiv_set(equivalent,
/// zone)`, the adjacency range (`0x00932104`, returned begin/end pair) is
/// walked record by record: a neighbour passing `mask_used` *and*
/// `in_equiv_set(equivalent, zone, other)` is linked with `LinkSets`.
pub fn pathfind_zone_manager_create_equivalency_set<'a, A>(
    union_first: &mut u16,
    manager_end: u16,
    variant: u32,
    equivalent: u32,
    ops: &mut dyn EquivalencySetOps,
    adjacency: A, // 0x00932104 range for a zone
) where
    A: Fn(u32) -> &'a [ZoneNeighborRecord],
{
    *union_first = 0;
    if manager_end != 0 {
        for i in 0..manager_end as u32 {
            ops.make_set(i as u16);
        }
    }
    for zone in 0..manager_end as u32 {
        if !ops.mask_used(variant, zone) {
            continue;
        }
        if !ops.could_be_in_equiv_set(equivalent, zone) {
            continue;
        }
        for rec in adjacency(zone) {
            let other = rec.zone as u32;
            if !ops.mask_used(variant, other) {
                continue;
            }
            if !ops.in_equiv_set(equivalent, zone, other) {
                continue;
            }
            ops.link_sets(zone as u16, other as u16);
        }
    }
}

// ---------------------------------------------------------------------------
// 0x006fa202 — Pathfinder::AdjustHordeMeleeDestination
// ---------------------------------------------------------------------------

/// The `Pathfinder_IsCheckForMeleeHordeAdjust` functor fields this row
/// reads back from the `0x006ed72b` constructor: `m_layer @ +0x14`, the
/// fallback cell pair `m_1C @ +0x1C`, and the fallback-presence flag
/// `m_24 @ +0x24`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MeleeAdjustInfo {
    pub layer: i32,     // +0x14
    pub fallback_cell: PfCellPt, // +0x1C
    pub has_fallback: i32,       // +0x24 (tested `== 0`)
}

/// `0x006fa202` — `Pathfinder::AdjustHordeMeleeDestination(obj, arg, dest)`.
///
/// The template kind guard (`*(template + 0x10B) & 2`, kindof 25) skips
/// everything and returns true. The centre flag is the unit's odd-parity
/// probe (`0x006ebbfb`); the cell base is `world_to_cell(cell, center,
/// dest)`. The functor is built by `0x006ed72b(this, obj, arg, *dest,
/// height)` with the terrain height from the `DAT_00dfec50` vtable slot
/// `+0x1C` (`getLayerHeight(dest.x, dest.y, layerForDest, 0, true)` —
/// both `layer_for_dest` (`0x006802fe`) and `height` are caller-resolved
/// inputs captured *before* `dest` moves). `0x006f92bc` spiral-searches an
/// acceptable cell (budget `TheWritableGlobalData + 0x11F4`): success
/// adjusts to the found cell; failure falls back only when the functor's
/// `m_24` is set (otherwise false); the adjusted point (3 floats out of
/// `0x006e8c4d adjustCellCenter(out, center, cell, m_layer)`) is copied
/// over `dest`.
pub fn pathfinder_adjust_horde_melee_destination(
    kind_25: bool,
    center: bool,
    dest: &mut PfVec3,
    layer_for_dest: i32,
    height: f32,
    search_radius: i32,
    mut init_info: impl FnMut(&mut MeleeAdjustInfo, &PfVec3, f32),
    mut find_cell: impl FnMut(&PfCellPt, i32, &mut PfCellPt, &MeleeAdjustInfo) -> bool,
    mut adjust_center: impl FnMut(&mut PfVec3, bool, &PfCellPt, i32),
) -> bool {
    if kind_25 {
        return true;
    }
    let mut cell = PfCellPt::default();
    world_to_cell(&mut cell, center, dest);
    let mut info = MeleeAdjustInfo::default();
    init_info(&mut info, dest, height);
    let _ = layer_for_dest; // consumed by the terrain height the caller resolved
    let mut found = PfCellPt::default();
    let target = if find_cell(&cell, search_radius, &mut found, &info) {
        found
    } else {
        if info.has_fallback == 0 {
            return false;
        }
        info.fallback_cell
    };
    let mut out = PfVec3::default();
    adjust_center(&mut out, center, &target, info.layer);
    *dest = out;
    true
}

// ---------------------------------------------------------------------------
// 0x006edcf7 / 0x006edda9 — AdjustToNearestGroundCell / ValidCell
// ---------------------------------------------------------------------------

/// Shared body of the twin rows `0x006edcf7` and `0x006edda9`.
///
/// `cell_from` is `0x006e7964(this, &cell, 1, position)` (centre-true
/// conversion; only `cell.x < 0` fails the row — the body's `-1 <
/// local_c` test). `scan` is the circular search — `0x006eae5f` for the
/// ground row (budget `TheWritableGlobalData + 0x1204`, context class
/// `0x002E7414`) or `0x006eb11b` for the valid row (budget `+0x1208`,
/// context `0x002E7440`); the context object holds only the pathfinder
/// handle, modelled as `pathfinder_ctx`. On success the body rewrites
/// `position.x = found.x * 10 + 1.0f` and `position.y = found.y * 10 +
/// 1.0f` (`DAT_00bbb8d8 = 1.0f`, the int multiply wrapping first), *then*
/// queries `position.z` through the terrain vtable slot `+0x1C` with the
/// *new* x/y (`getLayerHeight(x, y, LAYER_GROUND = 1, 0, true)`).
fn adjust_to_nearest_cell_impl(
    position: &mut PfVec3,
    search_budget: i32,
    pathfinder_ctx: i32,
    mut cell_from: impl FnMut(i32, &mut PfCellPt),
    mut scan: impl FnMut(&PfCellPt, i32, &mut PfCellPt, i32) -> bool,
    mut layer_height: impl FnMut(f32, f32) -> f32,
) -> bool {
    let mut cell = PfCellPt::default();
    cell_from(pathfinder_ctx, &mut cell);
    if !(cell.x > -1) {
        return false;
    }
    let mut found = PfCellPt { x: 0, y: 0 };
    if !scan(&cell, search_budget, &mut found, pathfinder_ctx) {
        return false;
    }
    let nx = (found.x.wrapping_mul(10)) as f32 + PF_ONE;
    let ny = (found.y.wrapping_mul(10)) as f32 + PF_ONE;
    position.x = nx;
    position.y = ny;
    position.z = layer_height(nx, ny);
    true
}

/// `0x006edcf7` — `Pathfinder::AdjustToNearestGroundCell(position)`
/// (ground-cell scan `0x006eae5f`, budget `+0x1204`).
pub fn pathfinder_adjust_to_nearest_ground_cell(
    position: &mut PfVec3,
    search_budget: i32,
    pathfinder_ctx: i32,
    cell_from: impl FnMut(i32, &mut PfCellPt),
    scan: impl FnMut(&PfCellPt, i32, &mut PfCellPt, i32) -> bool,
    layer_height: impl FnMut(f32, f32) -> f32,
) -> bool {
    adjust_to_nearest_cell_impl(
        position,
        search_budget,
        pathfinder_ctx,
        cell_from,
        scan,
        layer_height,
    )
}

/// `0x006edda9` — `Pathfinder::AdjustToNearestValidCell(position)`
/// (valid-cell scan `0x006eb11b`, budget `+0x1208`). Identical body apart
/// from the two callees and the budget field the caller supplies.
pub fn pathfinder_adjust_to_nearest_valid_cell(
    position: &mut PfVec3,
    search_budget: i32,
    pathfinder_ctx: i32,
    cell_from: impl FnMut(i32, &mut PfCellPt),
    scan: impl FnMut(&PfCellPt, i32, &mut PfCellPt, i32) -> bool,
    layer_height: impl FnMut(f32, f32) -> f32,
) -> bool {
    adjust_to_nearest_cell_impl(
        position,
        search_budget,
        pathfinder_ctx,
        cell_from,
        scan,
        layer_height,
    )
}

// ---------------------------------------------------------------------------
// 0x00765df0 — Path::SpliceAndPrepend
// ---------------------------------------------------------------------------

/// `PathNode`: `m_next @ +0x00`, `m_previous @ +0x04`,
/// `m_nextOptimized @ +0x08`, `m_position @ +0x0C` (28-byte node).
/// Links are raw node handles; `0` models null.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PathNodeMirror {
    pub next: i32,
    pub previous: i32,
    pub next_optimized: i32,
    pub position: PfVec3,
}

/// `Path`: `m_path @ +0x04`, `m_pathTail @ +0x08`, `m_selected @ +0x10`,
/// `m_distance @ +0x14` (the `+0x0C/+0x0D` optimisation flags and `+0x00`
/// are untouched by this row).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PathMirror {
    pub m_path: i32,
    pub m_path_tail: i32,
    pub m_selected: i32,
    pub m_distance: f32,
}

/// Handle-addressable node arena access, split into the four tiny
/// operations this row performs (the game's raw pointer derefs): the reads
/// return `None` exactly where the game would load through a null or
/// dangling node pointer (`position_of`/`next_of` callers panic there),
/// `link_tail` writes `m_next` and `m_nextOptimized` in one step, and
/// `set_previous` mirrors the unchecked `+0x04` store.
pub trait PathNodeOps {
    /// `PathNode::m_position @ +0x0C` read.
    fn position_of(&mut self, node: i32) -> Option<PfVec3>;
    /// `PathNode::m_next @ +0x00` read.
    fn next_of(&mut self, node: i32) -> Option<i32>;
    /// `m_next @ +0x00` and `m_nextOptimized @ +0x08` writes (tail splice).
    fn link_tail(&mut self, tail: i32, next: i32);
    /// `PathNode::m_previous @ +0x04` write.
    fn set_previous(&mut self, node: i32, previous: i32);
}

/// `0x00765df0` — `Path::SpliceAndPrepend(product)`.
///
/// A null supplied path (`None`) returns at once. A supplied path with no
/// tail is destroyed (`0x00764a89` dtor + `0x0042fd60` delete, one hook)
/// and returned. Otherwise `0x00763930` `DoAdvanceAlongPath(&tail->m_position,
/// false)` runs on the supplied tail's position; failure likewise destroys
/// the supplied path. On success this path reclaims every node strictly
/// before `m_selected` (free via `0x00665488`, the pooled-node free — the
/// body loads `*m_path` unchecked but only frees non-null nodes), then
/// splices:
/// `other.tail.nextOptimized = other.tail.next = selected.next` and
/// `selected.next.previous = other.tail` — the `selected.next` dereference
/// is the game's unchecked load (missing handle -> panic). The old
/// `m_path` (== `m_selected`) is freed once, this path adopts
/// `other.m_path`, `other.m_path` is zeroed, the supplied path is
/// destroyed, `m_selected = m_path` and `m_distance = 0.0`.
pub fn path_splice_and_prepend(
    this: &mut PathMirror,
    other: Option<&mut PathMirror>,
    mut nodes: impl PathNodeOps,
    mut advance: impl FnMut(&PfVec3, bool) -> bool,
    mut free_node: impl FnMut(i32),
    destroy_other: impl FnOnce(&mut PathMirror),
) {
    let other = match other {
        Some(o) => o,
        None => return,
    };
    if other.m_path_tail == 0 {
        destroy_other(other);
        return;
    }
    let tail_pos = match nodes.position_of(other.m_path_tail) {
        Some(p) => p,
        None => panic!("SpliceAndPrepend: bad m_pathTail handle (game wild-reads)"),
    };
    if !advance(&tail_pos, false) {
        destroy_other(other);
        return;
    }
    if this.m_path != this.m_selected {
        loop {
            let next = match nodes.next_of(this.m_path) {
                Some(n) => n,
                None => panic!("SpliceAndPrepend: wild read of *m_path (game loads it unchecked)"),
            };
            if this.m_path != 0 {
                free_node(this.m_path); // the game's null-guarded delete
            }
            this.m_path = next;
            if next == this.m_selected {
                break;
            }
        }
    }
    let next = match nodes.next_of(this.m_selected) {
        Some(n) => n,
        None => panic!("SpliceAndPrepend: null/dangling m_selected deref (game UB)"),
    };
    nodes.link_tail(other.m_path_tail, next);
    // `selected.next.m_previous = other.tail`: the game's unchecked `+0x04`
    // store through the raw link (a null/dangling `next` is its wild write).
    nodes.set_previous(next, other.m_path_tail);
    if this.m_path != 0 {
        free_node(this.m_path);
    }
    this.m_path = other.m_path;
    other.m_path = 0;
    destroy_other(other);
    this.m_selected = this.m_path;
    this.m_distance = 0.0;
}

// ---------------------------------------------------------------------------
// 0x00860cba — DynamicPortalBehaviour footprint/range query
// ---------------------------------------------------------------------------

/// The portal module data: `m_144 @ +0x144` (the offset vector; `+0x148`
/// and `+0x14C` are its y/z).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PortalModuleData {
    pub m_144: PfVec3, // +0x144
}

/// The portal's object view: `position @ +0x38`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PortalObjectView {
    pub position: PfVec3, // +0x38
}

/// `0x00860cba` — `DynamicPortalBehaviour::rva00460CBA(Coord3D *out)`.
///
/// The object's transformed x-axis unit vector (`(1, 0, 0)` through
/// `Thing::transformPoint`, `0x0070a812`) is reduced to the local offset
/// `transformed - position`; `pick` is the larger of `|dy|`, `|dx|`
/// (the body compares `|dx| <= |dy|` keeping the y pick — a NaN fabs
/// falls to the y branch literally); `scale = 1.0f / pick` divides from
/// `DAT_00bbb8d8`, the vector is scaled componentwise (z through the
/// literal `scale * 0.0f`, sign-preserving / NaN-producing like the
/// game), its length becomes the magnitude for the module offset: `out =
/// transformPoint(m_144.x * len, m_144.y * len, m_144.z unchanged)`.
pub fn dynamic_portal_footprint_offset(
    module_data: &PortalModuleData,
    object: &PortalObjectView,
    mut transform: impl FnMut(&PfVec3, &mut PfVec3),
    out: &mut PfVec3,
) {
    let unit = PfVec3 {
        x: PF_ONE,
        y: 0.0,
        z: 0.0,
    };
    let mut v = PfVec3::default();
    transform(&unit, &mut v);
    v.x -= object.position.x;
    v.y -= object.position.y;
    v.z -= object.position.z;
    let abs_y = f64::from(v.y).abs() as f32;
    let abs_x = f64::from(v.x).abs() as f32;
    let pick = if abs_x <= abs_y { abs_y } else { abs_x };
    let scale = PF_ONE / pick;
    v.x = scale * v.x;
    v.y = scale * v.y;
    v.z = scale * 0.0; // literal `local_10 * (float)0`
    let len = (v.x * v.x + v.y * v.y + v.z * v.z).sqrt();
    let src = PfVec3 {
        x: module_data.m_144.x * len,
        y: module_data.m_144.y * len,
        z: module_data.m_144.z,
    };
    transform(&src, out);
}

// ---------------------------------------------------------------------------
// 0x006ed0cc — line-contexts init (Rva002ED15AInfo::init)
// ---------------------------------------------------------------------------

/// The 16-byte query block (`m_48`): the third ctor argument, the two
/// condition bytes with their padding (the game's dword copies carry the
/// pad bytes too), the layer budget and the object flag byte.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QueryBlock16 {
    pub arg_c: i32,        // +0x00 (param_4)
    pub no_634: bool,      // +0x04 (`inner +0x634 == 0`)
    pub afbb: u8,          // +0x05 (0x0068afbb result)
    pub pad_06: [u8; 2],   // +0x06 (copied stack garbage in the game)
    pub budget_56c_m1: i32, // +0x08 (`*(inner + 0x56C) - 1`)
    pub ac62: u8,          // +0x0C (0x0068ac62 result)
    pub pad_0d: [u8; 3],   // +0x0D
}

/// The 56-byte `Rva002E6FDF` query record embedded at `m_08`. This row
/// writes `m_0C`/`m_10` (the unit split output pair), zeroes `m_11`,
/// stamps `m_14 = 12` and copies the 16-byte block over `m_1C..m_2B`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Query08 {
    pub m_00: i32,
    pub m_04: i32,
    pub m_08: i32,
    pub m_0c: i32,     // +0x14 abs — split output half 1 (0x006ebca7)
    pub m_10: u8,      // +0x18 abs — split output half 2
    pub m_11: u8,      // +0x19 abs — forced 0
    pub m_14: i32,     // +0x1C abs — forced 12
    pub m_18: i32,     // +0x20 abs
    pub copy_1c: QueryBlock16, // +0x24..+0x33 abs — verbatim copy of m_48
    pub m_2c: i32,     // +0x34 abs
    pub m_30: u8,
    pub m_31: u8,
    pub m_32: u8,
    pub m_34: i32,     // +0x3C abs
}

/// The full 0x5C-byte line-walk context (`Rva002ED15AInfo`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LineContextInfo {
    pub m_00: i32,      // +0x00 first argument (callback owner)
    pub m_04: i32,      // +0x04 Object handle
    pub query: Query08, // +0x08..+0x3F
    pub m_40: [u8; 8],  // +0x40
    pub block: QueryBlock16, // +0x48
    pub m_58: i32,      // +0x58
}

/// `0x006ed0cc` — the line-context initialiser
/// (`init(void *a, Object *b, void *c)` → this).
///
/// Field stores follow the body exactly: `m_00 = a`; `m_04 = b`; the
/// query constructor (`0x006e6fdf`) runs first on `m_08`; the inner block
/// at `*(b + 4)` supplies `+0x56C` (minus one -> budget) and the `+0x634`
/// flag (inverted into `no_634`); the two object predicates (`0x0068ac62`
/// -> `ac62`, `0x0068afbb` -> `afbb`) are resolved by the caller;
/// `m_58 = 0`; the whole 16-byte block at `m_48` is dword-copied onto
/// `m_08.m_1C` (pad bytes included); `m_08.m_11 = 0`, `m_08.m_14 = 12`;
/// finally `0x006ebca7` (the unit radius/centre split of `m_04`) writes
/// `m_08.m_0C` (i32) and `m_08.m_10` (byte). The game returns `this`.
pub fn pathfind_line_context_init(
    info: &mut LineContextInfo,
    a: i32,
    object: i32,
    c: i32,
    inner_56c: i32,    // *(Object block + 0x56C)
    inner_634: u8,     // *(Object block + 0x634)
    ac62: bool,        // 0x0068ac62()
    afbb: bool,        // 0x0068afbb()
    init_query: impl FnOnce(&mut Query08), // 0x006e6fdf
    split: impl FnOnce(i32, &mut i32, &mut u8), // 0x006ebca7
) {
    info.m_00 = a;
    info.m_04 = object;
    init_query(&mut info.query);
    info.block.arg_c = c;
    info.block.no_634 = inner_634 == 0;
    info.block.afbb = afbb as u8;
    info.block.ac62 = ac62 as u8;
    info.block.budget_56c_m1 = inner_56c - 1;
    info.m_58 = 0;
    info.query.copy_1c = info.block;
    info.query.m_11 = 0;
    info.query.m_14 = 12;
    split(object, &mut info.query.m_0c, &mut info.query.m_10);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v3(x: f32, y: f32, z: f32) -> PfVec3 {
        PfVec3 { x, y, z }
    }

    #[test]
    fn world_to_cell_center_and_plain() {
        let mut c = PfCellPt::default();
        // center (param_2 == 0): +0.5 bias
        world_to_cell(&mut c, false, &v3(14.0, -0.5, 0.0));
        // floor(1.4 + 0.5) = 1; floor(-0.05 + 0.5) = floor(0.45) = 0
        assert_eq!(c, PfCellPt { x: 1, y: 0 });
    }

    #[test]
    fn world_to_cell_exact_values() {
        let mut c = PfCellPt::default();
        world_to_cell(&mut c, false, &v3(10.0, 14.9, 0.0));
        assert_eq!(c.x, 1); // floor(1.0 + 0.5) = 1
        assert_eq!(c.y, 1); // floor(1.49 + 0.5) = 1
        world_to_cell(&mut c, true, &v3(14.9, 15.0, 0.0));
        assert_eq!(c.x, 1); // floor(1.49) = 1
        assert_eq!(c.y, 1); // floor(1.5)  = 1
        world_to_cell(&mut c, false, &v3(-0.1, -15.0, 0.0));
        // floor(-0.01 + 0.5) = 0; -15.0 * 0.1f = -1.5000000224 but + 0.5f
        // rounds (f32) to exactly -1.0 (2.24e-8 < half an ulp) -> floor -1
        assert_eq!(c.x, 0);
        assert_eq!(c.y, -1);
    }

    #[test]
    fn x87_fistp_edges() {
        assert_eq!(x87_fistp_i32(2.5), 2); // half-to-even down
        assert_eq!(x87_fistp_i32(3.5), 4); // half-to-even up
        assert_eq!(x87_fistp_i32(-2.5), -2);
        assert_eq!(x87_fistp_i32(2147483648.0), i32::MIN); // x87 invalid-int store
        assert_eq!(x87_fistp_i32(f32::NAN), i32::MIN);
        assert_eq!(x87_fistp_i32(f32::INFINITY), i32::MIN);
        assert_eq!(x87_fistp_i32(-2147483648.0), i32::MIN); // exactly in range
    }

    #[test]
    fn valid_movement_terrain_paths() {
        let table = [0u32, 1, 2, 4, 8, 16, 32, 64, 0, 0, 0, 0, 0, 0, 0, 0x8000_0000];
        // null cell -> 0
        assert_eq!(
            pathfinder_is_valid_movement_terrain(1, &v3(5.0, 5.0, 0.0), 0xf, &table, |_, _, _| None),
            0
        );
        // type 4 (obstacle): CONCAT31 tail from the raw flags
        let cell = PfCellFlags { flags: 0x00a1_0004 };
        let r = pathfinder_is_valid_movement_terrain(1, &v3(0.0, 0.0, 0.0), 0, &table, |_, _, _| Some(&cell));
        assert_eq!(r, (0x00a1_0004 & 0xffff_ff00) | 1);
        // type 5 (impassable)
        let cell5 = PfCellFlags { flags: 0x15 };
        assert_eq!(
            pathfinder_is_valid_movement_terrain(1, &v3(0.0, 0.0, 0.0), 0, &table, |_, _, _| Some(&cell5)),
            (0x15 & 0xffffff00) | 1
        );
        // non-ground clear (layer 2 -> flags & 0x3f0 = 0x20, type 0)
        let cellg = PfCellFlags { flags: 0x20 };
        assert_eq!(
            pathfinder_is_valid_movement_terrain(1, &v3(0.0, 0.0, 0.0), 0, &table, |_, _, _| Some(&cellg)),
            1
        );
        // ground clear: table AND mask
        let cellg0 = PfCellFlags { flags: 0x10 };
        // ground clear (layer bits = 1) falls through to the table path:
        // table[0] = 0 -> bit 0, tail = (legal >> 8) << 8 = 0
        assert_eq!(
            pathfinder_is_valid_movement_terrain(1, &v3(0.0, 0.0, 0.0), 0xf, &table, |_, _, _| Some(&cellg0)),
            0
        );
        let cell1 = PfCellFlags { flags: 0x11 }; // type 1
        let r = pathfinder_is_valid_movement_terrain(1, &v3(0.0, 0.0, 0.0), 0x00ff_0001, &table, |_, _, _| Some(&cell1));
        assert_eq!(r, (0x00ff_0001 & 0xffffff00) | 1); // table[1] = 1
        let r0 = pathfinder_is_valid_movement_terrain(1, &v3(0.0, 0.0, 0.0), 0x100, &table, |_, _, _| Some(&cell1));
        assert_eq!(r0, 0x100); // table[1] & 0x100 = 0: bit cleared, legal tail kept
    }

    #[test]
    fn radial_offset_normals() {
        let from = v3(0.0, 0.0, 0.0);
        let to = v3(10.0, 0.0, 0.0);
        // body deltas: local = from - to (negated segment), cross apex
        // is `to`: (obj.y - to.y)*local_c - (obj.x - to.x)*local_10
        // object above the segment: test = 2 * -10 = -20 <= 0 -> negate
        // local_10 (stays -0) -> normal (from-to)/10 = (-1, 0)
        let obj = PfObjectPos { position: v3(5.0, 2.0, 7.0) };
        let mut ins = PfVec3::default();
        compute_normal_radial_offset(&from, &mut ins, &to, &obj, 3.0);
        assert_eq!(ins, v3(2.0, 2.0, 7.0));
        // object below: test = -2 * -10 = 20 > 0 -> negate local_c ->
        // normal (10, 0)/10 = (1, 0)
        let obj2 = PfObjectPos { position: v3(5.0, -2.0, 0.0) };
        compute_normal_radial_offset(&from, &mut ins, &to, &obj2, 4.0);
        assert_eq!(ins, v3(9.0, -2.0, 0.0));
        // exactly collinear (test == 0.0): `<= 0.0` is true -> negate
        // local_10 (0 -> -0); normal still (-1, -0)
        let obj3 = PfObjectPos { position: v3(5.0, 0.0, 1.0) };
        compute_normal_radial_offset(&from, &mut ins, &to, &obj3, 5.0);
        assert_eq!(ins, v3(0.0, 0.0, 1.0));
        // degenerate segment: normal (0, 0, 0), `Coord3D::normalize`
        // skips (len == 0) -> insert == object position
        let same = v3(10.0, 0.0, 0.0);
        compute_normal_radial_offset(&same, &mut ins, &to, &obj3, 5.0);
        assert_eq!(ins, v3(5.0, 0.0, 1.0));
        // NaN test object: `<= 0.0` is false -> else branch (negate dx)
        let objn = PfObjectPos { position: v3(f32::NAN, f32::NAN, 0.0) };
        compute_normal_radial_offset(&from, &mut ins, &to, &objn, 1.0);
        assert!(ins.x.is_nan());
    }

    #[test]
    fn best_cell_visitor() {
        let mut v = BestCellVisitor {
            owner: 1,
            source: v3(12.0, 34.0, 5.0),
            closest: PfVec3::default(),
            cost: 100.0,
            found: false,
        };
        // kind 7 / 1 / nonzero rejected (literal order); only kind 0 visits
        assert!(!pathfinder_best_cell_visit(&mut v, 7, 1, 3));
        assert!(!pathfinder_best_cell_visit(&mut v, 1, 1, 3));
        assert!(!pathfinder_best_cell_visit(&mut v, 3, 1, 3));
        assert!(!v.found);
        // cell (1,3): world (10, 30), dist = 4 + 16 = 20 < 100
        assert!(!pathfinder_best_cell_visit(&mut v, 0, 1, 3));
        assert!(v.found);
        assert_eq!(v.closest, v3(10.0, 30.0, 5.0));
        assert_eq!(v.cost, 20.0);
        // farther cell does not update
        pathfinder_best_cell_visit(&mut v, 0, 2, 5);
        assert_eq!(v.cost, 20.0);
    }

    #[test]
    fn best_cell_visitor_nan_z_poisons() {
        let mut v = BestCellVisitor {
            source: v3(0.0, 0.0, f32::NAN),
            cost: f32::MAX,
            ..Default::default()
        };
        pathfinder_best_cell_visit(&mut v, 0, 0, 0);
        assert!(!v.found); // NaN dist: every comparison false
    }

    struct TestScan {
        relationships: i32,
        horde: bool,
        flag931ba: bool,
        issued: Option<(i32, PfVec3)>,
    }
    impl MoveAlliesScan for TestScan {
        fn relationship_to_mover(&self, _: &MoveAlliesUnit) -> i32 {
            self.relationships
        }
        fn horde_excluded(&self, _: &MoveAlliesUnit) -> bool {
            self.horde
        }
        fn flag_931ba(&self, _: &MoveAlliesUnit) -> bool {
            self.flag931ba
        }
        fn ai_move_away_from_unit(&mut self, ai: &MoveAlliesAi, _: &MoveAlliesUnit, dest: &PfVec3) {
            self.issued = Some((ai.goal.x as i32, *dest));
        }
    }

    #[test]
    fn move_allies_scans_and_commands() {
        let ai_idle = MoveAlliesAi {
            is_moving: false,
            goal_toward_mover: false,
            goal: v3(0.0, 0.0, 0.0),
        };
        let mover = MoveAlliesUnit {
            id: 7,
            radius: 5.0,
            contained_by: None,
            ai: None,
        };
        let other = MoveAlliesUnit {
            id: 9,
            radius: 1.0,
            contained_by: None,
            ai: Some(&ai_idle),
        };
        let node = OccupantNode {
            next: None,
            object: Some(&other),
        };
        let info = AlliesCellInfo {
            occupants: Some(&node),
        };
        let cell = AlliesScanCell {
            info: Some(&info),
        };
        let mut scan = TestScan {
            relationships: 2,
            horde: false,
            flag931ba: false,
            issued: None,
        };
        pathfinder_move_allies_cell_callback(&mover, 3, &v3(50.0, 50.0, 0.0), Some(&cell), &mut scan);
        assert!(scan.issued.is_some()); // idle ally: command, then stop
    }

    #[test]
    fn move_allies_skips_self_ignore_and_nonallies() {
        let ai_idle = MoveAlliesAi {
            is_moving: false,
            goal_toward_mover: false,
            goal: v3(0.0, 0.0, 0.0),
        };
        let mover = MoveAlliesUnit {
            id: 7,
            radius: 5.0,
            contained_by: None,
            ai: None,
        };
        let self_node = OccupantNode {
            next: None,
            object: Some(&mover), // pointer-identity self
        };
        let info = AlliesCellInfo {
            occupants: Some(&self_node),
        };
        let cell = AlliesScanCell {
            info: Some(&info),
        };
        let mut scan = TestScan {
            relationships: 2,
            horde: false,
            flag931ba: false,
            issued: None,
        };
        pathfinder_move_allies_cell_callback(&mover, 3, &v3(0.0, 0.0, 0.0), Some(&cell), &mut scan);
        assert!(scan.issued.is_none());
        // relationship ENEMIES (0) with a live other unit also skips
        let other = MoveAlliesUnit {
            id: 9,
            radius: 1.0,
            contained_by: None,
            ai: Some(&ai_idle),
        };
        let n2 = OccupantNode {
            next: None,
            object: Some(&other),
        };
        let info2 = AlliesCellInfo {
            occupants: Some(&n2),
        };
        let cell2 = AlliesScanCell {
            info: Some(&info2),
        };
        let mut scan2 = TestScan {
            relationships: 0,
            horde: false,
            flag931ba: false,
            issued: None,
        };
        pathfinder_move_allies_cell_callback(&mover, 3, &v3(0.0, 0.0, 0.0), Some(&cell2), &mut scan2);
        assert!(scan2.issued.is_none());
    }

    #[test]
    fn move_allies_moving_far_stays_moving() {
        // moving, not toward mover, goal far from dest (dist 20 > r 5): stays
        let ai = MoveAlliesAi {
            is_moving: true,
            goal_toward_mover: false,
            goal: v3(100.0, 0.0, 0.0),
        };
        let mover = MoveAlliesUnit {
            id: 7,
            radius: 5.0,
            contained_by: None,
            ai: None,
        };
        let other = MoveAlliesUnit {
            id: 9,
            radius: 1.0,
            contained_by: None,
            ai: Some(&ai),
        };
        let n = OccupantNode {
            next: None,
            object: Some(&other),
        };
        let info = AlliesCellInfo {
            occupants: Some(&n),
        };
        let cell = AlliesScanCell {
            info: Some(&info),
        };
        let mut scan = TestScan {
            relationships: 2,
            horde: false,
            flag931ba: false,
            issued: None,
        };
        pathfinder_move_allies_cell_callback(&mover, 3, &v3(0.0, 0.0, 0.0), Some(&cell), &mut scan);
        assert!(scan.issued.is_none());
        // same but goal within radius of dest -> treated as not moving
        let ai2 = MoveAlliesAi {
            is_moving: true,
            goal_toward_mover: false,
            goal: v3(3.0, 0.0, 0.0),
        };
        let other2 = MoveAlliesUnit {
            id: 9,
            radius: 1.0,
            contained_by: None,
            ai: Some(&ai2),
        };
        let n2 = OccupantNode {
            next: None,
            object: Some(&other2),
        };
        let info2 = AlliesCellInfo {
            occupants: Some(&n2),
        };
        let cell2 = AlliesScanCell {
            info: Some(&info2),
        };
        pathfinder_move_allies_cell_callback(&mover, 3, &v3(0.0, 0.0, 0.0), Some(&cell2), &mut scan);
        assert!(scan.issued.is_some());
    }

    #[test]
    fn move_allies_null_info_returns() {
        let mover = MoveAlliesUnit {
            id: 7,
            radius: 5.0,
            contained_by: None,
            ai: None,
        };
        let cell = AlliesScanCell { info: None };
        let mut scan = TestScan {
            relationships: 2,
            horde: false,
            flag931ba: false,
            issued: None,
        };
        pathfinder_move_allies_cell_callback(&mover, 3, &v3(0.0, 0.0, 0.0), Some(&cell), &mut scan);
        assert!(scan.issued.is_none());
    }

    #[test]
    fn connects_zones_scan() {
        // layer: destroyed, width 1, height 1, origin (10, 20)
        let layer = PathfindLayerView {
            destroyed: true,
            width: 1,
            height: 1,
            x_origin: 10,
            y_origin: 20,
        };
        // cell flags: the connect-layer field (bits 10..15) ->
        // (0x2000 >> 10) & 0x3f = 8, ground-or-bridge here
        let row = [BridgeLayerCell { flags: 0x2000 }];
        let cells = [&row[..]];
        const GROUND: ZoneGroundCell = ZoneGroundCell { zone: 5 };
        struct Scan;
        impl ZoneConnectScan for Scan {
            fn is_ground_or_bridge(&self, layer: u32) -> bool {
                layer == 8
            }
            fn get_cell(&self, layer: u32, x: i32, y: i32) -> Option<&ZoneGroundCell> {
                // getCell(LAYER_GROUND = 1, xOrigin + x, yOrigin + y)
                assert_eq!(layer, 1);
                assert_eq!((x, y), (10, 20));
                Some(&GROUND)
            }
            fn zone_step(&mut self, p: &mut ZoneProfile, z: u16) -> u16 {
                // on-stack profile: surfaces/mover byte from the
                // LocomotorSet, crusher/terrain-only false, layer -1
                assert_eq!(p.layer, -1);
                assert_eq!(p.surfaces, 3);
                assert_eq!(p.mover_flag, 9);
                z + 1 // 0x0093241f mapping
            }
            fn zone_resolve(&mut self, _p: &mut ZoneProfile, z: u16) -> u16 {
                z + 1 // 0x00931fd4 mapping
            }
        }
        // ground zone 5 maps 5 -> 6 -> 7; both flags compare the final value
        let loco = LocomotorSetView {
            surfaces: 3,
            flag_15: 9,
        };
        assert!(pathfind_layer_connects_zones(&layer, &cells, &loco, 7, 7, &mut Scan));
        assert!(!pathfind_layer_connects_zones(&layer, &cells, &loco, 7, 8, &mut Scan));
        // destroyed == false -> never connects
        let live = PathfindLayerView {
            destroyed: false,
            ..layer
        };
        assert!(!pathfind_layer_connects_zones(&live, &cells, &loco, 7, 7, &mut Scan));
    }

    struct EqOps {
        sets: Vec<u16>,
        links: Vec<(u16, u16)>,
    }
    impl EquivalencySetOps for EqOps {
        fn make_set(&mut self, z: u16) {
            self.sets.push(z);
        }
        fn mask_used(&self, _variant: u32, zone: u32) -> bool {
            zone != 1 // zone 1 excluded
        }
        fn could_be_in_equiv_set(&self, _equiv: u32, zone: u32) -> bool {
            zone != 3
        }
        fn in_equiv_set(&self, _equiv: u32, first: u32, second: u32) -> bool {
            first + second != 5
        }
        fn link_sets(&mut self, a: u16, b: u16) {
            self.links.push((a, b));
        }
    }

    #[test]
    fn create_equivalency_set_links() {
        let mut first = 0xffffu16;
        let neighbors = [
            ZoneNeighborRecord { zone: 1, count: 0 }, // mask_used fails
            ZoneNeighborRecord { zone: 2, count: 0 }, // link 0-2
            ZoneNeighborRecord { zone: 5, count: 0 }, // in_equiv_set fails (0+5)
        ];
        let mut ops = EqOps {
            sets: Vec::new(),
            links: Vec::new(),
        };
        pathfind_zone_manager_create_equivalency_set(
            &mut first,
            3, // end: zones 0..3
            1,
            2,
            &mut ops,
            |z| {
                if z == 0 {
                    &neighbors[..]
                } else {
                    &[]
                }
            },
        );
        assert_eq!(first, 0);
        assert_eq!(ops.sets, vec![0, 1, 2]);
        assert_eq!(ops.links, vec![(0, 2)]);
    }

    #[test]
    fn create_equivalency_set_empty_end() {
        let mut first = 7u16;
        let mut ops = EqOps {
            sets: Vec::new(),
            links: Vec::new(),
        };
        pathfind_zone_manager_create_equivalency_set(&mut first, 0, 0, 0, &mut ops, |_| &[
            ZoneNeighborRecord { zone: 9, count: 0 },
        ]);
        assert_eq!(first, 0);
        assert!(ops.sets.is_empty());
        assert!(ops.links.is_empty());
    }

    #[test]
    fn adjust_horde_melee_destination() {
        // kind guard returns true without touching dest
        let mut dest = v3(5.0, 5.0, 5.0);
        let r = pathfinder_adjust_horde_melee_destination(
            true,
            false,
            &mut dest,
            0,
            0.0,
            0,
            |_, _, _| panic!("init must not run"),
            |_, _, _, _| panic!("find must not run"),
            |_, _, _, _| panic!("adjust must not run"),
        );
        assert!(r);

        // no find, no fallback -> false
        let mut dest = v3(15.0, 15.0, 0.0);
        let r = pathfinder_adjust_horde_melee_destination(
            false,
            true,
            &mut dest,
            1,
            2.5,
            4,
            |info, d, h| {
                assert_eq!(d.x, 15.0);
                assert_eq!(h, 2.5);
                info.layer = 1;
                info.has_fallback = 0;
            },
            |cell, radius, found, _info| {
                // cell centre of 15 with center=true is (1,1); budget passes through
                assert_eq!(*cell, PfCellPt { x: 1, y: 1 });
                assert_eq!(radius, 4);
                let _ = found;
                false
            },
            |_, _, _, _| unreachable!(),
        );
        assert!(!r);

        // fallback path: dest rewritten from m_1C through adjustCellCenter
        let r = pathfinder_adjust_horde_melee_destination(
            false,
            true,
            &mut dest,
            1,
            2.5,
            4,
            |info, _, _| {
                info.layer = 2;
                info.has_fallback = 1;
                info.fallback_cell = PfCellPt { x: 3, y: 4 };
            },
            |_, _, _, _| false,
            |out, center, target, layer| {
                assert!(center);
                assert_eq!(*target, PfCellPt { x: 3, y: 4 });
                assert_eq!(layer, 2);
                *out = v3(35.0, 45.0, 0.0);
            },
        );
        assert!(r);
        assert_eq!(dest.x, 35.0);
        assert_eq!(dest.y, 45.0);
    }

    #[test]
    fn adjust_to_nearest_ground_cell_paths() {
        // cell.x < 0 -> false, scan never runs
        let mut pos = v3(-5.0, 0.0, 0.0);
        let r = pathfinder_adjust_to_nearest_ground_cell(
            &mut pos,
            50,
            0,
            |_, c| {
                c.x = -1;
                c.y = 7;
            },
            |_, _, _, _| panic!("scan must not run"),
            |_, _| panic!("height must not run"),
        );
        assert!(!r);

        // success: x = found.x*10 + 1, z from terrain over the new x/y
        let mut pos = v3(0.0, 0.0, 0.0);
        let r = pathfinder_adjust_to_nearest_ground_cell(
            &mut pos,
            50,
            3,
            |_, c| {
                c.x = 0;
            },
            |cell, budget, found, ctx| {
                assert_eq!(cell.x, 0);
                assert_eq!(budget, 50);
                assert_eq!(ctx, 3);
                found.x = -2;
                found.y = 12;
                true
            },
            |x, y| {
                assert_eq!(x, -19.0); // (-2*10) + 1
                assert_eq!(y, 121.0);
                -3.5
            },
        );
        assert!(r);
        assert_eq!(pos, v3(-19.0, 121.0, -3.5));
    }

    #[test]
    fn adjust_to_nearest_valid_cell_budget_differs() {
        let mut pos = v3(0.0, 0.0, 0.0);
        let r = pathfinder_adjust_to_nearest_valid_cell(
            &mut pos,
            77,
            0,
            |_, c| c.x = 5,
            |_, budget, found, _| {
                assert_eq!(budget, 77);
                found.x = 1;
                found.y = 1;
                true
            },
            |_, _| 0.0,
        );
        assert!(r);
        assert_eq!(pos.x, 11.0);
    }

    #[test]
    fn fit_check_grid_and_checks() {
        // radius 0 -> no grid checks; the two big callees still run
        let mut info_seen = None;
        let r = pathfinder_position_fit_check(
            0x1111,
            &v3(10.0, 10.0, 0.0),
            true,
            0,
            1,
            5,
            0x40,
            true,
            false,
            0x77,
            [0xAA, 0xBB],
            |_, _, _| panic!("no cells with radius 0"),
            |_, _| true,
            |_, info| {
                info_seen = Some(*info);
                true
            },
            |obj, zero, info, layer, radius, upper, tail| {
                assert_eq!(obj, 0x1111);
                assert_eq!(zero, 0);
                assert_eq!(layer, 1);
                assert_eq!(radius, 0);
                assert_eq!(upper, 1); // radius + center(=true)
                assert_eq!(tail, 0);
                info.radius
            },
        );
        assert!(r);
        let seen = info_seen.expect("checkForMovement must have run");
        assert_eq!(seen.m_14, 16); // kind bit clear
        assert_eq!(seen.m_18, 0x77);
        assert_eq!(seen.center_in_cell, true);
        assert_eq!(seen.m_11, false);

        // null cell -> false before movement
        let r = pathfinder_position_fit_check(
            0,
            &v3(10.0, 10.0, 0.0),
            true,
            1,
            1,
            5,
            0x00,
            true,
            true,
            0,
            [0, 0],
            |_, x, y| {
                if x == 0 && y == 0 {
                    Some(CELL)
                } else {
                    None
                }
            },
            |_, _| true,
            |_, _| true,
            |_, _, _, _, _, _, _| 0,
        );
        assert!(!r);
    }

    static CELL: FitCell = FitCell;

    #[test]
    fn fit_check_movement_probe_fields() {
        let mut probe_seen = None;
        let r = pathfinder_position_fit_check(
            1,
            &v3(10.0, 10.0, 0.0),
            true,
            1,
            3,
            5,
            0x40,
            true,
            true,
            9,
            [0xAA, 0xBB],
            |layer, _, _| {
                assert_eq!(layer, 3);
                Some(CELL)
            },
            |probe, _| {
                probe_seen = Some(*probe);
                true
            },
            |_, _| true,
            |_, _, _, _, _, _, _| 5, // nonzero -> false
        );
        assert!(!r);
        assert_eq!(
            probe_seen.unwrap(),
            MovementProbe {
                seed0: 0xAA,
                seed1: 0xBB,
                ignore_template_flag: false, // 0x40 != 0
                priority_minus_1: 4,
                afbb: true,
            }
        );
    }

    #[test]
    fn get_overlap_goal_units_collects() {
        // footprint diameter 2 -> half 1 rest 1 around centre (0,0) with
        // center=false? Use center=true cell base from pos (0,0).
        let unit_a = GoalUnit { id: 10, skip: false };
        let unit_b = GoalUnit { id: 11, skip: false };
        let unit_c = GoalUnit { id: 12, skip: true };
        let node_c = OccupantNode {
            next: None,
            object: Some(&unit_c),
        };
        let node_b = OccupantNode {
            next: Some(&node_c),
            object: Some(&unit_b),
        };
        let node_a = OccupantNode {
            next: Some(&node_b),
            object: Some(&unit_a),
        };
        let cinfo = GoalCellInfo {
            occupants: Some(&node_a),
        };
        let cell = GoalCell {
            info: Some(&cinfo),
        };
        let mut counter = 0u32;
        let mut out = [0i32; 16];
        let count = pathfinder_get_overlap_goal_units(
            1,     // layer
            true,  // center flag
            false, // bridge
            1,     // diameter -> half 0, rest 1: single cell (0,0)
            &v3(0.0, 0.0, 0.0),
            &mut counter,
            false, // mover skip predicate (discarded)
            |layer| layer == 1,
            |layer, x, y| {
                assert_eq!(layer, 1);
                if x == 0 && y == 0 {
                    Some(&cell)
                } else {
                    None
                }
            },
            &mut out,
        );
        assert_eq!(count, 2); // c skipped by its flag
        assert_eq!(&out[..2], &[10, 11]);
        assert_eq!(counter, 1); // scan counter bumped once
    }


    #[test]
    fn count_overlap_scores_containers() {
        let a = HordeUnit {
            handle: 100,
            container: 9,
        };
        let b = HordeUnit {
            handle: 101,
            container: 8,
        };
        let self_u = HordeUnit {
            handle: 77,
            container: 9,
        };
        let nb_self = OccupantNode {
            next: None,
            object: Some(&self_u),
        };
        let nb_b = OccupantNode {
            next: Some(&nb_self),
            object: Some(&b),
        };
        let na = OccupantNode {
            next: Some(&nb_b),
            object: Some(&a),
        };
        let info = HordeCellInfo {
            horde_nodes: Some(&na),
        };
        let cell = HordeCell {
            info: Some(&info),
        };
        let count = pathfinder_count_overlap_horde_goal_units(
            77,
            9,
            1,
            true,
            false,
            1,
            &v3(0.0, 0.0, 0.0),
            |layer| layer == 1,
            |_, x, y| {
                if x == 0 && y == 0 {
                    Some(&cell)
                } else {
                    None
                }
            },
        );
        assert_eq!(count, 3); // a same container +1, b other container +2, self (77) skipped
        // null container short-circuits
        let c0 = pathfinder_count_overlap_horde_goal_units(
            77,
            0,
            1,
            true,
            false,
            1,
            &v3(0.0, 0.0, 0.0),
            |_| panic!("never"),
            |_, _, _| panic!("never"),
        );
        assert_eq!(c0, 0);
    }

    struct Arena<'a> {
        cells: &'a mut [PathNodeMirror],
    }
    impl<'a> Arena<'a> {
        fn get(&self, node: i32) -> Option<&PathNodeMirror> {
            if node <= 0 {
                None // handle 0 models null: the game wild-reads it
            } else {
                self.cells.get(node as usize)
            }
        }
    }
    impl<'a> PathNodeOps for Arena<'a> {
        fn position_of(&mut self, node: i32) -> Option<PfVec3> {
            self.get(node).map(|n| n.position)
        }
        fn next_of(&mut self, node: i32) -> Option<i32> {
            self.get(node).map(|n| n.next)
        }
        fn link_tail(&mut self, tail: i32, next: i32) {
            let n = &mut self.cells[tail as usize];
            n.next_optimized = next;
            n.next = next;
        }
        fn set_previous(&mut self, node: i32, previous: i32) {
            if let Some(n) = self.cells.get_mut(node as usize) {
                n.previous = previous;
            }
        }
    }

    #[test]
    fn splice_and_prepend_surgery() {
        // handle 1 (this head, reclaimed) -> 2 (m_selected, freed after the
        // loop); node2.next = 5; other: head 3, tail 4 splices before 5.
        let mut arena = vec![
            PathNodeMirror::default(), // 0 = null
            PathNodeMirror {
                next: 2,
                ..Default::default()
            }, // 1 this.m_path
            PathNodeMirror {
                next: 5,
                ..Default::default()
            }, // 2 this.m_selected
            PathNodeMirror::default(), // 3 other.m_path
            PathNodeMirror {
                next: -9,
                next_optimized: -9,
                position: v3(1.0, 2.0, 3.0),
                ..Default::default()
            }, // 4 other.m_path_tail
            PathNodeMirror {
                previous: -9,
                ..Default::default()
            }, // 5 selected.next
        ];
        let mut freed = Vec::new();
        let mut path = PathMirror {
            m_path: 1,
            m_path_tail: 2,
            m_selected: 2,
            m_distance: 12.5,
        };
        let mut other = PathMirror {
            m_path: 3,
            m_path_tail: 4,
            m_selected: 3,
            m_distance: 3.0,
        };
        let advanced = std::cell::Cell::new(false);
        path_splice_and_prepend(
            &mut path,
            Some(&mut other),
            Arena {
                cells: &mut arena,
            },
            |pos, stop| {
                advanced.set(true);
                // DoAdvanceAlongPath(&tail.m_position, false)
                assert_eq!(*pos, v3(1.0, 2.0, 3.0));
                assert!(!stop);
                true
            },
            |h| freed.push(h),
            |o| {
                assert_eq!(o.m_path, 0); // handed over, then zeroed
            },
        );
        assert!(advanced.get());
        assert_eq!(freed, vec![1, 2]); // reclaim loop + guarded selected free
        // other's tail spliced in front of selected.next
        assert_eq!(arena[4].next, 5);
        assert_eq!(arena[4].next_optimized, 5);
        assert_eq!(arena[5].previous, 4);
        assert_eq!(path.m_path, 3);
        assert_eq!(path.m_selected, 3);
        assert_eq!(path.m_path_tail, 2); // untouched by this row
        assert_eq!(path.m_distance, 0.0);
        assert_eq!(other.m_path, 0);
    }

    #[test]
    fn splice_and_prepend_early_exits() {
        let mut freed = Vec::new();
        let mut destroyed = 0;
        let mut path = PathMirror::default();
        let mut other = PathMirror {
            m_path: 3,
            m_path_tail: 0,
            ..Default::default()
        };
        path_splice_and_prepend(
            &mut path,
            Some(&mut other),
            Arena { cells: &mut [] },
            |_, _| panic!("advance must not run"),
            |h| freed.push(h),
            |o| {
                destroyed += 1;
                o.m_path = 0;
            },
        );
        assert_eq!(destroyed, 1);
        assert!(freed.is_empty());
        // null argument: destroy never runs
        destroyed = 0;
        path_splice_and_prepend(
            &mut path,
            None,
            Arena { cells: &mut [] },
            |_, _| panic!(),
            |_| panic!(),
            |_| panic!(),
        );
        assert_eq!(destroyed, 0);
    }

    #[test]
    fn dynamic_portal_offset_scale() {
        // transform identity
        let data = PortalModuleData {
            m_144: v3(3.0, 4.0, 5.0),
        };
        let obj = PortalObjectView {
            position: v3(0.0, 0.0, 0.0),
        };
        let mut out = PfVec3::default();
        dynamic_portal_footprint_offset(&data, &obj, |s, d| *d = *s, &mut out);
        // unit (1,0,0): pick = max(0, 1) = 1, scale 1, len 1 -> (3, 4, 5)
        assert_eq!(out, v3(3.0, 4.0, 5.0));

        // transformed axis = (4, 2) with object at (3, 1): local (1, 1);
        // pick 1 -> scale 1 -> len sqrt(2)
        let obj2 = PortalObjectView {
            position: v3(3.0, 1.0, 0.0),
        };
        let mut out = PfVec3::default();
        dynamic_portal_footprint_offset(
            &data,
            &obj2,
            |s, d| {
                // identity for the unit vector, then second call returns src
                if s.x == 1.0 && s.y == 0.0 && s.z == 0.0 {
                    *d = v3(4.0, 2.0, 0.0);
                } else {
                    *d = *s;
                }
            },
            &mut out,
        );
        let l = core::f32::consts::SQRT_2; // sqrt(1+1)
        assert!((out.x - 3.0 * l).abs() < 1e-5);
        assert!((out.y - 4.0 * l).abs() < 1e-5);
        assert_eq!(out.z, 5.0);

        // zero-length local vector: scale = 1/0 = inf, z = inf*0 = NaN
        let obj3 = PortalObjectView {
            position: v3(1.0, 0.0, 0.0),
        };
        let mut out = PfVec3::default();
        dynamic_portal_footprint_offset(
            &data,
            &obj3,
            |s, d| {
                if s.x == 1.0 && s.y == 0.0 && s.z == 0.0 {
                    *d = v3(1.0, 0.0, 0.0);
                } else {
                    *d = *s;
                }
            },
            &mut out,
        );
        assert!(out.x.is_nan()); // len NaN -> m_144 * NaN
    }

    #[test]
    fn line_context_init_fills_layout() {
        let mut info = LineContextInfo {
            m_58: -1,
            ..Default::default()
        };
        let mut split_calls = None;
        pathfind_line_context_init(
            &mut info,
            0x11,
            0x22,
            0x33,
            8, // inner 0x56C
            0, // inner 0x634
            true,
            false,
            |q| q.m_18 = -7,
            |obj, half, rest| {
                split_calls = Some(obj);
                *half = 6;
                *rest = 1;
            },
        );
        assert_eq!(info.m_00, 0x11);
        assert_eq!(info.m_04, 0x22);
        assert_eq!(info.query.m_18, -7); // ctor effect kept
        assert_eq!(info.m_58, 0);
        assert_eq!(info.block.arg_c, 0x33);
        assert_eq!(info.block.no_634, true);
        assert_eq!(info.block.afbb, 0);
        assert_eq!(info.block.ac62, 1);
        assert_eq!(info.block.budget_56c_m1, 7);
        assert_eq!(info.query.copy_1c, info.block);
        assert_eq!(info.query.m_11, 0);
        assert_eq!(info.query.m_14, 12);
        assert_eq!(info.query.m_0c, 6);
        assert_eq!(info.query.m_10, 1);
        assert_eq!(split_calls, Some(0x22));
    }
}
