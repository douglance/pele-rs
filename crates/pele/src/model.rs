use crate::{Error, Span};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Direction {
    TB,
    BT,
    LR,
    RL,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Shape {
    Rectangle,
    Rounded,
    Diamond,
    Circle,
    Stadium,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LineStyle {
    Solid,
    Dashed,
    Thick,
}

#[derive(Clone, Debug)]
pub struct Node {
    pub id: String,
    pub label: String,
    pub shape: Shape,
    pub span: Span,
}
#[derive(Clone, Debug)]
pub struct Edge {
    pub source: usize,
    pub target: usize,
    pub label: String,
    pub style: LineStyle,
    pub arrow: bool,
    pub span: Span,
}
/// Constructed by `parse`; immutable graph access prevents invalid foreign models.
#[derive(Clone, Debug)]
pub struct Flowchart {
    pub(crate) direction: Direction,
    pub(crate) nodes: Vec<Node>,
    pub(crate) edges: Vec<Edge>,
}
impl Flowchart {
    pub fn direction(&self) -> Direction {
        self.direction
    }
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }
    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}
#[derive(Clone, Copy, Debug, Serialize, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
impl Rect {
    pub fn center(&self) -> Point {
        Point {
            x: self.x + self.width / 2.,
            y: self.y + self.height / 2.,
        }
    }
    pub(crate) fn overlaps(&self, b: &Self) -> bool {
        self.x < b.x + b.width - 0.001
            && b.x < self.x + self.width - 0.001
            && self.y < b.y + b.height - 0.001
            && b.y < self.y + self.height - 0.001
    }
}
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct PositionedNode {
    pub id: String,
    pub label: String,
    pub shape: Shape,
    pub bounds: Rect,
}
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct PositionedEdge {
    pub index: usize,
    pub source: String,
    pub target: String,
    pub label: String,
    pub style: LineStyle,
    pub arrow: bool,
    pub points: Vec<Point>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label_bounds: Option<Rect>,
}
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Geometry {
    pub version: u32,
    pub direction: Direction,
    pub nodes: Vec<PositionedNode>,
    pub edges: Vec<PositionedEdge>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Theme {
    pub background: String,
    pub foreground: String,
    pub surface: String,
    pub border: String,
    pub line: String,
}
impl Default for Theme {
    fn default() -> Self {
        Self {
            background: "#ffffff".into(),
            foreground: "#1f1f1f".into(),
            surface: "#f3f3f3".into(),
            border: "#8a8a8a".into(),
            line: "#1f1f1f".into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RenderOptions {
    pub theme: Theme,
    pub font_size: f64,
    pub edge_font_size: f64,
    pub node_gap: f64,
    pub rank_gap: f64,
    pub padding: f64,
    pub node_padding: f64,
    pub radius: f64,
    pub stroke_width: f64,
    pub title: String,
}
impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            font_size: 16.,
            edge_font_size: 14.,
            node_gap: 40.,
            rank_gap: 48.,
            padding: 16.,
            node_padding: 16.,
            radius: 4.,
            stroke_width: 1.5,
            title: "Flowchart".into(),
        }
    }
}
impl RenderOptions {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        for color in [
            &self.theme.background,
            &self.theme.foreground,
            &self.theme.surface,
            &self.theme.border,
            &self.theme.line,
        ] {
            if color.len() != 7
                || !color.starts_with('#')
                || !color[1..].bytes().all(|b| b.is_ascii_hexdigit())
            {
                return Err(Error::new("invalid_options", "Colors must be #RRGGBB"));
            }
        }
        for (name, value, min, max) in [
            ("font_size", self.font_size, 8., 72.),
            ("edge_font_size", self.edge_font_size, 8., 72.),
            ("node_gap", self.node_gap, 16., 512.),
            ("rank_gap", self.rank_gap, 24., 512.),
            ("padding", self.padding, 8., 512.),
            ("node_padding", self.node_padding, 8., 512.),
            ("radius", self.radius, 0., 128.),
            ("stroke_width", self.stroke_width, 0.5, 8.),
        ] {
            if !value.is_finite() || value < min || value > max {
                return Err(Error::new(
                    "invalid_options",
                    format!("{name} must be finite and in {min}..={max}"),
                ));
            }
        }
        if self.title.len() > 4096 || self.title.chars().any(invalid_xml) {
            return Err(Error::new("invalid_options", "Invalid accessible title"));
        }
        Ok(())
    }
}
pub(crate) fn invalid_xml(c: char) -> bool {
    !matches!(c, '\t'|'\n'|'\r'|'\u{20}'..='\u{d7ff}'|'\u{e000}'..='\u{fffd}'|'\u{10000}'..='\u{10ffff}')
}
pub(crate) fn q(v: f64) -> f64 {
    let v = (v * 1000.).round() / 1000.;
    if v == 0. {
        0.
    } else {
        v
    }
}
