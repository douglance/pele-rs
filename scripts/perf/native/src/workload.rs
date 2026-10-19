//! Matched public native workloads. NDJSON control/setup is outside timed regions.
use pele::compat::{self, public::*, text::MetricsResources, *};
use serde::Deserialize;
use serde_json::{json, Value as J};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    hint::black_box,
    io::{self, BufRead, Write},
    time::Instant,
};
const NOW: f64 = 1_704_067_200_000.;
#[derive(Deserialize)]
struct Case {
    id: String,
    source: String,
    options: J,
    resources: String,
}
struct Host {
    fixed: bool,
    calls: usize,
}
impl RenderResources for Host {
    fn text_width(&mut self, r: TextRequest<'_>) -> Result<f64, PeleError> {
        self.calls += 1;
        if self.fixed {
            Ok(r.text.encode_utf16().count() as f64
                * r.size
                * if r.style & 1 != 0 { 0.71 } else { 0.6 })
        } else {
            MetricsResources.text_width(r)
        }
    }
    fn now_millis(&mut self) -> Result<f64, PeleError> {
        Ok(NOW)
    }
}
struct State {
    source: String,
    options: RenderOptions,
    host: Host,
    typed: ParsedDocument,
    public: PublicDocument,
    graph: ModelGraph,
    binary: Vec<u8>,
    result: RenderResult,
}
fn error(e: impl std::fmt::Debug) -> String {
    format!("{e:?}")
}
fn summary(r: &RenderResult) -> J {
    json!({"type":r.diagram_type.as_str(),"widthBits":format!("{:016x}",r.rendered.width.to_bits()),
        "heightBits":format!("{:016x}",r.rendered.height.to_bits()),
        "svgSha256":format!("{:x}",Sha256::digest(r.rendered.svg.as_bytes())),"svgBytes":r.rendered.svg.len()})
}
impl State {
    fn new(c: Case, engine: &Engine, public: &PublicEngine) -> Result<Self, String> {
        let options: RenderOptions = serde_json::from_value(c.options).map_err(error)?;
        let mut host = Host {
            fixed: c.resources == "fixed",
            calls: 0,
        };
        let typed = engine.parse(&c.source, &options.parse).map_err(error)?;
        let mut document = public.parse(&c.source, &options.parse).map_err(error)?;
        let result = engine
            .render_document(&typed, &options, &mut host)
            .map_err(error)?;
        let rendered = document
            .render_operation(document.revision(), &options, &mut host)
            .map_err(error)?
            .result
            .map_err(error)?;
        if summary(&result) != summary(&rendered) {
            return Err("Native immutable/mutable output mismatch".into());
        }
        let graph = document.snapshot().map_err(error)?;
        let binary =
            encode_binary_exchange(0, &graph, b"{}", DEFAULT_BINARY_BUDGET).map_err(error)?;
        Ok(Self {
            source: c.source,
            options,
            host,
            typed,
            public: document,
            graph,
            binary,
            result,
        })
    }
    fn reset(&mut self, public: &PublicEngine) -> Result<(), String> {
        self.public = public
            .parse(&self.source, &self.options.parse)
            .map_err(error)?;
        self.public
            .render_operation(self.public.revision(), &self.options, &mut self.host)
            .map_err(error)?
            .result
            .map_err(error)?;
        self.host.calls = 0;
        Ok(())
    }
    fn once(&mut self, stage: &str, engine: &Engine, public: &PublicEngine) -> Result<u64, String> {
        let render_checksum = |r: RenderResult| {
            black_box(
                r.rendered.svg.len() as u64
                    ^ r.rendered.width.to_bits()
                    ^ r.rendered.height.to_bits(),
            )
        };
        Ok(match stage {
            "registry" => {
                let e = Engine::with_builtins();
                black_box(&e);
                33
            }
            "public-registry" => {
                let e = PublicEngine::with_builtins();
                black_box(&e);
                33
            }
            "parse" => black_box(
                engine
                    .parse(&self.source, &self.options.parse)
                    .map_err(error)?
                    .diagram_type()
                    .as_str()
                    .len() as u64,
            ),
            "parse-fresh-registry" => black_box(
                Engine::with_builtins()
                    .parse(&self.source, &self.options.parse)
                    .map_err(error)?
                    .diagram_type()
                    .as_str()
                    .len() as u64,
            ),
            "parse-public" => black_box(
                public
                    .parse(&self.source, &self.options.parse)
                    .map_err(error)?
                    .diagram_type()
                    .as_str()
                    .len() as u64,
            ),
            "parse-public-fresh-registry" => black_box(
                PublicEngine::with_builtins()
                    .parse(&self.source, &self.options.parse)
                    .map_err(error)?
                    .diagram_type()
                    .as_str()
                    .len() as u64,
            ),
            "render-source" => render_checksum(
                engine
                    .render_source(&self.source, &self.options, &mut self.host)
                    .map_err(error)?,
            ),
            "render-retained" => render_checksum(
                engine
                    .render_document(&self.typed, &self.options, &mut self.host)
                    .map_err(error)?,
            ),
            "render-mutable-core" => render_checksum(
                self.public
                    .render(&self.options, &mut self.host)
                    .map_err(error)?,
            ),
            "render-mutable-publish" => {
                let o = self
                    .public
                    .render_operation(self.public.revision(), &self.options, &mut self.host)
                    .map_err(error)?;
                render_checksum(o.result.map_err(error)?) ^ o.graph.nodes.len() as u64
            }
            "model-json" => black_box(
                serde_json::to_vec(&self.typed.model_view().map_err(error)?)
                    .map_err(error)?
                    .len() as u64,
            ),
            "graph-snapshot" => {
                black_box(self.public.snapshot().map_err(error)?.nodes.len() as u64)
            }
            "graph-json-encode" => {
                black_box(serde_json::to_vec(&self.graph).map_err(error)?.len() as u64)
            }
            "graph-json-roundtrip" => {
                let bytes = serde_json::to_vec(&self.graph).map_err(error)?;
                let g: ModelGraph = serde_json::from_slice(&bytes).map_err(error)?;
                g.validate().map_err(error)?;
                black_box(g.nodes.len() as u64)
            }
            "graph-binary-encode" => black_box(
                encode_binary_exchange(0, &self.graph, b"{}", DEFAULT_BINARY_BUDGET)
                    .map_err(error)?
                    .len() as u64,
            ),
            "graph-binary-decode" => black_box(
                decode_binary_exchange(&self.binary, DEFAULT_BINARY_BUDGET)
                    .map_err(error)?
                    .graph
                    .nodes
                    .len() as u64,
            ),
            "graph-apply" => {
                self.public
                    .apply(self.public.revision(), self.graph.clone())
                    .map_err(error)?;
                black_box(self.public.revision())
            }
            "result-json" => black_box(compat::render_result_envelope(&self.result).len() as u64),
            _ => return Err(format!("Unknown benchmark stage {stage}")),
        })
    }
}
fn handle(
    command: &J,
    states: &mut HashMap<String, State>,
    engine: &Engine,
    public: &PublicEngine,
) -> Result<J, String> {
    let op = command["op"].as_str().ok_or("Missing op")?;
    if op == "prepare" {
        let case: Case = serde_json::from_value(command["case"].clone()).map_err(error)?;
        let id = case.id.clone();
        let state = State::new(case, engine, public)?;
        let result = json!({"output":summary(&state.result),"graphNodes":state.graph.nodes.len(),"graphStrings":state.graph.strings.len(),
            "graphUtf16Units":state.graph.strings.iter().map(|s|s.0.len()).sum::<usize>(),
            "graphJsonBytes":serde_json::to_vec(&state.graph).map_err(error)?.len(),"graphBinaryBytes":state.binary.len()});
        states.insert(id, state);
        return Ok(result);
    }
    let id = command["case"].as_str().ok_or("Missing case")?;
    let state = states.get_mut(id).ok_or("Case not prepared")?;
    if op == "verify" {
        let mut outputs = vec![];
        outputs.push(summary(
            &engine
                .render_source(&state.source, &state.options, &mut state.host)
                .map_err(error)?,
        ));
        outputs.push(summary(
            &engine
                .render_document(&state.typed, &state.options, &mut state.host)
                .map_err(error)?,
        ));
        outputs.push(summary(
            &state
                .public
                .render_operation(state.public.revision(), &state.options, &mut state.host)
                .map_err(error)?
                .result
                .map_err(error)?,
        ));
        return Ok(
            json!({"outputs":outputs,"graphNodes":state.public.snapshot().map_err(error)?.nodes.len()}),
        );
    }
    if std::env::var("PELE_PERF_ALLOW_TIMING").as_deref() != Ok("1") {
        return Err(
            "Timed/profile runs require PELE_PERF_ALLOW_TIMING=1 and a coordinated quiet window"
                .into(),
        );
    }
    let stage = command["stage"].as_str().ok_or("Missing stage")?;
    let count = command["count"]
        .as_u64()
        .filter(|n| *n > 0 && *n <= 65_536)
        .ok_or("Invalid batch count")?;
    state.reset(public)?;
    let before = state.public.snapshot().map_err(error)?.nodes.len();
    if op == "allocations" {
        return crate::capture_allocations(|| {
            let mut checksum = 0u64;
            for _ in 0..count {
                checksum = checksum.wrapping_add(state.once(stage, engine, public)?);
            }
            Ok(checksum)
        });
    }
    if op != "measure" {
        return Err("Unknown op".into());
    }
    if crate::ALLOCATION_BUILD {
        return Err("Allocation-instrumented binary cannot produce elapsed timing samples".into());
    }
    let mut checksum = 0u64;
    let start = Instant::now();
    for _ in 0..count {
        checksum = checksum.wrapping_add(state.once(stage, engine, public)?);
    }
    let elapsed = start.elapsed().as_nanos();
    let after = state.public.snapshot().map_err(error)?.nodes.len();
    Ok(
        json!({"elapsedNs":elapsed,"count":count,"checksum":checksum,"textMeasurements":state.host.calls,"graphNodesBefore":before,"graphNodesAfter":after}),
    )
}
pub fn main() {
    let engine = Engine::with_builtins();
    let public = PublicEngine::with_builtins();
    let mut states = HashMap::new();
    let stdin = io::stdin();
    let mut out = io::BufWriter::new(io::stdout());
    for line in stdin.lock().lines() {
        let result = (|| {
            let c: J = serde_json::from_str(&line.map_err(error)?).map_err(error)?;
            handle(&c, &mut states, &engine, &public)
        })();
        let value = match result {
            Ok(v) => json!({"ok":true,"value":v}),
            Err(e) => json!({"ok":false,"error":e}),
        };
        if writeln!(out, "{value}").and_then(|_| out.flush()).is_err() {
            break;
        }
    }
}
