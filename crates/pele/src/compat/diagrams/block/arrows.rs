//! Pele block/arrows.ts, 458322a5 (MIT, kepano).
use crate::compat::{source::space, svg::num};
pub(super) const OVERHANG: f64 = 7.;
const RIGHT: u8 = 1;
const LEFT: u8 = 2;
const UP: u8 = 4;
const DOWN: u8 = 8;
pub(super) fn mask(directions: Option<&[String]>) -> u8 {
    let mut mask = 0;
    for d in directions.unwrap_or(&[]) {
        mask |= match d.trim_matches(space) {
            "right" => RIGHT,
            "left" => LEFT,
            "up" => UP,
            "down" => DOWN,
            "x" => RIGHT | LEFT,
            "y" => UP | DOWN,
            _ => 0,
        };
    }
    mask
}
pub(super) fn axis(mask: u8) -> char {
    if mask != 0 && mask & (UP | DOWN) == 0 {
        'x'
    } else if mask != 0 && mask & (RIGHT | LEFT) == 0 {
        'y'
    } else {
        ' '
    }
}
fn bit(mask: u8, flag: u8) -> f64 {
    f64::from(u8::from(mask & flag != 0))
}
pub(super) fn size(mask: u8, tw: f64, th: f64) -> (f64, f64, f64, f64) {
    let across = bit(mask, LEFT) - bit(mask, RIGHT);
    let along = bit(mask, UP) - bit(mask, DOWN);
    match axis(mask) {
        'x' => {
            let h = (th + 8.).max(14.) + 2. * OVERHANG;
            let depth = (h / 2.).min(20.);
            (
                (tw + 12.).max(16.) + depth * if mask == RIGHT | LEFT { 2. } else { 1. },
                h,
                across * depth / 2.,
                0.,
            )
        }
        'y' => {
            let w = (tw + 16.).max(14.) + 2. * OVERHANG;
            let depth = (w / 2.).min(20.);
            (
                w,
                (th + 8.).max(16.) + depth * if mask == UP | DOWN { 2. } else { 1. },
                0.,
                along * depth / 2.,
            )
        }
        _ => (
            (tw + 16.).max(24.) + 12. * (bit(mask, LEFT) + bit(mask, RIGHT)),
            (th + 8.).max(24.) + 12. * (bit(mask, UP) + bit(mask, DOWN)),
            across * 12. / 2.,
            along * 12. / 2.,
        ),
    }
}
pub(super) fn draw(mask: u8, w: f64, h: f64, attrs: &str) -> String {
    let x = w / 2.;
    let y = h / 2.;
    let mut p;
    match axis(mask) {
        'x' => {
            let depth = y.min(20.);
            let s = y - OVERHANG;
            let x0 = if mask & LEFT != 0 { -x + depth } else { -x };
            let x1 = if mask & RIGHT != 0 { x - depth } else { x };
            p = vec![x0, -s, x1, -s];
            if mask & RIGHT != 0 {
                p.extend([x1, -y, x, 0., x1, y]);
            }
            p.extend([x1, s, x0, s]);
            if mask & LEFT != 0 {
                p.extend([x0, y, -x, 0., x0, -y]);
            }
        }
        'y' => {
            let depth = x.min(20.);
            let s = x - OVERHANG;
            let y0 = if mask & UP != 0 { -y + depth } else { -y };
            let y1 = if mask & DOWN != 0 { y - depth } else { y };
            p = vec![s, y0, s, y1];
            if mask & DOWN != 0 {
                p.extend([x, y1, 0., y, -x, y1]);
            }
            p.extend([-s, y1, -s, y0]);
            if mask & UP != 0 {
                p.extend([-x, y0, 0., -y, x, y0]);
            }
        }
        _ => {
            let x0 = if mask & LEFT != 0 { -x + 12. } else { -x };
            let x1 = if mask & RIGHT != 0 { x - 12. } else { x };
            let y0 = if mask & UP != 0 { -y + 12. } else { -y };
            let y1 = if mask & DOWN != 0 { y - 12. } else { y };
            let cx = (x0 + x1) / 2.;
            let cy = (y0 + y1) / 2.;
            let kx = 10_f64.min((x1 - x0) / 2.);
            let ky = 10_f64.min((y1 - y0) / 2.);
            p = vec![x0, y0];
            if mask & UP != 0 {
                p.extend([cx - kx, y0, cx, -y, cx + kx, y0]);
            }
            p.extend([x1, y0]);
            if mask & RIGHT != 0 {
                p.extend([x1, cy - ky, x, cy, x1, cy + ky]);
            }
            p.extend([x1, y1]);
            if mask & DOWN != 0 {
                p.extend([cx + kx, y1, cx, y, cx - kx, y1]);
            }
            p.extend([x0, y1]);
            if mask & LEFT != 0 {
                p.extend([x0, cy + ky, -x, cy, x0, cy - ky]);
            }
        }
    }
    let points = p
        .chunks_exact(2)
        .map(|p| format!("{},{}", num(p[0]), num(p[1])))
        .collect::<Vec<_>>()
        .join(" ");
    format!("<polygon points=\"{points}\" stroke-linejoin=\"round\"{attrs}/>")
}
