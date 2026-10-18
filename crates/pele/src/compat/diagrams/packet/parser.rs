//! Ordered full-source tokenization, matching Pele common/tokens.ts and packet/parser.ts.
use super::{packet_option, PacketBlock, PacketModel};
use crate::compat::{source::space, svg::js_num, DiagramType, ErrorCode, Object, PeleError};
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    PacketBeta,
    Packet,
    Dash,
    Plus,
    Colon,
    Int,
    AccDescr,
    AccTitle,
    Title,
    String,
    Newline,
    End,
}
struct Token {
    kind: Kind,
    from: usize,
    to: usize,
}
fn blank(src: &str, p: usize) -> usize {
    let mut end = p;
    while src
        .as_bytes()
        .get(end)
        .is_some_and(|c| matches!(c, b' ' | b'\t'))
    {
        end += 1
    }
    end
}
fn line_end(src: &str, p: usize) -> usize {
    src[p..].find(['\n', '\r']).map_or(src.len(), |i| p + i)
}
fn to_comment(src: &str, p: usize) -> usize {
    let end = line_end(src, p);
    src[p..end].find("%%").map_or(end, |i| p + i)
}
fn keyword(src: &str, p: usize, word: &str) -> Option<usize> {
    if !src[p..].starts_with(word) {
        return None;
    }
    let end = p + word.len();
    if src[end..].starts_with("%%") || src[end..].chars().next().is_none_or(space) {
        Some(end)
    } else {
        None
    }
}
fn tokenize(src: &str) -> Result<Vec<Token>, PeleError> {
    let mut tokens = Vec::new();
    let mut p = 0;
    while p < src.len() {
        let tail = &src[p..];
        let mut found = None;
        if tail.starts_with("packet-beta") {
            found = Some((Kind::PacketBeta, p + 11))
        } else if let Some(end) = keyword(src, p, "packet") {
            found = Some((Kind::Packet, end))
        } else if tail.starts_with('-') && !tail.starts_with("---") {
            found = Some((Kind::Dash, p + 1))
        } else if tail.starts_with('+') {
            found = Some((Kind::Plus, p + 1))
        } else if tail.starts_with(':') {
            found = Some((Kind::Colon, p + 1))
        }
        let start = blank(src, p);
        if found.is_none() && src[start..].starts_with("accDescr") {
            let after = start + 8;
            let colon = blank(src, after);
            if src[colon..].starts_with(':') {
                found = Some((Kind::AccDescr, to_comment(src, colon + 1)))
            } else {
                let brace = src[after..]
                    .char_indices()
                    .find(|(_, c)| !space(*c))
                    .map_or(src.len(), |(i, _)| after + i);
                if src[brace..].starts_with('{') {
                    if let Some(end) = src[brace + 1..].find('}') {
                        found = Some((Kind::AccDescr, brace + end + 2))
                    }
                }
            }
        }
        if found.is_none() && src[start..].starts_with("accTitle") {
            let colon = blank(src, start + 8);
            if src[colon..].starts_with(':') {
                found = Some((Kind::AccTitle, to_comment(src, colon + 1)))
            }
        }
        if found.is_none() && src[start..].starts_with("title") {
            let after = start + 5;
            let end = if src
                .as_bytes()
                .get(after)
                .is_some_and(|c| matches!(c, b' ' | b'\t'))
            {
                to_comment(src, after + 1)
            } else {
                after
            };
            found = Some((Kind::Title, end))
        }
        if found.is_none() && tail.as_bytes()[0].is_ascii_digit() {
            if tail.starts_with('0') {
                found = Some((Kind::Int, p + 1))
            } else {
                let mut end = p;
                while src.as_bytes().get(end).is_some_and(u8::is_ascii_digit) {
                    end += 1
                }
                if src.as_bytes().get(end) == Some(&b'.') {
                    end -= 1
                }
                if end > p {
                    found = Some((Kind::Int, end))
                }
            }
        }
        if found.is_none() && tail.starts_with(['"', '\'']) {
            let quote = tail.as_bytes()[0];
            let mut at = p + 1;
            let mut closed = false;
            while at < src.len() {
                let c = src[at..].chars().next().unwrap();
                if c as u32 == quote as u32 {
                    at += 1;
                    closed = true;
                    break;
                }
                if c == '\\' {
                    at += 1;
                    let Some(c) = src[at..].chars().next() else {
                        break;
                    };
                    if matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}') {
                        break;
                    }
                    at += c.len_utf8()
                } else {
                    at += c.len_utf8()
                }
            }
            if closed {
                found = Some((Kind::String, at))
            }
        }
        if found.is_none() && tail.starts_with('\n') {
            found = Some((Kind::Newline, p + 1))
        } else if found.is_none() && tail.starts_with("\r\n") {
            found = Some((Kind::Newline, p + 2))
        }
        if let Some((kind, to)) = found {
            tokens.push(Token { kind, from: p, to });
            p = to;
            continue;
        }
        if start > p {
            p = start;
            continue;
        }
        if let Some(end) = super::super::common::hidden_yaml_end(src, p) {
            p = end;
            continue;
        }
        if tail.starts_with("%%{") {
            if let Some(end) = tail[3..].find("}%%").map(|i| p + 3 + i + 3) {
                let after = if src[end..].starts_with("\r\n") {
                    Some(end + 2)
                } else if src[end..].starts_with('\n') {
                    Some(end + 1)
                } else if src[end..].chars().next().is_none_or(space) {
                    Some(end)
                } else {
                    None
                };
                if let Some(end) = after {
                    p = end;
                    continue;
                }
            }
        }
        if tail.starts_with("%%") {
            p = line_end(src, p);
            continue;
        }
        if tail.starts_with('-') {
            tokens.push(Token {
                kind: Kind::Dash,
                from: p,
                to: p + 1,
            });
            p += 1;
            continue;
        }
        return Err(PeleError::syntax("packet", src, p, "", true));
    }
    tokens.push(Token {
        kind: Kind::End,
        from: src.len(),
        to: src.len(),
    });
    Ok(tokens)
}
struct Reader<'a> {
    src: &'a str,
    tokens: Vec<Token>,
    i: usize,
}
impl Reader<'_> {
    fn kind(&self) -> Kind {
        self.tokens[self.i].kind
    }
    fn take(&mut self) -> &str {
        let t = &self.tokens[self.i];
        self.i += 1;
        &self.src[t.from..t.to]
    }
    fn fail(&self, expected: &str) -> PeleError {
        let token = &self.tokens[self.i];
        let found = if token.kind == Kind::End {
            "end of input".into()
        } else {
            format!("`{}`", &self.src[token.from..token.to])
        };
        PeleError::syntax(
            "packet",
            self.src,
            token.from,
            &format!("Expecting {expected} but found {found}."),
            false,
        )
    }
    fn expect(&mut self, kind: Kind, name: &str) -> Result<&str, PeleError> {
        if self.kind() != kind {
            return Err(self.fail(&format!("token of type '{name}'")));
        }
        Ok(self.take())
    }
    fn end_line(&mut self) -> Result<(), PeleError> {
        if self.kind() == Kind::End {
            return Ok(());
        }
        if self.kind() != Kind::Newline {
            return Err(self.fail("a line break or the end of input"));
        }
        while self.kind() == Kind::Newline {
            self.i += 1
        }
        Ok(())
    }
}
fn single(s: &str) -> String {
    let mut out = String::new();
    let mut blanks = String::new();
    for c in s.trim_matches(space).chars() {
        if matches!(c, ' ' | '\t') {
            blanks.push(c)
        } else {
            if blanks.len() > 1 {
                out.push(' ')
            } else {
                out.push_str(&blanks)
            }
            blanks.clear();
            out.push(c)
        }
    }
    if blanks.len() > 1 {
        out.push(' ')
    } else {
        out.push_str(&blanks)
    }
    out
}
fn string_value(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s[1..s.len() - 1].chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(next) = chars.next() {
                out.push(match next {
                    'b' => '\x08',
                    'f' => '\x0c',
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    'v' => '\x0b',
                    '0' => '\0',
                    _ => next,
                })
            }
        } else {
            out.push(c)
        }
    }
    out
}
fn description(s: &str) -> String {
    let s = s.trim_start_matches([' ', '\t']);
    let after = &s[8..];
    let colon = after.trim_start_matches([' ', '\t']);
    if let Some(body) = colon.strip_prefix(':') {
        return single(body);
    }
    let body = after.trim_start_matches(space);
    let body = &body[1..body.len() - 1];
    // JavaScript multiline anchors recognize LF, CR, LS and PS. Preserve the separator
    // while trimming whitespace at its following anchor; blank-line runs disappear.
    let mut leading = String::new();
    let mut anchor = true;
    for c in body.chars() {
        if anchor && space(c) {
            continue;
        }
        leading.push(c);
        anchor = matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}')
    }
    let mut out = String::new();
    let mut from = 0;
    for (i, c) in leading.char_indices() {
        if matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}') {
            out.push_str(&single(&leading[from..i]));
            out.push(c);
            from = i + c.len_utf8();
        }
    }
    out.push_str(&single(&leading[from..]));
    out.trim_matches(space).into()
}
pub(super) fn parse(
    src: &str,
    title: Option<&str>,
    config: &Object,
) -> Result<PacketModel, PeleError> {
    let mut r = Reader {
        src,
        tokens: tokenize(src)?,
        i: 0,
    };
    while r.kind() == Kind::Newline {
        r.i += 1
    }
    if r.kind() == Kind::PacketBeta {
        r.i += 1
    } else {
        r.expect(Kind::Packet, "packet")?;
    }
    let (mut own_title, mut acc_title, mut acc_descr) = (None, None, None);
    let mut blocks = Vec::new();
    while r.kind() != Kind::End {
        match r.kind() {
            Kind::Newline => r.i += 1,
            Kind::Title => {
                let raw = r.take().trim_start_matches([' ', '\t']);
                own_title = Some(single(&raw[5..]));
                r.end_line()?
            }
            Kind::AccTitle => {
                let raw = r.take();
                acc_title = Some(single(&raw[raw.find(':').unwrap() + 1..]));
                r.end_line()?
            }
            Kind::AccDescr => {
                acc_descr = Some(description(r.take()));
                r.end_line()?
            }
            Kind::Int | Kind::Plus => {
                let (mut start, mut end, mut bits) = (None, None, None);
                if r.kind() == Kind::Plus {
                    r.i += 1;
                    bits = Some(
                        r.expect(Kind::Int, "INT")?
                            .parse::<f64>()
                            .unwrap_or(f64::INFINITY),
                    )
                } else {
                    start = Some(r.take().parse::<f64>().unwrap_or(f64::INFINITY));
                    if r.kind() == Kind::Dash {
                        r.i += 1;
                        end = Some(
                            r.expect(Kind::Int, "INT")?
                                .parse::<f64>()
                                .unwrap_or(f64::INFINITY),
                        )
                    }
                }
                r.expect(Kind::Colon, ":")?;
                let label: std::sync::Arc<str> =
                    string_value(r.expect(Kind::String, "STRING")?).into();
                blocks.push((start, end, bits, label));
                r.end_line()?;
            }
            _ => return Err(r.fail("a field, a title, or a line break")),
        }
    }
    let bits_per_row = packet_option(config, "bitsPerRow", 32., 1., 4096.).floor();
    let mut rows = Vec::new();
    let mut last_bit = -1.;
    let mut word = Vec::new();
    let mut row = 1.;
    let invalid = |message: String| {
        let mut e = PeleError::new(message, ErrorCode::Semantic);
        e.diagram_type = Some("packet".into());
        e
    };
    for (start, end, bits, label) in blocks {
        if let (Some(start), Some(end)) = (start, end) {
            if end < start {
                return Err(invalid(format!(
                    "Packet block {} - {} is invalid. End must be greater than start.",
                    js_num(start),
                    js_num(end)
                )));
            }
        }
        let mut start = start.unwrap_or(last_bit + 1.);
        if start != last_bit + 1. {
            return Err(invalid(format!(
                "Packet block {} - {} is not contiguous. It should start from {}.",
                js_num(start),
                js_num(end.unwrap_or(start)),
                js_num(last_bit + 1.)
            )));
        }
        if bits == Some(0.) {
            return Err(invalid(format!(
                "Packet block {} is invalid. Cannot have a zero bit field.",
                js_num(start)
            )));
        }
        let end = end.unwrap_or(start + bits.unwrap_or(1.) - 1.);
        let mut bits = bits.unwrap_or(end - start + 1.);
        last_bit = end;
        while word.len() as f64 <= bits_per_row + 1. && rows.len() < 10000 {
            let row_end = row * bits_per_row - 1.;
            if end <= row_end {
                word.push(PacketBlock {
                    start,
                    end,
                    bits,
                    label: label.clone(),
                });
                if end == row_end {
                    rows.push(std::mem::take(&mut word));
                    row += 1.;
                }
                break;
            }
            word.push(PacketBlock {
                start,
                end: row_end,
                bits: row_end - start,
                label: label.clone(),
            });
            rows.push(std::mem::take(&mut word));
            row += 1.;
            start = row_end + 1.;
            bits = end - start;
        }
    }
    if !word.is_empty() {
        rows.push(word)
    }
    Ok(PacketModel {
        diagram_type: DiagramType::Packet,
        title: own_title
            .filter(|s| !s.is_empty())
            .or_else(|| title.map(str::to_string)),
        acc_title: acc_title.filter(|s| !s.is_empty()),
        acc_descr: acc_descr.filter(|s| !s.is_empty()),
        bits_per_row,
        rows,
    })
}
