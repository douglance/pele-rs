//! Pele requirement/{types,db}.ts, 458322a5 (MIT, kepano).
use crate::compat::{DiagramType, ErrorCode, PeleError};
use serde::Serialize;
use std::collections::HashMap;
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Requirement {
    pub name: String,
    #[serde(rename = "type")]
    pub requirement_type: String,
    pub requirement_id: String,
    pub text: String,
    pub risk: String,
    pub verify_method: String,
    pub css_styles: Vec<String>,
    pub classes: Vec<String>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequirementElement {
    pub name: String,
    #[serde(rename = "type")]
    pub element_type: String,
    pub doc_ref: String,
    pub css_styles: Vec<String>,
    pub classes: Vec<String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct Relation {
    #[serde(rename = "type")]
    pub relation_type: String,
    pub src: String,
    pub dst: String,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequirementClass {
    pub id: String,
    pub styles: Vec<String>,
    pub text_styles: Vec<String>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequirementModel {
    #[serde(rename = "type")]
    pub diagram_type: DiagramType,
    pub direction: String,
    pub requirements: Vec<(String, Requirement)>,
    pub elements: Vec<(String, RequirementElement)>,
    pub relations: Vec<Relation>,
    pub classes: Vec<(String, RequirementClass)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acc_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acc_descr: Option<String>,
    /// Source builder state, omitted from the stable immutable model projection.
    /// Typed reconstruction cannot recover erased JavaScript builder history.
    #[serde(skip)]
    pub public_provenance: RequirementProvenance,
}
#[derive(Clone, Debug, Default)]
pub struct RequirementProvenance {
    pub pending: [String; 6],
    pub style_copies: usize,
}
pub(super) struct Db {
    pub model: RequirementModel,
    pub new_id: String,
    pub new_text: String,
    pub new_risk: String,
    pub new_verify: String,
    pub new_type: String,
    pub new_doc_ref: String,
    copies: usize,
    req: HashMap<String, usize>,
    els: HashMap<String, usize>,
    cls: HashMap<String, usize>,
}
impl Db {
    pub fn new(title: Option<&str>) -> Self {
        Self {
            model: RequirementModel {
                diagram_type: DiagramType::Requirement,
                direction: "TB".into(),
                requirements: vec![],
                elements: vec![],
                relations: vec![],
                classes: vec![],
                title: title.map(str::to_owned),
                acc_title: None,
                acc_descr: None,
                public_provenance: RequirementProvenance::default(),
            },
            new_id: String::new(),
            new_text: String::new(),
            new_risk: String::new(),
            new_verify: String::new(),
            new_type: String::new(),
            new_doc_ref: String::new(),
            copies: 0,
            req: HashMap::new(),
            els: HashMap::new(),
            cls: HashMap::new(),
        }
    }
    pub fn finish(mut self) -> RequirementModel {
        self.model.public_provenance = RequirementProvenance {
            pending: [
                self.new_id,
                self.new_text,
                self.new_risk,
                self.new_verify,
                self.new_type,
                self.new_doc_ref,
            ],
            style_copies: self.copies,
        };
        self.model
    }
    fn copy(&mut self, n: usize) -> Result<(), PeleError> {
        self.copies += n;
        if self.copies > 1000000 {
            let mut e = PeleError::new(
                "Requirement diagram applies too many style declarations.",
                ErrorCode::Limit,
            );
            e.diagram_type = Some("requirement".into());
            return Err(e);
        }
        Ok(())
    }
    pub fn requirement(&mut self, name: String, kind: String) {
        if !self.req.contains_key(&name) {
            self.req.insert(name.clone(), self.model.requirements.len());
            self.model.requirements.push((
                name.clone(),
                Requirement {
                    name,
                    requirement_type: kind,
                    requirement_id: std::mem::take(&mut self.new_id),
                    text: std::mem::take(&mut self.new_text),
                    risk: std::mem::take(&mut self.new_risk),
                    verify_method: std::mem::take(&mut self.new_verify),
                    css_styles: vec![],
                    classes: vec!["default".into()],
                },
            ));
        }
        self.new_id.clear();
        self.new_text.clear();
        self.new_risk.clear();
        self.new_verify.clear();
    }
    pub fn element(&mut self, name: String) {
        if !self.els.contains_key(&name) {
            self.els.insert(name.clone(), self.model.elements.len());
            self.model.elements.push((
                name.clone(),
                RequirementElement {
                    name,
                    element_type: std::mem::take(&mut self.new_type),
                    doc_ref: std::mem::take(&mut self.new_doc_ref),
                    css_styles: vec![],
                    classes: vec!["default".into()],
                },
            ));
        }
        self.new_type.clear();
        self.new_doc_ref.clear();
    }
    fn node(&self, id: &str) -> Option<(bool, usize)> {
        self.req
            .get(id)
            .map(|i| (true, *i))
            .or_else(|| self.els.get(id).map(|i| (false, *i)))
    }
    fn styles(&mut self, node: (bool, usize)) -> &mut Vec<String> {
        if node.0 {
            &mut self.model.requirements[node.1].1.css_styles
        } else {
            &mut self.model.elements[node.1].1.css_styles
        }
    }
    fn names(&mut self, node: (bool, usize)) -> &mut Vec<String> {
        if node.0 {
            &mut self.model.requirements[node.1].1.classes
        } else {
            &mut self.model.elements[node.1].1.classes
        }
    }
    pub fn set_style(&mut self, ids: Vec<String>, styles: Vec<String>) -> Result<(), PeleError> {
        for id in ids {
            let Some(node) = self.node(&id) else {
                return Ok(());
            };
            for s in &styles {
                let parts: Vec<_> = s.split(',').map(str::to_owned).collect();
                self.copy(parts.len())?;
                self.styles(node).extend(parts)
            }
        }
        Ok(())
    }
    pub fn set_class(&mut self, ids: Vec<String>, names: Vec<String>) -> Result<(), PeleError> {
        for id in ids {
            let Some(node) = self.node(&id) else { continue };
            for name in &names {
                self.names(node).push(name.clone());
                if let Some(i) = self.cls.get(name) {
                    let styles = self.model.classes[*i].1.styles.clone();
                    self.copy(styles.len())?;
                    self.styles(node).extend(styles);
                }
            }
        }
        Ok(())
    }
    pub fn define(&mut self, ids: Vec<String>, styles: Vec<String>) -> Result<(), PeleError> {
        for id in ids {
            let index = if let Some(i) = self.cls.get(&id) {
                *i
            } else {
                let i = self.model.classes.len();
                self.cls.insert(id.clone(), i);
                self.model.classes.push((
                    id.clone(),
                    RequirementClass {
                        id: id.clone(),
                        styles: vec![],
                        text_styles: vec![],
                    },
                ));
                i
            };
            for s in &styles {
                if s.contains("color") {
                    self.model.classes[index]
                        .1
                        .text_styles
                        .push(s.replacen("fill", "bgFill", 1));
                }
                self.model.classes[index].1.styles.push(s.clone());
            }
            let split: Vec<_> = styles
                .iter()
                .flat_map(|s| s.split(',').map(str::to_owned))
                .collect();
            for i in 0..self.model.requirements.len() {
                if self.model.requirements[i].1.classes.contains(&id) {
                    self.copy(split.len())?;
                    self.model.requirements[i]
                        .1
                        .css_styles
                        .extend(split.clone());
                }
            }
            for i in 0..self.model.elements.len() {
                if self.model.elements[i].1.classes.contains(&id) {
                    self.copy(split.len())?;
                    self.model.elements[i].1.css_styles.extend(split.clone());
                }
            }
        }
        Ok(())
    }
}
