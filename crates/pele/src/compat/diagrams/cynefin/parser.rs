//! Exact Cynefin token precedence and grammar, Pele 458322a5 (MIT).
use super::super::common::langium::{self as l, Reader, TokenSpec as T, END};
use super::model::*;
use crate::compat::{source::space, ParseContext, PeleError};
fn keyword(s: &str, p: usize) -> Option<usize> {
    l::keyword(s, p, "cynefin-beta")
}
fn domain(s: &str, p: usize) -> Option<usize> {
    ["complex", "complicated", "clear", "chaotic", "confusion"]
        .iter()
        .find_map(|n| s[p..].starts_with(n).then_some(p + n.len()))
}
static TOKENS: &[T] = &[
    T::literal("cynefin-beta:", &[]),
    T::matcher("cynefin-beta", keyword, false, Some("c"), None),
    T::literal("-->", &[]),
    T::literal(":", &[]),
    T::matcher("DOMAIN_NAME", domain, false, Some("c"), None),
    l::ACC_DESCR,
    l::ACC_TITLE,
    l::TITLE,
    l::STRING,
    l::NEWLINE,
    l::WHITESPACE,
    l::YAML,
    l::DIRECTIVE,
    l::SINGLE_LINE_COMMENT,
];
fn skip(r: &mut Reader<'_>) {
    while r.accept(9) {}
}
fn end_line(r: &mut Reader<'_>) -> Result<(), PeleError> {
    if r.kind() == END {
        return Ok(());
    }
    if r.kind() != 9 {
        return Err(r.fail("a line break or the end of input"));
    }
    skip(r);
    Ok(())
}
pub(super) fn parse(c: ParseContext<'_>) -> Result<CynefinModel, PeleError> {
    let mut r = Reader::new(c.source, TOKENS, "cynefin")?;
    let mut m = CynefinModel {
        diagram_type: "cynefin",
        title: None,
        acc_title: None,
        acc_descr: None,
        domains: vec![],
        transitions: vec![],
    };
    skip(&mut r);
    if !matches!(r.kind(), 0 | 1) {
        return Err(r.fail("'cynefin-beta'"));
    }
    r.i += 1;
    while r.kind() != END {
        match r.kind() {
            9 => r.i += 1,
            5 => {
                m.acc_descr = Some(l::acc_descr_value(r.take()));
                end_line(&mut r)?
            }
            6 => {
                m.acc_title = Some(l::acc_title_value(r.take()));
                end_line(&mut r)?
            }
            7 => {
                m.title = Some(l::title_value(r.take()));
                end_line(&mut r)?
            }
            4 => {
                let name = DomainName::parse(r.take());
                if r.accept(2) {
                    let to = DomainName::parse(r.expect(4)?);
                    let label = if r.accept(3) {
                        Some(l::string_value(r.expect(8)?)).filter(|s| !s.is_empty())
                    } else {
                        None
                    };
                    end_line(&mut r)?;
                    if name != to {
                        m.transitions.push(CynefinTransition {
                            from: name,
                            to,
                            label,
                        });
                    }
                } else {
                    let mut items = vec![];
                    skip(&mut r);
                    while r.kind() == 8 {
                        items.push(CynefinItem {
                            label: l::string_value(r.take()),
                        });
                        skip(&mut r)
                    }
                    let value = CynefinDomain { name, items };
                    if let Some((_, v)) = m.domains.iter_mut().find(|(n, _)| *n == name) {
                        *v = value
                    } else {
                        m.domains.push((name, value))
                    }
                }
            }
            _ => return Err(r.fail("a domain, a transition, a title, or a line break")),
        }
    }
    m.title = m
        .title
        .filter(|s| !s.is_empty())
        .or_else(|| c.title.map(str::to_owned));
    m.acc_title = m
        .acc_title
        .filter(|s| !s.is_empty())
        .map(|s| s.trim_start_matches(space).into());
    m.acc_descr = m.acc_descr.filter(|s| !s.is_empty()).map(|s| {
        let mut out = String::new();
        let mut after = false;
        for c in s.chars() {
            if after && space(c) {
                continue;
            }
            after = c == '\n';
            out.push(c)
        }
        out
    });
    Ok(m)
}
