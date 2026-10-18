//! Pele usecase/parser.ts, 458322a5 (MIT, kepano). Three-token lookahead,
//! source occurrence spans and deferred JSON/declaration resolution.
use super::{
    lexer::{tokenize, Source, Token},
    model::*,
    tokens::{NAMES, T},
};
use crate::compat::{source::space, ErrorCode, ParseContext, PeleError};
const LINE: u8 = 1;
const RELATION: u8 = 2;
const STYLE_PART: u8 = 4;
const FOLLOWS_NAME: u8 = 8;
const BOUNDARY_LINE: u8 = 16;
const CLOSES_LABEL: u8 = 32;
const METADATA_FIRST: u8 = 64;
const ARROWS:&str="DEPENDENCY_ARROW|GENERALIZATION|FORWARD_SOLID|BACKWARD_SOLID|MARKERLESS_SOLID|FORWARD_CIRCLE|BACKWARD_CIRCLE|FORWARD_CROSS|BACKWARD_CROSS";
const NAMES_TEXT: &str = "IDENTIFIER|PLAIN_STRING|MARKDOWN_STRING";
const LABELS: &str = "PLAIN_STRING|MARKDOWN_STRING|LABEL_TEXT";
const STATEMENTS:&str="ACC_TITLE_LINE|ACC_DESCR_LINE|ACC_DESCR_BLOCK|DIRECTION|ACTOR|SYSTEM_BOUNDARY|NOTE|JSON_DECLARATION_START|CLASS_DEF|CLASS|STYLE";
const STYLE_PARTS:&str="WORD|PLAIN_STRING|NUMBER|HASH_COLOR|CSS_IDENTIFIER|CSS_ESCAPED_COMMA|DASH|DOT|PERCENT|CSS_PUNCTUATION|COLON|LPAREN|RPAREN|LBRACKET|RBRACKET|LBRACE|RBRACE|AT|MARKERLESS_SOLID";
fn word(t: T) -> bool {
    (T::USECASE..=T::IDENTIFIER).contains(&t)
}
fn label_text(t: T) -> bool {
    (T::USECASE..=T::LABEL_SYMBOL).contains(&t)
}
fn key(t: T) -> bool {
    t == T::IDENTIFIER || t == T::PLAIN_STRING
}
fn flags(t: T) -> u8 {
    use T::*;
    let mut b = 0;
    if matches!(
        t,
        NEWLINE
            | COMMENT
            | ACC_TITLE_LINE
            | ACC_DESCR_LINE
            | ACC_DESCR_BLOCK
            | DIRECTION
            | ACTOR
            | SYSTEM_BOUNDARY
            | NOTE
            | JSON_DECLARATION_START
            | CLASS_DEF
            | CLASS
            | STYLE
            | IDENTIFIER
            | PLAIN_STRING
            | MARKDOWN_STRING
    ) {
        b |= LINE
    }
    if matches!(
        t,
        IDENTIFIER
            | DEPENDENCY_ARROW
            | GENERALIZATION
            | FORWARD_SOLID
            | BACKWARD_SOLID
            | MARKERLESS_SOLID
            | FORWARD_CIRCLE
            | BACKWARD_CIRCLE
            | FORWARD_CROSS
            | BACKWARD_CROSS
    ) {
        b |= RELATION
    }
    if word(t)
        || matches!(
            t,
            PLAIN_STRING
                | NUMBER
                | HASH_COLOR
                | CSS_IDENTIFIER
                | CSS_ESCAPED_COMMA
                | DASH
                | DOT
                | PERCENT
                | CSS_PUNCTUATION
                | COLON
                | LPAREN
                | RPAREN
                | LBRACKET
                | RBRACKET
                | LBRACE
                | RBRACE
                | AT
                | MARKERLESS_SOLID
        )
    {
        b |= STYLE_PART
    }
    if flags_relation(t) || matches!(t, STEREOTYPE_START | CLASS_SEPARATOR | NEWLINE | EOF) {
        b |= FOLLOWS_NAME
    }
    if matches!(
        t,
        NEWLINE | COMMENT | ACTOR | IDENTIFIER | PLAIN_STRING | MARKDOWN_STRING
    ) {
        b |= BOUNDARY_LINE
    }
    if matches!(
        t,
        FORWARD_SOLID | MARKERLESS_SOLID | FORWARD_CIRCLE | FORWARD_CROSS
    ) {
        b |= CLOSES_LABEL
    }
    if matches!(t, NEWLINE | IDENTIFIER | PLAIN_STRING | RBRACE) {
        b |= METADATA_FIRST
    }
    b
}
fn flags_relation(t: T) -> bool {
    matches!(
        t,
        T::IDENTIFIER
            | T::DEPENDENCY_ARROW
            | T::GENERALIZATION
            | T::FORWARD_SOLID
            | T::BACKWARD_SOLID
            | T::MARKERLESS_SOLID
            | T::FORWARD_CIRCLE
            | T::BACKWARD_CIRCLE
            | T::FORWARD_CROSS
            | T::BACKWARD_CROSS
    )
}
fn sequences() -> String {
    let mut out: Vec<String> = STATEMENTS.split('|').map(str::to_string).collect();
    let names: Vec<_> = NAMES_TEXT.split('|').collect();
    let metadata: Vec<String> = names
        .iter()
        .flat_map(|name| {
            ["NEWLINE", "IDENTIFIER", "PLAIN_STRING", "RBRACE"]
                .map(|next| format!("{name}, METADATA_START, {next}"))
        })
        .collect();
    out.extend(metadata.clone());
    let follows = format!("STEREOTYPE_START|CLASS_SEPARATOR|IDENTIFIER|{ARROWS}|NEWLINE|EOF");
    for (i, name) in names.iter().enumerate() {
        if i == 0 {
            out.push(format!("{name}, LPAREN"));
            out.push(format!("{name}, LBRACKET"));
        }
        out.extend(follows.split('|').map(|next| format!("{name}, {next}")))
    }
    out.extend(metadata);
    out.join("|")
}
#[derive(Clone, Default)]
struct Item {
    named: Named,
    shape: Option<String>,
    metadata: Option<Vec<DraftProperty>>,
    stereotype: Option<String>,
    stereotype_span: Option<Span>,
    classes: Vec<String>,
    class_spans: Vec<Span>,
    explicit: bool,
    node_span: Span,
}
struct Tail {
    id: Option<String>,
    id_span: Option<Span>,
    kind: String,
    arrow: usize,
    label: Option<DraftLabel>,
    minlen: usize,
    target: Item,
    span: Span,
}
struct Reader {
    source: Source,
    ts: Vec<Token>,
    p: usize,
    last_end: usize,
    drafts: Drafts,
    parent: Option<DraftBoundary>,
    pending_json: Vec<(usize, usize)>,
    edge_count: usize,
    note_count: usize,
    statement_index: usize,
}
impl Reader {
    fn t(&self) -> T {
        self.ts[self.p].kind
    }
    fn next(&self, k: usize) -> T {
        self.ts[self.p + k].kind
    }
    fn image(&self, k: usize) -> String {
        let t = self.ts[k];
        self.source.text(t.start, t.end)
    }
    fn span(&self, k: usize) -> Span {
        let t = self.ts[k];
        [t.start, t.end]
    }
    fn fail(&self, detail: String) -> PeleError {
        let t = self.ts[self.p];
        let (line, column) = self.source.loc(t.start);
        let mut e = PeleError::new(
            format!(
                "Error parsing usecase diagram: {detail} at line {line}, column {column} [{},{})",
                t.start, t.end
            ),
            ErrorCode::Syntax,
        );
        e.diagram_type = Some("usecase".into());
        e.line = line;
        e.column = column;
        e
    }
    fn mismatch(&self, name: &str) -> PeleError {
        self.fail(format!(
            "Expecting token of type --> {name} <-- but found --> '{}' <--",
            self.image(self.p)
        ))
    }
    fn expect(&mut self, t: T) -> Result<usize, PeleError> {
        if self.t() != t {
            return Err(self.mismatch(NAMES[t as usize]));
        }
        let k = self.p;
        self.p += 1;
        Ok(k)
    }
    fn alternative(&self, names: &str) -> PeleError {
        let list = names
            .split('|')
            .enumerate()
            .map(|(i, s)| format!("  {}. [{s}]", i + 1))
            .collect::<Vec<_>>()
            .join("\n");
        self.fail(format!(
            "Expecting: one of these possible Token sequences:\n{list}\nbut found: '{}'",
            self.image(self.p)
        ))
    }
    fn line_end(&mut self) -> Result<(), PeleError> {
        self.last_end = self.ts[self.p - 1].end;
        if self.t() == T::NEWLINE {
            self.p += 1
        } else if self.t() != T::EOF {
            return Err(self.alternative("NEWLINE|EOF"));
        }
        Ok(())
    }
    fn content(&self, k: usize) -> Span {
        let trim = match self.ts[k].kind {
            T::MARKDOWN_STRING => 2,
            T::PLAIN_STRING => 1,
            _ => 0,
        };
        [self.ts[k].start + trim, self.ts[k].end - trim]
    }
    fn token_label(&self, k: usize) -> DraftLabel {
        let span = self.content(k);
        DraftLabel {
            text: self.source.text(span[0], span[1]),
            kind: if self.ts[k].kind == T::MARKDOWN_STRING {
                "markdown"
            } else {
                "text"
            }
            .into(),
            span,
        }
    }
    fn label(&mut self) -> Result<DraftLabel, PeleError> {
        let t = self.t();
        if matches!(t, T::PLAIN_STRING | T::MARKDOWN_STRING) {
            let k = self.p;
            self.p += 1;
            return Ok(self.token_label(k));
        }
        if !label_text(t) {
            return Err(self.alternative(LABELS));
        }
        let first = self.p;
        while label_text(self.t()) {
            self.p += 1
        }
        let span = [self.ts[first].start, self.ts[self.p - 1].end];
        Ok(DraftLabel {
            text: self.source.text(span[0], span[1]),
            kind: "text".into(),
            span,
        })
    }
    fn property(&mut self) -> Result<DraftProperty, PeleError> {
        let k = self.p;
        if !key(self.t()) {
            return Err(self.alternative("IDENTIFIER|PLAIN_STRING"));
        }
        self.p += 1;
        self.expect(T::COLON)?;
        let value = self.p;
        let t = self.t();
        if !key(t) && t != T::TRUE && t != T::FALSE {
            return Err(self.alternative("IDENTIFIER|PLAIN_STRING|TRUE|FALSE"));
        }
        self.p += 1;
        let ks = self.content(k);
        let vs = self.content(value);
        Ok(DraftProperty {
            occurrence: MetadataOccurrence {
                key: self.source.text(ks[0], ks[1]),
                span: [self.ts[k].start, self.ts[value].end],
                key_span: ks,
                value_span: vs,
            },
            value: if t == T::TRUE {
                PropertyValue::Bool(true)
            } else if t == T::FALSE {
                PropertyValue::Bool(false)
            } else {
                PropertyValue::Text(self.source.text(vs[0], vs[1]))
            },
        })
    }
    fn separator(&self) -> bool {
        let (a, b, c) = (self.t(), self.next(1), self.next(2));
        if a == T::COMMA {
            return key(b) || (b == T::NEWLINE && (c == T::NEWLINE || key(c)));
        }
        a == T::NEWLINE
            && (b == T::COMMA
                || key(b)
                || (b == T::NEWLINE && (c == T::NEWLINE || c == T::COMMA || key(c))))
    }
    fn metadata(&mut self) -> Result<Vec<DraftProperty>, PeleError> {
        self.expect(T::METADATA_START)?;
        let mut out = vec![];
        while self.t() == T::NEWLINE {
            self.p += 1
        }
        if key(self.t()) {
            out.push(self.property()?);
            while self.separator() {
                if self.t() == T::COMMA {
                    self.p += 1
                } else {
                    while self.t() == T::NEWLINE {
                        self.p += 1
                    }
                    if self.t() == T::COMMA {
                        self.p += 1
                    }
                }
                while self.t() == T::NEWLINE {
                    self.p += 1
                }
                out.push(self.property()?)
            }
            if self.t() == T::COMMA {
                self.p += 1
            }
            while self.t() == T::NEWLINE {
                self.p += 1
            }
        }
        self.expect(T::RBRACE)?;
        Ok(out)
    }
    fn class_suffix(
        &mut self,
        classes: &mut Vec<String>,
        spans: &mut Vec<Span>,
    ) -> Result<(), PeleError> {
        loop {
            self.p += 1;
            let id = self.expect(T::IDENTIFIER)?;
            classes.push(self.image(id));
            spans.push(self.span(id));
            if self.t() != T::COMMA {
                break;
            }
        }
        Ok(())
    }
    fn name(&mut self, brackets: bool) -> Result<Item, PeleError> {
        let k = self.p;
        let t = self.t();
        let mut item = Item {
            named: Named {
                span: self.content(k),
                generated: t != T::IDENTIFIER,
                ..Default::default()
            },
            ..Default::default()
        };
        if t == T::IDENTIFIER {
            self.p += 1;
            item.named.id = self.image(k);
            let open = self.t();
            if open == T::LPAREN || (brackets && open == T::LBRACKET) {
                self.p += 1;
                item.named.label = self.label()?;
                self.expect(if open == T::LPAREN {
                    T::RPAREN
                } else {
                    T::RBRACKET
                })?;
                item.shape = Some(if open == T::LPAREN { "ellipse" } else { "rect" }.into())
            } else {
                item.named.label = self.token_label(k)
            }
        } else if matches!(t, T::PLAIN_STRING | T::MARKDOWN_STRING) {
            self.p += 1;
            item.named.label = self.token_label(k);
            item.named.id = generated(&item.named.label.text)
        } else {
            return Err(self.alternative(NAMES_TEXT));
        }
        Ok(item)
    }
    fn element(&mut self, actor: bool) -> Result<Item, PeleError> {
        let first = self.p;
        let mut item = self.name(!actor)?;
        if actor {
            item.shape = None
        }
        if self.t() == T::METADATA_START {
            item.metadata = Some(self.metadata()?)
        }
        if self.t() == T::STEREOTYPE_START {
            self.p += 1;
            let text = self.expect(T::STEREOTYPE_TEXT)?;
            self.expect(T::STEREOTYPE_END)?;
            item.stereotype = Some(self.image(text).trim_matches(space).into());
            item.stereotype_span = Some(self.span(text))
        }
        if self.t() == T::CLASS_SEPARATOR {
            self.class_suffix(&mut item.classes, &mut item.class_spans)?
        }
        item.explicit = item.shape.is_some()
            || item.metadata.is_some()
            || item.stereotype.as_ref().is_some_and(|s| !s.is_empty());
        item.node_span = [self.ts[first].start, self.ts[self.p - 1].end];
        Ok(item)
    }
    fn draft(&self, item: &Item, kind: &str) -> DraftElement {
        DraftElement {
            named: item.named.clone(),
            kind: kind.into(),
            parent: self.parent.clone(),
            shape: item.shape.clone(),
            metadata: item.metadata.clone(),
            stereotype: item.stereotype.clone(),
            classes: item.classes.clone(),
        }
    }
    fn labeled_right(&self, closers: u8) -> bool {
        if !(T::USECASE..=T::MARKDOWN_STRING).contains(&self.t()) {
            return false;
        }
        for k in self.p + 1..self.ts.len() {
            let t = self.ts[k].kind;
            if t == T::MARKERLESS_SOLID || flags(t) & closers != 0 {
                return true;
            }
            if !(T::USECASE..=T::MARKDOWN_STRING).contains(&t) {
                return false;
            }
        }
        false
    }
    fn minimum(&self, k: usize) -> usize {
        let dashes = self.ts[k].end
            - self.ts[k].start
            - if self.ts[k].kind == T::MARKERLESS_SOLID {
                0
            } else {
                1
            };
        dashes.saturating_sub(1).max(1)
    }
    fn tail(&mut self) -> Result<Tail, PeleError> {
        use T::*;
        let first = self.p;
        let mut tail = Tail {
            id: None,
            id_span: None,
            kind: "association".into(),
            arrow: 0,
            label: None,
            minlen: 1,
            target: Item::default(),
            span: [0, 0],
        };
        if self.t() == IDENTIFIER && self.next(1) == AT {
            tail.id = Some(self.image(self.p));
            tail.id_span = Some(self.span(self.p));
            self.p += 2
        }
        let k = self.p;
        let t = self.t();
        self.p += 1;
        match t {
            DEPENDENCY_ARROW => {
                self.expect(COLON)?;
                if !matches!(self.t(), INCLUDE | EXTEND) {
                    return Err(self.alternative("INCLUDE|EXTEND"));
                }
                tail.kind = if self.t() == INCLUDE {
                    "include"
                } else {
                    "extend"
                }
                .into();
                tail.label = Some(DraftLabel {
                    text: tail.kind.clone(),
                    kind: "text".into(),
                    span: self.span(self.p),
                });
                self.p += 1
            }
            GENERALIZATION => tail.kind = "generalization".into(),
            FORWARD_SOLID => tail.minlen = self.minimum(k),
            BACKWARD_SOLID => {
                tail.arrow = 1;
                if self.ts[k].end - self.ts[k].start == 3 && self.labeled_right(0) {
                    tail.label = Some(self.label()?);
                    let close = self.expect(MARKERLESS_SOLID)?;
                    tail.minlen = self.minimum(close)
                } else {
                    tail.minlen = self.minimum(k)
                }
            }
            MARKERLESS_SOLID => {
                tail.arrow = 2;
                if self.ts[k].end - self.ts[k].start == 2 && self.labeled_right(CLOSES_LABEL) {
                    tail.label = Some(self.label()?);
                    tail.arrow = match self.t() {
                        FORWARD_SOLID => 0,
                        FORWARD_CIRCLE => 3,
                        FORWARD_CROSS => 4,
                        MARKERLESS_SOLID => 2,
                        _ => {
                            return Err(self.alternative(
                                "FORWARD_SOLID|MARKERLESS_SOLID|FORWARD_CIRCLE|FORWARD_CROSS",
                            ))
                        }
                    };
                    if tail.arrow == 0 || tail.arrow == 2 {
                        tail.minlen = self.minimum(self.p)
                    }
                    self.p += 1
                } else {
                    tail.minlen = self.minimum(k)
                }
            }
            FORWARD_CIRCLE => tail.arrow = 3,
            FORWARD_CROSS => tail.arrow = 4,
            BACKWARD_CIRCLE | BACKWARD_CROSS => {
                tail.arrow = if t == BACKWARD_CIRCLE { 5 } else { 6 };
                if self.labeled_right(0) {
                    tail.label = Some(self.label()?);
                    self.expect(MARKERLESS_SOLID)?;
                }
            }
            _ => {
                self.p -= 1;
                return Err(self.alternative(ARROWS));
            }
        }
        tail.target = self.element(false)?;
        tail.span = [self.ts[first].start, self.ts[self.p - 1].end];
        Ok(tail)
    }
    fn relationship(
        &mut self,
        source: &Item,
        declares: bool,
        tail: Tail,
        mut nodes: Vec<NodeOccurrence>,
    ) -> GraphStatement {
        let target = tail.target;
        if target.explicit {
            let d = self.draft(&target, "usecase");
            self.drafts.elements.push(d)
        }
        self.drafts.relationships.push(DraftRelationship {
            source: endpoint(source, declares),
            target: endpoint(&target, target.explicit),
            span: tail.span,
            id: tail.id.clone(),
            id_span: tail.id_span,
            relationship_type: tail.kind,
            arrow_type: tail.arrow,
            label: tail.label.clone(),
            minlen: tail.minlen,
        });
        nodes.push(occurrence(&target, target.explicit));
        let id = tail.id.unwrap_or_else(|| {
            let id = format!("edge-{}", self.edge_count);
            self.edge_count += 1;
            id
        });
        GraphStatement {
            kind: "edge".into(),
            nodes: Some(nodes),
            edges: Some(vec![EdgeOccurrence {
                id,
                span: [0, 0],
                id_span: tail.id_span,
                label_span: tail.label.map(|l| l.span),
                ..Default::default()
            }]),
            ..Default::default()
        }
    }
    fn actors(&mut self, top: bool) -> Result<GraphStatement, PeleError> {
        self.p += 1;
        let mut items = vec![self.element(true)?];
        let mut tail = None;
        if self.t() == T::COMMA {
            while self.t() == T::COMMA {
                self.p += 1;
                items.push(self.element(true)?);
            }
        } else if top && flags(self.t()) & RELATION != 0 {
            tail = Some(self.tail()?)
        }
        self.line_end()?;
        for item in &items {
            let d = self.draft(item, "actor");
            self.drafts.elements.push(d)
        }
        let nodes = items.iter().map(|i| occurrence(i, true)).collect();
        Ok(if let Some(tail) = tail {
            self.relationship(&items[0], true, tail, nodes)
        } else {
            GraphStatement {
                kind: "node".into(),
                nodes: Some(nodes),
                ..Default::default()
            }
        })
    }
    fn entity(&mut self, top: bool) -> Result<GraphStatement, PeleError> {
        let mut source = self.element(false)?;
        let tail = if top && flags(self.t()) & RELATION != 0 {
            Some(self.tail()?)
        } else {
            None
        };
        self.line_end()?;
        if tail.is_none() {
            source.explicit = true;
            if source.shape.is_none() {
                source.shape = Some("ellipse".into())
            }
        }
        if source.explicit {
            let d = self.draft(&source, "usecase");
            self.drafts.elements.push(d)
        }
        let nodes = vec![occurrence(&source, source.explicit)];
        Ok(if let Some(tail) = tail {
            self.relationship(&source, source.explicit, tail, nodes)
        } else {
            GraphStatement {
                kind: "node".into(),
                nodes: Some(nodes),
                ..Default::default()
            }
        })
    }
    fn boundary(&mut self) -> Result<GraphStatement, PeleError> {
        self.p += 1;
        let mut named = self.name(true)?;
        let inline = if self.t() == T::METADATA_START {
            Some(self.metadata()?)
        } else {
            None
        };
        if self.t() == T::CLASS_SEPARATOR {
            self.class_suffix(&mut named.classes, &mut named.class_spans)?
        }
        let group = DraftBoundary {
            named: named.named.clone(),
            classes: named.classes,
        };
        self.line_end()?;
        self.drafts.boundaries.push(group.clone());
        self.parent = Some(group.clone());
        let mut children = vec![];
        while flags(self.t()) & BOUNDARY_LINE != 0 {
            children.push(self.line(false)?)
        }
        self.parent = None;
        let end = self.expect(T::END)?;
        self.line_end()?;
        let mut out = GraphStatement {
            kind: "group".into(),
            group: Some(group.named.id.clone()),
            id_span: Some(group.named.span),
            title_span: Some(group.named.label.span),
            end_span: Some(self.span(end)),
            class_spans: Some(named.class_spans),
            children: if children.is_empty() {
                None
            } else {
                Some(children)
            },
            ..Default::default()
        };
        if let Some(inline) = inline {
            out.metadata = Some(occurrences(&inline));
            self.drafts.metadata.push(DraftMetadata {
                target: group.named.id,
                span: group.named.span,
                properties: inline,
                statement: self.statement_index,
                child: None,
            })
        }
        Ok(out)
    }
    fn style(&mut self) -> Result<String, PeleError> {
        let first = self.p;
        let t = self.t();
        if word(t) || t == T::CSS_IDENTIFIER {
            self.p += 1
        } else if t == T::MARKERLESS_SOLID {
            self.p += 1;
            if !word(self.t()) {
                return Err(self.mismatch("WORD"));
            }
            self.p += 1
        } else {
            return Err(self.alternative("WORD|CSS_IDENTIFIER|MARKERLESS_SOLID"));
        }
        self.expect(T::COLON)?;
        if flags(self.t()) & STYLE_PART == 0 {
            return Err(self.fail(format!("Expecting: expecting at least one iteration which starts with one of these possible Token sequences::\n  <[{}]>\nbut found: '{}'",STYLE_PARTS.replace('|',"] ,["),self.image(self.p))));
        }
        while flags(self.t()) & STYLE_PART != 0 {
            self.p += 1
        }
        Ok(self
            .source
            .text(self.ts[first].start, self.ts[self.p - 1].end)
            .replace("\\,", ","))
    }
    fn styles(&mut self) -> Result<Vec<String>, PeleError> {
        let mut out = vec![self.style()?];
        while self.t() == T::COMMA {
            self.p += 1;
            out.push(self.style()?)
        }
        Ok(out)
    }
    fn identifiers(&mut self) -> Result<Vec<usize>, PeleError> {
        let mut out = vec![self.expect(T::IDENTIFIER)?];
        while self.t() == T::COMMA {
            self.p += 1;
            out.push(self.expect(T::IDENTIFIER)?)
        }
        Ok(out)
    }
    fn reference(&self, k: usize) -> NodeOccurrence {
        let span = self.content(k);
        NodeOccurrence {
            id: self.image(k),
            span,
            id_span: span,
            ..Default::default()
        }
    }
    fn top(&mut self) -> Result<GraphStatement, PeleError> {
        use T::*;
        let k = self.p;
        let t = self.t();
        let mut out = GraphStatement::default();
        if matches!(t, ACC_TITLE_LINE | ACC_DESCR_LINE | ACC_DESCR_BLOCK) {
            self.p += 1;
            let text = self.image(k);
            if t == ACC_DESCR_BLOCK {
                self.drafts.acc_description = text
                    [text.find('{').unwrap() + 1..text.rfind('}').unwrap()]
                    .trim_matches(space)
                    .into()
            } else if t == ACC_DESCR_LINE {
                self.drafts.acc_description = text[text.find(':').unwrap() + 1..]
                    .trim_matches(space)
                    .into()
            } else {
                self.drafts.acc_title = text[text.find(':').unwrap() + 1..]
                    .trim_matches(space)
                    .into()
            }
            out.kind = if t == ACC_TITLE_LINE {
                "accTitle"
            } else {
                "accDescr"
            }
            .into()
        } else if t == DIRECTION {
            self.p += 1;
            let value = self.t();
            if !(TD..=RL).contains(&value) {
                return Err(self.alternative("TD|TB|BT|LR|RL"));
            }
            self.drafts.direction = Some(if value == TD {
                "TB".into()
            } else {
                self.image(self.p)
            });
            self.p += 1;
            out.kind = "direction".into()
        } else if t == ACTOR {
            return self.actors(true);
        } else if t == SYSTEM_BOUNDARY {
            return self.boundary();
        } else if t == NOTE {
            self.p += 1;
            self.expect(FOR)?;
            let target = self.expect(IDENTIFIER)?;
            let text = self.label()?;
            self.drafts
                .notes
                .push((self.image(target), self.span(target), text.clone()));
            out.kind = "note".into();
            out.reference = Some(format!("note-{}", self.note_count));
            self.note_count += 1;
            out.ref_span = Some(text.span);
            out.nodes = Some(vec![self.reference(target)])
        } else if t == JSON_DECLARATION_START {
            self.p += 1;
            let literal = self.expect(JSON_OBJECT_LITERAL)?;
            let mut classes = vec![];
            let mut spans = vec![];
            if self.t() == CLASS_SEPARATOR {
                self.class_suffix(&mut classes, &mut spans)?
            }
            let mut from = self.ts[k].start + 4;
            while matches!(self.source.c(from), 32 | 9) {
                from += 1
            }
            let mut to = from;
            while self.source.c(to) < 128
                && ((self.source.c(to) as u8).is_ascii_alphanumeric() || self.source.c(to) == 95)
            {
                to += 1
            }
            let id = self.source.text(from, to);
            let index = self.drafts.jsons.len();
            self.drafts.jsons.push(DraftJson {
                id: id.clone(),
                value: Default::default(),
                property_order: vec![],
                span: [from, to],
                classes,
            });
            self.pending_json.push((index, literal));
            out.kind = "json".into();
            out.nodes = Some(vec![NodeOccurrence {
                id,
                span: [from, to],
                id_span: [from, to],
                defines: Some(true),
                class_spans: Some(spans.clone()),
                ..Default::default()
            }]);
            out.class_spans = Some(spans)
        } else if t == CLASS_DEF {
            self.p += 1;
            let ids = self.identifiers()?;
            let styles = self.styles()?;
            self.drafts
                .class_defs
                .push((ids.iter().map(|&k| self.image(k)).collect(), styles));
            out.kind = "classDef".into();
            out.reference = Some(self.image(ids[0]));
            out.ref_span = Some(self.span(ids[0]))
        } else if t == CLASS {
            self.p += 1;
            let targets = self.identifiers()?;
            let classes = self.identifiers()?;
            self.drafts.classes.push((
                targets
                    .iter()
                    .map(|&k| (self.image(k), self.span(k)))
                    .collect(),
                classes.iter().map(|&k| self.image(k)).collect(),
            ));
            out.kind = "classAssign".into();
            out.reference = Some(self.image(classes[0]));
            out.ref_span = Some(self.span(classes[0]));
            out.nodes = Some(targets.iter().map(|&k| self.reference(k)).collect())
        } else if t == STYLE {
            self.p += 1;
            let target = self.expect(IDENTIFIER)?;
            let styles = self.styles()?;
            self.drafts
                .styles
                .push((self.image(target), self.span(target), styles));
            out.kind = "style".into();
            out.nodes = Some(vec![self.reference(target)])
        } else {
            let next = self.next(1);
            if next == METADATA_START {
                if flags(self.next(2)) & METADATA_FIRST == 0 {
                    return Err(self.alternative(&sequences()));
                }
                self.p += 1;
                let mut target = self.reference(k);
                if t != IDENTIFIER {
                    target.id = generated(&self.source.text(target.span[0], target.span[1]))
                }
                let assigned = self.metadata()?;
                out.kind = "metadata".into();
                out.nodes = Some(vec![target.clone()]);
                out.metadata = Some(occurrences(&assigned));
                self.drafts.metadata.push(DraftMetadata {
                    target: target.id,
                    span: target.span,
                    properties: assigned,
                    statement: self.statement_index,
                    child: None,
                })
            } else {
                let follows = flags(next) & FOLLOWS_NAME != 0
                    || (t == IDENTIFIER && (next == LPAREN || next == LBRACKET));
                if !follows
                    || (t == IDENTIFIER
                        && [
                            "allowmixing",
                            "newpage",
                            "package",
                            "rectangle",
                            "skinparam",
                        ]
                        .contains(&self.image(k).to_lowercase().as_str()))
                {
                    return Err(self.alternative(&sequences()));
                }
                return self.entity(true);
            }
        }
        self.line_end()?;
        Ok(out)
    }
    fn line(&mut self, top: bool) -> Result<GraphStatement, PeleError> {
        let k = self.p;
        let t = self.t();
        if t == T::NEWLINE {
            self.p += 1;
            return Ok(GraphStatement {
                kind: "blank".into(),
                span: self.span(k),
                ..Default::default()
            });
        }
        if t == T::COMMENT {
            self.p += 1;
            self.line_end()?;
            return Ok(GraphStatement {
                kind: "comment".into(),
                span: self.span(k),
                ..Default::default()
            });
        }
        let mut out = if top {
            self.top()?
        } else if t == T::ACTOR {
            self.actors(false)?
        } else {
            self.entity(false)?
        };
        out.span = [self.ts[k].start, self.last_end];
        if let Some(edges) = &mut out.edges {
            for edge in edges {
                edge.span = out.span
            }
        }
        Ok(out)
    }
}
fn generated(s: &str) -> String {
    s.encode_utf16()
        .map(|c| {
            if c < 128 && ((c as u8).is_ascii_alphanumeric() || c == 95) {
                char::from_u32(c as u32).unwrap()
            } else {
                '_'
            }
        })
        .collect()
}
fn endpoint(item: &Item, declares: bool) -> DraftEndpoint {
    DraftEndpoint {
        named: item.named.clone(),
        declares,
        classed: !item.classes.is_empty(),
    }
}
fn occurrences(p: &[DraftProperty]) -> Vec<MetadataOccurrence> {
    p.iter().map(|p| p.occurrence.clone()).collect()
}
fn occurrence(item: &Item, defines: bool) -> NodeOccurrence {
    NodeOccurrence {
        id: item.named.id.clone(),
        span: item.node_span,
        id_span: item.named.span,
        label_span: Some(item.named.label.span),
        defines: if defines { Some(true) } else { None },
        stereotype_span: item.stereotype_span,
        metadata: item.metadata.as_deref().map(occurrences),
        class_spans: if item.class_spans.is_empty() {
            None
        } else {
            Some(item.class_spans.clone())
        },
    }
}
pub(super) fn parse(c: ParseContext<'_>) -> Result<UsecaseModel, PeleError> {
    let source = Source::new(c.source);
    let ts = tokenize(&source)?;
    let mut r = Reader {
        source,
        ts,
        p: 0,
        last_end: 0,
        drafts: Default::default(),
        parent: None,
        pending_json: vec![],
        edge_count: 0,
        note_count: 0,
        statement_index: 0,
    };
    let header = r.expect(T::USECASE)?;
    r.line_end()?;
    let mut statements = vec![];
    while flags(r.t()) & LINE != 0 {
        r.statement_index = statements.len();
        statements.push(r.line(true)?)
    }
    if r.t() != T::EOF {
        return Err(r.fail(format!(
            "Redundant input, expecting EOF but found: {}",
            r.image(r.p)
        )));
    }
    for &(index, literal) in &r.pending_json {
        let (value, order) =
            super::json::parse(&r.image(literal), r.source.loc(r.ts[literal].start))?;
        r.drafts.jsons[index].value = value;
        r.drafts.jsons[index].property_order = order
    }
    let span = r.span(header);
    super::db::resolve(
        c.source,
        r.drafts,
        span,
        statements,
        c.title.map(str::to_string),
    )
}
