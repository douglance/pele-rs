use pele::{compat::ErrorCode, graph::*};
use std::collections::BTreeSet;
fn node(id: &str) -> Node {
    Node {
        id: id.into(),
        label: format!("{id} <&>"),
        detail: "function".into(),
        external: false,
    }
}
fn edge(id: &str, from: &str, to: &str) -> Edge {
    Edge {
        id: id.into(),
        source: from.into(),
        target: to.into(),
        label: "calls".into(),
    }
}
fn fixture() -> Graph {
    Graph {
        nodes: ["root", "left", "right", "isolated"]
            .map(|id| Node {
                external: id == "isolated",
                ..node(id)
            })
            .to_vec(),
        edges: vec![
            edge("one", "root", "left"),
            edge("two", "root", "right"),
            edge("three", "left", "right"),
            edge("back", "right", "root"),
            edge("parallel", "root", "left"),
            edge("self", "left", "left"),
        ],
    }
}
fn crosses(a: Point, b: Point, r: Rect) -> bool {
    if a.y == b.y {
        r.y < a.y && a.y < r.y + r.height && a.x.min(b.x) < r.x + r.width && a.x.max(b.x) > r.x
    } else {
        r.x < a.x && a.x < r.x + r.width && a.y.min(b.y) < r.y + r.height && a.y.max(b.y) > r.y
    }
}
#[test]
fn native_scene_retains_identity_geometry_and_determinism() {
    let g = fixture();
    let a = render(&g, &Options::default()).expect("A1 native scene");
    let b = render(&g, &Options::default()).unwrap();
    assert_eq!(a.scene, b.scene);
    assert_eq!(a.svg, b.svg);
    assert_eq!(
        a.scene
            .nodes
            .iter()
            .map(|n| n.node.id.as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["root", "left", "right", "isolated"])
    );
    assert_eq!(
        a.scene
            .edges
            .iter()
            .map(|e| e.edge.id.as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["one", "two", "three", "back", "parallel", "self"])
    );
    check_routes(&a.scene);
    check_labels(&a.scene);
    let one = &a
        .scene
        .edges
        .iter()
        .find(|e| e.edge.id == "one")
        .unwrap()
        .points;
    let parallel = &a
        .scene
        .edges
        .iter()
        .find(|e| e.edge.id == "parallel")
        .unwrap()
        .points;
    assert_ne!(one, parallel);
    check_svg(&a.svg, &g);
    assert_eq!(a.svg, a.scene.to_svg(&Options::default()).unwrap());
}
#[test]
fn crossings_jump_only_on_horizontal_routes_away_from_corners() {
    let scene = crossing_scene();
    let svg = scene
        .to_svg(&Options::default())
        .expect("A2 crossing scene");
    let doc = roxmltree::Document::parse(&svg).unwrap();
    let path = |id| {
        doc.descendants()
            .find(|n| n.attribute("data-id") == Some(id))
            .unwrap()
            .descendants()
            .find(|n| n.has_tag_name("path"))
            .unwrap()
            .attribute("d")
            .unwrap()
    };
    assert_eq!(
        path("horizontal"),
        "M40.00,100.00L106.00,100.00A4,4 0 0 1 114.00,100.00L180.00,100.00"
    );
    assert_eq!(path("vertical"), "M110.00,20.00L110.00,180.00");
    assert!(svg.contains("M180,100L173,103L173,97Z"));
    assert!(svg.contains("M110,180L107,173L113,173Z"));
}
#[test]
fn invalid_graphs_and_exhausted_budgets_fail_explicitly() {
    let check = |g: Graph, word: &str| {
        let e = render(&g, &Options::default()).unwrap_err();
        assert_eq!(e.code, ErrorCode::Semantic);
        assert!(e.message.contains(word), "{}", e.message);
    };
    let mut g = fixture();
    g.nodes.push(node("root"));
    check(g, "duplicate");
    let mut g = fixture();
    g.edges.push(edge("one", "root", "left"));
    check(g, "duplicate");
    let mut g = fixture();
    g.edges[0].target = "missing".into();
    check(g, "endpoint");
    for options in [
        Options {
            work_limit: 1,
            ..Options::default()
        },
        Options {
            output_limit: 1,
            ..Options::default()
        },
        Options {
            max_nodes: 1,
            ..Options::default()
        },
    ] {
        let e = render(&fixture(), &options).unwrap_err();
        assert_eq!(e.code, ErrorCode::Limit);
    }
}

#[test]
fn real_ast_boundaries_survive_coordinate_translation() {
    let graph: Graph =
        serde_json::from_str(include_str!("fixtures/ast-scene-boundaries.json")).unwrap();
    let output = render(&graph, &Options::default())
        .expect("A1 real AST routes remain outside node interiors");
    assert_eq!(output.scene.edges.len(), 24);
}

#[test]
fn real_ast_call_routes_survive_coordinate_translation() {
    let graph: Graph =
        serde_json::from_str(include_str!("fixtures/ast-scene-call-boundaries.json")).unwrap();
    let output = render(&graph, &Options::default()).expect("A1 real AST call routes");
    assert!(!output.scene.edges.is_empty());
}

#[test]
fn visibility_search_routes_a_real_ast_control_flow() {
    let graph: Graph =
        serde_json::from_str(include_str!("fixtures/ast-scene-visibility-search.json")).unwrap();
    let output = render(&graph, &Options::default()).expect("A1 obstacle visibility search");
    assert_eq!(output.scene.edges.len(), 24);
    assert!(
        output.scene.edges.iter().any(|e| e.points.len() > 6),
        "fixture reaches visibility search beyond direct candidates"
    );
}

fn check_routes(scene: &Scene) {
    for e in &scene.edges {
        assert!(e.points.len() >= 2);
        for p in e.points.windows(2) {
            assert!(
                p[0].x == p[1].x || p[0].y == p[1].y,
                "diagonal {}",
                e.edge.id
            );
            assert_ne!(p[0], p[1]);
            for n in &scene.nodes {
                assert!(
                    !crosses(p[0], p[1], n.rect),
                    "{} cuts {}",
                    e.edge.id,
                    n.node.id
                );
            }
        }
    }
}

fn check_svg(source: &str, g: &Graph) {
    let svg = roxmltree::Document::parse(source).unwrap();
    assert_eq!(
        svg.descendants()
            .filter(|n| n.attribute("class") == Some("pele-edge"))
            .count(),
        6
    );
    for e in &g.edges {
        assert!(svg
            .descendants()
            .any(|n| n.attribute("data-id") == Some(&e.id)
                && n.attribute("data-source") == Some(&e.source)
                && n.attribute("data-target") == Some(&e.target)));
    }
    assert!(!svg.descendants().any(|n| n.has_tag_name("script")));
    assert_eq!(
        svg.descendants()
            .filter(|n| n.attribute("class") == Some("pele-node"))
            .count(),
        4
    );
    assert!(source.contains("&lt;&amp;&gt;"));
    assert!(svg
        .descendants()
        .any(|n| n.has_tag_name("text") && n.text() == Some("root <&>")));
    let isolated = svg
        .descendants()
        .find(|n| n.attribute("data-id") == Some("isolated"))
        .unwrap();
    assert!(isolated
        .descendants()
        .any(|n| n.has_tag_name("rect") && n.attribute("stroke-dasharray") == Some("4 3")));
}

fn overlaps(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.width && a.x + a.width > b.x && a.y < b.y + b.height && a.y + a.height > b.y
}
fn check_labels(scene: &Scene) {
    let mut occupied = scene.nodes.iter().map(|n| n.rect).collect::<Vec<_>>();
    for e in &scene.edges {
        assert!(e.label_box.width > 0. && e.label_box.height >= 20.);
        assert!(
            !occupied.iter().any(|r| overlaps(*r, e.label_box)),
            "overlapping label {}",
            e.edge.id
        );
        occupied.push(e.label_box);
    }
}
#[test]
fn malformed_retained_scenes_are_rejected_before_drawing() {
    let source = render(&fixture(), &Options::default()).unwrap().scene;
    for kind in 0..4 {
        let mut scene = source.clone();
        match kind {
            0 => scene.width = f64::NAN,
            1 => scene.nodes[0].rect.width = f64::NAN,
            2 => scene.edges[0].points = vec![Point { x: 0., y: 0. }, Point { x: 50., y: 50. }],
            _ => scene.edges[0].points = vec![Point { x: 0., y: 0. }, Point { x: 0., y: 0. }],
        };
        assert_eq!(
            scene.to_svg(&Options::default()).unwrap_err().code,
            ErrorCode::Semantic
        );
    }
}

fn crossing_scene() -> Scene {
    let positions = [
        ("a", 0., 80., 40., 40.),
        ("b", 180., 80., 40., 40.),
        ("c", 90., 0., 40., 20.),
        ("d", 90., 180., 40., 20.),
    ];
    Scene {
        width: 240.,
        height: 220.,
        nodes: positions
            .map(|(id, x, y, width, height)| PlacedNode {
                node: node(id),
                rect: Rect {
                    x,
                    y,
                    width,
                    height,
                },
            })
            .to_vec(),
        edges: vec![
            Route {
                edge: edge("horizontal", "a", "b"),
                points: vec![Point { x: 40., y: 100. }, Point { x: 180., y: 100. }],
                label_box: Rect::default(),
            },
            Route {
                edge: edge("vertical", "c", "d"),
                points: vec![Point { x: 110., y: 20. }, Point { x: 110., y: 180. }],
                label_box: Rect::default(),
            },
        ],
    }
}

#[test]
fn orthogonal_corners_round_without_losing_crossing_jumps() {
    let mut scene = crossing_scene();
    scene.nodes[1].rect.y = 120.;
    scene.edges[0].points = vec![
        Point { x: 40., y: 100. },
        Point { x: 80., y: 100. },
        Point { x: 80., y: 140. },
        Point { x: 180., y: 140. },
    ];
    let svg = scene.to_svg(&Options::default()).unwrap();
    let doc = roxmltree::Document::parse(&svg).unwrap();
    let group = doc
        .descendants()
        .find(|n| n.attribute("data-id") == Some("horizontal"))
        .unwrap();
    let path = group
        .descendants()
        .find(|n| n.attribute("class") == Some("graph-edge"))
        .unwrap()
        .attribute("d")
        .unwrap();
    assert_eq!(path,"M40.00,100.00L76.00,100.00Q80.00,100.00 80.00,104.00L80.00,136.00Q80.00,140.00 84.00,140.00L106.00,140.00A4,4 0 0 1 114.00,140.00L180.00,140.00");
}

#[test]
fn xml_invalid_graph_identities_are_rejected() {
    for id in ["\u{ffff}", "\u{fffe}"] {
        let mut graph = fixture();
        graph.nodes[0].id = id.into();
        for e in &mut graph.edges {
            if e.source == "root" {
                e.source = id.into();
            }
            if e.target == "root" {
                e.target = id.into();
            }
        }
        let error = render(&graph, &Options::default())
            .expect_err("invalid XML identity must not silently change in SVG");
        assert_eq!(error.code, ErrorCode::Semantic);
        assert!(error.message.contains("identity"));
    }
}
