//! Pele requirement/lexer.ts ordered six-state scanner, 458322a5 (MIT, kepano).
use super::tokens::{keyword, T};
use crate::compat::{
    source::{space, word},
    text::SPACE,
};
use regex::Regex;
use std::sync::OnceLock;
pub(super) struct Token {
    pub kind: T,
    pub text: String,
    pub start: usize,
}
fn scan(s: &str, p: usize, pred: impl Fn(char) -> bool) -> usize {
    s[p..]
        .char_indices()
        .find(|(_, c)| !pred(*c))
        .map_or(s.len(), |(i, _)| p + i)
}
fn lf(s: &str, p: usize) -> usize {
    s[p..].find('\n').map_or(s.len(), |i| p + i)
}
fn ci(s: &str, p: usize, w: &str) -> bool {
    s.get(p..p + w.len())
        .is_some_and(|v| v.eq_ignore_ascii_case(w))
}
fn acc(s: &str, p: usize, w: &str, separator: char) -> Option<usize> {
    if !ci(s, p, w) {
        return None;
    }
    let q = scan(s, p + w.len(), space);
    if !s[q..].starts_with(separator) {
        return None;
    }
    Some(scan(s, q + separator.len_utf8(), space))
}
struct Lexer<'a> {
    s: &'a str,
    p: usize,
    state: usize,
    stack: Vec<usize>,
    out: Vec<Token>,
    dirs: [Vec<(usize, usize)>; 4],
    seen: [usize; 4],
    line_end: Option<usize>,
}
impl Lexer<'_> {
    fn emit(&mut self, kind: T, start: usize, end: usize) {
        self.out.push(Token {
            kind,
            text: self.s[start..end].into(),
            start,
        });
        self.p = end;
    }
    fn push(&mut self, state: usize) {
        self.stack.push(self.state);
        self.state = state;
    }
    fn pop(&mut self) {
        if let Some(s) = self.stack.pop() {
            self.state = s;
        }
    }
    fn direction(&mut self) -> bool {
        if self.line_end.is_none_or(|end| end < self.p) {
            let end = scan(self.s, self.p, |c| {
                !matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}')
            });
            self.line_end = Some(end);
            for k in 0..4 {
                while self.seen[k] < self.dirs[k].len() && self.dirs[k][self.seen[k]].0 <= end {
                    self.seen[k] += 1;
                }
            }
        }
        for k in 0..4 {
            if let Some(j) = self.seen[k].checked_sub(1) {
                let (start, end) = self.dirs[k][j];
                if start >= self.p {
                    self.emit(
                        [
                            T::direction_tb,
                            T::direction_bt,
                            T::direction_rl,
                            T::direction_lr,
                        ][k],
                        self.p,
                        lf(self.s, end),
                    );
                    return true;
                }
            }
        }
        false
    }
    fn initial(&mut self, c: char) -> bool {
        let s = self.s;
        let p = self.p;
        if ci(s, p, "title") && s[p + 5..].chars().next().is_some_and(space) {
            let at = p + 5 + s[p + 5..].chars().next().unwrap().len_utf8();
            let q = scan(s, at, |c| !matches!(c, '#' | '\n' | ';'));
            if q > at {
                self.emit(T::title, p, q);
                return true;
            }
        }
        if let Some(e) = acc(s, p, "accTitle", ':') {
            self.push(3);
            self.emit(T::acc_title, p, e);
            return true;
        }
        if let Some(e) = acc(s, p, "accDescr", ':') {
            self.push(4);
            self.emit(T::acc_descr, p, e);
            return true;
        }
        if let Some(e) = acc(s, p, "accDescr", '{') {
            self.push(5);
            self.p = e;
            return true;
        }
        if !matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}') && self.direction() {
            return true;
        }
        if c == '\n' || s[p..].starts_with("\r\n") {
            let mut q = p;
            while q < s.len() {
                if s[q..].starts_with('\n') {
                    q += 1
                } else if s[q..].starts_with("\r\n") {
                    q += 2
                } else {
                    break;
                }
            }
            self.emit(T::NEWLINE, p, q);
            return true;
        }
        if space(c) {
            self.p = scan(s, p, space);
            return true;
        }
        if matches!(c, '#' | '%') {
            self.p = lf(s, p);
            return true;
        }
        if word(c) {
            let mut q = scan(s, p, word);
            if q - p <= 22 {
                if let Some(t) = keyword(&s[p..q].to_ascii_lowercase()) {
                    if matches!(t, T::STYLE | T::CLASSDEF | T::CLASS) {
                        self.push(2)
                    }
                    self.emit(t, p, q);
                    return true;
                }
            }
            q = scan(s, q, |c| {
                !matches!(c, ':' | ',' | '\r' | '\n' | '{' | '<' | '>' | '-' | '=')
            });
            self.out.push(Token {
                kind: T::unqString,
                text: s[p..q].trim_matches(space).into(),
                start: p,
            });
            self.p = q;
            return true;
        }
        let (t, n) = match c {
            '{' => (T::STRUCT_START, 1),
            '}' => (T::STRUCT_STOP, 1),
            ':' => {
                if s[p..].starts_with(":::") {
                    (T::STYLE_SEPARATOR, 3)
                } else {
                    (T::COLONSEP, 1)
                }
            }
            '<' => {
                if s[p..].starts_with("<-") {
                    (T::END_ARROW_L, 2)
                } else {
                    return false;
                }
            }
            '-' => {
                if s[p..].starts_with("->") {
                    (T::END_ARROW_R, 2)
                } else {
                    (T::LINE, 1)
                }
            }
            '"' => {
                self.push(1);
                self.p += 1;
                return true;
            }
            ',' => (T::COMMA, 1),
            _ => return false,
        };
        self.emit(t, p, p + n);
        true
    }
    fn style(&mut self, c: char) -> bool {
        let p = self.p;
        if word(c) {
            let q = scan(self.s, p, word);
            self.emit(T::ALPHA, p, q);
            return true;
        }
        let t = match c {
            ':' => T::COLON,
            ';' => T::SEMICOLON,
            '%' => T::PERCENT,
            '-' => T::MINUS,
            '#' => T::BRKT,
            ' ' => {
                self.p += 1;
                return true;
            }
            '"' => {
                self.push(1);
                self.p += 1;
                return true;
            }
            '\n' => {
                self.pop();
                self.p += 1;
                return true;
            }
            ',' => T::COMMA,
            _ => return false,
        };
        self.emit(t, p, p + 1);
        true
    }
}
pub(super) fn tokenize(s: &str) -> Vec<Token> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(&format!("(?i-u:direction){SPACE}+(?i-u:TB|BT|RL|LR)")).unwrap()
    });
    let mut dirs: [Vec<(usize, usize)>; 4] = std::array::from_fn(|_| vec![]);
    for m in re.find_iter(s) {
        let kind = match s[m.end() - 2..m.end()].to_ascii_uppercase().as_str() {
            "TB" => 0,
            "BT" => 1,
            "RL" => 2,
            _ => 3,
        };
        dirs[kind].push((m.start(), m.end()));
    }
    let mut l = Lexer {
        s,
        p: 0,
        state: 0,
        stack: vec![],
        out: vec![],
        dirs,
        seen: [0; 4],
        line_end: None,
    };
    while l.p < s.len() {
        let p = l.p;
        let c = s[p..].chars().next().unwrap();
        let ok = match l.state {
            0 => l.initial(c),
            2 => l.style(c),
            1 => {
                if c == '"' {
                    l.pop();
                    l.p += 1
                } else {
                    let end = s[p..].find('"').map_or(s.len(), |i| p + i);
                    l.emit(T::qString, p, end)
                }
                true
            }
            3 | 4 => {
                let kind = if l.state == 3 {
                    T::acc_title_value
                } else {
                    T::acc_descr_value
                };
                l.pop();
                l.emit(kind, p, lf(s, p));
                true
            }
            _ => {
                if c == '}' {
                    l.pop();
                    l.p += 1
                } else {
                    let end = s[p..].find('}').map_or(s.len(), |i| p + i);
                    l.emit(T::acc_descr_multiline_value, p, end);
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
    let last = match l.state {
        0 => Some(T::EOF),
        1 => Some(T::qString),
        3 => Some(T::acc_title_value),
        4 => Some(T::acc_descr_value),
        5 => Some(T::acc_descr_multiline_value),
        _ => None,
    };
    if let Some(t) = last {
        l.emit(t, s.len(), s.len())
    }
    l.emit(T::END, s.len(), s.len());
    l.out
}
