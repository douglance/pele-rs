# GraphWriter property-cache experiment

**Hold runtime adoption.** This isolated prototype identifies allocation cost; it makes no speed claim. Root runtime stayed unchanged. No timed benchmark ran while native/Swift correctness jobs ran.

The only candidate edit adds a borrowed-`str` positive cache to `GraphWriter::property`: maximum 64 names of at most 64 UTF-8 bytes. Generic/raw UTF-16 `string` interning, exact IDs/order, hash equality, validation and budgets remain unchanged. Baseline is commit `a34ca6c` plus the captured central-registry overlay. `evidence.json` records both exact input/binary hashes and all results.

**Correctness:** 21 small/medium/large cases matched pinned Pele's whole SVG hashes, byte lengths, types and dimension bits. Before/after complete native model graph hashes matched, including IDs and string order. Extra public GraphWriter workloads matched repeated/unique fields, raw lone surrogates versus U+FFFD, existing IDs, empty/long names and cache overflow. Both locked release builds passed. Each build produced 84 allocation rows and **zero elapsed rows**.

**Benefits:** ER public-parse allocations fell 11.6%/16.7%/17.9% across small/medium/large; Railroad fell 9.8%/17.2%/19.1%. Medium ER removes 1,059 allocations and 15,425 requested bytes; large ER removes 4,707 allocations and 93,249 bytes. Both medium/large ER heap peaks stayed unchanged.

**Failed tradeoffs:** Flowchart, Sequence and Gantt use other arenas here, so none improved. Pie adds 8 allocations and 453 requested bytes at every scale (small allocations +13.3%, peak +297 bytes). All-unique synthetic fields add 70 allocations, 8,836 requested bytes and **4,704 peak bytes**: +20.2% small-workload peak, +4.2% medium, +1.0% large. Repeated 768-field construction removes 684 allocations (68.6%) but still adds 4,445 peak bytes (13.2%). Inline writer size grows 120 to 168 bytes. These regressions prevent treating fewer allocations as a universal win. No further candidate iteration or Root promotion is included.

## Reproduce

Prerequisites: existing repository history at the pinned commit, its development `node_modules/pele` artifact, and Rust toolchain from `scripts/env.sh`. No install occurs. No binaries are bundled.

```sh
/path/to/evidence/reproduce.sh /absolute/path/to/pele-rs
```

This creates an isolated detached worktree, applies the exact baseline and harness patches, verifies the oracle digest, builds/profiles baseline, applies only `runtime.patch`, then builds/profiles the prototype. It verifies exact outputs/graphs before comparing counters. Root files remain untouched; Git records the temporary worktree. Results remain there for review. Record field-name matrices are prepared outside counters; array-index strings and diagnostic long-key construction are included equally in both builds. Counters measure requested allocation bytes, not RSS or JavaScript heap use.

`--prepare-only` validates checkout/patch/oracle prerequisites without compiling. Full counter capture already ran in `/tmp/pele-perf-property-53c3z0gu`; the reproduction script's preparation path was checked separately, and its reconstructed baseline/prototype source digests exactly matched both measured builds. Existing raw reports/binaries are deliberately outside this evidence bundle. `evidence.json` keeps all 21 output/graph fingerprints and complete allocation deltas.

Any later timing needs a parent-approved quiet window, the same 100 ms warmup driver for both versions, negative controls and explicit unique-key latency. This allocation-only result does not close performance, Swift, browser or parity gates.
