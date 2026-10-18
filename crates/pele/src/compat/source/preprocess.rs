//! Translation of Pele src/preprocess.ts at 458322a5 (MIT, kepano).
use super::{
    detect::{space, word},
    yaml,
};
use crate::compat::{ErrorCode, Object, PeleError, Value};
use serde::Serialize;
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedSource {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub config: Object,
    pub with_comments: String,
    pub line_offset: usize,
}
fn closing_end(text: &str, i: usize, fence: &str) -> Option<usize> {
    if text.as_bytes().get(i) != Some(&b'\n') || !text[i + 1..].starts_with(fence) {
        return None;
    }
    let mut last = None;
    for (j, c) in text[i + 1 + fence.len()..].char_indices() {
        if !space(c) {
            break;
        }
        if c == '\n' {
            last = Some(i + 1 + fence.len() + j)
        }
    }
    last
}
fn front_matter(text: &str) -> Option<(&str, &str, usize)> {
    let cursor = text
        .char_indices()
        .find(|(_, c)| !space(*c) || *c == '\n')
        .map_or(text.len(), |(i, _)| i);
    let indent = &text[..cursor];
    if !text[cursor..].starts_with("---") {
        return None;
    }
    let opening: Vec<_> = text[cursor + 3..]
        .char_indices()
        .take_while(|(_, c)| space(*c))
        .filter(|(_, c)| *c == '\n')
        .map(|(i, _)| cursor + 3 + i)
        .collect();
    let first = *opening.first()?;
    let fence = format!("{indent}---");
    let needle = format!("\n{fence}");
    let last = text
        .rmatch_indices(&needle)
        .map(|(i, _)| i)
        .find(|i| *i > first && closing_end(text, *i, &fence).is_some())?;
    let opening_end = *opening.iter().filter(|i| **i < last).next_back()?;
    let body_start = opening_end + 1;
    for (i, _) in text[body_start.saturating_sub(1)..].match_indices(&needle) {
        let i = i + body_start.saturating_sub(1);
        if i > last {
            break;
        }
        if i < body_start {
            continue;
        }
        if let Some(end) = closing_end(text, i, &fence) {
            return Some((indent, &text[body_start..i], end + 1));
        }
    }
    None
}
fn html_quotes(source: &str) -> String {
    let Some(last) = source.rfind('>').filter(|i| *i > 0) else {
        return source.into();
    };
    if !source.contains('<') {
        return source.into();
    }
    let mut out = String::new();
    let mut at = 0;
    while let Some(i) = source[at..last + 1].find('<').map(|i| i + at) {
        out.push_str(&source[at..i]);
        let mut name_end = i + 1;
        while source
            .as_bytes()
            .get(name_end)
            .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_')
        {
            name_end += 1
        }
        if name_end == i + 1 {
            out.push('<');
            at = i + 1;
            continue;
        }
        let Some(close) = source[name_end..last + 1].find('>').map(|n| name_end + n) else {
            out.push_str(&source[i..]);
            return out;
        };
        out.push_str(&source[i..name_end]);
        let attrs = &source[name_end..close];
        let mut a = 0;
        while let Some(j) = attrs[a..].find("=\"").map(|j| a + j) {
            out.push_str(&attrs[a..j]);
            if let Some(end) = attrs[j + 2..].find('"').map(|e| e + j + 2) {
                out.push_str("='");
                out.push_str(&attrs[j + 2..end]);
                out.push('\'');
                a = end + 1
            } else {
                a = j;
                break;
            }
        }
        out.push_str(&attrs[a..]);
        out.push('>');
        at = close + 1;
        if at > last {
            break;
        }
    }
    out.push_str(&source[at..]);
    out
}
struct Directive<'a> {
    from: usize,
    to: usize,
    kind: &'a str,
    body: Option<&'a str>,
}
fn skip_space(text: &str, at: usize) -> usize {
    text[at..]
        .char_indices()
        .find(|(_, c)| !space(*c))
        .map_or(text.len(), |(i, _)| at + i)
}
fn word_end(text: &str, at: usize) -> usize {
    text[at..]
        .char_indices()
        .find(|(_, c)| !word(*c))
        .map_or(text.len(), |(i, _)| at + i)
}
fn directive_matches(text: &str) -> Vec<Directive<'_>> {
    let mut out = Vec::new();
    let mut cursor = 0;
    while let Some(from) = text[cursor..].find("%%{").map(|i| cursor + i) {
        let start = skip_space(text, from + 3);
        let end = word_end(text, start);
        if start == end {
            cursor = from + 3;
            continue;
        }
        let kind = &text[start..end];
        let after = skip_space(text, end);
        let body_at = if text.as_bytes().get(after) == Some(&b':') {
            skip_space(text, after + 1)
        } else {
            after
        };
        let body_end = word_end(text, body_at);
        let (body, mut to) = if body_end > body_at {
            (None, body_end)
        } else {
            let mut end = body_at;
            for (i, c) in text[body_at..].char_indices() {
                let at = body_at + i;
                if text[at..].starts_with("}%%") || matches!(c, '\u{2028}' | '\u{2029}' | '\r') {
                    break;
                }
                end = at + c.len_utf8()
            }
            (Some(&text[body_at..end]), end)
        };
        to = skip_space(text, to);
        if text[to..].starts_with("}%%") {
            to += 3
        }
        out.push(Directive {
            from,
            to,
            kind,
            body,
        });
        cursor = to;
    }
    out
}
fn directives(text: &str, config: &mut Object) -> Result<String, PeleError> {
    if !text.contains("%%{") {
        return Ok(text.into());
    }
    let normalized = text.trim_matches(space).replace('\'', "\"");
    for d in directive_matches(&normalized) {
        if d.kind == "wrap" {
            config.insert("wrap".into(), Value::Bool(true))
        } else if matches!(d.kind, "init" | "initialize") {
            if let Some(body) = d.body {
                if crate::compat::value::json_depth_exceeds(body, 64) {
                    return Err(PeleError::new(
                        "Diagram is nested too deeply.",
                        ErrorCode::Limit,
                    ));
                }
                if let Ok(Value::Object(o)) =
                    crate::compat::value::parse_json(body.trim_matches(space))
                {
                    config.merge(&o)
                }
            }
        }
    }
    let mut out = String::new();
    let mut cursor = 0;
    for d in directive_matches(text) {
        out.push_str(&text[cursor..d.from]);
        cursor = d.to
    }
    out.push_str(&text[cursor..]);
    Ok(out)
}
fn cleanup_comments(text: &str) -> String {
    if !text.contains("%%") {
        return text.trim_start_matches(space).into();
    }
    let mut out = String::new();
    let mut at = 0;
    let mut cursor = 0;
    loop {
        let p = skip_space(text, at);
        if text[p..].starts_with("%%")
            && !text[p + 2..].starts_with('{')
            && text.as_bytes().get(p + 2).is_some_and(|c| *c != b'\n')
        {
            let end = text[p + 2..]
                .find('\n')
                .map_or(text.len(), |i| p + 2 + i + 1);
            out.push_str(&text[cursor..at]);
            cursor = end;
            at = end;
        } else {
            let Some(next) = text[p..]
                .find(['\n', '\u{2028}', '\u{2029}'])
                .map(|i| p + i + text[p + i..].chars().next().unwrap().len_utf8())
            else {
                break;
            };
            at = next;
        }
        if at >= text.len() {
            break;
        }
    }
    out.push_str(&text[cursor..]);
    out.trim_start_matches(space).into()
}
pub(crate) fn preprocess(source: &str) -> Result<PreparedSource, PeleError> {
    let mut text = html_quotes(&source.replace("\r\n", "\n").replace('\r', "\n"));
    let mut config = Object::default();
    let mut title = None;
    let mut line_offset = 0;
    if let Some((indent, body, length)) = front_matter(&text) {
        let body = if indent.is_empty() {
            body.to_string()
        } else {
            body.split('\n')
                .map(|l| l.strip_prefix(indent).unwrap_or(l))
                .collect::<Vec<_>>()
                .join("\n")
        };
        let parsed = yaml::parse(&body).map_err(|e| {
            PeleError::new(
                format!("Front matter is not valid YAML. {e}"),
                ErrorCode::Syntax,
            )
        })?;
        if let Value::Object(parsed) = parsed {
            if let Some(t) = parsed.get("title").filter(|t| t.truthy()) {
                title = Some(t.js_string())
            }
            if let Some(Value::Object(c)) = parsed.get("config") {
                config.merge(c)
            }
            if let Some(mode) = parsed.get("displayMode").filter(|v| v.truthy()) {
                if !matches!(config.get("gantt"), Some(Value::Object(_))) {
                    config.insert("gantt".into(), Value::Object(Object::default()))
                }
                if let Some(Value::Object(gantt)) = config.get_mut("gantt") {
                    gantt.insert("displayMode".into(), Value::String(mode.js_string()))
                }
            }
        }
        line_offset = text[..length].bytes().filter(|c| *c == b'\n').count();
        text = text[length..].into();
    }
    text = directives(&text, &mut config)?;
    Ok(PreparedSource {
        text: cleanup_comments(&text),
        with_comments: text,
        title,
        config,
        line_offset,
    })
}
