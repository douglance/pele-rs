# Native graph scenes

Render a directed code graph without reconstructing Mermaid source. The graph API retains caller node and edge identities, accepts cycles, self-edges, parallel edges, and isolated nodes, and returns serializable scene geometry beside SVG. Mermaid parsing and all existing diagram-family contracts remain available through `compat`.

```rust
use pele::graph::{self, Graph, Node, Edge, Options};
let graph = Graph {
    nodes: vec![
        Node { id: "parse".into(), label: "parse".into(), detail: "function".into(), external: false },
        Node { id: "token".into(), label: "Token".into(), detail: "enum".into(), external: true },
    ],
    edges: vec![Edge { id: "parse-ref-token".into(), source: "parse".into(), target: "token".into(), label: "refs".into() }],
};
let output = graph::render(&graph, &Options::default())?;
assert_eq!(output.scene.edges[0].edge.id, "parse-ref-token");
std::fs::write("graph.svg", output.svg)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

`Graph`, `Node`, and `Edge` serialize through Serde. `RenderedGraph` contains SVG and a `Scene`. Each placed node contains the original node record and a rectangle. Each route contains the original edge record, orthogonal points, and a label rectangle. Node labels and detail strings are shortened only for drawing; the scene retains the original strings. SVG `data-id`, `data-source`, and `data-target` retain graph identities. SVG labels are XML-escaped. External nodes use dashed borders. `contains` and `refs` edges use dashed lines.

`Scene::to_svg` can draw a retained or caller-authored scene after identity and geometry validation. Horizontal paths jump over vertical crossings away from corners. Orthogonal corners use a four-pixel radius. Arrowheads follow the last route segment. Geometry coordinates use pixels; intersection checks tolerate one millionth of a pixel to avoid false intersections after floating-point translation. SVG path coordinates are emitted to two decimal places.

The engine uses the existing Pele layered rank and ordering implementation, then chooses clean node ports and obstacle-free orthogonal routes. Direct candidates avoid grid work when possible. Otherwise a direction-aware visibility search charges length, turns, and nonpreferred ports. One work budget covers graph validation, routing, labels, scene validation, and drawing. Labels reserve space outside node rectangles and preceding labels. Failure to route or place a label is an explicit semantic error.

Defaults: 128 nodes, 512 edges, 20 million work units, and 32 MiB SVG output. Callers may request up to 256 nodes; edges remain capped at 512. Graph expansion and layout are bounded by those item caps. Text consumes the work budget by byte length. Duplicate identities within either node or edge collections, empty or control-containing identities, and missing endpoints fail before layout. Input and scene records remain owned by the caller. Rendering reads no files, time, or environment.

CSS variables use the existing `--pele-*` namespace. The scene format is useful for hit testing and future backends; this change ships SVG only. AST extraction, file grouping, semantic layers, source metadata, and sheet ownership belong to the host application.

## Reuse and provenance

- Layered placement: existing Pele Rust port, retaining its upstream Pele MIT attribution.
- Obstacle visibility search and crossing jumps: adapted from astviewer `web/src/layout/orthogonal-router.ts` and `web/src/canvas/line-jumps.ts`.
- Rounded orthogonal presentation and build-time SVG: design reference [Ondine](https://github.com/wevm/ondine), inspected at `8d42a0aa603d14e488c46dc91723f87d0f0d4a44`; no Ondine implementation copied.
- Serializable scene separated from output: design reference [Rusty Mermaid](https://github.com/base58ed/rusty-mermaid); no implementation copied.
- Bounded validation and canonical identity: design reference [TML](https://tuios.dev/docs/libraries/tml); no implementation copied.

Integration tests exercise the public API, real AST sheet fixtures, malformed inputs, exhausted limits, crossing jumps, geometry, identity preservation, and deterministic SVG. The astviewer adapter additionally audits its original records against every scene and actual SVG and captures the running viewer.
