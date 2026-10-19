//! Shared orchestration for owner-confined foreign models. No host pointers enter core.
use super::*;

pub struct ProviderInput {
    pub source: String,
    pub config: Object,
    pub title: Option<String>,
    pub line_offset: usize,
}
impl Inspection {
    /// Finish the already inspected source for the selected provider.
    pub fn into_provider_input(self, keep_comments: bool) -> ProviderInput {
        let mut source = if keep_comments {
            source::encode_entities(&self.prepared.with_comments)
        } else {
            self.encoded_text
        };
        source.push('\n');
        ProviderInput {
            source,
            config: self.prepared.config,
            title: self.prepared.title,
            line_offset: self.prepared.line_offset,
        }
    }
}
/// Shared responsive/icon/theme/output/link policy; foreign models stay in their owner.
/// Bounds accepted output bytes, not arbitrary provider runtime or allocation.
pub fn render_provider_bounded(
    descriptor: &ModuleDescriptor,
    config: &Object,
    options: &RenderOptions,
    limit: Option<OutputLimit>,
    resources: &mut dyn RenderResources,
    render: impl FnOnce(&RenderSettings, &mut dyn RenderResources) -> Result<Rendered, PeleError>,
) -> Result<RenderResult, PeleError> {
    super::engine::render_registered_policy(
        descriptor, config, options, resources, limit, true, render,
    )
}
