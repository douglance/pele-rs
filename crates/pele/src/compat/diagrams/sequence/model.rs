//! Pele sequence/{types,db}.ts complete ordered model and deferred statement application,
//! 458322a5 (MIT, kepano).
use super::{colors::is_css_color, kinds::*};
use crate::compat::{
    source::{space, yaml},
    DiagramType, ErrorCode, Object, PeleError, Value,
};
use serde::Serialize;
use std::collections::HashMap;
#[derive(Clone, Debug)]
pub(super) struct Text {
    pub text: String,
    pub wrap: Option<bool>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeqBox {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub wrap: bool,
    pub fill: String,
    pub actor_keys: Vec<String>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeqActor {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(serialize_with = "box_view")]
    pub r#box: Option<std::sync::Arc<SeqBox>>,
    pub name: String,
    pub description: String,
    pub wrap: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_actor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_actor: Option<String>,
    pub links: Vec<(String, super::json::SeqValue)>,
    pub properties: Vec<(String, super::json::SeqValue)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    pub r#type: String,
    #[serde(skip)]
    pub(super) box_index: Option<usize>,
}
#[derive(Clone, Debug, Serialize)]
pub struct SeqNumbering {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<f64>,
    pub visible: bool,
}
#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum MessageText {
    Text(String),
    Numbering(SeqNumbering),
}
impl MessageText {
    pub fn text(&self) -> Option<&str> {
        if let Self::Text(s) = self {
            Some(s)
        } else {
            None
        }
    }
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeqMessage {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
    pub message: MessageText,
    pub wrap: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activate: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placement: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub central_connection: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub answer: Option<super::json::SeqValue>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum NoteActors {
    Actor(String),
    Actors(Vec<String>),
}
#[derive(Clone, Debug, Serialize)]
pub struct SeqNote {
    pub actor: NoteActors,
    pub placement: u8,
    pub message: String,
    pub wrap: bool,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SequenceModel {
    #[serde(rename = "type")]
    pub diagram_type: DiagramType,
    pub actors: Vec<(String, SeqActor)>,
    pub created_actors: Vec<(String, usize)>,
    pub destroyed_actors: Vec<(String, usize)>,
    pub boxes: Vec<SeqBox>,
    pub messages: Vec<SeqMessage>,
    pub notes: Vec<SeqNote>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acc_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acc_descr: Option<String>,
}
#[derive(Clone, Debug)]
pub(super) enum Statement {
    Empty,
    Participant {
        actor: String,
        draw: Option<String>,
        description: Option<Text>,
        config: Option<String>,
        created: bool,
    },
    Destroy(String),
    Signal {
        from: Option<String>,
        to: Option<String>,
        text: Option<Text>,
        kind: u8,
        activate: bool,
        central: u8,
        validate_life: bool,
    },
    Note(NoteActors, u8, Text),
    Menu(String, String, Text),
    Number(SeqNumbering),
    BoxStart(String),
    BoxEnd,
}
pub(super) struct Db {
    pub model: SequenceModel,
    actor_index: HashMap<String, usize>,
    prev: Option<String>,
    current_box: Option<usize>,
    last_created: Option<String>,
    last_destroyed: Option<String>,
    wrap_enabled: Option<bool>,
    sequence_wrap: bool,
    active: HashMap<String, usize>,
}
pub(super) fn fail(message: impl Into<String>, code: ErrorCode) -> PeleError {
    let mut e = PeleError::new(message, code);
    e.diagram_type = Some("sequence".into());
    e
}
pub(super) fn parse_message(raw: &str) -> Text {
    let s = raw.trim_matches(space);
    let (text, wrap) = if let Some(s) = s.strip_prefix("wrap:").or_else(|| s.strip_prefix(":wrap:"))
    {
        (s, Some(true))
    } else if let Some(s) = s
        .strip_prefix("nowrap:")
        .or_else(|| s.strip_prefix(":nowrap:"))
    {
        (s, Some(false))
    } else {
        (s, None)
    };
    Text {
        text: text.trim_matches(space).into(),
        wrap,
    }
}
fn set<T>(map: &mut Vec<(String, T)>, key: String, value: T) {
    if let Some((_, old)) = map.iter_mut().find(|(k, _)| *k == key) {
        *old = value
    } else {
        map.push((key, value))
    }
}
impl Db {
    pub fn new(config: &Object, title: Option<&str>) -> Self {
        let wrap_enabled = match config.get("wrap") {
            Some(Value::Bool(b)) => Some(*b),
            _ => None,
        };
        let sequence_wrap = config
            .get("sequence")
            .and_then(Value::as_object)
            .and_then(|o| o.get("wrap"))
            == Some(&Value::Bool(true));
        Self {
            model: SequenceModel {
                diagram_type: DiagramType::Sequence,
                actors: vec![],
                created_actors: vec![],
                destroyed_actors: vec![],
                boxes: vec![],
                messages: vec![],
                notes: vec![],
                title: title.filter(|s| !s.is_empty()).map(str::to_owned),
                acc_title: None,
                acc_descr: None,
            },
            actor_index: HashMap::new(),
            prev: None,
            current_box: None,
            last_created: None,
            last_destroyed: None,
            wrap_enabled,
            sequence_wrap,
            active: HashMap::new(),
        }
    }
    fn auto_wrap(&self) -> bool {
        self.wrap_enabled.unwrap_or(self.sequence_wrap)
    }
    fn add_box(&mut self, raw: &str) {
        let mut split = None;
        for prefix in ["rgb", "rgba", "hsl", "hsla"] {
            if let Some(rest) = raw.strip_prefix(prefix) {
                let gap = rest.len() - rest.trim_start_matches(space).len();
                if rest[gap..].starts_with('(') {
                    let fn_len = prefix.len() + gap + 1;
                    if let Some(close) = raw.rfind(')') {
                        if close >= fn_len
                            && !raw[fn_len..].contains(['\r', '\u{2028}', '\u{2029}'])
                        {
                            split = Some((&raw[..close + 1], &raw[close + 1..]));
                            break;
                        }
                    }
                }
            }
        }
        if split.is_none() && !raw.contains(['\r', '\u{2028}', '\u{2029}']) {
            let end = raw
                .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                .unwrap_or(raw.len());
            split = Some((&raw[..end], &raw[end..]));
        }
        let mut color = split
            .map(|(a, _)| a)
            .filter(|s| !s.is_empty())
            .map(|s| s.trim_matches(space))
            .unwrap_or("transparent");
        let mut title = split
            .map(|(_, s)| s)
            .filter(|s| !s.is_empty())
            .map(|s| s.trim_matches(space));
        if !is_css_color(color) {
            color = "transparent";
            title = Some(raw.trim_matches(space));
        }
        let title = title.map(parse_message);
        let name = title.as_ref().and_then(|t| {
            if t.text.is_empty() {
                None
            } else {
                Some(t.text.clone())
            }
        });
        let wrap = title.and_then(|t| t.wrap).unwrap_or(self.auto_wrap());
        self.current_box = Some(self.model.boxes.len());
        self.model.boxes.push(SeqBox {
            name,
            wrap,
            fill: color.into(),
            actor_keys: vec![],
        });
    }
    fn add_actor(
        &mut self,
        id: &str,
        name: &str,
        mut description: Option<Text>,
        mut kind: Option<String>,
        metadata: Option<&str>,
    ) -> Result<(), PeleError> {
        let mut assigned = self.current_box;
        if let Some(metadata) = metadata {
            let raw = if metadata.contains('\n') {
                format!("{metadata}\n")
            } else {
                format!("{{\n{metadata}\n}}")
            };
            let doc = yaml::parse(&raw).map_err(|e| {
                fail(
                    format!("Config of participant \"{id}\" is not valid YAML. {e}"),
                    ErrorCode::Syntax,
                )
            })?;
            if let Some(doc) = doc.as_object() {
                if let Some(v) = doc.get("type").filter(|v| !matches!(v, Value::Null)) {
                    kind = Some(v.js_string())
                }
                if let Some(v) = doc.get("alias").filter(|v| v.truthy()) {
                    if description.as_ref().is_none_or(|d| d.text == name) {
                        description = Some(Text {
                            text: v.js_string(),
                            wrap: description.and_then(|d| d.wrap),
                        })
                    }
                }
            }
        }
        if let Some(&i) = self.actor_index.get(id) {
            let old = &self.model.actors[i].1;
            if let (Some(current), Some(previous)) = (self.current_box, old.box_index) {
                if current != previous {
                    let a = self.model.boxes[previous]
                        .name
                        .as_deref()
                        .unwrap_or("undefined");
                    let b = self.model.boxes[current]
                        .name
                        .as_deref()
                        .unwrap_or("undefined");
                    return Err(fail(format!("A same participant should only be defined in one Box: {} can't be in '{a}' and in '{b}' at the same time.",old.name),ErrorCode::Semantic));
                }
            }
            assigned = old.box_index.or(self.current_box);
            let early = name == old.name && description.is_none();
            self.model.actors[i].1.box_index = assigned;
            if early {
                return Ok(());
            }
        }
        if description.is_none() || kind.is_none() {
            description = Some(Text {
                text: name.into(),
                wrap: None,
            })
        }
        let d = description.unwrap();
        let actor = SeqActor {
            r#box: None,
            box_index: assigned,
            name: name.into(),
            description: d.text,
            wrap: d.wrap.unwrap_or(self.auto_wrap()),
            prev_actor: self.prev.clone(),
            next_actor: None,
            links: vec![],
            properties: vec![],
            details: None,
            r#type: kind.unwrap_or_else(|| "participant".into()),
        };
        let index = if let Some(i) = self.actor_index.get(id) {
            *i
        } else {
            let i = self.model.actors.len();
            self.actor_index.insert(id.into(), i);
            self.model.actors.push((id.into(), actor.clone()));
            i
        };
        self.model.actors[index].1 = actor;
        if let Some(prev) = self
            .prev
            .as_ref()
            .and_then(|p| self.actor_index.get(p))
            .copied()
        {
            self.model.actors[prev].1.next_actor = Some(id.into())
        }
        if let Some(b) = self.current_box {
            self.model.boxes[b].actor_keys.push(id.into())
        }
        self.prev = Some(id.into());
        Ok(())
    }
    fn signal(
        &mut self,
        from: Option<String>,
        to: Option<String>,
        text: Option<Text>,
        kind: u8,
        activate: bool,
        central: u8,
    ) -> Result<(), PeleError> {
        if kind == ACTIVE_END {
            let count = from
                .as_ref()
                .and_then(|s| self.active.get(s))
                .copied()
                .unwrap_or(0);
            if count < 1 {
                return Err(fail(
                    format!(
                        "Trying to inactivate an inactive participant ({})",
                        from.as_deref().unwrap_or("undefined")
                    ),
                    ErrorCode::Semantic,
                ));
            }
            self.active.insert(from.clone().unwrap(), count - 1);
        } else if kind == ACTIVE_START {
            if let Some(from) = &from {
                *self.active.entry(from.clone()).or_default() += 1;
            }
        }
        let wrap = text
            .as_ref()
            .and_then(|t| t.wrap)
            .unwrap_or(self.auto_wrap());
        let message = text.map(|t| t.text).unwrap_or_default();
        self.model.messages.push(SeqMessage {
            id: self.model.messages.len().to_string(),
            from,
            to,
            message: MessageText::Text(message),
            wrap,
            r#type: Some(kind),
            activate: Some(activate),
            placement: None,
            central_connection: Some(central),
            answer: None,
        });
        Ok(())
    }
    pub fn apply(mut self, statements: Vec<Statement>) -> Result<SequenceModel, PeleError> {
        for stmt in statements {
            match stmt {
                Statement::Empty => {}
                Statement::Participant {
                    actor,
                    draw,
                    description,
                    config,
                    created,
                } => {
                    if created {
                        if self.actor_index.contains_key(&actor) {
                            return Err(fail("It is not possible to have actors with the same id, even if one is destroyed before the next is created. Use 'AS' aliases to simulate the behavior",ErrorCode::Semantic));
                        }
                        self.last_created = Some(actor.clone());
                    }
                    self.add_actor(&actor, &actor, description, draw, config.as_deref())?;
                    if created {
                        set(
                            &mut self.model.created_actors,
                            actor,
                            self.model.messages.len(),
                        )
                    }
                }
                Statement::Destroy(a) => {
                    self.last_destroyed = Some(a.clone());
                    set(
                        &mut self.model.destroyed_actors,
                        a,
                        self.model.messages.len(),
                    )
                }
                Statement::Signal {
                    from,
                    to,
                    text,
                    kind,
                    activate,
                    central,
                    validate_life,
                } => {
                    if validate_life {
                        if let Some(a) = &self.last_created {
                            if to.as_ref() != Some(a) {
                                return Err(fail(format!("The created participant {a} does not have an associated creating message after its declaration. Please check the sequence diagram."),ErrorCode::Semantic));
                            }
                            self.last_created = None;
                        } else if let Some(a) = &self.last_destroyed {
                            if to.as_ref() != Some(a) && from.as_ref() != Some(a) {
                                return Err(fail(format!("The destroyed participant {a} does not have an associated destroying message after its declaration. Please check the sequence diagram."),ErrorCode::Semantic));
                            }
                            self.last_destroyed = None;
                        }
                    }
                    self.signal(from, to, text, kind, activate, central)?
                }
                Statement::Note(actor, placement, text) => {
                    let (from, to) = match &actor {
                        NoteActors::Actor(a) => (Some(a.clone()), Some(a.clone())),
                        NoteActors::Actors(a) => (a.first().cloned(), a.get(1).cloned()),
                    };
                    let wrap = text.wrap.unwrap_or(self.auto_wrap());
                    self.model.notes.push(SeqNote {
                        actor,
                        placement,
                        message: text.text.clone(),
                        wrap,
                    });
                    self.model.messages.push(SeqMessage {
                        id: self.model.messages.len().to_string(),
                        from,
                        to,
                        message: MessageText::Text(text.text),
                        wrap,
                        r#type: Some(NOTE),
                        activate: None,
                        placement: Some(placement),
                        central_connection: None,
                        answer: None,
                    })
                }
                Statement::Number(spec) => self.model.messages.push(SeqMessage {
                    id: self.model.messages.len().to_string(),
                    from: None,
                    to: None,
                    message: MessageText::Numbering(spec),
                    wrap: false,
                    r#type: Some(AUTONUMBER),
                    activate: None,
                    placement: None,
                    central_connection: None,
                    answer: None,
                }),
                Statement::Menu(kind, who, text) => {
                    let Some(i) = self.actor_index.get(&who).copied() else {
                        continue;
                    };
                    let actor = &mut self.model.actors[i].1;
                    match kind.as_str() {
                        "details" => actor.details = Some(text.text),
                        "link" => {
                            let len = text.text.encode_utf16().count() as isize;
                            let sep = text
                                .text
                                .find('@')
                                .map(|p| text.text[..p].encode_utf16().count() as isize)
                                .unwrap_or(-1);
                            let slice = |from: isize, to: isize| {
                                let from = if from < 0 {
                                    (len + from).max(0)
                                } else {
                                    from.min(len)
                                };
                                let to = if to < 0 {
                                    (len + to).max(0)
                                } else {
                                    to.min(len)
                                };
                                let data: Vec<_> = text.text.encode_utf16().collect();
                                String::from_utf16_lossy(
                                    &data[from as usize..to.max(from) as usize],
                                )
                            };
                            set(
                                &mut actor.links,
                                slice(0, sep - 1).trim_matches(space).into(),
                                super::json::SeqValue::value(Value::String(
                                    slice(sep + 1, len).trim_matches(space).into(),
                                )),
                            )
                        }
                        _ => {
                            if let Ok(values) = super::json::entries(&text.text) {
                                for (k, v) in values {
                                    if kind == "links" {
                                        set(&mut actor.links, k, v)
                                    } else {
                                        set(&mut actor.properties, k, v)
                                    }
                                }
                            }
                        }
                    }
                }
                Statement::BoxStart(s) => self.add_box(&s),
                Statement::BoxEnd => self.current_box = None,
            }
        }
        let boxes: Vec<_> = self
            .model
            .boxes
            .iter()
            .cloned()
            .map(std::sync::Arc::new)
            .collect();
        for (_, a) in &mut self.model.actors {
            a.r#box = a.box_index.map(|i| boxes[i].clone())
        }
        Ok(self.model)
    }
}

fn box_view<S: serde::Serializer>(
    value: &Option<std::sync::Arc<SeqBox>>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    value.as_deref().serialize(serializer)
}
