//! Pele usecase/lexer.ts, 458322a5 (MIT, kepano). UTF-16 positions and actual token priority.
use super::tokens::T;
use crate::compat::{ErrorCode, PeleError};
pub(super) struct Source {
    pub units: Vec<u16>,
}
impl Source {
    pub fn new(s: &str) -> Self {
        Self {
            units: s.encode_utf16().collect(),
        }
    }
    pub fn c(&self, i: usize) -> u16 {
        *self.units.get(i).unwrap_or(&0xffff)
    }
    pub fn len(&self) -> usize {
        self.units.len()
    }
    pub fn text(&self, a: usize, b: usize) -> String {
        String::from_utf16_lossy(&self.units[a.min(self.len())..b.min(self.len())])
    }
    pub fn starts(&self, i: usize, s: &str) -> bool {
        self.units
            .get(i..i.saturating_add(s.len()))
            .is_some_and(|u| u.iter().zip(s.bytes()).all(|(a, b)| *a == b as u16))
    }
    fn find(&self, i: usize, s: &str) -> Option<usize> {
        (i..self.len()).find(|&k| self.starts(k, s))
    }
    pub fn loc(&self, offset: usize) -> (usize, usize) {
        let mut line = 1;
        let mut start = 0;
        let mut i = 0;
        while i < offset {
            let c = self.c(i);
            if c == 10 || c == 13 {
                if c == 13 && i + 1 < offset && self.c(i + 1) == 10 {
                    i += 1
                }
                line += 1;
                start = i + 1
            }
            i += 1
        }
        (line, offset - start + 1)
    }
    pub fn location(&self, span: [usize; 2]) -> String {
        let (l, c) = self.loc(span[0]);
        format!("line {l}, column {c} [{},{})", span[0], span[1])
    }
}
#[derive(Clone, Copy, Debug)]
pub(super) struct Token {
    pub kind: T,
    pub start: usize,
    pub end: usize,
}
fn word(c: u16) -> bool {
    c < 128 && (c as u8).is_ascii_alphanumeric() || c == 95
}
fn digit(c: u16) -> bool {
    (48..=57).contains(&c)
}
fn letter(c: u16) -> bool {
    (65..=90).contains(&c) || (97..=122).contains(&c)
}
fn blank(c: u16) -> bool {
    c == 32 || c == 9
}
fn keyword(s: &Source, i: usize) -> Option<(T, usize)> {
    use T::*;
    for (name, t) in [
        ("actor", ACTOR),
        ("systemBoundary", SYSTEM_BOUNDARY),
        ("end", END),
        ("extend", EXTEND),
        ("direction", DIRECTION),
        ("TD", TD),
        ("TB", TB),
        ("BT", BT),
        ("LR", LR),
        ("RL", RL),
        ("note", NOTE),
        ("for", FOR),
        ("json", JSON),
        ("classDef", CLASS_DEF),
        ("class", CLASS),
        ("style", STYLE),
        ("include", INCLUDE),
        ("true", TRUE),
        ("false", FALSE),
        ("usecase-beta", USECASE),
    ] {
        let yes = if t == INCLUDE || t == EXTEND {
            (0..name.len()).all(|k| s.c(i + k) | 32 == name.as_bytes()[k] as u16)
        } else {
            s.starts(i, name)
        };
        if yes {
            return Some((t, name.len()));
        }
    }
    None
}
fn string_end(s: &Source, i: usize, q: u16) -> Option<usize> {
    for k in i + 1..s.len() {
        let c = s.c(k);
        if c == q {
            return Some(k + 1);
        }
        if c == 10 || c == 13 {
            return None;
        }
    }
    None
}
fn starts_token(s: &Source, i: usize) -> bool {
    let c = s.c(i);
    c != 34 && c != 39 || (c == 34 && s.c(i + 1) == 96) || string_end(s, i, c).is_some()
}
fn error(s: &Source, i: usize, length: usize) -> PeleError {
    let (line, column) = s.loc(i);
    let mut e=PeleError::new(format!("Error lexing usecase diagram: unexpected character: ->{}<- at offset: {i}, skipped {length} characters. at line {line}, column {column} [{i},{})",s.text(i,i+1),i+length),ErrorCode::Syntax);
    e.diagram_type = Some("usecase".into());
    e.line = line;
    e.column = column;
    e
}
pub(super) fn tokenize(s: &Source) -> Result<Vec<Token>, PeleError> {
    use T::*;
    let n = s.len();
    let mut out = vec![];
    let mut i = 0;
    let mut line_start = true;
    let mut no_brace = false;
    while i < n {
        let c = s.c(i);
        if blank(c) {
            i += 1;
            continue;
        }
        let mut end = i + 1;
        let mut fresh = false;
        let mut t = None;
        if word(c) {
            if c == 97 && line_start && s.starts(i, "acc") {
                let descr = s.starts(i + 3, "Descr");
                if descr || s.starts(i + 3, "Title") {
                    let mut j = i + 8;
                    while blank(s.c(j)) {
                        j += 1
                    }
                    if s.c(j) == 58 {
                        t = Some(if descr {
                            ACC_DESCR_LINE
                        } else {
                            ACC_TITLE_LINE
                        });
                        end = j + 1;
                        while end < n && s.c(end) != 10 && s.c(end) != 13 {
                            end += 1
                        }
                    } else if s.c(j) == 123 && descr && !no_brace {
                        if let Some(close) = s.find(j + 1, "}") {
                            t = Some(ACC_DESCR_BLOCK);
                            end = close + 1
                        } else {
                            no_brace = true
                        }
                    }
                }
            } else if c == 106 && s.starts(i, "json") && blank(s.c(i + 4)) {
                let mut j = i + 5;
                while blank(s.c(j)) {
                    j += 1
                }
                let id = j;
                while word(s.c(j)) {
                    j += 1
                }
                if j > id {
                    while blank(s.c(j)) {
                        j += 1
                    }
                    if s.c(j) == 64 {
                        j += 1;
                        while blank(s.c(j)) {
                            j += 1
                        }
                        if s.c(j) == 123 {
                            out.push(Token {
                                kind: JSON_DECLARATION_START,
                                start: i,
                                end: j,
                            });
                            let mut depth = 0;
                            let mut quoted = false;
                            let mut escaped = false;
                            let mut k = j;
                            while k < n {
                                let ch = s.c(k);
                                if quoted {
                                    if escaped {
                                        escaped = false
                                    } else if ch == 92 {
                                        escaped = true
                                    } else if ch == 34 {
                                        quoted = false
                                    }
                                } else if ch == 34 {
                                    quoted = true
                                } else if ch == 123 {
                                    depth += 1
                                } else if ch == 125 {
                                    depth -= 1;
                                    if depth == 0 {
                                        break;
                                    }
                                }
                                k += 1
                            }
                            i = if k < n { k + 1 } else { n };
                            out.push(Token {
                                kind: if k < n {
                                    JSON_OBJECT_LITERAL
                                } else {
                                    UNCLOSED_JSON_OBJECT_LITERAL
                                },
                                start: j,
                                end: i,
                            });
                            line_start = false;
                            continue;
                        }
                    }
                }
            }
            if t.is_none() {
                while end < n && word(s.c(end)) {
                    end += 1
                }
                if let Some((k, l)) = keyword(s, i) {
                    if k == USECASE || end - i == l {
                        t = Some(k);
                        end = i + l
                    } else {
                        t = Some(IDENTIFIER)
                    }
                } else if (c == 111 || c == 120) && s.c(i + 1) == 45 && s.c(i + 2) == 45 {
                    t = Some(if c == 111 {
                        BACKWARD_CIRCLE
                    } else {
                        BACKWARD_CROSS
                    });
                    end = i + 3
                } else if digit(c) {
                    let mut digits = i + 1;
                    while digit(s.c(digits)) {
                        digits += 1
                    }
                    if s.c(digits) == 46 && digit(s.c(digits + 1)) {
                        t = Some(NUMBER);
                        end = digits + 2;
                        while digit(s.c(end)) {
                            end += 1
                        }
                        while letter(s.c(end)) {
                            end += 1
                        }
                    } else {
                        t = Some(IDENTIFIER)
                    }
                } else {
                    t = Some(IDENTIFIER);
                    while s.c(end) == 45 && word(s.c(end + 1)) {
                        t = Some(CSS_IDENTIFIER);
                        end += 2;
                        while word(s.c(end)) {
                            end += 1
                        }
                    }
                }
            }
        } else if c == 10 || c == 13 {
            t = Some(NEWLINE);
            fresh = true;
            if c == 13 && s.c(end) == 10 {
                end += 1
            }
        } else if c == 34 || c == 39 {
            if c == 34 && s.c(i + 1) == 96 {
                if let Some(close) = s.find(i + 2, "`\"") {
                    t = Some(MARKDOWN_STRING);
                    end = close + 2
                } else {
                    t = Some(UNCLOSED_MARKDOWN_STRING);
                    end = n
                }
            } else {
                t = Some(PLAIN_STRING);
                if let Some(e) = string_end(s, i, c) {
                    end = e
                } else {
                    let mut resume = i + 1;
                    while resume < n && !starts_token(s, resume) {
                        resume += 1
                    }
                    return Err(error(s, i, resume - i));
                }
            }
        } else if c == 37 {
            if line_start && s.c(end) == 37 {
                t = Some(COMMENT);
                while end < n && s.c(end) != 10 && s.c(end) != 13 {
                    end += 1
                }
            } else {
                t = Some(PERCENT)
            }
        } else if c == 45 {
            if s.c(end) != 45 {
                t = Some(DASH)
            } else {
                while s.c(end) == 45 {
                    end += 1
                }
                let next = s.c(end);
                let two = end - i == 2;
                if two && next == 124 && s.c(end + 1) == 62 {
                    t = Some(GENERALIZATION);
                    end += 2
                } else if next == 62 {
                    t = Some(FORWARD_SOLID);
                    end += 1
                } else if two && (next == 111 || next == 120) {
                    t = Some(if next == 111 {
                        FORWARD_CIRCLE
                    } else {
                        FORWARD_CROSS
                    });
                    end += 1
                } else {
                    t = Some(MARKERLESS_SOLID)
                }
            }
        } else if c == 46 {
            let next = s.c(end);
            if next == 46 && s.c(end + 1) == 62 {
                t = Some(DEPENDENCY_ARROW);
                end += 2
            } else if digit(next) {
                t = Some(NUMBER);
                while digit(s.c(end)) {
                    end += 1
                }
                while letter(s.c(end)) {
                    end += 1
                }
            } else {
                t = Some(DOT)
            }
        } else if c == 60 {
            if s.c(end) == 60 {
                out.push(Token {
                    kind: STEREOTYPE_START,
                    start: i,
                    end: i + 2,
                });
                i += 2;
                if i >= n {
                    break;
                }
                if s.c(i) == 62 && s.c(i + 1) == 62 {
                    t = Some(STEREOTYPE_END);
                    end = i + 2
                } else {
                    let mut close = i;
                    let mut found = false;
                    while close < n {
                        let ch = s.c(close);
                        if ch == 10 || ch == 13 {
                            break;
                        }
                        if ch == 62 && s.c(close + 1) == 62 {
                            found = true;
                            break;
                        }
                        close += 1
                    }
                    if found
                        && !s
                            .text(i, close)
                            .trim_matches(crate::compat::source::space)
                            .is_empty()
                    {
                        out.push(Token {
                            kind: STEREOTYPE_TEXT,
                            start: i,
                            end: close,
                        });
                        t = Some(STEREOTYPE_END);
                        i = close;
                        end = close + 2
                    } else {
                        while close < n && s.c(close) != 10 && s.c(close) != 13 {
                            close += 1
                        }
                        if close == i {
                            while close < n && (s.c(close) == 10 || s.c(close) == 13) {
                                close += 1
                            }
                            return Err(error(s, i, close - i));
                        }
                        t = Some(UNCLOSED_STEREOTYPE_TEXT);
                        end = close
                    }
                }
            } else if s.c(end) == 45 && s.c(end + 1) == 45 {
                t = Some(BACKWARD_SOLID);
                end += 2;
                while s.c(end) == 45 {
                    end += 1
                }
            } else {
                t = Some(LABEL_PUNCTUATION)
            }
        } else if c == 58 {
            if s.c(end) == 58 && s.c(end + 1) == 58 {
                t = Some(CLASS_SEPARATOR);
                end += 2
            } else {
                t = Some(COLON)
            }
        } else if c == 64 {
            if s.c(end) == 123 {
                t = Some(METADATA_START);
                end += 1
            } else {
                t = Some(AT)
            }
        } else if c == 123 {
            t = Some(LBRACE)
        } else if c == 125 {
            t = Some(RBRACE)
        } else if c == 91 {
            t = Some(LBRACKET)
        } else if c == 93 {
            t = Some(RBRACKET)
        } else if c == 40 {
            t = Some(LPAREN)
        } else if c == 41 {
            t = Some(RPAREN)
        } else if c == 44 {
            t = Some(COMMA)
        } else if c == 35 {
            while s.c(end) < 128 && (s.c(end) as u8).is_ascii_hexdigit() {
                end += 1
            }
            t = Some(if end > i + 1 {
                HASH_COLOR
            } else {
                CSS_PUNCTUATION
            })
        } else if c == 92 {
            if s.c(end) == 44 {
                t = Some(CSS_ESCAPED_COMMA);
                end += 1
            } else {
                t = Some(LABEL_PUNCTUATION)
            }
        } else if c < 127 && c > 32 {
            t = Some(if "!#$&*+/=?^|~".as_bytes().contains(&(c as u8)) {
                CSS_PUNCTUATION
            } else {
                LABEL_PUNCTUATION
            })
        } else {
            t = Some(LABEL_SYMBOL);
            while end < n {
                let ch = s.c(end);
                if (32..127).contains(&ch) || ch == 9 || ch == 10 || ch == 13 {
                    break;
                }
                end += 1
            }
        }
        out.push(Token {
            kind: t.expect("every lexical branch selects a token"),
            start: i,
            end,
        });
        i = end;
        line_start = fresh;
    }
    for _ in 0..3 {
        out.push(Token {
            kind: EOF,
            start: n,
            end: n,
        })
    }
    Ok(out)
}
