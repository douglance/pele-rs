//! Versioned host extension; the existing v2 callback struct retains its ABI.
use super::{
    compatibility_call, compatibility_input, NativeResources, PeleBorrowedBytes, PeleHostResources,
    PeleResult,
};
use pele::compat::{
    CalendarFields, CivilDateTime, ErrorCode, PeleError, RenderResources, TextRequest, TrustedSvg,
};

type CalendarCallback = unsafe extern "C" fn(
    *mut std::ffi::c_void,
    u32,
    *const u8,
    usize,
    *mut PeleBorrowedBytes,
) -> u32;
#[repr(C)]
pub struct PeleHostResourcesV3 {
    pub base: PeleHostResources,
    pub calendar: Option<CalendarCallback>,
    pub calendar_instant: Option<CalendarCallback>,
    pub date_parser: Option<CalendarCallback>,
    pub utc_offset: Option<unsafe extern "C" fn(*mut std::ffi::c_void, f64, *mut i32) -> u32>,
}
struct Resources<'a> {
    inner: NativeResources<'a>,
    host: Option<&'a PeleHostResourcesV3>,
}
impl Resources<'_> {
    fn offset(&self, instant: f64) -> Result<Option<i32>, PeleError> {
        let host = self.host.unwrap();
        let mut output = 0;
        if unsafe { host.utc_offset.unwrap()(host.base.context, instant, &mut output) } != 0 {
            return Err(PeleError::new("Host timezone failed", ErrorCode::Semantic));
        }
        if output.unsigned_abs() > 86_400 {
            return Err(PeleError::new(
                "Host timezone returned invalid offset",
                ErrorCode::Semantic,
            ));
        }
        Ok(Some(output))
    }

    fn calendar<T: serde::de::DeserializeOwned>(
        &self,
        operation: u32,
        request: &impl serde::Serialize,
    ) -> Result<Option<T>, PeleError> {
        let host = self.host.unwrap();
        let callback = match operation {
            0 => host.calendar,
            1 => host.calendar_instant,
            2 => host.date_parser,
            _ => None,
        }
        .unwrap();
        let request = serde_json::to_vec(request)
            .map_err(|_| PeleError::new("Invalid host calendar request", ErrorCode::Semantic))?;
        let mut output = PeleBorrowedBytes {
            data: std::ptr::null(),
            len: 0,
        };
        if unsafe {
            callback(
                host.base.context,
                operation,
                request.as_ptr(),
                request.len(),
                &mut output,
            )
        } != 0
        {
            return Err(PeleError::new("Host calendar failed", ErrorCode::Semantic));
        }
        if output.data.is_null() && output.len == 0 {
            return Ok(None);
        }
        let text = unsafe { compatibility_input(output.data, output.len, 4096) }?;
        serde_json::from_str(text)
            .map_err(|_| PeleError::new("Host calendar returned invalid JSON", ErrorCode::Semantic))
    }
}
impl RenderResources for Resources<'_> {
    fn text_width(&mut self, request: TextRequest<'_>) -> Result<f64, PeleError> {
        self.inner.text_width(request)
    }
    fn icon(&mut self, name: &str) -> Result<Option<TrustedSvg>, PeleError> {
        self.inner.icon(name)
    }
    fn now_millis(&mut self) -> Result<f64, PeleError> {
        self.inner.now_millis()
    }
    fn parse_date(&mut self, input: &str) -> Result<Option<f64>, PeleError> {
        if self.host.is_some_and(|h| h.date_parser.is_some()) {
            let result: Option<f64> = self.calendar(2, &input)?;
            Ok(result
                .filter(|v| v.is_finite() && v.abs() <= 8.64e15)
                .map(f64::trunc))
        } else {
            DefaultDateParser(self).parse_date(input)
        }
    }
    fn calendar_fields(&mut self, instant_ms: f64) -> Result<Option<CalendarFields>, PeleError> {
        if self.host.is_some_and(|h| h.calendar.is_some()) {
            if !instant_ms.is_finite() || instant_ms.abs() > 8.64e15 {
                return Ok(None);
            }
            self.calendar(0, &instant_ms.trunc())
        } else if self.host.is_some_and(|h| h.utc_offset.is_some()) {
            let value = self.offset(instant_ms)?;
            Ok(value.and_then(|offset| pele::compat::fields_at_offset(instant_ms, offset)))
        } else {
            self.inner.calendar_fields(instant_ms)
        }
    }
    fn calendar_instant(&mut self, civil: CivilDateTime) -> Result<Option<f64>, PeleError> {
        if self.host.is_some_and(|h| h.calendar_instant.is_some()) {
            let anchor = pele::compat::text::MetricsResources.calendar_instant(civil)?;
            let result: Option<f64> = self.calendar(
                1,
                &serde_json::json!({"civil":civil,"utcAnchorMillis":anchor}),
            )?;
            Ok(result
                .filter(|v| v.is_finite() && v.abs() <= 8.64e15)
                .map(f64::trunc))
        } else if self.host.is_some_and(|h| h.utc_offset.is_some()) {
            pele::compat::instant_in_zone(civil, |instant| self.offset(instant))
        } else {
            self.inner.calendar_instant(civil)
        }
    }
}
// Use the core's V8 date parser with this provider's calendar, preserving local
// DST conversion without substituting Foundation's different legacy grammar.
struct DefaultDateParser<'a, 'b>(&'a mut Resources<'b>);
impl RenderResources for DefaultDateParser<'_, '_> {
    fn text_width(&mut self, request: TextRequest<'_>) -> Result<f64, PeleError> {
        self.0.text_width(request)
    }
    fn calendar_fields(&mut self, instant: f64) -> Result<Option<CalendarFields>, PeleError> {
        self.0.calendar_fields(instant)
    }
    fn calendar_instant(&mut self, civil: CivilDateTime) -> Result<Option<f64>, PeleError> {
        self.0.calendar_instant(civil)
    }
}
unsafe fn render(
    document: Option<*const PeleResult>,
    source: *const u8,
    source_len: usize,
    options: *const u8,
    options_len: usize,
    host: *const PeleHostResourcesV3,
) -> *mut PeleResult {
    compatibility_call(|| {
        let options = unsafe { compatibility_input(options, options_len, 1024 * 1024) }?;
        let options = pele::compat::render_options(options)?;
        let host = unsafe { host.as_ref() };
        let mut resources = Resources {
            inner: NativeResources {
                host: host.map(|h| &h.base),
            },
            host,
        };
        let rendered = if let Some(document) = document {
            let document = unsafe { document.as_ref() }
                .and_then(|r| r.document.as_ref())
                .ok_or_else(|| {
                    PeleError::new(
                        "render() takes diagram text or a model that parse() returned.",
                        ErrorCode::Semantic,
                    )
                })?;
            pele::compat::Engine::empty().render_document(document, &options, &mut resources)?
        } else {
            let source = unsafe { compatibility_input(source, source_len, 16 * 1024 * 1024) }?;
            pele::compat::Engine::with_builtins().render_source(source, &options, &mut resources)?
        };
        Ok(PeleResult {
            status: 0,
            data: pele::compat::render_result_envelope(&rendered).into_bytes(),
            document: None,
        })
    })
}
/// # Safety
/// Inputs, host and callbacks are synchronously borrowed. Calendar output is
/// borrowed until its next callback; Rust decodes it before returning control.
#[no_mangle]
pub unsafe extern "C" fn pele_render_with_resources_v3(
    source: *const u8,
    source_len: usize,
    options: *const u8,
    options_len: usize,
    host: *const PeleHostResourcesV3,
) -> *mut PeleResult {
    unsafe { render(None, source, source_len, options, options_len, host) }
}
/// # Safety
/// Document must be live. Other borrows follow pele_render_with_resources_v3.
#[no_mangle]
pub unsafe extern "C" fn pele_render_document_with_resources_v3(
    document: *const PeleResult,
    options: *const u8,
    options_len: usize,
    host: *const PeleHostResourcesV3,
) -> *mut PeleResult {
    unsafe {
        render(
            Some(document),
            std::ptr::null(),
            0,
            options,
            options_len,
            host,
        )
    }
}
