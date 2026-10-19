//! Four canonical ordered Railroad token tables, Pele 458322a5 (MIT).
use super::super::common::langium::{self as l, TokenSpec as T};
use crate::compat::source::space;
fn id(s: &str, p: usize) -> Option<usize> {
    name(s, p, true)
}
fn abnf_name(s: &str, p: usize) -> Option<usize> {
    name(s, p, false)
}
fn name(s: &str, p: usize, under: bool) -> Option<usize> {
    let b = s.as_bytes();
    if !b
        .get(p)
        .is_some_and(|c| c.is_ascii_alphabetic() || under && *c == b'_')
    {
        return None;
    }
    let mut e = p + 1;
    while b
        .get(e)
        .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'-' || under && *c == b'_')
    {
        e += 1
    }
    Some(e)
}
fn title(s: &str, p: usize) -> Option<usize> {
    if !s[p..].starts_with("title") {
        return None;
    }
    let e = p + 5;
    Some(
        if s.as_bytes()
            .get(e)
            .is_some_and(|c| matches!(c, b' ' | b'\t'))
        {
            comment_end(s, e + 1)
        } else {
            e
        },
    )
}
fn comment_end(s: &str, p: usize) -> usize {
    let e = l::line_end(s, p);
    s[p..e].find("%%").map_or(e, |i| p + i)
}
fn acc_title(s: &str, p: usize) -> Option<usize> {
    if !s[p..].starts_with("accTitle") {
        return None;
    }
    let e = l::blank(s, p + 8);
    s[e..].starts_with(':').then(|| comment_end(s, e + 1))
}
fn acc_descr(s: &str, p: usize) -> Option<usize> {
    if !s[p..].starts_with("accDescr") {
        return None;
    }
    let e = l::blank(s, p + 8);
    if s[e..].starts_with(':') {
        return Some(comment_end(s, e + 1));
    }
    let e = skip_space(s, p + 8);
    if !s[e..].starts_with('{') {
        return None;
    }
    s[e + 1..].find('}').map(|i| e + i + 2)
}
fn skip_space(s: &str, p: usize) -> usize {
    s[p..]
        .char_indices()
        .find(|(_, c)| !space(*c))
        .map_or(s.len(), |(i, _)| p + i)
}
fn acc_open(s: &str, p: usize) -> Option<usize> {
    if !s[p..].starts_with("accDescr") {
        return None;
    }
    let e = skip_space(s, p + 8);
    s[e..].starts_with('{').then_some(e + 1)
}
fn whitespace(s: &str, p: usize) -> Option<usize> {
    let mut e = p;
    while s
        .as_bytes()
        .get(e)
        .is_some_and(|c| matches!(c, b' ' | b'\t' | b'\r' | b'\n'))
    {
        e += 1
    }
    (e > p).then_some(e)
}
fn block(s: &str, p: usize) -> Option<usize> {
    s[p..].strip_prefix("/*")?.find("*/").map(|i| p + i + 4)
}
fn block_open(s: &str, p: usize) -> Option<usize> {
    s[p..].starts_with("/*").then_some(p + 2)
}
fn iso(s: &str, p: usize) -> Option<usize> {
    s[p..].strip_prefix("(*")?.find("*)").map(|i| p + i + 4)
}
fn iso_open(s: &str, p: usize) -> Option<usize> {
    s[p..].starts_with("(*").then_some(p + 2)
}
fn special(s: &str, p: usize) -> Option<usize> {
    if !s[p..].starts_with('?') {
        return None;
    }
    let mut solid = false;
    for (i, c) in s[p + 1..].char_indices() {
        if c == '?' {
            return solid.then_some(p + i + 2);
        }
        if c == ';' {
            return None;
        }
        solid |= !space(c)
    }
    None
}
fn abnf_string(s: &str, p: usize) -> Option<usize> {
    s[p..].strip_prefix('"')?.find('"').map(|i| p + i + 2)
}
fn abnf_number(s: &str, p: usize) -> Option<usize> {
    let b = s.as_bytes();
    if b.get(p) != Some(&b'%')
        || !b
            .get(p + 1)
            .is_some_and(|c| matches!(c, b'x' | b'X' | b'd' | b'D' | b'b' | b'B'))
    {
        return None;
    }
    let mut e = p + 2;
    while b.get(e).is_some_and(u8::is_ascii_hexdigit) {
        e += 1
    }
    if e == p + 2 {
        return None;
    }
    loop {
        if !b.get(e).is_some_and(|c| matches!(c, b'-' | b'.')) {
            break;
        }
        let mut q = e + 1;
        while b.get(q).is_some_and(u8::is_ascii_hexdigit) {
            q += 1
        }
        if q == e + 1 {
            break;
        }
        e = q
    }
    Some(e)
}
fn repeat(s: &str, p: usize) -> Option<usize> {
    let b = s.as_bytes();
    let mut e = p;
    while b.get(e).is_some_and(u8::is_ascii_digit) {
        e += 1
    }
    if b.get(e) != Some(&b'*') {
        return None;
    }
    e += 1;
    while b.get(e).is_some_and(u8::is_ascii_digit) {
        e += 1
    }
    Some(e)
}
fn exact(s: &str, p: usize) -> Option<usize> {
    let mut e = p;
    while s.as_bytes().get(e).is_some_and(u8::is_ascii_digit) {
        e += 1
    }
    (e > p).then_some(e)
}
fn abnf_comment(s: &str, p: usize) -> Option<usize> {
    s[p..].starts_with(';').then(|| l::line_end(s, p))
}
fn peg_comment(s: &str, p: usize) -> Option<usize> {
    s[p..].starts_with('#').then(|| l::line_end(s, p))
}
fn ir_open(s: &str, p: usize) -> Option<usize> {
    l::keyword(s, p, "railroad-beta")
}
fn ebnf_open(s: &str, p: usize) -> Option<usize> {
    l::keyword(s, p, "railroad-ebnf-beta")
}
fn abnf_opening(s: &str, p: usize) -> Option<usize> {
    l::keyword(s, p, "railroad-abnf-beta")
}
fn peg_open(s: &str, p: usize) -> Option<usize> {
    l::keyword(s, p, "railroad-peg-beta")
}
pub(super) static IR: &[T] = &[
    T {
        longer: &[17],
        ..T::matcher("railroad-beta", ir_open, false, Some("r"), None)
    },
    T::literal("nonterminal", &[17]),
    T::literal("zeroOrMore", &[17]),
    T::literal("oneOrMore", &[17]),
    T::literal("sequence", &[17]),
    T::literal("optional", &[17]),
    T::literal("terminal", &[17]),
    T::literal("special", &[17]),
    T::literal("choice", &[17]),
    T::literal("=", &[]),
    T::literal(";", &[]),
    T::literal("(", &[]),
    T::literal(",", &[]),
    T::literal(")", &[]),
    T::matcher("TITLE", title, false, Some("t"), None),
    T::matcher("ACC_TITLE", acc_title, false, Some("a"), None),
    T::matcher("ACC_DESCR", acc_descr, false, Some("a"), Some(acc_open)),
    T::matcher("RR_ID", id, false, None, None),
    T {
        name: "RR_STRING",
        ..l::STRING
    },
    T::matcher("RR_WHITESPACE", whitespace, true, Some("\t \r\n"), None),
    T {
        name: "RR_YAML",
        ..l::YAML
    },
    T {
        name: "RR_DIRECTIVE",
        ..l::DIRECTIVE
    },
    T {
        name: "RR_SINGLE_LINE_COMMENT",
        ..l::SINGLE_LINE_COMMENT
    },
    T::matcher("RR_BLOCK_COMMENT", block, true, Some("/"), Some(block_open)),
];
pub(super) static EBNF: &[T] = &[
    T {
        longer: &[19],
        ..T::matcher("railroad-ebnf-beta", ebnf_open, false, Some("r"), None)
    },
    T::literal("::=", &[]),
    T::literal("=", &[]),
    T::literal(";", &[]),
    T::literal("|", &[]),
    T::literal(",", &[]),
    T::literal("(", &[27]),
    T::literal(")", &[]),
    T::literal("[", &[]),
    T::literal("]", &[]),
    T::literal("{", &[]),
    T::literal("}", &[]),
    T::literal("?", &[21]),
    T::literal("*", &[]),
    T::literal("+", &[]),
    T::literal("-", &[23]),
    T::matcher("TITLE", title, false, Some("t"), None),
    T::matcher("ACC_TITLE", acc_title, false, Some("a"), None),
    T::matcher("ACC_DESCR", acc_descr, false, Some("a"), Some(acc_open)),
    T::matcher("EBNF_ID", id, false, None, None),
    T {
        name: "EBNF_STRING",
        ..l::STRING
    },
    T::matcher("EBNF_SPECIAL_SEQUENCE", special, false, Some("?"), None),
    T::matcher("EBNF_WHITESPACE", whitespace, true, Some("\t \r\n"), None),
    T {
        name: "EBNF_YAML",
        ..l::YAML
    },
    T {
        name: "EBNF_DIRECTIVE",
        ..l::DIRECTIVE
    },
    T {
        name: "EBNF_SINGLE_LINE_COMMENT",
        ..l::SINGLE_LINE_COMMENT
    },
    T::matcher(
        "EBNF_BLOCK_COMMENT",
        block,
        true,
        Some("/"),
        Some(block_open),
    ),
    T::matcher("EBNF_ISO_COMMENT", iso, true, Some("("), Some(iso_open)),
];
pub(super) static ABNF: &[T] = &[
    T {
        longer: &[11],
        ..T::matcher("railroad-abnf-beta", abnf_opening, false, Some("r"), None)
    },
    T::literal("=", &[]),
    T::literal(";", &[20]),
    T::literal("/", &[]),
    T::literal("(", &[]),
    T::literal(")", &[]),
    T::literal("[", &[]),
    T::literal("]", &[]),
    T::matcher("TITLE", title, false, Some("t"), None),
    T::matcher("ACC_TITLE", acc_title, false, Some("a"), None),
    T::matcher("ACC_DESCR", acc_descr, false, Some("a"), Some(acc_open)),
    T::matcher("ABNF_RULENAME", abnf_name, false, None, None),
    T::matcher("ABNF_STRING", abnf_string, false, Some("\""), None),
    T::matcher("ABNF_NUMVAL", abnf_number, false, Some("%"), None),
    T::matcher("ABNF_REPEAT", repeat, false, Some("0123456789*"), None),
    T::matcher("ABNF_EXACT_REPEAT", exact, false, Some("0123456789"), None),
    T::matcher("ABNF_WHITESPACE", whitespace, true, Some("\t \r\n"), None),
    T {
        name: "ABNF_YAML",
        ..l::YAML
    },
    T {
        name: "ABNF_DIRECTIVE",
        ..l::DIRECTIVE
    },
    T {
        name: "ABNF_SINGLE_LINE_COMMENT",
        ..l::SINGLE_LINE_COMMENT
    },
    T::matcher("ABNF_COMMENT", abnf_comment, true, Some(";"), None),
];
pub(super) static PEG: &[T] = &[
    T {
        longer: &[15],
        ..T::matcher("railroad-peg-beta", peg_open, false, Some("r"), None)
    },
    T::literal("<-", &[]),
    T::literal(";", &[]),
    T::literal("/", &[]),
    T::literal("&", &[]),
    T::literal("!", &[]),
    T::literal("?", &[]),
    T::literal("*", &[]),
    T::literal("+", &[]),
    T::literal("(", &[]),
    T::literal(")", &[]),
    T::literal(".", &[]),
    T::matcher("TITLE", title, false, Some("t"), None),
    T::matcher("ACC_TITLE", acc_title, false, Some("a"), None),
    T::matcher("ACC_DESCR", acc_descr, false, Some("a"), Some(acc_open)),
    T::matcher("PEG_ID", id, false, None, None),
    T {
        name: "PEG_STRING",
        ..l::STRING
    },
    T::matcher("PEG_WHITESPACE", whitespace, true, Some("\t \r\n"), None),
    T {
        name: "PEG_YAML",
        ..l::YAML
    },
    T {
        name: "PEG_DIRECTIVE",
        ..l::DIRECTIVE
    },
    T {
        name: "PEG_SINGLE_LINE_COMMENT",
        ..l::SINGLE_LINE_COMMENT
    },
    T::matcher("PEG_LINE_COMMENT", peg_comment, true, Some("#"), None),
];
