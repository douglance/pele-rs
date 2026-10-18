//! First additive foreign-provider lane. Opaque host models remain on their owner.
//! No Send/Sync promises, model reflection, graph projection, or host VM.
use super::{
    calendar, compatibility_input, public, registry, render_numbers, PeleHostResourcesV3,
    PeleRegistry, PeleRenderNumbersV9, PeleResult,
};
use pele::compat::{
    self,
    public::{HostExceptionToken, PublicError},
    DiagramType, EdgeLimit, Engine, ErrorCode, ModuleDescriptor, Object, PeleError,
    RenderResources, Rendered, TextRequest, Value,
};
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    ffi::c_void,
    panic::{catch_unwind, AssertUnwindSafe},
    ptr,
    rc::Rc,
    sync::{Mutex, MutexGuard},
};

const VERSION: u32 = 10;
const MAX_REPLY: usize = 64 * 1024 * 1024;
const MAX_OPTIONS: usize = 1024 * 1024;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PeleForeignBytesV10 {
    pub owner: *mut c_void,
    pub data: *const u8,
    pub len: usize,
    pub release: Option<unsafe extern "C" fn(*mut c_void)>,
}
impl Default for PeleForeignBytesV10 {
    fn default() -> Self {
        Self {
            owner: ptr::null_mut(),
            data: ptr::null(),
            len: 0,
            release: None,
        }
    }
}
#[repr(C)]
pub struct PeleForeignReplyV10 {
    pub status: u32,
    pub exception: u32,
    pub model: *mut c_void,
    /// describe/error: JSON; render: SVG; parse: absent.
    pub payload: PeleForeignBytesV10,
    /// Render-only JSON LinkInfo array; absent means [].
    pub links: PeleForeignBytesV10,
    pub width_bits: u64,
    pub height_bits: u64,
}
impl Default for PeleForeignReplyV10 {
    fn default() -> Self {
        Self {
            status: 0,
            exception: 0,
            model: ptr::null_mut(),
            payload: Default::default(),
            links: Default::default(),
            width_bits: 0,
            height_bits: 0,
        }
    }
}
#[repr(C)]
pub struct PeleProviderParseV10 {
    pub source: *const u8,
    pub source_len: usize,
    pub config: *const u8,
    pub config_len: usize,
    pub title: *const u8,
    pub title_len: usize,
    pub has_title: u32,
    pub line_offset: u64,
    /// 1 null or 2 IEEE number; undefined already resolved to 5000.
    pub edge_kind: u32,
    pub edge_bits: u64,
}
#[repr(C)]
pub struct PeleProviderRenderV10 {
    pub config: *const u8,
    pub config_len: usize,
    /// Original passive options, with responsive resolved. Numeric fields use numbers.
    pub options: *const u8,
    pub options_len: usize,
    pub numbers: PeleRenderNumbersV9,
    pub output_kind: u32,
    pub output_bits: u64,
}
/// Borrowed only inside a synchronous callback. Never retain this pointer.
pub struct PeleForeignCallV10<'a> {
    context: *mut c_void,
    resources: Option<&'a mut dyn RenderResources>,
    font_family: Option<&'a str>,
    font_family_mono: Option<&'a str>,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PeleDiagramProviderV10 {
    pub version: u32,
    pub struct_size: u32,
    pub context: *mut c_void,
    pub retain: Option<unsafe extern "C" fn(*mut c_void)>,
    pub release: Option<unsafe extern "C" fn(*mut c_void)>,
    pub release_model: Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
    pub describe: Option<
        unsafe extern "C" fn(
            *mut c_void,
            *mut PeleForeignCallV10<'_>,
            u32,
            *mut PeleForeignReplyV10,
        ),
    >,
    pub parse: Option<
        unsafe extern "C" fn(
            *mut c_void,
            *mut PeleForeignCallV10<'_>,
            *const PeleProviderParseV10,
            *mut PeleForeignReplyV10,
        ),
    >,
    pub render: Option<
        unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            *mut PeleForeignCallV10<'_>,
            *const PeleProviderRenderV10,
            *mut PeleForeignReplyV10,
        ),
    >,
    pub reserved: [u64; 2],
}
pub(super) struct ForeignRegistration {
    id: DiagramType,
    table: PeleDiagramProviderV10,
}
impl Drop for ForeignRegistration {
    fn drop(&mut self) {
        unsafe { self.table.release.expect("validated")(self.table.context) }
    }
}
#[derive(Default)]
pub(super) struct ProviderEntries(RefCell<Vec<Rc<ForeignRegistration>>>);
impl ProviderEntries {
    pub(super) fn is_empty(&self) -> bool {
        self.0.borrow().is_empty()
    }
    pub(super) fn get(&self, id: DiagramType) -> Option<Rc<ForeignRegistration>> {
        self.0.borrow().iter().find(|p| p.id == id).cloned()
    }
    pub(super) fn remove(&self, id: DiagramType) {
        let removed = {
            let mut entries = self.0.borrow_mut();
            entries
                .iter()
                .position(|p| p.id == id)
                .map(|i| entries.remove(i))
        };
        drop(removed);
    }
    fn insert(&self, provider: Rc<ForeignRegistration>) {
        let old = {
            let mut entries = self.0.borrow_mut();
            if let Some(index) = entries.iter().position(|p| p.id == provider.id) {
                Some(std::mem::replace(&mut entries[index], provider))
            } else {
                entries.push(provider);
                None
            }
        };
        drop(old);
    }
}
struct ForeignDocument {
    provider: Rc<ForeignRegistration>,
    model: *mut c_void,
    config: Object,
    revision: u64,
}
impl Drop for ForeignDocument {
    fn drop(&mut self) {
        unsafe {
            self.provider.table.release_model.expect("validated")(
                self.provider.table.context,
                self.model,
            )
        }
    }
}
/// Additive prototype handle; intentionally distinct from graph-backed v4 documents.
pub struct PeleProviderDocument {
    document: Mutex<ForeignDocument>,
}
struct Reply<'a> {
    value: PeleForeignReplyV10,
    provider: &'a ForeignRegistration,
}
impl<'a> Reply<'a> {
    fn new(provider: &'a ForeignRegistration) -> Self {
        Self {
            value: Default::default(),
            provider,
        }
    }
    fn error(&self, context: *mut c_void) -> Result<Option<PublicError>, PeleError> {
        validate_bytes(&self.value.payload)?;
        validate_bytes(&self.value.links)?;
        if self.value.payload.len.saturating_add(self.value.links.len) > MAX_REPLY {
            return Err(PeleError::new(
                "Foreign provider combined reply exceeds 64 MiB budget",
                ErrorCode::Limit,
            ));
        }
        if self.value.status != 0 && !self.value.model.is_null() {
            return Err(public::invalid(
                "Foreign provider returned a model with an error",
            ));
        }
        match self.value.status {
            0 if self.value.exception == 0 => Ok(None),
            1 if self.value.exception == 0 => {
                let data = unsafe { bytes(&self.value.payload) }?;
                let error = serde_json::from_slice::<PeleError>(data).map_err(|_| {
                    public::invalid("Foreign provider returned an invalid PeleError")
                })?;
                Ok(Some(error.into()))
            }
            2 if self.value.exception != 0 && !context.is_null() => Ok(Some(
                PublicError::HostException(HostExceptionToken(self.value.exception)),
            )),
            _ => Err(public::invalid(
                "Foreign provider returned an invalid status or exception token",
            )),
        }
    }
}
impl Drop for Reply<'_> {
    fn drop(&mut self) {
        unsafe {
            if !self.value.model.is_null() {
                self.provider.table.release_model.expect("validated")(
                    self.provider.table.context,
                    self.value.model,
                )
            }
            for value in [&self.value.payload, &self.value.links] {
                if let Some(release) = value.release {
                    release(value.owner)
                }
            }
        }
    }
}
fn validate_bytes(value: &PeleForeignBytesV10) -> Result<(), PeleError> {
    if value.len > MAX_REPLY {
        return Err(PeleError::new(
            "Foreign provider reply exceeds 64 MiB budget",
            ErrorCode::Limit,
        ));
    }
    if (value.len != 0 && value.data.is_null())
        || ((!value.data.is_null() || !value.owner.is_null() || value.len != 0)
            && value.release.is_none())
    {
        return Err(public::invalid(
            "Foreign provider returned invalid owned bytes",
        ));
    }
    Ok(())
}
unsafe fn bytes(value: &PeleForeignBytesV10) -> Result<&[u8], PeleError> {
    validate_bytes(value)?;
    Ok(if value.len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(value.data, value.len) }
    })
}
fn json(value: &impl Serialize, budget: usize) -> Result<Vec<u8>, PeleError> {
    let data = serde_json::to_vec(value)
        .map_err(|_| public::invalid("Foreign provider JSON encoding failed"))?;
    if data.len() > budget {
        return Err(PeleError::new(
            "Foreign provider metadata exceeds budget",
            ErrorCode::Limit,
        ));
    }
    Ok(data)
}
fn config_json(config: &Object) -> Result<Vec<u8>, PeleError> {
    let mut pending: Vec<_> = config
        .entries()
        .into_iter()
        .map(|(_, value)| value)
        .collect();
    while let Some(value) = pending.pop() {
        match value {
            Value::Number(value) if !value.is_finite() => {
                return Err(public::invalid(
                    "Foreign provider prototype requires finite Config numbers",
                ))
            }
            Value::Array(values) => pending.extend(values),
            Value::Object(values) => {
                pending.extend(values.entries().into_iter().map(|(_, value)| value))
            }
            _ => (),
        }
    }
    json(config, MAX_OPTIONS)
}
#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
struct Description {
    section: Option<String>,
    keep_comments: bool,
}
fn describe(
    provider: &ForeignRegistration,
    phase: u32,
    context: *mut c_void,
) -> Result<Description, PublicError> {
    let mut reply = Reply::new(provider);
    let mut call = PeleForeignCallV10 {
        context,
        resources: None,
        font_family: None,
        font_family_mono: None,
    };
    unsafe {
        provider.table.describe.expect("validated")(
            provider.table.context,
            &mut call,
            phase,
            &mut reply.value,
        )
    };
    if let Some(error) = reply.error(context)? {
        return Err(error);
    }
    if !reply.value.model.is_null() || reply.value.payload.len > 4096 || reply.value.links.len != 0
    {
        return Err(
            public::invalid("Foreign provider returned invalid descriptor metadata").into(),
        );
    }
    serde_json::from_slice(unsafe { bytes(&reply.value.payload) }?)
        .map_err(|_| public::invalid("Foreign provider returned an invalid descriptor").into())
}
unsafe fn document<'a>(
    handle: *const PeleProviderDocument,
) -> Result<MutexGuard<'a, ForeignDocument>, PeleError> {
    let owner = unsafe { handle.as_ref() }
        .ok_or_else(|| public::invalid("Foreign provider document is null"))?;
    owner
        .document
        .try_lock()
        .map_err(|_| public::invalid("Foreign provider document is already in use or unavailable"))
}
fn status(document: &ForeignDocument) -> serde_json::Value {
    serde_json::json!({"type":document.provider.id.as_str(), "revision":document.revision.to_string(), "modelKind":"foreign"})
}
fn operation_error(error: PublicError, revision: Option<u64>) -> Result<PeleResult, PeleError> {
    public::success(
        serde_json::json!({"revision":revision.map(|v|v.to_string()), "result":{"Err":error}}),
    )
}

/// # Safety
/// Registry and packet must be live and readable. Provider functions must not unwind.
#[no_mangle]
pub unsafe extern "C" fn pele_registry_register_provider_v10(
    handle: *mut PeleRegistry,
    id: *const u8,
    len: usize,
    table: *const PeleDiagramProviderV10,
) -> *mut PeleResult {
    public::bounded_call(|| {
        let id = unsafe { compatibility_input(id, len, 4096) }?;
        let id = registry::builtin(id)
            .ok_or_else(|| public::invalid("Unknown foreign provider diagram type"))?;
        if table.is_null() {
            return Err(public::invalid("Foreign provider packet is null"));
        }
        let header = table.cast::<u32>();
        let version = unsafe { ptr::read_unaligned(header) };
        let size = unsafe { ptr::read_unaligned(header.add(1)) };
        if version != VERSION || size as usize != std::mem::size_of::<PeleDiagramProviderV10>() {
            return Err(public::invalid(
                "Foreign provider packet version or size is invalid",
            ));
        }
        let table = unsafe { ptr::read_unaligned(table) };
        if table.retain.is_none()
            || table.release.is_none()
            || table.release_model.is_none()
            || table.describe.is_none()
            || table.parse.is_none()
            || table.render.is_none()
            || table.reserved != [0, 0]
        {
            return Err(public::invalid(
                "Foreign provider packet callbacks or reserved fields are invalid",
            ));
        }
        let _guard = unsafe { registry::registry(handle) }?;
        let result = public::success(true)?;
        unsafe { table.retain.expect("validated")(table.context) };
        let provider = Rc::new(ForeignRegistration { id, table });
        unsafe { &*handle }.providers.insert(provider);
        Ok(result)
    })
}
/// # Safety
/// Buffers and invocation context borrow synchronously; out_document is writable.
#[no_mangle]
pub unsafe extern "C" fn pele_registry_parse_provider_v10(
    handle: *const PeleRegistry,
    source: *const u8,
    source_len: usize,
    options: *const u8,
    options_len: usize,
    source_kind: u32,
    source_bits: u64,
    edge_kind: u32,
    edge_bits: u64,
    context: *mut c_void,
    out_document: *mut *mut PeleProviderDocument,
) -> *mut PeleResult {
    if !out_document.is_null() {
        unsafe { *out_document = ptr::null_mut() }
    }
    public::bounded_call(|| {
        let out = unsafe { out_document.as_mut() }
            .ok_or_else(|| public::invalid("Foreign provider document output is null"))?;
        render_numbers::check_lengths(options_len, Some(source_len))?;
        let limits = public::decode_parse_limits(source_kind, source_bits, edge_kind, edge_bits)?;
        let source =
            unsafe { compatibility_input(source, source_len, compat::MAX_PUBLIC_SOURCE_BYTES) }?;
        let options = unsafe { compatibility_input(options, options_len, MAX_OPTIONS) }?;
        let options = public::parse_limits_options(
            options,
            limits.source.is_some(),
            limits.max_edges.is_some(),
        )?;
        let _guard = unsafe { registry::registry(handle) }?;
        let inspection = Engine::empty().inspect_with_limits(source, &options, limits)?;
        let id = inspection.diagram_type.ok_or_else(|| {
            PeleError::new(
                "No diagram type detected for the given text.",
                ErrorCode::UnsupportedDiagram,
            )
        })?;
        let provider = unsafe { &*handle }.providers.get(id).ok_or_else(|| {
            let message = if _guard.registered(id) {
                format!(
                    "Diagram type \"{}\" has no foreign provider; use the built-in parse API.",
                    id.as_str()
                )
            } else {
                format!("Diagram type \"{}\" is not supported yet.", id.as_str())
            };
            let mut error = PeleError::new(message, ErrorCode::UnsupportedDiagram);
            error.diagram_type = Some(id.as_str().to_owned());
            error
        })?;
        let description = match describe(&provider, 0, context) {
            Ok(value) => value,
            Err(error) => return operation_error(error, None),
        };
        let prepared = inspection.into_provider_input(description.keep_comments);
        if prepared.source.len() > MAX_REPLY {
            return Err(PeleError::new(
                "Foreign prepared source exceeds callback budget",
                ErrorCode::Limit,
            ));
        }
        let config = config_json(&prepared.config)?;
        let (edge_kind, edge_bits) = match limits.max_edges {
            Some(EdgeLimit::Null) => (1, 0),
            Some(EdgeLimit::Number(value)) => (2, value.to_bits()),
            Some(EdgeLimit::Undefined) => (2, (compat::DEFAULT_MAX_EDGES as f64).to_bits()),
            None => (2, (options.max_edges as f64).to_bits()),
        };
        let title = prepared.title.as_deref();
        let request = PeleProviderParseV10 {
            source: prepared.source.as_ptr(),
            source_len: prepared.source.len(),
            config: config.as_ptr(),
            config_len: config.len(),
            title: title.map_or(ptr::null(), |v| v.as_ptr()),
            title_len: title.map_or(0, str::len),
            has_title: u32::from(title.is_some()),
            line_offset: prepared.line_offset as u64,
            edge_kind,
            edge_bits,
        };
        let mut call = PeleForeignCallV10 {
            context,
            resources: None,
            font_family: None,
            font_family_mono: None,
        };
        let mut reply = Reply::new(&provider);
        unsafe {
            provider.table.parse.expect("validated")(
                provider.table.context,
                &mut call,
                &request,
                &mut reply.value,
            )
        };
        if let Some(error) = reply.error(context)? {
            return operation_error(error, None);
        }
        if reply.value.model.is_null() || reply.value.payload.len != 0 || reply.value.links.len != 0
        {
            return Err(public::invalid(
                "Foreign provider returned an invalid parse model",
            ));
        }
        let model = std::mem::replace(&mut reply.value.model, ptr::null_mut());
        drop(reply);
        let document = ForeignDocument {
            provider,
            model,
            config: prepared.config,
            revision: 0,
        };
        let result = public::success(serde_json::json!({"result":{"Ok":status(&document)}}))?;
        *out = Box::into_raw(Box::new(PeleProviderDocument {
            document: Mutex::new(document),
        }));
        Ok(result)
    })
}
/// # Safety
/// Live idle document and writable out_model; borrowed pointer ends at document free.
#[no_mangle]
pub unsafe extern "C" fn pele_provider_model_v10(
    handle: *const PeleProviderDocument,
    out_model: *mut *mut c_void,
) -> *mut PeleResult {
    if !out_model.is_null() {
        unsafe { *out_model = ptr::null_mut() }
    }
    public::bounded_call(|| {
        let out = unsafe { out_model.as_mut() }
            .ok_or_else(|| public::invalid("Foreign provider model output is null"))?;
        let document = unsafe { document(handle) }?;
        let result = public::success(status(&document))?;
        *out = document.model;
        Ok(result)
    })
}
/// # Safety
/// Borrow a live idle document; this does not serialize its host model.
#[no_mangle]
pub unsafe extern "C" fn pele_provider_status_v10(
    handle: *const PeleProviderDocument,
) -> *mut PeleResult {
    public::bounded_call(|| {
        let document = unsafe { document(handle) }?;
        public::success(status(&document))
    })
}
/// # Safety
/// Free once, after all operations/callbacks return. Null is allowed.
#[no_mangle]
pub unsafe extern "C" fn pele_provider_document_free_v10(handle: *mut PeleProviderDocument) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if !handle.is_null() {
            drop(unsafe { Box::from_raw(handle) });
        }
    }));
}

struct UniqueOptions(serde_json::Map<String, serde_json::Value>);
impl<'de> Deserialize<'de> for UniqueOptions {
    fn deserialize<D: serde::Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = UniqueOptions;
            fn expecting(&self, out: &mut std::fmt::Formatter) -> std::fmt::Result {
                out.write_str("render options object")
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Self::Value, A::Error> {
                let mut result = serde_json::Map::new();
                while let Some((key, value)) = map.next_entry::<String, serde_json::Value>()? {
                    if result.insert(key, value).is_some() {
                        return Err(serde::de::Error::custom("Duplicate foreign render option"));
                    }
                }
                Ok(UniqueOptions(result))
            }
        }
        decoder.deserialize_map(Visitor)
    }
}
fn numeric_packet(settings: &compat::RenderSettings) -> PeleRenderNumbersV9 {
    let values = [
        settings.font_size,
        settings.max_width,
        settings.direction_breakpoint,
        settings.padding,
        settings.now,
    ];
    let bits: Vec<_> = values.iter().map(|v| v.map_or(0, f64::to_bits)).collect();
    PeleRenderNumbersV9 {
        version: 9,
        struct_size: 56,
        present: values
            .iter()
            .enumerate()
            .fold(0, |m, (i, v)| m | (u64::from(v.is_some()) << i)),
        font_size_bits: bits[0],
        max_width_bits: bits[1],
        direction_breakpoint_bits: bits[2],
        padding_bits: bits[3],
        now_bits: bits[4],
    }
}
/// # Safety
/// All inputs and host callbacks borrow synchronously. Revision failures do not dispatch.
#[no_mangle]
pub unsafe extern "C" fn pele_provider_render_v10(
    handle: *mut PeleProviderDocument,
    expected_revision: u64,
    options: *const u8,
    options_len: usize,
    numbers: *const PeleRenderNumbersV9,
    output_kind: u32,
    output_bits: u64,
    resources: *const PeleHostResourcesV3,
    context: *mut c_void,
) -> *mut PeleResult {
    public::bounded_call(|| {
        render_numbers::check_lengths(options_len, None)?;
        let _ =
            unsafe { render_numbers::options(ptr::null(), 0, numbers, output_kind, output_bits) }?;
        let raw = unsafe { compatibility_input(options, options_len, MAX_OPTIONS) }?;
        let mut passive: serde_json::Map<String, serde_json::Value> = if raw.trim().is_empty() {
            Default::default()
        } else {
            serde_json::from_str::<UniqueOptions>(raw)
                .map_err(|_| {
                    public::invalid("Invalid or duplicate foreign provider render options")
                })?
                .0
        };
        let mut decoding = passive.clone();
        for key in ["limit", "maxEdges", "config"] {
            decoding.remove(key);
        }
        let decoding = json(&decoding, MAX_OPTIONS)?;
        let (settings, output) = unsafe {
            render_numbers::options(
                decoding.as_ptr(),
                decoding.len(),
                numbers,
                output_kind,
                output_bits,
            )
        }?;
        for key in [
            "fontSize",
            "maxWidth",
            "directionBreakpoint",
            "padding",
            "now",
        ] {
            passive.remove(key);
        }
        if output_kind != 3 {
            passive.remove("outputLimit");
        }
        let mut document = unsafe { document(handle) }?;
        if document.revision != expected_revision {
            return Err(public::invalid("Stale foreign provider model revision"));
        }
        let next = document.revision.checked_add(1).ok_or_else(|| {
            PeleError::new("Foreign provider revision exhausted", ErrorCode::Limit)
        })?;
        let config = config_json(&document.config)?;
        let provider = document.provider.clone();
        let description = match describe(&provider, 1, context) {
            Ok(value) => value,
            Err(error) => {
                document.revision = next;
                return operation_error(error, Some(next));
            }
        };
        let descriptor = ModuleDescriptor {
            id: provider.id.into(),
            section: description.section,
            keep_comments: description.keep_comments,
        };
        let mut callback_error = None;
        let result = unsafe {
            calendar::with_resources(resources, |host| {
                compat::render_provider_bounded(
                    &descriptor,
                    &document.config,
                    &settings,
                    output,
                    host,
                    |settings, resources| {
                        if let Some(responsive) = settings.responsive {
                            passive
                                .insert("responsive".into(), serde_json::Value::Bool(responsive));
                        }
                        let encoded = json(&passive, MAX_OPTIONS)?;
                        let request = PeleProviderRenderV10 {
                            config: config.as_ptr(),
                            config_len: config.len(),
                            options: encoded.as_ptr(),
                            options_len: encoded.len(),
                            numbers: numeric_packet(settings),
                            output_kind,
                            output_bits,
                        };
                        let mut call = PeleForeignCallV10 {
                            context,
                            resources: Some(resources),
                            font_family: settings.font_family.as_deref(),
                            font_family_mono: settings.font_family_mono.as_deref(),
                        };
                        let mut reply = Reply::new(&provider);
                        provider.table.render.expect("validated")(
                            provider.table.context,
                            document.model,
                            &mut call,
                            &request,
                            &mut reply.value,
                        );
                        if let Some(error) = reply.error(context)? {
                            callback_error = Some(error);
                            return Err(public::invalid("Foreign provider operation stopped"));
                        }
                        if !reply.value.model.is_null() {
                            return Err(public::invalid("Foreign renderer returned a parse model"));
                        }
                        let svg = std::str::from_utf8(bytes(&reply.value.payload)?)
                            .map_err(|_| public::invalid("Foreign renderer SVG is not UTF-8"))?
                            .to_owned();
                        let links = if reply.value.links.len == 0 {
                            vec![]
                        } else {
                            serde_json::from_slice(bytes(&reply.value.links)?).map_err(|_| {
                                public::invalid("Foreign renderer links are invalid")
                            })?
                        };
                        Ok(Rendered {
                            svg,
                            links,
                            width: f64::from_bits(reply.value.width_bits),
                            height: f64::from_bits(reply.value.height_bits),
                        })
                    },
                )
            })
        };
        document.revision = next;
        match result {
            Ok(result) => public::success(
                serde_json::json!({"revision":next.to_string(),"result":{"Ok":compat::render_result_transport(&result)}}),
            ),
            Err(error) => {
                operation_error(callback_error.unwrap_or_else(|| error.into()), Some(next))
            }
        }
    })
}
/// # Safety
/// call must be the active callback's borrowed context.
#[no_mangle]
pub unsafe extern "C" fn pele_foreign_call_context_v10(
    call: *const PeleForeignCallV10<'_>,
) -> *mut c_void {
    unsafe { call.as_ref() }.map_or(ptr::null_mut(), |call| call.context)
}
/// # Safety
/// Active render call and borrowed UTF-8 text. Output is initialized before errors.
#[no_mangle]
pub unsafe extern "C" fn pele_foreign_text_width_v10(
    call: *mut PeleForeignCallV10<'_>,
    text: *const u8,
    len: usize,
    size_bits: u64,
    style: u32,
    out_bits: *mut u64,
) -> *mut PeleResult {
    if !out_bits.is_null() {
        unsafe { *out_bits = 0 }
    }
    public::bounded_call(|| {
        let out = unsafe { out_bits.as_mut() }
            .ok_or_else(|| public::invalid("Foreign text width output is null"))?;
        let text = unsafe { compatibility_input(text, len, MAX_OPTIONS) }?;
        if style > 7 {
            return Err(public::invalid("Foreign text style is invalid"));
        }
        let call =
            unsafe { call.as_mut() }.ok_or_else(|| public::invalid("Foreign call is null"))?;
        let resources = call
            .resources
            .as_deref_mut()
            .ok_or_else(|| public::invalid("Foreign resources are unavailable outside render"))?;
        let value = resources.text_width(TextRequest {
            text,
            size: f64::from_bits(size_bits),
            style: style as u8,
            font_family: call.font_family,
            font_family_mono: call.font_family_mono,
        })?;
        let result = public::success(true)?;
        *out = value.to_bits();
        Ok(result)
    })
}
