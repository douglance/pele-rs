//! Pele er/markers.ts crow's foot geometry, 458322a5 (MIT, kepano).
use crate::compat::svg::num;
pub(super) const MARKER_LENGTH: f64 = 18.;
pub(super) fn marker(kind: &str, x: f64, y: f64, dx: f64, dy: f64) -> String {
    let at = |back: f64, side: f64| {
        format!(
            "{},{}",
            num(x - dx * back - dy * side),
            num(y - dy * back + dx * side)
        )
    };
    let bar = |back: f64| format!("M{}L{}", at(back, -5.), at(back, 5.));
    let foot = format!("M{}L{}L{}", at(0., -6.), at(10., 0.), at(0., 6.));
    let ring = |back: f64| {
        format!(
            "<circle class=\"pele-marker\" cx=\"{}\" cy=\"{}\" r=\"3.5\" fill=\"var(--_bg)\"/>",
            num(x - dx * back),
            num(y - dy * back)
        )
    };
    let path = |d: String| format!("<path class=\"pele-marker\" d=\"{d}\"/>");
    match kind {
        "only_one" => path(bar(6.) + &bar(10.)),
        "zero_or_one" => path(bar(6.)) + &ring(13.5),
        "one_or_more" => path(foot + &bar(13.5)),
        "zero_or_more" => path(foot) + &ring(14.),
        "md_parent" => format!(
            "<path class=\"pele-marker\" d=\"M{}L{}L{}L{}Z\" fill=\"var(--_bg)\"/>",
            at(0., 0.),
            at(6., 4.),
            at(12., 0.),
            at(6., -4.)
        ),
        _ => String::new(),
    }
}
