//! ScriptEngine band — 8 rows (module `Script`, file `scriptw47_1`).
//!
//! The band spans three ScriptActions rows (the located-position action,
//! the give-upgrade special-ability action, and the team-health-percent
//! action), the Lua-script-engine `Events` element parser, two Lua glue
//! functions (`ObjectTestCanSufferFear`, `ObjectDoSpecialPower`), the
//! Condition save-chunk writer, and the `AttackPriorityInfo` combat-chain
//! priority modifier.
//!
//! | addr | size | their symbol | role |
//! |---|---|---|---|
//! | `0x007c7a30` | 195 | `ScriptActions::Rva003C7A30Do(Parameter*, int)` | resolve param object + object-by-id, copy the located object's position, module-gated pred/action with x/y stores at +0x31c/+0x320 |
//! | `0x007371f4` | 195 | `LuaScriptEngine::ProcessEventsElement(BfmeLexByteArray*)` | `Events` gate then a per-element dispatch loop over `InternalEvent`/`ScriptedEvent`/`ModelConditionEvent`/`ObjectStatusEvent` |
//! | `0x007c49ae` | 194 | `ScriptActions::rva003C49AE(Parameter*, Parameter*)` | two named lookups, the `SpecialAbilityGiveUpgrade` update (power type `0x33`) at `+0x284`, template find + give-upgrade call |
//! | `0x007c301d` | 188 | `ScriptActions::Rva003C301DDo(Parameter*, Parameter*)` (`ScriptActionsTeamHealthPercent.cpp`) | clamp the percent float at `+0xc` into `[0, K]` and drive every team member's Body-module virtuals |
//! | `0x0073508f` | 187 | `ObjectTestCanSufferFear(lua_State*)` | Lua query: object/number gate, difficulty-vs-counter and module/state vetoes, pushboolean, return 1 |
//! | `0x007b428c` | 183 | `Condition::WriteConditionDataChunk(DataChunkOutput&, Condition*)` | emit one `Condition` chunk per node: type, def name (`Bogus` fallback), aux count + do-while aux writes, two flag bytes |
//! | `0x007352f7` | 178 | `ObjectDoSpecialPower(lua_State*)` | Lua command: >1-arg + engine-global gate, named power template, module find, can-use-gated trigger virtual, return 1 |
//! | `0x00757025` | 172 | `AttackPriorityInfo::doModPriorityByCombatChain(unsigned, Object const*, Object const*)` | `-1` template-field pass-through, sentinel fallback constant, same-owner bonus, chain/divisor + priority, x87 → unsigned store |
//!
//! Faithful-model conventions for this module:
//! - Every foreign interaction (`FUN_xxx` callees, the `DAT_00dfec50`
//!   vtable slot, the Lua helpers, the DataChunkOutput writes, Body-module
//!   virtual slots) is a host-trait method; the mirror keeps the guard
//!   order, short-circuits, loop shapes and plain-field stores exactly as
//!   the decompiled body runs them. Handles are raw `i32` like the sibling
//!   wave modules; offset reads that the body performs directly
//!   (`+600`/`+0x258`, `+0x284`, `+0x3c`, `+0x4c` …) stay visible in the
//!   mirror through their method names.
//! - The two `+600` (`0x258`) module-presence guards (rows `0x007c7a30`
//!   and `0x0073508f`) are the same object field; the `0x007c301d`
//!   iterator's "float" `1.1405045e-38` is the decompiler reading the
//!   predicate-callback address `0x007C30A0` (bits `0x007C30A0`) as a
//!   float — the filter function pointer for the team walk.
//! - Float quirks pinned by tests: the `0x007c301d` clamp is the literal
//!   fmin/fmax pointer-selection (a NaN percent lands on `0.0`, `-0.0`
//!   survives); the `0x0073508f` counter compare casts the int to `f32`
//!   (`(float)iVar3 <= local_8`); `0x00757025` runs its divide/add in
//!   extended precision (modelled in `f64`) and stores the result through
//!   the x87 unsigned pattern (default nearest-even rounding, bias path
//!   at 2^31, indefinite integer value `0x80000000` for NaN/out-of-range).
//! - `0x007371f4` *returns* (not continues) on an unknown element token —
//!   the whole rest of the list is skipped.
//! - `0x007b428c` re-reads the aux count every iteration (the do-while
//!   test `param_2 < *(iVar1 + 8)`), so an aux writer that bumps the
//!   count extends the loop; the counter reuses the `param_2` register.

/// A named script `Parameter`: the object/team id string sits at `+0x10`
/// (`Parameter::getString` slot; the body copy-constructs a temp from it).
#[derive(Clone, Debug, Default)]
pub struct NamedParameter {
    pub id_string_10: String,
}

/// A numeric script `Parameter`: the float payload sits at `+0xc`
/// (`*(float *)(param_2 + 0xc)` in row `0x007c301d`).
#[derive(Clone, Copy, Debug, Default)]
pub struct NumericParameter {
    pub value_0c: f32,
}

/// The three-dword position record the `0x007c7a30` body builds from the
/// located object (`+0xc`/`+0x10`/`+0x14`): locals `local_18/-14/-10`,
/// handed to callees *by address* (so they may mutate it before the
/// stores read it back).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PositionRecord {
    pub x_0c: f32,
    pub y_10: f32,
    pub z_14: f32,
}

/// Host interactions for `0x007c7a30`.
pub trait SetPositionActionHost {
    /// `FUN_007588e7` — resolve the first `Parameter` to an object
    /// (`0` models null).
    fn object_from_parameter_007588e7(&mut self, param: i32) -> i32;
    /// `(**(code **)(*DAT_00dfec50 + 0x88))(id)` — vtable slot `+0x88`
    /// (vindex 34) on the `DAT_00dfec50` singleton: find object by id.
    fn find_object_by_id_dfec50_88(&mut self, id: i32) -> i32;
    /// Read `+0xc/+0x10/+0x14` (the position record) off the located
    /// object; the body runs this before the module guard.
    fn read_position_0c_14(&mut self, object: i32) -> PositionRecord;
    /// `*(int *)(iVar2 + 600)` — the `+0x258` module-presence slot.
    fn module_slot_0258(&mut self, object: i32) -> i32;
    /// `FUN_0066295e()` — side-effect helper, no modelled args.
    fn helper_0066295e(&mut self);
    /// `FUN_0068c01f()` — side-effect helper, no modelled args.
    fn helper_0068c01f(&mut self);
    /// `FUN_0068ecdb(&local_18)` — predicate over the position record
    /// (passed by address; may clamp it in place).
    fn pred_0068ecdb(&mut self, pos: &mut PositionRecord) -> bool;
    /// `FUN_0066c26d(&local_18, 1)` — position action (failure branch).
    fn action_0066c26d(&mut self, pos: &mut PositionRecord);
    /// Store the record's x into `target + 0x31c` and y into
    /// `target + 800` (i.e. `+0x320`) — **x and y only, z is not stored**.
    fn store_xy_031c_0320(&mut self, object: i32, x: f32, y: f32);
    /// `FUN_0076ea01(&local_18, 1)` — position action (success branch).
    fn action_0076ea01(&mut self, pos: &mut PositionRecord);
}

/// `0x007c7a30` — `ScriptActions::Rva003C7A30Do(Parameter*, int)`.
///
/// Body order, verbatim: resolve the parameter object (`FUN_007588e7`)
/// and short-circuit if null; the id lookup through the `DAT_00dfec50`
/// vtable slot `+0x88` runs *only* when the first resolve succeeded, and
/// its failure aborts before any position read. The located object's
/// position triple is then copied into locals, and the whole action block
/// runs only when the target's `+0x258` module slot is non-null: two
/// side-effect helpers, then the predicate on the (possibly clamped)
/// record — false takes `FUN_0066c26d(&pos, 1)`, true stores `pos.x` at
/// `+0x31c` and `pos.y` at `+0x320` (the post-predicate values, x/y only)
/// and tail-calls `FUN_0076ea01(&pos, 1)`.
pub fn script_action_try_set_position_from_object(
    host: &mut dyn SetPositionActionHost,
    param: i32,
    id: i32,
) {
    let target = host.object_from_parameter_007588e7(param);
    if target == 0 {
        return;
    }
    let located = host.find_object_by_id_dfec50_88(id);
    if located == 0 {
        return;
    }
    let mut pos = host.read_position_0c_14(located);
    if host.module_slot_0258(target) != 0 {
        host.helper_0066295e();
        host.helper_0068c01f();
        if !host.pred_0068ecdb(&mut pos) {
            host.action_0066c26d(&mut pos);
        } else {
            // `local_c = local_18` / `local_8 = local_14` are spill moves;
            // the stores read the post-predicate values.
            host.store_xy_031c_0320(target, pos.x_0c, pos.y_10);
            host.action_0076ea01(&mut pos);
        }
    }
}

/// `0x007371f4` — `LuaScriptEngine::ProcessEventsElement(BfmeLexByteArray*)`.
///
/// The first lexer token is consumed unconditionally; unless it is
/// exactly `"Events"` (the `strcmp == 0` test) the function returns
/// without touching the element scanner. Otherwise one `FUN_00942890`
/// element-state read seeds the loop; the loop condition is the literal
/// `state != 0 && state == 1` (a state of, say, `2` ends the loop rather
/// than entering). Each pass re-reads the token and dispatches against
/// the four event element names; an **unknown token `return`s from the
/// whole function** — the remaining list is dropped, and the trailing
/// element read is skipped. Handler names carry their `FUN` rows.
pub fn lua_script_engine_process_events_element(
    mut next_token: impl FnMut() -> String, // FUN_0070f2c7
    mut next_element: impl FnMut() -> i32,  // FUN_00942890
    mut internal_event: impl FnMut(),       // FUN_00732c7a
    mut scripted_event: impl FnMut(),       // FUN_00736be4
    mut model_condition_event: impl FnMut(), // FUN_00736c5d
    mut object_status_event: impl FnMut(),  // FUN_00736eb0
) {
    let token = next_token();
    if token != "Events" {
        return;
    }
    let mut state = next_element();
    while state != 0 && state == 1 {
        let token = next_token();
        if token == "InternalEvent" {
            internal_event();
        } else if token == "ScriptedEvent" {
            scripted_event();
        } else if token == "ModelConditionEvent" {
            model_condition_event();
        } else if token == "ObjectStatusEvent" {
            object_status_event();
        } else {
            return;
        }
        state = next_element();
    }
}

/// Host interactions for `0x007c49ae`.
pub trait GiveUpgradeHost {
    /// `FUN_007584e9` — resolve a `Parameter` id string to a
    /// thing/object handle (`0` models null).
    fn thing_from_parameter_007584e9(&mut self, id_string: &str) -> i32;
    /// `FUN_0079e968` — find the give-upgrade special-ability update;
    /// the body stages the power type `0x33` (`SpecialPowerType`
    /// GIVE_UPGRADE, confirmed against the reference's
    /// `findSpecialAbilityUpdate((SpecialPowerType)(0x33))`) into the
    /// arg slot before the call.
    fn find_give_upgrade_update_0079e968(
        &mut self,
        target: i32,
        other: i32,
        special_power_type: i32,
    ) -> i32;
    /// `FUN_0066f0f0` — object behind the update member at
    /// `update + 0x284` (`member @ +0x284`).
    fn object_via_member_0284_0066f0f0(&mut self, member: i32) -> i32;
    /// `FUN_0079e8ff()` — receive-permission predicate on the object.
    fn can_receive_0079e8ff(&mut self, object: i32) -> bool;
    /// `FUN_0069b6eb` — `TheSpecialPowerStore->findSpecialPowerTemplate`
    /// for the staged name string (`0` models null).
    fn find_template_0069b6eb(&mut self, name: &str) -> i32;
    /// `FUN_0079e8eb()` — the upgrade argument pulled after the template
    /// find succeeds.
    fn upgrade_argument_0079e8eb(&mut self) -> i32;
    /// `FUN_0068e01f(template, arg)` — the give-upgrade application.
    fn give_upgrade_0068e01f(&mut self, template: i32, argument: i32);
}

/// `0x007c49ae` — `ScriptActions::rva003C49AE(Parameter*, Parameter*)`,
/// the give-upgrade special-ability action (body stages the template
/// name `"SpecialAbilityGiveUpgrade"`, and the reference tree confirms
/// the update lookup runs with power type `0x33`).
///
/// Both parameter lookups *always* run (the temporaries are built and
/// resolved in sequence; the `iVar2 != 0 && iVar3 != 0` test comes after
/// both). Then update → member `+0x284` (pointer add wraps) → object →
/// predicate → template find; only a non-null template reaches the
/// give-upgrade call, and the upgrade-argument read happens *inside*
/// that guard.
pub fn special_ability_give_upgrade(
    host: &mut dyn GiveUpgradeHost,
    param1: &NamedParameter,
    param2: &NamedParameter,
) {
    let a = host.thing_from_parameter_007584e9(&param1.id_string_10);
    let b = host.thing_from_parameter_007584e9(&param2.id_string_10);
    if a == 0 || b == 0 {
        return;
    }
    let update = host.find_give_upgrade_update_0079e968(a, b, 0x33);
    if update == 0 {
        return;
    }
    let member = update.wrapping_add(0x284); // update::member @ +0x284
    let object = host.object_via_member_0284_0066f0f0(member);
    if object == 0 {
        return;
    }
    if !host.can_receive_0079e8ff(object) {
        return;
    }
    let template = host.find_template_0069b6eb("SpecialAbilityGiveUpgrade");
    if template != 0 {
        let argument = host.upgrade_argument_0079e8eb();
        host.give_upgrade_0068e01f(template, argument);
    }
}

/// Host interactions for `0x007c301d`.
pub trait TeamHealthPercentHost {
    /// `FUN_007584e9` — resolve the team id string (`0` models null).
    fn team_from_parameter_007584e9(&mut self, id_string: &str) -> i32;
    /// The team walk's per-member Body slot read:
    /// `piVar1 = *(int **)(member + 0x254)` (`body @ +0x254`); `0`
    /// models null.
    fn body_module_at_0254(&mut self, member: i32) -> i32;
    /// Body-module vtable `+0x54` (vindex 21), called with the member
    /// context and the clamped percent (args elided by the
    /// decompiler — shape taken from the `undefined1 *local_8` /
    /// `local_2c = param_1` staging in the body).
    fn body_virt_0x54(&mut self, body: i32, member: i32, percent: f32);
    /// Body-module vtable `+0x5c` (vindex 23), same member context.
    fn body_virt_0x5c(&mut self, body: i32, member: i32);
}

/// `0x007c301d` — `ScriptActions::Rva003C301DDo(Parameter*, Parameter*)`
/// (`ScriptActionsTeamHealthPercent.cpp`): set a team's health to a
/// clamped percent, per member.
///
/// The team lookup guard wraps *everything*, the clamp included, so a
/// failed lookup applies nothing (`None`). The clamp reproduces the
/// body's literal pointer selection: first
/// `x = (K <= v) ? K : v` — note `K <= NaN` is false, so a NaN percent
/// passes through as NaN — then `y = (0.0 <= x) ? x : 0.0`, which maps
/// NaN down to `+0.0` while `0.0 <= -0.0` keeps a clamped `-0.0`. `K` is
/// the image constant `_DAT_00bc292c` (value not recoverable from the
/// decompilation; it is a parameter).
///
/// `members` is the walk order the `FUN_00663864` iterator
/// (`FUN_00663526` advance) produces for the team; the iterator's
/// predicate is the callback at `0x007C30A0` the body stages as the
/// "float" `1.1405045e-38` (bits `0x007C30A0` — an address, not a
/// number). Members whose `+0x254` Body slot is null are skipped; each
/// surviving member takes the two Body virtuals with the *clamped*
/// value.
pub fn script_action_team_health_percent(
    host: &mut dyn TeamHealthPercentHost,
    team_param: &NamedParameter,
    percent_param: &NumericParameter,
    k_dat_00bc292c: f32,
    members: &[i32],
) -> Option<f32> {
    let team = host.team_from_parameter_007584e9(&team_param.id_string_10);
    if team == 0 {
        return None;
    }
    let v = percent_param.value_0c;
    // min by pointer select: `if (K <= v) p = &K else p = &v`
    let lo = if k_dat_00bc292c <= v { k_dat_00bc292c } else { v };
    // max by pointer select: `if (0.0 <= lo) p = &lo else p = &0.0`
    let percent = if 0.0f32 <= lo { lo } else { 0.0f32 };
    for &member in members {
        let body = host.body_module_at_0254(member);
        if body != 0 {
            host.body_virt_0x54(body, member, percent);
            host.body_virt_0x5c(body, member);
        }
    }
    Some(percent)
}

/// Host interactions for `0x0073508f` (`ObjectTestCanSufferFear`).
pub trait LuaFearHost {
    /// `FUN_00b47190(L, 1)` — the Lua object/userdata read for stack
    /// slot 1 (`0` models null).
    fn to_object_00b47190(&mut self) -> i32;
    /// `FUN_00b470a0(L, 1)` — the Lua type of slot 1; `1` is the
    /// number-like acceptance type.
    fn argument_type_00b470a0(&mut self) -> i32;
    /// `FUN_00b47510(L)` — the reject push (nil-style).
    fn reject_00b47510(&mut self);
    /// `FUN_00449dc5(handle)` — cast/validate to `Object` (`0` fails;
    /// the body feeds it the null handle on the number-type path).
    fn cast_object_00449dc5(&mut self, handle: i32) -> i32;
    /// `FUN_0068c149(4, &local_8, 0)` — the difficulty-level-4 float
    /// getter; fills `value` and reports whether one exists.
    fn difficulty_0068c149(&mut self, value: &mut f32) -> bool;
    /// `FUN_00633ff4(0, 1, s_C__projects_bfme2patch103_bfme2__00c0de00,
    /// 0x85a)` — the source-tagged counter read (constant args elided).
    fn counter_00633ff4(&mut self) -> i32;
    /// `*(int *)(iVar2 + 600)` — the object's `+0x258` module slot.
    fn module_slot_0258(&mut self, object: i32) -> i32;
    /// `FUN_00662fc3()` — the state/condition query the `0x2d` veto
    /// compares against.
    fn state_00662fc3(&mut self) -> i32;
    /// `FUN_00b47540(L, v)` — push a Lua boolean.
    fn push_boolean_00b47540(&mut self, value: i32);
}

/// `0x0073508f` — `ObjectTestCanSufferFear(lua_State*)`.
///
/// Quirks pinned by tests:
/// - The reject branch needs *both* the failed object read and a
///   non-`1` type: a null read with type `1` falls through into the
///   common branch with the **null** handle, so `FUN_00449dc5` sees `0`
///   and the function takes the reject-push-but-return-1 path (no
///   boolean pushed).
/// - Inside the object branch the result starts at `1`; the difficulty
///   veto only runs when the getter returned (`local_8` starts at `0.0`
///   and the getter owns its fill), and its compare is `(float)counter
///   <= local_8` — the int→f32 cast is the body's, exact only inside
///   2^24.
/// - The `+0x258`/state veto is `module != 0 && state == 0x2d`.
/// - `FUN_00b47540` (pushboolean) runs **only** on the non-null-object
///   path; the function's return count is `1` everywhere except the
///   initial two-condition reject.
pub fn object_test_can_suffer_fear(host: &mut dyn LuaFearHost) -> i32 {
    let handle = host.to_object_00b47190();
    if handle == 0 && host.argument_type_00b470a0() != 1 {
        host.reject_00b47510();
        return 0;
    }
    let object = host.cast_object_00449dc5(handle);
    if object == 0 {
        host.reject_00b47510();
    } else {
        let mut result = 1i32;
        let mut local_8 = 0.0f32;
        if host.difficulty_0068c149(&mut local_8) {
            let counter = host.counter_00633ff4();
            if (counter as f32) <= local_8 {
                result = 0;
            }
        }
        if host.module_slot_0258(object) != 0 && host.state_00662fc3() == 0x2d {
            result = 0;
        }
        host.push_boolean_00b47540(result);
    }
    1
}

/// A saved condition node (row `0x007b428c` walks the list through the
/// `next` slot). Node indices live in a caller-owned slice; index `0` is
/// the null sentinel (the slice's slot 0 is a placeholder, matching the
/// game's `NULL` list end).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConditionNode {
    /// `+4` — the condition type id.
    pub condition_type_04: i32,
    /// `+8` — the auxiliary datum count; **re-read every do-while test**,
    /// so an aux writer may extend or shorten the loop.
    pub aux_count_08: i32,
    /// `+0x4c` — first saved flag byte (`*(undefined1 *)` read).
    pub flag_4c: u8,
    /// `+0x4d` — second saved flag byte.
    pub flag_4d: u8,
    /// `+0x3c` — `next_3c` link: index of the next node, `0` ends the
    /// walk.
    pub next_3c: usize,
}

/// Host interactions for `0x007b428c` (the `DataChunkOutput` sink plus
/// the condition-definition lookups).
pub trait ConditionChunkHost {
    /// `FUN_00707c76("Condition", 6)` — open a named data chunk.
    fn open_data_chunk_00707c76(&mut self, name: &str, id: u32);
    /// `FUN_00706cff(dword)` — write an int datum.
    fn write_int_00706cff(&mut self, value: i32);
    /// `FUN_00706cff(*(undefined1 *)…)` — write a byte datum.
    fn write_byte_00706cff(&mut self, value: u8);
    /// `FUN_00707d29(s)` — write a string datum.
    fn write_string_00707d29(&mut self, value: &str);
    /// `FUN_00706c88()` — close the data chunk.
    fn close_data_chunk_00706c88(&mut self);
    /// `FUN_00603941(type)` — the condition-definition lookup
    /// (`0` models null).
    fn condition_def_lookup_00603941(&mut self, condition_type: i32) -> i32;
    /// `def + 0x10` — the definition's name string.
    fn def_name_at_0010(&mut self, def: i32) -> String;
    /// `FUN_00548e1a("Bogus")` — the fallback name when no definition
    /// matches the type.
    fn bogus_name_00548e1a(&mut self, fallback: &str) -> String;
    /// `FUN_007b3a38(out)` — write one aux datum of this condition
    /// (it may mutate the node, including its `+8` count).
    fn write_aux_datum_007b3a38(&mut self, out: usize, node: &mut ConditionNode);
}

/// `0x007b428c` — `Condition::WriteConditionDataChunk(DataChunkOutput&,
/// Condition*)`: save every node of the list.
///
/// Per node, in body order: open the `"Condition"` chunk (id 6); write
/// the type int (`+4`); write the name — definition lookup, `def+0x10`
/// when found, else the `"Bogus"` fallback string; write the aux count
/// int (`+8`); when `0 < count`, a **do-while** writes `count` aux
/// datums (the counter was staged into the old `param_2` register — the
/// list-head argument clobber is invisible to the walk, which runs on
/// `iVar1` — and the count is re-read each test, so a writer that bumps
/// `+8` keeps the loop going). Then the two flag bytes (`+0x4c`,
/// `+0x4d`) and the chunk close; the walk follows `+0x3c`.
pub fn condition_write_data_chunk(
    host: &mut dyn ConditionChunkHost,
    nodes: &mut [ConditionNode],
    head: usize,
) {
    let mut i = head;
    while i != 0 {
        host.open_data_chunk_00707c76("Condition", 6);
        host.write_int_00706cff(nodes[i].condition_type_04);
        let def = host.condition_def_lookup_00603941(nodes[i].condition_type_04);
        let name = if def == 0 {
            host.bogus_name_00548e1a("Bogus")
        } else {
            host.def_name_at_0010(def)
        };
        host.write_string_00707d29(&name);
        host.write_int_00706cff(nodes[i].aux_count_08);
        if 0 < nodes[i].aux_count_08 {
            let mut count = 0i32;
            loop {
                host.write_aux_datum_007b3a38(i, &mut nodes[i]);
                count += 1;
                // do-while test re-reads `+8` — the writer may have moved it
                if !(count < nodes[i].aux_count_08) {
                    break;
                }
            }
        }
        host.write_byte_00706cff(nodes[i].flag_4c);
        host.write_byte_00706cff(nodes[i].flag_4d);
        host.close_data_chunk_00706c88();
        i = nodes[i].next_3c;
    }
}

/// Host interactions for `0x007352f7` (`ObjectDoSpecialPower`).
pub trait LuaSpecialPowerHost {
    /// `FUN_00b46f30()` — `lua_gettop`.
    fn gettop_00b46f30(&mut self) -> i32;
    /// `DAT_00dfe6e8 != 0` — the script-engine-active global gate.
    fn engine_global_dfe6e8(&mut self) -> i32;
    /// `FUN_00b47190(L)` — stack slot 1 as userdata (`0` models null).
    fn to_object_00b47190(&mut self) -> i32;
    /// `FUN_00b470a0(L)` — the Lua type of slot 1; `1` tolerates the
    /// null handle and continues.
    fn argument_type_00b470a0(&mut self) -> i32;
    /// `FUN_00449dc5()` — cast/validate to `Object` (`0` fails).
    fn cast_object_00449dc5(&mut self, handle: i32) -> i32;
    /// `FUN_00b473b0(L)` — the stack slot 2 string (the power name; the
    /// body copy-constructs a `StringBase` temp from it).
    fn to_string_00b473b0(&mut self) -> String;
    /// `FUN_0069b6eb` — the special-power-template find for that name
    /// (`0` models null; the result is a pure guard).
    fn find_power_template_0069b6eb(&mut self, name: &str) -> i32;
    /// `FUN_00688609()` — the object's power-store lookup.
    fn power_store_00688609(&mut self, object: i32) -> i32;
    /// `*(undefined4 *)(iVar2 + 0x1c)` — the `+0x1c` field read the
    /// body stages before the module find.
    fn read_field_001c(&mut self, store: i32) -> i32;
    /// `FUN_00690e22()` — the special-power-module find
    /// (`0` models null).
    fn find_power_module_00690e22(&mut self, field_001c: i32) -> i32;
    /// module vtable `+0x04` (vindex 1) — the can-use predicate.
    fn module_virt_0x04(&mut self, module: i32) -> bool;
    /// module vtable `+0x28` (vindex 10) — the trigger.
    fn module_virt_0x28(&mut self, module: i32);
}

/// `0x007352f7` — `ObjectDoSpecialPower(lua_State*)`.
///
/// The entry gate is `gettop > 1 && DAT_00dfe6e8 != 0` (short-circuit;
/// the global is read only when the arg count passes). A null slot-1
/// handle survives the object stage only with type `1` (then
/// `FUN_00449dc5` sees the null and the cast fails → `return 0`). The
/// template find result is a guard only; after it, the `+0x1c` read and
/// the module find run. **A found module returns `1` no matter what the
/// can-use virtual says** — the virtual only decides whether the
/// trigger virtual runs; the `iStack_14 = 0x73537a/0x735391` stores are
/// the compiler's SEH/cleanup state addresses, not data.
pub fn object_do_special_power(host: &mut dyn LuaSpecialPowerHost) -> i32 {
    let top = host.gettop_00b46f30();
    if !(top > 1 && host.engine_global_dfe6e8() != 0) {
        return 0;
    }
    let handle = host.to_object_00b47190();
    if handle == 0 {
        if host.argument_type_00b470a0() != 1 {
            return 0;
        }
    }
    let object = host.cast_object_00449dc5(handle);
    if object == 0 {
        return 0;
    }
    let name = host.to_string_00b473b0();
    let template = host.find_power_template_0069b6eb(&name);
    if template == 0 {
        return 0;
    }
    let store = host.power_store_00688609(object);
    let field_001c = host.read_field_001c(store);
    let module = host.find_power_module_00690e22(field_001c);
    if module == 0 {
        return 0;
    }
    if host.module_virt_0x04(module) {
        host.module_virt_0x28(module);
    }
    1
}

/// Host interactions for `0x00757025`.
pub trait CombatChainHost {
    /// The two-step read `*(int *)(*(int *)(object + 4) + 0x520)`: the
    /// object's `+4` template pointer, then the template's `+0x520`
    /// field (`-1` disables the modifier).
    fn template_field_0520(&mut self, object: i32) -> i32;
    /// `FUN_006a8a81(other)` — the second object's combat-chain value.
    fn combat_chain_006a8a81(&mut self, other: i32) -> f32;
    /// `FUN_0068afa9()` — the current-owner read (the body calls it
    /// twice when the first check passes).
    fn owner_0068afa9(&mut self) -> i32;
    /// `FUN_006a8ab1(owner)` — the owner's chain-membership test
    /// (nonzero = in a chain group).
    fn chain_group_006a8ab1(&mut self, owner: i32) -> i32;
    /// `FUN_006c6acb()` — the chain's owner, compared against the
    /// second `FUN_0068afa9` read.
    fn chain_owner_006c6acb(&mut self) -> i32;
    /// `*(float *)(*(int *)(DAT_00dff0f8 + 0x18) + 0x54)` — the config
    /// object (`DAT_00dff0f8 + 0x18`) float at `+0x54`.
    fn config_float_0054(&mut self) -> f32;
}

/// The float→unsigned return conversion the `0x00757025` body ends with:
/// x87 store semantics with the default control word (round to nearest,
/// ties to even), MSVC's unsigned shape — values ≥ 2^31 go through the
/// bias path (`fist(v − 2^31) + 0x80000000`), and NaN or any value
/// outside `[−2^31, 2^32)` lands on the integer-indefinite
/// `0x80000000`. Negative values wrap two's-complement like the raw
/// 32-bit store.
fn x87_store_u32(x: f64) -> u32 {
    const INDEFINITE: u32 = 0x8000_0000;
    if x.is_nan() || x >= 4294967296.0 {
        return INDEFINITE;
    }
    if x >= 2147483648.0 {
        return (x87_fistp_i32(x - 2147483648.0) as u32).wrapping_add(0x8000_0000);
    }
    if x < -2147483648.0 {
        return INDEFINITE;
    }
    x87_fistp_i32(x) as u32
}

/// x87 32-bit integer store (`fistp`), default rounding (nearest, ties
/// to even); out-of-range/NaN yields the indefinite value `0x80000000`.
fn x87_fistp_i32(x: f64) -> i32 {
    if x.is_nan() || x.is_infinite() || x < -2147483648.0 || x >= 2147483648.0 {
        return i32::MIN;
    }
    x.round_ties_even() as i64 as i32
}

/// `0x00757025` —
/// `AttackPriorityInfo::doModPriorityByCombatChain(unsigned, Object
/// const*, Object const*) const`.
///
/// Body order: the template-field `-1` test on the *first* object passes
/// the priority through untouched (the `+4`→`+0x520` two-step read).
/// Otherwise the second object's chain value (extended to `float10`,
/// narrowed straight back to `float` by `param_2 = (float)fVar4`) is
/// compared against the `DAT_00bbb9ac` sentinel: equality returns the
/// `FXParticleSystem::CategoryModuleInfo<0>::CATEGORY`-named global's
/// float value (the decompiler's misattribution; a plain float constant,
/// taken as a parameter). Otherwise the owner's chain-group test
/// (`FUN_006a8ab1`) re-reads the owner and compares it against the
/// chain's owner (`FUN_006c6acb`); equal adds the `DAT_00bc292c` bonus
/// to the float. The final value is `chain / config_float_0054 +
/// priority`, computed in extended precision (modelled in `f64`, which
/// carries every f32-exact intermediate) and stored through the x87
/// unsigned pattern ([`x87_store_u32`]).
pub fn attack_priority_info_do_mod_priority_by_combat_chain(
    host: &mut dyn CombatChainHost,
    priority: u32,
    object_a: i32,
    object_b: i32,
    category_fallback: f32,
    sentinel_dat_00bbb9ac: f32,
    bonus_dat_00bc292c: f32,
) -> u32 {
    if host.template_field_0520(object_a) == -1 {
        return priority;
    }
    let mut chain = host.combat_chain_006a8a81(object_b);
    if chain == sentinel_dat_00bbb9ac {
        return x87_store_u32(category_fallback as f64);
    }
    let owner = host.owner_0068afa9();
    if host.chain_group_006a8ab1(owner) != 0 {
        let owner2 = host.owner_0068afa9();
        if host.chain_owner_006c6acb() == owner2 {
            chain += bonus_dat_00bc292c;
        }
    }
    let divisor = host.config_float_0054() as f64;
    let result = chain as f64 / divisor + priority as f64;
    x87_store_u32(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- row 0x007c7a30 -------------------------------------------------

    struct PosHost {
        log: Vec<String>,
        target: i32,
        located: i32,
        module: i32,
        pred_result: bool,
        pred_clamps: bool,
    }

    impl SetPositionActionHost for PosHost {
        fn object_from_parameter_007588e7(&mut self, _p: i32) -> i32 {
            self.log.push("resolve".into());
            self.target
        }
        fn find_object_by_id_dfec50_88(&mut self, _id: i32) -> i32 {
            self.log.push("byid".into());
            self.located
        }
        fn read_position_0c_14(&mut self, _o: i32) -> PositionRecord {
            self.log.push("readpos".into());
            PositionRecord { x_0c: 1.5, y_10: -2.5, z_14: 100.0 }
        }
        fn module_slot_0258(&mut self, _o: i32) -> i32 {
            self.log.push("mod258".into());
            self.module
        }
        fn helper_0066295e(&mut self) {
            self.log.push("h6295e".into());
        }
        fn helper_0068c01f(&mut self) {
            self.log.push("h8c01f".into());
        }
        fn pred_0068ecdb(&mut self, pos: &mut PositionRecord) -> bool {
            self.log.push("pred".into());
            if self.pred_clamps {
                pos.x_0c = 9.0;
                pos.y_10 = 8.0;
            }
            self.pred_result
        }
        fn action_0066c26d(&mut self, _pos: &mut PositionRecord) {
            self.log.push("a6c26d".into());
        }
        fn store_xy_031c_0320(&mut self, _o: i32, x: f32, y: f32) {
            self.log.push(format!("store {x},{y}"));
        }
        fn action_0076ea01(&mut self, _pos: &mut PositionRecord) {
            self.log.push("a76ea01".into());
        }
    }

    fn pos_host() -> PosHost {
        PosHost {
            log: Vec::new(),
            target: 1,
            located: 2,
            module: 1,
            pred_result: true,
            pred_clamps: false,
        }
    }

    #[test]
    fn position_action_guards_short_circuit() {
        let mut h = pos_host();
        h.target = 0;
        script_action_try_set_position_from_object(&mut h, 0x10, 7);
        assert_eq!(h.log, vec!["resolve"]); // id lookup never reached
        let mut h = pos_host();
        h.located = 0;
        script_action_try_set_position_from_object(&mut h, 0x10, 7);
        assert_eq!(h.log, vec!["resolve", "byid"]); // no position read
        let mut h = pos_host();
        h.module = 0;
        script_action_try_set_position_from_object(&mut h, 0x10, 7);
        assert_eq!(h.log, vec!["resolve", "byid", "readpos", "mod258"]);
    }

    #[test]
    fn position_action_branches_and_stores_post_predicate_xy() {
        let mut h = pos_host();
        h.pred_result = false;
        script_action_try_set_position_from_object(&mut h, 0x10, 7);
        assert_eq!(
            h.log,
            vec![
                "resolve", "byid", "readpos", "mod258", "h6295e", "h8c01f", "pred", "a6c26d"
            ]
        );
        let mut h = pos_host();
        h.pred_clamps = true; // the predicate mutates the record by address
        script_action_try_set_position_from_object(&mut h, 0x10, 7);
        assert_eq!(h.log[6..], ["pred", "store 9,8", "a76ea01"]);
    }

    // ---- row 0x007371f4 --------------------------------------------------


    #[test]
    fn events_gate_rejects_non_events_token() {
        // "NotEvents": the element scanner must never run (states list
        // empty => any next_element call would return 0 anyway; verify by
        // token count: only the single gate token consumed).
        let tokens = vec!["NotEvents".to_string()];
        let mut it = tokens.into_iter();
        let mut element_calls = 0;
        lua_script_engine_process_events_element(
            || it.next().unwrap_or_default(),
            || { element_calls += 1; 1 },
            || {},
            || {},
            || {},
            || {},
        );
        assert_eq!(element_calls, 0);
        assert!(it.next().is_none());
    }

    #[test]
    fn events_dispatch_loop_and_states() {
        // gate "Events", two elements: internal then status; state 2 ends
        // the loop without an early return.
        let tokens: Vec<String> = ["Events", "InternalEvent", "ObjectStatusEvent"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let mut ti = tokens.iter();
        let calls: std::cell::RefCell<Vec<String>> = std::cell::RefCell::new(Vec::new());
        let states = [1i32, 1, 2, 1];
        let mut si = states.iter();
        lua_script_engine_process_events_element(
            || ti.next().cloned().unwrap_or_default(),
            || *si.next().unwrap_or(&0),
            || calls.borrow_mut().push("internal".into()),
            || calls.borrow_mut().push("scripted".into()),
            || calls.borrow_mut().push("model".into()),
            || calls.borrow_mut().push("status".into()),
        );
        assert_eq!(*calls.borrow(), vec!["internal", "status"]);
    }

    #[test]
    fn events_unknown_token_returns_from_whole_function() {
        let tokens: Vec<String> = [
            "Events",
            "ScriptedEvent",
            "Nope",
            "InternalEvent",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let mut ti = tokens.iter();
        let mut si = [1i32, 1, 1, 1].iter();
        let calls: std::cell::RefCell<Vec<String>> = std::cell::RefCell::new(Vec::new());
        let mut elements_after_unknown = 0;
        lua_script_engine_process_events_element(
            || ti.next().cloned().unwrap_or_default(),
            || {
                let v = *si.next().unwrap_or(&0);
                if si.len() == 0 {
                    elements_after_unknown += 1;
                }
                v
            },
            || calls.borrow_mut().push("internal".into()),
            || calls.borrow_mut().push("scripted".into()),
            || calls.borrow_mut().push("model".into()),
            || calls.borrow_mut().push("status".into()),
        );
        assert_eq!(*calls.borrow(), vec!["scripted"]); // "InternalEvent" never ran
        assert_eq!(elements_after_unknown, 0); // trailing scanner skipped
    }

    #[test]
    fn events_state_zero_skips_the_loop_body() {
        let tokens: Vec<String> = vec!["Events".to_string()];
        let mut ti = tokens.iter();
        let mut calls = 0;
        lua_script_engine_process_events_element(
            || ti.next().cloned().unwrap_or_default(),
            || 0,
            || calls += 1,
            || {},
            || {},
            || {},
        );
        assert_eq!(calls, 0);
    }

    // ---- row 0x007c49ae --------------------------------------------------

    struct GiveHost {
        log: Vec<String>,
        a: i32,
        b: i32,
        update: i32,
        object: i32,
        can: bool,
        template: i32,
    }

    impl GiveUpgradeHost for GiveHost {
        fn thing_from_parameter_007584e9(&mut self, s: &str) -> i32 {
            self.log.push(format!("thing {s}"));
            if self.log.len() == 1 {
                self.a
            } else {
                self.b
            }
        }
        fn find_give_upgrade_update_0079e968(
            &mut self,
            a: i32,
            b: i32,
            ty: i32,
        ) -> i32 {
            self.log.push(format!("update {a} {b} {ty}"));
            self.update
        }
        fn object_via_member_0284_0066f0f0(&mut self, member: i32) -> i32 {
            self.log.push(format!("obj {member}"));
            self.object
        }
        fn can_receive_0079e8ff(&mut self, o: i32) -> bool {
            self.log.push(format!("can {o}"));
            self.can
        }
        fn find_template_0069b6eb(&mut self, name: &str) -> i32 {
            self.log.push(format!("tmpl {name}"));
            self.template
        }
        fn upgrade_argument_0079e8eb(&mut self) -> i32 {
            self.log.push("arg".into());
            0x55
        }
        fn give_upgrade_0068e01f(&mut self, t: i32, arg: i32) {
            self.log.push(format!("give {t} {arg}"));
        }
    }

    fn give_host() -> GiveHost {
        GiveHost {
            log: Vec::new(),
            a: 1,
            b: 2,
            update: 0x300,
            object: 4,
            can: true,
            template: 9,
        }
    }

    fn named(s: &str) -> NamedParameter {
        NamedParameter { id_string_10: s.into() }
    }

    #[test]
    fn give_upgrade_runs_both_lookups_before_the_and_guard() {
        let mut h = give_host();
        h.b = 0;
        special_ability_give_upgrade(&mut h, &named("x"), &named("y"));
        assert_eq!(h.log, vec!["thing x", "thing y"]);
        let mut h = give_host();
        h.a = 0;
        h.b = 0;
        special_ability_give_upgrade(&mut h, &named("x"), &named("y"));
        assert_eq!(h.log, vec!["thing x", "thing y"]);
    }

    #[test]
    fn give_upgrade_stages_type_33_and_wrapping_member() {
        let mut h = give_host();
        h.update = -0x100i32; // wrapping_add must not panic
        special_ability_give_upgrade(&mut h, &named("u"), &named("v"));
        assert_eq!(h.log[2], "update 1 2 51"); // 0x33 staged as the type arg
        assert_eq!(h.log[3], format!("obj {}", (-0x100i32).wrapping_add(0x284)));
    }

    #[test]
    fn give_upgrade_full_path_and_template_guard() {
        let mut h = give_host();
        special_ability_give_upgrade(&mut h, &named("u"), &named("v"));
        assert_eq!(
            h.log,
            vec![
                "thing u",
                "thing v",
                "update 1 2 51",
                "obj 1412", // 0x300 + 0x284 = 0x584
                "can 4",
                "tmpl SpecialAbilityGiveUpgrade",
                "arg",
                "give 9 85",
            ]
        );
        let mut h = give_host();
        h.template = 0;
        special_ability_give_upgrade(&mut h, &named("u"), &named("v"));
        assert_eq!(h.log.last().unwrap(), "tmpl SpecialAbilityGiveUpgrade");
        let mut h = give_host();
        h.can = false;
        special_ability_give_upgrade(&mut h, &named("u"), &named("v"));
        assert_eq!(h.log.last().unwrap(), "can 4");
    }

    // ---- row 0x007c301d --------------------------------------------------

    struct HealthHost {
        log: Vec<String>,
        team: i32,
        bodies: Vec<i32>, // parallel to members: the +0x254 slot value
    }

    impl TeamHealthPercentHost for HealthHost {
        fn team_from_parameter_007584e9(&mut self, s: &str) -> i32 {
            self.log.push(format!("team {s}"));
            self.team
        }
        fn body_module_at_0254(&mut self, m: i32) -> i32 {
            let v = self.bodies[m as usize];
            self.log.push(format!("body {m}"));
            v
        }
        fn body_virt_0x54(&mut self, b: i32, m: i32, p: f32) {
            self.log.push(format!("v54 {b} {m} {p}"));
        }
        fn body_virt_0x5c(&mut self, b: i32, m: i32) {
            self.log.push(format!("v5c {b} {m}"));
        }
    }

    fn health_host(team: i32, bodies: &[i32]) -> HealthHost {
        HealthHost { log: Vec::new(), team, bodies: bodies.to_vec() }
    }

    #[test]
    fn team_health_percent_guard_skips_everything() {
        let mut h = health_host(0, &[1]);
        let p = NumericParameter { value_0c: 2.0 };
        assert_eq!(script_action_team_health_percent(&mut h, &named("t"), &p, 1.0, &[0]), None);
        assert_eq!(h.log, vec!["team t"]);
    }

    #[test]
    fn team_health_percent_clamp_is_literal_pointer_select() {
        let k = 1.0f32;
        // above max -> K
        let mut h = health_host(1, &[]);
        let p = NumericParameter { value_0c: 5.0 };
        assert_eq!(
            script_action_team_health_percent(&mut h, &named("t"), &p, k, &[]),
            Some(1.0)
        );
        // below zero -> 0.0
        let mut h = health_host(1, &[]);
        let p = NumericParameter { value_0c: -3.0 };
        assert_eq!(
            script_action_team_health_percent(&mut h, &named("t"), &p, k, &[]),
            Some(0.0)
        );
        // NaN: K <= NaN is false -> lo = NaN; 0.0 <= NaN is false -> 0.0
        let mut h = health_host(1, &[]);
        let p = NumericParameter { value_0c: f32::NAN };
        let got = script_action_team_health_percent(&mut h, &named("t"), &p, k, &[]);
        assert_eq!(got, Some(0.0f32));
        // -0.0 in range: 0.0 <= -0.0 is true -> -0.0 survives
        let mut h = health_host(1, &[]);
        let p = NumericParameter { value_0c: -0.0 };
        let got = script_action_team_health_percent(&mut h, &named("t"), &p, k, &[]);
        assert_eq!(got.unwrap().to_bits(), (-0.0f32).to_bits());
    }

    #[test]
    fn team_health_percent_skips_members_without_body() {
        let mut h = health_host(1, &[0, 7]);
        let p = NumericParameter { value_0c: 0.5 };
        let members = [0i32, 1];
        let got = script_action_team_health_percent(&mut h, &named("t"), &p, 1.0, &members);
        assert_eq!(got, Some(0.5));
        assert_eq!(h.log, vec!["team t", "body 0", "body 1", "v54 7 1 0.5", "v5c 7 1"]);
    }

    // ---- row 0x0073508f --------------------------------------------------

    struct FearHost {
        log: Vec<String>,
        handle: i32,
        ty: i32,
        cast: i32,
        difficulty: Option<f32>,
        counter: i32,
        module: i32,
        state: i32,
    }

    impl LuaFearHost for FearHost {
        fn to_object_00b47190(&mut self) -> i32 {
            self.log.push("toobj".into());
            self.handle
        }
        fn argument_type_00b470a0(&mut self) -> i32 {
            self.log.push("type".into());
            self.ty
        }
        fn reject_00b47510(&mut self) {
            self.log.push("reject".into());
        }
        fn cast_object_00449dc5(&mut self, h: i32) -> i32 {
            self.log.push(format!("cast {h}"));
            self.cast
        }
        fn difficulty_0068c149(&mut self, value: &mut f32) -> bool {
            self.log.push("diff".into());
            match self.difficulty {
                Some(v) => {
                    *value = v;
                    true
                }
                None => false,
            }
        }
        fn counter_00633ff4(&mut self) -> i32 {
            self.log.push("counter".into());
            self.counter
        }
        fn module_slot_0258(&mut self, _o: i32) -> i32 {
            self.log.push("mod258".into());
            self.module
        }
        fn state_00662fc3(&mut self) -> i32 {
            self.log.push("state".into());
            self.state
        }
        fn push_boolean_00b47540(&mut self, v: i32) {
            self.log.push(format!("push {v}"));
        }
    }

    fn fear_host() -> FearHost {
        FearHost {
            log: Vec::new(),
            handle: 5,
            ty: 7,
            cast: 6,
            difficulty: None,
            counter: 0,
            module: 0,
            state: 0,
        }
    }

    #[test]
    fn fear_rejects_only_when_null_and_type_not_1() {
        let mut h = fear_host();
        h.handle = 0;
        h.ty = 2;
        assert_eq!(object_test_can_suffer_fear(&mut h), 0);
        assert_eq!(h.log, vec!["toobj", "type", "reject"]);
        // null handle with type 1: falls through with the null into cast
        let mut h = fear_host();
        h.handle = 0;
        h.ty = 1;
        h.cast = 0;
        assert_eq!(object_test_can_suffer_fear(&mut h), 1);
        assert_eq!(h.log, vec!["toobj", "type", "cast 0", "reject"]);
        // non-null handle: the type query never runs
        let mut h = fear_host();
        assert_eq!(object_test_can_suffer_fear(&mut h), 1);
        assert!(!h.log.contains(&"type".to_string()));
    }

    #[test]
    fn fear_difficulty_and_state_vetoes() {
        // (float)counter <= difficulty -> false result
        let mut h = fear_host();
        h.difficulty = Some(10.0);
        h.counter = 10;
        assert_eq!(object_test_can_suffer_fear(&mut h), 1);
        assert_eq!(h.log.last().unwrap(), "push 0");
        // counter above difficulty keeps true
        let mut h = fear_host();
        h.difficulty = Some(10.0);
        h.counter = 11;
        object_test_can_suffer_fear(&mut h);
        assert_eq!(h.log.last().unwrap(), "push 1");
        // +0x258 set and state 0x2d vetoes
        let mut h = fear_host();
        h.module = 1;
        h.state = 0x2d;
        object_test_can_suffer_fear(&mut h);
        assert_eq!(h.log.last().unwrap(), "push 0");
        // state 0x2d without the module does not veto (short-circuit)
        let mut h = fear_host();
        h.module = 0;
        h.state = 0x2d;
        object_test_can_suffer_fear(&mut h);
        assert!(!h.log.contains(&"state".to_string()));
        assert_eq!(h.log.last().unwrap(), "push 1");
    }

    #[test]
    fn fear_null_object_path_pushes_nothing() {
        let mut h = fear_host();
        h.cast = 0;
        assert_eq!(object_test_can_suffer_fear(&mut h), 1);
        assert!(!h.log.iter().any(|l| l.starts_with("push")));
        assert!(!h.log.iter().any(|l| l.starts_with("diff")));
    }

    // ---- row 0x007b428c --------------------------------------------------

    struct ChunkHost {
        log: Vec<String>,
        defs: Vec<(i32, i32)>, // (condition_type, def handle); missing -> 0
        names: Vec<(i32, String)>,
        aux_extend_to: Option<i32>, // writer bumps +8 until this, then stops
    }

    impl ConditionChunkHost for ChunkHost {
        fn open_data_chunk_00707c76(&mut self, name: &str, id: u32) {
            self.log.push(format!("open {name} {id}"));
        }
        fn write_int_00706cff(&mut self, v: i32) {
            self.log.push(format!("int {v}"));
        }
        fn write_byte_00706cff(&mut self, v: u8) {
            self.log.push(format!("byte {v}"));
        }
        fn write_string_00707d29(&mut self, s: &str) {
            self.log.push(format!("str {s}"));
        }
        fn close_data_chunk_00706c88(&mut self) {
            self.log.push("close".into());
        }
        fn condition_def_lookup_00603941(&mut self, t: i32) -> i32 {
            self.defs.iter().find(|(k, _)| *k == t).map(|(_, v)| *v).unwrap_or(0)
        }
        fn def_name_at_0010(&mut self, def: i32) -> String {
            self.names
                .iter()
                .find(|(k, _)| *k == def)
                .map(|(_, v)| v.clone())
                .unwrap_or_default()
        }
        fn bogus_name_00548e1a(&mut self, s: &str) -> String {
            format!("bogus:{s}")
        }
        fn write_aux_datum_007b3a38(&mut self, node: usize, n: &mut ConditionNode) {
            self.log.push(format!("aux {node}"));
            if let Some(to) = self.aux_extend_to {
                if n.aux_count_08 < to {
                    n.aux_count_08 += 1; // the game-legal count grow the do-while re-reads
                }
            }
        }
    }

    fn chunk_host() -> ChunkHost {
        ChunkHost {
            log: Vec::new(),
            defs: vec![(10, 0x40)],
            names: vec![(0x40, "HEALING".to_string())],
            aux_extend_to: None,
        }
    }

    #[test]
    fn condition_chunk_walks_list_and_writes_every_field() {
        let mut nodes = vec![
            ConditionNode::default(),
            ConditionNode {
                condition_type_04: 10,
                aux_count_08: 2,
                flag_4c: 1,
                flag_4d: 0,
                next_3c: 2,
            },
            ConditionNode {
                condition_type_04: 99, // no definition -> Bogus
                aux_count_08: 0,
                flag_4c: 0,
                flag_4d: 1,
                next_3c: 0,
            },
        ];
        let mut h = chunk_host();
        condition_write_data_chunk(&mut h, &mut nodes, 1);
        assert_eq!(
            h.log,
            vec![
                "open Condition 6",
                "int 10",
                "str HEALING",
                "int 2",
                "aux 1",
                "aux 1",
                "byte 1",
                "byte 0",
                "close",
                "open Condition 6",
                "int 99",
                "str bogus:Bogus",
                "int 0",
                "byte 0",
                "byte 1",
                "close",
            ]
        );
    }

    #[test]
    fn condition_chunk_zero_count_skips_do_while_but_negative_never_runs() {
        // count == 1 -> exactly one aux (do-while runs at least once).
        let mut nodes = vec![
            ConditionNode::default(),
            ConditionNode { condition_type_04: 10, aux_count_08: 1, ..ConditionNode::default() },
        ];
        let mut h = chunk_host();
        condition_write_data_chunk(&mut h, &mut nodes, 1);
        assert_eq!(h.log.iter().filter(|l| l.starts_with("aux")).count(), 1);
        // negative count -> the `0 < count` gate skips the loop entirely.
        let mut nodes = vec![
            ConditionNode::default(),
            ConditionNode { condition_type_04: 10, aux_count_08: -3, ..ConditionNode::default() },
        ];
        let mut h = chunk_host();
        condition_write_data_chunk(&mut h, &mut nodes, 1);
        assert!(!h.log.iter().any(|l| l.starts_with("aux")));
        assert!(h.log.iter().any(|l| l == "int -3"));
    }

    #[test]
    fn condition_chunk_do_while_re_reads_the_count_each_test() {
        let mut nodes = vec![
            ConditionNode::default(),
            ConditionNode { condition_type_04: 10, aux_count_08: 1, ..ConditionNode::default() },
        ];
        let mut h = chunk_host();
        h.aux_extend_to = Some(3); // writer extends the loop to three writes
        condition_write_data_chunk(&mut h, &mut nodes, 1);
        assert_eq!(h.log.iter().filter(|l| l.starts_with("aux")).count(), 3);
        assert_eq!(nodes[1].aux_count_08, 3);
    }

    #[test]
    fn condition_chunk_null_head_writes_nothing() {
        let mut nodes = vec![ConditionNode::default()];
        let mut h = chunk_host();
        condition_write_data_chunk(&mut h, &mut nodes, 0);
        assert!(h.log.is_empty());
    }

    // ---- row 0x007352f7 --------------------------------------------------

    struct PowerHost {
        log: Vec<String>,
        top: i32,
        engine: i32,
        handle: i32,
        ty: i32,
        object: i32,
        name: String,
        template: i32,
        module: i32,
        can: bool,
    }

    impl LuaSpecialPowerHost for PowerHost {
        fn gettop_00b46f30(&mut self) -> i32 {
            self.log.push("gettop".into());
            self.top
        }
        fn engine_global_dfe6e8(&mut self) -> i32 {
            self.log.push("engine".into());
            self.engine
        }
        fn to_object_00b47190(&mut self) -> i32 {
            self.log.push("toobj".into());
            self.handle
        }
        fn argument_type_00b470a0(&mut self) -> i32 {
            self.log.push("type".into());
            self.ty
        }
        fn cast_object_00449dc5(&mut self, h: i32) -> i32 {
            self.log.push(format!("cast {h}"));
            self.object
        }
        fn to_string_00b473b0(&mut self) -> String {
            self.log.push("tostr".into());
            self.name.clone()
        }
        fn find_power_template_0069b6eb(&mut self, n: &str) -> i32 {
            self.log.push(format!("tmpl {n}"));
            self.template
        }
        fn power_store_00688609(&mut self, o: i32) -> i32 {
            self.log.push(format!("store {o}"));
            0x900
        }
        fn read_field_001c(&mut self, s: i32) -> i32 {
            self.log.push(format!("f1c {s}"));
            0x1c
        }
        fn find_power_module_00690e22(&mut self, f: i32) -> i32 {
            self.log.push(format!("mod {f}"));
            self.module
        }
        fn module_virt_0x04(&mut self, m: i32) -> bool {
            self.log.push(format!("v04 {m}"));
            self.can
        }
        fn module_virt_0x28(&mut self, m: i32) {
            self.log.push(format!("v28 {m}"));
        }
    }

    fn power_host() -> PowerHost {
        PowerHost {
            log: Vec::new(),
            top: 3,
            engine: 1,
            handle: 5,
            ty: 0,
            object: 6,
            name: "SpecialPowerFireBlast".into(),
            template: 8,
            module: 0x77,
            can: true,
        }
    }

    #[test]
    fn power_entry_gate_short_circuits_the_global() {
        let mut h = power_host();
        h.top = 1;
        assert_eq!(object_do_special_power(&mut h), 0);
        assert_eq!(h.log, vec!["gettop"]); // engine global not read
        let mut h = power_host();
        h.engine = 0;
        assert_eq!(object_do_special_power(&mut h), 0);
        assert_eq!(h.log, vec!["gettop", "engine"]);
    }

    #[test]
    fn power_type_1_rescues_null_handle_but_the_cast_fails() {
        let mut h = power_host();
        h.handle = 0;
        h.ty = 1;
        h.object = 0;
        assert_eq!(object_do_special_power(&mut h), 0);
        assert_eq!(
            h.log,
            vec!["gettop", "engine", "toobj", "type", "cast 0"]
        );
        let mut h = power_host();
        h.handle = 0;
        h.ty = 2;
        assert_eq!(object_do_special_power(&mut h), 0);
        assert_eq!(h.log, vec!["gettop", "engine", "toobj", "type"]);
    }

    #[test]
    fn power_returns_1_with_module_even_when_can_use_is_false() {
        let mut h = power_host();
        h.can = false;
        assert_eq!(object_do_special_power(&mut h), 1);
        assert_eq!(h.log.last().unwrap(), "v04 119");
        assert!(!h.log.iter().any(|l| l.starts_with("v28")));
        let mut h = power_host();
        assert_eq!(object_do_special_power(&mut h), 1);
        assert_eq!(h.log.last().unwrap(), "v28 119");
    }

    #[test]
    fn power_template_and_module_guards() {
        let mut h = power_host();
        h.template = 0;
        assert_eq!(object_do_special_power(&mut h), 0);
        assert_eq!(h.log.last().unwrap(), "tmpl SpecialPowerFireBlast");
        let mut h = power_host();
        h.module = 0;
        assert_eq!(object_do_special_power(&mut h), 0);
        assert!(h.log.iter().any(|l| l == "f1c 2304")); // +0x1c read still runs
    }

    // ---- row 0x00757025 --------------------------------------------------

    struct ChainHost {
        log: Vec<String>,
        f520: i32,
        chain: f32,
        owners: Vec<i32>,
        group: i32,
        chain_owner: i32,
        divisor: f32,
    }

    impl CombatChainHost for ChainHost {
        fn template_field_0520(&mut self, o: i32) -> i32 {
            self.log.push(format!("f520 {o}"));
            self.f520
        }
        fn combat_chain_006a8a81(&mut self, o: i32) -> f32 {
            self.log.push(format!("chain {o}"));
            self.chain
        }
        fn owner_0068afa9(&mut self) -> i32 {
            self.log.push("owner".into());
            self.owners.remove(0)
        }
        fn chain_group_006a8ab1(&mut self, o: i32) -> i32 {
            self.log.push(format!("group {o}"));
            self.group
        }
        fn chain_owner_006c6acb(&mut self) -> i32 {
            self.log.push("chainowner".into());
            self.chain_owner
        }
        fn config_float_0054(&mut self) -> f32 {
            self.log.push("div".into());
            self.divisor
        }
    }

    fn chain_host() -> ChainHost {
        ChainHost {
            log: Vec::new(),
            f520: 0,
            chain: 30.0,
            owners: vec![1, 1],
            group: 1,
            chain_owner: 1,
            divisor: 2.0,
        }
    }

    #[test]
    fn chain_minus_1_field_passes_priority_through() {
        let mut h = chain_host();
        h.f520 = -1;
        assert_eq!(
            attack_priority_info_do_mod_priority_by_combat_chain(&mut h, 42, 1, 2, 9.0, 0.0, 1.0),
            42
        );
        assert_eq!(h.log, vec!["f520 1"]); // nothing else runs
    }

    #[test]
    fn chain_sentinel_returns_the_category_constant() {
        let mut h = chain_host();
        h.chain = 0.0;
        // DAT_00bbb9ac sentinel == 0.0 (modelled as the parameter)
        assert_eq!(
            attack_priority_info_do_mod_priority_by_combat_chain(
                &mut h,
                42,
                1,
                2,
                999.5,
                0.0,
                1.0
            ),
            1000 // nearest-even of 999.5 through the x87 pattern
        );
        assert!(!h.log.iter().any(|l| l == "owner"));
    }

    #[test]
    fn chain_bonus_divide_and_owner_reads() {
        // same owner: (30 + 1) / 2 + 42 = 57.5 -> nearest-even ties to 58
        let mut h = chain_host();
        h.owners = vec![3, 3];
        h.chain_owner = 3;
        assert_eq!(
            attack_priority_info_do_mod_priority_by_combat_chain(&mut h, 42, 1, 2, 9.0, 0.0, 1.0),
            58
        );
        // different owners: 30 / 2 + 42 = 57
        let mut h = chain_host();
        h.owners = vec![3, 4];
        h.chain_owner = 3;
        assert_eq!(
            attack_priority_info_do_mod_priority_by_combat_chain(&mut h, 42, 1, 2, 9.0, 0.0, 1.0),
            57
        );
        // group test zero: second owner read never happens
        let mut h = chain_host();
        h.group = 0;
        h.owners = vec![3];
        assert_eq!(
            attack_priority_info_do_mod_priority_by_combat_chain(&mut h, 42, 1, 2, 9.0, 0.0, 1.0),
            57
        );
        assert_eq!(h.log.iter().filter(|l| l.as_str() == "owner").count(), 1);
    }

    #[test]
    fn x87_unsigned_store_edges() {
        // default control word: ties to even
        assert_eq!(x87_store_u32(45.5), 46);
        assert_eq!(x87_store_u32(44.5), 44);
        assert_eq!(x87_store_u32(-0.6), (-1i32) as u32); // wraps two's complement
        // bias path for [2^31, 2^32)
        assert_eq!(x87_store_u32(2147483648.0), 0x8000_0000);
        assert_eq!(x87_store_u32(4294967295.4), 0xFFFF_FFFF);
        // indefinite integer
        assert_eq!(x87_store_u32(f64::NAN), 0x8000_0000);
        assert_eq!(x87_store_u32(4294967296.0), 0x8000_0000);
        assert_eq!(x87_store_u32(-2147483649.0), 0x8000_0000);
        // zero divisor -> infinity -> indefinite
        let mut h = chain_host();
        h.divisor = 0.0;
        assert_eq!(
            attack_priority_info_do_mod_priority_by_combat_chain(&mut h, 42, 1, 2, 9.0, 0.0, 1.0),
            0x8000_0000
        );
    }
}
