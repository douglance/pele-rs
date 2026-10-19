//! Literal detector ordering and prefix behavior from Pele src/detect.ts (458322a5).
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum DiagramType {
    #[serde(rename = "flowchart")]
    Flowchart,
    #[serde(rename = "pie")]
    Pie,
    #[serde(rename = "er")]
    Er,
    #[serde(rename = "mindmap")]
    Mindmap,
    #[serde(rename = "kanban")]
    Kanban,
    #[serde(rename = "timeline")]
    Timeline,
    #[serde(rename = "journey")]
    Journey,
    #[serde(rename = "class")]
    Class,
    #[serde(rename = "quadrantChart")]
    QuadrantChart,
    #[serde(rename = "xychart")]
    Xychart,
    #[serde(rename = "state")]
    State,
    #[serde(rename = "gantt")]
    Gantt,
    #[serde(rename = "architecture")]
    Architecture,
    #[serde(rename = "requirement")]
    Requirement,
    #[serde(rename = "sankey")]
    Sankey,
    #[serde(rename = "packet")]
    Packet,
    #[serde(rename = "radar")]
    Radar,
    #[serde(rename = "treemap")]
    Treemap,
    #[serde(rename = "info")]
    Info,
    #[serde(rename = "sequence")]
    Sequence,
    #[serde(rename = "gitGraph")]
    GitGraph,
    #[serde(rename = "c4")]
    C4,
    #[serde(rename = "block")]
    Block,
    #[serde(rename = "railroad")]
    Railroad,
    #[serde(rename = "agentflow")]
    Agentflow,
    #[serde(rename = "treeView")]
    TreeView,
    #[serde(rename = "cynefin")]
    Cynefin,
    #[serde(rename = "wardley")]
    Wardley,
    #[serde(rename = "eventmodeling")]
    Eventmodeling,
    #[serde(rename = "usecase")]
    Usecase,
    #[serde(rename = "venn")]
    Venn,
    #[serde(rename = "ishikawa")]
    Ishikawa,
    #[serde(rename = "swimlane")]
    Swimlane,
}
pub const DIAGRAM_TYPES: [DiagramType; 33] = [
    DiagramType::Flowchart,
    DiagramType::Pie,
    DiagramType::Er,
    DiagramType::Mindmap,
    DiagramType::Kanban,
    DiagramType::Timeline,
    DiagramType::Journey,
    DiagramType::Class,
    DiagramType::QuadrantChart,
    DiagramType::Xychart,
    DiagramType::State,
    DiagramType::Gantt,
    DiagramType::Architecture,
    DiagramType::Requirement,
    DiagramType::Sankey,
    DiagramType::Packet,
    DiagramType::Radar,
    DiagramType::Treemap,
    DiagramType::Info,
    DiagramType::Sequence,
    DiagramType::GitGraph,
    DiagramType::C4,
    DiagramType::Block,
    DiagramType::Railroad,
    DiagramType::Agentflow,
    DiagramType::TreeView,
    DiagramType::Cynefin,
    DiagramType::Wardley,
    DiagramType::Eventmodeling,
    DiagramType::Usecase,
    DiagramType::Venn,
    DiagramType::Ishikawa,
    DiagramType::Swimlane,
];
pub(crate) fn space(c: char) -> bool {
    matches!(c,'\t'..='\r'|' '| '\u{a0}'|'\u{1680}'|'\u{2000}'..='\u{200a}'|'\u{2028}'|'\u{2029}'|'\u{202f}'|'\u{205f}'|'\u{3000}'|'\u{feff}')
}
pub(crate) fn word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}
fn boundary(s: &str, n: usize) -> bool {
    s[n..].chars().next().is_none_or(|c| !word(c))
}
pub(crate) fn detect(source: &str) -> Option<DiagramType> {
    use DiagramType::*;
    let s = source.trim_start_matches(space);
    for (prefix, kind, bound) in [
        ("flowchart-elk", Flowchart, false),
        ("mindmap", Mindmap, false),
        ("architecture", Architecture, false),
        ("agentflow-beta", Agentflow, true),
    ] {
        if s.starts_with(prefix) && (!bound || boundary(s, prefix.len())) {
            return Some(kind);
        }
    }
    // Pinned regex anchors only C4Context. The other alternatives intentionally search anywhere.
    if s.starts_with("C4Context")
        || ["C4Container", "C4Component", "C4Dynamic", "C4Deployment"]
            .iter()
            .any(|p| s.contains(p))
    {
        return Some(C4);
    }
    for (prefix, kind, bound) in [
        ("kanban", Kanban, false),
        ("classDiagram", Class, false),
        ("erDiagram", Er, false),
        ("gantt", Gantt, false),
        ("info", Info, false),
        ("pie", Pie, false),
        ("requirement", Requirement, false),
        ("sequenceDiagram", Sequence, false),
        ("swimlane-beta", Swimlane, true),
        ("graph", Flowchart, false),
        ("flowchart", Flowchart, false),
        ("timeline", Timeline, false),
        ("gitGraph", GitGraph, false),
        ("stateDiagram", State, false),
        ("journey", Journey, false),
        ("quadrantChart", QuadrantChart, false),
        ("sankey", Sankey, false),
        ("packet", Packet, false),
        ("xychart", Xychart, false),
        ("block", Block, false),
        ("eventmodeling", Eventmodeling, false),
        ("treeView-beta", TreeView, false),
        ("radar-beta", Radar, false),
    ] {
        if s.starts_with(prefix) && (!bound || boundary(s, prefix.len())) {
            return Some(kind);
        }
    }
    let lower = s.to_ascii_lowercase();
    if lower.starts_with("ishikawa")
        && (boundary(s, 8) || lower.starts_with("ishikawa-beta") && boundary(s, 13))
    {
        return Some(Ishikawa);
    }
    if s.starts_with("treemap") {
        return Some(Treemap);
    }
    if [
        "railroad-beta",
        "railroad-ebnf-beta",
        "railroad-abnf-beta",
        "railroad-peg-beta",
    ]
    .iter()
    .any(|p| lower.starts_with(p))
    {
        return Some(Railroad);
    }
    if s.starts_with("venn-beta") {
        return Some(Venn);
    }
    if lower.starts_with("wardley-beta") {
        return Some(Wardley);
    }
    for (prefix, kind, colon) in [
        ("cynefin-beta", Cynefin, true),
        ("usecase-beta", Usecase, false),
    ] {
        if s.starts_with(prefix)
            && s[prefix.len()..]
                .chars()
                .next()
                .is_none_or(|c| space(c) || (colon && c == ':'))
        {
            return Some(kind);
        }
    }
    None
}

impl DiagramType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Flowchart => "flowchart",
            Self::Pie => "pie",
            Self::Er => "er",
            Self::Mindmap => "mindmap",
            Self::Kanban => "kanban",
            Self::Timeline => "timeline",
            Self::Journey => "journey",
            Self::Class => "class",
            Self::QuadrantChart => "quadrantChart",
            Self::Xychart => "xychart",
            Self::State => "state",
            Self::Gantt => "gantt",
            Self::Architecture => "architecture",
            Self::Requirement => "requirement",
            Self::Sankey => "sankey",
            Self::Packet => "packet",
            Self::Radar => "radar",
            Self::Treemap => "treemap",
            Self::Info => "info",
            Self::Sequence => "sequence",
            Self::GitGraph => "gitGraph",
            Self::C4 => "c4",
            Self::Block => "block",
            Self::Railroad => "railroad",
            Self::Agentflow => "agentflow",
            Self::TreeView => "treeView",
            Self::Cynefin => "cynefin",
            Self::Wardley => "wardley",
            Self::Eventmodeling => "eventmodeling",
            Self::Usecase => "usecase",
            Self::Venn => "venn",
            Self::Ishikawa => "ishikawa",
            Self::Swimlane => "swimlane",
        }
    }
}
