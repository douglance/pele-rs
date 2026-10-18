//! Sankey diagrams, direct Pele sankey/* 458322a5 translation (MIT, kepano).
mod layout;
mod model;
mod parser;
mod public_model;
pub use public_model::SankeyPublicState;
mod render;
use crate::compat::{
    DiagramModule, DiagramType, ModuleDescriptor, Object, ParseContext, PeleError, RenderResources,
    RenderSettings, Rendered,
};
pub use model::{SankeyLink, SankeyModel, SankeyNode};
#[derive(Default)]
pub struct Sankey;
impl DiagramModule for Sankey {
    type Model = SankeyModel;
    fn descriptor(&self) -> ModuleDescriptor {
        ModuleDescriptor::new(DiagramType::Sankey)
    }
    fn parse(&self, c: ParseContext<'_>) -> Result<Self::Model, PeleError> {
        parser::parse(c)
    }
    fn render(
        &self,
        m: &Self::Model,
        c: &Object,
        o: &RenderSettings,
        r: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError> {
        render::render(m, c, o, r)
    }
}
