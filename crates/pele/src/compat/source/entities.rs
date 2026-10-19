//! Pele src/preprocess.ts entity placeholders and color-semicolon scan (458322a5).
use super::detect::{space, word};
fn drop_color_semicolons(text: &str, keyword: &str) -> String {
    let mut out = String::new();
    let mut last = 0;
    let mut cursor = 0;
    while let Some(from) = text[cursor..].find(keyword).map(|i| cursor + i) {
        let end = text[from..]
            .find(['\n', '\r', '\u{2028}', '\u{2029}'])
            .map_or(text.len(), |i| from + i);
        let mut after_colon = false;
        let mut hash = None;
        for (i, c) in text[from + keyword.len()..end].char_indices() {
            if c == ':' {
                after_colon = true
            } else if c == '#' {
                if after_colon {
                    hash = Some(from + keyword.len() + i);
                    break;
                }
            } else if space(c) {
                after_colon = false
            }
        }
        if let Some(hash) = hash {
            if let Some(semi) = text[..end].rfind(';').filter(|i| *i > hash) {
                out.push_str(&text[last..semi]);
                last = semi + 1
            }
        }
        if end >= text.len() {
            break;
        }
        cursor = end + text[end..].chars().next().unwrap().len_utf8();
    }
    if last == 0 {
        text.into()
    } else {
        out.push_str(&text[last..]);
        out
    }
}
pub(super) fn encode(text: &str) -> String {
    if !text.contains('#') {
        return text.into();
    }
    let text = drop_color_semicolons(&drop_color_semicolons(text, "style"), "classDef");
    let mut out = String::new();
    let mut cursor = 0;
    while let Some(start) = text[cursor..].find('#').map(|i| cursor + i) {
        out.push_str(&text[cursor..start]);
        let end = text[start + 1..]
            .char_indices()
            .find(|(_, c)| !word(*c))
            .map_or(text.len(), |(i, _)| start + 1 + i);
        if end > start + 1 && text.as_bytes().get(end) == Some(&b';') {
            let inner = &text[start + 1..end];
            out.push_str(if inner.bytes().all(|c| c.is_ascii_digit()) {
                "ﬂ°°"
            } else {
                "ﬂ°"
            });
            out.push_str(inner);
            out.push_str("¶ß");
            cursor = end + 1
        } else {
            out.push('#');
            cursor = start + 1
        }
    }
    out.push_str(&text[cursor..]);
    out
}
