//! Owner-confined JavaScript providers. Rust owns preparation and render policy;
//! host objects stay in JS and are never projected through a built-in model codec.
use js_sys::{Array, Function, Object as JsObject, Reflect};
use pele::compat::{
    self, DiagramType, EdgeLimit, ErrorCode, LinkInfo, ModuleDescriptor, Object, PeleError,
    Rendered, Value,
};
use wasm_bindgen::prelude::*;

const OPTIONS_BYTES: usize = 1024 * 1024;
const OUTPUT_BYTES: usize = 64 * 1024 * 1024;
fn invalid(message: &str) -> PeleError {
    PeleError::new(message, ErrorCode::Semantic)
}
pub(super) fn native(error: PeleError) -> JsValue {
    JsValue::from_str(&super::render_numbers::failure(&error))
}
// Keep arbitrary JS thrown values distinct from native JSON error envelopes.
fn host(error: JsValue) -> JsValue {
    let wrapped = Array::new();
    wrapped.push(&error);
    wrapped.into()
}
fn get(object: &JsValue, key: &str) -> Result<JsValue, JsValue> {
    Reflect::get(object, &JsValue::from_str(key)).map_err(host)
}
fn set(object: &JsValue, key: &str, value: &JsValue) -> Result<(), JsValue> {
    Reflect::set(object, &JsValue::from_str(key), value)
        .map(|_| ())
        .map_err(host)
}
fn method(provider: &JsValue, name: &str) -> Result<Function, JsValue> {
    get(provider, name)?.dyn_into::<Function>().map_err(|_| {
        host(js_sys::TypeError::new(&format!("diagram.{name} is not a function")).into())
    })
}
fn text(value: JsValue, name: &str) -> Result<String, JsValue> {
    value.as_string().ok_or_else(|| native(invalid(name)))
}
pub(super) fn config_json(config: &Object) -> Result<String, PeleError> {
    let mut pending: Vec<_> = config
        .entries()
        .into_iter()
        .map(|(_, value)| value)
        .collect();
    while let Some(value) = pending.pop() {
        match value {
            Value::Number(value) if !value.is_finite() => {
                return Err(invalid("Foreign provider Config requires finite numbers"));
            }
            Value::Array(values) => pending.extend(values),
            Value::Object(values) => {
                pending.extend(values.entries().into_iter().map(|(_, value)| value))
            }
            _ => (),
        }
    }
    let json = serde_json::to_string(config).map_err(|_| invalid("Invalid provider Config"))?;
    if json.len() > OPTIONS_BYTES {
        return Err(PeleError::new(
            "Foreign provider Config exceeds transport budget",
            ErrorCode::Limit,
        ));
    }
    Ok(json)
}

struct Pending {
    provider: JsValue,
    config: JsValue,
    arguments: Array,
}
#[wasm_bindgen(js_name = ForeignDocument)]
pub struct ForeignDocument {
    kind: DiagramType,
    // Cleared when values transfer to the JS WeakMap. No permanent externref to
    // the model/provider can root the WeakMap key through a finalizer's handle.
    pending: Option<Pending>,
}
pub(super) fn prepare(
    kind: DiagramType,
    provider: JsValue,
    inspection: compat::Inspection,
    options: &compat::ParseOptions,
    limits: compat::ParseLimits,
) -> Result<ForeignDocument, JsValue> {
    let keep = get(&provider, "keepComments")?.is_truthy();
    let input = inspection.into_provider_input(keep);
    let config = js_sys::JSON::parse(&config_json(&input.config).map_err(native)?).map_err(host)?;
    let edges = match limits.max_edges {
        Some(EdgeLimit::Null) => JsValue::NULL,
        Some(EdgeLimit::Number(value)) => JsValue::from_f64(value),
        Some(EdgeLimit::Undefined) => JsValue::from_f64(compat::DEFAULT_MAX_EDGES as f64),
        None => JsValue::from_f64(options.max_edges as f64),
    };
    let limit_object = JsObject::new();
    set(&limit_object, "maxEdges", &edges)?;
    let arguments = Array::new();
    arguments.push(&JsValue::from_str(&input.source));
    arguments.push(&config);
    arguments.push(
        &input
            .title
            .map_or(JsValue::UNDEFINED, |v| JsValue::from_str(&v)),
    );
    arguments.push(&JsValue::from_f64(input.line_offset as f64));
    arguments.push(&limit_object);
    Ok(ForeignDocument {
        kind,
        pending: Some(Pending {
            provider,
            config,
            arguments,
        }),
    })
}
pub(super) fn retained_policy(kind: DiagramType) -> ForeignDocument {
    ForeignDocument {
        kind,
        pending: None,
    }
}
#[wasm_bindgen]
impl ForeignDocument {
    /// One ownership transfer to JS; values are retained in its weak-key entry.
    pub fn take_values(&mut self) -> Result<Array, JsValue> {
        let pending = self
            .pending
            .take()
            .ok_or_else(|| native(invalid("Foreign model values were already transferred")))?;
        // NativeRegistry's borrow has ended before caller code executes, so
        // ordinary register/close calls inside parse cannot invalidate selection.
        let model = method(&pending.provider, "parse")?
            .apply(&pending.provider, &pending.arguments)
            .map_err(host)?;
        let values = Array::new();
        values.push(&model);
        values.push(&pending.provider);
        values.push(&pending.config);
        Ok(values)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &self,
        model: JsValue,
        provider: JsValue,
        config: JsValue,
        config_json: &str,
        callback_options: JsValue,
        json: &str,
        mask: u32,
        numbers: &[f64],
        output_kind: u32,
        output_value: f64,
    ) -> Result<JsValue, JsValue> {
        if config_json.len() > OPTIONS_BYTES {
            return Err(native(PeleError::new(
                "Foreign provider Config exceeds transport budget",
                ErrorCode::Limit,
            )));
        }
        let retained_config: Object = serde_json::from_str(config_json)
            .map_err(|_| native(invalid("Invalid finite provider Config")))?;
        let (options, output) =
            super::render_numbers::options(json, mask, numbers, output_kind, output_value)
                .map_err(native)?;
        let section = get(&provider, "section")?;
        let section = if section.is_null() || section.is_undefined() {
            None
        } else {
            Some(text(section, "Foreign provider section must be a string")?)
        };
        let descriptor = ModuleDescriptor {
            id: self.kind.into(),
            section,
            keep_comments: false,
        };
        let mut thrown = None;
        let mut returned = JsValue::UNDEFINED;
        let mut returned_links = Array::new();
        let result = compat::render_provider_bounded(
            &descriptor,
            &retained_config,
            &options,
            output,
            &mut compat::text::MetricsResources,
            |settings, _resources| {
                let run = || -> Result<_, JsValue> {
                    set(
                        &callback_options,
                        "responsive",
                        &JsValue::from_bool(settings.responsive.unwrap_or(true)),
                    )?;
                    let value = method(&provider, "render")?
                        .call3(&provider, &model, &config, &callback_options)
                        .map_err(host)?;
                    let svg = text(get(&value, "svg")?, "Foreign provider SVG must be a string")?;
                    if svg.len() > OUTPUT_BYTES {
                        return Err(native(PeleError::new(
                            "Foreign provider SVG exceeds transport budget",
                            ErrorCode::Limit,
                        )));
                    }
                    let width = get(&value, "width")?.as_f64().ok_or_else(|| {
                        native(invalid("Foreign provider width must be a number"))
                    })?;
                    let height = get(&value, "height")?.as_f64().ok_or_else(|| {
                        native(invalid("Foreign provider height must be a number"))
                    })?;
                    let js_links = get(&value, "links")?;
                    if !Array::is_array(&js_links) {
                        return Err(native(invalid("Foreign provider links must be an array")));
                    }
                    let js_links = js_links.unchecked_into::<Array>();
                    if js_links.length() > 1_000_000 {
                        return Err(native(PeleError::new(
                            "Foreign provider links exceed work budget",
                            ErrorCode::Limit,
                        )));
                    }
                    let mut links = Vec::new();
                    let mut bytes = svg.len();
                    for entry in js_links.iter() {
                        let link = LinkInfo {
                            id: text(get(&entry, "id")?, "Foreign link id must be a string")?,
                            href: text(get(&entry, "href")?, "Foreign link href must be a string")?,
                            internal: get(&entry, "internal")?.as_bool().ok_or_else(|| {
                                native(invalid("Foreign link internal must be a boolean"))
                            })?,
                        };
                        bytes = bytes
                            .saturating_add(link.id.len())
                            .saturating_add(link.href.len());
                        if bytes > OUTPUT_BYTES {
                            return Err(native(PeleError::new(
                                "Foreign provider output exceeds transport budget",
                                ErrorCode::Limit,
                            )));
                        }
                        links.push(link);
                    }
                    Ok((
                        value,
                        js_links,
                        Rendered {
                            svg,
                            width,
                            height,
                            links,
                        },
                    ))
                };
                match run() {
                    Ok((value, links, rendered)) => {
                        returned = value;
                        returned_links = links;
                        Ok(rendered)
                    }
                    Err(error) => {
                        thrown = Some(error);
                        Err(invalid("Foreign provider callback stopped"))
                    }
                }
            },
        );
        if let Some(error) = thrown {
            return Err(error);
        }
        let result = result.map_err(native)?;
        for (index, link) in result.rendered.links.iter().enumerate() {
            set(
                &returned_links.get(index as u32),
                "id",
                &JsValue::from_str(&link.id),
            )?;
        }
        let output = JsObject::new();
        set(&output, "type", &JsValue::from_str(self.kind.as_str()))?;
        let output = JsObject::assign(&output, &returned.unchecked_into::<JsObject>());
        set(&output, "svg", &JsValue::from_str(&result.rendered.svg))?;
        Ok(output.into())
    }
}
/// Icon callbacks receive the same decoded authored names as built-in renderers.
#[wasm_bindgen]
pub fn foreign_decode_entities(text: &str) -> Result<String, JsValue> {
    if text.len() > OPTIONS_BYTES {
        return Err(native(PeleError::new(
            "Foreign icon name exceeds transport budget",
            ErrorCode::Limit,
        )));
    }
    compat::source::decode_entities(text).map_err(native)
}
