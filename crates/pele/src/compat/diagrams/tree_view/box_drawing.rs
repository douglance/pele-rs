//! Pele treeview/boxDrawing.ts, 458322a5 (MIT, kepano).
use crate::compat::{source::space, ErrorCode, PeleError};
use std::collections::HashMap;
fn branch(c: char) -> bool {
    matches!(c, '└' | '┗' | '├' | '┣')
}
fn dash(c: char) -> bool {
    matches!(c, '─' | '━')
}
fn vertical(c: char) -> bool {
    matches!(c, '│' | '┃')
}
fn any(c: char) -> bool {
    branch(c) || dash(c) || vertical(c)
}
fn metadata(line: &str) -> bool {
    let s = line.trim_start_matches(space);
    if let Some(t) = s.strip_prefix("title") {
        return t.starts_with([' ', '\t']);
    }
    for key in ["accTitle", "accDescr"] {
        if let Some(t) = s.strip_prefix(key) {
            let t = t.trim_start_matches([' ', '\t']);
            if t.starts_with(':') || (key == "accDescr" && t.starts_with('{')) {
                return true;
            }
        }
    }
    false
}
fn passes(line: &str) -> bool {
    let s = line.trim_matches(space);
    s.is_empty() || s.starts_with("%%") || metadata(line)
}
fn decoration(line: &str) -> bool {
    !line.is_empty() && line.chars().all(|c| space(c) || vertical(c))
}
fn fail(line: usize, message: &str) -> PeleError {
    let mut e = PeleError::new(format!("Line {line}: {message}"), ErrorCode::Syntax);
    e.diagram_type = Some("treeView".into());
    e.line = line;
    e
}
pub(super) fn preprocess(input: &str) -> Result<(String, HashMap<usize, usize>), PeleError> {
    let mut map = HashMap::new();
    if !input.chars().any(any) {
        return Ok((input.into(), map));
    }
    let lines: Vec<_> = input.split('\n').collect();
    let Some(keyword) = lines
        .iter()
        .position(|line| line.trim_matches(space) == "treeView-beta")
    else {
        return Ok((input.into(), map));
    };
    let content = lines
        .iter()
        .skip(keyword + 1)
        .filter(|line| !passes(line) && !decoration(line))
        .map(|line| line.replace('\t', "    "))
        .collect::<Vec<_>>();
    if !content.iter().any(|line| line.chars().any(any)) {
        return Ok((input.into(), map));
    }
    let segment = content
        .iter()
        .find_map(|line| {
            let at = line
                .char_indices()
                .find(|(_, c)| branch(*c))
                .map(|(i, _)| line[..i].encode_utf16().count())?;
            (at > 0).then_some(at)
        })
        .unwrap_or(4);
    let mut out = vec![];
    for (i, line) in lines.iter().enumerate() {
        let keep = if i <= keyword || passes(line) {
            Some((*line).to_owned())
        } else if decoration(line) {
            None
        } else {
            let normalized = line.replace('\t', "    ");
            if let Some((at, _)) = normalized.char_indices().find(|(_, c)| branch(*c)) {
                let index = normalized[..at].encode_utf16().count();
                let depth = (index as f64 / segment as f64 + 0.5).floor() as usize + 1;
                let mut pos = at + normalized[at..].chars().next().unwrap().len_utf8();
                while normalized[pos..].chars().next().is_some_and(dash) {
                    pos += normalized[pos..].chars().next().unwrap().len_utf8();
                }
                while normalized.as_bytes().get(pos) == Some(&b' ') {
                    pos += 1;
                }
                let value = normalized[pos..].trim_end_matches(space);
                if value.is_empty() {
                    return Err(fail(i+1,"Empty node — expected a filename or directory name after the box-drawing prefix"));
                }
                Some(format!("{}{}", "    ".repeat(depth), value))
            } else if !normalized.is_empty() && normalized.chars().all(|c| space(c) || any(c)) {
                None
            } else if normalized.chars().any(any) {
                Some((*line).into())
            } else if normalized.chars().next().is_some_and(space) {
                return Err(fail(i+1,"Unexpected indentation without box-drawing characters. In box-drawing format, use ├── or └── prefixes for indented nodes."));
            } else {
                Some((*line).into())
            }
        };
        if let Some(line) = keep {
            out.push(line);
            map.insert(out.len(), i + 1);
        }
    }
    Ok((out.join("\n"), map))
}
