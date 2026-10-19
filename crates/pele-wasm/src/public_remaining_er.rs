//! Retained native mutable adapters. JavaScript uses the common ModelBridge.
use pele::compat::{
    diagrams::{
        block::{Block, BlockModel},
        gantt::{Gantt, GanttModel},
        railroad::{Railroad, RailroadModel},
        requirement::{RequirementDiagram, RequirementModel},
        sequence::{Sequence, SequenceModel},
    },
    public::PublicDocument,
    ParsedDocument, PeleError,
};

pub(super) fn convert(document: &ParsedDocument) -> Option<Result<PublicDocument, PeleError>> {
    if let Some(model) = document.model::<GanttModel>() {
        Some(PublicDocument::from_module(
            Gantt,
            model,
            document.config().clone(),
        ))
    } else if let Some(model) = document.model::<SequenceModel>() {
        Some(PublicDocument::from_module(
            Sequence,
            model,
            document.config().clone(),
        ))
    } else if let Some(model) = document.model::<RequirementModel>() {
        Some(PublicDocument::from_module(
            RequirementDiagram,
            model,
            document.config().clone(),
        ))
    } else if let Some(model) = document.model::<BlockModel>() {
        Some(PublicDocument::from_module(
            Block,
            model,
            document.config().clone(),
        ))
    } else {
        document
            .model::<RailroadModel>()
            .map(|model| PublicDocument::from_module(Railroad, model, document.config().clone()))
    }
}
