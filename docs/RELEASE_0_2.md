# 0.2 source candidate and release gates

Status at source revision `fa07860`: Cargo and web manifests identify **0.2.0**,
but there is no published 0.2 tag, package or GitHub release. The published
v0.1.0 remains the legacy flowchart subset. This document describes implemented
source and outstanding acceptance, not a release announcement.

The target is the entire Pele 1.6.0 library at upstream revision
`458322a5b8abb1ae57ddf127c32017ab9fb14a92`: all 33 families and the inventoried
public APIs. The [parity inventory](PELE_PARITY.md) is the acceptance scope;
[the progress log](FULL_PARITY_PROGRESS.md) records revision-specific evidence.

## Implemented source

- All 33 Rust parsers/renderers and mutable family codecs, shared source
  preparation, layout/text/theme processing and SVG output.
- Typed immutable documents, editable owned graphs, schema-described methods,
  retained registration/config, extension IDs and independent registries.
- Low-level prepared parsers/renderers and explicit caller-graph constructors
  for all 33 families, without reconstructing source or injecting DB prototypes.
- C/Swift and WASM adapters for retained documents, model exchange, callbacks,
  precise render numbers and source/edge/output policies.
- Host-function binding, original-method delegation, partial mutation/error
  publication and scoped callback lifetimes. Gantt callback features have
  focused native, Swift and browser evidence. ClassMember live replacements
  and captured nested original calls are implemented with focused evidence.
- Browser mount/update/destroy and zoom, selective/lazy registries and custom
  providers. Built-ins share one WASM artifact.
- Native AppKit mount/update/destroy, zoom, presentation, link/accessibility
  controls, and owned font/approved raster resource adapters.
- Versioned 0.2 build/packaging scripts and a source-built downstream Swift
  consumer that exercises all 33 families and newer C13/C14 interfaces.

Adapters keep platform services outside the Rust diagram engine. No Kamui
runtime, JavaScript diagram engine, network image loader or GPU renderer is
required by the engine.

## Evidence completed before the final freeze

These are independent feature slices, not one passing final release matrix:

- The final consolidated Core run at `0c0c5a4` passes all 88 integration targets
  and 380 tests with zero failures or ignored tests, superseding the earlier
  failed complete run below. Source hashes match before and after execution.

- The final Class slice passed 17 Rust features, seven Swift features, strict C
  Class/Gantt integration and 30 browser checks across all three engines.
  Commits `0c0c5a4` and `fa07860` contain the implementation and feature tests.

- The consolidated Rust run executed all 87 integration targets: 376 tests
  passed and one C4 legacy error-boundary assertion failed. That defect is fixed;
  the 14-test C4/construction batch and restored-builtin raw recovery check pass.
  The original failure remains recorded in the progress log.
- A matching isolated ARM64 SDK passed 16 Swift prepared/construction,
  host-function and presentation integration features. They include 99 raw
  entry vectors and 67 caller-built model vectors across every family. Four
  later AgentFlow diagnostic vectors require the final matching run.
- Packed browser feature batches passed 105 distinct callback checks and 111
  checks after caller-built model coverage, across Chromium, Firefox and WebKit.
  A subsequent mono-font correction passed six targeted browser checks.
- Source-built downstream Swift consumption passed against an isolated ARM64
  release archive. Manifest checks distinguish local/new 0.2 APIs from legacy
  remote 0.1 behavior. This is not published-artifact consumption proof.
- Earlier active AppKit execution passed 26 interaction features. The expanded
  31-feature GUI suite has not completed: recent QA attempts remained inactive
  and executed no assertions. Those attempts are not passes.

Exact source/artifact hashes and logs are recorded in
[FULL_PARITY_PROGRESS.md](FULL_PARITY_PROGRESS.md) and the linked feature docs.
Older green matrices do not establish a later source revision's release status.

## Pending release acceptance

- [x] Complete ClassMember `getDisplayDetails` / `parseClassifier` live-callable
  behavior, including captured originals, nested calls, partial errors and
  native/Swift/packed-browser coverage.
- [x] Freeze final Core source and pass the combined Rust integration corpus: 88
  targets, 380 tests, zero failures/ignored at `0c0c5a4`. Formatting, immutable
  diff checks and unchanged source hashes also pass. The optimized validation
  profile uses LTO off/codegen16; distribution uses its normal release profile.
- [ ] Build one matching universal macOS arm64/x86_64 XCFramework, recording
  source, archive and header hashes.
- [ ] Execute strict C integrations and Swift CLI features on arm64 and actual
  x86_64/Rosetta, then run smoke and separate SwiftPM consumer checks.
- [ ] Execute all 31 native GUI features in an active, signed QA application;
  retain actual assertion and presentation evidence.
- [ ] Build the final WASM artifact and npm archive; execute the complete packed
  Chromium/Firefox/WebKit suite and TypeScript consumer checks against those
  exact bytes, with zero unexplained skips or failures.
- [ ] Verify crate packaging and standalone consumers, universal ZIP/checksums,
  and the generated versioned Swift manifest. Check downstream consumption
  again from the exact distribution artifacts.
- [ ] Record a current demo from the verified package and update README, release
  notes and PR evidence with final revision, artifacts and remaining boundaries.
- [ ] Publish the GitHub tag/release only after those gates pass; verify the
  published downloads, checksums and remote Swift/package consumers.

`scripts/package-release.sh` produces local artifacts and does not publish.
Running it while browser or Swift tests read their staging directories would
invalidate the evidence; packaging and verification must use stable artifacts.

## Compatibility boundaries carried into review

The compatibility target does not remove independent native graph, source,
layout, image or output work budgets. Source/config transport uses bounded
UTF-8/JSON; graph transport preserves UTF-16 code units and IEEE number bits.
Arbitrary JavaScript VM behavior, custom property interpreters and all
undeclared helper overrides are not provided. Explicit missing-helper failures
of ordinary caller-authored models are preserved at the relevant operation.

See [caller-authored model behavior](DECLARED_MODEL_CONSTRUCTION.md),
[VM diagnostics](WEB_VM_DIAGNOSTICS.md),
[extreme corpus boundaries](WEB_LEGACY_CORPUS.md),
[source limits](SOURCE_LIMITS.md), [parse limits](PARSE_LIMITS.md) and
[output limits](OUTPUT_LIMITS.md). Do not describe finite corpus coverage as
unrestricted JavaScript or Mermaid compatibility.

No performance improvement over Pele JavaScript is claimed. GPU acceleration is
not implemented; [existing benchmark evidence](PERFORMANCE.md) remains scoped
to its recorded revisions and workloads.
