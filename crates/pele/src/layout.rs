use crate::{
    model::q, text::Text, Direction, Error, Flowchart, Geometry, Point, PositionedEdge,
    PositionedNode, Rect, RenderOptions, Shape,
};
use std::collections::VecDeque;

/// Invariant-preserving scene; only the engine constructs layouts.
#[derive(Clone, Debug)]
pub struct Layout {
    pub(crate) geometry: Geometry,
    pub(crate) options: RenderOptions,
    pub(crate) node_text: Vec<Text>,
    pub(crate) edge_text: Vec<Text>,
    width: f64,
    height: f64,
}
impl Layout {
    pub fn geometry(&self) -> &Geometry {
        &self.geometry
    }
    pub fn width(&self) -> f64 {
        self.width
    }
    pub fn height(&self) -> f64 {
        self.height
    }
}

#[derive(Clone)]
struct Vertex {
    node: Option<usize>,
    w: f64,
    h: f64,
    x: f64,
    y: f64,
    rank: usize,
}
struct Route {
    edge: usize,
    chain: Vec<usize>,
    feedback: bool,
    tracks: Vec<(usize, usize)>,
    end_track: Option<(usize, usize)>,
}

pub fn layout(graph: &Flowchart, options: &RenderOptions) -> Result<Layout, Error> {
    options.validate()?;
    let horizontal = matches!(graph.direction, Direction::LR | Direction::RL);
    let mut glyphs = 0;
    let mut node_text = vec![];
    let mut edge_text = vec![];
    for n in &graph.nodes {
        node_text.push(
            Text::shape(&n.label, options.font_size, &mut glyphs).map_err(|mut e| {
                e.span = Some(n.span.clone());
                e
            })?,
        );
    }
    for e in &graph.edges {
        edge_text.push(
            Text::shape(&e.label, options.edge_font_size, &mut glyphs).map_err(|mut err| {
                err.span = Some(e.span.clone());
                err
            })?,
        );
    }
    let mut boxes = vec![
        Rect {
            x: 0.,
            y: 0.,
            width: 0.,
            height: 0.
        };
        graph.nodes.len()
    ];
    let mut paths = vec![vec![]; graph.edges.len()];
    let mut labels = vec![None; graph.edges.len()];
    let mut seen = vec![false; graph.nodes.len()];
    let mut component_x = 0.;
    let mut virtuals = 0usize;
    for root in 0..graph.nodes.len() {
        if seen[root] {
            continue;
        }
        let mut members = vec![];
        let mut queue = VecDeque::from([root]);
        seen[root] = true;
        while let Some(i) = queue.pop_front() {
            members.push(i);
            for e in &graph.edges {
                let next = if e.source == i {
                    Some(e.target)
                } else if e.target == i {
                    Some(e.source)
                } else {
                    None
                };
                if let Some(j) = next {
                    if !seen[j] {
                        seen[j] = true;
                        queue.push_back(j);
                    }
                }
            }
        }
        members.sort_unstable();
        let edges: Vec<usize> = graph
            .edges
            .iter()
            .enumerate()
            .filter(|(_, e)| members.binary_search(&e.source).is_ok())
            .map(|(i, _)| i)
            .collect();
        let mut local = vec![usize::MAX; graph.nodes.len()];
        let mut vertices = vec![];
        for &i in &members {
            let t = &node_text[i];
            let mut w = (t.width + options.node_padding * 2.).max(80.);
            let mut h = (t.height + options.node_padding * 2.).max(48.);
            match graph.nodes[i].shape {
                Shape::Diamond => {
                    w *= 2.;
                    h *= 2.;
                }
                Shape::Circle => {
                    let d = (w * w + h * h).sqrt();
                    w = d;
                    h = d;
                }
                Shape::Stadium => {
                    w += h / 2.;
                }
                _ => {}
            }
            if horizontal {
                std::mem::swap(&mut w, &mut h);
            }
            let outgoing = graph.edges.iter().filter(|e| e.source == i).count();
            let incoming = graph.edges.iter().filter(|e| e.target == i).count();
            // Reserve visible arrowhead separation rather than squeezing high fan-in
            // onto coincident ports. Port distribution uses 65% of this dimension.
            let port_width = (outgoing.max(incoming) + 1) as f64 * 12. / 0.65;
            w = w.max(port_width);
            if graph.nodes[i].shape == Shape::Circle {
                h = w;
            }
            local[i] = vertices.len();
            vertices.push(Vertex {
                node: Some(i),
                w,
                h,
                x: 0.,
                y: 0.,
                rank: 0,
            });
        }
        // Iterative DFS identifies feedback edges without dropping original edges.
        let mut adjacency = vec![vec![]; vertices.len()];
        for &ei in &edges {
            adjacency[local[graph.edges[ei].source]].push(ei);
        }
        let mut color = vec![0u8; vertices.len()];
        let mut back = vec![false; graph.edges.len()];
        for first in 0..vertices.len() {
            if color[first] != 0 {
                continue;
            }
            color[first] = 1;
            let mut stack = vec![(first, 0)];
            while let Some((v, k)) = stack.last_mut() {
                if *k == adjacency[*v].len() {
                    color[*v] = 2;
                    stack.pop();
                    continue;
                }
                let ei = adjacency[*v][*k];
                *k += 1;
                let target = local[graph.edges[ei].target];
                if color[target] == 1 {
                    back[ei] = true;
                } else if color[target] == 0 {
                    color[target] = 1;
                    stack.push((target, 0));
                }
            }
        }
        let mut indegree = vec![0; vertices.len()];
        for &ei in &edges {
            if !back[ei] {
                indegree[local[graph.edges[ei].target]] += 1;
            }
        }
        let mut queue: VecDeque<usize> = indegree
            .iter()
            .enumerate()
            .filter(|(_, d)| **d == 0)
            .map(|(i, _)| i)
            .collect();
        while let Some(v) = queue.pop_front() {
            for &ei in &adjacency[v] {
                if !back[ei] {
                    let to = local[graph.edges[ei].target];
                    vertices[to].rank = vertices[to].rank.max(vertices[v].rank + 1);
                    indegree[to] -= 1;
                    if indegree[to] == 0 {
                        queue.push_back(to);
                    }
                }
            }
        }
        let rank_count = vertices.iter().map(|v| v.rank).max().unwrap_or(0) + 1;
        // Gap 0 is above the first rank; gap rank+1 follows that rank.
        let mut gaps = vec![vec![]; rank_count + 1];
        let mut routes = vec![];
        for &ei in &edges {
            let e = &graph.edges[ei];
            let a = local[e.source];
            let b = local[e.target];
            let mut chain = vec![a];
            if !back[ei] {
                let span = vertices[b].rank - vertices[a].rank;
                virtuals += span.saturating_sub(1);
                if virtuals > 8192 {
                    return Err(Error::new(
                        "layout_limit",
                        "More than 8192 virtual route vertices",
                    ));
                }
                for rank in vertices[a].rank + 1..vertices[b].rank {
                    let middle = rank == vertices[a].rank + span / 2 && !e.label.is_empty();
                    let (lw, lh) = label_size(&edge_text[ei], horizontal);
                    let v = vertices.len();
                    vertices.push(Vertex {
                        node: None,
                        w: if middle { lw + 16. } else { 16. },
                        h: if middle { lh + 16. } else { 16. },
                        x: 0.,
                        y: 0.,
                        rank,
                    });
                    chain.push(v);
                }
            }
            chain.push(b);
            let mut tracks = vec![];
            let mut end_track = None;
            if back[ei] {
                let gap = vertices[a].rank + 1;
                let slot = gaps[gap].len();
                gaps[gap].push(ei);
                tracks.push((gap, slot));
                let gap = vertices[b].rank;
                let slot = gaps[gap].len();
                gaps[gap].push(ei);
                end_track = Some((gap, slot));
            } else {
                for pair in chain.windows(2) {
                    let gap = vertices[pair[0]].rank + 1;
                    let slot = gaps[gap].len();
                    gaps[gap].push(ei);
                    tracks.push((gap, slot));
                }
            }
            routes.push(Route {
                edge: ei,
                chain,
                feedback: back[ei],
                tracks,
                end_track,
            });
        }
        let mut ranks = vec![vec![]; rank_count];
        for (i, v) in vertices.iter().enumerate() {
            ranks[v.rank].push(i);
        }
        let mut neighbors = vec![vec![]; vertices.len()];
        for r in &routes {
            if !r.feedback {
                for pair in r.chain.windows(2) {
                    neighbors[pair[0]].push(pair[1]);
                    neighbors[pair[1]].push(pair[0]);
                }
            }
        }
        // Eight stable barycentric sweeps, retain the best crossing count.
        let mut best = ranks.clone();
        let mut best_score = crossings(&ranks, &routes, vertices.len());
        for pass in 0..8 {
            let order: Vec<usize> = if pass % 2 == 0 {
                (1..rank_count).collect()
            } else {
                (0..rank_count.saturating_sub(1)).rev().collect()
            };
            for ri in order {
                let mut positions = vec![0.; vertices.len()];
                for rank in &ranks {
                    for (pos, &v) in rank.iter().enumerate() {
                        positions[v] = pos as f64;
                    }
                }
                ranks[ri].sort_by(|a, b| {
                    let mean = |v: usize| {
                        let ns: Vec<usize> = neighbors[v]
                            .iter()
                            .copied()
                            .filter(|n| {
                                if pass % 2 == 0 {
                                    vertices[*n].rank < ri
                                } else {
                                    vertices[*n].rank > ri
                                }
                            })
                            .collect();
                        if ns.is_empty() {
                            positions[v]
                        } else {
                            ns.iter().map(|n| positions[*n]).sum::<f64>() / ns.len() as f64
                        }
                    };
                    mean(*a).total_cmp(&mean(*b)).then(a.cmp(b))
                });
            }
            let score = crossings(&ranks, &routes, vertices.len());
            if score < best_score {
                best_score = score;
                best = ranks.clone();
            }
        }
        ranks = best;
        let widths: Vec<f64> = ranks
            .iter()
            .map(|r| {
                r.iter().map(|v| vertices[*v].w).sum::<f64>()
                    + options.node_gap * r.len().saturating_sub(1) as f64
            })
            .collect();
        let body_width = widths.iter().copied().fold(0., f64::max);
        for (ri, rank) in ranks.iter().enumerate() {
            let mut x = (body_width - widths[ri]) / 2.;
            for &v in rank {
                vertices[v].x = x + vertices[v].w / 2.;
                x += vertices[v].w + options.node_gap;
            }
        }
        // Bounded neighbor alignment, followed by separation-preserving compaction.
        // A rank remains inside body_width even when every neighbor pulls one way.
        for pass in 0..4 {
            let order: Vec<usize> = if pass % 2 == 0 {
                (1..rank_count).collect()
            } else {
                (0..rank_count.saturating_sub(1)).rev().collect()
            };
            for ri in order {
                let rank = &ranks[ri];
                let desired: Vec<f64> = rank
                    .iter()
                    .map(|&v| {
                        let adjacent: Vec<_> = neighbors[v]
                            .iter()
                            .copied()
                            .filter(|&n| {
                                if pass % 2 == 0 {
                                    vertices[n].rank < ri
                                } else {
                                    vertices[n].rank > ri
                                }
                            })
                            .collect();
                        if adjacent.is_empty() {
                            vertices[v].x
                        } else {
                            adjacent.iter().map(|&n| vertices[n].x).sum::<f64>()
                                / adjacent.len() as f64
                        }
                    })
                    .collect();
                let mut left = 0.;
                for (&v, ideal) in rank.iter().zip(desired) {
                    vertices[v].x = ideal.max(left + vertices[v].w / 2.);
                    left = vertices[v].x + vertices[v].w / 2. + options.node_gap;
                }
                let mut right = body_width;
                for &v in rank.iter().rev() {
                    vertices[v].x = vertices[v].x.min(right - vertices[v].w / 2.);
                    right = vertices[v].x - vertices[v].w / 2. - options.node_gap;
                }
            }
        }
        let pitch: Vec<f64> = gaps
            .iter()
            .map(|g| {
                g.iter()
                    .map(|ei| label_size(&edge_text[*ei], horizontal).1 + 16.)
                    .fold(28., f64::max)
            })
            .collect();
        let gap_height: Vec<f64> = gaps
            .iter()
            .enumerate()
            .map(|(i, g)| options.rank_gap + g.len() as f64 * pitch[i])
            .collect();
        let mut gap_top = vec![0.; gaps.len()];
        let mut y = gap_height[0];
        for (ri, rank) in ranks.iter().enumerate() {
            let height = rank.iter().map(|v| vertices[*v].h).fold(0., f64::max);
            for &v in rank {
                vertices[v].y = y + height / 2.;
            }
            y += height;
            gap_top[ri + 1] = y;
            y += gap_height[ri + 1];
        }
        let track_y = |(gap, slot): (usize, usize)| {
            gap_top[gap] + options.rank_gap / 2. + (slot as f64 + 0.5) * pitch[gap]
        };
        for v in &vertices {
            if let Some(i) = v.node {
                boxes[i] = Rect {
                    x: v.x - v.w / 2. + component_x,
                    y: v.y - v.h / 2.,
                    width: v.w,
                    height: v.h,
                };
            }
        }
        let mut out_ports = vec![vec![]; graph.nodes.len()];
        let mut in_ports = out_ports.clone();
        for &ei in &edges {
            out_ports[graph.edges[ei].source].push(ei);
            in_ports[graph.edges[ei].target].push(ei);
        }
        // Ports follow neighbor position; edge index separates parallel links.
        for ports in &mut out_ports {
            ports.sort_by(|a, b| {
                vertices[local[graph.edges[*a].target]]
                    .x
                    .total_cmp(&vertices[local[graph.edges[*b].target]].x)
                    .then(a.cmp(b))
            });
        }
        for ports in &mut in_ports {
            ports.sort_by(|a, b| {
                vertices[local[graph.edges[*a].source]]
                    .x
                    .total_cmp(&vertices[local[graph.edges[*b].source]].x)
                    .then(a.cmp(b))
            });
        }
        let port = |node: usize, ei: usize, bottom: bool| {
            let v = &vertices[local[node]];
            let list = if bottom {
                &out_ports[node]
            } else {
                &in_ports[node]
            };
            let index = list.iter().position(|n| *n == ei).unwrap_or(0);
            let dx = ((index + 1) as f64 / (list.len() + 1) as f64 - 0.5) * v.w * 0.65;
            let radius = match graph.nodes[node].shape {
                Shape::Rounded => options.radius,
                Shape::Stadium => v.w.min(v.h) / 2.,
                _ => 0.,
            };
            let half = match graph.nodes[node].shape {
                Shape::Diamond => v.h / 2. * (1. - dx.abs() / (v.w / 2.)),
                Shape::Circle => v.h / 2. * (1. - (dx / (v.w / 2.)).powi(2)).sqrt(),
                _ => {
                    let r = radius.min(v.w / 2.).min(v.h / 2.);
                    let d = (dx.abs() - (v.w / 2. - r)).max(0.);
                    v.h / 2. - r + (r * r - d * d).max(0.).sqrt()
                }
            };
            Point {
                x: v.x + dx,
                y: v.y + if bottom { half } else { -half },
            }
        };
        let mut lane = body_width + options.node_gap;
        for route in routes {
            let ei = route.edge;
            let e = &graph.edges[ei];
            let start = port(e.source, ei, true);
            let end = port(e.target, ei, false);
            let mut points = vec![start];
            if route.feedback {
                let (lw, lh) = label_size(&edge_text[ei], horizontal);
                lane += lw / 2. + 16.;
                let sy = track_y(route.tracks[0]);
                let ty = track_y(route.end_track.unwrap());
                points.extend([
                    Point { x: start.x, y: sy },
                    Point { x: lane, y: sy },
                    Point { x: lane, y: ty },
                    Point { x: end.x, y: ty },
                    end,
                ]);
                if !e.label.is_empty() {
                    labels[ei] = Some(Rect {
                        x: lane - lw / 2. + component_x,
                        y: (sy + ty) / 2. - lh / 2.,
                        width: lw,
                        height: lh,
                    });
                }
                lane += lw / 2. + options.node_gap;
            } else {
                for (segment, pair) in route.chain.windows(2).enumerate() {
                    let from = &vertices[pair[0]];
                    let to = &vertices[pair[1]];
                    let fx = if segment == 0 { start.x } else { from.x };
                    let tx = if segment == route.chain.len() - 2 {
                        end.x
                    } else {
                        to.x
                    };
                    let ty = track_y(route.tracks[segment]);
                    points.extend([Point { x: fx, y: ty }, Point { x: tx, y: ty }]);
                    if to.node.is_none() {
                        points.push(Point { x: tx, y: to.y });
                    }
                    if route.chain.len() == 2 && !e.label.is_empty() {
                        let (lw, lh) = label_size(&edge_text[ei], horizontal);
                        labels[ei] = Some(Rect {
                            x: (fx + tx) / 2. - lw / 2. + component_x,
                            y: ty - lh / 2.,
                            width: lw,
                            height: lh,
                        });
                    }
                }
                points.push(end);
                if route.chain.len() > 2 && !e.label.is_empty() {
                    let v = &vertices[route.chain[(route.chain.len() - 1) / 2]];
                    let (lw, lh) = label_size(&edge_text[ei], horizontal);
                    labels[ei] = Some(Rect {
                        x: v.x - lw / 2. + component_x,
                        y: v.y - lh / 2.,
                        width: lw,
                        height: lh,
                    });
                }
            }
            for p in &mut points {
                p.x += component_x;
            }
            points.dedup();
            paths[ei] = points;
        }
        // Include all routes and labels before packing the next disconnected component.
        let mut left = component_x;
        for &ei in &edges {
            for p in &paths[ei] {
                left = left.min(p.x);
            }
            if let Some(r) = labels[ei] {
                left = left.min(r.x);
            }
        }
        let shift = (component_x - left).max(0.);
        if shift > 0. {
            for &i in &members {
                boxes[i].x += shift;
            }
            for &ei in &edges {
                for p in &mut paths[ei] {
                    p.x += shift;
                }
                if let Some(r) = &mut labels[ei] {
                    r.x += shift;
                }
            }
        }
        let mut right = component_x + body_width + shift;
        for &ei in &edges {
            for p in &paths[ei] {
                right = right.max(p.x);
            }
            if let Some(r) = labels[ei] {
                right = right.max(r.x + r.width);
            }
        }
        component_x = right + options.node_gap * 2.;
    }
    // Rotate geometry only; font outlines are laid out upright in final label rectangles.
    for r in &mut boxes {
        *r = transform_rect(*r, graph.direction);
    }
    for path in &mut paths {
        for p in path {
            *p = transform(*p, graph.direction);
        }
    }
    for r in labels.iter_mut().flatten() {
        *r = transform_rect(*r, graph.direction);
    }
    let mut min = Point {
        x: f64::INFINITY,
        y: f64::INFINITY,
    };
    let mut max = Point {
        x: f64::NEG_INFINITY,
        y: f64::NEG_INFINITY,
    };
    let mut add = |p: Point| {
        min.x = min.x.min(p.x);
        min.y = min.y.min(p.y);
        max.x = max.x.max(p.x);
        max.y = max.y.max(p.y);
    };
    for r in boxes.iter().chain(labels.iter().flatten()) {
        add(Point { x: r.x, y: r.y });
        add(Point {
            x: r.x + r.width,
            y: r.y + r.height,
        });
    }
    for path in &paths {
        for &p in path {
            add(p);
        }
    }
    let margin = options.padding + options.stroke_width + 8.;
    let width = q(max.x - min.x + margin * 2.);
    let height = q(max.y - min.y + margin * 2.);
    if !width.is_finite() || !height.is_finite() || width > 100000. || height > 100000. {
        return Err(Error::new("layout_limit", "Canvas exceeds 100000 units"));
    }
    let shift = |p: Point| Point {
        x: q(p.x - min.x + margin),
        y: q(p.y - min.y + margin),
    };
    let shift_rect = |r: &mut Rect| {
        let p = shift(Point { x: r.x, y: r.y });
        r.x = p.x;
        r.y = p.y;
        r.width = q(r.width);
        r.height = q(r.height);
    };
    for r in &mut boxes {
        shift_rect(r);
    }
    for path in &mut paths {
        for p in path {
            *p = shift(*p);
        }
    }
    for r in labels.iter_mut().flatten() {
        shift_rect(r);
    }
    validate_routes(graph, &boxes, &paths, &labels)?;
    let nodes = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| PositionedNode {
            id: n.id.clone(),
            label: n.label.clone(),
            shape: n.shape,
            bounds: boxes[i],
        })
        .collect();
    let edges = graph
        .edges
        .iter()
        .enumerate()
        .map(|(i, e)| PositionedEdge {
            index: i,
            source: graph.nodes[e.source].id.clone(),
            target: graph.nodes[e.target].id.clone(),
            label: e.label.clone(),
            style: e.style,
            arrow: e.arrow,
            points: paths[i].clone(),
            label_bounds: labels[i],
        })
        .collect();
    Ok(Layout {
        geometry: Geometry {
            version: 1,
            direction: graph.direction,
            nodes,
            edges,
        },
        options: options.clone(),
        node_text,
        edge_text,
        width,
        height,
    })
}

fn label_size(t: &Text, horizontal: bool) -> (f64, f64) {
    if horizontal {
        (t.height + 8., t.width + 12.)
    } else {
        (t.width + 12., t.height + 8.)
    }
}
fn transform(p: Point, d: Direction) -> Point {
    match d {
        Direction::TB => p,
        Direction::BT => Point { x: p.x, y: -p.y },
        Direction::LR => Point { x: p.y, y: p.x },
        Direction::RL => Point { x: -p.y, y: p.x },
    }
}
fn transform_rect(r: Rect, d: Direction) -> Rect {
    let a = transform(Point { x: r.x, y: r.y }, d);
    let b = transform(
        Point {
            x: r.x + r.width,
            y: r.y + r.height,
        },
        d,
    );
    Rect {
        x: a.x.min(b.x),
        y: a.y.min(b.y),
        width: (a.x - b.x).abs(),
        height: (a.y - b.y).abs(),
    }
}
fn crossings(ranks: &[Vec<usize>], routes: &[Route], n: usize) -> usize {
    let mut position = vec![0; n];
    for rank in ranks {
        for (i, &v) in rank.iter().enumerate() {
            position[v] = i;
        }
    }
    let mut segments = vec![vec![]; ranks.len()];
    let mut rank_of = vec![0; n];
    for (i, rank) in ranks.iter().enumerate() {
        for &v in rank {
            rank_of[v] = i;
        }
    }
    for r in routes {
        if !r.feedback {
            for p in r.chain.windows(2) {
                segments[rank_of[p[0]]].push((position[p[0]], position[p[1]]));
            }
        }
    }
    segments
        .iter()
        .map(|s| {
            let mut count = 0;
            for (i, a) in s.iter().enumerate() {
                for b in &s[i + 1..] {
                    if (a.0 < b.0 && a.1 > b.1) || (a.0 > b.0 && a.1 < b.1) {
                        count += 1;
                    }
                }
            }
            count
        })
        .sum()
}

fn validate_routes(
    graph: &Flowchart,
    boxes: &[Rect],
    paths: &[Vec<Point>],
    labels: &[Option<Rect>],
) -> Result<(), Error> {
    for (i, a) in boxes.iter().enumerate() {
        for b in &boxes[i + 1..] {
            if a.overlaps(b) {
                return Err(Error::new("layout_limit", "Unable to separate node bounds"));
            }
        }
    }
    for (i, label) in labels.iter().enumerate() {
        if let Some(a) = label {
            if boxes.iter().any(|b| a.overlaps(b))
                || labels[i + 1..].iter().flatten().any(|b| a.overlaps(b))
            {
                return Err(Error::new(
                    "layout_limit",
                    "Unable to reserve edge label space",
                ));
            }
        }
    }
    for (i, path) in paths.iter().enumerate() {
        for (j, r) in boxes.iter().enumerate() {
            if j == graph.edges[i].source || j == graph.edges[i].target {
                continue;
            }
            if path.windows(2).any(|p| segment_hits(p[0], p[1], *r)) {
                return Err(Error::new(
                    "layout_limit",
                    "Unable to route without crossing a node",
                ));
            }
        }
    }
    Ok(())
}
fn segment_hits(a: Point, b: Point, r: Rect) -> bool {
    let epsilon = 0.01;
    if (a.x - b.x).abs() < epsilon {
        a.x > r.x + epsilon
            && a.x < r.x + r.width - epsilon
            && a.y.min(b.y) < r.y + r.height - epsilon
            && a.y.max(b.y) > r.y + epsilon
    } else if (a.y - b.y).abs() < epsilon {
        a.y > r.y + epsilon
            && a.y < r.y + r.height - epsilon
            && a.x.min(b.x) < r.x + r.width - epsilon
            && a.x.max(b.x) > r.x + epsilon
    } else {
        true
    }
}
