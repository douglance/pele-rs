//! Pele common/literal-label.ts, 458322a5 (MIT, kepano).
use crate::compat::number_math::{round, NumberMath};
use crate::compat::{
    source::space,
    svg::{esc, label_svg, num},
    text::{measure, Label, Span},
    PeleError, RenderResources, RenderSettings,
};
#[allow(dead_code)]
pub(crate) fn literal_label(
    raw: &str,
    r: &mut dyn RenderResources,
    o: &RenderSettings,
    size: f64,
    room: f64,
    style: u8,
) -> Result<Label, PeleError> {
    let mut lines = vec![];
    for raw in raw.split('\n') {
        let chars: Vec<_> = raw.chars().collect();
        if chars.is_empty() {
            lines.push(String::new())
        }
        let mut from = 0;
        while from < chars.len() {
            let mut fits = |length: usize| {
                measure(
                    r,
                    o,
                    &chars[from..from + length].iter().collect::<String>(),
                    size,
                    style,
                )
                .map(|w| w <= room)
            };
            let mut hi = 1;
            while hi < chars.len() - from && fits(hi)? {
                hi = (chars.len() - from).min(hi * 2)
            }
            let mut lo = 1;
            while lo < hi {
                let mid = (lo + hi).div_ceil(2);
                if fits(mid)? {
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
            let s: String = chars[from..from + take].iter().collect();
            let line = if wraps {
                let t = s.trim_end_matches(space);
                if t.is_empty() {
                    s
                } else {
                    t.into()
                }
            } else {
                s
            };
            lines.push(line);
            from += take;
            while wraps && from < chars.len() && space(chars[from]) {
                from += 1;
            }
        }
    }
    if raw.is_empty() {
        lines.clear()
    }
    let widths = lines
        .iter()
        .map(|s| measure(r, o, s, size, style))
        .collect::<Result<Vec<_>, _>>()?;
    let width = widths.iter().copied().fold(0_f64, NumberMath::js_max);
    let line_height = round(size * 1.5);
    let height = lines.len() as f64 * line_height;
    let lines = lines
        .into_iter()
        .enumerate()
        .map(|(i, text)| {
            vec![Span {
                text,
                style,
                width: widths[i],
                icon: None,
            }]
        })
        .collect();
    Ok(Label {
        lines,
        widths,
        width,
        height,
        size,
        line_height,
    })
}
pub(crate) fn break_label(
    label: Label,
    r: &mut dyn RenderResources,
    o: &RenderSettings,
    room: f64,
) -> Result<Label, PeleError> {
    if label.width <= room {
        return Ok(label);
    }
    let mut lines = vec![];
    let mut widths = vec![];
    for (i, line) in label.lines.iter().enumerate() {
        if label.widths[i] <= room {
            lines.push(line.clone());
            widths.push(label.widths[i]);
            continue;
        }
        let mut current: Vec<Span> = vec![];
        let mut width = 0.;
        for span in line {
            if span.icon.is_some() {
                if !current.is_empty() && width + span.width > room {
                    lines.push(std::mem::take(&mut current));
                    widths.push(width);
                    width = 0.
                }
                current.push(span.clone());
                width += span.width;
                continue;
            }
            for ch in span.text.chars() {
                let last = current.last();
                let joined = last.is_some_and(|s| s.icon.is_none() && s.style == span.style);
                let next = if joined {
                    format!("{}{ch}", last.unwrap().text)
                } else {
                    ch.to_string()
                };
                let next_width = measure(r, o, &next, label.size, span.style)?;
                let mut added = next_width - if joined { last.unwrap().width } else { 0. };
                let mut joined = joined;
                if !current.is_empty() && width + added > room {
                    lines.push(std::mem::take(&mut current));
                    widths.push(width);
                    width = 0.;
                    joined = false;
                    added = measure(r, o, &ch.to_string(), label.size, span.style)?;
                }
                if joined {
                    let last = current.last_mut().unwrap();
                    last.text.push(ch);
                    last.width += added;
                } else {
                    current.push(Span {
                        text: ch.to_string(),
                        style: span.style,
                        width: added,
                        icon: None,
                    })
                }
                width += added;
            }
        }
        if !current.is_empty() || line.is_empty() {
            lines.push(current);
            widths.push(width)
        }
    }
    let width = widths.iter().copied().fold(0_f64, NumberMath::js_max);
    let height = lines.len() as f64 * label.line_height;
    Ok(Label {
        lines,
        widths,
        width,
        height,
        ..label
    })
}
#[allow(dead_code)]
pub(crate) fn left_label_svg(label: &Label, x: f64, top: f64, attrs: &str) -> String {
    let mut shared = 3;
    let mut any = false;
    for span in label.lines.iter().flatten() {
        if span.icon.is_some() || span.text.is_empty() {
            continue;
        }
        shared &= span.style;
        any = true
    }
    if !any {
        shared = 0
    }
    let open = format!(
        "<text{attrs}{}{}",
        if shared & 1 != 0 && !attrs.contains(" font-weight=\"") {
            " font-weight=\"var(--_w)\""
        } else {
            ""
        },
        if shared & 2 != 0 && !attrs.contains(" font-style=\"") {
            " font-style=\"italic\""
        } else {
            ""
        }
    );
    let mut body = String::new();
    for (i, spans) in label.lines.iter().enumerate() {
        let cy = top + (i as f64 + 0.5) * label.line_height;
        if spans.iter().any(|s| s.icon.is_some()) {
            let width = label.widths[i];
            let own = Label {
                lines: vec![spans.clone()],
                widths: vec![width],
                width,
                height: label.line_height,
                ..label.clone()
            };
            body.push_str(&label_svg(&own, x + width / 2., cy, attrs));
            continue;
        }
        let mut text = String::new();
        for span in spans {
            let style = span.style & !shared;
            let own = format!(
                "{}{}",
                if style & 1 != 0 {
                    " font-weight=\"var(--_w)\""
                } else {
                    ""
                },
                if style & 2 != 0 {
                    " font-style=\"italic\""
                } else {
                    ""
                }
            );
            if own.is_empty() {
                text.push_str(&esc(&span.text))
            } else {
                text.push_str(&format!("<tspan{own}>{}</tspan>", esc(&span.text)))
            }
        }
        if !text.is_empty() {
            body.push_str(&format!("{open} x=\"{}\" y=\"{}\" text-anchor=\"start\" xml:space=\"preserve\">{text}</text>",num(x),num(cy+label.size*0.35)))
        }
    }
    body
}
