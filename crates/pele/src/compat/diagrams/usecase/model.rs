//! Pele usecase/types.ts + draft contracts from db.ts, 458322a5 (MIT, kepano).
use crate::compat::DiagramType;
use serde::Serialize;
pub type Span = [usize; 2];
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Actor {
    pub id: String,
    pub label: String,
    pub label_type: String,
    #[serde(rename = "type")]
    pub actor_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    pub business: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stereotype: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    pub classes: Vec<String>,
    pub styles: Vec<String>,
}
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UseCase {
    pub id: String,
    pub label: String,
    pub label_type: String,
    pub shape: String,
    pub business: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stereotype: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    pub classes: Vec<String>,
    pub styles: Vec<String>,
}
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemBoundary {
    pub id: String,
    pub label: String,
    pub label_type: String,
    #[serde(rename = "type")]
    pub boundary_type: String,
    pub members: Vec<String>,
    pub classes: Vec<String>,
    pub styles: Vec<String>,
}
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Relationship {
    pub id: String,
    pub explicit_id: bool,
    pub source: String,
    pub target: String,
    #[serde(rename = "type")]
    pub relationship_type: String,
    pub arrow_type: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label_type: Option<String>,
    pub minlen: usize,
    pub classes: Vec<String>,
    pub styles: Vec<String>,
    pub animate: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub animation: Option<String>,
}
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsecaseNote {
    pub id: String,
    pub target: String,
    pub label: String,
    pub label_type: String,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsecaseJsonNode {
    pub id: String,
    pub value: super::json::JsonValue,
    pub property_order: Vec<(String, Vec<String>)>,
    pub classes: Vec<String>,
    pub styles: Vec<String>,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct ClassDef {
    pub id: String,
    #[serde(serialize_with = "arc_styles")]
    pub styles: std::sync::Arc<Vec<String>>,
}
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataOccurrence {
    pub key: String,
    pub span: Span,
    pub key_span: Span,
    pub value_span: Span,
}
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeOccurrence {
    pub id: String,
    pub span: Span,
    pub id_span: Span,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label_span: Option<Span>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defines: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stereotype_span: Option<Span>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Vec<MetadataOccurrence>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_spans: Option<Vec<Span>>,
}
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EdgeOccurrence {
    pub id: String,
    pub span: Span,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label_span: Option<Span>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_span: Option<Span>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Vec<MetadataOccurrence>>,
}
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphStatement {
    pub kind: String,
    pub span: Span,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nodes: Option<Vec<NodeOccurrence>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edges: Option<Vec<EdgeOccurrence>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_span: Option<Span>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_span: Option<Span>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_span: Option<Span>,
    #[serde(rename = "ref", skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ref_span: Option<Span>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<GraphStatement>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Vec<MetadataOccurrence>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_spans: Option<Vec<Span>>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsecaseModel {
    #[serde(rename = "type")]
    pub diagram_type: DiagramType,
    pub actors: Vec<(String, Actor)>,
    pub use_cases: Vec<(String, UseCase)>,
    pub system_boundaries: Vec<(String, SystemBoundary)>,
    pub relationships: Vec<Relationship>,
    pub notes: Vec<(String, UsecaseNote)>,
    pub json_nodes: Vec<(String, UsecaseJsonNode)>,
    pub class_defs: Vec<(String, ClassDef)>,
    pub direction: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub acc_title: String,
    pub acc_descr: String,
    pub acc_description: String,
    pub source: String,
    pub header_span: Span,
    pub statements: Vec<GraphStatement>,
}
impl Default for UsecaseModel {
    fn default() -> Self {
        Self {
            diagram_type: DiagramType::Usecase,
            actors: vec![],
            use_cases: vec![],
            system_boundaries: vec![],
            relationships: vec![],
            notes: vec![],
            json_nodes: vec![],
            class_defs: vec![],
            direction: "LR".into(),
            title: None,
            acc_title: String::new(),
            acc_descr: String::new(),
            acc_description: String::new(),
            source: String::new(),
            header_span: [0, 0],
            statements: vec![],
        }
    }
}
#[derive(Clone, Debug, Default)]
pub(super) struct DraftLabel {
    pub text: String,
    pub kind: String,
    pub span: Span,
}
#[derive(Clone, Debug)]
pub(super) enum PropertyValue {
    Text(String),
    Bool(bool),
}
impl PropertyValue {
    pub fn text(&self) -> Option<&str> {
        if let Self::Text(s) = self {
            Some(s)
        } else {
            None
        }
    }
    pub fn boolean(&self) -> Option<bool> {
        if let Self::Bool(b) = self {
            Some(*b)
        } else {
            None
        }
    }
}
#[derive(Clone, Debug)]
pub(super) struct DraftProperty {
    pub occurrence: MetadataOccurrence,
    pub value: PropertyValue,
}
#[derive(Clone, Debug, Default)]
pub(super) struct Named {
    pub id: String,
    pub label: DraftLabel,
    pub span: Span,
    pub generated: bool,
}
#[derive(Clone, Debug)]
pub(super) struct DraftBoundary {
    pub named: Named,
    pub classes: Vec<String>,
}
#[derive(Clone, Debug)]
pub(super) struct DraftElement {
    pub named: Named,
    pub kind: String,
    pub parent: Option<DraftBoundary>,
    pub shape: Option<String>,
    pub metadata: Option<Vec<DraftProperty>>,
    pub stereotype: Option<String>,
    pub classes: Vec<String>,
}
#[derive(Clone, Debug)]
pub(super) struct DraftEndpoint {
    pub named: Named,
    pub declares: bool,
    pub classed: bool,
}
#[derive(Clone, Debug)]
pub(super) struct DraftRelationship {
    pub source: DraftEndpoint,
    pub target: DraftEndpoint,
    pub span: Span,
    pub id: Option<String>,
    pub id_span: Option<Span>,
    pub relationship_type: String,
    pub arrow_type: usize,
    pub label: Option<DraftLabel>,
    pub minlen: usize,
}
#[derive(Clone, Debug)]
pub(super) struct DraftJson {
    pub id: String,
    pub value: super::json::JsonValue,
    pub property_order: Vec<(String, Vec<String>)>,
    pub span: Span,
    pub classes: Vec<String>,
}
#[derive(Clone, Debug)]
pub(super) struct DraftMetadata {
    pub target: String,
    pub span: Span,
    pub properties: Vec<DraftProperty>,
    pub statement: usize,
    pub child: Option<usize>,
}
#[derive(Clone, Debug, Default)]
pub(super) struct Drafts {
    pub elements: Vec<DraftElement>,
    pub boundaries: Vec<DraftBoundary>,
    pub jsons: Vec<DraftJson>,
    pub relationships: Vec<DraftRelationship>,
    pub notes: Vec<(String, Span, DraftLabel)>,
    pub metadata: Vec<DraftMetadata>,
    pub classes: Vec<(Vec<(String, Span)>, Vec<String>)>,
    pub styles: Vec<(String, Span, Vec<String>)>,
    pub class_defs: Vec<(Vec<String>, Vec<String>)>,
    pub direction: Option<String>,
    pub acc_title: String,
    pub acc_description: String,
}

fn arc_styles<S: serde::Serializer>(
    v: &std::sync::Arc<Vec<String>>,
    s: S,
) -> Result<S::Ok, S::Error> {
    v.as_ref().serialize(s)
}
