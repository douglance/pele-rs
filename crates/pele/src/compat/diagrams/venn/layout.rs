//! Exact circle/region geometry and bounded inferred-pair expansion, Pele 458322a5 (MIT).
use super::math::{acos, atan2, cos, hypot, max, min, sin};
use super::model::VennSubset;
use crate::compat::{ErrorCode, PeleError};
use std::collections::{HashMap, HashSet};
const TAU: f64 = std::f64::consts::PI * 2.;
pub(super) const MAX_INFERRED_PAIR_WORK: usize = 3_000_000;
#[derive(Clone, Debug)]
pub(super) struct Circle {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub r: f64,
}
#[derive(Clone, Copy)]
pub(super) struct Spot {
    pub x: f64,
    pub y: f64,
    pub margin: f64,
}
fn sorted(ids: &[String]) -> Vec<String> {
    let mut out = ids.to_vec();
    out.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
    out
}
fn pair_budget(subsets: &[VennSubset]) -> Result<(), PeleError> {
    let mut work = 0usize;
    for s in subsets {
        let k = s.sets.len();
        if k < 3 {
            continue;
        }
        let pairs = k.checked_mul(k - 1).map(|n| n / 2);
        work = work
            .checked_add(pairs.unwrap_or(usize::MAX))
            .unwrap_or(usize::MAX);
        if work > MAX_INFERRED_PAIR_WORK {
            let mut e = PeleError::new(
                "Venn inferred-pair expansion exceeds the work limit of 3000000 pairs.",
                ErrorCode::Limit,
            );
            e.diagram_type = Some("venn".into());
            return Err(e);
        }
    }
    Ok(())
}
fn ensure_pairs(subsets: &[VennSubset]) -> Result<Vec<VennSubset>, PeleError> {
    pair_budget(subsets)?;
    let mut present = HashSet::new();
    let mut sizes = HashMap::new();
    for s in subsets {
        present.insert(sorted(&s.sets));
        if s.sets.len() == 1 {
            sizes.insert(s.sets[0].clone(), s.size);
        }
    }
    let mut added = vec![];
    for s in subsets {
        if s.sets.len() < 3 {
            continue;
        }
        let members = sorted(&s.sets);
        for i in 0..members.len() - 1 {
            for j in i + 1..members.len() {
                let pair = vec![members[i].clone(), members[j].clone()];
                if !present.insert(pair.clone()) {
                    continue;
                }
                let size = match (sizes.get(&pair[0]), sizes.get(&pair[1])) {
                    (Some(a), Some(b)) => min(*a, *b) / 4.,
                    _ => 2.5,
                };
                added.push(VennSubset {
                    sets: pair,
                    size,
                    label: Some(String::new()),
                });
            }
        }
    }
    let mut out = subsets.to_vec();
    out.extend(added);
    Ok(out)
}
fn lens(r1: f64, r2: f64, d: f64) -> f64 {
    if d >= r1 + r2 {
        return 0.;
    }
    let small = min(r1, r2);
    if d <= (r1 - r2).abs() {
        return std::f64::consts::PI * small * small;
    }
    let a1 = r1 * r1 * acos((d * d + r1 * r1 - r2 * r2) / (2. * d * r1));
    let a2 = r2 * r2 * acos((d * d + r2 * r2 - r1 * r1) / (2. * d * r2));
    let a3 = 0.5
        * max(
            (-d + r1 + r2) * (d + r1 - r2) * (d - r1 + r2) * (d + r1 + r2),
            0.,
        )
        .sqrt();
    a1 + a2 - a3
}
fn distance_overlap(r1: f64, r2: f64, area: f64) -> f64 {
    let mut low = (r1 - r2).abs();
    let mut high = r1 + r2;
    if !(area > 0.) {
        return high;
    }
    if area >= std::f64::consts::PI * min(r1, r2).powi(2) {
        return low;
    }
    for _ in 0..48 {
        let middle = (low + high) / 2.;
        if lens(r1, r2, middle) > area {
            low = middle
        } else {
            high = middle
        }
    }
    (low + high) / 2.
}
fn margin(
    circles: &[Circle],
    inside: &[usize],
    outside: &[usize],
    x: f64,
    y: f64,
    floor: f64,
) -> f64 {
    let mut best = f64::INFINITY;
    for &i in inside {
        let c = &circles[i];
        let dx = x - c.x;
        let dy = y - c.y;
        let room = c.r - (dx * dx + dy * dy).sqrt();
        if room < best {
            if room < floor {
                return room;
            }
            best = room
        }
    }
    for &i in outside {
        let c = &circles[i];
        let dx = x - c.x;
        let dy = y - c.y;
        let room = (dx * dx + dy * dy).sqrt() - c.r;
        if room < best {
            if room < floor {
                return room;
            }
            best = room
        }
    }
    best
}
struct Search<'a> {
    circles: &'a [Circle],
    inside: &'a [usize],
    outside: &'a [usize],
    r: f64,
    cx: f64,
    cy: f64,
    x: f64,
    y: f64,
    best: f64,
    off: f64,
}
impl Search<'_> {
    fn consider(&mut self, x: f64, y: f64) -> bool {
        let m = margin(
            self.circles,
            self.inside,
            self.outside,
            x,
            y,
            self.best - self.r * 1e-9,
        );
        if !(m > self.best - self.r * 1e-9) {
            return false;
        }
        let distance = hypot(x - self.cx, y - self.cy);
        if m > self.best + self.r * 1e-9 || distance < self.off - self.r * 1e-9 {
            self.best = m;
            self.off = distance;
            self.x = x;
            self.y = y;
            true
        } else {
            false
        }
    }
}
pub(super) fn find_spot(circles: &[Circle], inside: &[usize], outside: &[usize]) -> Spot {
    let (mut cx, mut cy) = (0., 0.);
    let mut smallest = &circles[inside[0]];
    for &i in inside {
        let c = &circles[i];
        cx += c.x / inside.len() as f64;
        cy += c.y / inside.len() as f64;
        if c.r < smallest.r {
            smallest = c
        }
    }
    let r = smallest.r;
    let mut s = Search {
        circles,
        inside,
        outside,
        r,
        cx,
        cy,
        x: cx,
        y: cy,
        best: margin(circles, inside, outside, cx, cy, f64::NEG_INFINITY),
        off: 0.,
    };
    for gy in -10..=10 {
        for gx in -10..=10 {
            if gx * gx + gy * gy <= 100 {
                s.consider(
                    smallest.x + gx as f64 * r / 10.,
                    smallest.y + gy as f64 * r / 10.,
                );
            }
        }
    }
    let mut step = r / 10.;
    for _ in 0..40 {
        if !(step > r * 1e-6) {
            break;
        }
        let mut moved = false;
        for k in 0..8 {
            let angle = k as f64 * TAU / 8.;
            if s.consider(s.x + cos(angle) * step, s.y + sin(angle) * step) {
                moved = true
            }
        }
        if !moved {
            step /= 2.
        }
    }
    Spot {
        x: s.x,
        y: s.y,
        margin: s.best,
    }
}
pub(super) fn layout(subsets: &[VennSubset]) -> Result<Vec<Circle>, PeleError> {
    let mut sizes: Vec<(String, f64)> = vec![];
    let mut index = HashMap::new();
    for s in subsets {
        for id in &s.sets {
            if !index.contains_key(id) {
                index.insert(id.clone(), sizes.len());
                sizes.push((id.clone(), 10.));
            }
        }
    }
    for s in subsets {
        if s.sets.len() == 1 {
            sizes[index[&s.sets[0]]].1 = s.size;
        }
    }
    let mut largest = 0.;
    for (_, size) in &sizes {
        if size.is_finite() && *size > largest {
            largest = *size
        }
    }
    if largest == 0. {
        largest = 10.
    }
    let widest = (largest / std::f64::consts::PI).sqrt();
    let mut circles: Vec<_> = sizes
        .into_iter()
        .map(|(id, size)| {
            let r = if size.is_finite() && size > 0. {
                (size / std::f64::consts::PI).sqrt()
            } else {
                0.
            };
            Circle {
                id,
                x: 0.,
                y: 0.,
                r: max(r, widest * 0.3),
            }
        })
        .collect();
    let n = circles.len();
    if n < 2 {
        return Ok(circles);
    }
    let mut overlaps: Vec<(usize, f64)> = vec![];
    let mut oi: HashMap<usize, usize> = HashMap::new();
    for s in ensure_pairs(subsets)? {
        if s.sets.len() != 2 {
            continue;
        }
        let a = index[&s.sets[0]];
        let b = index[&s.sets[1]];
        if a != b {
            let key = a.min(b) * n + a.max(b);
            if let Some(&i) = oi.get(&key) {
                overlaps[i].1 = s.size
            } else {
                oi.insert(key, overlaps.len());
                overlaps.push((key, s.size));
            }
        }
    }
    let mut wanted = HashMap::new();
    for (key, area) in overlaps {
        wanted.insert(
            key,
            distance_overlap(circles[key / n].r, circles[key % n].r, area),
        );
    }
    let gap = widest * 0.15;
    let distance = |a: usize, b: usize| {
        wanted
            .get(&(a * n + b))
            .copied()
            .unwrap_or(circles[a].r + circles[b].r + gap)
    };
    if n == 2 {
        circles[1].x = distance(0, 1);
        return Ok(circles);
    }
    if n == 3 {
        let mut ab = distance(0, 1);
        let mut ac = distance(0, 2);
        let mut bc = distance(1, 2);
        let triple = subsets
            .iter()
            .any(|s| s.sets.iter().collect::<HashSet<_>>().len() == 3);
        for _ in 0..40 {
            let x = if ab > 0. {
                (ab * ab + ac * ac - bc * bc) / (2. * ab)
            } else {
                0.
            };
            circles[1].x = ab;
            circles[2].x = max(-ac, min(ac, x));
            circles[2].y = max(ac * ac - x * x, 0.).sqrt();
            if !triple || find_spot(&circles, &[0, 1, 2], &[]).margin > widest * 0.12 {
                break;
            }
            ab *= 0.94;
            ac *= 0.94;
            bc *= 0.94;
        }
        return Ok(circles);
    }
    let mut total = 0.;
    for c in &circles {
        total += 2. * c.r + gap;
    }
    let rounds = 300usize.min(3_000_000 / (n * n));
    if n <= 64 {
        let ring = total / TAU;
        for (i, c) in circles.iter_mut().enumerate() {
            let angle = i as f64 * TAU / n as f64 - std::f64::consts::PI / 2.;
            c.x = cos(angle) * ring;
            c.y = sin(angle) * ring;
        }
    } else {
        let columns = (n as f64).sqrt().ceil() as usize;
        let cell = 2. * widest + gap;
        for (i, c) in circles.iter_mut().enumerate() {
            c.x = (i % columns) as f64 * cell;
            c.y = (i / columns) as f64 * cell;
        }
    }
    let mut dx = vec![0.; n];
    let mut dy = vec![0.; n];
    let mut pulls = vec![0i32; n];
    for _ in 0..rounds {
        dx.fill(0.);
        dy.fill(0.);
        pulls.fill(0);
        for a in 0..n {
            for b in a + 1..n {
                let overlap = wanted.get(&(a * n + b)).copied();
                let apart = circles[a].r + circles[b].r + gap;
                let mut ux = circles[b].x - circles[a].x;
                let mut uy = circles[b].y - circles[a].y;
                let mut length = hypot(ux, uy);
                if overlap.is_none() && length >= apart {
                    continue;
                }
                if length < 1e-9 {
                    ux = cos((a + b) as f64);
                    uy = sin((a + b) as f64);
                    length = 1.
                }
                let movement = (length - overlap.unwrap_or(apart)) / 2. / length;
                dx[a] += ux * movement;
                dy[a] += uy * movement;
                dx[b] -= ux * movement;
                dy[b] -= uy * movement;
                pulls[a] += 1;
                pulls[b] += 1;
            }
        }
        for i in 0..n {
            if pulls[i] == 0 {
                continue;
            }
            circles[i].x += dx[i] / pulls[i] as f64;
            circles[i].y += dy[i] / pulls[i] as f64;
        }
    }
    Ok(circles)
}
#[derive(Clone)]
struct Corner {
    x: f64,
    y: f64,
    a: usize,
    b: usize,
}
fn within(c: &[Circle], x: f64, y: f64, skip: usize, other: usize) -> bool {
    for (i, c) in c.iter().enumerate() {
        if i != skip && i != other && hypot(x - c.x, y - c.y) > c.r * (1. + 1e-9) {
            return false;
        }
    }
    true
}
pub(super) fn region_path(c: &[Circle], f: fn(f64) -> String) -> String {
    let whole = |c: &Circle| {
        format!(
            "M{},{}A{},{} 0 1 1 {},{}A{},{} 0 1 1 {},{}Z",
            f(c.x - c.r),
            f(c.y),
            f(c.r),
            f(c.r),
            f(c.x + c.r),
            f(c.y),
            f(c.r),
            f(c.r),
            f(c.x - c.r),
            f(c.y)
        )
    };
    if c.len() == 1 {
        return whole(&c[0]);
    }
    let mut corners = vec![];
    for i in 0..c.len() {
        for j in i + 1..c.len() {
            let a = &c[i];
            let b = &c[j];
            let d = hypot(b.x - a.x, b.y - a.y);
            if d >= a.r + b.r || d <= (a.r - b.r).abs() || d == 0. {
                continue;
            }
            let along = (a.r * a.r - b.r * b.r + d * d) / (2. * d);
            let up = max(a.r * a.r - along * along, 0.).sqrt();
            let mx = a.x + (b.x - a.x) * along / d;
            let my = a.y + (b.y - a.y) * along / d;
            for sign in [1., -1.] {
                let x = mx + sign * up * (b.y - a.y) / d;
                let y = my - sign * up * (b.x - a.x) / d;
                if within(c, x, y, i, j) {
                    corners.push(Corner { x, y, a: i, b: j });
                }
            }
        }
    }
    if corners.len() < 2 {
        let mut smallest = 0;
        for i in 0..c.len() {
            if c[i].r < c[smallest].r {
                smallest = i
            }
        }
        for (i, other) in c.iter().enumerate() {
            if i != smallest
                && hypot(other.x - c[smallest].x, other.y - c[smallest].y) + c[smallest].r
                    > other.r * (1. + 1e-9)
            {
                return String::new();
            }
        }
        return whole(&c[smallest]);
    }
    let (mut cx, mut cy) = (0., 0.);
    for p in &corners {
        cx += p.x / corners.len() as f64;
        cy += p.y / corners.len() as f64;
    }
    corners.sort_by(|p, q| {
        (atan2(p.y - cy, p.x - cx) - atan2(q.y - cy, q.x - cx))
            .partial_cmp(&0.)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut d = String::new();
    for i in 0..corners.len() {
        let from = &corners[i];
        let to = &corners[(i + 1) % corners.len()];
        if i == 0 {
            d = format!("M{},{}", f(from.x), f(from.y))
        }
        let mut arc = None;
        let mut large = 0;
        let mut narrowest = f64::INFINITY;
        for id in [from.a, from.b] {
            if id != to.a && id != to.b {
                continue;
            }
            let c = &c[id];
            let start = atan2(from.y - c.y, from.x - c.x);
            let mut sweep = atan2(to.y - c.y, to.x - c.x) - start;
            if sweep <= 0. {
                sweep += TAU
            }
            let mx = c.x + c.r * cos(start + sweep / 2.);
            let my = c.y + c.r * sin(start + sweep / 2.);
            let width = hypot(mx - (from.x + to.x) / 2., my - (from.y + to.y) / 2.);
            if width < narrowest {
                narrowest = width;
                arc = Some(c);
                large = if sweep > std::f64::consts::PI { 1 } else { 0 };
            }
        }
        if let Some(c) = arc {
            d += &format!("A{},{} 0 {large} 1 {},{}", f(c.r), f(c.r), f(to.x), f(to.y));
        } else {
            d += &format!("L{},{}", f(to.x), f(to.y));
        }
    }
    d + "Z"
}
