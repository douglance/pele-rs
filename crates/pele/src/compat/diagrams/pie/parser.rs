//! Ordered full-source tokenization, matching Pele common/tokens.ts and pie/parser.ts.
use super::{PieModel, PieSection};
use crate::compat::{source::space, svg::js_num, DiagramType, ErrorCode, PeleError};
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    ShowData,
    Pie,
    Colon,
    Number,
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
#[derive(Default)]
struct Numbers {
    from: usize,
    end: usize,
    fraction: Option<usize>,
}
impl Numbers {
    fn scan(&mut self, src: &str, p: usize) -> Option<usize> {
        let b = src.as_bytes();
        let at = p + usize::from(b.get(p) == Some(&b'-'));
        if at < self.from || at >= self.end {
            self.from = at;
            self.end = at;
            while b.get(self.end).is_some_and(u8::is_ascii_digit) {
                self.end += 1
            }
            self.fraction = None;
            if self.end > at && b.get(self.end) == Some(&b'.') {
                let mut stop = self.end + 1;
                while b.get(stop).is_some_and(u8::is_ascii_digit) {
                    stop += 1
                }
                let n = stop - self.end - 1;
                if n > 0 && b.get(stop) != Some(&b'.') {
                    self.fraction = Some(stop)
                } else if n > 1 {
                    self.fraction = Some(stop - 1)
                }
            }
        }
        if self.end == at {
            return None;
        }
        if let Some(end) = self.fraction {
            return Some(end);
        }
        if b.get(at) == Some(&b'0') {
            return if b.get(at + 1) == Some(&b'.') {
                None
            } else {
                Some(at + 1)
            };
        }
        if b.get(self.end) != Some(&b'.') {
            return Some(self.end);
        }
        if self.end.saturating_sub(1) > at {
            Some(self.end - 1)
        } else {
            None
        }
    }
}
fn tokenize(src: &str) -> Result<Vec<Token>, PeleError> {
    let mut tokens = Vec::new();
    let mut p = 0;
    let mut numbers = Numbers::default();
    while p < src.len() {
        let tail = &src[p..];
        let mut found = None;
        if let Some(end) = keyword(src, p, "showData") {
            found = Some((Kind::ShowData, end))
        } else if let Some(end) = keyword(src, p, "pie") {
            found = Some((Kind::Pie, end))
        } else if tail.starts_with(':') {
            found = Some((Kind::Colon, p + 1))
        } else if tail
            .as_bytes()
            .first()
            .is_some_and(|c| *c == b'-' || c.is_ascii_digit())
        {
            if let Some(end) = numbers.scan(src, p) {
                found = Some((Kind::Number, end))
            }
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
        return Err(PeleError::syntax("pie", src, p, "", true));
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
            "pie",
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
pub(super) fn parse(src: &str, title: Option<&str>) -> Result<PieModel, PeleError> {
    let mut r = Reader {
        src,
        tokens: tokenize(src)?,
        i: 0,
    };
    while r.kind() == Kind::Newline {
        r.i += 1
    }
    r.expect(Kind::Pie, "pie")?;
    let show_data = if r.kind() == Kind::ShowData {
        r.i += 1;
        true
    } else {
        false
    };
    let (mut own_title, mut acc_title, mut acc_descr) = (None, None, None);
    let mut entries = Vec::new();
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
                let raw = raw[raw.find(':').unwrap() + 1..].to_string();
                acc_title = Some(single(&raw));
                r.end_line()?
            }
            Kind::AccDescr => {
                acc_descr = Some(description(r.take()));
                r.end_line()?
            }
            Kind::String => {
                let label = string_value(r.take());
                r.expect(Kind::Colon, ":")?;
                let value = r
                    .expect(Kind::Number, "NUMBER_PIE")?
                    .parse::<f64>()
                    .unwrap();
                entries.push(PieSection { label, value });
                r.end_line()?
            }
            _ => return Err(r.fail("a section, a title, or a line break")),
        }
    }
    let mut sections: Vec<PieSection> = Vec::new();
    for section in entries {
        if section.value < 0. {
            let mut e=PeleError::new(format!("\"{}\" has invalid value: {}. Negative values are not allowed in pie charts. All slice values must be >= 0.",section.label,js_num(section.value)),ErrorCode::Semantic);
            e.diagram_type = Some("pie".into());
            return Err(e);
        }
        if !sections.iter().any(|s| s.label == section.label) {
            sections.push(section)
        }
    }
    Ok(PieModel {
        diagram_type: DiagramType::Pie,
        title: own_title
            .filter(|s| !s.is_empty())
            .or_else(|| title.map(str::to_string)),
        acc_title: acc_title.filter(|s| !s.is_empty()),
        acc_descr: acc_descr.filter(|s| !s.is_empty()),
        show_data,
        sections,
    })
}
