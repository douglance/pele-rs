//! State lexer translated from Pele block/lexer.ts, 458322a5 (MIT, kepano).
use super::tokens::T;
use crate::compat::source::{space, word};
#[derive(Clone, Copy)]
enum S {
    Initial,
    String,
    MdString,
    Node,
    BlockArrow,
    ArrowDir,
    LLabel,
    ClassDef,
    ClassDefId,
    Class,
    ClassStyle,
    Style,
    StyleDefinition,
    AccTitle,
    AccDescr,
    AccDescrMulti,
}
pub(super) struct Token {
    pub kind: T,
    pub text: String,
    pub start: usize,
}
struct Lexer<'a> {
    src: &'a str,
    p: usize,
    state: S,
    stack: Vec<S>,
    out: Vec<Token>,
}
impl Lexer<'_> {
    fn at(&self, p: usize) -> Option<char> {
        self.src.get(p..)?.chars().next()
    }
    fn ws(&self, mut p: usize) -> usize {
        while let Some(c) = self.at(p).filter(|c| space(*c)) {
            p += c.len_utf8();
        }
        p
    }
    fn digits(&self, mut p: usize) -> usize {
        while self.src.as_bytes().get(p).is_some_and(u8::is_ascii_digit) {
            p += 1;
        }
        p
    }
    fn word(&self, mut p: usize) -> usize {
        while let Some(c) = self.at(p).filter(|c| word(*c)) {
            p += c.len_utf8();
        }
        p
    }
    fn starts(&self, p: usize, s: &str) -> bool {
        self.src.get(p..).is_some_and(|v| v.starts_with(s))
    }
    fn keyword(&self, s: &str) -> bool {
        self.starts(self.p, s) && self.at(self.p + s.len()).is_none_or(|c| !word(c))
    }
    fn put(&mut self, kind: T, text: String, end: usize) -> bool {
        self.out.push(Token {
            kind,
            text,
            start: self.p,
        });
        self.p = end;
        true
    }
    fn emit(&mut self, kind: T, end: usize) -> bool {
        self.put(kind, self.src[self.p..end].into(), end)
    }
    fn push(&mut self, s: S) {
        self.stack.push(self.state);
        self.state = s;
    }
    fn pop(&mut self) {
        if let Some(s) = self.stack.pop() {
            self.state = s;
        }
    }
    fn quote(&mut self) -> bool {
        if self.at(self.p + 1) == Some('`') {
            self.push(S::MdString);
            self.p += 2;
        } else {
            self.push(S::String);
            self.p += 1;
        }
        true
    }
    fn dotted(&self, mut q: usize) -> Option<usize> {
        if self.at(q) != Some('.') {
            return None;
        }
        while self.at(q) == Some('.') {
            q += 1;
        }
        if self.at(q) != Some('-') {
            return None;
        }
        Some(
            q + if matches!(self.at(q + 1), Some('x' | 'o' | '>')) {
                2
            } else {
                1
            },
        )
    }
    fn link_end(&self, mut q: usize) -> Option<usize> {
        if matches!(self.at(q), Some('x' | 'o' | '<')) {
            q += 1;
        }
        let line = self.at(q)?;
        if line == '.' {
            return self.dotted(q);
        }
        if !matches!(line, '-' | '=') {
            return None;
        }
        let mut k = q + 1;
        while self.at(k) == Some(line) {
            k += 1;
        }
        if k - q == 1 {
            return if line == '-' { self.dotted(k) } else { None };
        }
        if matches!(self.at(k), Some('x' | 'o' | '>')) {
            return Some(k + 1);
        }
        (k - q >= 3).then_some(k)
    }
    fn link(&mut self) -> bool {
        if let Some(e) = self.link_end(self.p) {
            return self.emit(T::LINK, self.ws(e));
        }
        let q = self.p + usize::from(self.at(self.p) == Some('<'));
        if self.starts(q, "--") || self.starts(q, "-.") || self.starts(q, "==") {
            self.push(S::LLabel);
            return self.emit(T::START_LINK, self.ws(q + 2));
        }
        false
    }
    fn line(&mut self, t: T) -> bool {
        let e = self.src[self.p..]
            .find('\n')
            .map_or(self.src.len(), |i| self.p + i);
        self.pop();
        self.emit(t, e)
    }
    fn id_list(&mut self, t: T, next: S) -> bool {
        let mut k = self.word(self.p);
        if k == self.p {
            return false;
        }
        while self.at(k) == Some(',') {
            let from = self.ws(k + 1);
            let to = self.word(from);
            if to == from {
                break;
            }
            k = to;
        }
        self.state = next;
        self.emit(t, k)
    }
    fn initial(&mut self, c: char) -> bool {
        let p = self.p;
        if space(c) {
            self.p = self.ws(p + c.len_utf8());
            return true;
        }
        match c {
            'b' => {
                if self.starts(p, "block") {
                    if self.keyword("block-beta") {
                        return self.emit(T::KEY, p + 10);
                    }
                    if self.at(p + 5) == Some(':') {
                        return self.emit(T::ID_BLOCK, p + 6);
                    }
                    if self.at(p + 5).is_none_or(|c| !word(c)) {
                        return self.emit(T::KEY, p + 5);
                    }
                }
            }
            'c' => {
                if self.starts(p, "columns") {
                    let q = self.ws(p + 7);
                    if q > p + 7 {
                        if self.starts(q, "auto") && self.at(q + 4).is_none_or(|c| !word(c)) {
                            return self.put(T::COLUMNS, "-1".into(), q + 4);
                        }
                        let e = self.digits(q);
                        if e > q {
                            return self.put(T::COLUMNS, self.src[q..e].into(), e);
                        }
                    }
                } else if self.starts(p, "class") {
                    let def = self.starts(p + 5, "Def");
                    let from = p + if def { 8 } else { 5 };
                    let q = self.ws(from);
                    if q > from {
                        self.push(if def { S::ClassDef } else { S::Class });
                        return self.emit(if def { T::CLASSDEF } else { T::CLASS }, q);
                    }
                }
            }
            '"' => return self.quote(),
            's' => {
                if self.starts(p, "space") {
                    if self.at(p + 5) == Some(':') {
                        let e = self.digits(p + 6);
                        if e > p + 6 {
                            return self.put(T::SPACE_BLOCK, self.src[p + 6..e].into(), e);
                        }
                    }
                    if self.at(p + 5).is_none_or(|c| !word(c)) {
                        return self.put(T::SPACE_BLOCK, "1".into(), p + 5);
                    }
                } else if self.starts(p, "style") {
                    let q = self.ws(p + 5);
                    if q > p + 5 {
                        self.push(S::Style);
                        return self.emit(T::STYLE, q);
                    }
                }
            }
            'd' => {
                if self.keyword("default") {
                    return self.emit(T::DEFAULT, p + 7);
                }
            }
            'l' => {
                if self.keyword("linkStyle") {
                    return self.emit(T::LINKSTYLE, p + 9);
                }
            }
            'i' => {
                if self.keyword("interpolate") {
                    return self.emit(T::INTERPOLATE, p + 11);
                }
            }
            'a' => {
                if self.starts(p, "accTitle") {
                    let q = self.ws(p + 8);
                    if self.at(q) == Some(':') {
                        self.push(S::AccTitle);
                        return self.emit(T::acc_title, self.ws(q + 1));
                    }
                } else if self.starts(p, "accDescr") {
                    let q = self.ws(p + 8);
                    if self.at(q) == Some(':') {
                        self.push(S::AccDescr);
                        return self.emit(T::acc_descr, self.ws(q + 1));
                    }
                    if self.at(q) == Some('{') {
                        self.push(S::AccDescrMulti);
                        self.p = self.ws(q + 1);
                        return true;
                    }
                }
            }
            'e' => {
                if self.keyword("end") {
                    return self.emit(T::end, self.ws(p + 3));
                }
            }
            '-' => {
                if self.at(p + 1) == Some(')') {
                    self.push(S::Node);
                    return self.emit(T::NODE_DSTART, p + 2);
                }
                return self.link();
            }
            '=' => return self.link(),
            '<' => {
                if self.at(p + 1) == Some('[') {
                    self.push(S::BlockArrow);
                    return self.emit(T::BLOCK_ARROW_START, p + 2);
                }
                return self.link();
            }
            '(' => {
                let d = self.at(p + 1);
                let len = if matches!(d, Some('-' | '[')) {
                    2
                } else if d == Some('(') {
                    if self.at(p + 2) == Some('(') {
                        3
                    } else {
                        2
                    }
                } else {
                    1
                };
                self.push(S::Node);
                return self.emit(T::NODE_DSTART, p + len);
            }
            ')' | '{' => {
                self.push(S::Node);
                return self.emit(
                    T::NODE_DSTART,
                    p + if self.at(p + 1) == Some(c) { 2 } else { 1 },
                );
            }
            '>' => {
                self.push(S::Node);
                return self.emit(T::NODE_DSTART, p + 1);
            }
            '[' => {
                let d = self.at(p + 1);
                self.push(S::Node);
                return self.emit(
                    T::NODE_DSTART,
                    p + if matches!(d, Some('[' | '|' | '(' | '\\' | '/')) {
                        2
                    } else {
                        1
                    },
                );
            }
            ':' => {
                let e = self.digits(p + 1);
                if e > p + 1 {
                    return self.put(T::SIZE, self.src[p + 1..e].into(), e);
                }
                return false;
            }
            _ => {}
        }
        let mut q = p;
        while let Some(ch) = self.at(q) {
            if space(ch)
                || matches!(
                    ch,
                    '(' | ')' | '[' | '{' | '}' | '<' | '>' | '-' | ':' | '='
                )
            {
                break;
            }
            q += ch.len_utf8();
        }
        q > p && self.emit(T::NODE_ID, q)
    }
    fn node_end(&mut self, c: char) -> bool {
        let p = self.p;
        let d = self.at(p + 1);
        let len = match c {
            '"' => return self.quote(),
            '(' => {
                if d == Some('-') {
                    2
                } else if d == Some('(') {
                    if self.at(p + 2) == Some('(') {
                        3
                    } else {
                        2
                    }
                } else {
                    1
                }
            }
            ')' => {
                if d == Some(')') {
                    if self.at(p + 2) == Some(')') {
                        3
                    } else {
                        2
                    }
                } else if d == Some(']') {
                    2
                } else {
                    1
                }
            }
            '}' => {
                if d == Some('}') {
                    2
                } else {
                    1
                }
            }
            '-' => {
                if d == Some(')') {
                    2
                } else {
                    0
                }
            }
            ']' => {
                if matches!(d, Some(']' | ')' | '>')) {
                    2
                } else {
                    1
                }
            }
            '\\' | '/' => {
                if d == Some(']') {
                    2
                } else {
                    0
                }
            }
            _ => 0,
        };
        if len == 0 {
            return false;
        }
        self.pop();
        self.emit(T::NODE_DEND, p + len)
    }
    fn arrow_dir(&mut self, c: char) -> bool {
        let p = self.p;
        if c == ')' {
            self.pop();
            self.pop();
            return self.put(T::BLOCK_ARROW_END, "]>".into(), self.ws(p + 1));
        }
        let from = self.ws(if c == ',' { p + 1 } else { p });
        for d in ["right", "left", "x", "y", "up", "down"] {
            if self.starts(from, d) {
                let e = self.ws(from + d.len());
                return self.put(
                    T::DIR,
                    self.src[if c == ',' { from } else { p }..e].into(),
                    e,
                );
            }
        }
        false
    }
}
pub(super) fn tokenize(src: &str) -> Vec<Token> {
    let mut l = Lexer {
        src,
        p: 0,
        state: S::Initial,
        stack: vec![],
        out: vec![],
    };
    loop {
        let Some(c) = l.at(l.p) else {
            let t = match l.state {
                S::Initial => Some(T::EOF),
                S::String => Some(T::STR),
                S::ClassDefId => Some(T::CLASSDEF_STYLEOPTS),
                S::ClassStyle => Some(T::STYLECLASS),
                S::StyleDefinition => Some(T::STYLE_DEFINITION_DATA),
                S::AccTitle => Some(T::acc_title_value),
                S::AccDescr => Some(T::acc_descr_value),
                S::AccDescrMulti => Some(T::acc_descr_multiline_value),
                _ => None,
            };
            if let Some(t) = t {
                l.put(t, String::new(), src.len());
            }
            break;
        };
        let p = l.p;
        let ok = match l.state {
            S::Initial => l.initial(c),
            S::String => {
                if c == '"' {
                    l.pop();
                    l.p += 1;
                } else {
                    let e = src[p..].find('"').map_or(src.len(), |i| p + i);
                    l.emit(T::STR, e);
                }
                true
            }
            S::MdString => {
                if c == '`' {
                    if l.at(p + 1) == Some('"') {
                        l.pop();
                        l.p += 2;
                        true
                    } else {
                        false
                    }
                } else if c == '"' {
                    false
                } else {
                    let e = src[p + c.len_utf8()..]
                        .find(['`', '"'])
                        .map_or(src.len(), |i| p + c.len_utf8() + i);
                    l.emit(T::MD_STR, e)
                }
            }
            S::Node => l.node_end(c),
            S::BlockArrow => {
                if c == '"' {
                    l.quote()
                } else if c == ']' && l.at(p + 1) == Some('>') {
                    let q = l.ws(p + 2);
                    if l.at(q) == Some('(') {
                        l.push(S::ArrowDir);
                        l.p = q + 1;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            S::ArrowDir => l.arrow_dir(c),
            S::LLabel => {
                if c == '"' {
                    if l.at(p + 1) == Some('`') {
                        l.quote()
                    } else {
                        l.emit(T::LINK_LABEL, p + 1);
                        l.push(S::String);
                        true
                    }
                } else if let Some(e) = l.link_end(l.ws(p)) {
                    l.pop();
                    l.emit(T::LINK, l.ws(e))
                } else {
                    false
                }
            }
            S::ClassDef => {
                let mut t = T::DEFAULT_CLASSDEF_ID;
                let mut k = if l.starts(p, "DEFAULT") { p + 7 } else { p };
                let mut q = l.ws(k);
                if k == p || q == k {
                    t = T::CLASSDEF_ID;
                    k = l.word(p);
                    q = l.ws(k);
                }
                if k > p && q > k {
                    l.state = S::ClassDefId;
                    l.emit(t, q)
                } else {
                    false
                }
            }
            S::ClassDefId => l.line(T::CLASSDEF_STYLEOPTS),
            S::Class => l.id_list(T::CLASSENTITY_IDS, S::ClassStyle),
            S::ClassStyle => l.line(T::STYLECLASS),
            S::Style => l.id_list(T::STYLE_ENTITY_IDS, S::StyleDefinition),
            S::StyleDefinition => l.line(T::STYLE_DEFINITION_DATA),
            S::AccTitle => l.line(T::acc_title_value),
            S::AccDescr => l.line(T::acc_descr_value),
            S::AccDescrMulti => {
                if c == '}' {
                    l.pop();
                    l.p += 1;
                } else {
                    let e = src[p..].find('}').map_or(src.len(), |i| p + i);
                    l.emit(T::acc_descr_multiline_value, e);
                }
                true
            }
        };
        if !ok {
            l.out.push(Token {
                kind: T::ERROR,
                text: String::new(),
                start: l.p,
            });
            return l.out;
        }
    }
    l.out.push(Token {
        kind: T::END,
        text: String::new(),
        start: src.len(),
    });
    l.out
}
