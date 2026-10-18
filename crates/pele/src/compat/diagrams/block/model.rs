//! Flat object arena preserves BlockDb references without recursive clone/drop.
use crate::compat::{DiagramType, ErrorCode, PeleError, Value};
use serde::{
    ser::{SerializeMap, SerializeSeq},
    Serialize, Serializer,
};
use std::collections::HashMap;
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockNode {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip)]
    pub children: Option<Vec<usize>>,
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "number")]
    pub columns: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "number")]
    pub width_in_columns: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "number")]
    pub width: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub directions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub styles: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arrow_type_start: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arrow_type_end: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thickness: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub css: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style_class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub styles_str: Option<String>,
    #[serde(skip)]
    pub extra: Vec<(String, usize)>,
}
fn number<S: Serializer>(n: &Option<f64>, s: S) -> Result<S::Ok, S::Error> {
    match n {
        Some(n) => Value::Number(*n).serialize(s),
        None => s.serialize_none(),
    }
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockClassDef {
    pub id: String,
    pub styles: Vec<String>,
    pub text_styles: Vec<String>,
}
#[derive(Clone, Debug)]
pub struct BlockModel {
    pub diagram_type: DiagramType,
    pub title: Option<String>,
    pub nodes: Vec<BlockNode>,
    pub root: usize,
    pub blocks: Vec<usize>,
    pub edges: Vec<usize>,
    pub classes: Vec<(String, BlockClassDef)>,
    pub by_id: Vec<(String, usize)>,
    pub(super) lookup: HashMap<String, usize>,
    pub(super) positions: HashMap<String, usize>,
}
struct Node<'a>(&'a BlockModel, usize);
struct Nodes<'a>(&'a BlockModel, &'a [usize]);
impl Serialize for Node<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let n = &self.0.nodes[self.1];
        let fields = serde_json::to_value(n).map_err(serde::ser::Error::custom)?;
        let mut map = s.serialize_map(None)?;
        for (k, v) in fields.as_object().unwrap() {
            map.serialize_entry(k, v)?;
        }
        if let Some(children) = &n.children {
            map.serialize_entry("children", &Nodes(self.0, children))?;
        }
        for (k, v) in &n.extra {
            map.serialize_entry(k, &Node(self.0, *v))?;
        }
        map.end()
    }
}
impl Serialize for Nodes<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut seq = s.serialize_seq(Some(self.1.len()))?;
        for &i in self.1 {
            seq.serialize_element(&Node(self.0, i))?;
        }
        seq.end()
    }
}
impl Serialize for BlockModel {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        struct ById<'a>(&'a BlockModel);
        impl Serialize for ById<'_> {
            fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                let mut seq = s.serialize_seq(Some(self.0.by_id.len()))?;
                for (id, node) in &self.0.by_id {
                    seq.serialize_element(&(id, Node(self.0, *node)))?;
                }
                seq.end()
            }
        }
        let mut map = s.serialize_map(None)?;
        map.serialize_entry("type", &self.diagram_type)?;
        if let Some(title) = &self.title {
            map.serialize_entry("title", title)?;
        }
        map.serialize_entry("root", &Node(self, self.root))?;
        map.serialize_entry("blocks", &Nodes(self, &self.blocks))?;
        map.serialize_entry("edges", &Nodes(self, &self.edges))?;
        map.serialize_entry("classes", &self.classes)?;
        map.serialize_entry("byId", &ById(self))?;
        map.end()
    }
}
impl BlockModel {
    pub(super) fn push(&mut self, n: BlockNode) -> usize {
        let i = self.nodes.len();
        self.nodes.push(n);
        i
    }
    pub(super) fn projection(&self) -> Result<(), PeleError> {
        let mut stack = vec![(self.root, 3usize)];
        for &(_, i) in &self.by_id {
            stack.push((i, 4));
        }
        while let Some((i, depth)) = stack.pop() {
            if depth > 125 {
                return Err(PeleError::new(
                    "Parsed model exceeds JSON projection depth",
                    ErrorCode::Limit,
                ));
            }
            let n = &self.nodes[i];
            if let Some(children) = &n.children {
                for &child in children {
                    stack.push((child, depth + 2));
                }
            }
            for &(_, child) in &n.extra {
                stack.push((child, depth + 1));
            }
        }
        Ok(())
    }
}
