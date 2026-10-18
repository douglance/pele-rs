//! Pele info/render.ts, 458322a5 (MIT, kepano).
use super::InfoModel;
use crate::compat::{
    svg::{document, label_svg},
    text::layout_label,
    PeleError, RenderResources, RenderSettings, Rendered,
};
// Layout resolves host icons; rendering preserves their reserved slots with empty markup.
pub(super) fn render(
    model: &InfoModel,
    options: &RenderSettings,
    resources: &mut dyn RenderResources,
) -> Result<Rendered, PeleError> {
    let size = options.font_size.unwrap_or(16.);
    let pad = options.padding.unwrap_or(8.);
    let title = layout_label(
        model.title.as_deref(),
        false,
        resources,
        options,
        size,
        4000.,
        1,
    )?;
    let title_height = if title.height > 0. {
        title.height + 8.
    } else {
        0.
    };
    let text = layout_label(
        Some("Pele v1.6.0"),
        false,
        resources,
        options,
        size,
        4000.,
        0,
    )?;
    let width = (title.width.max(text.width) + 2. * pad).ceil();
    let height = (title_height + text.height + 2. * pad).ceil();
    let content = format!(
        "{}{}",
        label_svg(
            &title,
            width / 2.,
            pad + title.height / 2.,
            " class=\"pele-title\" font-weight=\"var(--_tw)\""
        ),
        label_svg(
            &text,
            width / 2.,
            pad + title_height + text.height / 2.,
            " class=\"pele-version\" fill=\"var(--_m)\""
        )
    );
    Ok(Rendered {
        svg: document(
            "info",
            width,
            height,
            size,
            options,
            model.acc_title.as_deref(),
            model.acc_descr.as_deref(),
            &content,
        )?,
        width,
        height,
        links: Vec::new(),
    })
}
