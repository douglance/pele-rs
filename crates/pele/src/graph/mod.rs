//! Native, identity-preserving graph scenes for AST hosts.
use crate::compat::PeleError;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Graph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub label: String,
    pub detail: String,
    pub external: bool,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub label: String,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlacedNode {
    #[serde(flatten)]
    pub node: Node,
    pub rect: Rect,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Route {
    #[serde(flatten)]
    pub edge: Edge,
    pub points: Vec<Point>,
    pub label_box: Rect,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Scene {
    pub width: f64,
    pub height: f64,
    pub nodes: Vec<PlacedNode>,
    pub edges: Vec<Route>,
}
#[derive(Clone, Debug)]
pub struct Options {
    pub max_nodes: usize,
    pub max_edges: usize,
    pub work_limit: usize,
    pub output_limit: usize,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            max_nodes: 128,
            max_edges: 512,
            work_limit: 20_000_000,
            output_limit: 32 * 1024 * 1024,
        }
    }
}
#[derive(Clone, Debug)]
pub struct RenderedGraph {
    pub scene: Scene,
    pub svg: String,
}
mod drawing;
mod geometry;
mod labels;
mod path;
mod placement;
mod routing;
mod search;
mod validation;
pub fn render(graph: &Graph, options: &Options) -> Result<RenderedGraph, PeleError> {
    let mut budget = validation::Budget::new(options.work_limit);
    validation::graph(graph, options, &mut budget)?;
    let mut scene = placement::place(graph);
    routing::route(graph, &mut scene, &mut budget)?;
    labels::labels(&mut scene, &mut budget)?;
    placement::normalize(&mut scene);
    validation::scene(&scene, options, &mut budget)?;
    let svg = drawing::svg(&scene, options, &mut budget)?;
    Ok(RenderedGraph { scene, svg })
}
impl Scene {
    pub fn to_svg(&self, options: &Options) -> Result<String, PeleError> {
        let mut budget = validation::Budget::new(options.work_limit);
        validation::scene(self, options, &mut budget)?;
        drawing::svg(self, options, &mut budget)
    }
}
