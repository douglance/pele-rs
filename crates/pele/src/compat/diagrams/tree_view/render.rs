//! Dedicated TreeView row/guide renderer, Pele treeview/render.ts, 458322a5 (MIT).
use super::{icons, NodeType, TreeViewModel};
use crate::compat::{
    diagrams::common::literal_label::{left_label_svg, literal_label},
    source::{decode_entities, space},
    svg::{document, edges::theme::class_names, esc, esc_text, label_svg, num},
    text::{layout_label, measure, Label},
    Object, PeleError, RenderResources, RenderSettings, Rendered, Value,
};
fn setting(config: &Object, key: &str, fallback: f64, min: f64, max: f64) -> f64 {
    match config
        .get("treeView")
        .and_then(Value::as_object)
        .and_then(|c| c.get(key))
    {
        Some(Value::Number(v)) if *v >= min && *v <= max => *v,
        _ => fallback,
    }
}
struct Row {
    node: usize,
    x: f64,
    cy: f64,
    icon: Option<String>,
    text: String,
    label_x: f64,
    highlighted: bool,
    label: Option<Label>,
    description: Option<Label>,
    top: f64,
    height: f64,
}
pub(super) fn render(
    m: &TreeViewModel,
    c: &Object,
    o: &RenderSettings,
    r: &mut dyn RenderResources,
) -> Result<Rendered, PeleError> {
    let loose = draw(m, c, o, r, false)?;
    if o.max_width.is_none_or(|w| !(w > 0.) || loose.width <= w) {
        return Ok(loose);
    }
    let tight = draw(m, c, o, r, true)?;
    Ok(if tight.width < loose.width {
        tight
    } else {
        loose
    })
}
fn draw(
    m: &TreeViewModel,
    c: &Object,
    o: &RenderSettings,
    r: &mut dyn RenderResources,
    tight: bool,
) -> Result<Rendered, PeleError> {
    let size = o.font_size.unwrap_or(16.);
    let pad = o.padding.unwrap_or(8.);
    let icons = icons::Config::new(c);
    let padding_x = setting(c, "paddingX", 5., 0., 100.);
    let padding_y = setting(c, "paddingY", 2., 0., 100.);
    let row_indent = setting(c, "rowIndent", 15., 0., 400.);
    let thickness = setting(c, "lineThickness", 1., 0.25, 12.);
    let mut max_depth = 0;
    let mut depths = m.nodes[m.root]
        .children
        .iter()
        .map(|&i| (i, 0usize))
        .collect::<Vec<_>>();
    let mut i = 0;
    while tight && i < depths.len() {
        let (node, depth) = depths[i];
        max_depth = max_depth.max(depth);
        for &child in &m.nodes[node].children {
            depths.push((child, depth + 1));
        }
        i += 1;
    }
    let step = if tight {
        (row_indent + padding_x)
            .min(2_f64.max((o.max_width.unwrap_or(320.) - 120.) / max_depth.max(1) as f64))
    } else {
        row_indent + padding_x
    };
    let row_height = (size * 1.5 + 0.5).floor() + 2. * padding_y;
    let room = if tight {
        o.max_width
            .filter(|v| *v > 0.)
            .map_or(4000., |v| 80_f64.max(v - 2. * pad))
    } else {
        4000.
    };
    let title = layout_label(m.title.as_deref(), false, r, o, size, room, 1)?;
    let title_height = if title.height > 0. {
        title.height + 12.
    } else {
        0.
    };
    let mut rows = vec![];
    let mut row_of = vec![0; m.nodes.len()];
    let mut pending = vec![];
    let push = |list: &[usize], depth: usize, pending: &mut Vec<(usize, usize, bool)>| {
        let slot = list
            .iter()
            .any(|&i| icons::get(&m.nodes[i], &icons).is_some());
        for &i in list.iter().rev() {
            pending.push((i, depth, slot));
        }
    };
    push(&m.nodes[m.root].children, 0, &mut pending);
    let mut label_right = 0.;
    let mut any_highlight = false;
    let mut rows_height = 0.;
    while let Some((index, depth, slot)) = pending.pop() {
        let node = &m.nodes[index];
        let x = depth as f64 * step;
        let text = decode_entities(&node.name)?;
        let label_x = x + if slot { size + 6. } else { 0. };
        let room = 40_f64.max(o.max_width.unwrap_or(320.) - 2. * pad - 8. - label_x);
        let label = if tight {
            Some(literal_label(
                &text,
                r,
                o,
                size,
                room,
                if node.node_type == NodeType::Directory {
                    1
                } else {
                    0
                },
            )?)
        } else {
            None
        };
        let description = if tight {
            node.description
                .as_deref()
                .filter(|s| !s.is_empty())
                .map(|s| decode_entities(s).and_then(|s| literal_label(&s, r, o, size, room, 2)))
                .transpose()?
        } else {
            None
        };
        let width = if let Some(label) = &label {
            label.width
        } else {
            measure(
                r,
                o,
                &text,
                size,
                if node.node_type == NodeType::Directory {
                    1
                } else {
                    0
                },
            )?
        };
        if label_x + width > label_right {
            label_right = label_x + width;
        }
        let highlighted = node
            .css_class
            .as_deref()
            .is_some_and(|class| class.split(space).any(|name| name == "highlight"));
        if highlighted {
            any_highlight = true;
        }
        let label_height = label.as_ref().map_or(0., |l| l.height);
        let height = if tight {
            row_height
                .max(label_height + description.as_ref().map_or(0., |l| l.height) + 2. * padding_y)
        } else {
            row_height
        };
        let cy = rows_height
            + if tight {
                padding_y + size.max(label_height) / 2.
            } else {
                height / 2.
            };
        let row = Row {
            node: index,
            x,
            cy,
            icon: icons::get(node, &icons),
            text,
            label_x,
            highlighted,
            label,
            description,
            top: rows_height,
            height,
        };
        rows_height += height;
        row_of[index] = rows.len();
        rows.push(row);
        push(&node.children, depth + 1, &mut pending);
    }
    let desc_x = label_right + 16.;
    let mut width = label_right;
    for row in &rows {
        let Some(description) = m.nodes[row.node]
            .description
            .as_deref()
            .filter(|s| !s.is_empty())
        else {
            continue;
        };
        let w = if let Some(label) = &row.description {
            label.width
        } else {
            measure(r, o, &decode_entities(description)?, size, 2)?
        };
        width = width.max(if tight { row.label_x } else { desc_x } + w);
    }
    width = width.max(title.width);
    let inset = pad + if any_highlight { 4. } else { 0. };
    let top = pad + title_height;
    let total_width = (width + 2. * inset).ceil();
    let total_height = (top + rows_height + pad).ceil();
    let mut guides = String::new();
    for row in &rows {
        let children = &m.nodes[row.node].children;
        if children.is_empty() {
            continue;
        }
        let gx = inset + row.x + 7.;
        let tick = gx.max(inset + row.x + step - padding_x);
        let last = top + rows[row_of[*children.last().unwrap()]].cy;
        let radius = 4_f64.min(tick - gx);
        let mut d = format!(
            "M{},{}V{}",
            num(gx),
            num(top + row.top + row.height - 2.),
            num(last - radius)
        );
        if radius > 0. {
            d += &format!(
                "Q{},{} {},{}H{}",
                num(gx),
                num(last),
                num(gx + radius),
                num(last),
                num(tick)
            );
        }
        if tick > gx {
            for &child in children.iter().take(children.len() - 1) {
                d += &format!(
                    "M{},{}H{}",
                    num(gx),
                    num(top + rows[row_of[child]].cy),
                    num(tick)
                );
            }
        }
        guides += &format!("<path d=\"{d}\"/>");
    }
    let baseline = size * 0.35;
    let mut body = String::new();
    for row in &rows {
        let node = &m.nodes[row.node];
        let cy = top + row.cy;
        let directory = node.node_type == NodeType::Directory;
        let mut inner = String::new();
        if row.highlighted {
            let x = inset + row.x - 4.;
            inner+=&format!("<rect class=\"pele-tree-highlight\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"var(--pele-radius,4px)\" fill=\"var(--_c)\" fill-opacity=\"0.14\"/>",num(x),num(top+row.top+1.),num(total_width-inset+4.-x),num(row.height-2.));
        }
        if let Some(icon) = &row.icon {
            let drawn = r
                .direct_icon(icon)?
                .map(|v| v.markup().to_string())
                .unwrap_or_else(|| {
                    format!(
                        "<path d=\"{}\" stroke=\"var(--_m)\" stroke-width=\"1.5\"/>",
                        icons::glyph(icon, directory)
                    )
                });
            inner+=&format!("<svg class=\"pele-icon\" data-icon=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\">{drawn}</svg>",esc_text(icon)?,num(inset+row.x),num(cy-size/2.),num(size),num(size));
        }
        let bold = if directory {
            " font-weight=\"var(--_hw)\""
        } else {
            ""
        };
        if let Some(label) = &row.label {
            inner += &left_label_svg(
                label,
                inset + row.label_x,
                top + row.top + padding_y,
                &format!(" class=\"pele-label\"{bold}"),
            );
        } else if !row.text.is_empty() {
            inner+=&format!("<text class=\"pele-label\" x=\"{}\" y=\"{}\"{bold} xml:space=\"preserve\">{}</text>",num(inset+row.label_x),num(cy+baseline),esc(&row.text));
        }
        if let Some(description) = &row.description {
            inner += &left_label_svg(
                description,
                inset + row.label_x,
                top + row.top + padding_y + row.label.as_ref().map_or(0., |l| l.height),
                " class=\"pele-tree-description\" font-style=\"italic\" fill=\"var(--_m)\"",
            );
        } else if let Some(description) = node.description.as_deref().filter(|s| !s.is_empty()) {
            inner+=&format!("<text class=\"pele-tree-description\" x=\"{}\" y=\"{}\" font-style=\"italic\" fill=\"var(--_m)\" xml:space=\"preserve\">{}</text>",num(inset+desc_x),num(cy+baseline),esc(&decode_entities(description)?));
        }
        body += &format!(
            "<g class=\"pele-node pele-tree-{}{}\" data-id=\"{}\">{inner}</g>",
            if directory { "dir" } else { "file" },
            class_names(node.css_class.as_deref().unwrap_or("")),
            esc_text(&node.name)?
        );
    }
    let content = format!(
        "{}{}{body}",
        label_svg(
            &title,
            total_width / 2.,
            pad + title.height / 2.,
            " class=\"pele-title\" font-weight=\"var(--_tw)\""
        ),
        if guides.is_empty() {
            String::new()
        } else {
            format!("<g class=\"pele-tree-guides\" fill=\"none\" stroke=\"var(--_b)\" stroke-width=\"{}\">{guides}</g>",num(thickness))
        }
    );
    Ok(Rendered {
        svg: document(
            "treeView",
            total_width,
            total_height,
            size,
            o,
            m.acc_title.as_deref(),
            m.acc_descr.as_deref(),
            &content,
        )?,
        width: total_width,
        height: total_height,
        links: vec![],
    })
}
