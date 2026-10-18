#include "pele.h"
#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static uint64_t bits(double value) {
    uint64_t result;
    _Static_assert(sizeof(value) == sizeof(result), "IEEE binary64 required");
    memcpy(&result, &value, sizeof(result));
    return result;
}
static void failure(PeleResult *result, uint32_t status, const char *message) {
    assert(result && pele_result_status(result) == status);
    size_t count = pele_result_len(result);
    char *text = malloc(count + 1); assert(text);
    memcpy(text, pele_result_data(result), count); text[count] = 0;
    assert(strstr(text, message));
    free(text); pele_result_free(result);
}
static void rejected(const char *source, uint32_t kind, uint64_t number, const char *message) {
    PelePublicDocument *document = (PelePublicDocument *)(uintptr_t)1;
    PeleResult *result = pele_public_parse_source_limit_v6(
        (const uint8_t *)source, strlen(source), NULL, 0, kind, number, &document);
    assert(document == NULL); failure(result, 2, message);
}
static void accepted(const char *source, uint32_t kind, uint64_t number) {
    /* A conflicting integer source cap proves the explicit threshold wins. */
    const char *options = "{\"limit\":-1.5,\"config\":{},\"maxEdges\":2}";
    PelePublicDocument *document = NULL;
    PeleResult *result = pele_public_parse_source_limit_v6(
        (const uint8_t *)source, strlen(source), (const uint8_t *)options,
        strlen(options), kind, number, &document);
    assert(result && pele_result_status(result) == 0 && document);
    pele_result_free(result);
    result = pele_public_snapshot_binary_v5(document);
    assert(result && pele_result_status(result) == 0 && pele_result_len(result) > 24);
    assert(!memcmp(pele_result_data(result), "PELGBIN1", 8));
    pele_result_free(result);
    result = pele_public_render_operation_binary_v5(document, 0, NULL, 0, NULL);
    assert(result && pele_result_status(result) == 0);
    const uint8_t *data = pele_result_data(result);
    size_t size = pele_result_len(result); assert(size >= 24);
    size_t metadata_size = (size_t)data[20] | (size_t)data[21] << 8 |
        (size_t)data[22] << 16 | (size_t)data[23] << 24;
    assert(metadata_size <= size - 24);
    char *metadata = malloc(metadata_size + 1); assert(metadata);
    memcpy(metadata, data + size - metadata_size, metadata_size); metadata[metadata_size] = 0;
    assert(strstr(metadata, "\"Ok\":") && strstr(metadata, "\"type\":\"info\"")); free(metadata);
    pele_result_free(result); pele_public_document_free_v4(document);
}
int main(void) {
    for (int i = 0; i < 16; ++i) {
        accepted("info", 0, 0);
        accepted("info", 2, UINT64_C(0x7ff8000000000042));
        accepted("info", 2, UINT64_C(0x7ff0000000000000));
        accepted("info", 2, bits(4.5));
    }
    rejected("info", 1, 0, "limit of null characters.");
    rejected("info", 2, bits(3.5), "limit of 3.5 characters.");
    rejected("info", 2, UINT64_C(0x8000000000000000), "limit of 0 characters.");
    rejected("", 2, bits(-1), "limit of -1 characters.");
    rejected("", 2, UINT64_C(0xfff0000000000000), "limit of -Infinity characters.");
    /* 9 UTF-16 units, 11 UTF-8 bytes; comments remain part of the comparison. */
    const char *unicode = "info\n%%\xf0\x9f\x98\x80";
    rejected(unicode, 2, bits(8.5), "limit of 8.5 characters.");
    accepted(unicode, 2, bits(9));

    PelePublicDocument *document = (PelePublicDocument *)(uintptr_t)1;
    failure(pele_public_parse_source_limit_v6((const uint8_t *)(uintptr_t)1,
        16 * 1024 * 1024 + 1, NULL, 0, 2, UINT64_C(0x7ff0000000000000), &document),
        2, "declared input exceeds budget");
    assert(document == NULL);
    failure(pele_public_parse_source_limit_v6((const uint8_t *)"info", 4,
        (const uint8_t *)(uintptr_t)1, 1024 * 1024 + 1, 2,
        UINT64_C(0x7ff8000000000000), &document), 2, "declared input exceeds budget");
    assert(document == NULL);
    failure(pele_public_parse_source_limit_v6((const uint8_t *)"info", 4,
        NULL, 0, 99, 0, &document), 1, "unknown source-limit kind");
    assert(document == NULL);
    failure(pele_public_parse_source_limit_v6(NULL, 1, NULL, 0, 0, 0, &document), 1, "null input");
    assert(document == NULL);
    failure(pele_public_parse_source_limit_v6((const uint8_t *)"info", 4,
        NULL, 0, 0, 0, NULL), 1, "null document output");
    accepted("info", 0, 0);
    puts("Exact source-limit C integration passed: ownership, IEEE values, UTF-16 and budgets");
}
