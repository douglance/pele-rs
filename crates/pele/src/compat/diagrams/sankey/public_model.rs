//! Opt-in mutable Sankey data codec for Pele 458322a5 (MIT, kepano).
//!
//! The arena owns identity: node array entries, link endpoints and the observable
//! `lookup` Map share object IDs. Direct edits do not refresh that Map or
//! renumber nodes. The existing Sankey renderer consumes a concrete typed
//! projection imported at the edit boundary; rendering never reparses source.
use super::{Sankey, SankeyLink, SankeyModel, SankeyNode};
use crate::compat::{
    public::{Atom, GraphWriter, ModelGraph, Node, PublicDiagramModule},
    DiagramModule, DiagramType, ErrorCode, Object, PeleError, RenderResources, RenderSettings,
    Rendered,
};
use std::collections::HashMap;

pub struct SankeyPublicState {
    graph: ModelGraph,
    model: Result<SankeyModel, PeleError>,
}
fn invalid(message: &str) -> PeleError {
    let mut error = PeleError::new(message, ErrorCode::Semantic);
    error.diagram_type = Some("sankey".into());
    error
}
fn object_id(atom: &Atom) -> Result<u32, PeleError> {
    match atom {
        Atom::Ref(id) => Ok(*id),
        _ => Err(invalid("Sankey node and link records must be objects")),
    }
}
fn field<'a>(graph: &'a ModelGraph, id: u32, name: &str) -> Option<&'a Atom> {
    graph
        .nodes
        .get(id as usize)?
        .properties()
        .iter()
        .find_map(|p| graph.strings[p.key as usize].is(name).then_some(&p.value))
}
fn string(graph: &ModelGraph, atom: &Atom) -> Result<String, PeleError> {
    graph
        .text(atom)?
        .well_formed()
        .ok_or_else(|| invalid("Sankey rendering of lone UTF-16 surrogates is not implemented"))
}
fn optional_text(graph: &ModelGraph, key: &str) -> Result<Option<String>, PeleError> {
    match graph.field(key)? {
        None | Some(Atom::Undefined | Atom::Null) => Ok(None),
        Some(value) => string(graph, value).map(Some),
    }
}
// Canonical own array indexes only. Holes are retained in the arena, but a
// renderer read of a missing node/link record fails instead of allocating from
// an untrusted sparse length or silently shortening the array.
fn array<'a>(graph: &'a ModelGraph, name: &str) -> Result<Vec<&'a Atom>, PeleError> {
    let Some(Atom::Ref(id)) = graph.field(name)? else {
        return Err(invalid(&format!("Sankey {name} must be an Array")));
    };
    let Some(Node::Array { length, properties }) = graph.nodes.get(*id as usize) else {
        return Err(invalid(&format!("Sankey {name} must be an Array")));
    };
    let mut indexed = HashMap::new();
    for p in properties {
        let key = &graph.strings[p.key as usize].0;
        if key.is_empty() || key.len() > 10 || (key.len() > 1 && key[0] == 48) {
            continue;
        }
        let mut index = 0u64;
        let mut canonical = true;
        for &unit in key {
            if !(48..=57).contains(&unit) {
                canonical = false;
                break;
            }
            index = index * 10 + u64::from(unit - 48);
        }
        if canonical && index < u64::from(*length) {
            indexed.entry(index as u32).or_insert(&p.value);
        }
    }
    if indexed.len() != *length as usize {
        return Err(invalid(&format!(
            "Sankey {name} contains a missing array record"
        )));
    }
    Ok((0..*length).map(|i| indexed[&i]).collect())
}
fn index(graph: &ModelGraph, id: u32, length: usize) -> Result<usize, PeleError> {
    match field(graph, id, "index") {
        Some(Atom::Number(n)) if n.is_finite() && *n >= 0. && n.fract() == 0. => {
            if *n < length as f64 {
                Ok(*n as usize)
            } else {
                Err(invalid(
                    "Sankey link endpoint index is outside the node array",
                ))
            }
        }
        _ => Err(invalid(
            "Sankey link endpoint index must be a finite array index",
        )),
    }
}
fn imported(graph: &ModelGraph) -> Result<SankeyModel, PeleError> {
    let array_nodes = array(graph, "nodes")?;
    let array_links = array(graph, "links")?;
    let mut nodes = Vec::with_capacity(array_nodes.len());
    let mut labels: HashMap<u32, String> = HashMap::new();
    let mut indexes: HashMap<u32, usize> = HashMap::new();
    for value in array_nodes {
        let id = object_id(value)?;
        let label = if let Some(label) = labels.get(&id) {
            label.clone()
        } else {
            let value =
                field(graph, id, "id").ok_or_else(|| invalid("Sankey node ID must be a string"))?;
            let label = string(graph, value)?;
            labels.insert(id, label.clone());
            label
        };
        // Array node.index is not read by Pele's layout or labels. It may be
        // stale, absent or nonfinite until that record is used as an endpoint.
        // Keep the original value in the arena; this unused typed slot is inert.
        let unused_index = match field(graph, id, "index") {
            Some(Atom::Number(n))
                if n.is_finite() && *n >= 0. && n.fract() == 0. && *n <= usize::MAX as f64 =>
            {
                *n as usize
            }
            _ => 0,
        };
        nodes.push(SankeyNode {
            id: label,
            index: unused_index,
        });
    }
    let mut links = Vec::with_capacity(array_links.len());
    for value in array_links {
        let id = object_id(value)?;
        let mut endpoint = |name| -> Result<SankeyNode, PeleError> {
            let value = field(graph, id, name)
                .ok_or_else(|| invalid("Sankey links require source and target objects"))?;
            let object = object_id(value)?;
            let index = if let Some(index) = indexes.get(&object) {
                *index
            } else {
                let index = index(graph, object, nodes.len())?;
                indexes.insert(object, index);
                index
            };
            // The link's endpoint.id is not read by the renderer. An endpoint
            // may remain detached after nodes[i] is replaced; its own index
            // still selects the current array slot, exactly as in Pele.
            Ok(SankeyNode {
                id: labels.get(&object).cloned().unwrap_or_default(),
                index,
            })
        };
        let source = endpoint("source")?;
        let target = endpoint("target")?;
        let value = match field(graph, id, "value") {
            Some(Atom::Number(n)) => *n,
            // Number.isFinite rejects these before layout ever adds them.
            _ => f64::NAN,
        };
        links.push(SankeyLink {
            source,
            target,
            value,
        });
    }
    Ok(SankeyModel {
        diagram_type: DiagramType::Sankey,
        nodes,
        links,
        title: optional_text(graph, "title")?,
        acc_title: optional_text(graph, "accTitle")?,
        acc_descr: optional_text(graph, "accDescr")?,
    })
}
fn node(out: &mut GraphWriter, node: &SankeyNode) -> u32 {
    let label = out.string(node.id.as_str());
    let properties = vec![
        out.property("id", Atom::String(label)),
        out.property("index", Atom::Number(node.index as f64)),
    ];
    out.node(Node::Object { properties })
}
fn array_node(out: &mut GraphWriter, entries: impl Iterator<Item = Atom>) -> u32 {
    let properties: Vec<_> = entries
        .enumerate()
        .map(|(i, value)| out.property(&i.to_string(), value))
        .collect();
    out.node(Node::Array {
        length: properties.len() as u32,
        properties,
    })
}
impl PublicDiagramModule for Sankey {
    type PublicState = SankeyPublicState;
    fn create_public(&self, model: &SankeyModel) -> Result<Self::PublicState, PeleError> {
        let mut out = GraphWriter::default();
        let node_objects: Vec<_> = model.nodes.iter().map(|n| node(&mut out, n)).collect();
        let mut original_nodes = HashMap::new();
        for (record, object) in model.nodes.iter().zip(&node_objects) {
            original_nodes
                .entry((record.index, record.id.as_str()))
                .or_insert(*object);
        }
        let nodes = array_node(&mut out, node_objects.iter().copied().map(Atom::Ref));
        let mut link_objects = Vec::with_capacity(model.links.len());
        for link in &model.links {
            let mut endpoint = |endpoint: &SankeyNode| {
                // The source parser's records are owned value copies in the
                // immutable API. Recover its guaranteed index/record alias
                // once here; all subsequent edits use arena IDs exclusively.
                original_nodes
                    .get(&(endpoint.index, endpoint.id.as_str()))
                    .copied()
                    .unwrap_or_else(|| node(&mut out, endpoint))
            };
            let source = endpoint(&link.source);
            let target = endpoint(&link.target);
            let properties = vec![
                out.property("source", Atom::Ref(source)),
                out.property("target", Atom::Ref(target)),
                out.property("value", Atom::Number(link.value)),
            ];
            link_objects.push(out.node(Node::Object { properties }));
        }
        let links = array_node(&mut out, link_objects.into_iter().map(Atom::Ref));
        let entries = model
            .nodes
            .iter()
            .zip(&node_objects)
            .map(|(n, id)| (Atom::String(out.string(n.id.as_str())), Atom::Ref(*id)))
            .collect();
        let lookup = out.node(Node::Map {
            entries,
            properties: vec![],
        });
        let kind = out.string("sankey");
        let mut properties = vec![
            out.property("type", Atom::String(kind)),
            out.property("nodes", Atom::Ref(nodes)),
            out.property("links", Atom::Ref(links)),
        ];
        for (key, text) in [
            ("title", &model.title),
            ("accTitle", &model.acc_title),
            ("accDescr", &model.acc_descr),
        ] {
            let value = text.as_ref().map_or(Atom::Undefined, |text| {
                Atom::String(out.string(text.as_str()))
            });
            properties.push(out.property(key, value));
        }
        properties.push(out.property("lookup", Atom::Ref(lookup)));
        let root = out.node(Node::Object { properties });
        Ok(SankeyPublicState {
            graph: out.finish(Atom::Ref(root))?,
            model: Ok(model.clone()),
        })
    }
    fn export_public(&self, state: &Self::PublicState) -> Result<ModelGraph, PeleError> {
        Ok(state.graph.clone())
    }
    fn apply_public(
        &self,
        state: &mut Self::PublicState,
        graph: ModelGraph,
    ) -> Result<(), PeleError> {
        graph.validate()?;
        let model = imported(&graph);
        state.graph = graph;
        state.model = model;
        Ok(())
    }
    fn render_public(
        &self,
        state: &mut Self::PublicState,
        config: &Object,
        options: &RenderSettings,
        host: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError> {
        self.render(
            state.model.as_ref().map_err(Clone::clone)?,
            config,
            options,
            host,
        )
    }
}
