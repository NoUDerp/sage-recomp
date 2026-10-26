//! Wave 41: static enum **name-table** accessors — the game's `ScanIndexList`-style
//! machinery over NULL-terminated `const char*` tables in `.data`, plus the
//! `(n >> 5) * 4` bitset indexing that pairs with them.
//!
//! Every game function in this family bakes the *address* of one static table as an
//! immediate; the table itself is game data, so the Rust mirror takes the table as a
//! borrowed slice (the caller supplies the game's table image). Table identities are
//! pinned by their raw contents (decoded from `game/game.dat` `.data` at the cited VA):
//!
//! | table VA      | entries | head / tail evidence                                   | domain                    |
//! |---------------|---------|--------------------------------------------------------|---------------------------|
//! | `0x00dbaa98`  | 591     | `"TOPPLED" … "SPECIAL_WEAPON_SIX"`                      | model-condition flags     |
//! | `0x00dc828c`  | 11      | `"DEFAULT" … "DISABLED_SCRIPT_UNDERPOWERED"`            | disabled flags            |
//! | `0x00dbbe18`  | 218     | `"OBSTACLE" … "AMPHIBIOUS"`                             | KindOf mask               |
//! | `0x00dbc2c8`  | 104     | `"VETERAN" … "WEAPONSET_CREATE_A_HERO_WS_64"`           | weapon-set/upgrade flags  |
//! | `0x00da5f30`  | 101     | `"DESTROYED" … "COMMAND_BUTTON_TOGGLED"`                | model-state flags         |
//! | `0x00dc0b40`  | 154     | `"SPECIAL_INVALID" … "SPECIAL_SUMMON_ALLIES"`           | special-power flags       |
//! | `0x00dba9c8`  | 12      | `"TAUNT" … "CHEER_FOR_ABOUT_TO_CRUSH"`                  | emotion types             |
//! | `0x00dba9ac`  | 6       | `"BACK_AWAY" … "QUARREL"`                               | emotion-AI types          |
//! | `0x00dbaa40`  | 21      | `"VETERAN" … "CREATE_A_HERO_10"`                        | experience/upgrade flags  |
//! | `0x00dc85c4`  | 8       | `"AutoResolveUnit_Soldier" … "AutoResolveUnit_INVALID"` | auto-resolve unit kinds   |
//!
//! The game's scan terminates on a NULL *cell* (end of table); the Rust slices end at
//! that terminator, so `table.len()` is the NULL slot.
//!
//! Quirks pinned by the game and mirrored exactly:
//! - `(byte)idx & 0x1f` bit selection == `idx & 0x1f` (byte truncation is a no-op).
//! - dword slot `(idx >> 5) * 4` is computed on the full `u32` index — out-of-range
//!   indices read past the block in the game (UB); the Rust mirror indexes unchecked
//!   and panics instead (documented under faithful-quirk policy).
//! - set-by-name returns the *masked* miss value `idx & 0xffffff00` (for the `-1`
//!   sentinel that is `0xFFFFFF00`): callers testing the low byte see `0`, callers
//!   testing truthiness see non-zero — both callers exist (e.g. `0x008be78b` uses the
//!   byte form), so the exact bit pattern is returned, not a plain `-1`.

/// Case-insensitive ASCII compare mirroring MSVC `_strcmpi` for the ASCII-only
/// table names (locale folding is irrelevant: every table entry is `[A-Za-z0-9_]`).
fn name_eq(lhs: &str, rhs: &str) -> bool {
    lhs.len() == rhs.len()
        && lhs
            .bytes()
            .zip(rhs.bytes())
            .all(|(a, b)| a.to_ascii_lowercase() == b.to_ascii_lowercase())
}

/// Linear name -> index scan terminating on the NULL cell; `-1` on miss.
///
/// Rows (table in parentheses): `0x004b42ca` (0x00dbaa98 model-condition),
/// `0x0068c7ad` (0x00dc828c disabled), `0x0068c7e3` (0x00dbc2c8 weapon-set),
/// `0x0068c819` (0x00da5f30 model-state), `0x0068c84f` (0x00dc0b40 special-power),
/// `0x006aa2f9` (0x00dbbe18 KindOf), `0x008bdc6d` (0x00dbaa40 experience),
/// `0x008dc696` (0x00dba9c8 emotion), `0x008dc6cc` (0x00dba9ac emotion-AI).
pub fn scan_name<'a>(table: &[&'a str], name: &str) -> i32 {
    for (i, entry) in table.iter().enumerate() {
        if name_eq(entry, name) {
            return i as i32;
        }
    }
    -1
}

/// Bitset flag test -> table name: `table[idx]` if the bit is set, `None` otherwise.
/// The bitset block starts at the borrowed slice (game `param_1 + (idx >> 5) * 4`).
///
/// Rows (table): `0x004454f3` (0x00dbaa98), `0x0068c6fb` (0x00dc828c),
/// `0x0068c725` (0x00dbc2c8), `0x0068c753` (0x00da5f30), `0x0068c783` (0x00dc0b40),
/// `0x006aa2cf` (0x00dbbe18), `0x008be0c7` (0x00dbaa40).
pub fn flag_name<'a>(flags: &[u32], table: &[&'a str], index: u32) -> Option<&'a str> {
    let slot = (index >> 5) as usize;
    if flags[slot] & (1u32 << (index & 0x1f)) == 0 {
        None
    } else {
        Some(table[index as usize])
    }
}

/// Parse-shaped setter: scan `name` in `table`, set the bit on hit.
/// Returns `1` on hit; on miss returns the game's masked sentinel
/// `found_index & 0xffffff00` (i.e. `0xFFFFFF00` for `-1`) with the bitset untouched.
///
/// Rows (table): `0x0068f6bc` (0x00dc828c), `0x0068f733` (0x00dbc2c8),
/// `0x0068f762` (0x00da5f30), `0x0068f7d9` (0x00dc0b40),
/// `0x006aad23` (0x00dbbe18), `0x008be0f1` (0x00dbaa40).
pub fn set_flag_by_name(flags: &mut [u32], table: &[&str], name: &str) -> i32 {
    let idx = scan_name(table, name);
    if idx < 0 {
        return idx & !0x0000_00ff;
    }
    let slot = (idx as u32 >> 5) as usize;
    flags[slot] |= 1u32 << (idx as u32 & 0x1f);
    1
}

/// Direct bounded index -> name (no bit test): the game rejects `idx < 0` and
/// `idx > max` with NULL; the Rust mirror rejects with `None`. The bound equals
/// `table.len() - 1` in the game (terminator cell just past the max).
///
/// Rows (table, max): `0x00706218` (0x00dbbe18 KindOf, max `0xd9` = 217),
/// `0x00818be3` (0x00dc85c4 auto-resolve kinds, max `7`).
pub fn name_at<'a>(table: &[&'a str], index: i32) -> Option<&'a str> {
    if index < 0 || index as usize >= table.len() {
        return None;
    }
    Some(table[index as usize])
}

/// Clear one bit of the index-addressed bitset block (`this + 0` / `this + 0xe0`
/// are caller-side field offsets; the mirror takes the block itself).
///
/// Rows: `0x008d0f63` (block at `this + 0`), `0x008bf930` (block at `this + 0xe0`).
pub fn clear_flag(flags: &mut [u32], index: u32) {
    let slot = (index >> 5) as usize;
    flags[slot] &= !(1u32 << (index & 0x1f));
}

/// Test one bit of the index-addressed bitset block (game block at `this + 0x94`).
///
/// Row: `0x0044e536`.
pub fn test_flag(flags: &[u32], index: u32) -> bool {
    let slot = (index >> 5) as usize;
    (1u32 << (index & 0x1f)) & flags[slot] != 0
}

/// Single-dword bit test: the bit index is folded with `& 0x1f` (index `0x20` tests
/// bit 0 — the fold is a real game quirk, not a mask of an out-of-range dword).
/// The dword is passed by value because the game reaches it through field offsets
/// (`this + 0x360`) or a pointer hop (`*(this + 8) + 0x4c`).
///
/// Rows: `0x0068adf7` (`this + 0x360`), `0x008d81d7` (`*(this + 8) + 0x4c`).
pub fn test_flag_dword(cell: u32, index: u32) -> bool {
    cell & (1u32 << (index & 0x1f)) != 0
}

/// Set one bit of the index-addressed bitset block (blocks at `this + 0` / `this + 0xe0`).
///
/// Rows: `0x00a1b760` (`this + 0`), `0x008bf911` (`this + 0xe0`).
pub fn set_flag(flags: &mut [u32], index: u32) {
    let slot = (index >> 5) as usize;
    flags[slot] |= 1u32 << (index & 0x1f);
}

/// Dual structure: set bit `index` in the wide mask block **and** bit `index >> 5`
/// in the summary block (the game writes `summary + (index >> 10) * 4` with
/// `1 << ((index >> 5) & 0x1f)` — exactly `set_flag(summary, index >> 5)`).
///
/// Row: `0x00931008` (blocks at `*(this + 4)` / `*(this + 8)`).
pub fn set_flag_pair(mask: &mut [u32], summary: &mut [u32], index: u32) {
    set_flag(mask, index);
    set_flag(summary, index >> 5);
}

/// Test through a nullable index holder: the game passes `param_2 == 0` (NULL
/// holder) as `false`, else tests the bit stored at `*(holder + 0x38)`.
///
/// Row: `0x006aa8ef` (block at `this + 0xbc`).
pub fn test_flag_opt(flags: &[u32], index: Option<u32>) -> bool {
    match index {
        None => false,
        Some(i) => test_flag(flags, i),
    }
}

/// Gated clear: the game clears bit `*(holder + 0x38)` only when `*(holder + 4) == 0`.
///
/// Row: `0x006e15bc` (block at `this + 0x1d8`).
pub fn clear_flag_if(flags: &mut [u32], gate: u32, index: u32) {
    if gate == 0 {
        clear_flag(flags, index);
    }
}

/// One-hot of a stored index cell: `1 << ((byte)cell & 0x1f)` (byte truncation is
/// a no-op under `& 0x1f`).
///
/// Row: `0x00660e1e` (cell at `*(this + 8) + 0x54`).
pub fn one_hot(index_cell: u32) -> u32 {
    1u32 << (index_cell & 0x1f)
}

/// One-hot, complemented when the owner's flag byte is zero.
///
/// Row: `0x00661368` (flag byte `this + 0xc`, index cell `*(this + 8) + 0x54`).
pub fn one_hot_or_not(index_cell: u32, invert_flag: u8) -> u32 {
    let m = one_hot(index_cell);
    if invert_flag == 0 {
        !m
    } else {
        m
    }
}

/// Stride arithmetic: `one_hot(onehot_index_cell) * (count_cell - 1) * 2`, wrapping
/// exactly like the game's unsigned multiply chain.
///
/// Row: `0x0055e830` (cells at `this + 0xfc` / `this + 0xd8`).
pub fn stride_bytes(onehot_index_cell: u32, count_cell: i32) -> u32 {
    (1u32 << (onehot_index_cell & 0x1f)).wrapping_mul((count_cell.wrapping_sub(1)) as u32).wrapping_mul(2)
}

/// One-hot from a 1-based ordinal, keeping the game's signed-`char` truncation:
/// `x = 0x100` truncates to `(char)0` and yields `1 << 31` (wrap of `0 - 1`).
///
/// Row: `0x00a1b710`.
pub fn one_hot_ordinal(x: u32) -> u32 {
    if x == 0 {
        0
    } else {
        1u32 << ((((x as u8) as i8) as u32).wrapping_sub(1) & 0x1f)
    }
}

/// Pair-map scan: stride-2 cells `[{name, value}, …]`, NULL name cell terminates,
/// returns the paired value cell on a case-insensitive hit.
///
/// Row: `0x005db3fe` (table at `0x00bda640`).
pub fn scan_pair<'a, T>(table: &[(&'a str, &'a T)], name: &str) -> Option<&'a T> {
    table.iter().find(|(n, _)| name_eq(n, name)).map(|(_, v)| *v)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABLE: &[&str] = &["OBSTACLE", "SELECTABLE", "IMMOBILE", "CAN_ATTACK"];
    #[test]
    fn scan_is_case_insensitive_and_null_terminated() {
        assert_eq!(scan_name(TABLE, "CAN_ATTACK"), 3);
        assert_eq!(scan_name(TABLE, "can_attack"), 3);
        assert_eq!(scan_name(TABLE, "ObStAcLe"), 0);
        // miss runs off the end (NULL cell) -> -1
        assert_eq!(scan_name(TABLE, "AMPHIBIOUS"), -1);
        assert_eq!(scan_name(TABLE, ""), -1);
    }

    #[test]
    fn name_at_rejects_out_of_range() {
        assert_eq!(name_at(TABLE, 0), Some("OBSTACLE"));
        assert_eq!(name_at(TABLE, 3), Some("CAN_ATTACK"));
        assert_eq!(name_at(TABLE, 4), None); // terminator slot
        assert_eq!(name_at(TABLE, -1), None);
    }

    #[test]
    fn flag_name_requires_the_bit() {
        let flags = [0b100u32, 1u32 << 5];
        assert_eq!(flag_name(&flags, TABLE, 2), Some("IMMOBILE"));
        assert_eq!(flag_name(&flags, TABLE, 0), None);
        // cross-dword: index 37 = slot 1 bit 5, table must cover the index
        let big: Vec<&str> = (0..38).map(|i| if i == 37 { "TAIL" } else { "x" }).collect();
        assert_eq!(flag_name(&flags, &big, 37), Some("TAIL"));
    }

    #[test]
    fn set_by_name_hit_sets_bit() {
        let mut flags = [0u32; 2];
        assert_eq!(set_flag_by_name(&mut flags, TABLE, "can_attack"), 1);
        assert_eq!(flags, [1u32 << 3, 0u32]);
    }

    #[test]
    fn set_by_name_miss_returns_masked_sentinel() {
        let mut flags = [0xDEAD_BEEF, 0x0BAD_F00D];
        let r = set_flag_by_name(&mut flags, TABLE, "NOPE");
        assert_eq!(r, -256); // 0xFFFFFF00: low byte reads 0 (byte-test callers), int truthiness non-zero
        assert_eq!(r as u32, 0xffff_ff00);
        assert_eq!(flags, [0xDEAD_BEEF, 0x0BAD_F00D]);
    }

    #[test]
    fn clear_and_test_roundtrip() {
        let mut flags = [0xffff_ffff, 0xffff_ffff];
        clear_flag(&mut flags, 33);
        assert_eq!(flags, [0xffff_ffff, !(1u32 << 1)]);
        assert!(test_flag(&flags, 32));
        assert!(!test_flag(&flags, 33));
        clear_flag(&mut flags, 0);
        assert_eq!(flags[0], 0xffff_fffe);
    }

    #[test]
    fn dword_test_folds_index_mod_32() {
        assert!(test_flag_dword(1 << 31, 31));
        // game quirk: index 0x20 folds back to bit 0
        assert!(!test_flag_dword(1 << 31, 0x20));
        assert!(test_flag_dword(1, 0x20));
    }

    #[test]
    fn set_flag_and_summary_pair() {
        let mut mask = [0u32; 2];
        set_flag(&mut mask, 37);
        assert_eq!(mask, [0, 1u32 << 5]);
        let mut m = [0u32; 2];
        let mut s = [0u32; 1];
        set_flag_pair(&mut m, &mut s, 37);
        assert_eq!(m, [0, 1u32 << 5]);
        assert_eq!(s, [1u32 << 1]); // bit (37 >> 5) = 1 in the summary
    }

    #[test]
    fn opt_and_gated_variants() {
        let flags = [0u32, 1u32 << 5];
        assert!(test_flag_opt(&flags, Some(37)));
        assert!(!test_flag_opt(&flags, None));
        assert!(!test_flag_opt(&flags, Some(3)));
        let mut f = [0xffff_ffff, 0xffff_ffff];
        clear_flag_if(&mut f, 1, 3); // gate non-zero: untouched
        assert_eq!(f, [0xffff_ffff, 0xffff_ffff]);
        clear_flag_if(&mut f, 0, 3);
        assert_eq!(f[0], 0xffff_fff7);
    }

    #[test]
    fn one_hot_variants() {
        assert_eq!(one_hot(5), 1 << 5);
        assert_eq!(one_hot(0x105), 1 << 5); // byte truncation no-op under &0x1f
        assert_eq!(one_hot_or_not(5, 0), !(1u32 << 5));
        assert_eq!(one_hot_or_not(5, 1), 1u32 << 5);
        assert_eq!(stride_bytes(3, 4), 8 * 3 * 2);
        assert_eq!(stride_bytes(31, 2), (1u32 << 31).wrapping_mul(1).wrapping_mul(2)); // wraps to 0
        // signed-char ordinal quirk: 0x100 truncates to 0, then 0 - 1 wraps to bit 31
        assert_eq!(one_hot_ordinal(0x100), 1u32 << 31);
        assert_eq!(one_hot_ordinal(1), 1);
        assert_eq!(one_hot_ordinal(0), 0);
    }

    #[test]
    fn pair_scan_strides_and_terminates() {
        let vals = [7u32, 9, 11];
        let table = [("Attack", &vals[0]), ("Decay", &vals[1]), ("MinRange", &vals[2])];
        assert_eq!(scan_pair(&table, "decay"), Some(&9));
        assert_eq!(scan_pair(&table, "MINRANGE"), Some(&11));
        assert_eq!(scan_pair(&table, "nope"), None);
    }
}
