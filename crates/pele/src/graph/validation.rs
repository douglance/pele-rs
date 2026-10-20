use super::*;
use crate::compat::{ErrorCode, PeleError};
use std::collections::BTreeSet;
pub(super) fn invalid(message: impl Into<String>) -> PeleError {
    PeleError::new(message, ErrorCode::Semantic)
}
pub(super) fn limit(message: &str) -> PeleError {
    PeleError::new(message, ErrorCode::Limit)
}
pub(super) struct Budget {
    left: usize,
}
impl Budget {
    pub fn new(limit: usize) -> Self {
        Self { left: limit }
    }
    pub fn spend(&mut self, n: usize) -> Result<(), PeleError> {
        self.left = self
            .left
            .checked_sub(n)
            .ok_or_else(|| limit("graph work budget exhausted"))?;
        Ok(())
    }
}
fn identity<'a>(ids: impl Iterator<Item = &'a str>) -> Result<BTreeSet<&'a str>, PeleError> {
    let mut seen = BTreeSet::new();
    for id in ids {
        if id.is_empty()
            || id.len() > 1024
            || id
                .chars()
                .any(|c| c.is_control() || matches!(c, '\u{fffe}' | '\u{ffff}'))
        {
            return Err(invalid("invalid graph identity"));
        }
        if !seen.insert(id) {
            return Err(invalid(format!("duplicate graph identity {id}")));
        }
    }
    Ok(seen)
}
pub(super) fn graph(g: &Graph, o: &Options, b: &mut Budget) -> Result<(), PeleError> {
    if g.nodes.len() > o.max_nodes.min(256) || g.edges.len() > o.max_edges.min(512) {
        return Err(limit("graph node or edge limit exceeded"));
    }
    b.spend(g.nodes.len() * g.nodes.len() + g.nodes.len() * g.edges.len() + 1)?;
    let nodes = identity(g.nodes.iter().map(|n| n.id.as_str()))?;
    identity(g.edges.iter().map(|e| e.id.as_str()))?;
    for n in &g.nodes {
        b.spend(n.id.len() + n.label.len() + n.detail.len())?;
    }
    for e in &g.edges {
        b.spend(e.id.len() + e.label.len() + e.source.len() + e.target.len())?;
        if !nodes.contains(e.source.as_str()) || !nodes.contains(e.target.as_str()) {
            return Err(invalid(format!("missing graph endpoint for {}", e.id)));
        }
    }
    Ok(())
}
pub(super) fn scene(s: &Scene, o: &Options, b: &mut Budget) -> Result<(), PeleError> {
    if !s.width.is_finite() || !s.height.is_finite() || s.width <= 0. || s.height <= 0. {
        return Err(invalid("invalid scene dimensions"));
    }
    if s.nodes.len() > o.max_nodes.min(256) || s.edges.len() > o.max_edges.min(512) {
        return Err(limit("scene node or edge limit exceeded"));
    }
    let g = Graph {
        nodes: s.nodes.iter().map(|n| n.node.clone()).collect(),
        edges: s.edges.iter().map(|e| e.edge.clone()).collect(),
    };
    graph(&g, o, b)?;
    for n in &s.nodes {
        if !n.rect.valid() {
            return Err(invalid("invalid node rectangle"));
        }
    }
    for e in &s.edges {
        check_route(s, e, b)?;
    }
    Ok(())
}
fn check_route(s: &Scene, e: &Route, b: &mut Budget) -> Result<(), PeleError> {
    b.spend(e.points.len() * s.nodes.len() + 1)?;
    if e.points.len() < 2 || !e.label_box.valid() {
        return Err(invalid("invalid graph route"));
    }
    for p in e.points.windows(2) {
        if !p[0].finite()
            || !p[1].finite()
            || p[0] == p[1]
            || (p[0].x != p[1].x && p[0].y != p[1].y)
        {
            return Err(invalid(format!(
                "route must be finite and orthogonal: {} {:?} -> {:?}",
                e.edge.id, p[0], p[1]
            )));
        }
        if let Some(n) = s.nodes.iter().find(|n| n.rect.crosses(p[0], p[1])) {
            return Err(invalid(format!(
                "route {} crosses a node {}: {:?} -> {:?}, {:?}",
                e.edge.id, n.node.id, p[0], p[1], n.rect
            )));
        }
    }
    Ok(())
}
