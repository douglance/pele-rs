//! Direct Pele sequence/lexer.ts state scanner, 458322a5 (MIT, kepano).
use super::tokens::T;
use crate::compat::source::space;
#[derive(Clone, Debug)]
pub(super) struct Token {
    pub kind: T,
    pub text: String,
    pub start: usize,
}
#[derive(Clone, Copy, PartialEq)]
enum S {
    Initial,
    Id,
    Alias,
    Line,
    Config,
    Title,
    Descr,
    Multiline,
}
#[derive(Clone, Copy)]
enum C {
    ActorStart,
    ActorRest,
    Id,
    IdWide,
    Line,
    Menu,
}
fn word(c: Option<char>) -> bool {
    c.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
}
fn has(c: char, class: C) -> bool {
    if !c.is_ascii() {
        return if matches!(class, C::Id | C::Menu) {
            !space(c)
        } else {
            true
        };
    }
    match class {
        C::ActorStart => !"/\\+()<->:\n,;".contains(c),
        C::ActorRest => !"+<->:\n,;".contains(c),
        C::Id => !"<>:\n,;@".contains(c) && !space(c),
        C::IdWide => !"<>:\n,;@".contains(c),
        C::Line => !"#\n;".contains(c),
        C::Menu => !"/\\+()<>:\n,;-".contains(c) && !space(c),
    }
}
struct Scanner<'a> {
    src: &'a str,
    p: usize,
    state: S,
    stack: Vec<S>,
    out: Vec<Token>,
    next_nl: usize,
}
impl Scanner<'_> {
    fn ch(&self, q: usize) -> Option<char> {
        self.src.get(q..)?.chars().next()
    }
    fn ci(&self, q: usize, w: &str) -> bool {
        self.src
            .get(q..q + w.len())
            .is_some_and(|s| s.eq_ignore_ascii_case(w))
    }
    fn word(&self, w: &str) -> bool {
        self.ci(self.p, w) && !word(self.ch(self.p + w.len()))
    }
    fn emit(&mut self, t: T, end: usize) {
        self.emit_text(t, end, self.src[self.p..end].into())
    }
    fn emit_text(&mut self, t: T, end: usize, text: String) {
        self.out.push(Token {
            kind: t,
            start: self.p,
            text,
        });
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
    fn eol(&mut self, q: usize) -> usize {
        if self.next_nl < q {
            self.next_nl = self.src[q..].find('\n').map_or(self.src.len(), |i| i + q)
        }
        self.next_nl
    }
    fn run(&self, q: usize, pred: impl Fn(char) -> bool) -> usize {
        let mut q = q;
        while let Some(c) = self.ch(q) {
            if !pred(c) {
                break;
            }
            q += c.len_utf8()
        }
        q
    }
    fn skip_line(&mut self) -> bool {
        let q = self.run(self.p, |c| c != '\n' && space(c));
        if q == self.p {
            false
        } else {
            self.p = q;
            true
        }
    }
    fn line_run(&self, q: usize) -> usize {
        self.run(q, |c| has(c, C::Line))
    }
    fn begin(&mut self, t: T, len: usize, s: S) {
        self.emit(t, self.p + len);
        self.push(s)
    }
    fn keyword(&mut self, w: &str, t: T, s: Option<S>) -> bool {
        if !self.word(w) {
            return false;
        }
        self.emit(t, self.p + w.len());
        if let Some(s) = s {
            self.push(s)
        }
        true
    }
    fn menu(&mut self, w: &str, t: T) -> bool {
        if !self.ci(self.p, w) {
            return false;
        }
        let q = self.p + w.len();
        if !matches!(self.ch(q), Some(' ' | '\t')) {
            return false;
        }
        let q = self.run(q, |c| matches!(c, ' ' | '\t'));
        if !self.ch(q).is_some_and(|c| has(c, C::Menu)) {
            return false;
        }
        self.emit(t, self.p + w.len());
        true
    }
    fn arrow_ahead(&self, q: usize) -> bool {
        let c = self.ch(q);
        let d = self.ch(q + 1);
        if c == Some('-') {
            if matches!(d, Some('x' | 'X' | ')' | '\\' | '/' | '-')) {
                return true;
            }
            return d == Some('|') && matches!(self.ch(q + 2), Some('\\' | '/'));
        }
        if matches!(c, Some('/' | '\\')) {
            return (d == Some('|') || d == c) && self.ch(q + 2) == Some('-');
        }
        c == Some('(') && d == Some(')')
    }
    fn actor(&mut self) {
        let mut q = self.run(self.p + self.ch(self.p).unwrap().len_utf8(), |c| {
            has(c, C::ActorStart)
        });
        while q < self.src.len() && !self.arrow_ahead(q) {
            let r = self.run(q, |c| c == '-');
            let Some(c) = self.ch(r).filter(|c| has(*c, C::ActorRest)) else {
                break;
            };
            q = self.run(r + c.len_utf8(), |c| has(c, C::ActorRest));
        }
        self.emit_text(T::ACTOR, q, self.src[self.p..q].trim_matches(space).into())
    }
    fn number(&mut self) -> bool {
        let q = self.run(self.p, |c| c.is_ascii_digit());
        let whole = q > self.p;
        if self.ch(q) == Some('.') && self.ch(q + 1).is_some_and(|c| c.is_ascii_digit()) {
            let r = if self.ch(q + 2).is_some_and(|c| c.is_ascii_digit()) {
                q + 3
            } else {
                q + 2
            };
            if matches!(self.ch(r), Some(' ' | '\n')) {
                self.emit(T::NUM, r);
                return true;
            }
        }
        if whole && matches!(self.ch(q), Some(' ' | '\n')) {
            self.emit(T::NUM, q);
            return true;
        }
        false
    }
    fn dash(&mut self) {
        for (s, t) in [
            ("-->>", T::DOTTED_ARROW),
            ("-->", T::DOTTED_OPEN_ARROW),
            ("--x", T::DOTTED_CROSS),
            ("--X", T::DOTTED_CROSS),
            ("--)", T::DOTTED_POINT),
            ("--|\\", T::SOLID_ARROW_TOP_DOTTED),
            ("--|/", T::SOLID_ARROW_BOTTOM_DOTTED),
            ("--\\\\", T::STICK_ARROW_TOP_DOTTED),
            ("--//", T::STICK_ARROW_BOTTOM_DOTTED),
            ("->>", T::SOLID_ARROW),
            ("->", T::SOLID_OPEN_ARROW),
            ("-x", T::SOLID_CROSS),
            ("-X", T::SOLID_CROSS),
            ("-)", T::SOLID_POINT),
            ("-|\\", T::SOLID_ARROW_TOP),
            ("-|/", T::SOLID_ARROW_BOTTOM),
            ("-\\\\", T::STICK_ARROW_TOP),
            ("-//", T::STICK_ARROW_BOTTOM),
        ] {
            if self.src[self.p..].starts_with(s) {
                self.emit(t, self.p + s.len());
                return;
            }
        }
        self.emit(T::MINUS, self.p + 1)
    }
    fn reverse(&mut self, c: char) -> bool {
        let d = self.ch(self.p + 1);
        if (d != Some('|') && d != Some(c)) || self.ch(self.p + 2) != Some('-') {
            return false;
        }
        let dotted = self.ch(self.p + 3) == Some('-');
        let top = c == '/';
        let t = match (d == Some('|'), dotted, top) {
            (true, true, true) => T::SOLID_ARROW_TOP_REVERSE_DOTTED,
            (true, true, false) => T::SOLID_ARROW_BOTTOM_REVERSE_DOTTED,
            (true, false, true) => T::SOLID_ARROW_TOP_REVERSE,
            (true, false, false) => T::SOLID_ARROW_BOTTOM_REVERSE,
            (false, true, true) => T::STICK_ARROW_TOP_REVERSE_DOTTED,
            (false, true, false) => T::STICK_ARROW_BOTTOM_REVERSE_DOTTED,
            (false, false, true) => T::STICK_ARROW_TOP_REVERSE,
            (false, false, false) => T::STICK_ARROW_BOTTOM_REVERSE,
        };
        self.emit(t, self.p + if dotted { 4 } else { 3 });
        true
    }
    fn title(&mut self) -> bool {
        if !self.ci(self.p, "title") {
            return false;
        }
        let mut q = self.p + 5;
        let legacy = self.ch(q) == Some(':');
        if legacy {
            q += 1
        }
        let Some(c) = self.ch(q).filter(|c| space(*c)) else {
            return false;
        };
        let next = q + c.len_utf8();
        if !self.ch(next).is_some_and(|c| has(c, C::Line)) {
            return false;
        }
        self.emit(
            if legacy { T::legacy_title } else { T::title },
            self.line_run(next),
        );
        true
    }
    fn acc(&mut self) -> bool {
        let descr = self.ci(self.p, "accdescr");
        if !descr && !self.ci(self.p, "acctitle") {
            return false;
        }
        let q = self.run(self.p + 8, space);
        if self.ch(q) == Some(':') {
            self.emit(
                if descr { T::acc_descr } else { T::acc_title },
                self.run(q + 1, space),
            );
            self.push(if descr { S::Descr } else { S::Title });
            return true;
        }
        if descr && self.ch(q) == Some('{') {
            self.p = self.run(q + 1, space);
            self.push(S::Multiline);
            return true;
        }
        false
    }
    fn keywords(&mut self, c: char) -> bool {
        let no = None;
        match c.to_ascii_lowercase() {
            'a' => {
                self.keyword("actor", T::participant_actor, Some(S::Id))
                    || self.keyword("alt", T::alt, Some(S::Line))
                    || self.keyword("and", T::and, Some(S::Line))
                    || self.keyword("activate", T::activate, Some(S::Id))
                    || self.acc()
                    || self.keyword("autonumber", T::autonumber, no)
            }
            'b' => {
                self.keyword("box", T::r#box, Some(S::Line))
                    || self.keyword("break", T::r#break, Some(S::Line))
            }
            'c' => {
                self.keyword("create", T::create, no)
                    || self.keyword("critical", T::critical, Some(S::Line))
            }
            'd' => {
                self.keyword("destroy", T::destroy, Some(S::Id))
                    || self.menu("details", T::details)
                    || self.keyword("deactivate", T::deactivate, Some(S::Id))
            }
            'e' => {
                self.keyword("else", T::r#else, Some(S::Line)) || self.keyword("end", T::end, no)
            }
            'l' => {
                self.keyword("loop", T::r#loop, Some(S::Line))
                    || self.keyword("left of", T::left_of, no)
                    || self.menu("links", T::links)
                    || self.menu("link", T::link)
            }
            'n' => self.keyword("note", T::note, no),
            'o' => {
                self.keyword("opt", T::opt, Some(S::Line))
                    || self.keyword("option", T::option, Some(S::Line))
                    || self.keyword("over", T::over, no)
                    || self.keyword("off", T::off, no)
            }
            'p' => {
                self.keyword("participant", T::participant, Some(S::Id))
                    || self.keyword("par", T::par, Some(S::Line))
                    || self.keyword("par_over", T::par_over, Some(S::Line))
                    || self.menu("properties", T::properties)
            }
            'r' => {
                self.keyword("rect", T::rect, Some(S::Line))
                    || self.keyword("right of", T::right_of, no)
            }
            's' => self.keyword("sequencediagram", T::SD, no),
            't' => self.title(),
            _ => false,
        }
    }
    fn initial(&mut self) {
        let c = self.ch(self.p).unwrap();
        if c == '\n' {
            self.emit(T::NEWLINE, self.run(self.p + 1, |c| c == '\n'));
            return;
        }
        if space(c) {
            self.p = self.run(self.p + c.len_utf8(), space);
            return;
        }
        if c == '#' || c == '%' && self.ch(self.p + 1) != Some('{') {
            self.p = self.eol(self.p);
            return;
        }
        let next = self.p + c.len_utf8();
        if c != '}' && self.src[next..].starts_with("%%") {
            self.p = self.eol(self.p);
            return;
        }
        if (c.is_ascii_digit() || c == '.') && self.number() {
            return;
        }
        if c.is_ascii_alphabetic() && self.keywords(c) {
            return;
        }
        if c == ',' {
            self.emit(T::COMMA, self.p + 1)
        } else if c == ';' {
            self.emit(T::NEWLINE, self.p + 1)
        } else if has(c, C::ActorStart) {
            self.actor()
        } else if c == '-' {
            self.dash()
        } else if self.src[self.p..].starts_with("<<->>") {
            self.emit(T::BIDIRECTIONAL_SOLID_ARROW, self.p + 5)
        } else if self.src[self.p..].starts_with("<<-->>") {
            self.emit(T::BIDIRECTIONAL_DOTTED_ARROW, self.p + 6)
        } else if matches!(c, '/' | '\\') && self.reverse(c) {
        } else if c == ':' {
            self.emit(T::TXT, self.line_run(self.p + 1))
        } else if c == '+' {
            self.emit(T::PLUS, self.p + 1)
        } else if self.src[self.p..].starts_with("()") {
            self.emit(T::CENTRAL, self.p + 2)
        } else {
            self.emit(T::INVALID, self.p + c.len_utf8())
        }
    }
    fn id(&mut self) -> bool {
        if self.skip_line() {
            return true;
        }
        let c = self.ch(self.p).unwrap();
        if c == '#' {
            self.p = self.eol(self.p);
            return true;
        }
        if self.src[self.p..].starts_with("@{") {
            self.begin(T::CONFIG_START, 2, S::Config);
            return true;
        }
        let a = self.run(self.p, |c| has(c, C::Id));
        if a > self.p {
            let w = self.run(a, space);
            if self.src[w..].starts_with("@{") {
                self.emit(T::ACTOR, a);
                return true;
            }
            if w > a && self.ci(w, "as") && self.ch(w + 2).is_some_and(space) {
                self.emit(T::ACTOR, a);
                self.push(S::Alias);
                return true;
            }
        }
        let b = self.run(a, |c| has(c, C::IdWide));
        if b > self.p {
            let mut cut = b;
            if b < self.src.len() && !matches!(self.ch(b), Some('\n' | ';')) {
                cut = self.src[..b].char_indices().next_back().unwrap().0;
                while cut > self.p && self.ch(cut) != Some('#') {
                    cut = self.src[..cut].char_indices().next_back().unwrap().0;
                }
            }
            if cut > self.p {
                self.emit_text(
                    T::ACTOR,
                    cut,
                    self.src[self.p..cut].trim_matches(space).into(),
                );
                self.pop();
                return true;
            }
        }
        if self.ch(b) == Some('<') {
            let e = self.eol(self.p);
            self.emit(T::INVALID, e);
            self.pop();
            return true;
        }
        if c != '\n' {
            let e = self.eol(self.p);
            self.emit_text(
                T::INVALID,
                e,
                self.src[self.p..e].trim_matches(space).into(),
            );
            self.pop();
            return true;
        }
        false
    }
    fn step(&mut self) -> bool {
        match self.state {
            S::Initial => {
                self.initial();
                true
            }
            S::Id => self.id(),
            S::Alias => {
                if self.skip_line() {
                    return true;
                }
                if self.ch(self.p) == Some('#') {
                    self.p = self.eol(self.p);
                    return true;
                }
                self.pop();
                self.pop();
                if self.word("as") {
                    self.begin(T::AS, 2, S::Line)
                } else {
                    self.emit(T::NEWLINE, self.p)
                }
                true
            }
            S::Line => {
                if self.skip_line() {
                    return true;
                }
                if self.ch(self.p) == Some('#') {
                    self.p = self.eol(self.p);
                    return true;
                }
                self.emit(T::restOfLine, self.line_run(self.p));
                self.pop();
                true
            }
            S::Config => {
                if self.ch(self.p) != Some('}') {
                    let e = self.src[self.p..]
                        .find('}')
                        .map_or(self.src.len(), |i| i + self.p);
                    self.emit(T::CONFIG_CONTENT, e)
                } else {
                    let w = self.run(self.p + 1, space);
                    let alias =
                        w > self.p + 1 && self.ci(w, "as") && self.ch(w + 2).is_some_and(space);
                    self.emit(T::CONFIG_END, self.p + 1);
                    self.pop();
                    if alias {
                        self.push(S::Alias)
                    } else {
                        self.pop()
                    }
                }
                true
            }
            S::Title | S::Descr => {
                let e = self.eol(self.p);
                self.emit(
                    if self.state == S::Title {
                        T::acc_title_value
                    } else {
                        T::acc_descr_value
                    },
                    e,
                );
                self.pop();
                true
            }
            S::Multiline => {
                if self.ch(self.p) == Some('}') {
                    self.p += 1;
                    self.pop()
                } else {
                    let e = self.src[self.p..]
                        .find('}')
                        .map_or(self.src.len(), |i| i + self.p);
                    self.emit(T::acc_descr_multiline_value, e)
                }
                true
            }
        }
    }
}
pub(super) fn tokenize(src: &str) -> Vec<Token> {
    let mut s = Scanner {
        src,
        p: 0,
        state: S::Initial,
        stack: vec![],
        out: vec![],
        next_nl: 0,
    };
    while s.p < src.len() {
        if !s.step() {
            s.out.push(Token {
                kind: T::ERROR,
                text: String::new(),
                start: s.p,
            });
            return s.out;
        }
    }
    let t = match s.state {
        S::Initial | S::Alias => Some(T::NEWLINE),
        S::Line => Some(T::restOfLine),
        S::Title => Some(T::acc_title_value),
        S::Descr => Some(T::acc_descr_value),
        S::Multiline => Some(T::acc_descr_multiline_value),
        _ => None,
    };
    if let Some(t) = t {
        s.emit(t, src.len())
    }
    s.emit(T::END, src.len());
    s.out
}
