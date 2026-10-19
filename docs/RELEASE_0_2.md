# 0.2 release acceptance

0.2 ports all 33 Pele 1.6.0 diagram families and the inventoried native/web
interfaces through the independent Rust engine. Runtime implementation is frozen
at `0c0c5a4` / `fa07860`; later commits cover verification, packaging, notices,
demo and documentation. Publication remains gated on final distribution checks.
This document does not claim the GitHub tag or downloads already exist.

Oracle: upstream revision `458322a5b8abb1ae57ddf127c32017ab9fb14a92`.
The [parity inventory](PELE_PARITY.md) defines scope; the
[verification checkpoint](FULL_PARITY_PROGRESS.md) retains evidence and history.

## Delivered interfaces

- All 33 Rust parsers/renderers and mutable model codecs; shared source
  preparation, layout, text/theme processing and SVG output.
- Typed immutable documents, editable owned graphs, schema-described methods,
  retained handlers/config, extension IDs and independent registries.
- All 33 low-level prepared parsers/renderers and explicit caller-graph
  constructors, without reconstructing source or injecting DB prototypes.
- C/Swift and WASM adapters, precise render numbers and source/edge/output
  policies, host resources and scoped host-function binding.
- Gantt and ClassMember callable replacements, captured original delegation,
  nested calls, partial mutation/error publication and recovery.
- Browser mount/update/destroy, zoom, selective/lazy registries and custom
  providers; all built-ins share one WASM artifact.
- Native AppKit mount/update/destroy, zoom, link/accessibility controls and
  bounded native fonts/explicitly approved raster resources.
- Universal macOS arm64/x86_64 packaging and retained Rust dependency,
  generated-glue and standard-library/runtime notices (`415a04a`).

## Completed runtime verification

| Lane | Verified result |
| --- | --- |
| Rust | 380 tests across all 88 integration targets; zero failures/ignored |
| Native C | 14 strict integrations per architecture; 28 actual executions |
| Swift CLI | 88 features per architecture; 176 actual executions |
| Native rendering | All 33 families render and paint pixels on arm64 and x86_64/Rosetta |
| SwiftPM consumers | Independent local consumers pass on both architectures |
| AppKit | 31/31 features in the active signed QA app, zero failures |
| Packed browser | 285 unique passing checks: 95 Chromium, 95 Firefox, 95 WebKit |
| TypeScript | Independent consumer resolves exact extracted archive declarations |

The Rust corpus at `0c0c5a4` used release opt-level 3, LTO off, codegen16, two
jobs/test threads. Distribution artifacts use the normal release profile.
Native evidence preserves 502 source/SDK inputs unchanged. Intel SwiftPM test
discovery failed under Rosetta; all 25 non-GUI classes then executed through the
actual x86_64 XCTest runner, without product changes or repeating C coverage.

The browser result is **not one uninterrupted 285-check run**. The initial run
passed 284 with zero skips/flaky retries. macOS entered low-power sleep for
17,654 seconds (4h54m14s) during WebKit's Class corpus. That same check then passed
in 21,758 ms against unchanged source/archive, with unchanged assertions and
120,000 ms timeout. Original results and OS sleep evidence remain preserved;
all 285 distinct checks have passing evidence. The packed tree, archive entries,
three WASM copies and 33 attached diagnostic ledgers were verified exact.

The final GUI harness confirmation after `95d33ae` passes all four suites:
Mount 10, Zoom 9, Presentation 7, NativeResource 5. Run `run-scu8m8og` records
active PID 88918 and 31 executed features with zero failures. Earlier
inactive, zero-assertion attempts are retained as failed attempts, not passes.
Synthetic magnification and AX controls do not establish physical-trackpad or
spoken VoiceOver behavior.

## Final distribution/publication gates

- [x] Complete native/web Class callable slice and final Rust corpus.
- [x] Build matching universal SDK and execute native C/Swift, pixel and consumer lanes.
- [x] Execute all 31 AppKit features in an active signed application.
- [x] Complete the packed three-browser/TypeScript matrix with the preserved sleep account.
- [x] Retain production dependency and Rust runtime notices in distribution packaging.
- [x] Verify exact universal ZIP consumers on arm64 and x86_64/Rosetta.
- [x] Verify final web tarball: 104 entries, only NOTICE differs from browser acceptance;
  TypeScript plus 109 exact-final-WASM checks pass.
- [ ] Complete final crate/checksum and published remote distribution checks.
- [x] Record the full-library demo: 114 actual browser frames, 19 seconds, from the verified packed artifact.
- [ ] Publish the v0.2.0 tag/release, then verify actual downloads/checksums and remote consumers.

Runtime acceptance and published-artifact consumption are separate proof.
`scripts/package-release.sh` builds local artifacts; it does not publish.
Repacking notices/docs changes archive hashes and must preserve accepted
executable bytes; final consumers use the actual final distribution.

## Compatibility boundaries

Native graph/source/layout/image/output work budgets remain enforced. Source and
config use bounded UTF-8/JSON; mutable graph transport preserves UTF-16 code units
and IEEE number bits. Arbitrary JavaScript VM behavior, custom property
interpreters and every undeclared helper override are outside this contract.
Caller-authored models preserve upstream missing-helper errors at their actual
operation boundary. Corpus coverage does not imply unrestricted JavaScript or
Mermaid compatibility.

See [construction](DECLARED_MODEL_CONSTRUCTION.md),
[VM diagnostics](WEB_VM_DIAGNOSTICS.md),
[extreme corpus boundaries](WEB_LEGACY_CORPUS.md),
[source limits](SOURCE_LIMITS.md), [parse limits](PARSE_LIMITS.md) and
[output limits](OUTPUT_LIMITS.md). No speedup over Pele JavaScript is claimed;
GPU acceleration is not implemented. [Performance evidence](PERFORMANCE.md)
remains scoped to its recorded revisions/workloads.
