//! Actual public Class callbacks: retained identities, nested originals, and late errors.
use pele::compat::{public::*, *};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
const SOURCE: &str =
    "classDiagram\nclass A {\n +value$\n +run(x) int*\n}\nclass B {\n +other\n}\nA --> B";
fn field(g: &ModelGraph, a: &Atom, k: &str) -> Atom {
    let Atom::Ref(id) = a else { panic!() };
    g.nodes[*id as usize]
        .properties()
        .iter()
        .find(|p| g.strings[p.key as usize].is(k))
        .unwrap()
        .value
        .clone()
}
fn set(g: &mut ModelGraph, a: &Atom, k: &str, v: Atom) {
    let Atom::Ref(id) = a else { panic!() };
    let key = g.strings.len() as u32;
    g.strings.push(k.into());
    g.nodes[*id as usize].properties_mut().push(Property {
        key,
        value: v,
        flags: 7,
    });
}
fn first(g: &ModelGraph) -> Atom {
    let classes = field(g, &g.root, "classes");
    let Atom::Ref(id) = classes else { panic!() };
    let Node::Map { entries, .. } = &g.nodes[id as usize] else {
        panic!()
    };
    let members = field(g, &entries[0].1, "members");
    field(g, &members, "0")
}
struct Function {
    which: u32,
    mode: Rc<Cell<u8>>,
    trace: Rc<RefCell<Vec<String>>>,
}
impl HostFunction for Function {
    fn call(
        &self,
        m: &mut dyn ActiveModel,
        r: Atom,
        args: &[Atom],
        h: &mut dyn RenderResources,
    ) -> Result<Atom, PublicError> {
        assert!(args.is_empty());
        self.trace.borrow_mut().push(format!("call:{}", self.which));
        if self.which == 1 {
            let display = m.invoke_builtin(
                MethodCall {
                    method: MethodId(26),
                    receiver: r.clone(),
                    arguments: vec![],
                },
                h,
            )?;
            let root = m.root();
            m.set(&root, "savedMember", r.clone())?;
            m.set(&root, "savedDisplay", display.clone())?;
            let text = m.intern("Live label".into())?;
            m.set(&display, "displayText", text)?;
            if self.mode.get() == 1 {
                return Err(PublicError::HostException(HostExceptionToken(91)));
            }
            if self.mode.get() == 2 {
                return Ok(Atom::Ref(u32::MAX));
            }
            Ok(display)
        } else {
            let star = m.intern("*".into())?;
            m.set(&r, "classifier", star)?;
            m.invoke_builtin(
                MethodCall {
                    method: MethodId(27),
                    receiver: r,
                    arguments: vec![],
                },
                h,
            )
        }
    }
}
struct Host {
    mode: Rc<Cell<u8>>,
    trace: Rc<RefCell<Vec<String>>>,
}
impl RenderResources for Host {
    fn host_function(&mut self, id: u32) -> Result<Rc<dyn HostFunction>, PublicError> {
        Ok(Rc::new(Function {
            which: id,
            mode: self.mode.clone(),
            trace: self.trace.clone(),
        }))
    }
    fn text_width(&mut self, r: TextRequest<'_>) -> Result<f64, PeleError> {
        self.trace
            .borrow_mut()
            .push(format!("width:{}:{}", r.text, r.style));
        Ok(r.text.encode_utf16().count() as f64 * r.size * 0.6)
    }
}
fn setup() -> (PublicDocument, Host, Atom) {
    let mut doc = PublicEngine::with_builtins()
        .parse(SOURCE, &Default::default())
        .unwrap();
    let mut g = doc.snapshot().unwrap();
    let member = first(&g);
    set(&mut g, &member, "getDisplayDetails", Atom::HostReference(1));
    set(&mut g, &member, "parseClassifier", Atom::HostReference(2));
    doc.apply(0, g).unwrap();
    (
        doc,
        Host {
            mode: Rc::new(Cell::new(0)),
            trace: Rc::new(RefCell::new(vec![])),
        },
        member,
    )
}
#[test]
fn class_nested_original_methods_keep_aliases_and_late_render_order() {
    let (mut d, mut h, member) = setup();
    let out = d
        .render_operation_bounded(1, &Default::default(), Some(OutputLimit::Undefined), &mut h)
        .unwrap();
    let svg = out.result.unwrap().rendered.svg;
    assert!(svg.contains("Live label"));
    assert!(
        matches!(field(&out.graph,&out.graph.root,"savedMember"),Atom::Ref(id) if matches!(member,Atom::Ref(original) if original==id))
    );
    let trace = h.trace.borrow();
    assert!(trace[0].starts_with("width:A:"));
    let a = trace.iter().position(|s| s == "call:1").unwrap();
    assert_eq!(&trace[a..a + 3], ["call:1", "call:2", "width:Live label:2"]);
    drop(trace);
    let again = d
        .render_operation_bounded(2, &Default::default(), Some(OutputLimit::Undefined), &mut h)
        .unwrap();
    assert_eq!(again.result.unwrap().rendered.svg, svg);
}
#[test]
fn class_host_throw_and_invalid_result_publish_partial_graph_then_recover() {
    let (mut d, mut h, _) = setup();
    h.mode.set(1);
    let failed = d
        .render_operation_bounded(
            1,
            &Default::default(),
            Some(OutputLimit::Number(0.)),
            &mut h,
        )
        .unwrap();
    assert!(matches!(
        failed.result,
        Err(PublicError::HostException(HostExceptionToken(91)))
    ));
    assert_eq!(failed.revision, 2);
    assert!(!h.trace.borrow().iter().any(|s| s.starts_with("width:Live")));
    let saved = field(&failed.graph, &failed.graph.root, "savedDisplay");
    assert!(matches!(
        field(&failed.graph, &saved, "displayText"),
        Atom::String(_)
    ));
    h.mode.set(2);
    let invalid = d
        .render_operation_bounded(2, &Default::default(), Some(OutputLimit::Undefined), &mut h)
        .unwrap();
    assert!(invalid.result.is_err());
    h.mode.set(0);
    let limited = d
        .render_operation_bounded(
            3,
            &Default::default(),
            Some(OutputLimit::Number(0.)),
            &mut h,
        )
        .unwrap();
    assert!(matches!(limited.result,Err(PublicError::Pele(e))if e.code==ErrorCode::Limit));
    assert!(h.trace.borrow().iter().any(|s| s == "width:Live label:2"));
    assert!(d
        .render_operation_bounded(4, &Default::default(), Some(OutputLimit::Undefined), &mut h)
        .unwrap()
        .result
        .is_ok());
}
#[test]
fn class_readonly_nested_original_error_retains_caller_mutation() {
    let (mut d, mut h, member) = setup();
    let mut g = d.snapshot().unwrap();
    let Atom::Ref(id) = member else { panic!() };
    let key = g.strings.iter().position(|s| s.is("classifier")).unwrap() as u32;
    g.nodes[id as usize]
        .properties_mut()
        .iter_mut()
        .find(|p| p.key == key)
        .unwrap()
        .flags = 5;
    d.apply(1, g).unwrap();
    let result = d
        .render_operation_bounded(2, &Default::default(), Some(OutputLimit::Undefined), &mut h)
        .unwrap();
    assert!(matches!(
        result.result,
        Err(PublicError::JavaScript {
            name: JavaScriptErrorKind::TypeError,
            ..
        })
    ));
    assert_eq!(
        h.trace
            .borrow()
            .iter()
            .filter(|s| s.starts_with("call:"))
            .count(),
        2
    );
}
