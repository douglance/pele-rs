//! Exact number-valued browser parse thresholds, independent of native budgets.
use pele::compat::{EdgeLimit, ErrorCode, ParseLimits, ParseOptions, PeleError, SourceLimit};
use serde::{
    de::{
        self,
        value::{MapAccessDeserializer, StringDeserializer},
        DeserializeSeed, IgnoredAny, MapAccess, Visitor,
    },
    Deserialize,
};

fn invalid(message: impl Into<String>) -> PeleError {
    PeleError::new(message.into(), ErrorCode::Semantic)
}
struct FilteredMap<A> {
    inner: A,
    mask: u8,
    seen: u8,
}
impl<'de, A: MapAccess<'de>> MapAccess<'de> for FilteredMap<A> {
    type Error = A::Error;
    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error> {
        loop {
            let Some(key) = self.inner.next_key::<String>()? else {
                return Ok(None);
            };
            let bit = match key.as_str() {
                "limit" => 1,
                "maxEdges" => 2,
                _ => 0,
            };
            if self.mask & bit != 0 {
                if self.seen & bit != 0 {
                    return Err(de::Error::custom(format!("duplicate field `{key}`")));
                }
                self.seen |= bit;
                self.inner.next_value::<IgnoredAny>()?;
            } else {
                return seed
                    .deserialize(StringDeserializer::<A::Error>::new(key))
                    .map(Some);
            }
        }
    }
    fn next_value_seed<V: DeserializeSeed<'de>>(
        &mut self,
        seed: V,
    ) -> Result<V::Value, Self::Error> {
        self.inner.next_value_seed(seed)
    }
}
struct FilterOptions {
    mask: u8,
}
impl<'de> DeserializeSeed<'de> for FilterOptions {
    type Value = ParseOptions;
    fn deserialize<D: de::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<ParseOptions, D::Error> {
        struct OptionsVisitor {
            mask: u8,
        }
        impl<'de> Visitor<'de> for OptionsVisitor {
            type Value = ParseOptions;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("parse options object")
            }
            fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<ParseOptions, A::Error> {
                ParseOptions::deserialize(MapAccessDeserializer::new(FilteredMap {
                    inner: map,
                    mask: self.mask,
                    seen: 0,
                }))
            }
        }
        deserializer.deserialize_map(OptionsVisitor { mask: self.mask })
    }
}
/// Kinds 0/1/2 mean undefined/null/raw binary64; 3 preserves the legacy JSON
/// option. Only explicitly overridden fields bypass their integer decoder.
pub(super) fn decode_parse_options(
    source: &str,
    json: &str,
    source_kind: u32,
    source_value: f64,
    edge_kind: u32,
    edge_value: f64,
) -> Result<(ParseOptions, ParseLimits), PeleError> {
    if json.len() > 1024 * 1024 || source.len() > pele::compat::MAX_PUBLIC_SOURCE_BYTES {
        return Err(PeleError::new(
            "Browser parse transport exceeds byte budget",
            ErrorCode::Limit,
        ));
    }
    let source_limit = match source_kind {
        0 => Some(SourceLimit::Undefined),
        1 => Some(SourceLimit::Null),
        2 => Some(SourceLimit::Number(source_value)),
        3 => None,
        _ => return Err(invalid("Invalid browser source-limit kind")),
    };
    let edge_limit = match edge_kind {
        0 => Some(EdgeLimit::Undefined),
        1 => Some(EdgeLimit::Null),
        2 => Some(EdgeLimit::Number(edge_value)),
        3 => None,
        _ => return Err(invalid("Invalid browser edge-limit kind")),
    };
    let mask = u8::from(source_limit.is_some()) | (u8::from(edge_limit.is_some()) << 1);
    let options = if mask == 0 {
        pele::compat::parse_options(json)?
    } else if json.trim().is_empty() {
        ParseOptions::default()
    } else {
        let mut decoder = serde_json::Deserializer::from_str(json);
        FilterOptions { mask }
            .deserialize(&mut decoder)
            .and_then(|options| decoder.end().map(|_| options))
            .map_err(|error| invalid(format!("Invalid parse options: {error}")))?
    };
    Ok((
        options,
        ParseLimits {
            source: source_limit,
            max_edges: edge_limit,
        },
    ))
}
