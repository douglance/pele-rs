//! Ordered JavaScript/YAML values for the pinned Pele compatibility interface.
use serde::{
    de::{MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Value>),
    Object(Object),
}

/// Insertion order is retained. Enumeration follows JS Object.keys (integer keys first).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Object(Vec<(String, Value)>);
impl Object {
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.0.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }
    pub fn get_mut(&mut self, key: &str) -> Option<&mut Value> {
        self.0.iter_mut().find(|(k, _)| k == key).map(|(_, v)| v)
    }
    pub fn insert(&mut self, key: String, value: Value) {
        if let Some(v) = self.get_mut(&key) {
            *v = value;
        } else {
            self.0.push((key, value));
        }
    }
    pub fn entries(&self) -> Vec<(&str, &Value)> {
        let index = |s: &str| {
            s.parse::<u32>()
                .ok()
                .filter(|n| *n != u32::MAX && n.to_string() == s)
        };
        let mut keys: Vec<_> = self.0.iter().collect();
        keys.sort_by(|a, b| match (index(&a.0), index(&b.0)) {
            (Some(x), Some(y)) => x.cmp(&y),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            _ => std::cmp::Ordering::Equal,
        });
        keys.into_iter().map(|(k, v)| (k.as_str(), v)).collect()
    }
    pub(crate) fn merge(&mut self, source: &Object) {
        for (key, value) in source.entries() {
            if matches!(key, "__proto__" | "constructor" | "prototype") {
                continue;
            }
            if let Value::Object(incoming) = value {
                if !matches!(self.get(key), Some(Value::Object(_))) {
                    self.insert(key.into(), Value::Object(Object::default()));
                }
                if let Some(Value::Object(current)) = self.get_mut(key) {
                    current.merge(incoming);
                }
            } else {
                self.insert(key.into(), value.clone());
            }
        }
    }
}
impl Value {
    pub fn as_object(&self) -> Option<&Object> {
        if let Self::Object(o) = self {
            Some(o)
        } else {
            None
        }
    }
    pub(crate) fn truthy(&self) -> bool {
        match self {
            Self::Null => false,
            Self::Bool(b) => *b,
            Self::Number(n) => *n != 0.0 && !n.is_nan(),
            Self::String(s) => !s.is_empty(),
            _ => true,
        }
    }
    pub(crate) fn js_string(&self) -> String {
        match self {
            Self::Null => "null".into(),
            Self::Bool(b) => b.to_string(),
            Self::Number(n) if n.is_nan() => "NaN".into(),
            Self::Number(n) if n.is_infinite() => if n.is_sign_negative() {
                "-Infinity"
            } else {
                "Infinity"
            }
            .into(),
            Self::Number(n) if *n == 0.0 => "0".into(),
            Self::Number(n) => {
                let mut b = ryu_js::Buffer::new();
                b.format(*n).to_string()
            }
            Self::String(s) => s.clone(),
            Self::Object(_) => "[object Object]".into(),
            Self::Array(items) => items
                .iter()
                .map(|v| {
                    if matches!(v, Self::Null) {
                        String::new()
                    } else {
                        v.js_string()
                    }
                })
                .collect::<Vec<_>>()
                .join(","),
        }
    }
}
impl Serialize for Object {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = s.serialize_map(Some(self.0.len()))?;
        for (k, v) in self.entries() {
            map.serialize_entry(k, v)?;
        }
        map.end()
    }
}
impl Serialize for Value {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Null => s.serialize_none(),
            Self::Bool(b) => s.serialize_bool(*b),
            Self::Number(n) => {
                if n.is_finite() {
                    serde_json::from_str::<serde_json::Number>(&{
                        let mut b = ryu_js::Buffer::new();
                        b.format(*n).to_string()
                    })
                    .map_err(serde::ser::Error::custom)?
                    .serialize(s)
                } else {
                    s.serialize_none()
                }
            }
            Self::String(v) => s.serialize_str(v),
            Self::Array(a) => a.serialize(s),
            Self::Object(o) => o.serialize(s),
        }
    }
}
struct ValueVisitor;
impl<'de> Visitor<'de> for ValueVisitor {
    type Value = Value;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a JSON value")
    }
    fn visit_unit<E: serde::de::Error>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_none<E: serde::de::Error>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Value, E> {
        Ok(Value::Bool(v))
    }
    fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Value, E> {
        Ok(Value::Number(v as f64))
    }
    fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Value, E> {
        Ok(Value::Number(v as f64))
    }
    fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Value, E> {
        Ok(Value::Number(v))
    }
    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Value, E> {
        Ok(Value::String(v.into()))
    }
    fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Value, E> {
        Ok(Value::String(v))
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> Result<Value, A::Error> {
        let mut out = Vec::new();
        while let Some(v) = a.next_element()? {
            out.push(v)
        }
        Ok(Value::Array(out))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<Value, A::Error> {
        let mut out = Object::default();
        while let Some((k, v)) = a.next_entry()? {
            out.insert(k, v)
        }
        Ok(Value::Object(out))
    }
}
impl<'de> Deserialize<'de> for Value {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_any(ValueVisitor)
    }
}
impl<'de> Deserialize<'de> for Object {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        match Value::deserialize(d)? {
            Value::Object(o) => Ok(o),
            _ => Err(serde::de::Error::custom("expected a configuration object")),
        }
    }
}

// RawValue validates JSON grammar without converting overflowing JS numbers to errors.
// Recursion/depth policy is owned by source orchestration rather than YAML scalar parsing.
pub(crate) fn parse_json(text: &str) -> Result<Value, serde_json::Error> {
    parse_json_depth(text, 64, "configuration nesting is too deep")
}
pub(crate) fn parse_model_json(text: &str) -> Result<Value, serde_json::Error> {
    parse_json_depth(text, 128, "model nesting is too deep")
}
fn parse_json_depth(text: &str, limit: usize, message: &str) -> Result<Value, serde_json::Error> {
    if json_depth_exceeds(text, limit) {
        return Err(serde::de::Error::custom(message));
    }
    fn raw(input: &serde_json::value::RawValue) -> Result<Value, serde_json::Error> {
        let s = input.get();
        match s.as_bytes()[0] {
            b'{' => {
                struct Fields;
                impl<'de> Visitor<'de> for Fields {
                    type Value = Object;
                    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                        f.write_str("JSON object")
                    }
                    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Object, A::Error> {
                        let mut out = Object::default();
                        while let Some((key, value)) =
                            map.next_entry::<String, &serde_json::value::RawValue>()?
                        {
                            out.insert(key, raw(value).map_err(serde::de::Error::custom)?)
                        }
                        Ok(out)
                    }
                }
                let mut d = serde_json::Deserializer::from_str(s);
                serde::de::Deserializer::deserialize_map(&mut d, Fields).map(Value::Object)
            }
            b'[' => serde_json::from_str::<Vec<&serde_json::value::RawValue>>(s)?
                .into_iter()
                .map(raw)
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Array),
            b'"' => serde_json::from_str(s).map(Value::String),
            b'n' => Ok(Value::Null),
            b't' => Ok(Value::Bool(true)),
            b'f' => Ok(Value::Bool(false)),
            _ => Ok(Value::Number(
                s.parse::<f64>().map_err(serde::de::Error::custom)?,
            )),
        }
    }
    raw(serde_json::from_str(text)?)
}

pub(crate) fn json_depth_exceeds(text: &str, limit: usize) -> bool {
    let mut quoted = false;
    let mut escaped = false;
    let mut depth = 0usize;
    for c in text.chars() {
        if quoted {
            if escaped {
                escaped = false
            } else if c == '\\' {
                escaped = true
            } else if c == '"' {
                quoted = false
            }
        } else {
            match c {
                '"' => quoted = true,
                '{' | '[' => {
                    depth += 1;
                    if depth > limit {
                        return true;
                    }
                }
                '}' | ']' => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
    }
    false
}
