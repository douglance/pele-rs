//! Lazy Venn Jison lexer; parser-owned indentation changes its next token, Pele 458322a5 (MIT).
use super::model::VennModel;
use crate::compat::source::space;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub(super) enum T {
    End,
    Error,
    Eof,
    Newline,
    Venn,
    Title,
    Set,
    Union,
    Text,
    IndentText,
    Style,
    BracketLabel,
    Numeric,
    HexColor,
    RgbColor,
    RgbaColor,
    Identifier,
    String,
    Comma,
    Colon,
}
pub(super) const NAMES: [&str; 20] = [
    "$end",
    "INVALID",
    "EOF",
    "NEWLINE",
    "VENN",
    "TITLE",
    "SET",
    "UNION",
    "TEXT",
    "INDENT_TEXT",
    "STYLE",
    "BRACKET_LABEL",
    "NUMERIC",
    "HEXCOLOR",
    "RGBCOLOR",
    "RGBACOLOR",
    "IDENTIFIER",
    "STRING",
    "COMMA",
    "COLON",
];
fn word(c: Option<u8>) -> bool {
    c.is_some_and(|c| c.is_ascii_alphanumeric() || c == b'_')
}
pub(super) struct Lexer<'a> {
    pub src: &'a str,
    pub text: String,
    pub start: usize,
    at: usize,
    line_start: bool,
    ended: bool,
}
impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Self {
            src,
            text: String::new(),
            start: 0,
            at: 0,
            line_start: false,
            ended: false,
        }
    }
    fn word(&self, at: usize, w: &str) -> bool {
        self.src
            .as_bytes()
            .get(at..at + w.len())
            .is_some_and(|s| s.eq_ignore_ascii_case(w.as_bytes()))
    }
    fn keyword(&self, at: usize, w: &str) -> bool {
        self.word(at, w) && !word(self.src.as_bytes().get(at + w.len()).copied())
    }
    fn line_end(&self, p: usize) -> usize {
        self.src[p..].find('\n').map_or(self.src.len(), |i| p + i)
    }
    fn color(&self, at: usize, parts: usize) -> Option<usize> {
        let mut i = at + 1;
        for part in 0..parts {
            while self.src[i..].chars().next().is_some_and(space) {
                i += self.src[i..].chars().next().unwrap().len_utf8()
            }
            let from = i;
            while self
                .src
                .as_bytes()
                .get(i)
                .is_some_and(|c| c.is_ascii_digit() || *c == b'.')
            {
                i += 1
            }
            if i == from {
                return None;
            }
            while self.src[i..].chars().next().is_some_and(space) {
                i += self.src[i..].chars().next().unwrap().len_utf8()
            }
            if self.src.as_bytes().get(i) != Some(&if part == parts - 1 { b')' } else { b',' }) {
                return None;
            }
            i += 1;
        }
        Some(i)
    }
    fn token(&mut self, t: T, end: usize) -> T {
        self.text = self.src[self.at..end].into();
        self.at = end;
        t
    }
    pub fn next(&mut self, db: &mut VennModel) -> T {
        let src = self.src;
        let bytes = src.as_bytes();
        let n = src.len();
        loop {
            let mut i = self.at;
            self.start = i;
            self.text.clear();
            if i >= n {
                if self.ended {
                    return T::End;
                }
                self.ended = true;
                return T::Eof;
            }
            let ch = src[i..].chars().next().unwrap();
            let c = bytes[i];
            let next = i + ch.len_utf8();
            if (c == b'%' && bytes.get(i + 1) == Some(&b'%') && bytes.get(i + 2) != Some(&b'{'))
                || (ch != '}' && ch.len_utf16() == 1 && src[next..].starts_with("%%"))
            {
                self.at = self.line_end(if c == b'%' { i + 2 } else { next + 1 });
                continue;
            }
            if self.line_start {
                if matches!(c, b' ' | b'\t') {
                    let mut j = i + 1;
                    while bytes.get(j).is_some_and(|c| matches!(c, b' ' | b'\t')) {
                        j += 1
                    }
                    self.at = j;
                    if self.keyword(j, "text") && db.indent {
                        self.at = j + 4;
                        self.line_start = false;
                        self.text = src[i..j].into();
                        return T::IndentText;
                    }
                    continue;
                }
                if !matches!(c, b'\n' | b'\r') {
                    db.indent = false;
                    self.line_start = false;
                }
            }
            if matches!(c, b'\n' | b'\r') {
                i += 1;
                while bytes.get(i).is_some_and(|c| matches!(c, b'\n' | b'\r')) {
                    i += 1
                }
                self.line_start = true;
                return self.token(T::Newline, i);
            }
            if src[i..].starts_with("%%") {
                self.at = self.line_end(i);
                continue;
            }
            if matches!(c, b' ' | b'\t') {
                i += 1;
                while bytes.get(i).is_some_and(|c| matches!(c, b' ' | b'\t')) {
                    i += 1
                }
                self.at = i;
                continue;
            }
            if c.is_ascii_alphabetic() || c == b'_' {
                if self.word(i, "title")
                    && src
                        .get(i + 5..)
                        .and_then(|s| s.chars().next())
                        .is_some_and(space)
                {
                    let first = src[i + 5..].chars().next().unwrap();
                    let mut end = i + 5 + first.len_utf8();
                    let after = end;
                    while end < n && !matches!(bytes[end], b'#' | b'\n' | b';') {
                        end += src[end..].chars().next().unwrap().len_utf8()
                    }
                    if end > after {
                        return self.token(T::Title, end);
                    }
                }
                for (w, t) in [
                    ("venn-beta", T::Venn),
                    ("set", T::Set),
                    ("union", T::Union),
                    ("text", T::Text),
                    ("style", T::Style),
                ] {
                    if self.keyword(i, w) {
                        return self.token(t, i + w.len());
                    }
                }
                if self.word(i, "rgb") {
                    let alpha = bytes
                        .get(i + 3)
                        .is_some_and(|c| c.eq_ignore_ascii_case(&b'a'));
                    let open = i + if alpha { 4 } else { 3 };
                    if bytes.get(open) == Some(&b'(') {
                        if let Some(end) = self.color(open, if alpha { 4 } else { 3 }) {
                            return self.token(if alpha { T::RgbaColor } else { T::RgbColor }, end);
                        }
                    }
                }
                i += 1;
                while word(bytes.get(i).copied()) || bytes.get(i) == Some(&b'-') {
                    i += 1
                }
                return self.token(T::Identifier, i);
            }
            if c == b'[' {
                if bytes.get(i + 1) == Some(&b'"') {
                    let Some(quote) = src[i + 2..].find('"').map(|p| i + p + 2) else {
                        return T::Error;
                    };
                    if bytes.get(quote + 1) != Some(&b']') {
                        return T::Error;
                    }
                    self.at = quote + 2;
                    self.text = src[i + 2..quote].into();
                    return T::BracketLabel;
                }
                let mut end = i + 1;
                while end < n && !matches!(bytes[end], b']' | b'"') {
                    end += src[end..].chars().next().unwrap().len_utf8()
                }
                if end == i + 1 || bytes.get(end) != Some(&b']') {
                    return T::Error;
                }
                self.at = end + 1;
                self.text = src[i + 1..end].trim_matches(space).into();
                return T::BracketLabel;
            }
            if c.is_ascii_digit() || matches!(c, b'.' | b'+' | b'-') {
                if matches!(c, b'+' | b'-') {
                    i += 1
                }
                let from = i;
                while bytes.get(i).is_some_and(u8::is_ascii_digit) {
                    i += 1
                }
                if i > from {
                    if bytes.get(i) == Some(&b'.')
                        && bytes.get(i + 1).is_some_and(u8::is_ascii_digit)
                    {
                        i += 2;
                        while bytes.get(i).is_some_and(u8::is_ascii_digit) {
                            i += 1
                        }
                    }
                } else if bytes.get(i) == Some(&b'.')
                    && bytes.get(i + 1).is_some_and(u8::is_ascii_digit)
                {
                    i += 2;
                    while bytes.get(i).is_some_and(u8::is_ascii_digit) {
                        i += 1
                    }
                } else {
                    return T::Error;
                }
                return self.token(T::Numeric, i);
            }
            if c == b'#' {
                let mut end = i + 1;
                while end < i + 9 && bytes.get(end).is_some_and(u8::is_ascii_hexdigit) {
                    end += 1
                }
                return if end >= i + 4 {
                    self.token(T::HexColor, end)
                } else {
                    T::Error
                };
            }
            if c == b'"' {
                return if let Some(quote) = src[i + 1..].find('"').map(|p| p + i + 1) {
                    self.token(T::String, quote + 1)
                } else {
                    T::Error
                };
            }
            if c == b',' {
                return self.token(T::Comma, i + 1);
            }
            if c == b':' {
                return self.token(T::Colon, i + 1);
            }
            return T::Error;
        }
    }
}
