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
