//! Wave 48 (`Behavior` module, band `0x00853…0x008e9`): eight object-behavior
//! methods. Cross-module callees (`FUN_…`), vtable slots, and raw field loads
//! through handle pointers are modelled as host traits / slices (the
//! `dxstate.rs` / `systems1.rs` hook idiom); every address, offset, and quirk
//! below is quoted from the inlined decompiled bodies. Reference-tree facts
//! are limited to naming/offsets, marked as such.
//!
//! | addr | size | symbol | role |
//! |---|---|---|---|
//! | `0x008e90b1` | 200 | `?rva004E90B1@GateOpenBehaviorList@@QAEPAXPBUCoord3D@@PBVPlayer@@@Z` | closest gate among vector entries whose relationship to the player is code `2` (ALLY) |
//! | `0x0085f7b5` | 199 | `?onDie@SpawnBehavior@@UAEXPBVDamageInfo@@@Z` | die-gate: hit each live spawn once through its `+0x244` module chain, then kill remaining live spawns (die code `8`) when the owner byte `+0x18` is set |
//! | `0x008e9179` | 197 | `?rva004E9179@GateOpenBehaviorList@@QAEPAXPBUCoord3D@@PBVPlayer@@@Z` | same closest-scan filtered by exact controlling player instead of relationship |
//! | `0x0085a57c` | 195 | `?rva0045A57C@AutoAbilityBehavior@@QAE_NXZ` | auto-ability target predicate: gated conjunction over owner flag bits `+0x11c/+0x118/+0x120`, context `+0x20`, and `+0x258` sub-pointer |
//! | `0x008534f5` | 194 | `?onObjectCreated@GettingBuiltBehavior@@UAEXXZ` | pick config sub-record by `+0x1c`/worker-template byte, look it up, notify interface slot `+0x10` on the owner-template flag condition, option `0x57` extra notify |
//! | `0x008847a9` | 192 | `?update@SlaveWatcherBehavior@@UAE?AW4UpdateSleepTime@@XZ` | master gone/dead → resolve+clear two pointers and sleep `0x3fffffff`; else conditional 0x20-dword block copy into `this+0x14` |
//! | `0x00882a69` | 187 | `?onDamage@FireWeaponWhenDamagedBehavior@@UAEXPAVDamageInfo@@@Z` | kind-bit ∩ template `+0x11c`, threshold `+0x120` vs damage `+0x70`, weapon-set status selects a slot, `FUN_006ccfc7` miss fires `FUN_006ce76f(owner, owner+0x38)` |
//! | `0x00853f31` | 182 | `?rva00453F31@GettingBuiltBehavior@@UAEXH@Z` | status bit set → own virtual slot `+0x10`, then upward and downward `Object_ID` walk notifying consecutive same-gate objects |
//!
//! Faithful-quirk notes (all pinned by tests):
//! - Gate scans: the sentinel test is `best == 0`, not a separate flag — an
//!   entry whose value *is* 0 keeps `best` at 0, so a later qualifying entry
//!   replaces it even when farther away. First candidate always latches
//!   (distance `NaN` included); afterwards only a *strictly smaller* squared
//!   distance wins, so `NaN` never replaces a real best.
//! - The squared distance sums `z² + y² + x²` in exactly that order (the
//!   `fVar` numbering in the bodies), which changes rounding for mixed
//!   magnitudes — the order is preserved verbatim.
//! - `0x00882a69` shift amount is `((char)info[+0x10] - 1u) & 0x1f`: the
//!   32-bit kind dword truncates to `char` first (kind `33` → bit 0, kind `0`
//!   → bit 31). The float guard is written `f < amt || f == amt`, which a NaN
//!   operand on either side fails.
//! - `0x008847a9` returns the raw sleep-enum dwords from the body:
//!   `0x3fffffff` (`UPDATE_SLEEP_FOREVER` per the public SDK enum; value is
//!   from the body) and `1`.
//! - Short-circuit order is preserved in every conjunction: each `FUN_` hook
//!   runs exactly when the game would have called it.
//! - Reference facts (Open-BFME-2, naming only): the `GateOpenBehaviorList`
//!   vector spans `m_begin @ +0x00` / `m_end @ +0x04`; entries are
//!   `GateOpenAndCloseBehavior*`; the row-1 filter is the relationship-to-
//!   player getter compared with `ALLIES` (body constant `2`), row 3 compares
//!   `getControllingPlayer()` with the parameter pointer; the position getter
//!   returns the owning object's position (`Object+0x38`).

/// Partial `DamageInfo` view: only the fields the bodies here touch.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DamageInfo {
    /// `DamageInfo` dword `@ +0x10` — damage-kind code; `0x00882a69` maps it
    /// to a template flag bit via `((char)kind - 1u) & 0x1f`.
    pub kind_10: i32,
    /// `DamageInfo` float `@ +0x70` — damage amount.
    pub amount_70: f32,
}

// ---------------------------------------------------------------------------
// 1 + 3. GateOpenBehaviorList closest-gate scans
// ---------------------------------------------------------------------------

/// Hook surface for both closest-gate scans (`0x008e90b1` / `0x008e9179`).
pub trait GateClosestHost {
    /// `FUN_008989df` — relationship of the gate entry's owner to `player`;
    /// the row-1 body keeps entries where this equals `2` (ALLIES).
    fn relationship_to_008989df(&mut self, gate: i32, player: i32) -> i32;
    /// `FUN_008989d7` — `GateOpenAndCloseBehavior::getControllingPlayer`
    /// (reference name); compared with the `player` handle by row 3.
    fn controlling_player_008989d7(&mut self, gate: i32) -> i32;
    /// `FUN_008989ee` — owning object's position (`Object+0x38`, reference
    /// fact). Queried only after the filter passed (test-pinned).
    fn gate_position_008989ee(&mut self, gate: i32) -> [f32; 3];
}

/// Squared 3-D distance in the bodies' exact accumulation order
/// (`(z-qz)² + (y-qy)² + (x-qx)²`; float rounding is order-sensitive).
fn dist_sq_z_y_x(p: [f32; 3], q: [f32; 3]) -> f32 {
    let dz = p[2] - q[2];
    let dy = p[1] - q[1];
    let dx = p[0] - q[0];
    (dz * dz) + (dy * dy) + (dx * dx)
}

/// `0x008e90b1` — `GateOpenBehaviorList::rva004E90B1(Coord3D const*, Player const*)`.
/// `gates` models the vector spanned by `this[0]`/`this[1]` (`+0x00`/`+0x04`);
/// each entry is a `GateOpenAndCloseBehavior*` kept as an opaque `i32`
/// handle. The game returns the chosen entry in the `void*` result — modelled
/// as the raw handle, `0` for "none". A qualifying entry latches while the
/// running best is still `0` (so a literal-`0` entry never sticks), then only
/// strictly closer entries replace it.
pub fn gate_open_list_find_closest_allied(
    gates: &[i32],
    pos: [f32; 3],
    player: i32,
    host: &mut dyn GateClosestHost,
) -> i32 {
    let mut best: i32 = 0;
    let mut best_dist: f32 = 0.0;
    for &gate in gates {
        if host.relationship_to_008989df(gate, player) != 2 {
            continue;
        }
        let q = host.gate_position_008989ee(gate);
        let dist = dist_sq_z_y_x(pos, q);
        if best == 0 || dist < best_dist {
            best_dist = dist;
            best = gate;
        }
    }
    best
}

/// `0x008e9179` — `GateOpenBehaviorList::rva004E9179(Coord3D const*, Player const*)`.
/// Identical scan to [`gate_open_list_find_closest_allied`] except the
/// filter: `FUN_008989d7(entry) == player` (exact controlling-player match;
/// the body compares the returned pointer against the `Player*` parameter).
pub fn gate_open_list_find_closest_controlled(
    gates: &[i32],
    pos: [f32; 3],
    player: i32,
    host: &mut dyn GateClosestHost,
) -> i32 {
    let mut best: i32 = 0;
    let mut best_dist: f32 = 0.0;
    for &gate in gates {
        if host.controlling_player_008989d7(gate) != player {
            continue;
        }
        let q = host.gate_position_008989ee(gate);
        let dist = dist_sq_z_y_x(pos, q);
        if best == 0 || dist < best_dist {
            best_dist = dist;
            best = gate;
        }
    }
    best
}

// ---------------------------------------------------------------------------
// 2. SpawnBehavior::onDie
// ---------------------------------------------------------------------------

/// Slot window of `SpawnBehavior` used by `0x0085f7b5` (behavior subobject
/// base: owner back-pointer lives *before* `this`).
pub struct SpawnBehaviorSlots {
    /// `SpawnBehavior` back-pointer `this - 0x20`: owning `Object*`.
    pub owner_m20: i32,
    /// `SpawnBehavior` dword `this - 0x1c`: argument of the die gate.
    pub die_gate_m1c: i32,
    /// `SpawnBehavior::m_spawnedList @ +0x28`: pointer to the list header
    /// node (`begin = *header`, node link at `+0x00`, payload `Object_ID` at
    /// `+0x08`). The header walk is modelled by the `spawned_ids` slice.
    pub spawned_list_28: i32,
}

/// Hook surface for [`spawn_behavior_on_die`].
pub trait SpawnOnDieHost {
    /// `FUN_008ce588(this-0x1c, info)` — gate deciding whether the death
    /// cascade runs at all.
    fn die_gate_008ce588(&mut self, field_m1c: i32, info: &DamageInfo) -> bool;
    /// `FUN_00449dc5` — `TheGameLogic::findObjectByID` (`0` = not found).
    fn find_object_00449dc5(&mut self, id: i32) -> i32;
    /// dword array at `Object+0x244`, read until the terminating `0`
    /// (entries before the sentinel, in order).
    fn modules_0244(&mut self, obj: i32) -> Vec<i32>;
    /// load `*(entry+0x0c)`, call vtable slot `+0x68` through it; result
    /// `0` = miss. (`(int *)(**(code **)(*(int *)(*piVar5 + 0xc) + 0x68))()`)
    fn entry_virtual_0068(&mut self, entry: i32) -> i32;
    /// `candidate`'s vtable slot `+0x08` with the `DamageInfo*` argument.
    fn candidate_virtual_0008(&mut self, candidate: i32, info: &DamageInfo);
    /// `FUN_0068afd2(0)` — fires once per found spawn, after the module scan.
    fn reset_0068afd2(&mut self);
    /// `*(char*)(owner + 0x18) != 0` — second-pass gate.
    fn owner_byte_018(&mut self, owner: i32) -> bool;
    /// `*(byte*)(obj + 0x438) & 1` — live-state flag bit (reference: object
    /// status byte; set = the skip/cleanup side in every body here).
    fn status_bit_438_0(&mut self, obj: i32) -> bool;
    /// `FUN_006984d4(8, 0)` — constant-argument die call (kind code `8`).
    fn die_kind8_006984d4(&mut self);
}

/// `0x0085f7b5` — `SpawnBehavior::onDie(DamageInfo const*)`.
///
/// Flow: die gate `FUN_008ce588(this-0x1c, info)`; when it passes, walk the
/// spawned list once — each found object's `+0x244` null-terminated module
/// array is scanned, the first module whose vtable-`+0x68` probe returns
/// non-null gets its virtual slot `+0x08` called with the damage info and
/// the scan breaks — then `FUN_0068afd2(0)` per found object. Second pass
/// (only when owner byte `+0x18` is set): every found spawn whose `+0x438`
/// bit0 is *clear* receives `FUN_006984d4(8, 0)`.
pub fn spawn_behavior_on_die(
    slots: &SpawnBehaviorSlots,
    spawned_ids: &[i32],
    info: &DamageInfo,
    host: &mut dyn SpawnOnDieHost,
) {
    if !host.die_gate_008ce588(slots.die_gate_m1c, info) {
        return;
    }
    for &pid in spawned_ids {
        let obj = host.find_object_00449dc5(pid);
        if obj != 0 {
            for &entry in &host.modules_0244(obj) {
                let candidate = host.entry_virtual_0068(entry);
                if candidate != 0 {
                    host.candidate_virtual_0008(candidate, info);
                    break;
                }
            }
            host.reset_0068afd2();
        }
    }
    if host.owner_byte_018(slots.owner_m20) {
        for &pid in spawned_ids {
            let obj = host.find_object_00449dc5(pid);
            if obj != 0 && !host.status_bit_438_0(obj) {
                host.die_kind8_006984d4();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 4. AutoAbilityBehavior target predicate
// ---------------------------------------------------------------------------

/// Slot window of `AutoAbilityBehavior` used by `0x0085a57c`.
pub struct AutoAbilityBehaviorSlots {
    /// `AutoAbilityBehavior::m_ownerObject @ +0x08` (handle).
    pub owner_08: i32,
    /// slot `+0x20`: its *address* is the argument of `FUN_0071be3c`
    /// (the body passes `param_1 + 0x20`, not the value).
    pub slot_20_addr: i32,
}

/// Hook surface for [`auto_ability_behavior_target_predicate`]. Field
/// accessors re-read the owner each time they are called, exactly as the
/// body loads `iVar1 + off` at each conjunct (the intervening `FUN_` calls
/// may mutate the object).
pub trait AutoAbilityTargetHost {
    /// `FUN_004454ba()` — global gate; the body enters the predicate when it
    /// returns `0` (true skips the whole conjunction).
    fn gate_004454ba(&mut self) -> bool;
    /// `FUN_00731682(owner + 0x94)` — called only when `gate_004454ba` says
    /// enter-false; the body enters when it returns `0`.
    fn pred_00731682(&mut self, owner_addr_94: i32) -> bool;
    /// `FUN_0071be3c(this + 0x20)` — resolves the ability context handle.
    fn resolve_context_0071be3c(&mut self, slot_20_addr: i32) -> i32;
    /// `FUN_0075b069(owner)`.
    fn pred_0075b069(&mut self, owner: i32) -> bool;
    /// `FUN_006922d9(context)`.
    fn pred_006922d9(&mut self, context: i32) -> bool;
    /// `*(uint*)(owner + 0x11c)`.
    fn flags_11c(&mut self, owner: i32) -> u32;
    /// `*(int*)(owner + 0x118)`.
    fn field_118(&mut self, owner: i32) -> i32;
    /// `*(uint*)(owner + 0x120)`.
    fn flags_120(&mut self, owner: i32) -> u32;
    /// `FUN_0068f4ef()` — handle; `0` fails the tail.
    fn handle_0068f4ef(&mut self) -> i32;
    /// `FUN_00663546(context + 0x13c)` — must return `0`.
    fn pred_00663546(&mut self, context_addr_13c: i32) -> bool;
    /// `FUN_0044e536(0x4a)` — global option query; must return `0`.
    fn option_0044e536_4a(&mut self) -> bool;
    /// `*(int*)(owner + 600)` (`@ +0x258`): sub-object pointer.
    fn field_258(&mut self, owner: i32) -> i32;
    /// `*(*(int*)(owner + 600) + 0x34)` — deref; queried only when
    /// `field_258 != 0` (the body's `||` short-circuit).
    fn deref_258_field_34(&mut self, owner: i32) -> i32;
}

/// `0x0085a57c` — `AutoAbilityBehavior` target predicate (RVA slot
/// `rva0045A57C`, `bool ()`). True only when *both* global gates say enter
/// (`FUN_004454ba` false **or** `FUN_00731682(owner+0x94)` false),
/// `FUN_0075b069(owner)` holds, `FUN_006922d9(ctx)` holds, owner flag bit
/// `+0x11c` bit0 is clear, `field_118 > -1`, flag `+0x120` bit3 is clear,
/// `FUN_0068f4ef()` is non-null, and `FUN_00663546(ctx+0x13c)` **and**
/// `FUN_0044e536(0x4a)` return 0, with the `+0x258` sub-object either
/// absent or having a zero dword at its `+0x34`.
pub fn auto_ability_behavior_target_predicate(
    slots: &AutoAbilityBehaviorSlots,
    host: &mut dyn AutoAbilityTargetHost,
) -> bool {
    let owner = slots.owner_08;
    // `if (cVar2 == 0 || (cVar2 = FUN_00731682(iVar1 + 0x94), cVar2 == 0))`
    if host.gate_004454ba() && host.pred_00731682(owner.wrapping_add(0x94)) {
        return false;
    }
    let context = host.resolve_context_0071be3c(slots.slot_20_addr);
    if host.pred_0075b069(owner)
        && host.pred_006922d9(context)
        && (host.flags_11c(owner) & 1) == 0
        && host.field_118(owner) > -1
        && (host.flags_120(owner) >> 3 & 1) == 0
    {
        let handle = host.handle_0068f4ef();
        if handle != 0
            && !host.pred_00663546(context.wrapping_add(0x13c))
            && !host.option_0044e536_4a()
            && (host.field_258(owner) == 0 || host.deref_258_field_34(owner) == 0)
        {
            return true;
        }
    }
    false
}

// ---------------------------------------------------------------------------
// 5. GettingBuiltBehavior::onObjectCreated
// ---------------------------------------------------------------------------

/// Slot window of `GettingBuiltBehavior` used by `0x008534f5`.
pub struct GettingBuiltCreatedSlots {
    /// `this + 0x04`: production config block pointer (sub-records at
    /// `+0x14` / `+0x18`, flag byte at `+0x1c`).
    pub config_04: i32,
    /// `this + 0x08`: owning `Object*` handle.
    pub owner_08: i32,
    /// `this + 0x20`: interface object whose vtable slot `+0x10` is notified.
    pub iface_20: i32,
}

/// Hook surface for [`getting_built_behavior_on_object_created`].
pub trait GettingBuiltCreatedHost {
    /// `*(char*)(config + 0x1c)`.
    fn config_byte_1c(&mut self, config: i32) -> u8;
    /// `FUN_0068afa9()` — current builder-ish object, `0` when absent.
    fn builder_or_nil_0068afa9(&mut self) -> i32;
    /// `*(int*)(builder + 0x34)` — template pointer, `0` when absent.
    fn builder_template_034(&mut self, builder: i32) -> i32;
    /// `*(char*)(template + 0x1bc)`.
    fn template_byte_1bc(&mut self, template: i32) -> u8;
    /// `FUN_006d06ca(record_addr)` — record lookup by config sub-record
    /// address; `0` = none.
    fn lookup_record_006d06ca(&mut self, record_addr: i32) -> i32;
    /// `*(int*)(owner + 4)` — owner's `ThingTemplate*` handle.
    fn owner_template_004(&mut self, owner: i32) -> i32;
    /// `*(uint*)(handle + 0x11c)` — template/record flag dword (shared
    /// layout; also read off the looked-up record).
    fn flags_11c(&mut self, handle: i32) -> u32;
    /// `*(uint*)(handle + 0x118)` — second flag dword.
    fn flags_118(&mut self, handle: i32) -> u32;
    /// vtable slot `+0x10` on `iface_20` with argument `1`.
    fn iface_virtual_010(&mut self, iface: i32, arg: i32);
    /// `FUN_0044e536(0x57)` — global option query.
    fn option_0044e536_57(&mut self) -> bool;
    /// `FUN_0068bc58(0)` — target lookup, `0` when absent.
    fn find_0068bc58_0(&mut self) -> i32;
    /// vtable slot `+0x60` on that target with argument `1`.
    fn target_virtual_060(&mut self, target: i32, arg: i32);
}

/// `0x008534f5` — `GettingBuiltBehavior::onObjectCreated()`.
///
/// Record selection: `config == 0` → no record; `config[+0x1c] == 0` →
/// record at `config + 0x14`; otherwise the builder (`FUN_0068afa9`) must
/// exist (else no record at all), and its template byte `+0x1bc` picks
/// `config + 0x18` (set) or `config + 0x14` (clear — the `LAB_00853546`
/// join). The owner template (`owner[+4]`) flag test then notifies
/// `iface_20` vtable `+0x10(1)` when bit `0x20000000` of `+0x11c` is set,
/// or when bit `0x10000000` of `+0x118` is set **and** the record is
/// missing / carries either of those same bits. Finally option `0x57` plus
/// a non-null `FUN_0068bc58(0)` trigger vtable `+0x60(1)`.
pub fn getting_built_behavior_on_object_created(
    slots: &GettingBuiltCreatedSlots,
    host: &mut dyn GettingBuiltCreatedHost,
) {
    if slots.owner_08 == 0 {
        return;
    }
    let config = slots.config_04;
    let mut record: i32 = 0;
    if config != 0 {
        if host.config_byte_1c(config) == 0 {
            record = host.lookup_record_006d06ca(config.wrapping_add(0x14));
        } else {
            let builder = host.builder_or_nil_0068afa9();
            if builder != 0 {
                let tmpl = host.builder_template_034(builder);
                let flag_byte = if tmpl == 0 { 0 } else { host.template_byte_1bc(tmpl) };
                let rec_addr = if flag_byte == 0 {
                    config.wrapping_add(0x14)
                } else {
                    config.wrapping_add(0x18)
                };
                record = host.lookup_record_006d06ca(rec_addr);
            }
        }
    }
    let owner_template = host.owner_template_004(slots.owner_08);
    if (host.flags_11c(owner_template) & 0x2000_0000) != 0
        || ((host.flags_118(owner_template) & 0x1000_0000) != 0
            && (record == 0
                || (host.flags_11c(record) & 0x2000_0000) != 0
                || (host.flags_118(record) & 0x1000_0000) != 0))
    {
        host.iface_virtual_010(slots.iface_20, 1);
    }
    if host.option_0044e536_57() {
        let target = host.find_0068bc58_0();
        if target != 0 {
            host.target_virtual_060(target, 1);
        }
    }
}

// ---------------------------------------------------------------------------
// 6. SlaveWatcherBehavior::update
// ---------------------------------------------------------------------------

/// Block size the body copies: the `for (iVar5 = 0x20; ...)` dword loop.
pub const SLAVE_WATCH_BLOCK_DWORDS: usize = 0x20;

/// Slot window of `SlaveWatcherBehavior` used by `0x008847a9`.
pub struct SlaveWatcherSlots {
    /// `this - 0x08`: companion object handle (its `+0x284` block is the
    /// copy source).
    pub companion_m08: i32,
    /// `this - 0x0c`: own object handle (pointer slots `+0x08`/`+0x0c` are
    /// the cleanup targets; flag byte at `+0x10`).
    pub owner_m0c: i32,
    /// `this + 0x10`: watched object handle — cleared when culled.
    pub master_10: i32,
    /// `this + 0x14 .. +0x94`: the 0x20-dword block refreshed from the
    /// companion's `+0x284`.
    pub block_14: [u32; SLAVE_WATCH_BLOCK_DWORDS],
}

/// Hook surface for [`slave_watcher_behavior_update`].
pub trait SlaveWatcherUpdateHost {
    /// `FUN_00449dc5` — `TheGameLogic::findObjectByID`.
    fn find_object_00449dc5(&mut self, id: i32) -> i32;
    /// `*(byte*)(obj + 0x438) & 1` — set = the watched object is gone for
    /// this watcher's purposes.
    fn status_bit_438_0(&mut self, obj: i32) -> bool;
    /// `FUN_0066f26d(addr)` — resolve the pointer stored at `addr`.
    fn resolve_ptr_0066f26d(&mut self, addr: i32) -> i32;
    /// `FUN_00690d42(p)` — cleanup action on the `owner+0x0c` pointer.
    fn cleanup_00690d42(&mut self, p: i32);
    /// `FUN_00693077(p)` — cleanup action on the `owner+0x08` pointer.
    fn cleanup_00693077(&mut self, p: i32);
    /// `FUN_00692eea()` — post-cull / post-copy notification.
    fn notify_00692eea(&mut self);
    /// `*(char*)(owner + 0x10) != 0` — refresh gate.
    fn byte_010(&mut self, owner: i32) -> bool;
    /// `FUN_0085f4c2(companion + 0x284, this + 0x14)` — the two-address
    /// predicate; modelled over the two 0x20-dword blocks.
    fn block_predicate_0085f4c2(
        &mut self,
        src: &[u32; SLAVE_WATCH_BLOCK_DWORDS],
        dst: &[u32; SLAVE_WATCH_BLOCK_DWORDS],
    ) -> bool;
    /// `FUN_0068c557(this + 0x14)` — block touch after a successful copy.
    fn touch_block_0068c557(&mut self);
}

/// `0x008847a9` — `SlaveWatcherBehavior::update() -> UpdateSleepTime`.
///
/// When `this+0x10` (the watched handle) is set: resolve it; missing **or**
/// `+0x438` bit0 set → resolve `FUN_0066f26d(owner+0x0c)` and
/// `FUN_0066f26d(owner+0x08)`, run the non-null cleanups
/// (`FUN_00690d42` first, then `FUN_00693077`), notify `FUN_00692eea`,
/// clear `+0x10`, and return `0x3fffffff` (`UPDATE_SLEEP_FOREVER`, SDK
/// enum name; raw value from the body). Otherwise, with `owner[+0x10]` set
/// and `FUN_0085f4c2` true, the companion block `+0x284` overwrites
/// `this+0x14` (0x20 dwords), then `FUN_0068c557` + `FUN_00692eea`. Every
/// non-culled path returns `1`.
pub fn slave_watcher_behavior_update(
    w: &mut SlaveWatcherSlots,
    companion_block: &[u32; SLAVE_WATCH_BLOCK_DWORDS],
    host: &mut dyn SlaveWatcherUpdateHost,
) -> i32 {
    if w.master_10 != 0 {
        let master = host.find_object_00449dc5(w.master_10);
        if master == 0 || host.status_bit_438_0(master) {
            let p_c = host.resolve_ptr_0066f26d(w.owner_m0c.wrapping_add(0xc));
            let p_8 = host.resolve_ptr_0066f26d(w.owner_m0c.wrapping_add(8));
            if p_c != 0 {
                host.cleanup_00690d42(p_c);
            }
            if p_8 != 0 {
                host.cleanup_00693077(p_8);
            }
            host.notify_00692eea();
            w.master_10 = 0;
            return 0x3fff_ffff;
        }
        if host.byte_010(w.owner_m0c)
            && host.block_predicate_0085f4c2(companion_block, &w.block_14)
        {
            w.block_14.copy_from_slice(companion_block);
            host.touch_block_0068c557();
            host.notify_00692eea();
        }
    }
    1
}

// ---------------------------------------------------------------------------
// 7. FireWeaponWhenDamagedBehavior::onDamage
// ---------------------------------------------------------------------------

/// Slot window of `FireWeaponWhenDamagedBehavior` used by `0x00882a69`.
pub struct FireWeaponWhenDamagedSlots {
    /// `this - 0x08`: object whose vtable slot `+0x00` gates the handler.
    pub vobj_m08: i32,
    /// `this - 0x24`: `*(this - 0x24)` — flag/float carrier (object- or
    /// template-shaped: `+0x11c` dword, `+0x120` float).
    pub carrier_m24: i32,
    /// `this - 0x20`: owning `Object*` handle.
    pub owner_m20: i32,
    /// `this + 0x04`: weapon object used when the weapon-set status is not
    /// `1`/`2`/`3` (the `else` arm).
    pub weapon_04: i32,
    /// `this + 0x08`: weapon object for status `1`.
    pub weapon_08: i32,
    /// `this + 0x0c`: weapon object for status `2`.
    pub weapon_0c: i32,
    /// `this + 0x10`: weapon object for status `3`.
    pub weapon_10: i32,
}

/// Hook surface for [`fire_weapon_when_damaged_behavior_on_damage`].
pub trait FireWeaponDamagedHost {
    /// vtable slot `+0x00` on `*(this-0x08)` — gate.
    fn vobj_virtual_000(&mut self, vobj: i32) -> bool;
    /// `*(uint*)(carrier + 0x11c)` — kindof/weapon-kind flag dword.
    fn flags_11c(&mut self, carrier: i32) -> u32;
    /// `*(float*)(carrier + 0x120)` — damage threshold.
    fn threshold_120(&mut self, carrier: i32) -> f32;
    /// load `*(owner + 0x254)`, call its vtable slot `+0x20` — weapon-set
    /// status (`1`/`2`/`3`/other selects the behavior's weapon slot).
    fn weapon_status_0254_020(&mut self, owner: i32) -> i32;
    /// `FUN_006ccfc7()` — current fired-weapon object; `0` triggers the
    /// fallback fire.
    fn fired_weapon_006ccfc7(&mut self) -> i32;
    /// `FUN_006ce76f(owner, owner + 0x38)` — fire using the owner's own
    /// position block (`Object+0x38`).
    fn fire_at_self_006ce76f(&mut self, owner: i32, pos_addr: i32);
}

/// `0x00882a69` — `FireWeaponWhenDamagedBehavior::onDamage(DamageInfo*)`.
///
/// Gate: vtable-`+0x00` on `this-0x08`, then the damage kind selects a bit
/// `1u32 << (((char)info[+0x10] - 1u) & 0x1f)` that must intersect the
/// carrier's `+0x11c` flags, then the carrier threshold float `+0x120` must
/// satisfy `th < amount || th == amount` (NaN fails either side). The
/// owner's weapon-set status (`+0x254` object vtable `+0x20`) picks the
/// behavior slot — status `3` → `+0x10`, `2` → `+0x0c`, `1` → `+0x08`,
/// anything else → `+0x04` — and a null slot returns early. `FUN_006ccfc7`
/// replaces the value; only a `0` result fires
/// `FUN_006ce76f(owner, owner + 0x38)`.
pub fn fire_weapon_when_damaged_behavior_on_damage(
    slots: &FireWeaponWhenDamagedSlots,
    info: &DamageInfo,
    host: &mut dyn FireWeaponDamagedHost,
) {
    if !host.vobj_virtual_000(slots.vobj_m08) {
        return;
    }
    // `(char)info[+0x10]` truncates to the low byte sign-extended, then the
    // `- 1u` / `& 0x1f` math is unsigned.
    let kind_char = (info.kind_10 as u8) as i8;
    let shift = (kind_char as u32).wrapping_sub(1) & 0x1f;
    let bit = 1u32 << shift;
    if host.flags_11c(slots.carrier_m24) & bit == 0 {
        return;
    }
    let threshold = host.threshold_120(slots.carrier_m24);
    if !(threshold < info.amount_70 || threshold == info.amount_70) {
        return;
    }
    let weapon = match host.weapon_status_0254_020(slots.owner_m20) {
        3 => slots.weapon_10,
        2 => slots.weapon_0c,
        1 => slots.weapon_08,
        _ => slots.weapon_04,
    };
    if weapon == 0 {
        return;
    }
    if host.fired_weapon_006ccfc7() == 0 {
        host.fire_at_self_006ce76f(slots.owner_m20, slots.owner_m20.wrapping_add(0x38));
    }
}

// ---------------------------------------------------------------------------
// 8. GettingBuiltBehavior adjacent-site broadcast
// ---------------------------------------------------------------------------

/// Slot window of `GettingBuiltBehavior` used by `0x00853f31` (behavior
/// sits `+0x18` inside its owning object).
pub struct GettingBuiltSiteSlots {
    /// `this - 0x18`: owning `Object*` handle.
    pub owner_m18: i32,
}

/// Hook surface for [`getting_built_behavior_broadcast_to_adjacent_sites`].
pub trait BroadcastToSitesHost {
    /// `*(int*)(owner + 4)` — owner template handle (`uVar1`); read before
    /// the status test, so always queried.
    fn owner_template_004(&mut self, owner: i32) -> i32;
    /// `*(int*)(owner + 0x74)` — owner `Object_ID` (`iVar6`); read before
    /// the status test, so always queried.
    fn owner_id_074(&mut self, owner: i32) -> i32;
    /// `*(byte*)(obj + 0x438) & 1`.
    fn status_bit_438_0(&mut self, obj: i32) -> bool;
    /// own vtable slot `+0x10` (`(**(code **)(*param_1 + 0x10))(param_2)`).
    fn self_virtual_010(&mut self, value: i32);
    /// `FUN_00449dc5(id)` — `TheGameLogic::findObjectByID`.
    fn find_object_00449dc5(&mut self, id: i32) -> i32;
    /// `FUN_0073bb04(template)` — same-template-class gate; re-evaluated
    /// every iteration (constant per call, but its position in the
    /// short-circuit chain decides when it runs).
    fn template_gate_0073bb04(&mut self, template: i32) -> bool;
    /// `FUN_0068bd17()` — the notified interface/neighbor object, `0` skips.
    fn neighbor_or_nil_0068bd17(&mut self) -> i32;
    /// neighbor's vtable slot `+0x10` with the original `value`.
    fn neighbor_virtual_010(&mut self, neighbor: i32, value: i32);
}

/// `0x00853f31` — `GettingBuiltBehavior::rva00453F31(int)`: broadcast a
/// value to consecutive `Object_ID`s around the owner.
///
/// The owner's template (`+0x04`) and `Object_ID` (`+0x74`) are read
/// **before** the `+0x438` bit0 test (the body loads them unconditionally).
/// When the bit is set: call own vtable `+0x10(value)`, then walk IDs
/// `id+1, id+2, …` and separately `id-1, id-2, …`; each walk continues
/// only while the id resolves, `FUN_0073bb04(template)` holds, and the
/// found object carries the same `+0x438` bit0 — the first failure breaks
/// that direction. Every surviving neighbor gets `FUN_0068bd17()`'s result
/// (when non-null) notified through its vtable `+0x10(value)`. The downward
/// walk always starts from the original id (the upward walk mutates only
/// its own copy — `iVar5 = iVar6`, then `iVar5 + 1`).
pub fn getting_built_behavior_broadcast_to_adjacent_sites(
    slots: &GettingBuiltSiteSlots,
    value: i32,
    host: &mut dyn BroadcastToSitesHost,
) {
    let owner = slots.owner_m18;
    let template = host.owner_template_004(owner);
    let base_id = host.owner_id_074(owner);
    if !host.status_bit_438_0(owner) {
        return;
    }
    host.self_virtual_010(value);
    let mut probe = base_id;
    loop {
        probe = probe.wrapping_add(1);
        let obj = host.find_object_00449dc5(probe);
        if obj == 0 || !host.template_gate_0073bb04(template) {
            break;
        }
        if !host.status_bit_438_0(obj) {
            break;
        }
        let neighbor = host.neighbor_or_nil_0068bd17();
        if neighbor != 0 {
            host.neighbor_virtual_010(neighbor, value);
        }
    }
    let mut down = base_id;
    loop {
        down = down.wrapping_sub(1);
        let obj = host.find_object_00449dc5(down);
        if obj == 0 || !host.template_gate_0073bb04(template) {
            break;
        }
        if !host.status_bit_438_0(obj) {
            break;
        }
        let neighbor = host.neighbor_or_nil_0068bd17();
        if neighbor != 0 {
            host.neighbor_virtual_010(neighbor, value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- rows 1 + 3: gate closest scans -----------------------------------

    #[derive(Default)]
    struct GateHost {
        rels: Vec<(i32, i32)>,
        owners: Vec<(i32, i32)>,
        positions: Vec<(i32, [f32; 3])>,
        pos_queries: usize,
    }
    impl GateClosestHost for GateHost {
        fn relationship_to_008989df(&mut self, gate: i32, _player: i32) -> i32 {
            self.rels.iter().find(|(g, _)| *g == gate).map_or(0, |(_, r)| *r)
        }
        fn controlling_player_008989d7(&mut self, gate: i32) -> i32 {
            self.owners.iter().find(|(g, _)| *g == gate).map_or(0, |(_, o)| *o)
        }
        fn gate_position_008989ee(&mut self, gate: i32) -> [f32; 3] {
            self.pos_queries += 1;
            self.positions.iter().find(|(g, _)| *g == gate).map_or([0.0; 3], |(_, p)| *p)
        }
    }

    #[test]
    fn gate_scan_allied_filter_and_minimum() {
        let mut h = GateHost {
            rels: vec![(7, 2), (9, 5), (11, 2)],
            positions: vec![(7, [11.0, 0.0, 0.0]), (9, [0.0, 0.0, 0.0]), (11, [12.0, 0.0, 0.0])],
            ..Default::default()
        };
        // gate 7 at distance 1 from (10,0,0) beats gate 11 at 4; gate 9
        // (relationship 5) never reaches the position query.
        assert_eq!(gate_open_list_find_closest_allied(&[7, 9, 11], [10.0, 0.0, 0.0], 3, &mut h), 7);
        assert_eq!(h.pos_queries, 2, "position is queried only for passing filters");
    }

    #[test]
    fn gate_scan_ties_keep_the_first_and_nan_latches_only_when_empty() {
        let mut h = GateHost {
            rels: vec![(1, 2), (2, 2), (3, 2)],
            positions: vec![(1, [5.0, 0.0, 0.0]), (2, [5.0, 0.0, 0.0]), (3, [f32::NAN, 0.0, 0.0])],
            ..Default::default()
        };
        // strict `<`: an equal distance keeps the earlier entry; a NaN
        // distance cannot replace a real best.
        assert_eq!(gate_open_list_find_closest_allied(&[1, 2, 3], [0.0, 0.0, 0.0], 3, &mut h), 1);
        // ...but the first qualifying candidate latches even with NaN
        // distance (the `best == 0` branch).
        let mut h2 = GateHost {
            rels: vec![(3, 2)],
            positions: vec![(3, [f32::NAN, 0.0, 0.0])],
            ..Default::default()
        };
        assert_eq!(gate_open_list_find_closest_allied(&[3], [0.0, 0.0, 0.0], 3, &mut h2), 3);
    }

    #[test]
    fn gate_scan_zero_entry_never_latches() {
        // The `best == 0` sentinel treats a literal-0 entry as "no best
        // yet": the later (farther) entry replaces it.
        let mut h = GateHost {
            rels: vec![(0, 2), (5, 2)],
            positions: vec![(0, [1.0, 0.0, 0.0]), (5, [10.0, 0.0, 0.0])],
            ..Default::default()
        };
        assert_eq!(gate_open_list_find_closest_allied(&[0, 5], [0.0, 0.0, 0.0], 3, &mut h), 5);
        let mut empty = GateHost::default();
        assert_eq!(gate_open_list_find_closest_allied(&[], [0.0, 0.0, 0.0], 3, &mut empty), 0);
        assert_eq!(empty.pos_queries, 0);
    }

    #[test]
    fn gate_scan_controlled_variant_compares_owners() {
        let mut h = GateHost {
            owners: vec![(7, 4), (9, 3)],
            positions: vec![(7, [1.0, 0.0, 0.0]), (9, [2.0, 0.0, 0.0])],
            ..Default::default()
        };
        // player 3: only gate 9 passes; nearest-scan returns it.
        assert_eq!(gate_open_list_find_closest_controlled(&[7, 9], [0.0, 0.0, 0.0], 3, &mut h), 9);
        assert_eq!(h.pos_queries, 1, "mismatched owner skips the position query");
        // no match at all
        assert_eq!(gate_open_list_find_closest_controlled(&[7, 9], [0.0, 0.0, 0.0], 99, &mut h), 0);
        assert_eq!(h.pos_queries, 1);
    }

    // -- row 2: SpawnBehavior::onDie ---------------------------------------

    #[derive(Default)]
    struct SpawnHost {
        log: Vec<&'static str>,
        objects: Vec<(i32, i32)>, // spawned id -> object handle (0 = gone)
        mods: Vec<(i32, Vec<i32>)>,
        probes: Vec<(i32, i32)>, // module entry -> candidate (0 = miss)
        dead_objs: Vec<i32>,
        owner_byte: bool,
        gate_pass: bool,
    }
    impl SpawnOnDieHost for SpawnHost {
        fn die_gate_008ce588(&mut self, field_m1c: i32, _info: &DamageInfo) -> bool {
            self.log.push("gate");
            let _ = field_m1c;
            self.gate_pass
        }
        fn find_object_00449dc5(&mut self, id: i32) -> i32 {
            self.log.push("find");
            self.objects.iter().find(|(i, _)| *i == id).map_or(0, |(_, o)| *o)
        }
        fn modules_0244(&mut self, obj: i32) -> Vec<i32> {
            self.log.push("mods");
            self.mods.iter().find(|(o, _)| *o == obj).map(|(_, m)| m.clone()).unwrap_or_default()
        }
        fn entry_virtual_0068(&mut self, entry: i32) -> i32 {
            self.log.push("probe");
            self.probes.iter().find(|(e, _)| *e == entry).map_or(0, |(_, c)| *c)
        }
        fn candidate_virtual_0008(&mut self, candidate: i32, _info: &DamageInfo) {
            self.log.push("hit");
            let _ = candidate;
        }
        fn reset_0068afd2(&mut self) {
            self.log.push("reset");
        }
        fn owner_byte_018(&mut self, _owner: i32) -> bool {
            self.log.push("byte18");
            self.owner_byte
        }
        fn status_bit_438_0(&mut self, obj: i32) -> bool {
            self.log.push("status");
            self.dead_objs.contains(&obj)
        }
        fn die_kind8_006984d4(&mut self) {
            self.log.push("die8");
        }
    }

    fn spawn_slots() -> SpawnBehaviorSlots {
        SpawnBehaviorSlots { owner_m20: 0x1000, die_gate_m1c: 0x1234, spawned_list_28: 0x2000 }
    }

    #[test]
    fn spawn_die_gate_closed_runs_nothing() {
        let mut h = SpawnHost::default();
        spawn_behavior_on_die(&spawn_slots(), &[1, 2], &DamageInfo::default(), &mut h);
        assert_eq!(h.log, ["gate"]);
    }

    #[test]
    fn spawn_module_break_and_per_object_reset() {
        let mut h = SpawnHost {
            objects: vec![(1, 100), (2, 0)],
            mods: vec![(100, vec![5, 6])],
            probes: vec![(5, 0), (6, 77)],
            ..Default::default()
        };
        h.gate_pass = true;
        spawn_behavior_on_die(&spawn_slots(), &[1, 2], &DamageInfo::default(), &mut h);
        assert_eq!(
            h.log,
            [
                "gate", "find", "mods", "probe", "probe", "hit", "reset", // obj for id 1
                "find", // id 2 -> null object: no mods/probe/reset
                "byte18", // second-pass gate checked once, owner byte false
            ]
        );
    }

    #[test]
    fn spawn_second_pass_kills_only_live_spawns() {
        let mut h = SpawnHost {
            objects: vec![(1, 100), (2, 101), (3, 0)],
            mods: vec![(100, vec![]), (101, vec![])],
            dead_objs: vec![101],
            owner_byte: true,
            ..Default::default()
        };
        h.gate_pass = true;
        spawn_behavior_on_die(&spawn_slots(), &[1, 2, 3], &DamageInfo::default(), &mut h);
        // first pass: reset thrice (no modules -> no probes/hit); second
        // pass: die8 only for the live found object (100).
        let dies = h.log.iter().filter(|e| **e == "die8").count();
        let statuses = h.log.iter().filter(|e| **e == "status").count();
        assert_eq!(dies, 1);
        assert_eq!(statuses, 2, "status is queried for found objects only");
        assert_eq!(h.log.iter().filter(|e| **e == "find").count(), 6); // 3 + 3
    }

    // -- row 4: AutoAbilityBehavior target predicate ------------------------

    #[derive(Default)]
    struct AutoHost {
        log: Vec<&'static str>,
        gate: bool,
        pred94: bool,
        context: i32,
        pred75: bool,
        pred922: bool,
        f11c: u32,
        f118: i32,
        f120: u32,
        handle: i32,
        pred635: bool,
        opt4a: bool,
        f258: i32,
        deref34: i32,
    }
    impl AutoAbilityTargetHost for AutoHost {
        fn gate_004454ba(&mut self) -> bool { self.log.push("gate"); self.gate }
        fn pred_00731682(&mut self, addr: i32) -> bool {
            self.log.push("p94");
            assert_eq!(addr, 0x5000 + 0x94);
            self.pred94
        }
        fn resolve_context_0071be3c(&mut self, slot20: i32) -> i32 {
            self.log.push("ctx");
            assert_eq!(slot20, 0x4000 + 0x20); // the slot *address* is passed
            self.context
        }
        fn pred_0075b069(&mut self, owner: i32) -> bool {
            self.log.push("p75");
            assert_eq!(owner, 0x5000);
            self.pred75
        }
        fn pred_006922d9(&mut self, ctx: i32) -> bool {
            self.log.push("p922");
            assert_eq!(ctx, self.context);
            self.pred922
        }
        fn flags_11c(&mut self, _owner: i32) -> u32 { self.log.push("f11c"); self.f11c }
        fn field_118(&mut self, _owner: i32) -> i32 { self.log.push("f118"); self.f118 }
        fn flags_120(&mut self, _owner: i32) -> u32 { self.log.push("f120"); self.f120 }
        fn handle_0068f4ef(&mut self) -> i32 { self.log.push("h4ef"); self.handle }
        fn pred_00663546(&mut self, addr: i32) -> bool {
            self.log.push("p635");
            assert_eq!(addr, self.context + 0x13c);
            self.pred635
        }
        fn option_0044e536_4a(&mut self) -> bool { self.log.push("opt4a"); self.opt4a }
        fn field_258(&mut self, _owner: i32) -> i32 { self.log.push("f258"); self.f258 }
        fn deref_258_field_34(&mut self, _owner: i32) -> i32 {
            self.log.push("d34");
            self.deref34
        }
    }

    fn auto_slots() -> AutoAbilityBehaviorSlots {
        AutoAbilityBehaviorSlots { owner_08: 0x5000, slot_20_addr: 0x4000 + 0x20 }
    }

    #[test]
    fn auto_predicate_all_gates_open_returns_true() {
        let mut h = AutoHost {
            gate: false, // enters via the first disjunct; p94 must NOT run
            context: 0x9000,
            pred75: true,
            pred922: true,
            f11c: 0,
            f118: 0,
            f120: 0,
            handle: 1,
            f258: 0, // sub-object absent: deref must NOT run
            ..Default::default()
        };
        assert!(auto_ability_behavior_target_predicate(&auto_slots(), &mut h));
        assert_eq!(
            h.log,
            ["gate", "ctx", "p75", "p922", "f11c", "f118", "f120", "h4ef", "p635", "opt4a", "f258"]
        );
    }

    #[test]
    fn auto_predicate_second_gate_and_boundaries() {
        // gate true + pred94 true -> body skipped entirely (only 2 calls).
        let mut h = AutoHost { gate: true, pred94: true, ..Default::default() };
        assert!(!auto_ability_behavior_target_predicate(&auto_slots(), &mut h));
        assert_eq!(h.log, ["gate", "p94"]);

        // gate true + pred94 false -> enters; field_118 == -1 fails `> -1`.
        let mut h = AutoHost {
            gate: true,
            pred94: false,
            pred75: true,
            pred922: true,
            f118: -1,
            ..Default::default()
        };
        assert!(!auto_ability_behavior_target_predicate(&auto_slots(), &mut h));
        assert_eq!(h.log, ["gate", "p94", "ctx", "p75", "p922", "f11c", "f118"]);

        // bit0 of +0x11c set: f118/f120 not even read.
        let mut h = AutoHost {
            gate: false,
            pred75: true,
            pred922: true,
            f11c: 1,
            ..Default::default()
        };
        assert!(!auto_ability_behavior_target_predicate(&auto_slots(), &mut h));
        assert_eq!(h.log, ["gate", "ctx", "p75", "p922", "f11c"]);

        // bit3 of +0x120 set fails; bits outside >>3 & 1 must not.
        let mut h = AutoHost {
            gate: false,
            pred75: true,
            pred922: true,
            f120: 0xf7, // bit3 clear? 0xf7 = 1111_0111: bit3 = 0
            handle: 1,
            ..Default::default()
        };
        assert!(auto_ability_behavior_target_predicate(&auto_slots(), &mut h));
        let mut h = AutoHost {
            gate: false,
            pred75: true,
            pred922: true,
            f120: 8, // bit3 set -> fail, handle not fetched
            ..Default::default()
        };
        assert!(!auto_ability_behavior_target_predicate(&auto_slots(), &mut h));
        assert!(!h.log.contains(&"h4ef"));

        // tail: sub-object present with a NONZERO +0x34 dword fails.
        let mut h = AutoHost {
            gate: false,
            pred75: true,
            pred922: true,
            handle: 1,
            f258: 0x7000,
            deref34: 7,
            ..Default::default()
        };
        assert!(!auto_ability_behavior_target_predicate(&auto_slots(), &mut h));
        assert!(h.log.contains(&"d34"));
    }

    // -- row 5: GettingBuiltBehavior::onObjectCreated -----------------------

    #[derive(Default)]
    struct CreatedHost {
        log: Vec<String>,
        cfg_byte: u8,
        builder: i32,
        btmpl: i32,
        tmpl_byte: u8,
        flags: Vec<(i32, u32, u32)>, // handle -> (+0x11c, +0x118)
        iface_arg: Option<i32>,
        option57: bool,
        target: i32,
    }
    impl GettingBuiltCreatedHost for CreatedHost {
        fn config_byte_1c(&mut self, config: i32) -> u8 {
            self.log.push(format!("b1c{config:x}"));
            self.cfg_byte
        }
        fn builder_or_nil_0068afa9(&mut self) -> i32 { self.log.push("builder".into()); self.builder }
        fn builder_template_034(&mut self, b: i32) -> i32 {
            self.log.push(format!("btmpl{b:x}"));
            self.btmpl
        }
        fn template_byte_1bc(&mut self, t: i32) -> u8 {
            self.log.push(format!("t1bc{t:x}"));
            self.tmpl_byte
        }
        fn lookup_record_006d06ca(&mut self, addr: i32) -> i32 {
            self.log.push(format!("lookup{addr:x}"));
            addr // records are handles here: return the address itself
        }
        fn owner_template_004(&mut self, owner: i32) -> i32 {
            self.log.push(format!("otmpl{owner:x}"));
            owner + 4
        }
        fn flags_11c(&mut self, h: i32) -> u32 {
            self.log.push(format!("c{h:x}"));
            self.flags.iter().find(|(k, _, _)| *k == h).map_or(0, |(_, a, _)| *a)
        }
        fn flags_118(&mut self, h: i32) -> u32 {
            self.log.push(format!("8{h:x}"));
            self.flags.iter().find(|(k, _, _)| *k == h).map_or(0, |(_, _, b)| *b)
        }
        fn iface_virtual_010(&mut self, iface: i32, arg: i32) {
            self.log.push(format!("iface{iface:x}:{arg}"));
            self.iface_arg = Some(arg);
        }
        fn option_0044e536_57(&mut self) -> bool { self.log.push("opt57".into()); self.option57 }
        fn find_0068bc58_0(&mut self) -> i32 { self.log.push("find58".into()); self.target }
        fn target_virtual_060(&mut self, t: i32, arg: i32) {
            self.log.push(format!("v60{t:x}:{arg}"));
        }
    }

    fn created_slots(cfg: i32) -> GettingBuiltCreatedSlots {
        GettingBuiltCreatedSlots { config_04: cfg, owner_08: 0x5000, iface_20: 0x6000 }
    }

    #[test]
    fn created_null_owner_returns_at_once() {
        let mut h = CreatedHost::default();
        let s = GettingBuiltCreatedSlots { config_04: 0x3000, owner_08: 0, iface_20: 0 };
        getting_built_behavior_on_object_created(&s, &mut h);
        assert!(h.log.is_empty());
    }

    #[test]
    fn created_record_selection_matrix() {
        // config == 0: no config read, no lookup.
        let mut h = CreatedHost::default();
        getting_built_behavior_on_object_created(&created_slots(0), &mut h);
        assert!(!h.log.iter().any(|e| e.starts_with("lookup")));
        // cfg[+0x1c] == 0 -> +0x14.
        let mut h = CreatedHost::default();
        getting_built_behavior_on_object_created(&created_slots(0x3000), &mut h);
        assert!(h.log.contains(&"lookup3014".to_string()));
        // builder absent -> no lookup at all.
        let mut h = CreatedHost { builder: 0, cfg_byte: 1, ..Default::default() };
        getting_built_behavior_on_object_created(&created_slots(0x3000), &mut h);
        assert!(!h.log.iter().any(|e| e.starts_with("lookup")));
        // builder template absent -> byte reads skipped -> +0x14.
        let mut h = CreatedHost { builder: 0x700, btmpl: 0, cfg_byte: 1, ..Default::default() };
        getting_built_behavior_on_object_created(&created_slots(0x3000), &mut h);
        assert!(h.log.contains(&"lookup3014".to_string()));
        assert!(!h.log.iter().any(|e| e.starts_with("t1bc")));
        // template byte set -> +0x18.
        let mut h = CreatedHost { builder: 0x700, btmpl: 0x800, tmpl_byte: 1, cfg_byte: 1, ..Default::default() };
        getting_built_behavior_on_object_created(&created_slots(0x3000), &mut h);
        assert!(h.log.contains(&"lookup3018".to_string()));
    }

    #[test]
    fn created_flag_matrix_and_option57() {
        // owner template flag: +0x11c bit 0x20000000 set -> iface(1), and
        // flags_118 of the template is never read.
        let mut h = CreatedHost {
            flags: vec![(0x5004, 0x2000_0000, 0)],
            ..Default::default()
        };
        getting_built_behavior_on_object_created(&created_slots(0), &mut h);
        assert_eq!(h.iface_arg, Some(1));
        assert!(!h.log.contains(&"85004".to_string()));

        // +0x118 bit only, record == 0 (config 0) -> iface fires.
        let mut h = CreatedHost { flags: vec![(0x5004, 0, 0x1000_0000)], ..Default::default() };
        getting_built_behavior_on_object_created(&created_slots(0), &mut h);
        assert_eq!(h.iface_arg, Some(1));

        // +0x118 bit set, record present without either bit -> no iface.
        let mut h = CreatedHost { flags: vec![(0x5004, 0, 0x1000_0000)], ..Default::default() };
        getting_built_behavior_on_object_created(&created_slots(0x3000), &mut h);
        // record handle = 0x3014 (lookup echo), flags default 0.
        assert_eq!(h.iface_arg, None);

        // option57 false -> no find; true with null target -> no virtual;
        // true with target -> v60(1).
        let mut h = CreatedHost::default();
        getting_built_behavior_on_object_created(&created_slots(0), &mut h);
        assert!(!h.log.contains(&"find58".to_string()));
        let mut h = CreatedHost { option57: true, target: 0, ..Default::default() };
        getting_built_behavior_on_object_created(&created_slots(0), &mut h);
        assert!(h.log.contains(&"find58".to_string()));
        assert!(!h.log.iter().any(|e| e.starts_with("v60")));
        let mut h = CreatedHost { option57: true, target: 0x900, ..Default::default() };
        getting_built_behavior_on_object_created(&created_slots(0), &mut h);
        assert!(h.log.contains(&"v60900:1".to_string()));
    }

    // -- row 6: SlaveWatcherBehavior::update --------------------------------

    #[derive(Default)]
    struct WatcherHost {
        log: Vec<String>,
        master_obj: i32,
        master_dead: bool,
        ptrs: Vec<(i32, i32)>, // address -> pointer
        byte10: bool,
        predicate: bool,
    }
    impl SlaveWatcherUpdateHost for WatcherHost {
        fn find_object_00449dc5(&mut self, id: i32) -> i32 {
            self.log.push(format!("find{id:x}"));
            let _ = id;
            self.master_obj
        }
        fn status_bit_438_0(&mut self, obj: i32) -> bool {
            self.log.push(format!("st{obj:x}"));
            self.master_dead
        }
        fn resolve_ptr_0066f26d(&mut self, addr: i32) -> i32 {
            self.log.push(format!("res{addr:x}"));
            self.ptrs.iter().find(|(a, _)| *a == addr).map_or(0, |(_, p)| *p)
        }
        fn cleanup_00690d42(&mut self, p: i32) { self.log.push(format!("c0d42{p:x}")); }
        fn cleanup_00693077(&mut self, p: i32) { self.log.push(format!("c3077{p:x}")); }
        fn notify_00692eea(&mut self) { self.log.push("eea".into()); }
        fn byte_010(&mut self, owner: i32) -> bool {
            self.log.push(format!("b10{owner:x}"));
            self.byte10
        }
        fn block_predicate_0085f4c2(
            &mut self,
            _src: &[u32; SLAVE_WATCH_BLOCK_DWORDS],
            _dst: &[u32; SLAVE_WATCH_BLOCK_DWORDS],
        ) -> bool {
            self.log.push("pred".into());
            self.predicate
        }
        fn touch_block_0068c557(&mut self) { self.log.push("c557".into()); }
    }

    fn watcher_slots() -> SlaveWatcherSlots {
        SlaveWatcherSlots { companion_m08: 0xA00, owner_m0c: 0xB00, master_10: 0x77, block_14: [0; 32] }
    }

    #[test]
    fn watcher_clear_master_short_circuits() {
        let mut w = watcher_slots();
        w.master_10 = 0;
        let mut h = WatcherHost::default();
        assert_eq!(slave_watcher_behavior_update(&mut w, &[7; 32], &mut h), 1);
        assert!(h.log.is_empty());
    }

    #[test]
    fn watcher_culls_gone_master_in_body_order() {
        // master resolves to 0: the `master == 0` disjunct skips the status
        // query entirely.
        let mut w = watcher_slots();
        let mut h = WatcherHost {
            master_obj: 0,
            ptrs: vec![(0xB0C, 0xC000), (0xB08, 0)],
            ..Default::default()
        };
        assert_eq!(slave_watcher_behavior_update(&mut w, &[7; 32], &mut h), 0x3fff_ffff);
        assert_eq!(w.master_10, 0);
        assert_eq!(h.log, ["find77", "resb0c", "resb08", "c0d42c000", "eea"]);
        // (the +0x08 slot resolved to 0 -> its cleanup is skipped)

        // master present but flagged: status runs, same cull.
        let mut w = watcher_slots();
        let mut h = WatcherHost { master_obj: 0x44, master_dead: true, ..Default::default() };
        assert_eq!(slave_watcher_behavior_update(&mut w, &[7; 32], &mut h), 0x3fff_ffff);
        assert_eq!(h.log, ["find77", "st44", "resb0c", "resb08", "eea"]);
    }

    #[test]
    fn watcher_block_copy_requires_byte_and_predicate() {
        let src: [u32; 32] = std::array::from_fn(|i| i as u32 + 1);
        // byte10 false -> predicate not evaluated, no copy.
        let mut w = watcher_slots();
        let mut h = WatcherHost { master_obj: 0x44, ..Default::default() };
        assert_eq!(slave_watcher_behavior_update(&mut w, &src, &mut h), 1);
        assert!(!h.log.iter().any(|e| e == "pred"));
        assert_eq!(w.block_14, [0; 32]);
        // byte10 true, predicate false -> no copy/notify.
        let mut w = watcher_slots();
        let mut h = WatcherHost { master_obj: 0x44, byte10: true, ..Default::default() };
        assert_eq!(slave_watcher_behavior_update(&mut w, &src, &mut h), 1);
        assert_eq!(w.block_14, [0; 32]);
        // predicate true -> full 0x20-dword copy then touch + notify.
        let mut w = watcher_slots();
        let mut h = WatcherHost { master_obj: 0x44, byte10: true, predicate: true, ..Default::default() };
        assert_eq!(slave_watcher_behavior_update(&mut w, &src, &mut h), 1);
        assert_eq!(w.block_14, src);
        assert_eq!(h.log, ["find77", "st44", "b10b00", "pred", "c557", "eea"]);
    }

    // -- row 7: FireWeaponWhenDamagedBehavior::onDamage ----------------------

    #[derive(Default)]
    struct FireHost {
        log: Vec<&'static str>,
        gate: bool,
        flags: u32,
        threshold: f32,
        status: i32,
        fired: i32,
    }
    impl FireWeaponDamagedHost for FireHost {
        fn vobj_virtual_000(&mut self, _v: i32) -> bool { self.log.push("v0"); self.gate }
        fn flags_11c(&mut self, _c: i32) -> u32 { self.log.push("f11c"); self.flags }
        fn threshold_120(&mut self, _c: i32) -> f32 { self.log.push("f120"); self.threshold }
        fn weapon_status_0254_020(&mut self, _o: i32) -> i32 { self.log.push("wst"); self.status }
        fn fired_weapon_006ccfc7(&mut self) -> i32 { self.log.push("cfc7"); self.fired }
        fn fire_at_self_006ce76f(&mut self, owner: i32, pos: i32) {
            assert_eq!(pos, owner.wrapping_add(0x38));
            self.log.push("fire38");
        }
    }

    fn fw_slots() -> FireWeaponWhenDamagedSlots {
        FireWeaponWhenDamagedSlots {
            vobj_m08: 1,
            carrier_m24: 2,
            owner_m20: 0x5000,
            weapon_04: 0x11,
            weapon_08: 0x22,
            weapon_0c: 0x33,
            weapon_10: 0x44,
        }
    }

    #[test]
    fn fire_gate_and_kind_bit_quirks() {
        // gate false -> nothing after v0.
        let mut h = FireHost::default();
        fire_weapon_when_damaged_behavior_on_damage(&fw_slots(), &DamageInfo::default(), &mut h);
        assert_eq!(h.log, ["v0"]);

        // kind 1 -> bit 0; kind 33 wraps to bit 0 (char truncation); kind 0
        // -> bit 31. kind 2 with bit-0-only flags must stop before f120.
        let mut h = FireHost { gate: true, flags: 1, threshold: -1.0, fired: 1, ..Default::default() };
        fire_weapon_when_damaged_behavior_on_damage(&fw_slots(), &DamageInfo { kind_10: 1, amount_70: 5.0 }, &mut h);
        assert!(!h.log.contains(&"fire38"), "nonzero fired-weapon must not fire the fallback");
        let mut h = FireHost { gate: true, flags: 1, threshold: -1.0, fired: 9, ..Default::default() };
        fire_weapon_when_damaged_behavior_on_damage(&fw_slots(), &DamageInfo { kind_10: 33, amount_70: 5.0 }, &mut h);
        assert!(h.log.contains(&"cfc7"), "kind 33 must map to bit 0");
        let mut h = FireHost { gate: true, flags: 0x8000_0000, threshold: -1.0, fired: 9, ..Default::default() };
        fire_weapon_when_damaged_behavior_on_damage(&fw_slots(), &DamageInfo { kind_10: 0, amount_70: 5.0 }, &mut h);
        assert!(h.log.contains(&"cfc7"), "kind 0 must map to bit 31");
        let mut h = FireHost { gate: true, flags: 1, ..Default::default() };
        fire_weapon_when_damaged_behavior_on_damage(&fw_slots(), &DamageInfo { kind_10: 2, amount_70: 5.0 }, &mut h);
        assert_eq!(h.log, ["v0", "f11c"], "non-matching kind stops before the threshold load");
    }

    #[test]
    fn fire_threshold_nan_and_status_slot_arms() {
        let nan = f32::NAN;
        // threshold == amount passes; NaN on either side fails the
        // `th < amt || th == amt` guard.
        let mut h = FireHost { gate: true, flags: 1, threshold: 5.0, fired: 9, ..Default::default() };
        fire_weapon_when_damaged_behavior_on_damage(&fw_slots(), &DamageInfo { kind_10: 1, amount_70: 5.0 }, &mut h);
        assert!(h.log.contains(&"cfc7"));
        let mut h = FireHost { gate: true, flags: 1, threshold: nan, fired: 9, ..Default::default() };
        fire_weapon_when_damaged_behavior_on_damage(&fw_slots(), &DamageInfo { kind_10: 1, amount_70: 5.0 }, &mut h);
        assert!(!h.log.contains(&"wst"), "NaN threshold must stop the handler");
        let mut h = FireHost { gate: true, flags: 1, threshold: 5.0, fired: 9, ..Default::default() };
        fire_weapon_when_damaged_behavior_on_damage(&fw_slots(), &DamageInfo { kind_10: 1, amount_70: nan }, &mut h);
        assert!(!h.log.contains(&"wst"), "NaN amount must stop the handler");

        // status arms select slots; a null slot returns before cfc7.
        for (status, nonzero) in [(3i32, 0x44), (2, 0x33), (1, 0x22), (0, 0x11), (7, 0x11)] {
            let mut slots = fw_slots();
            let mut h = FireHost { gate: true, flags: 1, threshold: -1.0, status, fired: 9, ..Default::default() };
            fire_weapon_when_damaged_behavior_on_damage(&slots, &DamageInfo { kind_10: 1, amount_70: 5.0 }, &mut h);
            assert!(h.log.contains(&"cfc7"), "status {status} must use a non-null slot");
            match status {
                3 => slots.weapon_10 = 0,
                2 => slots.weapon_0c = 0,
                1 => slots.weapon_08 = 0,
                _ => slots.weapon_04 = 0,
            }
            let mut h = FireHost { gate: true, flags: 1, threshold: -1.0, status, fired: 9, ..Default::default() };
            fire_weapon_when_damaged_behavior_on_damage(&slots, &DamageInfo { kind_10: 1, amount_70: 5.0 }, &mut h);
            assert!(!h.log.contains(&"cfc7"), "status {status} null slot returns early");
            let _ = nonzero;
        }

        // fired_weapon == 0 -> fire at owner + 0x38.
        let mut h = FireHost { gate: true, flags: 1, threshold: -1.0, status: 3, fired: 0, ..Default::default() };
        fire_weapon_when_damaged_behavior_on_damage(&fw_slots(), &DamageInfo { kind_10: 1, amount_70: 5.0 }, &mut h);
        assert!(h.log.contains(&"fire38"));
    }

    // -- row 8: adjacent-site broadcast --------------------------------------

    #[derive(Default)]
    struct BcastHost {
        log: Vec<String>,
        objs: Vec<(i32, i32)>, // id -> object handle
        status_set: Vec<i32>,  // object handles with +0x438 bit0 set
        gate: bool,
        neighbor: i32,
    }
    impl BroadcastToSitesHost for BcastHost {
        fn owner_template_004(&mut self, o: i32) -> i32 { self.log.push(format!("otmpl{o:x}")); o + 4 }
        fn owner_id_074(&mut self, o: i32) -> i32 { self.log.push(format!("oid{o:x}")); 10 }
        fn status_bit_438_0(&mut self, o: i32) -> bool {
            self.log.push(format!("st{o:x}"));
            self.status_set.contains(&o)
        }
        fn self_virtual_010(&mut self, v: i32) { self.log.push(format!("self:{v}")); }
        fn find_object_00449dc5(&mut self, id: i32) -> i32 {
            self.log.push(format!("find{id}"));
            self.objs.iter().find(|(i, _)| *i == id).map_or(0, |(_, o)| *o)
        }
        fn template_gate_0073bb04(&mut self, t: i32) -> bool {
            self.log.push(format!("gate{t:x}"));
            self.gate
        }
        fn neighbor_or_nil_0068bd17(&mut self) -> i32 { self.log.push("nb".into()); self.neighbor }
        fn neighbor_virtual_010(&mut self, n: i32, v: i32) { self.log.push(format!("nbv{n:x}:{v}")); }
    }

    #[test]
    fn bcast_reads_owner_fields_before_the_status_gate() {
        let mut h = BcastHost::default(); // status_set empty -> owner fails
        getting_built_behavior_broadcast_to_adjacent_sites(
            &GettingBuiltSiteSlots { owner_m18: 0x5000 }, 9, &mut h,
        );
        assert_eq!(h.log, ["otmpl5000", "oid5000", "st5000"]);
    }

    #[test]
    fn bcast_walks_up_then_down_and_breaks_on_first_failure() {
        let mut h = BcastHost {
            objs: vec![(11, 111), (12, 112), (13, 113), (9, 119), (8, 118)],
            status_set: vec![0x5000, 111, 112, 119], // 113, 118 fail
            gate: true,
            neighbor: 0xEE,
            ..Default::default()
        };
        getting_built_behavior_broadcast_to_adjacent_sites(
            &GettingBuiltSiteSlots { owner_m18: 0x5000 }, 7, &mut h,
        );
        let notifies = h.log.iter().filter(|e| **e == "nbvee:7").count();
        assert_eq!(notifies, 3, "up: 11, 12 (113 fails) + down: 9 (8 missing-object? no: 118 fails)");
        // order: self virtual precedes the walks; the upward walk completes
        // (breaks at 13) before the downward walk starts at 9.
        let self_pos = h.log.iter().position(|e| e == "self:7").unwrap();
        let up13 = h.log.iter().position(|e| e == "find13").unwrap();
        let down9 = h.log.iter().position(|e| e == "find9").unwrap();
        assert!(self_pos < up13 && up13 < down9);
    }

    #[test]
    fn bcast_gate_failure_breaks_and_nil_neighbor_skips() {
        // template gate false breaks the walk after the first find; the
        // object's status must not be queried.
        let mut h = BcastHost {
            objs: vec![(11, 111), (9, 119)],
            status_set: vec![0x5000, 111, 119],
            gate: false,
            neighbor: 0xEE,
            ..Default::default()
        };
        getting_built_behavior_broadcast_to_adjacent_sites(
            &GettingBuiltSiteSlots { owner_m18: 0x5000 }, 1, &mut h,
        );
        assert_eq!(h.log.iter().filter(|e| **e == "nbvee:1").count(), 0);
        // nil neighbor: qualifying objects found but no virtual fires.
        let mut h = BcastHost {
            objs: vec![(11, 111), (9, 119)],
            status_set: vec![0x5000, 111, 119],
            gate: true,
            neighbor: 0,
            ..Default::default()
        };
        getting_built_behavior_broadcast_to_adjacent_sites(
            &GettingBuiltSiteSlots { owner_m18: 0x5000 }, 1, &mut h,
        );
        assert_eq!(h.log.iter().filter(|e| e.starts_with("nbv")).count(), 0);
        assert_eq!(h.log.iter().filter(|e| **e == "nb").count(), 2);
    }
}
