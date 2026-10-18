//! Direct Pele class/lexer.ts state machine, 458322a5 (MIT, kepano).
use super::tokens::T;
use crate::compat::source::{space, unicode::letter, word};
#[derive(Clone, Debug)]
pub struct Token {
    pub kind: T,
    pub text: String,
    pub start: usize,
}
#[derive(Clone, Copy)]
#[repr(usize)]
enum S {
    Initial,
    String,
    Bq,
    Generic,
    CallbackName,
    CallbackArgs,
    AccTitle,
    AccDescr,
    AccMulti,
    Class,
    ClassBody,
    Namespace,
    NamespaceBody,
}
const AT_END: [T; 13] = [
    T::EOF,
    T::STR,
    T::EOF,
    T::GENERICTYPE,
    T::CALLBACK_NAME,
    T::CALLBACK_ARGS,
    T::acc_title_value,
    T::acc_descr_value,
    T::acc_descr_multiline_value,
    T::EOF,
    T::EOF_IN_STRUCT,
    T::EOF,
    T::EOF_IN_STRUCT,
];
struct Lexer<'a> {
    src: &'a str,
    p: usize,
    state: S,
    stack: Vec<S>,
    tokens: Vec<Token>,
    dir_failed: usize,
}
impl Lexer<'_> {
    fn c(&self, p: usize) -> char {
        self.src
            .get(p..)
            .and_then(|s| s.chars().next())
            .unwrap_or('\0')
    }
    fn kw(&self, s: &str) -> bool {
        self.src[self.p..].starts_with(s) && !word(self.c(self.p + s.len()))
    }
    fn ws(&self, mut p: usize) -> usize {
        while p < self.src.len() && space(self.c(p)) {
            p += self.c(p).len_utf8()
        }
        p
    }
    fn until(&self, c: char) -> usize {
        self.src[self.p..]
            .find(c)
            .map_or(self.src.len(), |n| self.p + n)
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
    fn emit(&mut self, t: T, end: usize) -> bool {
        self.tokens.push(Token {
            kind: t,
            text: self.src[self.p..end].into(),
            start: self.p,
        });
        self.p = end;
        true
    }
    fn spaces(&mut self, leaves: bool) {
        let mut q = self.p;
        let mut k = None;
        while q < self.src.len() && space(self.c(q)) {
            if self.c(q) == '\n' {
                k = Some(q)
            }
            q += self.c(q).len_utf8()
        }
        if let Some(k) = k {
            if leaves {
                self.pop()
            }
            self.emit(T::NEWLINE, k + 1);
        } else {
            self.p = q
        }
    }
    fn keyword(&mut self) -> bool {
        for (s, t) in [
            ("cssClass", T::CSSCLASS),
            ("callback", T::CALLBACK),
            ("click", T::CLICK),
            ("link", T::LINK),
            ("note for", T::NOTE_FOR),
            ("note", T::NOTE),
            ("href", T::HREF),
        ] {
            if self.kw(s) {
                return self.emit(t, self.p + s.len());
            }
        }
        if self.src[self.p..].starts_with("<<") {
            return self.emit(T::ANNOTATION_START, self.p + 2);
        }
        if self.src[self.p..].starts_with(">>") {
            return self.emit(T::ANNOTATION_END, self.p + 2);
        }
        false
    }
    fn common(&mut self) -> bool {
        if self.keyword() {
            return true;
        }
        let c = self.c(self.p);
        let d = self.c(self.p + c.len_utf8());
        match c {
            '"' => {
                self.push(S::String);
                self.p += 1;
                return true;
            }
            '~' => {
                self.push(S::Generic);
                self.p += 1;
                return true;
            }
            '`' => {
                self.push(S::Bq);
                self.p += 1;
                return true;
            }
            '_' => {
                for s in ["_self", "_blank", "_parent", "_top"] {
                    if self.kw(s) {
                        return self.emit(T::LINK_TARGET, self.p + s.len());
                    }
                }
            }
            '<' => {
                return self.emit(
                    if d == '|' {
                        T::EXTENSION
                    } else {
                        T::DEPENDENCY
                    },
                    self.p + if d == '|' { 2 } else { 1 },
                )
            }
            '|' => {
                if d == '>' {
                    return self.emit(T::EXTENSION, self.p + 2);
                }
            }
            '>' => return self.emit(T::DEPENDENCY, self.p + 1),
            '*' => return self.emit(T::COMPOSITION, self.p + 1),
            'o' => {
                if !word(d) {
                    return self.emit(T::AGGREGATION, self.p + 1);
                }
            }
            '(' => {
                if d == ')' {
                    return self.emit(T::LOLLIPOP, self.p + 2);
                }
            }
            '-' => {
                return self.emit(
                    if d == '-' { T::LINE } else { T::MINUS },
                    self.p + if d == '-' { 2 } else { 1 },
                )
            }
            '.' => {
                return self.emit(
                    if d == '.' { T::DOTTED_LINE } else { T::DOT },
                    self.p + if d == '.' { 2 } else { 1 },
                )
            }
            ':' => {
                let mut q = self.p + 1;
                while q < self.src.len() && !matches!(self.c(q), ':' | '\n' | ';') {
                    q += self.c(q).len_utf8()
                }
                if q > self.p + 1 {
                    return self.emit(T::LABEL, q);
                }
                if self.src[self.p..].starts_with(":::") {
                    return self.emit(T::STYLE_SEPARATOR, self.p + 3);
                }
            }
            '+' => return self.emit(T::PLUS, self.p + 1),
            '%' => return self.emit(T::PCT, self.p + 1),
            '=' => return self.emit(T::EQUALS, self.p + 1),
            '[' => return self.emit(T::SQS, self.p + 1),
            ']' => return self.emit(T::SQE, self.p + 1),
            '!' | '#' | '$' | '&' | '\'' | ',' | '?' | '\\' | '/' => {
                return self.emit(T::PUNCTUATION, self.p + 1)
            }
            _ => {}
        }
        if word(c) {
            let mut q = self.p + c.len_utf8();
            while word(self.c(q)) {
                q += self.c(q).len_utf8()
            }
            return self.emit(T::ALPHA, q);
        }
        if letter(c as u32) {
            return self.emit(T::UNICODE_TEXT, self.p + c.len_utf8());
        }
        false
    }
    fn direction(&mut self) -> bool {
        if self.p < self.dir_failed {
            return false;
        }
        let end = self.src[self.p..]
            .find(['\n', '\r', '\u{2028}', '\u{2029}'])
            .map_or(self.src.len(), |n| self.p + n);
        let line = &self.src[self.p..end];
        if !line.contains("direction") {
            return false;
        }
        for (dir, t) in [
            ("TB", T::direction_tb),
            ("BT", T::direction_bt),
            ("RL", T::direction_rl),
            ("LR", T::direction_lr),
        ] {
            for (at, _) in line.rmatch_indices("direction") {
                let tail = &line[at + 9..];
                let trimmed = tail.trim_start_matches(space);
                if tail.len() > trimmed.len() && trimmed.starts_with(dir) {
                    let full_end = self.src[self.p..]
                        .find('\n')
                        .map_or(self.src.len(), |n| self.p + n);
                    return self.emit(t, full_end);
                }
            }
        }
        self.dir_failed = end;
        false
    }
    fn initial(&mut self) -> bool {
        if self.direction() {
            return true;
        }
        let c = self.c(self.p);
        if space(c) {
            self.spaces(false);
            return true;
        }
        if self.src[self.p..].starts_with("%%") {
            self.p = self.until('\n');
            while matches!(self.c(self.p), '\n' | '\r') {
                self.p += 1
            }
            return true;
        }
        if c == 'a'
            && (self.src[self.p..].starts_with("accTitle")
                || self.src[self.p..].starts_with("accDescr"))
        {
            let title = self.src[self.p..].starts_with("accTitle");
            let q = self.ws(self.p + 8);
            if self.c(q) == ':' {
                let e = self.ws(q + 1);
                self.emit(if title { T::acc_title } else { T::acc_descr }, e);
                self.push(if title { S::AccTitle } else { S::AccDescr });
                return true;
            }
            if self.c(q) == '{' && !title {
                self.p = self.ws(q + 1);
                self.push(S::AccMulti);
                return true;
            }
        }
        for (s, t) in [
            ("classDiagram-v2", T::CLASS_DIAGRAM),
            ("classDiagram", T::CLASS_DIAGRAM),
            ("classDef", T::CLASSDEF),
        ] {
            if self.kw(s) {
                return self.emit(t, self.p + s.len());
            }
        }
        if self.src[self.p..].starts_with("call") && space(self.c(self.p + 4)) {
            self.p = self.ws(self.p + 4);
            self.push(S::CallbackName);
            return true;
        }
        if self.kw("class") {
            self.push(S::Class);
            return self.emit(T::CLASS, self.p + 5);
        }
        if self.kw("namespace") {
            self.push(S::Namespace);
            return self.emit(T::NAMESPACE, self.p + 9);
        }
        if self.kw("style") {
            return self.emit(T::STYLE, self.p + 5);
        }
        if self.src[self.p..].starts_with("[*]") {
            return self.emit(T::EDGE_STATE, self.p + 3);
        }
        if c == ',' {
            return self.emit(T::COMMA, self.p + 1);
        }
        if c == '#' {
            return self.emit(T::BRKT, self.p + 1);
        }
        self.common() || (c == ':' && self.emit(T::COLON, self.p + 1))
    }
}
pub fn tokenize(src: &str) -> Vec<Token> {
    let mut l = Lexer {
        src,
        p: 0,
        state: S::Initial,
        stack: vec![],
        tokens: vec![],
        dir_failed: 0,
    };
    loop {
        if l.p >= src.len() {
            l.tokens.push(Token {
                kind: AT_END[l.state as usize],
                text: String::new(),
                start: src.len(),
            });
            break;
        }
        let c = l.c(l.p);
        let ok = match l.state {
            S::Initial => l.initial(),
            S::Class => {
                if space(c) {
                    l.spaces(true);
                    true
                } else if c == '}' {
                    l.pop();
                    l.pop();
                    l.emit(T::STRUCT_STOP, l.p + 1)
                } else if c == '{' {
                    l.push(S::ClassBody);
                    l.emit(T::STRUCT_START, l.p + 1)
                } else {
                    l.common()
                }
            }
            S::ClassBody => {
                if c == '"' {
                    l.push(S::String);
                    l.p += 1;
                    true
                } else if c == '}' {
                    l.pop();
                    l.emit(T::STRUCT_STOP, l.p + 1)
                } else if src[l.p..].starts_with("[*]") {
                    l.emit(T::EDGE_STATE, l.p + 3)
                } else if c == '{' {
                    l.emit(T::OPEN_IN_STRUCT, l.p + 1)
                } else if c == '\n' {
                    l.p += 1;
                    true
                } else {
                    let mut q = l.p + c.len_utf8();
                    while q < src.len() && !matches!(l.c(q), '{' | '}' | '\n') {
                        q += l.c(q).len_utf8()
                    }
                    l.emit(T::MEMBER, q)
                }
            }
            S::Namespace => {
                if space(c) {
                    l.spaces(true);
                    true
                } else if c == '{' {
                    l.push(S::NamespaceBody);
                    l.emit(T::STRUCT_START, l.p + 1)
                } else if c == '}' {
                    l.pop();
                    true
                } else if l.kw("namespace") {
                    l.push(S::Namespace);
                    l.emit(T::NAMESPACE, l.p + 9)
                } else {
                    l.common()
                }
            }
            S::NamespaceBody => {
                if space(c) {
                    l.spaces(false);
                    true
                } else if c == '}' {
                    l.pop();
                    l.emit(T::STRUCT_STOP, l.p + 1)
                } else if l.kw("namespace") {
                    l.push(S::Namespace);
                    l.emit(T::NAMESPACE, l.p + 9)
                } else if l.kw("class") {
                    l.push(S::Class);
                    l.emit(T::CLASS, l.p + 5)
                } else if src[l.p..].starts_with("[*]") {
                    l.emit(T::EDGE_STATE, l.p + 3)
                } else {
                    l.common()
                }
            }
            S::String => {
                if c == '"' {
                    l.pop();
                    l.p += 1;
                    true
                } else {
                    l.emit(T::STR, l.until('"'))
                }
            }
            S::Generic => {
                if c == '"' {
                    l.push(S::String);
                    l.p += 1;
                    true
                } else if l.keyword() {
                    true
                } else if c == '~' {
                    l.pop();
                    l.p += 1;
                    true
                } else {
                    l.emit(T::GENERICTYPE, l.until('~'))
                }
            }
            S::Bq => {
                if c == '"' {
                    l.push(S::String);
                    l.p += 1;
                    true
                } else if l.keyword() {
                    true
                } else if c == '~' {
                    l.push(S::Generic);
                    l.p += 1;
                    true
                } else if c == '`' {
                    l.pop();
                    l.p += 1;
                    true
                } else {
                    l.emit(T::BQUOTE_STR, l.until('`'))
                }
            }
            S::CallbackName => {
                if c == '(' {
                    let q = l.ws(l.p + 1);
                    l.pop();
                    if l.c(q) == ')' {
                        l.p = q + 1
                    } else {
                        l.push(S::CallbackArgs);
                        l.p += 1
                    }
                    true
                } else {
                    l.emit(T::CALLBACK_NAME, l.until('('))
                }
            }
            S::CallbackArgs => {
                if c == ')' {
                    l.pop();
                    l.p += 1;
                    true
                } else {
                    l.emit(T::CALLBACK_ARGS, l.until(')'))
                }
            }
            S::AccTitle | S::AccDescr => {
                let t = if matches!(l.state, S::AccTitle) {
                    T::acc_title_value
                } else {
                    T::acc_descr_value
                };
                l.emit(t, l.until('\n'));
                l.pop();
                true
            }
            S::AccMulti => {
                if c == '}' {
                    l.pop();
                    l.p += 1;
                    true
                } else {
                    l.emit(T::acc_descr_multiline_value, l.until('}'))
                }
            }
        };
        if !ok {
            l.tokens.push(Token {
                kind: T::ERROR,
                text: String::new(),
                start: l.p,
            });
            return l.tokens;
        }
    }
    l.tokens.push(Token {
        kind: T::END,
        text: String::new(),
        start: src.len(),
    });
    l.tokens
}
