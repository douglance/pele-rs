//! Open module identity; the pinned built-in detector remains a separate closed catalogue.
use super::DiagramType;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum DiagramId {
    Builtin(DiagramType),
    Extension(String),
}
impl From<DiagramType> for DiagramId {
    fn from(value: DiagramType) -> Self {
        Self::Builtin(value)
    }
}
impl DiagramId {
    pub fn extension(name: impl Into<String>) -> Self {
        Self::Extension(name.into())
    }
    pub fn as_str(&self) -> &str {
        match self {
            Self::Builtin(kind) => kind.as_str(),
            Self::Extension(name) => name,
        }
    }
}
impl std::fmt::Display for DiagramId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
impl Serialize for DiagramId {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}
impl<'de> Deserialize<'de> for DiagramId {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let name = String::deserialize(d)?;
        Ok(super::DIAGRAM_TYPES
            .iter()
            .find(|kind| kind.as_str() == name)
            .map_or_else(
                || Self::Extension(name.clone()),
                |kind| Self::Builtin(*kind),
            ))
    }
}
