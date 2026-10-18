// JSON.parse's value domain includes raw lone UTF16 units. Build bottom-up into
// the identity writer, so nesting never consumes the Rust stack. Kept private
// until its general public contract is reviewed.
#[derive(Debug)]
pub(crate) enum GraphJsonError {
    Syntax,
    Limit,
}
pub(crate) fn parse_graph(
    input: &crate::compat::public::JsString,
) -> Result<crate::compat::public::ModelGraph, GraphJsonError> {
    parse_graph_inner(input, false).map(|(graph, _)| graph)
}
/// Raw authored object keys and JSON Pointers, before numeric enumeration sorting.
/// Only reachable final values contribute; duplicate keys replace descendant order.
pub(crate) fn parse_graph_with_order(
    input: &crate::compat::public::JsString,
) -> Result<
    (
        crate::compat::public::ModelGraph,
        Vec<(
            crate::compat::public::JsString,
            Vec<crate::compat::public::JsString>,
        )>,
    ),
    GraphJsonError,
> {
    parse_graph_inner(input, true)
}
fn parse_graph_inner(
    input: &crate::compat::public::JsString,
    ordered: bool,
) -> Result<
    (
        crate::compat::public::ModelGraph,
        Vec<(
            crate::compat::public::JsString,
            Vec<crate::compat::public::JsString>,
        )>,
    ),
    GraphJsonError,
> {
    use crate::compat::public::{Atom, GraphWriter, JsString, Node, Property};
    const ITEMS: usize = 1_000_000;
    const WORK: usize = 8_000_000;
    const TEXT: usize = 16_000_000;
    struct Reader<'a> {
        s: &'a [u16],
        at: usize,
        writer: GraphWriter,
        work: usize,
        units: usize,
        nodes: usize,
    }
    impl Reader<'_> {
        fn blank(&mut self) {
            while self
                .s
                .get(self.at)
                .is_some_and(|u| matches!(u, 9 | 10 | 13 | 32))
            {
                self.at += 1
            }
        }
        fn string(&mut self) -> Result<JsString, GraphJsonError> {
            if self.s.get(self.at) != Some(&34) {
                return Err(GraphJsonError::Syntax);
            }
            self.at += 1;
            let mut out = Vec::new();
            loop {
                let c = *self.s.get(self.at).ok_or(GraphJsonError::Syntax)?;
                self.at += 1;
                match c {
                    34 => break,
                    0..=31 => return Err(GraphJsonError::Syntax),
                    92 => {
                        let e = *self.s.get(self.at).ok_or(GraphJsonError::Syntax)?;
                        self.at += 1;
                        out.push(match e {
                            34 | 47 | 92 => e,
                            98 => 8,
                            102 => 12,
                            110 => 10,
                            114 => 13,
                            116 => 9,
                            117 => {
                                let mut value = 0;
                                for _ in 0..4 {
                                    let x = *self.s.get(self.at).ok_or(GraphJsonError::Syntax)?;
                                    self.at += 1;
                                    let n = match x {
                                        48..=57 => x - 48,
                                        65..=70 => x - 55,
                                        97..=102 => x - 87,
                                        _ => return Err(GraphJsonError::Syntax),
                                    };
                                    value = value * 16 + n
                                }
                                value
                            }
                            _ => return Err(GraphJsonError::Syntax),
                        })
                    }
                    _ => out.push(c),
                }
                if out.len() > TEXT {
                    return Err(GraphJsonError::Limit);
                }
            }
            self.units = self
                .units
                .checked_add(out.len())
                .ok_or(GraphJsonError::Limit)?;
            if self.units > TEXT {
                return Err(GraphJsonError::Limit);
            }
            Ok(JsString(out))
        }
        fn number(&mut self) -> Result<Atom, GraphJsonError> {
            let start = self.at;
            if self.s.get(self.at) == Some(&45) {
                self.at += 1
            }
            match self.s.get(self.at) {
                Some(48) => self.at += 1,
                Some(49..=57) => {
                    self.at += 1;
                    while self.s.get(self.at).is_some_and(|u| matches!(u, 48..=57)) {
                        self.at += 1
                    }
                }
                _ => return Err(GraphJsonError::Syntax),
            }
            if self.s.get(self.at) == Some(&46) {
                self.at += 1;
                let at = self.at;
                while self.s.get(self.at).is_some_and(|u| matches!(u, 48..=57)) {
                    self.at += 1
                }
                if at == self.at {
                    return Err(GraphJsonError::Syntax);
                }
            }
            if self.s.get(self.at).is_some_and(|u| matches!(u, 69 | 101)) {
                self.at += 1;
                if self.s.get(self.at).is_some_and(|u| matches!(u, 43 | 45)) {
                    self.at += 1
                }
                let at = self.at;
                while self.s.get(self.at).is_some_and(|u| matches!(u, 48..=57)) {
                    self.at += 1
                }
                if at == self.at {
                    return Err(GraphJsonError::Syntax);
                }
            }
            let s: String = self.s[start..self.at]
                .iter()
                .map(|c| *c as u8 as char)
                .collect();
            Ok(Atom::Number(s.parse().map_err(|_| GraphJsonError::Syntax)?))
        }
        fn primitive(&mut self) -> Result<Atom, GraphJsonError> {
            self.blank();
            match self.s.get(self.at) {
                Some(34) => {
                    let s = self.string()?;
                    Ok(Atom::String(self.writer.string(s)))
                }
                Some(45 | 48..=57) => self.number(),
                _ => {
                    for (word, v) in [
                        ("true", Atom::Bool(true)),
                        ("false", Atom::Bool(false)),
                        ("null", Atom::Null),
                    ] {
                        let w: Vec<_> = word.encode_utf16().collect();
                        if self.s[self.at..].starts_with(&w) {
                            self.at += w.len();
                            return Ok(v);
                        }
                    }
                    Err(GraphJsonError::Syntax)
                }
            }
        }
        fn node(&mut self, n: Node) -> Result<Atom, GraphJsonError> {
            self.nodes += 1;
            if self.nodes > ITEMS {
                return Err(GraphJsonError::Limit);
            }
            Ok(Atom::Ref(self.writer.node(n)))
        }
    }
    enum Frame {
        Array {
            values: Vec<Atom>,
            need: bool,
        },
        Object {
            ordinal: usize,
            properties: Vec<Property>,
            keys: std::collections::HashMap<JsString, usize>,
            key: Option<JsString>,
            need: bool,
        },
    }
    fn index(key: &JsString) -> Option<u32> {
        if key.0.is_empty() || (key.0.len() > 1 && key.0[0] == 48) {
            return None;
        }
        let mut value = 0u64;
        for &c in &key.0 {
            if !(48..=57).contains(&c) {
                return None;
            }
            value = value.checked_mul(10)?.checked_add((c - 48) as u64)?;
            if value >= u32::MAX as u64 {
                return None;
            }
        }
        Some(value as u32)
    }
    if input.0.len() > TEXT {
        return Err(GraphJsonError::Limit);
    }
    let mut r = Reader {
        s: &input.0,
        at: 0,
        writer: GraphWriter::default(),
        work: 0,
        units: 0,
        nodes: 0,
    };
    let mut frames: Vec<Frame> = Vec::new();
    let mut next_object = 0usize;
    let mut authored = std::collections::HashMap::new();
    let mut value = None;
    let root;
    loop {
        r.work += 1;
        if r.work > WORK {
            return Err(GraphJsonError::Limit);
        }
        if let Some(v) = value.take() {
            match frames.last_mut() {
                None => {
                    root = v;
                    break;
                }
                Some(Frame::Array { values, need }) => {
                    values.push(v);
                    if values.len() > ITEMS {
                        return Err(GraphJsonError::Limit);
                    }
                    *need = false
                }
                Some(Frame::Object {
                    properties,
                    keys,
                    key,
                    need,
                    ..
                }) => {
                    let key = key.take().ok_or(GraphJsonError::Syntax)?;
                    if let Some(&i) = keys.get(&key) {
                        properties[i].value = v
                    } else {
                        if properties.len() >= ITEMS {
                            return Err(GraphJsonError::Limit);
                        }
                        keys.insert(key.clone(), properties.len());
                        let k = r.writer.string(key);
                        properties.push(Property {
                            key: k,
                            value: v,
                            flags: 7,
                        })
                    }
                    *need = false
                }
            }
        }
        r.blank();
        let mut close = false;
        if let Some(frame) = frames.last_mut() {
            match frame {
                Frame::Array { values, need } => {
                    if !*need {
                        match r.s.get(r.at) {
                            Some(44) => {
                                r.at += 1;
                                *need = true
                            }
                            Some(93) => close = true,
                            _ => return Err(GraphJsonError::Syntax),
                        }
                    } else if values.is_empty() && r.s.get(r.at) == Some(&93) {
                        close = true
                    }
                }
                Frame::Object {
                    properties,
                    key,
                    need,
                    ..
                } => {
                    if !*need {
                        match r.s.get(r.at) {
                            Some(44) => {
                                r.at += 1;
                                *need = true
                            }
                            Some(125) => close = true,
                            _ => return Err(GraphJsonError::Syntax),
                        }
                    } else if properties.is_empty() && r.s.get(r.at) == Some(&125) {
                        close = true
                    }
                    if !close {
                        r.blank();
                        *key = Some(r.string()?);
                        r.blank();
                        if r.s.get(r.at) != Some(&58) {
                            return Err(GraphJsonError::Syntax);
                        }
                        r.at += 1
                    }
                }
            }
        }
        if close {
            r.at += 1;
            let mut order = None;
            let node = match frames.pop().unwrap() {
                Frame::Array { values, .. } => {
                    let length = values.len() as u32;
                    let properties = values
                        .into_iter()
                        .enumerate()
                        .map(|(i, v)| r.writer.property(&i.to_string(), v))
                        .collect();
                    Node::Array { length, properties }
                }
                Frame::Object {
                    mut properties,
                    keys,
                    ordinal,
                    ..
                } => {
                    if ordered {
                        let mut items = keys.iter().collect::<Vec<_>>();
                        items.sort_by_key(|(_, index)| **index);
                        order = Some((
                            ordinal,
                            items
                                .into_iter()
                                .map(|(key, _)| key.clone())
                                .collect::<Vec<_>>(),
                        ));
                    }
                    let ids: std::collections::HashMap<_, _> = keys
                        .into_iter()
                        .map(|(k, i)| (properties[i].key, index(&k)))
                        .collect();
                    properties.sort_by(|a, b| match (ids[&a.key], ids[&b.key]) {
                        (Some(a), Some(b)) => a.cmp(&b),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        _ => std::cmp::Ordering::Equal,
                    });
                    Node::Object { properties }
                }
            };
            let node = r.node(node)?;
            if let (Some(order), Atom::Ref(id)) = (order, &node) {
                authored.insert(*id, order);
            }
            value = Some(node);
            continue;
        }
        r.blank();
        match r.s.get(r.at) {
            Some(91) => {
                r.at += 1;
                frames.push(Frame::Array {
                    values: vec![],
                    need: true,
                })
            }
            Some(123) => {
                r.at += 1;
                let ordinal = next_object;
                next_object += 1;
                frames.push(Frame::Object {
                    ordinal,
                    properties: vec![],
                    keys: std::collections::HashMap::new(),
                    key: None,
                    need: true,
                })
            }
            _ => value = Some(r.primitive()?),
        }
        if frames.len() > ITEMS {
            return Err(GraphJsonError::Limit);
        }
    }
    r.blank();
    if r.at != r.s.len() {
        return Err(GraphJsonError::Syntax);
    }
    let graph = r.writer.finish(root).map_err(|_| GraphJsonError::Limit)?;
    if !ordered {
        return Ok((graph, vec![]));
    }
    enum Visit {
        Value(Atom, Option<JsString>),
        Leave(bool),
    }
    let mut path = Vec::<JsString>::new();
    let mut pending = vec![Visit::Value(graph.root.clone(), None)];
    let mut order = Vec::new();
    let mut units = 0usize;
    while let Some(visit) = pending.pop() {
        r.work += 1;
        if r.work > WORK {
            return Err(GraphJsonError::Limit);
        }
        let (value, segment) = match visit {
            Visit::Leave(added) => {
                if added {
                    path.pop();
                }
                continue;
            }
            Visit::Value(value, segment) => (value, segment),
        };
        let added = segment.is_some();
        if let Some(segment) = segment {
            path.push(segment)
        }
        pending.push(Visit::Leave(added));
        let Atom::Ref(id) = value else { continue };
        match &graph.nodes[id as usize] {
            Node::Object { properties } => {
                let (ordinal, keys) = authored.remove(&id).ok_or(GraphJsonError::Syntax)?;
                let mut size = path.len();
                for segment in &path {
                    for unit in &segment.0 {
                        size = size.saturating_add(if matches!(unit, 47 | 126) { 2 } else { 1 });
                    }
                }
                units = units
                    .saturating_add(size)
                    .saturating_add(keys.iter().map(|key| key.0.len()).sum::<usize>());
                if units > TEXT {
                    return Err(GraphJsonError::Limit);
                }
                let mut pointer = Vec::with_capacity(size);
                for segment in &path {
                    pointer.push(47);
                    for unit in &segment.0 {
                        match unit {
                            47 => pointer.extend([126, 49]),
                            126 => pointer.extend([126, 48]),
                            unit => pointer.push(*unit),
                        }
                    }
                }
                order.push((ordinal, JsString(pointer), keys));
                for p in properties.iter().rev() {
                    pending.push(Visit::Value(
                        p.value.clone(),
                        Some(graph.strings[p.key as usize].clone()),
                    ))
                }
            }
            Node::Array { properties, .. } => {
                for p in properties.iter().rev() {
                    pending.push(Visit::Value(
                        p.value.clone(),
                        Some(graph.strings[p.key as usize].clone()),
                    ))
                }
            }
            _ => return Err(GraphJsonError::Syntax),
        }
    }
    order.sort_by_key(|(ordinal, _, _)| *ordinal);
    Ok((
        graph,
        order
            .into_iter()
            .map(|(_, pointer, keys)| (pointer, keys))
            .collect(),
    ))
}
