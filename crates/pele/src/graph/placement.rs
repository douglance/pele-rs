use super::*;
use crate::compat::{
    layout::layered::{layered, Kind, LEdge, LNode, LayeredOptions},
    text::metrics_width,
};
use std::collections::BTreeMap;
pub(super) fn display(text: &str, max: usize) -> String {
    let mut s = text
        .chars()
        .take(max)
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect::<String>();
    if text.chars().count() > max {
        s.push_str("...");
    }
    s
}
pub(super) fn place(g: &Graph) -> Scene {
    let positions = g
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.as_str(), i))
        .collect::<BTreeMap<_, _>>();
    let (mut nodes, mut edges) = prepare(g, &positions);
    layered(
        &mut nodes,
        &mut edges,
        &LayeredOptions {
            node_sep: 44.,
            edge_sep: 16.,
            rank_sep: 76.,
            port_sep: 14.,
            tracks: true,
            straight: false,
            apart: true,
            head_room: 0.,
        },
    );
    Scene {
        width: 1.,
        height: 1.,
        nodes: g
            .nodes
            .iter()
            .zip(nodes)
            .map(|(n, p)| PlacedNode {
                node: n.clone(),
                rect: Rect {
                    x: p.x - p.w / 2.,
                    y: p.y - p.h / 2.,
                    width: p.w,
                    height: p.h,
                },
            })
            .collect(),
        edges: Vec::new(),
    }
}
pub(super) fn normalize(s: &mut Scene) {
    let mut min = Point { x: 0., y: 0. };
    let mut max = Point { x: 0., y: 0. };
    for n in &s.nodes {
        bounds(&mut min, &mut max, n.rect);
    }
    for e in &s.edges {
        bounds(&mut min, &mut max, e.label_box);
        for p in &e.points {
            bounds(
                &mut min,
                &mut max,
                Rect {
                    x: p.x,
                    y: p.y,
                    width: 0.,
                    height: 0.,
                },
            );
        }
    }
    let (dx, dy) = (24. - min.x, 24. - min.y);
    for n in &mut s.nodes {
        n.rect.x += dx;
        n.rect.y += dy;
    }
    for e in &mut s.edges {
        e.label_box.x += dx;
        e.label_box.y += dy;
        for p in &mut e.points {
            p.x += dx;
            p.y += dy;
        }
        e.points = super::geometry::simplify(std::mem::take(&mut e.points));
    }
    s.width = (max.x - min.x + 48.).max(48.);
    s.height = (max.y - min.y + 48.).max(48.);
}
fn bounds(min: &mut Point, max: &mut Point, r: Rect) {
    min.x = min.x.min(r.x);
    min.y = min.y.min(r.y);
    max.x = max.x.max(r.x + r.width);
    max.y = max.y.max(r.y + r.height);
}

fn prepare(g: &Graph, positions: &BTreeMap<&str, usize>) -> (Vec<LNode>, Vec<LEdge>) {
    let nodes = g
        .nodes
        .iter()
        .map(|n| {
            let label = display(&n.label, 36);
            let detail = display(&n.detail, 28);
            LNode::new(
                metrics_width(&label, 14., 0)
                    .max(metrics_width(&detail, 11., 0))
                    .max(80.)
                    + 32.,
                64.,
                Kind::Node,
            )
        })
        .collect::<Vec<_>>();
    let edges = g
        .edges
        .iter()
        .filter(|e| e.source != e.target)
        .map(|e| {
            LEdge::new(
                positions[e.source.as_str()],
                positions[e.target.as_str()],
                1.,
                metrics_width(&display(&e.label, 24), 11., 0) + 16.,
                20.,
            )
        })
        .collect::<Vec<_>>();

    (nodes, edges)
}
