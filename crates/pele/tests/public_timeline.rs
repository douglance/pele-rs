//! Native mutable Timeline feature, Pele 1.6.0 / 458322a5.
//! Probe main entry node_modules/pele/dist/pele.js SHA256
//! d1ebc1889f3069d7759fe5db34c4ad4b4823285edbee2b537d0ef3d505a76d8d;
//! timeline/db.ts fca752daa5dd10855880ebff238c8381aecd99dffa1730cae71628a4a3377f16;
//! timeline/render.ts 36d032ff1828bfc128baa1a7926d58fbb8224a09927ef5ca8cacc02c2c9ede6e.
use pele::compat::{
    diagrams::timeline::{Timeline, TimelineModel},
    public::*,
    text::MetricsResources,
    *,
};
use serde_json::Value as Json;
use sha2::{Digest, Sha256};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
const SOURCE:&str="timeline\ntitle Before\naccTitle: Access\naccDescr: Details\nEarly : Arrival\nsection Plan\n2024 : Draft : Review\nsection Build\n2025 : Ship";
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
fn string(g: &mut ModelGraph, s: &str) -> Atom {
    let i = g.strings.len() as u32;
    g.strings.push(s.into());
    Atom::String(i)
}
fn set_text(g: &mut ModelGraph, id: u32, key: &str, s: &str) {
    let a = string(g, s);
    set(g, id, key, a)
}
fn object(g: &mut ModelGraph) -> u32 {
    let i = g.nodes.len() as u32;
    g.nodes.push(Node::Object { properties: vec![] });
    i
}
fn array(g: &mut ModelGraph, values: Vec<Atom>) -> u32 {
    let i = g.nodes.len() as u32;
    g.nodes.push(Node::Array {
        length: values.len() as u32,
        properties: vec![],
    });
    for (n, a) in values.into_iter().enumerate() {
        set(g, i, &n.to_string(), a)
    }
    i
}
fn read_text(g: &ModelGraph, a: &Atom) -> String {
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
fn ids(g: &ModelGraph) -> (u32, u32, u32) {
    let r = root(g);
    (
        r,
        reference(field(g, r, "sections")),
        reference(field(g, r, "periods")),
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
fn success(o: InvocationOutcome) -> ModelGraph {
    assert!(matches!(o.result, Ok(Atom::Undefined)));
    o.graph
}
struct Counted {
    parses: Arc<AtomicUsize>,
    renders: Arc<AtomicUsize>,
}
impl DiagramModule for Counted {
    type Model = TimelineModel;
    fn descriptor(&self) -> ModuleDescriptor {
        Timeline.descriptor()
    }
    fn parse(&self, c: ParseContext<'_>) -> Result<Self::Model, PeleError> {
        self.parses.fetch_add(1, Ordering::SeqCst);
        Timeline.parse(c)
    }
    fn render(
        &self,
        m: &Self::Model,
        c: &Object,
        o: &RenderSettings,
        h: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError> {
        Timeline.render(m, c, o, h)
    }
}
impl PublicDiagramModule for Counted {
    type PublicState = <Timeline as PublicDiagramModule>::PublicState;
    fn model_schema(&self) -> &'static ModelSchema {
        Timeline.model_schema()
    }
    fn create_public(&self, m: &Self::Model) -> Result<Self::PublicState, PeleError> {
        Timeline.create_public(m)
    }
    fn export_public(&self, s: &Self::PublicState) -> Result<ModelGraph, PeleError> {
        Timeline.export_public(s)
    }
    fn apply_public(&self, s: &mut Self::PublicState, g: ModelGraph) -> Result<(), PeleError> {
        Timeline.apply_public(s, g)
    }
    fn invoke_public(
        &self,
        s: &mut Self::PublicState,
        call: MethodCall,
        c: &Object,
        h: &mut dyn RenderResources,
    ) -> Result<Atom, PublicError> {
        Timeline.invoke_public(s, call, c, h)
    }
    fn render_public(
        &self,
        s: &mut Self::PublicState,
        c: &Object,
        o: &RenderSettings,
        h: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError> {
        self.renders.fetch_add(1, Ordering::SeqCst);
        Timeline.render_public(s, c, o, h)
    }
}
#[test]
fn timeline_instance_fields_receiver_identity_and_owned_registration() {
    let parses = Arc::new(AtomicUsize::new(0));
    let renders = Arc::new(AtomicUsize::new(0));
    let mut engine = PublicEngine::empty();
    engine.register(Counted {
        parses: parses.clone(),
        renders: renders.clone(),
    });
    let empty = engine
        .parse("timeline", &Default::default())
        .unwrap()
        .snapshot()
        .unwrap();
    for key in ["title", "accTitle", "accDescr"] {
        assert!(matches!(empty.field(key).unwrap(), Some(Atom::Undefined)))
    }
    let mut doc = engine.parse(SOURCE, &Default::default()).unwrap();
    let g = doc.snapshot().unwrap();
    let (r, sections, periods) = ids(&g);
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
            "direction",
            "sections",
            "periods",
            "currentSection"
        ]
    );
    assert!(g.nodes[r as usize]
        .properties()
        .iter()
        .all(|p| p.flags == 7));
    let p = reference(field(&g, periods, "0"));
    assert_eq!(keys(&g, p), ["text", "events", "section", "sectionIndex"]);
    assert!(g.nodes[p as usize]
        .properties()
        .iter()
        .all(|p| p.flags == 7));
    assert_eq!(
        read_text(&g, g.field("currentSection").unwrap().unwrap()),
        "Build"
    );
    assert_eq!(read_text(&g, field(&g, sections, "0")), "Plan");
    assert_eq!(doc.model_schema().id, "pele/timeline@1");
    assert_eq!(
        doc.model_schema()
            .methods
            .iter()
            .map(|m| (m.name, m.arity))
            .collect::<Vec<_>>(),
        [
            ("getCommonDb", 0),
            ("setDiagramTitle", 1),
            ("setAccTitle", 1),
            ("setAccDescription", 1),
            ("setDirection", 1),
            ("addSection", 1),
            ("addTask", 3),
            ("addEvent", 1)
        ]
    );
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
    let out = call(&mut doc, "getCommonDb", |_| vec![]);
    assert!(matches!(out.result,Ok(Atom::Ref(id))if id==r));
    let mut g = doc.snapshot().unwrap();
    set_text(&mut g, r, "type", "changed-type");
    set_text(&mut g, r, "title", "Edited");
    set_text(&mut g, r, "direction", "TD");
    doc.apply(doc.revision(), g).unwrap();
    let replacement = Arc::new(AtomicUsize::new(0));
    engine.register(Counted {
        parses: parses.clone(),
        renders: replacement.clone(),
    });
    drop(engine);
    let expected = SOURCE
        .replace("timeline\n", "timeline TD\n")
        .replace("title Before", "title Edited");
    let mut options = RenderOptions::default();
    options.render.max_width = Some(320.);
    assert_eq!(
        doc.render(&options, &mut MetricsResources).unwrap(),
        Engine::with_builtins()
            .render_source(&expected, &options, &mut MetricsResources)
            .unwrap()
    );
    assert_eq!(parses.load(Ordering::SeqCst), 2);
    assert_eq!(renders.load(Ordering::SeqCst), 2);
    assert_eq!(replacement.load(Ordering::SeqCst), 0);
    // Strict class getter returns any borrowed receiver unchanged, even primitives.
    let nan = f64::from_bits(0x7ff8_0000_0000_0042);
    let out = doc
        .invoke(
            doc.revision(),
            MethodCall {
                method: MethodId(1),
                receiver: Atom::Number(nan),
                arguments: vec![],
            },
            &mut MetricsResources,
        )
        .unwrap();
    assert!(matches!(out.result,Ok(Atom::Number(n))if n.to_bits()==nan.to_bits()));
}
#[test]
fn timeline_aliases_replacements_group_indexes_and_unused_utf16_match_pinned_svg() {
    let probes = [
        (
            "original",
            "19024ad2bd647f43b798b0b9ad90b7554025a4f5072e48fad2574a83ec6547b7",
        ),
        (
            "vertical",
            "7f72f803bf1962e9e0e7160c739e5c8727abfc1453acf29f5ac01a4cc76a23af",
        ),
        (
            "reverse-periods",
            "7f72f803bf1962e9e0e7160c739e5c8727abfc1453acf29f5ac01a4cc76a23af",
        ),
        (
            "reverse-events",
            "1ccd04381c6c976c9b5987391121ddc0f706db3053092d83ec1b73ebbf70b80d",
        ),
        (
            "reverse-sections",
            "2b5ff1dca1a80b5ba6f7ce7d755a2f930897b484df6e531a3bb01810fca989c6",
        ),
        (
            "alias-periods",
            "5ce023940a743d76cfe9f0abfecf05792e180b5e9fc47050758d8b265775c8b9",
        ),
        (
            "alias-events",
            "2e7c2af2b2019c3dbf08fcf90d6b8069611ca011c0b0a36180c1cc7b441201b8",
        ),
        (
            "alias-sections-events",
            "e0a770c5380ee993e8d25f47dd32e2b0f0ceb87f6f401dd726f60b1dbae80d4e",
        ),
        (
            "fractional-index",
            "b9224c606d1e09a3ca059c28f29dd129e7bb7ddfb07c5bbb089d4a1741d656b7",
        ),
        (
            "nan-index",
            "b9224c606d1e09a3ca059c28f29dd129e7bb7ddfb07c5bbb089d4a1741d656b7",
        ),
        (
            "infinity-index",
            "b9224c606d1e09a3ca059c28f29dd129e7bb7ddfb07c5bbb089d4a1741d656b7",
        ),
        (
            "negative-infinity-index",
            "b9224c606d1e09a3ca059c28f29dd129e7bb7ddfb07c5bbb089d4a1741d656b7",
        ),
        (
            "negative-zero-index",
            "7f72f803bf1962e9e0e7160c739e5c8727abfc1453acf29f5ac01a4cc76a23af",
        ),
        (
            "unused-surrogate",
            "7f72f803bf1962e9e0e7160c739e5c8727abfc1453acf29f5ac01a4cc76a23af",
        ),
        (
            "undefined-event",
            "ca26ae001f39eec42df4d4698bc299afb8469d3fc9c84b12d4ac301696464fad",
        ),
    ];
    let mut engine = PublicEngine::empty();
    engine.register(Timeline);
    for (name, expected) in probes {
        let mut doc = engine.parse(SOURCE, &Default::default()).unwrap();
        let mut g = doc.snapshot().unwrap();
        let (r, sections, periods) = ids(&g);
        let p0 = reference(field(&g, periods, "0"));
        let p1 = reference(field(&g, periods, "1"));
        let p2 = reference(field(&g, periods, "2"));
        let e1 = reference(field(&g, p1, "events"));
        match name {
            "vertical" => set_text(&mut g, r, "direction", "TD"),
            "reverse-periods" => {
                set(&mut g, periods, "0", Atom::Ref(p2));
                set(&mut g, periods, "2", Atom::Ref(p0));
            }
            "reverse-events" => {
                let a = field(&g, e1, "0").clone();
                let b = field(&g, e1, "1").clone();
                set(&mut g, e1, "0", b);
                set(&mut g, e1, "1", a);
            }
            "reverse-sections" => {
                let a = field(&g, sections, "0").clone();
                let b = field(&g, sections, "1").clone();
                set(&mut g, sections, "0", b);
                set(&mut g, sections, "1", a);
            }
            "alias-periods" => {
                let a = array(&mut g, vec![Atom::Ref(p1), Atom::Ref(p1)]);
                set(&mut g, r, "periods", Atom::Ref(a));
            }
            "alias-events" => {
                set(&mut g, p2, "events", Atom::Ref(e1));
                set_text(&mut g, e1, "0", "Shared");
            }
            "alias-sections-events" => set(&mut g, r, "sections", Atom::Ref(e1)),
            "fractional-index" => set(&mut g, p1, "sectionIndex", Atom::Number(0.5)),
            "nan-index" => set(&mut g, p1, "sectionIndex", Atom::Number(f64::NAN)),
            "infinity-index" => set(&mut g, p1, "sectionIndex", Atom::Number(f64::INFINITY)),
            "negative-infinity-index" => {
                set(&mut g, p1, "sectionIndex", Atom::Number(f64::NEG_INFINITY))
            }
            "negative-zero-index" => set(&mut g, p1, "sectionIndex", Atom::Number(-0.)),
            "unused-surrogate" => {
                let id = g.strings.len() as u32;
                g.strings.push(JsString(vec![0xd800]));
                set(&mut g, r, "currentSection", Atom::String(id));
                set(&mut g, p1, "section", Atom::String(id));
            }
            "undefined-event" => {
                let e = reference(field(&g, p2, "events"));
                set(&mut g, e, "0", Atom::Undefined);
            }
            _ => {}
        }
        doc.apply(0, g).unwrap();
        if name == "alias-periods" {
            success(call(&mut doc, "addEvent", |g| vec![string(g, "Shared")]));
        }
        if name == "alias-sections-events" {
            success(call(&mut doc, "addSection", |g| vec![string(g, "Extra")]));
        }
        let before = fingerprint(&doc.snapshot().unwrap());
        let mut options = RenderOptions::default();
        if name != "original" {
            options.render.max_width = Some(320.)
        }
        let result = doc.render(&options, &mut MetricsResources).unwrap();
        assert_eq!(hash(&result.rendered.svg), expected, "{name}");
        assert_eq!(doc.render(&options, &mut MetricsResources).unwrap(), result);
        assert_eq!(fingerprint(&doc.snapshot().unwrap()), before);
    }
}
#[test]
fn timeline_native_builders_getter_and_ignored_length_match_complete_render() {
    let mut engine = PublicEngine::empty();
    engine.register(Timeline);
    let mut doc = engine.parse("timeline", &Default::default()).unwrap();
    success(call(&mut doc, "addSection", |g| vec![string(g, " Plan ")]));
    success(call(&mut doc, "addTask", |g| {
        vec![string(g, "2024 "), Atom::Number(99.), string(g, "Draft ")]
    }));
    success(call(&mut doc, "addEvent", |g| vec![string(g, "Review")]));
    success(call(&mut doc, "addSection", |g| vec![string(g, "Build")]));
    success(call(&mut doc, "addTask", |g| {
        vec![string(g, "2025 "), Atom::Undefined, string(g, "Ship")]
    }));
    success(call(&mut doc, "setDirection", |g| vec![string(g, "TD")]));
    success(call(&mut doc, "setDiagramTitle", |g| {
        vec![string(g, "Built")]
    }));
    success(call(&mut doc, "setAccTitle", |g| {
        vec![string(g, "  Native")]
    }));
    success(call(&mut doc, "setAccDescription", |g| {
        vec![string(g, "one\n  two")]
    }));
    let g = doc.snapshot().unwrap();
    let (r, sections, periods) = ids(&g);
    assert_eq!(read_text(&g, field(&g, sections, "0")), " Plan ");
    let p = reference(field(&g, periods, "0"));
    assert_eq!(read_text(&g, field(&g, p, "section")), " Plan ");
    assert!(matches!(field(&g, p, "sectionIndex"), Atom::Number(0.)));
    assert_eq!(read_text(&g, field(&g, r, "accDescr")), "one\ntwo");
    for (width, expected) in [
        (
            None,
            "c3575e56633a91d857f76d711ad22406466341fd87b84e85445aac85b99e4942",
        ),
        (
            Some(320.),
            "a9bdf687128e318b22f124d3097d13343b4ae40b3c23b742beaf559e7d8d5fe6",
        ),
    ] {
        let mut o = RenderOptions::default();
        o.render.max_width = width;
        let result = doc.render(&o, &mut MetricsResources).unwrap();
        assert_eq!(hash(&result.rendered.svg), expected);
        let source="timeline TD\ntitle Built\naccTitle: Native\naccDescr {one\ntwo}\nsection Plan\n2024 : Draft : Review\nsection Build\n2025 : Ship";
        assert_eq!(
            result,
            Engine::with_builtins()
                .render_source(source, &o, &mut MetricsResources)
                .unwrap()
        );
    }
    // Bookkeeping may hold an arbitrary cyclic value; addTask preserves its
    // exact alias while the actual renderer only reads sectionIndex.
    let mut cycle = engine.parse(SOURCE, &Default::default()).unwrap();
    let mut g = cycle.snapshot().unwrap();
    let r = root(&g);
    set(&mut g, r, "currentSection", Atom::Ref(r));
    cycle.apply(0, g).unwrap();
    let g = success(call(&mut cycle, "addTask", |g| {
        vec![string(g, "Cycle"), g.root.clone(), string(g, "")]
    }));
    let periods = reference(g.field("periods").unwrap().unwrap());
    let p = reference(field(&g, periods, "3"));
    assert_eq!(reference(field(&g, p, "section")), r);
    let mut o = RenderOptions::default();
    o.render.max_width = Some(320.);
    assert_eq!(
        hash(
            &cycle
                .render(&o, &mut MetricsResources)
                .unwrap()
                .rendered
                .svg
        ),
        "cc0cb00488c716e7101f4ff43edc53b3f2382c4522dc030c19c2b89659162d55"
    );
}
struct FailingHost;
impl RenderResources for FailingHost {
    fn text_width(&mut self, _: TextRequest<'_>) -> Result<f64, PeleError> {
        Err(PeleError::new("Timeline host stopped", ErrorCode::Semantic))
    }
}
#[test]
fn timeline_partial_errors_lossless_deep_extras_and_resource_recovery() {
    let mut engine = PublicEngine::empty();
    engine.register(Timeline);
    let mut empty = engine.parse("timeline", &Default::default()).unwrap();
    let out = call(&mut empty, "addEvent", |g| vec![string(g, "Orphan")]);
    assert!(
        matches!(out.result,Err(PublicError::Pele(PeleError{code:ErrorCode::Semantic,ref message,..}))if message=="An event needs a time period before it.")
    );
    let mut g = empty.snapshot().unwrap();
    let r = root(&g);
    let invalid = object(&mut g);
    set(&mut g, r, "sections", Atom::Ref(invalid));
    empty.apply(empty.revision(), g).unwrap();
    let out = call(&mut empty, "addSection", |g| vec![string(g, "Partial")]);
    assert!(matches!(
        out.result,
        Err(PublicError::JavaScript {
            name: JavaScriptErrorKind::TypeError,
            ..
        })
    ));
    assert_eq!(
        read_text(
            &out.graph,
            out.graph.field("currentSection").unwrap().unwrap()
        ),
        "Partial"
    );
    assert_eq!(
        reference(out.graph.field("sections").unwrap().unwrap()),
        invalid
    );
    let mut shadow = engine.parse("timeline", &Default::default()).unwrap();
    let mut g = shadow.snapshot().unwrap();
    let (r, sections, _) = ids(&g);
    set(&mut g, sections, "push", Atom::Undefined);
    shadow.apply(0, g).unwrap();
    let out = call(&mut shadow, "addSection", |g| vec![string(g, "Shadowed")]);
    assert!(matches!(
        out.result,
        Err(PublicError::JavaScript {
            name: JavaScriptErrorKind::TypeError,
            ..
        })
    ));
    assert_eq!(
        read_text(&out.graph, field(&out.graph, r, "currentSection")),
        "Shadowed"
    );
    let mut frozen = engine.parse("timeline", &Default::default()).unwrap();
    let mut g = frozen.snapshot().unwrap();
    let (r, sections, _) = ids(&g);
    let i = g.nodes[r as usize]
        .properties()
        .iter()
        .position(|p| g.strings[p.key as usize].is("currentSection"))
        .unwrap();
    g.nodes[r as usize].properties_mut()[i].flags = 1;
    frozen.apply(0, g).unwrap();
    let out = call(&mut frozen, "addSection", |g| vec![string(g, "Blocked")]);
    assert!(matches!(
        out.result,
        Err(PublicError::JavaScript {
            name: JavaScriptErrorKind::TypeError,
            ..
        })
    ));
    assert!(matches!(
        out.graph.nodes[sections as usize],
        Node::Array { length: 0, .. }
    ));
    let mut doc = engine.parse(SOURCE, &Default::default()).unwrap();
    let expected = doc
        .render(&Default::default(), &mut MetricsResources)
        .unwrap();
    let mut g = doc.snapshot().unwrap();
    let r = root(&g);
    let mut at = r;
    for _ in 0..20_000 {
        let n = object(&mut g);
        set(&mut g, at, "extra", Atom::Ref(n));
        at = n;
    }
    set(&mut g, at, "cycle", Atom::Ref(r));
    let id = g.strings.len() as u32;
    g.strings.push(JsString(vec![0xd800, 97, 0xdc00]));
    set(&mut g, at, "utf16", Atom::String(id));
    let nan = f64::from_bits(0x7ff8_0000_0000_0042);
    set(&mut g, at, "nan", Atom::Number(nan));
    set(&mut g, at, "negativeZero", Atom::Number(-0.));
    let g: ModelGraph = serde_json::from_slice(&serde_json::to_vec(&g).unwrap()).unwrap();
    doc.apply(0, g).unwrap();
    let before = fingerprint(&doc.snapshot().unwrap());
    assert_eq!(
        doc.render(&Default::default(), &mut FailingHost)
            .unwrap_err()
            .message,
        "Timeline host stopped"
    );
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
    let out = call(&mut doc, "setAccTitle", |g| {
        let id = g.strings.len() as u32;
        g.strings.push(JsString(vec![32, 0xd800]));
        vec![Atom::String(id)]
    });
    let g = success(out);
    assert_eq!(
        g.text(g.field("accTitle").unwrap().unwrap()).unwrap().0,
        vec![0xd800]
    );
    assert!(doc
        .render(&Default::default(), &mut MetricsResources)
        .unwrap_err()
        .message
        .contains("lone UTF-16"));
    let mut oversized = engine.parse(SOURCE, &Default::default()).unwrap();
    let mut g = oversized.snapshot().unwrap();
    let (_, _, periods) = ids(&g);
    let p = reference(field(&g, periods, "0"));
    let label = string(&mut g, &"x".repeat(1_000_000));
    let events = array(&mut g, vec![label; 17]);
    set(&mut g, p, "events", Atom::Ref(events));
    oversized.apply(0, g).unwrap();
    assert_eq!(
        oversized
            .render(&Default::default(), &mut MetricsResources)
            .unwrap_err()
            .code,
        ErrorCode::Limit
    );
}
struct Host<'a>(&'a Json);
impl RenderResources for Host<'_> {
    fn text_width(&mut self, r: TextRequest<'_>) -> Result<f64, PeleError> {
        if self.0["measure"] == "fixed" {
            Ok(r.text.encode_utf16().count() as f64
                * r.size
                * if r.style & 1 != 0 { 0.71 } else { 0.6 })
        } else {
            MetricsResources.text_width(r)
        }
    }
    fn icon(&mut self, name: &str) -> Result<Option<TrustedSvg>, PeleError> {
        Ok(self.0["icons"][name]
            .as_str()
            .map(|s| TrustedSvg::from_trusted_markup(s.into())))
    }
}
#[test]
fn historical_timeline_corpus_matches_through_mutable_public_documents() {
    let fixture: Json =
        serde_json::from_str(include_str!("../../../fixtures/pele/timeline.json")).unwrap();
    let mut engine = PublicEngine::empty();
    engine.register(Timeline);
    let mut failures = vec![];
    for case in fixture["cases"].as_array().unwrap() {
        let options: RenderOptions = serde_json::from_value(case["options"].clone()).unwrap();
        let actual = engine
            .parse(case["source"].as_str().unwrap(), &options.parse)
            .and_then(|mut d| d.render(&options, &mut Host(&case["host"])));
        let actual = match actual {
            Ok(r) => {
                let mut result = serde_json::to_value(r).unwrap();
                if case["expected"]["result"].get("svgSha256").is_some() {
                    let svg = result.as_object_mut().unwrap().remove("svg").unwrap();
                    result["svgSha256"] = hash(svg.as_str().unwrap()).into();
                }
                serde_json::json!({"result":result})
            }
            Err(e) => serde_json::json!({"error":e}),
        };
        let expected = if case["expected"].get("error").is_some() {
            serde_json::json!({"error":case["expected"]["error"]})
        } else {
            serde_json::json!({"result":case["expected"]["result"]})
        };
        if actual != expected {
            failures.push((case["id"].clone(), actual, expected));
        }
    }
    assert!(
        failures.is_empty(),
        "{} mutable corpus mismatches: {:?}",
        failures.len(),
        failures.iter().take(5).collect::<Vec<_>>()
    );
}
