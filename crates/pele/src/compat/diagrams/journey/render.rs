//! Pele journey/render.ts and common/{fit,fit-width}.ts, 458322a5 (MIT, kepano).
use super::JourneyModel;
use crate::compat::number_math::{round, NumberMath};
use crate::compat::{
    source::space,
    svg::{document, esc_text, js_num, label_svg, num, series_color},
    text::{layout_label, measure, Label},
    Object, PeleError, RenderResources, RenderSettings, Rendered,
};
use regex::Regex;
use std::sync::OnceLock;
const RADIUS: &str = "var(--pele-radius,4px)";
fn plain(
    raw: Option<&str>,
    r: &mut dyn RenderResources,
    o: &RenderSettings,
    size: f64,
    max: f64,
    base: u8,
) -> Result<Label, PeleError> {
    let escaped=raw.map(|s|{static BR:OnceLock<Regex>=OnceLock::new();let br=BR.get_or_init(||Regex::new(r"(?i)^br[\t-\r \u{a0}\u{1680}\u{2000}-\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}\u{feff}]*/?>").unwrap());let mut out=String::new();for(i,c)in s.char_indices(){if c=='<'&&!br.is_match(&s[i+1..]){out.push_str("&lt;")}else{out.push(c)}}out.replace(":fa-","&colon;fa-")});
    layout_label(escaped.as_deref(), false, r, o, size, max, base)
}
fn fit(
    raw: Option<&str>,
    r: &mut dyn RenderResources,
    o: &RenderSettings,
    size: f64,
    max: f64,
    base: u8,
) -> Result<Label, PeleError> {
    let label = plain(raw, r, o, size, max, base)?;
    let Some(raw) = raw else { return Ok(label) };
    if label.width <= max {
        return Ok(label);
    }
    static UNIT: OnceLock<Regex> = OnceLock::new();
    let unit=UNIT.get_or_init(||Regex::new(r"(?i:<br[\t-\r \u{a0}\u{1680}\u{2000}-\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}\u{feff}]*/?>)|ﬂ°°?[A-Za-z0-9_]+¶ß|&#?[A-Za-z0-9_]+;|[\s\S]").unwrap());
    let (mut out, mut width) = (String::new(), 0.);
    for m in unit.find_iter(raw) {
        let u = m.as_str();
        let len = u.encode_utf16().count();
        if u.trim_matches(space).is_empty() || (len > 2 && u.starts_with('<')) {
            width = 0.
        } else {
            let w = measure(r, o, if len > 2 { "x" } else { u }, size, base)?;
            if width > 0. && width + w > max {
                out.push('\n');
                width = 0.
            }
            width += w
        }
        out.push_str(u)
    }
    plain(Some(&out), r, o, size, max, base)
}
fn face(x: f64, y: f64, score: f64) -> String {
    let (mood, mouth) = if score > 3. {
        ("happy", "M-5,2A6,6 0 0 0 5,2")
    } else if score < 3. {
        ("sad", "M-5,6.5A6,6 0 0 1 5,6.5")
    } else {
        ("neutral", "M-4.5,4.5H4.5")
    };
    format!("<g class=\"pele-face pele-face-{mood}\" transform=\"translate({},{})\"><circle r=\"11\" fill=\"var(--_bg)\" stroke=\"var(--_l)\"/><g fill=\"var(--_l)\"><circle cx=\"-3.6\" cy=\"-2.8\" r=\"1.3\"/><circle cx=\"3.6\" cy=\"-2.8\" r=\"1.3\"/></g><path d=\"{mouth}\" fill=\"none\" stroke=\"var(--_l)\" stroke-linecap=\"round\"/></g>",num(x),num(y))
}
fn finish(
    m: &JourneyModel,
    o: &RenderSettings,
    size: f64,
    width: f64,
    height: f64,
    body: String,
) -> Result<Rendered, PeleError> {
    Ok(Rendered {
        svg: document(
            "journey",
            width,
            height,
            size,
            o,
            m.acc_title.as_deref(),
            m.acc_descr.as_deref(),
            &body,
        )?,
        width,
        height,
        links: vec![],
    })
}
pub(super) fn render(
    m: &JourneyModel,
    _c: &Object,
    o: &RenderSettings,
    r: &mut dyn RenderResources,
) -> Result<Rendered, PeleError> {
    let across = horizontal(m, o, r)?;
    let Some(available) = o.max_width else {
        return Ok(across);
    };
    if o.auto_direction == Some(false) || !(available > 0.) || across.width <= available {
        return Ok(across);
    }
    let breakpoint = o.direction_breakpoint.filter(|v| *v > 0.).unwrap_or(640.);
    if available >= breakpoint {
        return Ok(across);
    }
    let down = vertical(m, o, r)?;
    Ok(if down.width < across.width {
        down
    } else {
        across
    })
}
struct Legend {
    name: String,
    label: Label,
    x: f64,
    y: f64,
    h: f64,
}
struct Run {
    start: usize,
    count: usize,
    label: Option<Label>,
}
fn horizontal(
    m: &JourneyModel,
    o: &RenderSettings,
    r: &mut dyn RenderResources,
) -> Result<Rendered, PeleError> {
    let size = o.font_size.unwrap_or(16.);
    let small = round(size * 0.875);
    let tiny = round(size * 0.75);
    let pad = o.padding.unwrap_or(8.);
    let tasks = &m.tasks;
    let n = tasks.len();
    let names: Vec<_> = m.actors().into_iter().filter(|s| !s.is_empty()).collect();
    let actors: std::collections::HashMap<&str, usize> = names
        .iter()
        .enumerate()
        .map(|(i, s)| (s.as_str(), i))
        .collect();
    let actor = |s: &str| actors.get(s).copied().unwrap_or(0);
    let mut text_w = 124.;
    let mut labels = Vec::new();
    for t in tasks {
        labels.push(plain(Some(&t.task), r, o, small, text_w, 0)?)
    }
    let widest = labels.iter().map(|l| l.width).fold(0., NumberMath::js_max);
    if widest > text_w {
        text_w = widest.ceil().js_min(200.);
        labels.clear();
        for t in tasks {
            labels.push(fit(Some(&t.task), r, o, small, text_w, 0)?)
        }
    }
    let card_w = text_w + 20.;
    let pitch = card_w + 12.;
    let plot_w = if n > 0 { n as f64 * pitch - 12. } else { 0. };
    let line_h = round(small * 1.5);
    let limit = plot_w.js_max(240.);
    let mut legend: Vec<Legend> = Vec::new();
    let (mut legend_w, mut legend_h, mut lx) = (0f64, 0f64, 0f64);
    for name in &names {
        let label = fit(Some(name), r, o, small, limit - 16., 0)?;
        let w = 16. + label.width;
        if lx > 0. && lx + w > limit {
            lx = 0.;
            legend_h += 4.
        }
        let y = if lx > 0. {
            legend.last().unwrap().y
        } else {
            legend_h
        };
        let h = label.height.js_max(line_h);
        legend.push(Legend {
            name: name.clone(),
            label,
            x: lx,
            y,
            h,
        });
        lx += w;
        legend_w = legend_w.js_max(lx);
        legend_h = legend_h.js_max(y + h);
        lx += 20.;
    }
    let body_w = if n > 0 { 24. + plot_w } else { 0. };
    let title = fit(m.title.as_deref(), r, o, size, body_w.js_max(320.), 1)?;
    let title_h = if title.height > 0. {
        title.height + 16.
    } else {
        0.
    };
    let width = body_w
        .js_max(title.width)
        .js_max(if n > 0 { 24. + legend_w } else { 0. });
    let left = pad + (width - body_w) / 2. + 24.;
    let top = pad + title_h;
    let mut runs = Vec::new();
    let mut band_h = 0f64;
    let mut i = 0;
    while i < n {
        let mut end = i + 1;
        while end < n && tasks[end].section == tasks[i].section {
            end += 1
        }
        let label = if tasks[i].section.is_empty() {
            None
        } else {
            let label = fit(
                Some(&tasks[i].section),
                r,
                o,
                small,
                (end - i) as f64 * pitch - 12. - 20.,
                1,
            )?;
            band_h = band_h.js_max(label.height.js_max(line_h) + 12.);
            Some(label)
        };
        runs.push(Run {
            start: i,
            count: end - i,
            label,
        });
        i = end;
    }
    let text_h = labels
        .iter()
        .map(|l| l.height)
        .fold(line_h, NumberMath::js_max);
    let dots_h = if names.is_empty() { 0. } else { 16. };
    let card_h = text_h + dots_h + 16.;
    let cards_y = top + if band_h > 0. { band_h + 6. } else { 0. };
    let chart_top = cards_y + card_h + 27.;
    let chart_bottom = chart_top + 104.;
    let right = left + plot_w;
    let mut body = String::new();
    let mut bottom = top;
    if n > 0 {
        let (mut grid, mut scale) = (String::new(), String::new());
        for score in (1..=5).rev() {
            let y = chart_top + (5 - score) as f64 * 26.;
            grid.push_str(&format!("M{},{}H{}", num(left - 6.), num(y), num(right)));
            scale.push_str(&format!(
                "<text x=\"{}\" y=\"{}\">{score}</text>",
                num(left - 12.),
                num(y + tiny * 0.35)
            ));
        }
        body.push_str(&format!("<path class=\"pele-grid\" d=\"{grid}\" fill=\"none\" stroke=\"var(--_a)\"/><g class=\"pele-scale\" font-size=\"{}\" text-anchor=\"end\" fill=\"var(--_m)\">{scale}</g>",js_num(tiny)));
        let (mut line, mut stems, mut faces) = (String::new(), String::new(), String::new());
        let mut cards = Vec::new();
        for (i, t) in tasks.iter().enumerate() {
            let x = left + i as f64 * pitch;
            let cx = x + card_w / 2.;
            let level = if t.score.is_nan() {
                3.
            } else {
                t.score.clamp(1., 5.)
            };
            let y = chart_top + (5. - level) * 26.;
            line.push_str(&format!(
                "{}{},{}",
                if i == 0 { "M" } else { "L" },
                num(cx),
                num(y)
            ));
            stems.push_str(&format!(
                "M{},{}V{}",
                num(cx),
                num(cards_y + card_h),
                num(y - 11.)
            ));
            faces.push_str(&face(cx, y, t.score));
            let mut dots = String::new();
            let people: Vec<_> = t.people.iter().filter(|s| !s.is_empty()).collect();
            let step =
                13f64.js_min((card_w - 30.) / (people.len().saturating_sub(1).max(1) as f64));
            for (k, name) in people.iter().enumerate() {
                dots.push_str(&format!("<circle class=\"pele-actor\" cx=\"{}\" cy=\"{}\" r=\"5\" fill=\"{}\" stroke=\"var(--_s)\"><title>{}</title></circle>",num(cx+(k as f64-(people.len()as f64-1.)/2.)*step),num(cards_y+card_h-13.),series_color(actor(name)),esc_text(name)?));
            }
            cards.push(format!("<g class=\"pele-node pele-task\" data-id=\"{}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{RADIUS}\" fill=\"var(--_s)\" stroke=\"var(--_b)\"/>{}{dots}</g>",esc_text(t.task.trim_matches(space))?,num(x),num(cards_y),num(card_w),num(card_h),label_svg(&labels[i],cx,cards_y+8.+text_h/2.,&format!(" font-size=\"{}\"",js_num(small)))));
        }
        body.push_str(&format!("<path class=\"pele-stems\" d=\"{stems}\" fill=\"none\" stroke=\"var(--_b)\" stroke-dasharray=\"2 3\"/>{}{faces}",if n>1{format!("<path class=\"pele-edge\" d=\"{line}\" fill=\"none\" stroke=\"var(--_l)\" stroke-linejoin=\"round\"/>")}else{String::new()}));
        for run in runs {
            let members = cards[run.start..run.start + run.count].join("");
            if let Some(label) = run.label {
                let x = left + run.start as f64 * pitch;
                let w = run.count as f64 * pitch - 12.;
                body.push_str(&format!("<g class=\"pele-cluster pele-section\" data-id=\"{}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{RADIUS}\" fill=\"var(--_a)\"/>{}{members}</g>",esc_text(tasks[run.start].section.trim_matches(space))?,num(x),num(top),num(w),num(band_h),label_svg(&label,x+w/2.,top+band_h/2.,&format!(" font-size=\"{}\" font-weight=\"var(--_hw)\"",js_num(small)))));
            } else {
                body.push_str(&members)
            }
        }
        bottom = chart_bottom + 11.;
        if !legend.is_empty() {
            let x0 = left + (plot_w - legend_w) / 2.;
            let y0 = bottom + 16.;
            let mut items = String::new();
            for item in legend {
                let cy = y0 + item.y + item.h / 2.;
                items.push_str(&format!("<g class=\"pele-legend-item\" data-id=\"{}\"><circle cx=\"{}\" cy=\"{}\" r=\"5\" fill=\"{}\"/>{}</g>",esc_text(&item.name)?,num(x0+item.x+5.),num(cy),series_color(actor(&item.name)),label_svg(&item.label,x0+item.x+16.+item.label.width/2.,cy,"")));
            }
            body.push_str(&format!(
                "<g class=\"pele-legend\" font-size=\"{}\">{items}</g>",
                js_num(small)
            ));
            bottom = y0 + legend_h;
        }
    }
    let total_width = (width + 2. * pad).ceil();
    let total_height = (bottom.js_max(pad) + pad).ceil();
    let content = label_svg(
        &title,
        pad + width / 2.,
        pad + title.height / 2.,
        " class=\"pele-title\" font-weight=\"var(--_tw)\"",
    ) + &body;
    finish(m, o, size, total_width, total_height, content)
}
fn vertical(
    m: &JourneyModel,
    o: &RenderSettings,
    r: &mut dyn RenderResources,
) -> Result<Rendered, PeleError> {
    let size = o.font_size.unwrap_or(16.);
    let small = round(size * 0.875);
    let pad = o.padding.unwrap_or(8.);
    let room = (o.max_width.unwrap_or(320.) - 2. * pad).js_max(140.);
    let title = fit(m.title.as_deref(), r, o, size, room, 1)?;
    let names: Vec<_> = m.actors().into_iter().filter(|s| !s.is_empty()).collect();
    let actors: std::collections::HashMap<&str, usize> = names
        .iter()
        .enumerate()
        .map(|(i, s)| (s.as_str(), i))
        .collect();
    let mut y = pad
        + if title.height != 0. && !title.height.is_nan() {
            title.height + 16.
        } else {
            0.
        };
    let mut section: Option<&str> = None;
    let mut body = String::new();
    let mut width = title.width;
    for (index, t) in m.tasks.iter().enumerate() {
        if Some(t.section.as_str()) != section {
            section = Some(&t.section);
            let head = fit(section, r, o, size, room - 16., 1)?;
            if head.height != 0. && !head.height.is_nan() {
                body.push_str(&label_svg(
                    &head,
                    pad + 8. + head.width / 2.,
                    y + head.height / 2.,
                    " class=\"pele-cluster-label\" font-weight=\"var(--_hw)\"",
                ));
                width = width.js_max(head.width + 16.);
                y += head.height + 8.;
            }
        }
        let label = fit(Some(&t.task), r, o, small, room - 72., 0)?;
        let mut people = Vec::new();
        for name in t.people.iter().filter(|s| !s.is_empty()) {
            people.push((name, fit(Some(name), r, o, small, room - 44., 0)?))
        }
        let text_h = label.height.js_max(42.);
        let mut h = 16. + text_h;
        for (_, label) in &people {
            h += 16f64.js_max(label.height) + 4.
        }
        let w = room.js_max(label.width + 72.);
        body.push_str(&format!("<g class=\"pele-node pele-journey-step\" data-id=\"{index}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{RADIUS}\" fill=\"var(--_s)\" stroke=\"var(--_b)\"/>",num(pad),num(y),num(w),num(h)));
        body.push_str(&label_svg(
            &label,
            pad + 10. + label.width / 2.,
            y + 8. + label.height / 2.,
            " class=\"pele-label\"",
        ));
        let cx = pad + w - 28.;
        body.push_str(&face(cx, y + 19., t.score));
        if !t.score.is_nan() {
            body.push_str(&format!("<text class=\"pele-journey-score\" x=\"{}\" y=\"{}\" font-size=\"{}\" text-anchor=\"middle\">{}</text>",num(cx),num(y+30.+small),js_num(small),num(t.score.clamp(1.,5.))));
        }
        let mut py = y + 8. + text_h + 4.;
        for (name, label) in people {
            let ph = 16f64.js_max(label.height);
            body.push_str(&format!(
                "<circle class=\"pele-actor\" cx=\"{}\" cy=\"{}\" r=\"5\" fill=\"{}\"/>{}",
                num(pad + 16.),
                num(py + ph / 2.),
                series_color(actors[name.as_str()]),
                label_svg(
                    &label,
                    pad + 28. + label.width / 2.,
                    py + ph / 2.,
                    &format!(
                        " class=\"pele-journey-actor\" font-size=\"{}\"",
                        js_num(small)
                    )
                )
            ));
            py += ph + 4.;
        }
        body.push_str("</g>");
        y += h;
        if index + 1 < m.tasks.len() {
            body.push_str(&format!(
                "<path class=\"pele-edge\" d=\"M{},{}v16\" fill=\"none\" stroke=\"var(--_l)\"/>",
                num(pad + w / 2.),
                num(y)
            ));
        }
        y += 16.;
        width = width.js_max(w);
    }
    let total_width = (width + 2. * pad).ceil();
    let height =
        ((pad + title.height).js_max(y - if m.tasks.is_empty() { 0. } else { 16. }) + pad).ceil();
    let content = label_svg(
        &title,
        total_width / 2.,
        pad + title.height / 2.,
        " class=\"pele-title\" font-weight=\"var(--_tw)\"",
    ) + &body;
    finish(m, o, size, total_width, height, content)
}
