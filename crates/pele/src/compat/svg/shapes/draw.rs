//! SVG outlines translated from Pele `src/svg/shapes.ts` at
//! 458322a5b8abb1ae57ddf127c32017ab9fb14a92 (MIT, Copyright 2026 kepano).
use super::super::num;
use super::{maximum, minimum};
const ROUNDED: &str = " rx=\"var(--pele-radius,4px)\"";
fn path(d: &str, a: &str) -> String {
    format!("<path d=\"{d}\"{a}/>")
}
fn poly(points: &[f64], a: &str) -> String {
    let points = points
        .chunks_exact(2)
        .map(|p| format!("{},{}", num(p[0]), num(p[1])))
        .collect::<Vec<_>>()
        .join(" ");
    format!("<polygon points=\"{points}\"{a}/>")
}
fn rect(w: f64, h: f64, a: &str, rx: &str) -> String {
    format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"{rx}{a}/>",
        num(-w / 2.),
        num(-h / 2.),
        num(w),
        num(h)
    )
}
fn circle(w: f64, a: &str) -> String {
    format!("<circle r=\"{}\"{a}/>", num(w / 2.))
}
fn wave(w: f64, h: f64, a: &str) -> String {
    let x = w / 2.;
    let top = -h / 2.;
    let base = h / 2. - 10.;
    let q = w / 4.;
    path(
        &format!(
            "M{},{}H{}V{}q{},-12 {},0t{},0Z",
            num(-x),
            num(top),
            num(x),
            num(base),
            num(-q),
            num(-2. * q),
            num(-2. * q)
        ),
        a,
    )
}
fn brace(x: f64, h: f64, dir: f64, line: &str) -> String {
    let y = h / 2.;
    let k = 6. * dir;
    format!(
        "<path d=\"M{},{}q{},0 {},6V-6q0,6 {},6q{},0 {},6V{}q0,6 {},6\" fill=\"none\"{line}/>",
        num(x),
        num(-y),
        num(k),
        num(k),
        num(k),
        num(-k),
        num(-k),
        num(y - 6.),
        num(-k)
    )
}

pub(crate) fn draw_shape(shape: &str, w: f64, h: f64, a: &str, line: &str) -> String {
    match shape {
        "rounded" => draw_rounded(shape, w, h, a, line),
        "stadium" => draw_stadium(shape, w, h, a, line),
        "fr-rect" => draw_fr_rect(shape, w, h, a, line),
        "cyl" | "lin-cyl" => draw_cyl(shape, w, h, a, line),
        "circle" | "sm-circ" => draw_circle(shape, w, h, a, line),
        "dbl-circ" => draw_dbl_circ(shape, w, h, a, line),
        "ellipse" => draw_ellipse(shape, w, h, a, line),
        "diam" => draw_diam(shape, w, h, a, line),
        "hex" => draw_hex(shape, w, h, a, line),
        "odd" => draw_odd(shape, w, h, a, line),
        "lean-r" => draw_lean_r(shape, w, h, a, line),
        "lean-l" => draw_lean_l(shape, w, h, a, line),
        "trap-b" => draw_trap_b(shape, w, h, a, line),
        "trap-t" => draw_trap_t(shape, w, h, a, line),
        "text" => draw_text(shape, w, h, a, line),
        "f-circ" => draw_f_circ(shape, w, h, a, line),
        "fr-circ" => draw_fr_circ(shape, w, h, a, line),
        "fork" => draw_fork(shape, w, h, a, line),
        "notch-rect" => draw_notch_rect(shape, w, h, a, line),
        "lin-rect" => draw_lin_rect(shape, w, h, a, line),
        "div-rect" => draw_div_rect(shape, w, h, a, line),
        "win-pane" => draw_win_pane(shape, w, h, a, line),
        "tri" => draw_tri(shape, w, h, a, line),
        "flip-tri" => draw_flip_tri(shape, w, h, a, line),
        "sl-rect" => draw_sl_rect(shape, w, h, a, line),
        "notch-pent" => draw_notch_pent(shape, w, h, a, line),
        "hourglass" => draw_hourglass(shape, w, h, a, line),
        "delay" => draw_delay(shape, w, h, a, line),
        "h-cyl" => draw_h_cyl(shape, w, h, a, line),
        "cross-circ" => draw_cross_circ(shape, w, h, a, line),
        "st-rect" => draw_st_rect(shape, w, h, a, line),
        "doc" => draw_doc(shape, w, h, a, line),
        "flag" => draw_flag(shape, w, h, a, line),
        "datastore" => draw_datastore(shape, w, h, a, line),
        "folder" => draw_folder(shape, w, h, a, line),
        "bucket" => draw_bucket(shape, w, h, a, line),
        "console" => draw_console(shape, w, h, a, line),
        "browser" => draw_browser(shape, w, h, a, line),
        "person" => draw_person(shape, w, h, a, line),
        "bang" => draw_bang(shape, w, h, a, line),
        "cloud" => draw_cloud(shape, w, h, a, line),
        "brace" => draw_brace(shape, w, h, a, line),
        "brace-r" => draw_brace_r(shape, w, h, a, line),
        "braces" => draw_braces(shape, w, h, a, line),
        "bolt" => draw_bolt(shape, w, h, a, line),
        "curv-trap" => draw_curv_trap(shape, w, h, a, line),
        "docs" => draw_docs(shape, w, h, a, line),
        "bow-rect" => draw_bow_rect(shape, w, h, a, line),
        "tag-rect" => draw_tag_rect(shape, w, h, a, line),
        "tag-doc" => draw_tag_doc(shape, w, h, a, line),
        "lin-doc" => draw_lin_doc(shape, w, h, a, line),
        "collapsedGroup" => draw_collapsed_group(shape, w, h, a, line),
        _ => rect(w, h, a, ROUNDED),
    }
}

fn draw_fr_rect(_shape: &str, w: f64, h: f64, a: &str, line: &str) -> String {
    let y = h / 2.;
    let x = w / 2. - 8.;
    rect(w, h, a, ROUNDED)
        + &format!(
            "<path d=\"M{},{}V{}M{},{}V{}\" fill=\"none\"{line}/>",
            num(-x),
            num(-y),
            num(y),
            num(x),
            num(-y),
            num(y)
        )
}

fn draw_cyl(shape: &str, w: f64, h: f64, a: &str, line: &str) -> String {
    let x = w / 2.;
    let y = h / 2.;
    let rx = x;
    let ry = minimum(12., maximum(6., w / 14.));
    let top = -y + ry;
    let bottom = y - ry;
    let mut out = path(
        &format!(
            "M{},{}A{},{} 0 0 1 {},{}V{}A{},{} 0 0 1 {},{}Z",
            num(-rx),
            num(top),
            num(rx),
            num(ry),
            num(rx),
            num(top),
            num(bottom),
            num(rx),
            num(ry),
            num(-rx),
            num(bottom)
        ),
        a,
    );
    let arc = |v: f64| {
        format!(
            "M{},{}A{},{} 0 0 0 {},{}",
            num(-rx),
            num(v),
            num(rx),
            num(ry),
            num(rx),
            num(v)
        )
    };
    out.push_str(&format!(
        "<path d=\"{}{}\" fill=\"none\"{line}/>",
        arc(top),
        if shape == "lin-cyl" {
            arc(top + 8.)
        } else {
            String::new()
        }
    ));
    out
}

fn draw_hex(_shape: &str, w: f64, h: f64, a: &str, _line: &str) -> String {
    let x = w / 2.;
    let y = h / 2.;
    let m = h / 4.;
    poly(
        &[-x + m, -y, x - m, -y, x, 0., x - m, y, -x + m, y, -x, 0.],
        a,
    )
}

fn draw_odd(_shape: &str, w: f64, h: f64, a: &str, _line: &str) -> String {
    let x = w / 2.;
    let y = h / 2.;
    let notch = h / 4.;
    poly(&[-x, -y, x, -y, x, y, -x, y, -x + notch, 0.], a)
}

fn draw_lean_r(_shape: &str, w: f64, h: f64, a: &str, _line: &str) -> String {
    let x = w / 2.;
    let y = h / 2.;
    let s = h / 4.;
    poly(&[-x + s, -y, x, -y, x - s, y, -x, y], a)
}

fn draw_lean_l(_shape: &str, w: f64, h: f64, a: &str, _line: &str) -> String {
    let x = w / 2.;
    let y = h / 2.;
    let s = h / 4.;
    poly(&[-x, -y, x - s, -y, x, y, -x + s, y], a)
}

fn draw_trap_b(_shape: &str, w: f64, h: f64, a: &str, _line: &str) -> String {
    let x = w / 2.;
    let y = h / 2.;
    let s = h / 4.;
    poly(&[-x + s, -y, x - s, -y, x, y, -x, y], a)
}

fn draw_trap_t(_shape: &str, w: f64, h: f64, a: &str, _line: &str) -> String {
    let x = w / 2.;
    let y = h / 2.;
    let s = h / 4.;
    poly(&[-x, -y, x, -y, x - s, y, -x + s, y], a)
}

fn draw_notch_rect(_shape: &str, w: f64, h: f64, a: &str, _line: &str) -> String {
    let x = w / 2.;
    let y = h / 2.;
    let c = 10.;
    poly(&[-x + c, -y, x, -y, x, y, -x, y, -x, -y + c], a)
}

fn draw_notch_pent(_shape: &str, w: f64, h: f64, a: &str, _line: &str) -> String {
    let x = w / 2.;
    let y = h / 2.;
    let c = 10.;
    poly(
        &[-x + c, -y, x - c, -y, x, -y + c, x, y, -x, y, -x, -y + c],
        a,
    )
}

fn draw_delay(_shape: &str, w: f64, h: f64, a: &str, _line: &str) -> String {
    let x = w / 2.;
    let y = h / 2.;
    let r = y;
    path(
        &format!(
            "M{},{}H{}A{},{} 0 0 1 {},{}H{}Z",
            num(-x),
            num(-r),
            num(x - r),
            num(r),
            num(r),
            num(x - r),
            num(r),
            num(-x)
        ),
        a,
    )
}

fn draw_h_cyl(_shape: &str, w: f64, h: f64, a: &str, line: &str) -> String {
    let x = w / 2.;
    let y = h / 2.;
    let ry = y;
    let rx = minimum(12., h / 4.);
    let l = -x + rx;
    let r = x - rx;
    path(
        &format!(
            "M{},{}H{}A{},{} 0 0 1 {},{}H{}A{},{} 0 0 1 {},{}Z",
            num(l),
            num(-ry),
            num(r),
            num(rx),
            num(ry),
            num(r),
            num(ry),
            num(l),
            num(rx),
            num(ry),
            num(l),
            num(-ry)
        ),
        a,
    ) + &format!(
        "<path d=\"M{},{}A{},{} 0 0 0 {},{}\" fill=\"none\"{line}/>",
        num(r),
        num(-ry),
        num(rx),
        num(ry),
        num(r),
        num(ry)
    )
}

fn draw_cross_circ(_shape: &str, w: f64, _h: f64, a: &str, line: &str) -> String {
    let x = w / 2.;
    let k = x * std::f64::consts::FRAC_1_SQRT_2;
    circle(w, a)
        + &format!(
            "<path d=\"M{},{}L{},{}M{},{}L{},{}\" fill=\"none\"{line}/>",
            num(-k),
            num(-k),
            num(k),
            num(k),
            num(k),
            num(-k),
            num(-k),
            num(k)
        )
}

fn draw_st_rect(_shape: &str, w: f64, h: f64, a: &str, _line: &str) -> String {
    let bw = w - 8.;
    let bh = h - 8.;
    let mut out = String::new();
    for i in 0..3 {
        let x = -w / 2. + f64::from(2 - i) * 4.;
        let y = -h / 2. + f64::from(i) * 4.;
        out.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"{ROUNDED}{a}/>",
            num(x),
            num(y),
            num(bw),
            num(bh)
        ));
    }
    out
}

fn draw_flag(_shape: &str, w: f64, h: f64, a: &str, _line: &str) -> String {
    let x = w / 2.;
    let y = h / 2. - 8.;
    let q = w / 4.;
    path(
        &format!(
            "M{},{}q{},-10 {},0t{},0V{}q{},10 {},0t{},0Z",
            num(-x),
            num(-y),
            num(q),
            num(2. * q),
            num(2. * q),
            num(y),
            num(-q),
            num(-2. * q),
            num(-2. * q)
        ),
        a,
    )
}

fn draw_folder(_shape: &str, w: f64, h: f64, a: &str, _line: &str) -> String {
    let x = w / 2.;
    let y = h / 2.;
    let tab = minimum(w * 0.4, 48.);
    path(
        &format!(
            "M{},{}H{}L{},{}H{}V{}H{}Z",
            num(-x),
            num(-y),
            num(-x + tab),
            num(-x + tab + 8.),
            num(-y + 8.),
            num(x),
            num(y),
            num(-x)
        ),
        a,
    )
}

fn draw_bucket(_shape: &str, w: f64, h: f64, a: &str, line: &str) -> String {
    let x = w / 2.;
    let y = h / 2.;
    let ry = 5.;
    let taper = h / 6.;
    path(
        &format!(
            "M{},{}A{},{} 0 0 1 {},{}L{},{}A{},{} 0 0 1 {},{}Z",
            num(-x),
            num(-y + ry),
            num(x),
            num(ry),
            num(x),
            num(-y + ry),
            num(x - taper),
            num(y - ry),
            num(x - taper),
            num(ry),
            num(-x + taper),
            num(y - ry)
        ),
        a,
    ) + &format!(
        "<path d=\"M{},{}A{},{} 0 0 0 {},{}\" fill=\"none\"{line}/>",
        num(-x),
        num(-y + ry),
        num(x),
        num(ry),
        num(x),
        num(-y + ry)
    )
}

fn draw_browser(_shape: &str, w: f64, h: f64, a: &str, line: &str) -> String {
    let x = w / 2.;
    let y = h / 2.;
    let mut out = rect(w, h, a, ROUNDED)
        + &format!(
            "<path d=\"M{},{}H{}\" fill=\"none\"{line}/>",
            num(-x),
            num(-y + 12.),
            num(x)
        );
    for i in 0..3 {
        out.push_str(&format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"1.5\" fill=\"none\"{line}/>",
            num(-x + 8. + f64::from(i) * 7.),
            num(-y + 6.)
        ));
    }
    out
}

fn draw_bang(_shape: &str, w: f64, h: f64, a: &str, _line: &str) -> String {
    let spikes = 14;
    let mut pts = Vec::with_capacity(spikes * 4);
    for i in 0..spikes * 2 {
        let angle = std::f64::consts::PI * i as f64 / spikes as f64 - std::f64::consts::PI / 2.;
        let k = if i % 2 == 0 { 1. } else { 0.82 };
        pts.extend([angle.cos() * (w / 2.) * k, angle.sin() * (h / 2.) * k]);
    }
    poly(&pts, a)
}

fn draw_cloud(_shape: &str, w: f64, h: f64, a: &str, _line: &str) -> String {
    let x = w / 2.;
    let y = h / 2.;
    let r = y * 0.55;
    path(
        &format!(
            "M{},{}A{},{} 0 0 1 {},{}A{},{} 0 0 1 {},{}A{},{} 0 0 1 {},{}A{},{} 0 0 1 {},{}Z",
            num(-x + r),
            num(y),
            num(r),
            num(r),
            num(-x + r * 0.6),
            num(-y * 0.1),
            num(x * 0.42),
            num(y * 0.7),
            num(-x * 0.1),
            num(-y * 0.72),
            num(x * 0.4),
            num(y * 0.62),
            num(x * 0.62),
            num(-y * 0.42),
            num(r * 1.05),
            num(r * 1.05),
            num(x - r),
            num(y)
        ),
        a,
    )
}

fn draw_curv_trap(_shape: &str, w: f64, h: f64, a: &str, _line: &str) -> String {
    let x = w / 2.;
    let y = h / 2.;
    let k = h * 0.3;
    path(
        &format!(
            "M{},{}H{}A{},{} 0 0 1 {},{}H{}L{},0Z",
            num(-x + k),
            num(-y),
            num(x - k),
            num(k),
            num(y),
            num(x - k),
            num(y),
            num(-x + k),
            num(-x)
        ),
        a,
    )
}

fn draw_docs(_shape: &str, w: f64, h: f64, a: &str, _line: &str) -> String {
    let mut out = String::new();
    for i in 0..3 {
        out.push_str(&format!(
            "<g transform=\"translate({},{})\">{}</g>",
            (1 - i) * 4,
            (i - 1) * 4,
            wave(w - 8., h - 8., a)
        ));
    }
    out
}

fn draw_bow_rect(_shape: &str, w: f64, h: f64, a: &str, _line: &str) -> String {
    let x = w / 2.;
    let y = h / 2.;
    let k = h * 0.2;
    path(
        &format!(
            "M{},{}H{}A{},{} 0 0 0 {},{}H{}A{},{} 0 0 1 {},{}Z",
            num(-x + k),
            num(-y),
            num(x),
            num(k),
            num(y),
            num(x),
            num(y),
            num(-x + k),
            num(k),
            num(y),
            num(-x + k),
            num(-y)
        ),
        a,
    )
}

fn draw_rounded(_shape: &str, w: f64, h: f64, a: &str, _line: &str) -> String {
    rect(w, h, a, &format!(" rx=\"{}\"", num(minimum(12., h / 2.))))
}

fn draw_stadium(_shape: &str, w: f64, h: f64, a: &str, _line: &str) -> String {
    rect(w, h, a, &format!(" rx=\"{}\"", num(h / 2.)))
}

fn draw_circle(_shape: &str, w: f64, _h: f64, a: &str, _line: &str) -> String {
    circle(w, a)
}

fn draw_dbl_circ(_shape: &str, w: f64, _h: f64, a: &str, line: &str) -> String {
    let x = w / 2.;
    circle(w, a) + &format!("<circle r=\"{}\" fill=\"none\"{line}/>", num(x - 5.))
}

fn draw_ellipse(_shape: &str, w: f64, h: f64, a: &str, _line: &str) -> String {
    let x = w / 2.;
    let y = h / 2.;
    format!("<ellipse rx=\"{}\" ry=\"{}\"{a}/>", num(x), num(y))
}

fn draw_diam(_shape: &str, w: f64, h: f64, a: &str, _line: &str) -> String {
    let x = w / 2.;
    let y = h / 2.;
    poly(&[0., -y, x, 0., 0., y, -x, 0.], a)
}

fn draw_text(_shape: &str, w: f64, h: f64, _a: &str, _line: &str) -> String {
    rect(w, h, " fill=\"none\" stroke=\"none\"", "")
}

fn draw_f_circ(_shape: &str, w: f64, _h: f64, _a: &str, line: &str) -> String {
    let x = w / 2.;
    format!("<circle r=\"{}\" fill=\"var(--_l)\"{line}/>", num(x))
}

fn draw_fr_circ(_shape: &str, w: f64, _h: f64, a: &str, line: &str) -> String {
    let x = w / 2.;
    circle(w, a) + &format!("<circle r=\"{}\" fill=\"var(--_l)\"{line}/>", num(x - 4.))
}

fn draw_fork(_shape: &str, w: f64, h: f64, _a: &str, line: &str) -> String {
    rect(w, h, &format!(" fill=\"var(--_l)\"{line}"), " rx=\"2\"")
}

fn draw_lin_rect(_shape: &str, w: f64, h: f64, a: &str, line: &str) -> String {
    let x = w / 2.;
    let y = h / 2.;
    rect(w, h, a, ROUNDED)
        + &format!(
            "<path d=\"M{},{}V{}\" fill=\"none\"{line}/>",
            num(-x + 8.),
            num(-y),
            num(y)
        )
}

fn draw_div_rect(_shape: &str, w: f64, h: f64, a: &str, line: &str) -> String {
    let x = w / 2.;
    let y = h / 2.;
    rect(w, h, a, ROUNDED)
        + &format!(
            "<path d=\"M{},{}H{}\" fill=\"none\"{line}/>",
            num(-x),
            num(-y + 8.),
            num(x)
        )
}

fn draw_win_pane(_shape: &str, w: f64, h: f64, a: &str, line: &str) -> String {
    let x = w / 2.;
    let y = h / 2.;
    rect(w, h, a, ROUNDED)
        + &format!(
            "<path d=\"M{},{}V{}M{},{}H{}\" fill=\"none\"{line}/>",
            num(-x + 8.),
            num(-y),
            num(y),
            num(-x),
            num(-y + 8.),
            num(x)
        )
}

fn draw_tri(_shape: &str, w: f64, h: f64, a: &str, _line: &str) -> String {
    let x = w / 2.;
    let y = h / 2.;
    poly(&[0., -y, x, y, -x, y], a)
}

fn draw_flip_tri(_shape: &str, w: f64, h: f64, a: &str, _line: &str) -> String {
    let x = w / 2.;
    let y = h / 2.;
    poly(&[-x, -y, x, -y, 0., y], a)
}

fn draw_sl_rect(_shape: &str, w: f64, h: f64, a: &str, _line: &str) -> String {
    let x = w / 2.;
    let y = h / 2.;
    poly(&[-x, -y + 10., x, -y, x, y, -x, y], a)
}

fn draw_hourglass(_shape: &str, w: f64, h: f64, a: &str, _line: &str) -> String {
    let x = w / 2.;
    let y = h / 2.;
    poly(&[-x, -y, x, -y, -x, y, x, y], a)
}

fn draw_doc(_shape: &str, w: f64, h: f64, a: &str, _line: &str) -> String {
    wave(w, h, a)
}

fn draw_datastore(_shape: &str, w: f64, h: f64, a: &str, line: &str) -> String {
    let x = w / 2.;
    let y = h / 2.;
    rect(
        w,
        h,
        &a.replacen("stroke=\"var(--_b)\"", "stroke=\"none\"", 1),
        "",
    ) + &format!(
        "<path d=\"M{},{}H{}M{},{}H{}\" fill=\"none\"{line}/>",
        num(-x),
        num(-y),
        num(x),
        num(-x),
        num(y),
        num(x)
    )
}

fn draw_console(_shape: &str, w: f64, h: f64, a: &str, line: &str) -> String {
    let x = w / 2.;
    let y = h / 2.;
    rect(w, h, a, ROUNDED)
        + &format!(
            "<path d=\"M{},{}H{}M{},{}l3,2.5l-3,2.5M{},{}h4\" fill=\"none\"{line}/>",
            num(-x),
            num(-y + 12.),
            num(x),
            num(-x + 7.),
            num(-y + 4.),
            num(-x + 13.),
            num(-y + 9.)
        )
}

fn draw_person(_shape: &str, w: f64, h: f64, a: &str, _line: &str) -> String {
    let x = w / 2.;
    let y = h / 2.;
    format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"12\"{a}/><circle cy=\"{}\" r=\"13\"{a}/>",num(-x),num(-y+26.),num(w),num(h-26.),num(-y+14.))
}

fn draw_brace(_shape: &str, w: f64, h: f64, _a: &str, line: &str) -> String {
    let x = w / 2.;
    rect(w, h, " fill=\"none\" stroke=\"none\"", "") + &brace(-x + 12., h, -1., line)
}

fn draw_brace_r(_shape: &str, w: f64, h: f64, _a: &str, line: &str) -> String {
    let x = w / 2.;
    rect(w, h, " fill=\"none\" stroke=\"none\"", "") + &brace(x - 12., h, 1., line)
}

fn draw_braces(_shape: &str, w: f64, h: f64, _a: &str, line: &str) -> String {
    let x = w / 2.;
    rect(w, h, " fill=\"none\" stroke=\"none\"", "")
        + &brace(-x + 12., h, -1., line)
        + &brace(x - 12., h, 1., line)
}

fn draw_bolt(_shape: &str, w: f64, h: f64, a: &str, _line: &str) -> String {
    let x = w / 2.;
    let y = h / 2.;
    poly(
        &[
            w * 0.15,
            -y,
            -x,
            h * 0.08,
            -w * 0.05,
            h * 0.08,
            -w * 0.15,
            y,
            x,
            -h * 0.08,
            w * 0.05,
            -h * 0.08,
        ],
        a,
    )
}

fn draw_tag_rect(_shape: &str, w: f64, h: f64, a: &str, line: &str) -> String {
    let x = w / 2.;
    let y = h / 2.;
    rect(w, h, a, ROUNDED)
        + &format!(
            "<path d=\"M{},{}L{},{}\" fill=\"none\"{line}/>",
            num(x - 12.),
            num(y),
            num(x),
            num(y - 12.)
        )
}

fn draw_tag_doc(_shape: &str, w: f64, h: f64, a: &str, line: &str) -> String {
    let x = w / 2.;
    let y = h / 2.;
    wave(w, h, a)
        + &format!(
            "<path d=\"M{},{}L{},{}\" fill=\"none\"{line}/>",
            num(x - 12.),
            num(y - 14.),
            num(x),
            num(y - 26.)
        )
}

fn draw_lin_doc(_shape: &str, w: f64, h: f64, a: &str, line: &str) -> String {
    let x = w / 2.;
    let y = h / 2.;
    wave(w, h, a)
        + &format!(
            "<path d=\"M{},{}V{}\" fill=\"none\"{line}/>",
            num(-x + 8.),
            num(-y),
            num(y - 6.)
        )
}

fn draw_collapsed_group(_shape: &str, w: f64, h: f64, a: &str, _line: &str) -> String {
    let x = w / 2.;
    let y = h / 2.;
    format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"{ROUNDED}{a}/><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"{ROUNDED}{a}/>",num(-x+6.),num(-y+8.),num(w-12.),num(h-8.),num(-x),num(-y),num(w),num(h-4.))
}
