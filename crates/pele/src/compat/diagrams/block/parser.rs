//! Pele block/parser.ts and db.ts, 458322a5 (MIT, kepano).
use super::{
    lexer::{tokenize, Token},
    model::{BlockClassDef, BlockModel, BlockNode},
    tokens::{NAMES, T},
};
use crate::compat::{source::space, DiagramType, ErrorCode, PeleError};
use std::collections::{HashMap, HashSet};
const START:&str="'NODE_ID', 'COLUMNS', 'SPACE_BLOCK', 'BLOCK_DIAGRAM_KEY', 'id-block', 'classDef', 'class', 'style'";
struct Parser<'a> {
    src: &'a str,
    tokens: Vec<Token>,
    i: usize,
    model: BlockModel,
    ids: usize,
    inner: HashMap<usize, Vec<usize>>,
}
fn int(s: &str) -> f64 {
    s.parse().unwrap_or(f64::INFINITY)
}
fn kind(s: &str) -> String {
    match s {
        "[]" => "square",
        "()" => "round",
        "(())" => "circle",
        ">]" => "rect_left_inv_arrow",
        "{}" => "diamond",
        "{{}}" => "hexagon",
        "([])" => "stadium",
        "[[]]" => "subroutine",
        "[()]" => "cylinder",
        "((()))" => "doublecircle",
        "[//]" => "lean_right",
        "[\\\\]" => "lean_left",
        "[/\\]" => "trapezoid",
        "[\\/]" => "inv_trapezoid",
        "<[]>" => "block_arrow",
        _ => "na",
    }
    .into()
}
impl Parser<'_> {
    fn current(&self) -> T {
        self.tokens.get(self.i).map_or(T::END, |t| t.kind)
    }
    fn take(&mut self) -> String {
        let t = self
            .tokens
            .get(self.i)
            .map_or(String::new(), |t| t.text.clone());
        self.i += 1;
        t
    }
    fn fail(&self, expected: &str) -> PeleError {
        let token = self
            .tokens
            .get(self.i)
            .unwrap_or_else(|| self.tokens.last().unwrap());
        if token.kind == T::ERROR {
            return PeleError::syntax("block", self.src, token.start, "", true);
        }
        PeleError::syntax(
            "block",
            self.src,
            token.start,
            &format!("Expecting {expected}, got '{}'", NAMES[token.kind as usize]),
            false,
        )
    }
    fn expect(&mut self, t: T) -> Result<String, PeleError> {
        if self.current() != t {
            return Err(self.fail(&format!("'{}'", NAMES[t as usize])));
        }
        Ok(self.take())
    }
    fn id(&mut self) -> String {
        self.ids += 1;
        format!("id-{}", self.ids)
    }
    fn node(&mut self) -> Result<BlockNode, PeleError> {
        let id = self.expect(T::NODE_ID)?;
        let mut node = BlockNode {
            id: Some(id),
            own_keys: ["id", "label", "type", "directions"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
            ..Default::default()
        };
        let mut shape = String::new();
        if self.current() == T::NODE_DSTART {
            shape = self.take();
            node.label = Some(self.expect(T::STR)?);
            shape += &self.expect(T::NODE_DEND)?;
        } else if self.current() == T::BLOCK_ARROW_START {
            shape = self.take();
            node.label = Some(self.expect(T::STR)?);
            let mut directions = vec![self.expect(T::DIR)?];
            while self.current() == T::DIR {
                directions.push(self.take());
            }
            node.directions = Some(directions);
            shape += &self.expect(T::BLOCK_ARROW_END)?;
        }
        node.kind = Some(kind(&shape));
        Ok(node)
    }
    fn node_statement(&mut self) -> Result<Vec<usize>, PeleError> {
        let mut first = self.node()?;
        first.remember("widthInColumns");
        first.width_in_columns = Some(if self.current() == T::SIZE {
            int(&self.take())
        } else {
            1.
        });
        let first_idx = self.model.push(first);
        let mut from = first_idx;
        let mut chain = None;
        while matches!(self.current(), T::LINK | T::START_LINK) {
            let label = if self.current() == T::START_LINK {
                self.i += 1;
                self.expect(T::LINK_LABEL)?;
                let label = self.expect(T::STR)?;
                self.expect(T::LINK)?;
                label
            } else {
                self.i += 1;
                String::new()
            };
            let link = self.tokens[self.i - 1].text.trim_matches(space).to_string();
            let mut to = self.node()?;
            if to.directions.is_some() {
                to.array_origins[0] = Some(self.model.array_origin());
            }
            let to_id = to.id.clone().unwrap();
            let from_id = self.model.nodes[from].id.clone().unwrap();
            let arrow_end = match link.chars().next_back() {
                Some('x') => "arrow_cross",
                Some('o') => "arrow_circle",
                Some('>') => "arrow_point",
                _ => "",
            };
            let arrow_start = match link.chars().next() {
                Some('x') => "arrow_cross",
                Some('o') => "arrow_circle",
                Some('<') => "arrow_point",
                _ => "arrow_open",
            };
            let c = chain.get_or_insert_with(|| {
                self.model.nodes[first_idx].width_in_columns = None;
                self.model.nodes[first_idx]
                    .own_keys
                    .retain(|key| key != "widthInColumns");
                vec![first_idx]
            });
            let edge = self.model.push(BlockNode {
                id: Some(format!("{from_id}-{to_id}")),
                start: Some(from_id),
                end: Some(to_id),
                label: Some(label),
                kind: Some("edge".into()),
                thickness: Some(
                    if link.contains("==") {
                        "thick"
                    } else {
                        "normal"
                    }
                    .into(),
                ),
                pattern: Some(
                    if link.contains(".-") {
                        "dotted"
                    } else {
                        "solid"
                    }
                    .into(),
                ),
                directions: to.directions.clone(),
                array_origins: [to.array_origins[0], None, None, None],
                arrow_type_end: Some(arrow_end.into()),
                arrow_type_start: Some(arrow_start.into()),
                own_keys: [
                    "id",
                    "start",
                    "end",
                    "label",
                    "type",
                    "thickness",
                    "pattern",
                    "directions",
                    "arrowTypeEnd",
                    "arrowTypeStart",
                ]
                .into_iter()
                .map(str::to_owned)
                .collect(),
                ..Default::default()
            });
            from = self.model.push(to);
            c.extend([edge, from]);
        }
        Ok(chain.unwrap_or_else(|| vec![first_idx]))
    }
    fn parse(&mut self) -> Result<Vec<usize>, PeleError> {
        self.expect(T::KEY)?;
        let mut open: Vec<(Vec<usize>, Option<Vec<usize>>)> = vec![];
        let mut list = vec![];
        loop {
            match self.current() {
                T::NODE_ID => list.extend(self.node_statement()?),
                T::COLUMNS => {
                    let value = int(&self.take());
                    list.push(self.model.push(BlockNode {
                        kind: Some("column-setting".into()),
                        columns: Some(value),
                        ..Default::default()
                    }));
                }
                T::SPACE_BLOCK => {
                    let width = int(&self.take());
                    let id = self.id();
                    list.push(
                        self.model.push(BlockNode {
                            id: Some(id),
                            kind: Some("space".into()),
                            label: Some(String::new()),
                            width: Some(width),
                            children: Some(vec![]),
                            own_keys: ["id", "type", "label", "width", "children"]
                                .into_iter()
                                .map(str::to_owned)
                                .collect(),
                            ..Default::default()
                        }),
                    );
                }
                T::KEY => {
                    self.i += 1;
                    open.push((list, None));
                    list = vec![];
                }
                T::ID_BLOCK => {
                    self.i += 1;
                    if self.current() != T::NODE_ID {
                        return Err(self.fail("'NODE_ID'"));
                    }
                    let header = self.node_statement()?;
                    open.push((list, Some(header)));
                    list = vec![];
                }
                T::end => {
                    let Some((parent, header)) = open.pop() else {
                        return Err(self.fail(START));
                    };
                    if list.is_empty() {
                        return Err(self.fail(START));
                    }
                    self.i += 1;
                    let id = self.id();
                    let mut node = match header {
                        None => BlockNode {
                            id: Some(id),
                            label: Some(String::new()),
                            own_keys: ["id", "type", "label", "children"]
                                .into_iter()
                                .map(str::to_owned)
                                .collect(),
                            ..Default::default()
                        },
                        Some(h) if h.len() == 1 => self.model.nodes[h[0]].clone(),
                        Some(h) => BlockNode {
                            extra: h
                                .into_iter()
                                .enumerate()
                                .map(|(i, n)| (i.to_string(), n))
                                .collect(),
                            ..Default::default()
                        },
                    };
                    node.kind = Some("composite".into());
                    node.remember("type");
                    node.remember("children");
                    let n = self.model.push(node);
                    self.inner.insert(n, list);
                    list = parent;
                    list.push(n);
                }
                T::CLASSDEF => {
                    self.i += 1;
                    let id = self.expect(T::CLASSDEF_ID)?.trim_matches(space).to_string();
                    let css = self
                        .expect(T::CLASSDEF_STYLEOPTS)?
                        .trim_matches(space)
                        .to_string();
                    list.push(self.model.push(BlockNode {
                        kind: Some("classDef".into()),
                        id: Some(id),
                        css: Some(css),
                        ..Default::default()
                    }));
                }
                T::CLASS => {
                    self.i += 1;
                    let id = self
                        .expect(T::CLASSENTITY_IDS)?
                        .trim_matches(space)
                        .to_string();
                    let css = self.expect(T::STYLECLASS)?.trim_matches(space).to_string();
                    list.push(self.model.push(BlockNode {
                        kind: Some("applyClass".into()),
                        id: Some(id),
                        style_class: Some(css),
                        ..Default::default()
                    }));
                }
                T::STYLE => {
                    self.i += 1;
                    let id = self
                        .expect(T::STYLE_ENTITY_IDS)?
                        .trim_matches(space)
                        .to_string();
                    let css = self
                        .expect(T::STYLE_DEFINITION_DATA)?
                        .trim_matches(space)
                        .to_string();
                    list.push(self.model.push(BlockNode {
                        kind: Some("applyStyles".into()),
                        id: Some(id),
                        styles_str: Some(css),
                        ..Default::default()
                    }));
                }
                T::EOF => {
                    if !open.is_empty() {
                        return Err(self.fail(&format!("{START}, 'end'")));
                    }
                    if list.is_empty() {
                        return Err(self.fail(START));
                    }
                    return Ok(list);
                }
                _ => {
                    return Err(self.fail(&if list.is_empty() {
                        START.into()
                    } else {
                        format!("{START}, '{}'", if open.is_empty() { "EOF" } else { "end" })
                    }))
                }
            }
        }
    }
    fn set_id(&mut self, id: String, node: usize) {
        if let Some(&position) = self.model.positions.get(&id) {
            self.model.by_id[position].1 = node;
        } else {
            self.model
                .positions
                .insert(id.clone(), self.model.by_id.len());
            self.model.by_id.push((id.clone(), node));
        }
        self.model.lookup.insert(id, node);
    }

    fn named(&mut self, id: &str, pending: &mut HashSet<String>) -> usize {
        if let Some(&i) = self.model.lookup.get(id) {
            return i;
        }
        let i = self.model.push(BlockNode {
            id: Some(id.into()),
            kind: Some("na".into()),
            children: Some(vec![]),
            own_keys: ["id", "type", "children"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
            ..Default::default()
        });
        self.set_id(id.into(), i);
        if pending.insert(id.into()) {
            self.model.pending.push(id.into());
        }
        i
    }
    fn hierarchy(&mut self, list: Vec<usize>) -> Result<(), PeleError> {
        struct Frame {
            list: Vec<usize>,
            at: usize,
            parent: usize,
            children: Vec<usize>,
        }
        let frame = |list: Vec<usize>, parent, _model: &BlockModel| Frame {
            list,
            at: 0,
            parent,
            children: vec![],
        };
        let mut stack = vec![frame(list, self.model.root, &self.model)];
        let mut pending = HashSet::new();
        let mut edge_count = HashMap::<String, usize>::new();
        let mut edge_order = HashMap::<String, usize>::new();
        let mut colors = 0;
        let mut spaces = 0.;
        while let Some(f) = stack.last_mut() {
            if f.at == f.list.len() {
                let f = stack.pop().unwrap();
                self.model.children(f.parent, f.children);
                continue;
            }
            let i = f.list[f.at];
            f.at += 1;
            let n = self.model.nodes[i].clone();
            match n.kind.as_deref() {
                Some("classDef") => {
                    let id = n.id.unwrap();
                    let idx = if let Some(k) = self.model.classes.iter().position(|(k, _)| k == &id)
                    {
                        k
                    } else {
                        self.model.classes.push((
                            id.clone(),
                            BlockClassDef {
                                id,
                                styles: vec![],
                                text_styles: vec![],
                            },
                        ));
                        self.model.classes.len() - 1
                    };
                    let def = &mut self.model.classes[idx].1;
                    for attr in n.css.unwrap_or_default().split(',') {
                        let fixed = attr.replacen(';', "", 1).trim_matches(space).to_string();
                        if attr.contains("color") {
                            def.text_styles.push(
                                fixed
                                    .replacen("fill", "bgFill", 1)
                                    .replacen("color", "fill", 1),
                            );
                        }
                        def.styles.push(fixed);
                    }
                    continue;
                }
                Some("applyClass") => {
                    for id in n.id.unwrap().split(',') {
                        let id = id.trim_matches(space);
                        let at = self.named(id, &mut pending);
                        self.model.nodes[at].remember("classes");
                        if self.model.nodes[at].classes.is_none() {
                            let origin = self.model.array_origin();
                            self.model.nodes[at].array_origins[1] = Some(origin);
                        }
                        self.model.nodes[at]
                            .classes
                            .get_or_insert_with(Vec::new)
                            .push(n.style_class.clone().unwrap_or_default());
                    }
                    continue;
                }
                Some("applyStyles") => {
                    if let Some(styles) = n.styles_str.filter(|s| !s.is_empty()) {
                        let at = self.named(&n.id.unwrap(), &mut pending);
                        self.model.nodes[at].remember("styles");
                        let origin = self.model.array_origin();
                        self.model.nodes[at].array_origins[2] = Some(origin);
                        self.model.nodes[at].styles =
                            Some(styles.split(',').map(str::to_owned).collect());
                    }
                    continue;
                }
                Some("column-setting") => {
                    self.model.nodes[f.parent].remember("columns");
                    self.model.nodes[f.parent].columns = Some(n.columns.unwrap_or(-1.));
                    continue;
                }
                Some("edge") => {
                    let id = n.id.unwrap();
                    let count = edge_count.entry(id.clone()).or_default();
                    if *count == 0 {
                        edge_order.insert(id.clone(), self.model.edge_count.len());
                        self.model.edge_count.push((id.clone(), 0));
                    }
                    *count += 1;
                    self.model.edge_count[edge_order[&id]].1 = *count;
                    self.model.nodes[i].id = Some(format!("{count}-{id}"));
                    self.model.edges.push(i);
                    continue;
                }
                _ => {}
            }
            if self.model.nodes[i].id.is_none() {
                self.model.nodes[i].remember("id");
                self.model.nodes[i].id = Some(self.id());
            }
            let id = self.model.nodes[i].id.clone().unwrap();
            let composite = n.kind.as_deref() == Some("composite");
            if self.model.nodes[i]
                .label
                .as_ref()
                .is_none_or(String::is_empty)
            {
                self.model.nodes[i].remember("label");
                self.model.nodes[i].label =
                    Some(if composite { String::new() } else { id.clone() });
            }
            let known = self.model.lookup.get(&id).copied();
            let fresh = known.is_none() || pending.remove(&id);
            if !fresh {
                let known = known.unwrap();
                if n.kind.as_deref() != Some("na") {
                    self.model.nodes[known].kind = n.kind.clone();
                }
                if self.model.nodes[i].label.as_deref() != Some(&id) {
                    self.model.nodes[known].label = self.model.nodes[i].label.clone();
                }
            } else {
                if let Some(known) = known {
                    self.model.nodes[i].remember("classes");
                    self.model.nodes[i].remember("styles");
                    self.model.nodes[i].array_origins[1] = self.model.nodes[known].array_origins[1];
                    self.model.nodes[i].array_origins[2] = self.model.nodes[known].array_origins[2];
                    self.model.nodes[i].classes = self.model.nodes[known].classes.clone();
                    self.model.nodes[i].styles = self.model.nodes[known].styles.clone();
                }
                if composite {
                    self.model.nodes[i].remember("colorIndex");
                    self.model.nodes[i].color_index = Some(colors);
                    colors += 1;
                }
                self.set_id(id.clone(), i);
            }
            let inner = self.inner.remove(&i);
            self.model.children(i, vec![]);
            if n.kind.as_deref() == Some("space") {
                let width = n.width.unwrap_or(1.);
                if !(width <= 100000. - spaces) {
                    return Err({
                        let mut e = PeleError::new(
                            "A block diagram can hold at most 100000 space blocks.",
                            ErrorCode::Limit,
                        );
                        e.diagram_type = Some("block".into());
                        e
                    });
                }
                spaces += width.max(0.);
                for j in 0..width as usize {
                    let mut copy = self.model.nodes[i].clone();
                    copy.id = Some(format!("{id}-{j}"));
                    copy.array_origins[3] = None;
                    let at = self.model.push(copy);
                    self.set_id(format!("{id}-{j}"), at);
                    stack.last_mut().unwrap().children.push(at);
                }
            } else if fresh {
                stack.last_mut().unwrap().children.push(i);
            }
            if let Some(inner) = inner.filter(|v| !v.is_empty()) {
                stack.push(frame(inner, i, &self.model));
            }
        }
        self.model.pending.retain(|key| pending.contains(key));
        self.model.colors = colors as f64;
        self.model.spaces = spaces;
        self.model.blocks = self.model.nodes[self.model.root]
            .children
            .clone()
            .unwrap_or_default();
        Ok(())
    }
}
pub(super) fn parse(src: &str, title: Option<&str>) -> Result<BlockModel, PeleError> {
    let mut model = BlockModel {
        diagram_type: DiagramType::Block,
        title: title.map(str::to_owned),
        nodes: vec![],
        root: 0,
        blocks: vec![],
        edges: vec![],
        classes: vec![],
        by_id: vec![],
        lookup: HashMap::new(),
        positions: HashMap::new(),
        pending: vec![],
        edge_count: vec![],
        ids: 0.,
        colors: 0.,
        spaces: 0.,
        next_array: 0,
    };
    model.root = model.push(BlockNode {
        id: Some("root".into()),
        kind: Some("composite".into()),
        children: Some(vec![]),
        columns: Some(-1.),
        own_keys: ["id", "type", "children", "columns"]
            .into_iter()
            .map(str::to_owned)
            .collect(),
        ..Default::default()
    });
    let mut parser = Parser {
        src,
        tokens: tokenize(src),
        i: 0,
        model,
        ids: 0,
        inner: HashMap::new(),
    };
    let list = parser.parse()?;
    parser.hierarchy(list)?;
    parser.model.ids = parser.ids as f64;
    Ok(parser.model)
}
