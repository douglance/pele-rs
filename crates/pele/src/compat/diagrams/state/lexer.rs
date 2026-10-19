//! Pele state/lexer.ts scanner, 458322a5 (MIT, kepano).
use super::tokens::T;
use crate::compat::source::{space, word};
use regex::Regex;
use std::sync::OnceLock;
#[derive(Clone, Copy, PartialEq, Eq)]
enum S {
    Initial,
    Struct,
    State,
    StateString,
    StateId,
    Scale,
    AccTitle,
    AccDescr,
    AccMultiline,
    ClassDef,
    ClassDefId,
    Class,
    ClassStyle,
    Style,
    StyleDefStyles,
    Note,
    NoteId,
    NoteText,
    FloatingNote,
    FloatingNoteId,
}
#[derive(Debug)]
pub(super) struct Token {
    pub kind: T,
    pub text: String,
    pub start: usize,
}
pub(super) struct Tokens {
    pub list: Vec<Token>,
    pub error: Option<String>,
}
struct Lexer<'a> {
    src: &'a str,
    p: usize,
    state: S,
    stack: Vec<S>,
    list: Vec<Token>,
    error: Option<String>,
    directions: Vec<usize>,
    stereotypes: Vec<usize>,
    line_ends: Vec<usize>,
    percent: Vec<usize>,
    braces: Vec<usize>,
    dir_range: Option<(usize, usize)>,
    dir_at: [Option<usize>; 4],
    dir_end: [usize; 4],
    stereo_range: Option<(usize, usize)>,
    stereo_at: [Option<usize>; 6],
}
fn next_match(list: &[usize], q: usize) -> Option<usize> {
    let i = list.partition_point(|p| *p < q);
    list.get(i).copied()
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
    fn emit(&mut self, kind: T, s: usize, e: usize) {
        self.emit_text(kind, self.src[s..e].into(), s, e)
    }
    fn emit_text(&mut self, kind: T, text: String, start: usize, end: usize) {
        self.list.push(Token { kind, text, start });
        self.p = end
    }
    fn push(&mut self, s: S) {
        self.stack.push(self.state);
        self.state = s
    }
    fn pop(&mut self) {
        if let Some(s) = self.stack.pop() {
            self.state = s
        }
    }
    fn keyword(&self, w: &str, p: usize) -> bool {
        self.src
            .get(p..p + w.len())
            .is_some_and(|s| s.eq_ignore_ascii_case(w))
    }
    fn ws(&self, mut p: usize) -> usize {
        while char::from_u32(self.c(p)).is_some_and(space) {
            p = self.next(p)
        }
        p
    }
    fn word_end(&self, mut p: usize) -> usize {
        while char::from_u32(self.c(p)).is_some_and(word) {
            p = self.next(p)
        }
        p
    }
    fn eol(&self, p: usize) -> usize {
        self.src[p..].find('\n').map_or(self.src.len(), |i| p + i)
    }
    fn line_end(&self, p: usize) -> usize {
        next_match(&self.line_ends, p).unwrap_or(self.src.len())
    }
    fn direction(&mut self, q: usize) -> Option<usize> {
        if self.dir_range.is_none_or(|(from, to)| q < from || q > to) {
            let first = next_match(&self.directions, q);
            let end = if first.is_none() {
                self.src.len()
            } else {
                self.line_end(q)
            };
            self.dir_range = Some((q, end));
            self.dir_at = [None; 4];
            let mut at = first;
            while let Some(d) = at.filter(|d| *d < end) {
                let w = self.ws(d + 9);
                if w > d + 9 {
                    let kind = if self.keyword("tb", w) {
                        Some(0)
                    } else if self.keyword("bt", w) {
                        Some(1)
                    } else if self.keyword("rl", w) {
                        Some(2)
                    } else if self.keyword("lr", w) {
                        Some(3)
                    } else {
                        None
                    };
                    if let Some(k) = kind {
                        self.dir_at[k] = Some(d);
                        self.dir_end[k] = w + 2
                    }
                }
                at = next_match(&self.directions, d + 1)
            }
        }
        (0..4).find(|i| self.dir_at[*i].is_some_and(|p| p >= q))
    }
    fn stereotype(&mut self, q: usize) -> Option<usize> {
        if self
            .stereo_range
            .is_none_or(|(from, to)| q < from || q > to)
        {
            let first = next_match(&self.stereotypes, q);
            let end = if first.is_none() {
                self.src.len()
            } else {
                self.line_end(q)
            };
            self.stereo_range = Some((q, end));
            self.stereo_at = [None; 6];
            let mut at = first;
            while let Some(d) = at.filter(|d| *d < end) {
                let letter = self.c(d + 2) | 32;
                let kind = if self.c(d) == 60 { 0 } else { 3 }
                    + if letter == 102 {
                        0
                    } else if letter == 106 {
                        1
                    } else {
                        2
                    };
                self.stereo_at[kind] = Some(d);
                at = next_match(&self.stereotypes, d + 1)
            }
        }
        (0..6).find(|i| self.stereo_at[*i].is_some_and(|p| p >= q))
    }
    fn emit_id(&mut self, s: usize, e: usize) -> bool {
        if let Some(at) = next_match(&self.percent, s).filter(|at| *at + 2 <= e) {
            if at == s {
                self.p = e;
                false
            } else {
                self.emit(T::ID, s, at);
                true
            }
        } else {
            self.emit(T::ID, s, e);
            true
        }
    }
    fn finish(mut self, t: T) -> Tokens {
        self.list.push(Token {
            kind: t,
            text: String::new(),
            start: self.p,
        });
        Tokens {
            list: self.list,
            error: self.error,
        }
    }
    fn initial(&mut self) -> bool {
        let p = self.p;
        let c = self.c(p);
        let root = self.state == S::Initial;
        let lc = c | 32;
        if root {
            for (w, t) in [
                ("click", T::CLICK),
                ("href", T::HREF),
                ("default", T::DEFAULT),
            ] {
                if self.keyword(w, p) && !char::from_u32(self.c(p + w.len())).is_some_and(word) {
                    self.emit(t, p, p + w.len());
                    return true;
                }
            }
            if c == 34 {
                if let Some(close) = self.src[p + 1..].find('"') {
                    self.emit(T::STRING, p, p + close + 2);
                    return true;
                }
            }
            if let Some(kind) = self.direction(p) {
                self.emit(
                    [
                        T::direction_tb,
                        T::direction_bt,
                        T::direction_rl,
                        T::direction_lr,
                    ][kind],
                    p,
                    self.eol(self.dir_end[kind]),
                );
                return true;
            }
            if c == 10 {
                let mut j = p + 1;
                while self.c(j) == 10 {
                    j += 1
                }
                self.emit(T::NL, p, j);
                return true;
            }
            if char::from_u32(c).is_some_and(space) {
                self.p = self.ws(p);
                return true;
            }
        } else if c != 10 && char::from_u32(c).is_some_and(space) {
            let mut j = self.next(p);
            while j < self.src.len()
                && self.c(j) != 10
                && char::from_u32(self.c(j)).is_some_and(space)
            {
                j = self.next(j)
            }
            self.p = j;
            return true;
        }
        if c == 35 || (c == 37 && self.c(p + 1) == 37 && self.c(p + 2) != 123) {
            self.p = self.eol(p);
            return true;
        }
        if lc == 115 {
            if root && self.keyword("scale", p) {
                let e = self.ws(p + 5);
                if e > p + 5 {
                    self.emit(T::scale, p, e);
                    self.push(S::Scale);
                    return true;
                }
            }
            if self.keyword("style", p) {
                let e = self.ws(p + 5);
                if e > p + 5 {
                    self.emit(T::style, p, e);
                    self.push(S::Style);
                    return true;
                }
            }
            if self.keyword("state", p) {
                let mut e = self.ws(p + 5);
                if e > p + 5 {
                    self.p = e;
                    self.push(S::State);
                    return true;
                }
                if root && self.keyword("diagram", p + 5) {
                    let from = if self.keyword("-v2", p + 12) {
                        p + 15
                    } else {
                        p + 12
                    };
                    e = self.ws(from);
                    if e > from {
                        self.emit(T::SD, p, e);
                        return true;
                    }
                }
            }
        } else if lc == 99 && self.keyword("class", p) {
            let def = self.keyword("def", p + 5);
            let from = if def { p + 8 } else { p + 5 };
            let e = self.ws(from);
            if e > from {
                self.emit(if def { T::classDef } else { T::class }, p, e);
                self.push(if def { S::ClassDef } else { S::Class });
                return true;
            }
        } else if lc == 97 && root && (self.keyword("acctitle", p) || self.keyword("accdescr", p)) {
            let title = self.c(p + 3) | 32 == 116;
            let q = self.ws(p + 8);
            let d = self.c(q);
            if d == 58 {
                self.emit(
                    if title { T::acc_title } else { T::acc_descr },
                    p,
                    self.ws(q + 1),
                );
                self.push(if title { S::AccTitle } else { S::AccDescr });
                return true;
            }
            if d == 123 && !title {
                self.p = self.ws(q + 1);
                self.push(S::AccMultiline);
                return true;
            }
        }
        if root {
            if c == 123 {
                self.emit(T::STRUCT_START, p, p + 1);
                self.push(S::Struct);
                return true;
            }
        } else {
            if let Some(kind) = self.direction(p) {
                self.emit(
                    [
                        T::direction_tb,
                        T::direction_bt,
                        T::direction_rl,
                        T::direction_lr,
                    ][kind],
                    p,
                    self.eol(self.dir_end[kind]),
                );
                return true;
            }
            if c == 125 {
                self.emit(T::STRUCT_STOP, p, p + 1);
                self.pop();
                return true;
            }
            if c == 10 {
                self.p += 1;
                return true;
            }
        }
        if lc == 110 && self.keyword("note", p) {
            let e = self.ws(p + 4);
            if e > p + 4 {
                self.emit(T::note, p, e);
                self.push(S::Note);
                return true;
            }
        }
        if root
            && lc == 104
            && self.keyword("hide empty description", p)
            && !char::from_u32(self.c(p + 22)).is_some_and(word)
        {
            self.emit(T::HIDE_EMPTY, p, p + 22);
            return true;
        }
        if c == 91 && self.c(p + 1) == 42 && self.c(p + 2) == 93 {
            self.emit(T::EDGE_STATE, p, p + 3);
            return true;
        }
        let mut j = p;
        while j < self.src.len() {
            let d = self.c(j);
            if matches!(d, 58 | 45 | 123) || char::from_u32(d).is_some_and(space) {
                break;
            }
            j = self.next(j)
        }
        if j > p {
            self.emit_id(p, j);
            return true;
        }
        if c == 58 {
            j = p + 1;
            while j < self.src.len() {
                let d = self.c(j);
                if matches!(d, 10 | 59) {
                    break;
                }
                if d == 58 {
                    let after = self.c(j + 1);
                    if j + 1 >= self.src.len() || matches!(after, 58 | 10 | 59) {
                        break;
                    }
                    j += 1
                }
                j = self.next(j)
            }
            if j > p + 1 {
                self.emit_text(T::DESCR, self.src[p..j].trim_matches(space).into(), p, j);
                return true;
            }
            if self.c(p + 1) == 58 && self.c(p + 2) == 58 {
                self.emit(T::STYLE_SEPARATOR, p, p + 3);
                return true;
            }
        } else if c == 45 && self.c(p + 1) == 45 {
            if self.c(p + 2) == 62 {
                self.emit(T::ARROW, p, p + 3);
                return true;
            }
            if !root {
                self.emit(T::CONCURRENT, p, p + 2);
                return true;
            }
        }
        if !root {
            return false;
        }
        self.emit(T::INVALID, p, self.next(p));
        true
    }
    fn step(&mut self) -> bool {
        let p = self.p;
        let c = self.c(p);
        match self.state {
            S::Initial | S::Struct => return self.initial(),
            S::State => {
                if c != 10 && char::from_u32(c).is_some_and(space) {
                    let mut j = self.next(p);
                    while j < self.src.len()
                        && self.c(j) != 10
                        && char::from_u32(self.c(j)).is_some_and(space)
                    {
                        j = self.next(j)
                    }
                    self.p = j;
                    return true;
                }
                if c == 35 || (c == 37 && self.c(p + 1) == 37 && self.c(p + 2) != 123) {
                    self.p = self.eol(p);
                    return true;
                }
                if let Some(k) = self.stereotype(p) {
                    let at = self.stereo_at[k].unwrap();
                    let choice = k == 2 || k == 5;
                    self.emit_text(
                        if choice {
                            T::CHOICE
                        } else if k % 3 == 0 {
                            T::FORK
                        } else {
                            T::JOIN
                        },
                        self.src[p..at].trim_matches(space).into(),
                        p,
                        at + if choice { 10 } else { 8 },
                    );
                    self.pop();
                    return true;
                }
                if c == 34 {
                    self.p += 1;
                    self.push(S::StateString);
                    return true;
                }
                let q = if c == 10 { self.ws(p) } else { p };
                if self.keyword("as", q) {
                    let e = self.ws(q + 2);
                    if e > q + 2 {
                        self.emit(T::AS, p, e);
                        self.push(S::StateId);
                        return true;
                    }
                }
                if c == 10 {
                    self.p += 1;
                    self.pop();
                    return true;
                }
                if c == 123 {
                    self.emit(T::STRUCT_START, p, p + 1);
                    self.pop();
                    self.push(S::Struct);
                    return true;
                }
                if char::from_u32(c).is_some_and(word) {
                    let first = self.word_end(p);
                    let second = self.ws(first);
                    if second > first && char::from_u32(self.c(second)).is_some_and(word) {
                        if let Some(brace) = next_match(&self.braces, second + 1)
                            .filter(|brace| *brace < self.line_end(second))
                        {
                            self.error = Some(format!(
                                "Error: State name must be a single word. Found: \"{}\"",
                                self.src[p..brace + 1].trim_matches(space)
                            ));
                            return false;
                        }
                    }
                }
                let mut j = self.next(p);
                while j < self.src.len()
                    && self.c(j) != 123
                    && !char::from_u32(self.c(j)).is_some_and(space)
                {
                    j = self.next(j)
                }
                self.emit(T::COMPOSIT_STATE, p, j)
            }
            S::StateString => {
                if c == 34 {
                    self.p += 1;
                    self.pop()
                } else {
                    let close = self.src[p..].find('"').map_or(self.src.len(), |i| p + i);
                    self.emit(T::STATE_DESCR, p, close)
                }
            }
            S::StateId => {
                let end = self.src[p..]
                    .find(['\n', '{'])
                    .map_or(self.src.len(), |i| p + i);
                if self.emit_id(p, end) {
                    self.pop()
                }
            }
            S::Scale => {
                if (48..=57).contains(&c) {
                    let mut j = p + 1;
                    while (48..=57).contains(&self.c(j)) {
                        j += 1
                    }
                    self.emit(T::WIDTH, p, j)
                } else {
                    let q = self.ws(p);
                    if q > p
                        && self.keyword("width", q)
                        && !char::from_u32(self.c(q + 5)).is_some_and(word)
                    {
                        self.p = q + 5;
                        self.pop()
                    } else {
                        return false;
                    }
                }
            }
            S::AccTitle | S::AccDescr => {
                let kind = if self.state == S::AccTitle {
                    T::acc_title_value
                } else {
                    T::acc_descr_value
                };
                self.emit(kind, p, self.eol(p));
                self.pop()
            }
            S::AccMultiline => {
                if c == 125 {
                    self.p += 1;
                    self.pop()
                } else {
                    let end = self.src[p..].find('}').map_or(self.src.len(), |i| p + i);
                    self.emit(T::acc_descr_multiline_value, p, end)
                }
            }
            S::ClassDef => {
                let default = self.keyword("default", p) && self.ws(p + 7) > p + 7;
                let j = if default { p + 7 } else { self.word_end(p) };
                let e = self.ws(j);
                if j == p || e == j {
                    return false;
                }
                self.emit(
                    if default {
                        T::DEFAULT_CLASSDEF_ID
                    } else {
                        T::CLASSDEF_ID
                    },
                    p,
                    e,
                );
                self.pop();
                self.push(S::ClassDefId)
            }
            S::ClassDefId => {
                self.emit(T::CLASSDEF_STYLEOPTS, p, self.eol(p));
                self.pop()
            }
            S::Class => {
                let mut j = self.word_end(p);
                if j == p {
                    return false;
                }
                while self.c(j) == 44 {
                    let k = self.ws(j + 1);
                    let e = self.word_end(k);
                    if e == k {
                        break;
                    }
                    j = e
                }
                self.emit(T::CLASSENTITY_IDS, p, j);
                self.pop();
                self.push(S::ClassStyle)
            }
            S::ClassStyle => {
                self.emit(T::STYLECLASS, p, self.eol(p));
                self.pop()
            }
            S::Style => {
                let mut j = p;
                while j < self.src.len()
                    && (char::from_u32(self.c(j)).is_some_and(word) || self.c(j) == 44)
                {
                    j = self.next(j)
                }
                let e = self.ws(j);
                if j == p || e == j {
                    return false;
                }
                self.emit(T::STYLE_IDS, p, e);
                self.pop();
                self.push(S::StyleDefStyles)
            }
            S::StyleDefStyles => {
                self.emit(T::STYLEDEF_STYLEOPTS, p, self.eol(p));
                self.pop()
            }
            S::Note => {
                for (w, t) in [("left of", T::left_of), ("right of", T::right_of)] {
                    if self.keyword(w, p) && !char::from_u32(self.c(p + w.len())).is_some_and(word)
                    {
                        self.emit(t, p, p + w.len());
                        self.pop();
                        self.push(S::NoteId);
                        return true;
                    }
                }
                if c == 34 {
                    self.p += 1;
                    self.pop();
                    self.push(S::FloatingNote)
                } else {
                    return false;
                }
            }
            S::NoteId => {
                let q = self.ws(p);
                let mut j = q;
                while j < self.src.len() {
                    let d = self.c(j);
                    if matches!(d, 58 | 45) || char::from_u32(d).is_some_and(space) {
                        break;
                    }
                    j = self.next(j)
                }
                if j == q {
                    return false;
                }
                if self.emit_id(p, j) {
                    self.pop();
                    self.push(S::NoteText)
                }
            }
            S::NoteText => {
                let q = self.ws(p);
                if self.c(q) == 58 {
                    let mut j = q + 1;
                    while j < self.src.len() && !matches!(self.c(j), 58 | 10 | 59) {
                        j = self.next(j)
                    }
                    if j > q + 1 {
                        let mut start = p;
                        let mut units = 0;
                        while start < j && units < 2 {
                            let ch = self.src[start..].chars().next().unwrap();
                            units += ch.len_utf16();
                            start += ch.len_utf8()
                        }
                        self.emit_text(
                            T::NOTE_TEXT,
                            self.src[start..j].trim_matches(space).into(),
                            p,
                            j,
                        );
                        self.pop();
                        return true;
                    }
                }
                let mut k = self.src[p..].find('\n').map(|i| p + i);
                let mut end = None;
                while let Some(at) = k {
                    let e = self.ws(at);
                    if self.keyword("end note", e)
                        && !char::from_u32(self.c(e + 8)).is_some_and(word)
                    {
                        end = Some(e);
                        break;
                    }
                    k = self.src[e..].find('\n').map(|i| e + i)
                }
                let Some(end) = end else { return false };
                self.emit_text(
                    T::NOTE_TEXT,
                    self.src[p..end].trim_matches(space).into(),
                    p,
                    end + 8,
                );
                self.pop()
            }
            S::FloatingNote => {
                let q = self.ws(p);
                if self.keyword("as", q) {
                    self.emit(T::AS, p, self.ws(q + 2));
                    self.pop();
                    self.push(S::FloatingNoteId)
                } else if c == 34 {
                    self.p += 1
                } else {
                    let close = self.src[p..].find('"').map_or(self.src.len(), |i| p + i);
                    self.emit(T::NOTE_TEXT, p, close)
                }
            }
            S::FloatingNoteId => {
                if self.emit_id(p, self.eol(p)) {
                    self.pop()
                }
            }
        }
        true
    }
}
pub(super) fn tokenize(src: &str) -> Tokens {
    static RE: OnceLock<[Regex; 2]> = OnceLock::new();
    let re = RE.get_or_init(|| {
        [
            Regex::new(r"(?i-u:direction)").unwrap(),
            Regex::new(r"(?i-u:<<(?:fork|join|choice)>>|\[\[(?:fork|join|choice)\]\])").unwrap(),
        ]
    });
    let mut l = Lexer {
        src,
        p: 0,
        state: S::Initial,
        stack: Vec::new(),
        list: Vec::new(),
        error: None,
        directions: re[0].find_iter(src).map(|m| m.start()).collect(),
        stereotypes: re[1].find_iter(src).map(|m| m.start()).collect(),
        line_ends: src
            .char_indices()
            .filter(|(_, c)| matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
            .map(|(i, _)| i)
            .collect(),
        percent: src.match_indices("%%").map(|(i, _)| i).collect(),
        braces: src.match_indices('{').map(|(i, _)| i).collect(),
        dir_range: None,
        dir_at: [None; 4],
        dir_end: [0; 4],
        stereo_range: None,
        stereo_at: [None; 6],
    };
    while l.p < src.len() {
        if !l.step() {
            return l.finish(T::ERROR);
        }
    }
    let t = match l.state {
        S::Initial => Some(T::NL),
        S::AccTitle => Some(T::acc_title_value),
        S::AccDescr => Some(T::acc_descr_value),
        S::AccMultiline => Some(T::acc_descr_multiline_value),
        S::ClassDefId => Some(T::CLASSDEF_STYLEOPTS),
        S::ClassStyle => Some(T::STYLECLASS),
        S::StyleDefStyles => Some(T::STYLEDEF_STYLEOPTS),
        S::StateString => Some(T::STATE_DESCR),
        S::FloatingNote => Some(T::NOTE_TEXT),
        S::StateId | S::FloatingNoteId => Some(T::ID),
        _ => None,
    };
    if let Some(t) = t {
        l.emit(t, src.len(), src.len())
    }
    l.finish(T::END)
}
