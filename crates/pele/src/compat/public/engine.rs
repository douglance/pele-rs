use super::{
    Atom, InvocationOutcome, MethodCall, ModelGraph, ModelSchema, PublicError, OPAQUE_MODEL_SCHEMA,
};
use crate::compat::{
    DiagramId, DiagramModule, Engine, ErrorCode, ModuleDescriptor, Object, ParseContext,
    ParseOptions, PeleError, RenderOptions, RenderResources, RenderResult, RenderSettings,
    Rendered,
};
use std::{any::Any, sync::Arc};
/// Typed codec opt-in. Parsing, geometry and rendering remain in the native module.
pub trait PublicDiagramModule: DiagramModule {
    type PublicState: Send + 'static;
    fn model_schema(&self) -> &'static ModelSchema {
        &OPAQUE_MODEL_SCHEMA
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
    fn export_public(&self, state: &Self::PublicState) -> Result<ModelGraph, PeleError>;
    fn apply_public(
        &self,
        state: &mut Self::PublicState,
        graph: ModelGraph,
    ) -> Result<(), PeleError>;
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
        Ok(Box::new(self.0.create_public(
            doc.model::<D::Model>().expect("paired public codec"),
        )?))
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
    codec: Box<dyn Erased>,
}
#[derive(Default)]
pub struct PublicEngine {
    engine: Engine,
    modules: Vec<Arc<Registration>>,
}
impl PublicEngine {
    pub fn empty() -> Self {
        Self::default()
    }
    pub fn register<D: PublicDiagramModule>(&mut self, module: D) {
        let descriptor = module.descriptor();
        let schema = module.model_schema();
        let shared = Arc::new(module);
        self.engine.register(Shared {
            module: shared.clone(),
            descriptor: descriptor.clone(),
        });
        let registration = Arc::new(Registration {
            descriptor,
            schema,
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
    /// Select an extension language without changing the built-in source detector.
    pub fn parse_as(
        &self,
        id: &DiagramId,
        source: &str,
        options: &ParseOptions,
    ) -> Result<PublicDocument, PeleError> {
        self.create_document(self.engine.parse_as(id, source, options)?)
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
        registration.schema.validate()?;
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
        schema.validate()?;
        let state: State = Box::new(module.create_public(model)?);
        Ok(Self {
            registration: Arc::new(Registration {
                descriptor,
                schema,
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
