//! Ordered TreeView Langium lexer/parser plus indentation database, Pele 458322a5 (MIT).
use super::{box_drawing, NodeType, TreeNode, TreeViewModel};
use crate::compat::{
    diagrams::common::langium::{self as l, Pattern, Reader, Token, TokenSpec, END},
    source::{space, word},
    DiagramType, PeleError,
};
fn blank(c: char) -> bool {
    matches!(c, ' ' | '\t')
}
fn annotation(src: &str, at: usize) -> bool {
    let tail = &src[at..];
    if tail.starts_with("##") || tail.starts_with("icon(") {
        return true;
    }
    let Some(after) = tail.strip_prefix(":::") else {
        return false;
    };
    after
        .trim_start_matches(blank)
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
}
fn bare(src: &str, at: usize) -> Option<usize> {
    let first = src[at..].chars().next()?;
    if blank(first)
        || matches!(first, '\n' | '\r' | '"' | '\'')
        || src[at..].starts_with(":::")
        || src[at..].starts_with("icon(")
        || src[at..].starts_with("##")
    {
        return None;
    }
    let mut i = at + first.len_utf8();
    while let Some(c) = src[i..].chars().next() {
        if matches!(c, '\n' | '\r') {
            break;
        }
        if blank(c) {
            let j = l::blank(src, i);
            if annotation(src, j) {
                break;
            }
            i = j;
        } else {
            i += c.len_utf8();
        }
    }
    Some(i)
}
fn keyword(src: &str, at: usize) -> Option<usize> {
    l::keyword(src, at, "treeView-beta")
}
fn class(src: &str, at: usize) -> Option<usize> {
    let mut q = l::blank(src, at);
    if q == at || !src[q..].starts_with(":::") {
        return None;
    }
    q = l::blank(src, q + 3);
    let first = src[q..].chars().next()?;
    if !first.is_ascii_alphabetic() && first != '_' {
        return None;
    }
    q += 1;
    while let Some(c) = src[q..].chars().next().filter(|c| word(*c) || *c == '-') {
        q += c.len_utf8();
    }
    Some(q)
}
fn icon(src: &str, at: usize) -> Option<usize> {
    let mut q = l::blank(src, at);
    if q == at || !src[q..].starts_with("icon(") {
        return None;
    }
    q += 5;
    while let Some(c) = src[q..].chars().next().filter(|c| word(*c) || *c == '-') {
        q += c.len_utf8();
    }
    if src[q..].starts_with(':') {
        q += 1;
        let from = q;
        while let Some(c) = src[q..].chars().next().filter(|c| word(*c) || *c == '-') {
            q += c.len_utf8();
        }
        if q == from {
            return None;
        }
    }
    src[q..].starts_with(')').then_some(q + 1)
}
fn descr(src: &str, at: usize) -> Option<usize> {
    let q = l::blank(src, at);
    (q > at && src[q..].starts_with("##")).then(|| l::line_end(src, q + 2))
}
fn indentation(src: &str, at: usize) -> Option<usize> {
    let q = l::blank(src, at);
    (q > at).then_some(q)
}
fn quoted(src: &str, at: usize) -> Option<usize> {
    let quote = src[at..].chars().next()?;
    if !matches!(quote, '"' | '\'') {
        return None;
    }
    src[at + 1..].find(quote).map(|i| at + i + 2)
}
fn comment(src: &str, at: usize) -> Option<usize> {
    src[at..]
        .starts_with("%%")
        .then(|| src[at..].find('\n').map_or(src.len(), |i| at + i))
}
static TOKENS: &[TokenSpec] = &[
    TokenSpec {
        longer: &[12],
        ..TokenSpec::matcher("treeView-beta", keyword, false, Some("t"), None)
    },
    l::ACC_DESCR,
    l::ACC_TITLE,
    l::TITLE,
    TokenSpec::matcher("CLASS_ANNOTATION", class, false, Some(" \t"), None),
    TokenSpec::matcher("ICON_ANNOTATION", icon, false, Some(" \t"), None),
    TokenSpec::matcher("DESC_ANNOTATION", descr, false, Some(" \t"), None),
    TokenSpec::matcher("INDENTATION", indentation, false, Some(" \t"), None),
    TokenSpec::matcher("QUOTED_NAME", quoted, false, Some("\"'"), None),
    TokenSpec::matcher("WS", indentation, true, Some(" \t"), None),
    TokenSpec::matcher("ML_COMMENT", comment, true, Some("%"), None),
    TokenSpec {
        hidden: true,
        ..l::NEWLINE
    },
    TokenSpec::matcher("BARE_NAME", bare, false, None, None),
];
fn acc(src: &str, at: usize, last: Option<usize>) -> Option<usize> {
    let p = l::blank(src, at);
    if !src[p..].starts_with("accDescr") {
        return None;
    }
    let after = p + 8;
    let colon = l::blank(src, after);
    if src[colon..].starts_with(':') {
        let end = l::line_end(src, colon + 1);
        return Some(
            src[colon + 1..end]
                .find("%%")
                .map_or(end, |i| colon + 1 + i),
        );
    }
    let brace = src[after..]
        .char_indices()
        .find(|(_, c)| !space(*c))
        .map_or(src.len(), |(i, _)| after + i);
    if src[brace..].starts_with('{') && last.is_some_and(|last| last > brace) {
        return src[brace + 1..].find('}').map(|i| brace + i + 2);
    }
    None
}
fn reader(src: &str) -> Result<Reader<'_>, PeleError> {
    let mut p = 0;
    let mut tokens = vec![];
    let last = src.rfind('}');
    while p < src.len() {
        let mut found = None;
        for (k, spec) in TOKENS.iter().enumerate() {
            let end = if k == 1 {
                acc(src, p, last)
            } else {
                match spec.pattern {
                    Pattern::Literal(v) => src[p..].starts_with(v).then_some(p + v.len()),
                    Pattern::Match(m) => m(src, p),
                }
            };
            if let Some(mut end) = end.filter(|e| *e > p) {
                let mut kind = k;
                for &alt in spec.longer {
                    if let Pattern::Match(m) = TOKENS[alt].pattern {
                        if let Some(e) = m(src, p).filter(|e| *e > end) {
                            end = e;
                            kind = alt;
                            break;
                        }
                    }
                }
                found = Some((kind, end));
                break;
            }
        }
        let Some((kind, end)) = found else {
            return Err(PeleError::syntax("treeView", src, p, "", true));
        };
        if !TOKENS[kind].hidden {
            tokens.push(Token {
                kind,
                start: p,
                end,
            });
        }
        p = end;
    }
    tokens.push(Token {
        kind: END,
        start: src.len(),
        end: src.len(),
    });
    Ok(Reader {
        src,
        tokens,
        i: 0,
        types: TOKENS,
        diagram: "treeView",
    })
}
fn read(src: &str, title: Option<&str>) -> Result<TreeViewModel, PeleError> {
    let mut r = reader(src)?;
    r.expect(0)?;
    let mut model = TreeViewModel {
        diagram_type: DiagramType::TreeView,
        title: title.map(str::to_owned),
        acc_title: None,
        acc_descr: None,
        root: 0,
        count: 1,
        nodes: vec![TreeNode {
            id: 0,
            level: -1,
            name: "/".into(),
            node_type: NodeType::Directory,
            icon: None,
            css_class: None,
            description: None,
            children: vec![],
        }],
    };
    loop {
        match r.kind() {
            1 => {
                let s = l::acc_descr_value(r.take());
                model.acc_descr = (!s.is_empty()).then_some(s);
            }
            2 => {
                let s = l::acc_title_value(r.take());
                model.acc_title = (!s.is_empty()).then_some(s);
            }
            3 => {
                let s = l::title_value(r.take());
                model.title = if s.is_empty() {
                    title.map(str::to_owned)
                } else {
                    Some(s)
                };
            }
            _ => break,
        }
    }
    let mut stack = vec![0];
    while r.kind() != END {
        let level = if r.kind() == 7 {
            r.take().len() as i64
        } else {
            0
        };
        let mut name = if r.kind() == 8 {
            let s = r.take();
            s[1..s.len() - 1].to_string()
        } else if r.kind() == 12 {
            r.take().trim_end_matches(blank).to_string()
        } else {
            return Err(r.fail("a name"));
        };
        let mut icon = None;
        let mut css_class = None;
        let mut description = None;
        loop {
            match r.kind() {
                4 => {
                    let s = r.take().trim_matches(space)[3..]
                        .trim_matches(space)
                        .to_string();
                    css_class = (!s.is_empty()).then_some(s);
                }
                5 => {
                    let s = r.take().trim_matches(space);
                    let value = &s[5..s.len() - 1];
                    icon = Some(if value.is_empty() {
                        "none".into()
                    } else {
                        value.into()
                    });
                }
                6 => {
                    let s = r.take().trim_matches(space)[2..]
                        .trim_matches(space)
                        .to_string();
                    description = (!s.is_empty()).then_some(s);
                }
                _ => break,
            }
        }
        let directory = name.ends_with('/');
        if directory {
            name.pop();
        }
        while level <= model.nodes[*stack.last().unwrap()].level {
            stack.pop();
        }
        let id = model.count;
        model.count += 1;
        model.nodes.push(TreeNode {
            id,
            level,
            name,
            node_type: if directory {
                NodeType::Directory
            } else {
                NodeType::File
            },
            icon,
            css_class,
            description,
            children: vec![],
        });
        model.nodes[*stack.last().unwrap()].children.push(id);
        stack.push(id);
    }
    Ok(model)
}
pub(super) fn parse(source: &str, title: Option<&str>) -> Result<TreeViewModel, PeleError> {
    let (text, map) = box_drawing::preprocess(source)?;
    read(&text, title).map_err(|mut e| {
        if !map.is_empty() {
            let old = e.line;
            if let Some(&line) = map.get(&old) {
                e.line = line;
                let head = e.message.find('\n').unwrap_or(e.message.len());
                let old_head = &e.message[..head];
                let mapped = old_head.replace(&format!("line {old}"), &format!("line {line}"));
                e.message = mapped + &e.message[head..];
            }
        }
        e
    })
}
