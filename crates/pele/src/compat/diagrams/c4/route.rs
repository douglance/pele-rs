//! Pele c4/route.ts geometric outline intersections and obstacle routing, 458322a5 (MIT, kepano).
use super::model::ElementRef;
use crate::compat::svg::num;
use std::collections::HashMap;
#[derive(Clone, Debug)]
pub(super) struct Box {
    pub id: ElementRef,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub form: String,
}
impl Box {
    pub fn new(id: ElementRef, form: String, w: f64, h: f64) -> Self {
        Self {
            id,
            x: 0.,
            y: 0.,
            w,
            h,
            form,
        }
    }
}
pub(super) const LANE: f64 = 20.;
pub(super) const HEAD: f64 = 26.;
const LOOP: f64 = 28.;
const NEAR: f64 = 6.;
const FRAME_NEAR: f64 = 3.;
const PORT_STEP: f64 = 12.;
pub(super) fn cyl_ry(w: f64) -> f64 {
    12_f64.min(6_f64.max(w / 14.))
}
pub(super) fn queue_rx(h: f64) -> f64 {
    12_f64.min(h / 4.)
}
#[derive(Clone, Debug)]
pub(super) struct Route {
    pub d: String,
    pub sx: f64,
    pub sy: f64,
    pub sdx: f64,
    pub sdy: f64,
    pub ex: f64,
    pub ey: f64,
    pub edx: f64,
    pub edy: f64,
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
    pub nx: f64,
    pub ny: f64,
    pub lean: f64,
    pub room: f64,
    pub points: Vec<f64>,
}
fn sign(v: f64) -> f64 {
    if v == 0. {
        v
    } else {
        v.signum()
    }
}
#[allow(clippy::too_many_arguments)]
fn hit_rect(x0: f64, y0: f64, x1: f64, y1: f64, px: f64, py: f64, dx: f64, dy: f64) -> f64 {
    let (mut enter, mut leave) = (0_f64, 1_f64);
    if dx == 0. {
        if px < x0 || px > x1 {
            return f64::INFINITY;
        }
    } else {
        let (a, b) = ((x0 - px) / dx, (x1 - px) / dx);
        enter = enter.max(a.min(b));
        leave = leave.min(a.max(b))
    }
    if dy == 0. {
        if py < y0 || py > y1 {
            return f64::INFINITY;
        }
    } else {
        let (a, b) = ((y0 - py) / dy, (y1 - py) / dy);
        enter = enter.max(a.min(b));
        leave = leave.min(a.max(b))
    }
    if enter <= leave {
        enter
    } else {
        f64::INFINITY
    }
}
#[allow(clippy::too_many_arguments)]
fn hit_ellipse(cx: f64, cy: f64, rx: f64, ry: f64, px: f64, py: f64, dx: f64, dy: f64) -> f64 {
    let (ux, uy, vx, vy) = (dx / rx, dy / ry, (px - cx) / rx, (py - cy) / ry);
    let a = ux * ux + uy * uy;
    let b = 2. * (ux * vx + uy * vy);
    let c = vx * vx + vy * vy - 1.;
    let disc = b * b - 4. * a * c;
    if a == 0. || disc < 0. {
        return f64::INFINITY;
    }
    let t = (-b - disc.sqrt()) / (2. * a);
    if (0. ..=1.).contains(&t) {
        t
    } else if c <= 0. {
        0.
    } else {
        f64::INFINITY
    }
}
fn hit(b: &Box, px: f64, py: f64, qx: f64, qy: f64) -> f64 {
    let (x, y, w, h) = (b.x, b.y, b.w, b.h);
    let (dx, dy) = (qx - px, qy - py);
    match b.form.as_str() {
        "person" => hit_rect(x, y + HEAD, x + w, y + h, px, py, dx, dy).min(hit_ellipse(
            x + w / 2.,
            y + 14.,
            13.,
            13.,
            px,
            py,
            dx,
            dy,
        )),
        "cyl" => {
            let ry = cyl_ry(w);
            hit_rect(x, y + ry, x + w, y + h - ry, px, py, dx, dy)
                .min(hit_ellipse(x + w / 2., y + ry, w / 2., ry, px, py, dx, dy))
                .min(hit_ellipse(
                    x + w / 2.,
                    y + h - ry,
                    w / 2.,
                    ry,
                    px,
                    py,
                    dx,
                    dy,
                ))
        }
        "h-cyl" => {
            let rx = queue_rx(h);
            hit_rect(x + rx, y, x + w - rx, y + h, px, py, dx, dy)
                .min(hit_ellipse(x + rx, y + h / 2., rx, h / 2., px, py, dx, dy))
                .min(hit_ellipse(
                    x + w - rx,
                    y + h / 2.,
                    rx,
                    h / 2.,
                    px,
                    py,
                    dx,
                    dy,
                ))
        }
        _ => hit_rect(x, y, x + w, y + h, px, py, dx, dy),
    }
}
pub(super) fn crosses(points: &[f64], x0: f64, y0: f64, x1: f64, y1: f64) -> bool {
    for i in (2..points.len()).step_by(2) {
        let (px, py) = (points[i - 2], points[i - 1]);
        if hit_rect(x0, y0, x1, y1, px, py, points[i] - px, points[i + 1] - py) <= 1. {
            return true;
        }
    }
    false
}
fn contains(outer: &Box, inner: &Box) -> bool {
    outer.x <= inner.x
        && outer.y <= inner.y
        && outer.x + outer.w >= inner.x + inner.w
        && outer.y + outer.h >= inner.y + inner.h
}
fn connect(a: &Box, b: &Box, shift: f64) -> [f64; 4] {
    let (ax, ay, bx, by) = (
        a.x + a.w / 2.,
        a.y + a.h / 2.,
        b.x + b.w / 2.,
        b.y + b.h / 2.,
    );
    let down = contains(a, b);
    if down || contains(b, a) {
        let (outer, inner) = if down { (a, b) } else { (b, a) };
        let (ix, iy) = (inner.x + inner.w / 2., inner.y + inner.h / 2.);
        let (left, right, top, bottom) = (
            inner.x - outer.x,
            outer.x + outer.w - inner.x - inner.w,
            inner.y - outer.y,
            outer.y + outer.h - inner.y - inner.h,
        );
        let least = left.min(right).min(top).min(bottom);
        let ox = if least == left {
            outer.x
        } else if least == right {
            outer.x + outer.w
        } else {
            ix
        };
        let oy = if least == left || least == right {
            iy
        } else if least == bottom {
            outer.y + outer.h
        } else {
            outer.y
        };
        let t = hit(inner, ox, oy, ix, iy).min(1.);
        let (mx, my) = (ox + (ix - ox) * t, oy + (iy - oy) * t);
        return if down {
            [ox, oy, mx, my]
        } else {
            [mx, my, ox, oy]
        };
    }
    let (mut px, mut py, mut qx, mut qy) = (ax, ay, bx, by);
    let (x0, x1, y0, y1) = (
        a.x.max(b.x),
        (a.x + a.w).min(b.x + b.w),
        a.y.max(b.y),
        (a.y + a.h).min(b.y + b.h),
    );
    if x0 < x1 {
        px = (x0 + x1) / 2.;
        qx = px
    } else if y0 < y1 {
        py = (y0 + y1) / 2.;
        qy = py
    }
    let len = (qx - px).hypot(qy - py);
    let len = if len == 0. { 1. } else { len };
    let (nx, ny) = (-(qy - py) / len * shift, (qx - px) / len * shift);
    let (mut ta, mut tb) = (
        hit(a, qx + nx, qy + ny, px + nx, py + ny),
        hit(b, px + nx, py + ny, qx + nx, qy + ny),
    );
    if ta <= 1. && tb <= 1. {
        px += nx;
        py += ny;
        qx += nx;
        qy += ny
    } else {
        px = ax;
        py = ay;
        qx = bx;
        qy = by;
        ta = hit(a, qx, qy, px, py).min(1.);
        tb = hit(b, px, py, qx, qy).min(1.)
    }
    [
        qx + (px - qx) * ta,
        qy + (py - qy) * ta,
        px + (qx - px) * tb,
        py + (qy - py) * tb,
    ]
}
#[derive(Default)]
pub(super) struct Lanes {
    upright: Vec<f64>,
    level: Vec<f64>,
    frames: [Vec<f64>; 2],
    taken: HashMap<ElementRef, Vec<(u8, f64)>>,
    pub squeezed: usize,
}
impl Lanes {
    fn free(&self, upright: bool, at: f64, a0: f64, a1: f64) -> bool {
        let (lo, hi) = (a0.min(a1), a0.max(a1));
        let near = |list: &[f64], gap: f64| {
            list.chunks_exact(3)
                .any(|p| (p[0] - at).abs() < gap && p[1] < hi && p[2] > lo)
        };
        !near(if upright { &self.upright } else { &self.level }, NEAR)
            && !near(&self.frames[usize::from(!upright)], FRAME_NEAR)
    }
    fn crossings(&self, points: &[f64]) -> usize {
        let mut count = 0;
        for i in (2..points.len()).step_by(2) {
            let upright = points[i] == points[i - 2];
            let at = if upright { points[i] } else { points[i + 1] };
            let (a, b) = if upright {
                (points[i - 1], points[i + 1])
            } else {
                (points[i - 2], points[i])
            };
            let (lo, hi) = (a.min(b), a.max(b));
            for p in if upright { &self.level } else { &self.upright }.chunks_exact(3) {
                if p[0] > lo && p[0] < hi && p[1] < at && p[2] > at {
                    count += 1
                }
            }
        }
        count
    }
    pub fn frame(&mut self, b: &Box) {
        self.frames[0].extend([b.x, b.y, b.y + b.h, b.x + b.w, b.y, b.y + b.h]);
        self.frames[1].extend([b.y, b.x, b.x + b.w, b.y + b.h, b.x, b.x + b.w])
    }
    fn take(&mut self, points: &[f64]) {
        for i in (2..points.len()).step_by(2) {
            if points[i] == points[i - 2] {
                self.upright.extend([
                    points[i],
                    points[i - 1].min(points[i + 1]),
                    points[i - 1].max(points[i + 1]),
                ])
            } else if points[i + 1] == points[i - 1] {
                self.level.extend([
                    points[i + 1],
                    points[i - 2].min(points[i]),
                    points[i - 2].max(points[i]),
                ])
            }
        }
    }
    fn ports(&self, b: &Box, side: u8, want: f64, lo: f64, hi: f64) -> Vec<f64> {
        let used = self.taken.get(&b.id);
        let mut out = Vec::new();
        for k in 0usize..9 {
            if out.len() >= 4 {
                break;
            }
            let at = want + if k & 1 != 0 { 1. } else { -1. } * (k as f64 / 2.).ceil() * PORT_STEP;
            if at < lo || at > hi {
                continue;
            }
            if !used.is_some_and(|v| {
                v.iter()
                    .any(|&(s, p)| s == side && (p - at).abs() < PORT_STEP)
            }) {
                out.push(at)
            }
        }
        if out.is_empty() {
            out.push(want)
        }
        out
    }
    fn claim(&mut self, b: &Box, side: u8, at: f64) {
        self.taken.entry(b.id).or_default().push((side, at))
    }
}
fn path(points: &[f64], start_trim: f64, end_trim: f64) -> String {
    let n = points.len();
    let (ux, uy) = (sign(points[2] - points[0]), sign(points[3] - points[1]));
    let mut d = format!(
        "M{},{}",
        num(points[0] + ux * start_trim),
        num(points[1] + uy * start_trim)
    );
    for i in (2..n - 2).step_by(2) {
        let (x, y) = (points[i], points[i + 1]);
        let (ix, iy) = (sign(x - points[i - 2]), sign(y - points[i - 1]));
        let (ox, oy) = (sign(points[i + 2] - x), sign(points[i + 3] - y));
        let r = 8_f64
            .min((x - points[i - 2] + y - points[i - 1]).abs() / 2.)
            .min((points[i + 2] - x + points[i + 3] - y).abs() / 2.);
        d += &format!(
            "{}Q{},{} {},{}",
            if ix != 0. {
                format!("H{}", num(x - ix * r))
            } else {
                format!("V{}", num(y - iy * r))
            },
            num(x),
            num(y),
            num(x + ox * r),
            num(y + oy * r)
        )
    }
    let (lx, ly) = (
        sign(points[n - 2] - points[n - 4]),
        sign(points[n - 1] - points[n - 3]),
    );
    d += &if lx != 0. {
        format!("H{}", num(points[n - 2] - lx * end_trim))
    } else {
        format!("V{}", num(points[n - 1] - ly * end_trim))
    };
    d
}
fn clear_of(points: &[f64], others: &[Box], a: &Box, b: &Box) -> bool {
    !others.iter().any(|other| {
        other.id != a.id
            && other.id != b.id
            && crosses(
                points,
                other.x,
                other.y,
                other.x + other.w,
                other.y + other.h,
            )
    })
}
fn bent(
    points: Vec<f64>,
    along: usize,
    start_trim: f64,
    end_trim: f64,
    nx: f64,
    lean: f64,
    room: f64,
) -> Route {
    let n = points.len();
    Route {
        d: path(&points, start_trim, end_trim),
        sx: points[0],
        sy: points[1],
        sdx: -sign(points[2] - points[0]),
        sdy: -sign(points[3] - points[1]),
        ex: points[n - 2],
        ey: points[n - 1],
        edx: sign(points[n - 2] - points[n - 4]),
        edy: sign(points[n - 1] - points[n - 3]),
        x0: points[along],
        y0: points[along + 1],
        x1: points[along + 2],
        y1: points[along + 3],
        nx,
        ny: 0.,
        lean,
        room,
        points,
    }
}
fn across(lo: f64, hi: f64) -> Vec<f64> {
    let mut out = Vec::new();
    let mid = (lo + hi) / 2.;
    let step = 7_f64.max((hi - lo) / 4.);
    for k in 0usize..9 {
        let at = mid + if k & 1 != 0 { 1. } else { -1. } * (k as f64 / 2.).ceil() * step;
        if at >= lo + NEAR && at <= hi - NEAR {
            out.push(at)
        }
    }
    out
}
struct Elbow {
    points: Vec<f64>,
    cost: f64,
    upright: bool,
    sides: [u8; 2],
}
fn consider(
    best: &mut Option<Elbow>,
    points: Vec<f64>,
    upright: bool,
    sides: [u8; 2],
    a: &Box,
    b: &Box,
    others: &[Box],
    lanes: &Lanes,
) -> bool {
    if !clear_of(&points, others, a, b) {
        return false;
    }
    for i in (2..points.len()).step_by(2) {
        let up = points[i] == points[i - 2];
        if !lanes.free(
            up,
            if up { points[i] } else { points[i + 1] },
            if up { points[i - 1] } else { points[i - 2] },
            if up { points[i + 1] } else { points[i] },
        ) {
            return false;
        }
    }
    let mut cost = lanes.crossings(&points) as f64 * 1e6;
    for i in (2..points.len()).step_by(2) {
        cost += (points[i] - points[i - 2]).abs() + (points[i + 1] - points[i - 1]).abs()
    }
    if best.as_ref().is_none_or(|best| cost < best.cost) {
        *best = Some(Elbow {
            points,
            cost,
            upright,
            sides,
        });
        true
    } else {
        false
    }
}
fn elbow(
    a: &Box,
    b: &Box,
    others: &[Box],
    lanes: &mut Lanes,
    start_trim: f64,
    end_trim: f64,
) -> Option<Route> {
    let (ax, ay, bx, by) = (
        a.x + a.w / 2.,
        a.y + a.h / 2.,
        b.x + b.w / 2.,
        b.y + b.h / 2.,
    );
    let mut best = None;
    if a.x + a.w <= b.x || b.x + b.w <= a.x {
        let right = bx > ax;
        let down = sign(by - ay);
        let sides = if right { [1, 0] } else { [0, 1] };
        let lanes_across = across(
            if right { a.x + a.w } else { b.x + b.w },
            if right { b.x } else { a.x },
        );
        'search: for y0 in lanes.ports(a, sides[0], ay + down * a.h / 4., a.y + 8., a.y + a.h - 8.)
        {
            for y1 in lanes.ports(b, sides[1], by - down * b.h / 4., b.y + 8., b.y + b.h - 8.) {
                for &lane in &lanes_across {
                    let x0 = lane + (ax - lane) * hit(a, lane, y0, ax, y0).min(1.);
                    let x1 = lane + (bx - lane) * hit(b, lane, y1, bx, y1).min(1.);
                    if consider(
                        &mut best,
                        vec![x0, y0, lane, y0, lane, y1, x1, y1],
                        true,
                        sides,
                        a,
                        b,
                        others,
                        lanes,
                    ) {
                        break 'search;
                    }
                }
            }
        }
    }
    if a.y + a.h <= b.y || b.y + b.h <= a.y {
        let below = by > ay;
        let sideways = sign(bx - ax);
        let sides = if below { [3, 2] } else { [2, 3] };
        let lanes_across = across(
            if below { a.y + a.h } else { b.y + b.h },
            if below { b.y } else { a.y },
        );
        'search: for x0 in lanes.ports(
            a,
            sides[0],
            ax + sideways * a.w / 4.,
            a.x + 8.,
            a.x + a.w - 8.,
        ) {
            for x1 in lanes.ports(
                b,
                sides[1],
                bx - sideways * b.w / 4.,
                b.x + 8.,
                b.x + b.w - 8.,
            ) {
                for &lane in &lanes_across {
                    let y0 = lane + (ay - lane) * hit(a, x0, lane, x0, ay).min(1.);
                    let y1 = lane + (by - lane) * hit(b, x1, lane, x1, by).min(1.);
                    if consider(
                        &mut best,
                        vec![x0, y0, x0, lane, x1, lane, x1, y1],
                        false,
                        sides,
                        a,
                        b,
                        others,
                        lanes,
                    ) {
                        break 'search;
                    }
                }
            }
        }
    }
    let best = best?;
    lanes.take(&best.points);
    lanes.claim(
        a,
        best.sides[0],
        if best.upright {
            best.points[1]
        } else {
            best.points[0]
        },
    );
    lanes.claim(
        b,
        best.sides[1],
        if best.upright {
            best.points[7]
        } else {
            best.points[6]
        },
    );
    Some(bent(
        best.points,
        2,
        start_trim,
        end_trim,
        0.,
        0.,
        if best.upright { 88. } else { 160. },
    ))
}
fn blocks(other: &Box, a: &Box, b: &Box, sx: f64, sy: f64, ex: f64, ey: f64) -> bool {
    other.id != a.id
        && other.id != b.id
        && hit_rect(
            other.x,
            other.y,
            other.x + other.w,
            other.y + other.h,
            sx,
            sy,
            ex - sx,
            ey - sy,
        ) <= 1.
}
fn lane_on(forward: bool, upright: bool, a: &Box, b: &Box, others: &[Box], ends: [f64; 4]) -> f64 {
    let mut lane = if forward {
        f64::NEG_INFINITY
    } else {
        f64::INFINITY
    };
    let mut widen = |b: &Box| {
        let edge = if upright { b.x } else { b.y };
        lane = if forward {
            lane.max(edge + if upright { b.w } else { b.h })
        } else {
            lane.min(edge)
        }
    };
    widen(a);
    widen(b);
    for other in others {
        if blocks(other, a, b, ends[0], ends[1], ends[2], ends[3]) {
            widen(other)
        }
    }
    lane
}
fn outside(
    forward: bool,
    lane: f64,
    upright: bool,
    a: &Box,
    b: &Box,
    others: &[Box],
    lo: f64,
    hi: f64,
) -> bool {
    for other in others {
        if other.id == a.id || other.id == b.id {
            continue;
        }
        let near = if upright {
            other.y < hi && other.y + other.h > lo
        } else {
            other.x < hi && other.x + other.w > lo
        };
        let start = if upright { other.x } else { other.y };
        let end = start + if upright { other.w } else { other.h };
        if near && if forward { end > lane } else { start < lane } {
            return false;
        }
    }
    true
}
struct Along {
    points: Vec<f64>,
    side: u8,
    p0: f64,
    p1: f64,
    step: usize,
}
fn along(
    forward: bool,
    upright: bool,
    a: &Box,
    b: &Box,
    others: &[Box],
    lanes: &Lanes,
    ends: [f64; 4],
) -> Along {
    let lane = lane_on(forward, upright, a, b, others, ends);
    let sign = if forward { 1. } else { -1. };
    let side = if upright {
        if forward {
            1
        } else {
            0
        }
    } else if forward {
        3
    } else {
        2
    };
    let (ax, ay, bx, by) = (
        a.x + a.w / 2.,
        a.y + a.h / 2.,
        b.x + b.w / 2.,
        b.y + b.h / 2.,
    );
    let towards = if upright {
        signum(by - ay)
    } else {
        signum(bx - ax)
    };
    let p0 = if upright {
        lanes.ports(a, side, ay + towards * a.h / 4., a.y + 8., a.y + a.h - 8.)[0]
    } else {
        lanes.ports(a, side, ax + towards * a.w / 4., a.x + 8., a.x + a.w - 8.)[0]
    };
    let p1 = if upright {
        lanes.ports(b, side, by - towards * b.h / 4., b.y + 8., b.y + b.h - 8.)[0]
    } else {
        lanes.ports(b, side, bx - towards * b.w / 4., b.x + 8., b.x + b.w - 8.)[0]
    };
    let points_at = |at: f64| {
        if upright {
            let x0 = at + (ax - at) * hit(a, at, p0, ax, p0).min(1.);
            let x1 = at + (bx - at) * hit(b, at, p1, bx, p1).min(1.);
            vec![x0, p0, at, p0, at, p1, x1, p1]
        } else {
            let y0 = at + (ay - at) * hit(a, p0, at, p0, ay).min(1.);
            let y1 = at + (by - at) * hit(b, p1, at, p1, by).min(1.);
            vec![p0, y0, p0, at, p1, at, p1, y1]
        }
    };
    let mut points = points_at(lane + sign * LANE);
    for k in 0..=4 {
        let at = lane + sign * (LANE + k as f64 * 7.);
        let tried = points_at(at);
        if !clear_of(&tried, others, a, b) {
            continue;
        }
        points = tried;
        if lanes.free(upright, at, p0, p1) {
            return Along {
                points,
                side,
                p0,
                p1,
                step: k,
            };
        }
    }
    Along {
        points,
        side,
        p0,
        p1,
        step: 5,
    }
}
fn signum(v: f64) -> f64 {
    sign(v)
}
pub(super) fn route(
    a: &Box,
    b: &Box,
    shift: f64,
    others: Option<&[Box]>,
    lanes: &mut Lanes,
    start_trim: f64,
    end_trim: f64,
) -> Route {
    if a.id == b.id {
        let spread = (a.h / 2. - 4.).min(10.);
        let len = LOOP.hypot(spread);
        let x = a.x + a.w;
        let cy = a.y + a.h / 2.;
        let (ux, uy) = (LOOP / len, spread / len);
        return Route {
            d: format!(
                "M{},{}C{},{} {},{} {},{}",
                num(x + ux * start_trim),
                num(cy - spread - uy * start_trim),
                num(x + LOOP),
                num(cy - spread * 2.),
                num(x + LOOP),
                num(cy + spread * 2.),
                num(x + ux * end_trim),
                num(cy + spread + uy * end_trim)
            ),
            sx: x,
            sy: cy - spread,
            sdx: -ux,
            sdy: uy,
            ex: x,
            ey: cy + spread,
            edx: -ux,
            edy: -uy,
            x0: x + LOOP + 4.,
            y0: cy,
            x1: x + LOOP + 4.,
            y1: cy,
            nx: 1.,
            ny: 0.,
            lean: 0.,
            room: 64.,
            points: Vec::new(),
        };
    }
    let ends = connect(a, b, shift);
    let [sx, sy, ex, ey] = ends;
    let upright = sx == ex;
    let blocked = others.is_some_and(|others| {
        !contains(a, b)
            && !contains(b, a)
            && others
                .iter()
                .any(|other| blocks(other, a, b, sx, sy, ex, ey))
    });
    if let Some(others) = others.filter(|_| blocked && !upright && sy != ey) {
        if let Some(around) = elbow(a, b, others, lanes, start_trim, end_trim) {
            return around;
        }
        lanes.squeezed += 1
    }
    if let Some(others) = others.filter(|_| blocked && (upright || sy == ey)) {
        let lo = if upright { a.y.min(b.y) } else { a.x.min(b.x) };
        let hi = if upright {
            (a.y + a.h).max(b.y + b.h)
        } else {
            (a.x + a.w).max(b.x + b.w)
        };
        let natural = if upright { ey > sy } else { ex < sx };
        let out = outside(
            natural,
            lane_on(natural, upright, a, b, others, ends),
            upright,
            a,
            b,
            others,
            lo,
            hi,
        );
        let back = outside(
            !natural,
            lane_on(!natural, upright, a, b, others, ends),
            upright,
            a,
            b,
            others,
            lo,
            hi,
        );
        let first = along(natural, upright, a, b, others, lanes, ends);
        let other = if out == back {
            Some(along(!natural, upright, a, b, others, lanes, ends))
        } else {
            None
        };
        let cost = |way: &Along| lanes.crossings(&way.points) * 8 + way.step;
        let forward = if back && !out
            || other
                .as_ref()
                .is_some_and(|other| cost(other) < cost(&first))
        {
            !natural
        } else {
            natural
        };
        let way = if forward == natural {
            first
        } else {
            other.unwrap_or_else(|| along(!natural, upright, a, b, others, lanes, ends))
        };
        let sign = if forward { 1. } else { -1. };
        if way.step > 4 {
            lanes.squeezed += 1
        }
        lanes.take(&way.points);
        lanes.claim(a, way.side, way.p0);
        lanes.claim(b, way.side, way.p1);
        return bent(
            way.points,
            2,
            start_trim,
            end_trim,
            if upright { sign } else { 0. },
            if upright { 30. } else { 0. },
            if upright { 88. } else { 160. },
        );
    }
    let len = (ex - sx).hypot(ey - sy);
    let len = if len == 0. { 1. } else { len };
    let (dx, dy) = ((ex - sx) / len, (ey - sy) / len);
    if upright || sy == ey {
        lanes.take(&[sx, sy, ex, ey])
    }
    Route {
        d: format!(
            "M{},{}L{},{}",
            num(sx + dx * start_trim),
            num(sy + dy * start_trim),
            num(ex - dx * end_trim),
            num(ey - dy * end_trim)
        ),
        sx,
        sy,
        sdx: -dx,
        sdy: -dy,
        ex,
        ey,
        edx: dx,
        edy: dy,
        x0: sx,
        y0: sy,
        x1: ex,
        y1: ey,
        nx: if shift != 0. { -dy } else { 0. },
        ny: if shift != 0. { dx } else { 0. },
        lean: 0.,
        room: if dx.abs() > dy.abs() { 88. } else { 160. },
        points: vec![sx, sy, ex, ey],
    }
}
