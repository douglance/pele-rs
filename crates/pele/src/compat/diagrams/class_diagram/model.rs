//! Pele class/{types,db,members}.ts, pinned 458322a5 (MIT, kepano).
use super::super::common::generics::parse_generic_types;
use crate::compat::{
    source::{decode_entities, space, url::sanitize_url},
    PeleError,
};
use serde::{
    ser::{SerializeMap, SerializeSeq},
    Serialize,
};
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassMember {
    pub id: String,
    pub member_type: String,
    pub visibility: String,
    pub classifier: String,
    pub parameters: String,
    pub return_type: String,
}
impl ClassMember {
    pub fn new(input: &str, method: bool) -> Self {
        let mut m = Self {
            id: String::new(),
            member_type: if method { "method" } else { "attribute" }.into(),
            visibility: String::new(),
            classifier: String::new(),
            parameters: String::new(),
            return_type: String::new(),
        };
        let visible = |c: char| matches!(c, '#' | '+' | '~' | '-');
        let mut parsed = false;
        if method {
            let mut start = 0;
            for segment in input.split_inclusive(['\n', '\r', '\u{2028}', '\u{2029}']) {
                let line = segment.trim_end_matches(['\n', '\r', '\u{2028}', '\u{2029}']);
                if let Some(close) = line.rfind(')') {
                    if let Some(open) = line[..close].rfind('(').filter(|p| *p > 0) {
                        let first = line.chars().next().unwrap();
                        let v = visible(first) && open > first.len_utf8();
                        if v {
                            m.visibility = first.to_string()
                        }
                        m.id = line[if v { first.len_utf8() } else { 0 }..open].into();
                        m.parameters = line[open + 1..close].trim_matches(space).into();
                        let rest = &input[start + close + 1..];
                        let next = rest.chars().next();
                        let skip = next.is_some_and(|c| matches!(c, '$' | '*') || space(c));
                        if skip {
                            m.classifier = next.unwrap().to_string().trim_matches(space).into()
                        }
                        let tail = &rest[if skip { next.unwrap().len_utf8() } else { 0 }..];
                        m.return_type = tail
                            .split(['\n', '\r', '\u{2028}', '\u{2029}'])
                            .next()
                            .unwrap()
                            .trim_matches(space)
                            .into();
                        if m.classifier.is_empty() && m.return_type.ends_with(['$', '*']) {
                            m.classifier = m.return_type.pop().unwrap().to_string()
                        }
                        parsed = true;
                        break;
                    }
                }
                start += segment.len();
            }
        }
        if !parsed {
            m.member_type = "attribute".into();
            let first = input.chars().next();
            if first.is_some_and(visible) {
                m.visibility = first.unwrap().to_string()
            }
            if input.ends_with(['$', '*']) {
                m.classifier = input.chars().last().unwrap().to_string()
            }
            m.id = input[m.visibility.len()..input.len() - m.classifier.len()].into();
        }
        m.id = if m.id.starts_with(' ') {
            format!(" {}", m.id.trim_matches(space))
        } else {
            m.id.trim_matches(space).into()
        };
        m
    }
    pub fn display(&self) -> String {
        let mut s = format!("{}{}", self.visibility, parse_generic_types(&self.id));
        if self.member_type == "method" {
            s.push_str(&format!(
                "({})",
                parse_generic_types(self.parameters.trim_matches(space))
            ));
            if !self.return_type.is_empty() {
                s.push_str(&format!(" : {}", parse_generic_types(&self.return_type)))
            }
        }
        s.trim_matches(space).into()
    }
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassNode {
    pub id: String,
    #[serde(rename = "type")]
    pub generic: String,
    pub label: String,
    pub css_classes: String,
    pub methods: Vec<ClassMember>,
    pub members: Vec<ClassMember>,
    pub annotations: Vec<String>,
    pub styles: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub have_callback: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tooltip: Option<String>,
}
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(untagged)]
pub enum RelationEnd {
    Number(u8),
    None(String),
}
impl RelationEnd {
    pub fn none() -> Self {
        Self::None("none".into())
    }
    pub fn is(&self, n: u8) -> bool {
        *self == Self::Number(n)
    }
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationType {
    pub type1: RelationEnd,
    pub type2: RelationEnd,
    pub line_type: u8,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassRelation {
    pub id1: String,
    pub id2: String,
    pub relation_title1: String,
    pub relation_title2: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub relation: RelationType,
}
#[derive(Clone, Debug, Serialize)]
pub struct ClassNote {
    pub id: String,
    #[serde(rename = "class", skip_serializing_if = "Option::is_none")]
    pub class_name: Option<String>,
    pub text: String,
    pub index: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassInterface {
    pub id: String,
    pub label: String,
    pub class_id: String,
}
#[derive(Clone, Debug)]
pub struct NamespaceNode {
    pub id: String,
    pub label: String,
    pub classes: Vec<String>,
    pub notes: Vec<String>,
    pub children: Vec<String>,
    pub parent: Option<String>,
    pub explicit: bool,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StyleClass {
    pub id: String,
    pub styles: Vec<String>,
    pub text_styles: Vec<String>,
}
#[derive(Clone, Debug)]
pub struct ClassModel {
    class_index: std::collections::HashMap<String, usize>,
    namespace_index: std::collections::HashMap<String, usize>,
    tags: std::collections::HashSet<(String, String)>,
    pub direction: String,
    pub classes: Vec<(String, ClassNode)>,
    pub relations: Vec<ClassRelation>,
    pub notes: Vec<(String, ClassNote)>,
    pub namespaces: Vec<(String, NamespaceNode)>,
    pub interfaces: Vec<ClassInterface>,
    pub style_classes: Vec<(String, StyleClass)>,
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
    pub namespace_stack: Vec<String>,
    pub with_css_class: Vec<(String, Vec<String>)>,
}
pub fn split_name(id: &str) -> (&str, &str) {
    if let Some(at) = id.find('~').filter(|i| *i > 0) {
        let tail = &id[at + 1..];
        (&id[..at], tail.split('~').next().unwrap())
    } else {
        (id, "")
    }
}
fn lookup(id: &str) -> String {
    split_name(&if id.starts_with(|c: char| c.is_ascii_digit()) {
        format!("classId-{id}")
    } else {
        id.into()
    })
    .0
    .into()
}
impl ClassModel {
    pub fn new(title: Option<&str>) -> Self {
        Self {
            class_index: Default::default(),
            namespace_index: Default::default(),
            tags: Default::default(),
            direction: "TB".into(),
            classes: vec![],
            relations: vec![],
            notes: vec![],
            namespaces: vec![],
            interfaces: vec![],
            style_classes: vec![],
            title: title.map(str::to_string),
            acc_title: None,
            acc_descr: None,
            namespace_stack: vec![],
            with_css_class: vec![],
        }
    }
    pub fn class(&self, id: &str) -> Option<&ClassNode> {
        self.class_index.get(id).map(|&i| &self.classes[i].1)
    }
    fn class_mut(&mut self, id: &str) -> Option<&mut ClassNode> {
        self.class_index
            .get(id)
            .copied()
            .map(|i| &mut self.classes[i].1)
    }
    pub fn ns(&self, id: &str) -> Option<&NamespaceNode> {
        self.namespace_index.get(id).map(|&i| &self.namespaces[i].1)
    }
    fn ns_mut(&mut self, id: &str) -> Option<&mut NamespaceNode> {
        self.namespace_index
            .get(id)
            .copied()
            .map(|i| &mut self.namespaces[i].1)
    }
    pub fn add_class(&mut self, id: &str) {
        let (name, generic) = split_name(id);
        if self.class(name).is_some() {
            return;
        }
        self.class_index.insert(name.into(), self.classes.len());
        self.classes.push((
            name.into(),
            ClassNode {
                id: name.into(),
                generic: generic.into(),
                label: name.into(),
                css_classes: "default".into(),
                methods: vec![],
                members: vec![],
                annotations: vec![],
                styles: vec![],
                parent: None,
                link: None,
                link_target: None,
                have_callback: None,
                tooltip: None,
            },
        ));
        self.tag(name, "default")
    }
    fn tag(&mut self, id: &str, css: &str) {
        if !self.tags.insert((id.into(), css.into())) {
            return;
        }
        if let Some((_, v)) = self.with_css_class.iter_mut().find(|(k, _)| k == css) {
            v.push(id.into())
        } else {
            self.with_css_class.push((css.into(), vec![id.into()]))
        }
    }
    fn ensure(&mut self, id: &str) -> &mut ClassNode {
        self.add_class(id);
        self.class_mut(split_name(id).0).unwrap()
    }
    pub fn set_label(&mut self, id: &str, label: String) {
        self.ensure(id).label = label
    }
    pub fn add_annotation(&mut self, id: &str, annotation: String) {
        self.ensure(id).annotations.push(annotation)
    }
    pub fn add_member(&mut self, id: &str, member: &str) {
        let n = self.ensure(id);
        let text = member.trim_matches(space);
        if text.starts_with("<<") && text.ends_with(">>") {
            n.annotations.push(text[2..text.len() - 2].into())
        } else if text.find(')').is_some_and(|i| i > 0) {
            let m = ClassMember::new(text, true);
            if m.member_type == "method" {
                n.methods.push(m)
            } else {
                n.members.push(m)
            }
        } else if !text.is_empty() {
            n.members.push(ClassMember::new(text, false))
        }
    }
    pub fn add_note(&mut self, text: String, class_name: Option<String>) -> String {
        let index = self.notes.len();
        let id = format!("note{index}");
        self.notes.push((
            id.clone(),
            ClassNote {
                id: id.clone(),
                class_name,
                text,
                index,
                parent: None,
            },
        ));
        id
    }
    fn add_interface(&mut self, label: String, class_id: String) -> String {
        let id = format!("interface{}", self.interfaces.len());
        self.interfaces.push(ClassInterface {
            id: id.clone(),
            label,
            class_id,
        });
        id
    }
    pub fn add_relation(&mut self, mut r: ClassRelation) {
        if r.relation.type1.is(4) && r.relation.type2 == RelationEnd::none() {
            self.add_class(&r.id2);
            r.id1 = self.add_interface(r.id1, r.id2.clone())
        } else if r.relation.type2.is(4) && r.relation.type1 == RelationEnd::none() {
            self.add_class(&r.id1);
            r.id2 = self.add_interface(r.id2, r.id1.clone())
        } else {
            self.add_class(&r.id1);
            self.add_class(&r.id2)
        }
        r.id1 = split_name(&r.id1).0.into();
        r.id2 = split_name(&r.id2).0.into();
        r.relation_title1 = r.relation_title1.trim_matches(space).into();
        r.relation_title2 = r.relation_title2.trim_matches(space).into();
        self.relations.push(r)
    }
    pub fn css_class(&mut self, ids: &str, name: &str) {
        for id in ids.split(',') {
            let id = lookup(id);
            if let Some(n) = self.class_mut(&id) {
                n.css_classes.push(' ');
                n.css_classes.push_str(name);
                for word in name.split(' ') {
                    self.tag(&id, word)
                }
            }
        }
    }
    pub fn define_class(&mut self, ids: Vec<String>, styles: Vec<String>) {
        for id in ids {
            let at = if let Some(at) = self.style_classes.iter().position(|(k, _)| k == &id) {
                at
            } else {
                self.style_classes.push((
                    id.clone(),
                    StyleClass {
                        id: id.clone(),
                        styles: vec![],
                        text_styles: vec![],
                    },
                ));
                self.style_classes.len() - 1
            };
            let def = &mut self.style_classes[at].1;
            for s in &styles {
                if s.contains("color") {
                    def.text_styles.push(s.replacen("fill", "bgFill", 1))
                }
                def.styles.push(s.clone())
            }
            if let Some((_, tagged)) = self.with_css_class.iter().find(|(k, _)| k == &id) {
                let tagged = tagged.clone();
                for node in tagged {
                    if let Some(n) = self.class_mut(&node) {
                        for s in &styles {
                            n.styles.extend(s.split(',').map(str::to_string))
                        }
                    }
                }
            }
        }
    }
    pub fn css_style(&mut self, id: &str, styles: Vec<String>) {
        if let Some(n) = self.class_mut(id) {
            for s in styles {
                n.styles.extend(s.split(',').map(str::to_string))
            }
        }
    }
    pub fn tooltip(&mut self, ids: &str, text: String) {
        for id in ids.split(',') {
            if let Some(n) = self.class_mut(split_name(id).0) {
                n.tooltip = Some(text.clone())
            }
        }
    }
    pub fn link(&mut self, ids: &str, link: &str, target: Option<String>) -> Result<(), PeleError> {
        let decoded = decode_entities(link)?;
        let url = decoded.trim_matches(space);
        for id in ids.split(',') {
            if let Some(n) = self.class_mut(&lookup(id)) {
                n.link = if url.is_empty() {
                    None
                } else {
                    Some(sanitize_url(url))
                };
                n.link_target = Some(target.clone().unwrap_or("_blank".into()))
            }
        }
        self.css_class(ids, "clickable");
        Ok(())
    }
    pub fn click(&mut self, ids: &str) {
        for id in ids.split(',') {
            if let Some(n) = self.class_mut(split_name(id).0) {
                n.have_callback = Some(true)
            }
        }
        self.css_class(ids, "clickable")
    }
    pub fn add_namespace(&mut self, id: &str, label: Option<String>) -> String {
        let prefix = self.namespace_stack.last().cloned();
        let qualified = prefix
            .as_ref()
            .map_or_else(|| id.into(), |p| format!("{p}.{id}"));
        self.namespace_stack.push(qualified.clone());
        if let Some(n) = self.ns_mut(&qualified) {
            n.explicit = true;
            if let Some(label) = label.filter(|s| !s.is_empty()) {
                n.label = label
            }
            return qualified;
        }
        let mut parent = prefix.clone();
        let mut from = prefix.map_or(0, |p| p.len() + 1);
        loop {
            let dot = qualified[from..].find('.').map(|p| p + from);
            let leaf = dot.is_none();
            let end = dot.unwrap_or(qualified.len());
            let current = &qualified[..end];
            if self.ns(current).is_none() {
                self.namespace_index
                    .insert(current.into(), self.namespaces.len());
                self.namespaces.push((
                    current.into(),
                    NamespaceNode {
                        id: current.into(),
                        label: if leaf {
                            label
                                .clone()
                                .filter(|s| !s.is_empty())
                                .unwrap_or_else(|| qualified[from..end].into())
                        } else {
                            qualified[from..end].into()
                        },
                        classes: vec![],
                        notes: vec![],
                        children: vec![],
                        parent: parent.clone(),
                        explicit: leaf,
                    },
                ))
            } else if leaf {
                self.ns_mut(current).unwrap().explicit = true
            }
            if let Some(parent_id) = &parent {
                if let Some(n) = self.ns_mut(parent_id) {
                    if !n.children.iter().any(|s| s == current) {
                        n.children.push(current.into())
                    }
                }
                if self.ns(current).unwrap().parent.is_none() {
                    self.ns_mut(current).unwrap().parent = parent.clone()
                }
            }
            if leaf {
                return qualified;
            }
            parent = Some(current.into());
            from = end + 1;
        }
    }
    pub fn add_to_namespace(&mut self, id: &str, classes: Vec<String>, notes: Vec<String>) {
        for name in classes {
            let name = split_name(&name).0.to_string();
            if let Some(n) = self.class_mut(&name) {
                n.parent = Some(id.into());
                let ns = self.ns_mut(id).unwrap();
                if !ns.classes.contains(&name) {
                    ns.classes.push(name)
                }
            }
        }
        for name in notes {
            if let Some((_, n)) = self.notes.iter_mut().find(|(k, _)| k == &name) {
                n.parent = Some(id.into());
                let ns = self.ns_mut(id).unwrap();
                if !ns.notes.contains(&name) {
                    ns.notes.push(name)
                }
            }
        }
    }
}
struct Namespace<'a> {
    model: &'a ClassModel,
    node: &'a NamespaceNode,
}
impl Serialize for Namespace<'_> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(None)?;
        let n = self.node;
        m.serialize_entry("id", &n.id)?;
        m.serialize_entry("label", &n.label)?;
        m.serialize_entry(
            "classes",
            &n.classes
                .iter()
                .filter_map(|k| self.model.class(k).map(|n| (k, n)))
                .collect::<Vec<_>>(),
        )?;
        m.serialize_entry(
            "notes",
            &n.notes
                .iter()
                .filter_map(|k| {
                    self.model
                        .notes
                        .iter()
                        .find(|(id, _)| id == k)
                        .map(|(_, n)| (k, n))
                })
                .collect::<Vec<_>>(),
        )?;
        m.serialize_entry(
            "children",
            &Namespaces {
                model: self.model,
                ids: Some(&n.children),
            },
        )?;
        if let Some(p) = &n.parent {
            m.serialize_entry("parent", p)?
        }
        m.serialize_entry("explicit", &n.explicit)?;
        m.end()
    }
}
struct Namespaces<'a> {
    model: &'a ClassModel,
    ids: Option<&'a [String]>,
}
impl Serialize for Namespaces<'_> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut seq = s.serialize_seq(None)?;
        if let Some(ids) = self.ids {
            for id in ids {
                if let Some(node) = self.model.ns(id) {
                    seq.serialize_element(&(
                        id,
                        Namespace {
                            model: self.model,
                            node,
                        },
                    ))?
                }
            }
        } else {
            for (id, node) in &self.model.namespaces {
                seq.serialize_element(&(
                    id,
                    Namespace {
                        model: self.model,
                        node,
                    },
                ))?
            }
        }
        seq.end()
    }
}
impl Serialize for ClassModel {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(None)?;
        m.serialize_entry("type", "class")?;
        m.serialize_entry("lineType", &serde_json::json!({"LINE":0,"DOTTED_LINE":1}))?;
        m.serialize_entry("relationType",&serde_json::json!({"AGGREGATION":0,"EXTENSION":1,"COMPOSITION":2,"DEPENDENCY":3,"LOLLIPOP":4}))?;
        m.serialize_entry("direction", &self.direction)?;
        m.serialize_entry("classes", &self.classes)?;
        m.serialize_entry("relations", &self.relations)?;
        m.serialize_entry("notes", &self.notes)?;
        m.serialize_entry(
            "namespaces",
            &Namespaces {
                model: self,
                ids: None,
            },
        )?;
        m.serialize_entry("interfaces", &self.interfaces)?;
        m.serialize_entry("styleClasses", &self.style_classes)?;
        if let Some(t) = &self.title {
            m.serialize_entry("title", t)?
        }
        if let Some(t) = &self.acc_title {
            m.serialize_entry("accTitle", t)?
        }
        if let Some(t) = &self.acc_descr {
            m.serialize_entry("accDescr", t)?
        }
        m.serialize_entry("namespaceStack", &self.namespace_stack)?;
        m.serialize_entry(
            "withCssClass",
            &self
                .with_css_class
                .iter()
                .map(|(k, _)| (k, serde_json::json!({})))
                .collect::<Vec<_>>(),
        )?;
        m.end()
    }
}
