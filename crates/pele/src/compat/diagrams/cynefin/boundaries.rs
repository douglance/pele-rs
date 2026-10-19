//! Mermaid's seeded domain waves and S-shaped cliff, Pele 458322a5 (MIT).
use crate::compat::svg::num;
fn int32(n: f64) -> u32 {
    if !n.is_finite() || n == 0. {
        return 0;
    }
    n.trunc().rem_euclid(4294967296.) as u32
}
pub(super) fn random(seed: f64) -> f64 {
    let mut t = int32(seed + 0x6d2b79f5u32 as f64);
    t = (t ^ (t >> 15)).wrapping_mul(t | 1);
    t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
    (t ^ (t >> 14)) as f64 / 4294967296.
}
pub(super) fn resolve_seed(configured: f64, id: &str) -> f64 {
    if configured.is_finite() && configured != 0. {
        return configured;
    }
    let mut hash = 0i32;
    for c in id.encode_utf16() {
        hash = hash.wrapping_mul(31).wrapping_add(c as i32)
    }
    hash as f64
}
pub(super) fn wave(
    from: f64,
    to: f64,
    center: f64,
    seed: f64,
    amplitude: f64,
    vertical: bool,
    pin_start: bool,
    pin_end: bool,
) -> Vec<f64> {
    let segments = 4;
    let step = (to - from) / segments as f64;
    let mut along = vec![];
    let mut across = vec![];
    for i in 0..=segments {
        let pinned = i == 0 && pin_start || i == segments && pin_end;
        along.push(from + i as f64 * step);
        across.push(if pinned {
            center
        } else {
            center + random(seed + i as f64 * if vertical { 17. } else { 23. }) * amplitude * 2.
                - amplitude
        });
    }
    let mut out = if vertical {
        vec![across[0], along[0]]
    } else {
        vec![along[0], across[0]]
    };
    for i in 0..segments {
        let mid = (along[i] + along[i + 1]) / 2.;
        let offset = amplitude
            * 1.5
            * if i % 2 == 0 { 1. } else { -1. }
            * random(
                seed + i as f64 * if vertical { 31. } else { 37. }
                    + if vertical { 7. } else { 11. },
            );
        if vertical {
            out.extend([
                across[i] + offset,
                mid,
                across[i + 1] - offset,
                mid,
                across[i + 1],
                along[i + 1],
            ])
        } else {
            out.extend([
                mid,
                across[i] + offset,
                mid,
                across[i + 1] - offset,
                along[i + 1],
                across[i + 1],
            ])
        }
    }
    out
}
pub(super) fn cliff(cx: f64, top: f64, bottom: f64, a: f64) -> Vec<f64> {
    let h = bottom - top;
    vec![
        cx,
        top,
        cx + a,
        top + h * 0.2,
        cx - a * 1.5,
        top + h * 0.55,
        cx + a * 0.5,
        top + h * 0.75,
        cx - a,
        top + h * 0.85,
        cx + a * 0.3,
        top + h * 0.95,
        cx,
        bottom,
    ]
}
pub(super) fn forward(c: &[f64]) -> String {
    let mut d = String::new();
    for v in c[2..].chunks_exact(6) {
        d += &format!(
            "C{},{} {},{} {},{}",
            num(v[0]),
            num(v[1]),
            num(v[2]),
            num(v[3]),
            num(v[4]),
            num(v[5])
        )
    }
    d
}
pub(super) fn backward(c: &[f64]) -> String {
    let mut d = String::new();
    for i in (2..c.len() - 5).step_by(6).rev() {
        d += &format!(
            "C{},{} {},{} {},{}",
            num(c[i + 2]),
            num(c[i + 3]),
            num(c[i]),
            num(c[i + 1]),
            num(c[i - 2]),
            num(c[i - 1])
        )
    }
    d
}
pub(super) fn at(c: &[f64], end: bool) -> String {
    let i = if end { c.len() - 2 } else { 0 };
    format!("{},{}", num(c[i]), num(c[i + 1]))
}
pub(super) fn curve(c: &[f64]) -> String {
    format!("M{}{}", at(c, false), forward(c))
}
pub(super) fn ellipse(cx: f64, cy: f64, rx: f64, ry: f64) -> String {
    let r = format!("{},{}", num(rx), num(ry));
    format!(
        "M{},{}A{r} 0 1,1 {},{}A{r} 0 1,1 {},{}Z",
        num(cx - rx),
        num(cy),
        num(cx + rx),
        num(cy),
        num(cx - rx),
        num(cy)
    )
}
