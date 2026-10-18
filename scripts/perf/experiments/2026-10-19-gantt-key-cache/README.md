# Gantt fixed field-ID cache experiment

Isolated candidate against committed `32e845d4b0e63476dcaba8ebe4dc8730bdc7fdd0`. Root runtime, SDK and target artifacts remain unchanged. This experiment captures allocation counters and correctness only; no elapsed performance samples or speed claims.

The sole runtime patch adds sixteen positive string-ID slots to Gantt's operation-local `Context::own`. Slot keys are `id`, `task`, `section`, `startTime`, `endTime`, `renderEndTime`, `manualEndTime`, `processed`, `active`, `done`, `crit`, `milestone`, `vert`, `classes`, `raw` and `order`. An unresolved slot retries the existing lookup; misses, property absence, values and property indexes are never cached. First canonical UTF-16 IDs, descriptors, graph order, work-budget ticks and scheduling remain authoritative. IDs live only for that operation. Other keys retain the original lookup. The slots contain 64 bytes of fixed inline storage and allocate no heap cache or owned key strings.

The consumer reuses the existing 21 public workloads and repeated/unique GraphWriter controls. Its allocation-only executable checks whole SVG hashes, dimensions, type, complete model graph hashes (including arena IDs, raw UTF-16, descriptors and order), and pre/post graph hashes around each allocation capture. Gantt additionally captures histories of 1, 16 and 128 prior renders, with reset before every sample. Four public apply/render/recovery probes cover duplicate UTF-16 key IDs, deleted/reinserted fields, readonly scheduling fields and a key absent from the entire string table that scheduling first interns during the operation. These probes compare complete baseline/candidate state and retain the ordinary source-equivalent SVG expectation.

Allocation comparison passed: all 21 source outputs and complete graph fingerprints, 126 allocation rows per build, 18 history rows and 12 mutation/error/recovery traces. Gantt core-render allocations fell 8.5% / 12.5% / 13.3% for 8 / 48 / 200 tasks; publication allocations fell 6.8% / 11.0% / 12.0%. Medium core/publication each remove 802 allocations, 660 reallocations and 21,410 requested bytes. Heap peaks remain unchanged at all scales/histories. Pie, every non-Gantt workload, repeated/unique-key controls, public parse and snapshots retain exactly the same allocation counters. No elapsed rows were collected.

All nine release integration tests passed: two source tests and seven mutable public tests, including the complete 18,994-case historical corpus with no stress/filter exclusions. The toolchain emitted an optional debug-stripping warning (`rust-objcopy` could not load libLLVM); it did not prevent building or running the test executables. Replay preparation passed and reconstructed baseline/candidate input digests matched both measured builds. The committed integration log removes trailing whitespace only; evidence retains its original and normalized hashes.

Complete results and targeted integration status are recorded in `evidence.json`. Counters describe requested allocation bytes, not RSS, physical memory, Swift allocations or JavaScript heap usage. Even a reduction would not establish latency improvement: key dispatch and 64 extra inline bytes could affect cold operations. Adoption remains held pending matched timings, including small/unique controls and fixed Gantt history lengths, in a coordinated build-free window. The existing performance-pass authorization covers that follow-up.

## Reproduce

```sh
/path/to/experiment/reproduce.sh /absolute/path/to/pele-rs
```

Requires the pinned existing Pele development dependency and Rust toolchain; no install occurs. Replay creates a detached temporary worktree, applies the recorded harness, builds only the allocation executable into that checkout's own target directory, captures baseline, applies only `runtime.patch`, captures candidate, runs the public Gantt/historical integration targets and verifies exact output/identity/control equality before reporting deltas. No Root runtime, SDK or target directory is written. No binaries are bundled.

`--prepare-only` checks checkout, oracle and patch prerequisites without compilation. Complete raw allocation rows and input/binary fingerprints accompany the compact comparison. The allocation executable cannot return elapsed samples; the driver never sends a measurement command.
