//! Pele state/types.ts and db.ts model, 458322a5 (MIT, kepano).
use crate::compat::{source::space, DiagramType, ParseContext, PeleError};
use serde::Serialize;
use std::collections::HashMap;
#[derive(Clone, Debug, Serialize)]
pub struct StateNote {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<String>,
    pub text: String,
}
#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum Description {
    Text(String),
    Lines(Vec<String>),
}
impl Description {
    pub(super) fn truthy(&self) -> bool {
        match self {
            Self::Text(s) => !s.is_empty(),
            Self::Lines(_) => true,
        }
    }
    pub(super) fn len(&self) -> usize {
        match self {
            Self::Text(s) => s.encode_utf16().count(),
            Self::Lines(v) => v.len(),
        }
    }
    pub(super) fn lines(&self) -> Vec<String> {
        match self {
            Self::Text(s) => vec![s.clone()],
            Self::Lines(v) => v.clone(),
        }
    }
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StateStmt {
    pub stmt: &'static str,
    pub id: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub state_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Description>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub descriptions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<Vec<Stmt>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<StateNote>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub styles: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_styles: Option<Vec<String>>,
}
// State documents can be thousands of composites deep. Clone and release their
// owned trees without growing the native stack; parsing and graph traversal are iterative.
impl StateStmt {
    fn shallow_clone(&self) -> Self {
        Self {
            stmt: self.stmt,
            id: self.id.clone(),
            state_type: self.state_type.clone(),
            description: self.description.clone(),
            descriptions: self.descriptions.clone(),
            doc: self.doc.as_ref().map(|_| Vec::new()),
            note: self.note.clone(),
            start: self.start,
            classes: self.classes.clone(),
            styles: self.styles.clone(),
            text_styles: self.text_styles.clone(),
        }
    }
}
impl Clone for StateStmt {
    fn clone(&self) -> Self {
        let mut result = self.shallow_clone();
        let mut pending = Vec::new();
        if let (Some(input), Some(output)) = (&self.doc, &mut result.doc) {
            pending.push((input.as_slice(), output));
        }
        while let Some((input, output)) = pending.pop() {
            output.extend(input.iter().map(|item| match item {
                Stmt::State(s) => Stmt::State(s.shallow_clone()),
                Stmt::Relation(r) => Stmt::Relation(RelationStmt {
                    stmt: r.stmt,
                    state1: r.state1.shallow_clone(),
                    state2: r.state2.shallow_clone(),
                    description: r.description.clone(),
                }),
                Stmt::Click(c) => Stmt::Click(ClickStmt {
                    stmt: c.stmt,
                    id: match &c.id {
                        ClickId::State(s) => ClickId::State(s.shallow_clone()),
                        ClickId::String(s) => ClickId::String(s.clone()),
                    },
                    url: c.url.clone(),
                    tooltip: c.tooltip.clone(),
                }),
                item => item.clone(),
            }));
            for (input, output) in input.iter().zip(output.iter_mut()) {
                fn add<'a>(
                    a: &'a StateStmt,
                    b: &'a mut StateStmt,
                    pending: &mut Vec<(&'a [Stmt], &'a mut Vec<Stmt>)>,
                ) {
                    if let (Some(a), Some(b)) = (&a.doc, &mut b.doc) {
                        pending.push((a.as_slice(), b));
                    }
                }
                match (input, output) {
                    (Stmt::State(a), Stmt::State(b)) => add(a, b, &mut pending),
                    (Stmt::Relation(a), Stmt::Relation(b)) => {
                        add(&a.state1, &mut b.state1, &mut pending);
                        add(&a.state2, &mut b.state2, &mut pending)
                    }
                    (Stmt::Click(a), Stmt::Click(b)) => {
                        if let (ClickId::State(a), ClickId::State(b)) = (&a.id, &mut b.id) {
                            add(a, b, &mut pending)
                        }
                    }
                    _ => {}
                }
            }
        }
        result
    }
}
impl Drop for StateStmt {
    fn drop(&mut self) {
        let mut pending = self.doc.take().unwrap_or_default();
        while let Some(mut item) = pending.pop() {
            match &mut item {
                Stmt::State(s) => pending.extend(s.doc.take().unwrap_or_default()),
                Stmt::Relation(r) => {
                    pending.extend(r.state1.doc.take().unwrap_or_default());
                    pending.extend(r.state2.doc.take().unwrap_or_default());
                }
                Stmt::Click(c) => {
                    if let ClickId::State(s) = &mut c.id {
                        pending.extend(s.doc.take().unwrap_or_default())
                    }
                }
                _ => {}
            }
        }
    }
}
impl StateStmt {
    pub(super) fn new(id: String, state_type: Option<&str>) -> Self {
        Self {
            stmt: "state",
            id,
            state_type: state_type.map(str::to_string),
            description: None,
            descriptions: None,
            doc: None,
            note: None,
            start: None,
            classes: None,
            styles: None,
            text_styles: None,
        }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct RelationStmt {
    pub stmt: &'static str,
    pub state1: StateStmt,
    pub state2: StateStmt,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct ClassDefStmt {
    pub stmt: &'static str,
    pub id: String,
    pub classes: String,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StyleStmt {
    pub stmt: &'static str,
    pub id: String,
    pub style_class: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct DirectionStmt {
    pub stmt: &'static str,
    pub value: String,
}
#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum ClickId {
    State(StateStmt),
    String(String),
}
impl ClickId {
    pub(super) fn id(&self) -> &str {
        match self {
            Self::State(s) => &s.id,
            Self::String(s) => s,
        }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct ClickStmt {
    pub stmt: &'static str,
    pub id: ClickId,
    pub url: String,
    pub tooltip: String,
}
#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum Stmt {
    Text(String),
    State(StateStmt),
    Relation(RelationStmt),
    ClassDef(ClassDefStmt),
    Style(StyleStmt),
    Direction(DirectionStmt),
    Click(ClickStmt),
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StateRelation {
    pub id1: String,
    pub id2: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relation_title: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StateClassDef {
    pub id: String,
    pub styles: Vec<String>,
    pub text_styles: Vec<String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct StateLink {
    pub url: String,
    pub tooltip: String,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StateModel {
    #[serde(skip)]
    pub public_divider_count: usize,
    #[serde(rename = "type")]
    pub diagram_type: DiagramType,
    pub direction: String,
    pub root_doc: Vec<Stmt>,
    pub states: Vec<(String, StateStmt)>,
    pub relations: Vec<StateRelation>,
    pub classes: Vec<(String, StateClassDef)>,
    pub links: Vec<(String, StateLink)>,
    pub graph: super::graph::StateGraph,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acc_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acc_descr: Option<String>,
}
pub(super) struct Db {
    pub model: StateModel,
    pub divider_count: usize,
    states: HashMap<String, usize>,
    classes: HashMap<String, usize>,
    links: HashMap<String, usize>,
}
impl Db {
    pub fn new(context: &ParseContext<'_>) -> Self {
        Self {
            model: StateModel {
                diagram_type: DiagramType::State,
                public_divider_count: 0,
                direction: "TB".into(),
                root_doc: Vec::new(),
                states: Vec::new(),
                relations: Vec::new(),
                classes: Vec::new(),
                links: Vec::new(),
                graph: super::graph::StateGraph::default(),
                title: context.title.filter(|s| !s.is_empty()).map(str::to_string),
                acc_title: None,
                acc_descr: None,
            },
            divider_count: 0,
            states: HashMap::new(),
            classes: HashMap::new(),
            links: HashMap::new(),
        }
    }
    pub fn divider(&mut self) -> String {
        self.divider_count += 1;
        format!("divider-id-{}", self.divider_count)
    }
    pub fn acc_title(&mut self, text: String) {
        self.model.acc_title = Some(text.trim_start_matches(space).into())
    }
    pub fn acc_descr(&mut self, text: String) {
        let mut out = String::new();
        let mut after = false;
        for c in text.chars() {
            if after && space(c) {
                continue;
            }
            out.push(c);
            after = c == '\n'
        }
        self.model.acc_descr = Some(out)
    }
    pub fn finish(mut self, mut doc: Vec<Stmt>) -> Result<StateModel, PeleError> {
        self.translate_doc("root", &mut doc);
        self.translate(&mut doc);
        self.model.direction = direction(&doc);
        self.extract(&doc);
        self.model.graph = super::graph::build(&doc, &self.model.states, &self.model.classes)?;
        self.model.root_doc = doc;
        self.model.public_divider_count = self.divider_count;
        Ok(self.model)
    }
    fn translate_doc(&mut self, _id: &str, doc: &mut Vec<Stmt>) {
        let mut regions = Vec::new();
        let mut current = Vec::new();
        for item in std::mem::take(doc) {
            if let Stmt::State(state) = &item {
                if state.state_type.as_deref() == Some("divider") && state.doc.is_none() {
                    let mut region = state.clone();
                    region.doc = Some(std::mem::take(&mut current));
                    regions.push(Stmt::State(region));
                    continue;
                }
            }
            current.push(item)
        }
        if !regions.is_empty() {
            if !current.is_empty() {
                let mut region = StateStmt::new(self.divider(), Some("divider"));
                region.doc = Some(current);
                regions.push(Stmt::State(region))
            }
            *doc = regions
        } else {
            *doc = current
        }
    }
    fn translate(&mut self, doc: &mut Vec<Stmt>) {
        enum Item<'a> {
            Statement(&'a mut Stmt),
            State(&'a mut StateStmt, bool),
        }
        let mut pending: Vec<(String, Item<'_>)> = doc
            .iter_mut()
            .rev()
            .map(|s| ("root".into(), Item::Statement(s)))
            .collect();
        while let Some((parent, item)) = pending.pop() {
            let (state, first) = match item {
                Item::State(s, first) => (s, first),
                Item::Statement(s) => match s {
                    Stmt::State(s) => (s, true),
                    Stmt::Relation(r) => {
                        pending.push((parent.clone(), Item::State(&mut r.state2, false)));
                        pending.push((parent, Item::State(&mut r.state1, true)));
                        continue;
                    }
                    _ => continue,
                },
            };
            if state.id == "[*]" {
                state.id = format!("{parent}_{}", if first { "start" } else { "end" });
                state.start = Some(first)
            } else {
                state.id = state.id.trim_matches(space).into()
            }
            if let Some(doc) = &mut state.doc {
                self.translate_doc(&state.id, doc);
                for child in doc.iter_mut().rev() {
                    pending.push((state.id.clone(), Item::Statement(child)))
                }
            }
        }
    }
    fn add_state(&mut self, item: &StateStmt, with_classes: bool) {
        let id = item.id.trim_matches(space).to_string();
        let i = if let Some(&i) = self.states.get(&id) {
            i
        } else {
            let i = self.model.states.len();
            let mut state = StateStmt::new(
                id.clone(),
                Some(item.state_type.as_deref().unwrap_or("default")),
            );
            state.doc = item.doc.clone();
            state.note = item.note.clone();
            state.descriptions = Some(Vec::new());
            state.classes = Some(Vec::new());
            state.styles = Some(Vec::new());
            state.text_styles = Some(Vec::new());
            self.model.states.push((id.clone(), state));
            self.states.insert(id.clone(), i);
            i
        };
        let state = &mut self.model.states[i].1;
        if state.doc.is_none() {
            state.doc = item.doc.clone()
        }
        if state.state_type.is_none() {
            state.state_type = Some(item.state_type.clone().unwrap_or_else(|| "default".into()))
        }
        if let Some(description) = item.description.as_ref().filter(|d| d.truthy()) {
            for text in description.lines() {
                let text = text.trim_matches(space);
                let text = if text.starts_with(':') {
                    text.replacen(':', "", 1).trim_matches(space).into()
                } else {
                    text.into()
                };
                state.descriptions.as_mut().unwrap().push(text)
            }
        }
        if let Some(note) = &item.note {
            state.note = Some(note.clone())
        }
        if with_classes {
            if let Some(classes) = &item.classes {
                for name in classes {
                    self.set_class(&id, name.trim_matches(space))
                }
            }
        }
    }
    fn named(&mut self, ids: &str) -> Vec<usize> {
        let mut result = Vec::new();
        for id in ids.split(',') {
            if let Some(&i) = self.states.get(id) {
                result.push(i)
            } else {
                let state = StateStmt::new(id.into(), Some("default"));
                self.add_state(&state, false);
                if let Some(&i) = self.states.get(id.trim_matches(space)) {
                    result.push(i)
                }
            }
        }
        result
    }
    fn set_class(&mut self, ids: &str, class: &str) {
        for i in self.named(ids) {
            self.model.states[i]
                .1
                .classes
                .as_mut()
                .unwrap()
                .push(class.into())
        }
    }
    fn add_class(&mut self, id: &str, attributes: &str) {
        let i = if let Some(&i) = self.classes.get(id) {
            i
        } else {
            let i = self.model.classes.len();
            self.classes.insert(id.into(), i);
            self.model.classes.push((
                id.into(),
                StateClassDef {
                    id: id.into(),
                    styles: Vec::new(),
                    text_styles: Vec::new(),
                },
            ));
            i
        };
        if attributes.is_empty() {
            return;
        }
        let found = &mut self.model.classes[i].1;
        for attrib in attributes.split(',') {
            let fixed = attrib.replacen(';', "", 1).trim_matches(space).to_string();
            if attrib.contains("color") {
                found.text_styles.push(
                    fixed
                        .replacen("fill", "bgFill", 1)
                        .replacen("color", "fill", 1),
                )
            }
            found.styles.push(fixed)
        }
    }
    fn extract(&mut self, doc: &[Stmt]) {
        for item in doc {
            match item {
                Stmt::State(s) => self.add_state(s, false),
                Stmt::Relation(r) => {
                    self.add_state(&r.state1, true);
                    self.add_state(&r.state2, true);
                    self.model.relations.push(StateRelation {
                        id1: r.state1.id.trim_matches(space).into(),
                        id2: r.state2.id.trim_matches(space).into(),
                        relation_title: Some(r.description.clone().unwrap_or_default()),
                    })
                }
                Stmt::ClassDef(c) => self.add_class(c.id.trim_matches(space), &c.classes),
                Stmt::Style(s) => {
                    if s.stmt == "style" {
                        let styles: Vec<_> = s
                            .style_class
                            .split(',')
                            .map(|s| s.replace(';', "").trim_matches(space).into())
                            .collect();
                        for i in self.named(s.id.trim_matches(space)) {
                            self.model.states[i].1.styles = Some(styles.clone())
                        }
                    } else {
                        self.set_class(s.id.trim_matches(space), &s.style_class)
                    }
                }
                Stmt::Click(c) => {
                    let id = c.id.id();
                    if let Some(&i) = self.links.get(id) {
                        self.model.links[i].1 = StateLink {
                            url: c.url.clone(),
                            tooltip: c.tooltip.clone(),
                        }
                    } else {
                        self.links.insert(id.into(), self.model.links.len());
                        self.model.links.push((
                            id.into(),
                            StateLink {
                                url: c.url.clone(),
                                tooltip: c.tooltip.clone(),
                            },
                        ))
                    }
                }
                _ => {}
            }
        }
    }
}
pub(super) fn direction(doc: &[Stmt]) -> String {
    doc.iter()
        .filter_map(|s| {
            if let Stmt::Direction(d) = s {
                Some(d.value.as_str())
            } else {
                None
            }
        })
        .next_back()
        .unwrap_or("TB")
        .into()
}
pub(super) fn trim_colon(text: &str) -> String {
    text.strip_prefix(':')
        .unwrap_or(text)
        .trim_matches(space)
        .into()
}
