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
