//! Pele usecase/graph.ts, 458322a5 (MIT, kepano). Typed graph projection,
//! authored-order JSON rows, class property precedence and relationship markers.
use super::{
    json::{JsonAtom, JsonValue},
    model::*,
};
use crate::compat::source::space;
use std::collections::HashMap;
#[derive(Clone, Debug)]
pub struct JsonRow {
    pub key: String,
    pub accessible_key: String,
    pub value: String,
}
#[derive(Clone, Default)]
pub(super) struct Node {
    pub id: String,
    pub label: String,
    pub label_type: String,
    pub shape: String,
    pub group: bool,
    pub padding: f64,
    pub classes: String,
    pub styles: Vec<String>,
    pub compiled: Vec<String>,
    pub actor_type: Option<String>,
    pub business: bool,
    pub icon: Option<String>,
    pub stereotype: Option<String>,
    pub parent: Option<String>,
    pub rows: Option<Vec<JsonRow>>,
    pub boundary_type: Option<String>,
}
#[derive(Clone, Default)]
pub(super) struct Edge {
    pub id: String,
    pub start: String,
    pub end: String,
    pub kind: String,
    pub dotted: bool,
    pub arrow_start: String,
    pub arrow_end: String,
    pub label: Option<String>,
    pub label_type: Option<String>,
    pub classes: String,
    pub styles: Vec<String>,
    pub compiled: Vec<String>,
    pub animate: bool,
    pub minlen: usize,
    pub internal: bool,
}
fn scalar(a: &JsonAtom) -> String {
    match a {
        JsonAtom::String(s) => s.clone(),
        JsonAtom::Number(n) => crate::compat::svg::js_num(*n),
        JsonAtom::Bool(b) => b.to_string(),
        JsonAtom::Null => "null".into(),
        _ => unreachable!(),
    }
}
pub fn flatten_json_rows(value: &JsonValue, order: &[(String, Vec<String>)]) -> Vec<JsonRow> {
    let order: HashMap<_, _> = order.iter().map(|(k, v)| (k.as_str(), v)).collect();
    let mut rows = vec![];
    let mut stack = vec![(value.root, String::new(), String::new())];
    while let Some((i, path, pointer)) = stack.pop() {
        match &value.atoms[i] {
            JsonAtom::Array(a) => {
                if a.is_empty() {
                    rows.push(JsonRow {
                        key: path.clone(),
                        accessible_key: path,
                        value: "[]".into(),
                    })
                } else if a
                    .iter()
                    .all(|&i| !matches!(value.atoms[i], JsonAtom::Array(_) | JsonAtom::Object(_)))
                {
                    for (k, &i) in a.iter().enumerate() {
                        rows.push(JsonRow {
                            key: if k == 0 { path.clone() } else { String::new() },
                            accessible_key: path.clone(),
                            value: scalar(&value.atoms[i]),
                        })
                    }
                } else {
                    for (k, &i) in a.iter().enumerate().rev() {
                        stack.push((i, format!("{path}[{k}]"), format!("{pointer}/{k}")))
                    }
                }
            }
            JsonAtom::Object(o) => {
                let lookup: HashMap<_, _> = o.iter().map(|(k, v)| (k.as_str(), *v)).collect();
                let fallback: Vec<_> = o.iter().map(|(k, _)| k.clone()).collect();
                let keys = order.get(pointer.as_str()).copied().unwrap_or(&fallback);
                if keys.is_empty() {
                    rows.push(JsonRow {
                        key: path.clone(),
                        accessible_key: path.clone(),
                        value: "{}".into(),
                    })
                }
                for key in keys.iter().rev() {
                    if let Some(&i) = lookup.get(key.as_str()) {
                        stack.push((
                            i,
                            if path.is_empty() {
                                key.clone()
                            } else {
                                format!("{path}.{key}")
                            },
                            format!("{pointer}/{}", key.replace('~', "~0").replace('/', "~1")),
                        ))
                    }
                }
            }
            a => rows.push(JsonRow {
                key: path.clone(),
                accessible_key: path,
                value: scalar(a),
            }),
        }
    }
    rows
}
pub(super) fn build(m: &UsecaseModel) -> (Vec<Node>, Vec<Edge>) {
    let defs: HashMap<_, _> = m
        .class_defs
        .iter()
        .map(|(id, v)| (id.as_str(), v))
        .collect();
    let compiled = |names: &[String]| {
        let mut out: Vec<String> = vec![];
        let mut index = HashMap::new();
        for name in std::iter::once("default").chain(names.iter().map(String::as_str)) {
            if let Some(def) = defs.get(name) {
                for raw in def.styles.iter() {
                    let style = raw.trim_matches(space);
                    let property = style
                        .split_once(':')
                        .map_or(style, |(p, _)| p)
                        .trim_matches(space);
                    if property.is_empty() {
                        continue;
                    }
                    if let Some(&i) = index.get(property) {
                        out[i] = style.into()
                    } else {
                        index.insert(property.to_string(), out.len());
                        out.push(style.into())
                    }
                }
            }
        }
        out
    };
    let mut nodes = vec![];
    let mut edges = vec![];
    for (_, a) in &m.actors {
        let mut names = vec![
            "default".into(),
            "usecase-actor".into(),
            format!("usecase-actor-{}", a.actor_type),
        ];
        if a.business {
            names.push("usecase-business".into())
        }
        names.extend(a.classes.iter().cloned());
        nodes.push(Node {
            id: a.id.clone(),
            label: a.label.clone(),
            label_type: a.label_type.clone(),
            shape: match a.actor_type.as_str() {
                "hollow" => "usecaseActorHollow",
                "awesome" => "usecaseActorAwesome",
                "icon" => "usecaseActorIcon",
                _ => "usecaseActor",
            }
            .into(),
            padding: 10.,
            classes: names.join(" "),
            styles: a.styles.clone(),
            compiled: compiled(&a.classes),
            actor_type: Some(a.actor_type.clone()),
            business: a.business,
            icon: a.icon.clone().filter(|s| !s.is_empty()),
            stereotype: a.stereotype.clone().filter(|s| !s.is_empty()),
            parent: a.parent_id.clone().filter(|s| !s.is_empty()),
            ..Default::default()
        })
    }
    for (_, a) in &m.use_cases {
        let ellipse = a.shape == "ellipse";
        let mut names = vec![
            "default".into(),
            "usecase-element".into(),
            format!("usecase-{}", a.shape),
        ];
        if a.business {
            names.push("usecase-business".into())
        }
        names.extend(a.classes.iter().cloned());
        nodes.push(Node {
            id: a.id.clone(),
            label: a.label.clone(),
            label_type: a.label_type.clone(),
            shape: if !ellipse {
                "rect"
            } else if a.business {
                "usecaseBusiness"
            } else {
                "usecaseEllipse"
            }
            .into(),
            padding: if ellipse { 20. } else { 10. },
            classes: names.join(" "),
            styles: a.styles.clone(),
            compiled: compiled(&a.classes),
            business: a.business,
            stereotype: a.stereotype.clone().filter(|s| !s.is_empty()),
            parent: a.parent_id.clone().filter(|s| !s.is_empty()),
            ..Default::default()
        })
    }
    for (_, a) in &m.notes {
        nodes.push(Node {
            id: a.id.clone(),
            label: a.label.clone(),
            label_type: a.label_type.clone(),
            shape: "note".into(),
            padding: 10.,
            classes: "default usecase-note".into(),
            compiled: compiled(&[]),
            ..Default::default()
        })
    }
    for (_, a) in &m.json_nodes {
        let mut names = vec!["default".into(), "usecase-json-table".into()];
        names.extend(a.classes.iter().cloned());
        nodes.push(Node {
            id: a.id.clone(),
            label: a.id.clone(),
            label_type: "text".into(),
            shape: "usecaseJsonTable".into(),
            padding: 10.,
            classes: names.join(" "),
            styles: a.styles.clone(),
            compiled: compiled(&a.classes),
            rows: Some(flatten_json_rows(&a.value, &a.property_order)),
            ..Default::default()
        })
    }
    for (_, a) in &m.system_boundaries {
        let mut names = vec![
            "default".into(),
            "system-boundary".into(),
            format!("system-boundary-{}", a.boundary_type),
        ];
        names.extend(a.classes.iter().cloned());
        nodes.push(Node {
            id: a.id.clone(),
            label: a.label.clone(),
            label_type: a.label_type.clone(),
            shape: "usecaseSystemBoundary".into(),
            group: true,
            padding: 20.,
            classes: names.join(" "),
            styles: a.styles.clone(),
            compiled: compiled(&a.classes),
            boundary_type: Some(a.boundary_type.clone()),
            ..Default::default()
        })
    }
    let markers = [
        ("none", "arrow_point"),
        ("arrow_point", "none"),
        ("none", "none"),
        ("none", "arrow_circle"),
        ("none", "arrow_cross"),
        ("arrow_circle", "none"),
        ("arrow_cross", "none"),
    ];
    for a in &m.relationships {
        let semantic = a.relationship_type == "include" || a.relationship_type == "extend";
        let (start, end) = if a.relationship_type == "generalization" {
            ("none", "extension")
        } else if a.relationship_type != "association" {
            markers[0]
        } else {
            markers.get(a.arrow_type).copied().unwrap_or(markers[2])
        };
        let mut names = vec![
            "default".into(),
            "relationship".into(),
            format!("relationship-{}", a.relationship_type),
        ];
        names.extend(a.classes.iter().cloned());
        if a.animate || a.animation.is_some() {
            names.push(format!(
                "edge-animation-{}",
                a.animation.as_deref().unwrap_or("fast")
            ))
        }
        edges.push(Edge {
            id: a.id.clone(),
            start: a.source.clone(),
            end: a.target.clone(),
            kind: a.relationship_type.clone(),
            dotted: semantic,
            arrow_start: start.into(),
            arrow_end: end.into(),
            label: if semantic {
                Some(a.relationship_type.clone())
            } else if a.relationship_type == "association" {
                a.label.clone().filter(|s| !s.is_empty())
            } else {
                None
            },
            label_type: if semantic {
                Some("text".into())
            } else if a.relationship_type == "association" {
                a.label_type.clone()
            } else {
                None
            },
            classes: names.join(" "),
            styles: a.styles.clone(),
            compiled: compiled(&a.classes),
            animate: a.animate,
            minlen: a.minlen,
            internal: false,
        })
    }
    for (_, a) in &m.notes {
        edges.push(Edge {
            id: format!("{}-edge", a.id),
            start: a.id.clone(),
            end: a.target.clone(),
            kind: "note".into(),
            dotted: true,
            arrow_start: "none".into(),
            arrow_end: "none".into(),
            classes: "default relationship relationship-note".into(),
            compiled: compiled(&[]),
            minlen: 1,
            internal: true,
            ..Default::default()
        })
    }
    (nodes, edges)
}
