//! Pele layout/layered.ts, 458322a5 (MIT, kepano).
//! Sugiyama ranks, virtual nodes, stable sweeps, Brandes–Köpf positioning and port routing.
use crate::compat::number_math::NumberMath;
use std::collections::{HashMap, HashSet};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Node,
    Dummy,
    Label,
    StartPort,
    EndPort,
}
#[derive(Clone, Debug)]
pub struct LNode {
    pub w: f64,
    pub h: f64,
    pub kind: Kind,
    pub pin: f64,
    pub span: f64,
    pub x: f64,
    pub y: f64,
    pub rank: usize,
}
impl LNode {
    pub fn new(w: f64, h: f64, kind: Kind) -> Self {
        Self {
            w,
            h,
            kind,
            pin: 0.,
            span: 0.,
            x: 0.,
            y: 0.,
            rank: 0,
        }
    }
}
#[derive(Clone, Debug)]
pub struct LEdge {
    pub tail: usize,
    pub head: usize,
    pub minlen: f64,
    pub label_w: f64,
    pub label_h: f64,
    pub tail_dx: f64,
    pub head_dx: f64,
    pub tail_fixed: bool,
    pub head_fixed: bool,
    pub reversed: bool,
    pub points: Vec<f64>,
    pub label_x: f64,
    pub label_y: f64,
}
impl LEdge {
    pub fn new(tail: usize, head: usize, minlen: f64, label_w: f64, label_h: f64) -> Self {
        Self {
            tail,
            head,
            minlen,
            label_w,
            label_h,
            tail_dx: 0.,
            head_dx: 0.,
            tail_fixed: false,
            head_fixed: false,
            reversed: false,
            points: Vec::new(),
            label_x: 0.,
            label_y: 0.,
        }
    }
}
#[derive(Clone, Debug)]
pub struct LayeredOptions {
    pub node_sep: f64,
    pub edge_sep: f64,
    pub rank_sep: f64,
    pub port_sep: f64,
    pub tracks: bool,
    pub straight: bool,
    pub apart: bool,
    pub head_room: f64,
}
#[derive(Clone, Debug)]
pub struct LayeredResult {
    pub width: f64,
    pub height: f64,
}
const MAX_SWEEPS: usize = 24;
const MAX_DUMMIES: usize = 20000;
const PORT_PITCH: f64 = 16.;
const MIN_PITCH: f64 = 8.;
const MIN_SLOPE: f64 = 0.125;
const MAX_GAP: f64 = 3.;
const TRACK_MARGIN: f64 = 18.;
const TRACK_PITCH: f64 = 10.;
const TRACK_SPREAD: f64 = 14.;
const LEVEL_WIDE: f64 = 2.;
const LEVEL_MARGIN: f64 = 28.;
const LEVEL_PITCH: f64 = 12.;
const LEVEL_LEAD: f64 = 12.;
const LEVEL_CLEAR: f64 = 8.;
const LEVEL_TOUCH: f64 = 4.;
const CROSSED_SLOPE: f64 = 0.5;
const IN_LINE: f64 = 0.5;
const SNAP: f64 = 12.;
const LEAN_CLEAR: f64 = 40.;
const MAX_WEIGHED: usize = 400;
const HELD: u8 = 0;
const LEAVES: u8 = 1;
const ARRIVES: u8 = 2;
fn dummy(k: Kind) -> bool {
    matches!(k, Kind::Dummy | Kind::Label)
}
#[derive(Clone)]
struct Graph {
    n: usize,
    real: usize,
    max_rank: usize,
    w: Vec<f64>,
    kind: Vec<Kind>,
    rank: Vec<usize>,
    ef: Vec<usize>,
    et: Vec<usize>,
    efd: Vec<f64>,
    etd: Vec<f64>,
    succ_start: Vec<usize>,
    succ_e: Vec<usize>,
    pred_start: Vec<usize>,
    pred_e: Vec<usize>,
    layer_start: Vec<usize>,
    order: Vec<usize>,
    pos: Vec<usize>,
    offsets: bool,
}
#[derive(Default)]
struct Pieces {
    from: Vec<f64>,
    to: Vec<f64>,
    rank: Vec<usize>,
    lower: Vec<Option<usize>>,
    track: Vec<isize>,
    levels: Vec<usize>,
    wide: Vec<bool>,
}
fn positions(g: &mut Graph) {
    for r in 0..=g.max_rank {
        let s = g.layer_start[r];
        for i in s..g.layer_start[r + 1] {
            g.pos[g.order[i]] = i - s
        }
    }
}
fn adjacency(
    n: usize,
    from: &[usize],
    to: &[usize],
) -> (Vec<usize>, Vec<usize>, Vec<usize>, Vec<usize>) {
    let mut succ = vec![0; n + 1];
    let mut pred = vec![0; n + 1];
    for e in 0..from.len() {
        succ[from[e] + 1] += 1;
        pred[to[e] + 1] += 1
    }
    for i in 0..n {
        succ[i + 1] += succ[i];
        pred[i + 1] += pred[i]
    }
    let mut se = vec![0; from.len()];
    let mut pe = vec![0; from.len()];
    let mut sf = succ[..n].to_vec();
    let mut pf = pred[..n].to_vec();
    for e in 0..from.len() {
        se[sf[from[e]]] = e;
        sf[from[e]] += 1;
        pe[pf[to[e]]] = e;
        pf[to[e]] += 1
    }
    (succ, se, pred, pe)
}
pub fn orient(nodes: &[LNode], edges: &mut [LEdge]) {
    let n = nodes.len();
    let mut out = vec![Vec::new(); n];
    for (i, e) in edges.iter_mut().enumerate() {
        e.reversed = nodes[e.head].kind == Kind::StartPort || nodes[e.tail].kind == Kind::EndPort;
        if !e.reversed {
            out[e.tail].push(i)
        }
    }
    let mut state = vec![0; n];
    let mut cursor = vec![0; n];
    let mut stack = Vec::new();
    for root in 0..n {
        if state[root] != 0 {
            continue;
        }
        stack.push(root);
        state[root] = 1;
        while let Some(&v) = stack.last() {
            if cursor[v] < out[v].len() {
                let i = out[v][cursor[v]];
                cursor[v] += 1;
                let w = edges[i].head;
                if state[w] == 1 {
                    edges[i].reversed = true
                } else if state[w] == 0 {
                    state[w] = 1;
                    stack.push(w)
                }
            } else {
                state[v] = 2;
                stack.pop();
            }
        }
    }
}
pub fn rank(nodes: &mut [LNode], edges: &[LEdge], step: usize) {
    let n = nodes.len();
    let mut from = Vec::new();
    let mut to = Vec::new();
    let mut len = Vec::new();
    for e in edges {
        from.push(if e.reversed { e.head } else { e.tail });
        to.push(if e.reversed { e.tail } else { e.head });
        len.push((e.minlen.max(1.) * step as f64) as i32)
    }
    let (os, oe, ins, ie) = adjacency(n, &from, &to);
    let mut r = vec![0i32; n];
    let mut indeg: Vec<_> = (0..n).map(|i| ins[i + 1] - ins[i]).collect();
    let mut topo: Vec<_> = (0..n).filter(|i| indeg[*i] == 0).collect();
    let mut q = 0;
    while q < topo.len() {
        let v = topo[q];
        for k in os[v]..os[v + 1] {
            let e = oe[k];
            let w = to[e];
            r[w] = r[w].max(r[v] + len[e]);
            indeg[w] -= 1;
            if indeg[w] == 0 {
                topo.push(w)
            }
        }
        q += 1
    }
    for _ in 0..8 {
        let mut changed = false;
        for &v in topo.iter().rev() {
            let ins_count = ins[v + 1] - ins[v];
            let outs = os[v + 1] - os[v];
            if ins_count == 0 && outs == 0 {
                continue;
            }
            let lo = (ins[v]..ins[v + 1])
                .map(|k| r[from[ie[k]]] + len[ie[k]])
                .max();
            let hi = (os[v]..os[v + 1]).map(|k| r[to[oe[k]]] - len[oe[k]]).min();
            let next = if outs > ins_count {
                hi.unwrap_or(r[v])
            } else if ins_count > outs {
                lo.unwrap_or(r[v])
            } else {
                r[v]
            };
            if next != r[v] {
                r[v] = next;
                changed = true
            }
        }
        if !changed {
            break;
        }
    }
    let vals: Vec<_> = (0..n)
        .filter(|i| !matches!(nodes[*i].kind, Kind::StartPort | Kind::EndPort))
        .map(|i| r[i])
        .collect();
    let mut min = *vals.iter().min().unwrap_or(&0);
    let max = *vals.iter().max().unwrap_or(&0);
    let mut start_ports = false;
    for i in 0..n {
        if nodes[i].kind == Kind::StartPort {
            r[i] = min - step as i32;
            start_ports = true
        } else if nodes[i].kind == Kind::EndPort {
            r[i] = max + step as i32
        }
    }
    if start_ports {
        min -= step as i32
    }
    for i in 0..n {
        nodes[i].rank = (r[i] - min) as usize
    }
}
pub fn ease(across: f64, along: f64) -> f64 {
    let across = if across == 0. || across.is_nan() {
        1.
    } else {
        across.abs()
    };
    (4. * along.abs() / across).js_min(0.5).js_max(0.25)
}
pub fn layered(nodes: &mut [LNode], edges: &mut [LEdge], opt: &LayeredOptions) -> LayeredResult {
    let real = nodes.len();
    if real == 0 {
        return LayeredResult {
            width: 0.,
            height: 0.,
        };
    }
    let m = edges.len();
    let mut step = 1;
    let mut seen = HashSet::new();
    for e in edges.iter() {
        let key = (e.tail.min(e.head), e.tail.max(e.head));
        if e.label_w > 0. || !seen.insert(key) {
            step = 2;
            break;
        }
    }
    orient(nodes, edges);
    rank(nodes, edges, step);
    let span: Vec<_> = edges
        .iter()
        .map(|e| nodes[e.head].rank.abs_diff(nodes[e.tail].rank))
        .collect();
    let mut dummies: usize = span.iter().map(|n| n.saturating_sub(1)).sum();
    let mut direct = vec![false; m];
    if dummies > MAX_DUMMIES {
        let mut longest: Vec<_> = (0..m).filter(|i| span[*i] > 1).collect();
        longest.sort_by_key(|i| (std::cmp::Reverse(span[*i]), *i));
        for i in longest {
            if dummies <= MAX_DUMMIES {
                break;
            }
            direct[i] = true;
            dummies -= span[i] - 1
        }
    }
    let n = real + dummies;
    let mut g = Graph {
        n,
        real,
        max_rank: 0,
        w: vec![0.; n],
        kind: vec![Kind::Node; n],
        rank: vec![0; n],
        ef: Vec::new(),
        et: Vec::new(),
        efd: Vec::new(),
        etd: Vec::new(),
        succ_start: Vec::new(),
        succ_e: Vec::new(),
        pred_start: Vec::new(),
        pred_e: Vec::new(),
        layer_start: Vec::new(),
        order: vec![0; n],
        pos: vec![0; n],
        offsets: false,
    };
    let mut h = vec![0.; n];
    for i in 0..real {
        g.w[i] = nodes[i].w;
        h[i] = nodes[i].h;
        g.kind[i] = nodes[i].kind;
        g.rank[i] = nodes[i].rank;
        g.max_rank = g.max_rank.max(nodes[i].rank)
    }
    let max_rank = g.max_rank;
    let spans = nodes.iter().any(|v| v.span > 0. && v.kind == Kind::Node);
    let mut efk = Vec::new();
    let mut etk = Vec::new();
    let mut first_dummy = vec![None; m];
    let mut label_node = vec![None; m];
    let mut first_seg = vec![0; m];
    let mut last_seg = vec![0; m];
    let mut heads = vec![0u8; max_rank + 1];
    let mut next = real;
    for ei in 0..m {
        if direct[ei] {
            continue;
        }
        let e = &edges[ei];
        let (a, b, adx, bdx) = if e.reversed {
            (e.head, e.tail, e.head_dx, e.tail_dx)
        } else {
            (e.tail, e.head, e.tail_dx, e.head_dx)
        };
        if adx != 0. || bdx != 0. {
            g.offsets = true
        }
        let ra = g.rank[a];
        let rb = g.rank[b];
        let mut label_rank = None;
        if e.label_w > 0. {
            let mut r = ra + ((rb - ra) >> 1);
            if step == 2 && rb - ra >= 4 {
                if r & 1 == 1 {
                    r = if e.reversed { r + 1 } else { r - 1 }
                }
            } else if step == 2 && r & 1 == 0 {
                r = if e.reversed { r + 1 } else { r - 1 }
            }
            label_rank = Some(r)
        }
        let mut prev = a;
        if rb - ra > 1 {
            first_dummy[ei] = Some(next)
        }
        first_seg[ei] = g.ef.len();
        for r in ra + 1..rb {
            let d = next;
            next += 1;
            g.rank[d] = r;
            if Some(r) == label_rank {
                g.w[d] = e.label_w;
                h[d] = e.label_h;
                g.kind[d] = Kind::Label;
                label_node[ei] = Some(d);
                heads[r] |= if e.reversed { 2 } else { 1 }
            } else {
                g.kind[d] = Kind::Dummy
            }
            g.ef.push(prev);
            g.et.push(d);
            g.efd.push(if prev == a { adx } else { 0. });
            g.etd.push(0.);
            efk.push(HELD);
            etk.push(HELD);
            prev = d
        }
        g.ef.push(prev);
        g.et.push(b);
        g.efd.push(if prev == a { adx } else { 0. });
        g.etd.push(bdx);
        efk.push(HELD);
        etk.push(HELD);
        if spans {
            if !(if e.reversed {
                e.head_fixed
            } else {
                e.tail_fixed
            }) {
                efk[first_seg[ei]] = if e.reversed { ARRIVES } else { LEAVES }
            }
            if !(if e.reversed {
                e.tail_fixed
            } else {
                e.head_fixed
            }) {
                let last = etk.len() - 1;
                etk[last] = if e.reversed { LEAVES } else { ARRIVES }
            }
        }
        last_seg[ei] = g.ef.len() - 1;
    }
    (g.succ_start, g.succ_e, g.pred_start, g.pred_e) = adjacency(n, &g.ef, &g.et);
    g.layer_start = vec![0; max_rank + 2];
    for i in 0..n {
        g.layer_start[g.rank[i] + 1] += 1
    }
    for r in 0..=max_rank {
        g.layer_start[r + 1] += g.layer_start[r]
    }
    {
        let mut by_rank = vec![0; n];
        let mut fill = g.layer_start[..max_rank + 1].to_vec();
        for i in 0..n {
            by_rank[fill[g.rank[i]]] = i;
            fill[g.rank[i]] += 1
        }
        fill.copy_from_slice(&g.layer_start[..max_rank + 1]);
        let mut visited = vec![false; n];
        let mut stack = Vec::new();
        for &root in &by_rank {
            if visited[root] {
                continue;
            }
            stack.push(root);
            while let Some(v) = stack.pop() {
                if visited[v] {
                    continue;
                }
                visited[v] = true;
                g.order[fill[g.rank[v]]] = v;
                fill[g.rank[v]] += 1;
                for j in (g.succ_start[v]..g.succ_start[v + 1]).rev() {
                    let w = g.et[g.succ_e[j]];
                    if !visited[w] {
                        stack.push(w)
                    }
                }
            }
        }
    }
    if nodes.iter().any(|n| n.pin != 0.) {
        for r in 0..=max_rank {
            g.order[g.layer_start[r]..g.layer_start[r + 1]].sort_by(|a, b| {
                let pa = if *a < real { nodes[*a].pin } else { 0. };
                let pb = if *b < real { nodes[*b].pin } else { 0. };
                pa.total_cmp(&pb)
            })
        }
    }
    positions(&mut g);
    if !g.ef.is_empty() && max_rank > 0 {
        reduce_crossings(&mut g, nodes);
        positions(&mut g)
    }
    if spans {
        spread(&mut g, nodes, &efk, &etk)
    }
    let mut xs = position(&mut g, opt);
    let empty = empty_ranks(&g);
    if spans {
        let free = free_ends(&g, &efk, &etk);
        align_labels(&g, nodes, &mut xs, &free, opt, empty.as_deref());
        settle(&mut g, nodes, &xs, &efk, &etk, &free, empty.as_deref());
        snap(&g, &mut xs, empty.as_deref(), opt);
        for ei in 0..m {
            if direct[ei] {
                continue;
            }
            let e = &mut edges[ei];
            let adx = g.efd[first_seg[ei]];
            let bdx = g.etd[last_seg[ei]];
            e.tail_dx = if e.reversed { bdx } else { adx };
            e.head_dx = if e.reversed { adx } else { bdx }
        }
    }
    let mut on_line = vec![false; n];
    if !opt.tracks {
        if let Some(empty) = &empty {
            straighten(
                &g,
                &mut xs,
                empty,
                &first_dummy,
                &span,
                if opt.apart { 0. } else { opt.edge_sep },
                &mut on_line,
            )
        }
        past_labels(&g, &mut xs, &first_dummy, &span, opt.edge_sep, &mut on_line)
    }
    let mut piece_start = vec![0; m + 1];
    let mut pieces = if opt.tracks || (spans && !opt.straight) {
        Some(Pieces::default())
    } else {
        None
    };
    let mut track_count = vec![0; max_rank + 1];
    if let Some(p) = &mut pieces {
        for ei in 0..m {
            piece_start[ei] = p.from.len();
            if direct[ei] {
                continue;
            }
            let bends = first_dummy[ei].map_or(0..0, |d| d..d + span[ei] - 1);
            let twin = bends
                .clone()
                .any(|d| !opt.tracks && empty.as_ref().is_some_and(|e| e[g.rank[d]]) != on_line[d]);
            if twin {
                continue;
            }
            let mut x = xs[g.ef[first_seg[ei]]] + g.efd[first_seg[ei]];
            let mut r = g.rank[g.ef[first_seg[ei]]];
            for d in bends {
                if empty.as_ref().is_some_and(|e| e[g.rank[d]]) {
                    continue;
                }
                p.from.push(x);
                p.to.push(xs[d]);
                p.rank.push(r);
                p.lower.push(Some(d));
                x = xs[d];
                r = g.rank[d]
            }
            p.from.push(x);
            p.to.push(xs[g.et[last_seg[ei]]] + g.etd[last_seg[ei]]);
            p.rank.push(r);
            p.lower.push(None)
        }
        piece_start[m] = p.from.len()
    }
    let port_only = |r: usize, kind: Kind| {
        let mut found = false;
        for i in g.layer_start[r]..g.layer_start[r + 1] {
            let k = g.kind[g.order[i]];
            if k == kind {
                found = true
            } else if k != Kind::Dummy {
                return false;
            }
        }
        found
    };
    let start_ranks = if port_only(0, Kind::StartPort) {
        step
    } else {
        0
    };
    let end_ranks = if port_only(max_rank, Kind::EndPort) {
        step
    } else {
        0
    };
    let mut run = vec![0f64; max_rank + 1];
    for e in 0..g.ef.len() {
        let across = (xs[g.ef[e]] + g.efd[e] - xs[g.et[e]] - g.etd[e]).abs();
        let r = g.rank[g.ef[e]];
        run[r] = run[r].max(across)
    }
    let mut top = vec![0.; max_rank + 1];
    let mut bottom = vec![0.; max_rank + 1];
    let usual = |r: usize| {
        if r < start_ranks || r >= max_rank.saturating_sub(end_ranks) {
            opt.port_sep / step as f64
        } else {
            opt.rank_sep / step as f64
        }
    };
    let mut gap = vec![0.; max_rank + 1];
    for r in 0..max_rank {
        gap[r] = if pieces.is_some() {
            usual(r)
        } else {
            usual(r).js_max((MAX_GAP * opt.rank_sep / step as f64).js_min(run[r] * MIN_SLOPE))
        }
    }
    let mut between = vec![true; max_rank + 1];
    for i in 0..real {
        between[g.rank[i]] = false
    }
    for r in 1..max_rank {
        if opt.tracks || !between[r] || !matches!(heads[r], 1 | 2) {
            continue;
        }
        let shift = (opt.head_room / 2.).js_min(gap[if heads[r] == 1 { r - 1 } else { r }] / 2.);
        gap[r - 1] += if heads[r] == 1 { -shift } else { shift };
        gap[r] += if heads[r] == 1 { shift } else { -shift }
    }
    let margin = if opt.tracks {
        TRACK_MARGIN
    } else {
        LEVEL_MARGIN
    };
    let least_pitch = if opt.tracks { TRACK_PITCH } else { LEVEL_PITCH };
    let most_pitch = if opt.tracks {
        TRACK_SPREAD
    } else {
        LEVEL_PITCH
    };
    let mut below = vec![0; max_rank + 1];
    if let Some(p) = &mut pieces {
        let mut room = vec![0.; max_rank + 1];
        for r in 0..max_rank {
            if empty.as_ref().is_some_and(|e| e[r]) {
                continue;
            }
            let mut next = r + 1;
            room[r] = gap[r];
            while next < max_rank && empty.as_ref().is_some_and(|e| e[next]) {
                room[r] += gap[next];
                next += 1
            }
            below[r] = next
        }
        p.wide = (0..p.from.len())
            .map(|k| (p.from[k] - p.to[k]).abs() > LEVEL_WIDE * room[p.rank[k]])
            .collect();
        let mut tall = vec![0.; max_rank + 1];
        assign_tracks(p, &mut track_count, !opt.tracks, &room, &mut tall);
        for r in 0..max_rank {
            if track_count[r] > 0 {
                gap[r] +=
                    (2. * margin + (track_count[r] as f64 - 1.) * least_pitch - room[r]).js_max(0.)
            } else if tall[r] > 0. {
                gap[r] += ((MAX_GAP * opt.rank_sep).js_min(tall[r]) - room[r]).js_max(0.)
            }
        }
    }
    let mut y = 0.;
    for r in 0..=max_rank {
        let h = (g.layer_start[r]..g.layer_start[r + 1])
            .map(|i| h[g.order[i]])
            .fold(0., f64::max);
        top[r] = y;
        bottom[r] = y + h;
        y += h + gap[r]
    }
    let height = y;
    let track_y = |p: &Pieces, k: usize, on: isize| {
        let r = p.rank[k];
        let count = if opt.tracks {
            track_count[r]
        } else {
            p.levels[k]
        };
        let pitch = if count > 1 {
            most_pitch.js_min((top[below[r]] - bottom[r] - 2. * margin) / (count as f64 - 1.))
        } else {
            0.
        };
        (bottom[r] + top[below[r]]) / 2. + (on as f64 - (count as f64 - 1.) / 2.) * pitch
    };
    let inset = |d: usize| {
        if g.kind[d] == Kind::Dummy && between[g.rank[d]] && !opt.tracks {
            (bottom[g.rank[d]] - top[g.rank[d]]) / 4.
        } else {
            0.
        }
    };
    let min_x = (0..n)
        .map(|i| xs[i] - g.w[i] / 2.)
        .fold(f64::INFINITY, f64::min);
    let max_x = (0..n)
        .map(|i| xs[i] + g.w[i] / 2.)
        .fold(f64::NEG_INFINITY, f64::max);
    let mut arrives = vec![false; max_rank + 1];
    let mut leaves = vec![false; max_rank + 1];
    if spans {
        for i in 0..real {
            if g.kind[i] != Kind::Node {
                continue;
            }
            if g.pred_start[i + 1] > g.pred_start[i] {
                arrives[g.rank[i]] = true
            }
            if g.succ_start[i + 1] > g.succ_start[i] {
                leaves[g.rank[i]] = true
            }
        }
    }
    for i in 0..real {
        let r = nodes[i].rank;
        nodes[i].x = xs[i] - min_x;
        nodes[i].y = if g.kind[i] == Kind::Node && leaves[r] && !arrives[r] {
            bottom[r] - nodes[i].h / 2.
        } else if g.kind[i] == Kind::Node && arrives[r] && !leaves[r] {
            top[r] + nodes[i].h / 2.
        } else {
            (top[r] + bottom[r]) / 2.
        }
    }
    for ei in 0..m {
        let e = &mut edges[ei];
        let first = &nodes[if e.reversed { e.head } else { e.tail }];
        let last = &nodes[if e.reversed { e.tail } else { e.head }];
        let mut pts = vec![
            first.x + if e.reversed { e.head_dx } else { e.tail_dx },
            first.y + first.h / 2.,
        ];
        if let Some(p) = pieces
            .as_ref()
            .filter(|_| piece_start[ei + 1] > piece_start[ei])
        {
            for k in piece_start[ei]..piece_start[ei + 1] {
                if p.track[k] >= 0 && (opt.tracks || p.wide[k]) {
                    let across = track_y(p, k, p.track[k]);
                    pts.extend_from_slice(&[p.from[k] - min_x, across, p.to[k] - min_x, across])
                } else if p.track[k] > 0 {
                    pts.extend_from_slice(&[p.from[k] - min_x, track_y(p, k, p.track[k] - 1)])
                }
                if let Some(d) = p.lower[k] {
                    pts.extend_from_slice(&[xs[d] - min_x, top[g.rank[d]] + inset(d)]);
                    if bottom[g.rank[d]] > top[g.rank[d]] {
                        pts.extend_from_slice(&[xs[d] - min_x, bottom[g.rank[d]] - inset(d)])
                    }
                }
            }
        } else if let Some(d0) = first_dummy[ei] {
            for d in d0..d0 + span[ei] - 1 {
                if on_line[d] {
                    continue;
                }
                pts.extend_from_slice(&[xs[d] - min_x, top[g.rank[d]] + inset(d)]);
                if bottom[g.rank[d]] > top[g.rank[d]] {
                    pts.extend_from_slice(&[xs[d] - min_x, bottom[g.rank[d]] - inset(d)])
                }
            }
        }
        pts.extend_from_slice(&[
            last.x + if e.reversed { e.tail_dx } else { e.head_dx },
            last.y - last.h / 2.,
        ]);
        if e.reversed {
            let mut i = 0;
            let mut j = pts.len() - 2;
            while i < j {
                pts.swap(i, j);
                pts.swap(i + 1, j + 1);
                i += 2;
                j -= 2
            }
        }
        if let Some(ln) = label_node[ei] {
            e.label_x = xs[ln] - min_x;
            e.label_y = (top[g.rank[ln]] + bottom[g.rank[ln]]) / 2.
        } else if e.label_w > 0. {
            e.label_x = (pts[0] + pts[pts.len() - 2]) / 2.;
            e.label_y = (pts[1] + pts[pts.len() - 1]) / 2.
        }
        e.points = pts
    }
    LayeredResult {
        width: max_x - min_x,
        height,
    }
}
fn crossing_count(g: &Graph) -> usize {
    let mut total = 0;
    for r in 0..g.max_rank {
        let mut place = vec![0; g.ef.len()];
        let mut south_len = g.layer_start[r + 2] - g.layer_start[r + 1];
        if g.offsets {
            let mut places = 0;
            for i in g.layer_start[r + 1]..g.layer_start[r + 2] {
                let v = g.order[i];
                let from = g.pred_start[v];
                let to = g.pred_start[v + 1];
                let mixed = (from + 1..to).any(|k| g.etd[g.pred_e[k]] != g.etd[g.pred_e[from]]);
                if mixed {
                    let mut ids = g.pred_e[from..to].to_vec();
                    ids.sort_by(|a, b| g.etd[*a].total_cmp(&g.etd[*b]));
                    for k in 0..ids.len() {
                        if k > 0 && g.etd[ids[k]] != g.etd[ids[k - 1]] {
                            places += 1
                        }
                        place[ids[k]] = places
                    }
                } else {
                    for k in from..to {
                        place[g.pred_e[k]] = places
                    }
                }
                places += 1
            }
            south_len = places
        }
        if south_len < 2 {
            continue;
        }
        let mut targets = Vec::new();
        for i in g.layer_start[r]..g.layer_start[r + 1] {
            let v = g.order[i];
            let from = g.succ_start[v];
            let to = g.succ_start[v + 1];
            let mut ids = g.succ_e[from..to].to_vec();
            if g.offsets && (from + 1..to).any(|k| g.efd[g.succ_e[k]] != g.efd[g.succ_e[from]]) {
                ids.sort_by(|a, b| {
                    g.efd[*a]
                        .total_cmp(&g.efd[*b])
                        .then(place[*a].cmp(&place[*b]))
                })
            } else {
                ids.sort_by_key(|e| {
                    if g.offsets {
                        place[*e]
                    } else {
                        g.pos[g.et[*e]]
                    }
                })
            }
            targets.extend(
                ids.into_iter()
                    .map(|e| if g.offsets { place[e] } else { g.pos[g.et[e]] }),
            )
        }
        let leaves = south_len.next_power_of_two();
        let mut tree = vec![0; 2 * leaves];
        for target in targets {
            let mut i = target + leaves;
            tree[i] += 1;
            while i > 1 {
                if i & 1 == 0 {
                    total += tree[i + 1]
                }
                i >>= 1;
                tree[i] += 1
            }
        }
    }
    total
}
fn out_of_order(g: &Graph) -> usize {
    let mut total = 0;
    for r in 0..=g.max_rank {
        for i in g.layer_start[r]..g.layer_start[r + 1] {
            if g.order[i] >= g.real {
                continue;
            }
            for j in i + 1..g.layer_start[r + 1] {
                if g.order[j] < g.order[i] {
                    total += 1
                }
            }
        }
    }
    total
}
fn sweep(g: &mut Graph, nodes: &[LNode], down: bool, bias_right: bool) {
    let mut bary = vec![0.; g.n];
    for step in 1..=g.max_rank {
        let r = if down { step } else { g.max_rank - step };
        let s = g.layer_start[r];
        let end = g.layer_start[r + 1];
        let (start, list, other, other_dx) = if down {
            (&g.pred_start, &g.pred_e, &g.ef, &g.efd)
        } else {
            (&g.succ_start, &g.succ_e, &g.et, &g.etd)
        };
        let mut moving = Vec::new();
        for i in s..end {
            let v = g.order[i];
            let pin = if v < g.real { nodes[v].pin } else { 0. };
            if pin != 0. {
                bary[v] = pin * 1e9;
                moving.push(v);
                continue;
            }
            let from = start[v];
            let to = start[v + 1];
            if from == to {
                bary[v] = -1.;
                continue;
            }
            let mut sum = 0.;
            for k in from..to {
                let u = other[list[k]];
                sum += g.pos[u] as f64;
                if g.offsets && g.w[u] > 0. {
                    sum += other_dx[list[k]] / g.w[u]
                }
            }
            bary[v] = sum / (to - from) as f64;
            moving.push(v)
        }
        if moving.len() < 2 {
            continue;
        }
        moving.sort_by(|a, b| {
            bary[*a].total_cmp(&bary[*b]).then(if bias_right {
                g.pos[*b].cmp(&g.pos[*a])
            } else {
                g.pos[*a].cmp(&g.pos[*b])
            })
        });
        let mut k = 0;
        for i in s..end {
            if bary[g.order[i]] != -1. {
                g.order[i] = moving[k];
                k += 1
            }
        }
        for i in s..end {
            g.pos[g.order[i]] = i - s
        }
    }
}
fn reduce_crossings(g: &mut Graph, nodes: &[LNode]) {
    let mut best = crossing_count(g);
    if best == 0 {
        return;
    }
    let initial = g.order.clone();
    let start = best;
    let small = g.n <= 600;
    let mut chosen = initial.clone();
    let mut chosen_out = usize::MAX;
    for attempt in 0..if small { 2 } else { 1 } {
        if attempt > 0 {
            g.order.clone_from(&initial);
            positions(g)
        }
        let mut found = start;
        let mut found_order = initial.clone();
        let mut stale = 0;
        for i in 0..MAX_SWEEPS {
            if stale >= 4 {
                break;
            }
            sweep(g, nodes, (i + attempt) % 2 == 0, i % 4 >= 2);
            let count = crossing_count(g);
            if count < found {
                found = count;
                found_order.clone_from(&g.order);
                stale = 0;
                if count == 0 {
                    break;
                }
            }
            stale += 1
        }
        g.order.clone_from(&found_order);
        let out = if small { out_of_order(g) } else { 0 };
        if attempt == 0 || found < best || (found == best && out < chosen_out) {
            best = found;
            chosen = found_order;
            chosen_out = out
        }
    }
    g.order.clone_from(&chosen);
    positions(g);
    if !small || nodes.iter().any(|v| v.pin != 0.) || chosen_out == 0 {
        return;
    }
    for r in 0..=g.max_rank {
        g.order[g.layer_start[r]..g.layer_start[r + 1]].reverse()
    }
    positions(g);
    if crossing_count(g) > best || out_of_order(g) >= chosen_out {
        g.order = chosen
    }
}
fn half_room(g: &Graph, opt: &LayeredOptions, v: usize) -> f64 {
    g.w[v] / 2.
        + if g.kind[v] == Kind::Node {
            opt.node_sep / 2.
        } else {
            opt.edge_sep / 2.
        }
}
fn position(g: &mut Graph, opt: &LayeredOptions) -> Vec<f64> {
    let n = g.n;
    let gaps: Vec<_> = (0..n).map(|v| half_room(g, opt, v)).collect();
    for v in 0..n {
        g.succ_e[g.succ_start[v]..g.succ_start[v + 1]].sort_by_key(|e| (g.pos[g.et[*e]], *e));
        g.pred_e[g.pred_start[v]..g.pred_start[v + 1]].sort_by_key(|e| (g.pos[g.ef[*e]], *e))
    }
    let mut conflict = vec![false; g.ef.len()];
    for r in 1..=g.max_rank {
        let prev_len = g.layer_start[r] - g.layer_start[r - 1];
        let s = g.layer_start[r];
        let end = g.layer_start[r + 1];
        let mut k0 = 0;
        let mut scan = s;
        for i in s..end {
            let v = g.order[i];
            let inner = if dummy(g.kind[v]) {
                let u = g.ef[g.pred_e[g.pred_start[v]]];
                dummy(g.kind[u]).then_some(u)
            } else {
                None
            };
            let k1 = inner.map_or(prev_len, |v| g.pos[v]);
            if inner.is_some() || i + 1 == end {
                for j in scan..=i {
                    let w = g.order[j];
                    for k in g.pred_start[w]..g.pred_start[w + 1] {
                        let e = g.pred_e[k];
                        let up = g.pos[g.ef[e]];
                        if (up < k0 || k1 < up) && !(dummy(g.kind[w]) && dummy(g.kind[g.ef[e]])) {
                            conflict[e] = true
                        }
                    }
                }
                scan = i + 1;
                k0 = k1
            }
        }
    }
    let mut results = Vec::new();
    for variant in 0..4 {
        let up = variant < 2;
        let right = variant & 1 == 1;
        let (start, list, other, own_dx, other_dx) = if up {
            (&g.pred_start, &g.pred_e, &g.ef, &g.etd, &g.efd)
        } else {
            (&g.succ_start, &g.succ_e, &g.et, &g.efd, &g.etd)
        };
        let mut root: Vec<_> = (0..n).collect();
        let mut align = root.clone();
        let mut shift = vec![0.; n];
        for step in 0..=g.max_rank {
            let r = if up { step } else { g.max_rank - step };
            let s = g.layer_start[r];
            let end = g.layer_start[r + 1];
            let adjacent = if up {
                r.checked_sub(1)
            } else {
                (r < g.max_rank).then_some(r + 1)
            };
            let adjacent_last = adjacent.map_or(0isize, |r| {
                (g.layer_start[r + 1] - g.layer_start[r]) as isize - 1
            });
            let mut prev_idx = -1isize;
            let row: Vec<_> = if right {
                (s..end).rev().collect()
            } else {
                (s..end).collect()
            };
            for i in row {
                let v = g.order[i];
                let from = start[v];
                let d = start[v + 1] - from;
                if d == 0 {
                    continue;
                }
                let lo = from + ((d - 1) >> 1);
                let hi = from + (d >> 1);
                for pass in 0..2 {
                    if pass == 1 && lo == hi {
                        break;
                    }
                    let e = list[if (pass == 0) != right { lo } else { hi }];
                    let w = other[e];
                    let idx = if right {
                        adjacent_last - g.pos[w] as isize
                    } else {
                        g.pos[w] as isize
                    };
                    if align[v] == v && prev_idx < idx && !conflict[e] {
                        align[w] = v;
                        align[v] = root[w];
                        root[v] = root[w];
                        shift[v] = shift[w] + other_dx[e] - own_dx[e];
                        prev_idx = idx
                    }
                }
            }
        }
        let mut from = Vec::new();
        let mut to = Vec::new();
        let mut weight = Vec::new();
        let mut indeg = vec![0; n];
        for r in 0..=g.max_rank {
            for i in g.layer_start[r] + 1..g.layer_start[r + 1] {
                let (a, b) = if right {
                    (g.order[i], g.order[i - 1])
                } else {
                    (g.order[i - 1], g.order[i])
                };
                let sep = gaps[a] + gaps[b];
                from.push(root[a]);
                to.push(root[b]);
                weight.push(if right {
                    sep - shift[a] + shift[b]
                } else {
                    sep + shift[a] - shift[b]
                });
                indeg[root[b]] += 1
            }
        }
        let (os, oe, _, _) = adjacency(n, &from, &to);
        let mut x = vec![0f64; n];
        let mut topo: Vec<_> = (0..n)
            .filter(|i| root[*i] == *i && indeg[*i] == 0)
            .collect();
        let mut q = 0;
        while q < topo.len() {
            let v = topo[q];
            for k in os[v]..os[v + 1] {
                let e = oe[k];
                let w = to[e];
                x[w] = x[w].max(x[v] + weight[e]);
                indeg[w] -= 1;
                if indeg[w] == 0 {
                    topo.push(w)
                }
            }
            q += 1
        }
        for &v in topo.iter().rev() {
            let min = (os[v]..os[v + 1])
                .map(|k| x[to[oe[k]]] - weight[oe[k]])
                .fold(f64::INFINITY, f64::min);
            if min != f64::INFINITY && min > x[v] {
                x[v] = min
            }
        }
        results.push(
            (0..n)
                .map(|i| {
                    if right {
                        -x[root[i]] + shift[i]
                    } else {
                        x[root[i]] + shift[i]
                    }
                })
                .collect::<Vec<_>>(),
        );
    }
    let mut narrowest = 0;
    let mut best = f64::INFINITY;
    let mut mins = Vec::new();
    let mut maxs = Vec::new();
    for (k, xs) in results.iter().enumerate() {
        let lo = (0..n)
            .map(|i| xs[i] - g.w[i] / 2.)
            .fold(f64::INFINITY, f64::min);
        let hi = (0..n)
            .map(|i| xs[i] + g.w[i] / 2.)
            .fold(f64::NEG_INFINITY, f64::max);
        mins.push(lo);
        maxs.push(hi);
        if hi - lo < best {
            best = hi - lo;
            narrowest = k
        }
    }
    for k in 0..4 {
        let delta = if k & 1 == 0 {
            mins[narrowest] - mins[k]
        } else {
            maxs[narrowest] - maxs[k]
        };
        if delta != 0. {
            for x in &mut results[k] {
                *x += delta
            }
        }
    }
    let mut out = vec![0.; n];
    for i in 0..n {
        let mut values = [results[0][i], results[1][i], results[2][i], results[3][i]];
        values.sort_by(f64::total_cmp);
        out[i] = (values[1] + values[2]) / 2.
    }
    out
}
fn side_ends(
    g: &Graph,
    v: usize,
    side: usize,
    span: f64,
    kinds: &[u8],
) -> (Vec<usize>, Vec<f64>, f64) {
    let (start, list, other) = if side == 0 {
        (&g.pred_start, &g.pred_e, &g.ef)
    } else {
        (&g.succ_start, &g.succ_e, &g.et)
    };
    let mut ids: Vec<_> = (start[v]..start[v + 1])
        .map(|k| list[k])
        .filter(|e| kinds[*e] != HELD)
        .collect();
    if ids.len() < 2 {
        return (ids, Vec::new(), 0.);
    }
    ids.sort_by_key(|e| (g.pos[other[*e]], *e));
    let mut room = vec![1.; ids.len()];
    room[0] = 0.;
    let mut total = ids.len() as f64 - 1.;
    if span / total < MIN_PITCH {
        for k in 1..ids.len() {
            room[k] = if kinds[ids[k]] == kinds[ids[k - 1]] && kinds[ids[k]] == LEAVES {
                0.5
            } else {
                1.
            }
        }
        total = room.iter().sum()
    }
    if span / total < MIN_PITCH {
        for k in 1..ids.len() {
            room[k] = if kinds[ids[k]] == kinds[ids[k - 1]] {
                0.
            } else {
                1.
            }
        }
        total = room.iter().sum()
    }
    if total == 0. {
        return (ids, room, 0.);
    }
    let pitch = PORT_PITCH.js_min(span / total);
    for r in room.iter_mut().skip(1) {
        *r *= pitch
    }
    (ids, room, total * pitch)
}
fn spread(g: &mut Graph, nodes: &[LNode], efk: &[u8], etk: &[u8]) {
    for v in 0..g.real {
        let span = nodes[v].span.js_min(g.w[v]);
        if span <= 0. || nodes[v].kind != Kind::Node {
            continue;
        }
        for side in 0..2 {
            let (ids, room, total) = side_ends(g, v, side, span, if side == 0 { etk } else { efk });
            if total == 0. {
                continue;
            }
            let dx = if side == 0 { &mut g.etd } else { &mut g.efd };
            let mut at = -total / 2.;
            for k in 0..ids.len() {
                at += room[k];
                dx[ids[k]] = at
            }
        }
    }
}
fn follow(g: &Graph, empty: Option<&[bool]>, mut e: usize, side: usize) -> usize {
    let Some(empty) = empty else { return e };
    for _ in 0..100000 {
        let u = if side == 0 { g.ef[e] } else { g.et[e] };
        if g.kind[u] != Kind::Dummy || !empty[g.rank[u]] {
            break;
        }
        e = if side == 0 {
            g.pred_e[g.pred_start[u]]
        } else {
            g.succ_e[g.succ_start[u]]
        }
    }
    e
}
fn free_ends(g: &Graph, efk: &[u8], etk: &[u8]) -> [Vec<usize>; 2] {
    let mut free = [vec![0; g.n], vec![0; g.n]];
    for e in 0..g.ef.len() {
        if etk[e] != HELD {
            free[0][g.et[e]] += 1
        }
        if efk[e] != HELD {
            free[1][g.ef[e]] += 1
        }
    }
    free
}
fn label_room(g: &Graph, xs: &[f64], v: usize, opt: &LayeredOptions) -> (f64, f64) {
    let s = g.layer_start[g.rank[v]];
    let i = s + g.pos[v];
    let clear = |u: usize| {
        g.w[u] / 2.
            + g.w[v] / 2.
            + if g.kind[u] == Kind::Node {
                opt.node_sep / 2. + opt.edge_sep / 2.
            } else if g.kind[u] == g.kind[v] {
                opt.edge_sep
            } else {
                opt.edge_sep / 2.
            }
    };
    (
        if i > s {
            xs[g.order[i - 1]] + clear(g.order[i - 1])
        } else {
            f64::NEG_INFINITY
        },
        if i + 1 < g.layer_start[g.rank[v] + 1] {
            xs[g.order[i + 1]] - clear(g.order[i + 1])
        } else {
            f64::INFINITY
        },
    )
}
fn align_labels(
    g: &Graph,
    nodes: &[LNode],
    xs: &mut [f64],
    free: &[Vec<usize>; 2],
    opt: &LayeredOptions,
    empty: Option<&[bool]>,
) {
    let placed = |v: usize| {
        g.kind[v] == Kind::Label
            || (g.kind[v] == Kind::Dummy && !empty.is_some_and(|e| e[g.rank[v]]))
    };
    let end = |xs: &[f64], e: usize, side: usize| {
        let u = if side == 0 { g.ef[e] } else { g.et[e] };
        (
            xs[u] + if side == 0 { g.efd[e] } else { g.etd[e] },
            if u < g.real && nodes[u].kind == Kind::Node {
                free[1 - side][u]
            } else {
                0
            },
        )
    };
    let choose = |x: f64, up: (f64, usize), down: (f64, usize), lo: f64, hi: f64| {
        let up_first = if up.1 != down.1 {
            up.1 < down.1
        } else {
            (up.0 - x).abs() <= (down.0 - x).abs()
        };
        let choices = if up_first {
            [up.0, down.0]
        } else {
            [down.0, up.0]
        };
        choices
            .into_iter()
            .find(|v| *v >= lo && *v <= hi)
            .unwrap_or_else(|| lo.js_max(hi.js_min(choices[0])))
    };
    let mut done = vec![false; g.n];
    for r in 1..g.max_rank {
        for i in g.layer_start[r]..g.layer_start[r + 1] {
            if !placed(g.order[i]) || done[g.order[i]] {
                continue;
            }
            let mut top = g.order[i];
            while dummy(g.kind[g.ef[g.pred_e[g.pred_start[top]]]]) {
                top = g.ef[g.pred_e[g.pred_start[top]]]
            }
            let mut bottom = top;
            let mut lo = f64::NEG_INFINITY;
            let mut hi = f64::INFINITY;
            let mut chain = Vec::new();
            let mut d = top;
            while dummy(g.kind[d]) {
                bottom = d;
                if placed(d) {
                    done[d] = true;
                    chain.push(d);
                    let (least, most) = label_room(g, xs, d, opt);
                    lo = lo.js_max(least);
                    hi = hi.js_min(most)
                }
                d = g.et[g.succ_e[g.succ_start[d]]]
            }
            if lo <= hi {
                let x = choose(
                    xs[chain[0]],
                    end(xs, g.pred_e[g.pred_start[top]], 0),
                    end(xs, g.succ_e[g.succ_start[bottom]], 1),
                    lo,
                    hi,
                );
                for d in chain {
                    xs[d] = x
                }
                continue;
            }
            for d in chain {
                let (least, most) = label_room(g, xs, d, opt);
                if least > most {
                    continue;
                }
                xs[d] = choose(
                    xs[d],
                    end(xs, follow(g, empty, g.pred_e[g.pred_start[d]], 0), 0),
                    end(xs, follow(g, empty, g.succ_e[g.succ_start[d]], 1), 1),
                    least,
                    most,
                )
            }
        }
    }
}
fn wanted(
    g: &Graph,
    nodes: &[LNode],
    xs: &[f64],
    free: &[Vec<usize>; 2],
    efk: &[u8],
    etk: &[u8],
    empty: Option<&[bool]>,
    e: usize,
    side: usize,
    v: usize,
    span: f64,
    again: bool,
) -> f64 {
    if free[side][v] < 2 {
        return 0.;
    }
    let e = follow(g, empty, e, side);
    let u = if side == 0 { g.ef[e] } else { g.et[e] };
    let point = xs[u] + if side == 0 { g.efd[e] } else { g.etd[e] };
    if again && (point - xs[v]).abs() <= span / 2. + IN_LINE {
        return (-span / 2.).js_max((span / 2.).js_min(point - xs[v]));
    }
    let mut middle = point;
    let mut width = 0.;
    if u < g.real
        && nodes[u].kind == Kind::Node
        && (if side == 0 { efk[e] } else { etk[e] }) != HELD
        && free[1 - side][u] >= 2
    {
        width = nodes[u].span.js_min(g.w[u]).js_max(0.);
        if width > 0. {
            middle = xs[u]
        }
    }
    let lo = (xs[v] - span / 2.).js_max(middle - width / 2.);
    let hi = (xs[v] + span / 2.).js_min(middle + width / 2.);
    if lo > hi + IN_LINE {
        let lean = (span / 2. - LEAN_CLEAR).js_max(0.);
        return (-lean).js_max(lean.js_min(point - xs[v]));
    }
    let place = (xs[v] * width + middle * span) / (width + span);
    lo.js_max(hi.js_min(place)) - xs[v]
}
fn settle(
    g: &mut Graph,
    nodes: &[LNode],
    xs: &[f64],
    efk: &[u8],
    etk: &[u8],
    free: &[Vec<usize>; 2],
    empty: Option<&[bool]>,
) {
    for pass in 0..6 {
        let mut moved = false;
        for v in 0..g.real {
            let span = nodes[v].span.js_min(g.w[v]);
            if span <= 0. || nodes[v].kind != Kind::Node {
                continue;
            }
            for side in 0..2 {
                let kinds = if side == 0 { etk } else { efk };
                let (ids, mut room, _) = side_ends(g, v, side, span, kinds);
                if ids.is_empty() {
                    continue;
                }
                if ids.len() == 1 {
                    room = vec![0.]
                }
                let (start, list, other) = if side == 0 {
                    (&g.pred_start, &g.pred_e, &g.ef)
                } else {
                    (&g.succ_start, &g.succ_e, &g.et)
                };
                let mut all = list[start[v]..start[v + 1]].to_vec();
                all.sort_by_key(|e| (g.pos[other[*e]], *e));
                let mut k = 0;
                let mut lo = -span / 2.;
                for i in 0..=all.len() {
                    let held = i < all.len() && kinds[all[i]] == HELD;
                    if i < all.len() && !held {
                        continue;
                    }
                    let mut size = 0;
                    for j in (0..i).rev() {
                        if kinds[all[j]] == HELD {
                            break;
                        }
                        size += 1
                    }
                    let current = |g: &Graph, e| if side == 0 { g.etd[e] } else { g.efd[e] };
                    let hi = if held {
                        current(g, all[i]) - PORT_PITCH
                    } else {
                        span / 2.
                    };
                    if size > 0 {
                        let mut sums = Vec::new();
                        let mut counts = Vec::new();
                        let mut before = 0.;
                        for j in 0..size {
                            if j > 0 {
                                before += room[k + j]
                            }
                            sums.push(
                                wanted(
                                    g,
                                    nodes,
                                    xs,
                                    free,
                                    efk,
                                    etk,
                                    empty,
                                    ids[k + j],
                                    side,
                                    v,
                                    span,
                                    pass > 0,
                                ) - before,
                            );
                            counts.push(1usize);
                            let mut together = j > 0 && room[k + j] == 0.;
                            while sums.len() > 1
                                && (together
                                    || sums[sums.len() - 2] / counts[counts.len() - 2] as f64
                                        > sums[sums.len() - 1] / counts[counts.len() - 1] as f64)
                            {
                                together = false;
                                let top = sums.pop().unwrap();
                                let pooled = counts.pop().unwrap();
                                let last = sums.len() - 1;
                                sums[last] += top;
                                counts[last] += pooled
                            }
                        }
                        let total = before;
                        let mut j = 0;
                        before = 0.;
                        for b in 0..sums.len() {
                            let level = if hi - total < lo {
                                (lo + hi - total) / 2.
                            } else {
                                lo.js_max((hi - total).js_min(sums[b] / counts[b] as f64))
                            };
                            for _ in 0..counts[b] {
                                if j > 0 {
                                    before += room[k + j]
                                }
                                let dx = if side == 0 { &mut g.etd } else { &mut g.efd };
                                if (dx[ids[k + j]] - level - before).abs() > 0.01 {
                                    moved = true
                                }
                                dx[ids[k + j]] = level + before;
                                j += 1
                            }
                        }
                        k += size
                    }
                    if held {
                        lo = current(g, all[i]) + PORT_PITCH
                    }
                }
            }
        }
        if !moved {
            break;
        }
    }
}
fn join(block: &mut [usize], members: &mut [Vec<usize>], x: usize, y: usize) {
    let mut keep = block[x];
    let mut drop = block[y];
    if keep == drop {
        return;
    }
    if members[keep].len() < members[drop].len() {
        std::mem::swap(&mut keep, &mut drop)
    }
    for v in std::mem::take(&mut members[drop]) {
        block[v] = keep;
        members[keep].push(v)
    }
}
fn snap(g: &Graph, xs: &mut [f64], empty: Option<&[bool]>, opt: &LayeredOptions) {
    let passed = |u: usize| g.kind[u] == Kind::Dummy && empty.is_some_and(|e| e[g.rank[u]]);
    let mut a = Vec::new();
    let mut b = Vec::new();
    let mut da = Vec::new();
    let mut db = Vec::new();
    for e in 0..g.ef.len() {
        if passed(g.ef[e]) {
            continue;
        }
        let f = follow(g, empty, e, 1);
        a.push(g.ef[e]);
        b.push(g.et[f]);
        da.push(g.efd[e]);
        db.push(g.etd[f])
    }
    let off = |xs: &[f64], k: usize| xs[b[k]] + db[k] - xs[a[k]] - da[k];
    let mut block: Vec<_> = (0..g.n).collect();
    let mut members: Vec<_> = (0..g.n).map(|v| vec![v]).collect();
    let mut near = Vec::new();
    for k in 0..a.len() {
        let d = off(xs, k).abs();
        if d < 0.01 {
            join(&mut block, &mut members, a[k], b[k])
        } else if d <= SNAP {
            near.push(k)
        }
    }
    near.sort_by(|x, y| {
        off(xs, *x)
            .abs()
            .total_cmp(&off(xs, *y).abs())
            .then(x.cmp(y))
    });
    for k in near {
        let d = off(xs, k);
        if d.abs() < 0.01 {
            join(&mut block, &mut members, a[k], b[k]);
            continue;
        }
        if d.abs() > SNAP || block[a[k]] == block[b[k]] {
            continue;
        }
        let upper = block[a[k]];
        let lower = block[b[k]];
        let first = if members[upper].len() <= members[lower].len() {
            upper
        } else {
            lower
        };
        for id in [first, if first == upper { lower } else { upper }] {
            let shift = if id == upper { d } else { -d };
            let free = members[id].iter().all(|&v| {
                let s = g.layer_start[g.rank[v]];
                let end = g.layer_start[g.rank[v] + 1];
                let i = s + g.pos[v];
                if i > s
                    && block[g.order[i - 1]] != id
                    && xs[v] + shift - xs[g.order[i - 1]]
                        < half_room(g, opt, v) + half_room(g, opt, g.order[i - 1]) - 0.01
                {
                    return false;
                }
                if i + 1 < end
                    && block[g.order[i + 1]] != id
                    && xs[g.order[i + 1]] - xs[v] - shift
                        < half_room(g, opt, v) + half_room(g, opt, g.order[i + 1]) - 0.01
                {
                    return false;
                }
                true
            });
            if !free {
                continue;
            }
            for &v in &members[id] {
                xs[v] += shift
            }
            join(&mut block, &mut members, a[k], b[k]);
            break;
        }
    }
}
fn empty_ranks(g: &Graph) -> Option<Vec<bool>> {
    let mut empty = vec![false; g.max_rank + 1];
    let mut any = false;
    for r in 1..g.max_rank {
        let only = g.layer_start[r + 1] > g.layer_start[r]
            && (g.layer_start[r]..g.layer_start[r + 1]).all(|i| g.kind[g.order[i]] == Kind::Dummy);
        if only {
            empty[r] = true;
            any = true
        }
    }
    any.then_some(empty)
}
fn straighten(
    g: &Graph,
    xs: &mut [f64],
    empty: &[bool],
    first_dummy: &[Option<usize>],
    span: &[usize],
    sep: f64,
    on_line: &mut [bool],
) {
    let mut want = vec![0.; xs.len()];
    let mut from = vec![0.; xs.len()];
    let mut to = vec![0.; xs.len()];
    for ei in 0..first_dummy.len() {
        let Some(d0) = first_dummy[ei] else { continue };
        let end = d0 + span[ei] - 1;
        let base = g.rank[d0];
        let mut d = d0;
        while d < end {
            if !empty[base + d - d0] {
                d += 1;
                continue;
            }
            let mut last = d;
            while last + 1 < end && empty[base + last + 1 - d0] {
                last += 1
            }
            let above = g.pred_e[g.pred_start[d]];
            let below = g.succ_e[g.succ_start[last]];
            let x0 = xs[g.ef[above]] + g.efd[above];
            let x1 = xs[g.et[below]] + g.etd[below];
            let steps = (last - d + 2) as f64;
            for k in d..=last {
                want[k] = x0 + (x1 - x0) * (k - d + 1) as f64 / steps;
                from[k] = x0;
                to[k] = x1
            }
            d = last + 1
        }
    }
    for r in 1..g.max_rank {
        if !empty[r] {
            continue;
        }
        let mut row = g.order[g.layer_start[r]..g.layer_start[r + 1]].to_vec();
        row.sort_by(|a, b| {
            from[*a]
                .total_cmp(&from[*b])
                .then(to[*a].total_cmp(&to[*b]))
                .then(a.cmp(b))
        });
        let mut i = 0;
        while i < row.len() {
            let mut j = i;
            while j < row.len()
                && (from[row[j]] - from[row[i]]).abs() < 1.
                && (to[row[j]] - to[row[i]]).abs() < 1.
            {
                j += 1
            }
            let mut twins = row[i..j].to_vec();
            twins.sort_by_key(|v| g.pos[*v]);
            for (k, &d) in twins.iter().enumerate() {
                xs[d] = want[d] + (k as f64 - (twins.len() as f64 - 1.) / 2.) * sep;
                if twins.len() == 1 || sep == 0. {
                    on_line[d] = true
                }
            }
            i = j
        }
    }
}
fn past_labels(
    g: &Graph,
    xs: &mut [f64],
    first_dummy: &[Option<usize>],
    span: &[usize],
    clear: f64,
    on_line: &mut [bool],
) {
    let mut labelled = vec![false; g.max_rank + 1];
    let mut any = false;
    for r in 1..g.max_rank {
        let mut labels = false;
        let mut others = false;
        for i in g.layer_start[r]..g.layer_start[r + 1] {
            let kind = g.kind[g.order[i]];
            if kind == Kind::Label {
                labels = true
            } else if kind != Kind::Dummy {
                others = true
            }
        }
        if labels && !others {
            labelled[r] = true;
            any = true
        }
    }
    if !any {
        return;
    }
    for ei in 0..first_dummy.len() {
        let Some(d0) = first_dummy[ei] else { continue };
        let end = d0 + span[ei] - 1;
        for d in d0..end {
            if g.kind[d] != Kind::Dummy || !labelled[g.rank[d]] {
                continue;
            }
            let mut first = d;
            let mut last = d;
            while first > d0 && on_line[first - 1] {
                first -= 1
            }
            while last + 1 < end && on_line[last + 1] {
                last += 1
            }
            let above = g.pred_e[g.pred_start[first]];
            let below = g.succ_e[g.succ_start[last]];
            let from = xs[g.ef[above]] + g.efd[above];
            let to = xs[g.et[below]] + g.etd[below];
            let lo = from.min(to) - clear;
            let hi = from.max(to) + clear;
            let r = g.rank[d];
            let free = (g.layer_start[r]..g.layer_start[r + 1]).all(|i| {
                let label = g.order[i];
                g.kind[label] != Kind::Label
                    || !(xs[label] + g.w[label] / 2. > lo && xs[label] - g.w[label] / 2. < hi)
            });
            if !free {
                continue;
            }
            on_line[d] = true;
            let steps = (last - first + 2) as f64;
            for k in first..=last {
                xs[k] = from + (to - from) * (k - first + 1) as f64 / steps
            }
        }
    }
}
fn find(group: &mut [usize], mut i: usize) -> usize {
    while group[i] != i {
        group[i] = group[group[i]];
        i = group[i]
    }
    i
}
struct TrackGeometry<'a> {
    from: &'a [f64],
    to: &'a [f64],
    lo: Vec<f64>,
    hi: Vec<f64>,
}
impl TrackGeometry<'_> {
    fn within(&self, x: f64, k: usize) -> usize {
        usize::from(x >= self.lo[k] - IN_LINE && x <= self.hi[k] + IN_LINE)
    }
    fn mates(&self, a: usize, b: usize) -> bool {
        (self.from[a] - self.from[b]).abs() < IN_LINE || (self.to[a] - self.to[b]).abs() < IN_LINE
    }
    fn overlap(&self, a: usize, b: usize) -> bool {
        self.lo[a] < self.hi[b] - IN_LINE && self.lo[b] < self.hi[a] - IN_LINE
    }
    fn clash(&self, a: usize, b: usize) -> bool {
        self.overlap(a, b) && !self.mates(a, b)
    }
    fn meet(&self, a: usize, b: usize) -> bool {
        self.overlap(a, b)
            || self.within(self.to[a], b)
                + self.within(self.from[b], a)
                + self.within(self.to[b], a)
                + self.within(self.from[a], b)
                > 0
    }
    fn crossings(&self, a: usize, b: usize) -> usize {
        self.within(self.to[a], b) + self.within(self.from[b], a)
    }
}
fn lay_tracks(
    geo: &TrackGeometry<'_>,
    list: &mut [usize],
    track: &mut [isize],
    levels: &mut [usize],
    curved: bool,
) -> usize {
    list.sort_by(|a, b| {
        (geo.hi[*b] - geo.lo[*b])
            .total_cmp(&(geo.hi[*a] - geo.lo[*a]))
            .then(geo.lo[*a].total_cmp(&geo.lo[*b]))
            .then(a.cmp(b))
    });
    let count = list.len();
    let mut above = vec![Vec::new(); count];
    let mut group: Vec<_> = (0..count).collect();
    let mut sequence = list.to_vec();
    if count <= MAX_WEIGHED {
        let mut waiting = vec![0isize; count];
        let mut after = vec![Vec::new(); count];
        for i in 0..count {
            for j in i + 1..count {
                let a = list[i];
                let b = list[j];
                if !geo.meet(a, b) {
                    continue;
                }
                let root_i = find(&mut group, i);
                let root_j = find(&mut group, j);
                group[root_i] = root_j;
                let ab = geo.crossings(a, b);
                let ba = geo.crossings(b, a);
                if ab == ba {
                    continue;
                }
                let (first, second) = if ab < ba { (i, j) } else { (j, i) };
                above[second].push(first);
                after[first].push(second);
                waiting[second] += 1
            }
        }
        let mut placed = vec![false; count];
        let mut result = Vec::new();
        for _ in 0..count {
            let mut pick = None;
            for i in 0..count {
                if placed[i] {
                    continue;
                }
                if pick.is_none_or(|p| waiting[i] < waiting[p]) {
                    pick = Some(i)
                }
                if waiting[i] == 0 {
                    break;
                }
            }
            let pick = pick.unwrap();
            placed[pick] = true;
            result.push(pick);
            for &j in &after[pick] {
                waiting[j] -= 1
            }
        }
        sequence = result.into_iter().map(|i| list[i]).collect()
    }
    let place: HashMap<_, _> = list.iter().enumerate().map(|(i, k)| (*k, i)).collect();
    let mut rows: Vec<Vec<usize>> = Vec::new();
    for k in sequence {
        let mut t = 0;
        for &i in &above[place[&k]] {
            if track[list[i]] >= t as isize {
                t = track[list[i]] as usize + 1
            }
        }
        loop {
            if rows.len() <= t {
                rows.resize_with(t + 1, Vec::new)
            }
            if rows[t].iter().all(|other| !geo.clash(k, *other)) {
                rows[t].push(k);
                break;
            }
            t += 1
        }
        track[k] = t as isize
    }
    let mut first = vec![isize::MAX; count];
    let mut last = vec![isize::MIN; count];
    for (i, k) in list.iter().enumerate() {
        let root = find(&mut group, i);
        first[root] = first[root].min(track[*k]);
        last[root] = last[root].max(track[*k])
    }
    for (i, k) in list.iter().enumerate() {
        let root = find(&mut group, i);
        levels[*k] = (last[root] - first[root] + 1) as usize;
        if curved {
            track[*k] -= first[root]
        }
    }
    rows.len()
}
fn sample_curve(geo: &TrackGeometry<'_>, k: usize, h: f64) -> [f64; 34] {
    let mut out = [0.; 34];
    let c = ease(geo.to[k] - geo.from[k], h);
    for i in 0..=16 {
        let t = i as f64 / 16.;
        let u = 1. - t;
        out[2 * i] = geo.from[k] + (geo.to[k] - geo.from[k]) * t * t * (3. - 2. * t);
        out[2 * i + 1] = h * (3. * c * t * u * u + 3. * (1. - c) * t * t * u + t * t * t)
    }
    out
}
fn curve_reach(points: &[f64; 34], line: &[f64; 34]) -> f64 {
    let mut least = f64::INFINITY;
    for i in 0..=16 {
        let px = points[2 * i];
        let py = points[2 * i + 1];
        for j in 0..16 {
            let dx = line[2 * j + 2] - line[2 * j];
            let dy = line[2 * j + 3] - line[2 * j + 1];
            let len = dx * dx + dy * dy;
            let t = (((px - line[2 * j]) * dx + (py - line[2 * j + 1]) * dy)
                / if len == 0. { 1. } else { len })
            .clamp(0., 1.);
            let d = (px - line[2 * j] - t * dx).hypot(py - line[2 * j + 1] - t * dy);
            least = least.min(d)
        }
    }
    least
}
fn assign_tracks(
    p: &mut Pieces,
    track_count: &mut [usize],
    curved: bool,
    room: &[f64],
    tall: &mut [f64],
) {
    let total = p.from.len();
    let mut by_rank: Vec<(usize, Vec<usize>)> = Vec::new();
    p.track = vec![-1; total];
    p.levels = vec![0; total];
    for k in 0..total {
        if (p.from[k] - p.to[k]).abs() < IN_LINE {
            continue;
        }
        if let Some((_, list)) = by_rank.iter_mut().find(|(r, _)| *r == p.rank[k]) {
            list.push(k)
        } else {
            by_rank.push((p.rank[k], vec![k]))
        }
    }
    let geo = TrackGeometry {
        from: &p.from,
        to: &p.to,
        lo: (0..total).map(|k| p.from[k].js_min(p.to[k])).collect(),
        hi: (0..total).map(|k| p.from[k].js_max(p.to[k])).collect(),
    };
    for (r, mut all) in by_rank {
        if !curved {
            track_count[r] = lay_tracks(&geo, &mut all, &mut p.track, &mut p.levels, curved);
            continue;
        }
        if all.len() > MAX_WEIGHED {
            continue;
        }
        let mut far: Vec<_> = all.iter().copied().filter(|k| p.wide[*k]).collect();
        lay_tracks(&geo, &mut far, &mut p.track, &mut p.levels, curved);
        let mut level: Vec<_> = far.iter().copied().filter(|k| p.levels[*k] > 1).collect();
        for i in 0..all.len() {
            for j in i + 1..all.len() {
                let a = all[i];
                let b = all[j];
                if (level.contains(&a) && level.contains(&b))
                    || !geo.clash(a, b)
                    || (p.from[a] < p.to[a]) != (p.from[b] < p.to[b])
                {
                    continue;
                }
                let h = (room[r] / 2.).js_max(room[r] - 2. * LEVEL_LEAD);
                let line_a = sample_curve(&geo, a, h);
                let line_b = sample_curve(&geo, b, h);
                let nearest = curve_reach(&line_a, &line_b).js_min(curve_reach(&line_b, &line_a));
                if nearest
                    < if p.wide[a] || p.wide[b] {
                        LEVEL_CLEAR
                    } else {
                        LEVEL_TOUCH
                    }
                {
                    if !level.contains(&a) {
                        level.push(a)
                    }
                    if !level.contains(&b) {
                        level.push(b)
                    }
                }
            }
        }
        for &k in &far {
            p.track[k] = -1;
            p.levels[k] = 0
        }
        if level.is_empty() {
            continue;
        }
        let mut crossing = 0;
        let mut alongside = 0;
        for i in 0..level.len() {
            for j in i + 1..level.len() {
                let a = level[i];
                let b = level[j];
                if !geo.overlap(a, b) {
                    continue;
                }
                if (p.from[a] < p.to[a]) != (p.from[b] < p.to[b]) {
                    crossing += 1
                } else if !geo.mates(a, b) {
                    alongside += 1
                }
            }
        }
        if crossing > alongside {
            for &k in &all {
                tall[r] = tall[r].max((geo.hi[k] - geo.lo[k]) * CROSSED_SLOPE)
            }
            continue;
        }
        let mut members: Vec<_> = all
            .into_iter()
            .filter(|k| {
                level.contains(k) || (!p.wide[*k] && level.iter().any(|j| geo.meet(*k, *j)))
            })
            .collect();
        lay_tracks(&geo, &mut members, &mut p.track, &mut p.levels, curved);
        track_count[r] = members.into_iter().map(|k| p.levels[k]).max().unwrap_or(0)
    }
}
