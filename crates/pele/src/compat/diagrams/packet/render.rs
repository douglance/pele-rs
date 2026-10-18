//! Pele packet/render.ts and common/{literal-label,fit-text,fit-width}.ts, 458322a5 (MIT, kepano).
use super::{packet_option, PacketBlock, PacketModel};
use crate::compat::{
    source::{decode_entities, space},
    svg::{self, esc, js_num, label_svg, num},
    text::{layout_label, measure, Label, Span},
    Object, PeleError, RenderResources, RenderSettings, Rendered, Value,
};
fn round(n: f64) -> f64 {
    (n + 0.5).floor()
}
fn literal(
    text: &str,
    r: &mut dyn RenderResources,
    o: &RenderSettings,
    size: f64,
    room: f64,
) -> Result<Label, PeleError> {
    let mut lines = Vec::new();
    for raw in text.split('\n') {
        let chars: Vec<char> = raw.chars().collect();
        if chars.is_empty() {
            lines.push(String::new())
        }
        let mut from = 0;
        while from < chars.len() {
            let fits = |n: usize, r: &mut dyn RenderResources| {
                measure(
                    r,
                    o,
                    &chars[from..from + n].iter().collect::<String>(),
                    size,
                    0,
                )
                .map(|w| w <= room)
            };
            let mut hi = 1;
            while hi < chars.len() - from && fits(hi, r)? {
                hi = (chars.len() - from).min(hi * 2)
            }
            let mut lo = 1;
            while lo < hi {
                let mid = (lo + hi + 1) / 2;
                if fits(mid, r)? {
                    lo = mid
                } else {
                    hi = mid - 1
                }
            }
            let mut take = lo;
            let wraps = from + take < chars.len();
            if wraps && !space(chars[from + take]) {
                for i in (1..take).rev() {
                    if space(chars[from + i]) {
                        take = i + 1;
                        break;
                    }
                }
            }
            let line: String = chars[from..from + take].iter().collect();
            let trimmed = line.trim_end_matches(space);
            lines.push(if wraps && !trimmed.is_empty() {
                trimmed.into()
            } else {
                line
            });
            from += take;
            while wraps && from < chars.len() && space(chars[from]) {
                from += 1
            }
        }
    }
    if text.is_empty() {
        lines.clear()
    }
    let line_height = round(size * 1.5);
    let mut out = Label {
        size,
        line_height,
        ..Label::default()
    };
    for line in lines {
        let width = measure(r, o, &line, size, 0)?;
        out.width = out.width.max(width);
        out.widths.push(width);
        out.lines.push(vec![Span {
            text: line,
            style: 0,
            icon: None,
            width,
        }]);
    }
    out.height = out.lines.len() as f64 * line_height;
    Ok(out)
}
fn fit(
    text: &str,
    max: f64,
    size: f64,
    min: f64,
    r: &mut dyn RenderResources,
    o: &RenderSettings,
) -> Result<Option<(String, f64)>, PeleError> {
    if text.is_empty() || max <= 0. {
        return Ok(None);
    }
    let width = measure(r, o, text, size, 0)?;
    if width <= max {
        return Ok(Some((text.into(), size)));
    }
    let mut fitted = min.max((size * max / width).floor());
    while fitted >= min {
        if measure(r, o, text, fitted, 0)? <= max {
            return Ok(Some((text.into(), fitted)));
        }
        fitted -= 1.;
    }
    let chars: Vec<char> = text.chars().collect();
    let mut low = 0;
    let mut high = chars.len() - 1;
    while low < high {
        let mid = (low + high + 1) / 2;
        let s = chars[..mid]
            .iter()
            .collect::<String>()
            .trim_end_matches(space)
            .to_string()
            + "…";
        if measure(r, o, &s, min, 0)? <= max {
            low = mid
        } else {
            high = mid - 1
        }
    }
    if low == 0 {
        return Ok(None);
    }
    let cut = chars[..low]
        .iter()
        .collect::<String>()
        .trim_end_matches(space)
        .to_string()
        + "…";
    Ok(if measure(r, o, &cut, min, 0)? <= max {
        Some((cut, min))
    } else {
        None
    })
}
pub(super) fn render(
    m: &PacketModel,
    c: &Object,
    o: &RenderSettings,
    r: &mut dyn RenderResources,
) -> Result<Rendered, PeleError> {
    let across = draw(m, c, o, r, false)?;
    let Some(available) = o.max_width.filter(|w| *w > 0.) else {
        return Ok(across);
    };
    let breakpoint = o.direction_breakpoint.filter(|v| *v > 0.).unwrap_or(640.);
    if o.auto_direction == Some(false) || across.width <= available || available >= breakpoint {
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
    m: &PacketModel,
    c: &Object,
    o: &RenderSettings,
    r: &mut dyn RenderResources,
    down: bool,
) -> Result<Rendered, PeleError> {
    let size = o.font_size.unwrap_or(16.);
    let label_size = round(size * 0.875);
    let bit_size = round(size * 0.625);
    let pad = o.padding.unwrap_or(8.);
    let packet = c.get("packet").and_then(Value::as_object);
    let row_height = packet_option(c, "rowHeight", 32., 1., 400.);
    let mut bit_width = packet_option(c, "bitWidth", 32., 1., 400.);
    let padding_x = packet_option(c, "paddingX", 5., 0., 400.);
    let padding_y = packet_option(c, "paddingY", 5., 0., 400.);
    let show_bits = packet.and_then(|p| p.get("showBits")) != Some(&Value::Bool(false));
    let descending =
        packet.and_then(|p| p.get("bitOrder")) == Some(&Value::String("descending".into()));
    let mut bits_per_row = m.bits_per_row;
    let mut rows = m.rows.clone();
    if down {
        let count = ((o.max_width.unwrap_or(320.) - 2. * pad) / bit_width.min(18.))
            .floor()
            .max(1.);
        bits_per_row = bits_per_row.min(2f64.powf(count.log2().floor()).max(8.));
        let last = m.rows.last().and_then(|r| r.last()).map_or(0., |b| b.end);
        if ((last + 1.) / bits_per_row).ceil() > 10000. {
            bits_per_row = m.bits_per_row
        }
        bit_width =
            bit_width.min(((o.max_width.unwrap_or(320.) - 2. * pad) / bits_per_row).max(18.));
        let mut whole: Vec<PacketBlock> = Vec::new();
        for b in m.rows.iter().flatten() {
            if let Some(prev) = whole.last_mut().filter(|p| {
                p.bits == p.end - p.start
                    && b.bits == b.end - b.start
                    && p.label == b.label
                    && b.start == p.end + 1.
                    && b.start % m.bits_per_row == 0.
            }) {
                prev.end = b.end;
                prev.bits = prev.end - prev.start
            } else {
                whole.push(b.clone())
            }
        }
        rows.clear();
        for block in whole {
            let mut start = block.start;
            while start <= block.end {
                let end = block
                    .end
                    .min(((start / bits_per_row).floor() + 1.) * bits_per_row - 1.);
                let index = (start / bits_per_row).floor() as usize;
                if rows.len() <= index {
                    rows.resize_with(index + 1, Vec::new)
                }
                rows[index].push(PacketBlock {
                    start,
                    end,
                    ..block.clone()
                });
                start = end + 1.;
            }
        }
    }
    let mut labels: Vec<Vec<Label>> = Vec::new();
    if down {
        for row in &rows {
            let mut line = Vec::new();
            for block in row {
                let room = ((block.end - block.start + 1.) * bit_width
                    - padding_x.min(bit_width - 1.)
                    - 6.)
                    .max(1.);
                line.push(literal(
                    decode_entities(&block.label)?.trim_matches(space),
                    r,
                    o,
                    label_size,
                    room,
                )?)
            }
            labels.push(line)
        }
    }
    let numbers = if show_bits { bit_size + 4. } else { 0. };
    let mut heights = Vec::new();
    let mut tops = Vec::new();
    let mut rows_height = 0.;
    for (i, _) in rows.iter().enumerate() {
        let mut height = row_height;
        if down {
            for label in &labels[i] {
                height = height.max(label.height + 10.)
            }
        }
        tops.push(rows_height);
        rows_height += numbers + height + padding_y;
        heights.push(height);
    }
    if !rows.is_empty() {
        rows_height -= padding_y
    }
    let gap = padding_x.min(bit_width - 1.);
    let width = if !rows.is_empty() {
        bits_per_row * bit_width - gap
    } else {
        0.
    };
    let title_room = if down {
        o.max_width
            .filter(|v| *v > 0.)
            .map_or(4000., |v| (v - 2. * pad).max(80.))
    } else {
        4000.
    };
    let title = layout_label(m.title.as_deref(), false, r, o, size, title_room, 1)?;
    let title_height = if title.height > 0. {
        title.height + if rows_height > 0. { 12. } else { 0. }
    } else {
        0.
    };
    let left = pad + ((title.width - width) / 2.).max(0.);
    let mut fields = String::new();
    let mut bits = String::new();
    let mut fits: Vec<(std::sync::Arc<str>, f64, Option<(String, f64)>)> = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        let y = pad + tops[i] + numbers;
        let height = heights[i];
        for (j, block) in row.iter().enumerate() {
            let count = block.end - block.start + 1.;
            let first = block.start % bits_per_row;
            let x = left
                + if descending {
                    bits_per_row - first - count
                } else {
                    first
                } * bit_width;
            let w = count * bit_width - gap;
            let fitted = if down {
                None
            } else {
                let max = w - 6.;
                if let Some((_, _, f)) =
                    fits.iter().find(|(l, v, _)| l == &block.label && *v == max)
                {
                    f.clone()
                } else {
                    let f = fit(
                        decode_entities(&block.label)?.trim_matches(space),
                        max,
                        label_size,
                        round(size * 0.5),
                        r,
                        o,
                    )?;
                    fits.push((block.label.clone(), max, f.clone()));
                    f
                }
            };
            let text = if down {
                label_svg(&labels[i][j], x + w / 2., y + height / 2., "")
            } else if let Some((text, font)) = fitted.filter(|(_, font)| *font <= height) {
                format!(
                    "<text x=\"{}\" y=\"{}\"{}>{}</text>",
                    num(x + w / 2.),
                    num(y + height / 2. + font * 0.35),
                    if font == label_size {
                        String::new()
                    } else {
                        format!(" font-size=\"{}\"", num(font))
                    },
                    esc(&text)
                )
            } else {
                String::new()
            };
            fields.push_str(&format!("<g class=\"pele-node\" data-id=\"{}-{}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"var(--pele-radius,4px)\" fill=\"var(--_s)\" stroke=\"var(--_b)\"/>{text}</g>",js_num(block.start),js_num(block.end),num(x),num(y),num(w),num(height)));
            if !show_bits {
                continue;
            }
            let leading = if descending { block.end } else { block.start };
            let trailing = if descending { block.start } else { block.end };
            if count == 1. {
                bits.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\">{}</text>",
                    num(x + w / 2.),
                    num(y - 4.),
                    js_num(leading)
                ))
            } else {
                bits.push_str(&format!("<text x=\"{}\" y=\"{}\">{}</text><text x=\"{}\" y=\"{}\" text-anchor=\"end\">{}</text>",num(x),num(y-4.),js_num(leading),num(x+w),num(y-4.),js_num(trailing)))
            }
        }
    }
    let total_width = (width.max(title.width) + 2. * pad).ceil();
    let total_height = (rows_height + title_height + 2. * pad).ceil();
    let content = format!(
        "{}{}{}",
        if fields.is_empty() {
            String::new()
        } else {
            format!("<g class=\"pele-packet-fields\" font-size=\"{}\" text-anchor=\"middle\">{fields}</g>",js_num(label_size))
        },
        if bits.is_empty() {
            String::new()
        } else {
            format!(
                "<g class=\"pele-packet-bits\" font-size=\"{}\" fill=\"var(--_m)\">{bits}</g>",
                js_num(bit_size)
            )
        },
        label_svg(
            &title,
            total_width / 2.,
            total_height - pad - title.height / 2.,
            " class=\"pele-title\" font-weight=\"var(--_tw)\""
        )
    );
    Ok(Rendered {
        svg: svg::document(
            "packet",
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
        links: Vec::new(),
    })
}
