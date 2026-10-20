use super::{
    geometry::simplify,
    routing::{clear, Port, STEP},
    validation::Budget,
    Point, Rect,
};
use crate::compat::PeleError;
use std::{
    cmp::Reverse,
    collections::{BTreeMap, BinaryHeap},
};
#[derive(Clone, Copy)]
struct Visit {
    cost: f64,
    state: usize,
}
struct Grid {
    xs: Vec<f64>,
    ys: Vec<f64>,
}
impl Grid {
    fn new(starts: &[Port], ends: &[Port], boxes: &[Rect]) -> Self {
        let mut xs = Vec::new();
        let mut ys = Vec::new();
        for p in starts.iter().chain(ends) {
            xs.push(p.p.x);
            ys.push(p.p.y);
        }
        for r in boxes {
            xs.extend([r.x - 8., r.x + r.width + 8.]);
            ys.extend([r.y - 8., r.y + r.height + 8.]);
        }
        xs.sort_by(f64::total_cmp);
        xs.dedup();
        ys.sort_by(f64::total_cmp);
        ys.dedup();
        Self { xs, ys }
    }
    fn cell(&self, p: Point) -> usize {
        self.xs.binary_search_by(|x| x.total_cmp(&p.x)).unwrap() * self.ys.len()
            + self.ys.binary_search_by(|y| y.total_cmp(&p.y)).unwrap()
    }
    fn point(&self, cell: usize) -> Point {
        Point {
            x: self.xs[cell / self.ys.len()],
            y: self.ys[cell % self.ys.len()],
        }
    }
    fn next(&self, cell: usize, dir: usize) -> Option<usize> {
        let x = cell / self.ys.len();
        let y = cell % self.ys.len();
        let (dx, dy) = STEP[dir];
        let (nx, ny) = (x as isize + dx as isize, y as isize + dy as isize);
        if nx < 0 || ny < 0 || nx as usize >= self.xs.len() || ny as usize >= self.ys.len() {
            None
        } else {
            Some(nx as usize * self.ys.len() + ny as usize)
        }
    }
}

struct Walker {
    grid: Grid,
    distance: Vec<f64>,
    previous: Vec<usize>,
    origin: Vec<Point>,
    queue: BinaryHeap<Reverse<(u64, usize)>>,
}
impl Walker {
    fn new(grid: Grid, b: &mut Budget) -> Result<Self, PeleError> {
        let size = grid.xs.len() * grid.ys.len() * 4;
        b.spend(size)?;
        Ok(Self {
            grid,
            distance: vec![f64::INFINITY; size],
            previous: vec![usize::MAX; size],
            origin: vec![Point::default(); size],
            queue: BinaryHeap::new(),
        })
    }
    fn seed(&mut self, s: Port, boxes: &[Rect], b: &mut Budget) -> Result<(), PeleError> {
        let cell = self.grid.cell(s.p);
        let Some(next) = self.grid.next(cell, s.dir) else {
            return Ok(());
        };
        let p = self.grid.point(next);
        if !clear(&[s.p, p], boxes, b)? {
            return Ok(());
        }
        let state = next * 4 + s.dir;
        let cost = s.price + s.p.distance(p);
        if cost < self.distance[state] {
            self.distance[state] = cost;
            self.origin[state] = s.p;
            self.queue.push(Reverse((cost.to_bits(), state)));
        }
        Ok(())
    }
    fn advance(&mut self, v: Visit, boxes: &[Rect], b: &mut Budget) -> Result<(), PeleError> {
        for dir in 0..4 {
            if dir == ((v.state % 4) ^ 1) {
                continue;
            }
            let Some(cell) = self.grid.next(v.state / 4, dir) else {
                continue;
            };
            let a = self.grid.point(v.state / 4);
            let z = self.grid.point(cell);
            if !clear(&[a, z], boxes, b)? {
                continue;
            }
            let state = cell * 4 + dir;
            let cost = v.cost + a.distance(z) + if v.state % 4 == dir { 0. } else { 30. };
            if cost < self.distance[state] {
                self.distance[state] = cost;
                self.previous[state] = v.state;
                self.origin[state] = self.origin[v.state];
                self.queue.push(Reverse((cost.to_bits(), state)));
            }
        }
        Ok(())
    }
    fn walk(
        &mut self,
        goals: &BTreeMap<usize, f64>,
        boxes: &[Rect],
        b: &mut Budget,
    ) -> Result<Option<usize>, PeleError> {
        let mut best = None;
        let mut price = f64::INFINITY;
        while let Some(Reverse((bits, state))) = self.queue.pop() {
            let v = Visit {
                cost: f64::from_bits(bits),
                state,
            };
            b.spend(1)?;
            if v.cost > self.distance[v.state] {
                continue;
            }
            if v.cost >= price {
                break;
            }
            if let Some(extra) = goals.get(&v.state) {
                if v.cost + extra < price {
                    best = Some(v.state);
                    price = v.cost + extra;
                }
            }
            self.advance(v, boxes, b)?;
        }
        Ok(best)
    }
    fn path(&self, mut at: usize) -> Vec<Point> {
        let mut points = vec![self.grid.point(at / 4)];
        while self.previous[at] != usize::MAX {
            at = self.previous[at];
            points.push(self.grid.point(at / 4));
        }
        points.push(self.origin[at]);
        points.reverse();
        simplify(points)
    }
}
pub(super) fn search(
    starts: &[Port],
    ends: &[Port],
    boxes: &[Rect],
    b: &mut Budget,
) -> Result<Option<Vec<Point>>, PeleError> {
    let mut walker = Walker::new(Grid::new(starts, ends, boxes), b)?;
    for s in starts {
        walker.seed(*s, boxes, b)?;
    }
    let goals = ends
        .iter()
        .map(|p| (walker.grid.cell(p.p) * 4 + p.dir, p.price))
        .collect();
    Ok(walker.walk(&goals, boxes, b)?.map(|at| walker.path(at)))
}
