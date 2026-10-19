# Performance: measured baseline and acceleration boundaries

Rust and WebGPU are implementation choices, not speedup evidence. The compatibility
port currently prioritizes matching pinned models, errors and SVG. No performance
claim applies to the legacy flowchart subset or the current 0.2 release. The
measurements below identify their own historical revisions and workloads.

## First browser measurement

2026-10-18, Apple M1 Pro, Chrome 155.0.8059.80, Rust 1.98.1, wasm-bindgen 0.2.128.
WASM is a release build optimized for size (`opt-level=s`, LTO, aborting panics).
Reference: Pele 1.6.0 at `458322a5b8abb1ae57ddf127c32017ab9fb14a92`.

This measures warm synchronous public APIs with browser Canvas measurement,
including options/result JSON, ownership and JS/WASM callbacks. Both implementations
produce identical SVG before timing. Each stage warms both implementations,
calibrates batches independently and alternates nine sample pairs. Loading,
compilation, DOM insertion, SVG painting, native Swift and GPU work are excluded.
Only four implemented families are sampled; graph-layout families remain unmeasured.

Large-source render medians from the initial run:

| Source | Pele JS | Current WASM | WASM / JS time |
| --- | ---: | ---: | ---: |
| Pie, 150 sections | 0.394 ms | 0.838 ms | 2.13 |
| Packet, 512 fields | 0.888 ms | 3.188 ms | 3.59 |
| Radar, 32 axes / 8 curves | 0.155 ms | 0.466 ms | 3.01 |
| Quadrant, 250 points | 1.013 ms | 1.363 ms | 1.35 |

Higher ratios mean slower WASM. Across the eight small/large source-render cases,
the current WASM path takes approximately 1.3–3.6 times the JS time. These are
single-machine engineering measurements, not a cross-platform product benchmark.
The initial run precedes bounded model projection, native AppKit text fixes,
round-trip JSON floating-point parsing and XYChart decimal-power portability fixes.
It is an archived baseline, not a timing of the latest worktree.
Raw samples, worktree state, browser version and runtime SHA256 hashes are in
[`benchmarks/2026-10-18-chrome-first.json`](benchmarks/2026-10-18-chrome-first.json).

Reproduce after a source WASM build:

```sh
scripts/build-wasm.sh
node scripts/benchmark-web.mjs
```

The script uses one browser and reuses or owns a local server. Results are written
to `dist/evidence/browser-benchmark.json`. Keep other builds and browser tests idle
while collecting timings. Public parse, source-render and retained-model rendering
are separate rows. The current script consumes returned metadata/SVG length;
this still excludes SVG materialization by a DOM renderer.

## Where improvement could come from

An isolated [Gantt field-ID cache experiment](../scripts/perf/experiments/2026-10-19-gantt-key-cache/README.md)
removes 802 allocations (12.5%) and 21,410 requested bytes from a medium mutable
core render. It adds 64 inline bytes per operation context, with no additional
heap peak or changes to Pie/unique-key controls. All 21 workload outputs and
complete graph fingerprints match; nine release integrations include the full
18,994-case Gantt corpus. This is allocation evidence, not latency evidence.
A separate [matched timing follow-up](../scripts/perf/experiments/2026-10-19-gantt-key-cache-latency/README.md)
records 161 paired trials at fixed histories against `42abbfb`. Medium/large
Gantt paired medians improved in that local run, but unchanged controls drifted
under heavy external load. Five warmups hit their operation cap; small/medium
history-128 latency was deliberately omitted before execution. The patch remains
isolated. These observations do not establish regression-free latency or a
general Rust/Swift/WASM speedup.

The next performance work needs profiles of parsing, preparation, text callbacks,
allocation/string building, model projection and serialization. Their costs are
different from browser SVG layout and paint. Candidate improvements include fewer
boundary crossings/copies, cached measurements and reusable parsing/layout work;
the present measurements do not establish which candidate dominates.

Workers can keep computation off the UI thread. That improves responsiveness when
work would otherwise block interaction; JS can use workers too, so it is not an
intrinsic Rust speedup. Module loading/compilation and worker lifetimes need their
own measurements. See [Google's Wasm performance patterns](https://web.dev/articles/webassembly-performance-patterns-for-web-apps).

WebGPU compute could accelerate sufficiently large parallel numeric/layout jobs,
given an appropriate algorithm. Setup, uploads, synchronization and readback must
be included in any comparison. SVG output ultimately needs CPU-readable geometry;
a GPU-backed canvas renderer would be a separate rendering backend. Small charts
may gain nothing from moving their work. These are engineering hypotheses, not
measured gains. [WebGPU's explainer](https://gpuweb.github.io/gpuweb/explainer/#buffer-mapping)
documents buffer copies and asynchronous CPU/GPU ownership transfer.

Any future accelerator should sit behind a backend interface, preserve a CPU path,
and have explicit precision/determinism rules. GPU execution currently has no
implementation or benchmark in this library. Full CPU feature parity remains the
release target; a future accelerator must not weaken that contract.
