//! Wave 47: `GameLogic/System` band — 16 rows (`Systems` tag).
//!
//! The band spans the fire-logic grid rebuilder, the terrain-resource
//! claim pair, the LivingWorld turn/phase orchestrators, the LinearCampaign
//! carryover vector-insert, the GameLogic scheduler wake path, the window
//! teardown slot, the army placer spawn search, the carryover-object
//! factory bookkeeping, the LivingWorld path search driver, and the AI
//! builder scorer.
//!
//! | addr | size | their symbol root | role |
//! |---|---|---|---|
//! | `0x00687015` | 399 | `FireLogicSystem::NewMap` | rebuild the 10-unit fire grid over the terrain extent and seed every cell from the terrain material table |
//! | `0x0075a4a3` | 380 | `TerrainResourceManager::isCellClaimable` | claim-state machine over a 16-byte resource cell (shroud gate, case 2/3 claimant scans, case-3 self-heal) |
//! | `0x006bc1da` | 351 | `LivingWorldLogic::GameLogic_tacticalBattleComplete` | settle every unit of the current battle by team, then complete it and notify listeners |
//! | `0x005ec1b4` | 349 | `LinearCampaign` carryover insert | STLport `vector<0xac record>::insert(position, first, last)` |
//! | `0x006b49a8` | 336 | `LivingWorldLogic` slot (`rva002B49A8`) | merge each player's unnamed armies that target the same region |
//! | `0x00776d49` | 329 | `GameLogic` slot (`rva00376D49`) | in-game window teardown + owned background-layout delete |
//! | `0x0064297f` | 323 | `GameLogic::friend_awakenUpdateModule` | move a sleeping update module into its phase vector and reschedule its frame |
//! | `0x0077f985` | 321 | `ArmyPlacer::GetFarthestSpawnPointFromPlayerStart` | pick the spawn/gather pair of highest cost among a region's entries |
//! | `0x006b3bcf` | 319 | `LivingWorldLogic::AdjustArmyTargetLocations` | release then re-reserve placement for every active army target |
//! | `0x0075aa43` | 316 | `TerrainResourceManager::tryToClaimCell` | claim/extend one cell against the state machine, counting attempts and successes |
//! | `0x0063d914` | 309 | `GameLogic::LivingWorldTacticalBattleComplete` | accumulate RTS stats per player, record the LivingWorld victor, then branch to the session/transition path |
//! | `0x006bd23e` | 303 | `LivingWorldLogic::CheckTurnPhaseTransitions` | turn-phase edge machine over `+0xf4`/`+0xf8` with listener notification |
//! | `0x0077dce8` | 294 | `CarryoverUnit::createSingleObject` | factory-create one carried object (factory as host call), apply experience/upgrades/record, hook the AI sink |
//! | `0x007efa0a` | 292 | `LivingWorldPathFinder::FindShortestPath` | open/closed search driver over the metric callback with path emit/count |
//! | `0x006bb15f` | 290 | `LivingWorldLogic` slot (`rva002BB15F`) | settle every delayed region victory (EVA event 21 on a plain local-player win), then clear the queue |
//! | `0x0099d089` | 288 | `LivingWorldAIBuilderScores` (`rva0059D089`) | castle-count-damped, defender-log-weighted builder score |
//!
//! Faithful-model conventions for this module:
//! - Rows that are orchestrators over foreign objects (battles, armies,
//!   regions, windows) keep their *local* bookkeeping — field stores,
//!   counters, guard order, loop shapes, phase machines — in the mirror,
//!   and model every foreign interaction as a host-trait method named by
//!   its row address or WorldBuilder-recovered role. Tests record the
//!   exact call sequence the decompiled body produces.
//! - Float math is IEEE single-precision as the image runs it (SSE units
//!   under `/arch:SSE`): products and sums are written in the decompiled
//!   association order; the `(int)ROUND(f)` conversions reproduce x87
//!   `fistp` (round-half-even, indefinite-integer `0x80000000` on NaN or
//!   out-of-range input) via [`x87_fistp_i32`].
//! - Pointer-difference idioms (`(end - begin) >> 2` with `& 0xfffffffc`
//!   gates, `* 0xac` element strides) are kept where the body computes
//!   them; where the model stores the elements themselves (`Vec`) the
//!   count is taken directly and the raw-dword form is documented.
//! - Constant dwords resolved from the image: `0x00bc2424` = `0.1f`
//!   (extent-to-cell scale, used by rows 1 and 16), `0x00bc26f8` /
//!   `0x00bc26f0` = `0.5f` (cell centres), `0x00bfb6e8` = `10.0f` (fire
//!   cell size), `0x00bbb8d8` = `1.0f`, `0x00bc8e28` = `6.0f`,
//!   `0x00bc28f4` = `2.0f`; the `FXParticleSystem::CategoryModuleInfo<0>`
//!   gate symbol in row 16 is the constant `0.0f`.

/// x87 `fistp` dword store: round-half-to-even, and the indefinite
/// integer `0x80000000` when the source is NaN, infinite, or outside the
/// dword range — exactly what the game's `(int)ROUND(x)` macro sites and
/// the `floor`-then-store chains of [`fire_logic_system_new_map`] rely
/// on. Rust's `as i32` *saturates*; the game does not, so the quirk is
/// modelled here rather than by the cast.
pub fn x87_fistp_i32(x: f32) -> i32 {
    const INDEFINITE: i32 = i32::MIN;
    if x.is_nan() || x.is_infinite() {
        return INDEFINITE;
    }
    if x < -2147483648.0f32 || x >= 2147483648.0f32 {
        return INDEFINITE;
    }
    // f32 round-half-even, then the exact float -> i32 conversion.
    let r = x.round_ties_even();
    r as i32
}

// ---------------------------------------------------------------------
// 0x00687015 — FireLogicSystem::NewMap
// ---------------------------------------------------------------------

/// One fire-logic cell: the 20-byte record at `m_storage` (+0x74) rows.
/// Layout from the body's stores: dword `+0x00` type, word `+0x04` fuel,
/// word `+0x06` check, dword `+0x08` the three fire bitfields, dword
/// `+0x0C` watcher-list head, dword `+0x10` object-node head. The
/// `+0x08` bitfields are `flammability` bits 0..9, `field10` bits 10..17,
/// `field18` bits 18..29 — the three masked read-modify-writes of the
/// body write exactly those windows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FireCell {
    pub cell_type: i32,
    pub fuel: u16,
    pub check: u16,
    pub bits: u32,
    pub watchers: i32,
    pub objects: i32,
}

/// One material entry: the four 24-byte records at `+0x10`, indexed by
/// the terrain material id. Only `+0x08` (word read for cell fuel),
/// `+0x0C` (12-bit field18), `+0x10` (10-bit field00) and `+0x14`
/// (8-bit field10) are read here.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FireMaterial {
    pub unknown00: i32,
    pub unknown04: i32,
    pub fuel: i32,
    pub field18: u32,
    pub field00: u32,
    pub field10: u32,
}

/// The grid members the row writes: `m_cells`/`m_storage` (+0x70/+0x74)
/// become the row/flat vectors (the body's row-pointer arithmetic
/// `m_cells[x] = m_storage + m_numCols * x * 0x14` is exactly a
/// row-major rectangle over a single flat allocation), plus the counts
/// at +0x78 (x/rows) and +0x7C (y/cols).
#[derive(Clone, Debug, Default)]
pub struct FireLogicGrid {
    /// `m_materials[4]` @ +0x10 (24-byte entries).
    pub materials: [FireMaterial; 4],
    /// `m_numRows` @ +0x78.
    pub num_rows: i32,
    /// `m_numCols` @ +0x7C.
    pub num_cols: i32,
    /// `m_cells` row starts @ +0x70 into `m_storage` @ +0x74.
    pub storage: Vec<FireCell>,
}

/// `0x00687015` — rebuild the fire grid over the terrain extent and seed
/// each cell centre from the terrain material.
///
/// Body order: grid teardown (`rva00286CC4` — the Vec replacement below
/// stands in for the two `operator new[]`/delete pair), the extent query
/// through `TheTerrainLogic` slot 0x30 (the caller supplies the high
/// corner as data), then
/// `num_rows = fistp(floor(extent_hi_x * 0.1f) as float) + 1` and the
/// same for `hi_y` into +0x78/+0x7C. The flat storage holds
/// `num_rows * num_cols` cells; every cell is zeroed (dwords at 0/8/12/16
/// plus the two words at 4/6 — the full 20 bytes), then the material id
/// at the cell centre `((x + 0.5) * 10.0, (y + 0.5) * 10.0)` is fetched
/// through the slot 0x60 lookup (returned byte masked `& 0xff`); for
/// `0 != type < 4` the material's `+0x08` word becomes the cell fuel and
/// its three dwords are merged into the cell's `+0x08` dword through
/// masks `0x3ffc0000` (`field18 << 18`), `0x3ff` (`field00`) and
/// `0x3fc00` (`field10 << 10`) — non-overlapping bitfields.
///
/// A negative or NaN extent still stores `fistp(...) + 1` (possibly
/// `0x80000000 + 1`); the build loops then simply run zero iterations,
/// matching the game's `0 < count` guards.
pub fn fire_logic_system_new_map(
    sys: &mut FireLogicGrid,
    extent_hi_x: f32,
    extent_hi_y: f32,
    mut material_at: impl FnMut(f32, f32) -> u8,
) {
    let floor_store = |h: f32| -> i32 {
        let prod = h * 0.1f32;
        x87_fistp_i32((prod as f64).floor() as f32)
    };
    sys.num_rows = floor_store(extent_hi_x).wrapping_add(1);
    sys.num_cols = floor_store(extent_hi_y).wrapping_add(1);

    // new Cell[rows*cols] then the row-pointer array; the mirror keeps
    // the flat rectangle only (row i starts at col_count*i by the body's
    // own arithmetic). A negative/wrapped count takes the zero-cell path,
    // as the game's `0 < m_numRows` do-while guard does.
    let cells = (sys.num_rows as i64) * (sys.num_cols as i64);
    sys.storage.clear();
    if cells > 0 && cells <= (i32::MAX as i64) * 32 {
        sys.storage.resize(cells as usize, FireCell::default());
    }

    if sys.num_rows <= 0 {
        return;
    }
    for row in 0..sys.num_rows {
        let world_x = ((row as f32) + 0.5f32) * 10.0f32;
        if sys.num_cols <= 0 {
            continue;
        }
        for col in 0..sys.num_cols {
            let idx = row as i64 * sys.num_cols as i64 + col as i64;
            let cell = &mut sys.storage[idx as usize];
            *cell = FireCell::default();
            let world_y = ((col as f32) + 0.5f32) * 10.0f32;
            let kind = material_at(world_x, world_y) as u32 & 0xff;
            if kind != 0 && kind < 4 {
                let m = sys.materials[kind as usize];
                cell.fuel = m.fuel as u16;
                let b = cell.bits;
                let b = b ^ ((m.field18.wrapping_shl(0x12) ^ b) & 0x3ffc_0000);
                let b = ((m.field00 ^ b) & 0x3ff) ^ b;
                cell.bits = (((m.field10.wrapping_shl(10)) ^ b) & 0x3fc00) ^ b;
            }
        }
    }
}

// ---------------------------------------------------------------------
// 0x0075a4a3 / 0x0075aa43 — TerrainResourceManager claim pair
// ---------------------------------------------------------------------

/// One terrain-resource cell: the 16-byte record array at +0x40 — an
/// 8-byte-claimant vector (stlport triple, modelled as the `Vec`) plus
/// the claim `state` dword at +0x0C. Claimant records are the pair
/// `(player id, object id)`; state meanings from the bodies: 0 free,
/// 1 blocked, 2 shared claim, 3 exclusive claim.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResourceCell {
    /// `values` stlport vector @ +0x00..+0x0C (8-byte POD records).
    pub claims: Vec<(i32, i32)>,
    /// `state` @ +0x0C.
    pub state: i32,
}

/// `0x0075a4a3` — may this cell be claimed.
///
/// Body order, verbatim: null-cell-array (`+0x40`), then `x < 0 ||
/// width34 <= x`, then `y < 0 || height38 <= y` (the decompiler keeps
/// three separate guards, not one range test). Then the shroud gate:
/// only when `!force && player > -1` is the cell centre
/// `((x + 0.5) * cell3C + origin1C, (y + 0.5) * cell3C + origin20, 0)`
/// measured; a shroud status of 2 fails immediately and the status is
/// kept for the case bodies (`0` when the gate never ran).
///
/// Cell index `(width * y + x) * 0x10` into the array (raw dword math;
/// the mirror takes `width*y + x` on the slice).
///
/// State machine (retail keeps cases 2 and 3 sharing the true-exit block
/// — reproduced by the shared tail below):
/// - 0 → claimable.
/// - 1 → not claimable.
/// - 2 → claimable unless (`!force && player != -1 && shroud == 0`),
///   in which case a claimant whose player id equals `player` blocks.
/// - 3 → when `!(force || shroud == 0)` claimable directly; otherwise an
///   empty claimant vector self-heals the state to 0 and is claimable;
///   with claims and `player > -1`, a dangling first claimant (object id
///   not found) erases the whole vector, sets state 0 and is claimable;
///   an object shroud status of 3 or 4 is claimable; else claimable only
///   with `force && (first.player == player || id == 0x7FFFF43 ||
///   id == first.object || own_id == first.object)` (`0x7FFFF43` is the
///   retail wildcard sentinel).
/// - any other state → not claimable.
#[allow(clippy::too_many_arguments)]
pub fn terrain_resource_manager_is_cell_claimable(
    cells: Option<&mut [ResourceCell]>,
    width: i32,
    height: i32,
    origin_x: f32,
    origin_y: f32,
    cell_size: f32,
    x: i32,
    y: i32,
    id: i32,
    own_id: i32,
    force: bool,
    player: i32,
    mut shroud_status: impl FnMut(i32, f32, f32) -> i32,
    mut object_exists: impl FnMut(i32) -> bool,
    mut object_shroud_status: impl FnMut(i32) -> i32,
) -> bool {
    if cells.is_none() || x < 0 || width <= x || y < 0 || height <= y {
        return false;
    }
    let mut shroud = 0i32;
    if !force && -1 < player {
        let px = ((x as f32) + 0.5f32) * cell_size + origin_x;
        let py = ((y as f32) + 0.5f32) * cell_size + origin_y;
        shroud = shroud_status(player, px, py);
        if shroud == 2 {
            return false;
        }
    }
    let cells = cells.unwrap();
    let idx = (width.wrapping_mul(y).wrapping_add(x)) as usize;
    let cell = &mut cells[idx];
    match cell.state {
        0 => true,
        1 => false,
        2 => {
            // body: the claimant scan runs only for `!force && player != -1
            // && shroud == 0`; every other case-2 cell is claimable
            if !force
                && player != -1
                && shroud == 0
                && cell.claims.iter().any(|c| c.0 == player)
            {
                return false;
            }
            true
        }
        3 => {
            if !(force || shroud == 0) {
                return true;
            }
            if cell.claims.is_empty() {
                cell.state = 0;
                return true;
            }
            if -1 < player {
                let first = cell.claims[0];
                if !object_exists(first.1) {
                    // erase(first, last) — the whole range — then state 0
                    cell.claims.clear();
                    cell.state = 0;
                    return true;
                }
                let status = object_shroud_status(player);
                if status == 3 || status == 4 {
                    return true;
                }
            }
            let first = cell.claims[0];
            force
                && (first.0 == player
                    || id == 0x07ff_f43
                    || id == first.1
                    || own_id == first.1)
        }
        _ => false,
    }
}

// ---------------------------------------------------------------------
// 0x005ec1b4 — LinearCampaign carryover vector insert
// ---------------------------------------------------------------------

/// One carryover record: the 0xac (172-byte) element the retail vector
/// instantiates. Its copy/assign/destroy thunks (0x005eba30 pair copy,
/// 0x005eb831 backward copy, 0x005eb9c8 destroy, 0x005ebacf forward
/// copy) are opaque at this row, so the record stays an opaque byte
/// block; the row is pure element-block movement.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CarryoverRecord {
    pub bytes: [u8; 172],
}

impl Default for CarryoverRecord {
    fn default() -> Self {
        Self { bytes: [0u8; 172] }
    }
}

/// `0x005ec1b4` — STLport 4.5.3 `vector::insert(position, first, last)`
/// for the 172-byte carryover record (the ECX vector is the +0x00/+0x04/
/// +0x08 start/finish/capacity triple; positions and the source range
/// arrive as pointers, here as a slice index and a slice).
///
/// Exact body structure (every count in the game is a byte difference
/// divided by 0xac — signed division, then unsigned comparisons):
/// - empty source range: return.
/// - `capacity - size >= n`: in-place paths.
///   - `elems_after > n`: duplicate the last `n` elements at the tail,
///     `finish += n`, shift `[position, old_finish - n)` up by `n` with
///     the backward copy, then copy the source over `position`.
///   - else: append `source[elems_after..]` (the `mid..last` block),
///     `finish += n - elems_after`; append the `[position, old_finish)`
///     block (`elems_after` elements), `finish += elems_after`; copy
///     `source[..elems_after]` over `position`.
/// - else: reallocate with the stlport growth rule
///   `new_len = old_size + max(old_size, n)` — copy `start..position`,
///   the source range, and `position..finish` into the new buffer, then
///   destroy and free the old one (`_M_clear`) and set the triple. The
///   mirror keeps that capacity observable: after the call
///   `vec.capacity()` is the old capacity on the in-place paths and
///   `old_size + max(old_size, n)` on the realloc path.
pub fn linear_campaign_carryover_insert(
    vec: &mut Vec<CarryoverRecord>,
    position: usize,
    source: &[CarryoverRecord],
) {
    let n = source.len();
    if n == 0 {
        return;
    }
    let size = vec.len();
    if vec.capacity() - size >= n {
        let elems_after = size - position;
        if elems_after > n {
            // uninitialized_copy(finish - n, finish, finish)
            vec.extend_from_within(size - n..size);
            // copy_backward(position, old_finish - n, old_finish)
            vec.copy_within(position..size - n, position + n);
            // copy(first, last, position)
            for i in 0..n {
                vec[position + i] = source[i];
            }
        } else {
            // uninitialized_copy(mid, last, finish); mid = first + elems_after
            vec.extend_from_slice(&source[elems_after..]);
            // uninitialized_copy(position, old_finish, finish)
            vec.extend_from_within(position..size);
            // copy(first, mid, position)
            for i in 0..elems_after {
                vec[position + i] = source[i];
            }
        }
    } else {
        let new_len = size + size.max(n);
        let mut fresh: Vec<CarryoverRecord> = Vec::with_capacity(new_len);
        fresh.extend_from_slice(&vec[..position]);
        fresh.extend_from_slice(source);
        fresh.extend_from_slice(&vec[position..size]);
        // _M_clear + _M_set: the old buffer's records are destroyed with
        // the old Vec; the new triple is start / new_finish / start + len.
        *vec = fresh;
    }
}

// ---------------------------------------------------------------------
// 0x006bc1da — LivingWorldLogic::GameLogic_tacticalBattleComplete
// ---------------------------------------------------------------------

/// The two LivingWorldLogic fields this row writes: +0xB6 (tactical
/// battle completed, byte store 1) and +0xB8 (battle region index,
/// dword store -1 — written whether or not the settle block ran).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TacticalBattleFlags {
    pub tactical_battle_completed: u8,
    pub battle_region_index: i32,
}

/// Foreign interactions of the settle walk, by row identity:
/// `rva002B2B2D` (the logic's current battle region), `rva0020E6B7`
/// (region manager +0xB0's current battle), the battle's +0x38 side,
/// `rva007F458A` (team for a side), the side vector
/// `(+0x1C - +0x18) / 0x1C` (signed division), `rva003F4DAE/003F468D/
/// 003F4DCA/003F48FE` (army count / army / unit count / unit), the
/// army's team at +0x34, the unit template name's emptiness (+0x18),
/// `ConsumeGarrisonUnitsFromRTS` (0x002BBD52), `rva002B8A9F` (destroy),
/// the owner query/transfer pair 0x003190A5/0x00319A12, the +0x10C
/// carryover push_back (`FUN_008dfcb0`), and the battle-complete/c
/// listener tail (`rva002BAF7E`, then the listener `forEach` rowed at
/// 0x002B616F).
pub trait TacticalBattleHost {
    fn battle_region(&mut self) -> i32;
    fn current_battle(&mut self) -> i32;
    fn battle_side(&mut self, battle: i32) -> i32;
    fn team_for_side(&mut self, side: i32) -> i32;
    fn side_count(&mut self, battle: i32) -> i32;
    fn army_count(&mut self, battle: i32, side: i32) -> i32;
    fn army_at(&mut self, battle: i32, side: i32, a: i32) -> i32;
    fn unit_count(&mut self, battle: i32, side: i32, a: i32) -> i32;
    fn unit_at(&mut self, battle: i32, side: i32, a: i32, u: i32) -> i32;
    fn army_team(&mut self, army: i32) -> i32;
    fn unit_name_empty(&mut self, unit: i32) -> bool;
    fn consume_garrison(&mut self, unit: i32, army: i32, battle: i32);
    fn destroy_unit(&mut self, unit: i32);
    fn unit_owner_query(&mut self, unit: i32) -> bool;
    fn unit_owner_release(&mut self, unit: i32);
    fn push_carryover_unit(&mut self, unit: i32);
    fn on_battle_complete(&mut self, battle: i32);
    fn notify_battle_listeners(&mut self, battle: i32);
}

/// `0x006bc1da` — settle every unit of the current tactical battle.
///
/// The region query runs first (its result gates the walk but is not
/// otherwise used), then +0xB6 is set to 1, then the battle is fetched
/// and the three-way guard (`battle != 0 && battle[+0x38] != -1 &&
/// region != 0`) decides whether the side/army/unit walk runs. The team
/// comes from the battle's own side; a unit whose *army* is on that team
/// and has an empty template name feeds the garrison; a unit off-team
/// with an empty name is destroyed, and off-team with a name optionally
/// releases its owner before going onto the +0x10C carryover list. The
/// complete/listener calls are inside the guard; the +0xB8 = -1 store is
/// outside it.
pub fn living_world_logic_game_logic_tactical_battle_complete(
    flags: &mut TacticalBattleFlags,
    host: &mut dyn TacticalBattleHost,
) {
    let region = host.battle_region();
    flags.tactical_battle_completed = 1;
    let battle = host.current_battle();
    let side0 = host.battle_side(battle);
    if battle != 0 && side0 != -1 && region != 0 {
        let team = host.team_for_side(side0);
        let sides = host.side_count(battle);
        let mut side = 0i32;
        while side < sides {
            let armies = host.army_count(battle, side);
            let mut a = 0i32;
            while a < armies {
                let army = host.army_at(battle, side, a);
                let units = host.unit_count(battle, side, a);
                let mut u = 0i32;
                while u < units {
                    let unit = host.unit_at(battle, side, a, u);
                    if host.army_team(army) == team {
                        if host.unit_name_empty(unit) {
                            host.consume_garrison(unit, army, battle);
                        }
                    } else if host.unit_name_empty(unit) {
                        host.destroy_unit(unit);
                    } else {
                        if host.unit_owner_query(unit) {
                            host.unit_owner_release(unit);
                        }
                        host.push_carryover_unit(unit);
                    }
                    u += 1;
                }
                a += 1;
            }
            side += 1;
        }
        host.on_battle_complete(battle);
        host.notify_battle_listeners(battle);
    }
    flags.battle_region_index = -1;
}

// ---------------------------------------------------------------------
// 0x006b49a8 — LivingWorldLogic army-region merge slot (rva002B49A8)
// ---------------------------------------------------------------------

/// Foreign views of the merge walk: the region-manager (+0xB0) member
/// field at +0x08 holding the army-set container, whose element vector
/// sits at +0x2C (`(end - begin) >> 2` count); the region record's
/// player id at +0x13C (-1 skips); the logic's player vector
/// +0x8C/+0x90; the player eligibility test 0x002E0BC0 (called with the
/// region's player id); each player's army span at +0x1B8/+0x1BC; the
/// army name (+0x18) emptiness; the army's target region 0x00318C32;
/// the merge 0x00319904; the vector `erase` at the iterator
/// (`FUN_006E10FE`, returns the iterator at the erased slot); and the
/// deleting destructor pair (`virtual dtor(0)` + `FUN_0042FD60` free).
pub trait UnnamedArmyMergeHost {
    fn army_set(&mut self) -> i32;
    fn set_count(&mut self, set: i32) -> i32;
    fn set_at(&mut self, set: i32, i: i32) -> i32;
    fn region_player(&mut self, region: i32) -> i32;
    fn player_count(&mut self) -> i32;
    fn player_at(&mut self, i: i32) -> i32;
    fn player_accepts(&mut self, player: i32, region_player: i32) -> bool;
    fn army_count(&mut self, player: i32) -> i32;
    fn army_at(&mut self, player: i32, j: i32) -> i32;
    fn army_name_empty(&mut self, army: i32) -> bool;
    fn army_region(&mut self, army: i32) -> i32;
    fn merge_armies(&mut self, first: i32, army: i32);
    fn erase_army(&mut self, player: i32, j: i32);
    fn delete_army(&mut self, army: i32);
}

/// `0x006b49a8` — merge unnamed armies that target the same region.
///
/// For every army-set region whose +0x13C player is not -1, and every
/// player accepting that player id, walk the player's +0x1B8 army span
/// keeping the first army that is unnamed *and* targets the region;
/// each later such army is merged into the kept one, erased from the
/// span (the iterator stays put — `erase` returns the slot of the next
/// element), and deleted. Armies that fail either test simply advance.
/// All counts are re-read every iteration exactly like the body's
/// reloads of `*local_c` and the span bounds.
pub fn living_world_logic_merge_unnamed_armies(host: &mut dyn UnnamedArmyMergeHost) {
    let set = host.army_set();
    if set == 0 {
        return;
    }
    let mut i = 0i32;
    while i < host.set_count(set) {
        let region = host.set_at(set, i);
        let rp = host.region_player(region);
        if rp != -1 {
            let mut p = 0i32;
            while p < host.player_count() {
                let player = host.player_at(p);
                if player != 0 && host.player_accepts(player, rp) {
                    let mut first = 0i32;
                    let mut j = 0i32;
                    while j < host.army_count(player) {
                        let army = host.army_at(player, j);
                        if !host.army_name_empty(army) || host.army_region(army) != region {
                            j += 1;
                        } else if first == 0 {
                            first = army;
                            j += 1;
                        } else {
                            host.merge_armies(first, army);
                            host.erase_army(player, j);
                            host.delete_army(army);
                        }
                    }
                }
                p += 1;
            }
        }
        i += 1;
    }
}

// ---------------------------------------------------------------------
// 0x00776d49 — GameLogic window cleanup slot (rva00376D49)
// ---------------------------------------------------------------------

/// The two members the slot owns: the background window layout at
/// +0x1B8 and its pending byte at +0x1BC.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GameLogicWindowBits {
    pub background: u32,
    pub background_pending: u8,
}

/// The teardown sequence's external calls, in body order. The first
/// block is unconditional subsystem churn (tribute screen, chat windows,
/// the two Apt enables each followed by the Apt window manager's slot
/// 0x28 through `*DAT_00DFE4CC`, the window manager `*DAT_00DFEF1C` slot
/// 0x28), then the OptionsMenu hide: `nameToKey` for both names, two
/// `winGetWindowFromId(0, key)` calls (slot 0xF0), and — when the
/// parent window resolves — `winSendSystemMsg(window, 0x4008, button,
/// buttonID)` (slot 0xE8). The guarded enables read globals
/// `DAT_00E05FAC` (strategic message box: `FUN_0094CBEF(1)`),
/// `DAT_00E04908` and `DAT_00E04910`.
pub trait WindowCleanupEvents {
    fn rva_008e400d(&mut self);
    fn rva_0090e9d3(&mut self);
    fn rva_008e855c(&mut self);
    fn rva_008e84ab(&mut self);
    fn rva_00911730(&mut self);
    fn hide_purchase_science(&mut self);
    fn hide_special_power_shortcut(&mut self);
    fn rva_004cf155(&mut self);
    fn rva_009cb265(&mut self);
    fn strategic_message_box_hide(&mut self);
    fn apt_tribute_enable(&mut self);
    fn apt_quit_enable(&mut self);
    fn apt_window_manager_slot28(&mut self);
    fn window_manager_slot28(&mut self);
    fn options_menu_back_click(&mut self);
    fn destroy_windows(&mut self, layout: u32);
    fn destructor_0(&mut self, layout: u32) -> u32;
    fn free(&mut self, ptr: u32);
    fn rva_00837e9c_zero(&mut self);
}

/// `0x00776d49` — in-game window teardown.
///
/// The tail is the row's only local bookkeeping and is mirrored field
/// for field: when the +0x1BC pending byte is set it is cleared first,
/// then a null +0x1B8 takes `FUN_00837E9C(0)`, while a live layout runs
/// `destroyWindows` (vtable slot 0x20), **re-reads** +0x1B8 (the body's
/// reload survives the destroy call — a dtor that clears the field
/// changes which arm runs), and either frees literal 0 or runs
/// `destructor(0)` (slot 0x04) and frees its returned pointer through
/// `FUN_0042FD60`. The final +0x1B8 = 0 store runs unconditionally.
pub fn game_logic_window_cleanup(
    bits: &mut GameLogicWindowBits,
    strategic_box_present: bool,
    tribute_window_present: bool,
    quit_menu_present: bool,
    ev: &mut dyn WindowCleanupEvents,
) {
    ev.rva_008e400d();
    ev.rva_0090e9d3();
    ev.rva_008e855c();
    ev.rva_008e84ab();
    ev.rva_00911730();
    ev.hide_purchase_science();
    ev.hide_special_power_shortcut();
    ev.rva_004cf155();
    ev.rva_009cb265();
    if strategic_box_present {
        ev.strategic_message_box_hide();
    }
    if tribute_window_present {
        ev.apt_tribute_enable();
        ev.apt_window_manager_slot28();
    }
    if quit_menu_present {
        ev.apt_quit_enable();
        ev.apt_window_manager_slot28();
    }
    ev.window_manager_slot28();
    ev.options_menu_back_click();

    if bits.background_pending != 0 {
        bits.background_pending = 0;
        if bits.background == 0 {
            ev.rva_00837e9c_zero();
        } else {
            ev.destroy_windows(bits.background);
            let p = bits.background;
            if p == 0 {
                ev.free(0);
            } else {
                let q = ev.destructor_0(p);
                ev.free(q);
            }
        }
    }
    bits.background = 0;
}

// ---------------------------------------------------------------------
// 0x0064297f — GameLogic::friend_awakenUpdateModule
// ---------------------------------------------------------------------

pub const UPDATE_SLEEP_FOREVER: u32 = 0x3fff_ffff;

/// One update module's scheduler fields as this row touches them:
/// next-call frame at +0x14, index-in-logic at +0x18, phase-in-logic at
/// +0x1C.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AwakenModule {
    pub next_call_frame: u32,
    pub index_in_logic: i32,
    pub phase_in_logic: i32,
}

/// The GameLogic scheduler members: frame at +0x40 (read through the
/// `DAT_00dfe78c` singleton), the four phase vectors at +0xC8 (twelve
/// bytes each — `iVar6 * 0xc + 0xc8`), the sleeping vector at
/// +0xF8/+0xFC, and the `current` pointer at +0x104.
#[derive(Clone, Debug, Default)]
pub struct AwakenScheduler {
    pub frame: u32,
    pub phase_updates: [Vec<usize>; 4],
    pub sleeping: Vec<usize>,
    pub current: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AwakenOutcome {
    /// one of the three entry guards rejected the call
    Rejected,
    /// object not in the game list but the module claims an index —
    /// the retail "fatal error! sleepy update module index mismatch"
    Fatal,
    /// the module moved sleeping -> phase `p` at index `i`
    Awakened { new_phase: i32, new_index: i32 },
    /// the module was already scheduled; only the frame was refreshed
    Rescheduled,
}

/// `0x0064297f` — wake path for a sleepy update module.
///
/// Entry guards, in body order, all rejecting: the module is the
/// scheduler's `current` (+0x104); `when` already equals the module's
/// next-call frame; or the three-part same-frame test (frame non-zero,
/// next-call frame equals frame, and `when == frame + 1` in unsigned
/// wrapping arithmetic).
///
/// With the owner object in the object list (+0xAC list membership —
/// `Object::isInList`, modelled as the bool the caller's lookup
/// produces): when `phase_in_logic < 0 && (when as i32) <
/// UPDATE_SLEEP_FOREVER` the module leaves the sleeping vector — the
/// swap-with-back runs only when `index < sleeping.len() - 1` and the
/// swapped module gets **phase -1** (its re-sleep) plus the new index,
/// while `pop_back` (end -= 4) runs unconditionally — then it lands on
/// `phase_updates[getUpdatePhase()]` (virtual slot 0x30; the mirror
/// takes it as a callback) with index = the vector's old size. The
/// frame store then clamps **signed**: `when > 0x3fffffff` (as i32)
/// becomes 0x3fffffff, so a 0x80000000-class frame survives verbatim.
/// Without list membership: an index other than -1 is the fatal-debug
/// path (debug facility rows 0x00438790/0x00DE0880 — no frame store),
/// otherwise just the clamped frame store.
pub fn game_logic_friend_awaken_update_module(
    modules: &mut [AwakenModule],
    sched: &mut AwakenScheduler,
    m: usize,
    when: u32,
    obj_in_object_list: bool,
    mut get_update_phase: impl FnMut(&mut AwakenModule) -> i32,
) -> AwakenOutcome {
    let frame = sched.frame;
    if sched.current == Some(m) {
        return AwakenOutcome::Rejected;
    }
    if when == modules[m].next_call_frame {
        return AwakenOutcome::Rejected;
    }
    if frame != 0
        && modules[m].next_call_frame == frame
        && when == frame.wrapping_add(1)
    {
        return AwakenOutcome::Rejected;
    }
    let phase = modules[m].phase_in_logic;
    let idx = modules[m].index_in_logic;
    if obj_in_object_list {
        if phase < 0 && (when as i32) < UPDATE_SLEEP_FOREVER as i32 {
            let new_phase = get_update_phase(&mut modules[m]);
            if idx < sched.sleeping.len() as i32 - 1 {
                let moved = sched.sleeping[sched.sleeping.len() - 1];
                sched.sleeping[idx as usize] = moved;
                modules[moved].phase_in_logic = -1;
                modules[moved].index_in_logic = idx;
            }
            sched.sleeping.pop();
            let new_index = sched.phase_updates[new_phase as usize].len() as i32;
            sched.phase_updates[new_phase as usize].push(m);
            modules[m].phase_in_logic = new_phase;
            modules[m].index_in_logic = new_index;
            let mut w = when;
            if (w as i32) > UPDATE_SLEEP_FOREVER as i32 {
                w = UPDATE_SLEEP_FOREVER;
            }
            modules[m].next_call_frame = w;
            return AwakenOutcome::Awakened {
                new_phase,
                new_index,
            };
        }
        let mut w = when;
        if (w as i32) > UPDATE_SLEEP_FOREVER as i32 {
            w = UPDATE_SLEEP_FOREVER;
        }
        modules[m].next_call_frame = w;
        AwakenOutcome::Rescheduled
    } else if idx != -1 {
        AwakenOutcome::Fatal
    } else {
        let mut w = when;
        if (w as i32) > UPDATE_SLEEP_FOREVER as i32 {
            w = UPDATE_SLEEP_FOREVER;
        }
        modules[m].next_call_frame = w;
        AwakenOutcome::Rescheduled
    }
}

// ---------------------------------------------------------------------
// 0x0077f985 — ArmyPlacer::GetFarthestSpawnPointFromPlayerStart
// ---------------------------------------------------------------------

/// One entry of the region's +0x1A8 record vector (0x18-byte stride,
/// the region id read at record +0x08).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SpawnRecord {
    pub region_id: i32,
}

/// The region argument: its +0x12C key and the +0x1A8..+0x1AC record
/// vector.
#[derive(Clone, Debug, Default)]
pub struct SpawnRegion {
    pub region_key: i32,
    pub records: Vec<SpawnRecord>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SpawnPoint {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// The opaque pair machinery of the row: `rva0037F4EA(key)` +
/// `rva0037F6EA(this, player)` build the player-start record (success
/// = its `m_1c`); `rva0020EAF6` looks up each entry's region;
/// `rva0037F4EA(key)` + `rva0037F87A` build the other-region record;
/// `rva0037F950(start, other)` evaluates the pair — its `m_44` flag and
/// `m_40` cost plus the pair's 8-dword farthest record (spawn at
/// `m_04..`, gather at `m_10..`).
pub trait FarthestSpawnHost {
    fn prepare_start(&mut self, region_key: i32, player: i32) -> bool;
    fn lookup_region(&mut self, region_id: i32) -> i32;
    fn prepare_other(&mut self, other_region_key: i32);
    fn evaluate_pair(&mut self) -> Option<(f32, SpawnPoint, SpawnPoint)>;
}

/// `0x0077f985` — farthest spawn point from the player start.
///
/// Guard: the start record must succeed and the region's record vector
/// (`(+0x1AC - +0x1A8) / 0x18`) must be non-empty. Best-tracking:
/// `highest = 0.0f`, then per record with a resolvable region, the pair
/// update needs `pair.ok && highest <= cost` — an ordinary IEEE `<=`,
/// so a NaN pair cost never wins. Final gate is `0.0f < highest` (NaN
/// highest fails); outputs are the winner's spawn/gather points.
pub fn army_placer_get_farthest_spawn_point_from_player_start(
    region: &SpawnRegion,
    player: i32,
    spawn: &mut [f32; 3],
    gather: &mut [f32; 3],
    host: &mut dyn FarthestSpawnHost,
) -> bool {
    if !host.prepare_start(region.region_key, player) || region.records.is_empty() {
        return false;
    }
    let mut highest = 0.0f32;
    let mut best: Option<(SpawnPoint, SpawnPoint)> = None;
    for rec in &region.records {
        let other = host.lookup_region(rec.region_id);
        if other == 0 {
            continue;
        }
        host.prepare_other(other);
        if let Some((cost, s, g)) = host.evaluate_pair() {
            if highest <= cost {
                highest = cost;
                best = Some((s, g));
            }
        }
    }
    if 0.0f32 < highest {
        let (s, g) = best.unwrap();
        spawn[0] = s.x;
        spawn[1] = s.y;
        spawn[2] = s.z;
        gather[0] = g.x;
        gather[1] = g.y;
        gather[2] = g.z;
        return true;
    }
    false
}

// ---------------------------------------------------------------------
// 0x006b3bcf — LivingWorldLogic::AdjustArmyTargetLocations
// ---------------------------------------------------------------------

/// Both passes walk the same nested players(+0x8C/+0x90) x player
/// armies(+0x1B8/+0x1BC) spans; pass one releases placement
/// (`rva00318C32` region + `rva003F0FA3` release), pass two re-reserves
/// (`rva00318FA1` connection + `rva003F26AA` spot finder + `rva0031986B`
/// target store with the two-word pair and tag 0).
pub trait ArmyTargetHost {
    fn player_count(&mut self) -> i32;
    fn player_at(&mut self, i: i32) -> i32;
    fn army_count(&mut self, player: i32) -> i32;
    fn army_at(&mut self, player: i32, j: i32) -> i32;
    fn army_active(&mut self, army: i32) -> bool;
    fn army_home_region(&mut self, army: i32) -> i32;
    fn region_release_army(&mut self, region: i32, army: i32);
    fn army_target_connection(&mut self, army: i32) -> i32;
    fn connection_find_spot(&mut self, connection: i32, army: i32) -> Option<(i32, i32)>;
    fn army_set_target(&mut self, army: i32, a: i32, b: i32);
}

/// `0x006b3bcf` — release, then re-reserve, every active army's target.
///
/// Two full passes (counts re-evaluated per iteration like the body's
/// span reloads). Pass one: active army -> region (unconditionally
/// released when non-null). Pass two: active army -> connection, and
/// when the connection finds a spot the army's target pair is stored
/// with tag 0.
pub fn living_world_logic_adjust_army_target_locations(host: &mut dyn ArmyTargetHost) {
    for pass in 0..2 {
        let mut i = 0i32;
        while i < host.player_count() {
            let player = host.player_at(i);
            let mut j = 0i32;
            while j < host.army_count(player) {
                let army = host.army_at(player, j);
                if host.army_active(army) {
                    if pass == 0 {
                        let region = host.army_home_region(army);
                        if region != 0 {
                            host.region_release_army(region, army);
                        }
                    } else {
                        let conn = host.army_target_connection(army);
                        if conn != 0 {
                            if let Some((a, b)) = host.connection_find_spot(conn, army) {
                                host.army_set_target(army, a, b);
                            }
                        }
                    }
                }
                j += 1;
            }
            i += 1;
        }
    }
}

// ---------------------------------------------------------------------
// 0x0075aa43 — TerrainResourceManager::tryToClaimCell
// ---------------------------------------------------------------------

/// Outcome of the `findObjectByID` + `getControllingPlayer` chain
/// (rows 0x00249DC5 / 0x0028AFA9; the player id sits at player +0x54).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClaimObjectLookup {
    NotFound,
    NoController,
    Player(i32),
}

/// `0x0075aa43` — claim one cell with attempt/success accounting.
///
/// Body order: the null cell array (+0x40) returns before anything;
/// the attempt counter is the game's two-counter functor view at
/// +0x10/+0x14 (parameters here, both raw dword increments). Then
/// `isCellClaimable(x, y, a, id, exclusive, -1)` — called with player
/// -1, so its shroud gate never runs — then the object/controller chain,
/// then the cell at `width * y + x` by state:
/// - 0: state becomes `(char)exclusive + 2` (3 or 2 — the body computes
///   the bool-to-int store), the claimant vector is cleared, the
///   `(player, id)` claim pushed, success counts.
/// - 2 exclusive: clear, push, state 3, success. Shared: a scan for an
///   existing claim by this player returns without counting; otherwise
///   push + success.
/// - 3 shared: return untouched. Exclusive with an existing first
///   claim: overwrite the first claimant pair in place and count.
///   Exclusive but empty: push **without** counting (the body's last
///   pre-LAB push has no success store — pinned by tests).
/// - any other state: return.
#[allow(clippy::too_many_arguments)]
pub fn terrain_resource_manager_try_to_claim_cell(
    mut cells: Option<&mut [ResourceCell]>,
    width: i32,
    height: i32,
    origin_x: f32,
    origin_y: f32,
    cell_size: f32,
    cell_x: i32,
    cell_y: i32,
    a: i32,
    id: i32,
    exclusive: bool,
    attempts: &mut i32,
    successes: &mut i32,
    mut find_object_player: impl FnMut(i32) -> ClaimObjectLookup,
) {
    if cells.is_none() {
        return;
    }
    *attempts = attempts.wrapping_add(1);
    if !terrain_resource_manager_is_cell_claimable(
        cells.as_deref_mut(),
        width,
        height,
        origin_x,
        origin_y,
        cell_size,
        cell_x,
        cell_y,
        a,
        id,
        exclusive,
        -1,
        |_p, _x, _y| 0,
        |_| false,
        |_| 0,
    ) {
        return;
    }
    let claimer = match find_object_player(id) {
        ClaimObjectLookup::NotFound => return,
        ClaimObjectLookup::NoController => return,
        ClaimObjectLookup::Player(p) => p,
    };
    let cells = cells.unwrap();
    let idx = (width.wrapping_mul(cell_y).wrapping_add(cell_x)) as usize;
    let cell = &mut cells[idx];
    match cell.state {
        0 => {
            cell.state = if exclusive { 3 } else { 2 };
            cell.claims.clear();
            cell.claims.push((claimer, id));
            *successes = successes.wrapping_add(1);
        }
        2 => {
            if exclusive {
                cell.claims.clear();
                cell.claims.push((claimer, id));
                cell.state = 3;
                *successes = successes.wrapping_add(1);
            } else {
                for claim in cell.claims.iter() {
                    if claim.0 == claimer {
                        return;
                    }
                }
                cell.claims.push((claimer, id));
                *successes = successes.wrapping_add(1);
            }
        }
        3 => {
            if !exclusive {
                return;
            }
            if cell.claims.is_empty() {
                cell.claims.push((claimer, id));
            } else {
                cell.claims[0] = (claimer, id);
                *successes = successes.wrapping_add(1);
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------
// 0x0063d914 — GameLogic::LivingWorldTacticalBattleComplete
// ---------------------------------------------------------------------

/// GameLogic-side stores of this row: the victor player index at +0x1B0.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LivingWorldBattleCompleteState {
    pub living_world_victor_index: i32,
}

/// The terminal branch the row takes after the victor stores.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BattleCompleteTail {
    /// `FUN_00776E92(1, 0)` + the transition (0x0023D0CD)
    ClearAndTransition,
    /// `GameLogic::sendEndSessionMsg` (0x0023D29F)
    EndSessionMsg,
    /// living-world manager slot (0x00211FA8)
    ManagerMsg,
}

/// Foreign chain in body order: the army-summary save over the object
/// list (0x0040F19D, GameLogic +0xAC), the current battle
/// (`rva0020E6B7`), the player-mask functor `FUN_006A7BC9(&mask)`
/// (mask starts 0xFFFFF and the callee rewrites it; the loop is a
/// do-while on the mask), per-player stat accumulation (0x002A9994),
/// the +0x3BC score block, the LivingWorld-player lookup (0x002B51F8 on
/// player +0x3AC), the battle side/slot queries (0x003F4752/0x003F4FAA)
/// and block handoff (0x003F470E), the victor lookup (0x0023D884, its
/// +0x14 player index), the battle +0x38 side store, and the four
/// terminal predicates/calls (0x00210DC9 manager flag, the
/// `DAT_00DFEF18` +0x18 flag, the 0x00210C66 GameLogic flag).
pub trait BattleCompleteHost {
    fn army_summary_save(&mut self);
    fn current_battle(&mut self) -> i32;
    fn each_player_from_mask(&mut self, mask: &mut i32) -> i32;
    fn accumulate_stats(&mut self, player: i32);
    fn player_score_block(&mut self, player: i32) -> i32;
    fn find_living_world_player(&mut self, player: i32) -> i32;
    fn battle_side_of(&mut self, battle: i32, lw_player: i32) -> i32;
    fn battle_slot_of(&mut self, side: i32, lw_player: i32) -> i32;
    fn battle_apply_block(&mut self, side: i32, slot: i32, block: i32);
    fn tactical_victor(&mut self) -> i32;
    fn victor_player_index(&mut self, victor: i32) -> i32;
    fn set_battle_side(&mut self, battle: i32, side: i32);
    fn manager_living_world_flag(&mut self) -> bool;
    fn living_world_flag_18(&mut self) -> bool;
    fn gamelogic_end_session_flag(&mut self) -> bool;
    fn send_end_session_msg(&mut self);
    fn manager_msg_211fa8(&mut self);
    fn clear_game_data(&mut self, a: bool, b: bool);
    fn transition_from_living_world(&mut self);
}

/// `0x0063d914` — LivingWorld tactical battle completion driver.
///
/// Per-player walk gates exactly as the body: a null battle (fetched
/// once, before the loop) or null +0x3BC block skips the side/slot
/// chain; side/slot of -1 skip the handoff. The victor store is
/// `victor ? victor[+0x14] : -1` at GameLogic +0x1B0, the +0x38 store
/// needs both battle and victor. The terminal test is the negated pair
/// from the reference (`managerFlag && !livingWorldFlag18`): the
/// decompiled form `(managerFlag == 0 || flag18 != 0)` picks
/// clear-and-transition; otherwise the 0x00210C66 flag picks
/// end-session versus the manager message.
pub fn game_logic_living_world_tactical_battle_complete(
    state: &mut LivingWorldBattleCompleteState,
    host: &mut dyn BattleCompleteHost,
) -> BattleCompleteTail {
    host.army_summary_save();
    let battle = host.current_battle();
    let mut mask: i32 = 0x000f_ffff;
    loop {
        let player = host.each_player_from_mask(&mut mask);
        if player != 0 {
            host.accumulate_stats(player);
            let block = host.player_score_block(player);
            if battle != 0 && block != 0 {
                let lw = host.find_living_world_player(player);
                if lw != 0 {
                    let side = host.battle_side_of(battle, lw);
                    if side != -1 {
                        let slot = host.battle_slot_of(side, lw);
                        if slot != -1 {
                            host.battle_apply_block(side, slot, block);
                        }
                    }
                }
            }
        }
        if mask == 0 {
            break;
        }
    }
    let victor = host.tactical_victor();
    state.living_world_victor_index = if victor != 0 {
        host.victor_player_index(victor)
    } else {
        -1
    };
    if battle != 0 && victor != 0 {
        let side = host.battle_side_of(battle, victor);
        host.set_battle_side(battle, side);
    }
    if !host.manager_living_world_flag() || host.living_world_flag_18() {
        host.clear_game_data(true, false);
        host.transition_from_living_world();
        BattleCompleteTail::ClearAndTransition
    } else if host.gamelogic_end_session_flag() {
        host.send_end_session_msg();
        BattleCompleteTail::EndSessionMsg
    } else {
        host.manager_msg_211fa8();
        BattleCompleteTail::ManagerMsg
    }
}

// ---------------------------------------------------------------------
// 0x006bd23e — LivingWorldLogic::CheckTurnPhaseTransitions
// ---------------------------------------------------------------------

/// The phase pair and the two condition-flag bytes the case-2 arm sets:
/// `m_turnPhase` @ +0xF4 (the authoritative current phase, written by
/// other rows), the row's own previous-phase latch @ +0xF8, and the
/// +0x108/+0x109 bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TurnPhaseState {
    pub turn_phase: i32,
    pub prev_phase: i32,
    pub flag_108: u8,
    pub flag_109: u8,
}

/// Case-body calls by row: the -1 latch helper `rva002B2BEC(local
/// player)`, phase-1's trio (`rva002B3B66`, this module's
/// [`living_world_logic_merge_unnamed_armies`], `rva002B3D0E`) plus the
/// region manager's battle-marker update (0x0020EBA0), phase-2's pair
/// ([`living_world_logic_process_delayed_region_victories`] +
/// `rva002B6900`), the turn-listener `forEach` (0x002B616F over
/// `FUN_005FF3A9`), phase-0's upgrade auto-mark (0x002B5334), the head-
/// to-head check (0x002BCE24), phase-2's manager calls (0x0020E4C8)
/// with the two condition queries (0x0020E72A, 0x002B5CBB), the retreat
/// preparation (0x002B9FDB), and `rva002B9DC7`.
pub trait PhaseTransitionHost {
    fn rva_002b2bec(&mut self);
    fn rva_002b3b66(&mut self);
    fn merge_unnamed_armies(&mut self);
    fn rva_002b3d0e(&mut self);
    fn update_battle_markers(&mut self);
    fn process_delayed_region_victories(&mut self);
    fn rva_002b6900(&mut self);
    fn rva_002b9dc7(&mut self);
    fn notify_turn_listeners(&mut self, from: i32, to: i32);
    fn auto_mark_upgrades(&mut self);
    fn check_head_to_head_battles(&mut self);
    fn adjust_army_target_locations(&mut self);
    fn manager_rva_0020e4c8(&mut self);
    fn manager_flag_query(&mut self) -> bool;
    fn phase_upgrade_query(&mut self) -> bool;
    fn prepare_retreats(&mut self);
}

/// `0x006bd23e` — turn-phase edge machine.
///
/// Everything runs only when the +0xF8 latch differs from the +0xF4
/// phase. The old phase picks the exit arm (-1 / 1 / 2 / 5; others do
/// nothing), a new phase of 0 runs `rva002B9DC7` before the listener
/// notification, then the new phase picks the entry arm: 0 -> merge +
/// auto-mark; 1 -> head-to-head then adjust; 5 -> adjust; 2 -> manager
/// churn and the two flag-byte sets; 4 -> retreat preparation only
/// (no adjust — the decompiler's fall-through shape puts 4 outside the
/// shared adjust tail). Finally +0xF8 := +0xF4.
pub fn living_world_logic_check_turn_phase_transitions(
    state: &mut TurnPhaseState,
    host: &mut dyn PhaseTransitionHost,
) {
    if state.prev_phase != state.turn_phase {
        match state.prev_phase {
            -1 => host.rva_002b2bec(),
            1 => {
                host.rva_002b3b66();
                host.merge_unnamed_armies();
                host.rva_002b3d0e();
                host.update_battle_markers();
            }
            2 => {
                host.process_delayed_region_victories();
                host.rva_002b6900();
            }
            5 => host.rva_002b3b66(),
            _ => {}
        }
        if state.turn_phase == 0 {
            host.rva_002b9dc7();
        }
        host.notify_turn_listeners(state.prev_phase, state.turn_phase);
        match state.turn_phase {
            0 => {
                host.merge_unnamed_armies();
                host.auto_mark_upgrades();
            }
            1 => {
                host.check_head_to_head_battles();
                host.adjust_army_target_locations();
            }
            2 => {
                host.manager_rva_0020e4c8();
                host.update_battle_markers();
                if host.manager_flag_query() {
                    state.flag_108 = 1;
                }
                if host.phase_upgrade_query() {
                    state.flag_109 = 1;
                }
            }
            4 => host.prepare_retreats(),
            5 => host.adjust_army_target_locations(),
            _ => {}
        }
    }
    state.prev_phase = state.turn_phase;
}

// ---------------------------------------------------------------------
// 0x0077dce8 — CarryoverUnit::createSingleObject
// ---------------------------------------------------------------------

/// The carryover record the row reads: the template-provider handle
/// (`GetThingTemplate`, row 0x0077DC52; 0 = absent -> return 0), the
/// experience float at +0x08, the 128-byte (32-word) upgrade bitset at
/// +0x10, and the revival record at +0x94 (dword +0x94, reset dword
/// +0x98, flag byte +0xA8 — the object-side twins live at +0x468/+0x46C/
/// +0x47C).
#[derive(Clone, Debug, PartialEq)]
pub struct CarryoverUnit {
    pub thing_template: i32,
    pub experience: f32,
    pub upgrades: [u32; 32],
    pub record_field00: i32,
    pub record_reset04: i32,
    pub record_flag14: u8,
}

/// The observable object-side state the row writes: experience tracker
/// (object +0x264 -> tracker +0x10 experience, +0x20 flag), the 128-byte
/// upgrade bitset at +0x284, and the record reset/flag at +0x46C/+0x47C.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CreatedObject {
    pub tracker_experience: f32,
    pub tracker_flag: u8,
    pub upgrades: [u32; 32],
    pub record_field00: i32,
    pub record_reset04: i32,
    pub record_flag14: u8,
}

/// Factory and object-services chain. `new_object` stands in for
/// `TheThingFactory->newObject(template, player[+0x2EC] team, &mask, 0)`
/// — the 16-byte `CreateMask` is memset-zero in the body, so it is not
/// a knob; the id parameter is the literal 0. `rva0028CBFD` is the
/// post-create object call, `rva0039B3D1(value, true)` the experience
/// helper, `rva002931F5(0)` the sink lookup, and the sink's AI pair
/// `slot31()` (gate) / `slot68(callback 0x0037DCCC, &upgrades, true)`.
pub trait CarryoverCreateHost {
    fn player_team(&mut self, player: i32) -> i32;
    fn new_object(&mut self, template: i32, team: i32) -> i32;
    fn rva_0028cbfd(&mut self, object: i32);
    fn rva_0039b3d1(&mut self, object: i32, value: f32);
    fn update_upgrade_modules(&mut self, object: i32);
    fn find_sink(&mut self, object: i32) -> i32;
    fn sink_ai_ready(&mut self, sink: i32) -> bool;
    fn sink_ai_apply_upgrades(&mut self, sink: i32);
}

/// `0x0077dce8` — create one carried unit through the factory.
///
/// Sequence, exactly: template lookup or return 0; save the
/// `TheGameLogic + 0x98` byte and clear it; zero the create mask; run
/// the factory with the player's +0x2EC team and id 0 — a null object
/// restores +0x98 and returns 0. On success: the object call, then the
/// experience gate written as the IEEE pair `(tracker <= value) &&
/// (value != tracker)` (literally *less than*, but kept as the two
/// comparisons — either alone flips differently under NaN), whose body
/// runs the experience helper and clears the tracker's +0x20 flag; the
/// 32-word upgrade OR into the object's +0x284 bitset; the upgrade
/// refresh; the record copy plus the two post-copy stores (+0x46C = 0,
/// +0x47C = 0); the sink chain (sink -> AI present -> `slot31()` gate ->
/// `slot68`); and the +0x98 restore on **both** success and failure
/// returns.
pub fn carryover_unit_create_single_object(
    unit: &CarryoverUnit,
    flag_98: &mut u8,
    object: &mut CreatedObject,
    player: i32,
    host: &mut dyn CarryoverCreateHost,
) -> i32 {
    if unit.thing_template == 0 {
        return 0;
    }
    let old_98 = *flag_98;
    *flag_98 = 0;
    let team = host.player_team(player);
    let obj = host.new_object(unit.thing_template, team);
    if obj == 0 {
        *flag_98 = old_98;
        return 0;
    }
    host.rva_0028cbfd(obj);
    let value = unit.experience;
    let tracker = object.tracker_experience;
    if tracker <= value && value != tracker {
        host.rva_0039b3d1(obj, value);
        object.tracker_flag = 0;
    }
    for i in 0..32 {
        object.upgrades[i] |= unit.upgrades[i];
    }
    host.update_upgrade_modules(obj);
    object.record_field00 = unit.record_field00;
    object.record_reset04 = unit.record_reset04;
    object.record_flag14 = unit.record_flag14;
    object.record_reset04 = 0;
    object.record_flag14 = 0;
    let sink = host.find_sink(obj);
    if sink != 0 && host.sink_ai_ready(sink) {
        host.sink_ai_apply_upgrades(sink);
    }
    *flag_98 = old_98;
    obj
}

// ---------------------------------------------------------------------
// 0x007efa0a — LivingWorldPathFinder::FindShortestPath
// ---------------------------------------------------------------------

/// Sentinels for the row's node handles (game null pointers).
pub const NULL_NODE: usize = usize::MAX;

/// One search node's fields the driver touches: key at +0x00, cost at
/// +0x1C, estimate at +0x20, total at +0x24, predecessor at +0x28.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PathNode {
    pub key: i32,
    pub cost: f32,
    pub estimate: f32,
    pub total: f32,
    pub previous: Option<usize>,
}

/// The finder object's slots this driver writes: start (+0x0C), goal
/// (+0x10), open (+0x14..+0x1C), closed (+0x20..+0x2C) — the vectors
/// become `Vec<usize>` handle lists.
#[derive(Clone, Debug, Default)]
pub struct PathSearch {
    pub start: usize,
    pub goal: usize,
    pub open: Vec<usize>,
    pub closed: Vec<usize>,
}

/// Callback and helper rows: the metric/callback object stored at +0x2C
/// exposes slot 0x08 (`notify opened`) and slot 0x0C (`goal test`);
/// `rva003EF6A0` finds (or creates) a node by key; `rva003EF8E1` runs
/// the relaxation (receiver's start/goal, the adjacency permission test
/// and the sorted open insert — a separate row); `rva003EF9DC` fills
/// the ObjectID output vector along the predecessor chain; `rva003EF634`
/// counts the chain with no output vector.
pub trait PathSearchHost {
    fn find_node(&mut self, key: i32) -> usize;
    fn opened(&mut self, current: usize);
    fn goal_test(&mut self, current: usize, goal: usize) -> bool;
    fn relax(
        &mut self,
        search: &mut PathSearch,
        nodes: &mut [PathNode],
        current: usize,
        opaque: i32,
        allow_other: bool,
    );
    fn fill_path(&mut self, current: usize, out: &mut Vec<i32>);
    fn count_path(&mut self, current: usize) -> i32;
}

/// `0x007efa0a` — the open/closed search driver.
///
/// The callback pointer is stored into +0x2C first. `from == to` short
/// circuits: with an output vector it is cleared and `from` pushed, and
/// the return is 0. Otherwise open and closed are range-erased (the
/// vector-erase rows 0x0031BD55), the output vector cleared when
/// present, then both endpoints go through the node provider into the
/// +0x0C/+0x10 slots — written even when null. When both nodes exist
/// the start's three cost floats and its predecessor are zeroed and it
/// is pushed onto open; the loop pops the **front** element (the
/// `erase(open.begin)` idiom; the loop gate is the body's
/// `(end - begin) & 0xfffffffc != 0`), notifies slot 0x08, and the goal
/// test either returns through fill (`out.size() - 1`, the
/// `(end - begin >> 2) - 1` dword form) or through the count row, else
/// relaxes and moves the node to closed. Exhaustion returns -1.
pub fn living_world_path_finder_find_shortest_path(
    search: &mut PathSearch,
    nodes: &mut [PathNode],
    opaque: i32,
    from: i32,
    to: i32,
    out: &mut Option<Vec<i32>>,
    allow_other: bool,
    host: &mut dyn PathSearchHost,
) -> i32 {
    if from == to {
        if let Some(v) = out.as_mut() {
            v.clear();
            v.push(from);
        }
        return 0;
    }
    search.open.clear();
    search.closed.clear();
    if let Some(v) = out.as_mut() {
        v.clear();
    }
    let start = host.find_node(from);
    search.start = start;
    let goal = host.find_node(to);
    search.goal = goal;
    if start != NULL_NODE && goal != NULL_NODE {
        nodes[start].cost = 0.0;
        nodes[start].estimate = 0.0;
        nodes[start].total = 0.0;
        nodes[start].previous = None;
        search.open.push(start);
        while !search.open.is_empty() {
            let current = search.open[0];
            search.open.remove(0);
            host.opened(current);
            if host.goal_test(current, goal) {
                if let Some(v) = out.as_mut() {
                    host.fill_path(current, v);
                    return v.len() as i32 - 1;
                }
                return host.count_path(current);
            }
            host.relax(search, nodes, current, opaque, allow_other);
            search.closed.push(current);
        }
    }
    -1
}

// ---------------------------------------------------------------------
// 0x006bb15f — LivingWorldLogic delayed region victories (rva002BB15F)
// ---------------------------------------------------------------------

/// One queue entry: the 12-byte record at the +0x148/+0x14C vector —
/// player handle at +0x00, region id at +0x04, flags at +0x08.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DelayedRegionVictory {
    pub player: i32,
    pub region_id: i32,
    pub flags: u32,
}

/// Foreign chain: the region lookup `rva0020EAF6` (region manager
/// +0xB0), the region owner dword at +0x13C, the local-player index
/// getter `rva002B2B66` (this+0x98's +0x14, or -1), the region centre
/// query (0x0020EA58), the terrain-lift helper `rva002BF5B0` (pos is
/// built as (centre.x, centre.y, 0) and lifted in place), EVA event 21
/// reporting (0x005DE2DA), the ownership helper `rva002B37B4`, the
/// region effects sync (0x007EF08B, called with the region), the queue
/// range-erase (0x002BF475-folded form `FUN_006A133B`), `rva002B9DC7`,
/// and the region-manager tail call (0x0020F483).
pub trait DelayedVictoryHost {
    fn lookup_region(&mut self, region_id: i32) -> i32;
    fn region_owner(&mut self, region: i32) -> i32;
    fn local_player_index(&mut self) -> i32;
    fn region_center(&mut self, region: i32) -> (f32, f32);
    fn lift_to_terrain(&mut self, x: f32, y: f32) -> (f32, f32, f32);
    fn report_eva_event_21(&mut self, x: f32, y: f32, z: f32);
    fn apply_ownership(&mut self, region: i32, player: i32);
    fn sync_region_effects(&mut self, region: i32);
    fn rva_002b9dc7(&mut self);
    fn rva_0020f483(&mut self);
}

/// `0x006bb15f` — settle every delayed region victory.
///
/// Per entry: a null region or null player skips (the `!player` test is
/// on the handle, not an index). The EVA branch needs a *plain* victory
/// — flag bit 0 set and bits 1|3 clear (`(flags & 1) != 0 &&
/// (flags & 0xA) == 0`) — and the region's +0x13C owner equal to the
/// local-player index; the centre is then lifted to the terrain and EVA
/// event 0x15 is reported with no second position. Every surviving entry
/// (EVA branch or not) goes through the ownership helper with the
/// *player handle* from the entry, then the effects sync. The queue is
/// range-cleared after the walk (the loop bound is the vector span; the
/// body reloads +0x148 per iteration — mirrored by fixed iteration,
/// nothing inside resizes), then `rva002B9DC7` and the manager call run
/// unconditionally.
pub fn living_world_logic_process_delayed_region_victories(
    queue: &mut Vec<DelayedRegionVictory>,
    host: &mut dyn DelayedVictoryHost,
) {
    for entry in queue.iter() {
        let region = host.lookup_region(entry.region_id);
        let player = entry.player;
        if region == 0 || player == 0 {
            continue;
        }
        let flags = entry.flags;
        if (flags & 1) != 0 && (flags & 10) == 0 {
            if host.region_owner(region) == host.local_player_index() {
                let (cx, cy) = host.region_center(region);
                let (px, py, pz) = host.lift_to_terrain(cx, cy);
                host.report_eva_event_21(px, py, pz);
            }
        }
        host.apply_ownership(region, player);
        host.sync_region_effects(region);
    }
    queue.clear();
    host.rva_002b9dc7();
    host.rva_0020f483();
}

// ---------------------------------------------------------------------
// 0x0099d089 — LivingWorldAIBuilder score (rva0059D089)
// ---------------------------------------------------------------------

/// The builder's score weights; only +0x08 (`m_08`) belongs to this
/// row's template instantiation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AiBuilderWeights {
    pub score_weight_08: i32,
}

/// The armies-map node this row reads: the region id at +0x14 (passed
/// by address to both queries) and the Castles object-id vector at
/// +0x24/+0x28 (only its size is used, through the raw
/// `(end - begin) & 0xfffffffc` gate and `>> 2` count — kept literal
/// here as dword fields).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BuilderRegionRecord {
    pub region_id: i32,
    pub castles_begin: i32,
    pub castles_end: i32,
}

/// The building-template record's field this row reads: the float at
/// +0x30 (the per-castle damping input).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BuilderTemplate {
    pub template_float_30: f32,
}

/// `rva00500659` returns its defender list by value (the caller frees
/// the hidden result buffer through `_free`, modelled inside the host)
/// and writes the defender count out; `rva004FFB00` returns the
/// closest-hero entry and writes the distance through its third
/// argument.
pub trait BuilderScoreHost {
    fn defenders(&mut self, player_id: i32, region_id: i32) -> i32;
    fn closest_hero_distance(&mut self, player_id: i32, region_id: i32) -> i32;
}

/// `0x0099d089` — the castle-damped, defender-weighted builder score.
///
/// `score = (float)m_08`, then damped once per Castle entry:
/// `score = (1.0f - tmpl[+0x30] * 0.1f) * score` — the gate is signed
/// on `(end - begin) & ~3` and the count is the arithmetic `>> 2`, so a
/// wrapped (negative) span runs zero iterations while still being > 0
/// masked... pinned literally below. Below 6 defenders:
/// `score = defenders * (float)log((double)(6.0f - defenders)) * score`
/// — the logarithm runs in double precision (the msvcr71 `log` import
/// thunk) and is truncated to float afterwards, so a half-ULP float
/// difference in `logf` would diverge; then when `0.0f < score` and
/// `0 < distance` the hero-distance falloff
/// `(2.0f / (distance + 1) + 0.5f) * score` applies. At 6+ defenders
/// the score is simply scaled by 0.1.
pub fn living_world_ai_builder_defender_score(
    builder: &AiBuilderWeights,
    tmpl: &BuilderTemplate,
    region: &BuilderRegionRecord,
    player_id: i32,
    host: &mut dyn BuilderScoreHost,
) -> f32 {
    let mut score = builder.score_weight_08 as f32;
    let diff = region.castles_end.wrapping_sub(region.castles_begin);
    if ((diff as u32) & 0xffff_fffc) as i32 > 0 {
        let n = diff >> 2;
        let mut i = 0;
        while i < n {
            i += 1;
            score = (1.0f32 - tmpl.template_float_30 * 0.1f32) * score;
        }
    }
    let defenders = host.defenders(player_id, region.region_id);
    let distance = host.closest_hero_distance(player_id, region.region_id);
    if defenders < 6 {
        let d = defenders as f32;
        let l = ((6.0f32 - d) as f64).ln() as f32;
        score = d * l * score;
        if 0.0f32 < score && 0 < distance {
            score = (2.0f32 / ((distance as f32) + 1.0f32) + 0.5f32) * score;
        }
    } else {
        score = score * 0.1f32;
    }
    score
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- row 1: x87 fistp + FireLogicSystem::NewMap ----

    #[test]
    fn fistp_rounds_half_even_and_overflows_indefinite() {
        assert_eq!(x87_fistp_i32(0.5), 0);
        assert_eq!(x87_fistp_i32(1.5), 2);
        assert_eq!(x87_fistp_i32(2.5), 2);
        assert_eq!(x87_fistp_i32(-2.5), -2);
        assert_eq!(x87_fistp_i32(f32::NAN), i32::MIN);
        assert_eq!(x87_fistp_i32(f32::INFINITY), i32::MIN);
        assert_eq!(x87_fistp_i32(2147483648.0), i32::MIN);
        // -2^31 is inside the dword range: exact store, not the sentinel.
        assert_eq!(x87_fistp_i32(-2147483648.0), i32::MIN);
    }

    #[test]
    fn new_map_counts_cells_off_the_extent() {
        let mut sys = FireLogicGrid::default();
        let mut n = 0;
        fire_logic_system_new_map(&mut sys, 100.0, 99.9, |_, _| {
            n += 1;
            0
        });
        assert_eq!(sys.num_rows, 11); // floor(10.0) + 1
        assert_eq!(sys.num_cols, 10); // floor(9.99) + 1
        assert_eq!(sys.storage.len(), 110);
        assert_eq!(n, 110);
    }

    #[test]
    fn new_map_nan_extent_stores_wrapped_counts_and_builds_nothing() {
        let mut sys = FireLogicGrid::default();
        fire_logic_system_new_map(&mut sys, f32::NAN, 10.0, |_, _| 2);
        assert_eq!(sys.num_rows, i32::MIN.wrapping_add(1));
        assert_eq!(sys.num_cols, 2);
        assert!(sys.storage.is_empty());
    }

    #[test]
    fn new_map_seeds_bitfields_with_mask_windows() {
        let mut sys = FireLogicGrid::default();
        sys.materials[2] = FireMaterial {
            fuel: 0x1_2345,
            field18: 0x000f_ffff, // only bits 0..11 land in the cell
            field00: 0xffff,      // only bits 0..9 land
            field10: 0xffff,      // only bits 0..7 land at cell bits 10..17
            ..Default::default()
        };
        fire_logic_system_new_map(&mut sys, 10.0, 10.0, |_, _| 2);
        // material id is masked & 0xff: 0x102 would index [2] out of range —
        // the type gate is 0 != (b & 0xff) < 4.
        let c = sys.storage[0];
        assert_eq!(c.fuel, 0x2345);
        assert_eq!(c.bits, (0xfff << 18) | (0xff << 10) | 0x3ff);
        assert_eq!(c.cell_type, 0);
        assert_eq!(c.watchers, 0);
        // ids outside 0..3 leave the cell untouched
        fire_logic_system_new_map(&mut sys, 10.0, 10.0, |_, _| 5);
        assert_eq!(sys.storage[0].fuel, 0);
    }

    // ---- rows 2/10: TerrainResourceManager claim pair ----

    fn claim_setup() -> (Vec<ResourceCell>, f32, f32, f32) {
        // 2x2 grid
        (
            vec![ResourceCell::default(); 4],
            0.0,
            0.0,
            10.0,
        )
    }

    #[test]
    fn claimable_guards_and_simple_states() {
        let (mut cells, ox, oy, cs) = claim_setup();
        assert!(!terrain_resource_manager_is_cell_claimable(
            None, 2, 2, ox, oy, cs, 0, 0, 0, 0, false, -1,
            |_, _, _| 0, |_| false, |_| 0,
        ));
        // out of range on either axis
        assert!(!terrain_resource_manager_is_cell_claimable(
            Some(&mut cells), 2, 2, ox, oy, cs, 2, 0, 0, 0, false, -1,
            |_, _, _| 0, |_| false, |_| 0,
        ));
        cells[0].state = 0;
        assert!(terrain_resource_manager_is_cell_claimable(
            Some(&mut cells), 2, 2, ox, oy, cs, 0, 0, 0, 0, false, -1,
            |_, _, _| 0, |_| false, |_| 0,
        ));
        cells[0].state = 1;
        assert!(!terrain_resource_manager_is_cell_claimable(
            Some(&mut cells), 2, 2, ox, oy, cs, 0, 0, 0, 0, true, 5,
            |_, _, _| 0, |_| false, |_| 0,
        ));
        // unknown state fails
        cells[0].state = 9;
        assert!(!terrain_resource_manager_is_cell_claimable(
            Some(&mut cells), 2, 2, ox, oy, cs, 0, 0, 0, 0, true, -1,
            |_, _, _| 0, |_| false, |_| 0,
        ));
    }

    #[test]
    fn claimable_shroud_gate_state2() {
        let (mut cells, ox, oy, cs) = claim_setup();
        cells[0].state = 2;
        cells[0].claims.push((3, 100));
        // shroud 2 blocks immediately (gate runs: !force && player > -1)
        assert!(!terrain_resource_manager_is_cell_claimable(
            Some(&mut cells), 2, 2, ox, oy, cs, 0, 0, 0, 0, false, 3,
            |p, x, y| {
                assert_eq!(p, 3);
                assert_eq!(x, 5.0); // (0 + 0.5) * 10
                assert_eq!(y, 5.0);
                2
            },
            |_| false,
            |_| 0,
        ));
        // shroud 0: the claimant scan runs; a match blocks, a mismatch passes
        assert!(!terrain_resource_manager_is_cell_claimable(
            Some(&mut cells), 2, 2, ox, oy, cs, 0, 0, 0, 0, false, 3,
            |_, _, _| 0, |_| false, |_| 0,
        ));
        assert!(terrain_resource_manager_is_cell_claimable(
            Some(&mut cells), 2, 2, ox, oy, cs, 0, 0, 0, 0, false, 4,
            |_, _, _| 0, |_| false, |_| 0,
        ));
        // a non-zero (non-2) shroud skips the scan: claimable outright
        assert!(terrain_resource_manager_is_cell_claimable(
            Some(&mut cells), 2, 2, ox, oy, cs, 0, 0, 0, 0, false, 3,
            |_, _, _| 1, |_| false, |_| 0,
        ));
        // force skips the gate and claims: true regardless
        assert!(terrain_resource_manager_is_cell_claimable(
            Some(&mut cells), 2, 2, ox, oy, cs, 0, 0, 0, 0, true, 3,
            |_, _, _| panic!("force skips the shroud gate"),
            |_| false,
            |_| 0,
        ));
    }

    #[test]
    fn claimable_state3_selfheal_and_sentinel() {
        let (mut cells, ox, oy, cs) = claim_setup();
        // empty claims: state 3 self-heals to 0 and passes
        cells[0].state = 3;
        assert!(terrain_resource_manager_is_cell_claimable(
            Some(&mut cells), 2, 2, ox, oy, cs, 0, 0, 1, 2, false, -1,
            |_, _, _| 0, |_| false, |_| 0,
        ));
        assert_eq!(cells[0].state, 0);
        // dangling first claimant: erase all, state 0, pass
        cells[0].state = 3;
        cells[0].claims.push((7, 55));
        assert!(terrain_resource_manager_is_cell_claimable(
            Some(&mut cells), 2, 2, ox, oy, cs, 0, 0, 1, 2, false, 3,
            |_, _, _| 0,
            |_| false, // object not found
            |_| 0,
        ));
        assert!(cells[0].claims.is_empty());
        assert_eq!(cells[0].state, 0);
        // live claimant, object shroud 3 -> claimable
        cells[1].state = 3;
        cells[1].claims.push((7, 55));
        assert!(terrain_resource_manager_is_cell_claimable(
            Some(&mut cells), 2, 2, ox, oy, cs, 1, 0, 1, 2, false, 3,
            |_, _, _| 0,
            |_| true,
            |_| 3,
        ));
        // live claimant, not force, no match -> denied
        assert!(!terrain_resource_manager_is_cell_claimable(
            Some(&mut cells), 2, 2, ox, oy, cs, 1, 0, 1, 2, false, 3,
            |_, _, _| 0,
            |_| true,
            |_| 0,
        ));
        // wildcard id sentinel
        assert!(terrain_resource_manager_is_cell_claimable(
            Some(&mut cells), 2, 2, ox, oy, cs, 1, 0, 0x07ff_f43, 2, true, 3,
            |_, _, _| 0,
            |_| true,
            |_| 0,
        ));
        // own_id matches the claimant object id
        assert!(terrain_resource_manager_is_cell_claimable(
            Some(&mut cells), 2, 2, ox, oy, cs, 1, 0, 1, 55, true, 3,
            |_, _, _| 0,
            |_| true,
            |_| 0,
        ));
    }

    fn rec(b: u8) -> CarryoverRecord {
        let mut r = CarryoverRecord::default();
        r.bytes[0] = b;
        r
    }

    fn firsts(v: &[CarryoverRecord]) -> Vec<u8> {
        v.iter().map(|r| r.bytes[0]).collect()
    }

    #[test]
    fn carryover_insert_in_place_paths_keep_capacity() {
        // path B: elems_after <= n
        let mut v = vec![rec(1), rec(2)];
        v.reserve_exact(8);
        linear_campaign_carryover_insert(&mut v, 1, &[rec(20), rec(21), rec(22), rec(23)]);
        assert_eq!(firsts(&v), vec![1, 20, 21, 22, 23, 2]);
        assert_eq!(v.len(), 6);
        // path A: elems_after > n
        let mut w = vec![rec(1), rec(2), rec(3), rec(4), rec(5)];
        w.reserve_exact(90);
        let cap = w.capacity();
        linear_campaign_carryover_insert(&mut w, 0, &[rec(20), rec(21)]);
        assert_eq!(firsts(&w), vec![20, 21, 1, 2, 3, 4, 5]);
        assert_eq!(w.capacity(), cap);
        // empty range: no-op
        let before = firsts(&w);
        linear_campaign_carryover_insert(&mut w, 2, &[]);
        assert_eq!(firsts(&w), before);
    }

    #[test]
    fn carryover_insert_realloc_uses_stlport_growth() {
        let mut v = vec![rec(1), rec(2), rec(3)];
        v.shrink_to_fit();
        assert_eq!(v.capacity(), 3);
        linear_campaign_carryover_insert(&mut v, 1, &[rec(20), rec(21)]);
        // new_len = 3 + max(3, 2) = 6
        assert_eq!(v.capacity(), 6);
        assert_eq!(firsts(&v), vec![1, 20, 21, 2, 3]);
        // n > old_size: 2 + max(2, 5) = 7
        let mut u = vec![rec(1), rec(2)];
        u.shrink_to_fit();
        linear_campaign_carryover_insert(&mut u, 0, &[rec(9); 5]);
        assert_eq!(u.capacity(), 7);
    }

    // ---- row 3: tactical battle complete ----

    struct BattleMock {
        log: Vec<String>,
        region: i32,
        battle: i32,
        side: i32,
        team: i32,
        sides: i32,
        armies: i32,
        units: i32,
        army_team: i32,
        names_empty: Vec<bool>,
        owner_queries: Vec<bool>,
    }

    impl TacticalBattleHost for BattleMock {
        fn battle_region(&mut self) -> i32 {
            self.log.push("region".into());
            self.region
        }
        fn current_battle(&mut self) -> i32 {
            self.log.push("battle".into());
            self.battle
        }
        fn battle_side(&mut self, _b: i32) -> i32 {
            self.side
        }
        fn team_for_side(&mut self, side: i32) -> i32 {
            assert_eq!(side, self.side);
            self.team
        }
        fn side_count(&mut self, _b: i32) -> i32 {
            self.sides
        }
        fn army_count(&mut self, _b: i32, _s: i32) -> i32 {
            self.armies
        }
        fn army_at(&mut self, _b: i32, _s: i32, a: i32) -> i32 {
            100 + a
        }
        fn unit_count(&mut self, _b: i32, _s: i32, _a: i32) -> i32 {
            self.units
        }
        fn unit_at(&mut self, _b: i32, _s: i32, _a: i32, u: i32) -> i32 {
            u
        }
        fn army_team(&mut self, _a: i32) -> i32 {
            self.army_team
        }
        fn unit_name_empty(&mut self, unit: i32) -> bool {
            self.names_empty[unit as usize]
        }
        fn consume_garrison(&mut self, u: i32, _a: i32, _b: i32) {
            self.log.push(format!("consume{u}"));
        }
        fn destroy_unit(&mut self, u: i32) {
            self.log.push(format!("destroy{u}"));
        }
        fn unit_owner_query(&mut self, u: i32) -> bool {
            self.owner_queries[u as usize]
        }
        fn unit_owner_release(&mut self, u: i32) {
            self.log.push(format!("release{u}"));
        }
        fn push_carryover_unit(&mut self, u: i32) {
            self.log.push(format!("push{u}"));
        }
        fn on_battle_complete(&mut self, _b: i32) {
            self.log.push("complete".into());
        }
        fn notify_battle_listeners(&mut self, _b: i32) {
            self.log.push("listeners".into());
        }
    }

    #[test]
    fn tactical_battle_complete_fates_and_flags() {
        let mut mock = BattleMock {
            log: vec![],
            region: 7,
            battle: 9,
            side: 1,
            team: 3,
            sides: 1,
            armies: 1,
            units: 4,
            army_team: 3, // on-team
            names_empty: vec![true, false, true, false],
            owner_queries: vec![false, false, true, true],
        };
        let mut flags = TacticalBattleFlags {
            tactical_battle_completed: 0,
            battle_region_index: 4,
        };
        living_world_logic_game_logic_tactical_battle_complete(&mut flags, &mut mock);
        // on-team: both empty-name units (0, 2) consumed; named stay
        assert_eq!(
            mock.log,
            vec!["region", "battle", "consume0", "consume2", "complete", "listeners"]
        );
        assert_eq!(flags.tactical_battle_completed, 1);
        assert_eq!(flags.battle_region_index, -1);

        // off-team army: named units get owner release + carryover push
        let mut mock2 = BattleMock { army_team: 8, ..mock };
        mock2.log = vec![];
        mock2.names_empty = vec![true, false, true, false];
        mock2.owner_queries = vec![false, false, true, true];
        let mut flags2 = TacticalBattleFlags::default();
        living_world_logic_game_logic_tactical_battle_complete(&mut flags2, &mut mock2);
        assert_eq!(
            mock2.log,
            vec![
                "region",
                "battle",
                "destroy0",
                "push1",
                "destroy2",
                "release3",
                "push3",
                "complete",
                "listeners",
            ]
        );
    }

    #[test]
    fn tactical_battle_complete_guard_skips_walk_but_latches() {
        let mut mock = BattleMock {
            log: vec![],
            region: 0, // fails the third guard term
            battle: 9,
            side: 1,
            team: 3,
            sides: 1,
            armies: 1,
            units: 1,
            army_team: 3,
            names_empty: vec![true],
            owner_queries: vec![false],
        };
        let mut flags = TacticalBattleFlags::default();
        living_world_logic_game_logic_tactical_battle_complete(&mut flags, &mut mock);
        assert_eq!(mock.log, vec!["region", "battle"]);
        assert_eq!(flags.tactical_battle_completed, 1);
        assert_eq!(flags.battle_region_index, -1);
    }

    // ---- row 5: unnamed army merge ----

    struct MergeMock {
        set: i32,
        regions: Vec<i32>,
        region_player: i32,
        players: Vec<i32>,
        armies: Vec<Vec<i32>>,
        named: Vec<i32>,        // army ids that carry a name
        army_regions: Vec<(i32, i32)>, // (army, target region)
        deleted: Vec<i32>,
        merged: Vec<(i32, i32)>,
    }

    impl UnnamedArmyMergeHost for MergeMock {
        fn army_set(&mut self) -> i32 {
            self.set
        }
        fn set_count(&mut self, _set: i32) -> i32 {
            self.regions.len() as i32
        }
        fn set_at(&mut self, _set: i32, i: i32) -> i32 {
            self.regions[i as usize]
        }
        fn region_player(&mut self, _r: i32) -> i32 {
            self.region_player
        }
        fn player_count(&mut self) -> i32 {
            self.players.len() as i32
        }
        fn player_at(&mut self, i: i32) -> i32 {
            self.players[i as usize]
        }
        fn player_accepts(&mut self, _p: i32, rp: i32) -> bool {
            rp == 7
        }
        fn army_count(&mut self, player: i32) -> i32 {
            let i = self.players.iter().position(|&p| p == player).unwrap();
            self.armies[i].len() as i32
        }
        fn army_at(&mut self, player: i32, j: i32) -> i32 {
            let i = self.players.iter().position(|&p| p == player).unwrap();
            self.armies[i][j as usize]
        }
        fn army_name_empty(&mut self, army: i32) -> bool {
            !self.named.contains(&army)
        }
        fn army_region(&mut self, army: i32) -> i32 {
            self.army_regions
                .iter()
                .find(|(a, _)| *a == army)
                .map(|(_, r)| *r)
                .unwrap_or(0)
        }
        fn merge_armies(&mut self, first: i32, army: i32) {
            self.merged.push((first, army));
        }
        fn erase_army(&mut self, player: i32, j: i32) {
            let i = self.players.iter().position(|&p| p == player).unwrap();
            self.armies[i].remove(j as usize);
        }
        fn delete_army(&mut self, army: i32) {
            self.deleted.push(army);
        }
    }

    #[test]
    fn merge_keeps_first_targets_and_merges_the_rest() {
        let mut mock = MergeMock {
            set: 5,
            regions: vec![11],
            region_player: 7,
            players: vec![1],
            armies: vec![vec![100, 101, 102, 103, 104]],
            named: vec![100, 104],
            army_regions: vec![
                (100, 11),
                (101, 11),
                (102, 12), // unnamed but wrong region
                (103, 11),
                (104, 11),
            ],
            deleted: vec![],
            merged: vec![],
        };
        living_world_logic_merge_unnamed_armies(&mut mock);
        assert_eq!(mock.merged, vec![(101, 103)]);
        assert_eq!(mock.deleted, vec![103]);
        // 100/104 skipped as named; 102 advanced for wrong region
        assert_eq!(mock.armies[0], vec![100, 101, 102, 104]);
    }

    #[test]
    fn merge_skips_region_without_player_and_refuses_player() {
        let mut mock = MergeMock {
            set: 5,
            regions: vec![11],
            region_player: -1,
            players: vec![1],
            armies: vec![vec![101, 103]],
            named: vec![],
            army_regions: vec![(101, 11), (103, 11)],
            deleted: vec![],
            merged: vec![],
        };
        living_world_logic_merge_unnamed_armies(&mut mock);
        assert!(mock.merged.is_empty());
        mock.region_player = 7;
        living_world_logic_merge_unnamed_armies(&mut mock);
        assert_eq!(mock.merged, vec![(101, 103)]);
    }

    // ---- row 6: window cleanup ----

    struct WinMock {
        log: Vec<&'static str>,
    }

    impl WindowCleanupEvents for WinMock {
        fn rva_008e400d(&mut self) { self.log.push("e400d"); }
        fn rva_0090e9d3(&mut self) { self.log.push("e9d3"); }
        fn rva_008e855c(&mut self) { self.log.push("e855c"); }
        fn rva_008e84ab(&mut self) { self.log.push("e84ab"); }
        fn rva_00911730(&mut self) { self.log.push("11730"); }
        fn hide_purchase_science(&mut self) { self.log.push("sci"); }
        fn hide_special_power_shortcut(&mut self) { self.log.push("pow"); }
        fn rva_004cf155(&mut self) { self.log.push("cf155"); }
        fn rva_009cb265(&mut self) { self.log.push("cb265"); }
        fn strategic_message_box_hide(&mut self) { self.log.push("msgbox"); }
        fn apt_tribute_enable(&mut self) { self.log.push("trib"); }
        fn apt_quit_enable(&mut self) { self.log.push("quit"); }
        fn apt_window_manager_slot28(&mut self) { self.log.push("apt28"); }
        fn window_manager_slot28(&mut self) { self.log.push("wm28"); }
        fn options_menu_back_click(&mut self) { self.log.push("opts"); }
        fn destroy_windows(&mut self, layout: u32) {
            self.log.push("destroy");
            assert_eq!(layout, 0x1234);
        }
        fn destructor_0(&mut self, layout: u32) -> u32 {
            self.log.push("dtor");
            layout ^ 0xff
        }
        fn free(&mut self, ptr: u32) {
            self.log.push("free");
            assert_eq!(ptr, 0x1234 ^ 0xff);
        }
        fn rva_00837e9c_zero(&mut self) { self.log.push("37e9c"); }
    }

    #[test]
    fn window_cleanup_guards_and_background_arms() {
        let mut bits = GameLogicWindowBits {
            background: 0x1234,
            background_pending: 1,
        };
        let mut ev = WinMock { log: vec![] };
        game_logic_window_cleanup(&mut bits, true, false, true, &mut ev);
        assert_eq!(
            ev.log,
            vec![
                "e400d", "e9d3", "e855c", "e84ab", "11730", "sci", "pow", "cf155",
               "cb265", "msgbox", "quit", "apt28", "wm28", "opts",
                "destroy", "dtor", "free"
            ]
        );
        assert_eq!(bits.background, 0);
        assert_eq!(bits.background_pending, 0);

        // pending with null background: the 0x00837E9C(0) arm, no destroy
        let mut bits2 = GameLogicWindowBits {
            background: 0,
            background_pending: 1,
        };
        let mut ev2 = WinMock { log: vec![] };
        game_logic_window_cleanup(&mut bits2, false, false, false, &mut ev2);
        let tail = &ev2.log[ev2.log.len() - 2..];
        assert_eq!(tail, &["opts", "37e9c"]);

        // clear pending: neither tail arm runs, and the global guards stay off
        let mut bits3 = GameLogicWindowBits {
            background: 0x1234,
            background_pending: 0,
        };
        let mut ev3 = WinMock { log: vec![] };
        game_logic_window_cleanup(&mut bits3, false, true, false, &mut ev3);
        assert!(!ev3.log.contains(&"destroy"));
        assert!(!ev3.log.contains(&"37e9c"));
        assert_eq!(bits3.background, 0);
    }

    // ---- row 7: friend_awakenUpdateModule ----

    fn sched_setup() -> (Vec<AwakenModule>, AwakenScheduler) {
        let modules = vec![
            AwakenModule {
                next_call_frame: 10,
                index_in_logic: 0,
                phase_in_logic: -1,
            },
            AwakenModule {
                next_call_frame: 10,
                index_in_logic: -1,
                phase_in_logic: 2,
            },
        ];
        let sched = AwakenScheduler {
            frame: 5,
            phase_updates: [vec![], vec![], vec![], vec![]],
            sleeping: vec![0],
            current: None,
        };
        (modules, sched)
    }

    #[test]
    fn awaken_entry_guards() {
        let (mut modules, mut sched) = sched_setup();
        // current module rejected
        sched.current = Some(0);
        assert_eq!(
            game_logic_friend_awaken_update_module(&mut modules, &mut sched, 0, 11, true, |_| 0),
            AwakenOutcome::Rejected
        );
        sched.current = None;
        // when already equals the module's next-call frame
        assert_eq!(
            game_logic_friend_awaken_update_module(&mut modules, &mut sched, 0, 10, true, |_| 0),
            AwakenOutcome::Rejected
        );
        // frame == next-call frame and when == frame + 1
        modules[0].next_call_frame = 5;
        assert_eq!(
            game_logic_friend_awaken_update_module(&mut modules, &mut sched, 0, 6, true, |_| 0),
            AwakenOutcome::Rejected
        );
    }

    #[test]
    fn awaken_moves_module_and_sleep_back_index() {
        let mut modules = vec![
            AwakenModule {
                next_call_frame: 10,
                index_in_logic: 1,
                phase_in_logic: -1,
            },
            AwakenModule {
                next_call_frame: 7,
                index_in_logic: 0,
                phase_in_logic: -1,
            },
            AwakenModule::default(),
            AwakenModule::default(),
            AwakenModule::default(),
            AwakenModule::default(),
            AwakenModule::default(),
            AwakenModule::default(),
            AwakenModule::default(),
            AwakenModule {
                next_call_frame: 3,
                index_in_logic: 2,
                phase_in_logic: -1,
            },
        ];
        let sched = AwakenScheduler {
            frame: 5,
            phase_updates: [vec![], vec![], vec![], vec![]],
            sleeping: vec![1, 0, 9],
            current: None,
        };
        let mut sched = sched;
        let out = game_logic_friend_awaken_update_module(
            &mut modules,
            &mut sched,
            0,
            12,
            true,
            |_| 2,
        );
        assert_eq!(
            out,
            AwakenOutcome::Awakened {
                new_phase: 2,
                new_index: 0
            }
        );
        // sleeping swapped the back element into slot 1: 9 -> index 1, phase -1
        assert_eq!(sched.sleeping, vec![1, 9]);
        assert_eq!(modules[9].index_in_logic, 1);
        assert_eq!(modules[9].phase_in_logic, -1);
        assert_eq!(sched.phase_updates[2], vec![0]);
        assert_eq!(modules[0].index_in_logic, 0);
        assert_eq!(modules[0].phase_in_logic, 2);
        assert_eq!(modules[0].next_call_frame, 12);
    }

    #[test]
    fn awaken_clamps_signed_and_reports_fatal() {
        let (mut modules, mut sched) = sched_setup();
        modules[0].index_in_logic = 0;
        modules[0].phase_in_logic = -1;
        // phase < 0 but when >= forever (signed): no sleeping move, plain
        // clamped store
        let out = game_logic_friend_awaken_update_module(
            &mut modules,
            &mut sched,
            0,
            0x4000_0000,
            true,
            |_| panic!("getUpdatePhase not reached when when >= forever"),
        );
        assert_eq!(out, AwakenOutcome::Rescheduled);
        assert_eq!(modules[0].next_call_frame, UPDATE_SLEEP_FOREVER);
        // a negative (0x80000000) when survives the signed clamp verbatim
        modules[0].next_call_frame = 10;
        let out = game_logic_friend_awaken_update_module(
            &mut modules,
            &mut sched,
            0,
            0x8000_0000,
            true,
            |_| 1,
        );
        assert_eq!(
            out,
            AwakenOutcome::Awakened {
                new_phase: 1,
                new_index: 0
            }
        );
        assert_eq!(modules[0].next_call_frame, 0x8000_0000);
        // not in the object list with an index: the fatal arm leaves the
        // frame untouched
        modules[1].index_in_logic = 3;
        let before = modules[1].next_call_frame;
        let out = game_logic_friend_awaken_update_module(
            &mut modules,
            &mut sched,
            1,
            42,
            false,
            |_| unreachable_err(),
        );
        assert_eq!(out, AwakenOutcome::Fatal);
        assert_eq!(modules[1].next_call_frame, before);
    }

    fn unreachable_err() -> i32 {
        panic!("getUpdatePhase must not run on the not-in-list path");
    }

    // ---- row 8: farthest spawn point ----

    struct SpawnMock {
        start_ok: bool,
        live: std::collections::HashSet<i32>,
        evals: Vec<Option<(f32, SpawnPoint, SpawnPoint)>>,
        n_eval: usize,
    }

    fn pt(v: f32) -> SpawnPoint {
        SpawnPoint { x: v, y: v, z: v }
    }

    impl FarthestSpawnHost for SpawnMock {
        fn prepare_start(&mut self, key: i32, player: i32) -> bool {
            assert_eq!(key, 0x12c);
            assert_eq!(player, 2);
            self.start_ok
        }
        fn lookup_region(&mut self, id: i32) -> i32 {
            if self.live.contains(&id) {
                1
            } else {
                0
            }
        }
        fn prepare_other(&mut self, _key: i32) {}
        fn evaluate_pair(&mut self) -> Option<(f32, SpawnPoint, SpawnPoint)> {
            let v = self.evals[self.n_eval];
            self.n_eval += 1;
            v
        }
    }

    #[test]
    fn farthest_tracks_best_and_requires_positive_cost() {
        let region = SpawnRegion {
            region_key: 0x12c,
            records: vec![
                SpawnRecord { region_id: 5 }, // not live -> skipped
                SpawnRecord { region_id: 6 },
                SpawnRecord { region_id: 7 },
                SpawnRecord { region_id: 8 },
                SpawnRecord { region_id: 9 },
            ],
        };
        let mut mock = SpawnMock {
            start_ok: true,
            live: [6, 7, 8, 9].into_iter().collect(),
            evals: vec![
                Some((5.0, pt(1.0), pt(2.0))),
                Some((3.0, pt(9.0), pt(9.0))),              // loses
                Some((f32::NAN, pt(8.0), pt(8.0))),         // NaN never wins
                Some((5.0, pt(3.0), pt(4.0))),              // tie wins (>=)
            ],
            n_eval: 0,
        };
        let mut spawn = [0.0; 3];
        let mut gather = [0.0; 3];
        assert!(army_placer_get_farthest_spawn_point_from_player_start(
            &region, 2, &mut spawn, &mut gather, &mut mock
        ));
        assert_eq!(spawn, [3.0, 3.0, 3.0]);
        assert_eq!(gather, [4.0, 4.0, 4.0]);

        // a zero-cost win still fails the final `0.0 < highest` gate
        let mut mock2 = SpawnMock {
            start_ok: true,
            live: [6].into_iter().collect(),
            evals: vec![Some((0.0, pt(1.0), pt(1.0)))],
            n_eval: 0,
        };
        assert!(!army_placer_get_farthest_spawn_point_from_player_start(
            &region, 2, &mut spawn, &mut gather, &mut mock2
        ));
        // failed start record: no evaluation at all
        let mut mock3 = SpawnMock {
            start_ok: false,
            live: [6].into_iter().collect(),
            evals: vec![],
            n_eval: 0,
        };
        assert!(!army_placer_get_farthest_spawn_point_from_player_start(
            &region, 2, &mut spawn, &mut gather, &mut mock3
        ));
        assert_eq!(mock3.n_eval, 0);
    }

    // ---- row 9: adjust army target locations ----

    struct TargetMock {
        log: Vec<String>,
        active: Vec<i32>,
    }

    impl ArmyTargetHost for TargetMock {
        fn player_count(&mut self) -> i32 {
            1
        }
        fn player_at(&mut self, _i: i32) -> i32 {
            1
        }
        fn army_count(&mut self, _p: i32) -> i32 {
            2
        }
        fn army_at(&mut self, _p: i32, j: i32) -> i32 {
            10 + j
        }
        fn army_active(&mut self, army: i32) -> bool {
            self.active.contains(&army)
        }
        fn army_home_region(&mut self, army: i32) -> i32 {
            if army == 10 {
                0
            } else {
                44
            }
        }
        fn region_release_army(&mut self, region: i32, army: i32) {
            self.log.push(format!("release{region}/{army}"));
        }
        fn army_target_connection(&mut self, _army: i32) -> i32 {
            77
        }
        fn connection_find_spot(&mut self, conn: i32, army: i32) -> Option<(i32, i32)> {
            if army == 11 {
                Some((conn, 7))
            } else {
                None
            }
        }
        fn army_set_target(&mut self, army: i32, a: i32, b: i32) {
            self.log.push(format!("target{army}={a},{b}"));
        }
    }

    #[test]
    fn targets_release_all_before_allocating() {
        let mut mock = TargetMock {
            log: vec![],
            active: vec![10, 11],
        };
        living_world_logic_adjust_army_target_locations(&mut mock);
        assert_eq!(
            mock.log,
            // army 10: null home region -> no release; pass two finds no spot
            // army 11: released once in pass one, targeted in pass two
            vec!["release44/11", "target11=77,7"]
        );
        let mut inactive = TargetMock {
            log: vec![],
            active: vec![],
        };
        living_world_logic_adjust_army_target_locations(&mut inactive);
        assert!(inactive.log.is_empty());
    }

    // ---- row 10: tryToClaimCell ----

    fn claim_probe(
        cell: &mut ResourceCell,
        exclusive: bool,
        id: i32,
        lookup: ClaimObjectLookup,
    ) -> (i32, i32) {
        let mut cells = vec![ResourceCell {
            state: cell.state,
            claims: cell.claims.clone(),
        }];
        let mut attempts = 0;
        let mut successes = 0;
        terrain_resource_manager_try_to_claim_cell(
            Some(&mut cells),
            1,
            1,
            0.0,
            0.0,
            10.0,
            0,
            0,
            0x31,
            id,
            exclusive,
            &mut attempts,
            &mut successes,
            |_| lookup,
        );
        cell.state = cells[0].state;
        cell.claims = cells[0].claims.clone();
        (attempts, successes)
    }

    #[test]
    fn try_claim_state0_sets_two_or_three_and_counts() {
        let mut cell = ResourceCell {
            state: 0,
            claims: vec![(9, 9)],
        };
        let (a, s) = claim_probe(&mut cell, false, 55, ClaimObjectLookup::Player(3));
        assert_eq!((a, s), (1, 1));
        assert_eq!(cell.state, 2);
        assert_eq!(cell.claims, vec![(3, 55)]); // old claims erased
        let mut cell3 = ResourceCell { state: 0, claims: vec![] };
        let (a, s) = claim_probe(&mut cell3, true, 55, ClaimObjectLookup::Player(3));
        assert_eq!((a, s), (1, 1));
        assert_eq!(cell3.state, 3);
    }

    #[test]
    fn try_claim_state2_and_3_semantics() {
        // shared duplicate claim: counted attempt, no success, no extra claim
        let mut cell = ResourceCell { state: 2, claims: vec![(3, 7)] };
        let (a, s) = claim_probe(&mut cell, false, 55, ClaimObjectLookup::Player(3));
        assert_eq!((a, s), (1, 0));
        assert_eq!(cell.claims, vec![(3, 7)]);
        // shared new player: push + success
        let mut cell = ResourceCell { state: 2, claims: vec![(4, 7)] };
        let (a, s) = claim_probe(&mut cell, false, 55, ClaimObjectLookup::Player(3));
        assert_eq!((a, s), (1, 1));
        assert_eq!(cell.claims, vec![(4, 7), (3, 55)]);
        // exclusive on shared state: restart as 3 with one claim
        let mut cell = ResourceCell { state: 2, claims: vec![(4, 7), (5, 8)] };
        let (a, s) = claim_probe(&mut cell, true, 55, ClaimObjectLookup::Player(3));
        assert_eq!((a, s), (1, 1));
        assert_eq!(cell.state, 3);
        assert_eq!(cell.claims, vec![(3, 55)]);
        // shared against exclusive cell: untouched
        let mut cell = ResourceCell { state: 3, claims: vec![(4, 7)] };
        let (a, s) = claim_probe(&mut cell, false, 55, ClaimObjectLookup::Player(3));
        assert_eq!((a, s), (1, 0));
        assert_eq!(cell.claims, vec![(4, 7)]);
        // exclusive re-claim by the same object: the claimable tail gate
        // matches `id == first.object`, and the in-place overwrite counts
        let mut cell = ResourceCell { state: 3, claims: vec![(4, 55)] };
        let (a, s) = claim_probe(&mut cell, true, 55, ClaimObjectLookup::Player(3));
        assert_eq!((a, s), (1, 1));
        assert_eq!(cell.claims, vec![(3, 55)]);
        // exclusive on an empty state-3 cell: the claimable self-heal flips
        // the state to 0 first, so try_to_claim lands on the state-0 path
        // (which stamps state 3 for exclusive and counts the success)
        let mut cell = ResourceCell { state: 3, claims: vec![] };
        let (a, s) = claim_probe(&mut cell, true, 55, ClaimObjectLookup::Player(3));
        assert_eq!((a, s), (1, 1));
        assert_eq!(cell.state, 3);
        assert_eq!(cell.claims, vec![(3, 55)]);
    }

    #[test]
    fn try_claim_fails_before_counting_or_after_lookup() {
        // null cell array: nothing counted
        let mut attempts = 0;
        let mut successes = 0;
        terrain_resource_manager_try_to_claim_cell(
            None, 1, 1, 0.0, 0.0, 10.0, 0, 0, 1, 2, false,
            &mut attempts, &mut successes,
            |_| ClaimObjectLookup::Player(3),
        );
        assert_eq!((attempts, successes), (0, 0));
        // blocked state 1: attempt only
        let mut cell = ResourceCell { state: 1, claims: vec![] };
        let (a, s) = claim_probe(&mut cell, false, 55, ClaimObjectLookup::Player(3));
        assert_eq!((a, s), (1, 0));
        // unresolvable object: attempt counted, no claim written
        let mut cell = ResourceCell { state: 0, claims: vec![] };
        let (a, s) = claim_probe(&mut cell, false, 55, ClaimObjectLookup::NotFound);
        assert_eq!((a, s), (1, 0));
        assert!(cell.claims.is_empty());
        let mut cell = ResourceCell { state: 0, claims: vec![] };
        let (a, s) = claim_probe(&mut cell, false, 55, ClaimObjectLookup::NoController);
        assert_eq!((a, s), (1, 0));
    }

    // ---- row 11: LivingWorld tactical battle complete ----

    struct CompleteMock {
        log: Vec<String>,
        battle: i32,
        players: std::collections::VecDeque<i32>,
        side: i32,
        slot: i32,
        victor: i32,
        mgr_flag: bool,
        flag18: bool,
        end_flag: bool,
    }

    impl BattleCompleteHost for CompleteMock {
        fn army_summary_save(&mut self) { self.log.push("save".into()); }
        fn current_battle(&mut self) -> i32 { self.battle }
        fn each_player_from_mask(&mut self, mask: &mut i32) -> i32 {
            match self.players.pop_front() {
                Some(p) => p,
                None => {
                    *mask = 0;
                    0
                }
            }
        }
        fn accumulate_stats(&mut self, p: i32) { self.log.push(format!("acc{p}")); }
        fn player_score_block(&mut self, p: i32) -> i32 {
            if p == 1 { 0 } else { 0x3bc0 + p }
        }
        fn find_living_world_player(&mut self, _p: i32) -> i32 { 5 }
        fn battle_side_of(&mut self, _b: i32, lw: i32) -> i32 {
            if lw == 5 { self.side } else { self.side }
        }
        fn battle_slot_of(&mut self, _s: i32, _lw: i32) -> i32 { self.slot }
        fn battle_apply_block(&mut self, s: i32, sl: i32, b: i32) {
            self.log.push(format!("block{s}/{sl}={b}"));
        }
        fn tactical_victor(&mut self) -> i32 { self.victor }
        fn victor_player_index(&mut self, _v: i32) -> i32 { 4 }
        fn set_battle_side(&mut self, b: i32, s: i32) {
            self.log.push(format!("side{b}={s}"));
        }
        fn manager_living_world_flag(&mut self) -> bool { self.mgr_flag }
        fn living_world_flag_18(&mut self) -> bool { self.flag18 }
        fn gamelogic_end_session_flag(&mut self) -> bool { self.end_flag }
        fn send_end_session_msg(&mut self) { self.log.push("endsession".into()); }
        fn manager_msg_211fa8(&mut self) { self.log.push("mgr211fa8".into()); }
        fn clear_game_data(&mut self, a: bool, b: bool) {
            self.log.push(format!("clear{a}{b}"));
        }
        fn transition_from_living_world(&mut self) { self.log.push("transition".into()); }
    }

    #[test]
    fn battle_complete_walk_stores_victor_and_branches() {
        let mut mock = CompleteMock {
            log: vec![],
            battle: 8,
            players: vec![1, 2].into_iter().collect(),
            side: 2,
            slot: 1,
            victor: 77,
            mgr_flag: true,
            flag18: false,
            end_flag: true,
        };
        let mut st = LivingWorldBattleCompleteState::default();
        let tail = game_logic_living_world_tactical_battle_complete(&mut st, &mut mock);
        assert_eq!(tail, BattleCompleteTail::EndSessionMsg);
        assert_eq!(st.living_world_victor_index, 4);
        // player 1 has a null +0x3BC block: accumulated but not handed off
        assert_eq!(
            mock.log,
            vec![
                "save", "acc1", "acc2", "block2/1=15298", "side8=2", "endsession"
            ]
        );
        // no victor: -1 store, no side store, manager-msg branch instead
        let mut mock2 = CompleteMock { victor: 0, end_flag: false, ..mock };
        mock2.log = vec![];
        mock2.players = vec![].into_iter().collect();
        let mut st2 = LivingWorldBattleCompleteState::default();
        let tail2 = game_logic_living_world_tactical_battle_complete(&mut st2, &mut mock2);
        assert_eq!(tail2, BattleCompleteTail::ManagerMsg);
        assert_eq!(st2.living_world_victor_index, -1);
        assert_eq!(mock2.log, vec!["save", "mgr211fa8"]);
        // manager flag clear picks the transition branch
        let mut mock3 = CompleteMock { mgr_flag: false, flag18: true, ..mock2 };
        mock3.log = vec![];
        let tail3 = game_logic_living_world_tactical_battle_complete(&mut st2, &mut mock3);
        assert_eq!(tail3, BattleCompleteTail::ClearAndTransition);
        assert_eq!(
            mock3.log,
            vec![
                "save".to_string(),
                "cleartruefalse".to_string(),
                "transition".to_string()
            ]
        );
    }

    // ---- row 12: turn phase transitions ----

    struct PhaseMock {
        log: Vec<&'static str>,
        mgr_flag: bool,
        upg_flag: bool,
    }

    impl PhaseTransitionHost for PhaseMock {
        fn rva_002b2bec(&mut self) { self.log.push("2bec"); }
        fn rva_002b3b66(&mut self) { self.log.push("3b66"); }
        fn merge_unnamed_armies(&mut self) { self.log.push("merge"); }
        fn rva_002b3d0e(&mut self) { self.log.push("3d0e"); }
        fn update_battle_markers(&mut self) { self.log.push("markers"); }
        fn process_delayed_region_victories(&mut self) { self.log.push("delayed"); }
        fn rva_002b6900(&mut self) { self.log.push("6900"); }
        fn rva_002b9dc7(&mut self) { self.log.push("9dc7"); }
        fn notify_turn_listeners(&mut self, from: i32, to: i32) {
            self.log.push(if from == -1 && to == 0 { "l:-1>0" } else { "l" });
        }
        fn auto_mark_upgrades(&mut self) { self.log.push("mark"); }
        fn check_head_to_head_battles(&mut self) { self.log.push("h2h"); }
        fn adjust_army_target_locations(&mut self) { self.log.push("adjust"); }
        fn manager_rva_0020e4c8(&mut self) { self.log.push("e4c8"); }
        fn manager_flag_query(&mut self) -> bool { self.mgr_flag }
        fn phase_upgrade_query(&mut self) -> bool { self.upg_flag }
        fn prepare_retreats(&mut self) { self.log.push("retreat"); }
    }

    #[test]
    fn phase_no_change_only_latches() {
        let mut st = TurnPhaseState {
            turn_phase: 3,
            prev_phase: 3,
            flag_108: 0,
            flag_109: 0,
        };
        let mut host = PhaseMock {
            log: vec![],
            mgr_flag: true,
            upg_flag: true,
        };
        living_world_logic_check_turn_phase_transitions(&mut st, &mut host);
        assert!(host.log.is_empty());
        assert_eq!(st.prev_phase, 3);
        assert_eq!(st.flag_108, 0);
    }

    #[test]
    fn phase_edges_and_case2_flags() {
        let mut st = TurnPhaseState {
            turn_phase: 0,
            prev_phase: -1,
            flag_108: 0,
            flag_109: 0,
        };
        let mut host = PhaseMock {
            log: vec![],
            mgr_flag: true,
            upg_flag: true,
        };
        living_world_logic_check_turn_phase_transitions(&mut st, &mut host);
        assert_eq!(host.log, vec!["2bec", "9dc7", "l:-1>0", "merge", "mark"]);
        assert_eq!(st.prev_phase, 0);

        // 1 -> 2: exit arm of phase 1, entry arm of 2 sets both flag bytes
        let mut st2 = TurnPhaseState {
            turn_phase: 2,
            prev_phase: 1,
            flag_108: 0,
            flag_109: 0,
        };
        let mut host2 = PhaseMock {
            log: vec![],
            mgr_flag: true,
            upg_flag: false,
        };
        living_world_logic_check_turn_phase_transitions(&mut st2, &mut host2);
        assert_eq!(
            host2.log,
            vec!["3b66", "merge", "3d0e", "markers", "l", "e4c8", "markers"]
        );
        assert_eq!(st2.flag_108, 1);
        assert_eq!(st2.flag_109, 0);

        // phase 4 entry prepares retreats only -- no adjust tail
        let mut st3 = TurnPhaseState {
            turn_phase: 4,
            prev_phase: 2,
            flag_108: 0,
            flag_109: 0,
        };
        let mut host3 = PhaseMock {
            log: vec![],
            mgr_flag: false,
            upg_flag: false,
        };
        living_world_logic_check_turn_phase_transitions(&mut st3, &mut host3);
        assert_eq!(host3.log, vec!["delayed", "6900", "l", "retreat"]);
        // 5 -> x runs adjust; old-phase 5 exit runs 3b66
        let mut st4 = TurnPhaseState {
            turn_phase: 1,
            prev_phase: 5,
            flag_108: 0,
            flag_109: 0,
        };
        let mut host4 = PhaseMock {
            log: vec![],
            mgr_flag: false,
            upg_flag: false,
        };
        living_world_logic_check_turn_phase_transitions(&mut st4, &mut host4);
        assert_eq!(host4.log, vec!["3b66", "l", "h2h", "adjust"]);
    }

    // ---- row 13: carryover single object ----

    struct CreateMock {
        log: Vec<String>,
        obj: i32,
        sink: i32,
        ai_ready: bool,
        team: i32,
    }

    impl CarryoverCreateHost for CreateMock {
        fn player_team(&mut self, player: i32) -> i32 {
            self.team = player * 2;
            self.team
        }
        fn new_object(&mut self, template: i32, team: i32) -> i32 {
            self.log.push(format!("new{template}/{team}"));
            self.obj
        }
        fn rva_0028cbfd(&mut self, _o: i32) { self.log.push("28cbfd".into()); }
        fn rva_0039b3d1(&mut self, _o: i32, v: f32) {
            self.log.push(format!("exp{v}"));
        }
        fn update_upgrade_modules(&mut self, _o: i32) { self.log.push("upgrades".into()); }
        fn find_sink(&mut self, _o: i32) -> i32 { self.sink }
        fn sink_ai_ready(&mut self, _s: i32) -> bool { self.ai_ready }
        fn sink_ai_apply_upgrades(&mut self, _s: i32) { self.log.push("slot68".into()); }
    }

    fn unit_setup() -> CarryoverUnit {
        let mut u = CarryoverUnit {
            thing_template: 4,
            experience: 25.0,
            upgrades: [0; 32],
            record_field00: 31,
            record_reset04: 9,
            record_flag14: 1,
        };
        u.upgrades[5] = 0xf00d;
        u
    }

    #[test]
    fn carryover_create_flags_and_order() {
        let mut flag98 = 0xAAu8;
        let mut object = CreatedObject {
            tracker_experience: 10.0,
            tracker_flag: 1,
            upgrades: [0; 32],
            record_field00: 0,
            record_reset04: 0,
            record_flag14: 1,
        };
        let mut host = CreateMock {
            log: vec![],
            obj: 8,
            sink: 9,
            ai_ready: true,
            team: 0,
        };
        let unit = unit_setup();
        let obj = carryover_unit_create_single_object(&unit, &mut flag98, &mut object, 3, &mut host);
        assert_eq!(obj, 8);
        assert_eq!(flag98, 0xAA); // restored
        assert_eq!(
            host.log,
            vec!["new4/6", "28cbfd", "exp25", "upgrades", "slot68"]
        );
        assert_eq!(object.tracker_flag, 0);
        assert_eq!(object.upgrades[5], 0xf00d);
        assert_eq!(object.record_field00, 31);
        assert_eq!(object.record_reset04, 0);
        assert_eq!(object.record_flag14, 0);
    }

    #[test]
    fn carryover_create_experience_gate_nan_and_failure() {
        // equal tracker value: helper skipped, flag kept
        let mut flag98 = 1u8;
        let mut object = CreatedObject {
            tracker_experience: 25.0,
            tracker_flag: 1,
            upgrades: [0; 32],
            ..Default::default()
        };
        let mut host = CreateMock {
            log: vec![],
            obj: 8,
            sink: 0,
            ai_ready: false,
            team: 0,
        };
        carryover_unit_create_single_object(&unit_setup(), &mut flag98, &mut object, 0, &mut host);
        assert_eq!(object.tracker_flag, 1);
        assert!(!host.log.iter().any(|l| l.starts_with("exp")));
        // NaN tracker: (tracker <= value) is false -- gate skipped
        object.tracker_experience = f32::NAN;
        let mut host = CreateMock {
            log: vec![],
            obj: 8,
            sink: 0,
            ai_ready: false,
            team: 0,
        };
        carryover_unit_create_single_object(&unit_setup(), &mut flag98, &mut object, 0, &mut host);
        assert!(!host.log.iter().any(|l| l.starts_with("exp")));
        // NaN value with finite tracker: (5 <= NaN) false -- skipped
        object.tracker_experience = 5.0;
        object.tracker_flag = 1;
        let mut unit = unit_setup();
        unit.experience = f32::NAN;
        let mut host = CreateMock {
            log: vec![],
            obj: 8,
            sink: 0,
            ai_ready: false,
            team: 0,
        };
        carryover_unit_create_single_object(&unit, &mut flag98, &mut object, 0, &mut host);
        assert_eq!(object.tracker_flag, 1);
        // factory failure restores +0x98 and returns 0
        let mut flag98b = 7u8;
        let mut obj2 = CreatedObject::default();
        let mut host = CreateMock {
            log: vec![],
            obj: 0,
            sink: 0,
            ai_ready: false,
            team: 0,
        };
        assert_eq!(
            carryover_unit_create_single_object(&unit_setup(), &mut flag98b, &mut obj2, 0, &mut host),
            0
        );
        assert_eq!(flag98b, 7);
        assert_eq!(host.log, vec!["new4/0"]);
        // no template: +0x98 untouched
        let mut no_tmpl = unit_setup();
        no_tmpl.thing_template = 0;
        let mut host = CreateMock {
            log: vec![],
            obj: 8,
            sink: 0,
            ai_ready: false,
            team: 0,
        };
        assert_eq!(
            carryover_unit_create_single_object(&no_tmpl, &mut flag98b, &mut obj2, 0, &mut host),
            0
        );
        assert!(host.log.is_empty());
        assert_eq!(flag98b, 7);
    }

    // ---- row 14: pathfinder search driver ----

    struct PathMock {
        keys: std::collections::HashMap<i32, usize>,
        graph: Vec<Vec<usize>>,
        opened: Vec<usize>,
    }

    impl PathSearchHost for PathMock {
        fn find_node(&mut self, key: i32) -> usize {
            self.keys.get(&key).copied().unwrap_or(NULL_NODE)
        }
        fn opened(&mut self, current: usize) {
            self.opened.push(current);
        }
        fn goal_test(&mut self, current: usize, goal: usize) -> bool {
            current == goal
        }
        fn relax(
            &mut self,
            search: &mut PathSearch,
            _nodes: &mut [PathNode],
            current: usize,
            _opaque: i32,
            _allow_other: bool,
        ) {
            for n in self.graph[current].clone() {
                search.open.push(n);
            }
        }
        fn fill_path(&mut self, current: usize, out: &mut Vec<i32>) {
            // stand-in for the predecessor-chain walk: push the keys found
            for (k, v) in &self.keys {
                if *v == current {
                    out.push(*k);
                }
            }
            out.push(-1); // pretend predecessor chain entries
        }
        fn count_path(&mut self, current: usize) -> i32 {
            current as i32 * 100 + 1
        }
    }

    #[test]
    fn pathfinder_short_circuits_same_endpoint() {
        let mut keys = std::collections::HashMap::new();
        keys.insert(7, 0usize);
        let mut mock = PathMock {
            keys,
            graph: vec![],
            opened: vec![],
        };
        let mut nodes = vec![PathNode::default()];
        let mut search = PathSearch::default();
        let mut out: Option<Vec<i32>> = Some(vec![9, 9]);
        assert_eq!(
            living_world_path_finder_find_shortest_path(
                &mut search,
                &mut nodes,
                0,
                7,
                7,
                &mut out,
                false,
                &mut mock
            ),
            0
        );
        assert_eq!(out, Some(vec![7]));
        // no output vector: still 0
        assert_eq!(
            living_world_path_finder_find_shortest_path(
                &mut search,
                &mut nodes,
                0,
                7,
                7,
                &mut None,
                false,
                &mut mock
            ),
            0
        );
    }

    #[test]
    fn pathfinder_three_node_chain() {
        let mut keys = std::collections::HashMap::new();
        keys.insert(10, 0usize); // A
        keys.insert(20, 1usize); // B
        keys.insert(30, 2usize); // C (goal)
        let mut mock = PathMock {
            keys,
            graph: vec![vec![1], vec![2], vec![]],
            opened: vec![],
        };
        let mut nodes = vec![PathNode {
            cost: 9.0,
            estimate: 9.0,
            total: 9.0,
            ..Default::default()
        }; 3];
        let mut search = PathSearch {
            open: vec![42],
            closed: vec![43],
            ..Default::default()
        };
        let mut out: Option<Vec<i32>> = Some(vec![5, 6]);
        let r = living_world_path_finder_find_shortest_path(
            &mut search,
            &mut nodes,
            0x77,
            10,
            30,
            &mut out,
            true,
            &mut mock,
        );
        assert_eq!(mock.opened, vec![0, 1, 2]);
        assert_eq!(search.open, Vec::<usize>::new()); // open fully drained
        assert_eq!(search.closed, vec![0, 1]);
        // start node's costs/previous zeroed
        assert_eq!(nodes[0], PathNode::default());
        // fill_path pushed key 30 + one chain entry: len 2 -> r = 1
        assert_eq!(r, 1);
        assert_eq!(out, Some(vec![30, -1]));
    }

    #[test]
    fn pathfinder_exhaustion_and_missing_endpoints() {
        let mut keys = std::collections::HashMap::new();
        keys.insert(10, 0usize);
        let mut mock = PathMock {
            keys,
            graph: vec![vec![]],
            opened: vec![],
        };
        let mut nodes = vec![PathNode::default()];
        let mut search = PathSearch::default();
        // goal missing: -1, slots still written (goal = NULL)
        assert_eq!(
            living_world_path_finder_find_shortest_path(
                &mut search,
                &mut nodes,
                0,
                10,
                99,
                &mut None,
                false,
                &mut mock
            ),
            -1
        );
        assert_eq!(search.start, 0);
        assert_eq!(search.goal, NULL_NODE);
        // reachable but no path: exhausts open, -1; goal-less pop runs count
        // branch only via goal_test false path
        assert!(mock.opened.is_empty());
        // goal lookup failed: the search body never runs
        assert!(search.closed.is_empty());
    }

    // ---- row 15: delayed region victories ----

    struct DelayedMock {
        log: Vec<String>,
        live: i32,
        owner: i32,
        local: i32,
    }

    impl DelayedVictoryHost for DelayedMock {
        fn lookup_region(&mut self, id: i32) -> i32 {
            self.log.push(format!("look{id}"));
            if id == 0 {
                0
            } else {
                self.live
            }
        }
        fn region_owner(&mut self, _region: i32) -> i32 {
            self.owner
        }
        fn local_player_index(&mut self) -> i32 {
            self.local
        }
        fn region_center(&mut self, _region: i32) -> (f32, f32) {
            (3.0, 4.0)
        }
        fn lift_to_terrain(&mut self, x: f32, y: f32) -> (f32, f32, f32) {
            (x + 1.0, y + 1.0, -5.0)
        }
        fn report_eva_event_21(&mut self, x: f32, y: f32, z: f32) {
            self.log.push(format!("eva{x},{y},{z}"));
        }
        fn apply_ownership(&mut self, region: i32, player: i32) {
            self.log.push(format!("own{region}/{player}"));
        }
        fn sync_region_effects(&mut self, region: i32) {
            self.log.push(format!("sync{region}"));
        }
        fn rva_002b9dc7(&mut self) { self.log.push("9dc7".into()); }
        fn rva_0020f483(&mut self) { self.log.push("f483".into()); }
    }

    #[test]
    fn delayed_victories_eva_gate_and_queue_clear() {
        let mut queue = vec![
            // entry 0: null region -> skipped entirely
            DelayedRegionVictory { player: 1, region_id: 0, flags: 1 },
            // entry 1: plain victory, owner == local -> eva + ownership
            DelayedRegionVictory { player: 3, region_id: 7, flags: 1 },
            // entry 2: victory-with-side-flags (bit1) -> no eva, still own+sync
            DelayedRegionVictory { player: 4, region_id: 8, flags: 3 },
            // entry 3: plain victory, owner mismatch -> no eva
            DelayedRegionVictory { player: 5, region_id: 9, flags: 1 },
        ];
        let mut host = DelayedMock {
            log: vec![],
            live: 12,
            owner: 2,
            local: 2,
        };
        living_world_logic_process_delayed_region_victories(&mut queue, &mut host);
        assert_eq!(
            host.log,
            vec![
                "look0",
                "look7",
                "eva4,5,-5",
                "own12/3",
                "sync12",
                "look8",
                "own12/4",
                "sync12",
                "look9",
                "eva4,5,-5",
                "own12/5",
                "sync12",
                "9dc7",
                "f483",
            ]
        );
        assert!(queue.is_empty());
    }

    #[test]
    fn delayed_victories_null_player_skips() {
        let mut queue = vec![DelayedRegionVictory { player: 0, region_id: 7, flags: 1 }];
        let mut host = DelayedMock {
            log: vec![],
            live: 12,
            owner: 2,
            local: 2,
        };
        living_world_logic_process_delayed_region_victories(&mut queue, &mut host);
        assert_eq!(host.log, vec!["look7", "9dc7", "f483"]);
    }

    // ---- row 16: AI builder defender score ----

    struct ScoreMock {
        defenders: i32,
        distance: i32,
        calls: Vec<(i32, i32)>,
    }

    impl BuilderScoreHost for ScoreMock {
        fn defenders(&mut self, player_id: i32, region_id: i32) -> i32 {
            self.calls.push((player_id, region_id));
            self.defenders
        }
        fn closest_hero_distance(&mut self, player_id: i32, region_id: i32) -> i32 {
            self.calls.push((player_id, region_id));
            self.distance
        }
    }

    fn builder_rec(begin: i32, end: i32) -> BuilderRegionRecord {
        BuilderRegionRecord {
            region_id: 44,
            castles_begin: begin,
            castles_end: end,
        }
    }

    #[test]
    fn builder_damping_counts_dwords_and_defender_branch() {
        // three castles, damp factor exactly 0.5 each: 8 * 0.5^3 = 1.0
        let b = AiBuilderWeights { score_weight_08: 8 };
        let t = BuilderTemplate { template_float_30: 5.0 }; // 5*0.1 = 0.5
        let r = builder_rec(0x10, 0x1c); // diff 12 -> 3 dwords
        let mut host = ScoreMock {
            defenders: 8,
            distance: 3,
            calls: vec![],
        };
        let s = living_world_ai_builder_defender_score(&b, &t, &r, 6, &mut host);
        assert_eq!(s, 1.0 * 0.1f32); // 6+ defenders: scale by 0.1
        assert_eq!(host.calls, vec![(6, 44), (6, 44)]);
        // wrapped span: no damping
        let mut host = ScoreMock {
            defenders: 6,
            distance: 3,
            calls: vec![],
        };
        assert_eq!(
            living_world_ai_builder_defender_score(&b, &t, &builder_rec(0x1c, 0x10), 6, &mut host),
            0.8
        );
        // span masked to zero (diff 1 -> 0 after & ~3)
        let mut host = ScoreMock {
            defenders: 6,
            distance: 3,
            calls: vec![],
        };
        assert_eq!(
            living_world_ai_builder_defender_score(&b, &t, &builder_rec(0, 1), 6, &mut host),
            0.8
        );
    }

    #[test]
    fn builder_defender_log_and_distance_falloff() {
        let b = AiBuilderWeights { score_weight_08: 10 };
        let t = BuilderTemplate { template_float_30: 0.0 };
        let r = builder_rec(0, 0);
        // five defenders: log(1.0) = 0 exactly -> score 0, falloff gate fails
        let mut host = ScoreMock {
            defenders: 5,
            distance: 1,
            calls: vec![],
        };
        assert_eq!(
            living_world_ai_builder_defender_score(&b, &t, &r, 0, &mut host),
            0.0
        );
        // two defenders, distance 1: 2 * ln(4) * 10, then * (2/2 + 0.5) = 1.5
        let mut host = ScoreMock {
            defenders: 2,
            distance: 1,
            calls: vec![],
        };
        let s = living_world_ai_builder_defender_score(&b, &t, &r, 0, &mut host);
        let expect: f32 = (2.0f64 * 4.0f64.ln() * 10.0) as f32 * 1.5;
        assert!((s - expect).abs() <= expect.abs() * 1e-6, "{s} vs {expect}");
        // NaN defenders: every ordered comparison is false -> 0.1 branch
        let mut host = ScoreMock {
            defenders: -1, // signed -1 < 6 -> low branch; score*... stays 0-ish
            distance: 5,
            calls: vec![],
        };
        let s = living_world_ai_builder_defender_score(&b, &t, &r, 0, &mut host);
        // -1 < 6 takes the low branch: -1 * ln(6 - -1) * 10; the negative
        // score fails the `0.0 < score` falloff gate
        let expect = -1.0f32 * (7.0f32).ln() * 10.0;
        assert!((s - expect).abs() <= expect.abs() * 1e-6, "{s} vs {expect}");
        // zero distance: no falloff even with positive score
        let mut host = ScoreMock {
            defenders: 1,
            distance: 0,
            calls: vec![],
        };
        let s = living_world_ai_builder_defender_score(&b, &t, &r, 0, &mut host);
        assert!((s - 1.0f32 * (5.0f32).ln() * 10.0).abs() <= 1e-4);
    }
}
