//! Direct state-machine translation of Pele agentflow/lexer.ts, 458322a5 (MIT, kepano).
use super::{model::Loc, tokens::T};
use crate::compat::source::{space, word};
use regex::Regex;
use std::sync::OnceLock;
#[derive(Clone, Copy, PartialEq, Eq)]
enum S {
    Initial,
    String,
    MdString,
    AccTitle,
    AccDescr,
    AccMultiline,
    Dir,
    Text,
    Ellipse,
    Trap,
    Edge,
    Click,
    CallbackName,
    CallbackArgs,
    ShapeData,
    ShapeString,
}
#[derive(Debug)]
pub(super) struct Token {
    pub kind: T,
    pub text: String,
    pub start: usize,
    pub loc: Loc,
}
const PATTERNS: &[&str] = &[
    r"accTitle\s*:\s*",
    r"accDescr\s*:\s*",
    r"accDescr\s*\{\s*",
    r"call\s+",
    r"click\s+",
    r"(?:\r?\n)*\s*\n",
    r"\s*(?:LR|RL|TB|BT|TD|BR)\b",
    r"\s*[<>^]",
    r"\s*v\b",
    r".*direction\s+TB[^\n]*",
    r".*direction\s+BT[^\n]*",
    r".*direction\s+RL[^\n]*",
    r".*direction\s+LR[^\n]*",
    r".*direction\s+TD[^\n]*",
    r"\s*[xo<]?--+[-xo>]\s*",
    r"\s*[xo<]?--\s*",
    r"\s*[xo<]?==+[=xo>]\s*",
    r"\s*[xo<]?==\s*",
    r"\s*[xo<]?-?\.+-[xo>]?\s*",
    r"\s*[xo<]?-\.\s*",
    r"\s*~~~+\s*",
    r"(?:\r?\n)+",
    r"[^\S\n\r]+",
    r"\(\s*\)",
];
fn patterns() -> &'static Vec<Regex> {
    static P: OnceLock<Vec<Regex>> = OnceLock::new();
    P.get_or_init(|| {
        PATTERNS.iter().map(|pattern| {
            // ECMAScript whitespace includes BOM and excludes NEL. Its word boundary
            // and dot also differ from Rust regex's default Unicode behavior.
            let pattern = if *pattern == r"[^\S\n\r]+" {
                r"[\t\x0b\x0c \u{a0}\u{1680}\u{2000}-\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}\u{feff}]+".to_string()
            } else {
                pattern.replace(r"\s", crate::compat::text::SPACE)
            };
            let pattern = pattern.replace(r"\b", r"(?-u:\b)")
                .replace(".*direction", r"[^\n\r\u{2028}\u{2029}]*direction");
            Regex::new(&format!("^(?:{pattern})")).unwrap()
        }).collect()
    })
}

struct Lexer<'a> {
    src: &'a str,
    p: usize,
    state: S,
    stack: Vec<S>,
    tokens: Vec<Token>,
    first_graph: bool,
    dir_failed: usize,
    dir_next: Option<usize>,
    line_end: Option<usize>,
    run_end: usize,
    run_at: Option<usize>,
    ws_from: usize,
    ws_to: usize,
    blank_from: usize,
    blank_to: usize,
}
impl Lexer<'_> {
    fn c(&self, p: usize) -> u32 {
        self.src
            .get(p..)
            .and_then(|s| s.chars().next())
            .map_or(u32::MAX, |c| c as u32)
    }
    fn next(&self, p: usize) -> usize {
        p + self.src[p..].chars().next().map_or(0, char::len_utf8)
    }
    fn at(&self, re: usize, p: usize) -> Option<usize> {
        patterns()[re].find(&self.src[p..]).map(|m| p + m.end())
    }
    fn emit(&mut self, kind: T, start: usize, end: usize) {
        self.tokens.push(Token {
            kind,
            text: self.src[start..end].into(),
            start,
            loc: Loc::default(),
        });
        self.p = end
    }
    fn push(&mut self, state: S) {
        self.stack.push(self.state);
        self.state = state
    }
    fn pop(&mut self) {
        if let Some(s) = self.stack.pop() {
            self.state = s
        }
    }
    fn kw(&self, w: &str) -> bool {
        self.src[self.p..].starts_with(w)
            && self.src[self.p + w.len()..]
                .chars()
                .next()
                .is_none_or(|c| !word(c))
    }
    fn skip_space(&mut self, mut p: usize) -> usize {
        if p >= self.ws_from && p <= self.ws_to {
            return self.ws_to;
        }
        let from = p;
        while self.src[p..].chars().next().is_some_and(space) {
            p = self.next(p)
        }
        self.ws_from = from;
        self.ws_to = p;
        p
    }
    fn comment_end(&self, p: usize) -> Option<usize> {
        if !self.src[p..].starts_with("%%") || self.src[p..].starts_with("%%{") {
            return None;
        }
        Some(self.src[p..].find('\n').map_or(self.src.len(), |i| p + i))
    }
    fn blank_end(&mut self, mut p: usize) -> usize {
        if p >= self.blank_from && p <= self.blank_to {
            return self.blank_to;
        }
        let from = p;
        while matches!(self.c(p), 32 | 9) {
            p += 1
        }
        self.blank_from = from;
        self.blank_to = p;
        p
    }
    fn arrow_end(&mut self, p: usize) -> Option<usize> {
        let p = self.skip_space(p);
        if !self.src[p..].starts_with("--") {
            return None;
        }
        let mut q = p + 2;
        while self.c(q) == 45 {
            q += 1
        }
        if matches!(self.c(q), 120 | 62) {
            Some(self.skip_space(q + 1))
        } else {
            None
        }
    }
    fn quote(&mut self) {
        if self.c(self.p + 1) == 96 {
            self.push(S::MdString);
            self.p += 2
        } else {
            self.push(S::String);
            self.p += 1
        }
    }
    fn opener(&mut self, c: u32) -> bool {
        let d = self.c(self.p + 1);
        let (kind, len, state) = match c {
            40 => match d {
                45 => (T::ELLIPSE_START, 2, S::Ellipse),
                91 => (T::STADIUMSTART, 2, S::Text),
                40 if self.c(self.p + 2) == 40 => (T::DOUBLECIRCLESTART, 3, S::Text),
                _ => (T::PS, 1, S::Text),
            },
            91 => match d {
                91 => (T::SUBROUTINESTART, 2, S::Text),
                40 => (T::CYLINDERSTART, 2, S::Text),
                47 => (T::TRAPSTART, 2, S::Trap),
                92 => (T::INVTRAPSTART, 2, S::Trap),
                _ => (T::SQS, 1, S::Text),
            },
            124 => (T::PIPE, 1, S::Text),
            123 => (T::DIAMOND_START, 1, S::Text),
            _ => return false,
        };
        self.push(state);
        self.emit(kind, self.p, self.p + len);
        true
    }
    fn graph(&mut self, len: usize) {
        if self.first_graph {
            self.first_graph = false;
            self.push(S::Dir)
        }
        self.emit(T::GRAPH, self.p, self.p + len)
    }
    fn node_string(&mut self) -> bool {
        let mut q = self.p;
        while q < self.src.len() {
            let c = self.c(q);
            if c<128&&("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!\"#$%&'*+.`?\\_/".contains(char::from_u32(c).unwrap())){q+=1}else if c==45&&q+1<self.src.len(){if matches!(self.c(q+1),62|45|46){break}q+=1}else{break}
        }
        if q == self.p {
            return false;
        }
        self.emit(T::NODE_STRING, self.p, q);
        true
    }
    fn link(&mut self, _c: u32) -> bool {
        let j = self.skip_space(self.p);
        if self.c(j) != 45 {
            return false;
        }
        let d = self.c(j + 1);
        let mut k = j + 2;
        if d == 45 {
            while self.c(k) == 45 {
                k += 1
            }
            if matches!(self.c(k), 120 | 62) {
                let end = self.skip_space(k + 1);
                self.emit(T::LINK, self.p, end);
            } else {
                self.push(S::Edge);
                let end = self.skip_space(j + 2);
                self.emit(T::START_LINK, self.p, end);
            }
            return true;
        }
        if d == 46 {
            while self.c(k) == 46 {
                k += 1
            }
            if self.c(k) == 45 {
                let end = self.skip_space(k + 1);
                self.emit(T::LINK, self.p, end);
                return true;
            }
        }
        false
    }
    fn initial(&mut self, c: u32) -> bool {
        let p = self.p;
        match c {
            97 => {
                for (re, state, kind) in [
                    (0, S::AccTitle, Some(T::acc_title)),
                    (1, S::AccDescr, Some(T::acc_descr)),
                    (2, S::AccMultiline, None),
                ] {
                    if let Some(e) = self.at(re, p) {
                        self.push(state);
                        if let Some(kind) = kind {
                            self.emit(kind, p, e)
                        } else {
                            self.p = e
                        }
                        return true;
                    }
                }
                if self.kw("agentflow-beta") {
                    self.graph(14);
                    return true;
                }
            }
            64 if self.c(p + 1) == 123 => {
                self.push(S::ShapeData);
                self.tokens.push(Token {
                    kind: T::SHAPE_DATA,
                    text: String::new(),
                    start: p,
                    loc: Loc::default(),
                });
                self.p += 2;
                return true;
            }
            99 => {
                if let Some(e) = self.at(3, p) {
                    self.push(S::CallbackName);
                    self.p = e;
                    return true;
                }
                for (w, kind) in [("classDef", T::CLASSDEF), ("class", T::CLASS)] {
                    if self.kw(w) {
                        self.emit(kind, p, p + w.len());
                        return true;
                    }
                }
                if let Some(e) = self.at(4, p) {
                    self.push(S::Click);
                    self.p = e;
                    return true;
                }
                if self.kw("connector") {
                    self.emit(T::connector, p, p + 9);
                    return true;
                }
            }
            34 => {
                self.quote();
                return true;
            }
            115 => {
                for (w, kind) in [("style", T::STYLE)] {
                    if self.kw(w) {
                        self.emit(kind, p, p + w.len());
                        return true;
                    }
                }
            }
            100 if self.kw("default") => {
                self.emit(T::DEFAULT, p, p + 7);
                return true;
            }
            108 if self.kw("linkStyle") => {
                self.emit(T::LINKSTYLE, p, p + 9);
                return true;
            }
            105 if self.kw("interpolate") => {
                self.emit(T::INTERPOLATE, p, p + 11);
                return true;
            }
            104 if self.src[p..].starts_with("href")
                && self.src[p + 4..].chars().next().is_some_and(space) =>
            {
                self.emit(T::HREF, p, self.next(p + 4));
                return true;
            }
            102 if self.kw("flow") => {
                self.emit(T::flow, p, p + 4);
                return true;
            }
            103 if self.kw("global") => {
                let end = self.blank_end(p + 6);
                self.emit(T::global, p, end);
                return true;
            }
            101 if self.kw("end") => {
                let end = self.blank_end(p + 3);
                self.emit(T::end, p, end);
                return true;
            }
            95 => {
                for w in ["_self", "_blank", "_parent", "_top"] {
                    if self.kw(w) {
                        self.emit(T::LINK_TARGET, p, p + w.len());
                        return true;
                    }
                }
            }
            _ => {}
        }
        if self.dir_next.is_some() && p >= self.dir_failed {
            if self.dir_next.is_some_and(|next| next < p) {
                self.dir_next = self.src[p..].find("direction").map(|i| p + i);
            }
            if let Some(next) = self.dir_next {
                if self.line_end.is_none_or(|end| end < p) {
                    self.line_end = Some(
                        self.src[p..]
                            .find(['\n', '\r', '\u{2028}', '\u{2029}'])
                            .map_or(self.src.len(), |i| p + i),
                    );
                }
                let line_end = self.line_end.unwrap();
                if next < line_end {
                    for (re, kind) in [
                        (9, T::direction_tb),
                        (10, T::direction_bt),
                        (11, T::direction_rl),
                        (12, T::direction_lr),
                        (13, T::direction_td),
                    ] {
                        if let Some(end) = self.at(re, p) {
                            self.emit(kind, p, end);
                            return true;
                        }
                    }
                    self.dir_failed = line_end;
                }
            }
        }

        if c != 34 && !char::from_u32(c).is_some_and(space) {
            if p >= self.run_end {
                let mut q = p;
                while q < self.src.len()
                    && self.c(q) != 34
                    && !char::from_u32(self.c(q)).is_some_and(space)
                {
                    q = self.next(q)
                }
                self.run_end = q;
                self.run_at = self.src[p..q].char_indices().rev().find_map(|(i, ch)| {
                    let k = p + i;
                    (ch == '@' && !matches!(self.c(k + 1), 123 | 34)).then_some(k)
                });
            }
            if let Some(at) = self.run_at.filter(|at| *at > p) {
                self.emit(T::LINK_ID, p, at + 1);
                return true;
            }
        }
        if (48..=57).contains(&c) {
            let mut q = p + 1;
            while (48..=57).contains(&self.c(q)) {
                q += 1
            }
            self.emit(T::NUM, p, q);
            return true;
        }
        match c {
            35 => self.emit(T::BRKT, p, p + 1),
            58 => self.emit(
                if self.src[p..].starts_with(":::") {
                    T::STYLE_SEPARATOR
                } else {
                    T::COLON
                },
                p,
                p + if self.src[p..].starts_with(":::") {
                    3
                } else {
                    1
                },
            ),
            38 => self.emit(T::AMP, p, p + 1),
            59 => self.emit(T::SEMI, p, p + 1),
            44 => self.emit(T::COMMA, p, p + 1),
            42 => self.emit(T::MULT, p, p + 1),
            60 => self.emit(T::TAGSTART, p, p + 1),
            45 => {
                if !self.link(c) && !self.node_string() {
                    self.emit(T::MINUS, p, p + 1)
                }
            }
            40 | 124 | 123 => return self.opener(c),
            91 => {
                if self.c(p + 1) == 124 {
                    self.emit(T::VERTEX_WITH_PROPS_START, p, p + 2)
                } else {
                    return self.opener(c);
                }
            }
            62 => {
                self.push(S::Text);
                self.emit(T::TAGEND, p, p + 1)
            }
            94 => self.emit(T::UP, p, p + 1),
            92 => {
                if self.c(p + 1) == 124 {
                    self.emit(T::SEP, p, p + 2)
                } else {
                    return self.node_string();
                }
            }
            37 => {
                if let Some(e) = self.comment_end(p) {
                    self.emit(T::COMMENT, p, e)
                } else {
                    return self.node_string();
                }
            }
            118 => {
                if self.src[p + 1..].chars().next().is_none_or(|c| !word(c)) {
                    self.emit(T::DOWN, p, p + 1)
                } else {
                    return self.node_string();
                }
            }
            _ => {
                if char::from_u32(c).is_some_and(space) {
                    if self.link(c) {
                        return true;
                    }
                    if matches!(c, 32 | 9) {
                        let q = self.blank_end(p);
                        if let Some(e) = self.comment_end(q) {
                            self.emit(T::COMMENT, p, e);
                            return true;
                        }
                    }
                    if let Some(e) = self.at(21, p) {
                        self.emit(T::NEWLINE, p, e)
                    } else {
                        self.emit(T::SPACE, p, self.next(p))
                    }
                } else if c < 128 {
                    return self.node_string();
                } else if crate::compat::source::unicode::letter(c) {
                    self.emit(T::UNICODE_TEXT, p, self.next(p))
                } else {
                    return false;
                }
            }
        }
        true
    }
    fn edge_text(&mut self, c: u32) -> bool {
        let p = self.p;
        if c == 34 {
            self.quote();
            return true;
        }
        if let Some(e) = self.arrow_end(p) {
            self.pop();
            self.emit(T::LINK, p, e);
            return true;
        }
        if c == 45 && self.c(p + 1) == 45 {
            return false;
        }
        let mut q = self.next(p);
        while q < self.src.len() {
            let c = self.c(q);
            if c == 34
                || (c == 45 && self.c(q + 1) == 45)
                || (char::from_u32(c).is_some_and(space) && self.arrow_end(q).is_some())
            {
                break;
            }
            q = self.next(q);
        }
        self.emit(T::EDGE_TEXT, p, q);
        true
    }
    fn text(&mut self, c: u32) -> bool {
        let p = self.p;
        let d = self.c(p + 1);
        match c {
            34 => self.quote(),
            40 | 91 | 123 => {
                self.opener(c);
            }
            93 => {
                self.pop();
                let (kind, len) = match d {
                    41 => (T::STADIUMEND, 2),
                    93 => (T::SUBROUTINEEND, 2),
                    _ => (T::SQE, 1),
                };
                self.emit(kind, p, p + len)
            }
            41 => {
                self.pop();
                let (kind, len) = if d == 93 {
                    (T::CYLINDEREND, 2)
                } else if d == 41 && self.c(p + 2) == 41 {
                    (T::DOUBLECIRCLEEND, 3)
                } else {
                    (T::PE, 1)
                };
                self.emit(kind, p, p + len)
            }
            124 => {
                self.pop();
                self.emit(T::PIPE, p, p + 1)
            }
            125 => {
                self.pop();
                self.emit(T::DIAMOND_STOP, p, p + 1)
            }
            37 => {
                if let Some(e) = self.comment_end(p) {
                    self.p = e
                } else {
                    return false;
                }
            }
            _ => {
                let mut q = self.next(p);
                while q < self.src.len()
                    && !matches!(self.c(q), 91 | 93 | 40 | 41 | 123 | 125 | 124 | 34 | 37)
                {
                    q = self.next(q)
                }
                self.emit(T::TEXT, p, q)
            }
        }
        true
    }
    fn ellipse(&mut self, c: u32) -> bool {
        let p = self.p;
        let d = self.c(p + 1);
        if c == 34 {
            self.quote()
        } else if c == 37 && self.comment_end(p).is_some() {
            self.p = self.comment_end(p).unwrap();
        } else if matches!(c, 45 | 47 | 41) && d == 41 {
            self.pop();
            self.emit(T::ELLIPSE_END, p, p + 2)
        } else if matches!(c, 40 | 91 | 123) {
            self.opener(c);
        } else if matches!(c, 41 | 93 | 125) {
            return false;
        } else {
            let mut q = self.next(p);
            while q < self.src.len() {
                let ch = self.c(q);
                if matches!(ch, 34 | 40 | 41 | 91 | 93 | 123 | 125) {
                    break;
                }
                if matches!(ch, 45 | 47) && self.c(q + 1) == 41 {
                    break;
                }
                if ch == 37 && self.comment_end(q).is_some() {
                    break;
                }
                q = self.next(q)
            }
            self.emit(T::TEXT, p, q)
        }
        true
    }
    fn trap(&mut self, c: u32) -> bool {
        let p = self.p;
        let d = self.c(p + 1);
        if c == 34 {
            self.quote();
            return true;
        }
        if c == 37 {
            if let Some(e) = self.comment_end(p) {
                self.p = e;
                return true;
            } else {
                return false;
            }
        }
        match c {
            40 => {
                if d == 93 {
                    self.pop();
                    self.emit(T::TRAPEND, p, p + 2)
                } else {
                    self.opener(c);
                }
            }
            91 | 123 => {
                self.opener(c);
            }
            92 | 47 => {
                if d == 93 {
                    self.pop();
                    self.emit(if c == 92 { T::TRAPEND } else { T::INVTRAPEND }, p, p + 2)
                } else {
                    self.emit(T::TEXT, p, p + 1)
                }
            }
            93 | 41 => {
                if d == 93 {
                    self.pop();
                    self.emit(T::TRAPEND, p, p + 2)
                } else {
                    return false;
                }
            }
            125 => return false,
            _ => {
                if matches!(c, 63 | 61) && d == 93 {
                    self.pop();
                    self.emit(T::TRAPEND, p, p + 2);
                    return true;
                }
                let mut q = self.next(p);
                while q < self.src.len()
                    && !matches!(self.c(q), 92 | 91 | 93 | 40 | 41 | 123 | 125 | 47 | 37)
                {
                    q = self.next(q)
                }
                self.emit(T::TEXT, p, q)
            }
        }
        true
    }
    fn step(&mut self) -> bool {
        let p = self.p;
        let c = self.c(p);
        match self.state {
            S::Initial => return self.initial(c),
            S::Text => return self.text(c),
            S::Ellipse => return self.ellipse(c),
            S::Trap => return self.trap(c),
            S::Edge => return self.edge_text(c),
            S::String => {
                if c == 34 {
                    if self.c(p + 1) == 96 {
                        self.push(S::MdString);
                        self.p += 2
                    } else {
                        self.pop();
                        self.p += 1
                    }
                } else {
                    let e = self.src[p..].find('"').map_or(self.src.len(), |i| p + i);
                    self.emit(T::STR, p, e)
                }
            }
            S::MdString => {
                if c == 96 {
                    if self.c(p + 1) == 34 {
                        self.pop();
                        self.p += 2
                    } else {
                        return false;
                    }
                } else if c == 34 {
                    self.quote()
                } else {
                    let e = self.src[p..]
                        .find(['`', '"'])
                        .map_or(self.src.len(), |i| p + i);
                    self.emit(T::MD_STR, p, e)
                }
            }
            S::AccTitle | S::AccDescr => {
                let e = self.src[p..].find('\n').map_or(self.src.len(), |i| p + i);
                let t = if self.state == S::AccTitle {
                    T::acc_title_value
                } else {
                    T::acc_descr_value
                };
                self.pop();
                self.emit(t, p, e)
            }
            S::AccMultiline => {
                if c == 125 {
                    self.pop();
                    self.p += 1
                } else {
                    let e = self.src[p..].find('}').map_or(self.src.len(), |i| p + i);
                    self.emit(T::acc_descr_multiline_value, p, e)
                }
            }
            S::Dir => {
                if c == 34 {
                    self.quote()
                } else if let Some(e) = self.at(5, p) {
                    self.pop();
                    self.emit(T::NODIR, p, e)
                } else if let Some(e) = self
                    .at(6, p)
                    .or_else(|| self.at(7, p))
                    .or_else(|| self.at(8, p))
                {
                    self.pop();
                    self.emit(T::DIR, p, e)
                } else {
                    return self.opener(c);
                }
            }
            S::Click => {
                if c == 34 {
                    self.quote()
                } else if char::from_u32(c).is_some_and(space) {
                    self.pop();
                    self.p = self.next(p)
                } else {
                    let mut q = self.next(p);
                    while q < self.src.len() && !char::from_u32(self.c(q)).is_some_and(space) {
                        q = self.next(q)
                    }
                    self.emit(T::CLICK, p, q)
                }
            }
            S::CallbackName => {
                if c == 40 {
                    let e = self.at(23, p);
                    self.pop();
                    if let Some(e) = e {
                        self.p = e
                    } else {
                        self.push(S::CallbackArgs);
                        self.p += 1
                    }
                } else {
                    let e = self.src[p..].find('(').map_or(self.src.len(), |i| p + i);
                    self.emit(T::CALLBACKNAME, p, e)
                }
            }
            S::CallbackArgs => {
                if c == 41 {
                    self.pop();
                    self.p += 1
                } else {
                    let e = self.src[p..].find(')').map_or(self.src.len(), |i| p + i);
                    self.emit(T::CALLBACKARGS, p, e)
                }
            }
            S::ShapeData => {
                if c == 34 {
                    self.push(S::ShapeString);
                    self.emit(T::SHAPE_DATA, p, p + 1)
                } else if c == 125 {
                    self.pop();
                    self.p += 1
                } else if c == 94 {
                    return false;
                } else {
                    let e = self.src[p..]
                        .find(['}', '^', '"'])
                        .map_or(self.src.len(), |i| p + i);
                    self.emit(T::SHAPE_DATA, p, e)
                }
            }
            S::ShapeString => {
                if c == 34 {
                    self.pop();
                    self.emit(T::SHAPE_DATA, p, p + 1)
                } else {
                    let e = self.src[p..].find('"').map_or(self.src.len(), |i| p + i);
                    static BR: OnceLock<Regex> = OnceLock::new();
                    let text = BR
                        .get_or_init(|| {
                            Regex::new(&format!(r"\n{}*", crate::compat::text::SPACE)).unwrap()
                        })
                        .replace_all(&self.src[p..e], "<br/>")
                        .into_owned();
                    self.tokens.push(Token {
                        kind: T::SHAPE_DATA,
                        text,
                        start: p,
                        loc: Loc::default(),
                    });
                    self.p = e
                }
            }
        }
        true
    }
}
pub(super) fn tokenize(src: &str) -> Vec<Token> {
    let mut l = Lexer {
        src,
        p: 0,
        state: S::Initial,
        stack: Vec::new(),
        tokens: Vec::new(),
        first_graph: true,
        dir_failed: 0,
        dir_next: src.find("direction"),
        line_end: None,
        run_end: 0,
        run_at: None,
        ws_from: usize::MAX,
        ws_to: 0,
        blank_from: usize::MAX,
        blank_to: 0,
    };
    let (mut line, mut column) = (1, 0);
    while l.p < src.len() {
        let start = l.p;
        let state = l.state;
        let before = l.tokens.len();
        let first_line = line;
        let first_column = column;
        let ok = l.step();
        let chars = matches!(state, S::Edge | S::Ellipse)
            && l.tokens
                .last()
                .is_some_and(|t| matches!(t.kind, T::TEXT | T::EDGE_TEXT))
            && l.tokens.len() > before;
        let segment = &src[start..l.p];
        if chars {
            for c in segment.chars() {
                if matches!(c, '\r' | '\n') {
                    line += 1;
                    column = 0
                } else {
                    column += c.len_utf16()
                }
            }
        } else {
            let mut last = None;
            let mut iter = segment.char_indices().peekable();
            while let Some((i, c)) = iter.next() {
                if c == '\n' {
                    line += 1;
                    last = Some(i + 1)
                } else if c == '\r' {
                    let mut end = i + 1;
                    if iter.peek().is_some_and(|(_, c)| *c == '\n') {
                        let (j, _) = iter.next().unwrap();
                        end = j + 1;
                    }
                    line += 1;
                    last = Some(end);
                }
            }
            if let Some(last) = last {
                column = segment[last..]
                    .split(['\u{2028}', '\u{2029}'])
                    .next()
                    .unwrap()
                    .encode_utf16()
                    .count()
            } else {
                column += segment.encode_utf16().count();
            }
        }
        if l.tokens.len() > before {
            for token in &mut l.tokens[before..] {
                token.loc = Loc {
                    first_line,
                    first_column,
                    last_line: line,
                    last_column: column,
                };
            }
        }
        if !ok {
            l.tokens.push(Token {
                kind: T::ERROR,
                text: String::new(),
                start: l.p,
                loc: Loc {
                    first_line: line,
                    first_column: column,
                    last_line: line,
                    last_column: column,
                },
            });
            return l.tokens;
        }
    }
    let kind = match l.state {
        S::Initial => Some(T::EOF),
        S::AccTitle => Some(T::acc_title_value),
        S::AccDescr => Some(T::acc_descr_value),
        S::AccMultiline => Some(T::acc_descr_multiline_value),
        S::Click => Some(T::CLICK),
        S::CallbackName => Some(T::CALLBACKNAME),
        S::CallbackArgs => Some(T::CALLBACKARGS),
        _ => None,
    };
    for t in kind.into_iter().chain(Some(T::END)) {
        l.tokens.push(Token {
            kind: t,
            text: String::new(),
            start: src.len(),
            loc: Loc {
                first_line: line,
                first_column: column,
                last_line: line,
                last_column: column,
            },
        });
    }
    l.tokens
}
