//! Low-level built-in diagram entries: prepared parse and raw mutable render.
use super::{wasm_registry::NativeRegistry, MutableDocument};
use pele::compat::{public::*, *};
use serde::Serialize;
use wasm_bindgen::prelude::*;
fn invalid(message: &str) -> PeleError {
    PeleError::new(message, ErrorCode::Semantic)
}
fn failure(error: PeleError) -> JsValue {
    JsValue::from_str(&super::render_numbers::failure(&error))
}
fn config(json: &str) -> Result<Object, PeleError> {
    if json.len() > 1024 * 1024 {
        return Err(PeleError::new(
            "Prepared config exceeds 1 MiB transport budget",
            ErrorCode::Limit,
        ));
    }
    serde_json::from_str(json).map_err(|_| invalid("Invalid prepared diagram config"))
}
fn edge(kind: u32, value: f64) -> Result<EdgeLimit, PeleError> {
    match kind {
        0 => Ok(EdgeLimit::Undefined),
        1 => Ok(EdgeLimit::Null),
        2 => Ok(EdgeLimit::Number(value)),
        _ => Err(invalid("Invalid prepared maxEdges kind")),
    }
}
#[wasm_bindgen]
impl NativeRegistry {
    #[allow(clippy::too_many_arguments)]
    pub fn parse_prepared(
        &self,
        id: &str,
        source: &str,
        json: &str,
        title: Option<String>,
        line_offset: f64,
        edge_kind: u32,
        edge_value: f64,
    ) -> Result<MutableDocument, JsValue> {
        let run = || -> Result<MutableDocument, PeleError> {
            if source.len() > MAX_PUBLIC_SOURCE_BYTES
                || title
                    .as_ref()
                    .is_some_and(|s| s.len() > MAX_PUBLIC_SOURCE_BYTES)
            {
                return Err(PeleError::new(
                    "Native transport: declared input exceeds budget",
                    ErrorCode::Limit,
                ));
            }
            let engine = self
                .inner
                .as_ref()
                .ok_or_else(|| invalid("Registry is closed"))?;
            if !line_offset.is_finite()
                || line_offset < 0.
                || line_offset.fract() != 0.
                || line_offset > (usize::MAX - source.len()) as f64
            {
                return Err(invalid("Prepared lineOffset must be a nonnegative integer within the native source-position range"));
            }
            let config = config(json)?;
            let limit = edge(edge_kind, edge_value)?;
            let id = DIAGRAM_TYPES
                .into_iter()
                .find(|kind| kind.as_str() == id)
                .map(DiagramId::Builtin)
                .ok_or_else(|| {
                    PeleError::new(
                        "Unknown prepared diagram type",
                        ErrorCode::UnsupportedDiagram,
                    )
                })?;
            Ok(MutableDocument {
                inner: engine.parse_prepared_as(
                    &id,
                    ParseContext {
                        source,
                        config: &config,
                        title: title.as_deref(),
                        line_offset: line_offset as usize,
                        max_edges: DEFAULT_MAX_EDGES,
                    },
                    limit,
                )?,
            })
        };
        run().map_err(failure)
    }
}
#[derive(Serialize)]
struct RawTransport<'a> {
    #[serde(flatten)]
    rendered: &'a Rendered,
    #[serde(
        rename = "$numbers",
        skip_serializing_if = "std::collections::BTreeMap::is_empty"
    )]
    numbers: std::collections::BTreeMap<&'static str, String>,
}
fn transport(rendered: &Rendered) -> RawTransport<'_> {
    let mut numbers = std::collections::BTreeMap::new();
    for (key, value) in [("width", rendered.width), ("height", rendered.height)] {
        if !value.is_finite() || (value == 0. && value.is_sign_negative()) {
            numbers.insert(key, format!("{:016x}", value.to_bits()));
        }
    }
    RawTransport { rendered, numbers }
}
#[wasm_bindgen]
impl MutableDocument {
    #[allow(clippy::too_many_arguments)]
    pub fn entry_type(&self) -> String {
        self.inner.diagram_type().as_str().into()
    }
    pub fn live_provider_view(&self) -> Result<js_sys::Array, JsValue> {
        let DiagramId::Builtin(kind) = self.inner.diagram_type() else {
            return Err(failure(invalid(
                "Foreign entry policy requires a built-in identity",
            )));
        };
        let config_json =
            super::foreign_provider::config_json(self.inner.retained_config()).map_err(failure)?;
        let config = js_sys::JSON::parse(&config_json)
            .map_err(super::callback_error)
            .map_err(failure)?;
        let values = js_sys::Array::new();
        values.push(&JsValue::from_str(kind.as_str()));
        values.push(&config);
        values.push(&super::foreign_provider::retained_policy(*kind).into());
        Ok(values)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn render_raw_operation_binary(
        &mut self,
        revision: &str,
        config_json: &str,
        json: &str,
        mask: u32,
        numbers: &[f64],
        width: Option<js_sys::Function>,
        icons: Option<js_sys::Function>,
        clock: Option<js_sys::Function>,
        inline_icons: Option<js_sys::Function>,
    ) -> Result<Vec<u8>, JsValue> {
        let run = || -> Result<Vec<u8>, PeleError> {
            let config = config(config_json)?;
            // Raw entries have no core output policy. JS removes parse-only
            // fields before decoding; numeric render fields retain raw f64.
            let (options, _) = super::render_numbers::options(json, mask, numbers, 3, 0.)?;
            let revision = revision
                .parse::<u64>()
                .map_err(|_| invalid("Invalid public model revision"))?;
            let outcome = self.inner.render_raw_operation(
                revision,
                &config,
                &options.render,
                &mut RawResources {
                    resources: super::render_numbers::NumericResources {
                        browser: super::BrowserResources { width, icons },
                        clock,
                    },
                    inline: super::BrowserResources {
                        width: None,
                        icons: inline_icons,
                    },
                },
            )?;
            let metadata = super::render_numbers::encode(&outcome.result.as_ref().map(transport))?;
            encode_binary_exchange(
                outcome.revision,
                &outcome.graph,
                &metadata,
                DEFAULT_BINARY_BUDGET,
            )
        };
        run().map_err(failure)
    }
}

struct RawResources {
    resources: super::render_numbers::NumericResources,
    inline: super::BrowserResources,
}
impl RenderResources for RawResources {
    fn text_width(&mut self, request: TextRequest<'_>) -> Result<f64, PeleError> {
        self.resources.text_width(request)
    }
    fn icon(&mut self, name: &str) -> Result<Option<TrustedSvg>, PeleError> {
        self.resources.icon(name)
    }
    fn raw_inline_icon(&mut self, name: &str) -> Result<Option<TrustedSvg>, PeleError> {
        self.inline.icon(name)
    }
    fn parse_date(&mut self, input: &str) -> Result<Option<f64>, PeleError> {
        self.resources.parse_date(input)
    }
    fn calendar_fields(&mut self, instant: f64) -> Result<Option<CalendarFields>, PeleError> {
        self.resources.calendar_fields(instant)
    }
    fn calendar_instant(&mut self, civil: CivilDateTime) -> Result<Option<f64>, PeleError> {
        self.resources.calendar_instant(civil)
    }
    fn now_millis(&mut self) -> Result<f64, PeleError> {
        self.resources.now_millis()
    }
}
