//! Pele treeview/model.ts, 458322a5 (MIT, kepano). Flat arena keeps clone/drop stack-safe.
use crate::compat::{DiagramType, ErrorCode, PeleError};
use serde::{
    ser::{SerializeMap, SerializeSeq},
    Serialize, Serializer,
};
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NodeType {
    File,
    Directory,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeNode {
    pub id: usize,
    pub level: i64,
    pub name: String,
    pub node_type: NodeType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub css_class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip)]
    pub children: Vec<usize>,
}
#[derive(Clone, Debug)]
pub struct TreeViewModel {
    pub diagram_type: DiagramType,
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
    pub root: usize,
    pub count: usize,
    pub nodes: Vec<TreeNode>,
}
struct Node<'a>(&'a TreeViewModel, usize);
impl Serialize for Node<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let n = &self.0.nodes[self.1];
        let fields = serde_json::to_value(n).map_err(serde::ser::Error::custom)?;
        let mut map = s.serialize_map(None)?;
        for (k, v) in fields.as_object().unwrap() {
            map.serialize_entry(k, v)?;
        }
        struct Children<'a>(&'a TreeViewModel, &'a [usize]);
        impl Serialize for Children<'_> {
            fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                let mut seq = s.serialize_seq(Some(self.1.len()))?;
                for &i in self.1 {
                    seq.serialize_element(&Node(self.0, i))?;
                }
                seq.end()
            }
        }
        map.serialize_entry("children", &Children(self.0, &n.children))?;
        map.end()
    }
}
impl Serialize for TreeViewModel {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut map = s.serialize_map(None)?;
        map.serialize_entry("type", &self.diagram_type)?;
        if let Some(t) = &self.title {
            map.serialize_entry("title", t)?;
        }
        if let Some(t) = &self.acc_title {
            map.serialize_entry("accTitle", t)?;
        }
        if let Some(t) = &self.acc_descr {
            map.serialize_entry("accDescr", t)?;
        }
        map.serialize_entry("root", &Node(self, self.root))?;
        map.serialize_entry("count", &self.count)?;
        map.end()
    }
}
impl TreeViewModel {
    pub(super) fn projection(&self) -> Result<(), PeleError> {
        let mut stack = vec![(self.root, 3usize)];
        while let Some((i, depth)) = stack.pop() {
            if depth > 125 {
                return Err(PeleError::new(
                    "Parsed model exceeds JSON projection depth",
                    ErrorCode::Limit,
                ));
            }
            for &child in &self.nodes[i].children {
                stack.push((child, depth + 2));
            }
        }
        Ok(())
    }
}
