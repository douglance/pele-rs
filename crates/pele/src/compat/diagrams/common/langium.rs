//! Reusable ordered token tables and Reader from Pele common/tokens.ts (458322a5, MIT).
//! A family supplies token patterns; this module owns precedence, longer alternatives,
//! hidden trivia, opener budgets, source spans and Chevrotain-style diagnostics.
use crate::compat::{source::space, PeleError};
pub(crate) type Matcher = fn(&str, usize) -> Option<usize>;
pub(crate) const END: usize = usize::MAX;
#[derive(Clone, Copy)]
pub(crate) enum Pattern {
    Literal(&'static str),
    Match(Matcher),
}
#[derive(Clone, Copy)]
pub(crate) struct TokenSpec {
    pub name: &'static str,
    pub pattern: Pattern,
    pub hidden: bool,
    pub opener: Option<Matcher>,
    pub first: Option<&'static str>,
    pub longer: &'static [usize],
}
impl TokenSpec {
    pub const fn literal(name: &'static str, longer: &'static [usize]) -> Self {
        Self {
            name,
            pattern: Pattern::Literal(name),
            hidden: false,
            opener: None,
            first: None,
            longer,
        }
    }
    pub const fn matcher(
        name: &'static str,
        matcher: Matcher,
        hidden: bool,
        first: Option<&'static str>,
        opener: Option<Matcher>,
    ) -> Self {
        Self {
            name,
            pattern: Pattern::Match(matcher),
            hidden,
            opener,
            first,
            longer: &[],
        }
    }
}
pub(crate) struct Token {
    pub kind: usize,
    pub start: usize,
    pub end: usize,
}
pub(crate) fn blank(src: &str, mut p: usize) -> usize {
    while src
        .as_bytes()
        .get(p)
        .is_some_and(|c| matches!(c, b' ' | b'\t'))
    {
        p += 1
    }
    p
}
pub(crate) fn line_end(src: &str, p: usize) -> usize {
    src[p..].find(['\n', '\r']).map_or(src.len(), |i| p + i)
}
fn to_comment(src: &str, p: usize) -> usize {
    let end = line_end(src, p);
    src[p..end].find("%%").map_or(end, |i| p + i)
}
pub(crate) fn keyword(src: &str, p: usize, word: &str) -> Option<usize> {
    let tail = &src[p..];
    if !tail.starts_with(word) {
        return None;
    }
    let end = p + word.len();
    if src[end..].starts_with("%%") || src[end..].chars().next().is_none_or(space) {
        Some(end)
    } else {
        None
    }
}
fn acc_descr(src: &str, p: usize) -> Option<usize> {
    let p = blank(src, p);
    if !src[p..].starts_with("accDescr") {
        return None;
    }
    let after = p + 8;
    let colon = blank(src, after);
    if src[colon..].starts_with(':') {
        return Some(to_comment(src, colon + 1));
    }
    let at = src[after..]
        .char_indices()
        .find(|(_, c)| !space(*c))
        .map_or(src.len(), |(i, _)| after + i);
    if !src[at..].starts_with('{') {
        return None;
    }
    src[at + 1..].find('}').map(|i| at + i + 2)
}
fn acc_descr_open(src: &str, p: usize) -> Option<usize> {
    let p = blank(src, p);
    if !src[p..].starts_with("accDescr") {
        return None;
    }
    let after = p + 8;
    let at = src[after..]
        .char_indices()
        .find(|(_, c)| !space(*c))
        .map_or(src.len(), |(i, _)| after + i);
    src[at..].starts_with('{').then_some(at + 1)
}
fn acc_title(src: &str, p: usize) -> Option<usize> {
    let p = blank(src, p);
    if !src[p..].starts_with("accTitle") {
        return None;
    }
    let at = blank(src, p + 8);
    src[at..].starts_with(':').then(|| to_comment(src, at + 1))
}
fn title(src: &str, p: usize) -> Option<usize> {
    let p = blank(src, p);
    if !src[p..].starts_with("title") {
        return None;
    }
    let at = p + 5;
    Some(
        if src
            .as_bytes()
            .get(at)
            .is_some_and(|c| matches!(c, b' ' | b'\t'))
        {
            to_comment(src, at + 1)
        } else {
            at
        },
    )
}
fn string(src: &str, p: usize) -> Option<usize> {
    let quote = *src.as_bytes().get(p)?;
    if !matches!(quote, b'\'' | b'"') {
        return None;
    }
    let mut at = p + 1;
    while at < src.len() {
        let c = src[at..].chars().next()?;
        if c as u32 == quote as u32 {
            return Some(at + 1);
        }
        at += c.len_utf8();
        if c == '\\' {
            let next = src[at..].chars().next()?;
            if matches!(next, '\n' | '\r' | '\u{2028}' | '\u{2029}') {
                return None;
            }
            at += next.len_utf8()
        }
    }
    None
}
fn newline(src: &str, p: usize) -> Option<usize> {
    if src[p..].starts_with("\r\n") {
        Some(p + 2)
    } else if src[p..].starts_with('\n') {
        Some(p + 1)
    } else {
        None
    }
}
fn whitespace(src: &str, p: usize) -> Option<usize> {
    let end = blank(src, p);
    (end > p).then_some(end)
}
fn yaml_open(src: &str, p: usize) -> Option<usize> {
    if !src[p..].starts_with("---") {
        return None;
    }
    let end = blank(src, p + 3);
    newline(src, end)
}
fn yaml(src: &str, p: usize) -> Option<usize> {
    super::hidden_yaml_end(src, p)
}
fn directive_open(src: &str, p: usize) -> Option<usize> {
    let p = blank(src, p);
    src[p..].starts_with("%%{").then_some(p + 3)
}
fn directive(src: &str, p: usize) -> Option<usize> {
    let after = directive_open(src, p)?;
    let mut from = after;
    while let Some(end) = src[from..].find("}%%").map(|i| from + i + 3) {
        if let Some(end) = newline(src, end) {
            return Some(end);
        }
        if src[end..].chars().next().is_none_or(space) {
            return Some(end);
        }
        from = end;
    }
    None
}
fn comment(src: &str, p: usize) -> Option<usize> {
    let p = blank(src, p);
    src[p..].starts_with("%%").then(|| line_end(src, p))
}
pub(crate) const ACC_DESCR: TokenSpec =
    TokenSpec::matcher("ACC_DESCR", acc_descr, false, Some("\t a"), None);
pub(crate) const ACC_DESCR_BLOCK: TokenSpec = TokenSpec {
    opener: Some(acc_descr_open),
    ..ACC_DESCR
};
pub(crate) const ACC_TITLE: TokenSpec =
    TokenSpec::matcher("ACC_TITLE", acc_title, false, Some("\t a"), None);
pub(crate) const TITLE: TokenSpec = TokenSpec::matcher("title", title, false, Some("\t t"), None);
pub(crate) const STRING: TokenSpec = TokenSpec::matcher("STRING", string, false, Some("\"'"), None);
pub(crate) const NEWLINE: TokenSpec =
    TokenSpec::matcher("NEWLINE", newline, false, Some("\r\n"), None);
pub(crate) const WHITESPACE: TokenSpec =
    TokenSpec::matcher("WHITESPACE", whitespace, true, Some("\t "), None);
pub(crate) const YAML: TokenSpec =
    TokenSpec::matcher("YAML", yaml, true, Some("-"), Some(yaml_open));
pub(crate) const DIRECTIVE: TokenSpec = TokenSpec::matcher(
    "DIRECTIVE",
    directive,
    true,
    Some("\t %"),
    Some(directive_open),
);
pub(crate) const SINGLE_LINE_COMMENT: TokenSpec =
    TokenSpec::matcher("SINGLE_LINE_COMMENT", comment, true, Some("\t %"), None);
fn match_at(src: &str, p: usize, spec: &TokenSpec) -> Option<usize> {
    match spec.pattern {
        Pattern::Literal(l) => src[p..].starts_with(l).then_some(p + l.len()),
        Pattern::Match(m) => m(src, p),
    }
}
fn match_budgeted(
    src: &str,
    p: usize,
    kind: usize,
    types: &[TokenSpec],
    dead: &mut [bool],
) -> Option<usize> {
    if dead[kind] {
        return None;
    }
    let spec = &types[kind];
    let matched = match_at(src, p, spec).filter(|end| *end > p);
    if matched.is_none() && spec.opener.is_some_and(|opener| opener(src, p).is_some()) {
        dead[kind] = true;
    }
    matched
}
pub(crate) fn tokenize(
    src: &str,
    types: &[TokenSpec],
    diagram: &str,
) -> Result<Vec<Token>, PeleError> {
    let mut tokens = Vec::new();
    let mut dead = vec![false; types.len()];
    let mut p = 0;
    while p < src.len() {
        let c = src[p..].chars().next().unwrap();
        let mut found = None;
        for (k, spec) in types.iter().enumerate() {
            if dead[k] {
                continue;
            }
            if c.is_ascii() {
                let allowed = match spec.pattern {
                    Pattern::Literal(l) => l.starts_with(c),
                    Pattern::Match(_) => spec.first.is_none_or(|s| s.contains(c)),
                };
                if !allowed {
                    continue;
                }
            }
            let Some(mut end) = match_budgeted(src, p, k, types, &mut dead) else {
                continue;
            };
            let mut kind = k;
            for &alt in spec.longer {
                if let Some(alt_end) =
                    match_budgeted(src, p, alt, types, &mut dead).filter(|to| *to > end)
                {
                    end = alt_end;
                    kind = alt;
                    break;
                }
            }
            found = Some((kind, end));
            break;
        }
        let Some((kind, end)) = found else {
            return Err(PeleError::syntax(diagram, src, p, "", true));
        };
        if !types[kind].hidden {
            tokens.push(Token {
                kind,
                start: p,
                end,
            })
        }
        p = end
    }
    tokens.push(Token {
        kind: END,
        start: src.len(),
        end: src.len(),
    });
    Ok(tokens)
}
pub(crate) struct Reader<'a> {
    pub src: &'a str,
    pub tokens: Vec<Token>,
    pub i: usize,
    pub types: &'static [TokenSpec],
    pub diagram: &'static str,
}
impl<'a> Reader<'a> {
    pub fn new(
        src: &'a str,
        types: &'static [TokenSpec],
        diagram: &'static str,
    ) -> Result<Self, PeleError> {
        Ok(Self {
            src,
            tokens: tokenize(src, types, diagram)?,
            i: 0,
            types,
            diagram,
        })
    }
    pub fn kind(&self) -> usize {
        self.kind_at(self.i)
    }
    pub fn kind_at(&self, i: usize) -> usize {
        self.tokens.get(i).map_or(END, |t| t.kind)
    }
    pub fn text(&self) -> &'a str {
        let t = &self.tokens[self.i];
        &self.src[t.start..t.end]
    }
    pub fn take(&mut self) -> &'a str {
        let text = self.text();
        self.i += 1;
        text
    }
    pub fn accept(&mut self, kind: usize) -> bool {
        if self.kind() != kind {
            return false;
        }
        self.i += 1;
        true
    }
    pub fn expect(&mut self, kind: usize) -> Result<&'a str, PeleError> {
        if self.kind() != kind {
            return Err(self.fail(&format!("token of type '{}'", self.types[kind].name)));
        }
        Ok(self.take())
    }
    pub fn fail(&self, expected: &str) -> PeleError {
        let found = if self.kind() == END {
            "end of input".into()
        } else {
            format!("`{}`", self.text())
        };
        PeleError::syntax(
            self.diagram,
            self.src,
            self.tokens[self.i].start,
            &format!("Expecting {expected} but found {found}."),
            false,
        )
    }
}
pub(crate) fn single(s: &str) -> String {
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
pub(crate) fn string_value(s: &str) -> String {
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
pub(crate) fn acc_descr_value(s: &str) -> String {
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

pub(crate) fn title_value(text: &str) -> String {
    single(&text.trim_start_matches([' ', '\t'])[5..])
}
pub(crate) fn acc_title_value(text: &str) -> String {
    let s = text.trim_start_matches([' ', '\t']);
    let tail = s[8..].trim_start_matches([' ', '\t']);
    single(&tail[1..])
}
