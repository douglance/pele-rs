//! Pele agentflow/graph.ts + colorSlots.ts, 458322a5 (MIT, kepano).
//! Resolves collapsed/global membership, containment diagnostics and stable kind palette slots.
use super::model::{vertex_kind, AgentDb, DiagnosticId, LabelType};
use crate::compat::{source::space, Object, Value};
use std::collections::{HashMap, HashSet};
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub(super) struct GraphNode {
    pub id: String,
    pub label: Option<String>,
    pub label_type: LabelType,
    pub shape: String,
    pub is_group: bool,
    pub parent_id: Option<String>,
    pub css_styles: Vec<String>,
    pub css_compiled_styles: Vec<String>,
    pub css_classes: String,
    pub dir: Option<String>,
    pub link: Option<String>,
    pub link_target: Option<String>,
    pub tooltip: Option<String>,
    pub color_index: Option<usize>,
    pub kind: Option<String>,
    pub metadata: Option<Object>,
}
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub(super) struct GraphEdge {
    pub id: String,
    pub is_user_defined_id: bool,
    pub start: String,
    pub end: String,
    pub edge_type: String,
    pub label: String,
    pub label_type: LabelType,
    pub thickness: Option<String>,
    pub pattern: Option<String>,
    pub minlen: Option<Value>,
    pub arrow_type_start: String,
    pub arrow_type_end: String,
    pub css_compiled_styles: Vec<String>,
    pub style: Vec<String>,
    pub classes: String,
    pub animate: Option<Value>,
    pub animation: Option<Value>,
    pub curve: Option<Value>,
    pub metadata: Option<Object>,
}
#[derive(Clone, Debug)]
pub(super) struct AgentGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}
fn container_slot(n: usize, palette: usize) -> usize {
    let slots = palette.saturating_sub(7).max(1);
    let slot = 7 + n % slots;
    if palette > 0 {
        slot % palette
    } else {
        slot
    }
}
fn collapsed(meta: &Option<Object>) -> bool {
    meta.as_ref().and_then(|m| m.get("view")) == Some(&Value::String("collapsed".into()))
}
pub(super) fn build(db: &mut AgentDb, config: &Object) -> AgentGraph {
    let subgraphs = db.model.subgraphs.clone();
    let positions: HashMap<_, _> = subgraphs
        .iter()
        .enumerate()
        .map(|(i, s)| (s.id.clone(), i))
        .collect();
    let mut hidden = HashSet::new();
    let mut hidden_order = vec![];
    let mut owner: HashMap<String, String> = HashMap::new();
    let mut walked = HashSet::new();
    for sg in &subgraphs {
        if !collapsed(&sg.metadata) || hidden.contains(&sg.id) {
            continue;
        }
        let mut pending = vec![sg.id.clone()];
        while let Some(id) = pending.pop() {
            let Some(&i) = positions.get(&id) else {
                continue;
            };
            for child in &subgraphs[i].nodes {
                if child == &sg.id || hidden.contains(child) {
                    continue;
                }
                hidden.insert(child.clone());
                hidden_order.push(child.clone());
                owner.insert(child.clone(), sg.id.clone());
                if !walked.contains(child) {
                    pending.push(child.clone());
                }
            }
        }
        walked.insert(sg.id.clone());
    }
    let mut collapsed_into: HashMap<String, String> = HashMap::new();
    for id in &hidden_order {
        let mut path = vec![id.clone()];
        let mut top = owner[id].clone();
        loop {
            let known = collapsed_into.get(&top);
            let up = known.or_else(|| owner.get(&top));
            let Some(up) = up else { break };
            if known.is_none() {
                path.push(top.clone());
            }
            top = up.clone();
            if known.is_some() {
                break;
            }
        }
        for id in path {
            collapsed_into.insert(id, top.clone());
        }
    }
    let mut parent_of: HashMap<String, String> = HashMap::new();
    for i in (0..subgraphs.len()).rev() {
        let sg = &subgraphs[i];
        if hidden.contains(&sg.id) {
            continue;
        }
        for id in &sg.nodes {
            let mut closes = false;
            if positions.get(id).is_some_and(|at| *at >= i) {
                let mut current = Some(sg.id.as_str());
                for _ in 0..=subgraphs.len() {
                    let Some(c) = current else { break };
                    if c == id {
                        closes = true;
                        break;
                    }
                    current = parent_of.get(c).map(String::as_str);
                }
            }
            if closes {
                db.emit(DiagnosticId::ContainmentViolation,"warning",format!("Container \"{}\" cannot contain \"{id}\" because \"{id}\" already contains it. The nesting that would close the loop is dropped.",sg.id),Some(id),None);
                continue;
            }
            parent_of.insert(id.clone(), sg.id.clone());
        }
    }
    let classes: HashMap<_, _> = db
        .model
        .classes
        .iter()
        .map(|(id, v)| (id.as_str(), v))
        .collect();
    let compiled = |names: &[String]| {
        let mut out = vec![];
        for name in names {
            if let Some(def) = classes.get(name.as_str()) {
                out.extend(
                    def.styles
                        .iter()
                        .chain(&def.text_styles)
                        .map(|s| s.trim_matches(space).to_owned()),
                );
            }
        }
        out
    };
    let mut nodes: Vec<GraphNode> = vec![];
    let mut by_id = HashMap::new();
    for sg in subgraphs.iter().rev() {
        if hidden.contains(&sg.id) {
            continue;
        }
        let is_collapsed = collapsed(&sg.metadata);
        let metadata = if is_collapsed {
            let mut meta = sg.metadata.clone().unwrap_or_default();
            meta.insert("containerType".into(), Value::String(sg.group_type.clone()));
            Some(meta)
        } else {
            sg.metadata.clone()
        };
        let node = GraphNode {
            id: sg.id.clone(),
            label: Some(sg.title.clone()),
            label_type: sg.label_type.clone(),
            shape: if is_collapsed {
                "collapsedGroup"
            } else {
                "flowGroup"
            }
            .into(),
            is_group: !is_collapsed,
            parent_id: parent_of.get(&sg.id).cloned(),
            css_styles: vec![],
            css_compiled_styles: compiled(&sg.classes),
            css_classes: sg.classes.join(" "),
            dir: sg.dir.clone(),
            link: None,
            link_target: None,
            tooltip: None,
            color_index: None,
            kind: None,
            metadata,
        };
        by_id.insert(sg.id.clone(), nodes.len());
        nodes.push(node);
    }
    let tooltip: HashMap<_, _> = db
        .model
        .tooltips
        .iter()
        .map(|(k, v)| (k.as_str(), v))
        .collect();
    for (_, v) in &db.model.nodes {
        if hidden.contains(&v.id) {
            continue;
        }
        if let Some(&i) = by_id.get(&v.id) {
            let node = &mut nodes[i];
            node.css_styles = v.styles.clone();
            node.css_compiled_styles = compiled(&v.classes);
            node.css_classes = v.classes.join(" ");
            continue;
        }
        let shape = v.shape.as_deref().map(super::shapes::resolve);
        let mut names = vec!["default".into(), "node".into()];
        names.extend(v.classes.iter().cloned());
        let node = GraphNode {
            id: v.id.clone(),
            label: v.text.clone(),
            label_type: v.label_type.clone(),
            shape: match shape {
                None | Some("square") => "squareRect",
                Some(s) => s,
            }
            .into(),
            is_group: false,
            parent_id: parent_of.get(&v.id).cloned(),
            css_styles: v.styles.clone(),
            css_compiled_styles: compiled(&names),
            css_classes: format!("default {}", v.classes.join(" ")),
            dir: v.dir.clone(),
            link: v.link.clone(),
            link_target: v.link_target.clone(),
            tooltip: tooltip.get(v.id.as_str()).cloned().cloned(),
            color_index: None,
            kind: None,
            metadata: v.metadata.clone(),
        };
        by_id.insert(v.id.clone(), nodes.len());
        nodes.push(node);
    }
    let default_curve = config
        .get("flowchart")
        .and_then(Value::as_object)
        .and_then(|c| c.get("curve"))
        .filter(|v| matches!(v, Value::String(_)))
        .cloned();
    let mut edges = vec![];
    for (i, raw) in db.model.edges.iter().enumerate() {
        let start = collapsed_into.get(&raw.start).unwrap_or(&raw.start);
        let end = collapsed_into.get(&raw.end).unwrap_or(&raw.end);
        if start == end && raw.start != raw.end {
            continue;
        }
        let mut styles = db.model.default_edge_style.clone().unwrap_or_default();
        if let Some(style) = &raw.style {
            styles.extend(style.iter().cloned());
        }
        let arrow = match raw.edge_type.as_deref() {
            Some("arrow_open") => "none",
            Some("arrow_cross") => "arrow_cross",
            _ => "arrow_point",
        };
        edges.push(GraphEdge {
            id: raw
                .id
                .clone()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| format!("L_{start}_{end}_{i}")),
            is_user_defined_id: raw.is_user_defined_id,
            start: start.clone(),
            end: end.clone(),
            edge_type: raw.edge_type.clone().unwrap_or_else(|| "normal".into()),
            label: raw.text.clone(),
            label_type: raw.label_type.clone(),
            thickness: raw.stroke.clone(),
            pattern: raw.stroke.clone(),
            minlen: raw.length.clone(),
            arrow_type_start: "none".into(),
            arrow_type_end: arrow.into(),
            css_compiled_styles: compiled(&raw.classes),
            style: styles,
            classes: "edge-thickness-normal edge-pattern-solid flowchart-link".into(),
            animate: raw.animate.clone(),
            animation: raw.animation.clone(),
            curve: raw
                .interpolate
                .clone()
                .filter(Value::truthy)
                .or_else(|| db.model.default_interpolate.clone().filter(Value::truthy))
                .or_else(|| default_curve.clone()),
            metadata: raw.metadata.clone(),
        });
    }
    super::shapes::normalize(&mut nodes, db);
    let mut children: HashMap<String, Vec<String>> = HashMap::new();
    for sg in &subgraphs {
        if let Some(parent) = parent_of.get(&sg.id) {
            children
                .entry(parent.clone())
                .or_default()
                .push(sg.id.clone());
        }
    }
    let mut order = HashMap::new();
    for sg in &subgraphs {
        if parent_of.contains_key(&sg.id) {
            continue;
        }
        let mut pending = vec![sg.id.clone()];
        while let Some(id) = pending.pop() {
            if order.contains_key(&id) {
                continue;
            }
            order.insert(id.clone(), order.len());
            if let Some(kids) = children.get(&id) {
                pending.extend(kids.iter().rev().cloned());
            }
        }
    }
    let vertices: HashMap<_, _> = db
        .model
        .nodes
        .iter()
        .map(|(k, v)| (k.as_str(), v))
        .collect();
    let connectors: HashSet<_> = db
        .model
        .connectors
        .iter()
        .map(|(k, _)| k.as_str())
        .collect();
    let mut fallback = order.len();
    for node in &mut nodes {
        if node.is_group || node.shape == "collapsedGroup" {
            let n = if let Some(i) = order.get(&node.id) {
                *i
            } else {
                let i = fallback;
                fallback += 1;
                i
            };
            node.color_index = Some(container_slot(n, 8));
            continue;
        }
        let kind = if connectors.contains(node.id.as_str()) {
            Some("connector")
        } else {
            vertices.get(node.id.as_str()).map(|v| vertex_kind(v))
        };
        if let Some(kind) = kind {
            if super::shapes::kind_slot(kind).is_some() {
                node.kind = Some(kind.into());
            }
        }
    }
    AgentGraph { nodes, edges }
}
