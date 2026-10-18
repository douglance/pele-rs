# First matched local baseline — 2026-10-18

The 21 generated small/medium/large cases produced identical whole SVG hashes, byte lengths, types and dimension bits through pinned Pele and Rust source, retained typed, and mutable publication render APIs. Three medium cases were then measured in a coordinated 14.9-second window: 54 native stage rows, 24 paired JS rows and 54 separately instrumented allocation rows. All post-run outputs still matched. This is performance evidence, not an additional parity certification.

Raw data: [2026-10-18-first-native-js.json](2026-10-18-first-native-js.json). Source/input digest `e86842e8b71deb47586a6234059defc586c7fb483cd5cb1f66a9f2319fcfc8e5`; uninstrumented binary `ac56915229fbfc5c886927478d77e7a85f0de7bc8673290fe2560ba2ab7b0ac6`. Git HEAD was `8783a22108c0d91722077f3f81016f8f503f009d` with recorded worktree changes; the input digest, rather than that commit alone, identifies the built runtime.

No Rust/Swift build or corpus job ran during samples. The shared host was not idle: initial load average 5.91/6.47/7.10, final 5.42/6.33/7.04; WindowServer ~42–44%, Chrome helper ~18–25%, unrelated xctest ~0.5–1%. No unrelated process was stopped. Node 25.9.0 and optimized native Rust ran on the same host; full machine/toolchain/provenance appears in the raw report.

## Observed API costs

Medians below are milliseconds per operation, from seven alternating trials. Each mutable batch began from fresh parsed public state plus one warm render outside timing. Native and JS batch sizes were independently calibrated. These are exploratory local observations.

| Family, 48 units | Typed parse, Rust / JS | Public parse, Rust / JS | Retained render, Rust / JS | Mutable core, Rust / JS | Mutable publication, Rust / JS |
|---|---:|---:|---:|---:|---:|
| Flowchart | .0957 / .2039 | .4990 / .1159 | .3374 / .4065 | .3330 / .3651 | .3715 / .3675 |
| ER | .1716 / .1364 | .4297 / .1268 | .8754 / .9461 | .8501 / .9074 | .9060 / .7223 |
| Gantt | .0765 / .1130 | .3682 / .0475 | .2517 / .2022 | .7936 / .2013 | .8941 / .1993 |

Public parse additionally constructs the identity arena and native mutable state; JS parse returns its raw model directly. Mutable publication additionally validates/clones graph state and returns a revision. These columns compare actual API contracts, not equal-cost parser internals. Gantt mutable core performs scheduling and graph mutation directly; it cannot be inferred from the immutable render cost.

The initial run used six one-operation warmups before calibration. Repeated JS reference stages changed as V8 warmed (for example, Flowchart parse .2039 → .1159 ms). Bootstrap intervals cannot remove that systematic warmup drift. Ratios from this first run should not support a general speed claim. The frozen driver now adds at least 100 ms of measured work per runtime/stage before calibration, records warmup operations and elapsed work, and retains `--warmup-ms 0` for the initial six-warmup behavior. A future comparison must use the same driver/settings for both versions.

## Allocation and transport attribution

Public registry construction measured .0064–.0065 ms with 173 allocations requesting 12,073 bytes. Fresh-registry public parse added exactly those 173 allocations and 12,073 bytes in each family. Registry cost is only around 1–2% of these medium public parses; caching it is not the priority here.

| Warm single operation | Flowchart allocations / requested bytes | ER allocations / requested bytes | Gantt allocations / requested bytes |
|---|---:|---:|---:|
| Typed parse | 2,026 / 204,699 | 2,069 / 192,835 | 1,103 / 93,583 |
| Public parse | 5,961 / 665,327 | 6,359 / 469,486 | 3,471 / 499,645 |
| Retained typed render | 4,840 / 805,610 | 11,723 / 1,852,233 | 2,805 / 301,483 |
| Mutable core render | 4,840 / 805,610 | 11,723 / 1,852,233 | 6,421 / 572,767 |
| Mutable publication | 5,628 / 917,390 | 12,813 / 1,980,009 | 7,292 / 708,623 |
| Graph snapshot | 394 / 55,890 | 545 / 63,888 | 435 / 65,072 |

Flowchart publication adds exactly two snapshots' 788 allocations and 111,780 requested bytes. ER adds exactly two snapshots' 1,090 allocations and 127,776 bytes. This agrees with preflight/post-operation snapshots in the implementation. It does not authorize dropping preflight validation or changing error/callback ordering.

| Native transport component | Flowchart | ER | Gantt |
|---|---:|---:|---:|
| Graph JSON roundtrip, ms | .4920 | .5749 | .5113 |
| Graph binary encode, ms | .0119 | .0137 | .0116 |
| Graph binary decode, ms | .0248 | .0314 | .0256 |
| JSON / binary bytes | 79,779 / 17,098 | 92,001 / 18,382 | 82,916 / 17,187 |

Binary encode plus decode is much cheaper here than JSON encode plus decode, with roughly one fifth the bytes. These are production codec components, not measured end-to-end Swift/C bridge calls; no bridge speedup is claimed.

## Gantt history boundary and next bounded proposal

The 48-task Gantt arena began each measured batch with 254 nodes after its untimed warm render. Sixteen mutable renders ended at 1,838 nodes: 99 new nodes per render. Both mutable-core and publication batches showed this same growth. This preserves new Date identities and previous arena IDs. JS retained its normal model, without a transported historical identity arena. The .7936/.8941 ms numbers average that specific 16-render history; they are not first-render latency or an indefinitely steady retained-document cost. Allocation profiles separately count one operation after the warm render, not the 16-operation average. A later history campaign should explicitly compare first operation and fixed 1/16/128-render histories with resets, while retaining exact graph/Date checks.

A bounded candidate is a **per-operation positive field-name-ID cache in Gantt's `Context::own`**. Every read currently constructs a temporary UTF-16 `JsString` before looking up an interned property name. Scheduling and typed import repeatedly read the same finite set of task fields. Cache successful `&str` → existing string-ID resolutions for that Context and use borrowed UTF-8 cache lookups afterward. Do not cache misses: a later operation can intern a formerly absent key. Keep graph string order, canonical IDs, descriptor lookup, work-budget ticks and error ordering exactly as before. Context lifetime remains one operation, so `apply` and external graph replacements cannot leave a stale cache.

This candidate targets Gantt mutable methods/render only. It does not change scheduling, Date creation, graph retention, typed geometry, any shared engine API, or other families. The allocation excess of mutable core over typed render is 3,616 allocations/271,284 requested bytes per warmed operation; the cache is a hypothesis about a portion of that excess, not measured attribution to one function yet. Success requires direct before/after allocation reduction, duration-warmed samples at fixed history lengths, all Gantt public graph/method/partial-error tests and the entire 18,994-case historical corpus. No runtime optimization is included in this harness slice.
