//! Pele git/parser.ts grammar using the reusable ordered Langium token reader (458322a5, MIT).
use super::model::{Ast, Statement};
use crate::compat::{
    diagrams::common::langium::{self as l, Pattern, Reader, TokenSpec, END},
    PeleError,
};
#[allow(non_camel_case_types)]
#[repr(usize)]
enum T {
    cherryPick,
    gitGraphColon,
    highlight,
    gitGraph,
    checkout,
    reverse,
    parent,
    commit,
    normal,
    branch,
    order,
    switch,
    kind,
    merge,
    msg,
    tag,
    id,
    lr,
    tb,
    bt,
    colon,
    accDescr,
    accTitle,
    title,
    int,
    string,
    newline,
    reference = 31,
}
fn reference(src: &str, p: usize) -> Option<usize> {
    let bytes = src.as_bytes();
    let word = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    if !bytes.get(p).is_some_and(|&c| word(c)) {
        return None;
    }
    let mut end = p + 1;
    while bytes
        .get(end)
        .is_some_and(|&c| word(c) || matches!(c, b'-' | b'.' | b'/'))
    {
        end += 1
    }
    while end > p + 1 && matches!(bytes[end - 1], b'.' | b'/') {
        end -= 1
    }
    Some(end)
}
fn integer(src: &str, p: usize) -> Option<usize> {
    let b = src.as_bytes();
    match b.get(p) {
        Some(b'0') => Some(p + 1),
        Some(b'1'..=b'9') => {
            let mut end = p + 1;
            while b.get(end).is_some_and(u8::is_ascii_digit) {
                end += 1
            }
            if b.get(end) == Some(&b'.') {
                end -= 1
            }
            (end > p).then_some(end)
        }
        _ => None,
    }
}
fn header(src: &str, p: usize) -> Option<usize> {
    l::keyword(src, p, "gitGraph")
}
const ALT: &[usize] = &[T::reference as usize];
const fn word(name: &'static str) -> TokenSpec {
    TokenSpec::literal(name, ALT)
}
const TOKENS: &[TokenSpec] = &[
    word("cherry-pick"),
    TokenSpec::literal("gitGraph:", &[]),
    word("HIGHLIGHT"),
    TokenSpec {
        longer: ALT,
        ..TokenSpec::matcher("gitGraph", header, false, Some("g"), None)
    },
    word("checkout"),
    word("REVERSE"),
    TokenSpec::literal("parent:", &[]),
    word("commit"),
    word("NORMAL"),
    word("branch"),
    TokenSpec::literal("order:", &[]),
    word("switch"),
    TokenSpec::literal("type:", &[]),
    word("merge"),
    TokenSpec::literal("msg:", &[]),
    TokenSpec::literal("tag:", &[]),
    TokenSpec::literal("id:", &[]),
    word("LR"),
    word("TB"),
    word("BT"),
    TokenSpec::literal(":", &[]),
    l::ACC_DESCR_BLOCK,
    l::ACC_TITLE,
    l::TITLE,
    TokenSpec::matcher("INT", integer, false, None, None),
    l::STRING,
    l::NEWLINE,
    l::WHITESPACE,
    l::YAML,
    l::DIRECTIVE,
    l::SINGLE_LINE_COMMENT,
    TokenSpec {
        name: "REFERENCE",
        pattern: Pattern::Match(reference),
        hidden: false,
        opener: None,
        first: None,
        longer: &[],
    },
];
fn end_line(r: &mut Reader<'_>) -> Result<(), PeleError> {
    if r.kind() == END {
        return Ok(());
    }
    if r.kind() != T::newline as usize {
        return Err(r.fail("a line break or the end of input"));
    }
    while r.kind() == T::newline as usize {
        r.i += 1
    }
    Ok(())
}
fn name(r: &mut Reader<'_>) -> Result<String, PeleError> {
    if r.kind() == T::reference as usize {
        return Ok(r.take().into());
    }
    if r.kind() == T::string as usize {
        return Ok(l::string_value(r.take()));
    }
    Err(r.fail("a branch name"))
}
fn string(r: &mut Reader<'_>) -> Result<String, PeleError> {
    Ok(l::string_value(r.expect(T::string as usize)?))
}
fn kind(r: &mut Reader<'_>) -> Result<u8, PeleError> {
    let code = if r.kind() == T::normal as usize {
        0
    } else if r.kind() == T::reverse as usize {
        1
    } else if r.kind() == T::highlight as usize {
        2
    } else {
        return Err(r.fail("NORMAL, REVERSE, or HIGHLIGHT"));
    };
    r.i += 1;
    Ok(code)
}
pub(super) fn parse(src: &str) -> Result<Ast, PeleError> {
    let mut r = Reader::new(src, TOKENS, "gitGraph")?;
    let mut ast = Ast::default();
    while r.kind() == T::newline as usize {
        r.i += 1
    }
    if !r.accept(T::gitGraphColon as usize) {
        r.expect(T::gitGraph as usize)?;
        let next = r.kind_at(r.i + 1);
        if matches!(r.kind(),k if k==T::lr as usize||k==T::tb as usize||k==T::bt as usize)
            && next == T::colon as usize
        {
            ast.direction = Some(r.take().into());
            r.i += 1;
        } else {
            r.accept(T::colon as usize);
        }
    }
    while r.kind() != END {
        let t = r.kind();
        if t == T::newline as usize {
            r.i += 1;
            continue;
        }
        if t == T::accDescr as usize {
            ast.acc_descr = Some(l::acc_descr_value(r.take()));
            end_line(&mut r)?;
            continue;
        }
        if t == T::accTitle as usize {
            ast.acc_title = Some(l::acc_title_value(r.take()));
            end_line(&mut r)?;
            continue;
        }
        if t == T::title as usize {
            ast.title = Some(l::title_value(r.take()));
            end_line(&mut r)?;
            continue;
        }
        let statement = if t == T::commit as usize {
            r.i += 1;
            let (mut id, mut message, mut ty) = (None, None, None);
            let mut tags = Vec::new();
            loop {
                if r.accept(T::id as usize) {
                    id = Some(string(&mut r)?)
                } else if r.accept(T::tag as usize) {
                    tags.push(string(&mut r)?)
                } else if r.accept(T::kind as usize) {
                    ty = Some(kind(&mut r)?)
                } else if r.accept(T::msg as usize) || r.kind() == T::string as usize {
                    message = Some(string(&mut r)?)
                } else {
                    break;
                }
            }
            Statement::Commit {
                id,
                message,
                tags,
                kind: ty,
            }
        } else if t == T::branch as usize {
            r.i += 1;
            let name = name(&mut r)?;
            let order = if r.accept(T::order as usize) {
                Some(
                    r.expect(T::int as usize)?
                        .parse::<f64>()
                        .unwrap_or(f64::NAN),
                )
            } else {
                None
            };
            Statement::Branch { name, order }
        } else if t == T::merge as usize {
            r.i += 1;
            let branch = name(&mut r)?;
            let (mut id, mut ty) = (None, None);
            let mut tags = Vec::new();
            loop {
                if r.accept(T::id as usize) {
                    id = Some(string(&mut r)?)
                } else if r.accept(T::tag as usize) {
                    tags.push(string(&mut r)?)
                } else if r.accept(T::kind as usize) {
                    ty = Some(kind(&mut r)?)
                } else {
                    break;
                }
            }
            Statement::Merge {
                branch,
                id,
                tags,
                kind: ty,
            }
        } else if t == T::checkout as usize || t == T::switch as usize {
            r.i += 1;
            Statement::Checkout(name(&mut r)?)
        } else if t == T::cherryPick as usize {
            r.i += 1;
            let (mut id, mut parent) = (None, None);
            let mut tags = Vec::new();
            loop {
                if r.accept(T::id as usize) {
                    id = Some(string(&mut r)?)
                } else if r.accept(T::tag as usize) {
                    tags.push(string(&mut r)?)
                } else if r.accept(T::parent as usize) {
                    parent = Some(string(&mut r)?)
                } else {
                    break;
                }
            }
            Statement::CherryPick { id, tags, parent }
        } else {
            return Err(r.fail("a statement, a title, or a line break"));
        };
        end_line(&mut r)?;
        ast.statements.push(statement)
    }
    Ok(ast)
}
