//! Pele IshikawaDb, 458322a5 (MIT, kepano). Flat arena keeps deep clone/drop bounded.
use crate::compat::{DiagramType, ErrorCode, PeleError};
use serde::{
    ser::{SerializeMap, SerializeSeq},
    Serialize, Serializer,
};
#[derive(Clone, Debug)]
pub struct IshikawaNode {
    pub text: String,
    pub children: Vec<usize>,
}
#[derive(Clone, Debug)]
pub struct IshikawaModel {
    pub diagram_type: DiagramType,
    pub title: Option<String>,
    pub root: Option<usize>,
    pub nodes: Vec<IshikawaNode>,
    stack: Vec<(f64, usize)>,
    base_level: Option<f64>,
}
impl IshikawaModel {
    pub(super) fn new(title: Option<&str>) -> Self {
        Self {
            diagram_type: DiagramType::Ishikawa,
            title: title.map(str::to_owned),
            root: None,
            nodes: vec![],
            stack: vec![],
            base_level: None,
        }
    }
    /// Append a cause using the same indentation-relative semantics as Pele's addNode.
    pub fn add_node(&mut self, raw_level: f64, text: String) {
        let node = self.nodes.len();
        self.nodes.push(IshikawaNode {
            text,
            children: vec![],
        });
        if self.root.is_none() {
            self.root = Some(node);
            self.stack = vec![(0., node)];
            return;
        }
        let base = *self.base_level.get_or_insert(raw_level);
        let delta = raw_level - base + 1.;
        let level = if delta.is_nan() { delta } else { delta.max(1.) };
        while self.stack.len() > 1 && self.stack.last().unwrap().0 >= level {
            self.stack.pop();
        }
        self.nodes[self.stack.last().unwrap().1].children.push(node);
        self.stack.push((level, node));
    }
    pub(super) fn projection(&self) -> Result<(), PeleError> {
        let Some(root) = self.root else {
            return Ok(());
        };
        let mut stack = vec![(root, 3usize)];
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
struct Node<'a>(&'a IshikawaModel, usize);
impl Serialize for Node<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        struct Children<'a>(&'a IshikawaModel, &'a [usize]);
        impl Serialize for Children<'_> {
            fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                let mut seq = s.serialize_seq(Some(self.1.len()))?;
                for &i in self.1 {
                    seq.serialize_element(&Node(self.0, i))?;
                }
                seq.end()
            }
        }
        let node = &self.0.nodes[self.1];
        let mut map = s.serialize_map(Some(2))?;
        map.serialize_entry("text", &node.text)?;
        map.serialize_entry("children", &Children(self.0, &node.children))?;
        map.end()
    }
}
impl Serialize for IshikawaModel {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut map = s.serialize_map(None)?;
        map.serialize_entry("type", &self.diagram_type)?;
        if let Some(title) = &self.title {
            map.serialize_entry("title", title)?;
        }
        if let Some(root) = self.root {
            map.serialize_entry("root", &Node(self, root))?;
        }
        map.end()
    }
}
