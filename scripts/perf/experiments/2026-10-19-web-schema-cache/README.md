# Web schema-cache / compiler-profile experiment: HOLD

The default engine is unchanged. The patch caches static built-in schema views
per renderer and avoids repeated JSON comparison for the identical schema object.
It is an experiment, not an applied production patch. Apply only in an isolated
checkout/package; rebuild before testing.

Three final independent-VM conditions use the identical warmed driver, 21 public
workloads, two measurement resources, three API stages and nine alternating
trials per row. Every SVG and dimension matches the pinned Pele oracle before
and after timing. The speed condition additionally uses opt-level 3 instead of s.

The schema-cache candidate did not establish a reliable overall improvement;
several parse cases improved, several regressed, and the median changed only
slightly. The opt-level 3 condition increased raw WASM from 7,684,713 to 10,999,641
bytes without establishing a regression-free overall latency gain. Neither
candidate meets parity with Pele JS. Both remain out of the default build.

Raw final conditions and methodology:
[report](../../../../docs/benchmarks/2026-10-19-vm/README.md).
Early six-operation-only warmups showed JIT drift. Those exploratory files are
preserved in the task's remote workspace and excluded from these final results.
