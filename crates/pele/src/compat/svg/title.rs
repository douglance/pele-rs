//! Pele svg/title.ts, 458322a5 (MIT, kepano).
use super::{label_svg, num};
use crate::compat::number_math::NumberMath;
use crate::compat::{text::layout_label, PeleError, RenderResources, RenderSettings};
pub(crate) struct Titled {
    pub width: f64,
    pub height: f64,
    pub content: String,
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn with_title(
    title: Option<&str>,
    resources: &mut dyn RenderResources,
    options: &RenderSettings,
    size: f64,
    pad: f64,
    width: f64,
    height: f64,
    content: String,
) -> Result<Titled, PeleError> {
    let label = layout_label(title, false, resources, options, size, 4000., 1)?;
    if label.height == 0. {
        return Ok(Titled {
            width,
            height,
            content,
        });
    }
    let shift = label.height + 12.;
    let full = width.js_max((label.width + 2. * pad).ceil());
    Ok(Titled {
        width: full,
        height: (height + shift).ceil(),
        content: format!(
            "{}<g transform=\"translate({},{})\">{content}</g>",
            label_svg(
                &label,
                full / 2.,
                pad + label.height / 2.,
                " class=\"pele-title\" font-weight=\"var(--_tw)\""
            ),
            num((full - width) / 2.),
            num(shift)
        ),
    })
}
