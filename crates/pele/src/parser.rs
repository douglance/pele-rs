use crate::{model::invalid_xml, Direction, Edge, Error, Flowchart, LineStyle, Node, Shape, Span};
use std::collections::BTreeMap;

pub const MAX_SOURCE: usize = 262_144;
pub const MAX_NODES: usize = 256;
pub const MAX_EDGES: usize = 1024;

pub fn parse(source: &str) -> Result<Flowchart, Error> {
    if source.len() > MAX_SOURCE {
        return Err(Error::new(
            "input_limit",
            "Source exceeds 262144 UTF-8 bytes",
        ));
    }
    let mut p = Parser {
        source,
        pos: 0,
        graph: Flowchart {
            direction: Direction::TB,
            nodes: vec![],
            edges: vec![],
        },
        ids: BTreeMap::new(),
    };
    p.separators()?;
    if p.rest().starts_with("---") {
        return Err(p.error("unsupported_syntax", "Frontmatter is not supported"));
    }
    let start = p.pos;
    let kind = p.identifier()?;
    if kind != "flowchart" && kind != "graph" {
        return Err(p.error_at("unsupported_diagram", "Expected flowchart or graph", start));
    }
    p.space();
    let start = p.pos;
    let dir = p.identifier()?;
    p.graph.direction = match dir.as_str() {
        "TB" | "TD" => Direction::TB,
        "BT" => Direction::BT,
        "LR" => Direction::LR,
        "RL" => Direction::RL,
        _ => return Err(p.error_at("syntax", "Expected TB, TD, BT, LR or RL", start)),
    };
    p.end_statement()?;
    p.separators()?;
    while p.pos < source.len() {
        let mut from = p.node()?;
        loop {
            p.space();
            if p.ends_statement() {
                break;
            }
            let start = p.pos;
            let (style, arrow) = if p.take("-.->") {
                (LineStyle::Dashed, true)
            } else if p.take("-.-") {
                (LineStyle::Dashed, false)
            } else if p.take("-->") {
                (LineStyle::Solid, true)
            } else if p.take("---") {
                (LineStyle::Solid, false)
            } else if p.take("==>") {
                (LineStyle::Thick, true)
            } else {
                return Err(p.error("unsupported_syntax", "Unsupported link or statement suffix"));
            };
            p.space();
            let label = if p.take("|") {
                p.label("|")?
            } else {
                String::new()
            };
            p.space();
            let to = p.node()?;
            if p.graph.edges.len() >= MAX_EDGES {
                return Err(p.error_at("graph_limit", "At most 1024 edges are supported", start));
            }
            p.graph.edges.push(Edge {
                source: from,
                target: to,
                label,
                style,
                arrow,
                span: p.span(start),
            });
            from = to;
        }
        p.end_statement()?;
        p.separators()?;
    }
    if p.graph.nodes.is_empty() {
        return Err(Error::new("empty_graph", "Flowchart contains no nodes"));
    }
    Ok(p.graph)
}

struct Parser<'a> {
    source: &'a str,
    pos: usize,
    graph: Flowchart,
    ids: BTreeMap<String, usize>,
}
impl Parser<'_> {
    fn rest(&self) -> &str {
        &self.source[self.pos..]
    }
    fn take(&mut self, s: &str) -> bool {
        if self.rest().starts_with(s) {
            self.pos += s.len();
            true
        } else {
            false
        }
    }
    fn space(&mut self) {
        while matches!(self.rest().chars().next(), Some(' ' | '\t' | '\r')) {
            self.pos += 1;
        }
    }
    fn error(&self, code: &str, msg: &str) -> Error {
        self.error_at(code, msg, self.pos)
    }
    fn error_at(&self, code: &str, msg: &str, start: usize) -> Error {
        let end = start
            + self.source[start..]
                .chars()
                .next()
                .map_or(0, char::len_utf8);
        Error::new(code, msg).at(self.source, start, end)
    }
    fn span(&self, start: usize) -> Span {
        Error::new("", "")
            .at(self.source, start, self.pos)
            .span
            .unwrap()
    }
    fn comment(&mut self) -> Result<bool, Error> {
        if self.rest().starts_with("%%{") {
            return Err(self.error(
                "unsupported_syntax",
                "Configuration directives are not supported",
            ));
        }
        if self.take("%%") {
            while self.pos < self.source.len() && !self.rest().starts_with('\n') {
                self.pos += self.rest().chars().next().unwrap().len_utf8();
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }
    fn separators(&mut self) -> Result<(), Error> {
        loop {
            self.space();
            if self.take(";") || self.take("\n") || self.comment()? {
                continue;
            }
            break;
        }
        Ok(())
    }
    fn ends_statement(&self) -> bool {
        self.rest().is_empty()
            || self.rest().starts_with([';', '\n'])
            || self.rest().starts_with("%%")
    }
    fn end_statement(&mut self) -> Result<(), Error> {
        self.space();
        if !self.ends_statement() {
            return Err(self.error(
                "unsupported_syntax",
                "Expected newline or semicolon; unsupported trailing syntax",
            ));
        }
        Ok(())
    }
    fn identifier(&mut self) -> Result<String, Error> {
        let start = self.pos;
        match self.rest().bytes().next() {
            Some(b) if b.is_ascii_alphabetic() || b == b'_' => self.pos += 1,
            _ => return Err(self.error("syntax", "Expected ASCII node identifier")),
        }
        while let Some(b) = self.rest().bytes().next() {
            if b.is_ascii_alphanumeric() || b == b'_' {
                self.pos += 1;
            } else {
                break;
            }
        }
        if self.pos - start > 128 {
            return Err(self.error_at("graph_limit", "Identifier exceeds 128 bytes", start));
        }
        Ok(self.source[start..self.pos].into())
    }
    fn node(&mut self) -> Result<usize, Error> {
        self.space();
        let start = self.pos;
        let id = self.identifier()?;
        if [
            "subgraph",
            "end",
            "direction",
            "style",
            "class",
            "classDef",
            "linkStyle",
            "click",
        ]
        .contains(&id.as_str())
        {
            return Err(self.error_at(
                "unsupported_syntax",
                "This statement is not supported in v1",
                start,
            ));
        }
        self.space();
        let explicit = if self.take("((") {
            Some((Shape::Circle, self.label("))")?))
        } else if self.take("([") {
            Some((Shape::Stadium, self.label("])")?))
        } else if self.take("[") {
            Some((Shape::Rectangle, self.label("]")?))
        } else if self.take("(") {
            Some((Shape::Rounded, self.label(")")?))
        } else if self.take("{") {
            Some((Shape::Diamond, self.label("}")?))
        } else {
            None
        };
        if let Some(&i) = self.ids.get(&id) {
            if let Some((shape, label)) = explicit {
                self.graph.nodes[i].shape = shape;
                self.graph.nodes[i].label = label;
                self.graph.nodes[i].span = self.span(start);
            }
            return Ok(i);
        }
        if self.graph.nodes.len() >= MAX_NODES {
            return Err(self.error_at("graph_limit", "At most 256 nodes are supported", start));
        }
        let (shape, label) = explicit.unwrap_or((Shape::Rectangle, id.clone()));
        let index = self.graph.nodes.len();
        self.ids.insert(id.clone(), index);
        self.graph.nodes.push(Node {
            id,
            label,
            shape,
            span: self.span(start),
        });
        Ok(index)
    }
    fn label(&mut self, close: &str) -> Result<String, Error> {
        let start = self.pos;
        let quoted = self.take("\"");
        let mut text = String::new();
        loop {
            if quoted && self.take("\"") {
                self.space();
                if !self.take(close) {
                    return Err(self.error("syntax", "Expected label closing delimiter"));
                }
                break;
            }
            if !quoted && self.take(close) {
                break;
            }
            let Some(c) = self.rest().chars().next() else {
                return Err(self.error_at("syntax", "Unterminated label", start));
            };
            if c == '`' {
                return Err(self.error("unsupported_syntax", "Markdown labels are not supported"));
            }
            if invalid_xml(c) {
                return Err(self.error("syntax", "Label contains an invalid XML character"));
            }
            if !quoted && matches!(c, '[' | ']' | '(' | ')' | '{' | '}' | '\n' | '\r' | '"') {
                return Err(self.error(
                    "unsupported_syntax",
                    "Quote labels containing delimiters or newlines",
                ));
            }
            self.pos += c.len_utf8();
            if quoted && c == '\\' {
                let Some(escaped) = self.rest().chars().next() else {
                    return Err(self.error("syntax", "Unterminated escape"));
                };
                self.pos += escaped.len_utf8();
                text.push(match escaped {
                    'n' => '\n',
                    '"' => '"',
                    '\\' => '\\',
                    _ => {
                        return Err(
                            self.error("syntax", "Supported escapes: quote, backslash and n")
                        )
                    }
                });
            } else if quoted && c == '\r' {
                self.take("\n");
                text.push('\n');
            } else {
                text.push(c);
            }
            if text.len() > 4096 || text.bytes().filter(|b| *b == b'\n').count() >= 64 {
                return Err(self.error_at(
                    "graph_limit",
                    "Label exceeds 4096 bytes or 64 lines",
                    start,
                ));
            }
        }
        Ok(if quoted { text } else { text.trim().to_owned() })
    }
}
