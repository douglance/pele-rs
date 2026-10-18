use crate::{render, Error, RenderOptions};
use serde_json::json;

pub const MAX_OPTIONS: usize = 16384;
pub const MAX_JSON: usize = 16 * 1024 * 1024;

/// Same versioned UTF-8 envelope on native and WASM. Never embeds source in errors.
pub fn render_json(source: &str, options_json: &str) -> String {
    render_envelope(source, options_json).1
}

pub fn error_envelope(error: &Error) -> String {
    serde_json::to_string(&json!({"version":1,"ok":false,"error":error}))
        .expect("finite error envelope")
}

pub fn render_envelope(source: &str, options_json: &str) -> (u32, String) {
    let result = (|| {
        if options_json.len() > MAX_OPTIONS {
            return Err(Error::new("input_limit", "Options exceed 16384 bytes"));
        }
        let options = if options_json.is_empty() {
            RenderOptions::default()
        } else {
            serde_json::from_str(options_json)
                .map_err(|_| Error::new("invalid_options", "Invalid or unknown render option"))?
        };
        render(source, &options)
    })();
    match result {
        Ok(rendered) => {
            let out = serde_json::to_string(&json!({"version":1,"ok":true,"result":rendered}))
                .expect("validated finite geometry");
            if out.len() > MAX_JSON {
                let e = Error::new("output_limit", "JSON result exceeds 16 MiB");
                (e.status(), error_envelope(&e))
            } else {
                (0, out)
            }
        }
        Err(e) => (e.status(), error_envelope(&e)),
    }
}
