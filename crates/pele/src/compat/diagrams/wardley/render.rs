//! Actual Wardley map projection, pipelines, links and responsive sizing, Pele 458322a5 (MIT).
use super::model::*;
use crate::compat::number_math::{round, NumberMath};
use crate::compat::{
    svg::{
        self,
        edges::{marker, marker_trim},
        esc, esc_text, label_svg, label_svg_with_resources, num,
    },
    text::{layout_label, Label},
    Object, PeleError, RenderResources, RenderSettings, Rendered, Value,
};
use std::collections::{HashMap, HashSet};
const LINE: &str = "var(--_l)";
const ACCENT: &str = "var(--_c)";
const RADIUS: &str = "var(--pele-radius,4px)";
fn setting(c: &Object, key: &str, fallback: f64, min: f64, max: f64) -> f64 {
    match c
        .get("wardley-beta")
        .and_then(Value::as_object)
        .and_then(|o| o.get(key))
    {
        Some(Value::Number(n)) if *n >= min && *n <= max => *n,
        _ => fallback,
    }
}
fn clamp(n: f64, min: f64, max: f64) -> f64 {
    if n < min {
        min
    } else if n > max {
        max
    } else {
        n
    }
}
fn dimension(c: &Object, given: Option<f64>, key: &str, fallback: f64) -> f64 {
    clamp(
        given
            .filter(|n| n.is_finite())
            .unwrap_or_else(|| setting(c, key, fallback, 1., 1e6)),
        200.,
        10000.,
    ) - 96.
}
fn start_label(label: &Label, x: f64, y: f64, attrs: &str) -> String {
    let mut shared = 3;
    let mut any = false;
    for s in label.lines.iter().flatten() {
        if s.icon.is_none() && !s.text.is_empty() {
            shared &= s.style;
            any = true
        }
    }
    if !any {
        shared = 0
    }
    let mut attrs = attrs.to_owned();
    if shared & 1 != 0 && !attrs.contains(" font-weight=\"") {
        attrs += " font-weight=\"var(--_w)\""
    }
    if shared & 2 != 0 && !attrs.contains(" font-style=\"") {
        attrs += " font-style=\"italic\""
    }
    let mut body = String::new();
    let mut baseline = y;
    for line in &label.lines {
        let mut first = true;
        for s in line {
            if s.icon.is_some() {
                continue;
            }
            let style = s.style & !shared;
            body += &format!(
                "<tspan{}{}{}>{}</tspan>",
                if first {
                    format!(" x=\"{}\" y=\"{}\"", num(x), num(baseline))
                } else {
                    String::new()
                },
                if style & 1 != 0 {
                    " font-weight=\"var(--_w)\""
                } else {
                    ""
                },
                if style & 2 != 0 {
                    " font-style=\"italic\""
                } else {
                    ""
                },
                esc(&s.text)
            );
            first = false;
        }
        baseline += label.line_height;
    }
    if body.is_empty() {
        String::new()
    } else {
        format!("<text{attrs} xml:space=\"preserve\">{body}</text>")
    }
}
struct Drawing<'a> {
    r: &'a mut dyn RenderResources,
    o: &'a RenderSettings,
    small: f64,
    plot_w: f64,
    plot_h: f64,
    tight: bool,
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}
impl Drawing<'_> {
    fn grow(&mut self, x0: f64, y0: f64, x1: f64, y1: f64) {
        if x0 < self.min_x {
            self.min_x = x0
        }
        if y0 < self.min_y {
            self.min_y = y0
        }
        if x1 > self.max_x {
            self.max_x = x1
        }
        if y1 > self.max_y {
            self.max_y = y1
        }
    }
    fn centered(
        &mut self,
        label: &Label,
        x: f64,
        y: f64,
        attrs: &str,
    ) -> Result<String, PeleError> {
        self.grow(
            x - label.width / 2.,
            y - label.height / 2.,
            x + label.width / 2.,
            y + label.height / 2.,
        );
        label_svg_with_resources(label, x, y, attrs, self.r)
    }
    fn starting(&mut self, label: &Label, x: f64, y: f64, attrs: &str) -> String {
        self.grow(
            x,
            y - label.size,
            x + label.width,
            y - label.size + label.height + 4.,
        );
        start_label(label, x, y, attrs)
    }
    fn text(
        &mut self,
        raw: &str,
        size: f64,
        width: Option<f64>,
        style: u8,
    ) -> Result<Label, PeleError> {
        let width = width.unwrap_or_else(|| {
            if self.tight {
                64f64.js_max(120f64.js_min(self.plot_w / 2.))
            } else {
                4000.
            }
        });
        layout_label(Some(raw), false, self.r, self.o, size, width, style)
    }
    fn x(&self, p: f64) -> f64 {
        p / 100. * self.plot_w
    }
    fn y(&self, p: f64) -> f64 {
        self.plot_h - p / 100. * self.plot_h
    }
}
struct Placed {
    node: usize,
    x: f64,
    y: f64,
}
fn reach(
    at: &Placed,
    node: &WardleyNode,
    ux: f64,
    uy: f64,
    d: &mut Drawing<'_>,
    labels: &mut HashMap<usize, Label>,
) -> Result<f64, PeleError> {
    if node.class_name.as_deref() == Some("anchor") {
        if !labels.contains_key(&at.node) {
            let label = d.text(
                &node.label,
                d.small,
                Some(if d.tight {
                    64f64.js_max(d.plot_w / 2.)
                } else {
                    4000.
                }),
                1,
            )?;
            labels.insert(at.node, label);
        }
        let label = &labels[&at.node];
        Ok(1. / (ux / (label.width / 2. + 5.)).hypot(uy / (d.small * 0.8)))
    } else if node.is_pipeline_parent == Some(true) {
        Ok(10. / std::f64::consts::SQRT_2)
    } else {
        Ok(
            if node.source_strategy.as_ref().is_some_and(|s| !s.is_empty()) {
                12.
            } else {
                6.
            },
        )
    }
}
fn occupy(taken: &mut [u8], i: usize, ux: f64, uy: f64) {
    if ux.abs() < 0.25 || uy.abs() < 0.25 {
        return;
    }
    taken[i] |= if ux > 0. {
        if uy < 0. {
            1
        } else {
            2
        }
    } else if uy < 0. {
        4
    } else {
        8
    };
}
pub(super) fn render(
    model: &WardleyModel,
    c: &Object,
    o: &RenderSettings,
    r: &mut dyn RenderResources,
) -> Result<Rendered, PeleError> {
    let natural = dimension(c, model.size.as_ref().map(|s| s.width), "width", 900.);
    let mut result = draw(model, c, o, r, natural, natural, false)?;
    let Some(available) = o.max_width.filter(|n| *n > 0.) else {
        return Ok(result);
    };
    if result.width <= available {
        return Ok(result);
    }
    let floor = 160f64.js_min(natural);
    let mut plot = natural;
    result = draw(model, c, o, r, plot, natural, true)?;
    for _ in 0..3 {
        if !(result.width > available && plot > floor) {
            break;
        }
        plot = floor.js_max(plot - (result.width - available));
        result = draw(model, c, o, r, plot, natural, true)?;
    }
    Ok(result)
}
fn draw(
    model: &WardleyModel,
    c: &Object,
    o: &RenderSettings,
    r: &mut dyn RenderResources,
    plot_w: f64,
    natural_w: f64,
    tight: bool,
) -> Result<Rendered, PeleError> {
    let size = o.font_size.unwrap_or(16.);
    let small = round(size * 0.8125);
    let tiny = round(size * 0.6875);
    let pad = o.padding.unwrap_or(8.);
    let plot_h = dimension(c, model.size.as_ref().map(|s| s.height), "height", 600.)
        * 0.75f64.js_max(plot_w / natural_w);
    let show_grid = matches!(
        c.get("wardley-beta")
            .and_then(Value::as_object)
            .and_then(|o| o.get("showGrid")),
        Some(Value::Bool(true))
    );
    let mut d = Drawing {
        r,
        o,
        small,
        plot_w,
        plot_h,
        tight,
        min_x: 0.,
        min_y: 0.,
        max_x: plot_w,
        max_y: plot_h,
    };
    let defaults = [
        "Genesis".into(),
        "Custom Built".into(),
        "Product".into(),
        "Commodity".into(),
    ];
    let stages = model
        .axes
        .stages
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or(&defaults);
    let bounds = model.axes.stage_boundaries.as_deref();
    let custom = bounds.is_some_and(|b| b.len() == stages.len());
    let mut frame = String::new();
    let mut stage_labels = String::new();
    let mut stage_bottom = plot_h;
    let mut previous = 0.;
    for (i, stage) in stages.iter().enumerate() {
        let start = if custom {
            previous
        } else {
            i as f64 / stages.len() as f64
        };
        let end = if custom {
            clamp(bounds.unwrap()[i], 0., 1.)
        } else {
            (i + 1) as f64 / stages.len() as f64
        };
        previous = end;
        if i > 0 {
            frame+=&format!("<path class=\"pele-wardley-stage\" d=\"M{},0V{}\" stroke=\"var(--_b)\" stroke-dasharray=\"4 4\"/>",num(start*plot_w),num(plot_h));
        }
        let label = d.text(
            stage,
            small,
            Some(48f64.js_max((end - start).abs() * plot_w - 12.)),
            0,
        )?;
        stage_labels += &d.centered(
            &label,
            (start + end) / 2. * plot_w,
            plot_h + 8. + label.height / 2.,
            &format!(
                " class=\"pele-wardley-stage-label\" font-size=\"{}\" fill=\"var(--_m)\"",
                svg::js_num(small)
            ),
        )?;
        stage_bottom = stage_bottom.js_max(plot_h + 8. + label.height);
    }
    if show_grid {
        for i in 1..4 {
            frame+=&format!("<path class=\"pele-wardley-grid\" d=\"M{},0V{}M0,{}H{}\" stroke=\"var(--_b)\" stroke-dasharray=\"2 6\"/>",num(plot_w*i as f64/4.),num(plot_h),num(plot_h*i as f64/4.),num(plot_w));
        }
    }
    frame += &format!(
        "<path class=\"pele-wardley-axis\" d=\"M0,0V{}H{}\" stroke=\"{LINE}\"/>",
        num(plot_h),
        num(plot_w)
    );
    let x_title = d.text(
        model.axes.x_label.as_deref().unwrap_or("Evolution"),
        small,
        None,
        0,
    )?;
    let y_title = d.text(
        model.axes.y_label.as_deref().unwrap_or("Visibility"),
        small,
        None,
        0,
    )?;
    let attrs = format!(
        " class=\"pele-wardley-axis-label\" font-size=\"{}\" font-weight=\"var(--_hw)\"",
        svg::js_num(small)
    );
    let axis_labels = d.centered(
        &x_title,
        plot_w / 2.,
        stage_bottom + 6. + x_title.height / 2.,
        &attrs,
    )? + &label_svg(
        &y_title,
        -16.,
        plot_h / 2.,
        &format!("{attrs} transform=\"rotate(-90 -16 {})\"", num(plot_h / 2.)),
    );
    d.grow(-16. - y_title.height / 2., 0., 0., 0.);
    let legend_top = stage_bottom + 6. + x_title.height + 10.;
    let mut positions = Vec::new();
    let mut indices = HashMap::new();
    for (i, node) in model.nodes.iter().enumerate() {
        let at = Placed {
            node: i,
            x: d.x(node.x),
            y: d.y(node.y),
        };
        if let Some(&p) = indices.get(&node.id) {
            positions[p] = at
        } else {
            indices.insert(node.id.clone(), positions.len());
            positions.push(at)
        }
    }
    let mut pipelines = String::new();
    let mut members: HashMap<&str, HashSet<&str>> = HashMap::new();
    for pipeline in &model.pipelines {
        members.insert(
            &pipeline.node_id,
            pipeline.component_ids.iter().map(String::as_str).collect(),
        );
        let mut parts: Vec<_> = pipeline
            .component_ids
            .iter()
            .filter_map(|id| indices.get(id).copied())
            .collect();
        if parts.is_empty() {
            continue;
        }
        parts.sort_by(|a, b| {
            positions[*a]
                .x
                .partial_cmp(&positions[*b].x)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let left = positions[parts[0]].x;
        let right = positions[*parts.last().unwrap()].x;
        let y = positions[parts[0]].y;
        let top = y - 12.;
        if let Some(&p) = indices.get(&pipeline.node_id) {
            positions[p].x = (left + right) / 2.;
            positions[p].y = top - 10. / 6.;
        }
        d.grow(left - 15., top, right + 15., top + 24.);
        pipelines+=&format!("<g class=\"pele-wardley-pipeline\" data-id=\"{}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"24\" rx=\"{RADIUS}\" fill=\"var(--_s)\" stroke=\"var(--_b)\"/>{}</g>",esc_text(&pipeline.node_id)?,num(left-15.),num(top),num(right-left+30.),if parts.len()>1{format!("<path d=\"M{},{}H{}\" stroke=\"{LINE}\" stroke-dasharray=\"4 4\"/>",num(left),num(y),num(right))}else{String::new()});
    }
    let mut anchor_labels = HashMap::new();
    let mut taken = vec![0; positions.len()];
    let mut links = String::new();
    for link in &model.links {
        let (Some(&fi), Some(&ti)) = (indices.get(&link.source), indices.get(&link.target)) else {
            continue;
        };
        if fi == ti
            || members
                .get(link.target.as_str())
                .is_some_and(|m| m.contains(link.source.as_str()))
        {
            continue;
        }
        let from = &positions[fi];
        let to = &positions[ti];
        let dx = to.x - from.x;
        let dy = to.y - from.y;
        let distance = dx.hypot(dy);
        if distance < 0.01 {
            continue;
        }
        let ux = dx / distance;
        let uy = dy / distance;
        let start = reach(
            from,
            &model.nodes[from.node],
            ux,
            uy,
            &mut d,
            &mut anchor_labels,
        )?;
        let end = reach(
            to,
            &model.nodes[to.node],
            ux,
            uy,
            &mut d,
            &mut anchor_labels,
        )?;
        if start + end >= distance {
            continue;
        }
        occupy(&mut taken, fi, ux, uy);
        occupy(&mut taken, ti, -ux, -uy);
        let mut x1 = from.x + ux * start;
        let mut y1 = from.y + uy * start;
        let mut x2 = to.x - ux * end;
        let mut y2 = to.y - uy * end;
        let mut heads = String::new();
        if matches!(link.flow.as_deref(), Some("forward" | "bidirectional")) {
            heads += &marker("arrow_point", x2, y2, ux, uy, LINE);
            x2 -= ux * marker_trim("arrow_point");
            y2 -= uy * marker_trim("arrow_point");
        }
        if matches!(link.flow.as_deref(), Some("backward" | "bidirectional")) {
            heads += &marker("arrow_point", x1, y1, -ux, -uy, LINE);
            x1 += ux * marker_trim("arrow_point");
            y1 += uy * marker_trim("arrow_point");
        }
        let mut caption = String::new();
        if let Some(text) = link.label.as_deref().filter(|s| !s.is_empty()) {
            let label = d.text(text, tiny + 1., Some(200.), 0)?;
            let side = if ux >= 0. { 1. } else { -1. };
            let px = uy * side;
            let py = -ux * side;
            let away = 5. + px.abs() * label.width / 2. + py.abs() * label.height / 2.;
            caption = d.centered(
                &label,
                (from.x + to.x) / 2. + px * away,
                (from.y + to.y) / 2. + py * away,
                &format!(
                    " class=\"pele-edge-label\" font-size=\"{}\" fill=\"var(--_m)\"",
                    svg::js_num(tiny + 1.)
                ),
            )?;
        }
        links+=&format!("<g class=\"pele-edge{}\" data-id=\"{}-{}\"><path d=\"M{},{}L{},{}\" stroke=\"{LINE}\"{}/>{heads}{caption}</g>",if link.dashed{" pele-wardley-dashed"}else{""},esc_text(&link.source)?,esc_text(&link.target)?,num(x1),num(y1),num(x2),num(y2),if link.dashed{" stroke-dasharray=\"6 6\""}else{""});
    }
    let mut trends = String::new();
    for trend in &model.trends {
        let Some(&i) = indices.get(&trend.node_id) else {
            continue;
        };
        let origin = &positions[i];
        let tx = d.x(trend.target_x);
        let ty = d.y(trend.target_y);
        let dx = tx - origin.x;
        let dy = ty - origin.y;
        let distance = dx.hypot(dy);
        let mut arrow = String::new();
        let start = if distance > 0. {
            reach(
                origin,
                &model.nodes[origin.node],
                dx / distance,
                dy / distance,
                &mut d,
                &mut anchor_labels,
            )?
        } else {
            0.
        };
        if distance > start + 10. {
            let ux = dx / distance;
            let uy = dy / distance;
            let ex = tx - ux * 8.;
            let ey = ty - uy * 8.;
            arrow = format!(
                "<path d=\"M{},{}L{},{}\" stroke=\"{ACCENT}\" stroke-dasharray=\"4 4\"/>{}",
                num(origin.x + ux * start),
                num(origin.y + uy * start),
                num(ex - ux * marker_trim("arrow_point")),
                num(ey - uy * marker_trim("arrow_point")),
                marker("arrow_point", ex, ey, ux, uy, ACCENT)
            );
        }
        d.grow(tx - 6., ty - 6., tx + 6., ty + 6.);
        trends+=&format!("<g class=\"pele-wardley-trend\" data-id=\"{}\">{arrow}<circle cx=\"{}\" cy=\"{}\" r=\"6\" fill=\"var(--_bg)\" stroke=\"{ACCENT}\"/></g>",esc_text(&trend.node_id)?,num(tx),num(ty));
    }
    let mut nodes = String::new();
    let tri = 1.2 * 6.;
    let dot = 0.7 * 6.;
    for (i, at) in positions.iter().enumerate() {
        let node = &model.nodes[at.node];
        let (x, y) = (at.x, at.y);
        let anchor = node.class_name.as_deref() == Some("anchor");
        let strategy = node.source_strategy.as_deref().filter(|s| !s.is_empty());
        let parent = node.is_pipeline_parent == Some(true);
        let mut shape = String::new();
        if let Some(strategy) = strategy {
            let fill = match strategy {
                "outsource" => "var(--_m)",
                "buy" => "var(--_a)",
                "build" => "var(--_s)",
                _ => "var(--_bg)",
            };
            shape+=&format!("<circle class=\"pele-wardley-{strategy}\" cx=\"{}\" cy=\"{}\" r=\"12\" fill=\"{fill}\" stroke=\"{LINE}\"/>",num(x),num(y));
        }
        if strategy == Some("market") {
            let bx = tri * (std::f64::consts::PI / 6.).cos();
            let by = y + tri * (std::f64::consts::PI / 6.).sin();
            shape += &format!(
                "<path d=\"M{},{}L{},{}H{}Z\" fill=\"none\" stroke=\"{LINE}\"/>",
                num(x),
                num(y - tri),
                num(x - bx),
                num(by),
                num(x + bx)
            );
            for (cx, cy) in [(x, y - tri), (x - bx, by), (x + bx, by)] {
                shape += &format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"var(--_bg)\" stroke=\"{LINE}\"/>",
                    num(cx),
                    num(cy),
                    num(dot)
                );
            }
        } else if parent {
            shape+=&format!("<rect x=\"{}\" y=\"{}\" width=\"10\" height=\"10\" fill=\"var(--_bg)\" stroke=\"{LINE}\"/>",num(x-5.),num(y-5.));
        } else if !anchor {
            shape += &format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"6\" fill=\"var(--_bg)\" stroke=\"{LINE}\"/>",
                num(x),
                num(y)
            );
        }
        if node.inertia == Some(true) {
            let offset =
                if parent { 5. } else { 6. } + 10. + if strategy.is_some() { 10. } else { 0. };
            shape+=&format!("<rect class=\"pele-wardley-inertia\" x=\"{}\" y=\"{}\" width=\"4\" height=\"12\" fill=\"{LINE}\"/>",num(x+offset),num(y-6.));
        }
        let ox = node.label_offset_x.map(|x| clamp(x, -5000., 5000.));
        let oy = node.label_offset_y.map(|y| clamp(y, -5000., 5000.));
        let caption = if anchor {
            let label = if let Some(l) = anchor_labels.get(&at.node) {
                l.clone()
            } else {
                d.text(
                    &node.label,
                    small,
                    Some(if tight {
                        64f64.js_max(plot_w / 2.)
                    } else {
                        4000.
                    }),
                    1,
                )?
            };
            d.centered(
                &label,
                x + ox.unwrap_or(0.),
                y + oy.unwrap_or(-3.),
                &format!(
                    " class=\"pele-label\" font-size=\"{}\" font-weight=\"var(--_hw)\"",
                    svg::js_num(small)
                ),
            )?
        } else {
            let label = d.text(&node.label, small, None, 0)?;
            let attrs = format!(" class=\"pele-label\" font-size=\"{}\"", svg::js_num(small));
            let lift = 8. + if strategy.is_some() { 10. } else { 0. };
            if ox.is_some() || oy.is_some() {
                d.starting(
                    &label,
                    x + ox.unwrap_or(lift),
                    y + oy.unwrap_or(-lift),
                    &attrs,
                )
            } else if node.in_pipeline == Some(true) {
                d.centered(&label, x, y + 12. + 4. + label.height / 2., &attrs)?
            } else {
                let busy = taken[i];
                let corner = if busy & 1 == 0 {
                    1
                } else if busy & 2 == 0 {
                    2
                } else if busy & 4 == 0 {
                    4
                } else if busy & 8 == 0 {
                    8
                } else {
                    1
                };
                let right = corner == 1 || corner == 2;
                let up = corner == 1 || corner == 4;
                d.starting(
                    &label,
                    if right {
                        x + lift
                    } else {
                        x - lift - label.width
                    },
                    if up {
                        y - lift
                    } else {
                        y + lift + small * 0.75
                    },
                    &attrs,
                )
            }
        };
        d.grow(x - 12., y - 12., x + 12. + 20., y + 12.);
        nodes += &format!(
            "<g class=\"pele-node pele-wardley-{}\" data-id=\"{}\">{shape}{caption}</g>",
            if anchor {
                "anchor"
            } else if parent {
                "pipeline-parent"
            } else if node.in_pipeline == Some(true) {
                "pipeline-component"
            } else {
                "component"
            },
            esc_text(&node.id)?
        );
    }
    let mut annotations = String::new();
    for a in &model.annotations {
        let label = d.text(&svg::js_num(a.number), tiny, Some(4000.), 1)?;
        let mut marks = String::new();
        for p in &a.coordinates {
            let x = d.x(p.x);
            let y = d.y(p.y);
            let radius = 10f64.js_max(label.width / 2. + 4.);
            d.grow(x - radius, y - radius, x + radius, y + radius);
            marks += &format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"var(--_bg)\" stroke=\"{LINE}\"/>{}",
                num(x),
                num(y),
                num(radius),
                label_svg(
                    &label,
                    x,
                    y,
                    &format!(
                        " font-size=\"{}\" font-weight=\"var(--_hw)\"",
                        svg::js_num(tiny)
                    )
                )
            );
        }
        annotations += &format!(
            "<g class=\"pele-wardley-annotation\" data-id=\"{}\">{marks}</g>",
            esc_text(&svg::js_num(a.number))?
        );
    }
    let mut listed: Vec<_> = model
        .annotations
        .iter()
        .filter(|a| a.text.as_ref().is_some_and(|s| !s.is_empty()))
        .collect();
    listed.sort_by(|a, b| {
        a.number
            .partial_cmp(&b.number)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    if !listed.is_empty() {
        let mut lines = vec![];
        let (mut width, mut height) = (0f64, 0f64);
        for a in listed {
            let line = d.text(
                &format!("{}. {}", svg::js_num(a.number), a.text.as_ref().unwrap()),
                small,
                Some(if tight {
                    80f64.js_max(plot_w - 20.)
                } else {
                    320.
                }),
                0,
            )?;
            width = width.js_max(line.width);
            height += line.height;
            lines.push(line)
        }
        width += 20.;
        height += 14.;
        let (bx, by) = if let Some(b) = &model.annotations_box {
            (
                clamp(d.x(b.x), 0., 0f64.js_max(plot_w - width)),
                clamp(d.y(b.y), 0., 0f64.js_max(plot_h - height)),
            )
        } else {
            (0., legend_top)
        };
        d.grow(bx, by, bx + width, by + height);
        let mut rows = String::new();
        let mut y = by + 7.;
        for line in &lines {
            rows += &start_label(
                line,
                bx + 10.,
                y + line.line_height / 2. + small * 0.35,
                &format!(" font-size=\"{}\"", svg::js_num(small)),
            );
            y += line.height;
        }
        annotations+=&format!("<g class=\"pele-wardley-annotations\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{RADIUS}\" fill=\"var(--_bg)\" stroke=\"var(--_b)\"/>{rows}</g>",num(bx),num(by),num(width),num(height));
    }
    let mut notes = String::new();
    for n in &model.notes {
        let label = d.text(&n.text, small, Some(360f64.js_min(plot_w * 0.6)), 1)?;
        notes += &d.starting(
            &label,
            d.x(n.x),
            d.y(n.y),
            &format!(
                " class=\"pele-wardley-note\" font-size=\"{}\" font-weight=\"var(--_hw)\"",
                svg::js_num(small)
            ),
        );
    }
    let mut forces = String::new();
    for (forward, list) in [(true, &model.accelerators), (false, &model.deaccelerators)] {
        for a in list {
            let x = d.x(a.x);
            let y = d.y(a.y);
            let tip = if forward { x + 60. } else { x };
            let tail = if forward { x } else { x + 60. };
            let neck = if forward { x + 40. } else { x + 20. };
            let path = format!(
                "M{},{}H{}V{}L{},{}L{},{}V{}H{}Z",
                num(tail),
                num(y - 15.),
                num(neck),
                num(y - 23.),
                num(tip),
                num(y),
                num(neck),
                num(y + 23.),
                num(y + 15.),
                num(tail)
            );
            d.grow(x, y - 23., x + 60., y + 23.);
            let label = d.text(&a.name, tiny + 1., Some(160.), 1)?;
            forces+=&format!("<g class=\"pele-wardley-{}\" data-id=\"{}\"><path d=\"{path}\" fill=\"var(--_s)\" stroke=\"{LINE}\"/>{}</g>",if forward{"accelerator"}else{"deaccelerator"},esc_text(&a.name)?,d.centered(&label,x+30.,y+35.,&format!(" font-size=\"{}\" font-weight=\"var(--_hw)\"",svg::js_num(tiny+1.)))?);
        }
    }
    let title = layout_label(
        model.title.as_deref(),
        false,
        d.r,
        o,
        size,
        if tight { 80f64.js_max(plot_w) } else { 4000. },
        1,
    )?;
    let title_height = if title.height > 0. {
        title.height + 12.
    } else {
        0.
    };
    let left = d.min_x.js_min(plot_w / 2. - title.width / 2.);
    let right = d.max_x.js_max(plot_w / 2. + title.width / 2.);
    let tx = pad - left;
    let ty = pad + title_height - d.min_y;
    let total_width = (right - left + 2. * pad).ceil();
    let total_height = (d.max_y - d.min_y + title_height + 2. * pad).ceil();
    let content=label_svg(&title,tx+plot_w/2.,pad+title.height/2.," class=\"pele-title\" font-weight=\"var(--_tw)\"")+&format!("<g transform=\"translate({},{})\"><g class=\"pele-wardley-frame\" fill=\"none\">{frame}</g>{stage_labels}{axis_labels}{pipelines}{}{nodes}{annotations}{notes}{forces}</g>",num(tx),num(ty),if links.is_empty(){String::new()}else{format!("<g class=\"pele-edges\" fill=\"none\">{links}</g>")}+&if trends.is_empty(){String::new()}else{format!("<g class=\"pele-wardley-trends\" fill=\"none\">{trends}</g>")});
    Ok(Rendered {
        svg: svg::document(
            "wardley",
            total_width,
            total_height,
            size,
            o,
            model.acc_title.as_deref(),
            model.acc_descr.as_deref(),
            &content,
        )?,
        width: total_width,
        height: total_height,
        links: vec![],
    })
}
