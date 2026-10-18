//! Ordered Langium grammar from Pele info/parser.ts, 458322a5 (MIT, kepano).
use super::InfoModel;
use crate::compat::{
    diagrams::common::langium::{self as l, Reader, TokenSpec, END},
    DiagramType, PeleError,
};
const SHOW_INFO: usize = 0;
const INFO: usize = 1;
const ACC_DESCR: usize = 2;
const ACC_TITLE: usize = 3;
const TITLE: usize = 4;
const NEWLINE: usize = 5;
fn show_info(src: &str, at: usize) -> Option<usize> {
    l::keyword(src, at, "showInfo")
}
fn info(src: &str, at: usize) -> Option<usize> {
    l::keyword(src, at, "info")
}
static TOKENS: &[TokenSpec] = &[
    TokenSpec::matcher("showInfo", show_info, false, Some("s"), None),
    TokenSpec::matcher("info", info, false, Some("i"), None),
    l::ACC_DESCR,
    l::ACC_TITLE,
    l::TITLE,
    l::NEWLINE,
    l::WHITESPACE,
    l::YAML,
    l::DIRECTIVE,
    l::SINGLE_LINE_COMMENT,
];
pub(super) fn parse(src: &str, title: Option<&str>) -> Result<InfoModel, PeleError> {
    let mut reader = Reader::new(src, TOKENS, "info")?;
    let mut model = InfoModel {
        diagram_type: DiagramType::Info,
        title: title.map(str::to_owned),
        acc_title: None,
        acc_descr: None,
    };
    while reader.accept(NEWLINE) {}
    reader.expect(INFO)?;
    while reader.accept(NEWLINE) {}
    if reader.accept(SHOW_INFO) {
        while reader.accept(NEWLINE) {}
    }
    while reader.kind() != END {
        match reader.kind() {
            ACC_DESCR => {
                let value = l::acc_descr_value(reader.take());
                model.acc_descr = (!value.is_empty()).then_some(value);
            }
            ACC_TITLE => {
                let value = l::acc_title_value(reader.take());
                model.acc_title = (!value.is_empty()).then_some(value);
            }
            TITLE => {
                let value = l::title_value(reader.take());
                model.title = if value.is_empty() {
                    title.map(str::to_owned)
                } else {
                    Some(value)
                };
            }
            _ => return Err(reader.fail("a title or the end of input")),
        }
        if reader.kind() == END {
            break;
        }
        if reader.kind() != NEWLINE {
            return Err(reader.fail("a line break or the end of input"));
        }
        while reader.accept(NEWLINE) {}
    }
    Ok(model)
}
