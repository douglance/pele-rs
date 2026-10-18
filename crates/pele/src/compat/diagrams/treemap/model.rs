//! Pele treemap/model.ts ordered hierarchy and class compilation, 458322a5 (MIT).
use crate::compat::DiagramType;
use serde::Serialize;
use std::sync::Arc;
fn styles_view<S: serde::Serializer>(
    v: &Option<Arc<Vec<String>>>,
    s: S,
) -> Result<S::Ok, S::Error> {
    v.as_deref().serialize(s)
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TreemapNode {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<TreemapNode>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_selector: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "styles_view"
    )]
    pub css_compiled_styles: Option<Arc<Vec<String>>>,
}
// Drain descendants iteratively so typed trees remain safe beyond the JSON view budget.
impl Drop for TreemapNode {
    fn drop(&mut self) {
        let mut stack = self.children.take().unwrap_or_default();
        while let Some(mut n) = stack.pop() {
            if let Some(children) = n.children.take() {
                stack.extend(children)
            }
        }
    }
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TreemapModel {
    #[serde(rename = "type")]
    pub diagram_type: DiagramType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acc_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acc_descr: Option<String>,
    pub roots: Vec<TreemapNode>,
    pub classes: Vec<(String, Vec<String>)>,
}
