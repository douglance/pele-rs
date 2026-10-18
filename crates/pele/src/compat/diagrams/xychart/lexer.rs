//! Direct Pele xychart/lexer.ts state stack scanner, 458322a5 (MIT, kepano).
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
    AxisData,
    AxisBand,
    Data,
    DataInner,
    String,
    Title,
    Descr,
    Multiline,
}
impl S {
    fn at_end(self) -> T {
        match self {
            Self::String => T::STR,
            Self::Title => T::acc_title_value,
            Self::Descr => T::acc_descr_value,
            Self::Multiline => T::acc_descr_multiline_value,
            _ => T::EOF,
        }
    }
}
fn word(c: Option<char>) -> bool {
    c.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
}
struct Scanner<'a> {
    src: &'a str,
    p: usize,
    state: S,
    stack: Vec<S>,
    out: Vec<Token>,
}
impl Scanner<'_> {
    fn ch(&self, p: usize) -> Option<char> {
        self.src.get(p..)?.chars().next()
    }
    fn word_at(&self, p: usize, w: &str) -> bool {
        self.src
            .get(p..p + w.len())
            .is_some_and(|s| s.eq_ignore_ascii_case(w))
    }
    fn kw(&self, w: &str) -> bool {
        self.word_at(self.p, w) && !word(self.ch(self.p + w.len()))
    }
    fn emit(&mut self, t: T, e: usize) {
        self.out.push(Token {
            kind: t,
            text: self.src[self.p..e].into(),
            start: self.p,
        });
        self.p = e;
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
    fn run(&self, p: usize, pred: impl Fn(char) -> bool) -> usize {
        let mut q = p;
        while let Some(c) = self.ch(q) {
            if !pred(c) {
                break;
            }
            q += c.len_utf8()
        }
        q
    }
    fn line_end(&self, p: usize) -> usize {
        self.src[p..].find('\n').map_or(self.src.len(), |i| i + p)
    }
    fn scan(&mut self) -> bool {
        let c = self.ch(self.p).unwrap();
        if self.state == S::String {
            if c == '"' {
                self.p += 1;
                self.pop()
            } else {
                let e = self.src[self.p..]
                    .find('"')
                    .map_or(self.src.len(), |i| i + self.p);
                self.emit(T::STR, e)
            }
            return true;
        }
        if matches!(self.state, S::Title | S::Descr) {
            self.emit(self.state.at_end(), self.line_end(self.p));
            self.pop();
            return true;
        }
        if self.state == S::Multiline {
            if c == '}' {
                self.p += 1;
                self.pop()
            } else {
                let e = self.src[self.p..]
                    .find('}')
                    .map_or(self.src.len(), |i| i + self.p);
                self.emit(T::acc_descr_multiline_value, e)
            }
            return true;
        }
        let after = self.p + c.len_utf8();
        let next = self.ch(after);
        if c == '%' && next == Some('%') {
            self.p = self.line_end(self.p + 2);
            return true;
        }
        if c != '}' && next == Some('%') && self.ch(after + 1) == Some('%') {
            self.p = self.line_end(after + 2);
            return true;
        }
        if matches!(c, '\n' | '\r') {
            if matches!(self.state, S::AxisData | S::Data) && (c == '\n' || next == Some('\n')) {
                self.emit(T::NEWLINE, self.p + if c == '\n' { 1 } else { 2 });
                self.pop()
            } else {
                self.emit(T::NEWLINE, self.run(after, |c| matches!(c, '\n' | '\r')))
            }
            return true;
        }
        if self.kw("title") {
            self.emit(T::title, self.p + 5);
            return true;
        }
        if self.word_at(self.p, "acc") {
            let descr = self.word_at(self.p + 3, "descr");
            if descr || self.word_at(self.p + 3, "title") {
                let k = self.run(self.p + 8, space);
                if self.ch(k) == Some(':') {
                    let e = self.run(k + 1, space);
                    self.emit(if descr { T::acc_descr } else { T::acc_title }, e);
                    self.push(if descr { S::Descr } else { S::Title });
                    return true;
                }
                if self.ch(k) == Some('{') && descr {
                    self.p = self.run(k + 1, space);
                    self.push(S::Multiline);
                    return true;
                }
            }
        }
        for (w, t, s, bound) in [
            ("xychart-beta", T::XYCHART, None, true),
            ("xychart", T::XYCHART, None, true),
            ("x-axis", T::X_AXIS, Some(S::AxisData), true),
            ("y-axis", T::Y_AXIS, Some(S::AxisData), true),
            ("vertical", T::CHART_ORIENTATION, None, false),
            ("horizontal", T::CHART_ORIENTATION, None, false),
            ("line", T::LINE, Some(S::Data), true),
            ("bar", T::BAR, Some(S::Data), true),
        ] {
            if if bound {
                self.kw(w)
            } else {
                self.word_at(self.p, w)
            } {
                self.emit(t, self.p + w.len());
                if let Some(s) = s {
                    self.push(s)
                }
                return true;
            }
        }
        if c == '[' {
            if self.state == S::AxisData {
                self.push(S::AxisBand)
            } else if self.state == S::Data {
                self.push(S::DataInner)
            }
            self.emit(T::SQUARE_BRACES_START, self.p + 1);
            return true;
        }
        if c == ']' {
            if matches!(self.state, S::DataInner | S::AxisBand) {
                self.pop()
            }
            self.emit(T::SQUARE_BRACES_END, self.p + 1);
            return true;
        }
        if self.state == S::AxisData && self.src[self.p..].starts_with("-->") {
            self.emit(T::ARROW_DELIMITER, self.p + 3);
            return true;
        }
        if matches!(self.state, S::AxisData | S::DataInner) {
            let q = if matches!(c, '+' | '-') {
                self.p + 1
            } else {
                self.p
            };
            let d = self.ch(q);
            let end = if d.is_some_and(|c| c.is_ascii_digit()) {
                let mut e = self.run(q + 1, |c| c.is_ascii_digit());
                if self.ch(e) == Some('.') && self.ch(e + 1).is_some_and(|c| c.is_ascii_digit()) {
                    e = self.run(e + 2, |c| c.is_ascii_digit())
                }
                Some(e)
            } else if d == Some('.') && self.ch(q + 1).is_some_and(|c| c.is_ascii_digit()) {
                Some(self.run(q + 2, |c| c.is_ascii_digit()))
            } else {
                None
            };
            if let Some(e) = end {
                self.emit(T::NUMBER_WITH_DECIMAL, e);
                return true;
            }
        }
        if c == '`' {
            let broken=format!("`){}{{ this.pushstate(md_string); }}\n<md_string>(?:(?!`\").)+{}{{ return md_str; }}\n<md_string>(?:`"," ".repeat(36)," ".repeat(18));
            if self.word_at(self.p, &broken) {
                self.p += broken.len();
                self.pop();
                return true;
            }
        }
        if c == '"' {
            self.p += 1;
            self.push(S::String);
            return true;
        }
        if c.is_ascii_alphabetic() {
            self.emit(T::ALPHA, self.run(after, |c| c.is_ascii_alphabetic()));
            return true;
        }
        if c.is_ascii_digit() {
            self.emit(T::NUM, self.run(after, |c| c.is_ascii_digit()));
            return true;
        }
        if space(c) {
            self.p = self.run(after, space);
            return true;
        }
        let t = match c {
            ':' => T::COLON,
            '+' => T::PLUS,
            ',' => T::COMMA,
            '=' => T::EQUALS,
            '*' => T::MULT,
            '#' => T::BRKT,
            '_' => T::UNDERSCORE,
            '.' => T::DOT,
            '&' => T::AMP,
            '-' => T::MINUS,
            ';' => T::SEMI,
            _ => return false,
        };
        self.emit(t, after);
        true
    }
}
pub(super) fn tokenize(src: &str) -> Vec<Token> {
    let mut s = Scanner {
        src,
        p: 0,
        state: S::Initial,
        stack: vec![],
        out: vec![],
    };
    while s.p < src.len() {
        if !s.scan() {
            s.out.push(Token {
                kind: T::ERROR,
                text: String::new(),
                start: s.p,
            });
            return s.out;
        }
    }
    let end = s.state.at_end();
    s.emit(end, src.len());
    s.emit(T::END, src.len());
    s.out
}
