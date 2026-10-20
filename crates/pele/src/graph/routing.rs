//! Obstacle visibility routing adapted from astviewer orthogonal-router.ts.
use super::{
    geometry::simplify,
    validation::{invalid, Budget},
    *,
};
use crate::compat::PeleError;
use std::collections::BTreeMap;
pub(super) const STEP: [(f64, f64); 4] = [(1., 0.), (-1., 0.), (0., 1.), (0., -1.)];
#[derive(Clone, Copy)]
pub(super) struct Port {
    pub(super) p: Point,
    pub(super) dir: usize,
    pub(super) price: f64,
}
impl Port {
    fn stub(self) -> Point {
        self.p.shift(STEP[self.dir].0 * 12., STEP[self.dir].1 * 12.)
    }
}
fn ports(r: Rect, slot: usize, total: usize, incoming: bool) -> Vec<Port> {
    let fraction = (slot + 1) as f64 / (total + 1) as f64;
    let x = r.x + r.width * (0.2 + 0.6 * fraction);
    let y = r.y + r.height * (0.2 + 0.6 * fraction);
    [
        (
            Point {
                x,
                y: r.y + r.height,
            },
            2,
        ),
        (Point { x, y: r.y }, 3),
        (
            Point {
                x: r.x + r.width,
                y,
            },
            0,
        ),
        (Point { x: r.x, y }, 1),
    ]
    .into_iter()
    .map(|(p, out)| Port {
        p,
        dir: if incoming { out ^ 1 } else { out },
        price: if (incoming && out == 3) || (!incoming && out == 2) {
            0.
        } else {
            18.
        },
    })
    .collect()
}
fn candidate(s: Port, e: Port, variant: usize) -> Vec<Point> {
    let a = s.stub();
    let out = Port {
        dir: e.dir ^ 1,
        ..e
    };
    let z = out.stub();
    let middle = match variant {
        0 => vec![Point { x: a.x, y: z.y }],
        1 => vec![Point { x: z.x, y: a.y }],
        2 => vec![
            Point {
                x: a.x,
                y: (a.y + z.y) / 2.,
            },
            Point {
                x: z.x,
                y: (a.y + z.y) / 2.,
            },
        ],
        _ => vec![
            Point {
                x: (a.x + z.x) / 2.,
                y: a.y,
            },
            Point {
                x: (a.x + z.x) / 2.,
                y: z.y,
            },
        ],
    };
    simplify([vec![s.p, a], middle, vec![z, e.p]].concat())
}
pub(super) fn clear(points: &[Point], boxes: &[Rect], b: &mut Budget) -> Result<bool, PeleError> {
    b.spend(points.len() * boxes.len() + 1)?;
    Ok(points
        .windows(2)
        .all(|p| !boxes.iter().any(|r| r.crosses(p[0], p[1]))))
}
fn fast(
    starts: &[Port],
    ends: &[Port],
    boxes: &[Rect],
    b: &mut Budget,
) -> Result<Option<Vec<Point>>, PeleError> {
    let mut best = None;
    let mut cost = f64::INFINITY;
    for (s, e) in starts.iter().flat_map(|s| ends.iter().map(move |e| (s, e))) {
        if s.p == e.p {
            continue;
        }
        for variant in 0..4 {
            let p = candidate(*s, *e, variant);
            let c = p.windows(2).map(|p| p[0].distance(p[1])).sum::<f64>()
                + (p.len() - 2) as f64 * 30.
                + s.price
                + e.price;
            if c < cost && clear(&p, boxes, b)? {
                best = Some(p);
                cost = c;
            }
        }
    }
    Ok(best)
}
pub(super) fn route(g: &Graph, s: &mut Scene, b: &mut Budget) -> Result<(), PeleError> {
    let by_id = s
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.node.id.clone(), i))
        .collect::<BTreeMap<_, _>>();
    let mut count = BTreeMap::<String, usize>::new();
    for e in &g.edges {
        *count.entry(e.source.clone()).or_default() += 1;
        *count.entry(e.target.clone()).or_default() += 1;
    }
    let mut used = BTreeMap::<String, usize>::new();
    for e in &g.edges {
        let source = by_id[&e.source];
        let target = by_id[&e.target];
        let from = slots(&mut used, &count, &s.nodes[source], false);
        let mut to = slots(&mut used, &count, &s.nodes[target], true);
        if source == target {
            to.retain(|p| p.dir == 2);
        }
        let obstacles = s
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| {
                if i == source || i == target {
                    n.rect
                } else {
                    n.rect.expand(8.)
                }
            })
            .collect::<Vec<_>>();
        let points = match fast(&from, &to, &obstacles, b)? {
            Some(p) => p,
            None => super::search::search(&from, &to, &obstacles, b)?
                .ok_or_else(|| invalid(format!("no obstacle-free route for {}", e.id)))?,
        };
        s.edges.push(Route {
            edge: e.clone(),
            points,
            label_box: Rect::default(),
        });
    }
    Ok(())
}
fn slots(
    used: &mut BTreeMap<String, usize>,
    count: &BTreeMap<String, usize>,
    n: &PlacedNode,
    incoming: bool,
) -> Vec<Port> {
    let slot = used.entry(n.node.id.clone()).or_default();
    let result = ports(n.rect, *slot, count[&n.node.id], incoming);
    *slot += 1;
    result
}
