//! Railroad box algebra, track routing and responsive folding, Pele 458322a5 (MIT).
use super::model::{RailroadModel as Model, RailroadNode as N};
use crate::compat::number_math::{round, NumberMath};
use crate::compat::{
    source::decode_entities,
    svg::{self, esc, esc_text, num},
    text::{layout_label, measure},
    Object, PeleError, RenderResources, RenderSettings, Rendered, Value,
};
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Terminal,
    Nonterminal,
    Special,
    Sequence,
    Choice,
    Bypass,
    Loop,
    Folded,
}
#[derive(Clone)]
struct Boxed {
    kind: Kind,
    width: f64,
    up: f64,
    down: f64,
    kids: Vec<usize>,
    text: String,
    id: String,
    reach: f64,
}
impl Boxed {
    fn new(kind: Kind, width: f64, up: f64, down: f64, kids: Vec<usize>) -> Self {
        Self {
            kind,
            width,
            up,
            down,
            kids,
            text: String::new(),
            id: String::new(),
            reach: 0.,
        }
    }
}
struct Metrics {
    size: f64,
    padding: f64,
    height: f64,
    gap: f64,
    sep: f64,
    arc: f64,
}
fn option(c: &Object, key: &str, fallback: f64, max: f64) -> f64 {
    match c
        .get("railroad")
        .and_then(Value::as_object)
        .and_then(|o| o.get(key))
    {
        Some(Value::Number(v)) if *v >= 0. => v.js_min(max),
        _ => fallback,
    }
}
fn shown(s: &str) -> Result<String, PeleError> {
    let text = if s.contains("¶ß") {
        decode_entities(&s.replace('&', "&amp;"))?
    } else {
        s.into()
    };
    Ok(text
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t"))
}
fn even(v: f64) -> f64 {
    (v / 2.).ceil() * 2.
}
fn below(offset: f64, a: &Boxed, b: &Boxed, m: &Metrics) -> f64 {
    (offset + a.down + m.sep + b.up).js_max(2. * m.arc)
}
fn count(min: f64, max: f64) -> String {
    if max == f64::INFINITY && min <= 1. {
        return String::new();
    }
    let text = |v: f64| {
        if v.is_finite() {
            svg::js_num(v)
        } else {
            "∞".into()
        }
    };
    if min == max {
        format!("{}×", text(min))
    } else if max == f64::INFINITY {
        format!("≥{}×", text(min))
    } else if min == 0. {
        format!("≤{}×", text(max))
    } else {
        format!("{}–{}×", text(min), text(max))
    }
}
fn put(a: &mut Vec<Boxed>, b: Boxed) -> usize {
    let i = a.len();
    a.push(b);
    i
}
fn bypass(a: &mut Vec<Boxed>, kid: usize, m: &Metrics) -> usize {
    let k = &a[kid];
    let reach = (k.up + m.sep).js_max(2. * m.arc);
    let mut b = Boxed::new(Kind::Bypass, k.width + 4. * m.arc, reach, k.down, vec![kid]);
    b.reach = reach;
    put(a, b)
}
fn leaf(
    kind: Kind,
    text: String,
    id: String,
    style: u8,
    m: &Metrics,
    o: &RenderSettings,
    r: &mut dyn RenderResources,
) -> Result<Boxed, PeleError> {
    let width = even(measure(r, o, &text, m.size, style)? + 2. * m.padding).js_max(m.height);
    let mut b = Boxed::new(kind, width, m.height / 2., m.height / 2., vec![]);
    b.text = text;
    b.id = id;
    Ok(b)
}
fn boxes(
    model: &Model,
    root: usize,
    m: &Metrics,
    room: f64,
    o: &RenderSettings,
    r: &mut dyn RenderResources,
) -> Result<(Vec<Boxed>, usize), PeleError> {
    // The pinned measurer assigns room by object identity in breadth-first
    // encounter order. A shared IR node uses its last assigned room everywhere.
    let mut budgets = vec![f64::INFINITY; model.nodes.len()];
    let mut work = 0usize;
    let budget_error = || {
        let mut e = PeleError::new(
            "Railroad layout exceeds work budget.",
            crate::compat::ErrorCode::Limit,
        );
        e.diagram_type = Some("railroad".into());
        e
    };
    if room != f64::INFINITY {
        let mut pending = vec![(root, room)];
        let mut n = 0;
        while n < pending.len() {
            work += 1;
            if work > 4_000_000 {
                return Err(budget_error());
            }
            let (i, width) = pending[n];
            n += 1;
            budgets[i] = width;
            let node = &model.nodes[i];
            let around = match node {
                N::Choice(_) | N::Optional(_) => 4. * m.arc,
                N::Repetition { min, .. } => {
                    if *min == 0. {
                        6. * m.arc
                    } else {
                        2. * m.arc
                    }
                }
                N::Sequence(_) => 6. * m.arc,
                _ => 0.,
            };
            for &child in node.children() {
                pending.push((child, 40_f64.js_max(width - around)));
            }
        }
    }
    let mut todo = vec![(root, false)];
    let mut a = vec![];
    let mut indices = vec![0; model.nodes.len()];
    while let Some((i, done)) = todo.pop() {
        work += 1;
        if work > 4_000_000 {
            return Err(budget_error());
        }
        let node = &model.nodes[i];
        let room = budgets[i];
        if !done {
            todo.push((i, true));
            for &n in node.children().iter().rev() {
                todo.push((n, false));
            }
            continue;
        }
        let kids: Vec<_> = node.children().iter().map(|i| indices[*i]).collect();
        let value = match node {
            N::Terminal(s) => {
                let s = shown(s)?;
                let text = if !s.is_empty() && s.bytes().all(|c| c == b' ') {
                    "␣".repeat(s.len())
                } else {
                    s
                };
                put(
                    &mut a,
                    leaf(Kind::Terminal, text, String::new(), 4, m, o, r)?,
                )
            }
            N::Nonterminal(s) => put(
                &mut a,
                leaf(Kind::Nonterminal, shown(s)?, s.clone(), 0, m, o, r)?,
            ),
            N::Special(s) => put(
                &mut a,
                leaf(
                    Kind::Special,
                    format!("? {} ?", shown(s)?),
                    String::new(),
                    2,
                    m,
                    o,
                    r,
                )?,
            ),
            N::Sequence(_) => {
                let mut width = kids.len().saturating_sub(1) as f64 * m.gap;
                let (mut up, mut down) = (0f64, 0f64);
                for &k in &kids {
                    width += a[k].width;
                    up = up.js_max(a[k].up);
                    down = down.js_max(a[k].down)
                }
                if width <= room || kids.len() < 2 {
                    put(&mut a, Boxed::new(Kind::Sequence, width, up, down, kids))
                } else {
                    let mut rows = vec![];
                    let mut row = vec![];
                    let mut row_w = 0.;
                    let flush = |row: &mut Vec<usize>,
                                 row_w: &mut f64,
                                 rows: &mut Vec<usize>,
                                 a: &mut Vec<Boxed>| {
                        let (mut up, mut down) = (0f64, 0f64);
                        for &k in row.iter() {
                            up = up.js_max(a[k].up);
                            down = down.js_max(a[k].down)
                        }
                        rows.push(put(
                            a,
                            Boxed::new(Kind::Sequence, *row_w, up, down, std::mem::take(row)),
                        ));
                        *row_w = 0.;
                    };
                    for &k in &kids {
                        if !row.is_empty()
                            && row_w + m.gap + a[k].width > 40f64.js_max(room - 6. * m.arc)
                        {
                            flush(&mut row, &mut row_w, &mut rows, &mut a)
                        }
                        row_w += if row.is_empty() { 0. } else { m.gap } + a[k].width;
                        row.push(k)
                    }
                    flush(&mut row, &mut row_w, &mut rows, &mut a);
                    if rows.len() < 2 {
                        put(&mut a, Boxed::new(Kind::Sequence, width, up, down, kids))
                    } else {
                        let (mut width, mut offset) = (0f64, 0f64);
                        for (j, &row) in rows.iter().enumerate() {
                            width = width.js_max(a[row].width);
                            if j > 0 {
                                offset += a[rows[j - 1]].down + m.sep.js_max(2. * m.arc) + a[row].up
                            }
                        }
                        let up = a[rows[0]].up;
                        let down = offset + a[*rows.last().unwrap()].down;
                        put(
                            &mut a,
                            Boxed::new(Kind::Folded, width + 6. * m.arc, up, down, rows),
                        )
                    }
                }
            }
            N::Choice(_) => {
                let (mut inner, mut offset) = (0f64, 0f64);
                for (j, &k) in kids.iter().enumerate() {
                    inner = inner.js_max(a[k].width);
                    if j > 0 {
                        offset = below(offset, &a[kids[j - 1]], &a[k], m)
                    }
                }
                let up = kids.first().map_or(0., |k| a[*k].up);
                let down = kids.last().map_or(0., |k| offset + a[*k].down);
                put(
                    &mut a,
                    Boxed::new(Kind::Choice, inner + 4. * m.arc, up, down, kids),
                )
            }
            N::Optional(_) => bypass(&mut a, kids[0], m),
            N::Repetition { min, max, .. } => {
                let k = kids[0];
                if *min == 1. && *max == 1. {
                    k
                } else {
                    let text = count(*min, *max);
                    let reach = (a[k].down + m.sep).js_max(2. * m.arc);
                    let label = if text.is_empty() {
                        0.
                    } else {
                        even(measure(r, o, &text, 12., 0)?)
                    };
                    let mut b = Boxed::new(
                        Kind::Loop,
                        a[k].width.js_max(label) + 2. * m.arc,
                        a[k].up,
                        reach + if text.is_empty() { 4.5 } else { 18. },
                        vec![k],
                    );
                    b.text = text;
                    b.reach = reach;
                    let n = put(&mut a, b);
                    if *min == 0. {
                        bypass(&mut a, n, m)
                    } else {
                        n
                    }
                }
            }
        };
        indices[i] = value;
    }
    Ok((a, indices[root]))
}
struct Painter<'a> {
    a: &'a [Boxed],
    m: &'a Metrics,
    stroke: &'a str,
    track: String,
    arrows: String,
    nodes: String,
    east_south: String,
    south_east: String,
    east_north: String,
    north_east: String,
    loop_down: String,
    loop_up: String,
}
impl<'a> Painter<'a> {
    fn new(a: &'a [Boxed], m: &'a Metrics, stroke: &'a str) -> Self {
        let arc = num(m.arc);
        let turn = |s, dx, dy| format!("a{arc},{arc} 0 0 {s} {dx}{arc},{dy}{arc}");
        Self {
            a,
            m,
            stroke,
            track: String::new(),
            arrows: String::new(),
            nodes: String::new(),
            east_south: turn(1, "", ""),
            south_east: turn(0, "", ""),
            east_north: turn(0, "", "-"),
            north_east: turn(1, "", "-"),
            loop_down: turn(0, "-", ""),
            loop_up: turn(0, "-", "-"),
        }
    }
    fn line(&mut self, x: f64, y: f64, end: f64) {
        if end > x {
            self.track += &format!("M{},{}H{}", num(x), num(y), num(end))
        }
    }
    fn place(&mut self, root: usize, left: f64, baseline: f64) -> Result<(), PeleError> {
        let mut pending = vec![(root, left, baseline)];
        let arc = self.m.arc;
        while let Some((i, x, y)) = pending.pop() {
            let b = &self.a[i];
            let kids = &b.kids;
            let right = x + b.width;
            match b.kind {
                Kind::Sequence => {
                    let mut kx = right;
                    for (j, &k) in kids.iter().enumerate().rev() {
                        kx -= self.a[k].width;
                        pending.push((k, kx, y));
                        if j > 0 {
                            self.line(kx - self.m.gap, y, kx)
                        }
                        kx -= self.m.gap;
                    }
                }
                Kind::Folded => {
                    let start = x + 2. * arc;
                    let mut offset = 0.;
                    let mut children = vec![];
                    self.line(x, y, start);
                    for (j, &k) in kids.iter().enumerate() {
                        let kid = &self.a[k];
                        let ky = y + offset;
                        children.push((k, start, ky));
                        if j + 1 < kids.len() {
                            let next = &self.a[kids[j + 1]];
                            let next_offset =
                                offset + kid.down + self.m.sep.js_max(2. * arc) + next.up;
                            let mid = y + offset + kid.down + self.m.sep.js_max(2. * arc) / 2.;
                            self.track += &format!(
                                "M{},{}H{}V{}H{}V{}H{}",
                                num(start + kid.width),
                                num(ky),
                                num(right - 2. * arc),
                                num(mid),
                                num(x + arc),
                                num(y + next_offset),
                                num(start)
                            );
                            self.arrows += &format!(
                                "M{},{}l7,3.5l-7,3.5z",
                                num(start - 7.),
                                num(y + next_offset - 3.5)
                            );
                            offset = next_offset;
                        } else {
                            self.track += &format!(
                                "M{},{}H{}V{}H{}",
                                num(start + kid.width),
                                num(ky),
                                num(right - arc),
                                num(y),
                                num(right)
                            );
                        }
                    }
                    pending.extend(children.into_iter().rev());
                }
                Kind::Choice => {
                    let inner = b.width - 4. * arc;
                    let rail_in = x + arc;
                    let rail_out = right - arc;
                    let mut offset = 0.;
                    let mut children = vec![];
                    for (j, &k) in kids.iter().enumerate() {
                        let kid = &self.a[k];
                        if j > 0 {
                            offset = below(offset, &self.a[kids[j - 1]], kid, self.m)
                        }
                        let ky = y + offset;
                        let kx = x + 2. * arc + (inner - kid.width) / 2.;
                        let last = j == kids.len() - 1;
                        if j == 0 {
                            self.line(x, y, kx);
                            self.line(kx + kid.width, y, right)
                        } else {
                            self.track += &if last {
                                format!(
                                    "M{},{}{}V{}",
                                    num(x),
                                    num(y),
                                    self.east_south,
                                    num(ky - arc)
                                )
                            } else {
                                format!("M{},{}", num(rail_in), num(ky - arc))
                            };
                            self.track += &self.south_east;
                            if kx > rail_in + arc {
                                self.track += &format!("H{}", num(kx));
                            }
                            self.track += &format!("M{},{}", num(kx + kid.width), num(ky));
                            if kx > rail_in + arc {
                                self.track += &format!("H{}", num(rail_out - arc));
                            }
                            self.track += &self.east_north;
                            if last {
                                self.track += &format!("V{}{}", num(y + arc), self.north_east)
                            }
                        }
                        children.push((k, kx, ky));
                    }
                    pending.extend(children.into_iter().rev());
                }
                Kind::Bypass => {
                    let k = kids[0];
                    let kid = &self.a[k];
                    let kx = x + 2. * arc;
                    self.line(x, y, kx);
                    self.line(kx + kid.width, y, right);
                    self.track += &format!(
                        "M{},{}{}V{}{}H{}{}V{}{}",
                        num(x),
                        num(y),
                        self.east_north,
                        num(y - b.reach + arc),
                        self.north_east,
                        num(right - 2. * arc),
                        self.east_south,
                        num(y - arc),
                        self.south_east
                    );
                    pending.push((k, kx, y));
                }
                Kind::Loop => {
                    let k = kids[0];
                    let kid = &self.a[k];
                    let kx = x + (b.width - kid.width) / 2.;
                    let loop_y = y + b.reach;
                    let middle = x + b.width / 2.;
                    self.line(x, y, kx);
                    self.line(kx + kid.width, y, right);
                    self.track += &format!(
                        "M{},{}{}V{}{}H{}{}V{}{}",
                        num(x + arc),
                        num(y),
                        self.loop_down,
                        num(loop_y - arc),
                        self.south_east,
                        num(right - arc),
                        self.east_north,
                        num(y + arc),
                        self.loop_up
                    );
                    self.arrows += &format!("M{},{}l7,-3.5v7z", num(middle - 3.5), num(loop_y));
                    if !b.text.is_empty() {
                        self.nodes+=&format!("<text class=\"pele-edge-label pele-count\" x=\"{}\" y=\"{}\" font-size=\"12\" fill=\"var(--_m)\">{}</text>",num(middle),num(loop_y+15.),esc(&b.text));
                    }
                    pending.push((k, kx, y));
                }
                _ => {
                    let text = &b.text;
                    let space =
                        if text.starts_with(' ') || text.ends_with(' ') || text.contains("  ") {
                            " xml:space=\"preserve\""
                        } else {
                            ""
                        };
                    let box_top = self.m.height / 2.;
                    let shape = format!(
                        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"",
                        num(x),
                        num(y - box_top),
                        num(b.width),
                        num(self.m.height)
                    );
                    let label = format!(
                        "<text x=\"{}\" y=\"{}\"{space}",
                        num(x + b.width / 2.),
                        num(y + self.m.size * 0.35)
                    );
                    let stroke = self.stroke;
                    if b.kind == Kind::Terminal {
                        self.nodes+=&format!("<g class=\"pele-node pele-terminal\">{shape} rx=\"{}\" fill=\"var(--_s)\" stroke=\"var(--_b)\"{stroke}/>{label} font-family=\"var(--_fm)\">{}</text></g>",num(box_top),esc(text));
                    } else if b.kind == Kind::Nonterminal {
                        self.nodes+=&format!("<g class=\"pele-node pele-nonterminal\" data-id=\"{}\">{shape} rx=\"var(--pele-radius,4px)\" fill=\"var(--_bg)\" stroke=\"var(--_b)\"{stroke}/>{label}>{}</text></g>",esc_text(&b.id)?,esc(text));
                    } else {
                        self.nodes+=&format!("<g class=\"pele-node pele-special\">{shape} rx=\"var(--pele-radius,4px)\" fill=\"var(--_bg)\" stroke=\"var(--_b)\" stroke-dasharray=\"4 3\"{stroke}/>{label} font-style=\"italic\"><tspan fill=\"var(--_m)\">? </tspan>{}<tspan fill=\"var(--_m)\"> ?</tspan></text></g>",esc(&text[2..text.len()-2]));
                    }
                }
            }
        }
        Ok(())
    }
}
pub(super) fn render(
    m: &Model,
    c: &Object,
    o: &RenderSettings,
    r: &mut dyn RenderResources,
) -> Result<Rendered, PeleError> {
    m.validate_nodes()?;
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
    model: &Model,
    c: &Object,
    o: &RenderSettings,
    r: &mut dyn RenderResources,
    tight: bool,
) -> Result<Rendered, PeleError> {
    let base = o.font_size.unwrap_or(16.);
    let font_size = option(c, "fontSize", 0., 200.);
    let size = if font_size != 0. {
        font_size
    } else {
        round(base * 0.875)
    };
    let head_size = if font_size != 0. { font_size } else { base };
    let padding = option(c, "padding", -1., 200.);
    let line_height = round(size * 1.5);
    let m = Metrics {
        size,
        padding: if padding < 0. { 12. } else { padding },
        height: even(line_height + if padding < 0. { 9. } else { 2. * padding }),
        gap: option(c, "horizontalSeparation", 16., 500.).js_min(if tight {
            12.
        } else {
            f64::INFINITY
        }),
        sep: option(c, "verticalSeparation", 10., 500.),
        arc: option(c, "arcRadius", 10., 200.).js_min(if tight { 6. } else { f64::INFINITY }),
    };
    let radius = if matches!(
        c.get("railroad")
            .and_then(Value::as_object)
            .and_then(|x| x.get("showMarkers")),
        Some(Value::Bool(false))
    ) {
        0.
    } else {
        option(c, "markerRadius", 4., 50.)
    };
    let stroke_width = option(c, "strokeWidth", 0., 20.);
    let stroke = if stroke_width > 0. {
        format!(" stroke-width=\"{}\"", num(stroke_width))
    } else {
        String::new()
    };
    let pad = o.padding.unwrap_or(8.);
    let title_room = if tight {
        o.max_width
            .filter(|w| *w > 0.)
            .map_or(4000., |w| 80f64.js_max(w - 2. * pad))
    } else {
        4000.
    };
    let title = layout_label(model.title.as_deref(), false, r, o, base, title_room, 1)?;
    let title_height = if title.height > 0. {
        title.height + 16.
    } else {
        0.
    };
    let head_height = round(head_size * 1.5);
    let mut rules = String::new();
    let mut width = title.width;
    let mut top = pad + title_height;
    let mut bottom = pad + title.height;
    for rule in &model.rules {
        let room = if tight {
            80f64.js_max(o.max_width.unwrap_or(320.) - 2. * pad - 4. * radius - 24. - 1.)
        } else {
            f64::INFINITY
        };
        let (a, root) = boxes(model, rule.definition, &m, room, o, r)?;
        let b = &a[root];
        let name = shown(&rule.name)?;
        let baseline = top + head_height + 6. + b.up;
        let start = pad + 2. * radius;
        let left = start + 12.;
        let end = left + b.width + 12.;
        let mut p = Painter::new(&a, &m, &stroke);
        p.line(start, baseline, left);
        p.line(left + b.width, baseline, end);
        p.place(root, left, baseline)?;
        let markers = if radius > 0. {
            format!("<circle class=\"pele-marker pele-start\" cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"var(--_l)\"/><circle class=\"pele-marker pele-end\" cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"var(--_l)\"/>",num(pad+radius),num(baseline),num(radius),num(end+radius),num(baseline),num(radius))
        } else {
            String::new()
        };
        rules+=&format!("<g class=\"pele-rule\" data-id=\"{}\"><text class=\"pele-rule-name\" x=\"{}\" y=\"{}\" font-size=\"{}\" font-weight=\"var(--_hw)\" text-anchor=\"start\">{}</text><path class=\"pele-edge pele-track\" d=\"{}\" fill=\"none\" stroke=\"var(--_l)\"{stroke}/>{}{markers}{}</g>",esc_text(&rule.name)?,num(pad),num(top+head_height/2.+head_size*0.35),num(head_size),esc(&name),p.track,if p.arrows.is_empty(){String::new()}else{format!("<path class=\"pele-marker\" d=\"{}\" fill=\"var(--_l)\"/>",p.arrows)},p.nodes);
        width = width
            .js_max(end + 2. * radius - pad)
            .js_max(measure(r, o, &name, head_size, 1)?);
        bottom = baseline + b.down;
        top = bottom + m.sep + 18.;
    }
    let width = (width + 2. * pad).ceil();
    let height = (bottom + pad).ceil();
    let content = svg::label_svg(
        &title,
        width / 2.,
        pad + title.height / 2.,
        " class=\"pele-title\" font-weight=\"var(--_tw)\"",
    ) + &if rules.is_empty() {
        String::new()
    } else {
        format!("<g class=\"pele-rules\" transform=\"translate(0.5,0.5)\" font-size=\"{}\" text-anchor=\"middle\">{rules}</g>",num(size))
    };
    Ok(Rendered {
        svg: svg::document(
            "railroad",
            width,
            height,
            base,
            o,
            model.acc_title.as_deref(),
            model.acc_descr.as_deref(),
            &content,
        )?,
        width,
        height,
        links: vec![],
    })
}
