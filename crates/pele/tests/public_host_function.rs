//! Public Gantt host-method feature: live identity, original delegation and errors.
use pele::compat::{public::*, text::MetricsResources, *};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
const NOW: f64 = 1704067200000.;
const SOURCE: &str = "gantt\ndateFormat YYYY-MM-DD\nsection S\nA:a,2024-01-01,1d\nB:b,after a,2d";
fn field(g: &ModelGraph, a: &Atom, key: &str) -> Atom {
    let Atom::Ref(id) = a else { panic!() };
    g.nodes[*id as usize]
        .properties()
        .iter()
        .find(|p| g.strings[p.key as usize].is(key))
        .unwrap()
        .value
        .clone()
}
fn set(g: &mut ModelGraph, key: &str, value: Atom) {
    let Atom::Ref(id) = g.root else { panic!() };
    let key_id = g.strings.len() as u32;
    g.strings.push(JsString::from(key));
    g.nodes[id as usize].properties_mut().push(Property {
        key: key_id,
        value,
        flags: 7,
    });
}
fn doc() -> PublicDocument {
    let mut d = PublicEngine::with_builtins()
        .parse(SOURCE, &Default::default())
        .unwrap();
    let mut g = d.snapshot().unwrap();
    set(&mut g, "getTasks", Atom::HostReference(7));
    d.apply(0, g).unwrap();
    d
}
struct Function {
    mode: Cell<u8>,
    seen: RefCell<Vec<Atom>>,
}
impl HostFunction for Function {
    fn call(
        &self,
        m: &mut dyn ActiveModel,
        receiver: Atom,
        args: &[Atom],
        h: &mut dyn RenderResources,
    ) -> Result<Atom, PublicError> {
        assert!(matches!(&args[0],Atom::Number(v) if *v==NOW));
        let tasks = m.invoke_builtin(
            MethodCall {
                method: MethodId(19),
                receiver: receiver.clone(),
                arguments: args.to_vec(),
            },
            h,
        )?;
        let first = m.read(&tasks, "0")?;
        let second = m.read(&tasks, "1")?;
        let end = m.read(&first, "endTime")?;
        let start = m.read(&second, "startTime")?;
        assert!(matches!((&end,&start),(Atom::Ref(a),Atom::Ref(b)) if a==b));
        assert_eq!(m.date_millis(&end)?, NOW + 86400000.);
        self.seen
            .borrow_mut()
            .extend([tasks.clone(), first.clone(), end]);
        let label = m.intern(JsString::from("Changed live"))?;
        m.set(&first, "task", label)?;
        // Captured native call leaves the host override in place.
        assert!(matches!(
            m.read(&receiver, "getTasks")?,
            Atom::HostReference(7)
        ));
        match self.mode.get() {
            1 => {
                let alternate = m.array()?;
                m.push(&alternate, first)?;
                Ok(alternate)
            }
            2 => Err(PublicError::HostException(HostExceptionToken(42))),
            3 => {
                let date = m.date(-0.5)?;
                assert_eq!(m.date_millis(&date)?.to_bits(), 0f64.to_bits());
                let s = m.intern(JsString(vec![0xd800, 65]))?;
                assert_eq!(m.string(&s)?, JsString(vec![0xd800, 65]));
                m.set(&receiver, "alias", s)?;
                Ok(tasks)
            }
            4 => Ok(Atom::Ref(u32::MAX)),
            _ => Ok(tasks),
        }
    }
}
struct Host {
    function: Rc<Function>,
    clock: usize,
    measured: usize,
}
impl RenderResources for Host {
    fn host_function(&mut self, id: u32) -> Result<Rc<dyn HostFunction>, PublicError> {
        assert_eq!(id, 7);
        Ok(self.function.clone())
    }
    fn now_millis(&mut self) -> Result<f64, PeleError> {
        self.clock += 1;
        Ok(NOW)
    }
    fn text_width(&mut self, r: TextRequest<'_>) -> Result<f64, PeleError> {
        self.measured += 1;
        MetricsResources.text_width(r)
    }
}
fn host() -> Host {
    Host {
        function: Rc::new(Function {
            mode: Cell::new(0),
            seen: RefCell::new(vec![]),
        }),
        clock: 0,
        measured: 0,
    }
}
#[test]
fn live_original_aliases_alternate_array_and_recovery() {
    let mut d = doc();
    let mut h = host();
    let full = d
        .render_operation_bounded(1, &Default::default(), Some(OutputLimit::Undefined), &mut h)
        .unwrap();
    let drawing = full.result.unwrap();
    assert!(drawing.rendered.svg.contains("Changed live"));
    assert_eq!(h.clock, 1);
    assert!(h.measured > 0);
    let tasks = field(&full.graph, &full.graph.root, "tasks");
    assert!(matches!((&tasks,&h.function.seen.borrow()[0]),(Atom::Ref(a),Atom::Ref(b)) if a==b));
    h.function.mode.set(1);
    let short = d
        .render_operation_bounded(2, &Default::default(), Some(OutputLimit::Undefined), &mut h)
        .unwrap();
    assert!(short.result.unwrap().rendered.height < drawing.rendered.height);
    assert!(matches!(field(&short.graph, &tasks, "1"), Atom::Ref(_))); // original array still has both tasks
    h.function.mode.set(2);
    let measured = h.measured;
    let failed = d
        .render_operation_bounded(3, &Default::default(), Some(OutputLimit::Undefined), &mut h)
        .unwrap();
    assert!(matches!(
        failed.result,
        Err(PublicError::HostException(HostExceptionToken(42)))
    ));
    assert_eq!(h.measured, measured);
    assert_eq!(failed.revision, 4);
    let first = field(&failed.graph, &tasks, "0");
    let label = field(&failed.graph, &first, "task");
    assert!(
        matches!(label,Atom::String(id) if failed.graph.strings[id as usize].is("Changed live"))
    );
    h.function.mode.set(3);
    let recovered = d
        .render_operation_bounded(4, &Default::default(), Some(OutputLimit::Undefined), &mut h)
        .unwrap();
    assert_eq!(recovered.result.unwrap().rendered.svg, drawing.rendered.svg);
    assert!(
        matches!(field(&recovered.graph,&recovered.graph.root,"alias"),Atom::String(id) if recovered.graph.strings[id as usize].0==[0xd800,65])
    );
}
#[test]
fn malformed_return_and_late_output_error_publish_partial_edits() {
    let mut d = doc();
    let mut h = host();
    h.function.mode.set(4);
    let failed = d
        .render_operation_bounded(1, &Default::default(), Some(OutputLimit::Undefined), &mut h)
        .unwrap();
    assert!(failed.result.is_err());
    assert_eq!(h.measured, 0);
    assert_eq!(failed.revision, 2);
    h.function.mode.set(0);
    let failed = d
        .render_operation_bounded(
            2,
            &Default::default(),
            Some(OutputLimit::Number(0.)),
            &mut h,
        )
        .unwrap();
    assert!(failed.result.is_err());
    assert!(h.measured > 0);
    assert_eq!(failed.revision, 3);
    assert!(d
        .render_operation_bounded(2, &Default::default(), None, &mut h)
        .is_err());
    assert!(d
        .render_operation_bounded(3, &Default::default(), Some(OutputLimit::Undefined), &mut h)
        .unwrap()
        .result
        .is_ok());
}
#[test]
fn unbound_reference_is_explicit_and_does_not_undo_prior_clock() {
    struct NoFunction {
        calls: usize,
    }
    impl RenderResources for NoFunction {
        fn text_width(&mut self, r: TextRequest<'_>) -> Result<f64, PeleError> {
            MetricsResources.text_width(r)
        }
        fn now_millis(&mut self) -> Result<f64, PeleError> {
            self.calls += 1;
            Ok(NOW)
        }
    }
    let mut d = doc();
    let mut h = NoFunction { calls: 0 };
    let failed = d
        .render_operation_bounded(1, &Default::default(), None, &mut h)
        .unwrap();
    assert!(
        matches!(failed.result,Err(PublicError::Pele(e)) if e.message=="Unbound host function reference")
    );
    assert_eq!(h.calls, 1);
    assert_eq!(failed.revision, 2);
}

#[test]
fn pinned_host_override_drawing_callback_order_and_precall_fields() {
    use serde_json::{json, Value};
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../fixtures/public/gantt-host-functions.json"
    ))
    .unwrap();
    struct OracleFunction {
        mode: String,
        events: Rc<RefCell<Vec<Value>>>,
    }
    impl HostFunction for OracleFunction {
        fn call(
            &self,
            model: &mut dyn ActiveModel,
            receiver: Atom,
            args: &[Atom],
            host: &mut dyn RenderResources,
        ) -> Result<Atom, PublicError> {
            let Atom::Number(now) = args[0] else { panic!() };
            self.events.borrow_mut().push(json!(["host", now]));
            let tasks = model.invoke_builtin(
                MethodCall {
                    method: MethodId(19),
                    receiver: receiver.clone(),
                    arguments: args.to_vec(),
                },
                host,
            )?;
            let first = model.read(&tasks, "0")?;
            let second = model.read(&tasks, "1")?;
            assert!(
                matches!((model.read(&first,"endTime")?,model.read(&second,"startTime")?),(Atom::Ref(a),Atom::Ref(b))if a==b)
            );
            let label = model.intern(JsString::from("Changed live"))?;
            model.set(&first, "task", label)?;
            if self.mode == "capturedFlags" {
                let mode = model.intern(JsString::from("compact"))?;
                model.set(&receiver, "displayMode", mode)?;
                model.set(&receiver, "topAxis", Atom::Bool(true))?;
            }
            if self.mode == "alternate" {
                let array = model.array()?;
                model.push(&array, first)?;
                Ok(array)
            } else {
                Ok(tasks)
            }
        }
    }
    struct OracleHost {
        function: Rc<OracleFunction>,
        events: Rc<RefCell<Vec<Value>>>,
    }
    impl RenderResources for OracleHost {
        fn host_function(&mut self, _: u32) -> Result<Rc<dyn HostFunction>, PublicError> {
            Ok(self.function.clone())
        }
        fn text_width(&mut self, r: TextRequest<'_>) -> Result<f64, PeleError> {
            self.events
                .borrow_mut()
                .push(json!(["width", r.text, r.size, r.style]));
            Ok(r.text.encode_utf16().count() as f64 * r.size * 0.6)
        }
    }
    fn numbers(value: &Value) -> Value {
        match value {
            Value::Number(n) => json!(n.as_f64().unwrap()),
            Value::Array(a) => Value::Array(a.iter().map(numbers).collect()),
            Value::Object(o) => {
                Value::Object(o.iter().map(|(k, v)| (k.clone(), numbers(v))).collect())
            }
            _ => value.clone(),
        }
    }
    for case in fixture["cases"].as_array().unwrap() {
        let mut doc = doc();
        let events = Rc::new(RefCell::new(vec![]));
        let mut host = OracleHost {
            function: Rc::new(OracleFunction {
                mode: case["mode"].as_str().unwrap().into(),
                events: events.clone(),
            }),
            events: events.clone(),
        };
        let mut options = RenderOptions::default();
        options.render.now = Some(NOW);
        let outcome = doc
            .render_operation_bounded(1, &options, Some(OutputLimit::Undefined), &mut host)
            .unwrap();
        let drawing =
            serde_json::to_value(render_result_transport(&outcome.result.unwrap())).unwrap();
        assert_eq!(drawing, case["drawing"], "{}", case["mode"]);
        assert_eq!(
            numbers(&serde_json::to_value(&*events.borrow()).unwrap()),
            numbers(&case["calls"]),
            "{}",
            case["mode"]
        );
    }
}
