//! Crossing jumps follow astviewer line-jumps.ts; corners use a small orthogonal radius.
use super::Point;
fn at(p: Point) -> String {
    format!("{:.2},{:.2}", p.x, p.y)
}
fn toward(a: Point, b: Point, d: f64) -> Point {
    let length = a.distance(b);
    if length == 0. {
        a
    } else {
        Point {
            x: a.x + (b.x - a.x) * d / length,
            y: a.y + (b.y - a.y) * d / length,
        }
    }
}
pub(super) fn path(points: &[Point], others: &[&[Point]]) -> String {
    if points.is_empty() {
        return String::new();
    }
    let mut out = format!("M{}", at(points[0]));
    let mut start = points[0];
    for i in 1..points.len() {
        let corner = points[i];
        let radius = if i + 1 < points.len() {
            4_f64
                .min(points[i - 1].distance(corner) / 2.)
                .min(corner.distance(points[i + 1]) / 2.)
        } else {
            0.
        };
        let end = toward(corner, points[i - 1], radius);
        jumps(&mut out, start, end, others);
        if radius > 0. {
            let next = toward(corner, points[i + 1], radius);
            out.push_str(&format!("Q{} {}", at(corner), at(next)));
            start = next;
        } else {
            start = end;
        }
    }
    out
}
fn jumps(out: &mut String, a: Point, z: Point, others: &[&[Point]]) {
    if a.y == z.y {
        let dir = (z.x - a.x).signum();
        let mut crossings = others
            .iter()
            .flat_map(|p| p.windows(2))
            .filter(|p| {
                p[0].x == p[1].x && p[0].y.min(p[1].y) < a.y - 0.5 && p[0].y.max(p[1].y) > a.y + 0.5
            })
            .map(|p| p[0].x)
            .filter(|x| (*x - a.x) * dir > 5. && (z.x - *x) * dir > 5.)
            .collect::<Vec<_>>();
        crossings.sort_by(|x, y| ((x - y) * dir).total_cmp(&0.));
        crossings.dedup();
        let mut last = a.x;
        for x in crossings {
            if (x - last) * dir < 9. {
                continue;
            }
            out.push_str(&format!(
                "L{}A4,4 0 0 {} {}",
                at(Point {
                    x: x - 4. * dir,
                    y: a.y
                }),
                usize::from(dir > 0.),
                at(Point {
                    x: x + 4. * dir,
                    y: a.y
                })
            ));
            last = x + 4. * dir;
        }
    }
    out.push_str(&format!("L{}", at(z)));
}
