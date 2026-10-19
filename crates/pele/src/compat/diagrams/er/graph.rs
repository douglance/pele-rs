//! Pele er/graph.ts resolved drawable graph, 458322a5 (MIT, kepano).
use super::model::*;
use std::collections::{HashMap, HashSet};
#[derive(Clone)]
pub(super) struct Node {
    pub id: String,
    pub label: String,
    pub alias: String,
    pub attributes: Vec<ErAttribute>,
    pub is_group: bool,
    pub parent_id: Option<String>,
    pub css_styles: Vec<String>,
    pub css_compiled_styles: Vec<String>,
    pub css_classes: String,
    pub dir: Option<String>,
}
pub(super) struct Edge {
    pub id: String,
    pub start: String,
    pub end: String,
    pub label: String,
    pub arrow_start: &'static str,
    pub arrow_end: &'static str,
    pub dashed: bool,
}
pub(super) fn build(model: &ErModel) -> (Vec<Node>, Vec<Edge>) {
    let compiled = |names: Vec<&str>| {
        let mut out = vec![];
        for n in names {
            if let Some((_, d)) = model.classes.iter().find(|(k, _)| k == n) {
                out.extend(
                    d.styles
                        .iter()
                        .chain(&d.text_styles)
                        .map(|s| s.trim_matches(crate::compat::source::space).to_owned()),
                )
            }
        }
        out
    };
    let mut parent = HashMap::new();
    let mut ids = HashSet::new();
    for s in model.subgraphs.iter().rev() {
        ids.insert(s.id.clone());
        for id in &s.nodes {
            parent.insert(id.clone(), s.id.clone());
        }
    }
    let mut nodes = vec![];
    for s in model.subgraphs.iter().rev() {
        nodes.push(Node {
            id: s.id.clone(),
            label: s.title.clone(),
            alias: String::new(),
            attributes: vec![],
            is_group: true,
            parent_id: parent.get(&s.id).cloned(),
            css_styles: s.css_styles.clone(),
            css_compiled_styles: compiled(s.classes.iter().map(String::as_str).collect()),
            css_classes: s.classes.join(" "),
            dir: s.dir.clone(),
        })
    }
    let mut moved = HashMap::new();
    for (name, e) in &model.entities {
        if ids.contains(name) {
            moved.insert(e.id.clone(), name.clone());
            continue;
        }
        nodes.push(Node {
            id: e.id.clone(),
            label: e.label.clone(),
            alias: e.alias.clone(),
            attributes: e.attributes.clone(),
            is_group: false,
            parent_id: parent.get(name).cloned(),
            css_styles: e.css_styles.clone(),
            css_compiled_styles: compiled(e.css_classes.split(' ').collect()),
            css_classes: e.css_classes.clone(),
            dir: None,
        })
    }
    let edges = model
        .relationships
        .iter()
        .enumerate()
        .map(|(i, r)| Edge {
            id: format!("id_{}_{}_{i}", r.entity_a, r.entity_b),
            start: moved.get(&r.entity_a).unwrap_or(&r.entity_a).clone(),
            end: moved.get(&r.entity_b).unwrap_or(&r.entity_b).clone(),
            label: r.role_a.clone(),
            arrow_start: r.rel_spec.card_b.marker(),
            arrow_end: r.rel_spec.card_a.marker(),
            dashed: r.rel_spec.rel_type == Identification::NON_IDENTIFYING,
        })
        .collect();
    (nodes, edges)
}
