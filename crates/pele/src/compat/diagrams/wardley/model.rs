//! Wardley builder result and ordered model population, Pele 458322a5 (MIT).
use super::parser::Ast;
use crate::compat::{source::space, svg::js_num, ErrorCode, PeleError, Value};
use serde::{Serialize, Serializer};
use std::collections::HashMap;
fn number<S: Serializer>(n: &f64, s: S) -> Result<S::Ok, S::Error> {
    Value::Number(*n).serialize(s)
}
fn optional_number<S: Serializer>(n: &Option<f64>, s: S) -> Result<S::Ok, S::Error> {
    n.map(Value::Number).serialize(s)
}
fn boundaries<S: Serializer>(v: &Option<Vec<f64>>, s: S) -> Result<S::Ok, S::Error> {
    v.as_ref()
        .map(|a| a.iter().copied().map(Value::Number).collect::<Vec<_>>())
        .serialize(s)
}
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WardleyNode {
    /// Real builder insertion order, including own undefined properties.
    #[serde(skip)]
    pub public_keys: Vec<&'static str>,
    pub id: String,
    pub label: String,
    #[serde(serialize_with = "number")]
    pub x: f64,
    #[serde(serialize_with = "number")]
    pub y: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(serialize_with = "optional_number")]
    pub label_offset_x: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(serialize_with = "optional_number")]
    pub label_offset_y: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_pipeline: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_pipeline_parent: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inertia: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_strategy: Option<String>,
}
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WardleyLink {
    pub source: String,
    pub target: String,
    pub dashed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flow: Option<String>,
}
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WardleyTrend {
    pub node_id: String,
    #[serde(serialize_with = "number")]
    pub target_x: f64,
    #[serde(serialize_with = "number")]
    pub target_y: f64,
}
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WardleyPipeline {
    pub node_id: String,
    pub component_ids: Vec<String>,
}
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WardleyPoint {
    #[serde(serialize_with = "number")]
    pub x: f64,
    #[serde(serialize_with = "number")]
    pub y: f64,
}
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WardleyAnnotation {
    #[serde(serialize_with = "number")]
    pub number: f64,
    pub coordinates: Vec<WardleyPoint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WardleyNote {
    pub text: String,
    #[serde(serialize_with = "number")]
    pub x: f64,
    #[serde(serialize_with = "number")]
    pub y: f64,
}
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WardleyForce {
    pub name: String,
    #[serde(serialize_with = "number")]
    pub x: f64,
    #[serde(serialize_with = "number")]
    pub y: f64,
}
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WardleyAxes {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stages: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(serialize_with = "boundaries")]
    pub stage_boundaries: Option<Vec<f64>>,
}
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WardleySize {
    #[serde(serialize_with = "number")]
    pub width: f64,
    #[serde(serialize_with = "number")]
    pub height: f64,
}
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WardleyModel {
    #[serde(rename = "type")]
    pub diagram_type: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acc_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acc_descr: Option<String>,
    pub nodes: Vec<WardleyNode>,
    pub links: Vec<WardleyLink>,
    pub trends: Vec<WardleyTrend>,
    pub pipelines: Vec<WardleyPipeline>,
    pub annotations: Vec<WardleyAnnotation>,
    pub notes: Vec<WardleyNote>,
    pub accelerators: Vec<WardleyForce>,
    pub deaccelerators: Vec<WardleyForce>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations_box: Option<WardleyPoint>,
    pub axes: WardleyAxes,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<WardleySize>,
}

fn semantic(message: String) -> PeleError {
    let mut e = PeleError::new(message, ErrorCode::Semantic);
    e.diagram_type = Some("wardley".into());
    e
}
fn percent(value: f64, context: &str) -> Result<f64, PeleError> {
    let n = if value <= 1. { value * 100. } else { value };
    if n < 0. || n > 100. {
        return Err(semantic(format!(
            "{context} must be between 0-1 (decimal) or 0-100 (percentage). Received: {}",
            js_num(value)
        )));
    }
    Ok(n)
}
fn coordinates(visibility: f64, evolution: f64, context: &str) -> Result<WardleyPoint, PeleError> {
    Ok(WardleyPoint {
        x: percent(evolution, &format!("{context} evolution"))?,
        y: percent(visibility, &format!("{context} visibility"))?,
    })
}
fn flow_port(s: Option<&str>) -> Option<String> {
    match s {
        Some("+<>") => Some("bidirectional".into()),
        Some("+<") => Some("backward".into()),
        Some("+>") => Some("forward".into()),
        _ => None,
    }
}
fn flow_arrow(s: Option<&str>) -> (Option<String>, Option<String>) {
    let Some(s) = s.filter(|s| s.starts_with('+')) else {
        return (None, None);
    };
    let label = if s.starts_with("+'") {
        s[2..].find('\'').map(|e| s[2..e + 2].to_owned())
    } else {
        None
    };
    let flow = if s.contains("<>") {
        Some("bidirectional")
    } else if s.contains('<') {
        Some("backward")
    } else if s.contains('>') {
        Some("forward")
    } else {
        None
    };
    (flow.map(str::to_owned), label)
}
struct Builder {
    model: WardleyModel,
    indices: HashMap<String, usize>,
    labels: HashMap<String, String>,
    trends: HashMap<String, usize>,
    pipelines: HashMap<String, usize>,
}
impl Builder {
    fn new() -> Self {
        Self {
            model: WardleyModel {
                diagram_type: "wardley",
                ..WardleyModel::default()
            },
            indices: HashMap::new(),
            labels: HashMap::new(),
            trends: HashMap::new(),
            pipelines: HashMap::new(),
        }
    }
    fn node(&mut self, mut n: WardleyNode, erase_flags: bool) {
        n.public_keys = self
            .indices
            .get(&n.id)
            .map_or_else(Vec::new, |&i| self.model.nodes[i].public_keys.clone());
        for key in [
            "id",
            "label",
            "x",
            "y",
            "className",
            "labelOffsetX",
            "labelOffsetY",
        ]
        .into_iter()
        .chain(
            erase_flags
                .then_some(["inertia", "sourceStrategy"])
                .into_iter()
                .flatten(),
        ) {
            if !n.public_keys.contains(&key) {
                n.public_keys.push(key);
            }
        }
        if let Some(&i) = self.indices.get(&n.id) {
            let prev = &self.model.nodes[i];
            if n.class_name.is_none() {
                n.class_name = prev.class_name.clone()
            }
            if n.label_offset_x.is_none() {
                n.label_offset_x = prev.label_offset_x
            }
            if n.label_offset_y.is_none() {
                n.label_offset_y = prev.label_offset_y
            }
            n.in_pipeline = n.in_pipeline.or(prev.in_pipeline);
            n.is_pipeline_parent = n.is_pipeline_parent.or(prev.is_pipeline_parent);
            if !erase_flags {
                n.inertia = n.inertia.or(prev.inertia);
                n.source_strategy = n.source_strategy.or_else(|| prev.source_strategy.clone())
            }
            self.model.nodes[i] = n;
        } else {
            let i = self.model.nodes.len();
            self.indices.insert(n.id.clone(), i);
            self.model.nodes.push(n)
        }
    }
    fn get(&self, name: &str) -> Option<&WardleyNode> {
        self.indices.get(name).map(|i| &self.model.nodes[*i])
    }
    fn resolve(&self, name: &str) -> String {
        if self.indices.contains_key(name) {
            name.into()
        } else {
            self.labels
                .get(name)
                .map_or_else(|| name.into(), Clone::clone)
        }
    }
}
pub(super) fn build(ast: Ast, title: Option<&str>) -> Result<WardleyModel, PeleError> {
    let mut b = Builder::new();
    let m = &mut b.model;
    m.title = ast
        .title
        .filter(|s| !s.is_empty())
        .or_else(|| title.map(str::to_owned));
    m.acc_title = ast
        .acc_title
        .filter(|s| !s.is_empty())
        .map(|s| s.trim_start_matches(space).into());
    m.acc_descr = ast.acc_descr.filter(|s| !s.is_empty()).map(|s| {
        let mut out = String::new();
        let mut after = false;
        for c in s.chars() {
            if after && space(c) {
                continue;
            }
            after = c == '\n';
            out.push(c)
        }
        out
    });
    m.size = ast
        .size
        .map(|(width, height)| WardleySize { width, height });
    if let Some(stages) = ast.evolution {
        m.axes.stages = Some(
            stages
                .iter()
                .map(|s| {
                    if let Some(second) = s.second_name.as_deref().filter(|s| !s.is_empty()) {
                        format!(
                            "{} / {}",
                            s.name.trim_matches(space),
                            second.trim_matches(space)
                        )
                    } else {
                        s.name.trim_matches(space).into()
                    }
                })
                .collect(),
        );
        m.axes.stage_boundaries = Some(stages.iter().filter_map(|s| s.boundary).collect());
    }
    for a in ast.anchors {
        let p = coordinates(a.visibility, a.evolution, &format!("Anchor \"{}\"", a.name))?;
        b.node(
            WardleyNode {
                id: a.name.clone(),
                label: a.name,
                x: p.x,
                y: p.y,
                class_name: Some("anchor".into()),
                ..Default::default()
            },
            false,
        )
    }
    for c in ast.components {
        let p = coordinates(
            c.at.visibility,
            c.at.evolution,
            &format!("Component \"{}\"", c.at.name),
        )?;
        b.node(
            WardleyNode {
                id: c.at.name.clone(),
                label: c.at.name,
                x: p.x,
                y: p.y,
                class_name: Some("component".into()),
                label_offset_x: c.label.map(|p| p.0),
                label_offset_y: c.label.map(|p| p.1),
                inertia: Some(c.inertia),
                source_strategy: c.strategy,
                ..Default::default()
            },
            true,
        )
    }
    for n in ast.notes {
        let p = coordinates(n.visibility, n.evolution, &format!("Note \"{}\"", n.name))?;
        b.model.notes.push(WardleyNote {
            text: n.name,
            x: p.x,
            y: p.y,
        })
    }
    for (parent, components) in ast.pipelines {
        let Some(p) = b.get(&parent) else {
            return Err(semantic(format!(
                "Pipeline \"{parent}\" must reference an existing component with coordinates."
            )));
        };
        let y = p.y;
        let pipeline = if let Some(&i) = b.pipelines.get(&parent) {
            b.model.pipelines[i].component_ids.clear();
            i
        } else {
            let i = b.model.pipelines.len();
            b.pipelines.insert(parent.clone(), i);
            b.model.pipelines.push(WardleyPipeline {
                node_id: parent.clone(),
                component_ids: vec![],
            });
            i
        };
        let node = &mut b.model.nodes[b.indices[&parent]];
        node.is_pipeline_parent = Some(true);
        if !node.public_keys.contains(&"isPipelineParent") {
            node.public_keys.push("isPipelineParent");
        }
        for c in components {
            let id = format!("{parent}_{}", c.name);
            let x = percent(
                c.evolution,
                &format!("Pipeline component \"{}\" evolution", c.name),
            )?;
            b.node(
                WardleyNode {
                    id: id.clone(),
                    label: c.name,
                    x,
                    y,
                    class_name: Some("pipeline-component".into()),
                    label_offset_x: c.label.map(|p| p.0),
                    label_offset_y: c.label.map(|p| p.1),
                    ..Default::default()
                },
                true,
            );
            b.model.pipelines[pipeline].component_ids.push(id.clone());
            let node = &mut b.model.nodes[b.indices[&id]];
            node.in_pipeline = Some(true);
            if !node.public_keys.contains(&"inPipeline") {
                node.public_keys.push("inPipeline");
            }
        }
    }
    for node in &b.model.nodes {
        b.labels
            .entry(node.label.clone())
            .or_insert_with(|| node.id.clone());
    }
    for l in ast.links {
        let (arrow_flow, arrow_label) = flow_arrow(l.arrow.as_deref());
        let flow = flow_port(l.from_port.as_deref())
            .or_else(|| flow_port(l.to_port.as_deref()))
            .or(arrow_flow);
        b.model.links.push(WardleyLink {
            source: b.resolve(&l.from),
            target: b.resolve(&l.to),
            dashed: l
                .arrow
                .as_ref()
                .is_some_and(|s| s.contains("-.->") || s.contains(".-.")),
            label: arrow_label.or(l.label),
            flow,
        });
    }
    for (name, target) in ast.evolves {
        if let Some(node) = b.get(&name) {
            let y = node.y;
            let trend = WardleyTrend {
                node_id: name.clone(),
                target_x: percent(target, &format!("Evolve target for \"{name}\""))?,
                target_y: y,
            };
            if let Some(&i) = b.trends.get(&name) {
                b.model.trends[i] = trend
            } else {
                b.trends.insert(name, b.model.trends.len());
                b.model.trends.push(trend);
            }
        }
    }
    if let Some(&(x, y)) = ast.annotations.first() {
        b.model.annotations_box = Some(coordinates(x, y, "Annotations box")?)
    }
    for (n, x, y, text) in ast.annotation {
        let p = coordinates(x, y, &format!("Annotation {}", js_num(n)))?;
        b.model.annotations.push(WardleyAnnotation {
            number: n,
            coordinates: vec![p],
            text: Some(text),
        });
    }
    for a in ast.accelerators {
        let p = coordinates(
            a.visibility,
            a.evolution,
            &format!("Accelerator \"{}\"", a.name),
        )?;
        b.model.accelerators.push(WardleyForce {
            name: a.name,
            x: p.x,
            y: p.y,
        });
    }
    for a in ast.deaccelerators {
        let p = coordinates(
            a.visibility,
            a.evolution,
            &format!("Deaccelerator \"{}\"", a.name),
        )?;
        b.model.deaccelerators.push(WardleyForce {
            name: a.name,
            x: p.x,
            y: p.y,
        });
    }
    Ok(b.model)
}
