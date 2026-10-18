//! Pele c4/layout.ts actual nested grid layout, 458322a5 (MIT, kepano).
use super::{
    model::{C4Model, C4Shape, ElementRef},
    route::{Box, HEAD, LANE},
};
use crate::compat::{
    source::space,
    text::{layout_label, Label},
    PeleError, RenderResources, RenderSettings,
};
use std::collections::HashMap;
pub(super) const GROUP_PAD: f64 = LANE + 8.;
pub(super) const HEAD_LEFT: f64 = 16.;
pub(super) const HEAD_TOP: f64 = 10.;
pub(super) struct Node {
    pub shape: usize,
    pub bbox: Box,
    pub labels: Vec<Label>,
    pub text_h: f64,
}
pub(super) struct Group {
    pub boundary: usize,
    pub bbox: Box,
    pub kids: Vec<usize>,
    pub nodes: Vec<usize>,
    pub labels: Vec<Label>,
    pub head: f64,
    pub head_w: f64,
    pub head_x: f64,
}
pub(super) struct Layout {
    pub groups: Vec<Group>,
    pub nodes: Vec<Node>,
    pub order: Vec<usize>,
    pub boxes: HashMap<ElementRef, Box>,
}
fn form(name: &str) -> Option<&'static str> {
    Some(match name {
        "person" => "person",
        "box" | "rounded" | "component" => "rect",
        "cylinder" | "database" | "db" => "cyl",
        "queue" | "pipe" => "h-cyl",
        _ => return None,
    })
}
fn form_of(shape: &C4Shape) -> String {
    if let Some(form) = form(&shape.shape.as_deref().unwrap_or("").to_lowercase())
        .or_else(|| form(&shape.sprite.as_deref().unwrap_or("").to_lowercase()))
    {
        return form.into();
    }
    if let Some(tags) = &shape.tags {
        for tag in tags.split(',') {
            if let Some(form) = form(&tag.trim_matches(space).to_lowercase()) {
                return form.into();
            }
        }
    }
    if shape.kind.ends_with("person") {
        "person"
    } else if shape.kind.ends_with("_db") {
        "cyl"
    } else if shape.kind.ends_with("_queue") {
        "h-cyl"
    } else {
        "rect"
    }
    .into()
}
fn measure(
    index: usize,
    shape: &C4Shape,
    resources: &mut dyn RenderResources,
    options: &RenderSettings,
    size: f64,
    small: f64,
    width: f64,
) -> Result<Node, PeleError> {
    let form = form_of(shape);
    let cap_w = if form == "h-cyl" { 36. } else { 0. };
    let cap_h = if form == "person" {
        HEAD
    } else if form == "cyl" {
        24.
    } else {
        0.
    };
    let inner = width - 28. - cap_w;
    let kind = format!("«{}»", shape.kind.replace('_', " "));
    let techn = shape
        .techn
        .as_ref()
        .filter(|s| !s.is_empty())
        .map(|s| format!("[{s}]"));
    let labels = vec![
        layout_label(Some(&kind), false, resources, options, small - 1., inner, 0)?,
        layout_label(
            Some(&shape.label),
            false,
            resources,
            options,
            size,
            inner,
            1,
        )?,
        layout_label(techn.as_deref(), false, resources, options, small, inner, 0)?,
        layout_label(
            shape.descr.as_deref(),
            false,
            resources,
            options,
            small,
            inner,
            0,
        )?,
    ];
    let mut tw = 0_f64;
    let mut text_h = if labels[3].height > 0. { 4. } else { 0. };
    for label in &labels {
        tw = tw.max(label.width);
        text_h += label.height
    }
    Ok(Node {
        shape: index,
        bbox: Box::new(
            ElementRef::Shape(index),
            form,
            width.max(tw + 28. + cap_w),
            64_f64.max(text_h + 24. + cap_h),
        ),
        labels,
        text_h,
    })
}
fn count(
    widths: impl Iterator<Item = f64>,
    limit: f64,
    gap: f64,
    room: Option<f64>,
    pad: f64,
) -> f64 {
    let Some(room) = room else { return limit };
    let widest = widths.fold(1_f64, f64::max);
    1_f64.max(limit.min(((room + gap - 2. * pad) / (widest + gap)).floor()))
}
fn rows(boxes: &mut [&mut Box], per_row: f64, gap_x: f64, gap_y: f64, stretch: bool) -> (f64, f64) {
    let mut width = 0_f64;
    let mut y = 0.;
    let per_row = (per_row.min(boxes.len().max(1) as f64)) as usize;
    for row in boxes.chunks_mut(per_row.max(1)) {
        let mut x = 0.;
        let row_h = row.iter().map(|b| b.h).fold(0_f64, f64::max);
        for b in row {
            b.x = x;
            b.y = y;
            if stretch {
                b.h = row_h
            }
            x += b.w + gap_x
        }
        width = width.max(x - gap_x);
        y += row_h + gap_y
    }
    (width, if boxes.is_empty() { 0. } else { y - gap_y })
}
pub(super) fn layout(
    db: &C4Model,
    resources: &mut dyn RenderResources,
    options: &RenderSettings,
    size: f64,
    small: f64,
    room: Option<f64>,
    gap: f64,
) -> Result<Layout, PeleError> {
    let pad = if room.is_some() { 12. } else { GROUP_PAD };
    let mut groups = Vec::new();
    for (i, boundary) in db.boundaries.iter().enumerate() {
        let label_room = room.map_or(4000., |room| {
            80_f64.max(room - 2. * HEAD_LEFT - 2. * GROUP_PAD * 2.)
        });
        let labels = if boundary.parent.is_some() {
            let typ = boundary
                .element_type
                .as_ref()
                .filter(|s| !s.is_empty())
                .map(|s| format!("[{s}]"));
            vec![
                layout_label(
                    Some(&boundary.label),
                    false,
                    resources,
                    options,
                    size,
                    label_room,
                    1,
                )?,
                layout_label(
                    typ.as_deref(),
                    false,
                    resources,
                    options,
                    small,
                    label_room,
                    0,
                )?,
                layout_label(
                    boundary.descr.as_deref(),
                    false,
                    resources,
                    options,
                    small,
                    label_room,
                    0,
                )?,
            ]
        } else {
            Vec::new()
        };
        let mut head = 0.;
        let mut head_w = 0_f64;
        for label in &labels {
            head += label.height;
            head_w = head_w.max(label.width)
        }
        groups.push(Group {
            boundary: i,
            bbox: Box::new(ElementRef::Boundary(i), "rect".into(), 0., 0.),
            kids: Vec::new(),
            nodes: Vec::new(),
            labels,
            head: if head > 0. {
                head + HEAD_TOP + 6. + LANE
            } else {
                pad
            },
            head_w,
            head_x: HEAD_LEFT,
        })
    }
    for i in 0..groups.len() {
        if let Some(p) = db.boundaries[i].parent {
            groups[p].kids.push(i)
        }
    }
    let mut order = vec![0usize];
    let mut depths = vec![None; groups.len()];
    depths[0] = Some(0usize);
    let mut i = 0;
    while i < order.len() {
        let at = order[i];
        for &kid in &groups[at].kids {
            depths[kid] = Some(depths[at].unwrap() + 1);
            order.push(kid)
        }
        i += 1
    }
    let mut nodes = Vec::new();
    for (i, shape) in db.shapes.iter().enumerate() {
        if let Some(p) = shape.parent {
            let depth = depths[p].unwrap_or(0);
            let width = room.map_or(216., |room| {
                100_f64.max(216_f64.min(room - 2. * depth as f64 * pad - 32.))
            });
            let index = nodes.len();
            nodes.push(measure(i, shape, resources, options, size, small, width)?);
            groups[p].nodes.push(index)
        }
    }
    let per_row = 1_f64.max(db.shape_in_row);
    let groups_per_row = 1_f64.max(db.boundary_in_row);
    let mut kids_top = vec![0.; groups.len()];
    for &i in order.iter().rev() {
        let gap_x = if room.is_some() { gap } else { 100. };
        let node_ids = &groups[i].nodes;
        let per = count(
            node_ids.iter().map(|&n| nodes[n].bbox.w),
            per_row,
            gap_x,
            room,
            pad,
        );
        let mut node_boxes: Vec<_> = node_ids.iter().map(|&n| nodes[n].bbox.clone()).collect();
        let mut boxes: Vec<_> = node_boxes.iter_mut().collect();
        let (grid_w, grid_h) = rows(&mut boxes, per, gap_x, 88., true);
        for (&n, b) in node_ids.iter().zip(node_boxes) {
            nodes[n].bbox = b
        }
        let kids = groups[i].kids.clone();
        let per = count(
            kids.iter().map(|&n| groups[n].bbox.w),
            groups_per_row,
            48.,
            room,
            pad,
        );
        let mut copies: Vec<Box> = kids.iter().map(|&n| groups[n].bbox.clone()).collect();
        let mut boxes: Vec<_> = copies.iter_mut().collect();
        let (kids_w, kids_h) = rows(&mut boxes, per, 48., 48., false);
        for (&n, b) in kids.iter().zip(copies) {
            groups[n].bbox = b
        }
        let between = if grid_h > 0. && kids_h > 0. { 48. } else { 0. };
        let frame = if i == 0 { 0. } else { pad };
        kids_top[i] = grid_h + between;
        groups[i].bbox.w = grid_w
            .max(kids_w)
            .max(groups[i].head_w + 2. * HEAD_LEFT - 2. * frame)
            + 2. * frame;
        groups[i].bbox.h =
            if frame != 0. { groups[i].head } else { 0. } + grid_h + between + kids_h + frame;
    }
    let mut boxes = HashMap::new();
    for &i in &order {
        let left = groups[i].bbox.x + if i == 0 { 0. } else { pad };
        let top = groups[i].bbox.y + if i == 0 { 0. } else { groups[i].head };
        for &n in &groups[i].nodes {
            nodes[n].bbox.x += left;
            nodes[n].bbox.y += top;
            boxes.insert(nodes[n].bbox.id, nodes[n].bbox.clone());
        }
        let kids = groups[i].kids.clone();
        for kid in kids {
            groups[kid].bbox.x += left;
            groups[kid].bbox.y += top + kids_top[i]
        }
        if i != 0 {
            boxes.insert(groups[i].bbox.id, groups[i].bbox.clone());
        }
    }
    Ok(Layout {
        groups,
        nodes,
        order,
        boxes,
    })
}
