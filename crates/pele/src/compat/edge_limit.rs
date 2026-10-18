//! Exact maxEdges semantics from Pele core/FlowDb/AgentDb, 458322a5 (MIT, kepano).
use super::{ErrorCode, PeleError, SourceLimit, Value, DEFAULT_MAX_EDGES};

/// Independent exact-parser edge-generation ceiling, matching public item work.
pub const MAX_PUBLIC_EDGES: usize = 1_000_000;

/// Undefined selects 5,000 at parse time. Null remains null and compares as zero.
#[derive(Clone, Copy, Debug, Default)]
pub enum EdgeLimit {
    #[default]
    Undefined,
    Null,
    Number(f64),
}
impl EdgeLimit {
    pub(crate) fn value(self) -> Value {
        match self {
            Self::Undefined => Value::Number(DEFAULT_MAX_EDGES as f64),
            Self::Null => Value::Null,
            Self::Number(value) => Value::Number(value),
        }
    }
}
/// Independent exact overrides. None retains the corresponding legacy typed
/// option; Some(Undefined) explicitly selects Pele's default for that option.
#[derive(Clone, Copy, Debug, Default)]
pub struct ParseLimits {
    pub source: Option<SourceLimit>,
    pub max_edges: Option<EdgeLimit>,
}

pub(crate) enum EdgePolicy {
    Legacy(usize),
    Exact(EdgeLimit),
}
impl EdgePolicy {
    /// Keep the pre-push check at the builder's original dispatch point.
    /// Semantic rejection precedes the independent native generation ceiling.
    pub(crate) fn check(&self, count: usize, diagram: &str) -> Result<(), PeleError> {
        let exceeded = match self {
            Self::Legacy(limit) => count >= *limit,
            Self::Exact(limit) => {
                let threshold = match limit {
                    EdgeLimit::Undefined => DEFAULT_MAX_EDGES as f64,
                    EdgeLimit::Null => 0.,
                    EdgeLimit::Number(value) => *value,
                };
                count as f64 >= threshold
            }
        };
        if exceeded {
            let threshold = match self {
                Self::Legacy(limit) => limit.to_string(),
                Self::Exact(limit) => limit.value().js_string(),
            };
            let mut error = PeleError::new(
                format!("Edge limit exceeded. {count} edges found, but the limit is {threshold}."),
                ErrorCode::Limit,
            );
            error.diagram_type = Some(diagram.to_owned());
            return Err(error);
        }
        if matches!(self, Self::Exact(_)) && count >= MAX_PUBLIC_EDGES {
            let mut error = PeleError::new(
                "Native edge generation exceeds work budget",
                ErrorCode::Limit,
            );
            error.diagram_type = Some(diagram.to_owned());
            return Err(error);
        }
        Ok(())
    }
}
