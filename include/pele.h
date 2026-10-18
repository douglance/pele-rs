#ifndef PELE_H
#define PELE_H
#include <stddef.h>
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
#define PELE_ABI_VERSION 1
typedef struct PeleResult PeleResult;
uint32_t pele_abi_version(void);
/* Synchronous borrow of readable input bytes. NULL is allowed only with length 0.
 * Limits are checked before borrowing. UTF-8 only; empty options selects defaults.
 * The returned immutable result must be freed exactly once, including errors. */
PeleResult *pele_render(const uint8_t *source, size_t source_len,
                        const uint8_t *options, size_t options_len);
/* Compatibility source preparation; result envelope version 2. Does not render.
 * Transport budgets: 16 MiB source / 1 MiB options, checked before borrowing.
 * Default source limit is 50,000 UTF-16 units, configurable in options JSON. */
PeleResult *pele_inspect(const uint8_t *source, size_t source_len,
                         const uint8_t *options, size_t options_len);
/* Owned version-2 capability report. Known types and installed renderers are separate. */
PeleResult *pele_capabilities(void);
/* Version-2 built-in parse/render. parse result retains a document until result_free.
 * render_document borrows it synchronously; source/options are copied into owned models.
 * A render/error result cannot serve as a document. Transport budgets match inspect. */
PeleResult *pele_parse_v2(const uint8_t *source, size_t source_len, const uint8_t *options, size_t options_len);
PeleResult *pele_render_v2(const uint8_t *source, size_t source_len, const uint8_t *options, size_t options_len);
PeleResult *pele_render_document_v2(const PeleResult *document, const uint8_t *options, size_t options_len);
PeleResult *pele_builtin_capabilities(void);
/* Read-only retained model operation, UTF-8 name <=4096 bytes and JSON args <=1 MiB. */
PeleResult *pele_document_query_v2(const PeleResult *, const uint8_t *, size_t, const uint8_t *, size_t);
/* Per-render resources. Callback status: 0 success, nonzero failure.
 * Icons are trusted host SVG, never source strings. Returned bytes are borrowed
 * until the next callback; Rust copies them immediately. All callbacks/context
 * are borrowed synchronously and never retained. NULL callback selects fallback. */
typedef struct PeleBorrowedBytes { const uint8_t *data; size_t len; } PeleBorrowedBytes;
typedef struct PeleHostResources {
    void *context;
    uint32_t (*width)(void *, const uint8_t *, size_t, double, uint32_t,
                      const uint8_t *, size_t, const uint8_t *, size_t, double *);
    uint32_t (*icon)(void *, const uint8_t *, size_t, PeleBorrowedBytes *);
    uint32_t (*now)(void *, double *);
} PeleHostResources;
PeleResult *pele_render_with_resources_v2(const uint8_t *, size_t, const uint8_t *, size_t, const PeleHostResources *);
PeleResult *pele_render_document_with_resources_v2(const PeleResult *, const uint8_t *, size_t, const PeleHostResources *);
/* Calendar extension preserves the v2 resource struct ABI. Operation 0 takes a
 * JSON millisecond instant, returning CalendarFields or null. Operation 1 takes
 * {civil:{year,month,day,hour,minute,second,millisecond},utcAnchorMillis:number|null},
 * returning the local millisecond instant or null. Month=1..12, weekday=0 Sunday,
 * utcOffsetMinutes is east-positive. Civil fields are normalized; resolve folds
 * to the earliest instant and gaps forward by their duration, as JavaScript Date.
 * The optional date_parser callback uses operation 2, takes a date string and returns its parsed instant or null.
 * Optional utc_offset receives an instant in milliseconds and writes integer offset
 * seconds east of UTC; returns zero on success. Rust resolves proleptic civil dates and DST gaps/folds.
 * Explicit calendar/calendar_instant callbacks take precedence over utc_offset.
 * Calendar output is borrowed until next callback; NULL/0 also means null. */
typedef struct PeleHostResourcesV3 {
    PeleHostResources base;
    uint32_t (*calendar)(void *, uint32_t, const uint8_t *, size_t, PeleBorrowedBytes *);
    uint32_t (*calendar_instant)(void *, uint32_t, const uint8_t *, size_t, PeleBorrowedBytes *);
    uint32_t (*date_parser)(void *, uint32_t, const uint8_t *, size_t, PeleBorrowedBytes *);
    uint32_t (*utc_offset)(void *, double, int32_t *);
} PeleHostResourcesV3;
PeleResult *pele_render_with_resources_v3(const uint8_t *, size_t, const uint8_t *, size_t, const PeleHostResourcesV3 *);
PeleResult *pele_render_document_with_resources_v3(const PeleResult *, const uint8_t *, size_t, const PeleHostResourcesV3 *);
/* NULL-safe accessors: status 3, data NULL, length 0. Status: 0 success,
 * 1 input error, 2 resource limit, 3 internal error. Bytes contain versioned JSON,
 * not a NUL-terminated string; data remains valid until result_free. */
/* Mutable public documents: synchronize destruction with all operations.
 * Graph JSON preserves UTF-16 code units, IEEE number bits and object identity.
 * Operation results use existing owned version-2 envelopes. Revisions are decimal
 * strings for snapshot/apply, uint64 JSON integers for invocation/render outcomes.
 * Expected revisions accept exact uint64 values. Graph input <=64 MiB.
 * NULL resources select default native resources. Parse sets *out_document=NULL
 * before work; a successful handle must be freed once. Recursive/concurrent
 * operations on one handle return an error; never free during an active call. */
typedef struct PelePublicDocument PelePublicDocument;
/* Owned selective registry. mode0 empty, mode1 all33 built-ins; all successful
 * results use owned version2 envelopes. create writes NULL before any failure
 * and publishes ownership only after result encoding. Free once after all calls
 * return; free(NULL) is safe. Concurrent/reentrant operations return busy rather
 * than blocking. Registries and documents have independent lifetimes: returned
 * documents retain their module/config/state after registration replacement/free.
 * Exact built-in IDs only for registration; duplicate registration replaces its
 * handler without adding a capability row. IDs are UTF-8 <=4096 bytes.
 * Discovery preprocesses source <=16MiB without parse thresholds or grammar
 * validation. detect returns {type:string|null}; preprocessing errors propagate.
 * supports returns bool, false on preprocessing failure or missing registration;
 * invalid UTF-8/native transport failures remain errors. capabilities includes
 * every known built-in even when unregistered. NULL registry operations fail.
 * Synchronize free with all operations; no registry pointer is retained by docs. */
typedef struct PeleRegistry PeleRegistry;
PeleResult *pele_registry_create_v8(uint32_t mode, PeleRegistry **out_registry);
void pele_registry_free_v8(PeleRegistry *registry);
PeleResult *pele_registry_capabilities_v8(const PeleRegistry *registry);
PeleResult *pele_registry_registered_v8(const PeleRegistry *registry, const uint8_t *id, size_t id_len);
PeleResult *pele_registry_register_builtin_v8(PeleRegistry *registry, const uint8_t *id, size_t id_len);
PeleResult *pele_registry_detect_type_v8(const PeleRegistry *registry, const uint8_t *source, size_t source_len);
PeleResult *pele_registry_supports_v8(const PeleRegistry *registry, const uint8_t *source, size_t source_len);
/* Same independent numeric policy tags as parse_limits_v7:0 undefined,1 null,
 * 2 raw IEEE-754 bits,3 legacy integer option. Source16MiB/options1MiB budgets
 * always apply. Only explicitly overridden fields bypass old integer decoding.
 * *out_document is NULL on every error. Missing-registration errors retain core
 * type/message. Live documents use the existing public document operations. */
PeleResult *pele_registry_parse_limits_v8(const PeleRegistry *registry,
    const uint8_t *source, size_t source_len, const uint8_t *options, size_t options_len,
    uint32_t source_kind, uint64_t source_bits, uint32_t edge_kind, uint64_t edge_bits,
    PelePublicDocument **out_document);
PeleResult *pele_public_parse_v4(const uint8_t *, size_t, const uint8_t *, size_t, PelePublicDocument **out_document);
/* Additive exact source-limit lane for well-formed UTF-8 source. limit_kind:
 * 0 undefined (default 50,000 UTF-16 units), 1 null, 2 IEEE-754 number bits.
 * Overrides only options.limit; config/maxEdges retain their existing contract.
 * NaN/infinity never disable the independent 16 MiB source-byte budget.
 * Ownership/error rules match parse_v4; unknown kinds fail without a handle. */
PeleResult *pele_public_parse_source_limit_v6(const uint8_t *, size_t,
                                           const uint8_t *, size_t,
                                           uint32_t limit_kind, uint64_t limit_bits,
                                           PelePublicDocument **out_document);
/* Independent parse policies. Each kind: 0 undefined, 1 null, 2 IEEE-754 bits,
 * 3 legacy options field (bits ignored). Explicit source overrides only limit;
 * explicit edge overrides only maxEdges. Non-overridden fields retain their
 * existing integer decoder, including errors. Undefined uses defaults50,000
 * UTF-16 source units/5,000 edges; null compares as0. Actual edge accounting is
 * diagram-specific. Native source16MiB/options1MiB budgets always remain active.
 * Source checks precede diagram parsing/edge checks. Parse_v4 ownership applies;
 * every failure leaves *out_document=NULL. Unknown tags reject without a handle. */
PeleResult *pele_public_parse_limits_v7(const uint8_t *source, size_t source_len,
    const uint8_t *options, size_t options_len, uint32_t source_kind,
    uint64_t source_bits, uint32_t edge_kind, uint64_t edge_bits,
    PelePublicDocument **out_document);
PeleResult *pele_public_snapshot_v4(const PelePublicDocument *);
/* Native schema and method operations. invoke returns an operation outcome with
 * graph/revision/result even when the method throws, subject to output budgets.
 * Outer errors include preflight rejection and export/transport failure after
 * dispatch; state may advance in the latter case. call JSON is bounded to 64 MiB. */
PeleResult *pele_public_schema_v4(const PelePublicDocument *document);
PeleResult *pele_public_invoke_v4(PelePublicDocument *document, uint64_t expected_revision,
                                const uint8_t *call, size_t call_length,
                                const PeleHostResourcesV3 *resources);
PeleResult *pele_public_apply_v4(PelePublicDocument *, uint64_t expected_revision, const uint8_t *, size_t);
PeleResult *pele_public_render_v4(PelePublicDocument *, const uint8_t *, size_t, const PeleHostResourcesV3 *);
/* Publishes post-render graph/revision on success or operation error. Outer
 * export/transport failure may follow mutation, as for invoke_v4. */
PeleResult *pele_public_render_operation_v4(PelePublicDocument *, uint64_t expected_revision,
                                          const uint8_t *, size_t, const PeleHostResourcesV3 *);
/* Binary graph schema1: PELGBIN1, LE version/revision/metadata length,
 * root/string/node tables, trailing JSON metadata. Each result is owned/freed
 * exactly as v4. A nonzero status carries the existing JSON error envelope. */
PeleResult *pele_public_snapshot_binary_v5(const PelePublicDocument *document);
PeleResult *pele_public_apply_binary_v5(PelePublicDocument *document, const uint8_t *graph, size_t graph_len);
PeleResult *pele_public_invoke_binary_v5(PelePublicDocument *document, uint64_t expected_revision, const uint8_t *call, size_t call_len, const PeleHostResourcesV3 *resources);
PeleResult *pele_public_render_operation_binary_v5(PelePublicDocument *document, uint64_t expected_revision, const uint8_t *options, size_t options_len, const PeleHostResourcesV3 *resources);
/* Exact output-limit lane, binary graph outcome as v5. limit_kind: 0 undefined,
 * 1 null (both default 4,000,000 UTF-16 units), 2 raw IEEE-754 number bits.
 * Overrides only options.outputLimit; other options retain existing decoding.
 * Rendering and host callbacks run before output comparison; operation errors
 * retain post-call graph/revision. Stale revisions reject before dispatch.
 * Independent returned SVG ceiling:64 MiB UTF-8, including with NaN/infinity.
 * This bounds accepted output, not all peak allocations inside renderers.
 * Options <=1 MiB, complete binary exchange <=64 MiB; outer export errors may
 * follow state mutation. Unknown kinds reject before dispatch. Ownership and
 * synchronous host callback rules match render_operation_binary_v5. */
PeleResult *pele_public_render_operation_output_limit_v6(PelePublicDocument *document,
    uint64_t expected_revision, const uint8_t *options, size_t options_len,
    uint32_t limit_kind, uint64_t limit_bits, const PeleHostResourcesV3 *resources);
void pele_public_document_free_v4(PelePublicDocument *);

uint32_t pele_result_status(const PeleResult *result);
const uint8_t *pele_result_data(const PeleResult *result);
size_t pele_result_len(const PeleResult *result);
void pele_result_free(PeleResult *result);
#ifdef __cplusplus
}
#endif
#endif
