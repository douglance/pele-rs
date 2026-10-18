//! Pele diagrams/mindmap/render.ts, 458322a5 (MIT, kepano).
use super::{MindmapModel, MindmapNode};
use crate::compat::{
    svg::{
        document,
        edges::theme::class_names,
        esc_text, label_svg, label_svg_with_resources, num, series_color,
        shapes::{draw_shape, shape_inset, shape_size},
    },
    text::{layout_label, Label},
    Object, PeleError, RenderResources, RenderSettings, Rendered, Value,
};
const GAP_X: f64 = 36.;
const GAP_Y: f64 = 8.;
const GROUP_GAP: f64 = 18.;
const BRANCH_GAP_X: f64 = 52.;
const BRANCH_GAP_Y: f64 = 28.;
const TEXT_PAD: f64 = 6.;
const PILL_PAD: f64 = 12.;
const ICON_GAP: f64 = 6.;
const INDENT: f64 = 28.;
const STEM: f64 = 12.;
const BEND: f64 = 8.;
const ROW_GAP: f64 = 10.;
fn left_label(
    l: &Label,
    left: f64,
    cy: f64,
    r: &mut dyn RenderResources,
) -> Result<String, PeleError> {
    if l.lines.len() < 2 {
        return label_svg_with_resources(l, left + l.width / 2., cy, " class=\"pele-label\"", r);
    }
    let mut out = String::new();
    for (k, line) in l.lines.iter().enumerate() {
        let w = l.widths[k];
        let one = Label {
            lines: vec![line.clone()],
            widths: vec![w],
            width: w,
            height: l.line_height,
            size: l.size,
            line_height: l.line_height,
        };
        out += &label_svg_with_resources(
            &one,
            left + w / 2.,
            cy + (k as f64 - (l.lines.len() as f64 - 1.) / 2.) * l.line_height,
            " class=\"pele-label\"",
            r,
        )?;
    }
    Ok(out)
}
fn stack(
    list: &[usize],
    nodes: &[MindmapNode],
    from: usize,
    to: usize,
    gap: f64,
    extent: &[f64],
    own: &[f64],
    anchor: &[f64],
    rel: &mut [f64],
) -> (f64, f64) {
    let mut y = 0.;
    let mut last = 0.;
    for k in from..to {
        let c = list[k];
        rel[c] = y;
        let grouped = !nodes[c].children.is_empty()
            || (k + 1 < to && !nodes[list[k + 1]].children.is_empty());
        last = if grouped { gap.max(GROUP_GAP) } else { gap };
        y += extent[c] + last;
    }
    let a = list[from];
    let b = list[to - 1];
    (
        (rel[a] + own[a] + anchor[a] + rel[b] + own[b] + anchor[b]) / 2.,
        y - last,
    )
}
pub(super) fn render(
    m: &MindmapModel,
    c: &Object,
    o: &RenderSettings,
    r: &mut dyn RenderResources,
) -> Result<Rendered, PeleError> {
    let across = draw(m, c, o, r, false)?;
    let Some(available) = o.max_width.filter(|w| *w > 0.) else {
        return Ok(across);
    };
    if o.auto_direction == Some(false)
        || across.width <= available
        || available >= o.direction_breakpoint.filter(|v| *v > 0.).unwrap_or(640.)
    {
        return Ok(across);
    }
    let down = draw(m, c, o, r, true)?;
    Ok(if down.width < across.width {
        down
    } else {
        across
    })
}
fn draw(
    m: &MindmapModel,
    c: &Object,
    o: &RenderSettings,
    r: &mut dyn RenderResources,
    down: bool,
) -> Result<Rendered, PeleError> {
    let size = o.font_size.unwrap_or(16.);
    let pad = o.padding.unwrap_or(8.);
    let wrap = c
        .get("mindmap")
        .and_then(Value::as_object)
        .and_then(|m| m.get("maxNodeWidth"))
        .and_then(|v| {
            if let Value::Number(n) = v {
                Some(*n)
            } else {
                None
            }
        })
        .filter(|v| *v > 0.)
        .map_or(200., |v| v.min(2000.));
    let glyph = size * 1.25;
    let nodes = &m.nodes;
    let n = nodes.len();
    let mut parent = vec![0usize; n];
    let mut depth = vec![0usize; n];
    for i in 0..n {
        for &child in &nodes[i].children {
            parent[child] = i;
            depth[child] = depth[i] + 1;
        }
    }
    let room = if down {
        o.max_width
            .filter(|v| *v > 0.)
            .map_or(f64::INFINITY, |v| v - 2. * pad)
    } else {
        f64::INFINITY
    };
    let mut labels = Vec::with_capacity(n);
    let mut shapes = Vec::with_capacity(n);
    let mut w = vec![0.; n];
    let mut h = vec![0.; n];
    let mut dy = vec![0.; n];
    let mut anchor = vec![0.; n];
    let mut drawn = vec![false; n];
    for i in 0..n {
        let node = &nodes[i];
        drawn[i] = if let Some(icon) = &node.icon {
            r.icon(icon)?.is_some_and(|s| !s.markup().is_empty())
        } else {
            false
        };
        let fit = wrap.min(72f64.max(
            room - depth[i] as f64 * INDENT
                - 2. * TEXT_PAD
                - if drawn[i] { glyph + ICON_GAP } else { 0. },
        ));
        let label = layout_label(Some(&node.descr), true, r, o, size, fit, 0)?;
        let tw = label.width
            + if drawn[i] {
                glyph + if label.width > 0. { ICON_GAP } else { 0. }
            } else {
                0.
            };
        let th = label.height.max((size * 1.5 + 0.5).floor());
        let mut shape = match node.node_type {
            1 => "rounded",
            2 => "rect",
            3 => "circle",
            4 => "cloud",
            5 => "bang",
            6 => "hex",
            _ => {
                if i == 0 {
                    "stadium"
                } else {
                    ""
                }
            }
        };
        if down && i == 0 && shape != "rect" && shape != "rounded" {
            shape = "stadium";
        }
        shapes.push(shape);
        if shape.is_empty() {
            let pill = !node.children.is_empty();
            w[i] = tw + 2. * if pill { PILL_PAD } else { TEXT_PAD };
            h[i] = th + if pill { 6. } else { 0. };
            anchor[i] = h[i] / 2.;
        } else {
            let s = shape_size(shape, tw, th);
            w[i] = s.w;
            h[i] = s.h;
            dy[i] = s.dy;
            anchor[i] = s.h / 2.;
        }
        labels.push(label);
    }
    let mut extent = vec![0.; n];
    let mut rel = vec![0.; n];
    let mut own = vec![0.; n];
    let mut kids = vec![0.; n];
    for i in (1..n).rev() {
        let children = &nodes[i].children;
        if children.is_empty() {
            extent[i] = h[i];
            continue;
        }
        let (middle, stacked) = stack(
            children,
            nodes,
            0,
            children.len(),
            GAP_Y,
            &extent,
            &own,
            &anchor,
            &mut rel,
        );
        let top = middle - anchor[i];
        let low = 0f64.min(top);
        extent[i] = stacked.max(top + h[i]) - low;
        own[i] = top - low;
        kids[i] = -low;
    }
    let mut x = vec![0.; n];
    let mut y = vec![0.; n];
    let mut band = vec![0.; n];
    let mut side = vec![0.; n];
    let mut trunk = vec![0.; n];
    if down {
        let mut row = 0.;
        for i in 0..n {
            if i > 0 && parent[i] == 0 {
                row += ROW_GAP;
            }
            x[i] = depth[i] as f64 * INDENT + w[i] / 2.;
            y[i] = row;
            row += h[i] + ROW_GAP;
        }
    } else if n > 0 {
        let branches = &nodes[0].children;
        let count = branches.len();
        let total: f64 = branches.iter().map(|&i| extent[i]).sum();
        let mut split = count;
        let mut best = f64::INFINITY;
        let mut right = 0.;
        for k in 1..count {
            right += extent[branches[k - 1]];
            let diff = (2. * right - total + (2. * k as f64 - count as f64) * BRANCH_GAP_Y).abs();
            if diff < best {
                best = diff;
                split = k;
            }
        }
        let mut from = 0;
        let mut to = split;
        let mut dir = 1.;
        while from < count {
            let (middle, _) = stack(
                branches,
                nodes,
                from,
                to,
                BRANCH_GAP_Y,
                &extent,
                &own,
                &anchor,
                &mut rel,
            );
            for &child in &branches[from..to] {
                band[child] = rel[child] - middle;
                side[child] = dir;
            }
            from = to;
            to = count;
            dir = -1.;
        }
        y[0] = -h[0] / 2.;
        for i in 0..n {
            if i > 0 {
                y[i] = band[i] + own[i];
            }
            for &child in &nodes[i].children {
                if i > 0 {
                    side[child] = side[i];
                    band[child] = band[i] + kids[i] + rel[child];
                }
                x[child] = x[i]
                    + side[child]
                        * (w[i] / 2. + if i > 0 { GAP_X } else { BRANCH_GAP_X } + w[child] / 2.);
            }
        }
    }
    let (mut min_x, mut max_x, mut min_y, mut max_y) = (0f64, 0f64, 0f64, 0f64);
    let mut out = String::new();
    let mut edges = String::new();
    let mut body = String::new();
    let mut color = String::new();
    for k in 1..=n {
        let i = k % n;
        let node = &nodes[i];
        let shape = shapes[i];
        let label = &labels[i];
        let cx = x[i];
        let top = y[i];
        min_x = min_x.min(cx - w[i] / 2.);
        max_x = max_x.max(cx + w[i] / 2.);
        min_y = min_y.min(top);
        max_y = max_y.max(top + h[i]);
        if (i == 0 || parent[i] == 0) && !body.is_empty() {
            out+=&format!("<g class=\"pele-branch pele-section-{}\"><g class=\"pele-edges\" fill=\"none\" stroke=\"{color}\" stroke-width=\"1.5\">{edges}</g>{body}</g>",nodes[k-1].section.unwrap_or(0));
            edges.clear();
            body.clear();
        }
        if i > 0 && down {
            if parent[i] == 0 {
                color = series_color(node.section.unwrap_or(0));
            }
            let p = parent[i];
            let stem = if p == 0 {
                STEM.max(h[0] / 2.).min(INDENT - BEND)
            } else {
                STEM
            };
            let x1 = x[p] - w[p] / 2. + stem.min(w[p] / 2.);
            let y1 = if trunk[p] != 0. {
                trunk[p]
            } else {
                y[p] + h[p]
            };
            let left = cx - w[i] / 2.;
            let x2 = left
                + if !shape.is_empty() {
                    shape_inset(shape, w[i], h[i], 3, 0.)
                } else {
                    0.
                };
            let y2 = top + anchor[i];
            let bend = 0f64.max(BEND.min(x2 - x1).min(y2 - y1));
            trunk[p] = y2 - bend;
            edges += &format!(
                "<path class=\"pele-edge\" d=\"M{},{}V{}Q{},{} {},{}H{}\"/>",
                num(x1),
                num(y1),
                num(y2 - bend),
                num(x1),
                num(y2),
                num(x1 + bend),
                num(y2),
                num(x2)
            );
        } else if i > 0 {
            if parent[i] == 0 {
                color = series_color(node.section.unwrap_or(0));
            }
            let p = parent[i];
            let dir = side[i];
            let from = if dir > 0. { 1 } else { 3 };
            let x1 = x[p]
                + dir
                    * (w[p] / 2.
                        - if !shapes[p].is_empty() {
                            shape_inset(shapes[p], w[p], h[p], from, 0.)
                        } else {
                            0.
                        });
            let y1 = y[p] + anchor[p];
            let near = cx - dir * w[i] / 2.;
            let x2 = near
                + if !shape.is_empty() {
                    dir * shape_inset(shape, w[i], h[i], 4 - from, 0.)
                } else {
                    0.
                };
            let y2 = top + anchor[i];
            let mid = num((x1 + x2) / 2.);
            edges += &format!(
                "<path class=\"pele-edge\" d=\"M{},{}C{mid},{} {mid},{} {},{}\"/>",
                num(x1),
                num(y1),
                num(y1),
                num(y2),
                num(x2),
                num(y2)
            );
        }
        let pill = shape.is_empty() && !node.children.is_empty();
        let tint = format!(" fill=\"{color}\" fill-opacity=\"0.22\" stroke=\"{color}\"");
        let mut inner = String::new();
        if !shape.is_empty() {
            inner = if i > 0 {
                draw_shape(shape, w[i], h[i], &tint, &format!(" stroke=\"{color}\""))
            } else {
                draw_shape(
                    shape,
                    w[i],
                    h[i],
                    " fill=\"var(--_s)\" stroke=\"var(--_b)\"",
                    " stroke=\"var(--_b)\"",
                )
            };
        } else if pill {
            inner = format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\"{tint}/>",
                num(-w[i] / 2.),
                num(-h[i] / 2.),
                num(w[i]),
                num(h[i]),
                num(h[i] / 2.)
            );
        }
        let mut shift = 0.;
        if let Some(icon) = node.icon.as_ref().filter(|_| drawn[i]) {
            shift = (glyph + if label.width > 0. { ICON_GAP } else { 0. }) / 2.;
            inner += &super::super::common::icon::icon_svg(
                icon,
                -shift - label.width / 2.,
                dy[i] - glyph / 2.,
                glyph,
                r,
            )?;
        }
        let group=format!("<g class=\"pele-node pele-shape-{}{}{}\" data-id=\"{}\" transform=\"translate({},{})\">{inner}{}</g>",if shape.is_empty(){if pill{"branch"}else{"text"}}else{shape},if i>0{""}else{" pele-root"},class_names(node.class.as_deref().unwrap_or("")),esc_text(&node.node_id)?,num(cx),num(top+h[i]/2.),if down&&shape.is_empty(){left_label(label,shift-label.width/2.,dy[i],r)?}else{label_svg_with_resources(label,shift,dy[i]," class=\"pele-label\"",r)?});
        if i > 0 {
            body += &group;
        } else {
            out += &group;
        }
    }
    let title_room = o
        .max_width
        .filter(|v| *v > 0.)
        .map_or(4000., |v| 80f64.max(v - 2. * pad));
    let title = layout_label(m.title.as_deref(), false, r, o, size, title_room, 1)?;
    let title_height = if title.height > 0. {
        title.height + 16.
    } else {
        0.
    };
    let width = (max_x - min_x).max(title.width);
    let total_w = (width + 2. * pad).ceil();
    let total_h = (max_y - min_y + title_height + 2. * pad).ceil();
    let content = label_svg(
        &title,
        total_w / 2.,
        pad + title.height / 2.,
        " class=\"pele-title\" font-weight=\"var(--_tw)\"",
    ) + &format!(
        "<g transform=\"translate({},{})\">{out}</g>",
        num(pad - min_x + (width - max_x + min_x) / 2.),
        num(pad + title_height - min_y)
    );
    Ok(Rendered {
        svg: document("mindmap", total_w, total_h, size, o, None, None, &content)?,
        width: total_w,
        height: total_h,
        links: Vec::new(),
    })
}
