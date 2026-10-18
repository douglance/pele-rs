use super::{
    module::*, source, Capability, DiagramId, DiagramType, ErrorCode, Inspection, Object,
    PeleError, PrepareOptions, Value, DIAGRAM_TYPES,
};
use serde::Serialize;
use std::{any::Any, collections::HashMap, sync::Arc};
type Model = Box<dyn Any + Send + Sync>;
trait ErasedModule: Send + Sync {
    fn parse(&self, context: ParseContext<'_>) -> Result<Model, PeleError>;
    fn render(
        &self,
        model: &Model,
        config: &Object,
        options: &RenderSettings,
        resources: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError>;
    fn model_view(&self, model: &Model) -> Result<Value, PeleError>;
}
struct Module<D>(D);
impl<D: DiagramModule> ErasedModule for Module<D> {
    fn parse(&self, context: ParseContext<'_>) -> Result<Model, PeleError> {
        Ok(Box::new(self.0.parse(context)?))
    }
    fn render(
        &self,
        model: &Model,
        config: &Object,
        options: &RenderSettings,
        resources: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError> {
        self.0.render(
            model.downcast_ref().expect("private module/model pairing"),
            config,
            options,
            resources,
        )
    }
    fn model_view(&self, model: &Model) -> Result<Value, PeleError> {
        let json = serde_json::to_string(
            model
                .downcast_ref::<D::Model>()
                .expect("private module/model pairing"),
        );
        json.and_then(|s| super::value::parse_json(&s))
            .map_err(|e| {
                PeleError::new(
                    format!("Module model cannot be serialized: {e}"),
                    ErrorCode::Semantic,
                )
            })
    }
}
struct Registration {
    descriptor: ModuleDescriptor,
    module: Box<dyn ErasedModule>,
}
/// Immutable document with exact registration/config. Cannot be fabricated from JSON.
pub struct ParsedDocument {
    registration: Arc<Registration>,
    model: Model,
    config: Object,
}
impl ParsedDocument {
    pub fn diagram_type(&self) -> &DiagramId {
        &self.registration.descriptor.id
    }
    pub fn model<T: Any>(&self) -> Option<&T> {
        self.model.downcast_ref()
    }
    pub fn model_view(&self) -> Result<Value, PeleError> {
        self.registration.module.model_view(&self.model)
    }
    pub fn config(&self) -> &Object {
        &self.config
    }
}
/// Ordered registry. Metadata is captured once; existing documents retain their registration.
#[derive(Default, Clone)]
pub struct Engine {
    modules: Vec<Arc<Registration>>,
}
impl Engine {
    /// Registers implemented built-ins only; consult capabilities for parity status.
    pub fn with_builtins() -> Self {
        let mut engine = Self::empty();
        engine.register(super::diagrams::pie::Pie);
        engine
    }
    pub fn empty() -> Self {
        Self::default()
    }
    pub fn register<D: DiagramModule>(&mut self, module: D) {
        let descriptor = module.descriptor();
        let registration = Arc::new(Registration {
            descriptor,
            module: Box::new(Module(module)),
        });
        if let Some(i) = self
            .modules
            .iter()
            .position(|m| m.descriptor.id == registration.descriptor.id)
        {
            self.modules[i] = registration
        } else {
            self.modules.push(registration)
        }
    }
    pub fn registered(&self, id: impl Into<DiagramId>) -> bool {
        let id = id.into();
        self.modules.iter().any(|m| m.descriptor.id == id)
    }
    pub fn capabilities(&self) -> Vec<Capability> {
        let mut caps: Vec<_> = DIAGRAM_TYPES
            .iter()
            .map(|kind| Capability {
                diagram_type: (*kind).into(),
                known: true,
                registered: self.registered(*kind),
                syntax_parity: false,
                semantic_parity: false,
                render_parity: false,
                host_parity: false,
            })
            .collect();
        for module in &self.modules {
            if let DiagramId::Extension(_) = module.descriptor.id {
                caps.push(Capability {
                    diagram_type: module.descriptor.id.clone(),
                    known: false,
                    registered: true,
                    syntax_parity: false,
                    semantic_parity: false,
                    render_parity: false,
                    host_parity: false,
                })
            }
        }
        caps
    }
    pub fn detect_type(&self, source: &str) -> Result<Option<DiagramType>, PeleError> {
        Ok(source::detect(&source::preprocess(source)?.text))
    }
    pub fn supports(&self, source: &str) -> bool {
        self.detect_type(source)
            .ok()
            .flatten()
            .is_some_and(|kind| self.registered(kind))
    }
    pub fn inspect(&self, source: &str, options: &PrepareOptions) -> Result<Inspection, PeleError> {
        if source.encode_utf16().count() > options.limit {
            return Err(PeleError::new(
                format!(
                    "Diagram source is longer than the limit of {} characters.",
                    options.limit
                ),
                ErrorCode::Limit,
            ));
        }
        let mut prepared = source::preprocess(source)?;
        let mut config = options.config.clone();
        for (k, v) in prepared.config.entries() {
            config.insert(k.into(), v.clone())
        }
        prepared.config = config;
        let kind = source::detect(&prepared.text);
        let encoded_text = source::encode_entities(&prepared.text);
        Ok(Inspection {
            diagram_type: kind,
            registered: kind.is_some_and(|kind| self.registered(kind)),
            prepared,
            encoded_text,
        })
    }
    pub fn parse(&self, source: &str, options: &ParseOptions) -> Result<ParsedDocument, PeleError> {
        let inspection = self.inspect(source, &options.preparation)?;
        let kind = inspection.diagram_type.ok_or_else(|| {
            PeleError::new(
                "No diagram type detected for the given text.",
                ErrorCode::UnsupportedDiagram,
            )
        })?;
        self.parse_prepared(inspection, &kind.into(), options.max_edges)
    }
    /// Explicit selection lets consumers add new languages without editing the built-in detector.
    pub fn parse_as(
        &self,
        id: &DiagramId,
        source: &str,
        options: &ParseOptions,
    ) -> Result<ParsedDocument, PeleError> {
        self.parse_prepared(
            self.inspect(source, &options.preparation)?,
            id,
            options.max_edges,
        )
    }
    fn parse_prepared(
        &self,
        inspection: Inspection,
        id: &DiagramId,
        max_edges: usize,
    ) -> Result<ParsedDocument, PeleError> {
        let registration = self
            .modules
            .iter()
            .find(|m| &m.descriptor.id == id)
            .ok_or_else(|| {
                let mut error = PeleError::new(
                    format!("Diagram type \"{id}\" is not supported yet."),
                    ErrorCode::UnsupportedDiagram,
                );
                error.diagram_type = Some(id.to_string());
                error
            })?
            .clone();
        let pre = inspection.prepared;
        let mut source = if registration.descriptor.keep_comments {
            source::encode_entities(&pre.with_comments)
        } else {
            inspection.encoded_text
        };
        source.push('\n');
        let model = registration.module.parse(ParseContext {
            source: &source,
            config: &pre.config,
            title: pre.title.as_deref(),
            line_offset: pre.line_offset,
            max_edges,
        })?;
        Ok(ParsedDocument {
            registration,
            model,
            config: pre.config,
        })
    }
    pub fn render_source(
        &self,
        source: &str,
        options: &RenderOptions,
        resources: &mut dyn RenderResources,
    ) -> Result<RenderResult, PeleError> {
        let document = self.parse(source, &options.parse)?;
        self.render_document(&document, options, resources)
    }
    pub fn render_document(
        &self,
        document: &ParsedDocument,
        options: &RenderOptions,
        resources: &mut dyn RenderResources,
    ) -> Result<RenderResult, PeleError> {
        let registration = &document.registration;
        let mut settings = options.render.clone();
        let section = registration
            .descriptor
            .section
            .as_deref()
            .unwrap_or(registration.descriptor.id.as_str());
        let fixed = document
            .config
            .get(section)
            .and_then(Value::as_object)
            .and_then(|s| s.get("useMaxWidth"))
            == Some(&Value::Bool(false));
        if settings.responsive.is_none() {
            settings.responsive = Some(!fixed)
        }
        let mut session = RenderSession {
            host: resources,
            icons: HashMap::new(),
            clock: settings.now.map(Ok),
        };
        let mut rendered = registration.module.render(
            &document.model,
            &document.config,
            &settings,
            &mut session,
        )?;
        if let Some(theme) = &settings.native_theme {
            rendered.svg = super::svg::resolved_svg(&rendered.svg, theme)?;
        }
        if rendered.svg.encode_utf16().count() > settings.output_limit() {
            let mut e = PeleError::new(
                format!(
                    "Diagram output is longer than the limit of {} characters.",
                    settings.output_limit()
                ),
                ErrorCode::Limit,
            );
            e.diagram_type = Some(registration.descriptor.id.to_string());
            return Err(e);
        }
        for link in &mut rendered.links {
            link.id = source::decode_entities(&link.id)?
        }
        Ok(RenderResult {
            diagram_type: registration.descriptor.id.clone(),
            rendered,
        })
    }
}
struct RenderSession<'a> {
    host: &'a mut dyn RenderResources,
    icons: HashMap<String, Option<TrustedSvg>>,
    clock: Option<Result<f64, PeleError>>,
}
impl RenderResources for RenderSession<'_> {
    fn text_width(&mut self, request: TextRequest<'_>) -> Result<f64, PeleError> {
        self.host.text_width(request)
    }
    fn icon(&mut self, name: &str) -> Result<Option<TrustedSvg>, PeleError> {
        if let Some(result) = self.icons.get(name) {
            return Ok(result.clone());
        }
        let result = self.host.icon(&source::decode_entities(name)?)?;
        self.icons.insert(name.into(), result.clone());
        Ok(result)
    }
    fn now_millis(&mut self) -> Result<f64, PeleError> {
        if self.clock.is_none() {
            self.clock = Some(self.host.now_millis())
        }
        self.clock.as_ref().unwrap().clone()
    }
}
impl Serialize for ParsedDocument {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.model_view()
            .map_err(serde::ser::Error::custom)?
            .serialize(s)
    }
}
