//! Mutable XYChart objects/builders against Pele 1.6.0, 458322a5.
//! Public entry node_modules/pele/dist/pele.js SHA256
//! d1ebc1889f3069d7759fe5db34c4ad4b4823285edbee2b537d0ef3d505a76d8d;
//! src/diagrams/xychart/db.ts SHA256
//! 888e3fb245bff87b73d44df9edc12bf2b340935eb0af6a94582154553bcdfff7.
//! Prototype probe: constructor plus 13 methods; all method descriptors flags6,
//! arities match schema below. Constructor minifier name is not an API promise.
use pele::compat::{
    diagrams::xychart::{XyChart, XyChartModel},
    public::*,
    text::MetricsResources,
    *,
};
use sha2::{Digest, Sha256};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
const SOURCE:&str="xychart-beta\ntitle Before\naccTitle: Access\naccDescr: Details\nx-axis Months [Jan,Feb,Mar]\ny-axis Revenue 0 --> 100\nbar Sales [10,20,30]\nline Growth [15 \"p\",25,35 \"q\"]";
fn reference(a: &Atom) -> u32 {
    if let Atom::Ref(i) = a {
        *i
    } else {
        panic!("reference")
    }
}
fn root(g: &ModelGraph) -> u32 {
    reference(&g.root)
}
fn field<'a>(g: &'a ModelGraph, id: u32, key: &str) -> &'a Atom {
    &g.nodes[id as usize]
        .properties()
        .iter()
        .find(|p| g.strings[p.key as usize].is(key))
        .unwrap()
        .value
}
fn set(g: &mut ModelGraph, id: u32, key: &str, v: Atom) {
    if let Some(i) = g.nodes[id as usize]
        .properties()
        .iter()
        .position(|p| g.strings[p.key as usize].is(key))
    {
        g.nodes[id as usize].properties_mut()[i].value = v;
        return;
    }
    let key_id = g.strings.len() as u32;
    g.strings.push(key.into());
    g.nodes[id as usize].properties_mut().push(Property {
        key: key_id,
        value: v,
        flags: 7,
    });
}
fn remove(g: &mut ModelGraph, id: u32, key: &str) {
    if let Some(i) = g.nodes[id as usize]
        .properties()
        .iter()
        .position(|p| g.strings[p.key as usize].is(key))
    {
        g.nodes[id as usize].properties_mut().remove(i);
    }
}
fn string(g: &mut ModelGraph, s: &str) -> Atom {
    let id = g.strings.len() as u32;
    g.strings.push(s.into());
    Atom::String(id)
}
fn set_text(g: &mut ModelGraph, id: u32, key: &str, s: &str) {
    let a = string(g, s);
    set(g, id, key, a)
}
fn object(g: &mut ModelGraph) -> u32 {
    let id = g.nodes.len() as u32;
    g.nodes.push(Node::Object { properties: vec![] });
    id
}
fn array(g: &mut ModelGraph, values: Vec<Atom>) -> u32 {
    let id = g.nodes.len() as u32;
    g.nodes.push(Node::Array {
        length: values.len() as u32,
        properties: vec![],
    });
    for (i, a) in values.into_iter().enumerate() {
        set(g, id, &i.to_string(), a)
    }
    id
}
fn xy_text(g: &mut ModelGraph, s: &str) -> Atom {
    let id = object(g);
    set_text(g, id, "text", s);
    set_text(g, id, "type", "text");
    Atom::Ref(id)
}
fn points(g: &mut ModelGraph, entries: &[(f64, &str)]) -> Atom {
    let mut items = vec![];
    for (v, s) in entries {
        let id = object(g);
        set(g, id, "value", Atom::Number(*v));
        set_text(g, id, "label", s);
        items.push(Atom::Ref(id));
    }
    Atom::Ref(array(g, items))
}
fn graph_string(g: &ModelGraph, a: &Atom) -> String {
    g.text(a).unwrap().well_formed().unwrap()
}
fn keys(g: &ModelGraph, id: u32) -> Vec<String> {
    g.nodes[id as usize]
        .properties()
        .iter()
        .map(|p| g.strings[p.key as usize].well_formed().unwrap())
        .collect()
}
fn fingerprint(g: &ModelGraph) -> String {
    serde_json::to_string(g).unwrap()
}
fn hash(s: &str) -> String {
    format!("{:x}", Sha256::digest(s.as_bytes()))
}
fn chart_ids(g: &ModelGraph) -> (u32, u32, u32, u32) {
    let r = root(g);
    (
        r,
        reference(field(g, r, "xAxis")),
        reference(field(g, r, "yAxis")),
        reference(field(g, r, "plots")),
    )
}
fn call(
    doc: &mut PublicDocument,
    name: &str,
    args: impl FnOnce(&mut ModelGraph) -> Vec<Atom>,
) -> InvocationOutcome {
    let method = doc
        .model_schema()
        .methods
        .iter()
        .find(|m| m.name == name)
        .unwrap()
        .id;
    let mut g = doc.snapshot().unwrap();
    let receiver = g.root.clone();
    let arguments = args(&mut g);
    doc.apply(doc.revision(), g).unwrap();
    doc.invoke(
        doc.revision(),
        MethodCall {
            method,
            receiver,
            arguments,
        },
        &mut MetricsResources,
    )
    .unwrap()
}
fn success(out: InvocationOutcome) -> ModelGraph {
    assert!(matches!(out.result, Ok(Atom::Undefined)));
    out.graph
}
struct Counted {
    parses: Arc<AtomicUsize>,
    renders: Arc<AtomicUsize>,
}
impl DiagramModule for Counted {
    type Model = XyChartModel;
    fn descriptor(&self) -> ModuleDescriptor {
        XyChart.descriptor()
    }
    fn parse(&self, c: ParseContext<'_>) -> Result<Self::Model, PeleError> {
        self.parses.fetch_add(1, Ordering::SeqCst);
        XyChart.parse(c)
    }
    fn render(
        &self,
        m: &Self::Model,
        c: &Object,
        o: &RenderSettings,
        h: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError> {
        XyChart.render(m, c, o, h)
    }
}
impl PublicDiagramModule for Counted {
    type PublicState = <XyChart as PublicDiagramModule>::PublicState;
    fn model_schema(&self) -> &'static ModelSchema {
        XyChart.model_schema()
    }
    fn create_public(&self, m: &Self::Model) -> Result<Self::PublicState, PeleError> {
        XyChart.create_public(m)
    }
    fn export_public(&self, s: &Self::PublicState) -> Result<ModelGraph, PeleError> {
        XyChart.export_public(s)
    }
    fn apply_public(&self, s: &mut Self::PublicState, g: ModelGraph) -> Result<(), PeleError> {
        XyChart.apply_public(s, g)
    }
    fn render_public(
        &self,
        s: &mut Self::PublicState,
        c: &Object,
        o: &RenderSettings,
        h: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError> {
        self.renders.fetch_add(1, Ordering::SeqCst);
        XyChart.render_public(s, c, o, h)
    }
    fn invoke_public(
        &self,
        s: &mut Self::PublicState,
        call: MethodCall,
        c: &Object,
        h: &mut dyn RenderResources,
    ) -> Result<Atom, PublicError> {
        XyChart.invoke_public(s, call, c, h)
    }
}
#[test]
fn xychart_instance_fields_source_equivalent_edits_and_retained_registration() {
    let parses = Arc::new(AtomicUsize::new(0));
    let renders = Arc::new(AtomicUsize::new(0));
    let mut engine = PublicEngine::empty();
    engine.register(Counted {
        parses: parses.clone(),
        renders: renders.clone(),
    });
    let empty = engine
        .parse("xychart-beta", &Default::default())
        .unwrap()
        .snapshot()
        .unwrap();
    for key in ["title", "accTitle", "accDescr", "orientation"] {
        assert!(matches!(empty.field(key).unwrap(), Some(Atom::Undefined)))
    }
    let mut doc = engine.parse(SOURCE, &Default::default()).unwrap();
    let g = doc.snapshot().unwrap();
    let (r, x, y, plots) = chart_ids(&g);
    assert!(matches!(
        g.nodes[r as usize],
        Node::Instance {
            prototype: PrototypeId(1),
            ..
        }
    ));
    assert_eq!(
        keys(&g, r),
        [
            "type",
            "title",
            "accTitle",
            "accDescr",
            "orientation",
            "xAxis",
            "yAxis",
            "hasSetXAxis",
            "hasSetYAxis",
            "plots"
        ]
    );
    assert!(g.nodes[r as usize]
        .properties()
        .iter()
        .all(|p| p.flags == 7));
    assert_eq!(keys(&g, x), ["type", "title", "categories"]);
    assert_eq!(keys(&g, y), ["type", "title", "min", "max"]);
    let p0 = reference(field(&g, plots, "0"));
    let p1 = reference(field(&g, plots, "1"));
    assert_eq!(keys(&g, p0), ["type", "title", "data"]);
    assert_eq!(keys(&g, p1), ["type", "title", "data", "pointLabels"]);
    assert_ne!(
        reference(field(&g, p0, "data")),
        reference(field(&g, p1, "data"))
    );
    assert_eq!(doc.model_schema().id, "pele/xychart@1");
    assert_eq!(doc.model_schema().methods.len(), 13);
    assert!(doc.model_schema().prototypes[0]
        .members
        .iter()
        .all(|m| m.flags == 6));
    let initial = doc
        .render(&Default::default(), &mut MetricsResources)
        .unwrap();
    assert_eq!(
        initial,
        Engine::with_builtins()
            .render_source(SOURCE, &Default::default(), &mut MetricsResources)
            .unwrap()
    );
    let mut g = g;
    set_text(&mut g, r, "type", "unrelated");
    set_text(&mut g, r, "title", "Edited");
    set_text(&mut g, r, "orientation", "horizontal");
    set_text(&mut g, r, "accTitle", "New access");
    set(&mut g, r, "accDescr", Atom::Undefined);
    set_text(&mut g, x, "title", "Labels");
    set_text(&mut g, y, "title", "Amount");
    set(&mut g, y, "min", Atom::Number(-20.));
    set(&mut g, y, "max", Atom::Number(80.));
    let cats = reference(field(&g, x, "categories"));
    for (i, s) in ["A", "B", "C"].iter().enumerate() {
        set_text(&mut g, cats, &i.to_string(), s)
    }
    for (p, title, values) in [
        (p0, "Bars", [20., 40., 60.]),
        (p1, "Points", [30., 50., 70.]),
    ] {
        set_text(&mut g, p, "title", title);
        let data = reference(field(&g, p, "data"));
        for (i, n) in values.into_iter().enumerate() {
            let point = reference(field(&g, data, &i.to_string()));
            set(&mut g, point, "1", Atom::Number(n));
        }
    }
    let labels = reference(field(&g, p1, "pointLabels"));
    for (i, s) in ["first", "", "last"].iter().enumerate() {
        set_text(&mut g, labels, &i.to_string(), s)
    }
    doc.apply(0, g).unwrap();
    let replacement = Arc::new(AtomicUsize::new(0));
    engine.register(Counted {
        parses: parses.clone(),
        renders: replacement.clone(),
    });
    drop(engine);
    let expected="xychart-beta horizontal\ntitle Edited\naccTitle: New access\nx-axis Labels [A,B,C]\ny-axis Amount -20 --> 80\nbar Bars [20,40,60]\nline Points [30 \"first\",50,70 \"last\"]";
    for max_width in [None, Some(320.)] {
        let mut options = RenderOptions::default();
        options.render.max_width = max_width;
        let rendered = doc.render(&options, &mut MetricsResources).unwrap();
        let expected = Engine::with_builtins()
            .render_source(expected, &options, &mut MetricsResources)
            .unwrap();
        assert_eq!(rendered, expected);
        assert_eq!(rendered.diagram_type, DiagramType::Xychart.into());
    }
    assert_eq!(parses.load(Ordering::SeqCst), 2);
    assert_eq!(renders.load(Ordering::SeqCst), 3);
    assert_eq!(replacement.load(Ordering::SeqCst), 0);
}
#[test]
fn xychart_aliases_reorder_replacements_nonfinite_and_unused_utf16_match_pinned_svg() {
    let probes = [
        (
            "original",
            "eddb5f80b0127edb831b3352cd46bfa3944fca43c6b2edcfac46f688205ec35f",
        ),
        (
            "horizontal",
            "14ace272a0a196a0d1df817f103e66a9eec206c1da466f6ee661d2fbf2636d75",
        ),
        (
            "reverse-plots",
            "3c4b3a776b8a3a46d066b4dd3a8b02d3643d3b3561210baf49e9f8b388e75528",
        ),
        (
            "reverse-categories",
            "d0b4a082c7f8aa669ac62408608bc15f7de662ce0f9e65938d8c6d731a66b2dd",
        ),
        (
            "alias-axes",
            "191d0a47d9266e5bfc7fa4245e1ea4224b655203a5e7c0c19f2ce21ca7432d9d",
        ),
        (
            "alias-plots",
            "517688820c2a1521c7069d3bba9fa26058b6794aec1fd51aac636252b1d14d24",
        ),
        (
            "alias-data",
            "f75321fa61dbb094559291320e03b7c5d6f9f2e6060693f25fe6ce03a840e9fb",
        ),
        (
            "auto-range",
            "864114dbd92a444eed1937683cea61f250d87893c4a737b127047162ab654c98",
        ),
        (
            "point-labels",
            "599c3c6d86a16636651c093e97d8e03a870ddaba0a3f5a1dc416a93dd6ffcdcb",
        ),
        (
            "missing-value",
            "13c4a371019eddf4d4817691a5b0ec384a5be2c0fffb43001f31f72a4fca393f",
        ),
        (
            "unused-surrogate",
            "1dd7b42050bd8b03720459f3471cfe551a834736d8c6d1e746d51e3c74655ec3",
        ),
        (
            "nonfinite",
            "4f4b5211dba6927822b9ec0176e8d355ba3ad553860b9c72ecc05c57ac1ce66e",
        ),
    ];
    let mut engine = PublicEngine::empty();
    engine.register(XyChart);
    for (name, expected) in probes {
        let mut doc = engine.parse(SOURCE, &Default::default()).unwrap();
        let mut g = doc.snapshot().unwrap();
        let (r, x, y, plots) = chart_ids(&g);
        let p0 = reference(field(&g, plots, "0"));
        let p1 = reference(field(&g, plots, "1"));
        let d0 = reference(field(&g, p0, "data"));
        let d1 = reference(field(&g, p1, "data"));
        match name {
            "horizontal" => set_text(&mut g, r, "orientation", "horizontal"),
            "reverse-plots" => {
                set(&mut g, plots, "0", Atom::Ref(p1));
                set(&mut g, plots, "1", Atom::Ref(p0));
            }
            "reverse-categories" => {
                let cats = reference(field(&g, x, "categories"));
                let a = field(&g, cats, "0").clone();
                let b = field(&g, cats, "2").clone();
                set(&mut g, cats, "0", b);
                set(&mut g, cats, "2", a);
            }
            "alias-axes" => {
                let a = object(&mut g);
                set_text(&mut g, a, "type", "linear");
                set_text(&mut g, a, "title", "Shared");
                set(&mut g, a, "min", Atom::Number(-10.));
                set(&mut g, a, "max", Atom::Number(50.));
                set(&mut g, r, "xAxis", Atom::Ref(a));
                set(&mut g, r, "yAxis", Atom::Ref(a));
            }
            "alias-plots" => {
                set(&mut g, plots, "1", Atom::Ref(p0));
                let point = reference(field(&g, d0, "0"));
                set(&mut g, point, "1", Atom::Number(70.));
            }
            "alias-data" => {
                set(&mut g, p1, "data", Atom::Ref(d0));
                let point = reference(field(&g, d0, "0"));
                set(&mut g, point, "1", Atom::Number(-7.));
            }
            "auto-range" => set(&mut g, r, "hasSetYAxis", Atom::Bool(false)),
            "point-labels" => {
                let mut items = vec![];
                for s in ["NEW", "中🙂", "last"] {
                    items.push(string(&mut g, s));
                }
                let a = array(&mut g, items);
                set(&mut g, p1, "pointLabels", Atom::Ref(a));
            }
            "missing-value" => {
                let p = reference(field(&g, d0, "0"));
                remove(&mut g, p, "1");
            }
            "unused-surrogate" => {
                let p = reference(field(&g, d0, "0"));
                let id = g.strings.len() as u32;
                g.strings.push(JsString(vec![0xd800]));
                set(&mut g, p, "0", Atom::String(id));
            }
            "nonfinite" => {
                for (d, values) in [
                    (d0, [f64::NAN, f64::INFINITY, f64::NEG_INFINITY]),
                    (d1, [-0., f64::from_bits(1), 1e308]),
                ] {
                    for (i, n) in values.into_iter().enumerate() {
                        let p = reference(field(&g, d, &i.to_string()));
                        set(&mut g, p, "1", Atom::Number(n));
                    }
                }
                set(&mut g, r, "hasSetYAxis", Atom::Bool(false));
            }
            _ => {}
        }
        let before = fingerprint(&g);
        doc.apply(0, g).unwrap();
        let mut options = RenderOptions::default();
        if name != "original" {
            options.render.max_width = Some(320.)
        }
        let a = doc.render(&options, &mut MetricsResources).unwrap();
        assert_eq!(hash(&a.rendered.svg), expected, "{name}");
        let b = doc.render(&options, &mut MetricsResources).unwrap();
        assert_eq!(a, b);
        assert_eq!(
            fingerprint(&doc.snapshot().unwrap()),
            before,
            "{name} must preserve graph"
        );
        let _ = y;
    }
}
#[test]
fn xychart_all_native_builders_preserve_axis_identity_return_values_and_pinned_render() {
    let mut engine = PublicEngine::empty();
    engine.register(XyChart);
    let mut doc = engine.parse(SOURCE, &Default::default()).unwrap();
    success(call(&mut doc, "setOrientation", |g| {
        vec![string(g, "horizontal")]
    }));
    let old_x = reference(doc.snapshot().unwrap().field("xAxis").unwrap().unwrap());
    let g = success(call(&mut doc, "setXAxisRangeData", |_| {
        vec![Atom::Number(-5.), Atom::Number(5.)]
    }));
    assert_ne!(reference(g.field("xAxis").unwrap().unwrap()), old_x);
    assert_eq!(keys(&g, old_x), ["type", "title", "categories"]);
    let g = success(call(&mut doc, "setXAxisBand", |g| {
        let a = xy_text(g, " C ");
        let b = xy_text(g, " D ");
        vec![Atom::Ref(array(g, vec![a, b]))]
    }));
    let x = reference(g.field("xAxis").unwrap().unwrap());
    let g = success(call(&mut doc, "setXAxisTitle", |g| {
        vec![xy_text(g, " Names ")]
    }));
    assert_eq!(reference(g.field("xAxis").unwrap().unwrap()), x);
    assert_eq!(graph_string(&g, field(&g, x, "title")), "Names");
    success(call(&mut doc, "setYAxisTitle", |g| {
        vec![xy_text(g, " Amount ")]
    }));
    success(call(&mut doc, "setYAxisRangeData", |_| {
        vec![Atom::Number(-20.), Atom::Number(80.)]
    }));
    success(call(&mut doc, "setDiagramTitle", |g| {
        vec![string(g, "Built")]
    }));
    success(call(&mut doc, "setAccTitle", |g| {
        vec![string(g, "  Native")]
    }));
    success(call(&mut doc, "setAccDescription", |g| {
        vec![string(g, "one\n  two")]
    }));
    let plots = reference(doc.snapshot().unwrap().field("plots").unwrap().unwrap());
    let out = call(&mut doc, "plotData", |g| {
        vec![Atom::Ref(array(
            g,
            vec![Atom::Number(1.), Atom::Number(2.)],
        ))]
    });
    let returned = reference(out.result.as_ref().unwrap());
    let p0 = reference(field(&out.graph, returned, "0"));
    assert_eq!(graph_string(&out.graph, field(&out.graph, p0, "0")), "C");
    assert!(matches!(field(&out.graph, p0, "1"), Atom::Number(1.)));
    assert_eq!(reference(out.graph.field("plots").unwrap().unwrap()), plots);
    success(call(&mut doc, "addPlot", |g| {
        vec![
            string(g, "line"),
            xy_text(g, " Extra "),
            points(g, &[(5., "extra"), (6., "")]),
        ]
    }));
    let mut g = doc.snapshot().unwrap();
    let r = root(&g);
    let empty = array(&mut g, vec![]);
    set(&mut g, r, "plots", Atom::Ref(empty));
    doc.apply(doc.revision(), g).unwrap();
    success(call(&mut doc, "setBarData", |g| {
        vec![xy_text(g, " Bars "), points(g, &[(20., ""), (40., "")])]
    }));
    success(call(&mut doc, "setLineData", |g| {
        vec![
            xy_text(g, " Points "),
            points(g, &[(30., " first "), (50., " second ")]),
        ]
    }));
    let rendered = doc
        .render(&Default::default(), &mut MetricsResources)
        .unwrap();
    assert_eq!(
        hash(&rendered.rendered.svg),
        "0eb07e164e830025faa5d330ad47e81354a015f4c5ecbe3d42d898855098eb03"
    );
    let g = doc.snapshot().unwrap();
    assert_eq!(
        graph_string(&g, g.field("accDescr").unwrap().unwrap()),
        "one\ntwo"
    );
    // Borrowed receivers have normal writable object behavior, not a fabricated brand check.
    let mut g = g;
    let borrowed = object(&mut g);
    let value = string(&mut g, "Borrowed");
    doc.apply(doc.revision(), g).unwrap();
    let out = doc
        .invoke(
            doc.revision(),
            MethodCall {
                method: MethodId(11),
                receiver: Atom::Ref(borrowed),
                arguments: vec![value],
            },
            &mut MetricsResources,
        )
        .unwrap();
    assert!(out.result.is_ok());
    assert_eq!(
        graph_string(&out.graph, field(&out.graph, borrowed, "title")),
        "Borrowed"
    );
    assert_eq!(
        graph_string(&out.graph, out.graph.field("title").unwrap().unwrap()),
        "Built"
    );
}
struct FailingHost;
impl RenderResources for FailingHost {
    fn text_width(&mut self, _: TextRequest<'_>) -> Result<f64, PeleError> {
        Err(PeleError::new("XYChart host stopped", ErrorCode::Semantic))
    }
}
#[test]
fn xychart_shared_long_label_expansion_is_bounded_before_native_copies() {
    let mut engine = PublicEngine::empty();
    engine.register(XyChart);
    let mut doc = engine.parse(SOURCE, &Default::default()).unwrap();
    let mut g = doc.snapshot().unwrap();
    let (_, _, _, plots) = chart_ids(&g);
    let p = reference(field(&g, plots, "1"));
    let label = string(&mut g, &"x".repeat(1_000_000));
    let labels = array(&mut g, vec![label.clone(); 17]);
    set(&mut g, p, "pointLabels", Atom::Ref(labels));
    doc.apply(0, g).unwrap();
    assert_eq!(
        doc.render(&Default::default(), &mut MetricsResources)
            .unwrap_err()
            .code,
        ErrorCode::Limit
    );
    let mut native = engine.parse(SOURCE, &Default::default()).unwrap();
    let out = call(&mut native, "setLineData", |g| {
        let label = string(g, &"x".repeat(1_000_000));
        let mut records = vec![];
        for i in 0..17 {
            let p = object(g);
            set(g, p, "value", Atom::Number(i as f64));
            set(g, p, "label", label.clone());
            records.push(Atom::Ref(p));
        }
        vec![xy_text(g, "Bounded"), Atom::Ref(array(g, records))]
    });
    assert!(matches!(
        out.result,
        Err(PublicError::Pele(PeleError {
            code: ErrorCode::Limit,
            ..
        }))
    ));
    assert!(native
        .render(&Default::default(), &mut MetricsResources)
        .is_ok());
}
#[test]
fn xychart_auto_range_builders_and_lossless_axis_setters_match_public_trace() {
    // Pinned public trace: plotData([]) leaves axes/plots unchanged; then
    // [2,9,4] returns [["1",2],["2",9],["3",4]], axes x1..3/y2..9;
    // [1,4] returns [["1",1],["3",4]], same x object, new y1..9.
    let mut engine = PublicEngine::empty();
    engine.register(XyChart);
    let mut doc = engine.parse("xychart-beta", &Default::default()).unwrap();
    let before = doc.snapshot().unwrap();
    let (_, x, y, plots) = chart_ids(&before);
    let out = call(&mut doc, "plotData", |g| vec![Atom::Ref(array(g, vec![]))]);
    assert!(out.result.is_ok());
    assert_eq!(reference(out.graph.field("xAxis").unwrap().unwrap()), x);
    assert_eq!(reference(out.graph.field("yAxis").unwrap().unwrap()), y);
    let out = call(&mut doc, "plotData", |g| {
        vec![Atom::Ref(array(
            g,
            vec![Atom::Number(2.), Atom::Number(9.), Atom::Number(4.)],
        ))]
    });
    let returned = reference(out.result.as_ref().unwrap());
    for (i, (key, n)) in [("1", 2.), ("2", 9.), ("3", 4.)].into_iter().enumerate() {
        let p = reference(field(&out.graph, returned, &i.to_string()));
        assert_eq!(graph_string(&out.graph, field(&out.graph, p, "0")), key);
        assert!(matches!(field(&out.graph,p,"1"),Atom::Number(v)if *v==n));
    }
    let (_, x, y, _) = chart_ids(&out.graph);
    assert!(matches!(field(&out.graph, y, "min"), Atom::Number(2.)));
    assert!(matches!(field(&out.graph, y, "max"), Atom::Number(9.)));
    let out = call(&mut doc, "plotData", |g| {
        vec![Atom::Ref(array(
            g,
            vec![Atom::Number(1.), Atom::Number(4.)],
        ))]
    });
    let (_, x2, y2, plots2) = chart_ids(&out.graph);
    assert_eq!(x, x2);
    assert_ne!(y, y2);
    assert_eq!(plots, plots2);
    assert!(matches!(field(&out.graph, y2, "min"), Atom::Number(1.)));
    assert!(matches!(field(&out.graph, y2, "max"), Atom::Number(9.)));
    let g = success(call(&mut doc, "setXAxisRangeData", |_| vec![]));
    let x = reference(g.field("xAxis").unwrap().unwrap());
    assert!(matches!(field(&g, x, "min"), Atom::Undefined));
    assert!(matches!(field(&g, x, "max"), Atom::Undefined));
    let g = success(call(&mut doc, "setXAxisTitle", |g| {
        let t = object(g);
        let id = g.strings.len() as u32;
        g.strings.push(JsString(vec![32, 0xd800, 32]));
        set(g, t, "text", Atom::String(id));
        vec![Atom::Ref(t)]
    }));
    assert_eq!(g.text(field(&g, x, "title")).unwrap().0, vec![0xd800]);
    let g = success(call(&mut doc, "setXAxisBand", |g| {
        let t = object(g);
        let id = g.strings.len() as u32;
        g.strings.push(JsString(vec![32, 0xdc00, 32]));
        set(g, t, "text", Atom::String(id));
        vec![Atom::Ref(array(g, vec![Atom::Ref(t)]))]
    }));
    let x = reference(g.field("xAxis").unwrap().unwrap());
    let cats = reference(field(&g, x, "categories"));
    assert_eq!(g.text(field(&g, cats, "0")).unwrap().0, vec![0xdc00]);
    // Explicit own aliases of the original native functions still dispatch.
    let mut aliases = engine.parse("xychart-beta", &Default::default()).unwrap();
    let mut g = aliases.snapshot().unwrap();
    let r = root(&g);
    for (k, id) in [("setXAxisRangeData", 3), ("plotData", 7), ("addPlot", 8)] {
        set(&mut g, r, k, Atom::BuiltinFunction(id));
    }
    aliases.apply(0, g).unwrap();
    let out = call(&mut aliases, "setLineData", |g| {
        vec![xy_text(g, "Native"), points(g, &[(2., ""), (4., "")])]
    });
    assert!(out.result.is_ok());
    assert!(aliases
        .render(&Default::default(), &mut MetricsResources)
        .is_ok());
}
#[test]
fn xychart_partial_method_errors_deep_transport_and_render_recovery_are_owned() {
    let mut engine = PublicEngine::empty();
    engine.register(XyChart);
    let mut partial = engine.parse("xychart-beta", &Default::default()).unwrap();
    let mut g = partial.snapshot().unwrap();
    let r = root(&g);
    let bad_plots = object(&mut g);
    set(&mut g, r, "plots", Atom::Ref(bad_plots));
    partial.apply(0, g).unwrap();
    let out = call(&mut partial, "setLineData", |g| {
        vec![xy_text(g, "P"), points(g, &[(2., "a"), (4., "b")])]
    });
    assert!(matches!(
        out.result,
        Err(PublicError::JavaScript {
            name: JavaScriptErrorKind::TypeError,
            ..
        })
    ));
    assert_eq!(
        reference(out.graph.field("plots").unwrap().unwrap()),
        bad_plots
    );
    assert!(matches!(
        out.graph.field("hasSetXAxis").unwrap(),
        Some(Atom::Bool(true))
    ));
    let y = reference(out.graph.field("yAxis").unwrap().unwrap());
    assert!(matches!(field(&out.graph, y, "min"), Atom::Number(2.)));
    assert!(matches!(field(&out.graph, y, "max"), Atom::Number(4.)));
    let mut frozen = engine.parse("xychart-beta", &Default::default()).unwrap();
    let mut g = frozen.snapshot().unwrap();
    let r = root(&g);
    let old_x = reference(field(&g, r, "xAxis"));
    let key = g.nodes[r as usize]
        .properties()
        .iter()
        .position(|p| g.strings[p.key as usize].is("hasSetXAxis"))
        .unwrap();
    g.nodes[r as usize].properties_mut()[key].flags = 1;
    frozen.apply(0, g).unwrap();
    let out = call(&mut frozen, "setXAxisRangeData", |_| {
        vec![Atom::Number(-1.), Atom::Number(1.)]
    });
    assert!(matches!(
        out.result,
        Err(PublicError::JavaScript {
            name: JavaScriptErrorKind::TypeError,
            ..
        })
    ));
    assert_ne!(reference(out.graph.field("xAxis").unwrap().unwrap()), old_x);
    assert!(matches!(
        out.graph.field("hasSetXAxis").unwrap(),
        Some(Atom::Bool(false))
    ));
    let mut doc = engine.parse(SOURCE, &Default::default()).unwrap();
    let expected = doc
        .render(&Default::default(), &mut MetricsResources)
        .unwrap();
    let mut g = doc.snapshot().unwrap();
    let r = root(&g);
    let mut at = r;
    for _ in 0..20_000 {
        let next = object(&mut g);
        set(&mut g, at, "extra", Atom::Ref(next));
        at = next;
    }
    set(&mut g, at, "cycle", Atom::Ref(r));
    let id = g.strings.len() as u32;
    g.strings.push(JsString(vec![0xd800, 97, 0xdc00]));
    set(&mut g, at, "utf16", Atom::String(id));
    let nan = f64::from_bits(0x7ff8_0000_0000_0042);
    set(&mut g, at, "number", Atom::Number(nan));
    let g: ModelGraph = serde_json::from_slice(&serde_json::to_vec(&g).unwrap()).unwrap();
    doc.apply(0, g).unwrap();
    let before = fingerprint(&doc.snapshot().unwrap());
    assert_eq!(
        doc.render(&Default::default(), &mut FailingHost)
            .unwrap_err()
            .message,
        "XYChart host stopped"
    );
    assert_eq!(before, fingerprint(&doc.snapshot().unwrap()));
    let mut options = RenderOptions::default();
    options.render.output_limit = Some(1);
    assert_eq!(
        doc.render(&options, &mut MetricsResources)
            .unwrap_err()
            .code,
        ErrorCode::Limit
    );
    assert_eq!(before, fingerprint(&doc.snapshot().unwrap()));
    assert_eq!(
        doc.render(&Default::default(), &mut MetricsResources)
            .unwrap(),
        expected
    );
    let mut g = doc.snapshot().unwrap();
    let (_, _, _, plots) = chart_ids(&g);
    let Node::Array { length, .. } = &mut g.nodes[plots as usize] else {
        panic!()
    };
    *length = u32::MAX;
    doc.apply(1, g).unwrap();
    assert_eq!(
        doc.render(&Default::default(), &mut MetricsResources)
            .unwrap_err()
            .code,
        ErrorCode::Limit
    );
    // Used lone-surrogate text remains exact in the graph, with an explicit renderer gap.
    let out = call(&mut partial, "setDiagramTitle", |g| {
        let i = g.strings.len() as u32;
        g.strings.push(JsString(vec![0xd800]));
        vec![Atom::String(i)]
    });
    assert!(out.result.is_ok());
    assert_eq!(
        out.graph
            .text(out.graph.field("title").unwrap().unwrap())
            .unwrap()
            .0,
        vec![0xd800]
    );
}
