//! Pele gantt/lexer.ts state-machine translation, 458322a5 (MIT, kepano).
use super::tokens::T;
use crate::compat::source::{space, word};
use regex::Regex;
use std::sync::OnceLock;
#[derive(Clone)]
pub struct Token {
    pub kind: T,
    pub text: String,
    pub start: usize,
}
#[derive(Clone, Copy)]
enum S {
    Initial,
    AccTitle,
    AccDescr,
    AccMulti,
    Href,
    CallbackName,
    CallbackArgs,
    Click,
}
fn patterns() -> &'static Vec<Regex> {
    static R: OnceLock<Vec<Regex>> = OnceLock::new();
    R.get_or_init(|| {
        [
            r"^(?i-u:accTitle)\s*:\s*",
            r"^(?i-u:accDescr)\s*:\s*",
            r"^(?i-u:accDescr)\s*\{\s*",
            r#"^(?i-u:href)\s+""#,
            r"^(?i-u:call)\s+",
            r"^(?i-u:click)\s+",
            r"^\(\s*\)",
            r"^(?i-u:weekday)\s+(?i-u:(monday|tuesday|wednesday|thursday|friday|saturday|sunday))(?-u:\b)",
            r"^(?i-u:weekend)\s+(?i-u:(?:(friday)|saturday))(?-u:\b)",
            r"^[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9](?-u:\b)",
        ]
        .iter()
        .map(|s| compiled_regex(s).unwrap())
        .collect()
    })
}
struct Lexer<'a> {
    src: &'a str,
    p: usize,
    state: S,
    tokens: Vec<Token>,
}
impl Lexer<'_> {
    fn c(&self, p: usize) -> char {
        self.src
            .get(p..)
            .and_then(|s| s.chars().next())
            .unwrap_or('\0')
    }
    fn until(&self, c: char, from: usize) -> usize {
        self.src[from..]
            .find(c)
            .map_or(self.src.len(), |n| from + n)
    }
    fn at(&self, i: usize) -> Option<usize> {
        patterns()[i]
            .find(&self.src[self.p..])
            .map(|m| self.p + m.end())
    }
    fn emit(&mut self, t: T, end: usize) {
        self.tokens.push(Token {
            kind: t,
            text: self.src[self.p..end].into(),
            start: self.p,
        });
        self.p = end
    }
    fn keyword(&mut self) -> bool {
        for (i, state, token) in [
            (0, S::AccTitle, Some(T::acc_title)),
            (1, S::AccDescr, Some(T::acc_descr)),
            (2, S::AccMulti, None),
            (3, S::Href, None),
            (4, S::CallbackName, None),
            (5, S::Click, None),
        ] {
            if let Some(e) = self.at(i) {
                self.state = state;
                if let Some(t) = token {
                    self.emit(t, e)
                } else {
                    self.p = e
                }
                return true;
            }
        }
        if let Some(cap) = patterns()[7].captures(&self.src[self.p..]) {
            let d = cap[1].to_ascii_lowercase();
            let t = match d.as_str() {
                "monday" => T::weekday_monday,
                "tuesday" => T::weekday_tuesday,
                "wednesday" => T::weekday_wednesday,
                "thursday" => T::weekday_thursday,
                "friday" => T::weekday_friday,
                "saturday" => T::weekday_saturday,
                _ => T::weekday_sunday,
            };
            self.emit(t, self.p + cap[0].len());
            return true;
        }
        if let Some(cap) = patterns()[8].captures(&self.src[self.p..]) {
            self.emit(
                if cap.get(1).is_some() {
                    T::weekend_friday
                } else {
                    T::weekend_saturday
                },
                self.p + cap[0].len(),
            );
            return true;
        }
        let rules = [
            ("axisFormat", T::axisFormat, 0),
            ("accDescription", T::accDescription, 0),
            ("gantt", T::gantt, 3),
            ("dateFormat", T::dateFormat, 0),
            ("inclusiveEndDates", T::inclusiveEndDates, 3),
            ("includes", T::includes, 0),
            ("topAxis", T::topAxis, 3),
            ("tickInterval", T::tickInterval, 0),
            ("todayMarker", T::todayMarker, 1),
            ("title", T::title, 2),
            ("excludes", T::excludes, 0),
            ("section", T::section, 2),
        ];
        for (s, t, mode) in rules {
            if !self.src[self.p..]
                .get(..s.len())
                .is_some_and(|v| v.eq_ignore_ascii_case(s))
            {
                continue;
            }
            let at = self.p + s.len();
            if mode == 3 {
                if !word(self.c(at)) {
                    self.emit(t, at);
                    return true;
                }
                continue;
            }
            if !space(self.c(at)) {
                continue;
            }
            let mut e = at + self.c(at).len_utf8();
            let start = e;
            while e < self.src.len()
                && self.c(e) != '\n'
                && (mode >= 2 || self.c(e) != ';')
                && (mode != 0 || self.c(e) != '#')
            {
                e += self.c(e).len_utf8()
            }
            if e > start {
                self.emit(t, e);
                return true;
            }
        }
        false
    }
}
pub fn tokenize(src: &str) -> Vec<Token> {
    let mut l = Lexer {
        src,
        p: 0,
        state: S::Initial,
        tokens: vec![],
    };
    loop {
        if l.p >= src.len() {
            let t = match l.state {
                S::Initial => T::EOF,
                S::AccTitle => T::acc_title_value,
                S::AccDescr => T::acc_descr_value,
                S::AccMulti => T::acc_descr_multiline_value,
                S::Href => T::href,
                S::CallbackName => T::callbackname,
                S::CallbackArgs => T::callbackargs,
                S::Click => T::click,
            };
            l.emit(t, src.len());
            break;
        }
        let c = l.c(l.p);
        match l.state {
            S::Initial => {
                let d = l.c(l.p + c.len_utf8());
                if c == '%' {
                    if d != '%' {
                        l.p = l.until('\n', l.p + 1);
                        while l.c(l.p) == '\n' {
                            l.p += 1
                        }
                    } else if l.c(l.p + 2) == '{' {
                        l.emit(T::open_directive, l.p + 3);
                        l.emit(T::ERROR, l.p);
                        return l.tokens;
                    } else {
                        l.p = l.until('\n', l.p + 2)
                    }
                } else if c != '}' && d == '%' {
                    l.p = l.until('\n', l.p + c.len_utf8() + 1)
                } else if c == '\n' {
                    let mut e = l.p + 1;
                    while l.c(e) == '\n' {
                        e += 1
                    }
                    l.emit(T::NL, e)
                } else if space(c) {
                    l.p += c.len_utf8();
                    while l.p < src.len() && space(l.c(l.p)) {
                        l.p += l.c(l.p).len_utf8()
                    }
                } else if l.keyword() {
                } else if c.is_ascii_digit() && l.at(9).is_some() {
                    l.emit(T::date, l.p + 10)
                } else if c != ':' {
                    let mut e = l.p + c.len_utf8();
                    while e < src.len() && !matches!(l.c(e), ':' | '\n') {
                        e += l.c(e).len_utf8()
                    }
                    l.emit(T::taskTxt, e)
                } else {
                    let mut e = l.p + 1;
                    while e < src.len() && !matches!(l.c(e), '#' | '\n' | ';') {
                        e += l.c(e).len_utf8()
                    }
                    l.emit(if e > l.p + 1 { T::taskData } else { T::COLON }, e)
                }
            }
            S::AccTitle | S::AccDescr => {
                let t = if matches!(l.state, S::AccTitle) {
                    T::acc_title_value
                } else {
                    T::acc_descr_value
                };
                l.state = S::Initial;
                l.emit(t, l.until('\n', l.p))
            }
            S::AccMulti => {
                if c == '}' {
                    l.state = S::Initial;
                    l.p += 1
                } else {
                    l.emit(T::acc_descr_multiline_value, l.until('}', l.p))
                }
            }
            S::Href => {
                if c == '"' {
                    l.state = S::Initial;
                    l.p += 1
                } else {
                    l.emit(T::href, l.until('"', l.p))
                }
            }
            S::CallbackName => {
                if c == '(' {
                    if let Some(e) = l.at(6) {
                        l.state = S::Initial;
                        l.p = e
                    } else {
                        l.state = S::CallbackArgs;
                        l.p += 1
                    }
                } else {
                    l.emit(T::callbackname, l.until('(', l.p))
                }
            }
            S::CallbackArgs => {
                if c == ')' {
                    l.state = S::Initial;
                    l.p += 1
                } else {
                    l.emit(T::callbackargs, l.until(')', l.p))
                }
            }
            S::Click => {
                if space(c) {
                    l.state = S::Initial;
                    l.p += c.len_utf8()
                } else {
                    let mut e = l.p + c.len_utf8();
                    while e < src.len() && !space(l.c(e)) {
                        e += l.c(e).len_utf8()
                    }
                    l.emit(T::click, e)
                }
            }
        }
    }
    l.tokens.push(Token {
        kind: T::END,
        text: String::new(),
        start: src.len(),
    });
    l.tokens
}

fn compiled_regex(pattern: &str) -> Result<Regex, regex::Error> {
    Regex::new(&pattern.replace(r"\s", crate::compat::text::SPACE))
}
