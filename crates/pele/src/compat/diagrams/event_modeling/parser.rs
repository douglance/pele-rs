//! Pele eventmodeling/parser.ts and database population, 458322a5 (MIT, kepano).
use super::model::*;
use crate::compat::{
    diagrams::common::langium::{self as l, Reader, TokenSpec, END},
    source::{space, word},
    DiagramType, ErrorCode, PeleError,
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
fn id(src: &str, at: usize) -> Option<usize> {
    let first = src[at..].chars().next()?;
    if !first.is_ascii_alphabetic() && first != '_' {
        return None;
    }
    let mut q = at + 1;
    while src[q..].chars().next().is_some_and(word) {
        q += 1;
    }
    Some(q)
}
fn frame_id(src: &str, at: usize) -> Option<usize> {
    let mut q = at;
    while q < at + 3 && src.as_bytes().get(q).is_some_and(u8::is_ascii_digit) {
        q += 1;
    }
    (q > at).then_some(q)
}
fn keyword(src: &str, at: usize) -> Option<usize> {
    l::keyword(src, at, "eventmodeling")
}
fn inline(src: &str, at: usize) -> Option<usize> {
    let open = src[at..].chars().next()?;
    let close = match open {
        '{' => '}',
        '"' => '"',
        '\'' => '\'',
        _ => return None,
    };
    let end = src[at + 1..]
        .find(['\n', '\r', '\u{2028}', '\u{2029}'])
        .map_or(src.len(), |i| at + 1 + i);
    src[at + 1..end].rfind(close).map(|i| at + i + 2)
}
fn newline(src: &str, at: usize) -> Option<usize> {
    if src[at..].starts_with("\r\n") {
        Some(at + 2)
    } else if src[at..].starts_with('\n') {
        Some(at + 1)
    } else {
        None
    }
}
fn block(src: &str, at: usize) -> Option<usize> {
    if !src[at..].starts_with('{') {
        return None;
    }
    let body = newline(src, l::blank(src, at + 1))?;
    let closer = |p| {
        if !src[p..].starts_with('}') {
            return None;
        }
        let after = p + 1;
        newline(src, after).or_else(|| {
            src[after..]
                .chars()
                .next()
                .is_none_or(space)
                .then_some(after)
        })
    };
    let mut p = body;
    while let Some(next) = src[p..].find('\n').map(|i| p + i + 1) {
        if let Some(end) = closer(next) {
            return Some(end);
        }
        p = next;
    }
    closer(body)
}
fn ws(src: &str, at: usize) -> Option<usize> {
    let mut p = at;
    while let Some(c) = src[p..].chars().next().filter(|c| space(*c)) {
        p += c.len_utf8();
    }
    (p > at).then_some(p)
}
fn ml_comment(src: &str, at: usize) -> Option<usize> {
    src[at..].strip_prefix("/*")?.find("*/").map(|i| at + i + 4)
}
fn sl_comment(src: &str, at: usize) -> Option<usize> {
    src[at..]
        .starts_with("//")
        .then(|| l::line_end(src, at + 2))
}
static TOKENS: &[TokenSpec] = &[
    TokenSpec {
        longer: &[34],
        ..TokenSpec::matcher("eventmodeling", keyword, false, Some("e"), None)
    },
    TokenSpec::literal("resetframe", &[34]),
    TokenSpec::literal("readmodel", &[34]),
    TokenSpec::literal("processor", &[34]),
    TokenSpec::literal("timeframe", &[34]),
    TokenSpec::literal("command", &[34]),
    TokenSpec::literal("entity", &[34]),
    TokenSpec::literal("event", &[34]),
    TokenSpec::literal("jsobj", &[34]),
    TokenSpec::literal("figma", &[34]),
    TokenSpec::literal("given", &[34]),
    TokenSpec::literal("json", &[34]),
    TokenSpec::literal("salt", &[34]),
    TokenSpec::literal("html", &[34]),
    TokenSpec::literal("text", &[34]),
    TokenSpec::literal("data", &[34]),
    TokenSpec::literal("note", &[34]),
    TokenSpec::literal("when", &[34]),
    TokenSpec::literal("then", &[34]),
    TokenSpec::literal("rmo", &[34]),
    TokenSpec::literal("cmd", &[34]),
    TokenSpec::literal("evt", &[34]),
    TokenSpec::literal("pcr", &[34]),
    TokenSpec::literal("uri", &[34]),
    TokenSpec::literal("->>", &[]),
    TokenSpec::literal("gwt", &[34]),
    TokenSpec::literal("ui", &[34]),
    TokenSpec::literal("md", &[34]),
    TokenSpec::literal("tf", &[34]),
    TokenSpec::literal("[[", &[]),
    TokenSpec::literal("]]", &[]),
    TokenSpec::literal("rf", &[34]),
    TokenSpec::literal("`", &[]),
    TokenSpec::literal(".", &[]),
    TokenSpec::matcher("EM_ID", id, false, None, None),
    TokenSpec::matcher("EM_FID", frame_id, false, None, None),
    TokenSpec::matcher("EM_DATA_INLINE", inline, false, Some("{\"'"), None),
    TokenSpec::matcher("EM_DATA_BLOCK", block, false, Some("{"), None),
    TokenSpec {
        name: "EM_ACC_DESCR",
        ..l::ACC_DESCR
    },
    TokenSpec {
        name: "EM_ACC_TITLE",
        ..l::ACC_TITLE
    },
    TokenSpec {
        name: "EM_TITLE",
        ..l::TITLE
    },
    TokenSpec::matcher("EM_WS", ws, true, None, None),
    TokenSpec {
        name: "EM_YAML",
        ..l::YAML
    },
    TokenSpec {
        name: "EM_DIRECTIVE",
        ..l::DIRECTIVE
    },
    TokenSpec {
        name: "EM_SINGLE_LINE_COMMENT",
        ..l::SINGLE_LINE_COMMENT
    },
    TokenSpec::matcher("EM_ML_COMMENT", ml_comment, true, Some("/"), None),
    TokenSpec::matcher("EM_SL_COMMENT", sl_comment, true, Some("/"), None),
];
fn entity(kind: usize) -> bool {
    matches!(kind, 2 | 3 | 5 | 7 | 19 | 20 | 21 | 22 | 26)
}
fn data_type(kind: usize) -> bool {
    matches!(kind, 8 | 9 | 11 | 12 | 13 | 14 | 23 | 27)
}
struct Frame {
    id: String,
    kind: String,
    entity: String,
    sources: Vec<String>,
    reference: Option<String>,
    data_type: Option<String>,
    inline: Option<String>,
    offset: usize,
    reset: bool,
}
fn qualified(r: &mut Reader<'_>) -> Result<String, PeleError> {
    let mut s = r.expect(34)?.to_string();
    while r.accept(33) {
        s.push('.');
        s += r.expect(34)?;
    }
    Ok(s)
}
fn datatype(r: &mut Reader<'_>) -> Result<Option<String>, PeleError> {
    if !r.accept(32) {
        return Ok(None);
    }
    if !data_type(r.kind()) {
        return Err(r.fail("a data type"));
    }
    let value = r.take().to_string();
    r.expect(32)?;
    Ok(Some(value))
}
fn statements(r: &mut Reader<'_>) -> Result<Vec<String>, PeleError> {
    let mut names = vec![];
    loop {
        if !entity(r.kind()) {
            return Err(r.fail("an entity type"));
        }
        r.i += 1;
        names.push(r.expect(34)?.to_string());
        if !entity(r.kind()) {
            break;
        }
    }
    Ok(names)
}
fn block_text(raw: &str) -> String {
    let open = raw.find('{');
    let close = raw.rfind('}');
    let inner = match (open, close) {
        (Some(open), Some(close)) if close > open => &raw[open + 1..close],
        (None, _) if raw.len() >= 2 => &raw[1..raw.len() - 1],
        _ => "",
    };
    let text = inner.replace('\r', "");
    let mut lines = text.split('\n').collect::<Vec<_>>();
    let first = lines
        .iter()
        .position(|s| !s.trim_matches(space).is_empty())
        .unwrap_or(lines.len());
    let last = lines
        .iter()
        .rposition(|s| !s.trim_matches(space).is_empty())
        .map_or(first, |i| i + 1);
    lines = lines[first..last].to_vec();
    let indent = lines
        .iter()
        .filter(|s| !s.trim_matches(space).is_empty())
        .map(|s| {
            s.chars()
                .take_while(|c| space(*c))
                .map(char::len_utf16)
                .sum::<usize>()
        })
        .min()
        .unwrap_or(usize::MAX);
    lines
        .into_iter()
        .map(|s| {
            let mut used = 0;
            let mut at = 0;
            for (i, c) in s.char_indices() {
                if used + c.len_utf16() > indent {
                    break;
                }
                used += c.len_utf16();
                at = i + c.len_utf8();
            }
            s[at..].trim_end_matches(space).to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}
pub(super) fn parse(src: &str, title: Option<&str>) -> Result<EventModelingModel, PeleError> {
    let mut r = Reader::new(src, TOKENS, "eventmodeling")?;
    r.expect(0)?;
    let mut frames = vec![];
    let mut data = HashMap::<String, Arc<str>>::new();
    let mut notes = vec![];
    let mut specifications = vec![];
    let mut entities = vec![];
    let mut own_title = None;
    let mut acc_title = None;
    let mut acc_descr = None;
    while r.kind() != END {
        match r.kind() {
            38 => {
                let text = l::acc_descr_value(r.take());
                acc_descr = Some(text);
            }
            39 => {
                acc_title = Some(l::acc_title_value(r.take()));
            }
            40 => {
                own_title = Some(l::title_value(r.take()));
            }
            6 => {
                r.i += 1;
                entities.push(qualified(&mut r)?);
            }
            1 | 4 | 28 | 31 => {
                let reset = matches!(r.kind(), 1 | 31);
                let offset = r.tokens[r.i].start;
                r.i += 1;
                let id = r.expect(35)?.into();
                if !entity(r.kind()) {
                    return Err(r.fail("an entity type"));
                }
                let kind = r.take().into();
                let entity = qualified(&mut r)?;
                let mut sources = vec![];
                while r.accept(24) {
                    sources.push(r.expect(35)?.into());
                }
                let reference = if r.accept(29) {
                    let id = r.expect(34)?.to_string();
                    r.expect(30)?;
                    Some(id)
                } else {
                    None
                };
                let (data_type, inline) = if matches!(r.kind(), 32 | 36) {
                    (datatype(&mut r)?, Some(r.expect(36)?.to_string()))
                } else {
                    (None, None)
                };
                frames.push(Frame {
                    id,
                    kind,
                    entity,
                    sources,
                    reference,
                    data_type,
                    inline,
                    offset,
                    reset,
                });
            }
            15 => {
                r.i += 1;
                let name = r.expect(34)?.to_string();
                datatype(&mut r)?;
                let raw = r.expect(37)?;
                data.entry(name)
                    .or_insert_with(|| Arc::from(block_text(raw)));
            }
            16 => {
                r.i += 1;
                let frame = r.expect(35)?.into();
                let data_type = datatype(&mut r)?;
                let text = block_text(r.expect(37)?);
                notes.push(EmNote {
                    frame,
                    data_type,
                    text,
                });
            }
            25 => {
                r.i += 1;
                let frame = r.expect(35)?.into();
                r.expect(10)?;
                let given = statements(&mut r)?;
                let when = if r.accept(17) {
                    statements(&mut r)?
                } else {
                    vec![]
                };
                r.expect(18)?;
                let then = statements(&mut r)?;
                specifications.push(EmSpecification {
                    frame,
                    given,
                    when,
                    then,
                });
            }
            _ => return Err(r.fail("a frame, an entity, a data block, a note, or a specification")),
        }
    }
    let mut ids = HashSet::new();
    for frame in &frames {
        if !ids.insert(frame.id.clone()) {
            let line = src[..frame.offset].bytes().filter(|c| *c == b'\n').count() + 1;
            let mut error = PeleError::new(
                format!(
                    "Duplicate event modeling frame ID \"{}\" on line {line}",
                    frame.id
                ),
                ErrorCode::Semantic,
            );
            error.diagram_type = Some("eventmodeling".into());
            error.line = line;
            return Err(error);
        }
    }
    let mut lanes = vec![];
    let mut lane_of = HashMap::new();
    let mut next = HashMap::<LaneGroup, usize>::new();
    let mut boxes = Vec::<Arc<EmBox>>::new();
    let mut relations = vec![];
    for frame in frames {
        let parts = frame.entity.split('.').collect::<Vec<_>>();
        let namespace = (parts.len() == 2).then(|| parts[0].to_string());
        let kind = match frame.kind.as_str() {
            "ui" => EntityKind::Ui,
            "pcr" | "processor" => EntityKind::Processor,
            "cmd" | "command" => EntityKind::Command,
            "rmo" | "readmodel" => EntityKind::Readmodel,
            _ => EntityKind::Event,
        };
        let group = match kind {
            EntityKind::Ui | EntityKind::Processor => LaneGroup::Trigger,
            EntityKind::Event => LaneGroup::Stream,
            _ => LaneGroup::State,
        };
        let key = (group, namespace.clone());
        let lane = lane_of
            .entry(key)
            .or_insert_with(|| {
                let (first, label, prefix) = match group {
                    LaneGroup::Trigger => (0, "UI/Automation", "UI/A: "),
                    LaneGroup::State => (100, "Command/Read Model", "C/RM: "),
                    LaneGroup::Stream => (200, "Events", "Stream: "),
                };
                let index = if namespace.is_some() {
                    let n = next.entry(group).or_insert(first + 1);
                    let index = *n;
                    *n += 1;
                    index
                } else {
                    first
                };
                let lane = Arc::new(EmLane {
                    index,
                    label: namespace
                        .as_ref()
                        .map_or(label.into(), |n| format!("{prefix}{n}")),
                    group,
                    namespace: namespace.clone(),
                });
                lanes.push(lane.clone());
                lane
            })
            .clone();
        let text = frame
            .reference
            .as_ref()
            .filter(|s| !s.is_empty())
            .and_then(|reference| data.get(reference).cloned())
            .or_else(|| frame.inline.as_ref().map(|s| Arc::from(block_text(s))));
        let b = Arc::new(EmBox {
            id: frame.id,
            kind,
            entity: frame.entity.clone(),
            name: if parts.len() == 2 {
                parts[1].into()
            } else {
                frame.entity
            },
            namespace,
            reset: frame.reset,
            lane,
            data: text,
            data_type: frame.data_type,
        });
        boxes.push(b.clone());
        if !frame.sources.is_empty() {
            let wanted = frame.sources.iter().collect::<HashSet<_>>();
            for source in &boxes {
                if wanted.contains(&source.id) {
                    relations.push(EmRelation {
                        source: source.clone(),
                        target: b.clone(),
                    });
                }
            }
        } else if boxes.len() > 1 && !b.reset {
            if let Some(source) = boxes[..boxes.len() - 1]
                .iter()
                .rev()
                .find(|source| !Arc::ptr_eq(&source.lane, &b.lane))
            {
                relations.push(EmRelation {
                    source: source.clone(),
                    target: b.clone(),
                });
            }
        }
    }
    lanes.sort_by_key(|lane| lane.index);
    Ok(EventModelingModel {
        diagram_type: DiagramType::Eventmodeling,
        title: own_title
            .filter(|s| !s.is_empty())
            .or_else(|| title.map(str::to_owned)),
        acc_title,
        acc_descr,
        boxes,
        lanes,
        relations,
        notes,
        specifications,
        entities,
    })
}
