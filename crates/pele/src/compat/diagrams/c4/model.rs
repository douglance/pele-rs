//! Pele c4/{types,db}.ts typed arena and ordered attributes, 458322a5 (MIT, kepano).
//! Boundary indexes retain shared object identity, including redeclarations and cycles.
use crate::compat::{source::space, DiagramType, ErrorCode, PeleError, Value};
use serde::{
    ser::{SerializeMap, SerializeSeq},
    Serialize, Serializer,
};
use std::collections::HashMap;
#[derive(Clone, Debug, Serialize)]
pub struct C4Named {
    pub key: String,
    pub value: String,
}
#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum C4Attr {
    Text(String),
    Named(C4Named),
}
#[derive(Clone, Debug)]
pub struct C4Element {
    pub alias: String,
    pub label: String,
    pub alias_attr: Option<C4Named>,
    pub label_attr: Option<C4Named>,
    pub parent: Option<usize>,
    pub element_type: Option<String>,
    pub descr: Option<String>,
    pub techn: Option<String>,
    pub sprite: Option<String>,
    pub tags: Option<String>,
    pub link: Option<String>,
    pub bg_color: Option<String>,
    pub font_color: Option<String>,
    pub border_color: Option<String>,
    pub shadowing: Option<String>,
    pub shape: Option<String>,
    pub legend_text: Option<String>,
    pub legend_sprite: Option<String>,
    pub extra: Option<Vec<(String, String)>>,
}
impl C4Element {
    fn new() -> Self {
        Self {
            alias: String::new(),
            label: String::new(),
            alias_attr: None,
            label_attr: None,
            parent: None,
            element_type: None,
            descr: None,
            techn: None,
            sprite: None,
            tags: None,
            link: None,
            bg_color: None,
            font_color: None,
            border_color: None,
            shadowing: None,
            shape: None,
            legend_text: None,
            legend_sprite: None,
            extra: None,
        }
    }
    fn set(&mut self, key: &str, value: String) -> bool {
        match key {
            "label" => self.label = value,
            "type" => self.element_type = Some(value),
            "descr" => self.descr = Some(value),
            "techn" => self.techn = Some(value),
            "sprite" => self.sprite = Some(value),
            "tags" => self.tags = Some(value),
            "link" => self.link = Some(value),
            "bgColor" => self.bg_color = Some(value),
            "fontColor" => self.font_color = Some(value),
            "borderColor" => self.border_color = Some(value),
            "shadowing" => self.shadowing = Some(value),
            "shape" => self.shape = Some(value),
            "legendText" => self.legend_text = Some(value),
            "legendSprite" => self.legend_sprite = Some(value),
            _ => return false,
        }
        true
    }
    fn named(&mut self, attr: &C4Named) {
        if !self.set(&attr.key, attr.value.clone()) {
            extra(&mut self.extra, &attr.key, &attr.value)
        }
    }
    fn put(&mut self, key: &str, attr: Option<&C4Attr>) {
        match attr {
            Some(C4Attr::Text(s)) => {
                self.set(key, s.clone());
            }
            Some(C4Attr::Named(n)) => self.named(n),
            None => {}
        }
    }
    fn label(&mut self, attr: Option<&C4Attr>) {
        self.label_attr = None;
        match attr {
            Some(C4Attr::Named(n)) => {
                self.label.clear();
                self.label_attr = Some(n.clone());
                self.named(n)
            }
            Some(C4Attr::Text(s)) => self.label = s.clone(),
            None => self.label.clear(),
        }
    }
    fn text(&mut self, key: &str, attr: Option<&C4Attr>) {
        if let Some(attr) = attr {
            self.put(key, Some(attr))
        } else {
            let target = if key == "descr" {
                &mut self.descr
            } else {
                &mut self.techn
            };
            if target.is_none() {
                *target = Some(String::new())
            }
        }
    }
}
#[derive(Clone, Debug)]
pub struct C4Shape {
    pub element: C4Element,
    pub kind: String,
}
#[derive(Clone, Debug)]
pub struct C4Boundary {
    pub element: C4Element,
    pub node_type: Option<String>,
}
impl std::ops::Deref for C4Shape {
    type Target = C4Element;
    fn deref(&self) -> &Self::Target {
        &self.element
    }
}
impl std::ops::Deref for C4Boundary {
    type Target = C4Element;
    fn deref(&self) -> &Self::Target {
        &self.element
    }
}
fn number<S: Serializer>(v: &f64, s: S) -> Result<S::Ok, S::Error> {
    Value::Number(*v).serialize(s)
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct C4Rel {
    #[serde(rename = "type")]
    pub rel_type: String,
    pub from: String,
    pub to: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_attr: Option<C4Named>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_attr: Option<C4Named>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label_attr: Option<C4Named>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub techn: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub descr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sprite: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_color: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "option_number"
    )]
    pub offset_x: Option<f64>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "option_number"
    )]
    pub offset_y: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<Vec<(String, String)>>,
}
impl C4Rel {
    fn new(kind: &str) -> Self {
        Self {
            rel_type: kind.into(),
            from: String::new(),
            to: String::new(),
            label: String::new(),
            from_attr: None,
            to_attr: None,
            label_attr: None,
            techn: None,
            descr: None,
            sprite: None,
            tags: None,
            link: None,
            text_color: None,
            line_color: None,
            offset_x: None,
            offset_y: None,
            extra: None,
        }
    }
    fn set(&mut self, key: &str, value: String) -> bool {
        match key {
            "label" => self.label = value,
            "techn" => self.techn = Some(value),
            "descr" => self.descr = Some(value),
            "sprite" => self.sprite = Some(value),
            "tags" => self.tags = Some(value),
            "link" => self.link = Some(value),
            "textColor" => self.text_color = Some(value),
            "lineColor" => self.line_color = Some(value),
            _ => return false,
        }
        true
    }
    fn named(&mut self, attr: &C4Named) {
        match attr.key.as_str() {
            "offsetX" => self.offset_x = Some(parse_int(&attr.value)),
            "offsetY" => self.offset_y = Some(parse_int(&attr.value)),
            _ => {
                if !self.set(&attr.key, attr.value.clone()) {
                    extra(&mut self.extra, &attr.key, &attr.value)
                }
            }
        }
    }
    fn put(&mut self, key: &str, attr: Option<&C4Attr>) {
        match attr {
            Some(C4Attr::Text(s)) => {
                self.set(key, s.clone());
            }
            Some(C4Attr::Named(n)) => self.named(n),
            None => {}
        }
    }
    fn label(&mut self, attr: &C4Attr) {
        self.label_attr = None;
        match attr {
            C4Attr::Text(s) => self.label = s.clone(),
            C4Attr::Named(n) => {
                self.label.clear();
                self.label_attr = Some(n.clone());
                self.named(n)
            }
        }
    }
    fn text(&mut self, key: &str, attr: Option<&C4Attr>) {
        if let Some(attr) = attr {
            self.put(key, Some(attr))
        } else {
            let target = if key == "descr" {
                &mut self.descr
            } else {
                &mut self.techn
            };
            if target.is_none() {
                *target = Some(String::new())
            }
        }
    }
}
fn option_number<S: Serializer>(v: &Option<f64>, s: S) -> Result<S::Ok, S::Error> {
    match v {
        Some(v) => number(v, s),
        None => s.serialize_none(),
    }
}
fn extra(target: &mut Option<Vec<(String, String)>>, key: &str, value: &str) {
    let list = target.get_or_insert_with(Vec::new);
    if let Some((_, v)) = list.iter_mut().find(|(k, _)| k == key) {
        *v = value.into()
    } else {
        list.push((key.into(), value.into()))
    }
}
pub(super) fn parse_int(text: &str) -> f64 {
    let text = text.trim_start_matches(space);
    let (sign, text) = if let Some(s) = text.strip_prefix('-') {
        (-1., s)
    } else if let Some(s) = text.strip_prefix('+') {
        (1., s)
    } else {
        (1., text)
    };
    let hex = text.starts_with("0x") || text.starts_with("0X");
    let text = if hex { &text[2..] } else { text };
    let n = text
        .bytes()
        .take_while(|c| {
            if hex {
                c.is_ascii_hexdigit()
            } else {
                c.is_ascii_digit()
            }
        })
        .count();
    if n == 0 {
        return f64::NAN;
    }
    let text = &text[..n];
    let value = if !hex {
        text.parse::<f64>().unwrap_or(f64::INFINITY)
    } else {
        let mut leading = true;
        let (mut bits, mut mant, mut round, mut sticky) = (0usize, 0u64, false, false);
        for c in text.bytes() {
            let digit = (c as char).to_digit(16).unwrap();
            for k in (0..4).rev() {
                let bit = (digit >> k) & 1;
                if leading && bit == 0 {
                    continue;
                }
                leading = false;
                bits += 1;
                if bits <= 53 {
                    mant = (mant << 1) | bit as u64
                } else if bits == 54 {
                    round = bit != 0
                } else {
                    sticky |= bit != 0
                }
            }
        }
        if bits <= 53 {
            mant as f64
        } else if bits > 1024 {
            f64::INFINITY
        } else {
            if round && (sticky || mant & 1 != 0) {
                mant += 1
            }
            (mant as f64) * 2_f64.powi((bits - 53) as i32)
        }
    };
    sign * value
}
#[derive(Clone, Debug)]
pub struct C4Model {
    pub diagram_type: DiagramType,
    pub c4_type: Option<String>,
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
    pub shapes: Vec<C4Shape>,
    pub boundaries: Vec<C4Boundary>,
    pub rels: Vec<C4Rel>,
    pub shape_in_row: f64,
    pub boundary_in_row: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ElementRef {
    Shape(usize),
    Boundary(usize),
}
impl C4Model {
    pub fn element(&self, id: ElementRef) -> &C4Element {
        match id {
            ElementRef::Shape(i) => &self.shapes[i].element,
            ElementRef::Boundary(i) => &self.boundaries[i].element,
        }
    }
    pub(super) fn find(&self, alias: &str) -> Option<ElementRef> {
        self.shapes
            .iter()
            .position(|s| s.alias_attr.is_none() && s.alias == alias)
            .map(ElementRef::Shape)
            .or_else(|| {
                self.boundaries
                    .iter()
                    .enumerate()
                    .skip(1)
                    .find(|(_, b)| b.alias_attr.is_none() && b.alias == alias)
                    .map(|(i, _)| ElementRef::Boundary(i))
            })
    }
    pub(super) fn validate_projection(&self) -> Result<(), PeleError> {
        let mut depths = vec![None; self.boundaries.len()];
        let mut seen = vec![usize::MAX; self.boundaries.len()];
        let fail = || {
            PeleError::new(
                "C4 model exceeds safe JSON projection depth or contains cyclic boundary parents",
                ErrorCode::Limit,
            )
        };
        for i in 0..self.boundaries.len() {
            if depths[i].is_some() {
                continue;
            }
            let mut path = Vec::new();
            let mut at = Some(i);
            while let Some(p) = at {
                if depths[p].is_some() {
                    break;
                }
                if seen[p] == i {
                    return Err(fail());
                }
                seen[p] = i;
                path.push(p);
                at = self.boundaries[p].parent
            }
            for p in path.into_iter().rev() {
                let b = &self.boundaries[p];
                let own = if b.alias_attr.is_some()
                    || b.label_attr.is_some()
                    || b.extra.as_ref().is_some_and(|e| !e.is_empty())
                {
                    2usize
                } else {
                    1
                };
                let depth = own.max(b.parent.and_then(|p| depths[p]).map_or(0, |v| v + 1));
                if depth + 2 > 120 {
                    return Err(fail());
                }
                depths[p] = Some(depth)
            }
        }
        for s in &self.shapes {
            let own = if s.alias_attr.is_some()
                || s.label_attr.is_some()
                || s.extra.as_ref().is_some_and(|e| !e.is_empty())
            {
                2usize
            } else {
                1
            };
            let depth = own.max(s.parent.and_then(|p| depths[p]).map_or(0, |v| v + 1));
            if depth + 2 > 120 {
                return Err(fail());
            }
        }
        Ok(())
    }
}
pub(super) struct Db {
    pub model: C4Model,
    shape_index: HashMap<String, usize>,
    boundary_index: HashMap<String, usize>,
    rel_index: HashMap<(String, String), usize>,
    current: usize,
    stack: Vec<usize>,
}
impl Db {
    pub fn new(title: Option<&str>) -> Self {
        let mut global = C4Element::new();
        global.alias = "global".into();
        global.label = "global".into();
        global.element_type = Some("global".into());
        Self {
            model: C4Model {
                diagram_type: DiagramType::C4,
                c4_type: None,
                title: title.filter(|s| !s.is_empty()).map(str::to_string),
                acc_title: None,
                acc_descr: None,
                shapes: Vec::new(),
                boundaries: vec![C4Boundary {
                    element: global,
                    node_type: None,
                }],
                rels: Vec::new(),
                shape_in_row: 4.,
                boundary_in_row: 2.,
            },
            shape_index: HashMap::new(),
            boundary_index: HashMap::new(),
            rel_index: HashMap::new(),
            current: 0,
            stack: Vec::new(),
        }
    }
    pub fn acc_descr(&mut self, text: String) {
        let mut out = String::new();
        let mut after = false;
        for c in text.chars() {
            if after && space(c) {
                continue;
            }
            out.push(c);
            after = c == '\n'
        }
        self.model.acc_descr = Some(out)
    }
    fn new_element(alias: &C4Attr) -> C4Element {
        let mut el = C4Element::new();
        match alias {
            C4Attr::Text(s) => el.alias = s.clone(),
            C4Attr::Named(n) => {
                el.alias_attr = Some(n.clone());
                el.named(n)
            }
        }
        el
    }
    fn shape(&mut self, attrs: &[C4Attr]) -> usize {
        let i = match &attrs[0] {
            C4Attr::Text(s) => self.shape_index.get(s).copied(),
            _ => None,
        }
        .unwrap_or_else(|| {
            let i = self.model.shapes.len();
            self.model.shapes.push(C4Shape {
                element: Self::new_element(&attrs[0]),
                kind: String::new(),
            });
            if let C4Attr::Text(s) = &attrs[0] {
                self.shape_index.insert(s.clone(), i);
            }
            i
        });
        self.model.shapes[i].element.label(attrs.get(1));
        i
    }
    pub fn add_shape(&mut self, kind: &str, attrs: &[C4Attr], container: bool) {
        let i = self.shape(attrs);
        let shape = &mut self.model.shapes[i];
        let at = if container {
            shape.element.text("techn", attrs.get(2));
            shape.element.text("descr", attrs.get(3));
            4
        } else {
            shape.element.text("descr", attrs.get(2));
            3
        };
        shape.element.put("sprite", attrs.get(at));
        shape.element.put("tags", attrs.get(at + 1));
        shape.element.put("link", attrs.get(at + 2));
        shape.kind = kind.into();
        shape.element.parent = Some(self.current)
    }
    pub fn boundary(&mut self, attrs: &[C4Attr], fallback: &str, node_type: Option<&str>) {
        let i = match &attrs[0] {
            C4Attr::Text(s) => self.boundary_index.get(s).copied(),
            _ => None,
        }
        .unwrap_or_else(|| {
            let i = self.model.boundaries.len();
            self.model.boundaries.push(C4Boundary {
                element: Self::new_element(&attrs[0]),
                node_type: None,
            });
            if let C4Attr::Text(s) = &attrs[0] {
                self.boundary_index.insert(s.clone(), i);
            }
            i
        });
        let b = &mut self.model.boundaries[i];
        b.element.label(attrs.get(1));
        if attrs.get(2).is_none() {
            b.element.element_type = Some(fallback.into())
        } else {
            b.element.put("type", attrs.get(2))
        }
        if let Some(typ) = node_type {
            b.element.text("descr", attrs.get(3));
            b.element.put("sprite", attrs.get(4));
            b.element.put("tags", attrs.get(5));
            b.element.put("link", attrs.get(6));
            b.node_type = Some(typ.into())
        } else {
            b.element.put("tags", attrs.get(3));
            b.element.put("link", attrs.get(4))
        }
        b.element.parent = Some(self.current);
        self.stack.push(self.current);
        self.current = i
    }
    pub fn pop(&mut self) {
        self.current = self.stack.pop().unwrap_or(0)
    }
    pub fn rel(&mut self, typ: &str, attrs: &[C4Attr]) {
        if attrs.len() < 3 {
            return;
        }
        let known = match (&attrs[0], &attrs[1]) {
            (C4Attr::Text(a), C4Attr::Text(b)) => Some((a.clone(), b.clone())),
            _ => None,
        };
        let i = known
            .as_ref()
            .and_then(|k| self.rel_index.get(k))
            .copied()
            .unwrap_or_else(|| {
                let i = self.model.rels.len();
                let mut rel = C4Rel::new(typ);
                if let Some(key) = &known {
                    self.rel_index.insert(key.clone(), i);
                }
                match &attrs[0] {
                    C4Attr::Text(s) => rel.from = s.clone(),
                    C4Attr::Named(n) => rel.from_attr = Some(n.clone()),
                }
                match &attrs[1] {
                    C4Attr::Text(s) => rel.to = s.clone(),
                    C4Attr::Named(n) => rel.to_attr = Some(n.clone()),
                }
                self.model.rels.push(rel);
                i
            });
        let rel = &mut self.model.rels[i];
        rel.rel_type = typ.into();
        rel.label(&attrs[2]);
        rel.text("techn", attrs.get(3));
        rel.text("descr", attrs.get(4));
        rel.put("sprite", attrs.get(5));
        rel.put("tags", attrs.get(6));
        rel.put("link", attrs.get(7))
    }
    pub fn update_element(&mut self, attrs: &[C4Attr]) {
        let C4Attr::Text(alias) = &attrs[0] else {
            return;
        };
        let target = self
            .shape_index
            .get(alias)
            .copied()
            .map(ElementRef::Shape)
            .or_else(|| {
                self.boundary_index
                    .get(alias)
                    .copied()
                    .map(ElementRef::Boundary)
            });
        let Some(target) = target else { return };
        let el = match target {
            ElementRef::Shape(i) => &mut self.model.shapes[i].element,
            ElementRef::Boundary(i) => &mut self.model.boundaries[i].element,
        };
        for (i, key) in [
            "bgColor",
            "fontColor",
            "borderColor",
            "shadowing",
            "shape",
            "sprite",
            "techn",
            "legendText",
            "legendSprite",
        ]
        .iter()
        .enumerate()
        {
            el.put(key, attrs.get(i + 1))
        }
    }
    pub fn update_rel(&mut self, attrs: &[C4Attr]) {
        let (Some(C4Attr::Text(a)), Some(C4Attr::Text(b))) = (attrs.first(), attrs.get(1)) else {
            return;
        };
        let Some(&i) = self.rel_index.get(&(a.clone(), b.clone())) else {
            return;
        };
        let rel = &mut self.model.rels[i];
        rel.put("textColor", attrs.get(2));
        rel.put("lineColor", attrs.get(3));
        for (i, x) in [(4, true), (5, false)] {
            match attrs.get(i) {
                Some(C4Attr::Text(s)) => {
                    if x {
                        rel.offset_x = Some(parse_int(s))
                    } else {
                        rel.offset_y = Some(parse_int(s))
                    }
                }
                Some(C4Attr::Named(n)) => rel.named(n),
                None => {}
            }
        }
    }
    pub fn update_layout(&mut self, attrs: &[C4Attr]) {
        for (i, shapes) in [(0, true), (1, false)] {
            let value = attrs
                .get(i)
                .map(|a| {
                    parse_int(match a {
                        C4Attr::Text(s) => s,
                        C4Attr::Named(n) => &n.value,
                    })
                })
                .unwrap_or(f64::NAN);
            if value >= 1. {
                if shapes {
                    self.model.shape_in_row = value
                } else {
                    self.model.boundary_in_row = value
                }
            }
        }
    }
}
struct ElementView<'a> {
    model: &'a C4Model,
    element: &'a C4Element,
    kind: Option<&'a str>,
    node_type: Option<&'a str>,
    depth: usize,
}
impl Serialize for ElementView<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if self.depth > 120 {
            return Err(serde::ser::Error::custom(
                "C4 parent JSON projection exceeds safe depth",
            ));
        }
        let el = self.element;
        let mut map = s.serialize_map(None)?;
        map.serialize_entry("alias", &el.alias)?;
        map.serialize_entry("label", &el.label)?;
        if let Some(attr) = &el.alias_attr {
            map.serialize_entry("aliasAttr", attr)?
        }
        if let Some(attr) = &el.label_attr {
            map.serialize_entry("labelAttr", attr)?
        }
        if let Some(p) = el.parent {
            let boundary = &self.model.boundaries[p];
            map.serialize_entry(
                "parent",
                &ElementView {
                    model: self.model,
                    element: &boundary.element,
                    kind: None,
                    node_type: boundary.node_type.as_deref(),
                    depth: self.depth + 1,
                },
            )?
        }
        if let Some(v) = &el.element_type {
            map.serialize_entry("type", v)?
        }
        if let Some(v) = &el.descr {
            map.serialize_entry("descr", v)?
        }
        if let Some(v) = &el.techn {
            map.serialize_entry("techn", v)?
        }
        if let Some(v) = &el.sprite {
            map.serialize_entry("sprite", v)?
        }
        if let Some(v) = &el.tags {
            map.serialize_entry("tags", v)?
        }
        if let Some(v) = &el.link {
            map.serialize_entry("link", v)?
        }
        if let Some(v) = &el.bg_color {
            map.serialize_entry("bgColor", v)?
        }
        if let Some(v) = &el.font_color {
            map.serialize_entry("fontColor", v)?
        }
        if let Some(v) = &el.border_color {
            map.serialize_entry("borderColor", v)?
        }
        if let Some(v) = &el.shadowing {
            map.serialize_entry("shadowing", v)?
        }
        if let Some(v) = &el.shape {
            map.serialize_entry("shape", v)?
        }
        if let Some(v) = &el.legend_text {
            map.serialize_entry("legendText", v)?
        }
        if let Some(v) = &el.legend_sprite {
            map.serialize_entry("legendSprite", v)?
        }
        if let Some(v) = &el.extra {
            map.serialize_entry("extra", v)?
        }
        if let Some(v) = self.kind {
            map.serialize_entry("kind", v)?
        }
        if let Some(v) = self.node_type {
            map.serialize_entry("nodeType", v)?
        }
        map.end()
    }
}
struct Elements<'a> {
    model: &'a C4Model,
    shapes: bool,
}
impl Serialize for Elements<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let n = if self.shapes {
            self.model.shapes.len()
        } else {
            self.model.boundaries.len()
        };
        let mut list = s.serialize_seq(Some(n))?;
        for i in 0..n {
            let (el, kind, node_type) = if self.shapes {
                let node = &self.model.shapes[i];
                (&node.element, Some(node.kind.as_str()), None)
            } else {
                let node = &self.model.boundaries[i];
                (&node.element, None, node.node_type.as_deref())
            };
            list.serialize_element(&ElementView {
                model: self.model,
                element: el,
                kind,
                node_type,
                depth: 0,
            })?
        }
        list.end()
    }
}
impl Serialize for C4Model {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut map = s.serialize_map(None)?;
        map.serialize_entry("type", &self.diagram_type)?;
        if let Some(v) = &self.c4_type {
            map.serialize_entry("c4Type", v)?
        }
        if let Some(v) = &self.title {
            map.serialize_entry("title", v)?
        }
        if let Some(v) = &self.acc_title {
            map.serialize_entry("accTitle", v)?
        }
        if let Some(v) = &self.acc_descr {
            map.serialize_entry("accDescr", v)?
        }
        map.serialize_entry(
            "shapes",
            &Elements {
                model: self,
                shapes: true,
            },
        )?;
        map.serialize_entry(
            "boundaries",
            &Elements {
                model: self,
                shapes: false,
            },
        )?;
        map.serialize_entry("rels", &self.rels)?;
        map.serialize_entry("shapeInRow", &Value::Number(self.shape_in_row))?;
        map.serialize_entry("boundaryInRow", &Value::Number(self.boundary_in_row))?;
        map.end()
    }
}
