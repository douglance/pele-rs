//! Pele class/graph.ts, 458322a5 (MIT, kepano).
use super::model::*;
use std::collections::HashMap;
#[derive(Clone, Debug)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub is_group: bool,
    pub shape: &'static str,
    pub parent: isize,
    pub class_node: Option<usize>,
}
#[derive(Clone, Debug)]
pub struct GraphEdge {
    pub id: String,
    pub source: isize,
    pub target: isize,
    pub label: Option<String>,
    pub arrow_type_start: &'static str,
    pub arrow_type_end: &'static str,
    pub start_label_right: String,
    pub end_label_left: String,
    pub pattern: &'static str,
}
pub struct ClassGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}
fn marker(end: &RelationEnd) -> &'static str {
    match end {
        RelationEnd::Number(0) => "aggregation",
        RelationEnd::Number(1) => "extension",
        RelationEnd::Number(2) => "composition",
        RelationEnd::Number(3) => "dependency",
        RelationEnd::Number(4) => "lollipop",
        _ => "none",
    }
}
pub fn build(db: &ClassModel, hierarchical: bool) -> ClassGraph {
    let mut nodes = vec![];
    let mut edges = vec![];
    let mut ns_at = HashMap::new();
    let mut class_at = HashMap::new();
    let mut interface_at = HashMap::new();
    for (_, ns) in &db.namespaces {
        if !hierarchical && !ns.explicit {
            continue;
        }
        ns_at.insert(ns.id.clone(), nodes.len() as isize);
        nodes.push(GraphNode {
            id: ns.id.clone(),
            label: if hierarchical {
                ns.label.clone()
            } else {
                ns.id.clone()
            },
            is_group: true,
            shape: "rect",
            parent: -1,
            class_node: None,
        })
    }
    let group = |id: Option<&str>| id.and_then(|id| ns_at.get(id).copied()).unwrap_or(-1);
    for n in &mut nodes {
        n.parent = if hierarchical {
            group(db.ns(&n.id).and_then(|n| n.parent.as_deref()))
        } else {
            -1
        }
    }
    let declared = |id: Option<&str>| {
        let mut current = id.map(str::to_string);
        if !hierarchical {
            while let Some(ref id) = current {
                let Some(ns) = db.ns(id) else { break };
                if ns.explicit {
                    break;
                }
                current = ns.parent.clone()
            }
        }
        current
    };
    for (i, (_, cls)) in db.classes.iter().enumerate() {
        let parent = declared(cls.parent.as_deref());
        class_at.insert(cls.id.clone(), nodes.len() as isize);
        nodes.push(GraphNode {
            id: cls.id.clone(),
            label: cls.label.clone(),
            is_group: false,
            shape: "classBox",
            parent: group(parent.as_deref()),
            class_node: Some(i),
        })
    }
    for (_, note) in &db.notes {
        let parent = declared(note.parent.as_deref());
        if let Some(target) = note
            .class_name
            .as_deref()
            .and_then(|id| class_at.get(id))
            .copied()
        {
            edges.push(GraphEdge {
                id: format!("edgeNote{}", note.index),
                source: nodes.len() as isize,
                target,
                label: None,
                arrow_type_start: "none",
                arrow_type_end: "none",
                start_label_right: String::new(),
                end_label_left: String::new(),
                pattern: "dotted",
            })
        }
        nodes.push(GraphNode {
            id: note.id.clone(),
            label: note.text.clone(),
            is_group: false,
            shape: "note",
            parent: group(parent.as_deref()),
            class_node: None,
        })
    }
    for item in &db.interfaces {
        let parent = class_at
            .get(&item.class_id)
            .map_or(-1, |i| nodes[*i as usize].parent);
        interface_at.insert(item.id.clone(), nodes.len() as isize);
        nodes.push(GraphNode {
            id: item.id.clone(),
            label: item.label.clone(),
            is_group: false,
            shape: "interface",
            parent,
            class_node: None,
        })
    }
    for (i, r) in db.relations.iter().enumerate() {
        let a = if r.relation.type1.is(4) && r.relation.type2 == RelationEnd::none() {
            &interface_at
        } else {
            &class_at
        };
        let b = if r.relation.type2.is(4) && r.relation.type1 == RelationEnd::none() {
            &interface_at
        } else {
            &class_at
        };
        edges.push(GraphEdge {
            id: format!("id_{}_{}_{}", r.id1, r.id2, i + 1),
            source: a.get(&r.id1).copied().unwrap_or(-1),
            target: b.get(&r.id2).copied().unwrap_or(-1),
            label: r.title.clone(),
            arrow_type_start: marker(&r.relation.type1),
            arrow_type_end: marker(&r.relation.type2),
            start_label_right: if r.relation_title1 == "none" {
                String::new()
            } else {
                r.relation_title1.clone()
            },
            end_label_left: if r.relation_title2 == "none" {
                String::new()
            } else {
                r.relation_title2.clone()
            },
            pattern: if r.relation.line_type == 1 {
                "dashed"
            } else {
                "solid"
            },
        })
    }
    ClassGraph { nodes, edges }
}
