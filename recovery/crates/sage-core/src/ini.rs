//! INI subsystem core — mirror of the function band 0x0042b900..0x0042f900.
//!
//! The game's `INI` object is a singleton-ish parser: a global line table
//! (file-read subsystem at 0x00a00c34/0x00a018b6, NOT mirrored here — line
//! text is supplied to [`Ini::new`] instead) plus a `strtok`-driven token
//! pump. Every `throw` below becomes an [`IniError`] with the game's level
//! and verbatim message (formats from the `.rdata` strings, addresses in
//! comments). The C `strtok` global state is modelled per-object inside
//! [`Ini`] — the game relies on it being seeded by the parse driver
//! (0x0042d7b2) and consumed by `nextToken` (0x0042def7); the save pointer
//! indexes the in-place line buffer exactly as the game's does.
//!
//! Not mirrored (platform/adjacent): the fopen legs of the open
//! pipeline (0x00a00c34/0x00a02072/0x00a01bbc — [`Ini::open`] takes the
//! line vector directly; same for the `gi.dat` fopen half of
//! 0x0042f840), the line-echo/log sinks (0x0042ccfc, DAT_00ddf57c),
//! the per-module block-handler registrars, and the `_mappedMixerFile`
//! Win32 shared-memory path (0x0042cf90).

use std::collections::HashMap;

/// INIException object `{char* message; int level}` (0x0042f681 builds it;
/// `_CxxThrowException(&obj, &DAT_00cfe2fc)` throws it).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IniError {
    pub level: u8,
    pub message: String,
}

impl IniError {
    pub(crate) fn new(level: u8, message: impl Into<String>) -> Self {
        Self {
            level,
            message: message.into(),
        }
    }
    /// Raw `int 1` throw against `DAT_00cfe2c8` (0x0042bc6e, 0x0042ec6e,
    /// 0x0042f1dc, 0x0042f21a): not an INIException. Surfaced as a level-0
    /// error with the empty message — callers treat that pair as the
    /// internal-assert path.
    pub(crate) fn internal() -> Self {
        Self {
            level: 0,
            message: String::new(),
        }
    }
    /// Like `internal`, but carrying a diagnostic message for the model's
    /// own unmodelled-dispatch guards (no game site raises these).
    pub(crate) fn internal_msg(message: impl Into<String>) -> Self {
        Self {
            level: 0,
            message: message.into(),
        }
    }
}

pub type IniResult<T> = Result<T, IniError>;

/// Delimiter strings stored by the INI ctor 0x0042cdba at +0x418..+0x42c
/// (bytes verified in `.rdata`).
pub const DELIMS: &str = " \n\r\t="; // +0x418 @0x00bbdd00
pub const DELIMS_PCT: &str = " \n\r\t=%%"; // +0x41c @0x00bbdcf8
pub const DELIMS_COLON: &str = " \n\r\t=:"; // +0x420 @0x00bbdcf0
pub const DELIMS_QUOTE: &str = "\"\n="; // +0x424 @0x00bbdcec
pub const END_TOKEN: &str = "END"; // +0x428 @0x00bbdce8
pub const END_SCRIPT_TOKEN: &str = "ENDSCRIPT"; // +0x42c @0x00bbdcdc

/// The INI object (layout from ctor 0x0042cdba + member use across the
/// band; offsets in comments).
pub struct Ini {
    /// +0x04 filename (set by the unmirrored open path 0x0042d2cb).
    pub filename: String,
    /// +0x08 load mode (mode==5 gates the MACRO pre-pass; not mirrored).
    pub mode: i32,
    /// +0x0c current line index into `lines`.
    pub current_line: usize,
    /// +0x10 next line index.
    pub next_line: usize,
    /// +0x14 line buffer (strncpy cap 0x403, NUL @+0x417).
    pub line: String,
    /// +0x430 EOF flag.
    pub eof: bool,
    /// +0x431 current block name (error context).
    pub cur_block_name: String,
    /// +0x86c pending-data string buffer (set via 0x0042cbd6 up-case path).
    pub pending: Option<String>,
    /// The global MACRO dictionary (c9e7 find / cd2f insert; the #define
    /// pre-pass 0x0042d2cb fills it in the game — here it is public state).
    pub macros: HashMap<String, (String, String)>,
    /// Global line table (FUN_009d5847 count / FUN_00a018b6 text).
    lines: Vec<String>,
    /// C `strtok` state: save index into `line` + seeded flag. The save
    /// pointer indexes the buffer in place, so re-filling `line` without
    /// reseeding behaves byte-identically to the game's dangling-in-place
    /// pointer.
    tok_save: usize,
    /// Block-handler registry (`DAT_00ddf578` linked list; handlers
    /// registered by static-init registrars, prepend order).
    pub blocks: Vec<&'static BlockHandler>,
    /// Global macro-table version counter (`DAT_00ddf5ac`), bumped on
    /// mode-5 MACRO overwrite.
    pub macros_version: u32,
}

/// A registered INI block handler (`{next, name, fn}` node of the
/// `DAT_00ddf578` list).
pub struct BlockHandler {
    pub name: &'static str,
    pub handler: fn(&mut Ini) -> IniResult<()>,
}

/// Mode-5 open gate state (globals `DAT_00ddf5b0/5ac/5b8` + the
/// 0x0042c025/0x0042cb98 gate outcomes; callers compute them from file
/// metadata).
#[derive(Default, Clone, Copy)]
pub struct OpenGate {
    /// macro table clean (`DAT_00ddf5b0 == DAT_00ddf5ac`)
    pub clean: bool,
    /// 0x0042c025: file mtime at/after the gate date
    pub file_newer: bool,
    /// 0x0042cb98: file is the current macro source
    pub macro_source: bool,
}

/// `struct tm`-style date stamp for [`Ini::file_newer_than`]
/// (0x0042c025; year = `tm_year+1900`, mon 1..12).
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct DateStamp {
    pub year: u32,
    pub mon: u32,
    pub mday: u32,
    pub hour: u32,
    pub min: u32,
    pub sec: u32,
}

/// The ten `gi.dat` global slots assigned by 0x0042f720 (game defaults
/// are `GI.DAT ERROR` pointers until a matching key lands).
#[derive(Default, Clone)]
pub struct GiValues {
    pub sku_name: Option<String>,
    pub game_name: Option<String>,
    pub game_reg_path: Option<String>,
    pub installer_reg_path: Option<String>,
    pub online_server: Option<String>,
    pub user_data_leaf_name: Option<String>,
    pub g1: Option<String>,
    pub g2: Option<String>,
    pub g3: Option<String>,
    pub g4: Option<String>,
}

/// 0x0042f720: `_strcmpi` chain over the ten known keys; first match
/// assigns the global (unknown keys are dropped silently).
pub fn set_gi_value(v: &mut GiValues, key: &str, value: &str) {
    let slot = [
        ("SkuName", &mut v.sku_name),
        ("GameName", &mut v.game_name),
        ("GameRegPath", &mut v.game_reg_path),
        ("InstallerRegPath", &mut v.installer_reg_path),
        ("OnlineServer", &mut v.online_server),
        ("UserDataLeafName", &mut v.user_data_leaf_name),
        ("G1", &mut v.g1),
        ("G2", &mut v.g2),
        ("G3", &mut v.g3),
        ("G4", &mut v.g4),
    ];
    for (k, f) in slot {
        if key.eq_ignore_ascii_case(k) {
            *f = Some(value.to_string());
            return; // game chain: first match wins
        }
    }
}

/// 0x0042f840 `gi.dat` reader (the fopen leg — path global
/// `DAT_00de01d0` + `gi.dat` (@0x00bbe448), mode `rt` — is the
/// caller's; here the bytes arrive directly): whole-file read, magic
/// dword `GI␠␠` (0x20204947), count dword at +4, then `count`
/// NUL-terminated key/value pairs from +8 fed to [`set_gi_value`].
/// Magic mismatch / unopenable file → default (all-`None`) values.
pub fn parse_gi_dat(bytes: &[u8]) -> GiValues {
    let mut out = GiValues::default();
    if bytes.len() < 8 || bytes[..4] != *b"GI  " {
        return out;
    }
    let count = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
    let mut p = 8usize;
    for _ in 0..count {
        // game walks NUL-terminated strings past the buffer end on
        // malformed files; here a missing key NUL stops the loop
        let key_end = match bytes[p..].iter().position(|c| *c == 0) {
            Some(i) => p + i,
            None => break,
        };
        let key = &bytes[p..key_end];
        let val_start = key_end + 1;
        let val_end = match bytes[val_start..].iter().position(|c| *c == 0) {
            Some(i) => val_start + i,
            None => bytes.len(),
        };
        let val = &bytes[val_start..val_end];
        // game ran raw bytes through the setter; lossy keeps CI key
        // matching honest for the ASCII key space
        set_gi_value(
            &mut out,
            &String::from_utf8_lossy(key),
            &String::from_utf8_lossy(val),
        );
        p = val_end + 1;
    }
    out
}
impl Ini {
    /// 0x0042cdba `INI::INI`: empty filename/mode, zeroed line indexes,
    /// empty line + block name, EOF clear. (Line-table global init
    /// FUN_00a01bbc is the `lines` argument.)
    pub fn new(lines: Vec<String>) -> Self {
        Self {
            filename: String::new(),
            mode: 0,
            current_line: 0,
            next_line: 0,
            line: String::new(),
            eof: false,
            cur_block_name: String::new(),
            pending: None,
            macros: HashMap::new(),
            macros_version: 0,
            lines,
            tok_save: 0,
            blocks: Vec::new(),
        }
    }

    /// 0x0042bf4a `INI::clearState` (post-load): filename empty, mode and
    /// both line indexes zero, EOF clear, echo target cleared (unmirrored).
    pub fn clear_state(&mut self) {
        self.filename = String::new();
        self.mode = 0;
        self.current_line = 0;
        self.next_line = 0;
        self.eof = false;
    }

    /// 0x0042d669 `nextLine`: fill +0x14 from the line table (truncated to
    /// 0x403), advance indexes, release +0x86c; EOF sets +0x430 and blanks
    /// the line. Line-log sink + echo virtual call are unmirrored.
    pub fn advance_line(&mut self) {
        if self.next_line < self.lines.len() {
            self.current_line = self.next_line;
            self.next_line += 1;
            let l = self.lines[self.current_line].clone();
            self.line = l.chars().take(0x403).collect();
            self.pending = None; // releaseBuffer(+0x86c)
            self.tok_save = 0; // game: driver re-seeds `strtok(line)` each
                               // iteration (0x0042d7b2); getters' `strtok(NULL)` then scans
                               // the rest of the line from position 0.
        } else {
            self.eof = true;
            self.line = String::new();
        }
    }

    /// 0x0042dc7f `INI::readFile` — the block-handler driver: opens the
    /// file via the 0x0042d2c1 thunk (-> 0x0042d2cb, mirrored by
    /// `open`), then while EOF (`+0x430`) is clear runs one line step
    /// (0x0042d669 `advance_line`) plus one block dispatch (0x0042c0f5
    /// thunk -> 0x0042c0ff `dispatch_block`), finally clears state
    /// (0x0042bf4a `clear_state`). The game loop tests EOF *before*
    /// advancing, so one dispatch still runs on the blank EOF line — a
    /// no-op there (`seed_tok` finds no token). Open failure releases
    /// the scratch filename buffer and returns false with line state
    /// untouched. The echo-sink global `DAT_00ddf57c` (driver arg at
    /// +0x10, consumed by `advance_line`'s log leg) and string-init leg
    /// 0x00440ea9 are unmirrored.
    pub fn read_file(
        &mut self,
        lines: Vec<String>,
        filename: &str,
        mode: i32,
        gate: &OpenGate,
    ) -> Result<bool, IniError> {
        if !self.open(lines, filename, mode, gate)? {
            return Ok(false); // gate fail: releaseBuffer(scratch), return 0
        }
        while !self.eof {
            self.advance_line();
            self.dispatch_block()?;
        }
        self.clear_state();
        Ok(true)
    }

    /// `strtok(line, delim)` reseed — the parse driver 0x0042d7b2 calls it
    /// with the non-null line buffer each iteration.
    pub fn seed_tok(&mut self, delim: &str) -> Option<String> {
        self.tok_save = 0;
        self.strtok_next(delim)
    }

    /// C `strtok(NULL, delim)` over `self.line` from `tok_save`.
    fn strtok_next(&mut self, delim: &str) -> Option<String> {
        let b = self.line.as_bytes();
        let mut i = self.tok_save;
        while i < b.len() && delim.as_bytes().contains(&b[i]) {
            i += 1;
        }
        if i >= b.len() {
            return None;
        }
        let start = i;
        while i < b.len() && !delim.as_bytes().contains(&b[i]) {
            i += 1;
        }
        // C strtok NULs the terminating delimiter; save lands PAST it,
        // so a later strtok with a different delimiter set cannot
        // re-swallow that char (e949 mixes +0x418 and +0x424 legs)
        self.tok_save = i + 1;
        Some(self.line[start..i].to_string())
    }

    /// 0x0042def7 `nextToken`: if +0x86c is non-empty, take the next token
    /// out of it (StringBase::nextToken; token found → remember it,
    /// not found → release +0x86c and fall through); else `strtok(NULL,
    /// delim.unwrap_or(+0x418))`.
    pub fn next_token(&mut self, delim: Option<&str>) -> Option<String> {
        let d = delim.unwrap_or(DELIMS);
        if let Some(p) = self.pending.take() {
            if !p.is_empty() {
                if let Some(t) = strtok_over(&p, d) {
                    return Some(t);
                }
                // not found: releaseBuffer(+0x86c), fall to strtok
            }
        }
        self.strtok_next(d)
    }

    /// 0x0042df97 `nextTokenOrThrow` ("get next data"): null token throws
    /// level 3 `Expected additional data after '%s'` — with the *delim
    /// string* as the argument (game quirk: param_2/`+0x418` fallback,
    /// not a token).
    pub fn token_or_throw(&mut self, delim: Option<&str>) -> IniResult<String> {
        let d: &str = match delim {
            Some(d) => d,
            None => DELIMS,
        };
        match self.next_token(delim) {
            Some(t) => Ok(t),
            None => Err(IniError::new(
                3,
                format!("Expected additional data after '{d}'"), // @0x00bbdfd8
            )),
        }
    }

    /// 0x0042d0b3 label resolve: tokens not starting with a digit are
    /// looked up case-insensitively in the global MACRO dictionary
    /// (0x0042c9e7); found → pair.second (node+8 string), else the
    /// original token. Digit-leading tokens skip the lookup entirely.
    pub fn resolve_label(&self, tok: &str) -> String {
        match tok.as_bytes().first() {
            Some(c) if c.is_ascii_digit() => tok.to_string(),
            _ => match self.macros.get(&ci_key(tok)) {
                Some((_, v)) => v.clone(),
                None => tok.to_string(),
            },
        }
    }

    /// 0x0042d14a bool: resolve, `_strcmpi` vs "yes"/"no" (0x00bbdd50 /
    /// 0x00bbdd4c — lower-case literals; comparison is case-insensitive).
    /// Throw message formats the *unresolved* token (game passes
    /// param_1, verified 0x0042d165).
    pub fn parse_bool(&self, tok: &str) -> IniResult<bool> {
        let r = self.resolve_label(tok);
        if r.eq_ignore_ascii_case("yes") {
            Ok(true)
        } else if r.eq_ignore_ascii_case("no") {
            Ok(false)
        } else {
            Err(IniError::new(
                3,
                format!("invalid boolean token {tok} -- expected Yes or No"), // @0x00bbdd1c
            ))
        }
    }

    /// 0x0042e850: `df97 → d14a`.
    pub fn get_bool(&mut self) -> IniResult<bool> {
        let t = self.token_or_throw(None)?;
        self.parse_bool(&t)
    }

    /// 0x0042eccf `getInt`: resolve; `#`-prefixed → math evaluator
    /// 0x0042e0c9→0x0042e0d3; else `sscanf %d` — conversion fails (no
    /// digits) → throw level 3. Trailing garbage after digits is ignored
    /// (C `sscanf` behavior).
    pub fn parse_i32(&mut self, tok: &str) -> IniResult<i32> {
        let r = self.resolve_label(tok);
        if r.starts_with('#') {
            return math_eval(self, |ini, t| ini.parse_i32(t));
        }
        match c_strtol(&r) {
            Some(v) => Ok(v as i32), // C %d into int (wrap on overflow)
            None => Err(IniError::new(
                3,
                format!(
                    "Expected signed integer value, math op, or predefined macro, but found '{r}'"
                ), // @0x00bbe118
            )),
        }
    }

    /// 0x0042ed3a `getUnsignedInt`: same shape, `sscanf %u` — C `%u`
    /// accepts a leading `-` and wraps mod 2^32.
    pub fn parse_u32(&mut self, tok: &str) -> IniResult<u32> {
        let r = self.resolve_label(tok);
        if r.starts_with('#') {
            return math_eval(self, |ini, t| ini.parse_u32(t));
        }
        match c_strtoul(&r) {
            Some(v) => Ok(v as u32),
            None => Err(IniError::new(
                3,
                format!(
                    "Expected unsigned integer value, math op, or predefined macro, but found '{r}'"
                ), // @0x00bbe168
            )),
        }
    }

    /// 0x0042eda5 `getReal`: same shape, `sscanf %f`; the game returns it
    /// as `long double` — modeled as f64, stored as f32 at field time.
    pub fn parse_real(&mut self, tok: &str) -> IniResult<f64> {
        let r = self.resolve_label(tok);
        if r.starts_with('#') {
            return math_eval(self, |ini, t| ini.parse_real(t));
        }
        match c_strtod(&r) {
            Some(v) => Ok(v),
            None => Err(IniError::new(
                3,
                format!(
                    "Expected floating point value, math op, or predefined macro, but found '{r}'"
                ), // @0x00bbe1c0
            )),
        }
    }

    /// 0x0042ef72 `getUnsignedBounded`: `df97 → ed3a`, then
    /// `bound != 0 && bound < v` → throw level 3.
    pub fn get_u32_bounded(&mut self, bound: u32) -> IniResult<u32> {
        let t = self.token_or_throw(None)?;
        let v = self.parse_u32(&t)?;
        if bound != 0 && bound < v {
            return Err(IniError::new(
                3,
                format!("value out of range, expected 0..{bound}"), // @0x00bbe2b0
            ));
        }
        Ok(v)
    }

    /// 0x0042ee22 / 0x0042ee6d / 0x0042eeb9 / 0x0042ef09: signed/unsigned
    /// narrow int getters — range check after `eccf`, message verbatim.
    pub fn get_i8(&mut self) -> IniResult<i8> {
        let t = self.token_or_throw(None)?;
        let v = self.parse_i32(&t)?;
        i8::try_from(v).map_err(|_| IniError::new(3, "value out of range, expected -128..127"))
        // @0x00bbe210
    }
    pub fn get_u8(&mut self) -> IniResult<u8> {
        let t = self.token_or_throw(None)?;
        let v = self.parse_i32(&t)?;
        u8::try_from(v).map_err(|_| IniError::new(3, "value out of range, expected 0..255"))
        // @0x00bbe238
    }
    pub fn get_i16(&mut self) -> IniResult<i16> {
        let t = self.token_or_throw(None)?;
        let v = self.parse_i32(&t)?;
        i16::try_from(v).map_err(|_| IniError::new(3, "value out of range, expected -32768..32767"))
        // @0x00bbe25c
    }
    pub fn get_u16(&mut self) -> IniResult<u16> {
        let t = self.token_or_throw(None)?;
        let v = self.parse_i32(&t)?;
        u16::try_from(v).map_err(|_| IniError::new(3, "value out of range, expected 0..65535"))
        // @0x00bbe288
    }

    /// 0x0042ee10 `getDuration`: `eda5 × DAT_00bcf628` (= f32 0.01).
    pub fn get_duration(&mut self) -> IniResult<f32> {
        let t = self.token_or_throw(None)?;
        let v = self.parse_real(&t)?;
        Ok(v as f32 * 0.01f32)
    }

    /// 0x0042bcab `scanIndexList`: `_strcmpi` walk of a NULL-terminated
    /// name list; miss with `throw_on_miss` → level 3 (via 0x0042bd39);
    /// empty list → level 2 internal. Returns `None` on soft miss.
    pub fn scan_index(tok: &str, names: &[&str], throw_on_miss: bool) -> IniResult<usize> {
        if names.is_empty() {
            return Err(IniError::new(
                2,
                "INTERNAL ERROR! scanIndexList: No name list provided!", // @0x00bbdaec
            ));
        }
        for (i, n) in names.iter().enumerate() {
            if tok.eq_ignore_ascii_case(n) {
                return Ok(i);
            }
        }
        if throw_on_miss {
            return Err(IniError::new(
                3,
                format!("Token '{tok}' is not a valid member of the index list"), // @0x00bbdb24
            ));
        }
        Ok(0) // soft-miss returns 0 with *found = 0
    }

    /// 0x0042ec4e `getEnum`: `df97 → 0x0042bd39` (always-throw scan).
    pub fn get_enum(&mut self, names: &[&str]) -> IniResult<i32> {
        let t = self.token_or_throw(None)?;
        Ok(Self::scan_index(&t, names, true)? as i32)
    }
    /// 0x0042ef56: `df97 → eccf` token-or-throw int getter.
    pub fn get_i32(&mut self) -> IniResult<i32> {
        let t = self.token_or_throw(None)?;
        self.parse_i32(&t)
    }
    /// 0x0042efc0: `df97 → eda5` token-or-throw real getter.
    pub fn get_real(&mut self) -> IniResult<f64> {
        let t = self.token_or_throw(None)?;
        self.parse_real(&t)
    }
    /// 0x0042ea59 (body behind the `0x0042f11e`/`0x0042ea4f` shell pair):
    /// `nextTokenOrThrow(NULL,+0x418)`; if the token opens with `"`,
    /// strip the quote, take ONE more token with the `+0x424`
    /// (`"\\n=`) set: with a continuation token, join with `" "`
    /// (@0x00bbd40c) **unless** `strlen(tok) > 1 && tok[1] == '\\t'`,
    /// appending it raw; without one, strip a single trailing `"`.
    /// Non-quoted tokens pass through verbatim. `None` (null token) →
    /// no assignment at all (`deed`/`f11e` leave the destination set).
    pub fn get_quoted_string(&mut self) -> IniResult<Option<String>> {
        let t = self.token_or_throw(None)?;
        if !t.starts_with('"') {
            return Ok(Some(t));
        }
        let mut scratch = String::from(&t[1..]);
        match self.token_or_throw(Some(DELIMS_QUOTE)) {
            Ok(cont) => {
                // mbslen > 1 and second byte != TAB → space join
                if cont.len() > 1 && cont.as_bytes()[1] != b'\t' {
                    scratch.push(' '); // @0x00bbd40c " "
                }
                scratch.push_str(&cont);
            }
            Err(e) if e.level == 3 && e.message.starts_with("Expected additional data") => {
                // deed returned null: trim one trailing quote from scratch
                if scratch.ends_with('"') {
                    scratch.pop();
                }
            }
            Err(e) => return Err(e),
        }
        Ok(Some(scratch))
    }

    /// 0x0042e06b `expectToken` + value fetch: next token (delims = the
    /// `+0x420` colon set — the decomp's single visible `df97` argument),
    /// `_strcmpi` vs label → mismatch throws level 3
    /// `Expected '%s' but found '%s'` (label first); then ONE more token
    /// is fetched and **returned in EAX** — callers (`0x0042f259`,
    /// `0x0042f507`) feed it straight to the typed leaf, so the second
    /// read is the value, not a separator to drop. (`R = 255` tokenizes
    /// as label `R` + value `255` since `=` is a delimiter in the set.)
    pub fn expect_value(&mut self, label: &str) -> IniResult<String> {
        let t = self.token_or_throw(Some(DELIMS_COLON))?;
        if !t.eq_ignore_ascii_case(label) {
            return Err(IniError::new(
                3,
                format!("Expected '{label}' but found '{t}'"), // @0x00bbdffc
            ));
        }
        self.token_or_throw(Some(DELIMS_COLON))
    }

    /// 0x0042f259 `getRGBf`: three `expect('R'/'G'/'B')` + `eccf` legs;
    /// each int checked 0..=255 *after* storing (throw level 3, message
    /// from 0x00bbe35a+2, i.e. without the `zD` literal prefix); output =
    /// int × DAT_00bbb8f0 (f32 1/255).
    pub fn get_rgb_f(&mut self) -> IniResult<[f32; 3]> {
        const SCALE: f32 = 0.003921568859368563; // DAT_00bbb8f0
        const LABELS: [&str; 3] = ["R", "G", "B"]; // DAT_00bbe388/384/bbb798
        let mut v = [0i32; 3];
        for i in 0..3 {
            let t = self.expect_value(LABELS[i])?;
            v[i] = self.parse_i32(&t)?;
            if v[i] < 0 || v[i] > 0xff {
                return Err(IniError::new(
                    3,
                    format!("color value {}={} out of range (0..255)", LABELS[i], v[i]), // @0x00bbe35a+2
                ));
            }
        }
        Ok([
            v[0] as f32 * SCALE,
            v[1] as f32 * SCALE,
            v[2] as f32 * SCALE,
        ])
    }

    /// 0x0042f507 / 0x0042f558: `X Y [Z]` label/value real triples with
    /// labels DAT_00bbe3c8/c4/c0 = "X","Y","Z".
    pub fn get_vec3(&mut self) -> IniResult<(f64, f64, f64)> {
        let mut out = [0.0f64; 3];
        for (i, l) in ["X", "Y", "Z"].iter().enumerate() {
            let t = self.expect_value(l)?;
            out[i] = self.parse_real(&t)?;
        }
        Ok((out[0], out[1], out[2]))
    }
    pub fn get_vec2(&mut self) -> IniResult<(f64, f64)> {
        let (x, y) = self.read_xy()?;
        Ok((x, y))
    }
    fn read_xy(&mut self) -> IniResult<(f64, f64)> {
        let t = self.expect_value("X")?;
        let x = self.parse_real(&t)?;
        let t = self.expect_value("Y")?;
        let y = self.parse_real(&t)?;
        Ok((x, y))
    }

    /// 0x0042dfe2 `getToken` (resolve flavor; the `deed`/`d0a9` pair are
    /// EH shells over the already-mirrored `nextToken`/`resolve`). Raw
    /// token absent and `was_resolved` is `None` → the same level-3
    /// "Expected additional data after '%s'" throw (delim arg in the
    /// message, @0x00bbdfd8) as `token_or_throw`; on success resolves via
    /// the MACRO dictionary and reports whether the resolve fired.
    pub fn get_resolved_token(
        &mut self,
        delim: Option<&str>,
        was_resolved: Option<&mut bool>,
    ) -> IniResult<String> {
        let d = delim.unwrap_or(DELIMS);
        let raw = match self.next_token(delim) {
            Some(t) => t,
            None => {
                if was_resolved.is_none() {
                    return Err(IniError::new(
                        3,
                        format!("Expected additional data after '{d}'"), // @0x00bbdfd8
                    ));
                }
                return Ok(String::new()); // game returns null ptr
            }
        };
        let r = self.resolve_label(&raw);
        if let Some(f) = was_resolved {
            *f = r != raw;
        }
        Ok(r)
    }

    /// 0x0042e03d: `get_resolved_token` without the throw — empty string
    /// on absent.
    pub fn get_optional_token(&mut self, delim: Option<&str>, was_resolved: &mut bool) -> String {
        match self.next_token(delim) {
            Some(raw) => {
                let r = self.resolve_label(&raw);
                *was_resolved = r != raw;
                r
            }
            None => String::new(),
        }
    }

    /// 0x0042e8a0 "rest of line": pump raw tokens to EOF (`deed(0)`),
    /// resolve each; an unresolved token is pushed whole (0x0042dbe6), a
    /// resolved one is re-split into the list on the default delimiters
    /// (StringBase::nextToken loop). The list is a `Vec<String>` —
    /// growth (0x0042d1cf) is Rust-native.
    pub fn get_rest_of_line(&mut self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        while let Some(raw) = self.next_token(None) {
            let r = self.resolve_label(&raw);
            if r == raw {
                out.push(raw);
            } else {
                let b = r.as_bytes();
                let dl = DELIMS.as_bytes();
                let mut pos = 0usize;
                loop {
                    while pos < b.len() && dl.contains(&b[pos]) {
                        pos += 1;
                    }
                    if pos >= b.len() {
                        break;
                    }
                    let start = pos;
                    while pos < b.len() && !dl.contains(&b[pos]) {
                        pos += 1;
                    }
                    out.push(r[start..pos].to_string());
                }
            }
        }
        out
    }

    /// 0x0042eb38 bitstring flag list (e.g. `ModelConditionFlags =
    /// NONE`, `+ENGINEERED -DEFECT`, plain name). Control flow from the
    /// decomp: `NONE` (strcmpi, DAT_00bbb50c) returns cleanly — it zeroes
    /// the mask **only** if no name preceded it, otherwise the mask is
    /// kept (the `local_5/local_6` gate only selects *whether* `*param_3`
    /// is zeroed; no throw). EOF also returns cleanly keeping the mask.
    /// The **first** plain name resets the mask then sets its bit;
    /// further plains OR in. `+NAME`/`-NAME` accumulate against the
    /// incoming mask but throw level 2 (empty message, f681) if a plain
    /// name already appeared; a plain name throws level 2 once ANY
    /// `+`/`-` fired. Missing/empty name list → level 2.
    /// Index masked `& 31` (game behavior for >31 entries, mirrored).
    pub fn get_bitstring(&mut self, mask: &mut u32, names: &[&str]) -> IniResult<()> {
        if names.is_empty() {
            return Err(IniError::new(2, "")); // f681(local,2) — level 2, null message
        }
        let mut had_plain = false;
        let mut had_signed = false;
        loop {
            let tok = match self.next_token(None) {
                Some(t) => t,
                None => return Ok(()), // EOF: keep the accumulated mask
            };
            if tok.eq_ignore_ascii_case("NONE") {
                // DAT_00bbb50c — terminator; zero only on a flag-free run.
                if !had_plain && !had_signed {
                    *mask = 0;
                }
                return Ok(());
            }
            let (set, bit) = if let Some(rest) = tok.strip_prefix('+') {
                if had_plain {
                    return Err(IniError::new(2, ""));
                }
                (true, Self::scan_index(rest, names, true)? & 31)
            } else if let Some(rest) = tok.strip_prefix('-') {
                if had_plain {
                    return Err(IniError::new(2, ""));
                }
                (false, Self::scan_index(rest, names, true)? & 31)
            } else {
                if had_signed {
                    return Err(IniError::new(2, "")); // local_6 gate
                }
                if !had_plain {
                    *mask = 0; // first plain resets (local_5 gate)
                }
                (true, Self::scan_index(&tok, names, true)? & 31)
            };
            if set {
                *mask |= 1 << bit;
            } else {
                *mask &= !(1 << bit);
            }
            if tok.starts_with('+') || tok.starts_with('-') {
                had_signed = true;
            } else {
                had_plain = true;
            }
        }
    }

    /// 0x0042f1dc / 0x0042f21a: bitstring into a u8/u16 field. Fresh
    /// `local_8` in the decomp: a `+`/`-`-only list would accumulate on
    /// uninitialized stack memory (game UB) — modeled starting at 0
    /// (plain/NONE paths reset deterministically, masking the bug for
    /// those inputs). Overflow beyond the narrow type throws the raw
    /// int (DAT_00cfe2c8 → level-0 empty-message `IniError`, same
    /// mechanism as the BlockTable overflow).
    pub fn get_bitstring_u8(&mut self, names: &[&str]) -> IniResult<u8> {
        let mut m = 0u32;
        self.get_bitstring(&mut m, names)?;
        if m & 0xffffff00 != 0 {
            return Err(IniError::internal());
        }
        Ok(m as u8) // game casts to signed char; bit-identical
    }
    pub fn get_bitstring_u16(&mut self, names: &[&str]) -> IniResult<u16> {
        let mut m = 0u32;
        self.get_bitstring(&mut m, names)?;
        if m & 0xffff0000 != 0 {
            return Err(IniError::internal());
        }
        Ok(m as u16)
    }

    /// 0x0042f2ff / 0x0042f3fe shared leg scanner: four labeled legs
    /// `R G B A` (DAT_00bbe388/384/bbb798/bbe3bc) pumped with the
    /// colon delimiters (+0x420) via the resolve flavor `deed`. A null
    /// token omits **only** the alpha leg (defaults to 0xff); omitting
    /// R/G/B throws level 3 `can't omit value for color %s`
    /// (@0x00bbe39c); a wrong label throws `expected '%s'`
    /// (@0x00bbe38c); values are `df97 → eccf` ints range-checked
    /// 0..255 with the same @0x00bbe35a+2 message as `get_rgb_f`.
    fn rgba_legs(&mut self) -> IniResult<[i32; 4]> {
        const LABELS: [&str; 4] = ["R", "G", "B", "A"];
        let mut v = [0i32; 4];
        for i in 0..4 {
            let mut resolved = false;
            let t = self.get_resolved_token(Some(DELIMS_COLON), Some(&mut resolved))?;
            if t.is_empty() {
                // deed returned NULL: only the alpha leg may be omitted.
                if i < 3 {
                    return Err(IniError::new(
                        3,
                        format!("can't omit value for color {}", LABELS[i]), // @0x00bbe39c
                    ));
                }
                v[i] = 0xff; // alpha may be omitted
            } else {
                if !t.eq_ignore_ascii_case(LABELS[i]) {
                    return Err(IniError::new(
                        3,
                        format!("expected '{}'", LABELS[i]), // @0x00bbe38c
                    ));
                }
                let t = self.token_or_throw(Some(DELIMS_COLON))?;
                v[i] = self.parse_i32(&t)?;
            }
            if v[i] < 0 || v[i] > 0xff {
                return Err(IniError::new(
                    3,
                    format!("color value {}={} out of range (0..255)", LABELS[i], v[i]), // @0x00bbe35a+2
                ));
            }
        }
        Ok(v)
    }

    /// 0x0042f2ff: RGBA as four ints (0..255 each).
    pub fn get_rgba(&mut self) -> IniResult<[i32; 4]> {
        self.rgba_legs()
    }

    /// 0x0042f3fe: RGBA packed little-endian `B | G<<8 | R<<16 | A<<24`
    /// (the `CONCAT` chain builds byte order R→MSB-adjacent; alpha
    /// last as written, mirrored from the decomp's byte soup).
    pub fn get_rgba_packed(&mut self) -> IniResult<u32> {
        let v = self.rgba_legs()?;
        Ok((v[3] as u32) << 24 | (v[0] as u32) << 16 | (v[1] as u32) << 8 | v[2] as u32)
    }

    /// 0x0042ec6e: enum name → **u8** index (`df97(0)` + always-throw
    /// scan 0x0042bd39); index outside 0..255 throws the raw int
    /// (level-0 `internal()`, DAT_00cfe2c8) — unreachable through
    /// `scan_index` (list index < 2^31) but the compare is mirrored.
    pub fn get_enum_u8(&mut self, names: &[&str]) -> IniResult<u8> {
        let t = self.token_or_throw(None)?;
        let i = Self::scan_index(&t, names, true)?;
        if i > 0xff {
            return Err(IniError::internal());
        }
        Ok(i as u8)
    }

    /// 0x0042e86c: bool token setting/clearing one bit of a mask field
    /// (`v2` in a token table = the bit): yes → `| mask`, no → `& !mask`.
    pub fn get_bool_mask(&mut self, mask: u32, field: &mut u32) -> IniResult<()> {
        let t = self.token_or_throw(None)?;
        if self.parse_bool(&t)? {
            *field |= mask;
        } else {
            *field &= !mask;
        }
        Ok(())
    }

    /// 0x0042efdc / 0x0042f02f / 0x0042f082: real getters that **store
    /// first, then validate** (the decomp writes `*param_3` before the
    /// compare; callers see the value even as the load aborts).
    /// Throw level 3 with `invalid Real value %1.7f` (@0x00bbe2d4
    /// `expected > 0` on `<= 0`; @0x00bbe300 `expected >= 0` on `< 0`;
    /// @0x00bbe32c `expected <= 0` on `> 0`).
    pub fn get_real_positive(&mut self, out: &mut f32) -> IniResult<()> {
        let t = self.token_or_throw(None)?;
        let v = self.parse_real(&t)? as f32;
        *out = v;
        if v <= 0.0 {
            return Err(IniError::new(
                3,
                format!("invalid Real value {:.7} -- expected > 0", v),
            ));
        }
        Ok(())
    }
    pub fn get_real_nonneg(&mut self, out: &mut f32) -> IniResult<()> {
        let t = self.token_or_throw(None)?;
        let v = self.parse_real(&t)? as f32;
        *out = v;
        if v < 0.0 {
            return Err(IniError::new(
                3,
                format!("invalid Real value {:.7} -- expected >= 0", v),
            ));
        }
        Ok(())
    }
    pub fn get_real_nonpos(&mut self, out: &mut f32) -> IniResult<()> {
        let t = self.token_or_throw(None)?;
        let v = self.parse_real(&t)? as f32;
        *out = v;
        if v > 0.0 {
            return Err(IniError::new(
                3,
                format!("invalid Real value {:.7} -- expected <= 0", v),
            ));
        }
        Ok(())
    }

    /// 0x0042f1ba: duration fetched with the percent delimiters
    /// (+0x41c = `DELIMS_PCT`), stored as f32 seconds
    /// (`get_duration` × DAT_00bcf628).
    pub fn get_duration_f32(&mut self) -> IniResult<f32> {
        let t = self.token_or_throw(Some(DELIMS_PCT))?;
        Ok((self.parse_real(&t)? * 0.01) as f32) // DAT_00bcf628
    }

    /// 0x0042f128: quoted-string store wrapper (`ea4f` shell → `ea59`
    /// body = `get_quoted_string`, then `StringBase::set` + release on
    /// the destination field). `get_quoted_string` covers both legs;
    /// the field model is Rust-native assignment.
    pub fn get_string_field(&mut self) -> IniResult<Option<String>> {
        self.get_quoted_string()
    }

    /// 0x0042f196: list-clear (`0x0042ccfc`) + rest-of-line refill
    /// (`0x0042e896` = `skip` shell of 0x0042e8a0). Modeled as
    /// [`get_rest_of_line`] returning a fresh list (the clear is
    /// implicit).
    pub fn get_rest_of_line_into(&mut self) -> Vec<String> {
        self.get_rest_of_line()
    }

    /// Register a block handler at the HEAD of the registry
    /// (0x0042c0ff walks `DAT_00ddf578` front-first; the registrars,
    /// e.g. 0x00babbf6, prepend: `_DAT = DAT_00ddf578;
    /// DAT_00ddf578 = &_DAT` — so a later registration shadows an
    /// earlier same-named one).
    pub fn register_block(&mut self, h: &'static BlockHandler) {
        self.blocks.insert(0, h);
    }

    /// 0x0042c0ff block dispatch: `strtok(line, +0x418)` for the block
    /// name; exact `strcmp` walk of the registry (case-SENSITIVE,
    /// node+4 name); unknown → level 5 `Unknown block '%s'...`
    /// (@0x00bbdbd0, filename leg = `+0x04` str, null → empty). The raw
    /// current line is copied into `+0x431` for the handler's lifetime,
    /// then reset to `NO_BLOCK` (@0x00bbdc74).
    pub fn dispatch_block(&mut self) -> IniResult<()> {
        let tok = match self.seed_tok(DELIMS) {
            Some(t) => t,
            None => return Ok(()),
        };
        let h = self.blocks.iter().find(|h| h.name == tok).copied();
        let h = match h {
            Some(h) => h,
            None => {
                return Err(IniError::new(
                    5,
                    format!(
                        "Unknown block '{tok}'.\n\nError parsing INI block '{tok}' in file '{}'.",
                        self.filename // +0x04; TheNullChr fallback = empty
                    ), // @0x00bbdbd0
                ));
            }
        };
        let saved = std::mem::take(&mut self.cur_block_name);
        self.cur_block_name = self.line.clone(); // mbscpy(+0x431, +0x14)
        let r = (h.handler)(self);
        self.cur_block_name = if r.is_ok() {
            "NO_BLOCK".to_string()
        } else {
            saved
        };
        r
    }

    /// 0x0042d702 script-block read (behind the `0x0042d6f8` `skip`
    /// shell): pump `nextLine`; stop when the line's first token is the
    /// script terminator `ENDSCRIPT` (+0x42c = DAT_00bbdcdc,
    /// `_strcmpi`); that line is NOT included. Raw lines are concatenated
    /// without extra separators — the game's line buffer carries the
    /// trailing newline from the file split [INFERENCE], so Rust joins
    /// with `\n`. A missing `ENDSCRIPT` spins forever in the game
    /// (unbounded do-loop past EOF — game bug); modeled as stopping at
    /// EOF and returning the accumulation.
    pub fn get_script_block(&mut self) -> String {
        let mut acc = String::new();
        loop {
            self.advance_line();
            if self.eof {
                break; // game: infinite concat of empty lines (bug)
            }
            let first = self.seed_tok(DELIMS); // strtok(line, +0x418)
            if let Some(t) = first {
                if t.eq_ignore_ascii_case("ENDSCRIPT") {
                    break;
                }
            }
            if !acc.is_empty() {
                acc.push('\n');
            }
            acc.push_str(&self.line);
        }
        acc
    }

    /// 0x0042de46: store wrapper (`set` of a StringBase field from the
    /// `d6f8` shell of 0x0042d702) — Rust-native assignment; the getter
    /// carries the semantics.
    pub fn get_script_block_into(&mut self) -> String {
        self.get_script_block()
    }

    /// 0x0042e949 quoted string with ONE continuation step (body
    /// behind `skip` shell 0x0042e896; 0x0042f164 = store wrapper):
    /// first token via the non-throwing pump (`deed` shell of
    /// 0x0042d0a9; pending-string leg first) —
    /// * no token → empty result (no throw);
    /// * token not starting with `"` → the raw token is the result;
    /// * `"…"` closed inside the token (len ≥ 2, last char `"`): inner
    ///   text with BOTH quotes stripped;
    /// * otherwise: buffer = token with the leading `"` stripped (empty
    ///   for a lone `"`), then exactly ONE continuation step:
    ///   `df97(+0x424)` grabs the next token (THROWS level 3 @0x00bbdfd8
    ///   if the line is exhausted). A step token with len ≥ 2 and second
    ///   char NOT `\t` appends `' '` (DAT_00bbd40c) + token; a token of
    ///   len < 2 or second char `\t` instead strips a trailing `"` from
    ///   the buffer if it has one (else appends nothing). There is NO
    ///   loop and NO line advance in e949 — callers repeat the call for
    ///   multi-token values.
    pub fn get_multiline_string(&mut self) -> IniResult<String> {
        let tok = match self.next_token(None) {
            Some(t) => t,
            None => return Ok(String::new()),
        };
        if !tok.starts_with('"') {
            return Ok(tok);
        }
        let mut acc = if tok.len() >= 2 {
            // mbscpy(+1): strip the leading quote (tail = tok[1..])
            let tail = tok[1..].to_string();
            if tail.as_bytes().last() == Some(&b'"') {
                // closed within the token: strip the trailing quote too
                return Ok(tail[..tail.len() - 1].to_string()); // *pcVar2 = '\0'
            }
            tail
        } else {
            String::new() // lone '"' → empty buffer
        };
        // exactly one continuation step: df97(+0x424) throws level 3
        // (@0x00bbdfd8) when the line has no further token
        let src = self.token_or_throw(Some(DELIMS_QUOTE))?;
        if src.len() < 2 || src.as_bytes()[1] == b'\t' {
            if acc.as_bytes().last() == Some(&b'"') {
                acc.pop();
            }
        } else {
            acc.push(' '); // DAT_00bbd40c
            acc.push_str(&src);
        }
        Ok(acc)
    }

    /// 0x0042f164: store wrapper around the 0x0042e93f `skip` shell of
    /// 0x0042e949 — Rust-native assignment; the getter carries it.
    pub fn get_multiline_string_into(&mut self) -> IniResult<String> {
        self.get_multiline_string()
    }

    /// 0x0042c025 file-mtime vs gate-date ladder: true iff the file's
    /// local-time mtime is at or after the gate stamp (fields =
    /// `tm_year+1900, tm_mon+1, tm_mday, tm_hour, tm_min, tm_sec`; an
    /// empty filename is always false). `struct tm` field order gives
    /// `param_2[4]/[5]/[6]` = hour/min/sec (the wDay slot `[3]` is
    /// skipped).
    pub fn file_newer_than(mtime: &DateStamp, gate: &DateStamp) -> bool {
        // game cascade: at each level, file>gate → true, file==gate →
        // descend, else fall out false; final level returns gate<=file.
        if gate.year < mtime.year {
            return true;
        }
        if gate.year <= mtime.year {
            if gate.mon < mtime.mon {
                return true;
            }
            if gate.mon <= mtime.mon {
                if gate.mday < mtime.mday {
                    return true;
                }
                if gate.mday <= mtime.mday {
                    if gate.hour < mtime.hour {
                        return true;
                    }
                    if gate.hour <= mtime.hour {
                        if gate.min < mtime.min {
                            return true;
                        }
                        if gate.min <= mtime.min {
                            return gate.sec <= mtime.sec;
                        }
                    }
                }
            }
        }
        false
    }

    /// 0x0042d2cb `INI::Open` pipeline (the file-open legs 0x00a00c34 /
    /// 0x00a02072 stay unmirrored — sage-core takes the line table
    /// directly), including the `#define` MACRO pre-pass:
    ///
    /// * mode-5 gate: with a clean macro table the file is skipped
    ///   unless `file_newer` (0x0042c025 mtime ladder); with a dirty
    ///   table it is skipped unless it is the current macro source
    ///   (0x0042cb98 tree compare) — skipped means "success, nothing
    ///   loaded" (returns false here).
    /// * pre-pass scans the line table from line 0 while the raw line
    ///   starts with `#define` (@0x00bbde00, case-sensitive); the first
    ///   non-`#define` line ENDS the pass (later defines are never
    ///   seen — contiguous-only rule). The `'#'` is NUL'd out in the
    ///   table IN PLACE (`*pcVar5 = '\0'`) before tokenizing.
    /// * tokens (default whitespace `nextToken`): `define`, NAME,
    ///   value tokens joined with a single space (DAT_00bbd40c).
    /// * filename ending in `map.ini` (@0x00bbddf8): level 8 throw
    ///   `%s:\nMACROs not allowed in map.ini.\n%s.` (@0x00bbddb0).
    /// * NAME scan: the game checks `'a' < c < 'z'` STRICTLY —
    ///   lowercase `a` and `z` sneak through (game bug, mirrored):
    ///   `%s:\nMACRO names must use UPPERCASE letters.\n%s.`
    ///   (@0x00bbdd80).
    /// * empty value → level 8 `%s:\nError parsing MACRO.\n%s has no
    ///   value` (@0x00bbdd54).
    /// * insert via the macro map (0x0042cd2f semantics, key
    ///   normalization via 0x0042c621/0x0042c4fd = ci_key):
    ///   mode != 5: existing name → level 8 `%s:\nDuplicate MACRO
    ///   names.\n%s.` (@0x00bbddd8); mode == 5: existing with a
    ///   different value → overwrite + bump `macros_version`
    ///   (DAT_00ddf5ac); new defines insert with their value.
    /// * on success `filename`/`mode` land in `+0x04`/`+0x08`.
    pub fn open(
        &mut self,
        mut lines: Vec<String>,
        filename: &str,
        mode: i32,
        gate: &OpenGate,
    ) -> Result<bool, IniError> {
        if mode == 5 {
            // clean table → date gate; dirty table → source gate
            let pass = if gate.clean {
                gate.file_newer
            } else {
                gate.macro_source
            };
            if !pass {
                return Ok(false); // goto LAB_0042d659: true-return, unloaded
            }
        }
        let mut i = 0;
        while i < lines.len() && lines[i].starts_with("#define") {
            lines[i].replace_range(0..1, "\0"); // '#' NUL'd in place
            let toks: Vec<&str> = lines[i].split_ascii_whitespace().collect(); // [define, NAME, ...]
            let name = match toks.get(1) {
                Some(t) => *t,
                None => "",
            };
            if filename.ends_with("map.ini") {
                return Err(IniError::new(
                    8,
                    format!(
                        "{filename}:\nMACROs not allowed in map.ini.\n{name}." // @0x00bbddb0
                    ),
                ));
            }
            if name.bytes().any(|c| c > b'a' && c < b'z') {
                // strict range: 'a'/'z' pass (game bug mirrored)
                return Err(IniError::new(
                    8,
                    format!(
                        "{filename}:\nMACRO names must use UPPERCASE letters.\n{name}." // @0x00bbdd80
                    ),
                ));
            }
            let value = toks[2..].join(" ");
            if value.is_empty() {
                return Err(IniError::new(
                    8,
                    format!(
                        "{filename}:\nError parsing MACRO.\n{name} has no value" // @0x00bbdd54
                    ),
                ));
            }
            match self.macros.get_mut(&ci_key(name)) {
                Some((_, v)) => {
                    if mode == 5 {
                        if *v != value {
                            *v = value;
                            self.macros_version += 1; // DAT_00ddf5ac
                        }
                    } else {
                        return Err(IniError::new(
                            8,
                            format!(
                                "{filename}:\nDuplicate MACRO names.\n{name}." // @0x00bbddd8
                            ),
                        ));
                    }
                }
                None => {
                    self.macros.insert(ci_key(name), (name.to_string(), value));
                }
            }
            i += 1;
        }
        self.filename = filename.to_string(); // StringBase::set(+0x04)
        self.mode = mode; // +0x08
        self.lines = lines;
        Ok(true)
    }

    /// 0x009d5847 global line-table count (`(end-start)/12`).
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// 0x00a018b6 global line-table index; out of range yields the null
    /// pointer → empty string.
    pub fn line_at(&self, i: usize) -> &str {
        self.lines.get(i).map(String::as_str).unwrap_or("")
    }

    /// 0x0042e0d3/0x0042e2a3/0x0042e474 math evaluators (see
    /// [`math_eval`]).
    pub fn math_int(&mut self, tok: &str) -> IniResult<i32> {
        let _ = tok; // first token up-cased into a dead StringBase copy (no strcmp against it)
        math_eval(self, |ini, t| ini.parse_i32(t))
    }
}

/// 0x0042e0d3 `#`-expression evaluator, parameterized leaf (int
/// 0x0042e0d3 / uint 0x0042e2a3 / float 0x0042e474 share the shape).
/// Grammar: the caller saw `#OP(`; the evaluator then fetches operand1
/// via `df97`, then the **operator token again** (`#ADD(` / `#SUBTRACT(` /
/// `#MULTIPLY(` / `#DIVIDE(`, @0x00bbdabc/c4/d0/dc), folds operands, and
/// stops on the terminator token `)` (@0x00bbdae8). SUBTRACT/DIVIDE take
/// exactly two operands or throw level 8; an unknown second token throws
/// level 3. Operator/terminator compares are case-**sensitive** `strcmp`
/// (verified: only the first token is up-cased, into a copy that is never
/// compared — dead code mirrored as no-op).
pub fn math_eval<T, F>(ini: &mut Ini, leaf: F) -> IniResult<T>
where
    T: MathVal,
    F: FnMut(&mut Ini, &str) -> IniResult<T>,
{
    let mut leaf = leaf;
    let op1 = ini.token_or_throw(None)?;
    let op = ini.token_or_throw(None)?;
    if op == "#ADD(" {
        let mut acc = leaf(ini, &op1)?;
        loop {
            let t = ini.token_or_throw(None)?;
            if t == ")" {
                return Ok(acc);
            }
            acc = T::add(acc, leaf(ini, &t)?);
        }
    }
    if op == "#SUBTRACT(" {
        let a = leaf(ini, &op1)?;
        let t2 = ini.token_or_throw(None)?;
        let b = leaf(ini, &t2)?;
        let acc = T::sub(a, b);
        let t = ini.token_or_throw(None)?;
        if t == ")" {
            return Ok(acc);
        }
        return Err(IniError::new(8, "#SUBTRACT takes only 2 operands")); // @0x00bbe074
    }
    if op == "#MULTIPLY(" {
        let mut acc = leaf(ini, &op1)?;
        loop {
            let t = ini.token_or_throw(None)?;
            if t == ")" {
                return Ok(acc);
            }
            acc = T::mul(acc, leaf(ini, &t)?);
        }
    }
    if op == "#DIVIDE(" {
        let a = leaf(ini, &op1)?;
        let t2 = ini.token_or_throw(None)?;
        let b = leaf(ini, &t2)?;
        let acc = T::div(a, b);
        let t = ini.token_or_throw(None)?;
        if t == ")" {
            return Ok(acc);
        }
        return Err(IniError::new(8, "#DIVIDE takes only 2 operands")); // @0x00bbe054
    }
    Err(IniError::new(
        3,
        format!("Expected known math operation after #, but found '{op}'"), // @0x00bbe01c
    ))
}

/// Arithmetic flavor per evaluator (C semantics: int wraps, u32 wraps,
/// division by zero traps in both — Rust panics stand in).
pub trait MathVal: Copy {
    fn add(a: Self, b: Self) -> Self;
    fn sub(a: Self, b: Self) -> Self;
    fn mul(a: Self, b: Self) -> Self;
    fn div(a: Self, b: Self) -> Self;
}
impl MathVal for i32 {
    fn add(a: Self, b: Self) -> Self {
        a.wrapping_add(b)
    }
    fn sub(a: Self, b: Self) -> Self {
        a.wrapping_sub(b)
    }
    fn mul(a: Self, b: Self) -> Self {
        a.wrapping_mul(b)
    }
    fn div(a: Self, b: Self) -> Self {
        a.wrapping_div(b)
    }
}
impl MathVal for u32 {
    fn add(a: Self, b: Self) -> Self {
        a.wrapping_add(b)
    }
    fn sub(a: Self, b: Self) -> Self {
        a.wrapping_sub(b)
    }
    fn mul(a: Self, b: Self) -> Self {
        a.wrapping_mul(b)
    }
    fn div(a: Self, b: Self) -> Self {
        a.wrapping_div(b)
    }
}
impl MathVal for f64 {
    fn add(a: Self, b: Self) -> Self {
        a + b
    }
    fn sub(a: Self, b: Self) -> Self {
        a - b
    }
    fn mul(a: Self, b: Self) -> Self {
        a * b
    }
    fn div(a: Self, b: Self) -> Self {
        a / b
    }
}

// ---------------------------------------------------------------------------
// Block-descriptor tables + parse driver

/// One static token-handler record: the 4-dword array entries the game
/// builds in `.rdata` (e.g. PTR_s_Alpha1_00c6b988 → `{"Alpha1", 0x0095b29c,
/// 0, 0xc}`), walked case-**sensitively** by 0x0042bc27.
#[derive(Clone, Copy, Debug)]
pub struct Entry {
    pub name: &'static str,
    /// Handler function VA (dispatch key).
    pub handler: u32,
    /// `entry[2]` — extra arg passed to the handler.
    pub v2: u32,
    /// `entry[3]` — byte offset into the destination object.
    pub off: usize,
}

/// A row of the 132-byte stack block from 0x0042de78:
/// `names[j]` = pointer to the static `Entry` array, `offsets[j]` = base
/// added to every `entry.off`.
#[derive(Clone, Copy, Debug)]
pub struct TableRow {
    pub entries: &'static [Entry],
    pub dest_off: usize,
}

/// The block-descriptor struct (0x0042baa0 zeroed / 0x0042bc6e push):
/// `names[16]` + `offsets[16]` + `count` at +0x80.
#[derive(Default)]
pub struct BlockTable {
    rows: [Option<TableRow>; 16],
    count: usize,
}

impl BlockTable {
    /// 0x0042baa0 `BlockTable::reset`.
    pub fn reset(&mut self) {
        self.rows = Default::default();
        self.count = 0;
    }

    /// 0x0042bc6e `push(name, offset)`: `count > 0xF` throws the raw int
    /// assert (DAT_00cfe2c8).
    pub fn push(&mut self, row: TableRow) -> IniResult<()> {
        if self.count > 0xF {
            return Err(IniError::internal());
        }
        self.rows[self.count] = Some(row);
        self.count += 1;
        Ok(())
    }
}

/// 0x0042de78 driver entry: reset, push the single static-table row, then
/// parse; `dest_off` is `param_2`-as-offset (the game passes the object
/// pointer; Rust models it as a base added to `entry.off`).
pub fn parse_table(ini: &mut Ini, tbl: &mut BlockTable, row: TableRow) -> IniResult<()> {
    tbl.reset();
    tbl.push(row)?;
    parse_block(ini, tbl, &mut |_, _, _, _| Ok(()))
}

/// 0x0042d7b2 `INI::parseBlock` core loop. Fidelity points:
/// * one field token per line — the rest of the line is ignored;
/// * END token compare is `_strcmpi` (case-insensitive), field names are
///   `strcmp` (case-sensitive, 0x0042bc27);
/// * EOF before END → level 4 `Missing 'END' token.`;
/// * unknown token → level 5 `Unknown field ...` with the block name.
/// `apply(ini, entry, total_off, v2)` is the game's handler call
/// `(*fn)(ini, dest, dest + offsets[j] + entry.off, entry.v2)`.
pub fn parse_block<F>(ini: &mut Ini, tbl: &BlockTable, apply: &mut F) -> IniResult<()>
where
    F: FnMut(&mut Ini, &Entry, usize, u32) -> IniResult<()>,
{
    let mut done = false;
    while !done {
        ini.advance_line();
        let tok = ini.seed_tok(DELIMS);
        if let Some(tok) = tok {
            if tok.eq_ignore_ascii_case(END_TOKEN) {
                done = true;
                continue;
            }
            let mut matched = false;
            'rows: for j in 0..tbl.count {
                let row = tbl.rows[j].unwrap();
                for e in row.entries {
                    // case-SENSITIVE: the entry scan is FUN_0042bc27, whose
                    // comparison is `strcmp` (only the END test at
                    // 0x0042d81b uses _strcmpi).
                    if e.name == tok {
                        apply(ini, e, row.dest_off + e.off, e.v2)?;
                        matched = true;
                        break 'rows;
                    }
                }
            }
            if !matched {
                return Err(IniError::new(
                    5,
                    format!(
                        "Unknown field '{tok}' in block '{}'.\n\nError parsing field '{tok}' in block '{}' in file '{}', line {}.\n",
                        ini.cur_block_name, ini.cur_block_name, ini.filename, ini.current_line + 1
                    ), // @0x00bbde68 (banner half unmirrored)
                ));
            }
        }
        if ini.eof {
            return Err(IniError::new(
                4,
                format!(
                    "Missing '{END_TOKEN}' token.\n\nError parsing block '{}' in file '{}', line {}.",
                    ini.cur_block_name, ini.filename, ini.current_line + 1
                ), // @0x00bbdf60
            ));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// C-stdlib conversion helpers (exact prefix semantics of sscanf %d/%u/%f)

fn ci_key(s: &str) -> String {
    s.to_ascii_lowercase()
}

/// `strtok`-style first-token extractor over a standalone string
/// (StringBase::nextToken path in 0x0042def7).
fn strtok_over(s: &str, delim: &str) -> Option<String> {
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() && delim.as_bytes().contains(&b[i]) {
        i += 1;
    }
    if i >= b.len() {
        return None;
    }
    let start = i;
    while i < b.len() && !delim.as_bytes().contains(&b[i]) {
        i += 1;
    }
    Some(s[start..i].to_string())
}

/// C `sscanf %d`: skip leading whitespace, optional sign, require ≥1
/// digit; trailing garbage ignored (conversion succeeded).
fn c_strtol(s: &str) -> Option<i64> {
    let t = s.trim_start();
    let (neg, rest) = match t.as_bytes().first() {
        Some(b'+') => (false, &t[1..]),
        Some(b'-') => (true, &t[1..]),
        _ => (false, t),
    };
    let mut acc: i64 = 0;
    let mut any = false;
    for c in rest.bytes() {
        if !c.is_ascii_digit() {
            break;
        }
        acc = acc.wrapping_mul(10).wrapping_add((c - b'0') as i64);
        any = true;
    }
    if !any {
        return None;
    }
    Some(if neg { -acc } else { acc })
}

/// C `sscanf %u` (MSVC): leading sign allowed, wraps mod 2^32.
fn c_strtoul(s: &str) -> Option<u64> {
    let t = s.trim_start();
    let (neg, rest) = match t.as_bytes().first() {
        Some(b'+') => (false, &t[1..]),
        Some(b'-') => (true, &t[1..]),
        _ => (false, t),
    };
    let mut acc: u64 = 0;
    let mut any = false;
    for c in rest.bytes() {
        if !c.is_ascii_digit() {
            break;
        }
        acc = acc.wrapping_mul(10).wrapping_add((c - b'0') as u64);
        any = true;
    }
    if !any {
        return None;
    }
    Some(if neg { 0u64.wrapping_sub(acc) } else { acc })
}

/// C `sscanf %f` / `strtod` prefix: `[+-]?digits[.digits][(e|E)[+-]digits]`,
/// at least one digit before or after the point.
fn c_strtod(s: &str) -> Option<f64> {
    let t = s.trim_start();
    let b = t.as_bytes();
    let mut i = 0;
    let mut sign = 1.0f64;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        if b[i] == b'-' {
            sign = -1.0;
        }
        i += 1;
    }
    let dstart = i;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    let int_digits = i - dstart;
    let mut frac_digits = 0;
    let mut estart = None;
    if i < b.len() && b[i] == b'.' {
        i += 1;
        let f = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        frac_digits = i - f;
    }
    if int_digits + frac_digits == 0 {
        return None;
    }
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        estart = Some(i);
        let mut j = i + 1;
        if j < b.len() && (b[j] == b'+' || b[j] == b'-') {
            j += 1;
        }
        let ed = j;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }
        if j == ed {
            estart = None; // bad exponent: not consumed
        } else {
            i = j;
        }
    }
    let _ = estart;
    t.get(..i)
        .and_then(|p| p.parse::<f64>().ok())
        .map(|v| v * sign)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ini(lines: &[&str]) -> Ini {
        let mut i = Ini::new(lines.iter().map(|s| s.to_string()).collect());
        i.advance_line(); // the driver reads the first line before tokenizing
        i
    }

    #[test]
    fn strtok_collapse_and_end_ci() {
        let mut i = ini(&["Alpha1   =  3", "End"]);
        assert_eq!(i.seed_tok(DELIMS).as_deref(), Some("Alpha1"));
        assert_eq!(i.next_token(None).as_deref(), Some("3"));
        assert_eq!(i.next_token(None), None);
        i.advance_line();
        assert_eq!(i.seed_tok(DELIMS).as_deref(), Some("End"));
    }

    #[test]
    fn bool_yes_no_and_throw() {
        let i = Ini::new(vec![]);
        assert!(i.parse_bool("YES").unwrap());
        assert!(!i.parse_bool("No").unwrap());
        let e = i.parse_bool("maybe").unwrap_err();
        assert_eq!(e.level, 3);
        assert_eq!(
            e.message,
            "invalid boolean token maybe -- expected Yes or No"
        );
    }

    #[test]
    fn macro_resolve() {
        let mut i = Ini::new(vec![]);
        i.macros.insert("fast".into(), ("fast".into(), "5".into()));
        assert_eq!(i.resolve_label("FAST"), "5");
        assert_eq!(i.resolve_label("1st"), "1st"); // digit-leading: no lookup
        let mut g = ini(&["X #ADD( 1 #ADD( 2 3 )"]);
        assert_eq!(g.seed_tok(DELIMS).as_deref(), Some("X"));
        // getter token is "#ADD(": op1="1", operator token again, then 2+3
        assert_eq!(g.get_u32_bounded(0).unwrap(), 6);
    }

    #[test]
    fn math_add_fold_and_arity() {
        let mut i = ini(&["#ADD( 1 #ADD( 2 3 )"]);
        assert_eq!(i.seed_tok(DELIMS).as_deref(), Some("#ADD("));
        assert_eq!(i.math_int("").unwrap(), 1 + 2 + 3);
        let mut j = ini(&["#SUBTRACT( 9 #SUBTRACT( 4 7 )"]);
        assert_eq!(j.seed_tok(DELIMS).as_deref(), Some("#SUBTRACT("));
        let e = j.math_int("").unwrap_err();
        assert_eq!(e.level, 8);
        assert_eq!(e.message, "#SUBTRACT takes only 2 operands");
    }

    #[test]
    fn math_unknown_op() {
        let mut i = ini(&["#POW( 1 2 )"]);
        assert_eq!(i.seed_tok(DELIMS).as_deref(), Some("#POW("));
        let e = i.math_int("").unwrap_err();
        assert_eq!(e.level, 3);
        assert_eq!(
            e.message,
            "Expected known math operation after #, but found '2'"
        );
    }

    #[test]
    fn rgb_scaled_and_range() {
        let mut i = ini(&["Color R = 170 G = 30 B = 102"]);
        i.seed_tok(DELIMS).unwrap(); // driver's first-token read
        let rgb = i.get_rgb_f().unwrap();
        assert_eq!(rgb[0], 170.0 * 0.003921568859368563);
        let mut j = ini(&["Color R = 256"]);
        j.seed_tok(DELIMS).unwrap();
        let e = j.get_rgb_f().unwrap_err();
        assert_eq!(e.level, 3);
        assert_eq!(e.message, "color value R=256 out of range (0..255)");
    }

    #[test]
    fn expect_mismatch() {
        let mut i = ini(&["x R 30"]);
        i.seed_tok(DELIMS).unwrap();
        let e = i.expect_value("G").unwrap_err();
        assert_eq!(e.message, "Expected 'G' but found 'R'");
        let mut ok = ini(&["x R = 30"]);
        ok.seed_tok(DELIMS).unwrap(); // consumes "x"
                                      // label "R" matched case-insensitively; the EAX-returned token is
                                      // the value itself (`=` is a delimiter, never its own token)
        assert_eq!(ok.expect_value("r").as_deref(), Ok("30"));
    }

    #[test]
    fn parse_block_full_field_per_line() {
        let statics: &[Entry] = &[
            Entry {
                name: "Alpha1",
                handler: 0x0095b29c,
                v2: 0,
                off: 0xc,
            },
            Entry {
                name: "Alpha2",
                handler: 0x0095b29c,
                v2: 0,
                off: 0x1c,
            },
            Entry {
                name: "",
                handler: 0,
                v2: 0,
                off: 0,
            },
        ];
        let mut tbl = BlockTable::default();
        tbl.push(TableRow {
            entries: statics,
            dest_off: 0,
        })
        .unwrap();
        let mut i = Ini::new(vec![
            "Alpha1 = 11".into(),
            "Alpha2 = 12 junk ignored here".into(),
            "end".into(),
        ]);
        let mut seen: Vec<(u32, usize)> = Vec::new();
        parse_block(&mut i, &tbl, &mut |ini, e, off, _v2| {
            seen.push((e.handler, off));
            let t = ini.token_or_throw(None)?;
            assert_eq!(ini.parse_i32(&t).unwrap(), 11 + seen.len() as i32 - 1);
            Ok(())
        })
        .unwrap();
        assert_eq!(seen, vec![(0x0095b29c, 0xc), (0x0095b29c, 0x1c)]);
    }

    #[test]
    fn parse_block_errors() {
        let statics: &[Entry] = &[Entry {
            name: "X",
            handler: 1,
            v2: 0,
            off: 0,
        }];
        let mut tbl = BlockTable::default();
        tbl.push(TableRow {
            entries: statics,
            dest_off: 0,
        })
        .unwrap();
        let mut i = Ini::new(vec!["Y 1".into()]);
        i.filename = "thing.ini".into();
        let e = parse_block(&mut i, &tbl, &mut |_, _, _, _| Ok(())).unwrap_err();
        assert_eq!(e.level, 5);
        assert!(e.message.starts_with("Unknown field 'Y' in block ''."));
        let mut j = Ini::new(vec!["X 1".into()]);
        let e = parse_block(&mut j, &tbl, &mut |_, _, _, _| Ok(())).unwrap_err();
        assert_eq!(e.level, 4);
        assert!(e.message.starts_with("Missing 'END' token."));
    }

    #[test]
    fn block_table_overflow_internal() {
        let mut t = BlockTable::default();
        static E: Entry = Entry {
            name: "n",
            handler: 0,
            v2: 0,
            off: 0,
        };
        for _ in 0..16 {
            t.push(TableRow {
                entries: std::slice::from_ref(&E),
                dest_off: 0,
            })
            .unwrap();
        }
        assert_eq!(
            t.push(TableRow {
                entries: std::slice::from_ref(&E),
                dest_off: 0
            })
            .unwrap_err(),
            IniError::internal()
        );
    }

    #[test]
    fn sscanf_prefix_semantics() {
        assert_eq!(c_strtol("12abc"), Some(12)); // trailing garbage OK
        assert_eq!(c_strtol("-7"), Some(-7));
        assert_eq!(c_strtol("abc"), None);
        assert_eq!(c_strtoul("-1"), Some(u64::MAX)); // wraps mod 2^32
        assert_eq!(c_strtod("1.5x"), Some(1.5));
        assert_eq!(c_strtod(".5"), Some(0.5));
        assert_eq!(c_strtod("."), None);
    }

    // -- wave 18: token utilities + bitstring (0x0042dfe2/e03d/e8a0/
    //    dbe6/d1cf/eb38/f1dc/f21a) --------------------------------------

    const BNAMES: &[&str] = &["ALPHA", "BRAVO", "CHARLIE"];

    #[test]
    fn bitstring_grammar() {
        // NONE alone → 0 even from a non-zero start mask.
        let mut i = ini(&["Flags = NONE", "end"]);
        i.seed_tok(DELIMS); // the parse block consumes the field name first
        let mut m = 0xA5u32;
        i.get_bitstring(&mut m, BNAMES).unwrap();
        assert_eq!(m, 0);
        // plain resets then sets; second plain ORs in (no reset).
        let mut i = ini(&["Flags = ALPHA BRAVO", "end"]);
        i.seed_tok(DELIMS);
        let mut m = 0xA5;
        i.get_bitstring(&mut m, BNAMES).unwrap();
        assert_eq!(m, 0b011);
        // accumulation against the incoming mask; EOF terminates clean.
        let mut i = ini(&["Flags = +BRAVO -CHARLIE"]);
        i.seed_tok(DELIMS);
        let mut m = 0b101;
        i.get_bitstring(&mut m, BNAMES).unwrap();
        assert_eq!(m, 0b011);
        // NONE after names terminates cleanly, mask kept (gate only
        // selects whether *param_3 is zeroed).
        let mut i = ini(&["Flags = ALPHA NONE", "end"]);
        i.seed_tok(DELIMS);
        let mut m = 0u32;
        i.get_bitstring(&mut m, BNAMES).unwrap();
        assert_eq!(m, 0b001);
        // plain after `+` and `+` after plain → level 2.
        for lines in [
            &["Flags = +ALPHA BRAVO", "end"][..],
            &["Flags = BRAVO +ALPHA", "end"][..],
        ] {
            let mut i = ini(lines);
            i.seed_tok(DELIMS);
            let mut m = 0u32;
            assert_eq!(i.get_bitstring(&mut m, BNAMES).unwrap_err().level, 2);
        }
        // unknown member → level 3 via the scan (0x0042bd39).
        let mut i = ini(&["Flags = DELTA", "end"]);
        i.seed_tok(DELIMS);
        let mut m = 0u32;
        assert_eq!(i.get_bitstring(&mut m, BNAMES).unwrap_err().level, 3);
    }

    #[test]
    fn bitstring_narrow_range_checks() {
        // 9th name = bit 8 → u8 getter throws raw int (level 0, DAT_00cfe2c8).
        let names: Vec<&str> = (0..9)
            .map(|i| ["A0", "A1", "A2", "A3", "A4", "A5", "A6", "A7", "A8"][i])
            .collect();
        let mut i = ini(&["F = A8", "end"]);
        i.seed_tok(DELIMS);
        let e = i.get_bitstring_u8(&names).unwrap_err();
        assert_eq!(e.level, 0);
        let mut i = ini(&["F = A8", "end"]);
        i.seed_tok(DELIMS);
        assert_eq!(i.get_bitstring_u16(&names).unwrap(), 1 << 8);
    }

    #[test]
    fn rest_of_line_macro_split() {
        // 0x0042e8a0: unresolved token pushed whole; resolved token
        // re-split on the default delimiters.
        let mut i = ini(&["Tail plain A1B2C3", "rest here", "end"]);
        // resolve_label returns pair.1 (node+8 string) on a hit.
        i.macros.insert(
            "a1b2c3".to_string(),
            ("A1B2C3".to_string(), "X Y".to_string()),
        );
        i.seed_tok(DELIMS);
        // single-line by construction: `nextToken` (def7) never advances
        // the line table; only the block driver does.
        assert_eq!(i.get_rest_of_line(), ["plain", "X", "Y"]);
    }

    #[test]
    fn resolved_token_variants() {
        // 0x0042e03d: absent → empty string, flag untouched false.
        let mut i = ini(&["Only"]);
        let mut f = false;
        assert_eq!(i.get_optional_token(None, &mut f), "Only");
        assert!(!f); // no macro hit
        let mut f2 = false;
        assert_eq!(i.get_optional_token(None, &mut f2), "");
        // 0x0042dfe2: absent with no flag out → level 3 token_or_throw msg.
        let mut i = ini(&["End"]);
        let mut f = false;
        i.get_resolved_token(None, Some(&mut f)).unwrap(); // "End"
        let e = i.get_resolved_token(None, None).unwrap_err();
        assert_eq!(e.level, 3);
    }

    // -- wave 18b: rgba/enum-u8/bool-mask/real-range (0x0042f2ff/f3fe/
    //    ec6e/e86c/efdc/f02f/f082/f1ba) ----------------------------------

    #[test]
    fn rgba_getters() {
        let mut i = ini(&["C = R = 10 G = 20 B = 30 A = 40"]);
        i.seed_tok(DELIMS); // consume the field name
        assert_eq!(i.get_rgba().unwrap(), [10, 20, 30, 40]);
        // alpha may be omitted (defaults 0xff); packed byte order.
        let mut i = ini(&["C = R = 1 G = 2 B = 3"]);
        i.seed_tok(DELIMS);
        assert_eq!(
            i.get_rgba_packed().unwrap(),
            0xff << 24 | 1 << 16 | 2 << 8 | 3
        );
        // wrong label → level 3 `expected 'B'`.
        let mut i = ini(&["C = R = 1 G = 2 Q = 3"]);
        i.seed_tok(DELIMS);
        let e = i.get_rgba().unwrap_err();
        assert_eq!(e.level, 3);
        assert!(e.message.contains("expected 'B'"));
        // omitted B leg → level 3 omit message.
        let mut i = ini(&["C = R = 1 G = 2"]);
        i.seed_tok(DELIMS);
        let e = i.get_rgba().unwrap_err();
        assert_eq!(e.level, 3);
        assert!(e.message.contains("can't omit value for color B"));
        // value out of 0..255.
        let mut i = ini(&["C = R = 256 G = 0 B = 0"]);
        i.seed_tok(DELIMS);
        let e = i.get_rgba().unwrap_err();
        assert_eq!(e.level, 3);
        assert!(e.message.contains("out of range (0..255)"));
    }

    #[test]
    fn real_range_getters_store_before_throw() {
        // the decomp writes *param_3 before the compare — out carries the
        // value even as the load aborts.
        let mut i = ini(&["V = 0"]);
        i.seed_tok(DELIMS);
        let mut v = 9.0f32;
        let e = i.get_real_positive(&mut v).unwrap_err();
        assert_eq!((e.level, v), (3, 0.0));
        assert!(e.message.contains("expected > 0"));
        let mut i = ini(&["V = 0"]);
        i.seed_tok(DELIMS);
        let mut v = 0.0f32;
        i.get_real_nonneg(&mut v).unwrap();
        let mut i = ini(&["V = 1"]);
        i.seed_tok(DELIMS);
        let mut v = 0.0f32;
        let e = i.get_real_nonpos(&mut v).unwrap_err();
        assert_eq!((e.level, v), (3, 1.0));
        assert!(e.message.contains("expected <= 0"));
    }

    #[test]
    fn enum_u8_bool_mask_duration() {
        let mut i = ini(&["E = CHARLIE"]);
        i.seed_tok(DELIMS);
        assert_eq!(i.get_enum_u8(BNAMES).unwrap(), 2);
        let mut i = ini(&["M = yes", "N = no"]);
        i.seed_tok(DELIMS);
        let mut field = 0b0101u32;
        i.get_bool_mask(0b0010, &mut field).unwrap(); // yes → set
        i.advance_line();
        i.seed_tok(DELIMS);
        i.get_bool_mask(0b0100, &mut field).unwrap(); // no → clear
        assert_eq!(field, 0b0011); // 0b0101 |0b0010 → 0b0111, &!0b0100
        let mut i = ini(&["D = 250"]);
        i.seed_tok(DELIMS);
        assert_eq!(i.get_duration_f32().unwrap(), 2.5);
    }

    fn h1(i: &mut Ini) -> IniResult<()> {
        i.mode = 1;
        Ok(())
    }
    fn h2(i: &mut Ini) -> IniResult<()> {
        i.mode = 2;
        Ok(())
    }

    #[test]
    fn block_dispatch() {
        static H1: BlockHandler = BlockHandler {
            name: "TESTBLK",
            handler: h1,
        };
        static H2: BlockHandler = BlockHandler {
            name: "TESTBLK",
            handler: h2,
        };
        let mut i = ini(&["TESTBLK extra"]);
        i.register_block(&H1);
        i.dispatch_block().unwrap();
        assert_eq!(i.mode, 1);
        // +0x431 restored to NO_BLOCK after the handler (0x00bbdc74)
        assert_eq!(i.cur_block_name, "NO_BLOCK");
        // registry prepend: last-registered shadows (0x00babbf6 order)
        let mut i = ini(&["TESTBLK"]);
        i.register_block(&H1);
        i.register_block(&H2);
        i.dispatch_block().unwrap();
        assert_eq!(i.mode, 2);
        // block name match is an exact strcmp (c0ff walk)
        let mut i = ini(&["testblk"]);
        i.register_block(&H1);
        let e = i.dispatch_block().unwrap_err();
        assert_eq!(e.level, 5);
        assert!(e.message.contains("Unknown block 'testblk'")); // @0x00bbdbd0
                                                                // no first token: no dispatch, no error
        let mut i = ini(&["   "]);
        i.register_block(&H1);
        i.dispatch_block().unwrap();
        assert_eq!(i.mode, 0);
    }

    /// 0x0042dc7f driver: open gate -> per-line advance + dispatch ->
    /// clear state. Every line's first token names a block handler
    /// (the file band strips comments/blanks before this point).
    #[test]
    fn read_file_driver() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static RUNS: AtomicUsize = AtomicUsize::new(0);
        static DRV: BlockHandler = BlockHandler {
            name: "DRVBLK",
            handler: |i| {
                RUNS.fetch_add(1, Ordering::Relaxed);
                i.mode = 7;
                Ok(())
            },
        };
        let mut i = Ini::new(vec![]);
        i.register_block(&DRV);
        let ok = i
            .read_file(
                vec!["DRVBLK a".into(), "DRVBLK b".into()],
                "art.ini",
                0,
                &OpenGate::default(),
            )
            .unwrap();
        assert!(ok);
        assert_eq!(RUNS.load(Ordering::Relaxed), 2);
        // clear_state after the loop: filename/mode zeroed, BOTH line
        // indexes zeroed, eof cleared (set inside the loop, cleared by
        // 0x0042bf4a), line blanked.
        assert_eq!(i.mode, 0);
        assert_eq!(i.filename, "");
        assert!(!i.eof);
        assert_eq!(i.next_line, 0);
        assert_eq!(i.current_line, 0);
        assert_eq!(i.line, "");
        // gate failure: false, line state untouched
        let mut i = Ini::new(vec![]);
        let ok = i
            .read_file(
                vec!["DRVBLK".into()],
                "art.ini",
                5,
                &OpenGate {
                    clean: false,
                    macro_source: false,
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(!ok);
        assert_eq!(i.next_line, 0);
        // unknown block propagates, state NOT cleared (bf4a skipped)
        static DRV2: BlockHandler = BlockHandler {
            name: "NOPE",
            handler: h1,
        };
        let mut i = Ini::new(vec![]);
        i.register_block(&DRV2);
        let e = i
            .read_file(vec!["BADBLK".into()], "art.ini", 0, &OpenGate::default())
            .unwrap_err();
        assert_eq!(e.level, 5);
        // single line errored on the FIRST dispatch: EOF was never
        // reached, and clear_state never ran — filename still landed.
        assert!(!i.eof);
        assert_eq!(i.next_line, 1);
        assert_eq!(i.filename, "art.ini");
    }

    #[test]
    fn script_block_read() {
        // d702 self-advances: build without the helper's pre-advance
        let mut i = Ini::new(vec![
            "a = 1".into(),
            "lorem ipsum".into(),
            "endscript".into(), // terminator is _strcmpi (CI)
            "junk".into(),
        ]);
        assert_eq!(i.get_script_block(), "a = 1\nlorem ipsum");
        assert_eq!(i.next_line, 3); // ENDSCRIPT line consumed, junk not
                                    // missing ENDSCRIPT: game spins past EOF; we stop at EOF
        let mut i = Ini::new(vec!["x".into(), "y".into()]);
        assert_eq!(i.get_script_block(), "x\ny");
        assert!(i.eof);
    }

    #[test]
    fn macro_prepass() {
        let mut i = Ini::new(vec![]);
        let ok = i
            .open(
                vec![
                    "#define FOO bar baz".into(),
                    "#define QUX 1".into(),
                    "X = 2".into(),
                ],
                "art.ini",
                0,
                &OpenGate::default(),
            )
            .unwrap();
        assert!(ok);
        assert_eq!(i.macros["foo"].1, "bar baz"); // value tokens joined by ' '
        assert_eq!(i.macros["qux"].1, "1");
        // '#' NUL'd in place in the table, rest intact
        assert_eq!(i.lines[0], "\0define FOO bar baz");
        assert_eq!(i.filename, "art.ini");
        // contiguous-only rule: defines after a non-#line are not seen
        let mut i = Ini::new(vec![]);
        i.open(
            vec!["X = 2".into(), "#define LATE 3".into()],
            "art.ini",
            0,
            &OpenGate::default(),
        )
        .unwrap();
        assert!(i.macros.is_empty());
        // duplicate under mode != 5 → level 8 @0x00bbddd8
        let mut i = Ini::new(vec![]);
        let e = i
            .open(
                vec!["#define A 1".into(), "#define A 2".into()],
                "art.ini",
                0,
                &OpenGate::default(),
            )
            .unwrap_err();
        assert_eq!(e.level, 8);
        assert_eq!(e.message, "art.ini:\nDuplicate MACRO names.\nA.");
        // mode 5: quiet overwrite + version bump only on change
        let mut i = Ini::new(vec![]);
        i.open(
            vec!["#define A 1".into(), "#define A 1".into()],
            "art.ini",
            5,
            &OpenGate {
                clean: false,
                macro_source: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(i.macros["a"].1, "1");
        assert_eq!(i.macros_version, 0); // equal value: no bump
        i.open(
            vec!["#define A 2".into()],
            "art.ini",
            5,
            &OpenGate {
                clean: false,
                macro_source: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(i.macros["a"].1, "2");
        assert_eq!(i.macros_version, 1);
        // lowercase scan is STRICT: only chars between a and z throw
        let mut i = Ini::new(vec![]);
        let e = i
            .open(
                vec!["#define FOObar 1".into()],
                "art.ini",
                0,
                &OpenGate::default(),
            )
            .unwrap_err();
        assert_eq!(e.level, 8); // @0x00bbdd80
        i.open(
            vec!["#define ZZZa 1".into()],
            "art.ini",
            0,
            &OpenGate::default(),
        )
        .unwrap(); // game bug: 'a'/'z' pass the strict range
                   // map.ini ban @0x00bbddb0 + no-value @0x00bbdd54
        let mut i = Ini::new(vec![]);
        let e = i
            .open(
                vec!["#define A 1".into()],
                "x/map.ini",
                0,
                &OpenGate::default(),
            )
            .unwrap_err();
        assert_eq!(e.message, "x/map.ini:\nMACROs not allowed in map.ini.\nA.");
        let mut i = Ini::new(vec![]);
        let e = i
            .open(vec!["#define A".into()], "art.ini", 0, &OpenGate::default())
            .unwrap_err();
        assert_eq!(e.message, "art.ini:\nError parsing MACRO.\nA has no value");
        // mode-5 gate: clean + not newer → silently skipped
        let mut i = Ini::new(vec![]);
        let ok = i
            .open(
                vec!["#define A 1".into()],
                "map.ini",
                5,
                &OpenGate::default(),
            )
            .unwrap();
        assert!(!ok);
        assert_eq!(i.filename, ""); // filename/mode not stored on skip
        assert!(i.macros.is_empty());
    }

    #[test]
    fn date_gate_and_line_table() {
        let g = DateStamp {
            year: 2005,
            mon: 3,
            mday: 14,
            hour: 12,
            min: 0,
            sec: 0,
        };
        let older = DateStamp {
            year: 2004,
            mon: 12,
            mday: 31,
            ..Default::default()
        };
        let eq = g;
        let newer = DateStamp {
            year: 2005,
            mon: 3,
            mday: 14,
            hour: 12,
            min: 0,
            sec: 1,
        };
        assert!(!Ini::file_newer_than(&older, &g));
        assert!(Ini::file_newer_than(&eq, &g)); // at-or-after
        assert!(Ini::file_newer_than(&newer, &g));
        let mon_earlier = DateStamp {
            year: 2005,
            mon: 2,
            mday: 28,
            ..Default::default()
        };
        assert!(!Ini::file_newer_than(&mon_earlier, &g));
        let i = Ini::new(vec!["a".into(), "b".into()]);
        assert_eq!(i.line_count(), 2); // 0x009d5847
        assert_eq!(i.line_at(1), "b"); // 0x00a018b6
        assert_eq!(i.line_at(9), ""); // OOB → null → ""
    }

    #[test]
    fn multiline_string() {
        // closed within one token: both quotes stripped
        let mut i = ini(&["S = \"hello\""]);
        i.seed_tok(DELIMS);
        assert_eq!(i.get_multiline_string().unwrap(), "hello");
        // not quoted: raw token passthrough
        let mut i = ini(&["S = plain"]);
        i.seed_tok(DELIMS);
        assert_eq!(i.get_multiline_string().unwrap(), "plain");
        // one continuation step: `"aa` + `bb` → space-joined
        let mut i = ini(&["S = \"aa bb"]);
        i.seed_tok(DELIMS);
        assert_eq!(i.get_multiline_string().unwrap(), "aa bb");
        // second char TAB → short branch: nothing appended
        let mut i = ini(&["S = \"acc a\tb"]);
        i.seed_tok(DELIMS);
        assert_eq!(i.get_multiline_string().unwrap(), "acc");
        // exhausted line → level 3 throw (@0x00bbdfd8), NOT ea59's trim
        let mut i = ini(&["S = \"aa"]);
        i.seed_tok(DELIMS);
        assert_eq!(i.get_multiline_string().unwrap_err().level, 3);
        // no token → empty, no throw
        let mut i = ini(&["S ="]);
        i.seed_tok(DELIMS);
        assert_eq!(i.get_multiline_string().unwrap(), "");
    }

    #[test]
    fn gi_dat() {
        // f720: CI key chain, unknown dropped
        let mut v = GiValues::default();
        set_gi_value(&mut v, "gamename", "LotROtE");
        set_gi_value(&mut v, "G2", "x");
        set_gi_value(&mut v, "Bogus", "y");
        assert_eq!(v.game_name.as_deref(), Some("LotROtE"));
        assert_eq!(v.g2.as_deref(), Some("x"));
        assert_eq!(v.online_server, None);
        // f840: magic + count + NUL pairs
        let mut f = b"GI  ".to_vec();
        f.extend_from_slice(&3u32.to_le_bytes());
        f.extend(b"GameName\0BFME2\0SkuName\0EA\0UNKNOWNKEY\0zz\0");
        let g = parse_gi_dat(&f);
        assert_eq!(g.game_name.as_deref(), Some("BFME2"));
        assert_eq!(g.sku_name.as_deref(), Some("EA"));
        assert_eq!(g.g1, None); // unknown key dropped
                                // count bounds the pair walk: a 4th pair beyond count=3 ignored
        let mut f2 = f.clone();
        f2.extend(b"GameName\0IGNORED\0");
        assert_eq!(parse_gi_dat(&f2).game_name.as_deref(), Some("BFME2"));
        // bad magic / short file → all None (fopen/magic guard)
        assert_eq!(parse_gi_dat(b"NOPE123456").game_name, None);
        assert_eq!(parse_gi_dat(b"GI ").game_name, None);
    }
}
