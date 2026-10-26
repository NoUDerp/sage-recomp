//! SAGE XML document parser — mis-clustered into the `Weapon` band
//! (consumer 0x00942997 errors `s_AutoResolveWeapon_entry_in_Objec_
//! 00c69568`; class vtable `PTR_FUN_00c69514` via ctor 0x00942383).
//!
//! Mirror of the tokenizer core `0x009423f0..0x00942924` (13 rows).
//! Statements returned by `next_statement`: 0 = EOF, 1 = element-start,
//! 2 = element-end (`</tag>` or `/>`), 3 = text, -1 = error (parses
//! stopped by `reset`, 0x00942459). Attributes land in a 4-entry argv
//! (`key` 0x21 B / `value` 0x40 B fields, stride 0x61, pinned by the
//! `lea eax,[eax+esi+0x2c]`/`0x4d`/`0x8c` offsets at 0x0094263b..
//! 0x00942687). Entities = 5-entry table at `PTR_DAT_00dd1e74` with
//! int values at `DAT_00dd1e78`: lt=60 gt=62 amp=38 quot=34 apos=39.
//!
//! The parser mutates the document in place (NUL-terminates tag names
//! at their first char, restored by the pushback pair 0x009427f1/
//! 0x0094288b) — mirrored by `doc: Vec<u8>`. Ghidra's per-function
//! pointer typing disagrees across the cluster; all field offsets were
//! pinned from disassembly (`cmp byte [esi+0x10],bl` at 0x00942896,
//! `inc dword [esi+4]` at 0x009426cb, etc.).

/// Statement codes (return of `next_statement`, via element 0x009424c4 /
/// text 0x00942791 / prologue tail 0x00942890).
pub const ST_EOF: i32 = 0;
pub const ST_ELEMENT_START: i32 = 1;
pub const ST_ELEMENT_END: i32 = 2;
pub const ST_TEXT: i32 = 3;
pub const ST_ERROR: i32 = -1;

const KEY_FIELD: usize = 0x21; // strncpy 0x20 + explicit NUL at +0x20
const VAL_FIELD: usize = 0x40; // strncpy 0x3f + explicit NUL at +0x3f
pub const ARGV_MAX: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Arg {
    pub key: [u8; KEY_FIELD],
    pub value: [u8; VAL_FIELD],
}

/// Token location: element names are positions in `doc`, text tokens
/// index the text buffer (the game stores a raw `char*` in +0x1c; the
/// two domains never mix within one statement).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Loc {
    Doc(usize),
    Text(usize),
}

#[derive(Clone, Debug)]
pub struct XmlCtx {
    /// +0x00 cursor (NULL after `reset`), as an index into `doc`
    /// (`doc` always carries a trailing 0).
    pub doc: Vec<u8>,
    pub cursor: Option<usize>,
    /// +0x04 element depth
    pub depth: i32,
    /// +0x08 line-start cursor (skip_ws bookkeeping)
    pub line_start: usize,
    /// +0x0c line counter
    pub line: i32,
    /// +0x10 "element self-closed" flag (`cmp byte [esi+0x10], bl`)
    pub end_flag: u8,
    /// +0x14 text buffer base + capacity index (+0x18)
    pub text_buf: Vec<u8>,
    pub text_cap: usize,
    /// +0x1c current token
    pub token: Option<Loc>,
    /// +0x20 pushback pointer (into `doc`) + +0x24 saved char
    pub push_ptr: Option<usize>,
    pub push_char: u8,
    /// +0x28 arg count, +0x2c argv[4]
    pub argc: usize,
    pub argv: [Arg; ARGV_MAX],
}

impl Default for XmlCtx {
    fn default() -> Self {
        Self {
            doc: vec![0],
            cursor: None,
            depth: 0,
            line_start: 0,
            line: 0,
            end_flag: 0,
            text_buf: vec![0],
            text_cap: 0,
            token: None,
            push_ptr: None,
            push_char: 0,
            argc: 0,
            argv: [Arg {
                key: [0; KEY_FIELD],
                value: [0; VAL_FIELD],
            }; ARGV_MAX],
        }
    }
}

fn is_ws(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}
fn is_alnum(c: u8) -> bool {
    c.is_ascii_alphanumeric()
}

impl XmlCtx {
    /// 0x00942806 — context init: zero header (+0x00..+0x14), line = 1,
    /// zero argv (0x184 bytes at +0x2c), set text buffer/capacity,
    /// cursor = doc. XML declaration: if the doc opens `<?`, the parser
    /// requires the exact 0x15-byte prolog `<?xml version="1.0"?>`
    /// (`s_<_xml_version__1_0__>_00c6954c`); a mismatch sets cursor 0
    /// and then *still* advances by 0x15 (faithful quirk).
    pub fn new(doc: &[u8], text_buf_len: usize, text_cap: usize) -> Self {
        let mut ctx = XmlCtx {
            doc: {
                let mut d = doc.to_vec();
                d.push(0);
                d
            },
            text_buf: vec![0; text_buf_len.max(1)],
            text_cap,
            ..Default::default()
        };
        ctx.line = 1;
        let start = 0usize;
        if ctx.doc[start] == b'<' && ctx.doc[start + 1] == b'?' {
            const PROLOG: &[u8] = b"<?xml version=\"1.0\"?>";
            if !ctx.doc[start..].starts_with(PROLOG) {
                ctx.cursor = Some(0);
            } else {
                ctx.cursor = Some(start);
            }
            ctx.cursor = ctx.cursor.map(|c| c + 0x15);
        } else {
            ctx.cursor = Some(start);
        }
        ctx
    }

    /// 0x009423f0 — advance past whitespace from `p`, bumping the line
    /// counter and recording the line start on each `'\n'` reached.
    /// Returns the position of the first non-space byte (or the NUL).
    pub fn skip_ws(&mut self, mut p: usize) -> usize {
        loop {
            if self.doc[p] == 0 {
                return p;
            }
            if !is_ws(self.doc[p]) {
                break;
            }
            if self.doc[p] == b'\n' {
                self.line += 1;
                self.line_start = p + 1;
            }
            p += 1;
        }
        p
    }

    /// 0x00942425 — advance past `[A-Za-z0-9_]+`.
    pub fn scan_word(&self, mut p: usize) -> usize {
        loop {
            if self.doc[p] == 0 {
                return p;
            }
            if !is_alnum(self.doc[p]) && self.doc[p] != b'_' {
                break;
            }
            p += 1;
        }
        p
    }

    /// 0x00942459 — abort/reset: the game first snapshots the current
    /// line into a 260-byte stack buffer (result never used — dead) and
    /// zeroes cursor, depth (+0x04), line start (+0x08) and line
    /// (+0x0c). `end_flag` (+0x10) is NOT touched.
    pub fn reset(&mut self) {
        self.cursor = None;
        self.depth = 0;
        self.line_start = 0;
        self.line = 0;
    }

    /// 0x009426db — entity decode at the cursor (`&name;`): skip the
    /// `&`, match the 5 table names (`PTR_DAT_00dd1e74`: lt/gt/amp/
    /// quot/apos), require `';'`, store the decoded byte into
    /// `text_buf[out_idx]`. Returns the game's 1/0.
    pub fn parse_entity(&mut self, out_idx: usize) -> u8 {
        let mut p = match self.cursor {
            None => return 0,
            Some(p) => p + 1,
        };
        self.cursor = Some(p);
        if self.doc[p] == 0 {
            return 0;
        }
        const TABLE: [(&[u8], u8); 5] = [
            (b"lt", 60),
            (b"gt", 62),
            (b"amp", 38),
            (b"quot", 34),
            (b"apos", 39),
        ];
        for (name, val) in TABLE {
            if self.doc[p..].starts_with(name) {
                p += name.len();
                self.cursor = Some(p);
                if self.doc[p] == b';' {
                    self.text_buf[out_idx] = val;
                    return 1;
                }
                return 0;
            }
        }
        0
    }

    /// 0x0094273b — `<!` comment: cursor must sit on `<`, skips 2,
    /// requires `--`, then scans for the `-->` terminator. On success
    /// the cursor lands after `>` and the fn returns 1; any other
    /// shape leaves the cursor mid-scan and returns 0.
    pub fn parse_comment(&mut self) -> u8 {
        let mut p = self.cursor.unwrap() + 2;
        self.cursor = Some(p);
        if self.doc[p] == b'-' {
            p += 1;
            self.cursor = Some(p);
            if self.doc[p] == b'-' {
                p += 1;
                self.cursor = Some(p);
                if self.doc[p] != 0 {
                    loop {
                        let mut q = self.cursor.unwrap();
                        while self.doc[q] != 0 && self.doc[q] != b'-' {
                            q += 1;
                        }
                        self.cursor = Some(q);
                        if self.doc[q] == 0 {
                            break;
                        }
                        let q1 = q + 1;
                        self.cursor = Some(q1);
                        if self.doc[q1] == 0 {
                            break;
                        }
                        if self.doc[q1] == b'-' {
                            let q2 = q + 2;
                            self.cursor = Some(q2);
                            if self.doc[q2] == 0 {
                                break;
                            }
                            if self.doc[q2] == b'>' {
                                self.cursor = Some(q + 3);
                                return 1;
                            }
                        }
                    }
                }
            }
        }
        0
    }

    /// 0x00942791 — text run: copy bytes into `text_buf`, decoding `&`
    /// entities (a failed entity, or the doc ending before any `<`, is
    /// the 0xffffffff error path — `reset` + -1). Terminates on `<` or
    /// a NUL cursor with NUL-terminated text, token = Text(0), 3.
    pub fn parse_text(&mut self) -> i32 {
        let mut i = 0usize;
        loop {
            let c = self.doc[self.cursor.unwrap()];
            if c == b'&' {
                if self.parse_entity(i) == 0 {
                    self.reset();
                    return ST_ERROR;
                }
            } else {
                self.text_buf[i] = c;
            }
            let c = self.doc[self.cursor.unwrap()];
            if c == b'<' || c == 0 {
                self.text_buf[i] = 0;
                self.token = Some(Loc::Text(0));
                return ST_TEXT;
            }
            let nx = self.cursor.unwrap() + 1;
            self.cursor = Some(nx);
            if self.doc[nx] == 0 {
                self.reset();
                return ST_ERROR;
            }
            if i <= self.text_cap {
                i += 1;
            }
        }
    }

    /// 0x009427f1 (and its 5-byte twin 0x0094288b) — pushback: restore
    /// the tag-name byte NUL-ed out by `element`, then clear the slots.
    pub fn pushback(&mut self) {
        if let Some(p) = self.push_ptr {
            self.doc[p] = self.push_char;
            self.push_ptr = None;
            self.push_char = 0;
        }
    }

    /// 0x009424c4 — element statement. Cursor sits on `<`; the body
    /// first does `skip_ws(cursor)+1` (consuming the `<`), then
    /// optionally `/'` for an end tag. Tag name is recorded at
    /// +0x1c; `key="value"` / `key='value'` pairs (max 4) are copied
    /// into argv with the game's temporary NUL-termination of the
    /// source. Trailing `/` sets `end_flag`; anything other than `/`
    /// or `>` errors. Returns 1 (start) / 2 (end) / -1.
    pub fn element(&mut self) -> i32 {
        let ws = self.skip_ws(self.cursor.unwrap());
        let mut p = ws + 1;
        self.end_flag = 0;
        if self.doc[p] == 0 {
            self.reset();
            return ST_ERROR;
        }
        let mut kind = 1i32;
        if self.doc[p] == b'/' {
            p += 1; // ws+2 = after the slash
            self.cursor = Some(p);
            kind = 2;
            if self.doc[p] == 0 {
                self.reset();
                return ST_ERROR;
            }
        } else {
            self.cursor = Some(p);
        }
        let name = self.skip_ws(self.cursor.unwrap());
        self.cursor = Some(name);
        if self.doc[name] == 0 {
            self.reset();
            return ST_ERROR;
        }
        self.token = Some(Loc::Doc(name));
        let wend = self.scan_word(name);
        self.cursor = Some(wend);
        if self.doc[wend] == 0 {
            self.reset();
            return ST_ERROR;
        }
        let mut q = self.skip_ws(wend);
        self.cursor = Some(q);
        if self.doc[q] == 0 {
            self.reset();
            return ST_ERROR;
        }
        self.argc = 0;
        if kind == 1 {
            while is_alnum(self.doc[self.cursor.unwrap()]) {
                let key_start = self.cursor.unwrap();
                let key_end = self.scan_word(key_start);
                self.cursor = Some(key_end);
                if self.doc[key_end] == 0 {
                    self.reset();
                    return ST_ERROR;
                }
                q = self.skip_ws(key_end);
                self.cursor = Some(q);
                if self.doc[q] == 0 || self.doc[q] != b'=' {
                    self.reset();
                    return ST_ERROR;
                }
                let v = q + 1;
                self.cursor = Some(v);
                if self.doc[v] == 0 {
                    self.reset();
                    return ST_ERROR;
                }
                q = self.skip_ws(v);
                self.cursor = Some(q);
                let quote = self.doc[q];
                if quote == 0 || (quote != b'"' && quote != b'\'') {
                    self.reset();
                    return ST_ERROR;
                }
                let v_start = q + 1;
                self.cursor = Some(v_start);
                let mut qe = v_start;
                if self.doc[qe] == 0 {
                    self.reset();
                    return ST_ERROR;
                }
                while self.doc[qe] != 0 && self.doc[qe] != quote {
                    qe += 1;
                }
                self.cursor = Some(qe);
                if self.doc[qe] == 0 {
                    self.reset();
                    return ST_ERROR;
                }
                let after = qe + 1;
                self.cursor = Some(after);
                if self.doc[after] == 0 {
                    self.reset();
                    return ST_ERROR;
                }
                q = self.skip_ws(after);
                self.cursor = Some(q);
                if self.doc[q] == 0 {
                    self.reset();
                    return ST_ERROR;
                }
                if self.argc < ARGV_MAX {
                    // strncpy semantics: copy then NUL-pad to the field
                    // size; the game NUL-terminates the source in place
                    // and restores it.
                    let saved_k = self.doc[key_end];
                    self.doc[key_end] = 0;
                    let key = &self.doc[key_start..key_end];
                    let f = &mut self.argv[self.argc].key;
                    f.fill(0);
                    let n = key.len().min(0x20);
                    f[..n].copy_from_slice(&key[..n]);
                    self.doc[key_end] = saved_k;
                    let saved_v = self.doc[qe];
                    self.doc[qe] = 0;
                    let val = &self.doc[v_start..qe];
                    let f = &mut self.argv[self.argc].value;
                    f.fill(0);
                    let n = val.len().min(0x3f);
                    f[..n].copy_from_slice(&val[..n]);
                    self.doc[qe] = saved_v;
                    self.argc += 1;
                }
            }
        }
        let c = self.doc[self.cursor.unwrap()];
        if c == b'/' {
            // flag only — the cursor stays on the slash; the driver's
            // continuation branch consumes it on the next statement
            self.end_flag = 1;
        } else if c != b'>' {
            self.reset();
            return ST_ERROR;
        } else {
            self.cursor = Some(self.cursor.unwrap() + 1);
        }
        // consumer-visible name termination: NUL at the WORD END
        // (disasm `mov [esi+0x20],ebx` with ebx = scan_word result);
        // the Doc token then reads to that NUL, and pushback restores
        self.push_ptr = Some(wend);
        self.push_char = self.doc[wend];
        self.doc[wend] = 0;
        if kind == 1 {
            self.depth += 1;
        } else {
            self.depth -= 1;
        }
        kind
    }

    /// 0x00942890 — statement driver: handles the `/>`-continuation
    /// (end_flag set + cursor on `/`), else pushback → skip_ws →
    /// EOF / text / element / `<!` comment loop.
    pub fn next_statement(&mut self) -> i32 {
        let cur_ok = self.cursor.is_some();
        let slash_after = cur_ok && self.doc[self.cursor.unwrap()] == b'/';
        if self.end_flag == 0 || !slash_after {
            self.pushback();
            if cur_ok {
                let mut p = self.cursor.unwrap();
                loop {
                    p = self.skip_ws(p);
                    self.cursor = Some(p);
                    if self.doc[p] == 0 {
                        self.token = None;
                        self.argc = 0;
                        return ST_EOF;
                    }
                    if self.doc[p] != b'<' {
                        return self.parse_text();
                    }
                    if self.doc[p + 1] != b'!' {
                        return self.element();
                    }
                    if self.parse_comment() == 0 {
                        break;
                    }
                    p = self.cursor.unwrap();
                }
            }
        } else {
            let nx = self.cursor.unwrap() + 1;
            self.cursor = Some(nx);
            if self.doc[nx] == b'>' {
                self.cursor = Some(nx + 1);
                return ST_ELEMENT_END; // end_flag NOT cleared (game)
            }
        }
        self.reset();
        ST_ERROR
    }

    /// 0x00942909 — `argv[i].key` if `0 <= i < argc`, else None (NULL).
    pub fn arg_key(&self, i: isize) -> Option<&[u8; KEY_FIELD]> {
        if i < 0 || (self.argc as isize) <= i {
            None
        } else {
            Some(&self.argv[i as usize].key)
        }
    }

    /// 0x00942924 — same guard, `argv[i].value`.
    pub fn arg_value(&self, i: isize) -> Option<&[u8; VAL_FIELD]> {
        if i < 0 || (self.argc as isize) <= i {
            None
        } else {
            Some(&self.argv[i as usize].value)
        }
    }

    /// 0x009c4ae1 — parser-side argv count accessor (the XML class
    /// exposes the statement's arg count; same +0x28 field).
    pub fn arg_count(&self) -> usize {
        self.argc
    }

    /// 0x00942953 — thiscall seek/restore: pushback, clear token
    /// (+0x1c) and argc (+0x28), then overwrite the five header dwords
    /// (+0x00 cursor .. +0x10 end_flag) from the caller's stack block
    /// (`rep movsd`, ECX=5, `ret 0x14`). Used to rewind the parser to
    /// a saved scan state.
    pub fn restore_scan_state(
        &mut self,
        cursor: Option<usize>,
        depth: i32,
        line_start: usize,
        line: i32,
        end_flag: u8,
    ) {
        self.pushback();
        self.token = None;
        self.argc = 0;
        self.cursor = cursor;
        self.depth = depth;
        self.line_start = line_start;
        self.line = line;
        self.end_flag = end_flag;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(doc: &[u8]) -> Vec<(i32, Option<Vec<u8>>)> {
        let mut ctx = XmlCtx::new(doc, 0x1000, 0x1000);
        let mut out = Vec::new();
        loop {
            let st = ctx.next_statement();
            if st == ST_EOF || st == ST_ERROR {
                out.push((st, None));
                return out;
            }
            let name = match ctx.token {
                Some(Loc::Doc(p)) => {
                    let mut v = Vec::new();
                    let mut i = p;
                    while ctx.doc[i] != 0 {
                        v.push(ctx.doc[i]);
                        i += 1;
                    }
                    Some(v)
                }
                Some(Loc::Text(_)) => {
                    let mut v = Vec::new();
                    let mut i = 0;
                    while ctx.text_buf[i] != 0 {
                        v.push(ctx.text_buf[i]);
                        i += 1;
                    }
                    Some(v)
                }
                None => None,
            };
            out.push((st, name));
        }
    }

    fn s(b: Option<Vec<u8>>) -> String {
        String::from_utf8(b.unwrap_or_default()).unwrap()
    }

    #[test]
    fn prolog_elements_comment_text_end() {
        let doc = b"<?xml version=\"1.0\"?><root a=\"1\" b='two'><!--note-->\
                    hello&gt;world<child/><x/></root>";
        let out = run(doc);
        // Game-faithful self-closing semantics: element() NULs the
        // word-end (the `/`), so the driver's continuation branch
        // never sees it at entry — `/>` comes back as a TEXT
        // statement (the pushback restores the slash before scanning).
        assert_eq!(
            out,
            vec![
                (ST_ELEMENT_START, Some("root".into())),
                (ST_TEXT, Some("hello>world".into())),
                (ST_ELEMENT_START, Some("child".into())),
                (ST_TEXT, Some("/>".into())),
                (ST_ELEMENT_START, Some("x".into())),
                (ST_TEXT, Some("/>".into())),
                (ST_ELEMENT_END, Some("root".into())),
                (ST_EOF, None),
            ]
        );
    }

    #[test]
    fn attributes_argv_fields_and_getters() {
        // key/value fields: strncpy NUL-padded; >4 args are dropped
        let doc = b"<w a=\"1\" b='2' c=\"3\" d=\"4\" e=\"5\"/><!--end-->";
        let mut ctx = XmlCtx::new(doc, 0x1000, 0x1000);
        let st = ctx.next_statement();
        assert_eq!(st, ST_ELEMENT_START);
        assert_eq!(ctx.argc, 4); // e="5" silently dropped (max 4)
        assert_eq!(ctx.arg_count(), 4);
        // (key field holds just the key; value field the value)
        assert_eq!(ctx.arg_key(0).unwrap()[0], b'a');
        assert_eq!(ctx.arg_value(0).unwrap()[0], b'1');
        assert_eq!(ctx.arg_key(1).unwrap()[0], b'b');
        assert_eq!(ctx.arg_value(1).unwrap()[0], b'2');
        assert_eq!(ctx.arg_key(4), None);
        assert_eq!(ctx.arg_key(-1), None);
        // pushback restored the name start char
        assert!(ctx.push_ptr.is_some()); // name NUL pending until next stmt
        assert!(ctx.depth == 1);
    }

    #[test]
    fn bad_shapes_error() {
        // unterminated attribute
        assert_eq!(run(b"<a k=\"v").last().unwrap().0, ST_ERROR);
        // entity without ';' is the text-error path
        assert_eq!(run(b"text&ampx<").last().unwrap().0, ST_ERROR);
        // comment missing `--` after `<!`
        assert_eq!(run(b"<x><!--ok--><").last().unwrap().0, ST_ERROR);
    }

    #[test]
    fn restore_scan_state_pushes_back_and_reloads_header() {
        let mut ctx = XmlCtx::new(b"<ab c=\"1\">", 0x40, 0x40);
        assert_eq!(ctx.next_statement(), ST_ELEMENT_START);
        assert_eq!(ctx.argc, 1);
        // element NUL-ed the word end (the space at idx 3)
        assert_eq!(ctx.doc[3], 0);
        ctx.push_ptr = Some(3);
        ctx.push_char = b' ';
        ctx.restore_scan_state(Some(0), 7, 3, 9, 1);
        assert_eq!(ctx.doc[3], b' '); // pushback executed first
        assert_eq!(ctx.token, None);
        assert_eq!(ctx.argc, 0);
        assert_eq!(ctx.cursor, Some(0));
        assert_eq!(
            (ctx.depth, ctx.line_start, ctx.line, ctx.end_flag),
            (7, 3, 9, 1)
        );
    }
}
