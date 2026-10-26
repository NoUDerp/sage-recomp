//! ScriptStore leaves — the script engine's id tables and node
//! store, 9 rows (0x006039xx indexers, 0x007b27xx/0x007b33xx/
//! 0x007b43xx store ops, 0x007b31xx 4-byte key helpers).
//!
//! Layout facts from the bodies:
//! * Table A (thing/template table): base `this + 0x20`, 0x80-byte
//!   records, valid ids 0..=0x256; out-of-range ids fold to 0
//!   (`0x00603926` — clamp, not rejection).
//! * Table B (script records): base `this + 0x12ba0`, 0x80-byte
//!   records, valid ids 0..=0xc9 (`0x00603941`).
//! * A table-A record exposes `+0x48` = element count and
//!   `+0x4c` = element array (0x007b27a3 nth with the same
//!   fold-to-zero guard style).
//! * The store body: `+0x04` current id, `+0x08` node count,
//!   `+0x0c` array of 0x28-byte node pointers.
//!
//! `0x007b4343` faithful notes: nodes are `operator new(0x28)`;
//! OOM leaves a NULL slot (unreachable in Rust — documented, not
//! modelled). Old nodes get `StringBase::releaseBuffer` on their
//! `+0x10` string then `operator delete` — mirrored as
//! `release_buffer` + drop.

use crate::string::StringAscii;

/// 0x00603926 — table-A slot lookup: `id` outside `0..=0x256` folds
/// to 0. Returns the record index (base `this + 0x20`, stride 0x80).
pub fn table_a_slot(id: i32) -> usize {
    let id = if id < 0 || 0x256 < id { 0 } else { id };
    id as usize
}

/// 0x00603941 — table-B slot lookup: `id` outside `0..=0xc9` folds
/// to 0. Returns the record index (base `this + 0x12ba0`, 0x80
/// stride).
pub fn table_b_slot(id: i32) -> usize {
    let id = if id < 0 || 0xc9 < id { 0 } else { id };
    id as usize
}

/// Table-A record (0x80 bytes): the fields the wave touches.
pub struct RecordA {
    /// +0x48 element count
    pub count: i32,
    /// +0x4c.. element array
    pub vals: Vec<i32>,
}

/// 0x007b27a3 — table-A record nth: `idx` outside `0..count`
/// yields 0 (fold).
pub fn nth_a(rec: &RecordA, idx: i32) -> i32 {
    if idx < 0 || rec.count <= idx {
        0
    } else {
        rec.vals[idx as usize]
    }
}

/// Table-B record head (0x80 bytes): first dword + the
/// `StringBase<char>` name at +0x04.
pub struct RecordB {
    pub flag: i32,
    pub name: StringAscii,
}

/// The store object (subset): current id +4, node count +8, node
/// pointer array +0xc.
pub struct Store {
    pub cur_id: i32,
    pub node_count: i32,
    pub nodes: Vec<Option<ScriptNode>>,
}

/// 0x28-byte node built by 0x007b333a: everything zero except
/// `+0x00` = key (the table-A `+0x4c` value) and `+0x08` = the
/// ctor's third arg. `+0x10` is a zero-initialised raw StringBase
/// (the game calls releaseBuffer on the raw record — a zeroed
/// StringBase is its empty state).
pub struct ScriptNode {
    pub key: i32,     // +0x00
    pub flag_04: u8,  // +0x04
    pub d_08: i32,    // +0x08
    pub str_10: StringAscii,
    pub d_14: i32,
    pub d_18: i32,
    pub d_1c: i32,
    pub d_20: i32,
    pub d_24: i32,
}

/// 0x007b333a — node ctor: `+0` = key, `+4` byte = 0, `+8` = d8,
/// the rest (incl. the +0x10 string) zeroed.
pub fn node_new(key: i32, d8: i32) -> ScriptNode {
    ScriptNode {
        key,
        flag_04: 0,
        d_08: d8,
        str_10: StringAscii::default(),
        d_14: 0,
        d_18: 0,
        d_1c: 0,
        d_20: 0,
        d_24: 0,
    }
}

/// 0x007b275a — first dword of the current table-B record.
pub fn current_flag(store: &Store, table_b: &[RecordB]) -> i32 {
    table_b[table_b_slot(store.cur_id)].flag
}

/// 0x007b3989 — copy-ctor of `out` from the current table-B
/// record's name (+0x04).
pub fn copy_current_name(store: &Store, table_b: &[RecordB], out: &mut StringAscii) {
    out.set_string_base(&table_b[table_b_slot(store.cur_id)].name);
}

/// 0x007b4343 — set id and rebuild the node array from the
/// table-A record's element list:
/// release+free all old nodes (old count), store id, count =
/// record count, then per element `node_new(nth_a(rec, i), 0)`.
/// The game's `operator new` OOM NULL-slot path is unreachable in
/// Rust and not modelled.
pub fn set_id_and_rebuild(store: &mut Store, table_a: &[RecordA], id: i32) {
    for slot in store.nodes.iter_mut() {
        if let Some(node) = slot.as_mut() {
            node.str_10.release_buffer();
        }
        *slot = None;
    }
    store.cur_id = id;
    let rec = &table_a[table_a_slot(id)];
    store.node_count = rec.count;
    store.nodes.clear();
    let n = if rec.count > 0 { rec.count as usize } else { 0 };
    for i in 0..n as i32 {
        store.nodes.push(Some(node_new(nth_a(rec, i), 0)));
    }
}

/// 0x007b31c7 — 4-byte key equality (`memcmp(a, b, 4) == 0`).
pub fn bytes_eq4(a: &[u8; 4], b: &[u8; 4]) -> bool {
    a == b
}

/// 0x007b31ad — 4-byte zero init (`memset(p, 0, 4)`).
pub fn zero4(p: &mut [u8; 4]) {
    *p = [0; 4];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_guards_fold_to_zero() {
        assert_eq!(table_a_slot(-1), 0);
        assert_eq!(table_a_slot(0x257), 0);
        assert_eq!(table_a_slot(0x256), 0x256);
        assert_eq!(table_a_slot(5), 5);
        assert_eq!(table_b_slot(-1), 0);
        assert_eq!(table_b_slot(0xca), 0);
        assert_eq!(table_b_slot(0xc9), 0xc9);
    }

    #[test]
    fn nth_a_guard() {
        let rec = RecordA { count: 3, vals: vec![7, 8, 9] };
        assert_eq!(nth_a(&rec, 0), 7);
        assert_eq!(nth_a(&rec, 2), 9);
        assert_eq!(nth_a(&rec, 3), 0); // count <= idx
        assert_eq!(nth_a(&rec, -1), 0);
    }

    #[test]
    fn current_accessors_use_current_id() {
        let store = Store { cur_id: 1, node_count: 0, nodes: Vec::new() };
        let tb = vec![
            RecordB { flag: 10, name: StringAscii::default() },
            RecordB { flag: 20, name: StringAscii::default() },
        ];
        assert_eq!(current_flag(&store, &tb), 20);
        // out-of-range id folds to slot 0
        let bad = Store { cur_id: 0x1000, node_count: 0, nodes: Vec::new() };
        assert_eq!(current_flag(&bad, &tb), 10);
    }

    #[test]
    fn set_id_and_rebuild_replaces_nodes_from_record() {
        let mut store = Store {
            cur_id: 7,
            node_count: 1,
            nodes: vec![Some(node_new(99, 0))],
        };
        let ta = vec![
            RecordA { count: 0, vals: vec![] },
            RecordA { count: 3, vals: vec![11, 22, 33] },
        ];
        set_id_and_rebuild(&mut store, &ta, 1);
        assert_eq!(store.cur_id, 1);
        assert_eq!(store.node_count, 3);
        let keys: Vec<i32> = store.nodes.iter().map(|n| n.as_ref().unwrap().key).collect();
        assert_eq!(keys, vec![11, 22, 33]);
        // out-of-range id rebuilds from slot 0 (empty)
        set_id_and_rebuild(&mut store, &ta, 0x300);
        assert_eq!(store.node_count, 0);
        assert!(store.nodes.is_empty());
    }

    #[test]
    fn node_new_zeroes_everything_but_key_and_d8() {
        let n = node_new(5, -2);
        assert_eq!(n.key, 5);
        assert_eq!(n.d_08, -2);
        assert_eq!(n.flag_04, 0);
        assert!(n.str_10.is_empty());
    }

    #[test]
    fn eq4_and_zero4() {
        assert!(bytes_eq4(&[1, 2, 3, 4], &[1, 2, 3, 4]));
        assert!(!bytes_eq4(&[1, 2, 3, 4], &[1, 2, 3, 5]));
        let mut b = [9u8; 4];
        zero4(&mut b);
        assert_eq!(b, [0; 4]);
    }
}
