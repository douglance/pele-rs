use serde::Serialize;

/// UTF-8 byte range; line and Unicode-scalar column are one-based.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Error {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<Span>,
}

impl Error {
    pub(crate) fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            span: None,
        }
    }
    pub(crate) fn at(mut self, source: &str, start: usize, end: usize) -> Self {
        let prefix = &source[..start];
        self.span = Some(Span {
            start,
            end,
            line: prefix.bytes().filter(|b| *b == b'\n').count() + 1,
            column: prefix.rsplit('\n').next().unwrap_or("").chars().count() + 1,
        });
        self
    }
    pub fn status(&self) -> u32 {
        match self.code.as_str() {
            "input_limit" | "graph_limit" | "layout_limit" | "output_limit" => 2,
            "internal" => 3,
            _ => 1,
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for Error {}
