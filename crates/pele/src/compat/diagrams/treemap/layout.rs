//! Pele treemap/layout.ts squarified rows, hierarchy sums and section insets.
use super::TreemapNode;
use crate::compat::number_math::{round, NumberMath};
pub(super) struct Box<'a> {
    pub node: &'a TreemapNode,
    parent: Option<usize>,
    pub series: usize,
    pub value: f64,
    children: Vec<usize>,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}
impl<'a> Box<'a> {
    fn new(node: &'a TreemapNode, parent: Option<usize>, series: usize) -> Self {
        Self {
            node,
            parent,
            series,
            value: if node.children.is_none() {
                node.value
                    .filter(|v| v.is_finite() && *v > 0.)
                    .unwrap_or(0.)
            } else {
                0.
            },
            children: vec![],
            x: 0.,
            y: 0.,
            w: 0.,
            h: 0.,
        }
    }
}
fn max(a: f64, b: f64) -> f64 {
    a.js_max(b)
}
fn squarify(
    all: &mut [Box<'_>],
    boxes: &[usize],
    total: f64,
    mut x: f64,
    mut y: f64,
    mut w: f64,
    mut h: f64,
    gap: f64,
) {
    let mut rest = total;
    let mut from = 0;
    while from < boxes.len() {
        let scale = w * h / rest;
        let side = w.js_min(h);
        let mut sum = all[boxes[from]].value * scale;
        let largest = sum;
        let mut worst = max(
            side * side * largest / (sum * sum),
            sum * sum / (side * side * largest),
        );
        let mut to = from + 1;
        while to < boxes.len() {
            let smallest = all[boxes[to]].value * scale;
            let next = sum + smallest;
            let ratio = max(
                side * side * largest / (next * next),
                next * next / (side * side * smallest),
            );
            if ratio > worst {
                break;
            }
            worst = ratio;
            sum = next;
            to += 1;
        }
        let thickness = sum / side;
        let mut offset = 0.;
        for (i, &k) in boxes.iter().enumerate().take(to).skip(from) {
            let b = &mut all[k];
            let length = if i == to - 1 {
                side - offset
            } else {
                b.value * scale / thickness
            };
            let bx = if w >= h { x } else { x + offset };
            let by = if w >= h { y + offset } else { y };
            let x0 = round(bx + gap / 2.);
            let y0 = round(by + gap / 2.);
            b.x = x0;
            b.y = y0;
            b.w = round(bx + if w >= h { thickness } else { length } - gap / 2.) - x0;
            b.h = round(by + if w >= h { length } else { thickness } - gap / 2.) - y0;
            offset += length;
            rest -= b.value;
        }
        if w >= h {
            x += thickness;
            w -= thickness;
        } else {
            y += thickness;
            h -= thickness;
        }
        from = to;
    }
}
fn place(
    all: &mut [Box<'_>],
    children: &[usize],
    placed: &mut Vec<usize>,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    gap: f64,
) {
    if w <= 0. || h <= 0. {
        return;
    }
    let mut shown: Vec<_> = children
        .iter()
        .copied()
        .filter(|i| all[*i].value > 0.)
        .collect();
    shown.sort_by(|a, b| {
        all[*b]
            .value
            .partial_cmp(&all[*a].value)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let total: f64 = shown.iter().map(|i| all[*i].value).sum();
    if shown.is_empty() || !total.is_finite() {
        return;
    }
    squarify(
        all,
        &shown,
        total,
        x - gap / 2.,
        y - gap / 2.,
        w + gap,
        h + gap,
        gap,
    );
    for i in shown {
        if all[i].w > 0. && all[i].h > 0. {
            placed.push(i)
        }
    }
}
pub(super) fn layout(roots: &[TreemapNode], width: f64, height: f64, gap: f64) -> Vec<Box<'_>> {
    let mut all: Vec<_> = roots
        .iter()
        .enumerate()
        .map(|(i, n)| Box::new(n, None, i))
        .collect();
    let top: Vec<_> = (0..all.len()).collect();
    let mut i = 0;
    while i < all.len() {
        let parent = i;
        let series = all[i].series;
        if let Some(children) = &all[i].node.children {
            for node in children {
                let k = all.len();
                all[parent].children.push(k);
                all.push(Box::new(node, Some(parent), series));
            }
        }
        i += 1;
    }
    for i in (0..all.len()).rev() {
        if let Some(parent) = all[i].parent {
            all[parent].value += all[i].value;
        }
    }
    let mut placed = vec![];
    place(&mut all, &top, &mut placed, 0., 0., width, height, gap);
    let mut i = 0;
    while i < placed.len() {
        let b = &all[placed[i]];
        let (x, y, w, h, children) = (
            b.x + 6.,
            b.y + 28.,
            b.w - 12.,
            b.h - 34.,
            b.children.clone(),
        );
        if !children.is_empty() {
            place(&mut all, &children, &mut placed, x, y, w, h, gap)
        }
        i += 1;
    }
    let mut values: Vec<_> = all.into_iter().map(Some).collect();
    placed
        .into_iter()
        .map(|i| values[i].take().unwrap())
        .collect()
}
