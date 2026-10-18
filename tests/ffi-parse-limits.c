#include "pele.h"
#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static uint64_t bits(double value) { uint64_t result; _Static_assert(sizeof(value) == sizeof(result), "IEEE binary64 required"); memcpy(&result, &value, sizeof(result)); return result; }
static void failure(PeleResult *result, uint32_t status, const char *expected) {
    assert(result && pele_result_status(result) == status);
    size_t count = pele_result_len(result); char *text = malloc(count + 1); assert(text);
    memcpy(text, pele_result_data(result), count); text[count] = 0;
    assert(strstr(text, expected)); free(text); pele_result_free(result);
}
static PeleResult *parse(const char *source, const char *options, uint32_t source_kind,
                        uint64_t source_bits, uint32_t edge_kind, uint64_t edge_bits,
                        PelePublicDocument **document) {
    return pele_public_parse_limits_v7((const uint8_t *)source, strlen(source),
        (const uint8_t *)options, options ? strlen(options) : 0,
        source_kind, source_bits, edge_kind, edge_bits, document);
}
static void accepted(const char *source, const char *options, uint32_t source_kind,
                     uint64_t source_bits, uint32_t edge_kind, uint64_t edge_bits,
                     const char *retained_bits) {
    PelePublicDocument *document = NULL;
    PeleResult *result = parse(source, options, source_kind, source_bits, edge_kind, edge_bits, &document);
    assert(result && pele_result_status(result) == 0 && document); pele_result_free(result);
    result = pele_public_snapshot_v4(document); assert(result && pele_result_status(result) == 0);
    if (retained_bits) {
        size_t count = pele_result_len(result); char *text = malloc(count + 1); assert(text);
        memcpy(text, pele_result_data(result), count); text[count] = 0;
        assert(strstr(text, retained_bits)); free(text);
    }
    pele_result_free(result);
    result = pele_public_render_operation_binary_v5(document, 0, NULL, 0, NULL);
    assert(result && pele_result_status(result) == 0 && pele_result_len(result) > 24);
    assert(!memcmp(pele_result_data(result), "PELGBIN1", 8));
    pele_result_free(result); pele_public_document_free_v4(document);
}
static void rejected(const char *source, const char *options, uint32_t source_kind,
                     uint64_t source_bits, uint32_t edge_kind, uint64_t edge_bits,
                     uint32_t status, const char *expected) {
    PelePublicDocument *document = (PelePublicDocument *)(uintptr_t)1;
    PeleResult *result = parse(source, options, source_kind, source_bits, edge_kind, edge_bits, &document);
    assert(document == NULL); failure(result, status, expected);
}
int main(void) {
    const char *chain = "flowchart TD\nA --> B --> C";
    const char *overridden = "{\"limit\":{\"ignored\":[null,-1.5]},\"maxEdges\":[-1.5],\"config\":{\"retained\":{\"flag\":\"kept\"}}}";
    for (unsigned i = 0; i < 16; ++i) {
        accepted(chain, overridden, 0, 0, 0, 0, NULL);
        accepted(chain, overridden, 2, UINT64_C(0x7ff8000000000042), 2,
                 UINT64_C(0x7ff8000000000042), "7ff8000000000042");
        accepted(chain, overridden, 2, UINT64_C(0x7ff0000000000000), 2,
                 bits(1.5), "3ff8000000000000");
        accepted(chain, NULL, 3, 0, 2, UINT64_C(0x7ff0000000000000), "7ff0000000000000");
    }
    rejected(chain, NULL, 0, 0, 1, 0, 2, "limit is null");
    rejected(chain, NULL, 0, 0, 2, UINT64_C(0x8000000000000000), 2, "limit is 0");
    rejected(chain, NULL, 0, 0, 2, bits(1), 2, "1 edges found");
    rejected(chain, NULL, 0, 0, 2, UINT64_C(0xfff0000000000000), 2, "limit is -Infinity");
    /* Source failure precedes syntax and edge accounting. */
    rejected("flowchart TD\nA --> B", NULL, 1, 0, 2, bits(-1), 2, "source is longer");
    rejected(chain, "{\"limit\":-1}", 3, 0, 0, 0, 1, "Invalid parse options");
    rejected(chain, "{\"maxEdges\":-1}", 0, 0, 3, 0, 1, "Invalid parse options");
    accepted(chain, "{\"limit\":-1,\"maxEdges\":5000}", 0, 0, 3, 0, NULL);
    accepted(chain, "{\"limit\":50000,\"maxEdges\":-1}", 3, 0, 0, 0, NULL);
    rejected(chain, "{\"maxEdges\":-1}", 3, 0, 3, 0, 1, "Invalid parse options");
    rejected(chain, "{\"config\":[]}", 0, 0, 0, 0, 1, "Invalid parse options");
    rejected(chain, NULL, 99, 0, 0, 0, 1, "unknown source-limit kind");
    rejected(chain, NULL, 0, 0, 99, 0, 1, "unknown edge-limit kind");
    PelePublicDocument *document = (PelePublicDocument *)(uintptr_t)1;
    failure(pele_public_parse_limits_v7((const uint8_t *)(uintptr_t)1, 16 * 1024 * 1024 + 1,
        NULL, 0, 2, UINT64_C(0x7ff8000000000042), 2, UINT64_C(0x7ff0000000000000), &document), 2, "declared input exceeds budget");
    assert(document == NULL);
    failure(pele_public_parse_limits_v7((const uint8_t *)chain, strlen(chain),
        (const uint8_t *)(uintptr_t)1, 1024 * 1024 + 1, 0, 0, 0, 0, &document), 2, "declared input exceeds budget");
    assert(document == NULL);
    failure(pele_public_parse_limits_v7(NULL, 1, NULL, 0, 0, 0, 0, 0, &document), 1, "null input");
    assert(document == NULL);
    failure(parse(chain, NULL, 0, 0, 0, 0, NULL), 1, "null document output");
    pele_public_document_free_v4(NULL); pele_result_free(NULL);
    puts("Exact parse-limits C integration passed: independent overrides, IEEE, ownership, precedence and budgets");
    return 0;
}
