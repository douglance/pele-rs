//! Pele er/{types,db}.ts, 458322a5 (MIT, kepano).
use serde::Serialize;
use std::collections::HashSet;
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErAttribute {
    pub r#type: String,
    pub name: String,
    pub keys: Vec<String>,
    pub comment: String,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErEntity {
    pub id: String,
    pub label: String,
    pub alias: String,
    pub attributes: Vec<ErAttribute>,
    pub css_classes: String,
    pub css_styles: Vec<String>,
}
#[derive(Clone, Copy, Debug, Serialize)]
#[allow(non_camel_case_types)]
pub enum Cardinality {
    ZERO_OR_ONE,
    ZERO_OR_MORE,
    ONE_OR_MORE,
    ONLY_ONE,
    MD_PARENT,
}
impl Cardinality {
    pub(super) fn marker(self) -> &'static str {
        match self {
            Self::ZERO_OR_ONE => "zero_or_one",
            Self::ZERO_OR_MORE => "zero_or_more",
            Self::ONE_OR_MORE => "one_or_more",
            Self::ONLY_ONE => "only_one",
            Self::MD_PARENT => "md_parent",
        }
    }
}
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum Identification {
    NON_IDENTIFYING,
    IDENTIFYING,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelSpec {
    pub card_a: Cardinality,
    pub rel_type: Identification,
    pub card_b: Cardinality,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErRelationship {
    pub entity_a: String,
    pub role_a: String,
    pub entity_b: String,
    pub rel_spec: RelSpec,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErClassDef {
    pub id: String,
    pub styles: Vec<String>,
    pub text_styles: Vec<String>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErSubgraph {
    pub id: String,
    pub nodes: Vec<String>,
    pub title: String,
    pub classes: Vec<String>,
    pub css_styles: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dir: Option<String>,
    pub label_type: String,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErModel {
    #[serde(rename = "type")]
    pub diagram_type: crate::compat::DiagramType,
    pub direction: String,
    pub entities: Vec<(String, ErEntity)>,
    pub relationships: Vec<ErRelationship>,
    pub classes: Vec<(String, ErClassDef)>,
    pub subgraphs: Vec<ErSubgraph>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acc_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acc_descr: Option<String>,
    #[serde(skip)]
    claimed: HashSet<String>,
}
pub(super) enum DocItem {
    Text(String),
    Dir(String),
}
impl ErModel {
    pub(super) fn new(title: Option<&str>) -> Self {
        Self {
            diagram_type: crate::compat::DiagramType::Er,
            direction: "TB".into(),
            entities: vec![],
            relationships: vec![],
            classes: vec![],
            subgraphs: vec![],
            title: title.filter(|s| !s.is_empty()).map(str::to_owned),
            acc_title: None,
            acc_descr: None,
            claimed: HashSet::new(),
        }
    }
    pub(super) fn add_entity(&mut self, name: &str, alias: &str) -> &mut ErEntity {
        let i = if let Some(i) = self.entities.iter().position(|(k, _)| k == name) {
            i
        } else {
            let i = self.entities.len();
            self.entities.push((
                name.into(),
                ErEntity {
                    id: format!("entity-{name}-{i}"),
                    label: name.into(),
                    alias: alias.into(),
                    attributes: vec![],
                    css_classes: "default".into(),
                    css_styles: vec![],
                },
            ));
            i
        };
        let e = &mut self.entities[i].1;
        if e.alias.is_empty() && !alias.is_empty() {
            e.alias = alias.into()
        }
        e
    }
    pub(super) fn relationship(&mut self, a: &str, role: String, b: &str, rel_spec: RelSpec) {
        let entity_a = if self.subgraphs.iter().any(|s| s.id == a) {
            a.into()
        } else {
            self.add_entity(a, "").id.clone()
        };
        let entity_b = if self.subgraphs.iter().any(|s| s.id == b) {
            b.into()
        } else {
            self.add_entity(b, "").id.clone()
        };
        self.relationships.push(ErRelationship {
            entity_a,
            role_a: role,
            entity_b,
            rel_spec,
        })
    }
    pub(super) fn styles(&mut self, ids: &[String], styles: &[String]) {
        for id in ids {
            if let Some((_, e)) = self.entities.iter_mut().find(|(k, _)| k == id) {
                e.css_styles.extend_from_slice(styles)
            }
            if let Some(s) = self.subgraphs.iter_mut().rev().find(|s| s.id == *id) {
                s.css_styles.extend_from_slice(styles)
            }
        }
    }
    pub(super) fn add_class(&mut self, ids: &[String], styles: &[String]) {
        for id in ids {
            let i = if let Some(i) = self.classes.iter().position(|(k, _)| k == id) {
                i
            } else {
                self.classes.push((
                    id.clone(),
                    ErClassDef {
                        id: id.clone(),
                        styles: vec![],
                        text_styles: vec![],
                    },
                ));
                self.classes.len() - 1
            };
            let def = &mut self.classes[i].1;
            for style in styles {
                if style.contains("color") {
                    def.text_styles.push(style.replacen("fill", "bgFill", 1))
                }
                def.styles.push(style.clone())
            }
        }
    }
    pub(super) fn set_class(&mut self, ids: &[String], names: &[String]) {
        for id in ids {
            if let Some((_, e)) = self.entities.iter_mut().find(|(k, _)| k == id) {
                for n in names {
                    e.css_classes.push(' ');
                    e.css_classes.push_str(n)
                }
            }
            if let Some(s) = self.subgraphs.iter_mut().rev().find(|s| s.id == *id) {
                s.classes.extend_from_slice(names)
            }
        }
    }
    pub(super) fn subgraph(&mut self, id: String, title: String, list: Vec<DocItem>) -> String {
        let mut seen = HashSet::new();
        let mut nodes = vec![];
        let mut dir = None;
        for item in list {
            match item {
                DocItem::Dir(d) => dir = Some(d),
                DocItem::Text(s) => {
                    let trimmed = s.trim_matches(crate::compat::source::space);
                    if trimmed.is_empty() || !seen.insert(trimmed.to_owned()) {
                        continue;
                    }
                    if !self.claimed.contains(&s) {
                        nodes.push(s)
                    }
                }
            }
        }
        self.claimed.extend(nodes.iter().cloned());
        let id = id.trim_matches(crate::compat::source::space).to_owned();
        self.subgraphs.push(ErSubgraph {
            id: id.clone(),
            title: title.trim_matches(crate::compat::source::space).to_owned(),
            nodes,
            classes: vec![],
            css_styles: vec![],
            dir,
            label_type: "markdown".into(),
        });
        id
    }
}
