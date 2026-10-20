use super::*;
use super::{
    placement::display,
    validation::{invalid, Budget},
};
use crate::compat::{text::metrics_width, PeleError};
pub(super) fn labels(s: &mut Scene, b: &mut Budget) -> Result<(), PeleError> {
    let mut occupied = s
        .nodes
        .iter()
        .map(|n| n.rect.expand(6.))
        .collect::<Vec<_>>();
    for e in &mut s.edges {
        if e.edge.label.is_empty() {
            continue;
        }
        let width = metrics_width(&display(&e.edge.label, 24), 11., 0) + 12.;
        let placed = place(&e.points, width, &occupied, b)?
            .ok_or_else(|| invalid(format!("no label space for {}", e.edge.id)))?;
        e.label_box = placed;
        occupied.push(placed.expand(4.));
    }
    Ok(())
}
fn place(
    points: &[Point],
    width: f64,
    occupied: &[Rect],
    b: &mut Budget,
) -> Result<Option<Rect>, PeleError> {
    let mut segments = points.windows(2).collect::<Vec<_>>();
    segments.sort_by(|a, z| z[0].distance(z[1]).total_cmp(&a[0].distance(a[1])));
    for segment in segments {
        for fraction in [0.5, 0.25, 0.75] {
            let p = Point {
                x: segment[0].x + (segment[1].x - segment[0].x) * fraction,
                y: segment[0].y + (segment[1].y - segment[0].y) * fraction,
            };
            for offset in [0., -24., 24., -48., 48.] {
                let r = Rect {
                    x: p.x - width / 2.,
                    y: p.y - 10. + offset,
                    width,
                    height: 20.,
                };
                b.spend(occupied.len() + 1)?;
                if !occupied.iter().any(|n| n.overlaps(r)) {
                    return Ok(Some(r));
                }
            }
        }
    }
    Ok(None)
}
