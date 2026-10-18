//! Direct parser translation of Pele agentflow/parser.ts, 458322a5 (MIT, kepano).
use super::{
    lexer::{tokenize, Token},
    model::{AgentDb, AgentflowModel, DocItem, FlowText, LabelType, LinkData, Loc},
    tokens::{NAMES, T},
};
use crate::compat::{source::space, Object, ParseContext, PeleError, Value};
use regex::Regex;
use std::sync::OnceLock;
const TEXT_TOKENS: &str = "'UNICODE_TEXT', 'TEXT', 'TAGSTART'";
const SEPARATORS: &str = "'COMMENT', 'SEMI', 'NEWLINE', 'EOF'";
const STATEMENT_START:&str="'COMMENT', 'SEMI', 'NEWLINE', 'SPACE', 'EOF', 'flow', 'global', 'connector', 'acc_title', 'acc_descr', 'acc_descr_multiline_value', 'STYLE', 'LINKSTYLE', 'CLASSDEF', 'CLASS', 'CLICK', 'NODE_STRING'";
fn separator(t: T) -> bool {
    matches!(t, T::COMMENT | T::SEMI | T::NEWLINE | T::EOF)
}

fn id(t: T) -> bool {
    matches!(
        t,
        T::NUM
            | T::NODE_STRING
            | T::DOWN
            | T::MINUS
            | T::DEFAULT
            | T::COMMA
            | T::COLON
            | T::AMP
            | T::BRKT
            | T::MULT
            | T::UNICODE_TEXT
    )
}
fn alpha(t: T) -> bool {
    matches!(
        t,
        T::NUM
            | T::UNICODE_TEXT
            | T::NODE_STRING
            | T::DIR
            | T::DOWN
            | T::MINUS
            | T::COMMA
            | T::COLON
            | T::AMP
            | T::BRKT
            | T::MULT
    )
}
fn no_tags(t: T) -> bool {
    matches!(
        t,
        T::NUM
            | T::NODE_STRING
            | T::SPACE
            | T::MINUS
            | T::AMP
            | T::UNICODE_TEXT
            | T::COLON
            | T::MULT
            | T::BRKT
            | T::STYLE
            | T::LINKSTYLE
            | T::CLASSDEF
            | T::CLASS
            | T::CLICK
            | T::GRAPH
            | T::DIR
            | T::flow
            | T::connector
            | T::global
            | T::end
            | T::DOWN
            | T::UP
            | T::START_LINK
    )
}
fn style_part(t: T) -> bool {
    matches!(
        t,
        T::NUM | T::NODE_STRING | T::COLON | T::SPACE | T::BRKT | T::STYLE
    )
}
struct Frame {
    global: bool,
    at: usize,
    data: Option<String>,
    data_from: usize,
    data_to: usize,
    id: Option<FlowText>,
    title: Option<FlowText>,
    same: bool,
    list: Vec<DocItem>,
    collect: bool,
}
struct Reader<'a> {
    src: &'a str,
    tokens: Vec<Token>,
    i: usize,
    pending: Option<String>,
    data_from: usize,
    data_to: usize,
    last_bare: bool,
    db: AgentDb,
}
impl Reader<'_> {
    fn t(&self) -> T {
        self.tokens.get(self.i).map_or(T::END, |t| t.kind)
    }
    fn take(&mut self) -> String {
        let s = self.tokens[self.i].text.clone();
        self.i += 1;
        s
    }
    fn fail(&self, expected: &str) -> PeleError {
        let t = &self.tokens[self.i.min(self.tokens.len() - 1)];
        if t.kind == T::ERROR {
            PeleError::syntax("agentflow", self.src, t.start, "", true)
        } else {
            PeleError::syntax(
                "agentflow",
                self.src,
                t.start,
                &format!("Expecting {expected}, got '{}'", NAMES[t.kind as usize]),
                false,
            )
        }
    }
    fn expect_also(&mut self, t: T, also: &str) -> Result<String, PeleError> {
        if self.t() != t {
            return Err(self.fail(&format!(
                "'{}'{}",
                NAMES[t as usize],
                if also.is_empty() {
                    String::new()
                } else {
                    format!(", {also}")
                }
            )));
        }
        Ok(self.take())
    }
    fn expect(&mut self, t: T) -> Result<String, PeleError> {
        self.expect_also(t, "")
    }
    fn separator(&mut self) -> Result<(), PeleError> {
        if !separator(self.t()) {
            return Err(self.fail(SEPARATORS));
        }
        self.i += 1;
        Ok(())
    }
    fn span(&self, a: usize, b: usize) -> Loc {
        let first = self.tokens[a].loc;
        let last = self.tokens[b].loc;
        Loc {
            first_line: first.first_line,
            first_column: first.first_column,
            last_line: last.last_line,
            last_column: last.last_column,
        }
    }
    fn attach(&mut self, id: &str, data: String, from: usize, to: usize) -> Result<(), PeleError> {
        let loc = self.span(from, to);
        self.db
            .add_vertex_loc(id, None, None, None, None, Some(data), Some(loc))?;
        self.db.extend_mapping(id, loc);
        Ok(())
    }
    fn attach_data(&mut self, id: &str, frame: &Frame) -> Result<(), PeleError> {
        if let Some(data) = &frame.data {
            let loc = self.span(frame.data_from, frame.data_to);
            self.db
                .add_vertex_loc(id, None, None, None, None, Some(data.clone()), Some(loc))?;
        }
        Ok(())
    }
    fn id_string(&mut self) -> Result<String, PeleError> {
        if !id(self.t()) {
            return Err(self.fail("'NODE_STRING'"));
        }
        let mut s = self.take();
        while id(self.t()) {
            s.push_str(&self.take())
        }
        Ok(s)
    }
    fn alpha_num(&mut self) -> Result<String, PeleError> {
        if !alpha(self.t()) {
            return Err(self.fail("'NODE_STRING'"));
        }
        let mut s = self.take();
        while alpha(self.t()) {
            s.push_str(&self.take())
        }
        Ok(s)
    }
    fn text(&mut self) -> Result<FlowText, PeleError> {
        let kind = match self.t() {
            T::STR => LabelType::String,
            T::MD_STR => LabelType::Markdown,
            T::TEXT | T::TAGSTART | T::TAGEND | T::UNICODE_TEXT => LabelType::Text,
            _ => return Err(self.fail(&format!("{TEXT_TOKENS}, 'TAGEND', 'STR', 'MD_STR'"))),
        };
        let mut text = self.take();
        while matches!(
            self.t(),
            T::TEXT | T::TAGSTART | T::TAGEND | T::UNICODE_TEXT
        ) {
            text.push_str(&self.take())
        }
        Ok(FlowText { text, kind })
    }
    fn closed(&mut self, t: T) -> Result<(), PeleError> {
        self.expect_also(t, TEXT_TOKENS)?;
        Ok(())
    }
    fn edge_text(&mut self) -> Result<FlowText, PeleError> {
        let kind = match self.t() {
            T::STR => LabelType::String,
            T::MD_STR => LabelType::Markdown,
            T::EDGE_TEXT | T::UNICODE_TEXT => LabelType::Text,
            _ => return Err(self.fail("'EDGE_TEXT', 'STR', 'MD_STR'")),
        };
        let mut text = self.take();
        while matches!(self.t(), T::EDGE_TEXT | T::UNICODE_TEXT) {
            text.push_str(&self.take())
        }
        Ok(FlowText { text, kind })
    }
    fn text_no_tags(&mut self) -> Result<FlowText, PeleError> {
        let kind = match self.t() {
            T::STR => LabelType::Text,
            T::MD_STR => LabelType::Markdown,
            t if no_tags(t) => LabelType::Text,
            _ => return Err(self.fail("'NODE_STRING', 'STR', 'MD_STR'")),
        };
        let mut text = self.take();
        while no_tags(self.t()) {
            text.push_str(&self.take())
        }
        Ok(FlowText { text, kind })
    }
    fn shape_data(&mut self) -> String {
        self.data_from = self.i;
        let mut s = self.take();
        while self.t() == T::SHAPE_DATA {
            s.push_str(&self.take())
        }
        self.data_to = self.i - 1;
        s
    }
    fn vertex(&mut self) -> Result<String, PeleError> {
        let from = self.i;
        let id = self.vertex_inner()?;
        let loc = self.span(from, self.i - 1);
        self.db.vertex_mapping(&id, self.last_bare, loc);
        Ok(id)
    }
    fn vertex_inner(&mut self) -> Result<String, PeleError> {
        let id = self.id_string()?;
        let t = self.t();
        self.last_bare = !matches!(
            t,
            T::SQS
                | T::DOUBLECIRCLESTART
                | T::PS
                | T::ELLIPSE_START
                | T::STADIUMSTART
                | T::SUBROUTINESTART
                | T::VERTEX_WITH_PROPS_START
                | T::CYLINDERSTART
                | T::DIAMOND_START
                | T::TAGEND
                | T::TRAPSTART
                | T::INVTRAPSTART
        );
        let mut props = None;
        let shape = match t {
            T::SQS => {
                self.i += 1;
                let text = self.text()?;
                self.closed(T::SQE)?;
                self.db
                    .add_vertex(&id, Some(text), Some("square"), None, None, None)?;
                return Ok(id);
            }
            T::DOUBLECIRCLESTART => {
                self.i += 1;
                "doublecircle"
            }
            T::PS => {
                self.i += 1;
                if self.t() == T::PS {
                    self.i += 1;
                    let text = self.text()?;
                    self.closed(T::PE)?;
                    self.expect(T::PE)?;
                    self.db
                        .add_vertex(&id, Some(text), Some("circle"), None, None, None)?;
                    return Ok(id);
                }
                "round"
            }
            T::ELLIPSE_START => {
                self.i += 1;
                "ellipse"
            }
            T::STADIUMSTART => {
                self.i += 1;
                "stadium"
            }
            T::SUBROUTINESTART => {
                self.i += 1;
                "subroutine"
            }
            T::VERTEX_WITH_PROPS_START => {
                self.i += 1;
                let field = self.expect(T::NODE_STRING)?;
                self.expect(T::COLON)?;
                let value = self.expect(T::NODE_STRING)?;
                self.expect(T::PIPE)?;
                let mut p = Object::default();
                p.insert(field, Value::String(value));
                props = Some(p);
                "rect"
            }
            T::CYLINDERSTART => {
                self.i += 1;
                "cylinder"
            }
            T::DIAMOND_START => {
                self.i += 1;
                if self.t() == T::DIAMOND_START {
                    self.i += 1;
                    let text = self.text()?;
                    self.closed(T::DIAMOND_STOP)?;
                    self.expect(T::DIAMOND_STOP)?;
                    self.db
                        .add_vertex(&id, Some(text), Some("hexagon"), None, None, None)?;
                    return Ok(id);
                }
                "diamond"
            }
            T::TAGEND => {
                self.i += 1;
                "odd"
            }
            T::TRAPSTART | T::INVTRAPSTART => {
                self.i += 1;
                let text = self.text()?;
                let shape = if t == T::TRAPSTART {
                    if self.t() == T::INVTRAPEND {
                        self.i += 1;
                        "lean_right"
                    } else {
                        self.closed(T::TRAPEND)?;
                        "trapezoid"
                    }
                } else if self.t() == T::TRAPEND {
                    self.i += 1;
                    "lean_left"
                } else {
                    self.closed(T::INVTRAPEND)?;
                    "inv_trapezoid"
                };
                self.db
                    .add_vertex(&id, Some(text), Some(shape), None, None, None)?;
                return Ok(id);
            }
            _ => {
                self.db.add_vertex(&id, None, None, None, None, None)?;
                return Ok(id);
            }
        };
        let label = self.text()?;
        let close = match t {
            T::DOUBLECIRCLESTART => T::DOUBLECIRCLEEND,
            T::PS => T::PE,
            T::ELLIPSE_START => T::ELLIPSE_END,
            T::STADIUMSTART => T::STADIUMEND,
            T::SUBROUTINESTART => T::SUBROUTINEEND,
            T::CYLINDERSTART => T::CYLINDEREND,
            T::DIAMOND_START => T::DIAMOND_STOP,
            _ => T::SQE,
        };
        self.closed(close)?;
        self.db
            .add_vertex(&id, Some(label), Some(shape), None, props, None)?;
        Ok(id)
    }
    fn styled_vertex(&mut self) -> Result<String, PeleError> {
        let id = self.vertex()?;
        if self.t() == T::STYLE_SEPARATOR {
            self.i += 1;
            let class = self.id_string()?;
            self.db.set_class(&id, &class)
        }
        Ok(id)
    }
    fn node(&mut self) -> Result<Vec<String>, PeleError> {
        let mut ids = vec![self.styled_vertex()?];
        self.pending = None;
        loop {
            let data = if self.t() == T::SHAPE_DATA {
                Some(self.shape_data())
            } else {
                None
            };
            let (from, to) = (self.data_from, self.data_to);
            if self.t() != T::SPACE {
                self.pending = data;
                return Ok(ids);
            }
            while self.t() == T::SPACE {
                self.i += 1
            }
            if self.t() != T::AMP {
                if data.is_some() {
                    return Err(self.fail("'AMP'"));
                }
                return Ok(ids);
            }
            self.i += 1;
            if self.t() != T::SPACE {
                return Err(self.fail("'SPACE'"));
            }
            while self.t() == T::SPACE {
                self.i += 1
            }
            let next = self.styled_vertex()?;
            if let Some(data) = data {
                self.attach(ids.last().unwrap(), data, from, to)?;
            }
            ids.push(next)
        }
    }
    fn link(&mut self) -> Result<LinkData, PeleError> {
        let id = if self.t() == T::LINK_ID {
            Some(self.take())
        } else {
            None
        };
        if self.t() == T::LINK {
            let end = self.take();
            let mut info = AgentDb::destruct_link(&end, None);
            info.id = id;
            if self.t() == T::PIPE {
                self.i += 1;
                info.text = Some(self.text()?);
                self.closed(T::PIPE)?;
                if self.t() == T::SPACE {
                    self.i += 1
                }
            }
            Ok(info)
        } else if self.t() == T::START_LINK {
            let start = self.take();
            let label = self.edge_text()?;
            let end = self.expect_also(T::LINK, "'EDGE_TEXT'")?;
            let mut info = AgentDb::destruct_link(&end, Some(&start));
            info.text = Some(label);
            info.id = id;
            Ok(info)
        } else {
            Err(self.fail("'LINK', 'START_LINK'"))
        }
    }
    fn vertex_statement(&mut self) -> Result<Vec<String>, PeleError> {
        let from = self.i;
        let mut stmt = self.node()?;
        if let Some(data) = self.pending.take() {
            self.attach(stmt.last().unwrap(), data, self.data_from, self.data_to)?;
        }
        let mut nodes = stmt.clone();
        loop {
            if !matches!(self.t(), T::LINK | T::START_LINK | T::LINK_ID) {
                return Ok(nodes);
            }
            let info = self.link()?;
            let next = self.node()?;
            if let Some(data) = self.pending.take() {
                self.attach(next.last().unwrap(), data, self.data_from, self.data_to)?;
            }
            self.db.add_link(&stmt, &next, &info)?;
            let loc = self.span(from, self.i - 1);
            self.db.edge_mapping(&next, loc);
            let mut all = next.clone();
            all.extend(nodes);
            nodes = all;
            stmt = next
        }
    }
    fn style(&mut self) -> Result<String, PeleError> {
        if !style_part(self.t()) {
            return Err(self.fail("'NODE_STRING', 'NUM', 'COLON', 'BRKT'"));
        }
        let mut text = self.take();
        while style_part(self.t()) {
            text.push_str(&self.take())
        }
        Ok(text)
    }
    fn styles(&mut self) -> Result<Vec<String>, PeleError> {
        let mut styles = vec![self.style()?];
        while self.t() == T::COMMA {
            self.i += 1;
            styles.push(self.style()?)
        }
        Ok(styles)
    }
    fn link_style(&mut self) -> Result<(), PeleError> {
        self.i += 1;
        self.expect(T::SPACE)?;
        let positions = if self.t() == T::DEFAULT {
            vec![self.take()]
        } else {
            let mut p = vec![self.expect_also(T::NUM, "'DEFAULT'")?];
            while self.t() == T::COMMA {
                self.i += 1;
                p.push(self.expect(T::NUM)?)
            }
            p
        };
        self.expect(T::SPACE)?;
        if self.t() != T::INTERPOLATE {
            let styles = self.styles()?;
            return self.db.update_link(&positions, styles, None);
        }
        self.i += 1;
        self.expect(T::SPACE)?;
        let interpolate = self.alpha_num()?;
        if self.t() == T::SPACE {
            self.i += 1;
            let styles = self.styles()?;
            self.db
                .update_link(&positions, Vec::new(), Some(&interpolate))?;
            self.db.update_link(&positions, styles, None)
        } else {
            self.db
                .update_link(&positions, Vec::new(), Some(&interpolate))
        }
    }
    fn link_tail(&mut self, id: &str, url: &str) -> Result<(), PeleError> {
        if self.t() != T::SPACE {
            return self.db.set_link(id, url, None);
        }
        self.i += 1;
        if self.t() == T::LINK_TARGET {
            let target = self.take();
            return self.db.set_link(id, url, Some(target));
        }
        let tooltip = self.expect_also(T::STR, "'LINK_TARGET'")?;
        if self.t() == T::SPACE {
            self.i += 1;
            let target = self.expect(T::LINK_TARGET)?;
            self.db.set_link(id, url, Some(target))?
        } else {
            self.db.set_link(id, url, None)?
        }
        self.db.set_tooltip(id, &tooltip);
        Ok(())
    }
    fn click(&mut self) -> Result<(), PeleError> {
        let id = self.take();
        match self.t() {
            T::CALLBACKNAME => {
                self.i += 1;
                if self.t() == T::CALLBACKARGS {
                    self.i += 1
                }
                let tooltip = if self.t() == T::SPACE {
                    self.i += 1;
                    Some(self.expect(T::STR)?)
                } else {
                    None
                };
                self.db.set_class(&id, "clickable");
                if let Some(tooltip) = tooltip {
                    self.db.set_tooltip(&id, &tooltip)
                }
                Ok(())
            }
            T::HREF => {
                self.i += 1;
                let url = self.expect(T::STR)?;
                self.link_tail(&id, &url)
            }
            T::STR => {
                let url = self.take();
                self.link_tail(&id, &url)
            }
            _ => {
                self.alpha_num()?;
                let tooltip = if self.t() == T::SPACE {
                    self.i += 1;
                    Some(self.expect(T::STR)?)
                } else {
                    None
                };
                self.db.set_class(&id, "clickable");
                if let Some(tooltip) = tooltip {
                    self.db.set_tooltip(&id, &tooltip)
                }
                Ok(())
            }
        }
    }
    fn header(&mut self, at: usize) -> Result<Frame, PeleError> {
        self.expect_also(T::SPACE, SEPARATORS)?;
        let id = self.text_no_tags()?;
        let mut title = FlowText {
            text: String::new(),
            kind: LabelType::Text,
        };
        if self.t() == T::SQS {
            self.i += 1;
            title = self.text()?;
            self.closed(T::SQE)?;
        }
        let data = if self.t() == T::SHAPE_DATA {
            Some(self.shape_data())
        } else {
            None
        };
        if !separator(self.t()) {
            return Err(self.fail(&format!("{SEPARATORS}, 'SHAPE_DATA'")));
        }
        Ok(Frame {
            global: false,
            at,
            id: Some(id),
            title: Some(title),
            same: false,
            data,
            data_from: self.data_from,
            data_to: self.data_to,
            list: vec![],
            collect: false,
        })
    }
    fn statement(&mut self) -> Result<DocItem, PeleError> {
        match self.t() {
            T::STYLE => {
                self.i += 1;
                self.expect(T::SPACE)?;
                let id = self.id_string()?;
                self.expect(T::SPACE)?;
                let styles = self.styles()?;
                self.db
                    .add_vertex(&id, None, None, Some(styles), None, None)?;
                self.separator()?
            }
            T::LINKSTYLE => {
                self.link_style()?;
                self.separator()?
            }
            T::CLASSDEF => {
                self.i += 1;
                self.expect(T::SPACE)?;
                let id = self.id_string()?;
                self.expect(T::SPACE)?;
                let styles = self.styles()?;
                self.db.add_class(&id, styles);
                self.separator()?
            }
            T::CLASS => {
                self.i += 1;
                self.expect(T::SPACE)?;
                let id = self.id_string()?;
                self.expect(T::SPACE)?;
                let class = self.id_string()?;
                self.db.set_class(&id, &class);
                self.separator()?
            }
            T::CLICK => {
                self.click()?;
                self.separator()?
            }
            T::connector => {
                let at = self.i;
                self.i += 1;
                let frame = self.header(at)?;
                let sep = self.i;
                self.i += 1;
                let id = self.db.add_connector(frame.id.clone(), frame.title.clone());
                self.attach_data(&id, &frame)?;
                let start = self.span(frame.at, frame.at);
                let end = self.span(sep, sep);
                self.db.block_mapping(&id, "connector", start, end);
                return Ok(DocItem::Text(id));
            }
            t @ (T::direction_tb
            | T::direction_bt
            | T::direction_rl
            | T::direction_lr
            | T::direction_td) => {
                self.i += 1;
                return Ok(DocItem::Dir(
                    ["TB", "BT", "RL", "LR", "TD"][t as usize - T::direction_tb as usize].into(),
                ));
            }
            T::acc_title => {
                self.i += 1;
                let value = self
                    .expect(T::acc_title_value)?
                    .trim_matches(space)
                    .to_string();
                self.db.acc_title(value.clone());
                return Ok(DocItem::Text(value));
            }
            T::acc_descr => {
                self.i += 1;
                let value = self
                    .expect(T::acc_descr_value)?
                    .trim_matches(space)
                    .to_string();
                self.db.acc_description(value.clone());
                return Ok(DocItem::Text(value));
            }
            T::acc_descr_multiline_value => {
                let value = self.take().trim_matches(space).to_string();
                self.db.acc_description(value.clone());
                return Ok(DocItem::Text(value));
            }
            t => {
                if !id(t) {
                    return Err(self.fail(STATEMENT_START));
                }
                let nodes = self.vertex_statement()?;
                self.separator()?;
                return Ok(DocItem::Nodes(nodes));
            }
        }
        Ok(DocItem::Nodes(Vec::new()))
    }
    fn document(&mut self) -> Result<(), PeleError> {
        let mut open: Vec<Frame> = vec![];
        let mut list = vec![];
        let mut collect = false;
        loop {
            match self.t() {
                T::END => break,
                T::end => {
                    let Some(frame) = open.pop() else { break };
                    let at = self.i;
                    self.i += 1;
                    if frame.global {
                        self.db.add_global(list);
                        list = frame.list;
                        collect = frame.collect;
                        continue;
                    }
                    let id = self.db.add_subgraph(
                        frame.id.clone(),
                        frame.title.clone(),
                        frame.same,
                        list,
                    );
                    self.attach_data(&id, &frame)?;
                    let start = self.span(frame.at, frame.at);
                    let end = self.span(at, at);
                    self.db.block_mapping(
                        frame.id.as_ref().map_or("", |v| &v.text),
                        "subgraph",
                        start,
                        end,
                    );
                    list = frame.list;
                    collect = frame.collect;
                    if collect {
                        list.push(DocItem::Text(id));
                    }
                }
                T::SEMI | T::NEWLINE | T::SPACE | T::EOF => {
                    if collect {
                        list.push(DocItem::Text(self.tokens[self.i].text.clone()));
                    }
                    self.i += 1;
                }
                T::COMMENT => self.i += 1,
                T::flow | T::global => {
                    let t = self.t();
                    let at = self.i;
                    self.i += 1;
                    let mut frame = if t == T::global || separator(self.t()) {
                        let frame = Frame {
                            global: t == T::global,
                            at,
                            id: None,
                            title: None,
                            same: false,
                            data: None,
                            data_from: 0,
                            data_to: 0,
                            list: vec![],
                            collect: false,
                        };
                        self.separator()?;
                        frame
                    } else {
                        let frame = self.header(at)?;
                        self.i += 1;
                        frame
                    };
                    frame.list = list;
                    frame.collect = collect;
                    open.push(frame);
                    list = vec![];
                    collect = true;
                }
                _ => {
                    let item = self.statement()?;
                    if collect && !matches!(&item,DocItem::Nodes(v)if v.is_empty()) {
                        list.push(item)
                    }
                }
            }
        }
        if !open.is_empty() {
            return Err(self.fail("'end'"));
        }
        Ok(())
    }
}

pub(super) fn parse(context: ParseContext<'_>) -> Result<AgentflowModel, PeleError> {
    static TRAIL: OnceLock<Regex> = OnceLock::new();
    let src = if context.source.contains('}') {
        TRAIL.get_or_init(||Regex::new(r"}[\t\x0b\x0c \r\u{a0}\u{1680}\u{2000}-\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}\u{feff}]*\n").unwrap())
            .replace_all(context.source, "}\n")
    } else {
        std::borrow::Cow::Borrowed(context.source)
    };
    let mut r = Reader {
        src: &src,
        tokens: tokenize(&src),
        i: 0,
        pending: None,
        data_from: 0,
        data_to: 0,
        last_bare: false,
        db: AgentDb::new(&context),
    };
    while matches!(r.t(), T::SPACE | T::NEWLINE | T::COMMENT) {
        r.i += 1
    }
    r.expect_also(T::GRAPH, "'SPACE', 'NEWLINE', 'COMMENT'")?;
    if r.t() == T::NODIR {
        r.i += 1;
        r.db.set_direction("TB")
    } else {
        let dir = r.expect_also(T::DIR, "'NODIR'")?;
        if r.t() == T::SPACE {
            while r.t() == T::SPACE {
                r.i += 1
            }
            if r.t() != T::NEWLINE {
                return Err(r.fail("'NEWLINE', 'SPACE'"));
            }
        } else if !matches!(r.t(), T::SEMI | T::NEWLINE | T::COMMENT) {
            return Err(r.fail("'COMMENT', 'SEMI', 'NEWLINE', 'SPACE'"));
        }
        r.i += 1;
        r.db.set_direction(&dir)
    }
    r.document()?;
    if r.t() != T::END {
        return Err(r.fail(STATEMENT_START));
    }
    r.db.finish_connectors();
    let graph = super::graph::build(&mut r.db, context.config);
    r.db.model.graph = Some(graph);
    Ok(r.db.model)
}
