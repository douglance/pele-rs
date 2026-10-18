//! Pele eventmodeling/render.ts, 458322a5 (MIT, kepano).
use super::model::*;
use crate::compat::{
    diagrams::common::literal_label::{break_label, left_label_svg, literal_label},
    source::decode_entities,
    svg::{
        document,
        edges::{marker, marker_trim},
        esc, esc_text, js_num, label_svg, num, series_color,
    },
    text::{layout_label, measure, Label},
    PeleError, RenderResources, RenderSettings, Rendered,
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
fn key<T>(v: &Arc<T>) -> usize {
    Arc::as_ptr(v) as usize
}
fn round(n: f64) -> f64 {
    (n + 0.5).floor()
}
fn series(kind: EntityKind) -> Option<usize> {
    match kind {
        EntityKind::Command => Some(0),
        EntityKind::Event => Some(1),
        EntityKind::Readmodel => Some(2),
        EntityKind::Processor => Some(6),
        _ => None,
    }
}
fn kind(kind: EntityKind) -> &'static str {
    match kind {
        EntityKind::Command => "command",
        EntityKind::Event => "event",
        EntityKind::Readmodel => "readmodel",
        EntityKind::Processor => "processor",
        EntityKind::Ui => "ui",
    }
}
// Pinned JS slices UTF-16 and can bisect a surrogate pair. Rust's UTF-8 SVG
// projection substitutes U+FFFD at those non-scalar boundaries. Exact code-unit
// outputs remain explicit in the fixture ledger; they are not passing parity cases.
fn hard_wrap(text: &str, columns: usize) -> Result<Vec<String>, PeleError> {
    let mut out = vec![];
    for line in decode_entities(text)?.split('\n') {
        let units = line.encode_utf16().collect::<Vec<_>>();
        if units.len() <= columns {
            out.push(line.into());
        } else {
            for chunk in units.chunks(columns) {
                out.push(String::from_utf16_lossy(chunk));
            }
        }
    }
    Ok(out)
}
fn lines(rows: &[String], x: f64, y: f64, step: f64, attrs: &str) -> String {
    let mut body = String::new();
    for (i, row) in rows.iter().enumerate() {
        if !row.is_empty() {
            body += &format!(
                "<tspan x=\"{}\" y=\"{}\">{}</tspan>",
                num(x),
                num(y + i as f64 * step),
                esc(row)
            );
        }
    }
    if body.is_empty() {
        body
    } else {
        format!("<text{attrs} xml:space=\"preserve\">{body}</text>")
    }
}
fn widest(
    rows: &[String],
    size: f64,
    style: u8,
    o: &RenderSettings,
    r: &mut dyn RenderResources,
) -> Result<f64, PeleError> {
    let mut width = 0_f64;
    for row in rows {
        width = width.max(measure(r, o, row, size, style)?);
    }
    Ok(width)
}
fn heading(heading: &str, frame: &str, m: &EventModelingModel) -> String {
    format!(
        "{heading} · {frame}{}",
        m.boxes
            .iter()
            .rfind(|b| b.id == frame)
            .map_or(String::new(), |b| format!(" {}", b.name))
    )
}
fn spec_rows(spec: &EmSpecification) -> Vec<String> {
    let mut rows = vec![format!("Given  {}", spec.given.join(", "))];
    if !spec.when.is_empty() {
        rows.push(format!("When  {}", spec.when.join(", ")));
    }
    rows.push(format!("Then  {}", spec.then.join(", ")));
    rows
}
pub(super) fn render(
    m: &EventModelingModel,
    o: &RenderSettings,
    r: &mut dyn RenderResources,
) -> Result<Rendered, PeleError> {
    let across = horizontal(m, o, r)?;
    let Some(available) = o.max_width.filter(|w| *w > 0.) else {
        return Ok(across);
    };
    if o.auto_direction == Some(false)
        || across.width <= available
        || available >= o.direction_breakpoint.filter(|b| *b > 0.).unwrap_or(640.)
    {
        return Ok(across);
    }
    let down = vertical(m, o, r)?;
    Ok(if down.width < across.width {
        down
    } else {
        across
    })
}
struct Placed {
    index: usize,
    name: Label,
    data: Vec<String>,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}
struct Band {
    lane: usize,
    label: Label,
    right: f64,
    y: f64,
    h: f64,
}
fn horizontal(
    m: &EventModelingModel,
    o: &RenderSettings,
    r: &mut dyn RenderResources,
) -> Result<Rendered, PeleError> {
    let size = o.font_size.unwrap_or(16.);
    let name_size = round(size * 0.875);
    let small = round(size * 0.8125);
    let mono = round(size * 0.75);
    let mono_step = round(mono * 1.4);
    let small_step = round(small * 1.5);
    let pad = o.padding.unwrap_or(8.);
    let mut bands = vec![];
    let mut band_of = HashMap::new();
    let mut label_column = 0_f64;
    for (i, lane) in m.lanes.iter().enumerate() {
        let label = layout_label(Some(&lane.label), false, r, o, small, 4000., 1)?;
        label_column = label_column.max(label.width);
        band_of.insert(key(lane), bands.len());
        bands.push(Band {
            lane: i,
            label,
            right: f64::NEG_INFINITY,
            y: 0.,
            h: 0.,
        });
    }
    label_column += 24.;
    let mut placed = Vec::<Placed>::new();
    let mut by_box = HashMap::new();
    for (index, b) in m.boxes.iter().enumerate() {
        let band = &mut bands[band_of[&key(&b.lane)]];
        let name = layout_label(Some(&b.name), false, r, o, name_size, 4000., 1)?;
        let data = match b.data.as_deref().filter(|s| !s.is_empty()) {
            Some(data) => hard_wrap(data, 48)?,
            None => vec![],
        };
        let w = 96_f64.max(name.width.max(widest(&data, mono, 4, o, r)?) + 24.);
        let h = 20.
            + name.height
            + if data.is_empty() {
                0.
            } else {
                6. + data.len() as f64 * mono_step
            };
        let mut x = 0.;
        if let Some(previous) = placed.last() {
            let stagger = previous.x + (previous.w / 2.).max(previous.w - 80.);
            x = if Arc::ptr_eq(&m.boxes[previous.index].lane, &b.lane) {
                band.right + 16.
            } else {
                stagger.max(band.right + 16.)
            };
        }
        band.right = x + w;
        band.h = band.h.max(h);
        by_box.insert(key(b), placed.len());
        placed.push(Placed {
            index,
            name,
            data,
            x,
            y: 0.,
            w,
            h,
        });
    }
    let content_right = placed.iter().fold(0_f64, |right, p| right.max(p.x + p.w));
    let lane_width = label_column + content_right + 12.;
    let title = layout_label(m.title.as_deref(), false, r, o, size, 4000., 1)?;
    let title_height = if title.height > 0. {
        title.height + 12.
    } else {
        0.
    };
    let left = pad;
    let top = pad + title_height;
    let mut y = top;
    let mut lanes = String::new();
    for band in &mut bands {
        band.y = y;
        band.h += 24.;
        lanes+=&format!("<g class=\"pele-cluster pele-em-lane\" data-id=\"{}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"var(--pele-radius,4px)\" fill=\"var(--_s)\"/>{}</g>",esc_text(&m.lanes[band.lane].label)?,num(left),num(y),num(lane_width),num(band.h),label_svg(&band.label,left+12.+band.label.width/2.,y+band.h/2.,&format!(" class=\"pele-cluster-label\" font-size=\"{}\" font-weight=\"var(--_hw)\" fill=\"var(--_m)\"",js_num(small))));
        y += band.h + 8.;
    }
    let mut bottom = if bands.is_empty() { top } else { y - 8. };
    let mut nodes = String::new();
    for item in &mut placed {
        let b = &m.boxes[item.index];
        let band = &bands[band_of[&key(&b.lane)]];
        item.x += left + label_column;
        item.y = band.y + (band.h - item.h) / 2.;
        let frame = format!(
            "x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"var(--pele-radius,4px)\"",
            num(item.x),
            num(item.y),
            num(item.w),
            num(item.h)
        );
        let shape = if let Some(series) = series(b.kind) {
            let color = series_color(series);
            format!("<rect {frame} fill=\"var(--_bg)\"/><rect {frame} fill=\"{color}\" fill-opacity=\"0.22\" stroke=\"{color}\"/>")
        } else {
            format!("<rect {frame} fill=\"var(--_bg)\" stroke=\"var(--_b)\"/>")
        };
        let text = format!(
            "{}{}",
            label_svg(
                &item.name,
                item.x + item.w / 2.,
                item.y + 10. + item.name.height / 2.,
                &format!(
                    " class=\"pele-label\" font-size=\"{}\" font-weight=\"var(--_hw)\"",
                    js_num(name_size)
                )
            ),
            lines(
                &item.data,
                item.x + 12.,
                item.y + 10. + item.name.height + 6. + mono_step / 2. + mono * 0.35,
                mono_step,
                &format!(
                    " class=\"pele-em-data\" font-family=\"var(--_fm)\" font-size=\"{}\"",
                    js_num(mono)
                )
            )
        );
        nodes += &format!(
            "<g class=\"pele-node pele-em-{}{}\" data-id=\"{}\">{shape}{text}</g>",
            kind(b.kind),
            if b.reset { " pele-em-reset" } else { "" },
            esc_text(&b.id)?
        );
    }
    let mut edges = String::new();
    let trim = marker_trim("arrow_point");
    for relation in &m.relations {
        let (Some(&si), Some(&ti)) = (
            by_box.get(&key(&relation.source)),
            by_box.get(&key(&relation.target)),
        ) else {
            continue;
        };
        if si == ti {
            continue;
        }
        let s = &placed[si];
        let t = &placed[ti];
        let (d, head);
        if Arc::ptr_eq(&relation.source.lane, &relation.target.lane) {
            let forward = t.x >= s.x;
            let sx = if forward { s.x + s.w } else { s.x };
            let tx = if forward { t.x } else { t.x + t.w };
            let sy = s.y + s.h / 2.;
            let ty = t.y + t.h / 2.;
            let dir = if tx >= sx { 1. } else { -1. };
            d = format!(
                "M{},{}L{},{}",
                num(sx),
                num(sy),
                num(tx - dir * trim),
                num(ty)
            );
            head = marker("arrow_point", tx, ty, dir, 0., "var(--_l)");
        } else {
            let down = t.y > s.y;
            let from = s.x.max(t.x);
            let to = (s.x + s.w).min(t.x + t.w);
            let inset = |w: f64| 24_f64.min(w / 4.);
            let sx = if to - from >= 16. {
                (from + to) / 2.
            } else if t.x > s.x {
                s.x + s.w - inset(s.w)
            } else {
                s.x + inset(s.w)
            };
            let tx = if to - from >= 16. {
                sx
            } else if t.x > s.x {
                t.x + inset(t.w)
            } else {
                t.x + t.w - inset(t.w)
            };
            let sy = if down { s.y + s.h } else { s.y };
            let ty = if down { t.y } else { t.y + t.h };
            let dir = if down { 1. } else { -1. };
            let end = ty - dir * trim;
            let mid = (sy + end) / 2.;
            d = if sx == tx {
                format!("M{},{}V{}", num(sx), num(sy), num(end))
            } else {
                format!(
                    "M{},{}C{},{} {},{} {},{}",
                    num(sx),
                    num(sy),
                    num(sx),
                    num(mid),
                    num(tx),
                    num(mid),
                    num(tx),
                    num(end)
                )
            };
            head = marker("arrow_point", tx, ty, 0., dir, "var(--_l)");
        }
        edges+=&format!("<g class=\"pele-edge\" data-id=\"{}-{}\"><path d=\"{d}\" fill=\"none\" stroke=\"var(--_l)\"/>{head}</g>",esc_text(&relation.source.id)?,esc_text(&relation.target.id)?);
    }
    let mut cards = String::new();
    let mut cards_right = 0_f64;
    let mut card = |kind: &str,
                    heading_text: &str,
                    frame: &str,
                    rows: Vec<String>|
     -> Result<(), PeleError> {
        let head = layout_label(
            Some(&heading(heading_text, frame, m)),
            false,
            r,
            o,
            small,
            4000.,
            1,
        )?;
        let w = head.width.max(widest(&rows, small, 0, o, r)?) + 24.;
        let h = 20. + head.height + rows.len() as f64 * small_step;
        let cy = bottom
            + if cards.is_empty() && bands.is_empty() {
                0.
            } else {
                8.
            };
        cards+=&format!("<g class=\"pele-em-{kind}\" data-id=\"{}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"var(--pele-radius,4px)\" fill=\"var(--_bg)\" stroke=\"var(--_b)\"/>{}{}</g>",esc_text(frame)?,num(left),num(cy),num(w),num(h),label_svg(&head,left+12.+head.width/2.,cy+10.+head.height/2.,&format!(" font-size=\"{}\" font-weight=\"var(--_hw)\"",js_num(small))),lines(&rows,left+12.,cy+10.+head.height+small_step/2.+small*0.35,small_step,&format!(" font-size=\"{}\"",js_num(small))));
        bottom = cy + h;
        cards_right = cards_right.max(w);
        Ok(())
    };
    for note in &m.notes {
        card("note", "Note", &note.frame, hard_wrap(&note.text, 80)?)?;
    }
    for spec in &m.specifications {
        let rows = spec_rows(spec)
            .iter()
            .map(|row| hard_wrap(row, 80))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect();
        card("specification", "Given / When / Then", &spec.frame, rows)?;
    }
    let width = (if bands.is_empty() { 0. } else { lane_width })
        .max(cards_right)
        .max(title.width);
    let total_width = (width + 2. * pad).ceil();
    let total_height = (bottom + pad).ceil();
    let content = format!(
        "{}{lanes}{edges}{nodes}{cards}",
        label_svg(
            &title,
            total_width / 2.,
            pad + title.height / 2.,
            " class=\"pele-title\" font-weight=\"var(--_tw)\""
        )
    );
    Ok(Rendered {
        svg: document(
            "eventmodeling",
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
struct VerticalPlaced {
    y: f64,
    h: f64,
    w: f64,
    index: usize,
    lane: f64,
}
fn label_lines(label: &Label) -> Vec<String> {
    label
        .lines
        .iter()
        .map(|line| line.iter().map(|span| span.text.as_str()).collect())
        .collect()
}
fn vertical(
    m: &EventModelingModel,
    o: &RenderSettings,
    r: &mut dyn RenderResources,
) -> Result<Rendered, PeleError> {
    let size = o.font_size.unwrap_or(16.);
    let small = round(size * 0.875);
    let mono = round(size * 0.75);
    let pad = o.padding.unwrap_or(8.);
    let room = 160_f64.max(o.max_width.unwrap_or(320.) - 2. * pad);
    let order = m
        .boxes
        .iter()
        .enumerate()
        .map(|(i, b)| (key(b), i))
        .collect::<HashMap<_, _>>();
    let mut runs = Vec::<(usize, usize, usize)>::new();
    let mut run_of = HashMap::new();
    let mut passed = HashSet::new();
    for relation in &m.relations {
        let (Some(&s), Some(&t)) = (
            order.get(&key(&relation.source)),
            order.get(&key(&relation.target)),
        ) else {
            continue;
        };
        if s == t {
            continue;
        }
        if s.abs_diff(t) < 2 {
            passed.insert(s.max(t));
            continue;
        }
        if let Some(&i) = run_of.get(&t) {
            let run: &mut (usize, usize, usize) = &mut runs[i];
            run.1 = run.1.min(s);
            run.2 = run.2.max(s);
        } else {
            run_of.insert(t, runs.len());
            runs.push((t, s.min(t), s.max(t)));
        }
    }
    runs.sort_by_key(|(_, from, _)| *from);
    let mut rail_of = HashMap::new();
    let mut rail_ends = Vec::<usize>::new();
    for (target, from, to) in runs {
        let rail = rail_ends
            .iter()
            .position(|&end| end < from)
            .unwrap_or(rail_ends.len());
        if rail == rail_ends.len() {
            rail_ends.push(to);
        } else {
            rail_ends[rail] = to;
        }
        rail_of.insert(target, rail);
    }
    let gutter = 24. + 8. * (rail_ends.len() as f64 - 1.).max(0.);
    let card_width = 120_f64.max(room - gutter);
    let text_room = card_width - 24.;
    let title = layout_label(m.title.as_deref(), false, r, o, size, room, 1)?;
    let mut y = pad
        + if title.height != 0. {
            title.height + 12.
        } else {
            0.
        };
    let mut bottom = pad + title.height;
    let mut width = title.width.max(card_width + gutter);
    let mut nodes = String::new();
    let mut edges = String::new();
    let mut cards = String::new();
    let mut placed = Vec::new();
    for (index, b) in m.boxes.iter().enumerate() {
        let lane_room = if passed.contains(&index) {
            text_room - 20.
        } else {
            text_room
        };
        let lane = layout_label(Some(&b.lane.label), false, r, o, small, lane_room, 1)?;
        let lane = break_label(lane, r, o, lane_room)?;
        let name = layout_label(Some(&b.name), false, r, o, small, text_room - 36., 1)?;
        let data = if let Some(data) = b.data.as_deref().filter(|s| !s.is_empty()) {
            Some(literal_label(
                &decode_entities(data)?,
                r,
                o,
                mono,
                text_room,
                4,
            )?)
        } else {
            None
        };
        let w = card_width
            .max(lane.width + 24.)
            .max(name.width + 24. + 36.)
            .max(data.as_ref().map_or(0., |d| d.width) + 24.);
        let head_h = lane.height + 12.;
        let h = 20. + name.height + data.as_ref().map_or(0., |d| d.height + 6.);
        let cy = y + head_h;
        let frame = format!(
            "x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"var(--pele-radius,4px)\"",
            num(pad),
            num(cy),
            num(w),
            num(h)
        );
        let shade = series(b.kind).map_or(String::new(), |series| {
            let color = series_color(series);
            format!("<rect {frame} fill=\"{color}\" fill-opacity=\"0.22\" stroke=\"{color}\"/>")
        });
        let data_svg = data.as_ref().map_or(String::new(), |data| {
            lines(
                &label_lines(data),
                pad + 12.,
                cy + 10. + name.height + 6. + data.line_height / 2. + mono * 0.35,
                data.line_height,
                &format!(
                    " class=\"pele-em-data\" font-family=\"var(--_fm)\" font-size=\"{}\"",
                    js_num(mono)
                ),
            )
        });
        nodes+=&format!("<g class=\"pele-cluster pele-em-lane\" data-id=\"{}\">{}<g class=\"pele-node pele-em-{}{}\" data-id=\"{}\"><rect {frame} fill=\"var(--_bg)\" stroke=\"var(--_b)\"/>{shade}{}<text class=\"pele-em-frame\" x=\"{}\" y=\"{}\" text-anchor=\"end\" font-size=\"{}\" fill=\"var(--_m)\">{}</text>{data_svg}</g></g>",esc_text(&b.lane.label)?,left_label_svg(&lane,pad+12.,y,&format!(" class=\"pele-cluster-label\" font-size=\"{}\" font-weight=\"var(--_hw)\" fill=\"var(--_m)\"",js_num(small))),kind(b.kind),if b.reset{" pele-em-reset"}else{""},esc_text(&b.id)?,label_svg(&name,pad+12.+name.width/2.,cy+10.+name.height/2.,&format!(" class=\"pele-label\" font-size=\"{}\" font-weight=\"var(--_hw)\"",js_num(small))),num(pad+w-12.),num(cy+10.+small),js_num(mono),esc(&b.id));
        placed.push(VerticalPlaced {
            y: cy,
            h,
            w,
            index,
            lane: lane.width,
        });
        width = width.max(w + gutter);
        bottom = cy + h;
        y = bottom + 28.;
    }
    for relation in &m.relations {
        let (Some(&si), Some(&ti)) = (
            order.get(&key(&relation.source)),
            order.get(&key(&relation.target)),
        ) else {
            continue;
        };
        if si == ti {
            continue;
        }
        let s = &placed[si];
        let t = &placed[ti];
        let down = t.y > s.y;
        let dir = if down { 1. } else { -1. };
        let sy = if down { s.y + s.h } else { s.y };
        let ty = if down { t.y } else { t.y + t.h };
        let (d, head);
        if s.index.abs_diff(t.index) == 1 {
            let cx = pad + (s.w.min(t.w) / 2.).max(12. + if down { t.lane } else { s.lane } + 16.);
            d = format!(
                "M{},{}V{}",
                num(cx),
                num(sy),
                num(ty - dir * marker_trim("arrow_point"))
            );
            head = marker("arrow_point", cx, ty, 0., dir, "var(--_l)");
        } else {
            let rail =
                pad + width - gutter + 12. + 8. * rail_of.get(&ti).copied().unwrap_or(0) as f64;
            let from = s.y + s.h / 2. - 5.;
            let to = t.y + t.h / 2. + 5.;
            d = format!(
                "M{},{}H{}V{}H{}",
                num(pad + s.w),
                num(from),
                num(rail),
                num(to),
                num(pad + t.w + marker_trim("arrow_point"))
            );
            head = marker("arrow_point", pad + t.w, to, -1., 0., "var(--_l)");
        }
        edges+=&format!("<g class=\"pele-edge\" data-id=\"{}-{}\"><path d=\"{d}\" fill=\"none\" stroke=\"var(--_l)\"/>{head}</g>",esc_text(&relation.source.id)?,esc_text(&relation.target.id)?);
    }
    let mut card = |kind: &str,
                    heading_text: &str,
                    frame: &str,
                    raw: &str|
     -> Result<(), PeleError> {
        let head = layout_label(
            Some(&heading(heading_text, frame, m)),
            false,
            r,
            o,
            small,
            text_room,
            1,
        )?;
        let body = literal_label(&decode_entities(raw)?, r, o, small, text_room, 0)?;
        let w = card_width.max(head.width + 24.).max(body.width + 24.);
        let h = 20. + head.height + body.height;
        cards+=&format!("<g class=\"pele-em-{kind}\" data-id=\"{}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"var(--pele-radius,4px)\" fill=\"var(--_bg)\" stroke=\"var(--_b)\"/>{}{}</g>",esc_text(frame)?,num(pad),num(y),num(w),num(h),label_svg(&head,pad+12.+head.width/2.,y+10.+head.height/2.,&format!(" font-size=\"{}\" font-weight=\"var(--_hw)\"",js_num(small))),lines(&label_lines(&body),pad+12.,y+10.+head.height+body.line_height/2.+small*0.35,body.line_height,&format!(" font-size=\"{}\"",js_num(small))));
        bottom = y + h;
        y = bottom + 8.;
        width = width.max(w);
        Ok(())
    };
    for note in &m.notes {
        card("note", "Note", &note.frame, &note.text)?;
    }
    for spec in &m.specifications {
        card(
            "specification",
            "Given / When / Then",
            &spec.frame,
            &spec_rows(spec).join("\n"),
        )?;
    }
    let total_width = (width + 2. * pad).ceil();
    let height = (bottom + pad).ceil();
    let content = format!(
        "{}{edges}{nodes}{cards}",
        label_svg(
            &title,
            total_width / 2.,
            pad + title.height / 2.,
            " class=\"pele-title\" font-weight=\"var(--_tw)\""
        )
    );
    Ok(Rendered {
        svg: document(
            "eventmodeling",
            total_width,
            height,
            size,
            o,
            m.acc_title.as_deref(),
            m.acc_descr.as_deref(),
            &content,
        )?,
        width: total_width,
        height,
        links: vec![],
    })
}
