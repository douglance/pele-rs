//! Complete opaque JSON values in sequence participant menus. Pinned JS menus accept
//! arbitrary nested JSON; a raw validated value preserves it without a recursive Rust
//! ownership/drop tree. The engine's model projection retains its independent budget.
use crate::compat::{value::parse_model_json, Value};
use serde::de::{MapAccess, Visitor};
use serde::{Serialize, Serializer};
use serde_json::value::RawValue;
use std::{fmt, sync::Arc};
#[derive(Clone, Debug)]
pub struct SeqValue {
    raw: Arc<RawValue>,
    value: Option<Value>,
}
impl SeqValue {
    fn raw(raw: Box<RawValue>) -> Self {
        let value = parse_model_json(raw.get()).ok();
        Self {
            raw: Arc::from(raw),
            value,
        }
    }
    pub(super) fn value(value: Value) -> Self {
        let raw = RawValue::from_string(serde_json::to_string(&value).expect("Value serializes"))
            .expect("Value JSON validates");
        Self {
            raw: Arc::from(raw),
            value: Some(value),
        }
    }
    /// The complete validated JSON, including data too deep for model transport.
    pub fn json(&self) -> &str {
        self.raw.get()
    }
    pub fn as_str(&self) -> Option<&str> {
        match self.value.as_ref() {
            Some(Value::String(s)) => Some(s),
            _ => None,
        }
    }
}
impl Serialize for SeqValue {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.raw.serialize(s)
    }
}
struct Fields;
impl<'de> Visitor<'de> for Fields {
    type Value = Vec<(String, SeqValue)>;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("JSON menu object")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut out: Vec<(String, SeqValue)> = vec![];
        while let Some((key, raw)) = map.next_entry::<String, Box<RawValue>>()? {
            let value = SeqValue::raw(raw);
            if let Some((_, old)) = out.iter_mut().find(|(k, _)| *k == key) {
                *old = value
            } else {
                out.push((key, value));
            }
        }
        let index = |s: &str| {
            s.parse::<u32>()
                .ok()
                .filter(|n| *n != u32::MAX && n.to_string() == s)
        };
        out.sort_by(|a, b| match (index(&a.0), index(&b.0)) {
            (Some(a), Some(b)) => a.cmp(&b),
            (Some(_), _) => std::cmp::Ordering::Less,
            (_, Some(_)) => std::cmp::Ordering::Greater,
            _ => std::cmp::Ordering::Equal,
        });
        Ok(out)
    }
}
pub(super) fn entries(text: &str) -> Result<Vec<(String, SeqValue)>, serde_json::Error> {
    let raw: Box<RawValue> = serde_json::from_str(text)?;
    let s = raw.get();
    match s.as_bytes()[0] {
        b'{' => serde::de::Deserializer::deserialize_map(
            &mut serde_json::Deserializer::from_str(s),
            Fields,
        ),
        b'[' => serde_json::from_str::<Vec<Box<RawValue>>>(s).map(|values| {
            values
                .into_iter()
                .enumerate()
                .map(|(i, v)| (i.to_string(), SeqValue::raw(v)))
                .collect()
        }),
        b'"' => serde_json::from_str::<String>(s).map(|text| {
            text.chars()
                .enumerate()
                .map(|(i, c)| (i.to_string(), SeqValue::value(Value::String(c.to_string()))))
                .collect()
        }),
        _ => Ok(vec![]),
    }
}
