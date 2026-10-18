#include "pele.h"
#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static char *text(PeleResult *result) {
    size_t count = pele_result_len(result);
    char *copy = malloc(count + 1); assert(copy);
    memcpy(copy, pele_result_data(result), count); copy[count] = 0; return copy;
}
static void ok(PeleResult *result) { assert(result && pele_result_status(result) == 0); }
static void failure(PeleResult *result, uint32_t status, const char *message) {
    assert(result && pele_result_status(result) == status);
    char *body = text(result); assert(strstr(body, message)); free(body); pele_result_free(result);
}
struct Context { PelePublicDocument *document; int calls; int fail; };
static uint32_t width(void *raw, const uint8_t *source, size_t length, double size, uint32_t style,
                      const uint8_t *font, size_t font_length, const uint8_t *mono, size_t mono_length,
                      double *output) {
    (void)source; (void)style; (void)font; (void)font_length; (void)mono; (void)mono_length;
    struct Context *context = raw; context->calls++;
    failure(pele_public_snapshot_v4(context->document), 1, "already in use");
    if (context->fail) return 1;
    *output = (double)length * size * 0.6; return 0;
}
int main(void) {
    PelePublicDocument *document = (PelePublicDocument *)(uintptr_t)1;
    failure(pele_public_parse_v4(NULL, 1, NULL, 0, &document), 1, "null input"); assert(document == NULL);
    failure(pele_public_parse_v4((const uint8_t *)"info", 4, NULL, 0, NULL), 1, "null document output");
    failure(pele_public_snapshot_v4(NULL), 1, "null document");
    failure(pele_public_schema_v4(NULL), 1, "null document");
    failure(pele_public_invoke_v4(NULL, 0, NULL, 64 * 1024 * 1024 + 1, NULL), 2, "budget");
    failure(pele_public_render_v4(NULL, NULL, 0, NULL), 1, "null document");
    failure(pele_public_render_operation_v4(NULL, 0, NULL, 0, NULL), 1, "null document");
    failure(pele_public_apply_v4(NULL, 0, NULL, 64 * 1024 * 1024 + 1), 2, "budget");
    const char *unavailable = "gitGraph\ncommit";
    failure(pele_public_parse_v4((const uint8_t *)unavailable, strlen(unavailable), NULL, 0, &document), 1, "codec"); assert(document == NULL);
    const char *source = "pie title Before\n\"A\":1\n\"B\":2";
    PeleResult *result = pele_public_parse_v4((const uint8_t *)source, strlen(source), NULL, 0, &document);
    ok(result); assert(document); char *metadata = text(result); assert(strstr(metadata, "\"revision\":\"0\"")); free(metadata); pele_result_free(result);
    result = pele_public_schema_v4(document); ok(result);
    char *schema = text(result); assert(strstr(schema, "\"methods\":[]")); free(schema); pele_result_free(result);
    const char *unknown_method = "{\"method\":0,\"receiver\":{\"kind\":\"ref\",\"value\":0},\"arguments\":[]}";
    failure(pele_public_invoke_v4(document, 0, (const uint8_t *)unknown_method, strlen(unknown_method), NULL), 1, "method is unavailable");
    failure(pele_public_invoke_v4(document, 0, (const uint8_t *)"{}", 2, NULL), 1, "Invalid public model method call");
    result = pele_public_snapshot_v4(document); ok(result);
    char *snapshot = text(result), *graph = strstr(snapshot, "\"graph\":"); assert(graph); graph += 8;
    size_t graph_len = strlen(graph) - 2; graph[graph_len] = 0;
    char *number = strstr(graph, "3ff0000000000000"); assert(number); memcpy(number, "4014000000000000", 16);
    PeleResult *applied = pele_public_apply_v4(document, 0, (const uint8_t *)graph, graph_len); ok(applied);
    metadata = text(applied); assert(strstr(metadata, "\"revision\":\"1\"")); free(metadata); pele_result_free(applied);
    failure(pele_public_apply_v4(document, 0, (const uint8_t *)graph, graph_len), 1, "Stale public model revision");
    failure(pele_public_apply_v4(document, 1, (const uint8_t *)"{}", 2), 1, "invalid model graph JSON");
    free(snapshot); pele_result_free(result);
    result = pele_public_render_v4(document, NULL, 0, NULL); ok(result);
    const char *changed = "pie title Before\n\"A\":5\n\"B\":2";
    PeleResult *expected = pele_render_v2((const uint8_t *)changed, strlen(changed), NULL, 0); ok(expected);
    assert(pele_result_len(result) == pele_result_len(expected));
    assert(!memcmp(pele_result_data(result), pele_result_data(expected), pele_result_len(result))); pele_result_free(result);
    struct Context context = {document, 0, 0};
    PeleHostResourcesV3 host = {0}; host.base.context = &context; host.base.width = width;
    result = pele_public_render_v4(document, NULL, 0, &host); ok(result); pele_result_free(result); assert(context.calls > 0);
    context.fail = 1; failure(pele_public_render_v4(document, NULL, 0, &host), 1, "Host text measurer failed");
    result = pele_public_render_v4(document, NULL, 0, NULL); ok(result);
    assert(pele_result_len(result) == pele_result_len(expected));
    assert(!memcmp(pele_result_data(result), pele_result_data(expected), pele_result_len(result)));
    pele_result_free(result); pele_result_free(expected);
    result = pele_public_snapshot_v4(document); ok(result); metadata = text(result); assert(strstr(metadata, "\"revision\":\"1\"")); free(metadata); pele_result_free(result);
    result = pele_public_render_operation_v4(document, 1, NULL, 0, NULL); ok(result);
    metadata = text(result); assert(strstr(metadata, "\"revision\":2")); assert(strstr(metadata, "\"Ok\":{")); assert(strstr(metadata, "\"graph\":{")); free(metadata); pele_result_free(result);
    failure(pele_public_render_operation_v4(document, 1, NULL, 0, NULL), 1, "Stale public model revision");
    const char *limited = "{\"outputLimit\":1}";
    result = pele_public_render_operation_v4(document, 2, (const uint8_t *)limited, strlen(limited), NULL); ok(result);
    metadata = text(result); assert(strstr(metadata, "\"revision\":3")); assert(strstr(metadata, "\"Err\":{")); assert(strstr(metadata, "\"code\":\"limit\"")); free(metadata); pele_result_free(result);
    result = pele_public_render_operation_v4(document, 3, NULL, 0, &host); ok(result);
    metadata = text(result); assert(strstr(metadata, "\"revision\":4")); assert(strstr(metadata, "Host text measurer failed")); free(metadata); pele_result_free(result);
    context.fail = 0;
    result = pele_public_render_operation_v4(document, 4, NULL, 0, &host); ok(result);
    metadata = text(result); assert(strstr(metadata, "\"revision\":5")); assert(strstr(metadata, "\"Ok\":{")); free(metadata); pele_result_free(result);
    result = pele_public_snapshot_v4(document); ok(result); metadata = text(result); assert(strstr(metadata, "\"revision\":\"5\"")); free(metadata); pele_result_free(result);
    pele_public_document_free_v4(document); pele_public_document_free_v4(NULL);
    for (int i = 0; i < 200; i++) {
        document = NULL; result = pele_public_parse_v4((const uint8_t *)"info", 4, NULL, 0, &document); ok(result); pele_result_free(result);
        result = pele_public_render_v4(document, NULL, 0, NULL); ok(result); pele_result_free(result); pele_public_document_free_v4(document);
    }
    puts("mutable public C integration passed"); return 0;
}
