//! Pele architecture/layout.ts grid, collision jumps and container placement,
//! 458322a5 (MIT, kepano). This is the architecture algorithm, not layered layout.
use super::model::*;
use std::collections::HashMap;
pub const DX: [i32; 4] = [-1, 1, 0, 0];
pub const DY: [i32; 4] = [0, 0, -1, 1];
pub fn side_index(c: char) -> usize {
    match c {
        'L' => 0,
        'R' => 1,
        'T' => 2,
        _ => 3,
    }
}
pub struct Grid {
    pub col: Vec<i32>,
    pub row: Vec<i32>,
    pub group_col: Vec<i32>,
    pub group_row: Vec<i32>,
    pub group_cols: Vec<i32>,
    pub group_rows: Vec<i32>,
    pub group_parent: Vec<isize>,
    pub group_empty: Vec<bool>,
    pub node_parent: Vec<isize>,
}
struct Link {
    a: usize,
    b: usize,
    sa: usize,
    sb: usize,
    ia: usize,
    ib: usize,
}
struct Placer {
    n: usize,
    g: usize,
    parent: Vec<usize>,
    depth: Vec<usize>,
    kids: Vec<Vec<usize>>,
    links: Vec<Link>,
    hint_links: Vec<Vec<usize>>,
    edge_links: Vec<Vec<usize>>,
    x: Vec<i32>,
    y: Vec<i32>,
    w: Vec<i32>,
    h: Vec<i32>,
    placed: Vec<bool>,
    order: Vec<usize>,
    cells: HashMap<(i32, i32), usize>,
    large: Vec<usize>,
    reach: HashMap<(i32, i32, i32, i32, i32, i32), i32>,
    first: usize,
}
impl Placer {
    fn add_link(&mut self, a: usize, b: usize, sa: usize, sb: usize, hint: bool) {
        if a == b {
            return;
        }
        let (mut ia, mut ib) = (a, b);
        let (mut ca, mut cb) = (self.parent[a], self.parent[b]);
        while ca != cb {
            if self.depth[ca] >= self.depth[cb] {
                ia = self.n + ca;
                ca = self.parent[ia]
            } else {
                ib = self.n + cb;
                cb = self.parent[ib]
            }
        }
        let k = self.links.len();
        self.links.push(Link {
            a,
            b,
            sa,
            sb,
            ia,
            ib,
        });
        let lists = if hint {
            &mut self.hint_links
        } else {
            &mut self.edge_links
        };
        lists[ia].push(k);
        lists[ib].push(k)
    }
    fn anchor(&self, node: usize, item: usize) -> (i32, i32) {
        let (mut ax, mut ay) = (0, 0);
        let mut cur = node;
        while cur != item {
            ax += self.x[cur];
            ay += self.y[cur];
            cur = self.n + self.parent[cur]
        }
        (ax, ay)
    }
    fn put(&mut self, item: usize, px: i32, py: i32) {
        self.x[item] = px;
        self.y[item] = py;
        self.placed[item] = true;
        self.order.push(item);
        if self.w[item] as i64 * self.h[item] as i64 > 64 {
            self.large.push(item);
            return;
        }
        for cy in py..py + self.h[item] {
            for cx in px..px + self.w[item] {
                self.cells.insert((cx, cy), item);
            }
        }
    }
    fn overlaps(&self, item: usize, rx: i32, ry: i32, rw: i32, rh: i32) -> bool {
        self.x[item] < rx + rw
            && rx < self.x[item] + self.w[item]
            && self.y[item] < ry + rh
            && ry < self.y[item] + self.h[item]
    }
    fn collide(&self, rx: i32, ry: i32, rw: i32, rh: i32) -> Option<usize> {
        if rw as i64 * rh as i64 > 64 {
            return self.order[self.first..]
                .iter()
                .copied()
                .find(|&i| self.overlaps(i, rx, ry, rw, rh));
        }
        for cy in ry..ry + rh {
            for cx in rx..rx + rw {
                if let Some(&i) = self.cells.get(&(cx, cy)) {
                    return Some(i);
                }
            }
        }
        self.large
            .iter()
            .copied()
            .find(|&i| self.overlaps(i, rx, ry, rw, rh))
    }
    fn slide(&mut self, rx: i32, ry: i32, rw: i32, rh: i32, sx: i32, sy: i32) -> i32 {
        let key = (rx, ry, rw, rh, sx, sy);
        let mut k = *self.reach.get(&key).unwrap_or(&0);
        while let Some(hit) = self.collide(rx + k * sx, ry + k * sy, rw, rh) {
            k = if sx > 0 {
                self.x[hit] + self.w[hit] - rx
            } else if sx < 0 {
                rx + rw - self.x[hit]
            } else if sy > 0 {
                self.y[hit] + self.h[hit] - ry
            } else {
                ry + rh - self.y[hit]
            }
        }
        self.reach.insert(key, k);
        k
    }
    fn follow(&mut self, from: usize, k: usize) {
        let link = &self.links[k];
        let forward = link.ia == from;
        let to = if forward { link.ib } else { link.ia };
        if self.placed[to] {
            return;
        }
        let (sp, sq) = if forward {
            (link.sa, link.sb)
        } else {
            (link.sb, link.sa)
        };
        let (pax, pay) = self.anchor(if forward { link.a } else { link.b }, from);
        let (ax, ay) = self.anchor(if forward { link.b } else { link.a }, to);
        let (mut dx, mut dy) = (DX[sp], DY[sp]);
        let (mut shift_x, mut shift_y) = (0, 0);
        let facing = DX[sp] + DX[sq] == 0 && DY[sp] + DY[sq] == 0;
        if sp == sq {
            dx = if DX[sp] == 0 { 1 } else { 0 };
            dy = 1 - dx
        } else if !facing {
            shift_x = -DX[sq];
            shift_y = -DY[sq]
        }
        let (mut bx, mut by) = if dx != 0 {
            (
                if dx > 0 {
                    self.x[from] + self.w[from]
                } else {
                    self.x[from] - self.w[to]
                },
                self.y[from] + pay + shift_y - ay,
            )
        } else {
            (
                self.x[from] + pax + shift_x - ax,
                if dy > 0 {
                    self.y[from] + self.h[from]
                } else {
                    self.y[from] - self.h[to]
                },
            )
        };
        if facing {
            let list = &self.edge_links[to];
            if list.len() > 1 {
                let mut wanted = vec![];
                for &j in list {
                    let l = &self.links[j];
                    let to_is_b = l.ib == to;
                    let other = if to_is_b { l.ia } else { l.ib };
                    if !self.placed[other]
                        || if to_is_b { l.sa } else { l.sb } != sp
                        || if to_is_b { l.sb } else { l.sa } != sq
                    {
                        continue;
                    }
                    let (ax, ay) = self.anchor(if to_is_b { l.a } else { l.b }, other);
                    let at = if dx != 0 {
                        self.y[other] + ay
                    } else {
                        self.x[other] + ax
                    };
                    let (ax, ay) = self.anchor(if to_is_b { l.b } else { l.a }, to);
                    wanted.push(at - if dx != 0 { ay } else { ax })
                }
                if wanted.len() > 1 {
                    wanted.sort();
                    if dx != 0 {
                        by = wanted[(wanted.len() - 1) >> 1]
                    } else {
                        bx = wanted[(wanted.len() - 1) >> 1]
                    }
                }
            }
            let px = if dx == 0 { 1 } else { 0 };
            let py = 1 - px;
            let forth = self.slide(bx, by, self.w[to], self.h[to], px, py);
            let back = if forth == 0 {
                0
            } else {
                self.slide(bx, by, self.w[to], self.h[to], -px, -py)
            };
            let near = if forth <= back { forth } else { -back };
            bx += px * near;
            by += py * near
        } else {
            let far = self.slide(bx, by, self.w[to], self.h[to], dx, dy);
            bx += dx * far;
            by += dy * far
        }
        self.put(to, bx, by)
    }
    fn layout(&mut self, c: usize) {
        let items = self.kids[c].clone();
        if items.is_empty() {
            return;
        }
        self.order.clear();
        let mut starts = vec![];
        for start in items {
            if self.placed[start] {
                continue;
            }
            self.first = self.order.len();
            starts.push(self.first);
            self.cells.clear();
            self.large.clear();
            self.reach.clear();
            self.put(start, 0, 0);
            let (mut hint_at, mut edge_at, mut link_at) = (self.first, self.first, 0);
            loop {
                if hint_at < self.order.len() {
                    let from = self.order[hint_at];
                    hint_at += 1;
                    for k in self.hint_links[from].clone() {
                        self.follow(from, k)
                    }
                    continue;
                }
                if edge_at >= self.order.len() {
                    break;
                }
                let from = self.order[edge_at];
                if link_at >= self.edge_links[from].len() {
                    edge_at += 1;
                    link_at = 0;
                    continue;
                }
                let k = self.edge_links[from][link_at];
                link_at += 1;
                self.follow(from, k)
            }
        }
        starts.push(self.order.len());
        let count = starts.len() - 1;
        let mut min_x = vec![0; count];
        let mut min_y = vec![0; count];
        let mut part_w = vec![0; count];
        let mut part_h = vec![0; count];
        let (mut area, mut widest) = (0f64, 0i32);
        for p in 0..count {
            let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
            for &item in &self.order[starts[p]..starts[p + 1]] {
                x0 = x0.min(self.x[item]);
                y0 = y0.min(self.y[item]);
                x1 = x1.max(self.x[item] + self.w[item]);
                y1 = y1.max(self.y[item] + self.h[item])
            }
            min_x[p] = x0;
            min_y[p] = y0;
            part_w[p] = x1 - x0;
            part_h[p] = y1 - y0;
            area += part_w[p] as f64 * part_h[p] as f64;
            widest = widest.max(part_w[p])
        }
        let limit = widest.max((area * 2.).sqrt().ceil() as i32);
        let (mut cx, mut cy, mut shelf, mut width) = (0, 0, 0, 0);
        for p in 0..count {
            if cx > 0 && cx + part_w[p] > limit {
                cy += shelf;
                cx = 0;
                shelf = 0
            }
            for &item in &self.order[starts[p]..starts[p + 1]] {
                self.x[item] += cx - min_x[p];
                self.y[item] += cy - min_y[p]
            }
            cx += part_w[p];
            shelf = shelf.max(part_h[p]);
            width = width.max(cx)
        }
        if c < self.g {
            self.w[self.n + c] = width;
            self.h[self.n + c] = cy + shelf
        }
    }
}
pub fn place(
    nodes: &[(String, ArchNode)],
    groups: &[(String, ArchGroup)],
    edges: &[ArchEdge],
    hints: &[ArchLayoutHint],
) -> Grid {
    let n = nodes.len();
    let g = groups.len();
    let root = g;
    let mut p = Placer {
        n,
        g,
        parent: vec![root; n + g],
        depth: vec![0; g + 1],
        kids: vec![vec![]; g + 1],
        links: vec![],
        hint_links: vec![vec![]; n + g],
        edge_links: vec![vec![]; n + g],
        x: vec![0; n + g],
        y: vec![0; n + g],
        w: vec![1; n + g],
        h: vec![1; n + g],
        placed: vec![false; n + g],
        order: vec![],
        cells: HashMap::new(),
        large: vec![],
        reach: HashMap::new(),
        first: 0,
    };
    let mut group_index = HashMap::new();
    for (i, (_, group)) in groups.iter().enumerate() {
        group_index.insert(group.id.as_str(), i);
        p.depth[i] = 1;
    }
    // The parent always precedes a child; resolve independently of the inserted index.
    for (i, (_, group)) in groups.iter().enumerate() {
        if let Some(parent) = group
            .parent
            .as_deref()
            .and_then(|id| group_index.get(id))
            .copied()
            .filter(|parent| *parent != i)
        {
            p.parent[n + i] = parent;
            p.depth[i] = p.depth[parent] + 1
        }
    }
    let mut node_index = HashMap::new();
    for (i, (_, node)) in nodes.iter().enumerate() {
        node_index.insert(node.id.as_str(), i);
        if let Some(&parent) = node.parent.as_deref().and_then(|id| group_index.get(id)) {
            p.parent[i] = parent
        }
        p.kids[p.parent[i]].push(i)
    }
    for i in 0..g {
        p.kids[p.parent[n + i]].push(n + i)
    }
    for hint in hints {
        let row = hint.direction == "row";
        for pair in hint.members.windows(2) {
            if let (Some(&a), Some(&b)) = (
                node_index.get(pair[0].as_str()),
                node_index.get(pair[1].as_str()),
            ) {
                p.add_link(a, b, if row { 1 } else { 3 }, if row { 0 } else { 2 }, true)
            }
        }
    }
    for like in [false, true] {
        for e in edges {
            if (e.lhs_dir == e.rhs_dir) != like {
                continue;
            }
            if let (Some(&a), Some(&b)) = (
                node_index.get(e.lhs_id.as_str()),
                node_index.get(e.rhs_id.as_str()),
            ) {
                p.add_link(a, b, side_index(e.lhs_dir), side_index(e.rhs_dir), false)
            }
        }
    }
    for c in (0..g).rev() {
        p.layout(c)
    }
    p.layout(root);
    let mut group_col = vec![0; g];
    let mut group_row = vec![0; g];
    let mut group_parent = vec![-1; g];
    let mut group_empty = vec![false; g];
    for i in 0..g {
        let parent = p.parent[n + i];
        group_parent[i] = if parent == root { -1 } else { parent as isize };
        group_col[i] = p.x[n + i] + if parent == root { 0 } else { group_col[parent] };
        group_row[i] = p.y[n + i] + if parent == root { 0 } else { group_row[parent] };
        group_empty[i] = p.kids[i].is_empty()
    }
    let mut col = vec![0; n];
    let mut row = vec![0; n];
    let mut node_parent = vec![-1; n];
    for i in 0..n {
        let parent = p.parent[i];
        node_parent[i] = if parent == root { -1 } else { parent as isize };
        col[i] = p.x[i] + if parent == root { 0 } else { group_col[parent] };
        row[i] = p.y[i] + if parent == root { 0 } else { group_row[parent] }
    }
    Grid {
        col,
        row,
        group_col,
        group_row,
        group_cols: p.w[n..].into(),
        group_rows: p.h[n..].into(),
        group_parent,
        group_empty,
        node_parent,
    }
}
