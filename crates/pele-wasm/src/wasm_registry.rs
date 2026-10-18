//! Owned web registry. Native registrations and retained documents stay in Rust.
use pele::compat::{public::PublicEngine, ErrorCode, PeleError, MAX_PUBLIC_SOURCE_BYTES};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = NativeRegistry)]
pub struct NativeRegistry {
    inner: Option<PublicEngine>,
}
fn closed() -> PeleError {
    PeleError::new("Registry is closed", ErrorCode::Semantic)
}
fn bounded(input: &str) -> Result<(), PeleError> {
    if input.len() > MAX_PUBLIC_SOURCE_BYTES {
        Err(PeleError::new(
            "Native transport: declared input exceeds budget",
            ErrorCode::Limit,
        ))
    } else {
        Ok(())
    }
}
fn failure(error: PeleError) -> JsValue {
    JsValue::from_str(&super::render_numbers::failure(&error))
}
#[wasm_bindgen]
impl NativeRegistry {
    #[wasm_bindgen(constructor)]
    pub fn new(all_builtins: bool) -> Self {
        Self {
            inner: Some(if all_builtins {
                PublicEngine::with_builtins()
            } else {
                PublicEngine::empty()
            }),
        }
    }
    pub fn registered(&self, id: &str) -> Result<String, JsValue> {
        let engine = self.inner.as_ref().ok_or_else(|| failure(closed()))?;
        bounded(id).map_err(failure)?;
        let id = pele::compat::DIAGRAM_TYPES
            .into_iter()
            .find(|kind| kind.as_str() == id)
            .map(pele::compat::DiagramId::Builtin)
            .unwrap_or_else(|| pele::compat::DiagramId::extension(id));
        Ok(super::public_result(&engine.registered(id)))
    }
    pub fn capabilities(&self) -> Result<String, JsValue> {
        let engine = self.inner.as_ref().ok_or_else(|| failure(closed()))?;
        Ok(super::public_result(&engine.capabilities()))
    }
    pub fn register_builtin(&mut self, id: &str) -> Result<(), JsValue> {
        let engine = self.inner.as_mut().ok_or_else(|| failure(closed()))?;
        bounded(id).map_err(failure)?;
        let kind = pele::compat::DIAGRAM_TYPES
            .into_iter()
            .find(|kind| kind.as_str() == id)
            .ok_or_else(|| {
                failure(PeleError::new(
                    format!("Unknown built-in diagram type \"{id}\""),
                    ErrorCode::UnsupportedDiagram,
                ))
            })?;
        engine.register_builtin(kind);
        Ok(())
    }
    pub fn detect_type(&self, source: &str) -> Result<String, JsValue> {
        let engine = self.inner.as_ref().ok_or_else(|| failure(closed()))?;
        bounded(source).map_err(failure)?;
        let kind = engine.detect_type(source).map_err(failure)?;
        Ok(super::public_result(&kind))
    }
    pub fn supports(&self, source: &str) -> Result<String, JsValue> {
        let engine = self.inner.as_ref().ok_or_else(|| failure(closed()))?;
        bounded(source).map_err(failure)?;
        Ok(super::public_result(&engine.supports(source)))
    }
    #[allow(clippy::too_many_arguments)]
    pub fn parse(
        &self,
        source: &str,
        json: &str,
        source_kind: u32,
        source_value: f64,
        edge_kind: u32,
        edge_value: f64,
    ) -> Result<super::MutableDocument, JsValue> {
        let engine = self.inner.as_ref().ok_or_else(|| failure(closed()))?;
        let (options, limits) = super::parse_limits::decode_parse_options(
            source,
            json,
            source_kind,
            source_value,
            edge_kind,
            edge_value,
        )
        .map_err(failure)?;
        Ok(super::MutableDocument {
            inner: engine
                .parse_with_limits(source, &options, limits)
                .map_err(failure)?,
        })
    }
    #[allow(clippy::too_many_arguments)]
    pub fn render_numbers_with_resources(
        &self,
        source: &str,
        json: &str,
        mask: u32,
        numbers: &[f64],
        output_kind: u32,
        output: f64,
        source_kind: u32,
        source_value: f64,
        edge_kind: u32,
        edge_value: f64,
        width: Option<js_sys::Function>,
        icons: Option<js_sys::Function>,
        clock: Option<js_sys::Function>,
    ) -> String {
        super::render_numbers::rendered((|| {
            let engine = self.inner.as_ref().ok_or_else(closed)?;
            let (options, limits, output) = super::render_numbers::options_with_parse_limits(
                source,
                json,
                mask,
                numbers,
                output_kind,
                output,
                source_kind,
                source_value,
                edge_kind,
                edge_value,
            )?;
            engine.render_source_bounded_with_limits(
                source,
                &options,
                limits,
                output,
                &mut super::render_numbers::NumericResources {
                    browser: super::BrowserResources { width, icons },
                    clock,
                },
            )
        })())
    }
    /// Retained PublicDocuments own their registration and survive registry close.
    pub fn close(&mut self) {
        self.inner.take();
    }
}
