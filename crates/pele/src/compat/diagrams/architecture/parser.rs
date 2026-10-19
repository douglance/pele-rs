//! Pele architecture/parser.ts, 458322a5 (MIT, kepano); shared Langium reader.
use super::super::common::langium::*;
use super::model::*;
use crate::compat::{
    source::{space, word},
    PeleError,
};
const HEADER: usize = 0;
const JUNCTION: usize = 1;
const SERVICE: usize = 2;
const COLUMN: usize = 3;
const GROUP: usize = 4;
const ALIGN: usize = 5;
const ROW: usize = 6;
const DASHES: usize = 7;
const IN: usize = 8;
const COLON: usize = 9;
const DASH: usize = 10;
const DIRECTION: usize = 11;
const ARROW_GROUP: usize = 12;
const INTO: usize = 13;
const DESCR: usize = 14;
const ACC: usize = 15;
const TITLE_T: usize = 16;
const STRING_T: usize = 17;
const ID: usize = 18;
const NL: usize = 19;
const YAML_T: usize = 21;
const ICON: usize = 24;
const LABEL: usize = 25;
fn id(src: &str, p: usize) -> Option<usize> {
    let mut chars = src[p..].char_indices();
    let (_, first) = chars.next()?;
    if !word(first) {
        return None;
    }
    let mut end = p + first.len_utf8();
    for (i, c) in chars {
        if !word(c) && c != '-' {
            break;
        }
        if word(c) {
            end = p + i + c.len_utf8()
        }
    }
    Some(end)
}
fn direction(src: &str, p: usize) -> Option<usize> {
    src.as_bytes()
        .get(p)
        .filter(|c| matches!(c, b'L' | b'R' | b'T' | b'B'))
        .map(|_| p + 1)
}
fn into(src: &str, p: usize) -> Option<usize> {
    src.as_bytes()
        .get(p)
        .filter(|c| matches!(c, b'<' | b'>'))
        .map(|_| p + 1)
}
fn icon(src: &str, p: usize) -> Option<usize> {
    if !src[p..].starts_with('(') {
        return None;
    }
    let mut at = p + 1;
    while at < src.len() {
        let c = src[at..].chars().next()?;
        if !(word(c) || matches!(c, '-' | ':')) {
            break;
        }
        at += c.len_utf8()
    }
    (at > p + 1 && src[at..].starts_with(')')).then_some(at + 1)
}
fn label(src: &str, p: usize) -> Option<usize> {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(||regex::Regex::new(r#"^\[(?:"([^"\\]|\\[^\n\r\u{2028}\u{2029}])*"|'([^'\\]|\\[^\n\r\u{2028}\u{2029}])*'|[^\[\]\r\n]+)\]"#).unwrap()).find(&src[p..]).map(|m|p+m.end())
}

const TYPES: &[TokenSpec] = &[
    TokenSpec::literal("architecture-beta", &[ID]),
    TokenSpec::literal("junction", &[ID]),
    TokenSpec::literal("service", &[ID]),
    TokenSpec::literal("column", &[ID]),
    TokenSpec::literal("group", &[ID]),
    TokenSpec::literal("align", &[ID]),
    TokenSpec::literal("row", &[ID]),
    TokenSpec::literal("--", &[YAML_T]),
    TokenSpec::literal("in", &[ID]),
    TokenSpec::literal(":", &[]),
    TokenSpec::literal("-", &[YAML_T]),
    TokenSpec::matcher("ARROW_DIRECTION", direction, false, Some("LRTB"), None),
    TokenSpec::literal("{group}", &[]),
    TokenSpec::matcher("ARROW_INTO", into, false, Some("<>"), None),
    ACC_DESCR,
    ACC_TITLE,
    TITLE,
    STRING,
    TokenSpec::matcher("ID", id, false, None, None),
    NEWLINE,
    WHITESPACE,
    YAML,
    DIRECTIVE,
    SINGLE_LINE_COMMENT,
    TokenSpec::matcher("ARCH_ICON", icon, false, Some("("), None),
    TokenSpec::matcher("ARCH_TITLE", label, false, Some("["), None),
];
#[derive(Default)]
pub struct Ast {
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
    pub groups: Vec<ArchGroup>,
    pub services: Vec<ArchNode>,
    pub junctions: Vec<ArchNode>,
    pub edges: Vec<ArchEdge>,
    pub hints: Vec<ArchLayoutHint>,
}
fn label_value(s: &str) -> String {
    let mut s = s[1..s.len() - 1].trim_matches(space).to_string();
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        s = s
            .get(1..s.len().saturating_sub(1))
            .unwrap_or("")
            .replace("\\\"", "\"")
            .replace("\\'", "'")
    }
    s.trim_matches(space).into()
}
fn end(r: &mut Reader<'_>) -> Result<(), PeleError> {
    if r.kind() == END {
        return Ok(());
    }
    if r.kind() != NL {
        return Err(r.fail("a line break or the end of input"));
    }
    while r.kind() == NL {
        r.i += 1
    }
    Ok(())
}
pub fn parse(src: &str) -> Result<Ast, PeleError> {
    let mut r = Reader::new(src, TYPES, "architecture")?;
    let mut a = Ast::default();
    while r.kind() == NL {
        r.i += 1
    }
    r.expect(HEADER)?;
    while r.kind() != END {
        match r.kind() {
            NL => {
                r.i += 1;
                continue;
            }
            DESCR => a.acc_descr = Some(acc_descr_value(r.take())),
            ACC => a.acc_title = Some(acc_title_value(r.take())),
            TITLE_T => a.title = Some(title_value(r.take())),
            GROUP => {
                r.i += 1;
                let id = r.expect(ID)?.into();
                let icon = if r.kind() == ICON {
                    let s = r.take();
                    Some(s[1..s.len() - 1].into())
                } else {
                    None
                };
                let title = if r.kind() == LABEL {
                    Some(label_value(r.take()))
                } else {
                    None
                };
                let parent = if r.accept(IN) {
                    Some(r.expect(ID)?.into())
                } else {
                    None
                };
                a.groups.push(ArchGroup {
                    id,
                    icon,
                    title,
                    parent,
                })
            }
            SERVICE | JUNCTION => {
                let service = r.kind() == SERVICE;
                r.i += 1;
                let id = r.expect(ID)?.into();
                let mut icon = None;
                let mut icon_text = None;
                let mut title = None;
                if service {
                    if r.kind() == STRING_T {
                        icon_text = Some(string_value(r.take()))
                    } else if r.kind() == ICON {
                        let s = r.take();
                        icon = Some(s[1..s.len() - 1].into())
                    }
                    if r.kind() == LABEL {
                        title = Some(label_value(r.take()))
                    }
                }
                let parent = if r.accept(IN) {
                    Some(r.expect(ID)?.into())
                } else {
                    None
                };
                let node = ArchNode {
                    id,
                    node_type: if service { "service" } else { "junction" }.into(),
                    icon,
                    icon_text,
                    title,
                    parent,
                };
                if service {
                    a.services.push(node)
                } else {
                    a.junctions.push(node)
                }
            }
            ID => {
                let lhs_id = r.take().into();
                let lhs_group = r.accept(ARROW_GROUP);
                r.expect(COLON)?;
                let lhs_dir = r.expect(DIRECTION)?.chars().next().unwrap();
                let lhs_into = r.accept(INTO);
                let title = if r.accept(DASHES) {
                    None
                } else {
                    if r.kind() != DASH {
                        return Err(r.fail("'--' or '-'"));
                    }
                    r.i += 1;
                    let title = label_value(r.expect(LABEL)?);
                    r.expect(DASH)?;
                    Some(title)
                };
                let rhs_into = r.accept(INTO);
                let rhs_dir = r.expect(DIRECTION)?.chars().next().unwrap();
                r.expect(COLON)?;
                let rhs_id = r.expect(ID)?.into();
                let rhs_group = r.accept(ARROW_GROUP);
                a.edges.push(ArchEdge {
                    lhs_id,
                    lhs_dir,
                    lhs_into,
                    lhs_group,
                    rhs_id,
                    rhs_dir,
                    rhs_into,
                    rhs_group,
                    title,
                })
            }
            ALIGN => {
                r.i += 1;
                if !matches!(r.kind(), ROW | COLUMN) {
                    return Err(r.fail("'row' or 'column'"));
                }
                let direction = r.take().into();
                let mut members = vec![r.expect(ID)?.into(), r.expect(ID)?.into()];
                while r.kind() == ID {
                    members.push(r.take().into())
                }
                a.hints.push(ArchLayoutHint { direction, members })
            }
            _ => return Err(r.fail(
                "a group, a service, a junction, an edge, an alignment, a title, or a line break",
            )),
        }
        end(&mut r)?
    }
    Ok(a)
}
