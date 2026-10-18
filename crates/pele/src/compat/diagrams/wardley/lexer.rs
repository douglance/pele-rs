//! Ordered Wardley scanner table, Pele 458322a5 (MIT).
use super::super::common::langium::{self as l, TokenSpec as T};
use crate::compat::source::space;
fn digits(s: &str, p: usize) -> usize {
    let mut e = p;
    while s.as_bytes().get(e).is_some_and(u8::is_ascii_digit) {
        e += 1
    }
    e
}
fn number(s: &str, p: usize) -> Option<usize> {
    // Earlier positions consume an entire fraction; an interior digit can only belong to an integer run.
    if p > 0 && s.as_bytes()[p - 1].is_ascii_digit() {
        return None;
    }
    let e = digits(s, p);
    if e == p || s.as_bytes().get(e) != Some(&b'.') {
        return None;
    }
    let stop = digits(s, e + 1);
    (stop > e + 1).then_some(stop)
}
fn int(s: &str, p: usize) -> Option<usize> {
    if s[p..].starts_with('0') {
        return Some(p + 1);
    }
    if !s
        .as_bytes()
        .get(p)
        .is_some_and(|c| matches!(c, b'1'..=b'9'))
    {
        return None;
    }
    let mut e = digits(s, p);
    if s.as_bytes().get(e) == Some(&b'.') {
        e -= 1
    }
    (e > p).then_some(e)
}
fn link_arrow(s: &str, p: usize) -> Option<usize> {
    for word in ["-->", "-.->", ">"] {
        if s[p..].starts_with(word) {
            return Some(p + word.len());
        }
    }
    if !s[p..].starts_with("+'") {
        return None;
    }
    let end = s[p + 2..].find('\'').map(|i| p + i + 3)?;
    for suffix in ["<>", "<", ">"] {
        if s[end..].starts_with(suffix) {
            return Some(end + suffix.len());
        }
    }
    None
}
fn link_port(s: &str, p: usize) -> Option<usize> {
    ["+<>", "+>", "+<"]
        .iter()
        .find_map(|a| s[p..].starts_with(a).then_some(p + a.len()))
}
fn link_label(s: &str, p: usize) -> Option<usize> {
    if !s[p..].starts_with(';') {
        return None;
    }
    let e = l::line_end(s, p + 1);
    (e > p + 1).then_some(e)
}
fn strategy(s: &str, p: usize) -> Option<usize> {
    ["build", "buy", "outsource", "market"]
        .iter()
        .find_map(|a| s[p..].starts_with(a).then_some(p + a.len()))
}
fn continuation(s: &str, p: usize) -> bool {
    let b = s.as_bytes().get(p).copied();
    b.is_some_and(|c| {
        c.is_ascii_alphanumeric()
            || matches!(c, b'_' | b'(' | b')' | b'&')
            || c == b'-' && s.as_bytes().get(p + 1) != Some(&b'>')
    })
}
fn name(s: &str, p: usize) -> Option<usize> {
    if s[p..].starts_with("accTitle")
        || s[p..].starts_with("accDescr")
        || s[p..]
            .strip_prefix("title")
            .is_some_and(|s| s.chars().next().is_some_and(space))
    {
        return None;
    }
    if !s.as_bytes().get(p).is_some_and(u8::is_ascii_alphabetic) {
        return None;
    }
    let mut e = p + 1;
    while continuation(s, e) {
        e += 1
    }
    loop {
        let next = l::blank(s, e);
        if next == e
            || !s
                .as_bytes()
                .get(next)
                .is_some_and(|c| c.is_ascii_alphabetic() || *c == b'(')
        {
            break;
        }
        e = next + 1;
        while continuation(s, e) {
            e += 1
        }
    }
    Some(e)
}
fn id(s: &str, p: usize) -> Option<usize> {
    let b = s.as_bytes();
    if !b
        .get(p)
        .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_')
    {
        return None;
    }
    let mut e = p + 1;
    while b
        .get(e)
        .is_some_and(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-'))
    {
        e += 1
    }
    while b.get(e - 1) == Some(&b'-') {
        e -= 1
    }
    Some(e)
}
fn word_0(s: &str, p: usize) -> Option<usize> {
    s[p..].starts_with("wardley-beta").then_some(p + 12)
}
fn word_1(s: &str, p: usize) -> Option<usize> {
    s[p..].starts_with("size").then_some(p + 4)
}
fn word_2(s: &str, p: usize) -> Option<usize> {
    s[p..].starts_with("evolution").then_some(p + 9)
}
fn word_3(s: &str, p: usize) -> Option<usize> {
    s[p..].starts_with("anchor").then_some(p + 6)
}
fn word_4(s: &str, p: usize) -> Option<usize> {
    s[p..].starts_with("component").then_some(p + 9)
}
fn word_5(s: &str, p: usize) -> Option<usize> {
    s[p..].starts_with("label").then_some(p + 5)
}
fn word_6(s: &str, p: usize) -> Option<usize> {
    s[p..].starts_with("inertia").then_some(p + 7)
}
fn word_7(s: &str, p: usize) -> Option<usize> {
    s[p..].starts_with("evolve").then_some(p + 6)
}
fn word_8(s: &str, p: usize) -> Option<usize> {
    s[p..].starts_with("pipeline").then_some(p + 8)
}
fn word_9(s: &str, p: usize) -> Option<usize> {
    s[p..].starts_with("note").then_some(p + 4)
}
fn word_10(s: &str, p: usize) -> Option<usize> {
    s[p..].starts_with("annotations").then_some(p + 11)
}
fn word_11(s: &str, p: usize) -> Option<usize> {
    s[p..].starts_with("annotation").then_some(p + 10)
}
fn word_12(s: &str, p: usize) -> Option<usize> {
    s[p..].starts_with("accelerator").then_some(p + 11)
}
fn word_13(s: &str, p: usize) -> Option<usize> {
    s[p..].starts_with("deaccelerator").then_some(p + 13)
}
pub(super) static TOKENS: &[T] = &[
    T::literal("[", &[]),
    T::literal(",", &[]),
    T::literal("]", &[]),
    T::literal("@", &[]),
    T::literal("/", &[]),
    T::literal("(", &[]),
    T::literal(")", &[]),
    T::literal("-", &[11, 13, 40]),
    T::literal("{", &[]),
    T::literal("}", &[]),
    T::matcher("WARDLEY_NUMBER", number, false, Some("0123456789"), None),
    T {
        name: "ARROW",
        ..T::literal("->", &[])
    },
    T::matcher("LINK_PORT", link_port, false, Some("+"), None),
    T::matcher("LINK_ARROW", link_arrow, false, Some("->+"), None),
    T::matcher("LINK_LABEL", link_label, false, Some(";"), None),
    T::matcher("STRATEGY", strategy, false, Some("bom"), None),
    T::matcher("KW_WARDLEY", word_0, false, Some("w"), None),
    T::matcher("KW_SIZE", word_1, false, Some("s"), None),
    T::matcher("KW_EVOLUTION", word_2, false, Some("e"), None),
    T::matcher("KW_ANCHOR", word_3, false, Some("a"), None),
    T::matcher("KW_COMPONENT", word_4, false, Some("c"), None),
    T::matcher("KW_LABEL", word_5, false, Some("l"), None),
    T::matcher("KW_INERTIA", word_6, false, Some("i"), None),
    T::matcher("KW_EVOLVE", word_7, false, Some("e"), None),
    T::matcher("KW_PIPELINE", word_8, false, Some("p"), None),
    T::matcher("KW_NOTE", word_9, false, Some("n"), None),
    T::matcher("KW_ANNOTATIONS", word_10, false, Some("a"), None),
    T::matcher("KW_ANNOTATION", word_11, false, Some("a"), None),
    T::matcher("KW_ACCELERATOR", word_12, false, Some("a"), None),
    T::matcher("KW_DEACCELERATOR", word_13, false, Some("d"), None),
    T::matcher("NAME_WITH_SPACES", name, false, None, None),
    T {
        name: "WS",
        ..l::WHITESPACE
    },
    l::ACC_DESCR_BLOCK,
    l::ACC_TITLE,
    l::TITLE,
    T::matcher("INT", int, false, Some("0123456789"), None),
    l::STRING,
    T::matcher("ID", id, false, None, None),
    l::NEWLINE,
    l::WHITESPACE,
    l::YAML,
    l::DIRECTIVE,
    l::SINGLE_LINE_COMMENT,
];
