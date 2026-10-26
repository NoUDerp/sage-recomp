//! Wave 45: the three-level FX module store — 28 rows from the `0x007f4`
//! band (band tag FX; corroborated in-band: `0x007f4261` carries the
//! RTTI-derived name `FXParticleSystem::CategoryModuleInfo<1>::~CategoryModuleInfo<1>`,
//! and `0x007f427d` in the same band swaps a pointer vector in parallel with a
//! 0x68-byte instance array, the FX module-instance layout).
//!
//! The band is one container class seen through its accessors. Layout
//! recovered from the bodies (three nested containers plus one deref'd class):
//!
//! ```text
//! outer (`this`)        +0x18 begin / +0x1c end / +0x20 cap  -> 0x1c-byte records
//!                       (also read by neighbours: +0x14, +0x24 ptr, +0x30, +0x38)
//! record   (0x1c)       +0x00 id            (matched against the target's +0x34
//!                                            cell — see 0x006bc1da/0x007f4634)
//!                       +0x04/+0x08/+0x0c   info vector       (0x30-byte infos)
//!                       +0x10/+0x14/+0x18   module-pointer vector (dwords)
//! info     (0x30)       +0x00 pointer to the keyed target object
//!                       +0x04/+0x08/+0x0c   module-pointer vector (dwords)
//!                       +0x10/+0x14/+0x18   parallel 0x68-byte module array
//!                       +0x1c..+0x2c        tail block (written by 0x007f470e)
//! target   (+0x00 hop)  +0x14 identity id, +0x34 family id, +0x44 counter
//! ```
//!
//! The 0x68-byte module instances are never dereferenced inside this band —
//! only addressed (`0x007f4921`) — so the mirror keeps their base cell as a raw
//! dword and models payloads for the vectors that *are* walked (records, infos,
//! module-pointer dwords). As in `drawdata`/`fxstore2`, the two raw-address rows
//! take the owning base cell and reproduce the game's pointer arithmetic.
//!
//! Row -> function map:
//!
//! | Row            | Function                                   | Game shape                                                       |
//! |----------------|--------------------------------------------|------------------------------------------------------------------|
//! | `0x007f458a`   | `record_id`                                | `this+0x18 + idx*0x1c` dword 0                                   |
//! | `0x007f459a`   | `total_info_count`                         | sum of `(*(rec+8) - *(rec+4)) / 0x30` over records               |
//! | `0x007f45df`   | `first_record_with_infos`                  | index of first record with a non-empty info vector, else `0`      |
//! | `0x007f4634`   | `record_by_id`                             | record with `+0x00 == *(param_2+0x34)`; game returns ptr/NULL    |
//! | `0x007f468d`   | `info_target_ptr`                          | `info[idx] + 0x00` dword (target pointer)                        |
//! | `0x007f46a8`   | `record_module_count`                      | `(*(rec+0x14) - *(rec+0x10)) >> 2`                               |
//! | `0x007f46c1`   | `record_module_at`                         | guarded (`0 <= idx < count`) read of the record's module vector  |
//! | `0x007f46f2`   | `info_tail_addr`                           | `infos_begin + 0x1c + idx * 0x30` (raw address)                  |
//! | `0x007f470e`   | `init_info_tail`                           | `info+0x28/0x24/0x20` copies + `info+0x2c = 0x3039`              |
//! | `0x007f4752`   | `record_index_by_id`                       | id search -> index, miss `0xffffffff`                            |
//! | `0x007f4798`   | `info_index_by_target_id`                  | info with `*(info+0 + 0x14) == needle`, miss `0xffffffff`        |
//! | `0x007f47e6`   | `count_infos_with_clear_target_counter`    | infos with `*(info+0 + 0x44) == 0`                               |
//! | `0x007f4831`   | `sum_infos_with_clear_target_counter`      | that count summed over records (`0 < n` guard)                   |
//! | `0x007f486c`   | `find_target_id_in_records`                | nested search: found `(id & ~0xff) \| 1`, miss `n & 0xffffff00`  |
//! | `0x007f48fe`   | `info_module_at`                           | `*(info+0x04 begin + idx*4)` (unchecked)                         |
//! | `0x007f4921`   | `module_instance_addr`                     | `idx * 0x68 + *(info + 0x10)` (raw address)                      |
//! | `0x007f4dae`   | `record_info_count`                        | `(*(rec+8) - *(rec+4)) / 0x30`                                   |
//! | `0x007f4cc0`   | `info_by_target_ptr`                       | info with `+0x00 == needle`; game returns ptr/NULL               |
//! | `0x007f4dca`   | `info_module_count`                        | `(*(info+8) - *(info+4)) >> 2`                                   |
//!
//! Skipped rows (call-chain / dispatch bodies, per the wave's skip rule):
//!
//! | Row            | Reason                                                            |
//! |----------------|-------------------------------------------------------------------|
//! | `0x007f48ef`   | one-line tail call `FUN_007f486c(*(this+0x14))` — call chain       |
//! | `0x007f4944`   | per-record loop whose only effect is `FUN_007f4342` (visitor)      |
//! | `0x007f4a46`   | sort pass: comparator through `(**(code **) *param_4)` + swap call |
//! | `0x007f4abb`   | tail call `FUN_007f44a9` — call chain                             |
//! | `0x007f4ae2`   | per-record `FUN_007f4398`, `FUN_007f43d3`, SEH frame, `FUN_007f498a` |
//! | `0x007f4b51`   | `FUN_007b8b44` call + `(**(code **)**(code **)(this+4))(0)` vtable |
//! | `0x007f4b93`   | `FUN_0061983a` / `FUN_007f4407` / `FUN_007f417e` call chain + SEH  |
//! | `0x007f4bf3`   | element loop through `(*(code *)*puVar4)(puVar4[1])` + SEH frame   |
//! | `0x007f4c60`   | element loop calling `FUN_007f41a4` (5-arg indirect dispatcher)    |
//!
//! Quirks pinned by the game and mirrored exactly:
//! - Every container index is **unchecked** in the game (`begin + idx * stride`
//!   in 32-bit int arithmetic, wraps, then derefs). The mirror replays that same
//!   wrapping byte offset: an index whose wrapped offset is stride-aligned and
//!   inside the block is read exactly as the game reads it, and any other index
//!   (out of range, or a wrap landing between element slots) panics instead of
//!   the game's stray read (faithful-quirk policy: that read is UB).
//! - `0x007f4634` (record pointer or NULL) and `0x007f4cc0` (info pointer or
//!   NULL) return the *found element*, so the mirror returns `Option<&_>`.
//! - `0x007f4752` and `0x007f4798` return the element **index**, and a miss is
//!   the `0xffffffff` (-1) sentinel — a hit at element 0 is `0`, which is also
//!   what an empty vector yields, so callers must test `-1`, not `0`.
//! - `0x007f486c` returns a *masked* value, not an index: Ghidra prints
//!   `CONCAT31((int3)(id >> 8), 1)` = `(id & 0xffffff00) | 1` on a hit (low byte
//!   = 1, upper 24 bits carried from the stored id) and `n & 0xffffff00` on a
//!   miss (`n` = record count). Callers test the low byte (`if (cVar != 0)`,
//!   e.g. `0x008ee0c6`/`0x008ee260`), which is exactly why the miss value must
//!   keep its low byte zero.
//! - `0x007f45df` collapses "hit at index 0" and "no hit" to `0` for the same
//!   reason; the game never returns a distinct miss marker there.
//! - Counts: the game derives them from pointer cells — `/ 0x30` truncates
//!   toward zero, `>> 2` floors (they differ only for a corrupt `end < begin`,
//!   which yields a negative count and an unsigned loop bound of `0xffff_ffff`).
//!   The mirror keeps well-formed vector lengths; the distinction is
//!   unreachable without heap corruption and is not modelled.
//! - `0x007f470e` seeds the info tail block; the `+0x28` value is
//!   `FUN_0079b769` = `FUN_0079b756` = the wrapping sum of 20 dwords at
//!   `+0x20` of another object, so the mirror takes it as a caller-supplied
//!   dword. `+0x2c` gets the constant `0x3039` (12345).

use std::rc::Rc;

/// `0x1c`-byte record of the outer vector (`this + 0x18/0x1c`).
#[derive(Clone, Debug, Default)]
pub struct FxRecord {
    /// `+0x00` record id (the family id; callers pass the target's `+0x34` cell)
    pub id: u32,
    /// `+0x04` begin / `+0x08` end / `+0x0c` cap — the 0x30-byte info vector
    pub infos: Vec<FxModuleInfo>,
    /// `+0x10` begin / `+0x14` end / `+0x18` cap — module-pointer dwords
    pub module_ptrs: Vec<u32>,
}

/// 0x30-byte info element of a record's `+0x04` vector.
#[derive(Clone, Debug, Default)]
pub struct FxModuleInfo {
    /// `+0x00` raw pointer cell to the keyed target object
    pub target_ptr: u32,
    /// resolved target of `+0x00` (the game derefs it at `+0x14` and `+0x44`)
    pub target: Rc<FxTarget>,
    /// `+0x04` begin / `+0x08` end / `+0x0c` cap — module-pointer dwords
    pub module_ptrs: Vec<u32>,
    /// `+0x10` begin of the parallel 0x68-byte module array (`+0x14` end,
    /// `+0x18` cap). Kept as a raw dword: no row in this band reads *inside*
    /// a module instance, they only address one (`0x007f4921`).
    pub modules_begin: u32,
    /// `+0x20` <- source object `+0xd4` (written by `0x007f470e`)
    pub cell_20: u32,
    /// `+0x24` <- source object `+0x74`
    pub cell_24: u32,
    /// `+0x28` <- 20-dword stat sum (`FUN_0079b769`)
    pub cell_28: u32,
    /// `+0x2c` <- `0x3039` marker
    pub cell_2c: u32,
}

/// Object reached through an info's `+0x00` pointer (class unresolved by this
/// band; only these cells are read).
#[derive(Clone, Debug, Default)]
pub struct FxTarget {
    /// `+0x14` identity id — the needle of `0x007f4798` / `0x007f486c`
    pub identity_14: i32,
    /// `+0x34` family id — equals the owning record's `+0x00` id (`0x006bc1da`)
    pub family_34: u32,
    /// `+0x44` counter — tested against 0 by `0x007f47e6`
    pub counter_44: i32,
}

/// record stride of the outer vector (bytes).
const RECORD_STRIDE: u32 = 0x1c;
/// info stride of a record's `+0x04` info vector (bytes).
const INFO_STRIDE: u32 = 0x30;
/// element stride of a module-pointer vector (bytes).
const MODULE_STRIDE: u32 = 4;

/// Game vector indexing: the game computes `begin + index * stride` in 32-bit
/// int arithmetic — the multiply wraps and nothing is bounds-checked, so a
/// negative or huge index derefs whatever the wrapped byte offset lands on
/// (UB). The mirror replays the same wrapping byte offset and resolves it only
/// when that offset is stride-aligned and inside the borrowed block; otherwise
/// it panics (faithful-quirk policy). A wrapped offset that *is* aligned and
/// in range is read exactly as the game reads it.
fn slot(index: i32, stride_bytes: u32, len: usize) -> usize {
    let off = (index as u32).wrapping_mul(stride_bytes);
    if off % stride_bytes != 0 || (off / stride_bytes) as usize >= len {
        panic!(
            "index {index} * {stride_bytes:#x} -> offset {off:#x} is not an aligned element \
             slot inside the vector (len {len}); the game derefs the wrapped address"
        );
    }
    (off / stride_bytes) as usize
}

/// `+0x00` of record `index` — the game reads
/// `*(this + 0x18)[index * 0x1c]` with no bounds check.
///
/// Row: `0x007f458a`.
pub fn record_id(records: &[FxRecord], index: i32) -> u32 {
    records[slot(index, RECORD_STRIDE, records.len())].id
}

/// Sum of the info counts of every record (`0x007f459a` computes
/// `iVar5 + (end - begin) / 0x30` per record in an `int`, i.e. wrapping).
/// The game's `(end - begin) / 0x1c != 0` guard only skips the loop of an empty
/// outer vector, which folds to the same `0` result.
///
/// Row: `0x007f459a`.
pub fn total_info_count(records: &[FxRecord]) -> i32 {
    records
        .iter()
        .fold(0i32, |acc, rec| acc.wrapping_add(rec.infos.len() as i32))
}

/// Index of the first record whose info vector is non-empty. A hit at index 0
/// and a total miss both return `0` — the game's own collapse (`return uVar2`
/// inside the loop, `return 0` after it).
///
/// Row: `0x007f45df`.
pub fn first_record_with_infos(records: &[FxRecord]) -> u32 {
    for (i, rec) in records.iter().enumerate() {
        if !rec.infos.is_empty() {
            return i as u32;
        }
    }
    0
}

/// Record whose `+0x00` id equals `family_id` (the game loads the needle from
/// `*(param_2 + 0x34)`). The game returns the record pointer or `0x0`; the
/// mirror returns the record itself or `None`. First match wins.
///
/// Row: `0x007f4634`.
pub fn record_by_id<'a>(records: &'a [FxRecord], family_id: u32) -> Option<&'a FxRecord> {
    records.iter().find(|rec| rec.id == family_id)
}

/// `+0x00` of `info_index` inside record `record_index` — the raw target
/// pointer cell (`0x006bc1da` then derefs it at `+0x34`).
///
/// Row: `0x007f468d`.
pub fn info_target_ptr(records: &[FxRecord], record_index: i32, info_index: i32) -> u32 {
    let rec = &records[slot(record_index, RECORD_STRIDE, records.len())];
    rec.infos[slot(info_index, INFO_STRIDE, rec.infos.len())].target_ptr
}

/// Count of the record's own module-pointer vector (`+0x10`/`+0x14`), which
/// the game computes as `(*(rec + 0x14) - *(rec + 0x10)) >> 2`.
///
/// Row: `0x007f46a8`.
pub fn record_module_count(record: &FxRecord) -> i32 {
    record.module_ptrs.len() as i32
}

/// Guarded read of the record's module-pointer vector: the game tests
/// `param_3 >= 0` first, then `param_3 < count` (signed), and returns the
/// `0xffffffff` sentinel when either test fails. The successful read is the
/// unchecked `*(begin + param_3 * 4)`.
///
/// Row: `0x007f46c1`.
pub fn record_module_at(record: &FxRecord, index: i32) -> u32 {
    if index >= 0 && (index as usize) < record.module_ptrs.len() {
        record.module_ptrs[index as usize]
    } else {
        0xffff_ffff
    }
}

/// Address the game hands back for record-index `info_index`: the info vector
/// base plus `0x1c + index * 0x30` (int arithmetic, wraps) — i.e. the info's
/// tail block. The mirror takes the base dword read from `*(record + 0x04)`.
///
/// Row: `0x007f46f2`.
pub fn info_tail_addr(infos_begin: u32, info_index: i32) -> u32 {
    infos_begin
        .wrapping_add(0x1c)
        .wrapping_add((info_index as u32).wrapping_mul(0x30))
}

/// Initialise an info's tail block exactly in the game's store order: `+0x28`
/// (20-dword stat sum from `FUN_0079b769`), `+0x24` (source `+0x74`), `+0x20`
/// (source `+0xd4`), then the `0x3039` marker at `+0x2c`. The source cells and
/// the stat sum are caller-resolved (`param_4 + 0x74`, `param_4 + 0xd4`).
///
/// Row: `0x007f470e`.
pub fn init_info_tail(info: &mut FxModuleInfo, stat_sum: u32, src_74: u32, src_d4: u32) {
    info.cell_28 = stat_sum;
    info.cell_24 = src_74;
    info.cell_20 = src_d4;
    info.cell_2c = 0x3039;
}

/// Index of the record whose `+0x00` id equals `family_id` (needle read from
/// `*(param_2 + 0x34)`); miss is the `0xffffffff` (-1) sentinel. Same
/// first-match-wins scan as `record_by_id`.
///
/// Row: `0x007f4752`.
pub fn record_index_by_id(records: &[FxRecord], family_id: u32) -> u32 {
    for (i, rec) in records.iter().enumerate() {
        if rec.id == family_id {
            return i as u32;
        }
    }
    0xffff_ffff
}

/// Index of the first info in `record` whose target's `+0x14` id equals
/// `identity_id`; miss is `0xffffffff`. The game walks the info vector at
/// stride `0x30` and derefs `*info` each step.
///
/// Row: `0x007f4798`.
pub fn info_index_by_target_id(record: &FxRecord, identity_id: i32) -> u32 {
    for (i, info) in record.infos.iter().enumerate() {
        if info.target.identity_14 == identity_id {
            return i as u32;
        }
    }
    0xffff_ffff
}

/// Count of infos in `record` whose target's `+0x44` cell is zero.
///
/// Row: `0x007f47e6`.
pub fn count_infos_with_clear_target_counter(record: &FxRecord) -> i32 {
    record
        .infos
        .iter()
        .filter(|info| info.target.counter_44 == 0)
        .count() as i32
}

/// The same count summed over every record (`0x007f4831` guards with the
/// signed test `0 < (end - begin) / 0x1c` and accumulates in an `int`).
///
/// Row: `0x007f4831`.
pub fn sum_infos_with_clear_target_counter(records: &[FxRecord]) -> i32 {
    records.iter().fold(0i32, |acc, rec| {
        acc.wrapping_add(count_infos_with_clear_target_counter(rec))
    })
}

/// Nested search: scan every record's infos for a target whose `+0x14` id
/// equals `identity_id`. The return value is *not* an index — Ghidra prints
/// `CONCAT31((int3)(id >> 8), 1)`, i.e. the stored id with its low byte forced
/// to 1 (`(id & 0xffffff00) | 1`) on a hit, and `record_count & 0xffffff00` on
/// a miss, so only the low byte is a reliable found/miss flag.
///
/// Row: `0x007f486c`.
pub fn find_target_id_in_records(records: &[FxRecord], identity_id: i32) -> u32 {
    for rec in records {
        for info in &rec.infos {
            if info.target.identity_14 == identity_id {
                return (info.target.identity_14 as u32 & 0xff_ff_ff00) | 1;
            }
        }
    }
    (records.len() as u32) & 0xff_ff_ff00
}

/// `param_4`-th module pointer of info `info_index` in record `record_index`
/// (`*(info + 0x04 begin + param_4 * 4)`); unguarded and unchecked, unlike
/// `0x007f46c1` — the mirror panics when the wrapped offset leaves the vector.
///
/// Row: `0x007f48fe`.
pub fn info_module_at(
    records: &[FxRecord],
    record_index: i32,
    info_index: i32,
    module_index: i32,
) -> u32 {
    let rec = &records[slot(record_index, RECORD_STRIDE, records.len())];
    let info = &rec.infos[slot(info_index, INFO_STRIDE, rec.infos.len())];
    info.module_ptrs[slot(module_index, MODULE_STRIDE, info.module_ptrs.len())]
}

/// Address of module instance `module_index` in the info's parallel 0x68-byte
/// array: the game computes `module_index * 0x68 + *(info + 0x10)` (multiply
/// first, int arithmetic, wraps). The mirror takes that `+0x10` base cell.
///
/// Row: `0x007f4921`.
pub fn module_instance_addr(modules_begin: u32, module_index: i32) -> u32 {
    (module_index as u32)
        .wrapping_mul(0x68)
        .wrapping_add(modules_begin)
}

/// Info count of record `+0x04`/`+0x08`, which the game computes as
/// `(*(rec + 8) - *(rec + 4)) / 0x30` (signed division).
///
/// Row: `0x007f4dae`.
pub fn record_info_count(record: &FxRecord) -> i32 {
    record.infos.len() as i32
}

/// Info whose `+0x00` pointer cell equals `target_ptr` — pointer equality, not
/// id equality. The game returns the element address or `0x0`; the mirror
/// returns the info or `None`. (Same `+0x04`/`+0x08` vector viewed with `this`
/// = record.)
///
/// Row: `0x007f4cc0`.
pub fn info_by_target_ptr<'a>(record: &'a FxRecord, target_ptr: u32) -> Option<&'a FxModuleInfo> {
    record
        .infos
        .iter()
        .find(|info| info.target_ptr == target_ptr)
}

/// Module-pointer count of an info's own `+0x04`/`+0x08` vector, computed by
/// the game as `(*(info + 8) - *(info + 4)) >> 2` (arithmetic shift).
///
/// Row: `0x007f4dca`.
pub fn info_module_count(info: &FxModuleInfo) -> i32 {
    info.module_ptrs.len() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(identity: i32, family: u32, counter: i32) -> Rc<FxTarget> {
        Rc::new(FxTarget {
            identity_14: identity,
            family_34: family,
            counter_44: counter,
        })
    }

    fn info(target_ptr: u32, identity: i32, counter: i32) -> FxModuleInfo {
        FxModuleInfo {
            target_ptr,
            target: target(identity, target_ptr, counter),
            ..Default::default()
        }
    }

    #[test]
    fn record_id_reads_the_id_cell_and_bounds_panic() {
        let recs = vec![
            FxRecord {
                id: 0x1234,
                ..Default::default()
            },
            FxRecord {
                id: 0x0bad_c0de,
                ..Default::default()
            },
        ];
        assert_eq!(record_id(&recs, 0), 0x1234);
        assert_eq!(record_id(&recs, 1), 0x0bad_c0de);
    }

    #[test]
    #[should_panic]
    fn record_id_negative_index_is_out_of_range() {
        let recs = vec![FxRecord { id: 1, ..Default::default() }];
        let _ = record_id(&recs, -1);
    }

    #[test]
    fn total_info_count_sums_per_record_counts() {
        let recs = vec![
            FxRecord {
                infos: vec![info(1, 1, 0), info(2, 2, 0)],
                ..Default::default()
            },
            FxRecord {
                infos: vec![info(3, 3, 0)],
                ..Default::default()
            },
            FxRecord::default(),
        ];
        assert_eq!(total_info_count(&recs), 3);
        assert_eq!(total_info_count(&[]), 0);
    }

    #[test]
    fn first_record_with_infos_skips_empty_records() {
        let recs = vec![
            FxRecord::default(),
            FxRecord::default(),
            FxRecord {
                infos: vec![info(9, 9, 0)],
                ..Default::default()
            },
        ];
        assert_eq!(first_record_with_infos(&recs), 2);
        // no record has infos -> 0
        assert_eq!(first_record_with_infos(&[FxRecord::default()]), 0);
        // hit at index 0 collapses onto the same 0 (pinned game quirk)
        let hit_first = vec![FxRecord {
            infos: vec![info(9, 9, 0)],
            ..Default::default()
        }];
        assert_eq!(first_record_with_infos(&hit_first), 0);
    }

    #[test]
    fn record_by_id_matches_first_and_reports_miss_as_none() {
        let recs = vec![
            FxRecord {
                id: 7,
                module_ptrs: vec![0x1111],
                ..Default::default()
            },
            FxRecord {
                id: 7,
                module_ptrs: vec![0x2222],
                ..Default::default()
            },
            FxRecord {
                id: 9,
                ..Default::default()
            },
        ];
        // first match wins (the game returns the first record address)
        assert_eq!(record_by_id(&recs, 7).unwrap().module_ptrs, vec![0x1111]);
        assert_eq!(record_by_id(&recs, 9).unwrap().id, 9);
        assert!(record_by_id(&recs, 8).is_none());
        assert!(record_by_id(&[], 7).is_none());
    }

    #[test]
    fn record_index_by_id_uses_the_minus_one_sentinel() {
        let recs = vec![
            FxRecord {
                id: 5,
                ..Default::default()
            },
            FxRecord {
                id: 6,
                ..Default::default()
            },
        ];
        assert_eq!(record_index_by_id(&recs, 5), 0);
        assert_eq!(record_index_by_id(&recs, 6), 1);
        assert_eq!(record_index_by_id(&recs, 7), 0xffff_ffff);
        assert_eq!(record_index_by_id(&[], 5), 0xffff_ffff);
    }

    #[test]
    fn info_target_ptr_returns_the_raw_pointer_cell() {
        let recs = vec![FxRecord {
            infos: vec![info(0x00c3_7064, 3, 0), info(0x7ff0_0000, 4, 0)],
            ..Default::default()
        }];
        assert_eq!(info_target_ptr(&recs, 0, 0), 0x00c3_7064);
        assert_eq!(info_target_ptr(&recs, 0, 1), 0x7ff0_0000);
    }

    #[test]
    #[should_panic]
    fn info_target_ptr_index_past_the_info_vector_panics() {
        let recs = vec![FxRecord {
            infos: vec![info(1, 1, 0)],
            ..Default::default()
        }];
        let _ = info_target_ptr(&recs, 0, 1);
    }

    #[test]
    fn info_index_by_target_id_uses_the_minus_one_sentinel() {
        let rec = FxRecord {
            infos: vec![info(1, 40, 0), info(2, 41, 0)],
            ..Default::default()
        };
        assert_eq!(info_index_by_target_id(&rec, 40), 0);
        assert_eq!(info_index_by_target_id(&rec, 41), 1);
        assert_eq!(info_index_by_target_id(&rec, 42), 0xffff_ffff);
        // a negative needle is compared signed by the game, never a match here
        assert_eq!(info_index_by_target_id(&rec, -1), 0xffff_ffff);
    }

    #[test]
    fn target_counter_counts_and_sums_per_record() {
        let rec = FxRecord {
            infos: vec![info(1, 1, 0), info(2, 2, 7), info(3, 3, -1), info(4, 4, 0)],
            ..Default::default()
        };
        assert_eq!(count_infos_with_clear_target_counter(&rec), 2);
        let recs = vec![
            rec.clone(),
            FxRecord {
                infos: vec![info(5, 5, 3)],
                ..Default::default()
            },
            FxRecord::default(),
        ];
        assert_eq!(sum_infos_with_clear_target_counter(&recs), 2);
        assert_eq!(sum_infos_with_clear_target_counter(&[]), 0);
    }

    #[test]
    fn find_target_id_returns_masked_id_on_hit() {
        let recs = vec![
            FxRecord {
                infos: vec![info(0x1000, 0x0000_1234, 0)],
                ..Default::default()
            },
            FxRecord {
                infos: vec![info(0x2000, 0x00ab_cdef, 0)],
                ..Default::default()
            },
        ];
        // low byte forced to 1, upper 24 bits carried from the stored id
        assert_eq!(find_target_id_in_records(&recs, 0x0000_1234), 0x0000_1201);
        // an id whose low byte is already non-zero is still overwritten with 1
        assert_eq!(find_target_id_in_records(&recs, 0x00ab_cdef), 0x00ab_cd01);
        // absent id: miss value keeps the low byte clear
        assert_eq!(find_target_id_in_records(&recs, 0x0000_9999), 0);
    }

    #[test]
    fn find_target_id_miss_keeps_low_byte_clear() {
        let recs = vec![FxRecord::default(); 3];
        assert_eq!(find_target_id_in_records(&recs, 1), 0);
        // >= 256 records: the record count survives the 0xffffff00 mask
        let many = vec![FxRecord::default(); 0x100];
        assert_eq!(find_target_id_in_records(&many, 1), 0x100);
        assert_eq!(find_target_id_in_records(&[], 1), 0);
    }

    #[test]
    fn record_module_at_guard_matrix() {
        let rec = FxRecord {
            module_ptrs: vec![0xaa, 0xbb, 0xcc],
            ..Default::default()
        };
        assert_eq!(record_module_count(&rec), 3);
        assert_eq!(record_module_at(&rec, 0), 0xaa);
        assert_eq!(record_module_at(&rec, 2), 0xcc);
        assert_eq!(record_module_at(&rec, 3), 0xffff_ffff); // == count
        assert_eq!(record_module_at(&rec, -1), 0xffff_ffff);
        assert_eq!(record_module_at(&rec, i32::MIN), 0xffff_ffff);
        assert_eq!(record_module_at(&FxRecord::default(), 0), 0xffff_ffff);
    }

    #[test]
    fn info_module_at_reads_the_infos_own_module_vector() {
        let recs = vec![FxRecord {
            infos: vec![FxModuleInfo {
                target: target(1, 1, 0),
                module_ptrs: vec![0x11, 0x22],
                ..Default::default()
            }],
            ..Default::default()
        }];
        assert_eq!(info_module_count(&recs[0].infos[0]), 2);
        assert_eq!(info_module_at(&recs, 0, 0, 1), 0x22);
    }

    #[test]
    #[should_panic]
    fn info_module_at_past_the_module_vector_panics() {
        let recs = vec![FxRecord {
            infos: vec![FxModuleInfo {
                target: target(1, 1, 0),
                module_ptrs: vec![0x11, 0x22],
                ..Default::default()
            }],
            ..Default::default()
        }];
        let _ = info_module_at(&recs, 0, 0, 2);
    }

    #[test]
    #[should_panic]
    fn info_module_at_negative_index_panics() {
        let recs = vec![FxRecord {
            infos: vec![FxModuleInfo {
                target: target(1, 1, 0),
                module_ptrs: vec![0x11, 0x22],
                ..Default::default()
            }],
            ..Default::default()
        }];
        let _ = info_module_at(&recs, 0, 0, -1);
    }

    #[test]
    fn info_tail_addr_replays_the_pointer_math() {
        // base + 0x1c + idx * 0x30
        assert_eq!(info_tail_addr(0x00c3_7064, 0), 0x00c3_7080);
        assert_eq!(info_tail_addr(0x00c3_7064, 1), 0x00c3_70b0);
        // 0x30 bytes per info, negative index steps back
        assert_eq!(info_tail_addr(0x00c3_7064, -1), 0x00c3_7050);
        // wrapping over the 32-bit edge
        assert_eq!(info_tail_addr(0xffff_ffd0, 1), 0x0000_001c);
    }

    #[test]
    fn module_instance_addr_multiplies_then_adds_with_wrap() {
        assert_eq!(module_instance_addr(0x1000, 0), 0x1000);
        assert_eq!(module_instance_addr(0x1000, 2), 0x10d0);
        assert_eq!(module_instance_addr(0x1000, -1), 0x0f98);
        assert_eq!(module_instance_addr(0x20, -1), 0xffff_ffb8);
        // the index multiply wraps on its own before the add
        assert_eq!(module_instance_addr(0x0, 0x1000_0000), 0x8000_0000);
    }

    #[test]
    fn wrapped_index_offsets_resolve_when_stride_aligned() {
        // `index * 0x1c` is 32-bit int arithmetic: 0x4000_0000 * 0x1c wraps to
        // byte offset 0 and 0x4000_0001 * 0x1c wraps to 0x1c, so the game really
        // does re-read records 0 and 1 — the mirror replays that exactly.
        let recs = vec![
            FxRecord {
                id: 0xaa,
                ..Default::default()
            },
            FxRecord {
                id: 0xbb,
                ..Default::default()
            },
        ];
        assert_eq!(record_id(&recs, 0x4000_0000), 0xaa);
        assert_eq!(record_id(&recs, 0x4000_0001), 0xbb);
    }

    #[test]
    #[should_panic]
    fn wrapped_index_offset_between_element_slots_panics() {
        // 0x2000_0001 * 0x1c wraps to 0x8000_001c, which is not a multiple of
        // 0x1c: the game would read a dword straddling two records.
        let recs = vec![FxRecord {
            id: 0xaa,
            ..Default::default()
        }];
        let _ = record_id(&recs, 0x2000_0001);
    }

    #[test]
    fn init_info_tail_stores_the_source_cells_and_the_marker() {
        let mut i = FxModuleInfo {
            target: target(1, 1, 0),
            cell_20: 0xdead,
            cell_24: 0xbeef,
            cell_28: 0xcafe,
            cell_2c: 0xf00d,
            ..Default::default()
        };
        init_info_tail(&mut i, 0x0000_0042, 0x1111_1111, 0x2222_2222);
        assert_eq!(i.cell_28, 0x0000_0042); // <- FUN_0079b769 stat sum
        assert_eq!(i.cell_24, 0x1111_1111); // <- source +0x74
        assert_eq!(i.cell_20, 0x2222_2222); // <- source +0xd4
        assert_eq!(i.cell_2c, 0x3039); // marker, written last
    }

    #[test]
    fn info_by_target_ptr_compares_the_raw_cell_not_the_id() {
        let rec = FxRecord {
            infos: vec![
                FxModuleInfo {
                    target_ptr: 0x0010_0000,
                    target: target(77, 5, 0),
                    ..Default::default()
                },
                FxModuleInfo {
                    target_ptr: 0x0020_0000,
                    target: target(77, 5, 1),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        // same identity id, different pointers: the pointer cell decides
        assert_eq!(info_by_target_ptr(&rec, 0x0020_0000).unwrap().target_ptr, 0x0020_0000);
        assert_eq!(
            info_by_target_ptr(&rec, 0x0020_0000).unwrap().target.counter_44,
            1
        );
        assert!(info_by_target_ptr(&rec, 0x0030_0000).is_none());
        assert_eq!(record_info_count(&rec), 2);
    }
}
