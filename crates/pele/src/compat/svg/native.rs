//! Resolved native export over the compatible SVG. Source text is never rewritten.
use crate::compat::{ErrorCode, PeleError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeTheme {
    pub background: String,
    pub foreground: String,
    pub muted: String,
    pub line: String,
    pub surface: String,
    pub surface_alt: String,
    pub border: String,
    pub accent: String,
    pub series: [String; 8],
}
impl Default for NativeTheme {
    fn default() -> Self {
        Self {
            background: "#ffffff".into(),
            foreground: "#1f1f1f".into(),
            muted: "#6e6e6e".into(),
            line: "#1f1f1f".into(),
            surface: "#f3f3f3".into(),
            surface_alt: "#e6e6e6".into(),
            border: "#8a8a8a".into(),
            accent: "#5b7bd5".into(),
            series: [
                "#4c78a8", "#f58518", "#54a24b", "#e45756", "#72b7b2", "#eeca3b", "#b279a2",
                "#9d755d",
            ]
            .map(str::to_string),
        }
    }
}
impl NativeTheme {
    fn tokens(&self) -> Result<BTreeMap<String, String>, PeleError> {
        let mut values = BTreeMap::new();
        for (name, color) in [
            ("bg", &self.background),
            ("fg", &self.foreground),
            ("muted", &self.muted),
            ("line", &self.line),
            ("surface", &self.surface),
            ("surface-alt", &self.surface_alt),
            ("border", &self.border),
            ("accent", &self.accent),
        ] {
            values.insert(format!("--pele-{name}"), color.clone());
        }
        for (i, color) in self.series.iter().enumerate() {
            values.insert(format!("--pele-series-{}", i + 1), color.clone());
        }
        if values.values().any(|v| {
            v.len() != 7
                || !v.starts_with('#')
                || !v.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
        }) {
            return Err(PeleError::new(
                "Native theme colors must be #RRGGBB",
                ErrorCode::Semantic,
            ));
        }
        Ok(values)
    }
}
fn resolve(
    value: &str,
    tokens: &BTreeMap<String, String>,
    depth: usize,
) -> Result<String, PeleError> {
    if depth > 32 {
        return Err(PeleError::new(
            "Native SVG token nesting exceeds 32",
            ErrorCode::Limit,
        ));
    }
    let mut out = String::new();
    let mut from = 0;
    while let Some(start) = value[from..].find("var(").map(|i| from + i) {
        out.push_str(&value[from..start]);
        let mut level = 1;
        let mut comma = None;
        let mut end = None;
        for (i, c) in value[start + 4..].char_indices() {
            let i = start + 4 + i;
            match c {
                '(' => level += 1,
                ')' => {
                    level -= 1;
                    if level == 0 {
                        end = Some(i);
                        break;
                    }
                }
                ',' if level == 1 && comma.is_none() => comma = Some(i),
                _ => {}
            }
        }
        let end = end.ok_or_else(|| {
            PeleError::new(
                "Native SVG contains an unclosed CSS token",
                ErrorCode::Semantic,
            )
        })?;
        let key = value[start + 4..comma.unwrap_or(end)].trim();
        let replacement = tokens
            .get(key)
            .map(String::as_str)
            .or_else(|| comma.map(|i| value[i + 1..end].trim()))
            .ok_or_else(|| {
                PeleError::new(
                    format!("Native SVG requires a value for CSS token {key}"),
                    ErrorCode::Semantic,
                )
            })?;
        out.push_str(&resolve(replacement, tokens, depth + 1)?);
        from = end + 1;
    }
    out.push_str(&value[from..]);
    Ok(out)
}

fn tags(svg: &str) -> Vec<std::ops::Range<usize>> {
    let mut out = Vec::new();
    let mut cursor = 0;
    while let Some(start) = svg[cursor..].find('<').map(|i| cursor + i) {
        let mut p = start + 1;
        let mut quote = 0u8;
        while p < svg.len() {
            let c = svg.as_bytes()[p];
            if quote != 0 {
                if c == quote {
                    quote = 0
                }
            } else if matches!(c, b'"' | b'\'') {
                quote = c
            } else if c == b'>' {
                p += 1;
                break;
            }
            p += 1
        }
        if p > svg.len() {
            break;
        }
        out.push(start..p);
        cursor = p;
        if cursor >= svg.len() {
            break;
        }
    }
    out
}
struct Attribute<'a> {
    name: &'a str,
    from: usize,
    to: usize,
}
fn attributes(tag: &str) -> Vec<Attribute<'_>> {
    let b = tag.as_bytes();
    let mut p = 1;
    while p < b.len() && !b[p].is_ascii_whitespace() && b[p] != b'>' {
        p += 1
    }
    let mut out = Vec::new();
    while p < b.len() {
        while p < b.len() && b[p].is_ascii_whitespace() {
            p += 1
        }
        let start = p;
        while p < b.len() && !b[p].is_ascii_whitespace() && !matches!(b[p], b'=' | b'>' | b'/') {
            p += 1
        }
        if p == start {
            break;
        }
        let name = &tag[start..p];
        while p < b.len() && b[p].is_ascii_whitespace() {
            p += 1
        }
        if b.get(p) != Some(&b'=') {
            continue;
        }
        p += 1;
        while p < b.len() && b[p].is_ascii_whitespace() {
            p += 1
        }
        let Some(&quote) = b.get(p).filter(|q| matches!(q, b'"' | b'\'')) else {
            break;
        };
        p += 1;
        let from = p;
        while p < b.len() && b[p] != quote {
            p += 1
        }
        if p == b.len() {
            break;
        }
        out.push(Attribute { name, from, to: p });
        p += 1;
    }
    out
}
pub(crate) fn resolved_svg(svg: &str, theme: &NativeTheme) -> Result<String, PeleError> {
    let ranges = tags(svg);
    let mut tokens = theme.tokens()?;
    if let Some(root) = ranges.first() {
        let root = &svg[root.clone()];
        for attr in attributes(root) {
            if attr.name == "style" {
                for decl in root[attr.from..attr.to].split(';') {
                    if let Some((key, value)) = decl.split_once(':') {
                        if key.trim().starts_with("--_") {
                            tokens.insert(key.trim().into(), value.trim().into());
                        }
                    }
                }
                break;
            }
        }
    }
    let mut out = String::new();
    let mut cursor = 0;
    for range in ranges {
        out.push_str(&svg[cursor..range.start]);
        let tag = &svg[range.clone()];
        let mut last = 0;
        for attr in attributes(tag) {
            if !matches!(
                attr.name,
                "style" | "fill" | "stroke" | "font-family" | "font-weight" | "rx" | "ry"
            ) {
                continue;
            }
            out.push_str(&tag[last..attr.from]);
            let value = &tag[attr.from..attr.to];
            let mut value = resolve(value, &tokens, 0)?;
            if attr.name == "style" {
                value = value
                    .split(';')
                    .filter(|v| !v.trim_start().starts_with("--_"))
                    .collect::<Vec<_>>()
                    .join(";")
            }
            out.push_str(&value);
            last = attr.to;
        }
        out.push_str(&tag[last..]);
        cursor = range.end;
    }
    out.push_str(&svg[cursor..]);
    Ok(out)
}
