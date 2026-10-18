//! Feature-slice integration: callers' Rust interface, independent XML and raster consumers.
use pele::{parse, render, render_json, Direction, RenderOptions};
use serde_json::Value;
use std::{fs, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn manifest() -> Vec<Value> {
    serde_json::from_str(&fs::read_to_string(root().join("fixtures/manifest.json")).unwrap())
        .unwrap()
}
fn source(f: &Value) -> String {
    f["source"].as_str().map(str::to_owned).unwrap_or_else(|| {
        fs::read_to_string(root().join("fixtures").join(f["file"].as_str().unwrap())).unwrap()
    })
}

#[test]
fn shared_flowchart_corpus_renders_or_reports_documented_error() {
    for fixture in manifest() {
        let source = source(&fixture);
        let result = render(&source, &RenderOptions::default());
        if let Some(code) = fixture["error"].as_str() {
            assert_eq!(result.unwrap_err().code, code, "{}", fixture["name"]);
            continue;
        }
        let diagram = result.unwrap_or_else(|e| panic!("{}: {e}", fixture["name"]));
        assert_eq!(
            diagram.geometry.nodes.len(),
            fixture["nodes"].as_u64().unwrap() as usize
        );
        assert_eq!(
            diagram.geometry.edges.len(),
            fixture["edges"].as_u64().unwrap() as usize
        );
        assert_eq!(
            serde_json::to_value(diagram.geometry.direction).unwrap(),
            fixture["direction"]
        );
        assert_eq!(
            render(&source, &RenderOptions::default()).unwrap().svg,
            diagram.svg,
            "determinism"
        );
        let xml = roxmltree::Document::parse(&diagram.svg).unwrap();
        for n in xml.descendants().filter(|n| n.is_element()) {
            assert!(
                ["svg", "rect", "g", "title", "desc", "path", "circle", "polygon"]
                    .contains(&n.tag_name().name())
            );
            assert!(!n
                .attributes()
                .any(|a| a.name().starts_with("on") || a.name() == "href"));
        }
        assert!(!diagram.svg.contains("var("));
        assert!(!diagram.svg.contains("<text"));
        assert!(diagram.svg.contains("<path"));
        for n in &diagram.geometry.nodes {
            let r = n.bounds;
            assert!(
                r.x >= 0.
                    && r.y >= 0.
                    && r.x + r.width <= diagram.width
                    && r.y + r.height <= diagram.height
            );
        }
        for e in &diagram.geometry.edges {
            assert!(e.points.len() >= 2);
            assert!(e.points.iter().all(|p| p.x.is_finite()
                && p.y.is_finite()
                && p.x >= 0.
                && p.y >= 0.
                && p.x <= diagram.width
                && p.y <= diagram.height));
        }
        let tree =
            resvg::usvg::Tree::from_str(&diagram.svg, &resvg::usvg::Options::default()).unwrap();
        let mut image = resvg::tiny_skia::Pixmap::new(
            (diagram.width * 0.5).ceil() as u32,
            (diagram.height * 0.5).ceil() as u32,
        )
        .unwrap();
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::from_scale(0.5, 0.5),
            &mut image.as_mut(),
        );
        let painted = image
            .data()
            .chunks_exact(4)
            .filter(|p| p[0] < 180 && p[3] > 0)
            .count();
        assert!(painted > 50, "{} has no meaningful pixels", fixture["name"]);
    }
}

#[test]
fn joins_cycles_parallel_edges_and_all_directions_preserve_graph_meaning() {
    for direction in ["TB", "BT", "LR", "RL"] {
        let source=format!("flowchart {direction}\nA-->B-->D\nA-->C-->E-->D\nA-->|skip|D\nD-->|return|A\nB-->|again|B\nA-->|parallel|B");
        let d = render(&source, &RenderOptions::default()).unwrap();
        let node = |id: &str| {
            d.geometry
                .nodes
                .iter()
                .find(|n| n.id == id)
                .unwrap()
                .bounds
                .center()
        };
        let forward = |id: &str| {
            let p = node(id);
            match d.geometry.direction {
                Direction::TB => p.y,
                Direction::BT => -p.y,
                Direction::LR => p.x,
                Direction::RL => -p.x,
            }
        };
        assert!(forward("D") > forward("B") && forward("D") > forward("E"));
        assert_eq!(d.geometry.edges.len(), 9);
        assert_ne!(d.geometry.edges[0].points, d.geometry.edges[8].points);
        for (i, a) in d.geometry.nodes.iter().enumerate() {
            for b in &d.geometry.nodes[i + 1..] {
                let a = a.bounds;
                let b = b.bounds;
                assert!(
                    a.x + a.width <= b.x
                        || b.x + b.width <= a.x
                        || a.y + a.height <= b.y
                        || b.y + b.height <= a.y
                );
            }
        }
    }
    let g = parse("flowchart LR; A[First]; A-->B; A{Last}; A-->B").unwrap();
    assert_eq!(g.nodes()[0].label, "Last");
    assert_eq!(g.edges().len(), 2);
}

#[test]
fn untrusted_inputs_and_work_budgets_fail_without_partial_output() {
    let code = |s: &str, options: &str| {
        let v: Value = serde_json::from_str(&render_json(s, options)).unwrap();
        assert_eq!(v["ok"], false);
        assert!(v.get("result").is_none());
        v["error"]["code"].as_str().unwrap().to_owned()
    };
    assert_eq!(code(&"A".repeat(262145), ""), "input_limit");
    assert_eq!(
        code(&format!("graph TD\nA[{}]", "B".repeat(4097)), ""),
        "graph_limit"
    );
    let nodes = (0..257)
        .map(|i| format!("N{i}"))
        .collect::<Vec<_>>()
        .join(";");
    assert_eq!(code(&format!("graph TD;{nodes}"), ""), "graph_limit");
    let edges = "A-->B;".repeat(1025);
    assert_eq!(code(&format!("graph TD;{edges}"), ""), "graph_limit");
    for o in [
        "{\"font_size\":0}",
        "{\"font_size\":-1}",
        "{\"font_size\":1e300}",
        "{\"font_size\":NaN}",
        "{\"typo\":1}",
        "{\"theme\":{\"line\":\"url(https://example.com)\"}}",
    ] {
        assert_eq!(code("graph TD;A", o), "invalid_options");
    }
    let chain = (0..255)
        .map(|i| format!("N{i}-->N{};", i + 1))
        .collect::<String>();
    assert_eq!(
        code(&format!("graph TD;{chain}{}", "N0-->N255;".repeat(100)), ""),
        "layout_limit"
    );
    let huge = (0..32)
        .map(|i| {
            format!(
                "N{i}[{}]{};",
                "B".repeat(2048),
                if i < 31 {
                    format!("-->N{}", i + 1)
                } else {
                    String::new()
                }
            )
        })
        .collect::<String>();
    assert_eq!(code(&format!("graph TD;{huge}"), ""), "output_limit");
    assert!(
        render("graph TD;A-->B", &RenderOptions::default()).is_ok(),
        "failed render cannot poison next call"
    );
}

#[test]
fn public_errors_include_correct_utf8_source_locations() {
    let source = "graph TB\r\nA[é]; click A callback";
    let err = render(source, &RenderOptions::default()).unwrap_err();
    let span = err.span.unwrap();
    assert_eq!(span.line, 2);
    assert_eq!(span.column, 7);
    assert_eq!(&source[span.start..span.end], "c");
}

#[test]
fn varied_workflows_keep_every_edge_and_find_unobstructed_routes() {
    for seed in 0..32usize {
        let direction = ["TB", "BT", "LR", "RL"][seed % 4];
        let mut source = format!("flowchart {direction};");
        for i in 0..12 {
            source.push_str(&format!("N{i}[Step {i}];"));
            if i > 0 {
                source.push_str(&format!("N{}-->N{i};", i - 1));
            }
        }
        let mut edges = 11;
        for i in 0..9 {
            if (i + seed) % 3 != 0 {
                source.push_str(&format!("N{i}-->|branch {i}|N{};", i + 3));
                edges += 1;
            }
        }
        source.push_str("N11-->|Retry|N0;N4-->|Loop|N4;Separate[Independent];");
        let diagram = render(&source, &RenderOptions::default())
            .unwrap_or_else(|e| panic!("workflow {seed}: {e}"));
        assert_eq!(diagram.geometry.nodes.len(), 13);
        assert_eq!(diagram.geometry.edges.len(), edges + 2);
    }
    let parallel = format!("graph TB;{}", "A-->B;".repeat(128));
    let rendered = render(&parallel, &RenderOptions::default()).unwrap();
    assert_eq!(rendered.geometry.edges.len(), 128);
    let mut ports: Vec<f64> = rendered
        .geometry
        .edges
        .iter()
        .map(|e| e.points.last().unwrap().x)
        .collect();
    ports.sort_by(f64::total_cmp);
    assert!(ports.windows(2).all(|p| p[1] - p[0] >= 11.99));
}
