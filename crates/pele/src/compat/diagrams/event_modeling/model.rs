//! Pele eventmodeling/model.ts, 458322a5 (MIT, kepano). Shared boxes retain relation/lane identity.
use crate::compat::DiagramType;
use serde::{ser::SerializeSeq, Serialize, Serializer};
use std::sync::Arc;
pub(super) fn arc<S: Serializer, T: Serialize>(v: &Arc<T>, s: S) -> Result<S::Ok, S::Error> {
    v.as_ref().serialize(s)
}
fn arcs<S: Serializer, T: Serialize>(v: &[Arc<T>], s: S) -> Result<S::Ok, S::Error> {
    let mut seq = s.serialize_seq(Some(v.len()))?;
    for v in v {
        seq.serialize_element(v.as_ref())?;
    }
    seq.end()
}
fn text<S: Serializer>(v: &Option<Arc<str>>, s: S) -> Result<S::Ok, S::Error> {
    match v {
        Some(v) => s.serialize_str(v),
        None => s.serialize_none(),
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EntityKind {
    Ui,
    Processor,
    Command,
    Readmodel,
    Event,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LaneGroup {
    Trigger,
    State,
    Stream,
}
#[derive(Clone, Debug, Serialize)]
pub struct EmLane {
    pub index: usize,
    pub label: String,
    pub group: LaneGroup,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmBox {
    pub id: String,
    pub kind: EntityKind,
    pub entity: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    pub reset: bool,
    #[serde(serialize_with = "arc")]
    pub lane: Arc<EmLane>,
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "text")]
    pub data: Option<Arc<str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_type: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct EmRelation {
    #[serde(serialize_with = "arc")]
    pub source: Arc<EmBox>,
    #[serde(serialize_with = "arc")]
    pub target: Arc<EmBox>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmNote {
    pub frame: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_type: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct EmSpecification {
    pub frame: String,
    pub given: Vec<String>,
    pub when: Vec<String>,
    pub then: Vec<String>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventModelingModel {
    #[serde(rename = "type")]
    pub diagram_type: DiagramType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acc_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acc_descr: Option<String>,
    #[serde(serialize_with = "arcs")]
    pub boxes: Vec<Arc<EmBox>>,
    #[serde(serialize_with = "arcs")]
    pub lanes: Vec<Arc<EmLane>>,
    pub relations: Vec<EmRelation>,
    pub notes: Vec<EmNote>,
    pub specifications: Vec<EmSpecification>,
    pub entities: Vec<String>,
}
