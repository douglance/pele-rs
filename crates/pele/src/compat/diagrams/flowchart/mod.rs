//! Full flowchart port of pinned Pele 458322a5 (MIT, kepano).
//! Source/model layer is public while the full renderer is being translated.
mod lexer;
mod tokens;
mod model;
mod parser;
mod shapes;
pub use model::{FlowchartModel,FlowNode,FlowEdge,FlowSubgraph,FlowClassDef,LabelType};
use crate::compat::{Engine,ParseContext,ParseOptions,PeleError};
/// Parse through the same bounded preparation, directive, encoding and configuration path as Engine.
/// This family parser does not install an incomplete renderer in Engine's registry.
pub fn parse(source:&str,options:&ParseOptions)->Result<FlowchartModel,PeleError>{let inspection=Engine::empty().inspect(source,&options.preparation)?;let mut encoded=inspection.encoded_text;encoded.push('\n');parser::parse(ParseContext{source:&encoded,config:&inspection.prepared.config,title:inspection.prepared.title.as_deref(),line_offset:inspection.prepared.line_offset,max_edges:options.max_edges})}
