//! Pele usecase/json.ts, 458322a5 (MIT, kepano). Iterative JSON arena retains
//! authored property order and survives nested JSON beyond transport projection depth.
use super::lexer::Source;
use crate::compat::{ErrorCode, PeleError};
use serde::{Serialize, Serializer};
use std::collections::HashMap;
#[derive(Clone, Debug)]
pub enum JsonAtom {
    Object(Vec<(String, usize)>),
    Array(Vec<usize>),
    String(String),
    Number(f64),
    Bool(bool),
    Null,
    /// Native render projection of an undefined property value.
    Undefined,
    /// Native render projection of an absent array index.
    Hole,
}
#[derive(Clone, Debug)]
pub struct JsonValue {
    pub atoms: Vec<JsonAtom>,
    pub root: usize,
    raw: Box<serde_json::value::RawValue>,
    pub depth: usize,
    // Original escaped JSON before the legacy UTF8 atom projection.
    pub(super) authored: Option<std::sync::Arc<str>>,
}
impl Default for JsonValue {
    fn default() -> Self {
        Self {
            atoms: vec![JsonAtom::Object(vec![])],
            root: 0,
            raw: serde_json::value::RawValue::from_string("{}".into()).unwrap(),
            depth: 1,
            authored: None,
        }
    }
}
impl JsonValue {
    // Render-only projection; authoritative public values remain in ModelGraph.
    pub(super) fn from_render_atoms(atoms: Vec<JsonAtom>, root: usize) -> Self {
        Self {
            atoms,
            root,
            raw: serde_json::value::RawValue::from_string("{}".into()).unwrap(),
            depth: 1,
            authored: None,
        }
    }
}
impl Serialize for JsonValue {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.raw.serialize(s)
    }
}
struct Frame {
    order_index: usize,
    node: usize,
    pointer: String,
    key: String,
    index: usize,
    object: bool,
    from: usize,
    ranges: HashMap<String, (usize, usize)>,
    keys: HashMap<String, usize>,
}
struct Reader<'a> {
    s: &'a Source,
    p: usize,
    atoms: Vec<JsonAtom>,
    order: Vec<Option<(String, Vec<String>)>>,
    created: Vec<usize>,
    frames: Vec<Frame>,
    max_depth: usize,
}
#[derive(Debug)]
struct Bad {
    offset: usize,
    kind: &'static str,
}
impl Reader<'_> {
    fn skip(&mut self) {
        while matches!(self.s.c(self.p), 9 | 10 | 13 | 32) {
            self.p += 1
        }
    }
    fn string(&mut self) -> Result<String, Bad> {
        let from = self.p;
        self.p += 1;
        let mut units = vec![];
        while self.p < self.s.len() {
            let c = self.s.c(self.p);
            self.p += 1;
            if c == 34 {
                return Ok(String::from_utf16_lossy(&units));
            }
            if c < 32 {
                return Err(Bad {
                    offset: self.p - 1,
                    kind: "control",
                });
            }
            if c != 92 {
                units.push(c);
                continue;
            }
            let c = self.s.c(self.p);
            self.p += 1;
            match c {
                34 | 92 | 47 => units.push(c),
                98 => units.push(8),
                102 => units.push(12),
                110 => units.push(10),
                114 => units.push(13),
                116 => units.push(9),
                117 => {
                    let mut code = 0;
                    for _ in 0..4 {
                        let c = self.s.c(self.p);
                        let Some(d) = char::from_u32(c as u32).and_then(|c| c.to_digit(16)) else {
                            return Err(Bad {
                                offset: self.p,
                                kind: "unicode",
                            });
                        };
                        code = code * 16 + d as u16;
                        self.p += 1
                    }
                    units.push(code)
                }
                _ => {
                    return Err(Bad {
                        offset: self.p - 1,
                        kind: "escape",
                    })
                }
            }
        }
        Err(Bad {
            offset: self.p.max(from),
            kind: "string-end",
        })
    }
    fn number(&mut self) -> Result<f64, Bad> {
        let from = self.p;
        if self.s.c(self.p) == 45 {
            self.p += 1
        }
        if self.s.c(self.p) == 48 {
            self.p += 1
        } else if (48..=57).contains(&self.s.c(self.p)) {
            while (48..=57).contains(&self.s.c(self.p)) {
                self.p += 1
            }
        } else {
            return Err(Bad {
                offset: self.p,
                kind: "minus",
            });
        }
        if self.s.c(self.p) == 46 {
            self.p += 1;
            if !(48..=57).contains(&self.s.c(self.p)) {
                return Err(Bad {
                    offset: self.p,
                    kind: "fraction",
                });
            }
            while (48..=57).contains(&self.s.c(self.p)) {
                self.p += 1
            }
        }
        if self.s.c(self.p) | 32 == 101 {
            self.p += 1;
            if matches!(self.s.c(self.p), 43 | 45) {
                self.p += 1
            }
            if !(48..=57).contains(&self.s.c(self.p)) {
                return Err(Bad {
                    offset: self.p,
                    kind: "exponent",
                });
            }
            while (48..=57).contains(&self.s.c(self.p)) {
                self.p += 1
            }
        }
        if (48..=57).contains(&self.s.c(self.p)) {
            return Err(Bad {
                offset: self.p,
                kind: "number",
            });
        }
        Ok(self.s.text(from, self.p).parse().unwrap_or(f64::NAN))
    }
    fn literal(&mut self, word: &str) -> Result<(), Bad> {
        for b in word.bytes() {
            if self.s.c(self.p) != b as u16 {
                return Err(Bad {
                    offset: self.p,
                    kind: "value",
                });
            }
            self.p += 1
        }
        Ok(())
    }
    fn parse(&mut self) -> Result<usize, Bad> {
        let mut state = 0;
        let mut pointer = String::new();
        let mut root = 0;
        let mut latest = 0;
        loop {
            if state == 0 {
                self.skip();
                let c = self.s.c(self.p);
                let node = self.atoms.len();
                let atom = match c {
                    123 => {
                        self.p += 1;
                        JsonAtom::Object(vec![])
                    }
                    91 => {
                        self.p += 1;
                        JsonAtom::Array(vec![])
                    }
                    34 => JsonAtom::String(self.string()?),
                    116 => {
                        self.literal("true")?;
                        JsonAtom::Bool(true)
                    }
                    102 => {
                        self.literal("false")?;
                        JsonAtom::Bool(false)
                    }
                    110 => {
                        self.literal("null")?;
                        JsonAtom::Null
                    }
                    45 | 48..=57 => JsonAtom::Number(self.number()?),
                    _ => {
                        return Err(Bad {
                            offset: self.p,
                            kind: "value",
                        })
                    }
                };
                self.atoms.push(atom);
                if self.frames.is_empty() {
                    root = node
                }
                self.max_depth = self.max_depth.max(self.frames.len() + 1);
                if let Some(f) = self.frames.last_mut() {
                    match &mut self.atoms[f.node] {
                        JsonAtom::Object(pairs) => {
                            if let Some(&i) = f.keys.get(&f.key) {
                                pairs[i].1 = node
                            } else {
                                f.keys.insert(f.key.clone(), pairs.len());
                                pairs.push((f.key.clone(), node))
                            }
                        }
                        JsonAtom::Array(a) => a.push(node),
                        _ => unreachable!(),
                    }
                }
                let order_index = self.order.len();
                let object = c == 123;
                let array = c == 91;
                state = 2;
                latest = node;
                if object {
                    let at = self.order.len();
                    self.order.push(Some((pointer.clone(), vec![])));
                    self.created.push(at)
                }
                if object || array {
                    self.skip();
                    if self.s.c(self.p) == if object { 125 } else { 93 } {
                        self.p += 1
                    } else {
                        self.frames.push(Frame {
                            order_index,
                            node,
                            pointer: pointer.clone(),
                            key: String::new(),
                            index: 0,
                            object,
                            from: 0,
                            ranges: HashMap::new(),
                            keys: HashMap::new(),
                        });
                        if object {
                            state = 1
                        } else {
                            pointer.push_str("/0");
                            state = 0
                        }
                    }
                }
            } else if state == 1 {
                if self.s.c(self.p) != 34 {
                    return Err(Bad {
                        offset: self.p,
                        kind: if self.frames.last().unwrap().keys.is_empty() {
                            "key"
                        } else {
                            "key-after-comma"
                        },
                    });
                }
                let key = self.string()?;
                let f = self.frames.last_mut().unwrap();
                if let Some(&(from, to)) = f.ranges.get(&key) {
                    for k in from..to {
                        self.order[self.created[k]] = None
                    }
                } else {
                    let at = f.order_index;
                    self.order[at].as_mut().unwrap().1.push(key.clone())
                }
                f.key = key.clone();
                f.from = self.created.len();
                pointer = format!(
                    "{}/{}",
                    f.pointer,
                    key.replace('~', "~0").replace('/', "~1")
                );
                self.skip();
                if self.s.c(self.p) != 58 {
                    return Err(Bad {
                        offset: self.p,
                        kind: "colon",
                    });
                }
                self.p += 1;
                state = 0
            } else {
                let Some(f) = self.frames.last_mut() else {
                    break;
                };
                if f.object {
                    f.ranges.insert(f.key.clone(), (f.from, self.created.len()));
                } else {
                    f.index += 1
                }
                let close = if f.object { 125 } else { 93 };
                self.skip();
                let c = self.s.c(self.p);
                if c == close {
                    self.p += 1;
                    latest = self.frames.pop().unwrap().node
                } else if c != 44 {
                    return Err(Bad {
                        offset: self.p,
                        kind: if self.frames.last().unwrap().object {
                            "object-close"
                        } else {
                            "array-close"
                        },
                    });
                } else {
                    self.p += 1;
                    self.skip();
                    let f = self.frames.last().unwrap();
                    pointer = format!("{}/{}", f.pointer, f.index);
                    state = if f.object { 1 } else { 0 }
                }
            }
        }
        let _ = latest;
        self.skip();
        if self.p != self.s.len() {
            return Err(Bad {
                offset: self.p,
                kind: "trailing",
            });
        }
        Ok(root)
    }
}
fn raw(atoms: &[JsonAtom], root: usize) -> String {
    enum Item<'a> {
        Node(usize),
        Text(&'a str),
        Key(&'a str),
    }
    let mut out = String::new();
    let mut stack = vec![Item::Node(root)];
    while let Some(item) = stack.pop() {
        match item {
            Item::Text(s) => out.push_str(s),
            Item::Key(s) => out.push_str(&serde_json::to_string(s).unwrap()),
            Item::Node(i) => match &atoms[i] {
                JsonAtom::String(s) => out.push_str(&serde_json::to_string(s).unwrap()),
                JsonAtom::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
                JsonAtom::Null | JsonAtom::Undefined | JsonAtom::Hole => out.push_str("null"),
                JsonAtom::Number(n) => out.push_str(&if n.is_finite() {
                    crate::compat::svg::js_num(*n)
                } else {
                    "null".into()
                }),
                JsonAtom::Object(pairs) => {
                    out.push('{');
                    stack.push(Item::Text("}"));
                    for (k, (key, v)) in pairs.iter().enumerate().rev() {
                        stack.push(Item::Node(*v));
                        stack.push(Item::Text(":"));
                        stack.push(Item::Key(key));
                        if k > 0 {
                            stack.push(Item::Text(","))
                        }
                    }
                }
                JsonAtom::Array(a) => {
                    out.push('[');
                    stack.push(Item::Text("]"));
                    for (k, v) in a.iter().enumerate().rev() {
                        stack.push(Item::Node(*v));
                        if k > 0 {
                            stack.push(Item::Text(","))
                        }
                    }
                }
            },
        }
    }
    out
}
pub(super) fn parse(
    text: &str,
    origin: (usize, usize),
) -> Result<(JsonValue, Vec<(String, Vec<String>)>), PeleError> {
    let source = Source::new(text);
    let mut r = Reader {
        s: &source,
        p: 0,
        atoms: vec![],
        order: vec![],
        created: vec![],
        frames: vec![],
        max_depth: 0,
    };
    let root = match r.parse() {
        Ok(root) => root,
        Err(b) => {
            let (local_line, local_col) = source.loc(b.offset.min(source.len()));
            let line = origin.0 + local_line - 1;
            let column = if local_line == 1 {
                origin.1 + local_col - 1
            } else {
                local_col
            };
            let message = error_message(&source, &b);
            let mut e = PeleError::new(
                format!("Invalid JSON: {message} (line {line}, column {column})"),
                ErrorCode::Syntax,
            );
            e.diagram_type = Some("usecase".into());
            e.line = line;
            e.column = column;
            return Err(e);
        }
    };
    if !matches!(r.atoms[root], JsonAtom::Object(_)) {
        let mut e = PeleError::new(
            format!(
                "JSON value must have an object root (line {}, column {})",
                origin.0, origin.1
            ),
            ErrorCode::Syntax,
        );
        e.diagram_type = Some("usecase".into());
        e.line = origin.0;
        e.column = origin.1;
        return Err(e);
    }
    let json = raw(&r.atoms, root);
    let value = JsonValue {
        atoms: r.atoms,
        root,
        raw: serde_json::value::RawValue::from_string(json).unwrap(),
        depth: r.max_depth,
        authored: Some(text.into()),
    };
    Ok((value, r.order.into_iter().flatten().collect()))
}
fn error_message(s: &Source, b: &Bad) -> String {
    let p = b.offset;
    let (line, column) = s.loc(p);
    if p >= s.len() {
        return match b.kind{"object-close"=>format!("Expected ',' or '}}' after property value in JSON at position {p} (line {line} column {column})"),"array-close"=>format!("Expected ',' or ']' after array element in JSON at position {p} (line {line} column {column})"),"string-end"=>format!("Unterminated string in JSON at position {p} (line {line} column {column})"),_=>"Unexpected end of JSON input".into()};
    }
    let reason = match b.kind {
        "value" if (48..=57).contains(&s.c(p)) || s.c(p) == 45 => "Unexpected number in JSON",
        "value" if s.c(p) == 34 => "Unexpected string in JSON",
        "key" => "Expected property name or '}' in JSON",
        "key-after-comma" => "Expected double-quoted property name in JSON",
        "number" => "Unexpected number in JSON",
        "colon" => "Expected ':' after property name in JSON",
        "object-close" => "Expected ',' or '}' after property value in JSON",
        "array-close" => "Expected ',' or ']' after array element in JSON",
        "trailing" => "Unexpected non-whitespace character after JSON",
        "control" => "Bad control character in string literal in JSON",
        "escape" => "Bad escaped character in JSON",
        "unicode" => "Bad Unicode escape in JSON",
        "minus" => "No number after minus sign in JSON",
        "fraction" => "Unterminated fractional number in JSON",
        "exponent" => "Exponent part is missing a number in JSON",
        _ => {
            let (from, to) = if s.len() <= 20 {
                (0, s.len())
            } else {
                (if p >= 10 { p - 10 } else { 0 }, (p + 10).min(s.len()))
            };
            return format!(
                "Unexpected token '{}', {}\"{}\"{} is not valid JSON",
                s.text(p, p + 1),
                if from > 0 || p >= 10 && s.len() > 20 {
                    "..."
                } else {
                    ""
                },
                s.text(from, to),
                if to < s.len() { "..." } else { "" }
            );
        }
    };
    format!("{reason} at position {p} (line {line} column {column})")
}
