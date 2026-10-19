//! Pele diagrams/radar/parser.ts and common/tokens.ts, 458322a5 (MIT, kepano).
use super::{RadarAxis, RadarCurve, RadarModel, RadarOptions};
use crate::compat::{source::space, DiagramType, ErrorCode, PeleError};
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    RadarColon,
    Radar,
    ShowLegend,
    Graticule,
    Curve,
    Ticks,
    Axis,
    Max,
    Min,
    Colon,
    Comma,
    Open,
    Close,
    OpenBrace,
    CloseBrace,
    GraticuleValue,
    Boolean,
    AccDescr,
    AccTitle,
    Title,
    Number,
    String,
    Id,
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
        let at = p;
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
            return Some(at + 1);
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
fn id_end(src: &str, p: usize) -> Option<usize> {
    let b = src.as_bytes();
    if !b
        .get(p)
        .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_')
    {
        return None;
    }
    let mut end = p + 1;
    while b
        .get(end)
        .is_some_and(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-'))
    {
        end += 1
    }
    while end > p + 1 && b[end - 1] == b'-' {
        end -= 1
    }
    Some(end)
}
fn tokenize(src: &str) -> Result<Vec<Token>, PeleError> {
    let mut tokens = Vec::new();
    let mut p = 0;
    let mut numbers = Numbers::default();
    let last_brace = src.rfind('}');
    while p < src.len() {
        let tail = &src[p..];
        let start = blank(src, p);
        let mut found = None;
        if tail.starts_with("radar-beta:") {
            found = Some((Kind::RadarColon, p + 11))
        } else if let Some(end) = keyword(src, p, "radar-beta") {
            found = Some((Kind::Radar, end))
        }
        if found.is_none() {
            for (word, kind) in [
                ("showLegend", Kind::ShowLegend),
                ("graticule", Kind::Graticule),
                ("curve", Kind::Curve),
                ("ticks", Kind::Ticks),
                ("axis", Kind::Axis),
                ("max", Kind::Max),
                ("min", Kind::Min),
            ] {
                if tail.starts_with(word) {
                    let end = p + word.len();
                    found = Some(match id_end(src, p) {
                        Some(id) if id > end => (Kind::Id, id),
                        _ => (kind, end),
                    });
                    break;
                }
            }
        }
        if found.is_none() {
            for (c, kind) in [
                (':', Kind::Colon),
                (',', Kind::Comma),
                ('[', Kind::Open),
                (']', Kind::Close),
                ('{', Kind::OpenBrace),
                ('}', Kind::CloseBrace),
            ] {
                if tail.starts_with(c) {
                    found = Some((kind, p + 1));
                    break;
                }
            }
        }
        if found.is_none() {
            for word in ["circle", "polygon"] {
                if tail.starts_with(word) {
                    found = Some((Kind::GraticuleValue, p + word.len()));
                    break;
                }
            }
        }
        if found.is_none() {
            for word in ["true", "false"] {
                if tail.starts_with(word) {
                    found = Some((Kind::Boolean, p + word.len()));
                    break;
                }
            }
        }
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
                if src[brace..].starts_with('{') && last_brace.is_some_and(|last| brace < last) {
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
        if found.is_none() {
            if let Some(end) = numbers.scan(src, p) {
                found = Some((Kind::Number, end))
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
        if found.is_none() {
            if let Some(end) = id_end(src, p) {
                found = Some((Kind::Id, end))
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

        return Err(PeleError::syntax("radar", src, p, "", true));
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
            "radar",
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
impl Reader<'_> {
    fn accept(&mut self, k: Kind) -> bool {
        if self.kind() == k {
            self.i += 1;
            true
        } else {
            false
        }
    }
    fn newlines(&mut self) {
        while self.accept(Kind::Newline) {}
    }
    fn label(&mut self) -> Result<Option<String>, PeleError> {
        if !self.accept(Kind::Open) {
            return Ok(None);
        }
        let value = string_value(self.expect(Kind::String, "STRING")?);
        self.expect(Kind::Close, "]")?;
        Ok(Some(value))
    }
    fn entry(&mut self, detailed: bool) -> Result<(Option<String>, f64), PeleError> {
        let name = if detailed {
            let name = self.expect(Kind::Id, "ID")?.to_string();
            self.accept(Kind::Colon);
            Some(name)
        } else {
            None
        };
        let value = self.expect(Kind::Number, "NUMBER")?.parse::<f64>().unwrap();
        Ok((name, value))
    }
    fn curve(&mut self) -> Result<(String, Option<String>, Vec<(Option<String>, f64)>), PeleError> {
        let name = self.expect(Kind::Id, "ID")?.to_string();
        let label = self.label()?;
        self.expect(Kind::OpenBrace, "{")?;
        self.newlines();
        if !matches!(self.kind(), Kind::Number | Kind::Id) {
            return Err(self.fail("a number or an axis name"));
        }
        let detailed = self.kind() == Kind::Id;
        let mut entries = vec![self.entry(detailed)?];
        while self.accept(Kind::Comma) {
            self.newlines();
            entries.push(self.entry(detailed)?)
        }
        self.newlines();
        self.expect(Kind::CloseBrace, "}")?;
        Ok((name, label, entries))
    }
    fn is_option(&self) -> bool {
        matches!(
            self.kind(),
            Kind::ShowLegend | Kind::Ticks | Kind::Max | Kind::Min | Kind::Graticule
        )
    }
    fn option(&mut self, options: &mut RadarOptions) -> Result<(), PeleError> {
        let kind = self.kind();
        self.take();
        match kind {
            Kind::ShowLegend => {
                options.show_legend = self.expect(Kind::Boolean, "BOOLEAN")? == "true"
            }
            Kind::Graticule => {
                options.graticule = self.expect(Kind::GraticuleValue, "GRATICULE")?.into()
            }
            Kind::Ticks => options.ticks = self.expect(Kind::Number, "NUMBER")?.parse().unwrap(),
            Kind::Max => options.max = Some(self.expect(Kind::Number, "NUMBER")?.parse().unwrap()),
            Kind::Min => options.min = self.expect(Kind::Number, "NUMBER")?.parse().unwrap(),
            _ => unreachable!(),
        }
        Ok(())
    }
}
fn semantic(message: impl Into<String>) -> PeleError {
    let mut error = PeleError::new(message, ErrorCode::Semantic);
    error.diagram_type = Some("radar".into());
    error
}
pub(super) fn parse(src: &str, title: Option<&str>) -> Result<RadarModel, PeleError> {
    let mut r = Reader {
        src,
        tokens: tokenize(src)?,
        i: 0,
    };
    let mut options = RadarOptions::default();
    let mut axes = Vec::new();
    let mut curve_ast = Vec::new();
    let (mut own_title, mut acc_title, mut acc_descr) = (None, None, None);
    r.newlines();
    if !r.accept(Kind::RadarColon) {
        r.expect(Kind::Radar, "radar-beta")?;
        r.accept(Kind::Colon);
    }
    r.newlines();
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
            Kind::Axis => {
                r.i += 1;
                loop {
                    let name = r.expect(Kind::Id, "ID")?.to_string();
                    let label = r.label()?.unwrap_or_else(|| name.clone());
                    axes.push(RadarAxis { name, label });
                    if !r.accept(Kind::Comma) {
                        break;
                    }
                }
            }
            Kind::Curve => {
                r.i += 1;
                loop {
                    curve_ast.push(r.curve()?);
                    if !r.accept(Kind::Comma) {
                        break;
                    }
                }
            }
            _ => {
                if !r.is_option() {
                    return Err(r.fail("an axis, a curve, an option, a title, or a line break"));
                }
                r.option(&mut options)?;
                while r.accept(Kind::Comma) {
                    if !r.is_option() {
                        return Err(r.fail("an option"));
                    }
                    r.option(&mut options)?
                }
            }
        }
    }
    let mut curves = Vec::new();
    for (name, label, entries) in curve_ast {
        let values = if entries[0].0.is_none() {
            entries.iter().map(|e| e.1).collect()
        } else {
            if axes.is_empty() {
                return Err(semantic(
                    "Axes must be populated before curves for reference entries",
                ));
            }
            let mut by_axis = std::collections::HashMap::new();
            for (axis, value) in &entries {
                if let Some(axis) = axis {
                    by_axis.entry(axis.as_str()).or_insert(*value);
                }
            }
            let mut values = Vec::new();
            for axis in &axes {
                let Some(value) = by_axis.get(axis.name.as_str()) else {
                    return Err(semantic(format!("Missing entry for axis {}", axis.label)));
                };
                values.push(*value)
            }
            values
        };
        curves.push(RadarCurve {
            label: label.unwrap_or_else(|| name.clone()),
            name,
            entries: values,
        })
    }
    if options.ticks > 32. {
        options.ticks = 32.
    }
    Ok(RadarModel {
        diagram_type: DiagramType::Radar,
        title: own_title
            .filter(|s| !s.is_empty())
            .or_else(|| title.map(str::to_string)),
        acc_title: acc_title.filter(|s| !s.is_empty()),
        acc_descr: acc_descr.filter(|s| !s.is_empty()),
        axes,
        curves,
        options,
    })
}
