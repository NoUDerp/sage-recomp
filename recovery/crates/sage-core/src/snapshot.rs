//! Snapshot record leaves — 5 rows (0x007b4826/0x007b483d/
//! 0x007b485a pool probes, 0x007b3f09 payload-chain walk,
//! 0x007b3f78 record copy-assign).
//!
//! Layout facts from the bodies: records are 0x14 bytes —
//! `+0x00/+0x04` two dwords, `+0x08` a `StringBase<char>` (12 B),
//! `+0x0c` used byte, `+0x0e` count i16, `+0x10` payload pointer.
//! The pool is a record array reached at owner `+0x18`; the
//! walking accessor addresses records off a base at owner `+0x0c`.
//! Payload nodes chain through `+0x00` (next) with payload data
//! starting at `+0x04` (the accessors return `node + 4`).
//!
//! Faithful notes:
//! * 0x007b483d compares the pool slot's i16 count sign-extended
//!   against the probe record's `+0x08` int key — a `0xffff` key
//!   does NOT match a `-1` count.
//! * 0x007b3f09 walks `count − k` links from the chain head and
//!   returns the node's `+4`; the chain must therefore have a
//!   head link before the first data node (sentinel), and
//!   over-running the chain is UB in the game (Rust: panic via
//!   Option unwrap-free indexing).
//! * 0x007b3f78 is a copy-ASSIGN: the StringBase at `+0x08` uses
//!   assignment semantics (release old, copy) — mirrored by
//!   `set_string_base`. Raw payload pointers alias after the copy
//!   (game has shared payload ownership; mirrored with shared
//!   references).

use crate::string::StringAscii;

/// Payload chain node: `+0x00` next link, `+0x04` payload data
/// start (accessors hand out `node + 4`).
pub struct PayloadNode<'a> {
    pub next: Option<&'a PayloadNode<'a>>,
    pub d_04: u32,
}

/// 0x14-byte snapshot record.
pub struct SnapshotRec<'a> {
    pub d_00: i32,
    pub d_04: i32,
    pub name: StringAscii,
    pub used: u8,
    pub count: i16,
    pub payload: Option<&'a PayloadNode<'a>>,
}

/// Probe record consumed by the pool accessors: pool slot index at
/// `+0x04`, key at `+0x08`.
pub struct IndexRec {
    pub d_00: i32,
    pub pool_idx: i32,
    pub key: i32,
}

/// 0x007b4826 — payload of the pool slot named by the probe
/// record (`pool_idx`); the game adds 4 to the payload pointer —
/// data start, i.e. the node itself here.
pub fn nth_payload<'a>(pool: &[SnapshotRec<'a>], rec: &IndexRec) -> Option<&'a PayloadNode<'a>> {
    pool[rec.pool_idx as usize].payload
}

/// 0x007b483d — probe key vs the pool slot's i16 count,
/// sign-extended (0xffff key ≠ −1 count).
pub fn key_match(pool: &[SnapshotRec], rec: &IndexRec) -> bool {
    pool[rec.pool_idx as usize].count as i32 == rec.key
}

/// 0x007b485a — key match OR the pool slot's `used` byte set.
pub fn usable(pool: &[SnapshotRec], rec: &IndexRec) -> bool {
    // faithful short-circuit order: key_match first
    key_match(pool, rec) || pool[rec.pool_idx as usize].used != 0
}

/// 0x007b3f09 — advance `count − k` links from the record's chain
/// head and return that node's `+4` (the node here). `k >= count`
/// leaves the head; over-running the chain is game-UB and panics
/// (None unwrap) here.
pub fn nth_payload_of<'a>(rec: &SnapshotRec<'a>, k: i32) -> &'a PayloadNode<'a> {
    let mut node = rec.payload.expect("0x007b3f09 on null payload chain (game UB)");
    let n = rec.count as i32;
    if k < n {
        let mut steps = n - k;
        loop {
            steps -= 1;
            node = node.next.expect("payload chain over-run (game UB)");
            if steps == 0 {
                break;
            }
        }
    }
    node
}

/// 0x007b3f78 — record copy-assign: two dwords, name via
/// StringBase assignment (release+copy), used byte, count i16,
/// payload pointer copy (shared after).
pub fn rec_assign<'a>(dst: &mut SnapshotRec<'a>, src: &SnapshotRec<'a>) {
    dst.d_00 = src.d_00;
    dst.d_04 = src.d_04;
    dst.name.set_string_base(&src.name);
    dst.used = src.used;
    dst.count = src.count;
    dst.payload = src.payload;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::string::Ascii;

    fn name(s: &str) -> StringAscii {
        let mut n = StringAscii::default();
        n.set(s.as_bytes().iter().copied().map(Ascii).collect::<Vec<_>>().as_slice());
        n
    }

    fn rec<'a>(used: u8, count: i16, payload: Option<&'a PayloadNode<'a>>) -> SnapshotRec<'a> {
        SnapshotRec { d_00: 0, d_04: 0, name: StringAscii::default(), used, count, payload }
    }

    #[test]
    fn key_match_sign_extends_i16() {
        let pool = [rec(0, -1, None)];
        assert!(key_match(&pool, &IndexRec { d_00: 0, pool_idx: 0, key: -1 }));
        assert!(!key_match(&pool, &IndexRec { d_00: 0, pool_idx: 0, key: 0xffff }));
        let pool = [rec(0, 5, None)];
        assert!(key_match(&pool, &IndexRec { d_00: 0, pool_idx: 0, key: 5 }));
    }

    #[test]
    fn usable_or_semantics() {
        let pool = [rec(1, 7, None), rec(0, 7, None)];
        assert!(usable(&pool, &IndexRec { d_00: 0, pool_idx: 0, key: 99 })); // used=1
        assert!(!usable(&pool, &IndexRec { d_00: 0, pool_idx: 1, key: 99 })); // used=0, key no
        assert!(usable(&pool, &IndexRec { d_00: 0, pool_idx: 1, key: 7 })); // key match
    }

    #[test]
    fn payload_walk_count_minus_k_links() {
        // sentinel head + 3 data nodes
        let n3 = PayloadNode { next: None, d_04: 30 };
        let n2 = PayloadNode { next: Some(&n3), d_04: 20 };
        let n1 = PayloadNode { next: Some(&n2), d_04: 10 };
        let n0 = PayloadNode { next: Some(&n1), d_04: 0 };
        let r = SnapshotRec {
            d_00: 0,
            d_04: 0,
            name: StringAscii::default(),
            used: 0,
            count: 3,
            payload: Some(&n0),
        };
        assert_eq!(nth_payload_of(&r, 0).d_04, 30); // 3 links: n0->n1->n2->n3
        assert_eq!(nth_payload_of(&r, 2).d_04, 10); // 1 link
        assert_eq!(nth_payload_of(&r, 3).d_04, 0); // k >= count: head
        let p = [rec(0, 4, Some(&n0))];
        let probe = IndexRec { d_00: 0, pool_idx: 0, key: 4 };
        assert!(nth_payload(&p, &probe).is_some());
    }

    #[test]
    fn rec_assign_copies_all_fields() {
        let node = PayloadNode { next: None, d_04: 77 };
        let mut src = rec(1, -3, Some(&node));
        src.d_00 = 11;
        src.d_04 = 22;
        src.name = name("abc");
        let mut dst = rec(0, 0, None);
        dst.name = name("old_long_name_to_release");
        rec_assign(&mut dst, &src);
        assert_eq!((dst.d_00, dst.d_04, dst.used, dst.count), (11, 22, 1, -3));
        assert_eq!(dst.name.str().len(), 3);
        assert_eq!(dst.payload.unwrap().d_04, 77);
    }
}
