//! Venn DB and declared projection, Pele 458322a5 (MIT).
use crate::compat::{source::space, ErrorCode, PeleError, Value};
use serde::{Serialize, Serializer};
use std::collections::HashSet;
fn number<S: Serializer>(n: &f64, s: S) -> Result<S::Ok, S::Error> {
    Value::Number(*n).serialize(s)
}
pub(super) fn normalize(s: &str) -> String {
    let s = s.trim_matches(space);
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        s[1..s.len() - 1].into()
    } else {
        s.into()
    }
}
fn ids(v: Vec<String>) -> Vec<String> {
    let mut v: Vec<_> = v.into_iter().map(|s| normalize(&s)).collect();
    v.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
    v
}
pub(super) fn semantic(msg: impl Into<String>) -> PeleError {
    let mut e = PeleError::new(msg, ErrorCode::Semantic);
    e.diagram_type = Some("venn".into());
    e
}
#[derive(Clone, Debug, Serialize)]
pub struct VennSubset {
    pub sets: Vec<String>,
    #[serde(serialize_with = "number")]
    pub size: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct VennText {
    pub sets: Vec<String>,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct VennStyle {
    pub targets: Vec<String>,
    pub styles: Vec<(String, String)>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VennModel {
    #[serde(rename = "type")]
    pub diagram_type: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub subsets: Vec<VennSubset>,
    pub text_nodes: Vec<VennText>,
    pub styles: Vec<VennStyle>,
    #[serde(skip)]
    known: HashSet<String>,
    #[serde(skip)]
    pub(super) current: Option<Vec<String>>,
    #[serde(skip)]
    pub(super) indent: bool,
}
impl VennModel {
    pub(super) fn new(title: Option<&str>) -> Self {
        Self {
            diagram_type: "venn",
            title: title.filter(|s| !s.is_empty()).map(str::to_owned),
            subsets: vec![],
            text_nodes: vec![],
            styles: vec![],
            known: HashSet::new(),
            current: None,
            indent: false,
        }
    }
    pub(super) fn subset(&mut self, list: Vec<String>, label: Option<String>, size: Option<f64>) {
        let n = list.len();
        let sets = ids(list);
        self.current = Some(sets.clone());
        if sets.len() == 1 {
            self.known.insert(sets[0].clone());
        }
        self.subsets.push(VennSubset {
            sets,
            size: size.unwrap_or(10. / (n as f64).powi(2)),
            label: label.filter(|s| !s.is_empty()).map(|s| normalize(&s)),
        });
    }
    pub(super) fn text(&mut self, list: Vec<String>, id: String, label: Option<String>) {
        self.text_nodes.push(VennText {
            sets: ids(list),
            id: normalize(&id),
            label: label.filter(|s| !s.is_empty()).map(|s| normalize(&s)),
        });
    }
    pub(super) fn style(&mut self, list: Vec<String>, data: Vec<(String, String)>) {
        let mut styles: Vec<(String, String)> = vec![];
        for (k, v) in data {
            let v = if !v.is_empty() { normalize(&v) } else { v };
            if let Some((_, old)) = styles.iter_mut().find(|(key, _)| *key == k) {
                *old = v
            } else {
                styles.push((k, v))
            }
        }
        self.styles.push(VennStyle {
            targets: ids(list),
            styles,
        });
    }
    pub(super) fn validate_union(&self, list: &[String]) -> Result<(), PeleError> {
        let unknown: Vec<_> = list
            .iter()
            .map(|s| normalize(s))
            .filter(|s| !self.known.contains(s))
            .collect();
        if !unknown.is_empty() {
            return Err(semantic(format!(
                "unknown set identifier: {}",
                unknown.join(", ")
            )));
        }
        Ok(())
    }
}
