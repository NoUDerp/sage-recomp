//! Wave 47: `GameLogic/Object` band — 8 rows (module `Object`).
//!
//! The band spans the DozerAIUpdate phantom-structure teardown, the
//! SpawnPointProductionExitUpdate door-exit reservation, the
//! PartitionFilterRejectBehind accept test, the AIUpdateInterface
//! locomotor chooser and the two BFME private place-at-last-position
//! commands, the SymbioticStructuresBody drawable refresh, and the
//! ActiveBody damage-FX cooldown gate + record scan.
//!
//! | addr | size | their symbol root | role |
//! |---|---|---|---|
//! | `0x00889d09` | 200 | `DozerAIUpdate::destroyPhantomStructure` | resolve the phantom ID at `+0xc0`, conditionally refund damage + revoke the build, clear the ID |
//! | `0x008a3e3e` | 198 | `SpawnPointProductionExitUpdate::exitObjectViaDoor` | bone-init gate, first-free-occupier scan over the 10-slot spawn arrays, reserve + place the new object |
//! | `0x00661abb` | 195 | `Rva00261ABBFilter` (`PartitionFilterRejectBehind`) | normalize the filter's 3-float direction (cascade `0x0044233a`), dot against the object vector, strict `0.0f <` |
//! | `0x00663fa7` | 194 | `AIUpdateInterface::chooseGoodLocomotorFromCurrentSet` | terrain-looked-up locomotor, else the current one, else create from mask 0x80/8/1; on change clear bits 3/4/6 at `locomotor+0x44` |
//! | `0x008c0c52` | 193 | `SymbioticStructuresBody` (`rva004C0C52`, the pinned refresh) | host (ID `+0x104`) + owner checks, then the pinned Drawable member `(0.2, 0.7, 2.0)` mode 5 on the local-player-selected drawable, mode 0 on the other |
//! | `0x008bed62` | 191 | `ActiveBody::doDamageFX` | `+0xfc`-gated damage-type/time cooldown (early return skips the record scan), then scan the 12-byte records at `+0xe0/+0xe4` |
//! | `0x0066bb5a` | 188 | `AIUpdateInterface::bfmePrivateCommand24` | gate, vtable slot +0x14, place/snap the last position record, store `+0x48`/`+0x1a0`, vtable slot +0x20 with command 0x36 |
//! | `0x0066bc16` | 188 | `AIUpdateInterface::bfmePrivateCommand25` | byte-identical to row 7 except the final vtable argument `0x3d` |
//!
//! Faithful-model conventions (sibling precedent: `pathfinder1.rs`,
//! `systems1.rs`, `scriptw47_1.rs`):
//! - Foreign callees (`FUN_…`) and Ghidra-hidden `thiscall` receivers are
//!   host-trait methods named by their row address; the mirror keeps the
//!   local bookkeeping (field stores, guard order, call order) and tests
//!   pin the exact call sequences. `FUN_00449dc5` is
//!   `TheGameLogic::findObjectByID` and `FUN_0068afa9` the controlling-
//!   player read (both resolved in `contain1.rs`).
//! - Resolved image constants: `0x00bc4dd8 = 0.2f`, `0x00bc8180 = 0.7f`,
//!   `0x00bc28f4 = 2.0f` (the three floats the refresh row hands the
//!   pinned Drawable member; `0x00bc28f4 = 2.0f` also pinned in
//!   `systems1.rs`), `FXParticleSystem::CategoryModuleInfo<0>::CATEGORY`
//!   = `0.0f` (per `systems1.rs`), `DAT_00dfeee8 + 0x10` =
//!   `ThePlayerList` local player, and `FUN_00691c9b(3)` =
//!   `setDisabled(DISABLED_HELD)` (exit row; enum value 3).
//! - The normalize helper `0x0044233a` is duplicated from `math.rs`
//!   (`sin_poly_bits` — same address) because modules must be
//!   self-contained; this row feeds it the squared length **as raw f32
//!   bits** (the dword stack slot) and multiplies at `float10` width —
//!   modeled as `f64` products narrowed on store; f80-vs-f64 double-
//!   rounding edges are out of scope.
//! - Row 5 note: both arms of the decompiled `==/!=` test call the *same*
//!   function `0x009508e2` (`Object::getDrawable`) because Ghidra elides
//!   the `thiscall` receiver; the reference-tree comment for this exact
//!   row records the fact that the mode-5 receiver is the host object
//!   when the owning Object belongs to the local player, else the Object
//!   itself, and mode 0 gets the other drawable.

// ---------------------------------------------------------------------
// 0x00889d09 — DozerAIUpdate::destroyPhantomStructure
// ---------------------------------------------------------------------

/// The `DozerAIUpdate` members this row touches.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DozerPhantomState {
    /// Phantom structure object ID @ +0xc0 (`0` = none).
    pub phantom_id_at_0c0: i32,
    /// Flag byte @ +0x28: nonzero skips the damage-refund block.
    pub flag_28: u8,
    /// Guard dword at this-0x3dc (a field of the enclosing module
    /// block; `this` is a subobject so the body addresses it negative).
    pub dword_m3dc: i32,
}

/// Foreign interactions of the teardown row, one method per call site
/// (Ghidra-hidden `thiscall` receivers recorded in the doc comments).
pub trait DozerPhantomHost {
    /// `FUN_00449dc5` — `TheGameLogic::findObjectByID` (`0` = null).
    fn find_object_00449dc5(&mut self, id: i32) -> i32;
    /// `FUN_0068afa9` — controlling-player pointer (`0` = null).
    fn owner_0068afa9(&mut self) -> i32;
    /// `FUN_00a29228` — the damage amount to refund.
    fn amount_00a29228(&mut self) -> i32;
    /// `FUN_007b0d7c(amount, owner + 0x3bc, 1)` — refund-to-owner
    /// helper; the second arg is the pointer-arithmetic value
    /// `owner + 0x3bc` (wrapping dword add).
    fn call_007b0d7c(&mut self, amount: i32, owner_plus_3bc: i32, one: i32);
    /// `FUN_0079bad2(object_field_04, -amount)`.
    fn call_0079bad2(&mut self, object_field_04: i32, neg_amount: i32);
    /// `object + 4` dword read (through the object handle).
    fn object_dword_04(&mut self, object: i32) -> i32;
    /// `FUN_0079cbce(object, 0xffffffff)`.
    fn call_0079cbce(&mut self, object: i32, minus_one: i32);
    /// `FUN_00642c09(object)`.
    fn call_00642c09(&mut self, object: i32);
    /// `FUN_0069f93a(phantom_id)`.
    fn call_0069f93a(&mut self, phantom_id: i32);
    /// `FUN_006acecc(phantom_id)`.
    fn call_006acecc(&mut self, phantom_id: i32);
}

/// `0x00889d09` — destroy the phantom structure.
///
/// Body order: with a nonzero phantom ID at `+0xc0`, resolve it
/// (`0` = nothing to do but still clear the ID); read the controlling
/// player **unconditionally** (the body calls `0068afa9` before the
/// `+0x28` flag test even when the flag skips the block); with
/// `flag_28 == 0` refund `amount` to `owner + 0x3bc`, push `-amount`
/// onto the object's `+4` dword and `call_0079cbce(obj, -1)`; then
/// `00642c09(obj)`, `0069f93a(id)`, and — guarded by `dword_m3dc != 0`
/// *and* a second non-null owner read — `006acecc(id)`. The ID is
/// cleared whenever it started nonzero. `-amount` is the x86 `neg`
/// (`wrapping_neg`; `-i32::MIN` wraps like the game).
pub fn dozer_ai_update_destroy_phantom_structure(
    st: &mut DozerPhantomState,
    host: &mut impl DozerPhantomHost,
) {
    if st.phantom_id_at_0c0 != 0 {
        let object = host.find_object_00449dc5(st.phantom_id_at_0c0);
        if object != 0 {
            let owner = host.owner_0068afa9();
            if st.flag_28 == 0 {
                let amount = host.amount_00a29228();
                host.call_007b0d7c(amount, owner.wrapping_add(0x3bc), 1);
                let field_04 = host.object_dword_04(object);
                host.call_0079bad2(field_04, amount.wrapping_neg());
                host.call_0079cbce(object, -1);
            }
            host.call_00642c09(object);
            host.call_0069f93a(st.phantom_id_at_0c0);
            if st.dword_m3dc != 0 {
                if host.owner_0068afa9() != 0 {
                    host.call_006acecc(st.phantom_id_at_0c0);
                }
            }
        }
        st.phantom_id_at_0c0 = 0;
    }
}

// ---------------------------------------------------------------------
// 0x008a3e3e — SpawnPointProductionExitUpdate::exitObjectViaDoor
// ---------------------------------------------------------------------

/// The exit-update receiver's spawn arrays. The retail receiver is the
/// `ExitUpdateInterface` subobject (`reference` note); the arrays sit at
/// these this-relative offsets: count `+0x08`, the 12-byte
/// `Coord3D` slots `+0x0c`, the exit angles `+0x84`, the occupier IDs
/// `+0xac` (each array 10 deep; a count above 10 is the game's
/// out-of-bounds walk — the index panics below).
#[derive(Clone, Debug, PartialEq)]
pub struct SpawnPointExitState {
    /// `m_bonesInitialized` byte @ +0x04.
    pub bones_initialized: u8,
    /// `m_spawnPointCount` @ +0x08.
    pub spawn_point_count: i32,
    /// `m_worldCoordSpawnPoints[10]` @ +0x0c.
    pub spawn_points: [[f32; 3]; 10],
    /// `m_worldAngleSpawnPoints[10]` @ +0x84.
    pub spawn_angles: [f32; 10],
    /// `m_spawnPointOccupier[10]` @ +0xac (`0` = free slot).
    pub occupier: [i32; 10],
    /// Creation-object pointer, dword at this-0x18 (module +0x08).
    pub creation_object: i32,
}

/// Foreign interactions of the door-exit row.
pub trait SpawnPointExitHost {
    /// `FUN_008a3cea` — `initializeBonePositions` (body passes the
    /// module base as hidden `this`; modeled as a plain call).
    fn init_bones_008a3cea(&mut self);
    /// `FUN_0070aa80` — `Thing::setPosition` on the new object with
    /// the copied slot (`&local_14` in the body).
    fn set_position_0070aa80(&mut self, pos: [f32; 3]);
    /// `FUN_0070ab9d` — `Thing::setOrientation` with the copied angle.
    fn set_orientation_0070ab9d(&mut self, angle: f32);
    /// `FUN_0068b511` — layer read on the **creation** object (hidden
    /// `this`; reference: `Object::rva0028B511` const getter).
    fn get_layer_0068b511(&mut self) -> i32;
    /// `FUN_0068b4ce` — `Object::setLayer` on the new object.
    fn set_layer_0068b4ce(&mut self, layer: i32);
    /// `FUN_006e7178(newObj)` — pathfinder map add
    /// (`TheAI->m_pathfinder->AddObjectToPathfindMap`).
    fn add_pathfind_006e7178(&mut self);
    /// `FUN_00691c9b(3)` — `Object::setDisabled(DISABLED_HELD)`.
    fn set_disabled_00691c9b(&mut self, kind: i32);
}

/// `0x008a3e3e` — reserve a spawn slot and place the new object.
///
/// Body order: `bones_initialized == 0` triggers the bone-init call;
/// then only with a nonzero creation object, scan the occupier array
/// from slot 0 for the first zero (bounded by `spawn_point_count`).
/// Quirks kept verbatim: the scan is a do-while, so a **negative**
/// count skips the loop yet still fails the `idx != count` test and
/// proceeds with index 0; a full array (`idx == count`) exits. On a
/// free slot the angle and the 12-byte position are copied **before**
/// the occupier store (`m_spawnPointOccupier[i] = newObj->m_id`, the
/// dword at `Object + 0x74`), then
/// set-position → set-orientation → layer copy (get on the creation
/// object, set on the new one) → pathfinder add → `setDisabled(3)`.
/// The `ExitDoorType` argument is never read.
pub fn spawn_point_exit_object_via_door(
    st: &mut SpawnPointExitState,
    new_object_id_at_74: i32,
    host: &mut impl SpawnPointExitHost,
) {
    if st.bones_initialized == 0 {
        host.init_bones_008a3cea();
    }
    if st.creation_object != 0 {
        let count = st.spawn_point_count;
        let mut idx = 0i32;
        if 0 < count {
            loop {
                if st.occupier[idx as usize] == 0 {
                    break;
                }
                idx += 1;
                if idx >= count {
                    break;
                }
            }
        }
        if idx != count {
            let angle = st.spawn_angles[idx as usize];
            let pos = st.spawn_points[idx as usize];
            st.occupier[idx as usize] = new_object_id_at_74;
            host.set_position_0070aa80(pos);
            host.set_orientation_0070ab9d(angle);
            let layer = host.get_layer_0068b511();
            host.set_layer_0068b4ce(layer);
            host.add_pathfind_006e7178();
            host.set_disabled_00691c9b(3);
        }
    }
}

// ---------------------------------------------------------------------
// 0x00661abb — Rva00261ABBFilter (PartitionFilterRejectBehind)::filter
// ---------------------------------------------------------------------

/// `0x0044233a` — duplicated verbatim from `math.rs` (`sin_poly_bits`,
/// same address) because this module must be self-contained. In this
/// row it is the normalize-magnitude helper: the body feeds it the
/// squared length through the dword argument slot (raw f32 bits) and
/// multiplies every component by the result.
fn float_cascade_0044233a(i: i32) -> f32 {
    let f = ((0xbe6eb508u32.wrapping_sub(i as u32)) >> 1) as f32;
    let fd = f as f64;
    let t = fd * fd * ((i - 0x800000) as f32) as f64;
    let a = 1.5 - t;
    let b = t * a * a;
    let c = 1.5 - b;
    ((1.5 - c * b * c) * a * fd * c) as f32
}

/// The filter's data block (member `+0x08` dereferences it): three
/// floats at block `+0x08`, `+0x18`, `+0x28` — the direction the body
/// normalizes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RejectBehindFilterData {
    pub float_08: f32,
    pub float_18: f32,
    pub float_28: f32,
}

/// `0x00661abb` — partition-filter accept test.
///
/// `fill_00661988` models `FUN_00661988(&local_20, obj)`: it fills the
/// three stack floats in address order `(local_20, local_1c, local_18)`
/// from the object — the mirror pairs them componentwise with
/// `(+0x08, +0x18, +0x28)` exactly as the return expression does.
///
/// Body: squared length in the decompiled association
/// (`u28*u28 + u18*u18 + u08*u08`); when `!= 0.0f` (NaN passes the
/// test; `-0.0f` does not), each component is multiplied by the
/// `0x0044233a` cascade of the squared length — `float10` products
/// narrowed on store, modeled as `f64` products cast to `f32`; finally
/// `return 0.0f < v[2]*u28 + v[1]*u18 + v[0]*u08` (strict: NaN and
/// exactly-zero dots both reject).
pub fn reject_behind_filter_accept(
    data: &RejectBehindFilterData,
    object: i32,
    fill_00661988: impl FnOnce(i32) -> [f32; 3],
) -> bool {
    let (mut u08, mut u18, mut u28) = (data.float_08, data.float_18, data.float_28);
    let squared = u28 * u28 + u18 * u18 + u08 * u08;
    if squared != 0.0f32 {
        let factor = f64::from(float_cascade_0044233a(squared.to_bits() as i32));
        // Decompiler order: local_14 (+0x08) first, then local_10
        // (+0x18), then local_c with the factor as the left operand
        // (commutative at f80 width; kept for shape).
        u08 = (f64::from(u08) * factor) as f32;
        u18 = (f64::from(u18) * factor) as f32;
        u28 = (factor * f64::from(u28)) as f32;
    }
    let v = fill_00661988(object);
    let dot = v[2] * u28 + v[1] * u18 + v[0] * u08;
    0.0f32 < dot
}

// ---------------------------------------------------------------------
// 0x00663fa7 — AIUpdateInterface::chooseGoodLocomotorFromCurrentSet
// ---------------------------------------------------------------------

/// The `AIUpdateInterface` members this row reads/writes:
/// `m_curLocomotor` @ `+0x1f0` and the owning `Object` @ `+0x08`. The
/// embedded locomotor set block at `+0x1cc` and the owner field at
/// `+0x38` are passed to the lookups as **this-relative addresses**;
/// the mirror never dereferences them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LocomotorChoiceState {
    pub object_at_08: i32,
    pub cur_locomotor_at_1f0: i32,
}

/// Foreign interactions of the locomotor chooser.
pub trait LocomotorChoiceHost {
    /// `FUN_0068b511(this + 0x1cc, owner + 0x38)` — the visible args
    /// in this row are the set-block and owner-field addresses (the
    /// exit row's reference says the function is a layer getter; the
    /// extra args here are Ghidra's stale-register reads — a host may
    /// ignore them).
    fn probe_0068b511(&mut self) -> i32;
    /// `FUN_006e74c6(prev, set +0x1cc, owner field +0x38)` — terrain
    /// locomotor lookup (`0` = none).
    fn find_locomotor_006e74c6(&mut self, prev: i32) -> i32;
    /// Template byte `+0x11f` bit 0x80 read through `owner + 4`
    /// (`ThingTemplate`).
    fn template_flag_11f_bit80(&mut self) -> bool;
    /// Guarded `(**(code**)(**(int**)(owner + 600) + 0x184))()`:
    /// `None` when `owner + 0x258` is null (virtual skipped), else the
    /// virtual-slot 0x184 result.
    fn owner_258_virtual_184(&mut self) -> Option<i32>;
    /// `FUN_005e48e8(mask)` — create a locomotor from the mask
    /// (`0x80`, `8`, or `1`).
    fn create_locomotor_005e48e8(&mut self, mask: u32) -> i32;
    /// `FUN_00713eaf()` — the singleton lookup (`0` = null); the body
    /// calls it up to twice.
    fn lookup_00713eaf(&mut self) -> i32;
    /// `*(undefined1 *)(handle + 0xc) = 1` — the byte store through
    /// the second lookup result; the body does **not** recheck null
    /// (the game's UB — a null handle here crashes).
    fn store_byte_1_at_0c(&mut self, handle: i32);
    /// The locomotor's dword at `+0x44` (its behavior-bit flags).
    fn locomotor_dword_44(&mut self, locomotor: i32) -> &mut u32;
}

/// `0x00663fa7` — pick the locomotor.
///
/// Body: the terrain lookup chain
/// (`0068b511` → `006e74c6`) supplies the candidate; if it is null the
/// *current* locomotor (if any) is reused as-is (no creation, no
/// template read). Only when both are null does the template path run:
/// template byte `+0x11f` bit 0x80 set → mask `0x80`; else if
/// `owner + 0x258` is non-null its virtual at `+0x184` answers
/// nonzero → mask `8`; else mask `1` — each path creates via
/// `005e48e8`. The candidate is stored at `+0x1f0`; when the stored
/// value **differs from the previous**, the pair of `00713eaf` lookups
/// runs (second lookup stored to `+0xc` unchecked, quirk pinned) and
/// the three read-modify-writes clear bits 3, 4 and 6 of
/// `locomotor + 0x44` (net `& !0x58`; the decompiler's three re-loads
/// of `+0x1f0` are the same dword).
pub fn ai_update_choose_good_locomotor_from_current_set(
    st: &mut LocomotorChoiceState,
    host: &mut impl LocomotorChoiceHost,
) {
    let old = st.cur_locomotor_at_1f0;
    let probe = host.probe_0068b511();
    let found = host.find_locomotor_006e74c6(probe);
    let new_loco = if found != 0 {
        found
    } else if old != 0 {
        old
    } else {
        let mask = if host.template_flag_11f_bit80() {
            0x80
        } else if let Some(v) = host.owner_258_virtual_184() {
            if v != 0 {
                8
            } else {
                1
            }
        } else {
            1
        };
        host.create_locomotor_005e48e8(mask)
    };
    st.cur_locomotor_at_1f0 = new_loco;
    if old != new_loco {
        if host.lookup_00713eaf() != 0 {
            let second = host.lookup_00713eaf();
            host.store_byte_1_at_0c(second);
        }
        let flags = host.locomotor_dword_44(new_loco);
        *flags &= 0xfffffff7;
        *flags &= 0xffffffef;
        *flags &= 0xffffffbf;
    }
}

// ---------------------------------------------------------------------
// 0x008c0c52 — SymbioticStructuresBody pinned refresh (rva004C0C52)
// ---------------------------------------------------------------------

/// The refreshed body's state: the host object ID at `+0x104` and the
/// owning `Object` at `+0x08`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SymbioticRefreshState {
    pub host_id_at_104: i32,
    pub object_at_08: i32,
}

/// Foreign interactions of the refresh row.
pub trait SymbioticRefreshHost {
    /// `FUN_00449dc5` — `TheGameLogic::findObjectByID` (`0` = null).
    fn find_object_00449dc5(&mut self, id: i32) -> i32;
    /// `FUN_0068afa9` — controlling player of the owning Object
    /// (`0` = null).
    fn owner_0068afa9(&mut self) -> i32;
    /// `FUN_009508e2` — drawable fetch; the two call sites differ only
    /// in the hidden `this` (host object vs owning Object).
    fn drawable_009508e2_of_host(&mut self) -> i32;
    fn drawable_009508e2_of_object(&mut self) -> i32;
    /// The pinned Drawable member (`rva00275DCE`):
    /// `FUN_00675dce(mode, 0.2f, 0.7f, 2.0f)` on a drawable.
    fn drawable_refresh_00675dce(&mut self, drawable: i32, mode: i32, a: f32, b: f32, c: f32);
}

/// The three float globals the refresh row passes verbatim:
/// `DAT_00bc4dd8 = 0.2f`, `DAT_00bc8180 = 0.7f`, `DAT_00bc28f4 = 2.0f`.
pub const SYMBIOTIC_REFRESH_A: f32 = 0.2;
pub const SYMBIOTIC_REFRESH_B: f32 = 0.7;
pub const SYMBIOTIC_REFRESH_C: f32 = 2.0;

/// `0x008c0c52` — the pinned symbiotic refresh.
///
/// Body order: the host lookup runs first (even before the owner
/// check); both must be non-null. `local_player` is
/// `*(ThePlayerList + 0x10)` (`DAT_00dfeee8 + 0x10`); with it and the
/// owner both non-null, the mode-5 drawable is the **host's** when
/// `local_player == owner`, else the Object's (the decompile shows both
/// arms calling `009508e2` — hidden `this` differs, reference fact),
/// and the mode-0 drawable is the other object's. Both drawables are
/// fetched before either refresh call; each refresh only runs when its
/// drawable is non-null.
pub fn symbiotic_structures_body_refresh(
    st: &SymbioticRefreshState,
    local_player: i32,
    host: &mut impl SymbioticRefreshHost,
) {
    let host_object = host.find_object_00449dc5(st.host_id_at_104);
    if host_object != 0 && st.object_at_08 != 0 {
        let owner = host.owner_0068afa9();
        if local_player != 0 && owner != 0 {
            let (mode5_drawable, mode0_drawable) = if local_player == owner {
                (
                    host.drawable_009508e2_of_host(),
                    host.drawable_009508e2_of_object(),
                )
            } else {
                (
                    host.drawable_009508e2_of_object(),
                    host.drawable_009508e2_of_host(),
                )
            };
            if mode5_drawable != 0 {
                host.drawable_refresh_00675dce(
                    mode5_drawable,
                    5,
                    SYMBIOTIC_REFRESH_A,
                    SYMBIOTIC_REFRESH_B,
                    SYMBIOTIC_REFRESH_C,
                );
            }
            if mode0_drawable != 0 {
                host.drawable_refresh_00675dce(
                    mode0_drawable,
                    0,
                    SYMBIOTIC_REFRESH_A,
                    SYMBIOTIC_REFRESH_B,
                    SYMBIOTIC_REFRESH_C,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------
// 0x008bed62 — ActiveBody::doDamageFX
// ---------------------------------------------------------------------

/// The `DamageInfo` dwords this row reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DamageInfoView {
    pub dword_08: i32,
    pub dword_14: i32,
    pub dword_2c: i32,
    pub dword_70: i32,
}

/// One 12-byte record of the vector at `+0xe0/+0xe4` (the body's
/// `piVar5 + 3` stride).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BodyFxRecord {
    pub dword_00: i32,
    pub dword_04: i32,
    pub dword_08: i32,
}

/// The `ActiveBody` state the row touches.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActiveBodyDamageFxState {
    /// Owning `Object` @ +0x08.
    pub object_at_08: i32,
    /// Next-FX time, u32 @ +0x38.
    pub next_fx_time_38: u32,
    /// Last FX damage type, i32 @ +0x3c.
    pub last_fx_type_3c: i32,
    /// Guard dword @ +0xfc (zero = the whole cooldown block is inert).
    pub dword_0fc: i32,
    /// The 12-byte record vector between `+0xe0`/`+0xe4`.
    pub records: Vec<BodyFxRecord>,
}

/// Foreign interactions of the damage-FX row.
pub trait ActiveBodyDamageFxHost {
    /// `FUN_00449dc5` — `TheGameLogic::findObjectByID(info.dword_08)`.
    fn find_object_00449dc5(&mut self, id: i32) -> i32;
    /// `FUN_007608df(kind, info.dword_70, source, object)` — the
    /// should-play gate (char → bool).
    fn can_play_fx_007608df(&mut self, kind: i32, field_70: i32, source: i32, object: i32) -> bool;
    /// `FUN_007605d5(kind, source)` — the cooldown span.
    fn fx_span_007605d5(&mut self, kind: i32, source: i32) -> i32;
    /// `FUN_005f08d3(object, 0, 0)`.
    fn call_005f08d3(&mut self, object: i32, zero_a: i32, zero_b: i32);
}

/// `0x008bed62` — damage FX gate and record scan.
///
/// Body: `dword_0fc != 0` enables the cooldown block; a repeat of the
/// same damage type (`dword_14 == +0x3c`) **while the time**
/// (`DAT_00dfe78c + 0x40`, passed in as `time_dfe78c_40`, unsigned) is
/// below the stored `+0x38` triggers the `return` that skips the
/// record scan too — pinned by tests. Otherwise resolve the source
/// (`+8`), ask the play gate; on yes, record the type at `+0x3c` and
/// set `+0x38 = span + time` (u32 wrapping add — the game's
/// `iVar4 + uVar1`). The scan then walks every record
/// (`dword_04 == info.dword_2c`, `dword_08 == 0`, `dword_00 != 0`) and
/// calls `005f08d3` once per match (no break).
pub fn active_body_do_damage_fx(
    st: &mut ActiveBodyDamageFxState,
    info: &DamageInfoView,
    time_dfe78c_40: u32,
    host: &mut impl ActiveBodyDamageFxHost,
) {
    let kind = info.dword_14;
    if st.dword_0fc != 0 {
        if kind == st.last_fx_type_3c && time_dfe78c_40 < st.next_fx_time_38 {
            return;
        }
        let source = host.find_object_00449dc5(info.dword_08);
        if host.can_play_fx_007608df(kind, info.dword_70, source, st.object_at_08) {
            st.last_fx_type_3c = kind;
            let span = host.fx_span_007605d5(kind, source);
            st.next_fx_time_38 = (span as u32).wrapping_add(time_dfe78c_40);
        }
    }
    for rec in &st.records {
        if rec.dword_04 == info.dword_2c && rec.dword_08 == 0 && rec.dword_00 != 0 {
            host.call_005f08d3(st.object_at_08, 0, 0);
        }
    }
}

// ---------------------------------------------------------------------
// 0x0066bb5a / 0x0066bc16 — AIUpdateInterface::bfmePrivateCommand24/25
// ---------------------------------------------------------------------

/// The `AIUpdateInterface` stores these commands write.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BfmeCommandState {
    /// Command source type store @ +0x48.
    pub field_048: i32,
    /// The `M` parameter store @ +0x1a0.
    pub dword_1a0: u32,
}

/// Foreign interactions of the private-command pair.
pub trait BfmeCommandHost {
    /// `FUN_006907a1()` — the command gate (char → bool).
    fn gate_006907a1(&mut self) -> bool;
    /// `(**(code **)(**(int **)(this + 0x30) + 0x14))()` — vtable slot
    /// 5 on the interface at `+0x30`, before the list-size check.
    fn vtable_0030_slot14(&mut self);
    /// `FUN_008d745c(&record, DAT_00bbb8e0)` — place the record; the
    /// global constant `DAT_00bbb8e0` (dword value) rides along.
    fn place_008d745c(&mut self, record: &mut [u32; 3], global_bbb8e0: u32);
    /// `FUN_0066b403(&record)` — snap the record.
    fn snap_0066b403(&mut self, record: &mut [u32; 3]);
    /// `FUN_00751759(param_2)` — the command list consumer (gets the
    /// vector address; the element count models it here — the record
    /// copy is a local scratch, the list itself is unmodified).
    fn consume_00751759(&mut self, count: usize);
    /// `FUN_00668d88(param_3)` — the `Object` consumer.
    fn target_00668d88(&mut self, object: i32);
    /// `(**(code **)(**(int **)(this + 0x30) + 0x20))(code)` — vtable
    /// slot 8 with the command code (0x36 / 0x3d).
    fn vtable_0030_slot20(&mut self, code: i32);
}

/// Shared body of `0x0066bb5a`/`0x0066bc16` (the pair differs only in
/// the `vtable_0030_slot20` code constant).
///
/// Body order: the gate runs first; the slot-14 vtable call happens
/// **before** the list-size check. The count is the body's
/// `(end - begin) / 0xc` pointer-difference division (exactly the
/// element count for a well-formed vector; modeled by `positions.len()`);
/// an empty list skips everything after the slot-14 call. Otherwise
/// the **last** record is copied, placed (`008d745c` +
/// `DAT_00bbb8e0`), and — only when the source type is 0 or 1 —
/// snapped; then the list consumer, the `+0x48` store (the raw source
/// type), the object consumer, the `+0x1a0` store (the raw `M`), and
/// the slot-20 vtable call. The scratch copy's mutations never reach
/// the caller's list (the body discards it).
fn bfme_private_command_place_last(
    st: &mut BfmeCommandState,
    positions: &[[u32; 3]],
    object: i32,
    param_4: u32,
    source_type: i32,
    global_bbb8e0: u32,
    code: i32,
    host: &mut impl BfmeCommandHost,
) {
    if !host.gate_006907a1() {
        return;
    }
    host.vtable_0030_slot14();
    let count = positions.len();
    if count != 0 {
        let mut last = positions[count - 1];
        host.place_008d745c(&mut last, global_bbb8e0);
        if source_type == 0 || source_type == 1 {
            host.snap_0066b403(&mut last);
        }
        host.consume_00751759(count);
        st.field_048 = source_type;
        host.target_00668d88(object);
        st.dword_1a0 = param_4;
        host.vtable_0030_slot20(code);
    }
}

/// `0x0066bb5a` — BFME private command 24: the shared place-last body
/// with vtable code `0x36`.
pub fn ai_update_bfme_private_command24(
    st: &mut BfmeCommandState,
    positions: &[[u32; 3]],
    object: i32,
    param_4: u32,
    source_type: i32,
    global_bbb8e0: u32,
    host: &mut impl BfmeCommandHost,
) {
    bfme_private_command_place_last(st, positions, object, param_4, source_type, global_bbb8e0, 0x36, host);
}

/// `0x0066bc16` — BFME private command 25: the shared place-last body
/// with vtable code `0x3d` (the byte-identical twin of command 24).
pub fn ai_update_bfme_private_command25(
    st: &mut BfmeCommandState,
    positions: &[[u32; 3]],
    object: i32,
    param_4: u32,
    source_type: i32,
    global_bbb8e0: u32,
    host: &mut impl BfmeCommandHost,
) {
    bfme_private_command_place_last(st, positions, object, param_4, source_type, global_bbb8e0, 0x3d, host);
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- row 1: 0x00889d09 — destroyPhantomStructure ----

    struct PhantomMock {
        log: Vec<String>,
        found: i32,
        owners: Vec<i32>,
        amounts: Vec<i32>,
        obj4: i32,
    }

    impl DozerPhantomHost for PhantomMock {
        fn find_object_00449dc5(&mut self, id: i32) -> i32 {
            self.log.push(format!("find {id}"));
            self.found
        }
        fn owner_0068afa9(&mut self) -> i32 {
            self.log.push("owner".into());
            self.owners.remove(0)
        }
        fn amount_00a29228(&mut self) -> i32 {
            self.log.push("amount".into());
            self.amounts.remove(0)
        }
        fn call_007b0d7c(&mut self, a: i32, b: i32, one: i32) {
            self.log.push(format!("7b0d7c {a} {b:#x} {one}"));
        }
        fn call_0079bad2(&mut self, f4: i32, neg: i32) {
            self.log.push(format!("79bad2 {f4} {neg}"));
        }
        fn object_dword_04(&mut self, object: i32) -> i32 {
            self.log.push(format!("obj4 {object}"));
            self.obj4
        }
        fn call_0079cbce(&mut self, o: i32, m: i32) {
            self.log.push(format!("79cbce {o} {m}"));
        }
        fn call_00642c09(&mut self, o: i32) {
            self.log.push(format!("642c09 {o}"));
        }
        fn call_0069f93a(&mut self, id: i32) {
            self.log.push(format!("69f93a {id}"));
        }
        fn call_006acecc(&mut self, id: i32) {
            self.log.push(format!("6acecc {id}"));
        }
    }

    #[test]
    fn phantom_zero_id_is_inert() {
        let mut st = DozerPhantomState { phantom_id_at_0c0: 0, flag_28: 0, dword_m3dc: 9 };
        let mut h = PhantomMock { log: vec![], found: 1, owners: vec![1], amounts: vec![1], obj4: 0 };
        dozer_ai_update_destroy_phantom_structure(&mut st, &mut h);
        assert!(h.log.is_empty(), "no calls and no clear for a zero ID");
        assert_eq!(st.phantom_id_at_0c0, 0);
    }

    #[test]
    fn phantom_null_object_only_clears() {
        let mut st = DozerPhantomState { phantom_id_at_0c0: 42, flag_28: 0, dword_m3dc: 9 };
        let mut h = PhantomMock { log: vec![], found: 0, owners: vec![1], amounts: vec![1], obj4: 0 };
        dozer_ai_update_destroy_phantom_structure(&mut st, &mut h);
        assert_eq!(h.log, vec!["find 42"]);
        assert_eq!(st.phantom_id_at_0c0, 0, "ID cleared even when the object is null");
    }

    #[test]
    fn phantom_full_chain_order_and_negation() {
        let mut st = DozerPhantomState { phantom_id_at_0c0: 7, flag_28: 0, dword_m3dc: 1 };
        let mut h = PhantomMock {
            log: vec![],
            found: 0x1234,
            owners: vec![0x2000, 0x9],
            amounts: vec![5],
            obj4: 77,
        };
        dozer_ai_update_destroy_phantom_structure(&mut st, &mut h);
        assert_eq!(
            h.log,
            vec![
                "find 7",
                "owner",
                "amount",
                "7b0d7c 5 0x23bc 1",
                "obj4 4660",
                "79bad2 77 -5",
                "79cbce 4660 -1",
                "642c09 4660",
                "69f93a 7",
                "owner",
                "6acecc 7",
            ]
        );
        assert_eq!(st.phantom_id_at_0c0, 0);
    }

    #[test]
    fn phantom_flag_set_skips_block_but_reads_owner() {
        let mut st = DozerPhantomState { phantom_id_at_0c0: 7, flag_28: 1, dword_m3dc: 0 };
        let mut h = PhantomMock { log: vec![], found: 5, owners: vec![0x2000], amounts: vec![5], obj4: 0 };
        dozer_ai_update_destroy_phantom_structure(&mut st, &mut h);
        // owner is read before the flag test; the refund block and the
        // m3dc block are skipped entirely.
        assert_eq!(h.log, vec!["find 7", "owner", "642c09 5", "69f93a 7"]);
    }

    #[test]
    fn phantom_m3dc_guard_and_null_owner() {
        let mut st = DozerPhantomState { phantom_id_at_0c0: 3, flag_28: 1, dword_m3dc: 4 };
        let mut h = PhantomMock { log: vec![], found: 8, owners: vec![0x2000, 0], amounts: vec![], obj4: 0 };
        dozer_ai_update_destroy_phantom_structure(&mut st, &mut h);
        assert_eq!(h.log, vec!["find 3", "owner", "642c09 8", "69f93a 3", "owner"]);
        assert!(!h.log.iter().any(|l| l.starts_with("6acecc")), "null second owner blocks the call");
    }

    #[test]
    fn phantom_negation_wraps_at_i32_min() {
        let mut st = DozerPhantomState { phantom_id_at_0c0: 1, flag_28: 0, dword_m3dc: 0 };
        let mut h = PhantomMock { log: vec![], found: 2, owners: vec![0], amounts: vec![i32::MIN], obj4: 0 };
        dozer_ai_update_destroy_phantom_structure(&mut st, &mut h);
        assert!(h.log.iter().any(|l| l == "79bad2 0 -2147483648"), "wrapping neg: {:#?}", h.log);
        // owner + 0x3bc with a null owner is still the plain add.
        assert!(h.log.iter().any(|l| l == "7b0d7c -2147483648 0x3bc 1"));
    }

    // ---- row 2: 0x008a3e3e — exitObjectViaDoor ----

    struct ExitMock {
        log: Vec<String>,
        layer: i32,
    }

    impl SpawnPointExitHost for ExitMock {
        fn init_bones_008a3cea(&mut self) {
            self.log.push("init".into());
        }
        fn set_position_0070aa80(&mut self, p: [f32; 3]) {
            self.log.push(format!("pos {:?}", p));
        }
        fn set_orientation_0070ab9d(&mut self, a: f32) {
            self.log.push(format!("orient {a}"));
        }
        fn get_layer_0068b511(&mut self) -> i32 {
            self.log.push("getlayer".into());
            self.layer
        }
        fn set_layer_0068b4ce(&mut self, l: i32) {
            self.log.push(format!("setlayer {l}"));
        }
        fn add_pathfind_006e7178(&mut self) {
            self.log.push("pathfind".into());
        }
        fn set_disabled_00691c9b(&mut self, k: i32) {
            self.log.push(format!("disabled {k}"));
        }
    }

    fn exit_state(bones: u8, count: i32) -> SpawnPointExitState {
        SpawnPointExitState {
            bones_initialized: bones,
            spawn_point_count: count,
            spawn_points: [
                [1.0, 2.0, 3.0],
                [4.0, 5.0, 6.0],
                [7.0, 8.0, 9.0],
                [0.0; 3],
                [0.0; 3],
                [0.0; 3],
                [0.0; 3],
                [0.0; 3],
                [0.0; 3],
                [0.0; 3],
            ],
            spawn_angles: [10.0, 20.0, 30.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            occupier: [0; 10],
            creation_object: 0x4000,
        }
    }

    #[test]
    fn exit_bone_gate_and_full_flow() {
        let mut st = exit_state(0, 3);
        st.occupier[0] = 11; // slot 0 taken -> first free is index 1
        let mut h = ExitMock { log: vec![], layer: 2 };
        spawn_point_exit_object_via_door(&mut st, 99, &mut h);
        assert_eq!(st.occupier[1], 99, "reservation written");
        assert_eq!(st.bones_initialized, 0, "the game never sets the flag here");
        assert_eq!(
            h.log,
            vec![
                "init",
                "pos [4.0, 5.0, 6.0]",
                "orient 20",
                "getlayer",
                "setlayer 2",
                "pathfind",
                "disabled 3",
            ]
        );
    }

    #[test]
    fn exit_full_array_and_zero_count_skip() {
        let mut st = exit_state(1, 2);
        st.occupier[0] = 5;
        st.occupier[1] = 6;
        let mut h = ExitMock { log: vec![], layer: 0 };
        spawn_point_exit_object_via_door(&mut st, 99, &mut h);
        assert!(h.log.is_empty(), "idx == count: no placement");

        let mut st = exit_state(1, 0);
        let mut h = ExitMock { log: vec![], layer: 0 };
        spawn_point_exit_object_via_door(&mut st, 99, &mut h);
        assert!(h.log.is_empty(), "count 0: idx 0 == count, skip");
        assert_eq!(st.occupier[0], 0);
    }

    #[test]
    fn exit_negative_count_proceeds_with_index_zero() {
        // do-while quirk: a negative count skips the scan but still
        // fails `idx != count`, reserving slot 0.
        let mut st = exit_state(1, -3);
        st.occupier[0] = 7; // even an occupied slot 0 gets overwritten
        let mut h = ExitMock { log: vec![], layer: 1 };
        spawn_point_exit_object_via_door(&mut st, 123, &mut h);
        assert_eq!(st.occupier[0], 123);
        assert_eq!(h.log[0], "pos [1.0, 2.0, 3.0]");
    }

    #[test]
    fn exit_null_creation_object_gates_placement() {
        let mut st = exit_state(1, 3);
        st.creation_object = 0;
        let mut h = ExitMock { log: vec![], layer: 0 };
        spawn_point_exit_object_via_door(&mut st, 9, &mut h);
        assert!(h.log.is_empty());
        assert_eq!(st.occupier[0], 0);
    }

    // ---- row 3: 0x00661abb — PartitionFilterRejectBehind ----

    #[test]
    fn reject_behind_zero_vector_uses_raw_dot_strict() {
        let d = RejectBehindFilterData::default();
        // s == 0 skips normalization; dot is exactly 0 -> strict < rejects.
        assert!(!reject_behind_filter_accept(&d, 7, |_| [1.0, 2.0, 3.0]));
    }

    #[test]
    fn reject_behind_negative_zero_squared_skips_normalize() {
        // -0.0 * -0.0 + 0 = +0.0, so `squared != 0.0f` is false and the
        // cascade path is skipped. With s == 0 every component is a
        // signed zero, so the raw dot can only be +0.0/-0.0 and the
        // strict `0.0f <` comparison always rejects here.
        let d = RejectBehindFilterData { float_08: -0.0, float_18: 0.0, float_28: 0.0 };
        assert!(!reject_behind_filter_accept(&d, 0, |_| [1.0, 1.0, 1.0]));
    }

    #[test]
    fn reject_behind_nan_dot_rejects_nan_squared_normalizes() {
        let d = RejectBehindFilterData { float_08: f32::NAN, float_18: 0.0, float_28: 1.0 };
        // squared is NaN (!= 0.0 is true) -> cascade multiplies; the dot
        // is NaN and every comparison is false.
        assert!(!reject_behind_filter_accept(&d, 0, |_| [1.0, 1.0, 1.0]));
    }

    #[test]
    fn reject_behind_component_pairing_and_magnitude() {
        // No normalization when s == 0 pins the component pairing:
        // dot = v[2]*u28 + v[1]*u18 + v[0]*u08 with the (impossible for
        // s==0 except all-zero) case replaced by a cascade-path check
        // recomputed at the mirror's precision.
        let d = RejectBehindFilterData { float_08: 1.0, float_18: 2.0, float_28: 3.0 };
        let v = [1.0f32, 1.0, 1.0];
        let s: f32 = 3.0 * 3.0 + 2.0 * 2.0 + 1.0 * 1.0; // association per the body
        let f = f64::from(float_cascade_0044233a(s.to_bits() as i32));
        let u08 = (f64::from(1.0f32) * f) as f32;
        let u18 = (f64::from(2.0f32) * f) as f32;
        let u28 = (f * f64::from(3.0f32)) as f32;
        let dot = v[2] * u28 + v[1] * u18 + v[0] * u08;
        let expect = 0.0f32 < dot;
        assert_eq!(reject_behind_filter_accept(&d, 0, |_| v), expect);
        // A sign-flipped object vector flips the dot (the cascade factor
        // is shared, so the comparison must flip too — pairing check).
        assert_eq!(reject_behind_filter_accept(&d, 0, |_| [-v[0], -v[1], -v[2]]), !expect);
    }

    // ---- row 4: 0x00663fa7 — chooseGoodLocomotorFromCurrentSet ----

    struct LocoMock {
        log: Vec<String>,
        found: i32,
        template_bit: bool,
        probe258: Option<i32>,
        lookups: Vec<i32>,
        flags44: u32,
    }

    impl LocomotorChoiceHost for LocoMock {
        fn probe_0068b511(&mut self) -> i32 {
            self.log.push("probe".into());
            0xAA
        }
        fn find_locomotor_006e74c6(&mut self, prev: i32) -> i32 {
            self.log.push(format!("find {prev:#x}"));
            self.found
        }
        fn template_flag_11f_bit80(&mut self) -> bool {
            self.log.push("tmpl".into());
            self.template_bit
        }
        fn owner_258_virtual_184(&mut self) -> Option<i32> {
            self.log.push("vf258".into());
            self.probe258
        }
        fn create_locomotor_005e48e8(&mut self, mask: u32) -> i32 {
            self.log.push(format!("create {mask:#x}"));
            (0x9000 | mask) as i32
        }
        fn lookup_00713eaf(&mut self) -> i32 {
            self.log.push("lookup".into());
            self.lookups.remove(0)
        }
        fn store_byte_1_at_0c(&mut self, h: i32) {
            self.log.push(format!("store0c {h}"));
        }
        fn locomotor_dword_44(&mut self, loco: i32) -> &mut u32 {
            self.log.push(format!("flags44 {loco:#x}"));
            &mut self.flags44
        }
    }

    #[test]
    fn locomotor_found_is_used_verbatim_no_template() {
        let mut st = LocomotorChoiceState { object_at_08: 1, cur_locomotor_at_1f0: 0 };
        let mut h = LocoMock {
            log: vec![],
            found: 0x55,
            template_bit: true,
            probe258: Some(1),
            lookups: vec![3, 4],
            flags44: 0xffff_ffff,
        };
        ai_update_choose_good_locomotor_from_current_set(&mut st, &mut h);
        assert_eq!(st.cur_locomotor_at_1f0, 0x55);
        assert_eq!(h.log[0..2], ["probe", "find 0xaa"]);
        assert_eq!(h.log[2..], ["lookup", "lookup", "store0c 4", "flags44 0x55"]);
        assert_eq!(h.flags44, 0xffff_ffa7, "bits 3/4/6 cleared, rest kept");
    }

    #[test]
    fn locomotor_reuses_current_when_lookup_null() {
        let mut st = LocomotorChoiceState { object_at_08: 1, cur_locomotor_at_1f0: 0x77 };
        let mut h = LocoMock {
            log: vec![],
            found: 0,
            template_bit: false,
            probe258: None,
            lookups: vec![],
            flags44: 0,
        };
        ai_update_choose_good_locomotor_from_current_set(&mut st, &mut h);
        assert_eq!(st.cur_locomotor_at_1f0, 0x77);
        // old == new: no lookup pair, no flag touch.
        assert_eq!(h.log, ["probe", "find 0xaa"]);
    }

    #[test]
    fn locomotor_template_bit_creates_0x80() {
        let mut st = LocomotorChoiceState { object_at_08: 1, cur_locomotor_at_1f0: 0 };
        let mut h = LocoMock {
            log: vec![],
            found: 0,
            template_bit: true,
            probe258: Some(1),
            lookups: vec![0], // first lookup null: store skipped
            flags44: 0x58,
        };
        ai_update_choose_good_locomotor_from_current_set(&mut st, &mut h);
        assert_eq!(st.cur_locomotor_at_1f0, 0x9080);
        assert!(h.log.iter().any(|l| l == "create 0x80"));
        assert!(!h.log.iter().any(|l| l == "vf258"), "bit set skips the 0x258 virtual");
        assert!(h.log.iter().any(|l| l == "lookup"));
        assert!(!h.log.iter().any(|l| l.starts_with("store0c")), "null first lookup skips the store");
        assert_eq!(h.flags44, 0, "0x58 was exactly the cleared bits");
    }

    #[test]
    fn locomotor_258_paths_pick_masks() {
        // non-null pointer + nonzero virtual result -> mask 8
        let mut st = LocomotorChoiceState { object_at_08: 1, cur_locomotor_at_1f0: 0 };
        let mut h = LocoMock { log: vec![], found: 0, template_bit: false, probe258: Some(7), lookups: vec![0], flags44: 0 };
        ai_update_choose_good_locomotor_from_current_set(&mut st, &mut h);
        assert!(h.log.iter().any(|l| l == "create 0x8"), "{:?}", h.log);

        // non-null pointer + zero virtual result -> mask 1
        let mut st = LocomotorChoiceState { object_at_08: 1, cur_locomotor_at_1f0: 0 };
        let mut h = LocoMock { log: vec![], found: 0, template_bit: false, probe258: Some(0), lookups: vec![0], flags44: 0 };
        ai_update_choose_good_locomotor_from_current_set(&mut st, &mut h);
        assert!(h.log.iter().any(|l| l == "create 0x1"));

        // null pointer -> virtual skipped, mask 1
        let mut st = LocomotorChoiceState { object_at_08: 1, cur_locomotor_at_1f0: 0 };
        let mut h = LocoMock { log: vec![], found: 0, template_bit: false, probe258: None, lookups: vec![0], flags44: 0 };
        ai_update_choose_good_locomotor_from_current_set(&mut st, &mut h);
        // the guard reads owner+0x258 either way; only the virtual
        // behind it is skipped (both modeled in one host method)
        assert!(h.log.iter().any(|l| l == "vf258"));
        assert!(h.log.iter().any(|l| l == "create 0x1"));
    }

    #[test]
    fn locomotor_second_lookup_null_stores_anyway() {
        // Quirk: the body null-checks only the FIRST 00713eaf result;
        // the second handle (even 0) is stored through unchecked.
        let mut st = LocomotorChoiceState { object_at_08: 1, cur_locomotor_at_1f0: 0 };
        let mut h = LocoMock { log: vec![], found: 3, template_bit: false, probe258: None, lookups: vec![9, 0], flags44: 0 };
        ai_update_choose_good_locomotor_from_current_set(&mut st, &mut h);
        assert!(h.log.iter().any(|l| l == "store0c 0"), "{:?}", h.log);
        assert_eq!(h.log.iter().filter(|l| *l == "lookup").count(), 2);
    }

    // ---- row 5: 0x008c0c52 — symbiotic refresh ----

    struct RefreshMock {
        log: Vec<String>,
        host_obj: i32,
        owner: i32,
        host_draw: i32,
        obj_draw: i32,
    }

    impl SymbioticRefreshHost for RefreshMock {
        fn find_object_00449dc5(&mut self, id: i32) -> i32 {
            self.log.push(format!("find {id}"));
            self.host_obj
        }
        fn owner_0068afa9(&mut self) -> i32 {
            self.log.push("owner".into());
            self.owner
        }
        fn drawable_009508e2_of_host(&mut self) -> i32 {
            self.log.push("draw-host".into());
            self.host_draw
        }
        fn drawable_009508e2_of_object(&mut self) -> i32 {
            self.log.push("draw-obj".into());
            self.obj_draw
        }
        fn drawable_refresh_00675dce(&mut self, d: i32, mode: i32, a: f32, b: f32, c: f32) {
            self.log.push(format!("5dce {d} {mode} {a} {b} {c}"));
        }
    }

    #[test]
    fn refresh_guards_run_in_order() {
        let st = SymbioticRefreshState { host_id_at_104: 21, object_at_08: 0 };
        let mut h = RefreshMock { log: vec![], host_obj: 5, owner: 4, host_draw: 1, obj_draw: 2 };
        symbiotic_structures_body_refresh(&st, 4, &mut h);
        assert_eq!(h.log, vec!["find 21"], "null owner object: nothing after the lookup");

        let st = SymbioticRefreshState { host_id_at_104: 21, object_at_08: 9 };
        let mut h = RefreshMock { log: vec![], host_obj: 0, owner: 4, host_draw: 1, obj_draw: 2 };
        symbiotic_structures_body_refresh(&st, 4, &mut h);
        assert_eq!(h.log, vec!["find 21"], "null host object: stop");

        let st = SymbioticRefreshState { host_id_at_104: 21, object_at_08: 9 };
        let mut h = RefreshMock { log: vec![], host_obj: 5, owner: 0, host_draw: 1, obj_draw: 2 };
        symbiotic_structures_body_refresh(&st, 4, &mut h);
        assert_eq!(h.log, vec!["find 21", "owner"], "null owner: stop");

        let st = SymbioticRefreshState { host_id_at_104: 21, object_at_08: 9 };
        let mut h = RefreshMock { log: vec![], host_obj: 5, owner: 4, host_draw: 1, obj_draw: 2 };
        symbiotic_structures_body_refresh(&st, 0, &mut h);
        assert_eq!(h.log, vec!["find 21", "owner"], "null local player: stop");
    }

    #[test]
    fn refresh_local_player_picks_mode5_target() {
        let st = SymbioticRefreshState { host_id_at_104: 21, object_at_08: 9 };
        let mut h = RefreshMock { log: vec![], host_obj: 5, owner: 4, host_draw: 0x11, obj_draw: 0x22 };
        symbiotic_structures_body_refresh(&st, 4, &mut h);
        assert_eq!(
            h.log,
            vec![
                "find 21",
                "owner",
                "draw-host",
                "draw-obj",
                "5dce 17 5 0.2 0.7 2",
                "5dce 34 0 0.2 0.7 2",
            ],
            "local player == owner: host drawable gets mode 5"
        );
    }

    #[test]
    fn refresh_non_local_player_flips_and_null_drawables_skip() {
        let st = SymbioticRefreshState { host_id_at_104: 21, object_at_08: 9 };
        let mut h = RefreshMock { log: vec![], host_obj: 5, owner: 8, host_draw: 0x11, obj_draw: 0 };
        symbiotic_structures_body_refresh(&st, 4, &mut h);
        assert_eq!(
            h.log,
            vec![
                "find 21",
                "owner",
                "draw-obj",
                "draw-host",
                "5dce 17 0 0.2 0.7 2",
            ],
            "not local: the Object drawable would get mode 5 (null: skipped); the host's gets mode 0"
        );
    }

    // ---- row 6: 0x008bed62 — doDamageFX ----

    struct FxMock {
        log: Vec<String>,
        src: i32,
        can: bool,
        span: i32,
    }

    impl ActiveBodyDamageFxHost for FxMock {
        fn find_object_00449dc5(&mut self, id: i32) -> i32 {
            self.log.push(format!("find {id}"));
            self.src
        }
        fn can_play_fx_007608df(&mut self, k: i32, f70: i32, s: i32, o: i32) -> bool {
            self.log.push(format!("can {k} {f70} {s} {o}"));
            self.can
        }
        fn fx_span_007605d5(&mut self, k: i32, s: i32) -> i32 {
            self.log.push(format!("span {k} {s}"));
            self.span
        }
        fn call_005f08d3(&mut self, o: i32, a: i32, b: i32) {
            self.log.push(format!("5f08d3 {o} {a} {b}"));
        }
    }

    fn fx_state(gate: i32, last: i32, next: u32, recs: Vec<BodyFxRecord>) -> ActiveBodyDamageFxState {
        ActiveBodyDamageFxState {
            object_at_08: 0x600,
            next_fx_time_38: next,
            last_fx_type_3c: last,
            dword_0fc: gate,
            records: recs,
        }
    }

    #[test]
    fn damage_fx_cooldown_early_return_skips_records() {
        let mut st = fx_state(1, 4, 100, vec![BodyFxRecord { dword_00: 1, dword_04: 55, dword_08: 0 }]);
        let info = DamageInfoView { dword_08: 0, dword_14: 4, dword_2c: 55, dword_70: 0 };
        let mut h = FxMock { log: vec![], src: 1, can: true, span: 9 };
        active_body_do_damage_fx(&mut st, &info, 99, &mut h);
        assert!(h.log.is_empty(), "same type + time < next: return before the scan");
        assert_eq!(st.next_fx_time_38, 100, "state untouched on the early return");
    }

    #[test]
    fn damage_fx_time_at_or_past_next_proceeds_unsigned() {
        // u32 comparison: time == next is NOT below -> proceed.
        let mut st = fx_state(1, 4, 100, vec![]);
        let info = DamageInfoView { dword_08: 30, dword_14: 4, dword_2c: 0, dword_70: 7 };
        let mut h = FxMock { log: vec![], src: 0x44, can: true, span: -1 };
        active_body_do_damage_fx(&mut st, &info, 100, &mut h);
        assert_eq!(st.last_fx_type_3c, 4);
        assert_eq!(st.next_fx_time_38, (-1i32 as u32).wrapping_add(100), "u32 wrapping add");
        assert_eq!(h.log, ["find 30", "can 4 7 68 1536", "span 4 68"]);
    }

    #[test]
    fn damage_fx_gate_zero_runs_scan_only() {
        let recs = vec![
            BodyFxRecord { dword_00: 0, dword_04: 55, dword_08: 0 }, // dword_00 must be nonzero
            BodyFxRecord { dword_00: 1, dword_04: 55, dword_08: 1 }, // dword_08 must be zero
            BodyFxRecord { dword_00: 1, dword_04: 54, dword_08: 0 }, // id mismatch
            BodyFxRecord { dword_00: 2, dword_04: 55, dword_08: 0 }, // match
            BodyFxRecord { dword_00: 3, dword_04: 55, dword_08: 0 }, // match again (no break)
        ];
        let mut st = fx_state(0, 4, 0, recs);
        let info = DamageInfoView { dword_08: 9, dword_14: 4, dword_2c: 55, dword_70: 0 };
        let mut h = FxMock { log: vec![], src: 1, can: true, span: 1 };
        active_body_do_damage_fx(&mut st, &info, 5, &mut h);
        assert_eq!(h.log, vec!["5f08d3 1536 0 0", "5f08d3 1536 0 0"]);
    }

    #[test]
    fn damage_fx_gate_reject_leaves_state() {
        let mut st = fx_state(1, 1, 0, vec![]);
        let info = DamageInfoView { dword_08: 9, dword_14: 2, dword_2c: 0, dword_70: 0 };
        let mut h = FxMock { log: vec![], src: 0, can: false, span: 5 };
        active_body_do_damage_fx(&mut st, &info, 7, &mut h);
        assert_eq!(st.last_fx_type_3c, 1);
        assert_eq!(st.next_fx_time_38, 0);
        assert!(!h.log.iter().any(|l| l.starts_with("span")));
    }

    // ---- rows 7+8: 0x0066bb5a / 0x0066bc16 — private commands ----

    struct CmdMock {
        log: Vec<String>,
        gate: bool,
    }

    impl BfmeCommandHost for CmdMock {
        fn gate_006907a1(&mut self) -> bool {
            self.log.push("gate".into());
            self.gate
        }
        fn vtable_0030_slot14(&mut self) {
            self.log.push("vf14".into());
        }
        fn place_008d745c(&mut self, r: &mut [u32; 3], g: u32) {
            self.log.push(format!("place {:?} {g:#x}", *r));
            r[0] = r[0].wrapping_add(1); // model the in-place mutation
        }
        fn snap_0066b403(&mut self, r: &mut [u32; 3]) {
            self.log.push(format!("snap {:?}", *r));
            r[2] = 0;
        }
        fn consume_00751759(&mut self, n: usize) {
            self.log.push(format!("consume {n}"));
        }
        fn target_00668d88(&mut self, o: i32) {
            self.log.push(format!("target {o}"));
        }
        fn vtable_0030_slot20(&mut self, code: i32) {
            self.log.push(format!("vf20 {code:#x}"));
        }
    }

    fn positions() -> Vec<[u32; 3]> {
        vec![[1, 1, 1], [2, 2, 2], [30, 31, 32]]
    }

    #[test]
    fn private_command_gate_false_is_inert() {
        let mut st = BfmeCommandState::default();
        let mut h = CmdMock { log: vec![], gate: false };
        ai_update_bfme_private_command24(&mut st, &positions(), 5, 6, 0, 0, &mut h);
        assert_eq!(h.log, ["gate"]);
        assert_eq!(st, BfmeCommandState::default());
    }

    #[test]
    fn private_command_empty_list_after_slot14_stops() {
        let mut st = BfmeCommandState::default();
        let mut h = CmdMock { log: vec![], gate: true };
        ai_update_bfme_private_command24(&mut st, &[], 5, 6, 0, 0, &mut h);
        assert_eq!(h.log, ["gate", "vf14"]);
        assert_eq!(st, BfmeCommandState::default());
    }

    #[test]
    fn private_command24_full_sequence_uses_last_record_and_stores() {
        let mut st = BfmeCommandState::default();
        let mut h = CmdMock { log: vec![], gate: true };
        ai_update_bfme_private_command24(&mut st, &positions(), 0x777, 0xdead_beef, 0, 0xbbb8_e0ff, &mut h);
        assert_eq!(st.field_048, 0);
        assert_eq!(st.dword_1a0, 0xdead_beef);
        assert_eq!(
            h.log,
            vec![
                "gate",
                "vf14",
                "place [30, 31, 32] 0xbbb8e0ff",
                "snap [31, 31, 32]",
                "consume 3",
                "target 1911",
                "vf20 0x36",
            ]
        );
    }

    #[test]
    fn private_command25_source_type_outside_zero_one_skips_snap() {
        let mut st = BfmeCommandState::default();
        let mut h = CmdMock { log: vec![], gate: true };
        ai_update_bfme_private_command25(&mut st, &positions(), 1, 9, 2, 0, &mut h);
        assert!(!h.log.iter().any(|l| l.starts_with("snap")));
        assert_eq!(st.field_048, 2, "the raw source type is stored anyway");
        assert!(h.log.iter().any(|l| l == "vf20 0x3d"), "row 8 uses code 0x3d");
    }

    #[test]
    fn private_commands_differ_only_in_the_vtable_code() {
        let mut a = CmdMock { log: vec![], gate: true };
        let mut b = CmdMock { log: vec![], gate: true };
        let mut st1 = BfmeCommandState::default();
        let mut st2 = BfmeCommandState::default();
        ai_update_bfme_private_command24(&mut st1, &positions(), 3, 4, 1, 7, &mut a);
        ai_update_bfme_private_command25(&mut st2, &positions(), 3, 4, 1, 7, &mut b);
        assert_eq!(st1, st2);
        assert_eq!(a.log.len(), b.log.len());
        assert_eq!(&a.log[..a.log.len() - 1], &b.log[..b.log.len() - 1]);
        assert_eq!(a.log[a.log.len() - 1], "vf20 0x36");
        assert_eq!(b.log[b.log.len() - 1], "vf20 0x3d");
    }
}
