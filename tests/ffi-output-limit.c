#include "pele.h"
#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static uint64_t bits(double value) { uint64_t result; _Static_assert(sizeof(value) == sizeof(result), "IEEE binary64 required"); memcpy(&result, &value, sizeof(result)); return result; }
static void ok(PeleResult *result) { assert(result && pele_result_status(result) == 0); }
static void failure(PeleResult *result, uint32_t status, const char *message) {
    assert(result && pele_result_status(result) == status);
    size_t count = pele_result_len(result); char *text = malloc(count + 1); assert(text);
    memcpy(text, pele_result_data(result), count); text[count] = 0;
    assert(strstr(text, message)); free(text); pele_result_free(result);
}
static uint32_t little32(const uint8_t *p) { return (uint32_t)p[0] | (uint32_t)p[1] << 8 | (uint32_t)p[2] << 16 | (uint32_t)p[3] << 24; }
static uint64_t little64(const uint8_t *p) { uint64_t value = 0; for (unsigned i = 0; i < 8; ++i) value |= (uint64_t)p[i] << (8 * i); return value; }
static void outcome(PeleResult *result, uint64_t revision, const char *expected) {
    ok(result); const uint8_t *data = pele_result_data(result); size_t size = pele_result_len(result);
    assert(size >= 24 && !memcmp(data, "PELGBIN1", 8) && little64(data + 12) == revision);
    size_t count = little32(data + 20); assert(count <= size - 24);
    char *metadata = malloc(count + 1); assert(metadata);
    memcpy(metadata, data + size - count, count); metadata[count] = 0;
    assert(strstr(metadata, expected)); free(metadata); pele_result_free(result);
}
static PelePublicDocument *parsed(const char *source) {
    PelePublicDocument *document = NULL;
    PeleResult *result = pele_public_parse_v4((const uint8_t *)source, strlen(source), NULL, 0, &document);
    ok(result); assert(document); pele_result_free(result); return document;
}
static PeleResult *render(PelePublicDocument *document, uint64_t revision, uint32_t kind, uint64_t value) {
    /* Explicit v6 argument overrides any legacy field shape, while other options decode. */
    const char *options = "{\"outputLimit\":{\"ignored\":[-1.5,null]},\"responsive\":false}";
    return pele_public_render_operation_output_limit_v6(document, revision,
        (const uint8_t *)options, strlen(options), kind, value, NULL);
}
struct Context { PelePublicDocument *document; unsigned calls; int fail; };
static uint32_t width(void *raw, const uint8_t *text, size_t count, double size, uint32_t style,
    const uint8_t *font, size_t font_count, const uint8_t *mono, size_t mono_count, double *output) {
    (void)text; (void)font; (void)font_count; (void)mono; (void)mono_count;
    struct Context *context = raw; ++context->calls;
    failure(pele_public_snapshot_binary_v5(context->document), 1, "already in use");
    if (context->fail) return 1;
    *output = (double)count * size * ((style & 1) ? 0.71 : 0.6); return 0;
}
int main(void) {
    for (unsigned repeat = 0; repeat < 16; ++repeat) {
        PelePublicDocument *document = parsed("info title Output"); uint64_t revision = 0;
        outcome(render(document, revision, 0, 0), revision + 1, "\"Ok\":"); ++revision;
        outcome(render(document, revision, 1, 0), revision + 1, "\"Ok\":"); ++revision;
        outcome(render(document, revision, 2, UINT64_C(0x7ff8000000000042)), revision + 1, "\"Ok\":"); ++revision;
        outcome(render(document, revision, 2, UINT64_C(0x7ff0000000000000)), revision + 1, "\"Ok\":"); ++revision;
        outcome(render(document, revision, 2, bits(0.5)), revision + 1, "limit of 0.5 characters."); ++revision;
        outcome(render(document, revision, 2, UINT64_C(0x8000000000000000)), revision + 1, "limit of 0 characters."); ++revision;
        outcome(render(document, revision, 2, UINT64_C(0xfff0000000000000)), revision + 1, "limit of -Infinity characters."); ++revision;
        outcome(render(document, revision, 2, bits(-1)), revision + 1, "\"Err\":"); ++revision;
        failure(render(document, 0, 0, 0), 1, "Stale public model revision");
        failure(render(document, revision, 99, 0), 1, "unknown output-limit kind");
        failure(pele_public_render_operation_output_limit_v6(document, revision, (const uint8_t *)(uintptr_t)1,
            1024 * 1024 + 1, 2, UINT64_C(0x7ff0000000000000), NULL), 2, "declared input exceeds budget");
        failure(pele_public_render_operation_output_limit_v6(document, revision, NULL, 1, 0, 0, NULL), 1, "null input");
        const char *invalid = "{\"responsive\":{}}";
        failure(pele_public_render_operation_output_limit_v6(document, revision, (const uint8_t *)invalid,
            strlen(invalid), 0, 0, NULL), 1, "Invalid render options");
        /* Invalid calls and stale revisions never dispatch or advance state. */
        outcome(render(document, revision, 2, bits(1e20)), revision + 1, "\"Ok\":"); ++revision;
        pele_public_document_free_v4(document);
    }
    failure(render(NULL, 0, 0, 0), 1, "null document");
    const char *sources[] = {"classDiagram\nclass LongIdentifier {\n+String field\n}",
        "gantt\ndateFormat YYYY-MM-DD\nsection S\nA :a,2024-01-01,1d\nB :b,after a,2d"};
    for (unsigned i = 0; i < 2; ++i) {
        PelePublicDocument *document = parsed(sources[i]); struct Context context = {document, 0, 1};
        PeleHostResourcesV3 resources = {0}; resources.base.context = &context; resources.base.width = width;
        const char *options = "{\"now\":1704067200000}";
        outcome(pele_public_render_operation_output_limit_v6(document, 0, (const uint8_t *)options,
            strlen(options), 2, bits(0), &resources), 1, "Host text measurer failed");
        assert(context.calls == 1); context.fail = 0; context.calls = 0;
        outcome(pele_public_render_operation_output_limit_v6(document, 1, (const uint8_t *)options,
            strlen(options), 2, bits(0), &resources), 2, "limit of 0 characters.");
        assert(context.calls > 0);
        outcome(pele_public_render_operation_output_limit_v6(document, 2, (const uint8_t *)options,
            strlen(options), 0, 0, &resources), 3, "\"Ok\":");
        pele_public_document_free_v4(document);
    }
    pele_public_document_free_v4(NULL); pele_result_free(NULL);
    puts("Exact output-limit C integration passed: IEEE, owned outcomes, revisions, callbacks and budgets");
    return 0;
}
