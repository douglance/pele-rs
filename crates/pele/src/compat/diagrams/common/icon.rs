//! Pele common/icon.ts, 458322a5 (MIT, kepano).
use crate::compat::{
    svg::{esc_text, num},
    PeleError, RenderResources,
};
pub(crate) fn icon_svg(
    name: &str,
    x: f64,
    y: f64,
    side: f64,
    r: &mut dyn RenderResources,
) -> Result<String, PeleError> {
    let inner = r
        .icon(name)?
        .map(|s| s.markup().to_string())
        .unwrap_or_default();
    Ok(format!(
        "<svg class=\"pele-icon\" data-icon=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\">{inner}</svg>",
        esc_text(name)?,
        num(x),
        num(y),
        num(side),
        num(side)
    ))
}
