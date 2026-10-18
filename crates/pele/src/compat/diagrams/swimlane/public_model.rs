//! Swimlane shares actual FlowDb identity/fields/builders, Pele 458322a5 (MIT).
//! Only the retained module renderer differs, exactly as swimlane/index.ts.
use super::{Swimlane, SwimlaneModel};
use crate::compat::{
    diagrams::flowchart::{Flowchart, FlowchartPublicState},
    public::*,
    DiagramModule, Object, PeleError, RenderResources, RenderSettings, Rendered,
};
pub type SwimlanePublicState = FlowchartPublicState;
impl PublicDiagramModule for Swimlane {
    type PublicState = SwimlanePublicState;
    fn model_schema(&self) -> &'static ModelSchema {
        Flowchart.model_schema()
    }
    fn create_public(&self, model: &SwimlaneModel) -> Result<Self::PublicState, PeleError> {
        Flowchart.create_public(model)
    }
    fn export_public(&self, state: &Self::PublicState) -> Result<ModelGraph, PeleError> {
        Flowchart.export_public(state)
    }
    fn apply_public(
        &self,
        state: &mut Self::PublicState,
        graph: ModelGraph,
    ) -> Result<(), PeleError> {
        Flowchart.apply_public(state, graph)
    }
    fn invoke_public(
        &self,
        state: &mut Self::PublicState,
        call: MethodCall,
        config: &Object,
        host: &mut dyn RenderResources,
    ) -> Result<Atom, PublicError> {
        Flowchart.invoke_public(state, call, config, host)
    }
    fn render_public(
        &self,
        state: &mut Self::PublicState,
        config: &Object,
        options: &RenderSettings,
        host: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError> {
        self.render(state.render_model()?, config, options, host)
    }
}
