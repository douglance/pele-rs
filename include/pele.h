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
/* NULL-safe accessors: status 3, data NULL, length 0. Status: 0 success,
 * 1 input error, 2 resource limit, 3 internal error. Bytes contain versioned JSON,
 * not a NUL-terminated string; data remains valid until result_free. */
uint32_t pele_result_status(const PeleResult *result);
const uint8_t *pele_result_data(const PeleResult *result);
size_t pele_result_len(const PeleResult *result);
void pele_result_free(PeleResult *result);
#ifdef __cplusplus
}
#endif
#endif
