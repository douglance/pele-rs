//! Pele gantt/render.ts direct translation, 458322a5 (MIT, kepano).
use super::{
    axis::{auto_ticks, interval_ticks, time_format},
    dates::Calendar,
    model::{GanttModel, GanttTask},
};
use crate::compat::{
    source::url::linked,
    svg::{
        document, edges::theme::resolve_style, esc, esc_text, label_svg, label_svg_with_resources,
        num,
    },
    text::{layout_label, measure, Label},
    Object, PeleError, RenderResources, RenderSettings, Rendered, Value,
};
use regex::Regex;
use std::{collections::HashMap, sync::OnceLock};
fn setting(c: &Object, key: &str, min: f64, max: f64) -> Option<f64> {
    match c
        .get("gantt")
        .and_then(Value::as_object)
        .and_then(|c| c.get(key))
    {
        Some(Value::Number(n)) if n.is_finite() => Some(n.max(min).min(max)),
        _ => None,
    }
}
fn text<'a>(c: &'a Object, key: &str) -> Option<&'a str> {
    match c
        .get("gantt")
        .and_then(Value::as_object)
        .and_then(|c| c.get(key))
    {
        Some(Value::String(s)) => Some(s),
        _ => None,
    }
}
fn flag(c: &Object, key: &str) -> bool {
    c.get("gantt")
        .and_then(Value::as_object)
        .and_then(|c| c.get(key))
        == Some(&Value::Bool(true))
}
fn one_line(
    raw: &str,
    size: f64,
    o: &RenderSettings,
    r: &mut dyn RenderResources,
) -> Result<Label, PeleError> {
    static R: OnceLock<Regex> = OnceLock::new();
    let raw = R
        .get_or_init(|| compiled_regex(r"(?i)<br\s*/?>").unwrap())
        .replace_all(raw, " ");
    let mut label = layout_label(Some(&raw), false, r, o, size, 4000., 0)?;
    if label.lines.len() > 1 {
        label.lines.truncate(1);
        label.widths.truncate(1);
        label.width = label.widths[0];
        label.height = label.line_height
    }
    Ok(label)
}
fn scheduled(t: &GanttTask) -> bool {
    t.start_time.is_some_and(f64::is_finite) && t.end_time.is_some_and(f64::is_finite)
}
struct Bar {
    task: usize,
    label: Label,
    row: usize,
    x0: f64,
    x1: f64,
    label_x: f64,
    inside: bool,
}
pub fn render(
    model: &GanttModel,
    c: &Object,
    o: &RenderSettings,
    r: &mut dyn RenderResources,
) -> Result<Rendered, PeleError> {
    let size = o.font_size.unwrap_or(16.);
    let pad = o.padding.unwrap_or(8.);
    let now = if let Some(now) = o.now.filter(|n| n.is_finite()) {
        now
    } else {
        r.now_millis()?
    };
    let mut cal = Calendar::new(r);
    let mut m = model.clone();
    m.schedule(now, &mut cal)?;
    let result = draw(&mut m, c, o, &mut cal, size, pad, now)?;
    if let Some(e) = cal.error {
        return Err(e);
    }
    Ok(result)
}
/// Draw an already scheduled public model. Scheduling and its mutations belong
/// to the public store; draw's calendar-cache work remains observable afterward.
pub(super) fn render_scheduled(
    model: &mut GanttModel,
    c: &Object,
    o: &RenderSettings,
    r: &mut dyn RenderResources,
) -> Result<Rendered, PeleError> {
    let mut cal = Calendar::public(r);
    let now = model.now;
    let result = draw(
        model,
        c,
        o,
        &mut cal,
        o.font_size.unwrap_or(16.),
        o.padding.unwrap_or(8.),
        now,
    );
    if let Some(e) = cal.error {
        return Err(e);
    }
    result
}
fn draw(
    m: &mut GanttModel,
    c: &Object,
    o: &RenderSettings,
    cal: &mut Calendar<'_>,
    size: f64,
    pad: f64,
    now: f64,
) -> Result<Rendered, PeleError> {
    let label_size = setting(c, "fontSize", 4., 72.).unwrap_or((size * 0.8125 + 0.5).floor());
    let section_size = setting(c, "sectionFontSize", 4., 72.).unwrap_or(label_size);
    let tick_size = (size * 0.75 + 0.5).floor();
    let bar_h = setting(c, "barHeight", 4., 400.).unwrap_or((label_size * 1.85 + 0.5).floor());
    let bar_gap = setting(c, "barGap", 0., 400.).unwrap_or((bar_h / 4. + 0.5).floor());
    let row_h = bar_h + bar_gap;
    let section_styles =
        (setting(c, "numberSectionStyles", 1., 1000.).unwrap_or(4.) + 0.5).floor() as usize;
    let compact = m.display_mode == "compact";
    let top_axis = m.top_axis || flag(c, "topAxis");
    let tasks = &m.tasks;
    let row_tasks: Vec<usize> = tasks
        .iter()
        .enumerate()
        .filter_map(|(i, t)| (!t.vert).then_some(i))
        .collect();
    let mut verts: Vec<usize> = tasks
        .iter()
        .enumerate()
        .filter_map(|(i, t)| (t.vert && scheduled(t)).then_some(i))
        .collect();
    let mut categories: Vec<String> = vec![];
    for &i in &row_tasks {
        if !categories.contains(&tasks[i].section) {
            categories.push(tasks[i].section.clone())
        }
    }
    let mut row_of = HashMap::new();
    let mut row_section: Vec<String> = vec![];
    if compact {
        for section in &categories {
            let group: Vec<usize> = row_tasks
                .iter()
                .copied()
                .filter(|&i| tasks[i].section == *section)
                .collect();
            let mut sorted: Vec<usize> = group
                .iter()
                .copied()
                .filter(|&i| scheduled(&tasks[i]))
                .collect();
            sorted.sort_by(|&a, &b| {
                tasks[a]
                    .start_time
                    .unwrap()
                    .total_cmp(&tasks[b].start_time.unwrap())
                    .then_with(|| {
                        m.public_task_order(a)
                            .partial_cmp(&m.public_task_order(b))
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
            });
            let offset = row_section.len();
            let mut free = vec![];
            for i in sorted {
                let start = tasks[i].start_time.unwrap();
                let row = free
                    .iter()
                    .position(|&end| start >= end)
                    .unwrap_or(free.len());
                if row == free.len() {
                    free.push(tasks[i].end_time.unwrap())
                } else {
                    free[row] = tasks[i].end_time.unwrap()
                }
                row_of.insert(m.public_task_identity(i), offset + row);
            }
            let mut rows = free.len();
            for i in group {
                if !row_of.contains_key(&m.public_task_identity(i)) {
                    row_of.insert(m.public_task_identity(i), offset + rows);
                    rows += 1
                }
            }
            row_section.extend(std::iter::repeat_n(section.clone(), rows.max(1)))
        }
    } else {
        for &i in &row_tasks {
            row_of.insert(m.public_task_identity(i), row_section.len());
            row_section.push(tasks[i].section.clone())
        }
    }
    let row_count = row_section.len();
    let (mut start, mut stop) = (f64::INFINITY, f64::NEG_INFINITY);
    for t in tasks {
        if scheduled(t) {
            start = start.min(t.start_time.unwrap());
            stop = stop.max(t.end_time.unwrap())
        }
    }
    let title = layout_label(m.title.as_deref(), false, cal.resources, o, size, 4000., 1)?;
    let title_h = if title.height > 0. {
        title.height + 16.
    } else {
        0.
    };
    if !(start <= stop) {
        let width = (title.width.max(1.) + 2. * pad).ceil();
        let height = (title.height.max(1.) + 2. * pad).ceil();
        let content = label_svg(
            &title,
            width / 2.,
            pad + title.height / 2.,
            " class=\"pele-title\" font-weight=\"var(--_tw)\"",
        );
        return Ok(Rendered {
            svg: document(
                "gantt",
                width,
                height,
                size,
                o,
                m.acc_title.as_deref(),
                m.acc_descr.as_deref(),
                &content,
            )?,
            width,
            height,
            links: vec![],
        });
    }
    let mut section_labels = HashMap::new();
    let mut section_width = 0f64;
    for section in &categories {
        let label = layout_label(
            Some(section),
            false,
            cal.resources,
            o,
            section_size,
            180.,
            0,
        )?;
        section_width = section_width.max(label.width);
        section_labels.insert(section.clone(), label);
    }
    let axis_format = if !m.axis_format.is_empty() {
        m.axis_format.as_str()
    } else if m.date_format == "D" {
        "%d"
    } else {
        text(c, "axisFormat").unwrap_or("%Y-%m-%d")
    };
    let a = time_format(cal, axis_format, start);
    cal.check()?;
    let b = time_format(cal, axis_format, stop);
    cal.check()?;
    let sample_width = measure(cal.resources, o, &a, tick_size, 0)?.max(measure(
        cal.resources,
        o,
        &b,
        tick_size,
        0,
    )?);
    let left = pad
        + setting(c, "leftPadding", 0., 2000.).unwrap_or(
            (if section_width > 0. {
                section_width + 24.
            } else {
                0.
            })
            .max(sample_width / 2.),
        );
    let right_pad = setting(c, "rightPadding", 0., 2000.).unwrap_or(16f64.max(sample_width / 2.));
    let natural = setting(c, "useWidth", 100., 20000.).unwrap_or(800.);
    let target = if let Some(a) = o.max_width.filter(|n| *n > 0.) {
        natural.min(280.).max(natural.min(a))
    } else {
        natural
    };
    let plot_width = 240f64.max(target - left - right_pad - pad);
    let right = left + plot_width;
    let scale = if stop > start {
        plot_width / (stop - start)
    } else {
        0.
    };
    let x = |t: f64| {
        if scale == 0. {
            left + plot_width / 2.
        } else {
            left + (t - start) * scale
        }
    };
    let tick_h = (tick_size * 1.75 + 0.5).floor();
    let top_padding = setting(c, "topPadding", 0., 2000.);
    let grid_start = setting(c, "gridLineStartPadding", 0., 2000.);
    let head_h = pad + title_h + if top_axis { tick_h } else { 0. };
    let plot_top = head_h.max(top_padding.unwrap_or(0.));
    let grid_lead = match (top_padding, grid_start) {
        (Some(top), Some(grid)) => (top - grid).min(plot_top - head_h).max(0.),
        _ => 0.,
    };
    let plot_h = row_count.max(1) as f64 * row_h;
    let plot_bottom = plot_top + plot_h;
    let mut bars = vec![];
    let mut extent = right + right_pad;
    for &i in &row_tasks {
        let t = &tasks[i];
        let label = one_line(&t.task, label_size, o, cal.resources)?;
        let row = row_of[&m.public_task_identity(i)];
        if !scheduled(t) {
            let label_x = left + 8. + label.width / 2.;
            extent = extent.max(label_x + label.width / 2.);
            bars.push(Bar {
                task: i,
                label,
                row,
                x0: f64::NAN,
                x1: f64::NAN,
                label_x,
                inside: false,
            });
            continue;
        }
        let mut x0 = x(t.start_time.unwrap());
        let mut x1 = x(t.render_end_time.or(t.end_time).unwrap())
            .max(x0 + if t.milestone { 0. } else { 2. });
        if t.milestone {
            let mid = x0 + (x(t.end_time.unwrap()) - x0) / 2.;
            x0 = mid - bar_h / 2.;
            x1 = mid + bar_h / 2.
        }
        let inside = !t.milestone && label.width + 16. <= x1 - x0;
        let mut label_x = (x0 + x1) / 2.;
        if !inside {
            let fits_right = x1 + 8. + label.width <= right + right_pad;
            let fits_left = x0 - 8. - label.width >= left;
            label_x = if fits_right || !fits_left {
                x1 + 8. + label.width / 2.
            } else {
                x0 - 8. - label.width / 2.
            };
            extent = extent.max(label_x + label.width / 2.)
        }
        bars.push(Bar {
            task: i,
            label,
            row,
            x0,
            x1,
            label_x,
            inside,
        })
    }
    let interval = m
        .tick_interval
        .as_deref()
        .filter(|s| !s.is_empty())
        .or_else(|| text(c, "tickInterval"))
        .unwrap_or("");
    let weekday = m
        .weekday
        .as_deref()
        .or_else(|| text(c, "weekday"))
        .unwrap_or("sunday");
    let count = (plot_width / (sample_width + 24.)).floor().clamp(2., 10.);
    let ticks = interval_ticks(
        cal,
        interval,
        weekday,
        start,
        stop,
        (plot_width / 3.).ceil() as usize,
    )
    .unwrap_or_else(|| auto_ticks(cal, start, stop, count));
    cal.check()?;
    let (mut grid, mut axis_bottom, mut axis_top) = (String::new(), String::new(), String::new());
    let mut last_right = f64::NEG_INFINITY;
    let mut last_label = String::new();
    let bottom_y = plot_bottom + tick_h / 2. + 2.;
    let top_y = plot_top - grid_lead - tick_h / 2. - 2.;
    for t in ticks {
        let tx = x(t);
        grid += &format!(
            "M{},{}V{}",
            num(tx),
            num(plot_top - grid_lead),
            num(plot_bottom)
        );
        let label = time_format(cal, axis_format, t);
        cal.check()?;
        let width = measure(cal.resources, o, &label, tick_size, 0)?;
        if tx - width / 2. < last_right + 8. || label == last_label {
            continue;
        }
        last_right = tx + width / 2.;
        last_label = label.clone();
        extent = extent.max(last_right);
        let body = format!("text-anchor=\"middle\">{}</text>", esc(&label));
        axis_bottom += &format!(
            "<text x=\"{}\" y=\"{}\" {body}",
            num(tx),
            num(bottom_y + tick_size * 0.35)
        );
        if top_axis {
            axis_top += &format!(
                "<text x=\"{}\" y=\"{}\" {body}",
                num(tx),
                num(top_y + tick_size * 0.35)
            )
        }
    }
    let (mut vert_lines, mut vert_labels) = (String::new(), String::new());
    let mut vert_ends = vec![];
    let vert_top = plot_bottom + tick_h + 4.;
    let vert_line = (label_size * 1.5 + 0.5).floor();
    verts.sort_by(|&a, &b| {
        tasks[a]
            .start_time
            .unwrap()
            .total_cmp(&tasks[b].start_time.unwrap())
    });
    for i in verts {
        let t = &tasks[i];
        let vx = x(t.start_time.unwrap());
        let label = one_line(&t.task, label_size, o, cal.resources)?;
        let cx = (pad + label.width / 2.).max(vx);
        let line = vert_ends
            .iter()
            .position(|&end| cx - label.width / 2. >= end + 8.)
            .unwrap_or(vert_ends.len());
        if line == vert_ends.len() {
            vert_ends.push(cx + label.width / 2.)
        } else {
            vert_ends[line] = cx + label.width / 2.
        }
        extent = extent.max(vert_ends[line]);
        vert_lines += &format!(
            "<path class=\"pele-vert\" data-id=\"{}\" d=\"M{},{}V{}\"/>",
            esc_text(&t.id)?,
            num(vx),
            num(plot_top - grid_lead),
            num(plot_bottom)
        );
        vert_labels += &label_svg(
            &label,
            cx,
            vert_top + line as f64 * vert_line + vert_line / 2.,
            &format!(
                " class=\"pele-vert-label\" data-id=\"{}\"",
                esc_text(&t.id)?
            ),
        )
    }
    let width = (extent.max(pad + title.width) + pad).ceil();
    let height = (plot_bottom
        + tick_h
        + if !vert_ends.is_empty() {
            4. + vert_ends.len() as f64 * vert_line
        } else {
            0.
        }
        + pad)
        .ceil();
    let (mut bands, mut section_out) = (String::new(), String::new());
    let mut row = 0;
    while row < row_count {
        let section = &row_section[row];
        let mut end = row + 1;
        while end < row_count && row_section[end] == *section {
            end += 1
        }
        let index = categories.iter().position(|s| s == section).unwrap() % section_styles;
        let y = plot_top + row as f64 * row_h;
        let h = (end - row) as f64 * row_h;
        if categories.len() > 1 || !section.is_empty() {
            bands+=&format!("<rect class=\"pele-section pele-section-{index}\" x=\"0\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",num(y),num(width),num(h),if index%2==0{"var(--_s)"}else{"none"})
        }
        let label = &section_labels[section];
        section_out += &label_svg_with_resources(
            label,
            pad + 8. + label.width / 2.,
            y + h / 2.,
            &format!(
                " class=\"pele-section-label\" data-id=\"{}\"",
                esc_text(section)?
            ),
            cal.resources,
        )?;
        row = end
    }
    let mut excluded = String::new();
    if (!m.excludes.is_empty() || !m.includes.is_empty()) && cal.add(start, 6., "y") > stop {
        let mut from = None;
        let mut to = None;
        let mut day = start;
        loop {
            let invalid = day <= stop && m.invalid(day, cal)?;
            if invalid {
                if from.is_none() {
                    from = Some(day)
                }
                to = Some(day)
            } else if let (Some(a), Some(b)) = (from.take(), to.take()) {
                let a = cal.today(a);
                cal.check()?;
                let end_civil = cal.fields(b).map(|f| f.civil);
                cal.check()?;
                let Some(mut end_civil) = end_civil else {
                    continue;
                };
                end_civil.hour = 23;
                end_civil.minute = 59;
                end_civil.second = 59;
                end_civil.millisecond = 999;
                let b = cal.local(end_civil);
                cal.check()?;
                let ex0 = left.max(x(a));
                let ex1 = right.min(x(b));
                if ex1 > ex0 {
                    excluded += &format!(
                        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/>",
                        num(ex0),
                        num(plot_top),
                        num(ex1 - ex0),
                        num(plot_h)
                    )
                }
            }
            if day > stop {
                break;
            }
            day = cal.add_days(day, 1.);
            cal.check()?;
            if !day.is_finite() {
                break;
            }
        }
    }
    let mut links = vec![];
    let mut tasks_out = String::new();
    for bar in bars {
        let t = &m.tasks[bar.task];
        let cy = plot_top + bar.row as f64 * row_h + row_h / 2.;
        let state = if t.active {
            "active"
        } else if t.done {
            "done"
        } else {
            ""
        };
        let mut classes = "pele-task".to_string();
        if !state.is_empty() {
            classes += &format!(" pele-{state}")
        }
        if t.crit {
            classes += " pele-crit"
        }
        if t.milestone {
            classes += " pele-milestone"
        }
        if t.classes.iter().any(|s| s == "clickable") {
            classes += " pele-clickable"
        }
        let mut shape = String::new();
        let mut text_fill = "";
        if bar.x0.is_nan() {
            classes += " pele-unscheduled";
            text_fill = " fill=\"var(--_m)\""
        } else {
            let color = if t.crit {
                "var(--pele-critical,#e45756)"
            } else {
                "var(--_c)"
            };
            let paint = if state == "done" {
                "fill=\"var(--_a)\"".into()
            } else if state == "active" {
                format!("fill=\"{color}\" fill-opacity=\"0.25\"")
            } else {
                format!("fill=\"{color}\"")
            };
            let stroke = format!(
                "stroke=\"{}\"",
                if state == "done" && !t.crit {
                    "var(--_b)"
                } else {
                    color
                }
            );
            if t.milestone {
                let cx = (bar.x0 + bar.x1) / 2.;
                let r = bar_h / 2.;
                shape=format!("<path d=\"M{},{}L{},{}L{},{}L{},{}Z\" {paint} {stroke} stroke-linejoin=\"round\"/>",num(cx),num(cy-r),num(cx+r),num(cy),num(cx),num(cy+r),num(cx-r),num(cy))
            } else {
                shape=format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"var(--pele-radius,4px)\" {paint} {stroke}/>",num(bar.x0),num(cy-bar_h/2.),num(bar.x1-bar.x0),num(bar_h))
            }
            if bar.inside {
                if state.is_empty() {
                    text_fill = " fill=\"var(--_bg)\""
                }
            } else if state == "done" {
                text_fill = " fill=\"var(--_m)\""
            }
        }
        let mut body = shape
            + &label_svg(
                &bar.label,
                bar.label_x,
                cy,
                &format!(" class=\"pele-label\"{text_fill}"),
            );
        if let Some((_, url)) = m.links.iter().find(|(id, _)| id == &t.id) {
            body = linked(&body, url, &t.id, &mut links, o, "")
        }
        tasks_out += &format!(
            "<g class=\"{classes}\" data-id=\"{}\">{body}</g>",
            esc_text(&t.id)?
        )
    }
    let mut today = String::new();
    if m.today_marker != "off" && now >= start && now <= stop && stop > start {
        let tx = x(now);
        let top = plot_top - grid_lead;
        let style = if m.today_marker.is_empty() {
            String::new()
        } else {
            resolve_style(
                &m.today_marker
                    .split(',')
                    .map(str::to_string)
                    .collect::<Vec<_>>(),
            )
            .line
        };
        today=format!("<g class=\"pele-today\"><path d=\"M{},{}V{}\" fill=\"none\" stroke=\"var(--_c)\" stroke-width=\"2\"{style}/>{}</g>",num(tx),num(top),num(plot_bottom),if style.is_empty(){format!("<path d=\"M{},{}H{}L{},{}Z\" fill=\"var(--_c)\"/>",num(tx-4.),num(top-5.),num(tx+4.),num(tx),num(top+1.))}else{String::new()})
    }
    let axis_line = format!(
        "M{},{}H{}{}",
        num(left),
        num(plot_bottom),
        num(right),
        if top_axis {
            format!(
                "M{},{}H{}",
                num(left),
                num(plot_top - grid_lead),
                num(right)
            )
        } else {
            String::new()
        }
    );
    let content=format!("{}{}{}<g class=\"pele-grid\" fill=\"none\"><path d=\"{grid}\" stroke=\"var(--_b)\" stroke-dasharray=\"1 3\"/><path d=\"{axis_line}\" stroke=\"var(--_b)\"/></g>{}{today}<g class=\"pele-tasks\" font-size=\"{}\">{tasks_out}</g><g class=\"pele-section-labels\" font-size=\"{}\">{section_out}</g><g class=\"pele-axis\" font-size=\"{}\" fill=\"var(--_m)\">{axis_bottom}{axis_top}</g>{}",label_svg(&title,width/2.,pad+title.height/2.," class=\"pele-title\" font-weight=\"var(--_tw)\""),if bands.is_empty(){String::new()}else{format!("<g class=\"pele-sections\">{bands}</g>")},if excluded.is_empty(){String::new()}else{format!("<g class=\"pele-excluded\" fill=\"var(--_a)\">{excluded}</g>")},if vert_lines.is_empty(){String::new()}else{format!("<g class=\"pele-verts\" fill=\"none\" stroke=\"var(--_l)\" stroke-width=\"1.5\" stroke-dasharray=\"4 3\">{vert_lines}</g>")},num(label_size),num(section_size),num(tick_size),if vert_labels.is_empty(){String::new()}else{format!("<g class=\"pele-vert-labels\" font-size=\"{}\">{vert_labels}</g>",num(label_size))});
    Ok(Rendered {
        svg: document(
            "gantt",
            width,
            height,
            size,
            o,
            m.acc_title.as_deref(),
            m.acc_descr.as_deref(),
            &content,
        )?,
        width,
        height,
        links,
    })
}

fn compiled_regex(pattern: &str) -> Result<Regex, regex::Error> {
    Regex::new(&pattern.replace(r"\s", crate::compat::text::SPACE))
}
