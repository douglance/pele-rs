//! Pele state/graph.ts normalized drawing graph, 458322a5 (MIT, kepano).
use super::model::{direction, Description, StateClassDef, StateStmt, Stmt};
use crate::compat::{source::space, ErrorCode, PeleError};
use serde::Serialize;
use std::collections::HashMap;
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StateNode {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shape: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub description: Vec<String>,
    pub css_classes: String,
    pub css_compiled_styles: Vec<String>,
    pub css_styles: Vec<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub node_type: Option<String>,
    pub is_group: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note_for: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_index: Option<usize>,
}
#[derive(Clone, Debug, Serialize)]
pub struct StateEdge {
    pub id: String,
    pub start: String,
    pub end: String,
    pub label: String,
    pub classes: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct StateGraph {
    pub nodes: Vec<StateNode>,
    pub edges: Vec<StateEdge>,
}
#[derive(Clone)]
struct Draft {
    shape: Option<String>,
    description: Description,
    css_classes: String,
    css_styles: Vec<String>,
    node_type: Option<String>,
    dir: Option<String>,
    color_index: Option<usize>,
}
struct Builder<'a> {
    graph: StateGraph,
    by_id: HashMap<String, usize>,
    drafts: HashMap<String, Draft>,
    slots: HashMap<String, Option<usize>>,
    counter: usize,
    next_slot: usize,
    states: HashMap<&'a str, &'a StateStmt>,
    classes: HashMap<&'a str, &'a StateClassDef>,
}
impl Builder<'_> {
    fn insert(&mut self, mut node: StateNode) {
        if node.id.is_empty() || matches!(node.id.as_str(), "</join></fork>" | "</choice>") {
            return;
        }
        if !node.css_classes.is_empty() {
            for name in node.css_classes.split(' ') {
                if let Some(class) = self.classes.get(name) {
                    node.css_compiled_styles
                        .extend(class.styles.iter().cloned())
                }
            }
        }
        if let Some(&i) = self.by_id.get(&node.id) {
            if node.parent_id.is_none() {
                node.parent_id = self.graph.nodes[i].parent_id.clone()
            }
            self.graph.nodes[i] = node
        } else {
            self.by_id.insert(node.id.clone(), self.graph.nodes.len());
            self.graph.nodes.push(node)
        }
    }
    fn visit(&mut self, parent: Option<&StateStmt>, item: &StateStmt, alt: bool) {
        let id = &item.id;
        if id == "root" {
            return;
        }
        let known = self.states.get(id.as_str()).copied();
        let class_str = known
            .and_then(|s| s.classes.as_ref())
            .map_or_else(String::new, |s| s.join(" "));
        let styles = known
            .and_then(|s| s.styles.as_ref())
            .cloned()
            .unwrap_or_default();
        let mut draft = self.drafts.remove(id).unwrap_or_else(|| {
            let shape = if item.state_type.as_deref() != Some("default") {
                item.state_type.clone()
            } else {
                Some(
                    match item.start {
                        Some(true) => "stateStart",
                        Some(false) => "stateEnd",
                        None => "rect",
                    }
                    .into(),
                )
            };
            Draft {
                shape,
                description: Description::Text(id.clone()),
                css_classes: format!("{class_str} statediagram-state"),
                css_styles: styles.clone(),
                node_type: None,
                dir: None,
                color_index: None,
            }
        });
        if let Some(description) = item.description.as_ref().filter(|s| s.truthy()) {
            match &draft.description {
                Description::Lines(lines) => {
                    draft.shape = Some("rectWithTitle".into());
                    let mut full = lines.clone();
                    full.extend(description.lines());
                    draft.description = Description::Lines(full)
                }
                Description::Text(s) if !s.is_empty() => {
                    draft.shape = Some("rectWithTitle".into());
                    let mut full = if s == id { Vec::new() } else { vec![s.clone()] };
                    full.extend(description.lines());
                    draft.description = Description::Lines(full)
                }
                _ => {
                    draft.shape = Some("rect".into());
                    draft.description = description.clone()
                }
            }
        }
        if draft.description.len() == 1 && draft.shape.as_deref() == Some("rectWithTitle") {
            draft.shape = Some(
                if draft.node_type.as_deref() == Some("group") {
                    "roundedWithTitle"
                } else {
                    "rect"
                }
                .into(),
            )
        }
        if draft.node_type.is_none() {
            if let Some(doc) = &item.doc {
                draft.node_type = Some("group".into());
                draft.dir = Some(direction(doc));
                draft.shape = Some(
                    if item.state_type.as_deref() == Some("divider") {
                        "divider"
                    } else {
                        "roundedWithTitle"
                    }
                    .into(),
                );
                if draft.shape.as_deref() == Some("divider")
                    && parent.is_some_and(|p| self.slots.contains_key(&p.id))
                {
                    draft.color_index = self.slots[&parent.unwrap().id]
                } else {
                    let slot = self.next_slot;
                    self.next_slot += 1;
                    draft.color_index =
                        if !class_str.trim_matches(space).is_empty() || !styles.is_empty() {
                            None
                        } else {
                            Some(slot)
                        }
                }
                self.slots.insert(id.clone(), draft.color_index);
                draft.css_classes = format!(
                    "{} statediagram-cluster {}",
                    draft.css_classes,
                    if alt { "statediagram-cluster-alt" } else { "" }
                )
            }
        }
        let full = draft.description.lines();
        let label = match &draft.description {
            Description::Text(s) => Some(s.clone()),
            Description::Lines(lines) => lines.first().cloned(),
        };
        let mut node = StateNode {
            id: id.clone(),
            shape: draft.shape.clone(),
            label,
            description: if matches!(draft.description, Description::Lines(_)) {
                full.into_iter().skip(1).collect()
            } else {
                Vec::new()
            },
            css_classes: draft.css_classes.clone(),
            css_compiled_styles: Vec::new(),
            css_styles: draft.css_styles.clone(),
            node_type: draft.node_type.clone(),
            is_group: draft.node_type.as_deref() == Some("group"),
            dir: draft.dir.clone(),
            color_index: draft.color_index,
            parent_id: parent.filter(|p| p.id != "root").map(|p| p.id.clone()),
            position: None,
            note_for: None,
        };
        if node.shape.as_deref() == Some("divider") {
            node.label = Some(String::new())
        }
        self.drafts.insert(id.clone(), draft.clone());
        let Some(note) = &item.note else {
            self.insert(node);
            return;
        };
        let aside = |id: String, shape: &str, classes: String, group: bool| StateNode {
            id,
            shape: Some(shape.into()),
            label: Some(note.text.clone()),
            description: Vec::new(),
            css_classes: classes,
            css_compiled_styles: Vec::new(),
            css_styles: Vec::new(),
            node_type: Some(if group { "group" } else { "node" }.into()),
            is_group: group,
            dir: None,
            position: note.position.clone(),
            note_for: None,
            color_index: None,
            parent_id: None,
        };
        let mut note_node = aside(
            format!("{id}----note-{}", self.counter),
            "note",
            "statediagram-note".into(),
            false,
        );
        self.counter += 1;
        let note_parent = format!("{id}----parent");
        note_node.parent_id = Some(note_parent.clone());
        note_node.note_for = Some(id.clone());
        let note_id = note_node.id.clone();
        self.insert(aside(note_parent, "noteGroup", draft.css_classes, true));
        self.insert(note_node);
        self.insert(node);
        let (from, to) = if note.position.as_deref() == Some("left of") {
            (note_id, id.clone())
        } else {
            (id.clone(), note_id)
        };
        self.graph.edges.push(StateEdge {
            id: format!("{from}-{to}"),
            start: from,
            end: to,
            label: String::new(),
            classes: "transition note-edge".into(),
            pattern: Some("dashed".into()),
        });
    }
}
pub(super) fn build(
    root: &[Stmt],
    states: &[(String, StateStmt)],
    classes: &[(String, StateClassDef)],
) -> Result<StateGraph, PeleError> {
    let mut b = Builder {
        graph: StateGraph::default(),
        by_id: HashMap::new(),
        drafts: HashMap::new(),
        slots: HashMap::new(),
        counter: 0,
        next_slot: 0,
        states: states.iter().map(|(k, v)| (k.as_str(), v)).collect(),
        classes: classes.iter().map(|(k, v)| (k.as_str(), v)).collect(),
    };
    let mut stack = vec![(None, root, 0usize, true)];
    while let Some((parent, doc, at, alt)) = stack.last_mut() {
        if *at >= doc.len() {
            stack.pop();
            continue;
        }
        let item = &doc[*at];
        *at += 1;
        match item {
            Stmt::State(s) => {
                b.visit(*parent, s, *alt);
                if let Some(doc) = &s.doc {
                    let next_alt = !*alt;
                    stack.push((Some(s), doc, 0, next_alt))
                }
            }
            Stmt::Relation(r) => {
                b.visit(*parent, &r.state1, *alt);
                b.visit(*parent, &r.state2, *alt);
                b.graph.edges.push(StateEdge {
                    id: format!("edge{}", b.counter),
                    start: r.state1.id.clone(),
                    end: r.state2.id.clone(),
                    label: r.description.clone().unwrap_or_default(),
                    classes: "transition".into(),
                    pattern: None,
                });
                b.counter += 1
            }
            _ => {}
        }
    }
    for node in &b.graph.nodes {
        if node.is_group && !node.description.is_empty() {
            let mut e=PeleError::new(format!("Group nodes can only have label. Remove the additional description for node [{}]",node.id),ErrorCode::Semantic);
            e.diagram_type = Some("state".into());
            return Err(e);
        }
    }
    Ok(b.graph)
}
