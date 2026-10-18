//! Common Railroad IR. Flat references keep deep grammar trees stack-safe (Pele 458322a5, MIT).
use crate::compat::{ErrorCode, PeleError, Value};
use serde::{
    ser::{SerializeMap, SerializeSeq},
    Serialize, Serializer,
};
#[derive(Clone, Debug)]
pub enum RailroadNode {
    Terminal(String),
    Nonterminal(String),
    Special(String),
    Sequence(Vec<usize>),
    Choice(Vec<usize>),
    Optional(usize),
    Repetition { element: usize, min: f64, max: f64 },
}
impl RailroadNode {
    pub fn children(&self) -> &[usize] {
        match self {
            Self::Sequence(v) | Self::Choice(v) => v,
            Self::Optional(n) | Self::Repetition { element: n, .. } => std::slice::from_ref(n),
            _ => &[],
        }
    }
}
#[derive(Clone, Debug)]
pub struct RailroadRule {
    pub name: String,
    pub definition: usize,
}
#[derive(Clone, Debug)]
pub struct RailroadModel {
    pub notation: String,
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
    pub rules: Vec<RailroadRule>,
    pub nodes: Vec<RailroadNode>,
}
impl RailroadModel {
    pub(super) fn new(notation: &str, title: Option<&str>) -> Self {
        Self {
            notation: notation.into(),
            title: title.filter(|s| !s.is_empty()).map(str::to_owned),
            acc_title: None,
            acc_descr: None,
            rules: vec![],
            nodes: vec![],
        }
    }
    pub(super) fn add(&mut self, n: RailroadNode) -> usize {
        let i = self.nodes.len();
        self.nodes.push(n);
        i
    }
    pub(super) fn sequence(&mut self, v: Vec<usize>) -> usize {
        if v.len() == 1 {
            v[0]
        } else {
            self.add(RailroadNode::Sequence(v))
        }
    }
    pub(super) fn choice(&mut self, v: Vec<usize>) -> usize {
        if v.len() == 1 {
            v[0]
        } else {
            self.add(RailroadNode::Choice(v))
        }
    }
    /// Later duplicate definitions win lookup while every rule remains in source order.
    pub fn get_rule(&self, name: &str) -> Option<&RailroadRule> {
        self.rules.iter().rev().find(|r| r.name == name)
    }
    /// Check retained arena references before a host-supplied model is traversed.
    pub(super) fn validate_nodes(&self) -> Result<(), PeleError> {
        let mut colors = vec![0u8; self.nodes.len()];
        let mut todo: Vec<_> = self.rules.iter().map(|r| (r.definition, false)).collect();
        while let Some((i, done)) = todo.pop() {
            if i >= self.nodes.len() {
                return Err(PeleError::new(
                    "Railroad node reference is outside the model arena.",
                    ErrorCode::Semantic,
                ));
            }
            if done {
                colors[i] = 2;
                continue;
            }
            if colors[i] == 2 {
                continue;
            }
            if colors[i] == 1 {
                return Err(PeleError::new(
                    "Railroad model contains a cyclic node reference.",
                    ErrorCode::Semantic,
                ));
            }
            colors[i] = 1;
            todo.push((i, true));
            todo.extend(self.nodes[i].children().iter().rev().map(|&n| (n, false)));
        }
        Ok(())
    }
    pub fn validate_projection(&self) -> Result<(), PeleError> {
        self.validate_nodes()?;
        let mut stack: Vec<_> = self.rules.iter().map(|r| (r.definition, 3)).collect();
        while let Some((i, d)) = stack.pop() {
            if d >= 120 {
                return Err(PeleError::new(
                    "Model JSON projection exceeds depth limit of 120.",
                    ErrorCode::Limit,
                ));
            }
            let node = &self.nodes[i];
            let step = if matches!(node, RailroadNode::Sequence(_) | RailroadNode::Choice(_)) {
                2
            } else {
                1
            };
            stack.extend(node.children().iter().map(|&c| (c, d + step)));
        }
        Ok(())
    }
}
struct NodeRef<'a> {
    model: &'a RailroadModel,
    index: usize,
    depth: usize,
}
struct Nodes<'a> {
    model: &'a RailroadModel,
    indices: &'a [usize],
    depth: usize,
}
impl Serialize for Nodes<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut a = s.serialize_seq(Some(self.indices.len()))?;
        for &index in self.indices {
            a.serialize_element(&NodeRef {
                model: self.model,
                index,
                depth: self.depth,
            })?;
        }
        a.end()
    }
}
impl Serialize for NodeRef<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if self.depth > 120 {
            return Err(serde::ser::Error::custom("Model JSON depth exceeds 120"));
        }
        let mut m = s.serialize_map(None)?;
        match &self.model.nodes[self.index] {
            RailroadNode::Terminal(v) => {
                m.serialize_entry("type", "terminal")?;
                m.serialize_entry("value", v)?;
            }
            RailroadNode::Nonterminal(v) => {
                m.serialize_entry("type", "nonterminal")?;
                m.serialize_entry("name", v)?;
            }
            RailroadNode::Special(v) => {
                m.serialize_entry("type", "special")?;
                m.serialize_entry("text", v)?;
            }
            RailroadNode::Sequence(v) | RailroadNode::Choice(v) => {
                let seq = matches!(&self.model.nodes[self.index], RailroadNode::Sequence(_));
                m.serialize_entry("type", if seq { "sequence" } else { "choice" })?;
                m.serialize_entry(
                    if seq { "elements" } else { "alternatives" },
                    &Nodes {
                        model: self.model,
                        indices: v,
                        depth: self.depth + 2,
                    },
                )?;
            }
            RailroadNode::Optional(i) => {
                m.serialize_entry("type", "optional")?;
                m.serialize_entry(
                    "element",
                    &NodeRef {
                        model: self.model,
                        index: *i,
                        depth: self.depth + 1,
                    },
                )?;
            }
            RailroadNode::Repetition { element, min, max } => {
                m.serialize_entry("type", "repetition")?;
                m.serialize_entry(
                    "element",
                    &NodeRef {
                        model: self.model,
                        index: *element,
                        depth: self.depth + 1,
                    },
                )?;
                m.serialize_entry("min", &Value::Number(*min))?;
                m.serialize_entry("max", &Value::Number(*max))?;
            }
        }
        m.end()
    }
}
struct Rules<'a>(&'a RailroadModel);
impl Serialize for Rules<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Rule<'a> {
            name: &'a str,
            definition: NodeRef<'a>,
        }
        let mut a = s.serialize_seq(Some(self.0.rules.len()))?;
        for r in &self.0.rules {
            a.serialize_element(&Rule {
                name: &r.name,
                definition: NodeRef {
                    model: self.0,
                    index: r.definition,
                    depth: 3,
                },
            })?;
        }
        a.end()
    }
}
impl Serialize for RailroadModel {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(None)?;
        m.serialize_entry("type", "railroad")?;
        m.serialize_entry("notation", &self.notation)?;
        if let Some(s) = &self.title {
            m.serialize_entry("title", s)?;
        }
        if let Some(s) = &self.acc_title {
            m.serialize_entry("accTitle", s)?;
        }
        if let Some(s) = &self.acc_descr {
            m.serialize_entry("accDescr", s)?;
        }
        m.serialize_entry("rules", &Rules(self))?;
        m.end()
    }
}
