//! Pele common/outline.ts, 458322a5 (MIT, kepano). Shared indented-outline grammar.
use crate::compat::{
    source::{space, word},
    PeleError,
};
const END: u8 = 0;
const ERROR: u8 = 1;
const SPACELINE: u8 = 2;
const NL: u8 = 3;
const EOF: u8 = 4;
const KEYWORD: u8 = 5;
const SPACELIST: u8 = 6;
const ICON: u8 = 7;
const CLASS: u8 = 8;
const DSTART: u8 = 9;
const ID: u8 = 10;
const DESCR: u8 = 11;
const DEND: u8 = 12;
const DATA: u8 = 13;
const NAMES: [&str; 14] = [
    "1",
    "INVALID",
    "SPACELINE",
    "NL",
    "EOF",
    "",
    "SPACELIST",
    "ICON",
    "CLASS",
    "NODE_DSTART",
    "NODE_ID",
    "NODE_DESCR",
    "NODE_DEND",
    "SHAPE_DATA",
];
pub fn token_name(t: u8, kanban: bool) -> &'static str {
    if t == KEYWORD {
        if kanban {
            "KANBAN"
        } else {
            "MINDMAP"
        }
    } else {
        NAMES[t as usize]
    }
}
pub fn get_type(start: &str, end: &str) -> u8 {
    match start {
        "[" => 2,
        "(" => {
            if end == ")" {
                1
            } else {
                4
            }
        }
        "((" => 3,
        ")" => 4,
        "))" => 5,
        "{{" => 6,
        _ => 0,
    }
}
pub trait OutlineDb {
    fn get_type(&self, start: &str, end: &str) -> u8 {
        get_type(start, end)
    }
    fn add_node(
        &mut self,
        level: usize,
        id: String,
        descr: String,
        node_type: u8,
        shape_data: Option<String>,
    ) -> Result<(), PeleError>;
    fn decorate_node(&mut self, icon: Option<String>, class: Option<String>);
}
#[derive(Debug)]
pub struct Tokens {
    pub types: Vec<u8>,
    pub texts: Vec<String>,
    pub starts: Vec<usize>,
}
fn ch(s: &str, p: usize) -> Option<char> {
    s.get(p..)?.chars().next()
}
fn letters(s: &str, p: usize, w: &str) -> bool {
    s.get(p..p + w.len())
        .is_some_and(|v| v.eq_ignore_ascii_case(w))
}
fn scan(s: &str, p: usize, pred: impl Fn(char) -> bool) -> usize {
    s[p..]
        .char_indices()
        .find(|(_, c)| !pred(*c))
        .map_or(s.len(), |(i, _)| p + i)
}
fn line(s: &str, p: usize) -> usize {
    scan(s, p, |c| {
        c != '\n' && c != '\r' && c != '\u{2028}' && c != '\u{2029}'
    })
}
fn upto(s: &str, p: usize, c: char) -> usize {
    s[p..].find(c).map_or(s.len(), |i| p + i)
}
pub fn tokenize(src: &str, kanban: bool) -> Tokens {
    let n = src.len();
    let mut out = Tokens {
        types: vec![],
        texts: vec![],
        starts: vec![],
    };
    let mut state = 0;
    let mut p = 0;
    let mut next_break = None;
    while p < n {
        let c = ch(src, p).unwrap();
        let d = ch(src, p + c.len_utf8());
        let from = p;
        let kind;
        let mut text = None;
        let e;
        match state {
            0 => {
                let w = scan(src, p, space);
                if kanban && src[p..].starts_with("@{") {
                    kind = Some(DATA);
                    text = Some(String::new());
                    e = p + 2;
                    state = 6
                } else if src[w..].starts_with("%%") {
                    kind = Some(SPACELINE);
                    e = line(src, w)
                } else if w > p {
                    let b = *next_break.get_or_insert_with(|| upto(src, p, '\n'));
                    let b = if b < p {
                        let next = upto(src, p, '\n');
                        next_break = Some(next);
                        next
                    } else {
                        b
                    };
                    if b >= w {
                        kind = Some(SPACELIST);
                        e = w
                    } else if let Some(last) =
                        src[p..w].rfind('\n').map(|i| p + i).filter(|i| *i > p)
                    {
                        kind = Some(SPACELINE);
                        e = last + 1
                    } else {
                        kind = Some(NL);
                        e = p + 1
                    }
                } else if letters(src, p, if kanban { "kanban" } else { "mindmap" })
                    && !ch(src, p + if kanban { 6 } else { 7 }).is_some_and(word)
                {
                    kind = Some(KEYWORD);
                    e = p + if kanban { 6 } else { 7 }
                } else if src[p..].starts_with(":::") {
                    p += 3;
                    state = 1;
                    continue;
                } else if src[p..].starts_with("::")
                    && letters(src, p + 2, "icon")
                    && ch(src, p + 6) == Some('(')
                {
                    p += 7;
                    state = 2;
                    continue;
                } else if (c == '-' && d == Some(')'))
                    || (c == '(' && matches!(d, Some('-' | '(')))
                    || (matches!(c, ')' | '{') && d == Some(c))
                {
                    kind = Some(DSTART);
                    e = p + 2;
                    state = 3
                } else if matches!(c, ')' | '(' | '[') {
                    kind = Some(DSTART);
                    e = p + 1;
                    state = 3
                } else {
                    e = scan(src, p, |c| {
                        !matches!(c, '(' | '[' | '\n' | ')' | '{' | '}') && (!kanban || c != '@')
                    });
                    if e == p {
                        break;
                    }
                    kind = Some(ID)
                }
            }
            3 => {
                if c == '"' {
                    state = if d == Some('`') { 5 } else { 4 };
                    p += if d == Some('`') { 2 } else { 1 };
                    continue;
                } else if matches!(c, ')' | '(') {
                    kind = Some(DEND);
                    e = p + if d == Some(c) || (c == '(' && d == Some('-')) {
                        2
                    } else {
                        1
                    };
                    state = 0
                } else if c == ']' {
                    kind = Some(DEND);
                    e = p + 1;
                    state = 0
                } else if (c == '}' && d == Some('}')) || (c == '-' && d == Some(')')) {
                    kind = Some(DEND);
                    e = p + 2;
                    state = 0
                } else {
                    e = if c == '}' {
                        line(src, p)
                    } else {
                        scan(src, p, |c| !matches!(c, ')' | ']' | '(' | '}'))
                    };
                    if e == p {
                        break;
                    }
                    kind = Some(DESCR)
                }
            }
            4 => {
                if c == '"' {
                    p += 1;
                    state = 3;
                    continue;
                } else {
                    e = upto(src, p, '"');
                    kind = Some(DESCR)
                }
            }
            5 => {
                if c == '`' && d == Some('"') {
                    p += 2;
                    state = 3;
                    continue;
                } else {
                    e = scan(src, p, |c| c != '`' && c != '"');
                    if e == p {
                        break;
                    }
                    kind = Some(DESCR)
                }
            }
            1 => {
                e = line(src, p);
                if e > p {
                    kind = Some(CLASS);
                    state = 0
                } else if c == '\n' {
                    p += 1;
                    state = 0;
                    continue;
                } else {
                    break;
                }
            }
            2 => {
                if c == ')' {
                    p += 1;
                    state = 0;
                    continue;
                } else {
                    kind = Some(ICON);
                    e = upto(src, p, ')')
                }
            }
            6 => {
                if c == '"' {
                    kind = Some(DATA);
                    e = p + 1;
                    state = 7
                } else if c == '}' {
                    p += 1;
                    state = 0;
                    continue;
                } else {
                    e = scan(src, p, |c| !matches!(c, '}' | '^' | '"'));
                    if e == p {
                        break;
                    }
                    kind = Some(DATA)
                }
            }
            _ => {
                kind = Some(DATA);
                if c == '"' {
                    e = p + 1;
                    state = 6
                } else {
                    e = upto(src, p, '"');
                    let mut t = String::new();
                    let raw = &src[p..e];
                    let mut at = 0;
                    while let Some(i) = raw[at..].find('\n') {
                        t.push_str(&raw[at..at + i]);
                        t.push_str("<br/>");
                        at = scan(raw, at + i + 1, space)
                    }
                    t.push_str(&raw[at..]);
                    text = Some(t)
                }
            }
        }
        if let Some(k) = kind {
            out.types.push(k);
            out.texts.push(text.unwrap_or_else(|| src[from..e].into()));
            out.starts.push(from)
        }
        p = e;
    }
    let mut put = |k| {
        out.types.push(k);
        out.texts.push(String::new());
        out.starts.push(p)
    };
    if p < n {
        put(ERROR)
    } else {
        if state == 0 {
            put(EOF)
        }
        put(END)
    }
    out
}
fn stop(t: u8) -> bool {
    matches!(t, SPACELINE | NL | EOF)
}
pub fn parse_outline(src: &str, db: &mut impl OutlineDb, kanban: bool) -> Result<(), PeleError> {
    let diagram = if kanban { "kanban" } else { "mindmap" };
    let ts = tokenize(src, kanban);
    let mut i = 0;
    let types = &ts.types;
    let texts = &ts.texts;
    let fail = |i: usize, expected: &str| {
        let t = *types.get(i).unwrap_or(&END);
        PeleError::syntax(
            diagram,
            src,
            *ts.starts.get(i).unwrap_or(&src.len()),
            if t == ERROR {
                String::new()
            } else {
                format!("Expecting {expected}, got '{}'", token_name(t, kanban))
            }
            .as_str(),
            t == ERROR,
        )
    };
    let data = if kanban { ", 'SHAPE_DATA'" } else { "" };
    let keyword = format!("'{}'", token_name(KEYWORD, kanban));
    if types[i] == SPACELINE {
        i += 1;
        while matches!(types[i], SPACELINE | NL) {
            i += 1
        }
        if types[i] != KEYWORD {
            return Err(fail(i, &format!("'SPACELINE', 'NL', {keyword}")));
        }
    } else if types[i] != KEYWORD {
        return Err(fail(i, &format!("'SPACELINE', {keyword}")));
    }
    i += 1;
    let mut first = "'SPACELINE', 'NL', 'SPACELIST', 'ICON', 'CLASS', 'NODE_DSTART', 'NODE_ID'";
    if types[i] == NL {
        i += 1;
        first = "'SPACELINE', 'SPACELIST', 'ICON', 'CLASS', 'NODE_DSTART', 'NODE_ID'"
    }
    loop {
        let mut t = types[i];
        let mut level = 0;
        let indented = t == SPACELIST;
        if indented {
            level = texts[i].encode_utf16().count();
            i += 1;
            t = types[i]
        }
        if matches!(t, ID | DSTART) {
            let mut id = None;
            let descr;
            let mut ty = 0;
            if t == ID {
                id = Some(texts[i].clone());
                i += 1
            }
            if types[i] == DSTART {
                let start = &texts[i];
                i += 1;
                if types[i] != DESCR {
                    return Err(fail(i, "'NODE_DESCR'"));
                }
                descr = texts[i].clone();
                i += 1;
                if types[i] != DEND {
                    return Err(fail(i, "'NODE_DEND'"));
                }
                let end = &texts[i];
                i += 1;
                if !stop(types[i]) && types[i] != DATA {
                    return Err(fail(i, &format!("'SPACELINE', 'NL', 'EOF'{data}")));
                }
                ty = db.get_type(start, end)
            } else {
                if !stop(types[i]) && types[i] != DATA {
                    return Err(fail(
                        i,
                        &format!("'SPACELINE', 'NL', 'EOF', 'NODE_DSTART'{data}"),
                    ));
                }
                descr = id.clone().unwrap()
            }
            let shape_data = if types[i] == DATA {
                let mut s = texts[i].clone();
                i += 1;
                while types[i] == DATA {
                    s.push_str(&texts[i]);
                    i += 1
                }
                if !stop(types[i]) {
                    return Err(fail(i, &format!("'SPACELINE', 'NL', 'EOF'{data}")));
                }
                Some(s)
            } else {
                None
            };
            db.add_node(
                level,
                id.unwrap_or_else(|| descr.clone()),
                descr,
                ty,
                shape_data,
            )?;
        } else if matches!(t, ICON | CLASS) {
            let text = texts[i].clone();
            i += 1;
            if !stop(types[i]) {
                return Err(fail(i, "'SPACELINE', 'NL', 'EOF'"));
            }
            db.decorate_node(
                (t == ICON).then_some(text.clone()),
                (t == CLASS).then_some(text),
            )
        } else if indented {
            if !stop(t) {
                return Err(fail(
                    i,
                    "'SPACELINE', 'NL', 'EOF', 'ICON', 'CLASS', 'NODE_DSTART', 'NODE_ID'",
                ));
            }
        } else if t == SPACELINE {
            i += 1;
            if !stop(types[i]) {
                return Err(fail(i, "'SPACELINE', 'NL', 'EOF'"));
            }
        } else {
            return Err(fail(i, first));
        }
        i += 1;
        while matches!(types[i], NL | EOF) {
            i += 1
        }
        if types[i] == END {
            return Ok(());
        }
        if !matches!(types[i], SPACELINE | SPACELIST | ICON | CLASS | DSTART | ID) {
            return Err(fail(
                i,
                "'SPACELINE', 'NL', 'EOF', 'SPACELIST', 'ICON', 'CLASS', 'NODE_DSTART', 'NODE_ID'",
            ));
        }
    }
}
