//! Recursive grid measured bottom-up and arranged top-down, Pele block/layout.ts.
use super::render::View;
use crate::compat::{ErrorCode, PeleError};
pub(super) fn position(columns: f64, position: f64) -> Result<(f64, usize), PeleError> {
    if columns == 0. || !columns.is_finite() || columns.fract() != 0. {
        let mut e = PeleError::new("Columns must be an integer !== 0.", ErrorCode::Semantic);
        e.diagram_type = Some("block".into());
        return Err(e);
    }
    Ok(if columns < 0. {
        (position, 0)
    } else {
        (position % columns, (position / columns).floor() as usize)
    })
}
pub(super) fn layout(order: &mut [View], gap: f64) -> Result<(f64, f64), PeleError> {
    for i in (0..order.len()).rev() {
        if order[i].children.is_empty() {
            continue;
        }
        let inset = if i == 0 { 0. } else { gap };
        let mut heights = Vec::<f64>::new();
        let mut pos = 0.;
        let mut cols = 0_f64;
        let mut cw = 0_f64;
        let mut tallest = 0_f64;
        let columns = order[i].columns;
        let children = order[i].children.clone();
        for j in children {
            let child = &mut order[j];
            let (px, py) = position(columns, pos)?;
            let mut span = if child.span >= 1. {
                child.span.min(1000.)
            } else {
                1.
            };
            if columns > 0. && span > columns - px {
                span = columns - px;
            }
            child.span = span;
            child.col = px;
            child.row = py;
            pos += span;
            if px + span > cols {
                cols = px + span;
            }
            if py == heights.len() {
                heights.push(0.);
            }
            if child.children.is_empty() {
                heights[py] = -1.;
            } else if heights[py] >= 0. && child.nh > heights[py] {
                heights[py] = child.nh;
            }
            if child.space {
                continue;
            }
            let width = (child.nw - gap * (span - 1.)) / span;
            if width > cw {
                cw = width;
            }
            if child.nh > tallest {
                tallest = child.nh;
            }
        }
        let mut height = (heights.len() as f64 - 1.) * gap;
        for h in &mut heights {
            if *h < 0. {
                *h = tallest;
            }
            height += *h;
        }
        order[i].cols = cols;
        order[i].nw = order[i].nw.max(cols * cw + (cols - 1.) * gap + 2. * inset);
        order[i].nh = height + 2. * inset + order[i].head;
        order[i].heights = heights;
    }
    order[0].w = order[0].nw;
    order[0].h = order[0].nh;
    order[0].x = order[0].w / 2.;
    order[0].y = order[0].h / 2.;
    for i in 0..order.len() {
        if order[i].children.is_empty() {
            continue;
        }
        let b = &order[i];
        let inset = if i == 0 { 0. } else { gap };
        let cw = (b.w - 2. * inset - (b.cols - 1.) * gap) / b.cols;
        let room = b.h - 2. * inset - b.head - (b.heights.len() as f64 - 1.) * gap;
        let needed = b.heights.iter().sum::<f64>();
        let scale = if needed > 0. { room / needed } else { 0. };
        let share = if needed > 0. {
            0.
        } else {
            room / b.heights.len() as f64
        };
        let left = b.x - b.w / 2. + inset;
        let mut top = b.y - b.h / 2. + inset + b.head;
        let heights = b.heights.clone();
        let mut row = 0;
        let children = b.children.clone();
        for j in children {
            let child = &mut order[j];
            while row < child.row {
                top += heights[row] * scale + share + gap;
                row += 1;
            }
            child.w = cw * child.span + gap * (child.span - 1.);
            child.h = heights[row] * scale + share;
            child.x = left + child.col * (cw + gap) + child.w / 2.;
            child.y = top + child.h / 2.;
        }
    }
    Ok((order[0].w, order[0].h))
}
