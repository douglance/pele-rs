//! Pele block/render.ts outline intersections, bounded obstacle routing and arc paths.
use super::{
    arrows,
    render::{Kind, View},
};
use crate::compat::svg::{edges::EdgePath, num, shapes::shape_inset};
use std::f64::consts::{FRAC_1_SQRT_2, FRAC_PI_2, FRAC_PI_4, PI};
fn round(v: &View) -> bool {
    v.kind == Kind::Leaf && matches!(v.shape, "circle" | "dbl-circ")
}
fn edge_y(v: &View, x: f64, sign: f64) -> f64 {
    let hw = v.bw / 2.;
    let hh = v.bh / 2.;
    let dx = (x - v.x).abs();
    let reach = if round(v) {
        (hw * hw - dx * dx).max(0.).sqrt()
    } else if v.kind == Kind::Leaf && v.shape == "diam" {
        hh * (1. - dx / hw).max(0.)
    } else if v.kind == Kind::Leaf {
        hh - shape_inset(v.shape, v.bw, v.bh, if sign > 0. { 2 } else { 0 }, 0.)
    } else if v.kind == Kind::Arrow && arrows::axis(v.mask) == 'x' {
        hh - arrows::OVERHANG
    } else {
        hh
    };
    v.y + sign * reach
}
fn edge_x(v: &View, y: f64, sign: f64) -> f64 {
    let hw = v.bw / 2.;
    let hh = v.bh / 2.;
    let dy = (y - v.y).abs();
    let reach = if round(v) {
        (hh * hh - dy * dy).max(0.).sqrt()
    } else if v.kind == Kind::Leaf && v.shape == "diam" {
        hw * (1. - dy / hh).max(0.)
    } else if v.kind == Kind::Leaf {
        hw - shape_inset(v.shape, v.bw, v.bh, if sign > 0. { 1 } else { 3 }, 0.)
    } else if v.kind == Kind::Arrow && arrows::axis(v.mask) == 'y' {
        hw - arrows::OVERHANG
    } else {
        hw
    };
    v.x + sign * reach
}
fn border(v: &View, tx: f64, ty: f64, out: &mut Vec<f64>) {
    let dx = tx - v.x;
    let dy = ty - v.y;
    let hw = v.bw / 2.;
    let hh = v.bh / 2.;
    let nonzero = |n: f64| if n == 0. || n.is_nan() { 1. } else { n };
    let mut k = if round(v) {
        hw / nonzero(dx.hypot(dy))
    } else if v.kind == Kind::Leaf && v.shape == "diam" {
        1. / nonzero(dx.abs() / hw + dy.abs() / hh)
    } else {
        (if dx == 0. {
            f64::INFINITY
        } else {
            hw / dx.abs()
        })
        .min(if dy == 0. {
            f64::INFINITY
        } else {
            hh / dy.abs()
        })
    };
    if k == f64::INFINITY {
        k = 0.;
    }
    out.extend([v.x + dx * k, v.y + dy * k]);
}
fn crosses(x1: f64, y1: f64, x2: f64, y2: f64, v: &View) -> bool {
    let l = v.x - v.bw / 2. + 1.;
    let r = v.x + v.bw / 2. - 1.;
    let t = v.y - v.bh / 2. + 1.;
    let b = v.y + v.bh / 2. - 1.;
    if (x1 < l && x2 < l) || (x1 > r && x2 > r) || (y1 < t && y2 < t) || (y1 > b && y2 > b) {
        return false;
    }
    if x1 == x2 || y1 == y2 {
        return true;
    }
    let dx = x2 - x1;
    let dy = y2 - y1;
    let side = |x: f64, y: f64| {
        let n = dx * (y - y1) - dy * (x - x1);
        if n == 0. {
            0.
        } else {
            n.signum()
        }
    };
    let first = side(l, t);
    first == 0. || side(r, t) != first || side(r, b) != first || side(l, b) != first
}
fn hits(p: &[f64], s: usize, t: usize, obstacles: &[usize], order: &[View]) -> usize {
    let mut count = 0;
    for &i in obstacles {
        if i == s || i == t {
            continue;
        }
        for k in (0..p.len() - 2).step_by(2) {
            if crosses(p[k], p[k + 1], p[k + 2], p[k + 3], &order[i]) {
                count += 1;
                break;
            }
        }
    }
    count
}
pub(super) fn route(
    si: usize,
    ti: usize,
    shift: f64,
    obstacles: Option<&[usize]>,
    order: &[View],
) -> Vec<f64> {
    let s = &order[si];
    let t = &order[ti];
    let sl = s.x - s.bw / 2.;
    let sr = s.x + s.bw / 2.;
    let st = s.y - s.bh / 2.;
    let sb = s.y + s.bh / 2.;
    let tl = t.x - t.bw / 2.;
    let tr = t.x + t.bw / 2.;
    let tt = t.y - t.bh / 2.;
    let tb = t.y + t.bh / 2.;
    let x1 = sl.max(tl);
    let x2 = sr.min(tr);
    let y1 = st.max(tt);
    let y2 = sb.min(tb);
    let over_x = x2 - x1 > 6.;
    let over_y = y2 - y1 > 6.;
    if over_x && over_y {
        let inner_s = s.bw * s.bh <= t.bw * t.bh;
        let (inner, outer) = if inner_s { (s, t) } else { (t, s) };
        let up = inner.y - inner.bh / 2. - (outer.y - outer.bh / 2.);
        let down = outer.y + outer.bh / 2. - (inner.y + inner.bh / 2.);
        let left = inner.x - inner.bw / 2. - (outer.x - outer.bw / 2.);
        let right = outer.x + outer.bw / 2. - (inner.x + inner.bw / 2.);
        let least = up.min(down).min(left).min(right);
        let p = if least == down {
            vec![
                inner.x,
                edge_y(inner, inner.x, 1.),
                inner.x,
                outer.y + outer.bh / 2.,
            ]
        } else if least == up {
            vec![
                inner.x,
                edge_y(inner, inner.x, -1.),
                inner.x,
                outer.y - outer.bh / 2.,
            ]
        } else if least == right {
            vec![
                edge_x(inner, inner.y, 1.),
                inner.y,
                outer.x + outer.bw / 2.,
                inner.y,
            ]
        } else {
            vec![
                edge_x(inner, inner.y, -1.),
                inner.y,
                outer.x - outer.bw / 2.,
                inner.y,
            ]
        };
        return if inner_s {
            p
        } else {
            vec![p[2], p[3], p[0], p[1]]
        };
    }
    if over_x {
        let margin = 6_f64.min((x2 - x1) / 2.);
        let x = ((x1 + x2) / 2. + shift).max(x1 + margin).min(x2 - margin);
        let down = if t.y > s.y { 1. } else { -1. };
        return vec![x, edge_y(s, x, down), x, edge_y(t, x, -down)];
    }
    if over_y {
        let margin = 6_f64.min((y2 - y1) / 2.);
        let y = ((y1 + y2) / 2. + shift).max(y1 + margin).min(y2 - margin);
        let right = if t.x > s.x { 1. } else { -1. };
        return vec![edge_x(s, y, right), y, edge_x(t, y, -right), y];
    }
    let right = if t.x > s.x { 1. } else { -1. };
    let down = if t.y > s.y { 1. } else { -1. };
    let across = vec![
        edge_x(s, s.y, right),
        s.y,
        t.x,
        s.y,
        t.x,
        edge_y(t, t.x, -down),
    ];
    let along = vec![
        s.x,
        edge_y(s, s.x, down),
        s.x,
        t.y,
        edge_x(t, t.y, -right),
        t.y,
    ];
    let wide = (t.x - s.x).abs() >= (t.y - s.y).abs();
    let (mut best, other) = if wide {
        (across, along)
    } else {
        (along, across)
    };
    let Some(obstacles) = obstacles else {
        return best;
    };
    let mut least = hits(&best, si, ti, obstacles, order);
    if least == 0 {
        return best;
    }
    let count = hits(&other, si, ti, obstacles, order);
    if count < least {
        best = other;
        least = count;
    }
    if least == 0 {
        return best;
    }
    let mut direct = vec![];
    border(s, t.x, t.y, &mut direct);
    border(t, s.x, s.y, &mut direct);
    if hits(&direct, si, ti, obstacles, order) < least {
        direct
    } else {
        best
    }
}
pub(super) fn trace(p: &[f64], mut start: f64, mut end: f64) -> EdgePath {
    let n = p.len();
    let sx = p[0];
    let sy = p[1];
    let ex = p[n - 2];
    let ey = p[n - 1];
    let mut ax = p[2] - sx;
    let mut ay = p[3] - sy;
    let first = ax.hypot(ay);
    let f = if first == 0. { 1. } else { first };
    ax /= f;
    ay /= f;
    let mut bx = ex - p[n - 4];
    let mut by = ey - p[n - 3];
    let last = bx.hypot(by);
    let l = if last == 0. { 1. } else { last };
    bx /= l;
    by /= l;
    if n == 4 && start + end > first {
        let scale = first / (start + end);
        start *= scale;
        end *= scale;
    }
    start = start.min(first);
    end = end.min(last);
    let mut d = format!("M{},{}", num(sx + ax * start), num(sy + ay * start));
    if n == 6 {
        let r = 0_f64.max(8_f64.min(first - start).min(last - end));
        d += &format!(
            "L{},{}Q{},{} {},{}",
            num(p[2] - ax * r),
            num(p[3] - ay * r),
            num(p[2]),
            num(p[3]),
            num(p[2] + bx * r),
            num(p[3] + by * r)
        );
    }
    d += &format!("L{},{}", num(ex - bx * end), num(ey - by * end));
    EdgePath {
        d,
        sx,
        sy,
        sdx: -ax,
        sdy: -ay,
        ex,
        ey,
        edx: bx,
        edy: by,
    }
}
pub(super) fn loop_arc(v: &View, k: usize) -> (f64, f64, f64, f64, f64) {
    let hw = v.bw / 2.;
    let hh = v.bh / 2.;
    let r = (10. + 5. * (k as f64 - 1.)).min(hw).min(hh).max(1.);
    if round(v) {
        let px = v.x + hw * FRAC_1_SQRT_2;
        let py = v.y - hw * FRAC_1_SQRT_2;
        let half = 2. * (r / (2. * hw)).min(1.).asin();
        let angle = |at: f64| (v.y - hw * at.sin() - py).atan2(v.x + hw * at.cos() - px);
        return (px, py, r, angle(FRAC_PI_4 + half), angle(FRAC_PI_4 - half));
    }
    if v.kind == Kind::Leaf && v.shape == "diam" {
        let slope = hh.atan2(hw);
        return (v.x + hw / 2., v.y - hh / 2., r, slope + PI, slope);
    }
    (v.x + hw, v.y - hh, r, PI, FRAC_PI_2)
}
pub(super) fn loop_path(v: &View, k: usize, start: f64, end: f64) -> EdgePath {
    let (px, py, r, a1, a2) = loop_arc(v, k);
    let from = a1 + start / r;
    let to = a2 - end / r;
    let mut sweep = (to - from) % (2. * PI);
    if sweep <= 0. {
        sweep += 2. * PI;
    }
    let x1 = px + r * from.cos();
    let y1 = py + r * from.sin();
    let x2 = px + r * to.cos();
    let y2 = py + r * to.sin();
    let sx = px + r * a1.cos();
    let sy = py + r * a1.sin();
    let ex = px + r * a2.cos();
    let ey = py + r * a2.sin();
    let sl = (sx - x1).hypot(sy - y1);
    let el = (ex - x2).hypot(ey - y2);
    EdgePath {
        d: format!(
            "M{},{}A{},{} 0 {} 1 {},{}",
            num(x1),
            num(y1),
            num(r),
            num(r),
            if sweep > PI { 1 } else { 0 },
            num(x2),
            num(y2)
        ),
        sx,
        sy,
        sdx: if sl > 0. { (sx - x1) / sl } else { a1.sin() },
        sdy: if sl > 0. { (sy - y1) / sl } else { -a1.cos() },
        ex,
        ey,
        edx: if el > 0. { (ex - x2) / el } else { -a2.sin() },
        edy: if el > 0. { (ey - y2) / el } else { a2.cos() },
    }
}
pub(super) fn label_spot(
    p: &[f64],
    at: usize,
    w: f64,
    h: f64,
    s: usize,
    t: usize,
    obstacles: Option<&[usize]>,
    order: &[View],
) -> f64 {
    let Some(obstacles) = obstacles else {
        return 0.5;
    };
    for spot in [0.5, 0.3, 0.7, 0.15, 0.85] {
        let x = p[at] + (p[at + 2] - p[at]) * spot;
        let y = p[at + 1] + (p[at + 3] - p[at + 1]) * spot;
        let clear = obstacles.iter().all(|&i| {
            i == s
                || i == t
                || (x - order[i].x).abs() >= (w + order[i].bw) / 2.
                || (y - order[i].y).abs() >= (h + order[i].bh) / 2.
        });
        if clear {
            return spot;
        }
    }
    0.5
}
