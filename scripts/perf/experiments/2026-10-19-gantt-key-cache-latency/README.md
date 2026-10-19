# Matched native Gantt cache follow-up — runtime adoption held

**HOLD the runtime patch.** All fourteen selected Gantt conditions had negative paired median deltas, but unchanged Pie/unique controls drifted during a busy shared-host run. This does not establish regression-free latency, statistical significance or a general speedup. Root runtime and SDK remain unchanged.

The paired baseline is committed numeric-edge revision `42abbfbead6a8fa0c23bd578b6f09b78c747056e`; candidate adds only `runtime.patch`. The previous allocation experiment remains historical evidence for `32e845d`; its allocation reductions are not relabelled as measurements of this revision.

Completed predeclared reduced campaign: Gantt 8/48/200 tasks at histories 1/16, plus large 128; both native core/publication APIs; seven interleaved AB/BA pairs per condition. Pie 8/48/200 and unique-key controls use matched bounded batches. Total: 23 conditions, 161 paired trials (322 measured samples), within a hard 60-second invocation ceiling. Small/medium 128 latency was explicitly omitted before execution. No midrun selection or retry occurred.

Before timing, both builds passed all 21 shared whole-SVG/type/dimension/graph checks, 23 timer-free paired prechecks and 12 mutation/error/recovery traces. Every timed pair retained exact pre/post graph fingerprints, Date/revision/descriptor identities and published-snapshot equality. Input/workflow/binary fingerprints matched before and after. The OS guard rejected an actual owned compiler-name child and cleared after only that child stopped. Native children received an allowlisted environment; no credentials, arguments or environment dumps are included.

Observed paired median deltas were roughly −48 to −88 µs for medium Gantt and −160 to −722 µs for large Gantt. Small history 1 publication was nearly flat (−0.333 µs). Unchanged Pie small core moved +3.547 µs; unique-large control moved +25.234 µs. These controls prevent clean latency attribution. Full raw values, ordering, calibration and medians are retained; ratio-of-medians and median paired differences are distinct statistics.

An external xctest consumed about 96–98% CPU, load averaged 12.3 before/10.2 after, and desktop processes also consumed CPU. It was recorded and left running. Five warmup requests reached the 512-operation cap before the 100 ms work target; no equivalent warmed-state claim follows. No further timing or builds were run to search for a preferred result.

The timer covers actual native API return; final returned-value disposal, fixed-history setup, snapshot/hash verification and control I/O stay outside. This is neither an end-to-end Swift/FFI measurement nor comparable by subtraction with previous native components.

Files: [raw timing](timing.json), [results table](results.md), [summary/paired deltas](summary.json), [timer-free proof](verified.json), [build inputs](builds.json), [prepared sources](prepared.json), [candidate patch](runtime.patch), [guard proof](guard.log). Captured workflow sources under `workflow/` retain their exact measured hashes.

## Reproduce the workflow

Existing pinned Pele dependency and Rust toolchain required; no install occurs. Source preparation creates detached checkouts outside Root. Builds use only their own target directories. Native timing requires a separately coordinated parent GO under the existing user request.

```sh
node workflow/prepare.mjs --repo /absolute/path/to/pele-rs \
  --commit 42abbfbead6a8fa0c23bd578b6f09b78c747056e \
  --runtime-patch runtime.patch --out /tmp/NEW-ISOLATED-DIRECTORY
node workflow/build.mjs --prepared /tmp/NEW-ISOLATED-DIRECTORY/prepared.json
node workflow/run.mjs --prepared /tmp/NEW-ISOLATED-DIRECTORY/prepared.json \
  --history128 large-only
# Only during the coordinated quiet window:
node workflow/run.mjs --prepared /tmp/NEW-ISOLATED-DIRECTORY/prepared.json \
  --history128 large-only --quiet-run
```

Plan-only mode performs no builds or timings: `node workflow/run.mjs --plan-only --history128 large-only`. Reproduction must retain declared coverage and the hard cap; incomplete runs save raw/half-pair evidence and stop only owned consumers. Historical binaries are not bundled.
