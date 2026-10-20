use super::{Point, Rect};
impl Point {
    pub(super) fn finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
    pub(super) fn distance(self, b: Self) -> f64 {
        (self.x - b.x).abs() + (self.y - b.y).abs()
    }
    pub(super) fn shift(self, x: f64, y: f64) -> Self {
        Self {
            x: self.x + x,
            y: self.y + y,
        }
    }
}
impl Rect {
    pub(super) fn valid(self) -> bool {
        [self.x, self.y, self.width, self.height]
            .iter()
            .all(|n| n.is_finite())
            && self.width >= 0.
            && self.height >= 0.
    }
    pub(super) fn crosses(self, a: Point, b: Point) -> bool {
        let epsilon = 0.000001;
        if a.y == b.y {
            self.y + epsilon < a.y
                && a.y < self.y + self.height - epsilon
                && a.x.min(b.x) < self.x + self.width - epsilon
                && a.x.max(b.x) > self.x + epsilon
        } else {
            self.x + epsilon < a.x
                && a.x < self.x + self.width - epsilon
                && a.y.min(b.y) < self.y + self.height - epsilon
                && a.y.max(b.y) > self.y + epsilon
        }
    }
    pub(super) fn overlaps(self, b: Self) -> bool {
        self.x < b.x + b.width
            && self.x + self.width > b.x
            && self.y < b.y + b.height
            && self.y + self.height > b.y
    }
    pub(super) fn expand(self, m: f64) -> Self {
        Self {
            x: self.x - m,
            y: self.y - m,
            width: self.width + 2. * m,
            height: self.height + 2. * m,
        }
    }
}
pub(super) fn simplify(points: Vec<Point>) -> Vec<Point> {
    let mut out: Vec<Point> = Vec::new();
    for p in points {
        if out.last() == Some(&p) {
            continue;
        }
        if out.len() > 1 {
            let a = out[out.len() - 2];
            let b = out[out.len() - 1];
            if forward(a, b, p) {
                out.pop();
            }
        }
        out.push(p);
    }
    out
}

fn forward(a: Point, b: Point, p: Point) -> bool {
    (a.x == b.x && b.x == p.x && (b.y - a.y) * (p.y - b.y) >= 0.)
        || (a.y == b.y && b.y == p.y && (b.x - a.x) * (p.x - b.x) >= 0.)
}
