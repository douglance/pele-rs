//! Shape geometry translated from Pele `src/svg/shapes.ts` at
//! 458322a5b8abb1ae57ddf127c32017ab9fb14a92 (MIT, Copyright 2026 kepano).
mod draw;
pub(crate) use draw::draw_shape;

pub(crate) const PAD_X: f64 = 18.;
pub(crate) const PAD_Y: f64 = 12.;
const CORNER: f64 = 12.;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ShapeSize {
    pub w: f64,
    pub h: f64,
    pub dy: f64,
}

fn size(w: f64, h: f64, dy: f64) -> ShapeSize {
    ShapeSize { w, h, dy }
}
// JavaScript Math.min/max propagate NaN, unlike Rust's f64::min/max.
fn minimum(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.min(b)
    }
}
fn maximum(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.max(b)
    }
}
fn box_size(tw: f64, th: f64) -> ShapeSize {
    size(
        maximum(tw + 2. * PAD_X, 2. * PAD_X + 12.),
        th + 2. * PAD_Y,
        0.,
    )
}
fn tri_size(tw: f64, th: f64, flip: bool) -> ShapeSize {
    let w = maximum(1.6 * tw + 40., 80.);
    let h = (th + 10.) / (1. - (tw + 8.) / w);
    let dy = h / 2. - 10. - th / 2.;
    size(w, h, if flip { -dy } else { dy })
}

pub(crate) fn shape_size(shape: &str, tw: f64, th: f64) -> ShapeSize {
    let h = th + 2. * PAD_Y;
    match shape {
        "rounded" | "notch-rect" | "datastore" | "tag-rect" => box_size(tw, th),
        "notch-pent" => size(tw + 2. * PAD_X, h, 0.),
        "stadium" => size(tw + h * 0.5 + 2. * PAD_X - 8., h, 0.),
        "fr-rect" => size(tw + 2. * PAD_X + 16., h, 0.),
        "cyl" | "lin-cyl" => {
            let w = maximum(tw + 2. * PAD_X, 56.);
            let ry = minimum(12., maximum(6., w / 14.));
            let extra = if shape == "lin-cyl" { 8. } else { 0. };
            size(w, h + 2. * ry + extra, ry / 2. + extra / 2.)
        }
        "circle" | "dbl-circ" => {
            let double = shape == "dbl-circ";
            let d = maximum(
                (tw * tw + th * th).sqrt() + if double { 26. } else { 16. },
                if double { 54. } else { 44. },
            );
            size(d, d, 0.)
        }
        "ellipse" => size(tw * 1.3 + 2. * PAD_X, th * 1.2 + 2. * PAD_Y, 0.),
        "diam" => {
            let a = tw + 16.;
            let b = th + 8.;
            size(a + 1.6 * b, a / 1.6 + b, 0.)
        }
        "hex" | "lean-r" | "lean-l" | "trap-b" | "trap-t" => size(tw + 2. * PAD_X + h / 2., h, 0.),
        "odd" | "delay" => size(tw + 2. * PAD_X + h / 4., h, 0.),
        "text" => size(tw + 8., th + 8., 0.),
        "sm-circ" | "f-circ" => size(14., 14., 0.),
        "fr-circ" => size(18., 18., 0.),
        "fork" => size(72., 8., 0.),
        "lin-rect" => size(tw + 2. * PAD_X + 8., h, 0.),
        "div-rect" => size(tw + 2. * PAD_X, h + 8., 4.),
        "win-pane" | "st-rect" => size(tw + 2. * PAD_X + 8., h + 8., 4.),
        "tri" | "flip-tri" => tri_size(tw, th, shape == "flip-tri"),
        "sl-rect" => size(tw + 2. * PAD_X, h + 10., 5.),
        "hourglass" => size(32., 40., 0.),
        "h-cyl" => size(tw + 2. * PAD_X + 2. * minimum(12., h / 4.), h, 0.),
        "cross-circ" => size(44., 44., 0.),
        "doc" | "tag-doc" => size(tw + 2. * PAD_X, h + 10., -5.),
        "flag" => size(tw + 2. * PAD_X, h + 16., 0.),
        "folder" => size(maximum(tw + 2. * PAD_X, 64.), h + 8., 4.),
        "bucket" => size(tw + 2. * PAD_X + 20., h + 10., 3.),
        "console" | "browser" => size(maximum(tw + 2. * PAD_X, 72.), h + 12., 6.),
        "person" => size(maximum(tw + 2. * PAD_X, 56.), h + 26., 13.),
        "bang" => size(tw + 2. * PAD_X + 28., h + 24., 0.),
        "cloud" => size(tw + 2. * PAD_X + 32., h + 24., 0.),
        "brace" | "brace-r" => size(tw + 2. * PAD_X + 12., h, 0.),
        "braces" => size(tw + 2. * PAD_X + 24., h, 0.),
        "bolt" => size(36., 56., 0.),
        "curv-trap" => size(tw + 2. * PAD_X + h * 0.6, h, 0.),
        "docs" => size(tw + 2. * PAD_X + 8., h + 18., -1.),
        "bow-rect" => size(tw + 2. * PAD_X + h * 0.4, h, 0.),
        "lin-doc" => size(tw + 2. * PAD_X + 8., h + 10., -5.),
        "collapsedGroup" => size(tw + 2. * PAD_X + 8., h + 8., -2.),
        _ => box_size(tw, th),
    }
}

fn bulge(t: f64, half: f64, depth: f64) -> f64 {
    let k = minimum(1., t.abs() / half);
    depth * (1. - (1. - k * k).sqrt())
}

pub(crate) fn shape_inset(shape: &str, w: f64, h: f64, side: i32, t: f64) -> f64 {
    let vertical = side & 1 != 0;
    match shape {
        "circle" | "dbl-circ" | "ellipse" | "sm-circ" | "f-circ" | "fr-circ" | "cross-circ" => {
            if vertical {
                bulge(t, h / 2., w / 2.)
            } else {
                bulge(t, w / 2., h / 2.)
            }
        }
        "stadium" => {
            if vertical {
                bulge(t, h / 2., h / 2.)
            } else {
                0.
            }
        }
        "cyl" | "lin-cyl" => {
            if vertical {
                0.
            } else {
                bulge(t, w / 2., minimum(12., maximum(6., w / 14.)))
            }
        }
        "diam" => {
            if vertical {
                t.abs() * w / h
            } else {
                t.abs() * h / w
            }
        }
        "hex" => {
            if vertical {
                t.abs() / 2.
            } else {
                0.
            }
        }
        "odd" => {
            if side == 3 {
                h / 4.
            } else {
                0.
            }
        }
        "lean-r" | "lean-l" | "trap-b" | "trap-t" => {
            if vertical {
                h / 8.
            } else {
                0.
            }
        }
        "tri" | "flip-tri" | "bolt" => {
            if vertical {
                w / 4.
            } else {
                0.
            }
        }
        "sl-rect" => {
            if side == 0 {
                5.
            } else {
                0.
            }
        }
        "hourglass" => {
            if vertical {
                w / 2.
            } else {
                0.
            }
        }
        "doc" | "docs" | "tag-doc" | "lin-doc" => {
            if side == 2 {
                10.
            } else {
                0.
            }
        }
        "flag" => {
            if vertical {
                0.
            } else {
                8.
            }
        }
        "bucket" => {
            if vertical {
                h / 12.
            } else {
                0.
            }
        }
        "bang" => 3.,
        "cloud" => {
            if vertical {
                2.
            } else {
                1.
            }
        }
        "bow-rect" => {
            if side == 1 {
                h * 0.2
            } else {
                0.
            }
        }
        _ => 0.,
    }
}

pub(crate) fn shape_span(shape: &str, w: f64, h: f64, sideways: bool) -> f64 {
    let across = if sideways { h } else { w };
    let span = match shape {
        "rect" | "rounded" | "fr-rect" | "text" | "notch-rect" | "lin-rect" | "div-rect"
        | "win-pane" | "notch-pent" | "datastore" | "console" | "browser" | "tag-rect" => {
            across - 2. * CORNER
        }
        "stadium" => {
            if sideways {
                h * 0.6
            } else {
                w - h
            }
        }
        "cyl" => {
            if sideways {
                h - 2. * CORNER - 12.
            } else {
                w * 0.6
            }
        }
        "lin-cyl" => {
            if sideways {
                h - 2. * CORNER - 20.
            } else {
                w * 0.6
            }
        }
        "circle" | "dbl-circ" | "ellipse" | "diam" | "sm-circ" | "f-circ" | "fr-circ"
        | "cross-circ" => across * 0.6,
        "hex" => {
            if sideways {
                h * 0.5
            } else {
                w - h / 2. - CORNER
            }
        }
        "odd" => {
            if sideways {
                0.
            } else {
                w - 2. * CORNER
            }
        }
        "lean-r" | "lean-l" | "trap-b" | "trap-t" => {
            if sideways {
                0.
            } else {
                w - 2. * (h / 4.) - 2. * CORNER
            }
        }
        "fork" => across - 8.,
        "st-rect" => across - 2. * CORNER - 8.,
        _ => 0.,
    };
    maximum(0., span)
}

pub(crate) fn shape_has_label(shape: &str) -> bool {
    !matches!(
        shape,
        "sm-circ" | "f-circ" | "fr-circ" | "fork" | "hourglass" | "cross-circ" | "bolt"
    )
}

pub(crate) fn end_side(route: &[f64], at: usize, x: f64, y: f64, w: f64, h: f64) -> i32 {
    let dx = route.get(at).copied().unwrap_or(f64::NAN) - x;
    let dy = route.get(at + 1).copied().unwrap_or(f64::NAN) - y;
    if (dy.abs() - h / 2.).abs() < 0.5 {
        return if dy < 0. { 0 } else { 2 };
    }
    if (dx.abs() - w / 2.).abs() < 0.5 {
        return if dx < 0. { 3 } else { 1 };
    }
    -1
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn inset_route(
    route: &mut [f64],
    at: usize,
    shape: &str,
    view_w: f64,
    view_h: f64,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    is_group: bool,
) {
    if is_group || at + 1 >= route.len() {
        return;
    }
    let side = end_side(route, at, x, y, w, h);
    if side == 0 || side == 2 {
        let amount = shape_inset(shape, view_w, view_h, side, route[at] - x) + (h - view_h) / 2.;
        route[at + 1] += if side == 0 { amount } else { -amount };
    } else if side >= 0 {
        let amount =
            shape_inset(shape, view_w, view_h, side, route[at + 1] - y) + (w - view_w) / 2.;
        route[at] += if side == 3 { amount } else { -amount };
    }
}
