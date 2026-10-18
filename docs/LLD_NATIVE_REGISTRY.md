# Owned native registry

Status: implemented from Astra architecture; verification recorded in
`FULL_PARITY_PROGRESS.md`. Target:
Pele 1.6.0, source `458322a5b8abb1ae57ddf127c32017ab9fb14a92`.

## Contract

The pinned core entry begins empty. `register` selects diagrams; the full entry
registers all 33. Existing C/Swift convenience parsers construct all built-ins for
every call. Add one owned registry that retains the existing Rust `PublicEngine`;
do not introduce another module system. Keep `Pele.Engine` and previous exports.

Rust adds `register_builtin(DiagramType)`, `registered(impl Into<DiagramId>)`,
`capabilities()`, `detect_type(&str)` and `supports(&str)` to `PublicEngine`.
Builtin installation shares the existing typed `register<D>` path with custom
providers; `with_builtins` shares this dispatch. Registration alone never changes
capability parity flags. Existing exact parse-limit and document operations remain
shared. Documents retain module/schema/config independently of their registry.

Names are exact canonical IDs: no case folding, source-keyword aliases or trimming.
Unknown membership queries return false. Unknown foreign `registerBuiltin` names
fail without mutation. Duplicate installation replaces later parses through the
existing registry machinery. There is no unregister operation.

Discovery preprocesses without parsing grammar or applying the default source
limit. Detection may throw on malformed front matter; supports catches that
failure and returns false. Registered but syntactically invalid Flowchart source
can be supported. A source over 50,000 UTF-16 units can detect/support while parse
rejects it. Empty-registry parse retains the upstream unsupported error, rather
than the convenience bridge's unavailable-codec diagnostic.

## Ownership and host interface

New C v8 `PeleRegistry` owns `Mutex<PublicEngine>`. Create mode 0 selects empty;
mode 1 selects all built-ins. Operations cover capabilities, membership,
register-builtin, detection, supports and parse-with-independent-limits. They use
existing owned version-2 result envelopes. Create/register return true; queries
return booleans; detection returns `{type: string-or-null}`. Parse publishes the
existing owned public document plus `{type, revision}` metadata.

Validate writable output pointers and set them null before fallible work inside
panic containment. Encode result metadata before publishing ownership. Free is
null-safe and must not race any operation. Use try-lock to reject busy/poisoned
handles. Source/option/ID byte budgets remain 16 MiB / 1 MiB / 4,096, checked before
borrowing. Transport rejection remains an error even for supports. Parse reuses
the v7 independent policy decoder and preserves all non-overridden JSON checks.

Swift `Pele.Registry` is a final class, deliberately not Sendable. Throwing static
factories `empty()` / `allBuiltins()` own an optional native handle. Methods mirror
the registry operations; `parseMutable(_:limits:options:)` returns the existing
`MutableDocument`. Reuse its owned-document decoder. An active/closed guard and
explicit idempotent `close()` mirror document ownership. Documents remain usable
after registry closure. The legacy published binary excludes `Registry.swift`.

## Acceptance and boundaries

Integration suites exercise empty, selective and all-builtins registries; all 33
canonical IDs; wrong case/aliases/unknown names; duplicates; separate owners;
complete parse errors; discovery distinctions; exact limit composition; and
retained document rendering after owner disposal. Rust additionally proves custom
provider replacement keeps the old document's registration/config/schema/methods.
C checks pointer ownership, null/unknown modes, byte guards, malformed UTF-8 and
repeated lifetimes. Swift proves the actual packaged native bridge and closed-owner
behavior. Use the pinned independent core/full oracle and record artifact hashes.

This layer closes selective native registry ownership. Arbitrary C callback
providers, live JavaScript Diagram-object changes, source-free constructors,
runtime `model.constructor`, callback overrides/coercion, lossless UTF-16
execution, browser lifecycle APIs and release gates remain separate.
