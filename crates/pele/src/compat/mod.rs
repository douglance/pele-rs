//! Full Pele compatibility interface, under construction. Legacy v1 remains separate.
//! Detection recognizes all pinned families; capabilities never imply an absent renderer.
mod engine;
mod error;
mod id;
pub use id::DiagramId;
mod module;
pub use engine::{Engine, ParsedDocument};
pub use module::*;
pub mod source;
mod value;
pub use error::{ErrorCode, PeleError};
use serde::{Deserialize, Serialize};
pub use source::{DiagramType, PreparedSource, DIAGRAM_TYPES};
pub use value::{Object, Value};
pub const UPSTREAM_REVISION: &str = "458322a5b8abb1ae57ddf127c32017ab9fb14a92";
pub const DEFAULT_LIMIT: usize = 50_000;
pub const DEFAULT_OUTPUT_LIMIT: usize = 4_000_000;
pub const DEFAULT_MAX_EDGES: usize = 5_000;
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct PrepareOptions {
    pub limit: usize,
    pub config: Object,
}
impl Default for PrepareOptions {
    fn default() -> Self {
        Self {
            limit: DEFAULT_LIMIT,
            config: Object::default(),
        }
    }
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Inspection {
    #[serde(rename = "type")]
    pub diagram_type: Option<DiagramType>,
    pub registered: bool,
    pub prepared: PreparedSource,
    pub encoded_text: String,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Capability {
    #[serde(rename = "type")]
    pub diagram_type: DiagramId,
    pub known: bool,
    pub registered: bool,
    pub syntax_parity: bool,
    pub semantic_parity: bool,
    pub render_parity: bool,
    pub host_parity: bool,
}
/// Version 2 preparation contract, shared by C and WASM. Does not promise rendering parity.
pub fn inspect_envelope(source: &str, options_json: &str) -> (u32, String) {
    let result = if options_json.trim().is_empty() {
        Ok(PrepareOptions::default())
    } else {
        serde_json::from_str::<PrepareOptions>(options_json).map_err(|e| {
            PeleError::new(
                format!("Invalid preparation options: {e}"),
                ErrorCode::Semantic,
            )
        })
    }
    .and_then(|options| Engine::empty().inspect(source, &options));
    match result {
        Ok(result) => (0, success_envelope(&result)),
        Err(error) => (
            if error.code == ErrorCode::Limit { 2 } else { 1 },
            error_envelope(&error),
        ),
    }
}
#[derive(Serialize)]
struct SuccessEnvelope<'a, T> {
    version: u32,
    ok: bool,
    result: &'a T,
}
#[derive(Serialize)]
struct ErrorEnvelope<'a> {
    version: u32,
    ok: bool,
    error: &'a PeleError,
}
fn success_envelope<T: Serialize>(result: &T) -> String {
    serde_json::to_string(&SuccessEnvelope {
        version: 2,
        ok: true,
        result,
    })
    .expect("serializable engine value")
}
pub fn error_envelope(error: &PeleError) -> String {
    serde_json::to_string(&ErrorEnvelope {
        version: 2,
        ok: false,
        error,
    })
    .expect("serializable error")
}
pub fn capabilities_json() -> String {
    success_envelope(&Engine::empty().capabilities())
}
