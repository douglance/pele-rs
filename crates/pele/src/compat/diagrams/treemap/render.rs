//! Pele treemap/render.ts and common/fit-text.ts, 458322a5 (MIT, kepano).
use super::{format::Formatter, layout, TreemapModel};
use crate::compat::{
    source::{decode_entities, space},
    svg::{
        self,
        edges::theme::{class_names, resolve_style},
        esc, esc_text, js_num, label_svg, num, series_color,
    },
    text::{layout_label, measure},
    Object, PeleError, RenderResources, RenderSettings, Rendered, Value,
};
fn option(c: Option<&Object>, key: &str, fallback: f64, min: f64, max: f64) -> f64 {
    match c.and_then(|c| c.get(key)) {
        Some(Value::Number(n)) if *n >= min => n.min(max),
        _ => fallback,
    }
}
fn fitted(
    text: &str,
    max: f64,
    size: f64,
    min: f64,
    style: u8,
    r: &mut dyn RenderResources,
    o: &RenderSettings,
) -> Result<Option<(String, f64)>, PeleError> {
    if text.is_empty() || max <= 0. {
        return Ok(None);
    }
    let mut width = measure(r, o, text, size, style)?;
    if width <= max {
        return Ok(Some((text.into(), size)));
    }
    let mut fitted = min.max((size * max / width).floor());
    while fitted >= min {
        width = measure(r, o, text, fitted, style)?;
        if width <= max {
            return Ok(Some((text.into(), fitted)));
        }
        fitted -= 1.;
    }
    let chars: Vec<_> = text.chars().collect();
    let (mut low, mut high) = (0, chars.len() - 1);
    while low < high {
        let mid = (low + high + 1) / 2;
        let cut = chars[..mid]
            .iter()
            .collect::<String>()
            .trim_end_matches(space)
            .to_string()
            + "…";
        if measure(r, o, &cut, min, style)? <= max {
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
    Ok(if measure(r, o, &cut, min, style)? <= max {
        Some((cut, min))
    } else {
        None
    })
}
pub(super) fn render(
    m: &TreemapModel,
    c: &Object,
    o: &RenderSettings,
    r: &mut dyn RenderResources,
) -> Result<Rendered, PeleError> {
    let size = o.font_size.unwrap_or(16.);
    let round = |n: f64| (n + 0.5).floor();
    let small = round(size * 0.875);
    let tiny = round(size * 0.75);
    let least = round(size * 0.5625);
    let conf = match c.get("treemap") {
        Some(Value::Object(v)) => Some(v),
        _ => None,
    };
    let pad = o
        .padding
        .unwrap_or_else(|| option(conf, "diagramPadding", 8., 0., 2000.));
    let natural = option(conf, "nodeWidth", 64., 1., 400.) * 10.;
    let width = round(if let Some(available) = o.max_width.filter(|v| *v > 0.) {
        natural.min(available - 2. * pad).max(200_f64.min(natural))
    } else {
        natural
    });
    let height = round(option(conf, "nodeHeight", 40., 1., 400.) * 10.);
    let show = conf.and_then(|c| c.get("showValues")) != Some(&Value::Bool(false));
    let spec = match conf.and_then(|c| c.get("valueFormat")) {
        Some(Value::String(s)) => s.as_str(),
        _ => ",",
    };
    let format = Formatter::new(spec);
    let boxes = layout::layout(
        &m.roots,
        width,
        height,
        option(conf, "padding", 4., 0., 200.),
    );
    let room = o
        .max_width
        .filter(|v| *v > 0.)
        .map_or(4000., |v| 80_f64.max(v - 2. * o.padding.unwrap_or(8.)));
    let title = layout_label(m.title.as_deref(), false, r, o, size, room, 1)?;
    let title_height = if title.height > 0. {
        title.height + 8.
    } else {
        0.
    };
    let left = pad + 0_f64.max((title.width - width) / 2.);
    let top = pad + title_height;
    let (mut sections, mut leaves) = (String::new(), String::new());
    for b in boxes {
        let node = b.node;
        let style = resolve_style(
            node.css_compiled_styles
                .as_deref()
                .map(Vec::as_slice)
                .unwrap_or(&[]),
        );
        let color = series_color(b.series);
        let x = left + b.x;
        let y = top + b.y;
        let name = decode_entities(&node.name)?;
        let name = name.trim_matches(space);
        let value = if show {
            format.format(b.value)
        } else {
            String::new()
        };
        let group = format!("data-id=\"{}\">", esc_text(&node.name)?);
        let rect = format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"var(--pele-radius,4px)\" fill=\"{color}\"",
            num(x),
            num(y),
            num(b.w),
            num(b.h)
        );
        let classes = class_names(node.class_selector.as_deref().unwrap_or(""));
        if node.children.is_some() {
            let mut text = String::new();
            if b.h >= 28. {
                let room = b.w - 16.;
                let value_width = if value.is_empty() {
                    0.
                } else {
                    measure(r, o, &value, tiny, 0)?
                };
                let with_value = value_width > 0. && room - value_width - 12. >= 3. * small;
                let label = fitted(
                    name,
                    if with_value {
                        room - value_width - 12.
                    } else {
                        room
                    },
                    small,
                    small,
                    1,
                    r,
                    o,
                )?;
                let baseline = y + 15.;
                if let Some((label, _)) = label {
                    text.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" font-weight=\"var(--_hw)\"{}>{}</text>",
                        num(x + 8.),
                        num(baseline + small * 0.35),
                        style.text,
                        esc(&label)
                    ));
                }
                if with_value {
                    text.push_str(&format!("<text class=\"pele-value\" x=\"{}\" y=\"{}\" font-size=\"{}\" text-anchor=\"end\" fill=\"var(--_m)\"{}>{}</text>",num(x+b.w-8.),num(baseline+tiny*0.35),js_num(tiny),style.text,esc(&value)));
                }
            }
            let tint = if style.shape.contains("\"fill:") || style.shape.contains(";fill:") {
                ""
            } else {
                " fill-opacity=\"0.12\""
            };
            sections.push_str(&format!("<g class=\"pele-node pele-section{classes}\" {group}{rect}{tint} stroke=\"{color}\"{}/>{text}</g>",style.shape));
            continue;
        }
        let mut text = String::new();
        let room = b.w - 8.;
        let tallest = size.min(((b.h - 4.) / 1.25).floor());
        let label = if tallest >= least {
            fitted(name, room, tallest, least, 0, r, o)?
        } else {
            None
        };
        if let Some((label, label_size)) = label {
            let cx = num(x + b.w / 2.);
            let cy = y + b.h / 2.;
            let value_size = least.max(round(label_size * 0.8));
            let lines = (label_size + value_size) * 1.25;
            if !value.is_empty()
                && lines <= b.h - 4.
                && measure(r, o, &value, value_size, 0)? <= room
            {
                let first = cy - lines / 2. + label_size * 1.25 / 2.;
                let second = cy + lines / 2. - value_size * 1.25 / 2.;
                text=format!("<text x=\"{cx}\" y=\"{}\" font-size=\"{}\"{}>{}</text><text class=\"pele-value\" x=\"{cx}\" y=\"{}\" font-size=\"{}\"{}>{}</text>",num(first+label_size*0.35),num(label_size),style.text,esc(&label),num(second+value_size*0.35),num(value_size),style.text,esc(&value));
            } else {
                text = format!(
                    "<text x=\"{cx}\" y=\"{}\" font-size=\"{}\"{}>{}</text>",
                    num(cy + label_size * 0.35),
                    num(label_size),
                    style.text,
                    esc(&label)
                );
            }
        }
        leaves.push_str(&format!(
            "<g class=\"pele-node pele-leaf{classes}\" {group}{rect}{}/>{text}</g>",
            style.shape
        ));
    }
    let total_width = (width.max(title.width) + 2. * pad).ceil();
    let total_height = (height + title_height + 2. * pad).ceil();
    let content = format!(
        "{}{}{}",
        label_svg(
            &title,
            total_width / 2.,
            pad + title.height / 2.,
            " class=\"pele-title\" font-weight=\"var(--_tw)\""
        ),
        if sections.is_empty() {
            String::new()
        } else {
            format!(
                "<g class=\"pele-sections\" font-size=\"{}\">{sections}</g>",
                js_num(small)
            )
        },
        if leaves.is_empty() {
            String::new()
        } else {
            format!(
                "<g class=\"pele-leaves\" text-anchor=\"middle\" fill=\"var(--_bg)\">{leaves}</g>"
            )
        }
    );
    let svg = svg::document(
        "treemap",
        total_width,
        total_height,
        size,
        o,
        m.acc_title.as_deref(),
        m.acc_descr.as_deref(),
        &content,
    )?;
    Ok(Rendered {
        svg,
        width: total_width,
        height: total_height,
        links: vec![],
    })
}
