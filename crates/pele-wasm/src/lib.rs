use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn version() -> u32 {
    1
}

/// Returns a versioned JSON envelope; syntax failures are values, not traps.
#[wasm_bindgen]
pub fn render(source: &str, options_json: &str) -> String {
    pele::render_json(source, options_json)
}

/// New compatibility interface. Legacy render() continues to return version 1.
#[wasm_bindgen]
pub fn inspect(source: &str, options_json: &str) -> String {
    pele::compat::inspect_envelope(source, options_json).1
}
#[wasm_bindgen]
pub fn capabilities() -> String {
    pele::compat::capabilities_json()
}

#[wasm_bindgen(js_name=ParsedDocument)]
pub struct ParsedDocument {
    inner: pele::compat::ParsedDocument,
}
#[wasm_bindgen]
impl ParsedDocument {
    pub fn query(&self, operation: &str, arguments_json: &str) -> String {
        let run = || -> Result<pele::compat::Value, pele::compat::PeleError> {
            use pele::compat::{ErrorCode, PeleError};
            if operation.len() > 4096 || arguments_json.len() > 1024 * 1024 {
                return Err(PeleError::new(
                    "Model query input exceeds budget",
                    ErrorCode::Limit,
                ));
            }
            let arguments = serde_json::from_str(arguments_json).map_err(|_| {
                PeleError::new("Invalid model query arguments", ErrorCode::Semantic)
            })?;
            self.inner.query(operation, &arguments)
        };
        match run() {
            Ok(value) => pele::compat::success_envelope(&value),
            Err(error) => pele::compat::error_envelope(&error),
        }
    }
    pub fn model(&self) -> String {
        match pele::compat::document_envelope(&self.inner) {
            Ok(s) => s,
            Err(e) => pele::compat::error_envelope(&e),
        }
    }
    pub fn render(&self, options_json: &str) -> String {
        match pele::compat::render_options(options_json).and_then(|options| {
            pele::compat::Engine::empty().render_document(
                &self.inner,
                &options,
                &mut pele::compat::text::MetricsResources,
            )
        }) {
            Ok(rendered) => pele::compat::render_result_envelope(&rendered),
            Err(e) => pele::compat::error_envelope(&e),
        }
    }
}
#[wasm_bindgen]
pub fn parse_v2(source: &str, options_json: &str) -> Result<ParsedDocument, JsValue> {
    pele::compat::parse_options(options_json)
        .and_then(|options| pele::compat::Engine::with_builtins().parse(source, &options))
        .map(|inner| ParsedDocument { inner })
        .map_err(|e| JsValue::from_str(&pele::compat::error_envelope(&e)))
}
#[wasm_bindgen]
pub fn render_v2(source: &str, options_json: &str) -> String {
    pele::compat::render_envelope(source, options_json).1
}
#[wasm_bindgen]
pub fn builtin_capabilities() -> String {
    pele::compat::builtin_capabilities_json()
}

struct BrowserResources {
    width: Option<js_sys::Function>,
    icons: Option<js_sys::Function>,
}
fn callback_error(error: JsValue) -> pele::compat::PeleError {
    let message = js_sys::Reflect::get(&error, &JsValue::from_str("message"))
        .ok()
        .and_then(|v| v.as_string())
        .or_else(|| error.as_string())
        .unwrap_or_else(|| "Host callback failed".into());
    pele::compat::PeleError::new(message, pele::compat::ErrorCode::Semantic)
}
impl pele::compat::RenderResources for BrowserResources {
    fn text_width(
        &mut self,
        r: pele::compat::TextRequest<'_>,
    ) -> Result<f64, pele::compat::PeleError> {
        if let Some(callback) = &self.width {
            let args = js_sys::Array::new();
            args.push(&r.text.into());
            args.push(&r.size.into());
            args.push(&(r.style as u32).into());
            args.push(&r.font_family.map_or(JsValue::UNDEFINED, JsValue::from_str));
            args.push(
                &r.font_family_mono
                    .map_or(JsValue::UNDEFINED, JsValue::from_str),
            );
            callback
                .apply(&JsValue::UNDEFINED, &args)
                .map_err(callback_error)?
                .as_f64()
                .ok_or_else(|| callback_error("Text measurer must return a number".into()))
        } else {
            Ok(pele::compat::text::metrics_width(r.text, r.size, r.style))
        }
    }
    fn icon(
        &mut self,
        name: &str,
    ) -> Result<Option<pele::compat::TrustedSvg>, pele::compat::PeleError> {
        let Some(callback) = &self.icons else {
            return Ok(None);
        };
        let value = callback
            .call1(&JsValue::UNDEFINED, &name.into())
            .map_err(callback_error)?;
        if value.is_null() || value.is_undefined() {
            return Ok(None);
        }
        value
            .as_string()
            .map(|s| Some(pele::compat::TrustedSvg::from_trusted_markup(s)))
            .ok_or_else(|| {
                callback_error("Icon resolver must return trusted SVG markup or null".into())
            })
    }
    fn parse_date(&mut self, input: &str) -> Result<Option<f64>, pele::compat::PeleError> {
        let instant = js_sys::Date::parse(input);
        Ok(instant.is_finite().then_some(instant))
    }
    fn calendar_fields(
        &mut self,
        instant_ms: f64,
    ) -> Result<Option<pele::compat::CalendarFields>, pele::compat::PeleError> {
        let date = js_sys::Date::new(&instant_ms.into());
        if !date.get_time().is_finite() {
            return Ok(None);
        }
        Ok(Some(pele::compat::CalendarFields {
            civil: pele::compat::CivilDateTime {
                year: date.get_full_year() as i32,
                month: (date.get_month() + 1) as u8,
                day: date.get_date() as u8,
                hour: date.get_hours() as u8,
                minute: date.get_minutes() as u8,
                second: date.get_seconds() as u8,
                millisecond: date.get_milliseconds() as u16,
            },
            weekday: date.get_day() as u8,
            utc_offset_minutes: -date.get_timezone_offset() as i32,
        }))
    }
    fn calendar_instant(
        &mut self,
        c: pele::compat::CivilDateTime,
    ) -> Result<Option<f64>, pele::compat::PeleError> {
        // Construct with a full 400-year cycle for years 0..99, then restore the
        // exact year; Date's historical constructor adjustment belongs to core.
        let year = if (0..100).contains(&c.year) {
            c.year + 400
        } else {
            c.year
        };
        let args = js_sys::Array::new();
        for value in [
            year,
            i32::from(c.month) - 1,
            i32::from(c.day),
            i32::from(c.hour),
            i32::from(c.minute),
            i32::from(c.second),
            i32::from(c.millisecond),
        ] {
            args.push(&JsValue::from_f64(f64::from(value)));
        }
        let constructor = js_sys::Reflect::get(&js_sys::global(), &"Date".into())
            .map_err(callback_error)?
            .unchecked_into::<js_sys::Function>();
        let date = js_sys::Reflect::construct(&constructor, &args)
            .map_err(callback_error)?
            .unchecked_into::<js_sys::Date>();
        if (0..100).contains(&c.year) {
            date.set_full_year(c.year as u32);
        }
        let instant = date.get_time();
        Ok(instant.is_finite().then_some(instant))
    }
    fn now_millis(&mut self) -> Result<f64, pele::compat::PeleError> {
        Ok(js_sys::Date::now())
    }
}
#[wasm_bindgen]
impl ParsedDocument {
    pub fn render_with_resources(
        &self,
        options_json: &str,
        width: Option<js_sys::Function>,
        icons: Option<js_sys::Function>,
    ) -> String {
        match pele::compat::render_options(options_json).and_then(|options| {
            pele::compat::Engine::empty().render_document(
                &self.inner,
                &options,
                &mut BrowserResources { width, icons },
            )
        }) {
            Ok(result) => pele::compat::render_result_envelope(&result),
            Err(e) => pele::compat::error_envelope(&e),
        }
    }
}
#[wasm_bindgen]
pub fn render_with_resources(
    source: &str,
    options_json: &str,
    width: Option<js_sys::Function>,
    icons: Option<js_sys::Function>,
) -> String {
    match pele::compat::render_options(options_json).and_then(|options| {
        pele::compat::Engine::with_builtins().render_source(
            source,
            &options,
            &mut BrowserResources { width, icons },
        )
    }) {
        Ok(result) => pele::compat::render_result_envelope(&result),
        Err(e) => pele::compat::error_envelope(&e),
    }
}

#[wasm_bindgen]
pub fn measure_text(text: &str, size: f64, style: u8) -> f64 {
    pele::compat::text::metrics_width(text, size, style)
}
