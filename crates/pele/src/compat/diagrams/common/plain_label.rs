//! Pele common/fit.ts plainLabel, 458322a5 (MIT, kepano).
use crate::compat::{
    text::{layout_label, Label, SPACE},
    PeleError, RenderResources, RenderSettings,
};
use regex::Regex;
use std::sync::OnceLock;
pub(crate) fn plain_label(
    raw: Option<&str>,
    r: &mut dyn RenderResources,
    o: &RenderSettings,
    size: f64,
    max: f64,
    base: u8,
) -> Result<Label, PeleError> {
    let Some(raw) = raw else {
        return layout_label(None, false, r, o, size, max, base);
    };
    static BR: OnceLock<Regex> = OnceLock::new();
    let br = BR.get_or_init(|| Regex::new(&format!("(?i)^br{SPACE}*/?>")).unwrap());
    let mut text = String::new();
    for (i, c) in raw.char_indices() {
        if c == '<' && !br.is_match(&raw[i + 1..]) {
            text.push_str("&lt;")
        } else {
            text.push(c)
        }
    }
    let text = text.replace(":fa-", "&colon;fa-");
    layout_label(Some(&text), false, r, o, size, max, base)
}
