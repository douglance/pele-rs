use super::{
    Atom, FunctionShape, InvocationOutcome, MethodCall, ModelGraph, ModelSchema, ModelSchemaView,
    PublicError, RenderOutcome, SharedFunction, OPAQUE_MODEL_SCHEMA,
};
use crate::compat::{
    DiagramId, DiagramModule, EdgeLimit, Engine, ErrorCode, ModuleDescriptor, Object, OutputLimit,
    ParseContext, ParseLimits, ParseOptions, PeleError, RenderOptions, RenderResources,
    RenderResult, RenderSettings, Rendered, SourceLimit,
};
use std::{any::Any, sync::Arc};
/// Typed codec opt-in. Parsing, geometry and rendering remain in the native module.
pub trait PublicDiagramModule: DiagramModule {
    type PublicState: Send + 'static;
    fn model_schema(&self) -> &'static ModelSchema {
        &OPAQUE_MODEL_SCHEMA
    }
    fn shared_functions(&self) -> &'static [SharedFunction] {
        &[]
    }
    fn function_shapes(&self) -> &'static [FunctionShape] {
        &[]
    }
    /// Method IDs, receiver identity and arguments come from this document's
    /// current arena. The module owns native behavior, including partial edits.
    fn invoke_public(
        &self,
        _state: &mut Self::PublicState,
        _call: MethodCall,
        _config: &Object,
        _host: &mut dyn RenderResources,
    ) -> Result<Atom, PublicError> {
        Err(PeleError::new("Public model methods are unavailable", ErrorCode::Semantic).into())
    }
    fn create_public(&self, model: &Self::Model) -> Result<Self::PublicState, PeleError>;
    /// Seed exact parse-time edge options before deriving public state. Once
    /// created, the mutable graph owns its options; provenance is not reapplied.
    fn create_public_with_edge_limit(
        &self,
        model: &Self::Model,
        _limit: EdgeLimit,
    ) -> Result<Self::PublicState, PeleError> {
        self.create_public(model)
    }
    fn export_public(&self, state: &Self::PublicState) -> Result<ModelGraph, PeleError>;
    fn apply_public(
        &self,
        state: &mut Self::PublicState,
        graph: ModelGraph,
    ) -> Result<(), PeleError>;
    /// Stateful render with native JavaScript/lossless error categories. Existing
    /// modules keep their render implementation through this default adapter.
    fn render_public_operation(
        &self,
        state: &mut Self::PublicState,
        config: &Object,
        options: &RenderSettings,
        host: &mut dyn RenderResources,
    ) -> Result<Rendered, PublicError> {
        self.render_public(state, config, options, host)
            .map_err(Into::into)
    }
    /// Exact post-render output policy, retaining native JavaScript/host errors.
    /// Override when the provider privately enforces legacy integer output limits.
    fn render_public_operation_with_output_limit(
        &self,
        state: &mut Self::PublicState,
        config: &Object,
        options: &RenderSettings,
        host: &mut dyn RenderResources,
        _limit: OutputLimit,
    ) -> Result<Rendered, PublicError> {
        self.render_public_operation(state, config, options, host)
    }
    fn render_public(
        &self,
        state: &mut Self::PublicState,
        config: &Object,
        options: &RenderSettings,
        host: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError>;
}
struct Shared<D> {
    module: Arc<D>,
    descriptor: ModuleDescriptor,
}
impl<D: DiagramModule> DiagramModule for Shared<D> {
    type Model = D::Model;
    fn descriptor(&self) -> ModuleDescriptor {
        self.descriptor.clone()
    }
    fn query(
        &self,
        model: &Self::Model,
        operation: &str,
        arguments: &crate::compat::Value,
    ) -> Result<crate::compat::Value, PeleError> {
        self.module.query(model, operation, arguments)
    }
    fn parse(&self, c: ParseContext<'_>) -> Result<Self::Model, PeleError> {
        self.module.parse(c)
    }
    fn parse_with_edge_limit(
        &self,
        c: ParseContext<'_>,
        limit: EdgeLimit,
    ) -> Result<Self::Model, PeleError> {
        self.module.parse_with_edge_limit(c, limit)
    }
    fn validate_model_projection(&self, m: &Self::Model) -> Result<(), PeleError> {
        self.module.validate_model_projection(m)
    }
    fn render(
        &self,
        m: &Self::Model,
        c: &Object,
        o: &RenderSettings,
        h: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError> {
        self.module.render(m, c, o, h)
    }
    fn render_with_output_limit(
        &self,
        m: &Self::Model,
        c: &Object,
        o: &RenderSettings,
        h: &mut dyn RenderResources,
        limit: OutputLimit,
    ) -> Result<Rendered, PeleError> {
        self.module.render_with_output_limit(m, c, o, h, limit)
    }
}
type State = Box<dyn Any + Send>;
trait Erased: Send + Sync {
    fn create(&self, doc: &crate::compat::ParsedDocument) -> Result<State, PeleError>;
    fn graph(&self, state: &State) -> Result<ModelGraph, PeleError>;
    fn apply(&self, state: &mut State, graph: ModelGraph) -> Result<(), PeleError>;
    fn invoke(
        &self,
        state: &mut State,
        call: MethodCall,
        config: &Object,
        host: &mut dyn RenderResources,
    ) -> Result<Atom, PublicError>;
    fn render_operation(
        &self,
        state: &mut State,
        config: &Object,
        options: &RenderSettings,
        host: &mut dyn RenderResources,
    ) -> Result<Rendered, PublicError>;
    fn render_operation_with_output_limit(
        &self,
        state: &mut State,
        config: &Object,
        options: &RenderSettings,
        host: &mut dyn RenderResources,
        limit: OutputLimit,
    ) -> Result<Rendered, PublicError>;
    fn render(
        &self,
        state: &mut State,
        config: &Object,
        options: &RenderSettings,
        host: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError>;
}
struct Codec<D>(Arc<D>);
impl<D: PublicDiagramModule> Erased for Codec<D> {
    fn create(&self, doc: &crate::compat::ParsedDocument) -> Result<State, PeleError> {
        let model = doc.model::<D::Model>().expect("paired public codec");
        Ok(Box::new(match doc.edge_limit() {
            Some(limit) => self.0.create_public_with_edge_limit(model, limit)?,
            None => self.0.create_public(model)?,
        }))
    }
    fn graph(&self, s: &State) -> Result<ModelGraph, PeleError> {
        self.0
            .export_public(s.downcast_ref().expect("paired public state"))
    }
    fn apply(&self, s: &mut State, g: ModelGraph) -> Result<(), PeleError> {
        self.0
            .apply_public(s.downcast_mut().expect("paired public state"), g)
    }
    fn invoke(
        &self,
        s: &mut State,
        call: MethodCall,
        c: &Object,
        h: &mut dyn RenderResources,
    ) -> Result<Atom, PublicError> {
        self.0
            .invoke_public(s.downcast_mut().expect("paired public state"), call, c, h)
    }
    fn render_operation(
        &self,
        state: &mut State,
        config: &Object,
        options: &RenderSettings,
        host: &mut dyn RenderResources,
    ) -> Result<Rendered, PublicError> {
        self.0.render_public_operation(
            state.downcast_mut().expect("paired public state"),
            config,
            options,
            host,
        )
    }
    fn render_operation_with_output_limit(
        &self,
        state: &mut State,
        config: &Object,
        options: &RenderSettings,
        host: &mut dyn RenderResources,
        limit: OutputLimit,
    ) -> Result<Rendered, PublicError> {
        self.0.render_public_operation_with_output_limit(
            state.downcast_mut().expect("paired public state"),
            config,
            options,
            host,
            limit,
        )
    }
    fn render(
        &self,
        s: &mut State,
        c: &Object,
        o: &RenderSettings,
        h: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError> {
        self.0
            .render_public(s.downcast_mut().expect("paired public state"), c, o, h)
    }
}
struct Registration {
    descriptor: ModuleDescriptor,
    schema: &'static ModelSchema,
    shared_functions: &'static [SharedFunction],
    function_shapes: &'static [FunctionShape],
    codec: Box<dyn Erased>,
}
#[derive(Default)]
pub struct PublicEngine {
    engine: Engine,
    modules: Vec<Arc<Registration>>,
}
impl PublicEngine {
    /// All built-in mutable codecs. Register extensions/replacements through the
    /// same typed interface; registration alone does not certify runtime parity.
    pub fn with_builtins() -> Self {
        let mut engine = Self::empty();
        engine.register(crate::compat::diagrams::venn::Venn);
        engine.register(crate::compat::diagrams::ishikawa::Ishikawa);
        engine.register(crate::compat::diagrams::swimlane::Swimlane);
        engine.register(crate::compat::diagrams::wardley::Wardley);
        engine.register(crate::compat::diagrams::event_modeling::EventModeling);
        engine.register(crate::compat::diagrams::usecase::Usecase);
        engine.register(crate::compat::diagrams::cynefin::Cynefin);
        engine.register(crate::compat::diagrams::tree_view::TreeView);
        engine.register(crate::compat::diagrams::agentflow::AgentFlow);
        engine.register(crate::compat::diagrams::requirement::RequirementDiagram);
        engine.register(crate::compat::diagrams::block::Block);
        engine.register(crate::compat::diagrams::railroad::Railroad);
        engine.register(crate::compat::diagrams::treemap::Treemap);
        engine.register(crate::compat::diagrams::c4::C4);
        engine.register(crate::compat::diagrams::info::Info);
        engine.register(crate::compat::diagrams::git_graph::GitGraph);
        engine.register(crate::compat::diagrams::architecture::Architecture);
        engine.register(crate::compat::diagrams::sankey::Sankey);
        engine.register(crate::compat::diagrams::state::State);
        engine.register(crate::compat::diagrams::gantt::Gantt);
        engine.register(crate::compat::diagrams::sequence::Sequence);
        engine.register(crate::compat::diagrams::xychart::XyChart);
        engine.register(crate::compat::diagrams::class_diagram::Class);
        engine.register(crate::compat::diagrams::er::Er);
        engine.register(crate::compat::diagrams::flowchart::Flowchart);
        engine.register(crate::compat::diagrams::kanban::Kanban);
        engine.register(crate::compat::diagrams::mindmap::Mindmap);
        engine.register(crate::compat::diagrams::pie::Pie);
        engine.register(crate::compat::diagrams::quadrant_chart::QuadrantChart);
        engine.register(crate::compat::diagrams::packet::Packet);
        engine.register(crate::compat::diagrams::radar::Radar);
        engine.register(crate::compat::diagrams::timeline::Timeline);
        engine.register(crate::compat::diagrams::journey::Journey);
        engine
    }
    pub fn empty() -> Self {
        Self::default()
    }
    pub fn register<D: PublicDiagramModule>(&mut self, module: D) {
        let descriptor = module.descriptor();
        let schema = module.model_schema();
        let shared_functions = module.shared_functions();
        let function_shapes = module.function_shapes();
        let shared = Arc::new(module);
        self.engine.register(Shared {
            module: shared.clone(),
            descriptor: descriptor.clone(),
        });
        let registration = Arc::new(Registration {
            descriptor,
            schema,
            shared_functions,
            function_shapes,
            codec: Box::new(Codec(shared)),
        });
        if let Some(slot) = self
            .modules
            .iter_mut()
            .find(|m| m.descriptor.id == registration.descriptor.id)
        {
            *slot = registration
        } else {
            self.modules.push(registration)
        }
    }
    pub fn parse(&self, source: &str, options: &ParseOptions) -> Result<PublicDocument, PeleError> {
        self.create_document(self.engine.parse(source, options)?)
    }
    /// Parse through the retained native codec with an exact number-valued
    /// source limit. Only the legacy source limit is overridden.
    pub fn parse_with_source_limit(
        &self,
        source: &str,
        options: &ParseOptions,
        limit: SourceLimit,
    ) -> Result<PublicDocument, PeleError> {
        self.create_document(
            self.engine
                .parse_with_source_limit(source, options, limit)?,
        )
    }
    pub fn parse_with_limits(
        &self,
        source: &str,
        options: &ParseOptions,
        limits: ParseLimits,
    ) -> Result<PublicDocument, PeleError> {
        self.create_document(self.engine.parse_with_limits(source, options, limits)?)
    }
    pub fn parse_as_with_limits(
        &self,
        id: &DiagramId,
        source: &str,
        options: &ParseOptions,
        limits: ParseLimits,
    ) -> Result<PublicDocument, PeleError> {
        self.create_document(
            self.engine
                .parse_as_with_limits(id, source, options, limits)?,
        )
    }
    /// Select an extension language without changing the built-in source detector.
    pub fn parse_as(
        &self,
        id: &DiagramId,
        source: &str,
        options: &ParseOptions,
    ) -> Result<PublicDocument, PeleError> {
        self.create_document(self.engine.parse_as(id, source, options)?)
    }
    /// Select an extension while keeping the public byte budget and exact
    /// source comparison, without changing the extension's ParseContext shape.
    pub fn parse_as_with_source_limit(
        &self,
        id: &DiagramId,
        source: &str,
        options: &ParseOptions,
        limit: SourceLimit,
    ) -> Result<PublicDocument, PeleError> {
        self.create_document(
            self.engine
                .parse_as_with_source_limit(id, source, options, limit)?,
        )
    }
    fn create_document(
        &self,
        document: crate::compat::ParsedDocument,
    ) -> Result<PublicDocument, PeleError> {
        let registration = self
            .modules
            .iter()
            .find(|m| m.descriptor.id == *document.diagram_type())
            .expect("paired public registration")
            .clone();
        ModelSchemaView {
            model: registration.schema,
            shared_functions: registration.shared_functions,
            function_shapes: registration.function_shapes,
        }
        .validate()?;
        let state = registration.codec.create(&document)?;
        Ok(PublicDocument {
            registration,
            state,
            config: document.config().clone(),
            revision: 0,
        })
    }
}
/// One serialized mutable operation stream. This type is deliberately not Sync.
pub struct PublicDocument {
    registration: Arc<Registration>,
    state: State,
    config: Object,
    revision: u64,
}
impl PublicDocument {
    /// Adapt an existing typed model without parsing its source again.
    /// The document owns the exact module and effective config supplied here.
    pub fn from_module<D: PublicDiagramModule>(
        module: D,
        model: &D::Model,
        config: Object,
    ) -> Result<Self, PeleError> {
        let descriptor = module.descriptor();
        let schema = module.model_schema();
        let shared_functions = module.shared_functions();
        let function_shapes = module.function_shapes();
        ModelSchemaView {
            model: schema,
            shared_functions,
            function_shapes,
        }
        .validate()?;
        let state: State = Box::new(module.create_public(model)?);
        Ok(Self {
            registration: Arc::new(Registration {
                descriptor,
                schema,
                shared_functions,
                function_shapes,
                codec: Box::new(Codec(Arc::new(module))),
            }),
            state,
            config,
            revision: 0,
        })
    }
    pub fn diagram_type(&self) -> &DiagramId {
        &self.registration.descriptor.id
    }
    pub fn model_schema(&self) -> &'static ModelSchema {
        self.registration.schema
    }
    pub fn schema_view(&self) -> ModelSchemaView {
        ModelSchemaView {
            model: self.registration.schema,
            shared_functions: self.registration.shared_functions,
            function_shapes: self.registration.function_shapes,
        }
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn snapshot(&self) -> Result<ModelGraph, PeleError> {
        let graph = self.registration.codec.graph(&self.state)?;
        graph.validate_schema(self.registration.schema)?;
        Ok(graph)
    }
    pub fn apply(&mut self, expected_revision: u64, graph: ModelGraph) -> Result<(), PeleError> {
        if expected_revision != self.revision {
            return Err(PeleError::new(
                "Stale public model revision",
                ErrorCode::Semantic,
            ));
        }
        let next_revision = self
            .revision
            .checked_add(1)
            .ok_or_else(|| PeleError::new("Public model revision exhausted", ErrorCode::Limit))?;
        graph.validate_schema(self.registration.schema)?;
        self.registration.codec.apply(&mut self.state, graph)?;
        self.revision = next_revision;
        Ok(())
    }
    /// Dispatch through the retained module, then publish every prior mutation
    /// even if the operation returns a native, JavaScript or host exception.
    /// Stale/malformed calls never reach module code. A codec that exports an
    /// invalid graph is a protocol error; its state and advanced revision remain.
    pub fn invoke(
        &mut self,
        expected_revision: u64,
        call: MethodCall,
        host: &mut dyn RenderResources,
    ) -> Result<InvocationOutcome, PeleError> {
        if expected_revision != self.revision {
            return Err(PeleError::new(
                "Stale public model revision",
                ErrorCode::Semantic,
            ));
        }
        let next = self
            .revision
            .checked_add(1)
            .ok_or_else(|| PeleError::new("Public model revision exhausted", ErrorCode::Limit))?;
        let before = self.registration.codec.graph(&self.state)?;
        call.validate(&before, self.registration.schema)?;
        let mut result = self
            .registration
            .codec
            .invoke(&mut self.state, call, &self.config, host);
        self.revision = next;
        let graph = self.snapshot()?;
        // A malformed native return is an operation protocol error, while its
        // successful partial mutations still publish in the same outcome.
        if let Ok(atom) = &result {
            let checked = graph.validate_atom(atom).and_then(|_| {
                if let Atom::BuiltinFunction(id) = atom {
                    if self
                        .registration
                        .schema
                        .method(super::MethodId(*id))
                        .is_none()
                    {
                        return Err(PeleError::new(
                            "Unknown public builtin function",
                            ErrorCode::Semantic,
                        ));
                    }
                }
                Ok(())
            });
            if let Err(e) = checked {
                result = Err(e.into());
            }
        }
        Ok(InvocationOutcome {
            revision: next,
            graph,
            result,
        })
    }
    /// Render through the retained handler, publishing post-render state on
    /// success or operation error. Stale revisions reject before dispatch.
    pub fn render_operation(
        &mut self,
        expected_revision: u64,
        options: &RenderOptions,
        host: &mut dyn RenderResources,
    ) -> Result<RenderOutcome, PeleError> {
        self.render_operation_policy(expected_revision, options, host, None)
    }
    /// Exact output comparison after rendering. Dispatched mutations/revision
    /// publish on limit and host errors, under the existing graph export budgets.
    pub fn render_operation_with_output_limit(
        &mut self,
        expected_revision: u64,
        options: &RenderOptions,
        limit: OutputLimit,
        host: &mut dyn RenderResources,
    ) -> Result<RenderOutcome, PeleError> {
        self.render_operation_policy(expected_revision, options, host, Some(limit))
    }
    fn render_operation_policy(
        &mut self,
        expected_revision: u64,
        options: &RenderOptions,
        host: &mut dyn RenderResources,
        limit: Option<OutputLimit>,
    ) -> Result<RenderOutcome, PeleError> {
        if expected_revision != self.revision {
            return Err(PeleError::new(
                "Stale public model revision",
                ErrorCode::Semantic,
            ));
        }
        let next = self
            .revision
            .checked_add(1)
            .ok_or_else(|| PeleError::new("Public model revision exhausted", ErrorCode::Limit))?;
        self.snapshot()?;
        let mut operation_error = None;
        let result = crate::compat::engine::render_registered_policy(
            &self.registration.descriptor,
            &self.config,
            options,
            host,
            limit,
            |settings, session| {
                let codec = &self.registration.codec;
                let result = match limit {
                    Some(limit) => codec.render_operation_with_output_limit(
                        &mut self.state,
                        &self.config,
                        settings,
                        session,
                        limit,
                    ),
                    None => {
                        codec.render_operation(&mut self.state, &self.config, settings, session)
                    }
                };
                result.map_err(|error| {
                    operation_error = Some(error);
                    // The orchestration layer only needs to stop. Preserve
                    // the real typed error outside its legacy error lane.
                    PeleError::new("Public render operation failed", ErrorCode::Semantic)
                })
            },
        )
        .map_err(|error| operation_error.unwrap_or_else(|| error.into()));
        self.revision = next;
        let graph = self.snapshot()?;
        Ok(RenderOutcome {
            revision: next,
            graph,
            result,
        })
    }
    pub fn render(
        &mut self,
        options: &RenderOptions,
        host: &mut dyn RenderResources,
    ) -> Result<RenderResult, PeleError> {
        crate::compat::engine::render_registered(
            &self.registration.descriptor,
            &self.config,
            options,
            host,
            |settings, session| {
                self.registration
                    .codec
                    .render(&mut self.state, &self.config, settings, session)
            },
        )
    }
}
