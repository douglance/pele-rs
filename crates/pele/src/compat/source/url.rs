//! Pele util/url.ts sanitizer and host scheme gates, 458322a5 (MIT, kepano).
use super::{decode_entities, space};
use crate::compat::svg::esc;
use crate::compat::{LinkInfo, PeleError, RenderSettings};
use regex::Regex;
use std::sync::OnceLock;
fn re(index: usize) -> &'static Regex {
    static R: OnceLock<Vec<Regex>> = OnceLock::new();
    &R.get_or_init(|| {
        [
            r"(?m)^([^A-Za-z0-9_]*)(?i-u:javascript|data|vbscript)",
            r"&#([A-Za-z0-9_]+)(^[A-Za-z0-9_]|;)?",
            r"(?i)&(newline|tab);",
            r"[\u0000-\u001F\u007F-\u009F -‍﻿]",
            r"(?im)^.+(:|&colon;)",
            r"(\\|%5[cC])((%(6[eE]|72|74))|[nrt])",
            r"(?i-u:^([a-z][a-z0-9+.-]*):)",
            r"^[\\/]{2}",
        ]
        .iter()
        .map(|p| Regex::new(p).unwrap())
        .collect()
    })[index]
}
fn decode(s: &str) -> String {
    let mut out = Vec::new();
    let b = s.as_bytes();
    let mut p = 0;
    while p < b.len() {
        if b[p] == b'%' {
            if p + 2 >= b.len() {
                return s.into();
            }
            let Some(a) = (b[p + 1] as char).to_digit(16) else {
                return s.into();
            };
            let Some(c) = (b[p + 2] as char).to_digit(16) else {
                return s.into();
            };
            out.push((a * 16 + c) as u8);
            p += 3
        } else {
            out.push(b[p]);
            p += 1
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| s.into())
}
fn html_code(s: &str) -> String {
    let n = if s.starts_with("0x") || s.starts_with("0X") {
        u64::from_str_radix(&s[2..], 16).ok()
    } else if s.is_empty() {
        Some(0)
    } else {
        s.parse::<f64>().ok().map(|v| {
            if v.is_finite() {
                v.trunc().rem_euclid(65536.) as u64
            } else {
                0
            }
        })
    };
    let unit = n.unwrap_or(0) as u16;
    char::decode_utf16([unit])
        .map(|v| v.unwrap_or('\u{fffd}'))
        .collect()
}
pub(crate) fn sanitize_url(url: &str) -> String {
    if url.is_empty() {
        return "about:blank".into();
    }
    let mut decoded = decode(url.trim_matches(space));
    for round in 1..=33 {
        if round > 32 {
            return "about:blank".into();
        }
        decoded = re(3).replace_all(&decoded, "").into_owned();
        decoded = re(1)
            .replace_all(&decoded, |c: &regex::Captures<'_>| html_code(&c[1]))
            .into_owned();
        decoded = re(2).replace_all(&decoded, "").into_owned();
        decoded = re(3).replace_all(&decoded, "").into_owned();
        decoded = re(5).replace_all(&decoded, "").trim_matches(space).into();
        decoded = decode(&decoded);
        if ![3, 1, 2, 5].iter().any(|i| re(*i).is_match(&decoded)) {
            break;
        }
    }
    if decoded.is_empty() {
        return "about:blank".into();
    }
    if decoded.starts_with(['.', '/']) {
        return decoded;
    }
    let trimmed = decoded.trim_start_matches(space);
    // JavaScript multiline anchors recognize LF, CR, LS and PS. The source
    // sanitizer removes the first two, but LS/PS remain significant here.
    let Some(scheme) = trimmed
        .split(['\n', '\r', '\u{2028}', '\u{2029}'])
        .find_map(|line| re(4).find(line))
    else {
        return decoded;
    };
    let protocol = scheme
        .as_str()
        .to_lowercase()
        .trim_matches(space)
        .to_string();
    if re(0).is_match(&protocol) {
        return "about:blank".into();
    }
    let normalized = trimmed.replace('\\', "/");
    if protocol == "mailto:" || protocol.contains("://") {
        return normalized;
    }
    if protocol == "http:" || protocol == "https:" {
        return ::url::Url::parse(&normalized)
            .map_or_else(|_| "about:blank".into(), |u| u.to_string());
    }
    normalized
}
pub(crate) fn safe_url(url: &str) -> Result<String, PeleError> {
    Ok(sanitize_url(&decode_entities(url)?))
}
fn allow(url: &str, schemes: &[String]) -> Option<String> {
    if re(7).is_match(url) {
        return None;
    }
    if let Some(c) = re(6).captures(url) {
        if !schemes.contains(&c[1].to_lowercase()) {
            return None;
        }
    }
    Some(url.into())
}
pub(crate) fn link_url(url: &str, options: &RenderSettings) -> Option<String> {
    if options.links == Some(false) {
        None
    } else {
        allow(
            url,
            options.link_schemes.as_deref().unwrap_or(&[
                "http".into(),
                "https".into(),
                "mailto".into(),
                "tel".into(),
            ]),
        )
    }
}
pub(crate) fn image_url(url: &str, options: &RenderSettings) -> Option<String> {
    if options.images == Some(false) {
        None
    } else {
        allow(
            url,
            options
                .image_schemes
                .as_deref()
                .unwrap_or(&["http".into(), "https".into()]),
        )
    }
}
pub(crate) fn rel_attr(options: &RenderSettings) -> String {
    let rel = options.link_rel.as_deref().unwrap_or("noopener");
    if rel.is_empty() {
        String::new()
    } else {
        format!(" rel=\"{}\"", esc(rel))
    }
}
pub(crate) fn linked(
    body: &str,
    url: &str,
    id: &str,
    links: &mut Vec<LinkInfo>,
    options: &RenderSettings,
    attrs: &str,
) -> String {
    let Some(href) = link_url(url, options) else {
        return body.into();
    };
    links.push(LinkInfo {
        id: id.into(),
        href: href.clone(),
        internal: false,
    });
    format!(
        "<a href=\"{}\"{attrs}{}>{body}</a>",
        esc(&href),
        rel_attr(options)
    )
}
