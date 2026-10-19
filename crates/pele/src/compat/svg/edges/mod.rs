//! Translated from Pele `src/svg/edges.ts`, MIT (kepano), commit
//! 458322a5b8abb1ae57ddf127c32017ab9fb14a92. Route geometry remains in Rust.
pub(crate) mod cluster;
pub(crate) mod theme;

use super::{esc_text, label_svg_with_resources, num};
use crate::compat::number_math::NumberMath;
use crate::compat::{layout::layered::ease, text::Label, PeleError, RenderResources};
use std::collections::HashMap;

pub(crate) const ARROW: f64 = 8.;
pub(crate) const LEAD: f64 = 6.;
const CORNER: f64 = 12.;
const HANDLE: f64 = 0.62;

#[derive(Clone, Debug, Default)]
pub(crate) struct EdgePath {
    pub d: String,
    pub sx: f64,
    pub sy: f64,
    pub sdx: f64,
    pub sdy: f64,
    pub ex: f64,
    pub ey: f64,
    pub edx: f64,
    pub edy: f64,
}

#[derive(Default)]
pub(crate) struct SideReach(HashMap<(usize, i32), f64>);
impl SideReach {
    pub fn add(&mut self, node: usize, side: i32, kind: &str) {
        let trim = marker_trim(kind);
        if side >= 0 && trim > 0. {
            let value = self.0.entry((node, side)).or_default();
            if trim + LEAD > *value {
                *value = trim + LEAD;
            }
        }
    }
    pub fn of(&self, node: usize, side: i32) -> f64 {
        if side < 0 {
            0.
        } else {
            self.0.get(&(node, side)).copied().unwrap_or(0.)
        }
    }
}

fn near(ax: f64, ay: f64, bx: f64, by: f64) -> bool {
    (ax - bx).abs() < 0.01 && (ay - by).abs() < 0.01
}
fn sign_or_one(value: f64) -> f64 {
    if value == 0. || value.is_nan() {
        1.
    } else {
        value.signum()
    }
}
struct Route {
    xs: Vec<f64>,
    ys: Vec<f64>,
    axes: Vec<f64>,
    linear: bool,
}
impl Route {
    fn aligned(&self, i: usize) -> bool {
        (self.xs[i] - self.xs[i - 1]).abs() < 0.01 || (self.ys[i] - self.ys[i - 1]).abs() < 0.01
    }
    fn straight(&self, i: usize) -> bool {
        self.linear || self.aligned(i)
    }
    fn unit(&self, i: usize, j: usize, axis: f64) -> (f64, f64) {
        if !self.straight(i.max(j)) {
            return if axis == 0. {
                (0., sign_or_one(self.ys[j] - self.ys[i]))
            } else {
                (sign_or_one(self.xs[j] - self.xs[i]), 0.)
            };
        }
        let dx = self.xs[j] - self.xs[i];
        let dy = self.ys[j] - self.ys[i];
        let len = dx.hypot(dy);
        let len = if len == 0. || len.is_nan() { 1. } else { len };
        (dx / len, dy / len)
    }
    fn length(&self, i: usize) -> f64 {
        (self.xs[i] - self.xs[i - 1]).abs() + (self.ys[i] - self.ys[i - 1]).abs()
    }
    fn turns(&self, i: usize) -> bool {
        !self.linear
            && i > 0
            && i + 1 < self.xs.len()
            && self.aligned(i)
            && self.aligned(i + 1)
            && self.length(i) >= 0.01
            && self.length(i + 1) >= 0.01
            && ((self.xs[i] - self.xs[i - 1]).abs() < 0.01)
                != ((self.xs[i + 1] - self.xs[i]).abs() < 0.01)
    }
    fn along(&self, i: usize, ux: f64, uy: f64) -> f64 {
        (self.xs[i] - self.xs[i - 1]).abs() * ux.abs()
            + (self.ys[i] - self.ys[i - 1]).abs() * uy.abs()
    }
    fn insert(&mut self, i: usize, x: f64, y: f64, axis: f64) {
        self.xs.insert(i, x);
        self.ys.insert(i, y);
        self.axes.insert(i, axis);
    }
}
fn lead(reach: f64, trim: f64, run: f64) -> f64 {
    let room = (reach - LEAD - trim).js_max(0.).js_min(run / 4.);
    room + (reach - trim - room).js_min((run - room) / 4.)
}
fn line_to(d: &mut String, at: &mut (f64, f64), x: f64, y: f64) {
    if near(x, y, at.0, at.1) {
        return;
    }
    d.push_str(&format!("L{},{}", num(x), num(y)));
    *at = (x, y);
}

/// A route consists of x/y/axis triples; layout supplies at least two triples.
pub(crate) fn route_path(
    route: &[f64],
    curve: Option<&str>,
    start_trim: f64,
    end_trim: f64,
    start_reach: Option<f64>,
    end_reach: Option<f64>,
) -> EdgePath {
    let stepped = matches!(curve, Some("step" | "stepBefore" | "stepAfter"));
    let mut r = Route {
        xs: Vec::new(),
        ys: Vec::new(),
        axes: Vec::new(),
        linear: curve == Some("linear"),
    };
    for triple in route.chunks_exact(3) {
        let (x, y, axis) = (triple[0], triple[1], triple[2]);
        if stepped && !r.xs.is_empty() {
            let last = r.xs.len() - 1;
            let (px, py) = (r.xs[last], r.ys[last]);
            if (x - px).abs() >= 0.01 && (y - py).abs() >= 0.01 {
                let my = match curve {
                    Some("stepBefore") => py,
                    Some("stepAfter") => y,
                    _ => (py + y) / 2.,
                };
                let mx = match curve {
                    Some("stepBefore") => px,
                    Some("stepAfter") => x,
                    _ => (px + x) / 2.,
                };
                let turns = if axis == 0. {
                    [(px, my), (x, my)]
                } else {
                    [(mx, py), (mx, y)]
                };
                for (tx, ty) in turns {
                    if !near(tx, ty, px, py) && !near(tx, ty, x, y) {
                        r.xs.push(tx);
                        r.ys.push(ty);
                        r.axes.push(axis);
                    }
                }
            }
        }
        r.xs.push(x);
        r.ys.push(y);
        r.axes.push(axis);
    }
    // Invalid routes cannot originate in a layout. Keep internal failures bounded.
    if r.xs.len() < 2 {
        return EdgePath::default();
    }
    let mut last = r.xs.len() - 1;
    let (sdx, sdy) = r.unit(0, 1, r.axes[1]);
    let (edx, edy) = r.unit(last - 1, last, r.axes[last]);
    let (sx, sy, ex, ey) = (r.xs[0], r.ys[0], r.xs[last], r.ys[last]);
    r.xs[0] += sdx * start_trim;
    r.ys[0] += sdy * start_trim;
    r.xs[last] -= edx * end_trim;
    r.ys[last] -= edy * end_trim;
    let start_reach = start_reach.unwrap_or(if start_trim > 0. {
        start_trim + LEAD
    } else {
        0.
    });
    let end_reach = end_reach.unwrap_or(if end_trim > 0. { end_trim + LEAD } else { 0. });
    let arrives = if end_reach > end_trim && (!r.straight(last) || r.turns(last - 1)) {
        lead(end_reach, end_trim, r.along(last, edx, edy))
    } else {
        0.
    };
    let leaves = if start_reach > start_trim && (!r.straight(1) || r.turns(1)) {
        lead(start_reach, start_trim, r.along(1, sdx, sdy))
    } else {
        0.
    };
    if arrives >= 1. {
        r.insert(
            last,
            r.xs[last] - edx * arrives,
            r.ys[last] - edy * arrives,
            r.axes[last],
        );
        last += 1;
    }
    if leaves >= 1. {
        r.insert(1, r.xs[0] + sdx * leaves, r.ys[0] + sdy * leaves, r.axes[1]);
        last += 1;
    }
    let mut d = format!("M{},{}", num(r.xs[0]), num(r.ys[0]));
    let mut at = (r.xs[0], r.ys[0]);
    for i in 1..=last {
        let (x, y) = (r.xs[i], r.ys[i]);
        if !r.straight(i) {
            let down = r.axes[i] == 0.;
            let along = if down { y - at.1 } else { x - at.0 };
            let across = if down { x - at.0 } else { y - at.1 };
            let k = along.abs() * ease(across, along);
            let signed = if along == 0. { along } else { along.signum() } * k;
            let (ux, uy) = if down { (0., signed) } else { (signed, 0.) };
            d.push_str(&format!(
                "C{},{} {},{} {},{}",
                num(at.0 + ux),
                num(at.1 + uy),
                num(x - ux),
                num(y - uy),
                num(x),
                num(y)
            ));
            at = (x, y);
            continue;
        }
        if !r.turns(i) {
            if i < last
                && r.aligned(i)
                && r.aligned(i + 1)
                && (x - r.xs[i - 1]) * (r.xs[i + 1] - x) + (y - r.ys[i - 1]) * (r.ys[i + 1] - y)
                    > 0.
            {
                continue;
            }
            line_to(&mut d, &mut at, x, y);
            continue;
        }
        let mut before = if r.turns(i - 1) {
            r.length(i) / 2.
        } else {
            r.length(i)
        };
        let mut after = if r.turns(i + 1) {
            r.length(i + 1) / 2.
        } else {
            r.length(i + 1)
        };
        if stepped {
            before = CORNER.js_min(before).js_min(after);
            after = before;
        }
        let (ux, uy) = r.unit(i - 1, i, r.axes[i]);
        let (vx, vy) = r.unit(i, i + 1, r.axes[i + 1]);
        line_to(&mut d, &mut at, x - ux * before, y - uy * before);
        at = (x + vx * after, y + vy * after);
        if stepped {
            d.push_str(&format!(
                "Q{},{} {},{}",
                num(x),
                num(y),
                num(at.0),
                num(at.1)
            ));
        } else {
            d.push_str(&format!(
                "C{},{} {},{} {},{}",
                num(x - ux * before * (1. - HANDLE)),
                num(y - uy * before * (1. - HANDLE)),
                num(x + vx * after * (1. - HANDLE)),
                num(y + vy * after * (1. - HANDLE)),
                num(at.0),
                num(at.1)
            ));
        }
    }
    EdgePath {
        d,
        sx,
        sy,
        sdx: -sdx,
        sdy: -sdy,
        ex,
        ey,
        edx,
        edy,
    }
}

pub(crate) fn marker_trim(kind: &str) -> f64 {
    match kind {
        "arrow_point" => ARROW - 1.,
        "arrow_circle" => ARROW,
        "arrow_cross" => 4.,
        _ => 0.,
    }
}
pub(crate) fn marker(kind: &str, x: f64, y: f64, dx: f64, dy: f64, color: &str) -> String {
    let (nx, ny) = (-dy, dx);
    match kind {
        "arrow_point" => {
            let (bx, by, half) = (x - dx * ARROW, y - dy * ARROW, ARROW * 0.42);
            format!("<path class=\"pele-marker\" d=\"M{},{}L{},{}L{},{}Z\" fill=\"{color}\" stroke=\"none\"/>", num(x), num(y), num(bx + nx * half), num(by + ny * half), num(bx - nx * half), num(by - ny * half))
        }
        "arrow_circle" => format!("<circle class=\"pele-marker\" cx=\"{}\" cy=\"{}\" r=\"3.5\" fill=\"{color}\" stroke=\"none\"/>", num(x - dx * 4.), num(y - dy * 4.)),
        "arrow_cross" => {
            let (cx, cy) = (x - dx * 4., y - dy * 4.);
            let (ux, uy, vx, vy) = ((dx + nx) * 3.5, (dy + ny) * 3.5, (dx - nx) * 3.5, (dy - ny) * 3.5);
            format!("<path class=\"pele-marker\" d=\"M{},{}L{},{}M{},{}L{},{}\"/>", num(cx - ux), num(cy - uy), num(cx + ux), num(cy + uy), num(cx - vx), num(cy - vy), num(cx + vx), num(cy + vy))
        }
        _ => String::new(),
    }
}
pub(crate) fn edge_label_svg(
    id: &str,
    label: &Label,
    x: f64,
    y: f64,
    attrs: &str,
    resources: &mut dyn RenderResources,
) -> Result<String, PeleError> {
    let w = label.width + 8.;
    Ok(format!("<g class=\"pele-edge-label\" data-id=\"{}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"3\" fill=\"var(--_bg)\"/>{}</g>", esc_text(id)?, num(x - w / 2.), num(y - label.height / 2.), num(w), num(label.height), label_svg_with_resources(label, x, y, attrs, resources)?))
}

#[derive(Clone, Debug, Default)]
pub(crate) struct LoopLabel {
    pub label_w: f64,
    pub label_h: f64,
    pub label_x: f64,
    pub label_y: f64,
}
pub(crate) fn loop_path(
    cx: f64,
    cy: f64,
    w: f64,
    h: f64,
    k: usize,
    sideways: bool,
    base: f64,
    trim: f64,
    e: &mut LoopLabel,
) -> EdgePath {
    let half = if sideways { w } else { h } / 2.;
    let spread = (half - 4.).js_min(8. + k as f64 * 6.);
    let reach = base + k as f64 * 8.;
    let len = reach.hypot(spread);
    let len = if len == 0. || len.is_nan() { 1. } else { len };
    let at = |out: f64, along: f64| {
        if sideways {
            format!("{},{}", num(cx + along), num(cy + h / 2. + out))
        } else {
            format!("{},{}", num(cx + w / 2. + out), num(cy + along))
        }
    };
    let (tx, ty) = (reach / len * trim, spread / len * trim);
    if sideways {
        e.label_x = cx;
        e.label_y = cy + h / 2. + reach + 4. + e.label_h / 2.;
    } else {
        e.label_x = cx + w / 2. + reach + 4. + e.label_w / 2.;
        e.label_y = cy;
    }
    EdgePath {
        d: format!(
            "M{}C{} {} {}",
            at(0., -spread),
            at(reach, -spread * 2.),
            at(reach, spread * 2.),
            at(tx, spread + ty)
        ),
        sx: if sideways { cx - spread } else { cx + w / 2. },
        sy: if sideways { cy + h / 2. } else { cy - spread },
        sdx: if sideways { 0. } else { -1. },
        sdy: if sideways { -1. } else { 0. },
        ex: if sideways { cx + spread } else { cx + w / 2. },
        ey: if sideways { cy + h / 2. } else { cy + spread },
        edx: if sideways {
            -spread / len
        } else {
            -reach / len
        },
        edy: if sideways {
            -reach / len
        } else {
            -spread / len
        },
    }
}
