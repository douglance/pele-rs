mod decode;
mod detect;
mod entities;
mod preprocess;
mod yaml;
pub(crate) use detect::detect;
pub use detect::{DiagramType, DIAGRAM_TYPES};
pub(crate) use preprocess::preprocess;
pub use preprocess::PreparedSource;
pub fn encode_entities(text: &str) -> String {
    entities::encode(text)
}

pub fn decode_entities(text: &str) -> Result<String, crate::compat::PeleError> {
    decode::decode(text)
}
