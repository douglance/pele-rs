//! Standalone deterministic flowcharts. See docs/LLD.md and README for supported syntax.
#![forbid(unsafe_code)]
pub mod compat;
mod contract;
mod error;
mod layout;
mod model;
mod parser;
mod svg;
mod text;

pub use contract::{error_envelope, render_envelope, render_json, MAX_OPTIONS};
pub use error::{Error, Span};
pub use layout::{layout, Layout};
pub use model::*;
pub use parser::{parse, MAX_SOURCE};
use serde::Serialize;
pub use svg::render_svg;

#[derive(Clone, Debug, Serialize)]
pub struct Rendered {
    pub svg: String,
    pub width: f64,
    pub height: f64,
    pub geometry: Geometry,
}

pub fn render(source: &str, options: &RenderOptions) -> Result<Rendered, Error> {
    options.validate()?;
    let graph = parse(source)?;
    let scene = layout(&graph, options)?;
    Ok(Rendered {
        svg: render_svg(&scene)?,
        width: scene.width(),
        height: scene.height(),
        geometry: scene.geometry().clone(),
    })
}
