//! Pele sankey/db.ts ordered nodes and links, 458322a5 (MIT, kepano).
use crate::compat::DiagramType;
use serde::Serialize;
#[derive(Clone, Debug, Serialize)]
pub struct SankeyNode {
    pub id: String,
    pub index: usize,
}
#[derive(Clone, Debug, Serialize)]
pub struct SankeyLink {
    pub source: SankeyNode,
    pub target: SankeyNode,
    pub value: f64,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SankeyModel {
    #[serde(rename = "type")]
    pub diagram_type: DiagramType,
    pub nodes: Vec<SankeyNode>,
    pub links: Vec<SankeyLink>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acc_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acc_descr: Option<String>,
}
impl SankeyModel {
    pub(super) fn new(title: Option<&str>) -> Self {
        Self {
            diagram_type: DiagramType::Sankey,
            nodes: vec![],
            links: vec![],
            title: title.map(str::to_owned),
            acc_title: None,
            acc_descr: None,
        }
    }
}
