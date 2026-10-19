//! Iterative four-notation grammar parsers, translated from pinned Pele 458322a5 (MIT).
use super::super::common::langium::{self as l, Reader, END};
use super::{
    lexer,
    model::{RailroadModel as Model, RailroadNode as N, RailroadRule},
};
use crate::compat::{source::space, ParseContext, PeleError};
fn unescape(s: &str) -> String {
    if s.len() < 2 {
        return String::new();
    }
    let mut out = String::new();
    let mut chars = s[1..s.len() - 1].chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(c) = chars.next() {
                out.push(match c {
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    _ => c,
                })
            }
        } else {
            out.push(c)
        }
    }
    out
}
fn rep(m: &mut Model, element: usize, min: f64, max: f64) -> usize {
    m.add(N::Repetition { element, min, max })
}
fn opt(m: &mut Model, n: usize) -> usize {
    m.add(N::Optional(n))
}
pub(super) fn parse(c: ParseContext<'_>) -> Result<Model, PeleError> {
    let trimmed = c.source.trim_start_matches(space).to_ascii_lowercase();
    let (notation, tokens, meta, name, eq, semi) = if trimmed.starts_with("railroad-ebnf-beta") {
        ("ebnf", lexer::EBNF, 16, 19, 2, 3)
    } else if trimmed.starts_with("railroad-abnf-beta") {
        ("abnf", lexer::ABNF, 8, 11, 1, 2)
    } else if trimmed.starts_with("railroad-peg-beta") {
        ("peg", lexer::PEG, 12, 15, 1, 2)
    } else {
        ("railroad", lexer::IR, 14, 17, 9, 10)
    };
    let mut r = Reader::new(c.source, tokens, "railroad")?;
    let mut m = Model::new(notation, c.title);
    let (mut title, mut acc_title, mut acc_descr) = (None, None, None);
    r.expect(0)?;
    loop {
        let k = r.kind();
        if k == meta {
            let text = l::title_value(r.take());
            let q = text.chars().next();
            let text = if q.is_some_and(|c| matches!(c, '\'' | '"')) && text.ends_with(q.unwrap()) {
                if notation == "abnf" {
                    if text.len() < 2 {
                        String::new()
                    } else {
                        text[1..text.len() - 1].into()
                    }
                } else {
                    unescape(&text)
                }
            } else {
                text
            };
            title = Some(text);
        } else if k == meta + 1 {
            let text = l::acc_title_value(r.take());
            acc_title = Some(text.trim_start_matches(space).to_owned());
        } else if k == meta + 2 {
            let text = l::acc_descr_value(r.take());
            {
                let mut out = String::new();
                let mut after_nl = false;
                for c in text.chars() {
                    if after_nl && space(c) {
                        continue;
                    }
                    after_nl = c == '\n';
                    out.push(c)
                }
                acc_descr = Some(out)
            }
        } else {
            break;
        }
    }
    if let Some(t) = title.filter(|s| !s.is_empty()) {
        m.title = Some(t)
    }
    m.acc_title = acc_title.filter(|s| !s.is_empty());
    m.acc_descr = acc_descr.filter(|s| !s.is_empty());
    while r.kind() != END {
        let name = r.expect(name)?.to_owned();
        if notation == "ebnf" && !r.accept(eq) {
            r.expect(1)?;
        } else if notation != "ebnf" {
            r.expect(eq)?;
        }
        let definition = match notation {
            "ebnf" => ebnf(&mut r, &mut m)?,
            "abnf" => abnf(&mut r, &mut m)?,
            "peg" => peg(&mut r, &mut m)?,
            _ => ir(&mut r, &mut m)?,
        };
        r.expect(semi)?;
        m.rules.push(RailroadRule { name, definition });
    }
    Ok(m)
}
fn ir(r: &mut Reader<'_>, m: &mut Model) -> Result<usize, PeleError> {
    let mut open: Vec<(usize, Vec<usize>)> = vec![];
    loop {
        let k = r.kind();
        let mut v = match k {
            1 | 6 | 7 => {
                r.i += 1;
                r.expect(11)?;
                let text = unescape(r.expect(18)?);
                r.expect(13)?;
                m.add(match k {
                    1 => N::Nonterminal(text),
                    6 => N::Terminal(text),
                    _ => N::Special(text),
                })
            }
            2..=5 | 8 => {
                r.i += 1;
                r.expect(11)?;
                open.push((k, vec![]));
                continue;
            }
            _ => return Err(r.fail("an expression")),
        };
        loop {
            let Some((kind, kids)) = open.last_mut() else {
                return Ok(v);
            };
            kids.push(v);
            if matches!(*kind, 4 | 8) && r.accept(12) {
                break;
            }
            r.expect(13)?;
            let (k, kids) = open.pop().unwrap();
            v = match k {
                4 => m.sequence(kids),
                8 => m.choice(kids),
                5 => opt(m, kids[0]),
                2 => rep(m, kids[0], 0., f64::INFINITY),
                _ => rep(m, kids[0], 1., f64::INFINITY),
            };
        }
    }
}
#[derive(Default)]
struct Frame {
    alternatives: Vec<usize>,
    elements: Vec<usize>,
    kind: usize,
    except: bool,
    repeat: Option<String>,
    predicate: Option<String>,
}
impl Frame {
    fn close_sequence(&mut self, m: &mut Model) {
        self.alternatives
            .push(m.sequence(std::mem::take(&mut self.elements)))
    }
    fn finish(mut self, m: &mut Model) -> usize {
        self.close_sequence(m);
        m.choice(self.alternatives)
    }
}
fn ebnf(r: &mut Reader<'_>, m: &mut Model) -> Result<usize, PeleError> {
    let mut stack = vec![];
    let mut top = Frame::default();
    let mut except = false;
    loop {
        let k = r.kind();
        let mut v = match k {
            20 => m.add(N::Terminal(unescape(r.take()))),
            19 => m.add(N::Nonterminal(r.take().into())),
            21 => {
                let s = r.take();
                m.add(N::Special(s[1..s.len() - 1].trim_matches(space).into()))
            }
            6 | 8 | 10 => {
                r.i += 1;
                stack.push(top);
                top = Frame {
                    kind: k,
                    except,
                    ..Frame::default()
                };
                except = false;
                continue;
            }
            _ => return Err(r.fail("a terminal, a name, or an opening bracket")),
        };
        loop {
            if except {
                let previous = top.elements.pop().unwrap();
                let dash = m.add(N::Terminal("-".into()));
                v = m.add(N::Sequence(vec![previous, dash, v]));
                except = false
            }
            top.elements.push(v);
            while matches!(r.kind(), 12..=14) {
                v = match r.kind() {
                    12 => opt(m, v),
                    13 => rep(m, v, 0., f64::INFINITY),
                    _ => rep(m, v, 1., f64::INFINITY),
                };
                r.i += 1;
                *top.elements.last_mut().unwrap() = v;
            }
            let k = r.kind();
            if top.kind > 0 && k == top.kind + 1 {
                r.i += 1;
                let kind = top.kind;
                except = top.except;
                let node = top.finish(m);
                v = match kind {
                    8 => opt(m, node),
                    10 => rep(m, node, 0., f64::INFINITY),
                    _ => node,
                };
                top = stack.pop().unwrap();
                continue;
            }
            if k == 15 {
                r.i += 1;
                except = true
            } else if k == 5 {
                r.i += 1;
            } else if k == 4 {
                r.i += 1;
                top.close_sequence(m)
            } else if !matches!(k, 20 | 19 | 21 | 6 | 8 | 10) {
                if stack.is_empty() {
                    return Ok(top.finish(m));
                }
                return Err(r.fail(&format!("token of type '{}'", r.types[top.kind + 1].name)));
            }
            break;
        }
    }
}
fn number(s: &str) -> f64 {
    s.parse().unwrap_or(f64::INFINITY)
}
fn repeated(m: &mut Model, v: usize, text: Option<String>) -> usize {
    let Some(s) = text else { return v };
    let (min, max) = if let Some(star) = s.find('*') {
        (
            if star > 0 { number(&s[..star]) } else { 0. },
            if star < s.len() - 1 {
                number(&s[star + 1..])
            } else {
                f64::INFINITY
            },
        )
    } else {
        let n = number(&s);
        (n, n)
    };
    if min == 0. && max == 1. {
        opt(m, v)
    } else {
        rep(m, v, min, max)
    }
}
fn abnf(r: &mut Reader<'_>, m: &mut Model) -> Result<usize, PeleError> {
    let mut stack = vec![];
    let mut top = Frame::default();
    loop {
        let mut repeat = if matches!(r.kind(), 14 | 15) {
            Some(r.take().into())
        } else {
            None
        };
        let k = r.kind();
        let mut v = match k {
            12 => {
                let s = r.take();
                m.add(N::Terminal(s[1..s.len() - 1].into()))
            }
            13 => m.add(N::Terminal(r.take().into())),
            11 => m.add(N::Nonterminal(r.take().into())),
            4 | 6 => {
                r.i += 1;
                stack.push(top);
                top = Frame {
                    kind: k,
                    repeat,
                    ..Frame::default()
                };
                continue;
            }
            _ => return Err(r.fail("a string, a number, a name, or an opening bracket")),
        };
        loop {
            v = repeated(m, v, repeat);
            top.elements.push(v);
            let k = r.kind();
            if top.kind > 0 && k == top.kind + 1 {
                r.i += 1;
                let kind = top.kind;
                repeat = top.repeat.take();
                let node = top.finish(m);
                v = if kind == 6 { opt(m, node) } else { node };
                top = stack.pop().unwrap();
                continue;
            }
            if k == 3 {
                r.i += 1;
                top.close_sequence(m)
            } else if !matches!(k, 12 | 13 | 11 | 4 | 6 | 14 | 15) {
                if stack.is_empty() {
                    return Ok(top.finish(m));
                }
                return Err(r.fail(&format!("token of type '{}'", r.types[top.kind + 1].name)));
            }
            break;
        }
    }
}
fn predicate_label(m: &Model, n: usize) -> String {
    match &m.nodes[n] {
        N::Terminal(s) => format!("\"{s}\""),
        N::Nonterminal(s) | N::Special(s) => s.clone(),
        _ => "(...)".into(),
    }
}
fn peg(r: &mut Reader<'_>, m: &mut Model) -> Result<usize, PeleError> {
    let mut stack = vec![];
    let mut top = Frame::default();
    loop {
        let mut predicate = if matches!(r.kind(), 4 | 5) {
            Some(r.take().to_owned())
        } else {
            None
        };
        let mut v = match r.kind() {
            16 => m.add(N::Terminal(unescape(r.take()))),
            15 => m.add(N::Nonterminal(r.take().into())),
            11 => m.add(N::Special(r.take().into())),
            9 => {
                r.i += 1;
                stack.push(top);
                top = Frame {
                    predicate,
                    ..Frame::default()
                };
                continue;
            }
            _ => return Err(r.fail("a literal, a name, a dot, or an opening parenthesis")),
        };
        loop {
            if matches!(r.kind(), 6..=8) {
                v = match r.kind() {
                    6 => opt(m, v),
                    7 => rep(m, v, 0., f64::INFINITY),
                    _ => rep(m, v, 1., f64::INFINITY),
                };
                r.i += 1;
            }
            if let Some(op) = predicate {
                let text = op + &predicate_label(m, v);
                v = m.add(N::Special(text))
            }
            top.elements.push(v);
            let k = r.kind();
            if k == 10 && !stack.is_empty() {
                r.i += 1;
                predicate = top.predicate.take();
                v = top.finish(m);
                top = stack.pop().unwrap();
                continue;
            }
            if k == 3 {
                r.i += 1;
                top.close_sequence(m)
            } else if !matches!(k, 4 | 5 | 16 | 15 | 11 | 9) {
                if stack.is_empty() {
                    return Ok(top.finish(m));
                }
                return Err(r.fail("token of type ')'"));
            }
            break;
        }
    }
}
