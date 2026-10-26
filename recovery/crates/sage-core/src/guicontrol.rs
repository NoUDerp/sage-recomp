//! GUI control template defaults — GUI band (token block
//! `0x00c0bc00..0x00c0c23c`). Two static stores modelled:
//!
//! * `DefaultsStore.raw` mirrors the raw per-control default blobs
//!   `DAT_00e01318(0x10) 00e01328(0x34) 00e0135c(8) 00e01364(8)
//!   00e0136c(0x28) 00e01398(0x54) 00e013f0(0x4c)` selected by the
//!   strcmp-chain 0x00715445 (which additionally `memset(slot,0,4)`s
//!   the selected blob — faithful quirk);
//! * `DefaultsStore.parsed` mirrors the strtok/atoi parse targets
//!   `DAT_00e0143c..00e01494` (0x0071553a), delimiters
//!   `DAT_00c0c23c` = `" \t\n\r"` (read raw at file off
//!   0x7b9400+(0xc0c23c−0xbb9400) → `20 09 0a 0d 00`).
//!
//! The nine indexed setters (0x007143b9..0x007144d8) act on a
//! different object: three stride-0xc tables at bases +0x48 (cells
//! 6..14), +0xb4 (0xf..0x17), +0x120 (0x18..0x20 of a flat cell
//! array) — modelled as three 9×3-u32 tables with the game's
//! `idx < 0 || 8 < idx → -3` guard. Template assignment-operator
//! 0x00713ebd copies the 0x1a8-byte GuiControlTemplate except the +0
//! slot; teardown 0x00715acc releases the four static `StringBase`s
//! at `DAT_00e01308..0x1314`.
//!
//! NOT mirrored from this cluster: 0x007174cb (tail-calls vtable-laden
//! FUN_00716518 + unmodelled cursor helpers FUN_00714046/0x7146dc/
//! 0x9c4acd) and 0x007157be (getters through FUN_00714046).

use crate::string::{Ascii, StringBase};

pub const ERR_RANGE: i32 = -3; // 0xfffffffd

/// The nine zero-call indexed setters; `idx` is the game's i32.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SlotTables {
    /// bases +0x48 (setter 0x007143b9 dword0, 0x007143dd dword1,
    /// 0x007143ff dword2 via +0x48/0x4c/0x50)
    pub a: [[u32; 3]; 9],
    /// +0xb4/0xb8 (0x00714421/0x00714445)
    pub b: [[u32; 3]; 9],
    /// +0x120/0x124/0x128 (0x0071448f/0x007144b3/0x007144d8)
    pub c: [[u32; 3]; 9],
}

macro_rules! setter {
    ($name:ident, $tab:ident, $cell:expr) => {
        pub fn $name(&mut self, idx: i32, v: u32) -> i32 {
            if idx < 0 || 8 < idx {
                return ERR_RANGE;
            }
            self.$tab[idx as usize][$cell] = v;
            0
        }
    };
}
impl SlotTables {
    setter!(set_a0, a, 0); // 0x007143b9  (idx+6)*0xc
    setter!(set_a1, a, 1); // 0x007143dd  idx*0xc+0x4c
    setter!(set_a2, a, 2); // 0x007143ff  idx*0xc+0x50
    setter!(set_b0, b, 0); // 0x00714421  (idx+0xf)*0xc
    setter!(set_b1, b, 1); // 0x00714445  idx*0xc+0xb8
    setter!(set_b2, b, 2); // 0x0071446a  idx*0xc+0xbc
    setter!(set_c0, c, 0); // 0x0071448f  (idx+0x18)*0xc
    setter!(set_c1, c, 1); // 0x007144b3  idx*0xc+0x124
    setter!(set_c2, c, 2); // 0x007144d8  idx*0xc+0x128
}

/// Control name → raw-default slot (0x00715445 strcmp chain, VAs as
/// in the decompiled names).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RawSlot {
    VertHorzSlider, // s_VERTSLIDER_00c0c0c4 / s_HORZSLIDER_00c0c0b8 → 0x10
    ScrollList,     // s_SCROLLLISTBOX_00c0c0a8 → 0x4c
    Tab,            // s_TABCONTROL_00c0c06c → 0x54
    EntryField,     // s_ENTRYFIELD_00c0c09c → 0x28
    StaticText,     // s_STATICTEXT_00c0c090 → 8
    RadioButton,    // s_RADIOBUTTON_00c0c0dc → 8
    Combo,          // s_COMBOBOX_00c0c058 → 0x34
}

/// Parsed per-control defaults (DAT_00e0143c..00e01494 region, one
/// Rust struct instead of overlapping statics; field offsets from the
/// 0x0071553a stores).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ParsedDefaults {
    // slider slot @DAT_00e0143c, size 0x10
    pub slider: [i32; 2], // +0, +4
    // scroll-list slot @DAT_00e01488, size 0x4c
    pub list_size: i16,        // _DAT_00e01488 (+0, i16 store)
    pub list_flags: [bool; 5], // DAT_00e01490..0x94 (+8..+0xc)
    // entry-field slot @DAT_00e0145c, size 0x28
    pub entry_flags: u32, // _DAT_00e01468 (+0xc)
    pub entry_width: i16, // _DAT_00e0146c (+0x10, i16 store)
    pub entry_bool: bool, // DAT_00e0146e (+0x12)
    // static-text slot @DAT_00e01454, size 8
    pub static_flag: bool, // DAT_00e01458 (+4)
    // radio slot @DAT_00e0144c, size 8
    pub radio_value: i32, // _DAT_00e01450 (+4)
}

#[derive(Clone, Debug, Default)]
pub struct DefaultsStore {
    pub raw: [Vec<u8>; 7], // sized 0x10,0x34,8,8,0x28,0x54,0x4c (VertHorz, Combo, Radio, Static, Entry, Tab, List)
    pub parsed: ParsedDefaults,
}

fn raw_sizes() -> [usize; 7] {
    [0x10, 0x34, 8, 8, 0x28, 0x54, 0x4c]
}

impl DefaultsStore {
    pub fn new() -> Self {
        Self {
            raw: raw_sizes().map(|n| vec![0; n]),
            parsed: ParsedDefaults::default(),
        }
    }

    /// 0x00715445 — name lookup in chain order (VERTSLIDER, HORZSLIDER
    /// share slot; unknown → None, which the game signals by returning
    /// NULL without writing `*param_2`). Quirk kept: the selected blob
    /// gets `memset(.,0,4)` before being returned, and `out_size` is
    /// written only when the game's `param_2 != 0` (None here means the
    /// caller passed NULL — we always write when Some).
    pub fn lookup_raw(&mut self, name: &[u8], out_size: &mut usize) -> Option<&mut [u8]> {
        let name = ncstr(name);
        let slot = if name == b"VERTSLIDER" || name == b"HORZSLIDER" {
            RawSlot::VertHorzSlider
        } else if name == b"SCROLLLISTBOX" {
            RawSlot::ScrollList
        } else if name == b"TABCONTROL" {
            RawSlot::Tab
        } else if name == b"ENTRYFIELD" {
            RawSlot::EntryField
        } else if name == b"STATICTEXT" {
            RawSlot::StaticText
        } else if name == b"RADIOBUTTON" {
            RawSlot::RadioButton
        } else if name == b"COMBOBOX" {
            RawSlot::Combo
        } else {
            return None;
        };
        let i = match slot {
            RawSlot::VertHorzSlider => 0,
            RawSlot::Combo => 1,
            RawSlot::RadioButton => 2,
            RawSlot::StaticText => 3,
            RawSlot::EntryField => 4,
            RawSlot::Tab => 5,
            RawSlot::ScrollList => 6,
        };
        *out_size = raw_sizes()[i];
        self.raw[i][..4].fill(0); // memset(_Dst,0,4)
        Some(&mut self.raw[i])
    }
}

/// Game names arrive NUL-terminated (strcmp chains); accept either form.
fn ncstr(s: &[u8]) -> &[u8] {
    match s.last() {
        Some(0) => &s[..s.len() - 1],
        _ => s,
    }
}

/// strtok over the game's delimiter set `" \t\n\r"` (DAT_00c0c23c):
/// consecutive delimiters collapse; returns successive tokens.
struct Strtok<'a> {
    s: &'a [u8],
    pos: usize,
}
impl<'a> Strtok<'a> {
    fn new(s: &'a [u8]) -> Self {
        Self { s, pos: 0 }
    }
    fn next(&mut self) -> Option<&'a [u8]> {
        let is_d = |c: u8| c == b' ' || c == b'\t' || c == b'\n' || c == b'\r';
        while self.pos < self.s.len() && is_d(self.s[self.pos]) {
            self.pos += 1;
        }
        if self.pos >= self.s.len() {
            return None;
        }
        let start = self.pos;
        while self.pos < self.s.len() && !is_d(self.s[self.pos]) {
            self.pos += 1;
        }
        Some(&self.s[start..self.pos])
    }
}

/// CRT atoi; the game crashes (NULL deref) when a token is missing —
/// the mirror yields 0 and is documented rather than panicking.
fn atoi(t: Option<&[u8]>) -> i32 {
    let t = match t {
        Some(t) => t,
        None => return 0,
    };
    let mut i = 0;
    while i < t.len() && matches!(t[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
        i += 1;
    }
    let neg = if i < t.len() && (t[i] == b'-' || t[i] == b'+') {
        let n = t[i] == b'-';
        i += 1;
        n
    } else {
        false
    };
    let mut v: i32 = 0;
    while i < t.len() && t[i].is_ascii_digit() {
        v = v.wrapping_mul(10).wrapping_add((t[i] - b'0') as i32);
        i += 1;
    }
    if neg {
        v.wrapping_neg()
    } else {
        v
    }
}

/// 0x0071553a — parse the token string for the named control into the
/// static parsed region; returns the slot whose defaults were written
/// (None = unknown control, game writes `*param_2 = 0`). Always
/// returns Ok(()) — the game returns 1 unconditionally.
pub fn parse_control_defaults(
    store: &mut DefaultsStore,
    name: &[u8],
    tokens: &[u8],
) -> Option<ParsedSlot> {
    let name = ncstr(name);
    let mut tk = Strtok::new(tokens);
    let p = &mut store.parsed;
    if name == b"VERTSLIDER" || name == b"HORZSLIDER" {
        // memset(&DAT_00e0143c,0,0x10)
        p.slider = [0; 2];
        p.slider[0] = atoi(tk.next());
        p.slider[1] = atoi(tk.next());
        Some(ParsedSlot::Slider)
    } else if name == b"SCROLLLISTBOX" {
        // memset(&DAT_00e01488,0,0x4c)
        p.list_size = 0;
        p.list_flags = [false; 5];
        p.list_size = atoi(tk.next()) as i16;
        for f in p.list_flags.iter_mut() {
            *f = atoi(tk.next()) != 0;
        }
        Some(ParsedSlot::ScrollList)
    } else if name == b"ENTRYFIELD" {
        // memset(&DAT_00e0145c,0,0x28)
        p.entry_flags = 0;
        p.entry_width = 0;
        p.entry_bool = false;
        p.entry_width = atoi(tk.next()) as i16;
        tk.next(); // second token skipped by the game
        p.entry_bool = match tk.next() {
            None => false,
            Some(t) => atoi(Some(t)) != 0,
        };
        // _DAT_00e01468 = 0 then flag bits from the 4th token
        let t = tk.next();
        if t.is_some() {
            if atoi(t) == 1 {
                p.entry_flags |= 0x20;
            }
            if atoi(t) == 2 {
                p.entry_flags |= 0x40;
            }
            if atoi(t) == 3 {
                p.entry_flags |= 0x10;
            }
        }
        Some(ParsedSlot::EntryField)
    } else if name == b"STATICTEXT" {
        p.static_flag = atoi(tk.next()) != 0;
        tk.next();
        Some(ParsedSlot::StaticText)
    } else if name == b"RADIOBUTTON" {
        p.radio_value = atoi(tk.next());
        Some(ParsedSlot::RadioButton)
    } else {
        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParsedSlot {
    Slider,
    ScrollList,
    EntryField,
    StaticText,
    RadioButton,
}

/// GuiControlTemplate, 0x1a8 bytes (assignment-op 0x00713ebd); the +0
/// slot (RTTI/type ptr) is never copied.
#[derive(Clone, Debug)]
pub struct GuiTemplate {
    pub hdr: [u32; 5],                 // +0x04..+0x14
    pub blocks: [[f32; 27]; 3],        // +0x18 / +0x84 / +0xf0, 9 triples
    pub tail: [u32; 11],               // +0x15c..+0x184
    pub names: [StringBase<Ascii>; 4], // +0x188..+0x194
    pub tail2: [u32; 4],               // +0x198..+0x1a4
}

impl GuiTemplate {
    /// 0x00713ebd — memberwise copy (three 9×3 float blocks, the 5+11
    /// +4 scalar dwords, and four `StringBase::set`).
    pub fn assign_from(&mut self, o: &Self) {
        self.hdr = o.hdr;
        self.blocks = o.blocks;
        self.tail = o.tail;
        for i in 0..4 {
            self.names[i].set_string_base(&o.names[i]);
        }
        self.tail2 = o.tail2;
    }
}

/// The four static GUI layout strings `DAT_00e01308..0x1314`.
#[derive(Clone, Debug, Default)]
pub struct GuiStaticStrings(pub [StringBase<Ascii>; 4]);

impl GuiStaticStrings {
    /// 0x00715acc — release all four buffers.
    pub fn release_all(&mut self) {
        for s in self.0.iter_mut() {
            s.release_buffer();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setter_guard_and_slots() {
        let mut t = SlotTables::default();
        assert_eq!(t.set_a0(0, 7), 0);
        assert_eq!(t.a[0][0], 7);
        assert_eq!(t.set_a0(-1, 1), ERR_RANGE);
        assert_eq!(t.set_a0(9, 1), ERR_RANGE);
        assert_eq!(t.a[0][0], 7);
        assert_eq!(t.set_c2(8, 0xdead), 0);
        assert_eq!(t.c[8][2], 0xdead);
        assert_eq!(t.set_b1(3, 5), 0);
        assert_eq!(t.b[3][1], 5);
    }

    #[test]
    fn raw_lookup_sizes_and_memset_quirk() {
        let mut st = DefaultsStore::new();
        st.raw[0][..4].copy_from_slice(&[1, 2, 3, 4]);
        let mut size = 0usize;
        let s = st.lookup_raw(b"HORZSLIDER\0", &mut size).unwrap();
        assert_eq!(size, 0x10);
        assert_eq!(&s[..4], &[0, 0, 0, 0]); // memset(_Dst,0,4)
        assert!(st.lookup_raw(b"COMBOBOX\0", &mut size).is_some());
        assert_eq!(size, 0x34);
        assert!(st.lookup_raw(b"NOPE\0", &mut size).is_none());
    }

    #[test]
    fn parse_branches() {
        let mut st = DefaultsStore::new();
        assert_eq!(
            parse_control_defaults(&mut st, b"VERTSLIDER\0", b"12 -3").unwrap(),
            ParsedSlot::Slider
        );
        assert_eq!(st.parsed.slider, [12, -3]);
        parse_control_defaults(&mut st, b"SCROLLLISTBOX\0", b"300\t1 0\n1 0 1").unwrap();
        assert_eq!(st.parsed.list_size, 300);
        assert_eq!(st.parsed.list_flags, [true, false, true, false, true]);
        parse_control_defaults(&mut st, b"ENTRYFIELD\0", b"40 tok1 1 2").unwrap();
        assert_eq!(st.parsed.entry_width, 40);
        assert!(st.parsed.entry_bool);
        assert_eq!(st.parsed.entry_flags, 0x40);
        assert!(parse_control_defaults(&mut st, b"COMBOBOX\0", b"1").is_none());
    }

    #[test]
    fn template_assign_copies_everything_but_type_slot() {
        fn sb(s: &[u8]) -> StringBase<Ascii> {
            let mut b = StringBase::new();
            b.set(&s.iter().copied().map(Ascii).collect::<Vec<_>>());
            b
        }
        let mut a = GuiTemplate {
            hdr: [0; 5],
            blocks: [[0.0; 27]; 3],
            tail: [0; 11],
            names: [sb(b""), sb(b""), sb(b""), sb(b"")],
            tail2: [0; 4],
        };
        let b = GuiTemplate {
            hdr: [1, 2, 3, 4, 5],
            blocks: [[7.5; 27]; 3],
            tail: [9; 11],
            names: [sb(b"tag"), sb(b"tag"), sb(b"tag"), sb(b"tag")],
            tail2: [6, 7, 8, 9],
        };
        a.assign_from(&b);
        assert_eq!(a.hdr, [1, 2, 3, 4, 5]);
        assert_eq!(a.blocks[1][26], 7.5);
        assert_eq!(a.tail2, [6, 7, 8, 9]);
        assert_eq!(
            a.names[2].str().iter().map(|c| c.0).collect::<Vec<u8>>(),
            b"tag"
        );
    }
}
