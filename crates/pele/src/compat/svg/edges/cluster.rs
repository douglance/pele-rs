//! Pele `src/svg/cluster.ts`, MIT (kepano), commit 458322a5b8abb1ae57ddf127c32017ab9fb14a92.
use super::super::{label_svg_with_resources, num};
use crate::compat::{text::Label, PeleError, RenderResources};
use std::collections::HashMap;

// Rounded JS number keys, including magnitudes outside signed integer range.
pub(crate) type Crossings = HashMap<u64, Vec<f64>>;
#[derive(Clone, Copy, Debug)]
pub(crate) struct ClusterBox {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}
fn rounded_y(value: f64) -> u64 {
    let floor = value.floor();
    let rounded = if value - floor < 0.5 {
        floor
    } else {
        floor + 1.
    };
    if rounded == 0. {
        0.
    } else if rounded.is_nan() {
        f64::NAN
    } else {
        rounded
    }
    .to_bits()
}
pub(crate) fn mark_crossings(crossings: &mut Crossings, route: &[f64]) {
    for point in route.chunks_exact(3) {
        crossings
            .entry(rounded_y(point[1]))
            .or_default()
            .push(point[0]);
    }
}
pub(crate) fn cluster_title_x(c: &ClusterBox, width: f64, crossings: &Crossings, pad: f64) -> f64 {
    if width == 0. {
        return c.x;
    }
    let top = c.y - c.h / 2.;
    let left = c.x - c.w / 2.;
    let half = width / 2.;
    let candidates = [left + pad - 6. + half, c.x, left + c.w - pad + 6. - half];
    let Some(xs) = crossings.get(&rounded_y(top)) else {
        return candidates[0];
    };
    let mut best = candidates[0];
    let mut best_gap = -1.;
    for x in candidates {
        let mut gap = f64::INFINITY;
        for cx in xs {
            if *cx > left && *cx < left + c.w {
                gap = gap.min((cx - x).abs() - half);
            }
        }
        if gap >= 6. {
            return x;
        }
        if gap > best_gap + 0.5 {
            best_gap = gap;
            best = x;
        }
    }
    best
}
pub(crate) fn title_crossed(c: &ClusterBox, width: f64, x: f64, crossings: &Crossings) -> bool {
    if width <= 0. {
        return false;
    }
    let Some(xs) = crossings.get(&rounded_y(c.y - c.h / 2.)) else {
        return false;
    };
    let left = c.x - c.w / 2.;
    xs.iter()
        .any(|cx| *cx > left && *cx < left + c.w && (cx - x).abs() < width / 2. + 4.)
}
/// `parents` preserves compound-layout node indexes; negative indexes denote roots.
pub(crate) fn struck_title(
    label: &Label,
    x: f64,
    y: f64,
    attrs: &str,
    id: &str,
    parents: &[i32],
    index: usize,
    resources: &mut dyn RenderResources,
) -> Result<String, PeleError> {
    let mut depth = 1;
    let mut p = parents[index];
    while p >= 0 && depth < 8 {
        depth += 1;
        p = parents[p as usize];
    }
    let fill = format!(
        "color-mix(in srgb,var(--_a) {}%,var(--_bg))",
        num(100. * (1. - 0.5_f64.powi(depth)))
    );
    let outline =
        format!(" fill=\"none\" stroke=\"{fill}\" stroke-width=\"5\" stroke-linejoin=\"round\"");
    Ok(format!(
        "<g class=\"pele-cluster-title\" data-id=\"{id}\">{}{}</g>",
        label_svg_with_resources(label, x, y, &outline, resources)?,
        label_svg_with_resources(label, x, y, attrs, resources)?
    ))
}
