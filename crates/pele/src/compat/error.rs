//! Pele src/errors.ts at 458322a5. Locations count UTF-16 code units.
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ErrorCode {
    UnsupportedDiagram,
    Syntax,
    Semantic,
    Limit,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PeleError {
    pub name: String,
    pub message: String,
    pub code: ErrorCode,
    #[serde(rename = "type")]
    pub diagram_type: Option<String>,
    pub line: usize,
    pub column: usize,
    pub snippet: String,
}
impl PeleError {
    pub fn new(message: impl Into<String>, code: ErrorCode) -> Self {
        Self {
            name: "PeleError".into(),
            message: message.into(),
            code,
            diagram_type: None,
            line: 0,
            column: 0,
            snippet: String::new(),
        }
    }
    /// `offset` is a UTF-8 boundary; compatibility columns are UTF-16 units.
    pub fn syntax(kind: &str, source: &str, offset: usize, detail: &str, lexical: bool) -> Self {
        let offset = offset.min(source.len());
        let start = source[..offset].rfind('\n').map_or(0, |i| i + 1);
        let end = source[offset..]
            .find('\n')
            .map_or(source.len(), |i| offset + i);
        let line = source[..offset].bytes().filter(|c| *c == b'\n').count() + 1;
        let column = source[start..offset].encode_utf16().count() + 1;
        let snippet = format!("{}\n{}^", &source[start..end], "-".repeat(column - 1));
        let message = if lexical {
            format!("Lexical error on line {line}. Unrecognized text.\n{snippet}")
        } else {
            format!("Parse error on line {line}:\n{snippet}\n{detail}")
        };
        Self {
            name: "PeleError".into(),
            message,
            code: ErrorCode::Syntax,
            diagram_type: Some(kind.into()),
            line,
            column,
            snippet,
        }
    }
}
impl std::fmt::Display for PeleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for PeleError {}
