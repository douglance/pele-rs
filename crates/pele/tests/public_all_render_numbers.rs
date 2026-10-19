//! Bounded all-family public source/immutable/mutable render audit. Each case owns a child process;
//! timeouts and oracle harness outcomes remain unresolved observations, never
//! claimed equivalent Pele errors. No native incremental-allocation guarantee.
use pele::compat::{public::*, *};
use serde_json::{json, Value as J};
use std::{
    io::Read,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

fn bits(number: f64) -> String {
    format!("{:016x}", number.to_bits())
}
fn number(value: &J) -> f64 {
    f64::from_bits(u64::from_str_radix(value["bits"].as_str().unwrap(), 16).unwrap())
}
fn options(case: &J) -> RenderOptions {
    let mut options: RenderOptions = serde_json::from_value(case["legacyOptions"].clone()).unwrap();
    for (key, value) in case["numeric"].as_object().unwrap() {
        let value = Some(number(value));
        match key.as_str() {
            "fontSize" => options.render.font_size = value,
            "maxWidth" => options.render.max_width = value,
            "directionBreakpoint" => options.render.direction_breakpoint = value,
            "padding" => options.render.padding = value,
            "now" => options.render.now = value,
            _ => panic!("Unexpected numeric override"),
        }
    }
    options
}
#[derive(Default)]
struct Host {
    calls: Vec<J>,
    exceeded: bool,
}
impl RenderResources for Host {
    fn text_width(&mut self, request: TextRequest<'_>) -> Result<f64, PeleError> {
        if self.calls.len() >= 2000 {
            self.exceeded = true;
            return Err(PeleError::new(
                "Native audit callback harness budget",
                ErrorCode::Limit,
            ));
        }
        let width = request.text.encode_utf16().count() as f64
            * request.size
            * if request.style & 1 != 0 { 0.71 } else { 0.6 };
        self.calls.push(json!([
            "width",
            request.text.encode_utf16().collect::<Vec<_>>(),
            bits(request.size),
            request.style,
            bits(width)
        ]));
        Ok(width)
    }
    fn now_millis(&mut self) -> Result<f64, PeleError> {
        self.calls.push(json!(["clock", bits(1704067200000.)]));
        Ok(1704067200000.)
    }
}

fn canonical(g: &ModelGraph, schema: ModelSchemaView) -> J {
    use std::collections::{HashMap, VecDeque};
    fn atom(
        g: &ModelGraph,
        a: &Atom,
        ids: &mut HashMap<u32, usize>,
        queue: &mut VecDeque<u32>,
        functions: &mut Vec<u32>,
    ) -> J {
        match a {
            Atom::Undefined => serde_json::json!(["undefined"]),
            Atom::Null => serde_json::json!(["null"]),
            Atom::Bool(v) => serde_json::json!(["bool", v]),
            Atom::Number(v) => serde_json::json!(["number", format!("{:016x}", v.to_bits())]),
            Atom::String(i) => serde_json::json!(["string", g.strings[*i as usize].0]),
            Atom::Ref(id) => {
                let next = ids.len();
                let index = *ids.entry(*id).or_insert_with(|| {
                    queue.push_back(*id);
                    next
                });
                serde_json::json!(["ref", index])
            }
            Atom::BuiltinFunction(id) => {
                let index = functions
                    .iter()
                    .position(|method| method == id)
                    .unwrap_or_else(|| {
                        functions.push(*id);
                        functions.len() - 1
                    });
                json!(["functionObservation", index])
            }
            Atom::HostReference(id) => serde_json::json!(["host", id]),
        }
    }
    let mut ids = HashMap::new();
    let mut queue = VecDeque::new();
    let mut functions = Vec::new();
    let root = atom(g, &g.root, &mut ids, &mut queue, &mut functions);
    let mut nodes = Vec::new();
    while let Some(id) = queue.pop_front() {
        let n = &g.nodes[id as usize];
        let mut out = serde_json::json!({"kind":match n{Node::Error{..}=>"error",Node::Instance{..}=>"instance",Node::Object{..}=>"object",Node::Array{..}=>"array",Node::Map{..}=>"map",Node::Set{..}=>"set",Node::Date{..}=>"date"}});
        match n {
            Node::Error { error_type, .. } => {
                out["errorType"] = serde_json::to_value(error_type).unwrap();
            }
            Node::Instance { prototype, .. } => out["prototype"] = prototype.0.into(),
            Node::Array { length, .. } => out["length"] = (*length).into(),
            Node::Map { entries, .. } => {
                out["entries"] = entries
                    .iter()
                    .map(|(k, v)| {
                        serde_json::json!([
                            atom(g, k, &mut ids, &mut queue, &mut functions),
                            atom(g, v, &mut ids, &mut queue, &mut functions)
                        ])
                    })
                    .collect()
            }
            Node::Set { values, .. } => {
                out["values"] = values
                    .iter()
                    .map(|v| atom(g, v, &mut ids, &mut queue, &mut functions))
                    .collect()
            }
            Node::Date { milliseconds, .. } => {
                out["milliseconds"] = bits(*milliseconds).into();
            }
            _ => {}
        }
        let mut properties = n.properties().iter().collect::<Vec<_>>();
        let index = |p: &&Property| {
            g.strings[p.key as usize].well_formed().and_then(|s| {
                s.parse::<u32>()
                    .ok()
                    .filter(|n| *n != u32::MAX && n.to_string() == s)
            })
        };
        properties.sort_by(|a, b| match (index(a), index(b)) {
            (Some(x), Some(y)) => x.cmp(&y),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            _ => std::cmp::Ordering::Equal,
        });
        out["properties"] = properties
            .into_iter()
            .map(|p| {
                serde_json::json!([
                    g.strings[p.key as usize].0,
                    atom(g, &p.value, &mut ids, &mut queue, &mut functions),
                    p.flags
                ])
            })
            .collect();
        nodes.push(out);
    }
    let mut result = json!({"root":root,"nodes":nodes});
    if !functions.is_empty() {
        result["functionObservations"] = functions.iter().map(|id| {
            let arity = schema.model.methods.iter().find(|method| method.id.0 == *id).unwrap().arity;
            let constructible = schema.function_shapes.iter().find(|shape| shape.method.0 == *id).is_some_and(|shape| shape.constructible);
            json!({"arity":arity,"ownKeys":if constructible {vec!["length","name","prototype"]}else{vec!["length","name"]}})
        }).collect();
    }
    result
}

fn expected_graph(value: &J) -> J {
    let mut value = value.clone();
    value.as_object_mut().unwrap().remove("functionScope");
    if let Some(functions) = value.get_mut("functionObservations") {
        for function in functions.as_array_mut().unwrap() {
            function.as_object_mut().unwrap().remove("name");
        }
    }
    value
}
fn observation(result: Result<RenderResult, PeleError>, host: Host) -> J {
    let observation = if host.exceeded {
        json!({"harness":{"kind":"native-callback-budget"}})
    } else {
        match result {
            Ok(result) => {
                json!({"result":result,"widthBits":bits(result.rendered.width),"heightBits":bits(result.rendered.height)})
            }
            Err(error) => json!({"error":error}),
        }
    };
    json!({"observation":observation,"calls":host.calls})
}
#[test]
fn all_render_numbers_worker() {
    let Some(input) = std::env::var_os("PELE_ALL_NUMERIC_CASE") else {
        return;
    };
    let output = std::env::var_os("PELE_ALL_NUMERIC_RESULT").unwrap();
    let case: J = serde_json::from_reader(std::fs::File::open(input).unwrap()).unwrap();
    let engine = Engine::with_builtins();
    let options = options(&case);
    let source = case["source"].as_str().unwrap();
    let policy = Some(OutputLimit::Undefined);
    let mut host = Host::default();
    let result = engine.render_source_bounded(source, &options, policy, &mut host);
    let source_result = observation(result, host);
    let document = engine.parse(source, &options.parse).unwrap();
    let mut host = Host::default();
    let result = engine.render_document_bounded(&document, &options, policy, &mut host);
    let immutable_result = observation(result, host);
    let mut document = PublicEngine::with_builtins()
        .parse(source, &options.parse)
        .unwrap();
    let schema = document.schema_view();
    let before = canonical(&document.snapshot().unwrap(), schema);
    let mut host = Host::default();
    let outcome = document
        .render_operation_bounded(0, &options, policy, &mut host)
        .unwrap();
    assert_eq!(outcome.revision, 1);
    let after = canonical(&outcome.graph, schema);
    let result = outcome.result.map_err(|error| match error {
        PublicError::Pele(error) => error,
        error => PeleError::new(format!("{error:?}"), ErrorCode::Semantic),
    });
    let mut mutable_result = observation(result, host);
    mutable_result["graphBefore"] = before;
    mutable_result["graphAfter"] = after;
    std::fs::write(
        output,
        serde_json::to_vec(
            &json!({"source":source_result,"immutable":immutable_result,"mutable":mutable_result}),
        )
        .unwrap(),
    )
    .unwrap();
}

fn equal_json(actual: &J, expected: &J) -> bool {
    match (actual, expected) {
        (J::Number(a), J::Number(b)) => a.as_f64() == b.as_f64(),
        (J::Array(a), J::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| equal_json(a, b))
        }
        (J::Object(a), J::Object(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(key, value)| b.get(key).is_some_and(|other| equal_json(value, other)))
        }
        (J::String(a), J::String(b))
            if a.len() == 16
                && b.len() == 16
                && u64::from_str_radix(a, 16).is_ok()
                && u64::from_str_radix(b, 16).is_ok() =>
        {
            equal_computed_bits(actual, expected)
        }
        _ => actual == expected,
    }
}
fn equal_computed_bits(actual: &J, expected: &J) -> bool {
    let parse = |value: &J| {
        value
            .as_str()
            .and_then(|s| u64::from_str_radix(s, 16).ok())
            .map(f64::from_bits)
    };
    match (parse(actual), parse(expected)) {
        (Some(a), Some(b)) => (a.is_nan() && b.is_nan()) || a.to_bits() == b.to_bits(),
        _ => actual == expected,
    }
}
fn trace_difference(actual: &J, expected: &J) -> Option<J> {
    let (Some(actual), Some(expected)) = (actual.as_array(), expected.as_array()) else {
        return Some(json!({"shape":true}));
    };
    if actual.len() != expected.len() {
        return Some(json!({"actualCount":actual.len(),"expectedCount":expected.len()}));
    }
    for (index, (a, e)) in actual.iter().zip(expected).enumerate() {
        let same = if e[0] == "clock" {
            a == e
        } else {
            a[0] == e[0]
                && a[1] == e[1]
                && a[3] == e[3]
                && equal_computed_bits(&a[2], &e[2])
                && equal_computed_bits(&a[4], &e[4])
        };
        if !same {
            return Some(json!({"index":index,"actual":a,"expected":e}));
        }
    }
    None
}

#[test]
fn all_families_numeric_render_options_match_pinned_oracle_in_owned_processes() {
    let bytes = include_bytes!("../../../fixtures/pele/all-render-numbers.json.gz");
    let mut json_text = String::new();
    flate2::read::GzDecoder::new(bytes.as_slice())
        .read_to_string(&mut json_text)
        .unwrap();
    let fixture: J = serde_json::from_str(&json_text).unwrap();
    assert_eq!(fixture["revision"], UPSTREAM_REVISION);
    assert_eq!(
        fixture["oracleSha256"],
        "d1ebc1889f3069d7759fe5db34c4ad4b4823285edbee2b537d0ef3d505a76d8d"
    );
    let cases = fixture["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 1023);
    assert_eq!(fixture["sources"].as_object().unwrap().len(), 33);
    let root = std::env::temp_dir().join(format!("pele-all-numeric-audit-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let executable = std::env::current_exe().unwrap();
    let mut mismatches = vec![];
    let mut unresolved = vec![];
    let selected = std::env::var("PELE_ALL_NUMERIC_FILTER").ok();
    let mut executed = 0;
    for (index, case) in cases.iter().enumerate() {
        let id = case["id"].as_str().unwrap();
        if selected.as_ref().is_some_and(|value| !id.contains(value)) {
            continue;
        }
        if case["expected"].get("harness").is_some() {
            unresolved.push(json!({"id":id,"oracle":case["expected"]["harness"]}));
            continue;
        }
        let input = root.join("case.json");
        let output = root.join("result.json");
        std::fs::write(&input, serde_json::to_vec(case).unwrap()).unwrap();
        if output.exists() {
            std::fs::remove_file(&output).unwrap()
        }
        let mut child = Command::new(&executable)
            .args([
                "--exact",
                "all_render_numbers_worker",
                "--nocapture",
                "--test-threads=1",
            ])
            .env_clear()
            .env("TZ", "UTC")
            .env("LANG", "C")
            .env("PELE_ALL_NUMERIC_CASE", &input)
            .env("PELE_ALL_NUMERIC_RESULT", &output)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let start = Instant::now();
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break Some(status);
            }
            if start.elapsed() >= Duration::from_secs(3) {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
            thread::sleep(Duration::from_millis(5));
        };
        executed += 1;
        if !status.is_some_and(|status| status.success()) || !output.exists() {
            unresolved.push(json!({"id":id,"nativeHarness":if status.is_none(){"owned-worker-timeout"}else{"owned-worker-failure"}}));
            continue;
        }
        let actual: J = serde_json::from_reader(std::fs::File::open(&output).unwrap()).unwrap();
        let expected = &case["expected"];
        for lane in ["source", "immutable", "mutable"] {
            let actual_lane = &actual[lane];
            if let Some(harness) = actual_lane["observation"].get("harness") {
                unresolved.push(json!({"id":id,"lane":lane,"nativeHarness":harness}));
                continue;
            }
            let value = &actual_lane["observation"];
            let result_same = if expected.get("result").is_some() {
                equal_json(&value["result"], &expected["result"])
                    && equal_computed_bits(&value["widthBits"], &expected["widthBits"])
                    && equal_computed_bits(&value["heightBits"], &expected["heightBits"])
            } else {
                equal_json(&value["error"], &expected["error"])
            };
            let trace = trace_difference(&actual_lane["calls"], &expected["calls"]);
            let before_same = lane != "mutable"
                || equal_json(
                    &actual_lane["graphBefore"],
                    &expected_graph(&expected["graphBefore"]),
                );
            let after_same = lane != "mutable"
                || equal_json(
                    &actual_lane["graphAfter"],
                    &expected_graph(&expected["graphAfter"]),
                );
            if !result_same || trace.is_some() || !before_same || !after_same {
                let evidence = root.join(format!("mismatch-{index}-{lane}.json"));
                std::fs::write(
                    &evidence,
                    serde_json::to_vec_pretty(&json!({"case":case,"native":actual})).unwrap(),
                )
                .unwrap();
                mismatches.push(json!({"id":id,"lane":lane,"resultMismatch":!result_same,"beforeMismatch":!before_same,"afterMismatch":!after_same,"trace":trace,"evidence":evidence}));
            }
        }
    }
    let summary = json!({"executed":executed,"mismatches":mismatches,"unresolved":unresolved,
        "lane":"bounded source, immutable and mutable render operations; complete mutable graphs with function identity/arity/construction shape; function source/name excluded","evidenceRoot":root});
    std::fs::write(
        root.join("summary.json"),
        serde_json::to_vec_pretty(&summary).unwrap(),
    )
    .unwrap();
    eprintln!("ALL_NUMERIC_AUDIT {}", summary);
    assert!(
        summary["mismatches"].as_array().unwrap().is_empty()
            && summary["unresolved"].as_array().unwrap().is_empty(),
        "All-family numeric parity remains unresolved; inspect {}/summary.json",
        root.display()
    );
    assert!(executed > 0, "Filter selected no cases");
}
