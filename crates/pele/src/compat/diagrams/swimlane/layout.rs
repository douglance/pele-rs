//! Exact Pele layout/lanes.ts, 458322a5 (MIT, kepano). Shared ranks, lane
//! sweeps, dummy ranks, margin handoffs and iterative route/spacing relaxation.
use crate::compat::layout::{
    compound::{CEdge, CNode, CompoundResult, Dir},
    layered::{orient, rank, Kind, LEdge, LNode, LayeredOptions},
};
use crate::compat::number_math::{round, NumberMath};
use std::collections::HashSet;
const BUDGET: usize = 2_000_000;
struct Plan<'a> {
    lanes: Vec<usize>,
    lane_slot: Vec<isize>,
    lane_pads: Vec<f64>,
    end_pad: f64,
    item_of: Vec<isize>,
    real: Vec<usize>,
    lnodes: Vec<LNode>,
    ledges: Vec<LEdge>,
    edge_of: Vec<usize>,
    item_lane: Vec<usize>,
    step: usize,
    sideways: bool,
    flipped: bool,
    opt: &'a LayeredOptions,
}
struct Placed {
    width: f64,
    height: f64,
    routed: Vec<bool>,
    margins: Vec<bool>,
    work: usize,
}
pub(super) fn lane_layout(
    nodes: &mut [CNode],
    edges: &mut [CEdge],
    dir: Dir,
    opt: &LayeredOptions,
    lane_pad: f64,
) -> CompoundResult {
    let n = nodes.len();
    if n == 0 {
        return CompoundResult {
            width: 0.,
            height: 0.,
        };
    }
    let sideways = dir.sideways();
    let lanes: Vec<_> = (0..n)
        .rev()
        .filter(|&i| nodes[i].parent < 0 && nodes[i].is_group)
        .collect();
    let lane_count = lanes.len().max(1);
    let mut lane_slot = vec![-1; n];
    for (k, &id) in lanes.iter().enumerate() {
        lane_slot[id] = k as isize
    }
    let mut has_child = vec![false; n];
    for node in nodes.iter() {
        if node.parent >= 0 {
            has_child[node.parent as usize] = true
        }
    }
    let mut lane_pads = vec![lane_pad; lane_count];
    let mut end_pad = lane_pad;
    let mut item_of = vec![-1; n];
    let mut real = vec![];
    for i in 0..n {
        if lane_slot[i] >= 0 {
            continue;
        }
        if !nodes[i].is_group || !has_child[i] {
            item_of[i] = real.len() as isize;
            real.push(i)
        }
    }
    let mut lnodes: Vec<_> = real
        .iter()
        .map(|&i| {
            LNode::new(
                if sideways { nodes[i].h } else { nodes[i].w },
                if sideways { nodes[i].w } else { nodes[i].h },
                Kind::Node,
            )
        })
        .collect();
    let mut ledges = vec![];
    let mut edge_of = vec![];
    let mut step = 1;
    let mut seen = HashSet::new();
    for (ei, e) in edges.iter().enumerate() {
        let a = item_of[e.src];
        let b = item_of[e.dst];
        if a < 0 || b < 0 || a == b {
            continue;
        }
        let (a, b) = (a as usize, b as usize);
        let key = (a.min(b), a.max(b));
        if e.label_w > 0. || seen.contains(&key) {
            step = 2
        }
        seen.insert(key);
        ledges.push(LEdge::new(
            a,
            b,
            e.minlen,
            if sideways { e.label_h } else { e.label_w },
            if sideways { e.label_w } else { e.label_h },
        ));
        edge_of.push(ei)
    }
    orient(&lnodes, &mut ledges);
    rank(&mut lnodes, &ledges, step);
    let mut item_lane = vec![];
    for &i in &real {
        let mut t = i;
        let mut room = lane_pad;
        for _ in 0..10000 {
            if nodes[t].parent < 0 {
                break;
            }
            t = nodes[t].parent as usize;
            if lane_slot[t] < 0 {
                room += nodes[t]
                    .pad_x
                    .js_max(nodes[t].pad_top)
                    .js_max(nodes[t].pad_bottom)
            }
        }
        let lane = lane_slot[t].max(0) as usize;
        lane_pads[lane] = lane_pads[lane].max(room);
        end_pad = end_pad.max(room);
        item_lane.push(lane)
    }
    let plan = Plan {
        lanes,
        lane_slot,
        lane_pads,
        end_pad,
        item_of,
        real,
        lnodes,
        ledges,
        edge_of,
        item_lane,
        step,
        sideways,
        flipped: matches!(dir, Dir::BT | Dir::RL),
        opt,
    };
    let m = plan.ledges.len();
    let mut plain = vec![false; m];
    let mut margins = vec![false; lane_count];
    let mut placed = plan.place(nodes, edges, &plain, &margins);
    for round in 0..3 {
        if placed.work > BUDGET / 8 {
            break;
        }
        let next = if round == 0 {
            placed.routed.clone()
        } else {
            plain
                .iter()
                .zip(&placed.routed)
                .map(|(a, b)| *a && *b)
                .collect()
        };
        if next == plain && placed.margins == margins {
            break;
        }
        plain = next;
        margins = placed.margins;
        placed = plan.place(nodes, edges, &plain, &margins)
    }
    if plain.iter().zip(&placed.routed).any(|(a, b)| *a && !*b) {
        placed = plan.place(nodes, edges, &vec![false; m], &vec![false; lane_count])
    }
    CompoundResult {
        width: placed.width,
        height: placed.height,
    }
}
fn runs(row: &[usize], lane: &[usize]) -> Vec<(usize, usize)> {
    let mut out = vec![];
    let mut from = 0;
    while from < row.len() {
        let mut to = from;
        while to + 1 < row.len() && lane[row[to + 1]] == lane[row[from]] {
            to += 1
        }
        out.push((from, to));
        from = to + 1
    }
    out
}
fn gap(a: usize, b: usize, dummy: &[bool], opt: &LayeredOptions) -> f64 {
    if dummy[a] || dummy[b] {
        opt.edge_sep
    } else {
        opt.node_sep
    }
}
fn run_size(
    row: &[usize],
    from: usize,
    to: usize,
    size: &[f64],
    dummy: &[bool],
    opt: &LayeredOptions,
) -> f64 {
    let mut total = size[row[from]];
    for k in from..to {
        total += gap(row[k], row[k + 1], dummy, opt) + size[row[k + 1]]
    }
    total
}
fn settle(row: &mut [usize], lane: &[usize], key: &[f64], pos: &mut [f64]) {
    row.sort_by(|&a, &b| {
        lane[a]
            .cmp(&lane[b])
            .then_with(|| {
                key[a]
                    .partial_cmp(&key[b])
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then(a.cmp(&b))
    });
    for (k, &i) in row.iter().enumerate() {
        pos[i] = k as f64
    }
}
impl Plan<'_> {
    fn place(
        &self,
        nodes: &mut [CNode],
        edges: &mut [CEdge],
        plain: &[bool],
        margins: &[bool],
    ) -> Placed {
        let n = nodes.len();
        let m = self.ledges.len();
        let count = self.lane_pads.len();
        let pad: Vec<_> = self
            .lane_pads
            .iter()
            .enumerate()
            .map(|(i, &p)| if margins[i] { p.js_max(20.) } else { p })
            .collect();
        let mut used = vec![false; count];
        let span: Vec<_> = self
            .ledges
            .iter()
            .map(|e| self.lnodes[e.head].rank.abs_diff(self.lnodes[e.tail].rank))
            .collect();
        let dummies: usize = span
            .iter()
            .zip(plain)
            .map(|(s, p)| if *p { 0 } else { s.saturating_sub(1) })
            .sum();
        let direct = dummies > 20000;
        let total = self.real.len() + if direct { 0 } else { dummies };
        let mut ranks = vec![0; total];
        let mut lane = vec![0; total];
        let mut size = vec![0.; total];
        let mut depth = vec![0.; total];
        let mut dummy = vec![false; total];
        let mut max_rank = 0;
        for k in 0..self.real.len() {
            ranks[k] = self.lnodes[k].rank;
            lane[k] = self.item_lane[k];
            size[k] = self.lnodes[k].w;
            depth[k] = self.lnodes[k].h;
            max_rank = max_rank.max(ranks[k])
        }
        let mut first_dummy = vec![-1isize; m];
        let mut label_item = vec![-1isize; m];
        let mut links = vec![];
        let mut next = self.real.len();
        for (k, e) in self.ledges.iter().enumerate() {
            let (a, b) = if e.reversed {
                (e.head, e.tail)
            } else {
                (e.tail, e.head)
            };
            if direct || span[k] < 2 || plain[k] {
                links.push((a, b));
                continue;
            }
            let (ra, rb) = (ranks[a], ranks[b]);
            let mut label_rank = -1;
            if e.label_w > 0. {
                label_rank = (ra + ((rb - ra) >> 1)) as isize;
                if self.step == 2 && label_rank & 1 == 0 {
                    label_rank -= 1
                }
            }
            first_dummy[k] = next as isize;
            let mut prev = a;
            for r in ra + 1..rb {
                let d = next;
                next += 1;
                ranks[d] = r;
                lane[d] = (lane[a] as f64
                    + round((lane[b] as f64 - lane[a] as f64) * (r - ra) as f64 / (rb - ra) as f64))
                    as usize;
                dummy[d] = true;
                if r as isize == label_rank {
                    size[d] = e.label_w;
                    depth[d] = e.label_h;
                    label_item[k] = d as isize
                }
                links.push((prev, d));
                prev = d
            }
            links.push((prev, b))
        }
        let mut up_start = vec![0; total + 1];
        let mut down_start = vec![0; total + 1];
        for &(a, b) in &links {
            down_start[a + 1] += 1;
            up_start[b + 1] += 1
        }
        for i in 0..total {
            up_start[i + 1] += up_start[i];
            down_start[i + 1] += down_start[i]
        }
        let mut up = vec![0; links.len()];
        let mut down = vec![0; links.len()];
        let mut uf = up_start[..total].to_vec();
        let mut df = down_start[..total].to_vec();
        for &(a, b) in &links {
            down[df[a]] = b;
            df[a] += 1;
            up[uf[b]] = a;
            uf[b] += 1
        }
        let mut rows = vec![vec![]; max_rank + 1];
        for i in 0..total {
            rows[ranks[i]].push(i)
        }
        let mut pos = vec![0.; total];
        let mut key = vec![0.; total];
        for row in &mut rows {
            for (k, &i) in row.iter().enumerate() {
                key[i] = k as f64
            }
            settle(row, &lane, &key, &mut pos)
        }
        for sweep in 0..8 {
            if max_rank == 0 {
                break;
            }
            let downward = sweep & 1 == 0;
            for s in 1..=max_rank {
                let row = &mut rows[if downward { s } else { max_rank - s }];
                let (start, list) = if downward {
                    (&up_start, &up)
                } else {
                    (&down_start, &down)
                };
                for &i in row.iter() {
                    let mut sum = 0.;
                    let count = start[i + 1] - start[i];
                    for k in start[i]..start[i + 1] {
                        sum += pos[list[k]]
                    }
                    key[i] = if count > 0 {
                        sum / count as f64
                    } else {
                        pos[i]
                    }
                }
                settle(row, &lane, &key, &mut pos)
            }
        }
        let mut breadth = vec![0_f64; count];
        let mut head: f64 = if self.lanes.is_empty() { 0. } else { 34. };
        for (k, &i) in self.lanes.iter().enumerate() {
            breadth[k] = (nodes[i].min_w - 2. * pad[k]).js_max(0.);
            head = head.max(nodes[i].pad_top)
        }
        for row in &rows {
            for (from, to) in runs(row, &lane) {
                let size = run_size(row, from, to, &size, &dummy, self.opt);
                let l = lane[row[from]];
                breadth[l] = breadth[l].max(size)
            }
        }
        let mut lane_start = vec![0.; count + 1];
        for k in 0..count {
            lane_start[k + 1] = lane_start[k] + breadth[k] + 2. * pad[k]
        }
        let mut cross = vec![0.; total];
        let mut want = vec![0.; total];
        for row in &rows {
            for (from, to) in runs(row, &lane) {
                let l = lane[row[from]];
                let mut at = (lane_start[l] + pad[l] + (lane_start[l + 1] - pad[l])
                    - run_size(row, from, to, &size, &dummy, self.opt))
                    / 2.;
                for k in from..=to {
                    cross[row[k]] = at + size[row[k]] / 2.;
                    if k < to {
                        at += size[row[k]] + gap(row[k], row[k + 1], &dummy, self.opt)
                    }
                }
            }
        }
        for pass in 0..4 {
            if max_rank == 0 {
                break;
            }
            let downward = pass & 1 == 0;
            for s in 1..=max_rank {
                let row = &rows[if downward { s } else { max_rank - s }];
                let (start, list) = if downward {
                    (&up_start, &up)
                } else {
                    (&down_start, &down)
                };
                for &i in row {
                    let (mut sum, mut count) = (0., 0);
                    for k in start[i]..start[i + 1] {
                        if lane[list[k]] != lane[i] {
                            continue;
                        }
                        sum += cross[list[k]];
                        count += 1
                    }
                    want[i] = if count > 0 {
                        sum / count as f64
                    } else {
                        cross[i]
                    }
                }
                for (from, to) in runs(row, &lane) {
                    let l = lane[row[from]];
                    let mut floor = lane_start[l] + pad[l];
                    for k in from..=to {
                        let i = row[k];
                        cross[i] = want[i].js_max(floor + size[i] / 2.);
                        if k < to {
                            floor = cross[i] + size[i] / 2. + gap(i, row[k + 1], &dummy, self.opt)
                        }
                    }
                    let mut ceil = lane_start[l + 1] - pad[l];
                    for k in (from..=to).rev() {
                        let i = row[k];
                        cross[i] = cross[i].js_min(ceil - size[i] / 2.);
                        if k > from {
                            ceil = cross[i] - size[i] / 2. - gap(row[k - 1], i, &dummy, self.opt)
                        }
                    }
                }
            }
        }
        let mut band_start = vec![0.; max_rank + 1];
        let mut band_end = vec![0.; max_rank + 1];
        let full = self.opt.rank_sep / self.step as f64;
        let mut sep = vec![full; max_rank + 1];
        if plain.iter().any(|p| *p) {
            sep.fill(full / 3.);
            for (k, e) in self.ledges.iter().enumerate() {
                let (a, b) = if e.reversed {
                    (e.head, e.tail)
                } else {
                    (e.tail, e.head)
                };
                if plain[k] {
                    if e.label_h <= 0. || span[k] > self.step {
                        continue;
                    }
                    let room = full.js_min(
                        (e.label_h + 40. - depth[a].js_min(depth[b]) / 2.) / span[k] as f64,
                    );
                    for r in ranks[a]..ranks[b] {
                        sep[r] = sep[r].max(room)
                    }
                    continue;
                }
                let d0 = first_dummy[k];
                if d0 < 0 || span[k] as f64 <= self.step as f64 * e.minlen {
                    for r in ranks[a]..ranks[b] {
                        sep[r] = full
                    }
                    continue;
                }
                for d in d0 - 1..d0 + span[k] as isize - 1 {
                    let u = if d < d0 { a } else { d as usize };
                    let v = if d + 1 < d0 + span[k] as isize - 1 {
                        (d + 1) as usize
                    } else {
                        b
                    };
                    if lane[u] != lane[v] || (cross[u] - cross[v]).abs() >= 0.5 {
                        sep[ranks[u]] = full
                    }
                }
            }
        }
        let mut at = head + self.end_pad;
        for r in 0..=max_rank {
            let mut d: f64 = 0.;
            for &i in &rows[r] {
                d = d.max(depth[i])
            }
            band_start[r] = at;
            band_end[r] = at + d;
            at += d + if r == max_rank { 0. } else { sep[r] }
        }
        let length = at + self.end_pad;
        let breadth_total = lane_start[count];
        let coords = Coordinates {
            sideways: self.sideways,
            flipped: self.flipped,
            head,
            length,
        };
        let axis = if self.sideways { 1. } else { 0. };
        for (k, &i) in self.real.iter().enumerate() {
            let a = (band_start[ranks[k]] + band_end[ranks[k]]) / 2.;
            (nodes[i].x, nodes[i].y) = coords.point(cross[k], a)
        }
        for (k, &i) in self.lanes.iter().enumerate() {
            let c = (lane_start[k] + lane_start[k + 1]) / 2.;
            let size = lane_start[k + 1] - lane_start[k];
            let lane = &mut nodes[i];
            lane.x = if self.sideways { length / 2. } else { c };
            lane.y = if self.sideways { c } else { length / 2. };
            lane.w = if self.sideways { length } else { size };
            lane.h = if self.sideways { size } else { length };
            lane.pad_top = head;
        }
        let mut depth_of = vec![0; n];
        let mut inner = vec![];
        for i in 0..n {
            if !nodes[i].is_group || self.lane_slot[i] >= 0 || self.item_of[i] >= 0 {
                continue;
            }
            let mut d = 0;
            let mut p = nodes[i].parent;
            while p >= 0 && d < 10000 {
                d += 1;
                p = nodes[p as usize].parent
            }
            depth_of[i] = d;
            inner.push(i)
        }
        inner.sort_by(|&a, &b| depth_of[b].cmp(&depth_of[a]).then(a.cmp(&b)));
        let mut boxes = vec![
            [
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY
            ];
            n
        ];
        for &i in &self.real {
            let p = nodes[i].parent;
            if p >= 0 && self.lane_slot[p as usize] < 0 {
                grow(&mut boxes[p as usize], &nodes[i])
            }
        }
        for g in inner {
            let c = &mut nodes[g];
            let b = boxes[g];
            if b[0] == f64::INFINITY {
                continue;
            }
            c.x = (b[0] + b[2]) / 2.;
            c.w = (b[2] - b[0] + 2. * c.pad_x).js_max(c.min_w);
            c.y = (b[1] - c.pad_top + b[3] + c.pad_bottom) / 2.;
            c.h = b[3] - b[1] + c.pad_top + c.pad_bottom;
            if c.parent >= 0 && self.lane_slot[c.parent as usize] < 0 {
                grow(&mut boxes[c.parent as usize], c)
            }
        }
        for e in edges.iter_mut() {
            e.route.clear()
        }
        for (k, e) in self.ledges.iter().enumerate() {
            let out = &mut edges[self.edge_of[k]];
            let (a, b) = if e.reversed {
                (e.head, e.tail)
            } else {
                (e.tail, e.head)
            };
            let mut pts = vec![(
                cross[a],
                band_end[ranks[a]] - (band_end[ranks[a]] - band_start[ranks[a]] - depth[a]) / 2.,
            )];
            let d0 = first_dummy[k];
            if d0 >= 0 {
                for d in d0 as usize..d0 as usize + span[k] - 1 {
                    pts.push((cross[d], band_start[ranks[d]]));
                    if band_end[ranks[d]] > band_start[ranks[d]] {
                        pts.push((cross[d], band_end[ranks[d]]))
                    }
                }
            }
            pts.push((
                cross[b],
                band_start[ranks[b]] + (band_end[ranks[b]] - band_start[ranks[b]] - depth[b]) / 2.,
            ));
            for (c, a) in pts {
                let (x, y) = coords.point(c, a);
                out.route.extend([x, y, axis]);
            }
            if e.reversed {
                let count = out.route.len() / 3;
                for i in 0..count / 2 {
                    let j = count - 1 - i;
                    out.route.swap(i * 3, j * 3);
                    out.route.swap(i * 3 + 1, j * 3 + 1)
                }
            }
            let label = label_item[k];
            if label >= 0 {
                let label = label as usize;
                let mid = (band_start[ranks[label]] + band_end[ranks[label]]) / 2.;
                (out.label_x, out.label_y) = coords.point(cross[label], mid)
            } else {
                let last = out.route.len() - 3;
                out.label_x = (out.route[0] + out.route[last]) / 2.;
                out.label_y = (out.route[1] + out.route[last + 1]) / 2.;
            }
        }
        for e in edges.iter_mut() {
            if !e.route.is_empty() || e.src == e.dst {
                continue;
            }
            let (ax, ay) = border(&nodes[e.src], &nodes[e.dst]);
            let (bx, by) = border(&nodes[e.dst], &nodes[e.src]);
            let flat = if (bx - ax).abs() > (by - ay).abs() {
                1.
            } else {
                0.
            };
            e.route = vec![ax, ay, flat, bx, by, flat];
            e.label_x = (ax + bx) / 2.;
            e.label_y = (ay + by) / 2.;
        }
        let route_points: usize = edges.iter().map(|e| e.route.len() / 3).sum();
        let work = m.saturating_mul(self.real.len() + edges.len() + route_points);
        let mut routed = vec![false; m];
        if work <= BUDGET {
            let labels = edges
                .iter()
                .map(|e| {
                    if self.sideways {
                        (e.label_y, coords.flip(e.label_x))
                    } else {
                        (e.label_x, coords.flip(e.label_y))
                    }
                })
                .collect();
            let mut router = Router {
                edges,
                real: self.real.len(),
                size: &size,
                depth: &depth,
                cross: &cross,
                ranks: &ranks,
                band_start: &band_start,
                band_end: &band_end,
                coords: &coords,
                labels,
                breadth: breadth_total,
                length,
                sideways: self.sideways,
                axis,
            };
            for (k, e) in self.ledges.iter().enumerate() {
                let ei = self.edge_of[k];
                let (a, b) = (e.tail, e.head);
                if lane[a] == lane[b] || (cross[b] - cross[a]).abs() < 1. {
                    continue;
                }
                let (ac, bc) = (cross[a], cross[b]);
                let (aa, ba) = (router.amid(a), router.amid(b));
                let dc = (bc - ac).signum();
                let da = (ba - aa).signum();
                let first = vec![
                    ac + dc * size[a] / 2.,
                    aa,
                    bc,
                    aa,
                    bc,
                    ba - da * depth[b] / 2.,
                ];
                if router.attempt(&first, 2., a, b, ei, e) {
                    routed[k] = true;
                    continue;
                }
                let second = vec![
                    ac,
                    aa + da * depth[a] / 2.,
                    ac,
                    ba,
                    bc - dc * size[b] / 2.,
                    ba,
                ];
                if router.attempt(&second, 2., a, b, ei, e) {
                    routed[k] = true;
                    continue;
                }
                if work > BUDGET / 8 {
                    continue;
                }
                let mut around = vec![];
                let lanes = if lane[a] < lane[b] {
                    [lane[b], lane[a]]
                } else {
                    [lane[a], lane[b]]
                };
                for l in lanes {
                    let inset = (pad[l] - 14.).js_max((pad[l] / 2.).js_min(6.));
                    for ch in [lane_start[l] + inset, lane_start[l + 1] - inset] {
                        if (ch - ac).abs() < size[a] / 2. + 4.
                            || (ch - bc).abs() < size[b] / 2. + 4.
                        {
                            continue;
                        }
                        let pts = vec![
                            ac + (ch - ac).signum() * size[a] / 2.,
                            aa,
                            ch,
                            aa,
                            ch,
                            ba,
                            bc + (ch - bc).signum() * size[b] / 2.,
                            ba,
                        ];
                        let (x0, y0) = coords.point(pts[0], pts[1]);
                        let (x1, y1) = coords.point(pts[6], pts[7]);
                        let mut cost =
                            (router.meets(x0, y0, ei) + router.meets(x1, y1, ei)) as f64 * 1e6;
                        for p in (2..pts.len()).step_by(2) {
                            let (x0, y0) = coords.point(pts[p - 2], pts[p - 1]);
                            let (x1, y1) = coords.point(pts[p], pts[p + 1]);
                            cost += router.crossings(
                                x0.js_min(x1),
                                y0.js_min(y1),
                                x0.js_max(x1),
                                y0.js_max(y1),
                                ei,
                            ) as f64
                                * 1e6
                                + (x1 - x0).abs()
                                + (y1 - y0).abs()
                        }
                        around.push((pts, cost, l))
                    }
                }
                around.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
                for (pts, _, l) in around {
                    if router.attempt(&pts, 1., a, b, ei, e) {
                        routed[k] = true;
                        used[l] = true;
                        break;
                    }
                }
            }
        }
        Placed {
            width: if self.sideways { length } else { breadth_total },
            height: if self.sideways { breadth_total } else { length },
            routed,
            margins: used,
            work,
        }
    }
}
struct Coordinates {
    sideways: bool,
    flipped: bool,
    head: f64,
    length: f64,
}
impl Coordinates {
    fn flip(&self, a: f64) -> f64 {
        if self.flipped {
            self.head + self.length - a
        } else {
            a
        }
    }
    fn point(&self, c: f64, a: f64) -> (f64, f64) {
        if self.sideways {
            (self.flip(a), c)
        } else {
            (c, self.flip(a))
        }
    }
}
fn grow(b: &mut [f64; 4], c: &CNode) {
    b[0] = b[0].min(c.x - c.w / 2.);
    b[1] = b[1].min(c.y - c.h / 2.);
    b[2] = b[2].max(c.x + c.w / 2.);
    b[3] = b[3].max(c.y + c.h / 2.)
}
fn border(from: &CNode, to: &CNode) -> (f64, f64) {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let hw = (from.w / 2.).js_max(0.5);
    let hh = (from.h / 2.).js_max(0.5);
    if dx.abs() < hw && dy.abs() < hh {
        return if hw - dx.abs() < hh - dy.abs() {
            (from.x + if dx < 0. { -hw } else { hw }, to.y)
        } else {
            (to.x, from.y + if dy < 0. { -hh } else { hh })
        };
    }
    let scale = (if dx != 0. {
        hw / dx.abs()
    } else {
        f64::INFINITY
    })
    .js_min(if dy != 0. {
        hh / dy.abs()
    } else {
        f64::INFINITY
    });
    (from.x + dx * scale, from.y + dy * scale)
}
struct Router<'a> {
    edges: &'a mut [CEdge],
    real: usize,
    size: &'a [f64],
    depth: &'a [f64],
    cross: &'a [f64],
    ranks: &'a [usize],
    band_start: &'a [f64],
    band_end: &'a [f64],
    coords: &'a Coordinates,
    labels: Vec<(f64, f64)>,
    breadth: f64,
    length: f64,
    sideways: bool,
    axis: f64,
}
fn overlaps(c0: f64, a0: f64, c1: f64, a1: f64, lc: f64, la: f64, cw: f64, ah: f64) -> bool {
    c1 > lc - cw / 2. && c0 < lc + cw / 2. && a1 > la - ah / 2. && a0 < la + ah / 2.
}
impl Router<'_> {
    fn amid(&self, i: usize) -> f64 {
        (self.band_start[self.ranks[i]] + self.band_end[self.ranks[i]]) / 2.
    }
    fn clear(&self, c0: f64, a0: f64, c1: f64, a1: f64, src: isize, dst: isize, ei: usize) -> bool {
        for i in 0..self.real {
            if i as isize != src
                && i as isize != dst
                && overlaps(
                    c0,
                    a0,
                    c1,
                    a1,
                    self.cross[i],
                    self.amid(i),
                    self.size[i] + 8.,
                    self.depth[i] + 8.,
                )
            {
                return false;
            }
        }
        for (i, e) in self.edges.iter().enumerate() {
            if i != ei
                && e.label_w > 0.
                && e.src != e.dst
                && overlaps(
                    c0,
                    a0,
                    c1,
                    a1,
                    self.labels[i].0,
                    self.labels[i].1,
                    if self.sideways { e.label_h } else { e.label_w },
                    if self.sideways { e.label_w } else { e.label_h },
                )
            {
                return false;
            }
        }
        true
    }
    fn crossings(&self, x0: f64, y0: f64, x1: f64, y1: f64, ei: usize) -> usize {
        let mut count = 0;
        for (i, e) in self.edges.iter().enumerate() {
            if i == ei {
                continue;
            }
            let r = &e.route;
            for j in (3..r.len()).step_by(3) {
                if r[j - 3].js_max(r[j]) > x0
                    && r[j - 3].js_min(r[j]) < x1
                    && r[j - 2].js_max(r[j + 1]) > y0
                    && r[j - 2].js_min(r[j + 1]) < y1
                {
                    count += 1;
                    break;
                }
            }
        }
        count
    }
    fn meets(&self, x: f64, y: f64, ei: usize) -> usize {
        let mut count = 0;
        for (i, e) in self.edges.iter().enumerate() {
            let r = &e.route;
            if i == ei || r.is_empty() {
                continue;
            }
            let last = r.len() - 3;
            if ((r[0] - x).abs() < 1. && (r[1] - y).abs() < 1.)
                || ((r[last] - x).abs() < 1. && (r[last + 1] - y).abs() < 1.)
            {
                count += 1
            }
        }
        count
    }
    fn retraces(&self, x0: f64, y0: f64, x1: f64, y1: f64, ei: usize) -> bool {
        let upright = (x1 - x0).abs() < 0.5;
        let (lo, hi) = if upright {
            (y0.js_min(y1), y0.js_max(y1))
        } else {
            (x0.js_min(x1), x0.js_max(x1))
        };
        let line = if upright { x0 } else { y0 };
        for (i, e) in self.edges.iter().enumerate() {
            if i == ei || e.src == self.edges[ei].src || e.dst == self.edges[ei].dst {
                continue;
            }
            let r = &e.route;
            for j in (3..r.len()).step_by(3) {
                let axis = if upright { 0 } else { 1 };
                let (u0, u1) = (r[j - 3 + axis], r[j + axis]);
                if (u0 - line).abs() >= 4. || (u1 - line).abs() >= 4. {
                    continue;
                }
                let axis = if upright { 1 } else { 0 };
                let (v0, v1) = (r[j - 3 + axis], r[j + axis]);
                if hi.js_min(v0.js_max(v1)) - lo.js_max(v0.js_min(v1)) > 1. {
                    return true;
                }
            }
        }
        false
    }
    fn attempt(
        &mut self,
        pts: &[f64],
        margin: f64,
        a: usize,
        b: usize,
        ei: usize,
        e: &LEdge,
    ) -> bool {
        for p in (2..pts.len()).step_by(2) {
            if !self.clear(
                pts[p - 2].js_min(pts[p]) - margin,
                pts[p - 1].js_min(pts[p + 1]) - margin,
                pts[p - 2].js_max(pts[p]) + margin,
                pts[p - 1].js_max(pts[p + 1]) + margin,
                a as isize,
                b as isize,
                ei,
            ) {
                return false;
            }
            let (x0, y0) = self.coords.point(pts[p - 2], pts[p - 1]);
            let (x1, y1) = self.coords.point(pts[p], pts[p + 1]);
            if self.retraces(x0, y0, x1, y1, ei) {
                return false;
            }
        }
        let (mut lc, mut la, mut longest) = (0., 0., -1.);
        let out = &self.edges[ei];
        if out.label_w > 0. {
            let (cw, ah) = (e.label_w, e.label_h);
            for p in (2..pts.len()).step_by(2) {
                let cc = (pts[p - 2] + pts[p]) / 2.;
                let ca = (pts[p - 1] + pts[p + 1]) / 2.;
                let across = (pts[p] - pts[p - 2]).abs();
                let along = (pts[p + 1] - pts[p - 1]).abs();
                let run = across + along;
                if run <= longest
                    || (if across > 0. {
                        across < cw + 8.
                    } else {
                        along < ah + 8.
                    })
                {
                    continue;
                }
                if cc - cw / 2. < 0.
                    || cc + cw / 2. > self.breadth
                    || ca - ah / 2. < 0.
                    || ca + ah / 2. > self.length
                {
                    continue;
                }
                if !self.clear(
                    cc - cw / 2.,
                    ca - ah / 2.,
                    cc + cw / 2.,
                    ca + ah / 2.,
                    -1,
                    -1,
                    ei,
                ) {
                    continue;
                }
                let (lx, ly) = self.coords.point(cc, ca);
                if self.crossings(
                    lx - out.label_w / 2.,
                    ly - out.label_h / 2.,
                    lx + out.label_w / 2.,
                    ly + out.label_h / 2.,
                    ei,
                ) > 0
                {
                    continue;
                }
                lc = cc;
                la = ca;
                longest = run;
            }
            if longest < 0. {
                return false;
            }
        }
        let mut way = vec![pts[0], pts[1]];
        for p in (2..pts.len()).step_by(2) {
            let dx = pts[p] - pts[p - 2];
            let dy = pts[p + 1] - pts[p - 1];
            let run = dx.abs() + dy.abs();
            let first = if p > 2 { 20. } else { 0. };
            let last = if p + 2 < pts.len() { 20. } else { 0. };
            if run > first + last + 1. {
                if first != 0. {
                    way.extend([pts[p - 2] + dx / run * first, pts[p - 1] + dy / run * first])
                }
                if last != 0. {
                    way.extend([pts[p] - dx / run * last, pts[p + 1] - dy / run * last])
                }
            }
            way.extend([pts[p], pts[p + 1]])
        }
        let out = &mut self.edges[ei];
        out.route.clear();
        for p in (0..way.len()).step_by(2) {
            let crosswise = if p == 0 {
                way[0] != way[2]
            } else {
                way[p] != way[p - 2]
            };
            let (x, y) = self.coords.point(way[p], way[p + 1]);
            out.route
                .extend([x, y, if crosswise { 1. - self.axis } else { self.axis }])
        }
        if out.label_w > 0. {
            (out.label_x, out.label_y) = self.coords.point(lc, la);
            self.labels[ei] = (lc, la)
        }
        true
    }
}
