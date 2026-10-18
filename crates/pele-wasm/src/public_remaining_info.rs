//! Remaining public codecs: concrete Rust models retain their existing renderers.
use pele::compat::{
    diagrams::{
        c4::{C4Model, C4},
        cynefin::{Cynefin, CynefinModel},
        usecase::{Usecase, UsecaseModel},
        venn::{Venn, VennModel},
        wardley::{Wardley, WardleyModel},
    },
    public::PublicDocument,
    ParsedDocument, PeleError,
};

pub(super) fn convert(document: &ParsedDocument) -> Option<Result<PublicDocument, PeleError>> {
    if let Some(model) = document.model::<C4Model>() {
        Some(PublicDocument::from_module(
            C4,
            model,
            document.config().clone(),
        ))
    } else if let Some(model) = document.model::<UsecaseModel>() {
        Some(PublicDocument::from_module(
            Usecase,
            model,
            document.config().clone(),
        ))
    } else if let Some(model) = document.model::<CynefinModel>() {
        Some(PublicDocument::from_module(
            Cynefin,
            model,
            document.config().clone(),
        ))
    } else if let Some(model) = document.model::<WardleyModel>() {
        Some(PublicDocument::from_module(
            Wardley,
            model,
            document.config().clone(),
        ))
    } else {
        document
            .model::<VennModel>()
            .map(|model| PublicDocument::from_module(Venn, model, document.config().clone()))
    }
}
