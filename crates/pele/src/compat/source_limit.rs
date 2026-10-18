//! Exact source-limit comparison from Pele core.ts at 458322a5 (MIT, kepano).
//! This well-formed-string lane keeps native byte budgets separate from the
//! number-valued Pele threshold. It does not change edge or output limits.
use super::{ErrorCode, PeleError, Value, DEFAULT_LIMIT};

/// Independent byte budget for the additive public source-limit APIs.
/// NaN and infinity never disable this native transport/preparation boundary.
pub const MAX_PUBLIC_SOURCE_BYTES: usize = 16 * 1024 * 1024;

/// JavaScript source-limit argument. Omission and explicit undefined use the
/// default; null compares as zero while remaining `null` in the error message.
/// Numbers retain their IEEE representation, including NaN, infinities and -0.
#[derive(Clone, Copy, Debug, Default)]
pub enum SourceLimit {
    #[default]
    Undefined,
    Null,
    Number(f64),
}
impl SourceLimit {
    pub(crate) fn check_bytes(source: &str) -> Result<(), PeleError> {
        if source.len() > MAX_PUBLIC_SOURCE_BYTES {
            return Err(PeleError::new(
                "Native transport: declared input exceeds budget",
                ErrorCode::Limit,
            ));
        }
        Ok(())
    }
    pub(crate) fn check(self, source: &str) -> Result<(), PeleError> {
        Self::check_bytes(source)?;
        let (threshold, display) = match self {
            Self::Undefined => (DEFAULT_LIMIT as f64, Value::Number(DEFAULT_LIMIT as f64)),
            Self::Null => (0., Value::Null),
            Self::Number(value) => (value, Value::Number(value)),
        };
        // The independent byte budget bounds this integer below 2^53, so its
        // conversion is exact. Compare directly: no rounding or usize sentinel.
        if source.encode_utf16().count() as f64 > threshold {
            return Err(PeleError::new(
                format!(
                    "Diagram source is longer than the limit of {} characters.",
                    display.js_string()
                ),
                ErrorCode::Limit,
            ));
        }
        Ok(())
    }
}
