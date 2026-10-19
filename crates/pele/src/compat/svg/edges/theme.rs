//! Remaining flow style utilities from Pele `src/svg/theme.ts`, MIT (kepano),
//! commit 458322a5b8abb1ae57ddf127c32017ab9fb14a92. Root/font/series helpers live in the parent.
use super::super::esc;
use crate::compat::{source::space, text::SPACE};
use regex::Regex;
use std::sync::OnceLock;

#[derive(Clone, Debug, Default)]
pub(crate) struct ResolvedStyle {
    pub shape: String,
    pub line: String,
    pub text: String,
    pub stroke: Option<String>,
    pub bold: bool,
    pub italic: bool,
    pub font_size: Option<f64>,
}
const SHAPE_PROPS: &[&str] = &[
    "fill",
    "fill-opacity",
    "stroke",
    "stroke-width",
    "stroke-dasharray",
    "stroke-dashoffset",
    "stroke-linecap",
    "stroke-linejoin",
    "stroke-opacity",
    "opacity",
    "rx",
    "ry",
];
const TEXT_PROPS: &[&str] = &[
    "color",
    "font-size",
    "font-family",
    "font-weight",
    "font-style",
    "text-decoration",
    "letter-spacing",
    "word-spacing",
    "opacity",
];
fn set(entries: &mut Vec<(String, String)>, prop: &str, value: &str) {
    if let Some((_, old)) = entries.iter_mut().find(|(key, _)| key == prop) {
        *old = value.into();
    } else {
        entries.push((prop.into(), value.into()));
    }
}
fn get<'a>(entries: &'a [(String, String)], prop: &str) -> Option<&'a str> {
    entries
        .iter()
        .find(|(key, _)| key == prop)
        .map(|(_, value)| value.as_str())
}
fn css_attr(css: String) -> String {
    if css.is_empty() {
        String::new()
    } else {
        format!(" style=\"{}\"", esc(&css))
    }
}
// Number coercion used only by Pele's font-weight comparison.
fn numeric_weight(s: &str) -> f64 {
    let s = s.trim_matches(space);
    if s.is_empty() {
        return 0.;
    }
    let radix = if s.starts_with("0x") || s.starts_with("0X") {
        Some(16)
    } else if s.starts_with("0o") || s.starts_with("0O") {
        Some(8)
    } else if s.starts_with("0b") || s.starts_with("0B") {
        Some(2)
    } else {
        None
    };
    if let Some(radix) = radix {
        if s.len() == 2 {
            return f64::NAN;
        }
        let mut value = 0.;
        for c in s[2..].chars() {
            let Some(digit) = c.to_digit(radix) else {
                return f64::NAN;
            };
            value = value * radix as f64 + digit as f64;
        }
        return value;
    }
    match s {
        "Infinity" | "+Infinity" => f64::INFINITY,
        "-Infinity" => f64::NEG_INFINITY,
        _ if s
            .chars()
            .any(|c| !matches!(c, '0'..='9' | '+' | '-' | '.' | 'e' | 'E')) =>
        {
            f64::NAN
        }
        _ => s.parse().unwrap_or(f64::NAN),
    }
}
pub(crate) fn resolve_style(declarations: &[String]) -> ResolvedStyle {
    static UNSAFE: OnceLock<Regex> = OnceLock::new();
    static IMPORTANT: OnceLock<Regex> = OnceLock::new();
    static SIZE: OnceLock<Regex> = OnceLock::new();
    let unsafe_value = UNSAFE.get_or_init(|| {
        Regex::new(&format!(
            r"(?i-u:url){SPACE}*\(|(?i-u:expression){SPACE}*\(|(?i-u:javascript:)|[<>{{}};\\]|(?i-u:@import|image-set)|/\*"
        ))
        .unwrap()
    });
    let important = IMPORTANT.get_or_init(|| Regex::new(r"(?i-u:!important)").unwrap());
    let size_re = SIZE.get_or_init(|| Regex::new(r"^[0-9.]+(px)?$").unwrap());
    let mut shape = Vec::new();
    let mut text = Vec::new();
    for decl in declarations {
        let Some(at) = decl.find(':').filter(|at| *at > 0) else {
            continue;
        };
        let prop = decl[..at].trim_matches(space).to_lowercase();
        let removed = important.replace_all(&decl[at + 1..], "");
        let value = removed.trim_matches(space);
        if value.is_empty() || unsafe_value.is_match(value) {
            continue;
        }
        if SHAPE_PROPS.contains(&prop.as_str()) {
            set(&mut shape, &prop, value);
        }
        if TEXT_PROPS.contains(&prop.as_str()) {
            set(
                &mut text,
                if prop == "color" { "fill" } else { &prop },
                value,
            );
        }
    }
    let mut shape_css = String::new();
    let mut line_css = String::new();
    for (prop, value) in &shape {
        shape_css.push_str(&format!("{prop}:{value};"));
        if prop != "fill" && prop != "fill-opacity" {
            line_css.push_str(&format!("{prop}:{value};"));
        }
    }
    let mut text_css = String::new();
    for (prop, value) in &text {
        text_css.push_str(&format!("{prop}:{value};"));
    }
    let weight = get(&text, "font-weight");
    let bold = matches!(weight, Some("bold" | "bolder"))
        || weight.is_some_and(|w| numeric_weight(w) >= 600.);
    let size = get(&text, "font-size").filter(|s| size_re.is_match(s));
    // JS parseFloat consumes the longest numeric prefix, including values such as `12.3.4px`.
    let font_size = size
        .and_then(|s| {
            let mut dot = false;
            let end = s
                .char_indices()
                .find_map(|(i, c)| {
                    if c == '.' && !dot {
                        dot = true;
                        None
                    } else if c.is_ascii_digit() {
                        None
                    } else {
                        Some(i)
                    }
                })
                .unwrap_or(s.len());
            s[..end].parse::<f64>().ok()
        })
        .filter(|n| *n > 0. && *n <= 200.);
    ResolvedStyle {
        shape: css_attr(shape_css),
        line: css_attr(line_css),
        text: css_attr(text_css),
        stroke: get(&shape, "stroke").map(str::to_owned),
        bold,
        italic: get(&text, "font-style") == Some("italic"),
        font_size,
    }
}
pub(crate) fn class_names(raw: &str) -> String {
    let mut cleaned = String::new();
    let mut in_gap = false;
    for c in raw.chars() {
        if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
            cleaned.push(c);
            in_gap = false;
        } else if !in_gap {
            cleaned.push(' ');
            in_gap = true;
        }
    }
    let cleaned = cleaned.trim();
    if cleaned.is_empty() {
        String::new()
    } else {
        format!(" {cleaned}")
    }
}
