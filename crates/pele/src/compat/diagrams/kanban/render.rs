//! Pele kanban/render.ts, 458322a5 (MIT, kepano).
use super::super::common::icon::icon_svg;
use super::{KanbanModel, KanbanNode};
use crate::compat::{
    source::{
        space,
        url::{link_url, rel_attr, safe_url},
    },
    svg::{
        document, edges::theme::class_names, esc, esc_text, js_num, label_svg,
        label_svg_with_resources, num,
    },
    text::{layout_label, measure, Label, Span},
    LinkInfo, Object, PeleError, RenderResources, RenderSettings, Rendered, Value,
};
const RADIUS: &str = "var(--pele-radius,4px)";
fn fit(
    raw: Option<&str>,
    md: bool,
    r: &mut dyn RenderResources,
    o: &RenderSettings,
    size: f64,
    max: f64,
    base: u8,
) -> Result<Label, PeleError> {
    let mut label = layout_label(raw, md, r, o, size, max, base)?;
    if label.width <= max {
        return Ok(label);
    }
    let mut lines = Vec::new();
    let mut widths = Vec::new();
    let mut widest = 0f64;
    let mut line = Vec::new();
    let mut width = 0.;
    fn commit(
        lines: &mut Vec<Vec<Span>>,
        widths: &mut Vec<f64>,
        widest: &mut f64,
        line: &mut Vec<Span>,
        width: &mut f64,
    ) {
        lines.push(std::mem::take(line));
        widths.push(*width);
        *widest = widest.max(*width);
        *width = 0.;
    }
    for (i, original) in label.lines.iter().enumerate() {
        if label.widths[i] <= max {
            line = original.clone();
            width = label.widths[i];
            commit(&mut lines, &mut widths, &mut widest, &mut line, &mut width);
            continue;
        }
        for span in original {
            if span.icon.is_some() {
                if width > 0. && width + span.width > max {
                    commit(&mut lines, &mut widths, &mut widest, &mut line, &mut width)
                }
                line.push(span.clone());
                width += span.width;
                continue;
            }
            let mut text = String::new();
            let mut from = width;
            for ch in span.text.chars() {
                let w = measure(r, o, &ch.to_string(), size, span.style)?;
                if width > 0. && width + w > max {
                    if !text.is_empty() {
                        line.push(Span {
                            text: std::mem::take(&mut text),
                            style: span.style,
                            width: width - from,
                            icon: None,
                        })
                    }
                    commit(&mut lines, &mut widths, &mut widest, &mut line, &mut width);
                    from = 0.
                }
                text.push(ch);
                width += w
            }
            if !text.is_empty() {
                line.push(Span {
                    text,
                    style: span.style,
                    width: width - from,
                    icon: None,
                })
            }
        }
        commit(&mut lines, &mut widths, &mut widest, &mut line, &mut width)
    }
    label.lines = lines;
    label.widths = widths;
    label.width = widest;
    label.height = label.lines.len() as f64 * label.line_height;
    Ok(label)
}
fn text_block(
    label: &Label,
    x: f64,
    y: f64,
    attrs: &str,
    r: &mut dyn RenderResources,
    end: bool,
) -> Result<String, PeleError> {
    let mut out = String::new();
    for (i, line) in label.lines.iter().enumerate() {
        let cy = y + (i as f64 + 0.5) * label.line_height;
        if line.len() == 1 && line[0].icon.is_none() {
            let mut a = attrs.to_string();
            if line[0].style & 1 != 0 && !a.contains(" font-weight=\"") {
                a.push_str(" font-weight=\"var(--_w)\"")
            }
            if line[0].style & 2 != 0 && !a.contains(" font-style=\"") {
                a.push_str(" font-style=\"italic\"")
            }
            out.push_str(&format!(
                "<text{a} x=\"{}\" y=\"{}\"{}>{}</text>",
                num(x),
                num(cy + label.size * 0.35),
                if end { " text-anchor=\"end\"" } else { "" },
                esc(&line[0].text)
            ))
        } else {
            let width = label.widths[i];
            let mut one = label.clone();
            one.lines = vec![line.clone()];
            one.widths = vec![width];
            one.width = width;
            one.height = label.line_height;
            out.push_str(&label_svg_with_resources(
                &one,
                x + if end { -width } else { width } / 2.,
                cy,
                attrs,
                r,
            )?);
        }
    }
    Ok(out)
}
fn has(name: Option<&str>, r: &mut dyn RenderResources) -> Result<bool, PeleError> {
    if let Some(name) = name.filter(|s| !s.is_empty()) {
        Ok(r.icon(name)?.is_some_and(|s| !s.markup().is_empty()))
    } else {
        Ok(false)
    }
}
struct Card<'a> {
    item: &'a KanbanNode,
    label: Label,
    ticket: Label,
    assigned: Label,
    meta: f64,
    height: f64,
}
pub(super) fn render(
    m: &KanbanModel,
    c: &Object,
    o: &RenderSettings,
    r: &mut dyn RenderResources,
) -> Result<Rendered, PeleError> {
    let size = o.font_size.unwrap_or(16.);
    let small = (size * 0.875 + 0.5).floor();
    let line = (size * 1.5 + 0.5).floor();
    let small_line = (small * 1.5 + 0.5).floor();
    let pad = o.padding.unwrap_or(8.);
    let conf = c.get("kanban").and_then(Value::as_object);
    let own = match conf.and_then(|c| c.get("sectionWidth")) {
        Some(Value::Number(n)) if *n > 0. => n.clamp(80., 2000.),
        _ => 200.,
    };
    let room = o
        .max_width
        .filter(|n| *n > 0.)
        .map_or(f64::INFINITY, |n| n - 2. * pad);
    let count = m.sections.len();
    let fits = count as f64 * (own + 12.) - 12. <= room;
    let per_row = if fits {
        count.max(1)
    } else {
        (((room + 12.) / (own + 12.)).floor().max(1.)) as usize
    };
    let column_width = if fits {
        own
    } else {
        own.min(room)
            .max((((room + 12.) / per_row as f64).floor() - 12.).min(own * 2.))
    };
    let base_url = match conf.and_then(|c| c.get("ticketBaseUrl")) {
        Some(Value::String(s)) => s.as_str(),
        _ => "",
    };
    let card_width = column_width - 16.;
    let text_width = card_width - 20.;
    let glyph = small * 1.25;
    let mut links = Vec::new();
    let mut titles = Vec::new();
    let mut count_widths = Vec::new();
    let mut cards = Vec::new();
    let mut head_height = line;
    let mut body_heights = Vec::new();
    for section in &m.sections {
        let count_width = measure(r, o, &section.items.len().to_string(), small, 0)?;
        let reserved = count_width
            + 12.
            + if has(section.node.icon.as_deref(), r)? {
                glyph + 6.
            } else {
                0.
            };
        let title = fit(
            Some(&section.node.label),
            true,
            r,
            o,
            size,
            card_width - 8. - reserved,
            1,
        )?;
        head_height = head_height.max(title.height);
        count_widths.push(count_width);
        titles.push(title);
        let mut list = Vec::new();
        let mut height = 0.;
        for item in &section.items {
            let icon_room = if has(item.icon.as_deref(), r)? {
                glyph + 6.
            } else {
                0.
            };
            let label = fit(
                Some(&item.label),
                true,
                r,
                o,
                small,
                text_width - icon_room,
                0,
            )?;
            let ticket = fit(item.ticket.as_deref(), false, r, o, small, text_width, 0)?;
            let assigned = fit(item.assigned.as_deref(), false, r, o, small, text_width, 0)?;
            let meta = if ticket.width + assigned.width + 8. > text_width {
                ticket.height + assigned.height
            } else {
                ticket.height.max(assigned.height)
            };
            let card_height =
                20. + label.height.max(small_line) + if meta != 0. { 4. + meta } else { 0. };
            list.push(Card {
                item,
                label,
                ticket,
                assigned,
                meta,
                height: card_height,
            });
            height += card_height + 8.
        }
        cards.push(list);
        body_heights.push(height)
    }
    head_height += 16.;
    let mut row_heights = Vec::new();
    let mut row_tops = Vec::new();
    let mut y = 0.;
    for first in (0..count).step_by(per_row) {
        let tallest = body_heights[first..(first + per_row).min(count)]
            .iter()
            .copied()
            .fold(8., f64::max);
        row_tops.push(y);
        row_heights.push(head_height + tallest);
        y += head_height + tallest + 12.
    }
    let board_height = row_tops
        .last()
        .zip(row_heights.last())
        .map_or(0., |(a, b)| a + b);
    let title_room = o
        .max_width
        .filter(|n| *n > 0.)
        .map_or(4000., |n| (n - 2. * pad).max(80.));
    let title = layout_label(m.title.as_deref(), false, r, o, size, title_room, 1)?;
    let title_height = if title.height > 0. {
        title.height + 16.
    } else {
        0.
    };
    let board_width = (count.min(per_row) as f64 * (column_width + 12.) - 12.).max(0.);
    let width = board_width.max(title.width);
    let left = pad + (width - board_width) / 2.;
    let edge = column_width - 12.;
    let mut out = String::new();
    for (index, section) in m.sections.iter().enumerate() {
        let row = index / per_row;
        let mut body = format!(
            "<rect width=\"{}\" height=\"{}\" rx=\"{RADIUS}\" fill=\"var(--_s)\"/>{}<text class=\"pele-count\" x=\"{}\" y=\"{}\" text-anchor=\"end\" font-size=\"{}\" fill=\"var(--_m)\">{}</text>",
            js_num(column_width),
            num(row_heights[row]),
            text_block(
                &titles[index],
                12.,
                8.,
                " class=\"pele-cluster-label\" font-weight=\"var(--_hw)\"",
                r,
                false
            )?,
            js_num(edge),
            num(8. + line / 2. + small * 0.35),
            js_num(small),
            section.items.len()
        );
        if has(section.node.icon.as_deref(), r)? {
            body.push_str(&icon_svg(
                section.node.icon.as_ref().unwrap(),
                edge - count_widths[index] - 6. - glyph,
                8. + (line - glyph) / 2.,
                glyph,
                r,
            )?)
        }
        let mut y = head_height;
        for card in &cards[index] {
            let item = card.item;
            let priority = item
                .priority
                .as_deref()
                .unwrap_or("")
                .trim_matches(space)
                .to_lowercase();
            let color = match priority.as_str() {
                "very high" => Some("stroke=\"var(--pele-critical,#e45756)\""),
                "high" => Some("stroke=\"var(--pele-critical,#e45756)\" opacity=\"0.5\""),
                "low" => Some("stroke=\"var(--_m)\" opacity=\"0.5\""),
                "very low" => Some("stroke=\"var(--_m)\""),
                _ => None,
            };
            let mut inner = format!(
                "<rect width=\"{}\" height=\"{}\" rx=\"{RADIUS}\" fill=\"var(--_bg)\" stroke=\"var(--_b)\"/>",
                js_num(card_width),
                num(card.height)
            );
            if let Some(color) = color {
                inner.push_str(&format!("<path class=\"pele-priority\" d=\"M2.5,5V{}\" fill=\"none\" {color} stroke-width=\"3\" stroke-linecap=\"round\"/>",num(card.height-5.)))
            }
            inner.push_str(&text_block(
                &card.label,
                10.,
                10.,
                " class=\"pele-label\"",
                r,
                false,
            )?);
            if has(item.icon.as_deref(), r)? {
                inner.push_str(&icon_svg(
                    item.icon.as_ref().unwrap(),
                    card_width - 10. - glyph,
                    10. + (small_line - glyph) / 2.,
                    glyph,
                    r,
                )?)
            }
            let bottom = card.height - 10.;
            if let Some(ticket) = item.ticket.as_deref().filter(|s| !s.is_empty()) {
                let top = bottom - card.meta;
                let href = if base_url.is_empty() {
                    None
                } else {
                    link_url(&safe_url(&base_url.replacen("#TICKET#", ticket, 1))?, o)
                };
                if let Some(href) = href {
                    inner.push_str(&format!(
                        "<a href=\"{}\" target=\"_blank\"{}>{}</a>",
                        esc(&href),
                        rel_attr(o),
                        text_block(
                            &card.ticket,
                            10.,
                            top,
                            " class=\"pele-ticket\" fill=\"var(--_c)\"",
                            r,
                            false
                        )?
                    ));
                    links.push(LinkInfo {
                        id: item.id.clone(),
                        href,
                        internal: false,
                    })
                } else {
                    inner.push_str(&text_block(
                        &card.ticket,
                        10.,
                        top,
                        " class=\"pele-ticket\" fill=\"var(--_m)\"",
                        r,
                        false,
                    )?);
                }
            }
            if item.assigned.as_deref().is_some_and(|s| !s.is_empty()) {
                inner.push_str(&text_block(
                    &card.assigned,
                    card_width - 10.,
                    bottom - card.assigned.height,
                    " class=\"pele-assigned\" fill=\"var(--_m)\"",
                    r,
                    true,
                )?)
            }
            body.push_str(&format!("<g class=\"pele-node pele-card{}{}\" data-id=\"{}\" transform=\"translate(8,{})\" font-size=\"{}\">{inner}</g>",if color.is_some(){format!(" pele-priority-{}",priority.replacen(' ',"-",1))}else{String::new()},class_names(item.css_classes.as_deref().unwrap_or("")),esc_text(&item.id)?,num(y),js_num(small)));
            y += card.height + 8.;
        }
        out.push_str(&format!("<g class=\"pele-cluster pele-column{}\" data-id=\"{}\" transform=\"translate({},{})\">{body}</g>",class_names(section.node.css_classes.as_deref().unwrap_or("")),esc_text(&section.node.id)?,num(left+(index%per_row)as f64*(column_width+12.)),num(pad+title_height+row_tops[row])));
    }
    let total_width = (width + 2. * pad).ceil();
    let total_height = (board_height + title_height + 2. * pad).ceil();
    let content = label_svg(
        &title,
        total_width / 2.,
        pad + title.height / 2.,
        " class=\"pele-title\" font-weight=\"var(--_tw)\"",
    ) + &out;
    Ok(Rendered {
        svg: document(
            "kanban",
            total_width,
            total_height,
            size,
            o,
            None,
            None,
            &content,
        )?,
        width: total_width,
        height: total_height,
        links,
    })
}
