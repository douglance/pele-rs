//! Pele treemap/parser.ts + model.ts, using the actual common ordered token reader.
use super::model::*;
use crate::compat::{
    diagrams::common::langium::{self as l, Pattern, Reader, TokenSpec},
    source::space,
    DiagramType, ParseContext, PeleError,
};
use std::collections::HashMap;
const DESCR: usize = 0;
const TITLE_ACC: usize = 1;
const TITLE: usize = 2;
const KEYWORD: usize = 3;
const CLASS: usize = 4;
const STYLE: usize = 5;
const COLON: usize = 6;
const COMMA: usize = 7;
const INDENT: usize = 8;
const ID: usize = 12;
const NUMBER: usize = 13;
const STRING: usize = 14;
fn keyword(s: &str, p: usize) -> Option<usize> {
    if s[p..].starts_with("treemap-beta") {
        Some(p + 12)
    } else if s[p..].starts_with("treemap") {
        Some(p + 7)
    } else {
        None
    }
}
fn id(s: &str, p: usize) -> Option<usize> {
    let b = s.as_bytes();
    if !b
        .get(p)
        .is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_')
    {
        return None;
    }
    let mut q = p + 1;
    while b
        .get(q)
        .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_')
    {
        q += 1
    }
    Some(q)
}
fn class(s: &str, p: usize) -> Option<usize> {
    if !s[p..].starts_with("classDef") {
        return None;
    }
    let p = p + 8;
    let mut q = p;
    for c in s[q..].chars() {
        if !space(c) {
            break;
        }
        q += c.len_utf8()
    }
    if q == p {
        return None;
    }
    let start = q;
    q = id(s, q)?;
    if q - start < 2 {
        return None;
    }
    let end = q;
    for c in s[q..].chars() {
        if !space(c) {
            break;
        }
        q += c.len_utf8()
    }
    if q > end {
        while q < s.len() && !matches!(s.as_bytes()[q], b';' | b'\r' | b'\n') {
            q += s[q..].chars().next()?.len_utf8()
        }
    } else {
        q = end
    }
    if s.as_bytes().get(q) == Some(&b';') {
        q += 1
    }
    Some(q)
}
fn indent(s: &str, p: usize) -> Option<usize> {
    let q = l::blank(s, p);
    (q > p).then_some(q)
}
fn comment(s: &str, p: usize) -> Option<usize> {
    s[p..]
        .starts_with("%%")
        .then(|| s[p..].find('\n').map_or(s.len(), |q| p + q))
}
fn newline(s: &str, p: usize) -> Option<usize> {
    if s[p..].starts_with("\r\n") {
        Some(p + 2)
    } else if s[p..].starts_with('\n') {
        Some(p + 1)
    } else {
        None
    }
}
fn number(s: &str, p: usize) -> Option<usize> {
    let mut q = p;
    while s
        .as_bytes()
        .get(q)
        .is_some_and(|c| c.is_ascii_digit() || matches!(c, b'_' | b'.' | b','))
    {
        q += 1
    }
    (q > p).then_some(q)
}
fn string(s: &str, p: usize) -> Option<usize> {
    let quote = *s.as_bytes().get(p)?;
    if !matches!(quote, b'\'' | b'"') {
        return None;
    }
    s[p + 1..].find(quote as char).map(|q| p + q + 2)
}
const fn lit(name: &'static str, value: &'static str) -> TokenSpec {
    TokenSpec {
        name,
        pattern: Pattern::Literal(value),
        hidden: false,
        opener: None,
        first: None,
        longer: &[],
    }
}
const TYPES: [TokenSpec; 15] = [
    l::ACC_DESCR,
    l::ACC_TITLE,
    l::TITLE,
    TokenSpec::matcher("TREEMAP_KEYWORD", keyword, false, None, None),
    TokenSpec::matcher("CLASS_DEF", class, false, None, None),
    lit("STYLE_SEPARATOR", ":::"),
    lit("SEPARATOR", ":"),
    lit("COMMA", ","),
    TokenSpec::matcher("INDENTATION", indent, false, None, None),
    TokenSpec::matcher("WS", indent, true, None, None),
    TokenSpec::matcher("ML_COMMENT", comment, true, None, None),
    TokenSpec::matcher("NL", newline, true, None, None),
    TokenSpec::matcher("ID2", id, false, None, None),
    TokenSpec::matcher("NUMBER2", number, false, None, None),
    TokenSpec::matcher("STRING2", string, false, None, None),
];
// NUMBER2 has no exponent/sign. JS parseFloat stops at underscore or second dot.
fn value(s: &str) -> f64 {
    let s = s.replace(',', "");
    let mut p = 0;
    let mut dot = false;
    let mut digits = false;
    for b in s.bytes() {
        if b.is_ascii_digit() {
            digits = true;
            p += 1
        } else if b == b'.' && !dot {
            dot = true;
            p += 1
        } else {
            break;
        }
    }
    if digits {
        s[..p].parse().unwrap_or(f64::NAN)
    } else {
        f64::NAN
    }
}
struct Row {
    level: usize,
    name: String,
    leaf: bool,
    value: Option<f64>,
    selector: Option<String>,
}
pub(super) fn parse(c: ParseContext<'_>) -> Result<TreemapModel, PeleError> {
    let mut r = Reader::new(c.source, &TYPES, "treemap")?;
    let (mut title, mut acc_title, mut acc_descr) = (None, None, None);
    let mut rows = vec![];
    let mut classes: Vec<(String, Vec<String>)> = vec![];
    let mut class_index: HashMap<String, usize> = HashMap::new();
    r.expect(KEYWORD)?;
    while r.kind() != l::END {
        let kind = r.kind();
        if kind == DESCR {
            acc_descr = Some(l::acc_descr_value(r.take()));
            continue;
        }
        if kind == TITLE_ACC {
            acc_title = Some(l::acc_title_value(r.take()));
            continue;
        }
        if kind == TITLE {
            title = Some(l::title_value(r.take()));
            continue;
        }
        if !matches!(kind, INDENT | STRING | CLASS) {
            return Err(r.fail("a node, a class definition, or a title"));
        }
        let level = if kind == INDENT { r.take().len() } else { 0 };
        if r.kind() == CLASS {
            let raw = r.take();
            let start = raw[8..].find(|c| !space(c)).unwrap() + 8;
            let end = id(raw, start).unwrap();
            let key = &raw[start..end];
            let index = if let Some(i) = class_index.get(key) {
                *i
            } else {
                let i = classes.len();
                class_index.insert(key.into(), i);
                classes.push((key.into(), vec![]));
                i
            };
            let tail = raw[end..].trim_start_matches(space);
            let text = tail
                .strip_suffix(';')
                .unwrap_or(tail)
                .replace("\\,", "§§§")
                .replace(',', ";")
                .replace("§§§", ",");
            classes[index].1.extend(text.split(';').map(str::to_owned));
            continue;
        }
        let raw = r.expect(STRING)?;
        let name = raw[1..raw.len() - 1].to_owned();
        let after = if r.kind_at(r.i) == INDENT {
            r.i + 1
        } else {
            r.i
        };
        let leaf = matches!(r.kind_at(after), COLON | COMMA);
        let v = if leaf {
            r.i = after + 1;
            r.accept(INDENT);
            Some(value(r.expect(NUMBER)?))
        } else {
            None
        };
        let selector = if r.accept(STYLE) {
            Some(r.expect(ID)?.to_owned())
        } else {
            None
        };
        rows.push(Row {
            level,
            name,
            leaf,
            value: v,
            selector,
        });
    }
    let compiled: Vec<_> = classes
        .iter()
        .map(|(_, v)| std::sync::Arc::new(v.clone()))
        .collect();
    let mut nodes: Vec<Option<TreemapNode>> = vec![];
    let mut children: Vec<Vec<usize>> = vec![];
    let mut roots = vec![];
    let mut stack: Vec<(usize, usize)> = vec![];
    for row in rows {
        let styles = row
            .selector
            .as_ref()
            .and_then(|s| class_index.get(s))
            .map(|i| compiled[*i].clone())
            .filter(|s| !s.is_empty());
        let i = nodes.len();
        nodes.push(Some(TreemapNode {
            name: row.name,
            children: if row.leaf { None } else { Some(vec![]) },
            value: row.value,
            class_selector: row.selector,
            css_compiled_styles: styles,
        }));
        children.push(vec![]);
        while stack.last().is_some_and(|(_, level)| *level >= row.level) {
            stack.pop();
        }
        if let Some((parent, _)) = stack.last() {
            children[*parent].push(i)
        } else {
            roots.push(i)
        }
        if !row.leaf {
            stack.push((i, row.level));
        }
    }
    for i in (0..nodes.len()).rev() {
        if !children[i].is_empty() {
            let child_nodes = children[i]
                .iter()
                .map(|k| nodes[*k].take().unwrap())
                .collect();
            nodes[i].as_mut().unwrap().children = Some(child_nodes);
        }
    }
    let roots = roots
        .into_iter()
        .map(|i| nodes[i].take().unwrap())
        .collect();
    let title = title
        .filter(|s| !s.is_empty())
        .or_else(|| c.title.map(str::to_owned));
    let acc_title = acc_title
        .filter(|s| !s.is_empty())
        .map(|s| s.trim_start_matches(space).into());
    let acc_descr = acc_descr.filter(|s| !s.is_empty()).map(|s| {
        let mut out = String::new();
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            out.push(c);
            if c == '\n' {
                while chars.peek().is_some_and(|c| space(*c)) {
                    chars.next();
                }
            }
        }
        out
    });
    Ok(TreemapModel {
        diagram_type: DiagramType::Treemap,
        title,
        acc_title,
        acc_descr,
        roots,
        classes,
    })
}
