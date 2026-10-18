//! Pele c4/lexer.ts bounded state scanner, 458322a5 (MIT, kepano).
use super::tokens::T;
use crate::compat::source::{space, word};
#[derive(Clone, Copy, PartialEq, Eq)]
enum S {
    Initial,
    Struct,
    Attribute,
    String,
    StringKv,
    StringKey,
    StringValue,
    Acc,
    AccMultiline,
}
pub(super) struct Token {
    pub kind: T,
    pub start: usize,
    pub end: usize,
}
struct Scanner<'a> {
    src: &'a str,
    p: usize,
    state: S,
    acc_value: T,
    out: Vec<Token>,
    directions: Vec<usize>,
    line_ends: Vec<usize>,
    dir_range: Option<(usize, usize)>,
    dir_at: [Option<usize>; 4],
    dir_end: [usize; 4],
}
fn next_match(list: &[usize], p: usize) -> Option<usize> {
    list.get(list.partition_point(|v| *v < p)).copied()
}
impl Scanner<'_> {
    fn ch(&self, p: usize) -> Option<char> {
        self.src.get(p..)?.chars().next()
    }
    fn next(&self, p: usize) -> usize {
        p + self.ch(p).map_or(0, char::len_utf8)
    }
    fn up_to(&self, c: char, p: usize) -> usize {
        self.src[p..].find(c).map_or(self.src.len(), |i| i + p)
    }
    fn ws(&self, mut p: usize) -> usize {
        while self.ch(p).is_some_and(space) {
            p = self.next(p)
        }
        p
    }
    fn emit(&mut self, kind: T, start: usize, end: usize) {
        self.out.push(Token { kind, start, end });
        self.p = end
    }
    fn statement(&mut self, name: &str, kind: T) -> bool {
        let q = self.p + name.len();
        if !self.src[self.p..].starts_with(name) || !self.ch(q).is_some_and(space) {
            return false;
        }
        let from = self.next(q);
        let mut end = from;
        while let Some(c) = self.ch(end) {
            if matches!(c, '#' | '\n' | ';') {
                break;
            }
            end += c.len_utf8()
        }
        if end == from {
            return false;
        }
        self.emit(kind, self.p, end);
        true
    }
    fn direction(&mut self, p: usize) -> Option<usize> {
        if self.dir_range.is_none_or(|(from, to)| p < from || p > to) {
            let first = next_match(&self.directions, p);
            let end = if first.is_none() {
                self.src.len()
            } else {
                next_match(&self.line_ends, p).unwrap_or(self.src.len())
            };
            self.dir_range = Some((p, end));
            self.dir_at = [None; 4];
            let mut at = first;
            while let Some(d) = at.filter(|d| *d < end) {
                let after = d + 9;
                let w = self.ws(after);
                if w > after {
                    for (k, name) in ["TB", "BT", "RL", "LR"].iter().enumerate() {
                        if self.src[w..].starts_with(name) {
                            self.dir_at[k] = Some(d);
                            self.dir_end[k] = self.up_to('\n', w + 2);
                            break;
                        }
                    }
                }
                at = next_match(&self.directions, d + 1)
            }
        }
        (0..4).find(|i| self.dir_at[*i].is_some_and(|d| d >= p))
    }
    fn run(mut self) -> Vec<Token> {
        let n = self.src.len();
        let mut done = false;
        loop {
            if done {
                self.emit(T::END, n, n);
                break;
            }
            if self.p >= n {
                done = true
            }
            let p = self.p;
            let c = self.ch(p);
            match self.state {
                S::Initial => {
                    if p >= n {
                        self.emit(T::EOF, n, n);
                        continue;
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
                            self.dir_end[kind],
                        );
                        continue;
                    }
                    if c == Some('t') && self.statement("title", T::title) {
                        continue;
                    }
                    if c == Some('a') {
                        if self.statement("accDescription", T::accDescription) {
                            continue;
                        }
                        let title = self.src[p..].starts_with("accTitle");
                        if title || self.src[p..].starts_with("accDescr") {
                            let q = self.ws(p + 8);
                            if self.ch(q) == Some(':') {
                                self.emit(
                                    if title { T::acc_title } else { T::acc_descr },
                                    p,
                                    self.ws(q + 1),
                                );
                                self.acc_value = if title {
                                    T::acc_title_value
                                } else {
                                    T::acc_descr_value
                                };
                                self.state = S::Acc;
                                continue;
                            }
                            if self.ch(q) == Some('{') && !title {
                                self.p = self.ws(q + 1);
                                self.state = S::AccMultiline;
                                continue;
                            }
                        }
                    }
                    if self.src[p..].starts_with("%%") {
                        let mut q = self.up_to('\n', p + 2);
                        while matches!(self.ch(q), Some('\n' | '\r')) {
                            q += 1
                        }
                        self.p = q;
                        continue;
                    }
                    if c.is_some_and(space) {
                        let mut q = p;
                        let mut last = None;
                        while self.ch(q).is_some_and(space) {
                            if self.ch(q) == Some('\n') {
                                last = Some(q)
                            }
                            q = self.next(q)
                        }
                        if let Some(last) = last {
                            self.emit(T::NEWLINE, p, last + 1)
                        } else {
                            self.p = q
                        }
                        continue;
                    }
                    if c == Some('{') {
                        self.emit(T::LBRACE, p, p + 1);
                        continue;
                    }
                    if c == Some('}') {
                        self.emit(T::RBRACE, p, p + 1);
                        continue;
                    }
                    if c.is_some_and(word) {
                        let mut q = p + 1;
                        while self.ch(q).is_some_and(word) {
                            q += 1
                        }
                        if q - p <= 19 {
                            if let Some(kind) = keyword(&self.src[p..q]) {
                                if kind > T::C4_DEPLOYMENT {
                                    self.state = S::Struct
                                }
                                self.emit(kind, p, q);
                                continue;
                            }
                        }
                    }
                    self.emit(T::ERROR, p, p);
                    break;
                }
                S::Struct => {
                    if p >= n {
                        self.emit(T::EOF_IN_STRUCT, n, n)
                    } else if c == Some('(') {
                        let mut q = p + 1;
                        while self.ch(q) == Some(' ') {
                            q += 1
                        }
                        if self.ch(q) == Some(',') {
                            self.emit(T::ATTRIBUTE_EMPTY, p, q + 1)
                        } else {
                            self.p += 1
                        }
                        self.state = S::Attribute
                    } else if c == Some(')') {
                        self.p += 1;
                        self.state = S::Initial
                    } else {
                        self.emit(T::ERROR, p, p);
                        break;
                    }
                }
                S::Attribute => {
                    if p >= n {
                        continue;
                    }
                    if c == Some(')') {
                        self.p += 1;
                        self.state = S::Initial
                    } else if c == Some(',') {
                        if self.ch(p + 1) == Some(',') {
                            self.emit(T::ATTRIBUTE_EMPTY, p, p + 2)
                        } else {
                            self.p += 1
                        }
                    } else {
                        let mut q = p;
                        while self.ch(q) == Some(' ') {
                            q += 1
                        }
                        match self.ch(q) {
                            Some('"') => {
                                if self.ch(q + 1) == Some('"') {
                                    self.emit(T::ATTRIBUTE_EMPTY, p, q + 2)
                                } else {
                                    self.p = q + 1;
                                    self.state = S::String
                                }
                            }
                            Some('$') => {
                                self.p = q + 1;
                                self.state = S::StringKv
                            }
                            _ => self.emit(T::STR, p, self.up_to(',', p)),
                        }
                    }
                }
                S::String => {
                    if c == Some('"') {
                        self.p += 1;
                        self.state = S::Attribute
                    } else {
                        self.emit(T::STR, p, self.up_to('"', p))
                    }
                }
                S::StringKv => {
                    self.emit(T::STR_KEY, p, self.up_to('=', p));
                    self.state = S::StringKey
                }
                S::StringKey => {
                    if p >= n {
                        continue;
                    }
                    let mut q = p + 1;
                    if c == Some('=') {
                        while self.ch(q) == Some(' ') {
                            q += 1
                        }
                    }
                    if c != Some('=') || self.ch(q) != Some('"') {
                        self.emit(T::ERROR, p, p);
                        break;
                    }
                    self.p = q + 1;
                    self.state = S::StringValue
                }
                S::StringValue => {
                    if p >= n {
                        continue;
                    }
                    if c == Some('"') {
                        self.p += 1;
                        self.state = S::Attribute
                    } else {
                        self.emit(T::STR_VALUE, p, self.up_to('"', p))
                    }
                }
                S::Acc => {
                    self.emit(self.acc_value, p, self.up_to('\n', p));
                    self.state = S::Initial
                }
                S::AccMultiline => {
                    if c == Some('}') {
                        self.p += 1;
                        self.state = S::Initial
                    } else {
                        self.emit(T::acc_descr_multiline_value, p, self.up_to('}', p))
                    }
                }
            }
        }
        self.out
    }
}
fn keyword(name: &str) -> Option<T> {
    Some(match name {
        "C4Context" => T::C4_CONTEXT,
        "C4Container" => T::C4_CONTAINER,
        "C4Component" => T::C4_COMPONENT,
        "C4Dynamic" => T::C4_DYNAMIC,
        "C4Deployment" => T::C4_DEPLOYMENT,
        "Person" => T::PERSON,
        "Person_Ext" => T::PERSON_EXT,
        "System" => T::SYSTEM,
        "SystemDb" => T::SYSTEM_DB,
        "SystemQueue" => T::SYSTEM_QUEUE,
        "System_Ext" => T::SYSTEM_EXT,
        "SystemDb_Ext" => T::SYSTEM_EXT_DB,
        "SystemQueue_Ext" => T::SYSTEM_EXT_QUEUE,
        "Container" => T::CONTAINER,
        "ContainerDb" => T::CONTAINER_DB,
        "ContainerQueue" => T::CONTAINER_QUEUE,
        "Container_Ext" => T::CONTAINER_EXT,
        "ContainerDb_Ext" => T::CONTAINER_EXT_DB,
        "ContainerQueue_Ext" => T::CONTAINER_EXT_QUEUE,
        "Component" => T::COMPONENT,
        "ComponentDb" => T::COMPONENT_DB,
        "ComponentQueue" => T::COMPONENT_QUEUE,
        "Component_Ext" => T::COMPONENT_EXT,
        "ComponentDb_Ext" => T::COMPONENT_EXT_DB,
        "ComponentQueue_Ext" => T::COMPONENT_EXT_QUEUE,
        "Enterprise_Boundary" => T::ENTERPRISE_BOUNDARY,
        "System_Boundary" => T::SYSTEM_BOUNDARY,
        "Boundary" => T::BOUNDARY,
        "Container_Boundary" => T::CONTAINER_BOUNDARY,
        "Deployment_Node" => T::NODE,
        "Node" => T::NODE,
        "Node_L" => T::NODE_L,
        "Node_R" => T::NODE_R,
        "Rel" => T::REL,
        "BiRel" => T::BIREL,
        "Rel_Up" => T::REL_U,
        "Rel_U" => T::REL_U,
        "Rel_Down" => T::REL_D,
        "Rel_D" => T::REL_D,
        "Rel_Left" => T::REL_L,
        "Rel_L" => T::REL_L,
        "Rel_Right" => T::REL_R,
        "Rel_R" => T::REL_R,
        "Rel_Back" => T::REL_B,
        "RelIndex" => T::REL_INDEX,
        "UpdateElementStyle" => T::UPDATE_EL_STYLE,
        "UpdateRelStyle" => T::UPDATE_REL_STYLE,
        "UpdateLayoutConfig" => T::UPDATE_LAYOUT_CONFIG,
        _ => return None,
    })
}
pub(super) fn tokenize(src: &str) -> Vec<Token> {
    let directions = src.match_indices("direction").map(|(i, _)| i).collect();
    let line_ends = src
        .char_indices()
        .filter(|(_, c)| matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
        .map(|(i, _)| i)
        .collect();
    Scanner {
        src,
        p: 0,
        state: S::Initial,
        acc_value: T::acc_title_value,
        out: Vec::new(),
        directions,
        line_ends,
        dir_range: None,
        dir_at: [None; 4],
        dir_end: [0; 4],
    }
    .run()
}
