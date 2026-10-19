//! Pele layout/compound.ts nested groups and border ports, 458322a5 (MIT, kepano).
use super::layered::{layered, orient, rank, Kind, LEdge, LNode, LayeredOptions};
use crate::compat::number_math::NumberMath;
use std::collections::HashMap;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    TB,
    BT,
    LR,
    RL,
}
impl Dir {
    pub fn sideways(self) -> bool {
        matches!(self, Self::LR | Self::RL)
    }
}
pub fn direction(dir: Option<&str>) -> Dir {
    match dir {
        Some("BT") => Dir::BT,
        Some("LR") => Dir::LR,
        Some("RL") => Dir::RL,
        _ => Dir::TB,
    }
}
#[derive(Clone, Debug)]
pub struct CNode {
    pub w: f64,
    pub h: f64,
    pub parent: isize,
    pub is_group: bool,
    pub dir: Option<Dir>,
    pub pad_x: f64,
    pub pad_top: f64,
    pub pad_bottom: f64,
    pub min_w: f64,
    pub seq: usize,
    pub span: f64,
    pub x: f64,
    pub y: f64,
}
impl CNode {
    pub fn new(w: f64, h: f64, parent: isize) -> Self {
        Self {
            w,
            h,
            parent,
            is_group: false,
            dir: None,
            pad_x: 0.,
            pad_top: 0.,
            pad_bottom: 0.,
            min_w: 0.,
            seq: 0,
            span: 0.,
            x: 0.,
            y: 0.,
        }
    }
}
#[derive(Clone, Debug)]
pub struct CEdge {
    pub src: usize,
    pub dst: usize,
    pub minlen: f64,
    pub label_w: f64,
    pub label_h: f64,
    pub route: Vec<f64>,
    pub label_x: f64,
    pub label_y: f64,
}
#[derive(Clone, Copy, Debug)]
pub struct CompoundResult {
    pub width: f64,
    pub height: f64,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Top,
    Right,
    Bottom,
    Left,
}
fn start_side(dir: Dir) -> Side {
    match dir {
        Dir::TB => Side::Top,
        Dir::BT => Side::Bottom,
        Dir::LR => Side::Left,
        Dir::RL => Side::Right,
    }
}
fn opposite(side: Side) -> Side {
    match side {
        Side::Top => Side::Bottom,
        Side::Right => Side::Left,
        Side::Bottom => Side::Top,
        Side::Left => Side::Right,
    }
}
#[derive(Clone)]
struct PortEdge {
    e: usize,
    inner: usize,
    out: bool,
    side: Side,
    px: f64,
    py: f64,
    route: Vec<f64>,
}
#[derive(Clone)]
struct LevelEdge {
    e: usize,
    a: usize,
    b: usize,
}
#[derive(Clone)]
struct Level {
    id: isize,
    dir: Dir,
    items: Vec<usize>,
    edges: Vec<LevelEdge>,
    ports: Vec<PortEdge>,
    port_of: HashMap<usize, usize>,
    routes: HashMap<usize, Vec<f64>>,
    x: f64,
    y: f64,
}
impl Level {
    fn new(id: isize, dir: Dir, items: Vec<usize>) -> Self {
        Self {
            id,
            dir,
            items,
            edges: Vec::new(),
            ports: Vec::new(),
            port_of: HashMap::new(),
            routes: HashMap::new(),
            x: 0.,
            y: 0.,
        }
    }
}
const MAX_PORTS: usize = 40000;
pub fn shift_layout(nodes: &mut [CNode], edges: &mut [CEdge], ox: f64, oy: f64) {
    for c in nodes {
        c.x += ox;
        c.y += oy
    }
    for e in edges {
        for p in e.route.chunks_exact_mut(3) {
            p[0] += ox;
            p[1] += oy
        }
        e.label_x += ox;
        e.label_y += oy
    }
}
fn build(
    level_index: usize,
    sized: bool,
    levels: &[Level],
    level_of: &HashMap<isize, usize>,
    nodes: &[CNode],
    edges: &[CEdge],
) -> (Vec<LNode>, Vec<LEdge>, Vec<Option<usize>>) {
    let level = &levels[level_index];
    let vertical = !level.dir.sideways();
    let start = start_side(level.dir);
    let mut local = vec![0; nodes.len()];
    let mut lnodes = Vec::new();
    let mut ledges = Vec::new();
    for (i, &item) in level.items.iter().enumerate() {
        local[item] = i;
        let node = &nodes[item];
        let mut ln = if sized {
            LNode::new(
                if vertical { node.w } else { node.h },
                if vertical { node.h } else { node.w },
                Kind::Node,
            )
        } else {
            LNode::new(0., 0., Kind::Node)
        };
        ln.span = if node.is_group {
            node.span.js_min(ln.w - 2. * node.pad_x)
        } else {
            node.span
        };
        lnodes.push(ln)
    }
    let below = |item: usize, ei: usize, out: bool| {
        if out {
            edges[ei].src != item
        } else {
            edges[ei].dst != item
        }
    };
    let held = |item: usize, ei: usize, out: bool| nodes[item].is_group && below(item, ei, out);
    let offset = |item: usize, ei: usize, out: bool| {
        if !sized || !held(item, ei, out) {
            return 0.;
        }
        let child = &levels[level_of[&(item as isize)]];
        let port = &child.ports[child.port_of[&(ei * 2 + usize::from(!out))]];
        if vertical {
            port.px - nodes[item].w / 2.
        } else {
            port.py - nodes[item].h / 2.
        }
    };
    for le in &level.edges {
        let e = &edges[le.e];
        let mut edge = LEdge::new(local[le.a], local[le.b], e.minlen, 0., 0.);
        if sized {
            edge.label_w = if vertical { e.label_w } else { e.label_h };
            edge.label_h = if vertical { e.label_h } else { e.label_w };
            edge.tail_dx = offset(le.a, le.e, true);
            edge.head_dx = offset(le.b, le.e, false);
            edge.tail_fixed = held(le.a, le.e, true);
            edge.head_fixed = held(le.b, le.e, false)
        }
        ledges.push(edge)
    }
    let mut dummies = vec![None; level.ports.len()];
    for (i, port) in level.ports.iter().enumerate() {
        if port.side == start || port.side == opposite(start) {
            let d = lnodes.len();
            lnodes.push(LNode::new(
                0.,
                0.,
                if port.side == start {
                    Kind::StartPort
                } else {
                    Kind::EndPort
                },
            ));
            dummies[i] = Some(d);
            let mut edge = if port.out {
                LEdge::new(local[port.inner], d, 1., 0., 0.)
            } else {
                LEdge::new(d, local[port.inner], 1., 0., 0.)
            };
            if port.out {
                edge.tail_dx = offset(port.inner, port.e, true);
                edge.tail_fixed = held(port.inner, port.e, true)
            } else {
                edge.head_dx = offset(port.inner, port.e, false);
                edge.head_fixed = held(port.inner, port.e, false)
            }
            ledges.push(edge)
        } else {
            lnodes[local[port.inner]].pin = if matches!(port.side, Side::Top | Side::Left) {
                -1.
            } else {
                1.
            }
        }
    }
    (lnodes, ledges, dummies)
}
fn descend(
    from: usize,
    levels: &mut Vec<Level>,
    level_of: &mut HashMap<isize, usize>,
    nodes: &[CNode],
    children: &[Vec<usize>],
    placed: &mut [bool],
    depth: &mut [usize],
) {
    let mut q = from;
    while q < levels.len() {
        let id = levels[q].id;
        let dir = levels[q].dir;
        for item in levels[q].items.clone() {
            placed[item] = true;
            depth[item] = if id < 0 { 0 } else { depth[id as usize] + 1 };
            if nodes[item].is_group {
                level_of.insert(item as isize, levels.len());
                levels.push(Level::new(
                    item as isize,
                    nodes[item].dir.unwrap_or(dir),
                    children[item].clone(),
                ))
            }
        }
        q += 1
    }
}
pub fn compound_layout(
    nodes: &mut [CNode],
    edges: &mut [CEdge],
    root_dir: Dir,
    opt: &LayeredOptions,
) -> CompoundResult {
    let n = nodes.len();
    let mut level_of = HashMap::from([(-1, 0)]);
    let mut children = vec![Vec::new(); n];
    let mut root_items = Vec::new();
    for i in 0..n {
        let p = nodes[i].parent;
        if p >= 0 && (p as usize) < n && p as usize != i && nodes[p as usize].is_group {
            children[p as usize].push(i)
        } else {
            nodes[i].parent = -1;
            root_items.push(i)
        }
    }
    let mut levels = vec![Level::new(-1, root_dir, root_items)];
    let mut depth = vec![0; n];
    let mut placed = vec![false; n];
    descend(
        0,
        &mut levels,
        &mut level_of,
        nodes,
        &children,
        &mut placed,
        &mut depth,
    );
    for i in 0..n {
        if placed[i] {
            continue;
        }
        let former = nodes[i].parent;
        children[former as usize].retain(|c| *c != i);
        nodes[i].parent = -1;
        levels[0].items.push(i);
        placed[i] = true;
        depth[i] = 0;
        if nodes[i].is_group {
            let from = levels.len();
            level_of.insert(i as isize, from);
            levels.push(Level::new(
                i as isize,
                nodes[i].dir.unwrap_or(root_dir),
                children[i].clone(),
            ));
            descend(
                from,
                &mut levels,
                &mut level_of,
                nodes,
                &children,
                &mut placed,
                &mut depth,
            )
        }
    }
    for q in (1..levels.len()).rev() {
        if let Some(seq) = levels[q].items.iter().map(|i| nodes[*i].seq).min() {
            nodes[levels[q].id as usize].seq = seq
        }
    }
    for level in &mut levels {
        level.items.sort_by_key(|i| (nodes[*i].seq, *i))
    }
    let mut ports = 0;
    for e in edges.iter_mut() {
        let (mut a, mut b) = (e.src, e.dst);
        let mut crossed = 0;
        while depth[a] > depth[b] {
            a = nodes[a].parent as usize;
            crossed += 1
        }
        while depth[b] > depth[a] {
            b = nodes[b].parent as usize;
            crossed += 1
        }
        while a != b && nodes[a].parent != nodes[b].parent {
            a = nodes[a].parent as usize;
            b = nodes[b].parent as usize;
            crossed += 2
        }
        if a == b {
            continue;
        }
        if ports + crossed > MAX_PORTS {
            e.src = a;
            e.dst = b
        } else {
            ports += crossed
        }
    }
    let mut loops = Vec::new();
    let mut stray = Vec::new();
    for (ei, e) in edges.iter_mut().enumerate() {
        e.route.clear();
        if e.src == e.dst {
            loops.push(ei);
            continue;
        }
        let (mut a, mut b) = (e.src, e.dst);
        let mut a_path = Vec::new();
        let mut b_path = Vec::new();
        while depth[a] > depth[b] {
            a_path.push(a);
            a = nodes[a].parent as usize
        }
        while depth[b] > depth[a] {
            b_path.push(b);
            b = nodes[b].parent as usize
        }
        if a == b {
            stray.push(ei);
            continue;
        }
        while nodes[a].parent != nodes[b].parent {
            a_path.push(a);
            b_path.push(b);
            a = nodes[a].parent as usize;
            b = nodes[b].parent as usize
        }
        levels[level_of[&nodes[a].parent]]
            .edges
            .push(LevelEdge { e: ei, a, b });
        for (path, out) in [(a_path, true), (b_path, false)] {
            for inner in path {
                let level = &mut levels[level_of[&nodes[inner].parent]];
                let index = level.ports.len();
                level.ports.push(PortEdge {
                    e: ei,
                    inner,
                    out,
                    side: Side::Top,
                    px: 0.,
                    py: 0.,
                    route: Vec::new(),
                });
                level.port_of.insert(ei * 2 + usize::from(!out), index);
            }
        }
    }
    for q in 0..levels.len() {
        for port in levels[q].ports.clone() {
            if nodes[port.inner].is_group
                && (if port.out {
                    edges[port.e].src != port.inner
                } else {
                    edges[port.e].dst != port.inner
                })
            {
                let child = &mut levels[level_of[&(port.inner as isize)]];
                let i = child.port_of[&(port.e * 2 + usize::from(!port.out))];
                child.ports[i].side = port.side
            }
        }
        if levels[q].edges.is_empty() {
            continue;
        }
        let needed = levels[q].edges.iter().any(|le| {
            (nodes[le.a].is_group && edges[le.e].src != le.a)
                || (nodes[le.b].is_group && edges[le.e].dst != le.b)
        });
        if !needed {
            continue;
        }
        let (mut ln, mut le, _) = build(q, false, &levels, &level_of, nodes, edges);
        orient(&ln, &mut le);
        rank(&mut ln, &le, 1);
        let start = start_side(levels[q].dir);
        for (i, edge) in levels[q].edges.clone().iter().enumerate() {
            let above = !le[i].reversed;
            if nodes[edge.a].is_group && edges[edge.e].src != edge.a {
                let child = &mut levels[level_of[&(edge.a as isize)]];
                let p = child.port_of[&(edge.e * 2)];
                child.ports[p].side = if above { opposite(start) } else { start }
            }
            if nodes[edge.b].is_group && edges[edge.e].dst != edge.b {
                let child = &mut levels[level_of[&(edge.b as isize)]];
                let p = child.port_of[&(edge.e * 2 + 1)];
                child.ports[p].side = if above { start } else { opposite(start) }
            }
        }
    }
    let mut total = CompoundResult {
        width: 0.,
        height: 0.,
    };
    for q in (0..levels.len()).rev() {
        let group = usize::try_from(levels[q].id).ok();
        let vertical = !levels[q].dir.sideways();
        let flip = matches!(levels[q].dir, Dir::BT | Dir::RL);
        let (mut ln, mut le, dummies) = build(q, true, &levels, &level_of, nodes, edges);
        let size = layered(&mut ln, &mut le, opt);
        let content_w = if vertical { size.width } else { size.height };
        let content_h = if vertical { size.height } else { size.width };
        let pad_x = group.map_or(0., |i| nodes[i].pad_x);
        let pad_top = group.map_or(0., |i| nodes[i].pad_top);
        let box_w = (content_w + 2. * pad_x).js_max(group.map_or(0., |i| nodes[i].min_w));
        let box_h = content_h + pad_top + group.map_or(0., |i| nodes[i].pad_bottom);
        let off_x = (box_w - content_w) / 2.;
        let off_y = pad_top;
        let tx = |x: f64, y: f64| {
            off_x
                + if vertical {
                    x
                } else if flip {
                    size.height - y
                } else {
                    y
                }
        };
        let ty = |x: f64, y: f64| {
            off_y
                + if vertical {
                    if flip {
                        size.height - y
                    } else {
                        y
                    }
                } else {
                    x
                }
        };
        let axis = if vertical { 0. } else { 1. };
        let convert = |points: &[f64]| {
            let mut out = Vec::new();
            for p in points.chunks_exact(2) {
                out.extend_from_slice(&[tx(p[0], p[1]), ty(p[0], p[1]), axis])
            }
            out
        };
        for (i, &item) in levels[q].items.iter().enumerate() {
            nodes[item].x = tx(ln[i].x, ln[i].y);
            nodes[item].y = ty(ln[i].x, ln[i].y)
        }
        for (i, edge) in levels[q].edges.clone().iter().enumerate() {
            levels[q].routes.insert(edge.e, convert(&le[i].points));
            let e = &mut edges[edge.e];
            if e.label_w > 0. {
                e.label_x = tx(le[i].label_x, le[i].label_y);
                e.label_y = ty(le[i].label_x, le[i].label_y)
            }
        }
        let mut k = levels[q].edges.len();
        for (i, port) in levels[q].ports.clone().iter().enumerate() {
            let item = &nodes[port.inner];
            let mut p = port.clone();
            if dummies[i].is_some() {
                let mut route = convert(&le[k].points);
                k += 1;
                let at = if port.out { route.len() - 3 } else { 0 };
                let mut bx = route[at];
                let mut by = route[at + 1];
                match port.side {
                    Side::Top => by = 0.,
                    Side::Bottom => by = box_h,
                    Side::Left => bx = 0.,
                    Side::Right => bx = box_w,
                }
                if port.out {
                    route.extend_from_slice(&[bx, by, axis])
                } else {
                    route.splice(0..0, [bx, by, axis]);
                }
                p.px = bx;
                p.py = by;
                p.route = route
            } else {
                let mut ix = item.x;
                let mut iy = item.y;
                if item.is_group
                    && (if port.out {
                        edges[port.e].src != port.inner
                    } else {
                        edges[port.e].dst != port.inner
                    })
                {
                    let child = &levels[level_of[&(port.inner as isize)]];
                    let inner = &child.ports[child.port_of[&(port.e * 2 + usize::from(!port.out))]];
                    ix += inner.px - item.w / 2.;
                    iy += inner.py - item.h / 2.
                } else {
                    match port.side {
                        Side::Top => iy -= item.h / 2.,
                        Side::Bottom => iy += item.h / 2.,
                        Side::Left => ix -= item.w / 2.,
                        Side::Right => ix += item.w / 2.,
                    }
                }
                let mut bx = ix;
                let mut by = iy;
                match port.side {
                    Side::Top => by = 0.,
                    Side::Bottom => by = box_h,
                    Side::Left => bx = 0.,
                    Side::Right => bx = box_w,
                }
                let axis = if matches!(port.side, Side::Top | Side::Bottom) {
                    0.
                } else {
                    1.
                };
                p.px = bx;
                p.py = by;
                p.route = if port.out {
                    vec![ix, iy, axis, bx, by, axis]
                } else {
                    vec![bx, by, axis, ix, iy, axis]
                }
            }
            levels[q].ports[i] = p
        }
        if let Some(i) = group {
            nodes[i].w = box_w;
            nodes[i].h = box_h
        } else {
            total = CompoundResult {
                width: box_w,
                height: box_h,
            }
        }
    }
    for q in 0..levels.len() {
        let ox = levels[q].x;
        let oy = levels[q].y;
        for item in levels[q].items.clone() {
            let node = &mut nodes[item];
            node.x += ox;
            node.y += oy;
            if node.is_group {
                let child = &mut levels[level_of[&(item as isize)]];
                child.x = node.x - node.w / 2.;
                child.y = node.y - node.h / 2.
            }
        }
    }
    let append = |route: &mut Vec<f64>, part: &[f64], ox: f64, oy: f64| {
        for p in part.chunks_exact(3) {
            let x = p[0] + ox;
            let y = p[1] + oy;
            let len = route.len();
            if len >= 3 && (route[len - 3] - x).abs() < 0.01 && (route[len - 2] - y).abs() < 0.01 {
                continue;
            }
            route.extend_from_slice(&[x, y, p[2]])
        }
    };
    for level in &levels {
        for le in &level.edges {
            let e = &mut edges[le.e];
            let mut route = Vec::new();
            let mut item = e.src;
            while nodes[item].parent != level.id {
                let lv = &levels[level_of[&nodes[item].parent]];
                let p = &lv.ports[lv.port_of[&(le.e * 2)]];
                append(&mut route, &p.route, lv.x, lv.y);
                item = nodes[item].parent as usize
            }
            append(&mut route, &level.routes[&le.e], level.x, level.y);
            let mut inward = Vec::new();
            item = e.dst;
            while nodes[item].parent != level.id {
                inward.push(level_of[&nodes[item].parent]);
                item = nodes[item].parent as usize
            }
            for &lv in inward.iter().rev() {
                let lv = &levels[lv];
                let p = &lv.ports[lv.port_of[&(le.e * 2 + 1)]];
                append(&mut route, &p.route, lv.x, lv.y)
            }
            e.route = route;
            if e.label_w > 0. {
                e.label_x += level.x;
                e.label_y += level.y
            }
        }
    }
    for ei in stray {
        let e = &mut edges[ei];
        let a = &nodes[e.src];
        let b = &nodes[e.dst];
        e.route = vec![a.x, a.y, 0., b.x, b.y, 0.];
        e.label_x = (a.x + b.x) / 2.;
        e.label_y = (a.y + b.y) / 2.
    }
    for ei in loops {
        let e = &mut edges[ei];
        let node = &nodes[e.src];
        e.label_x = node.x + node.w / 2. + 28. + e.label_w / 2.;
        e.label_y = node.y
    }
    total
}
