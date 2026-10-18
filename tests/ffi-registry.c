#include "pele.h"
#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static char *text(PeleResult *result) {
    size_t length = pele_result_len(result); char *copy = malloc(length + 1); assert(copy);
    memcpy(copy, pele_result_data(result), length); copy[length] = 0; return copy;
}
static void success(PeleResult *result, const char *expected) {
    assert(result && pele_result_status(result) == 0);
    char *body = text(result); assert(strstr(body, expected)); free(body); pele_result_free(result);
}
static void failure(PeleResult *result, uint32_t status, const char *expected) {
    assert(result && pele_result_status(result) == status);
    char *body = text(result); assert(strstr(body, expected)); free(body); pele_result_free(result);
}
static size_t occurrences(const char *text, const char *needle) {
    size_t count = 0, length = strlen(needle); const char *p = text;
    while ((p = strstr(p, needle))) { ++count; p += length; } return count;
}
static PeleRegistry *create(uint32_t mode) {
    PeleRegistry *registry = NULL; success(pele_registry_create_v8(mode, &registry), "\"result\":true");
    assert(registry); return registry;
}
static void capabilities(PeleRegistry *registry, size_t registered) {
    PeleResult *result = pele_registry_capabilities_v8(registry); assert(pele_result_status(result) == 0);
    char *body = text(result); assert(occurrences(body, "\"known\":true") == 33);
    assert(occurrences(body, "\"registered\":true") == registered); free(body); pele_result_free(result);
}
static PeleResult *parse(PeleRegistry *registry, const char *source, const char *options,
                        uint32_t source_kind, uint64_t source_bits,
                        uint32_t edge_kind, uint64_t edge_bits, PelePublicDocument **out) {
    return pele_registry_parse_limits_v8(registry, (const uint8_t *)source, strlen(source),
        (const uint8_t *)options, options ? strlen(options) : 0,
        source_kind, source_bits, edge_kind, edge_bits, out);
}
int main(void) {
    const char *ids[] = {"flowchart","pie","er","mindmap","kanban","timeline","journey","class",
        "quadrantChart","xychart","state","gantt","architecture","requirement","sankey","packet",
        "radar","treemap","info","sequence","gitGraph","c4","block","railroad","agentflow",
        "treeView","cynefin","wardley","eventmodeling","usecase","venn","ishikawa","swimlane"};
    PeleRegistry *bad = (PeleRegistry *)(uintptr_t)1;
    failure(pele_registry_create_v8(99, &bad), 1, "unknown creation mode"); assert(bad == NULL);
    failure(pele_registry_create_v8(0, NULL), 1, "null registry output");
    failure(pele_registry_capabilities_v8(NULL), 1, "null registry");
    for (unsigned repeat = 0; repeat < 8; ++repeat) {
        PeleRegistry *registry = create(0), *separate = create(0), *all = create(1);
        capabilities(registry, 0); capabilities(all, 33);
        success(pele_registry_detect_type_v8(registry, (const uint8_t *)"info", 4), "\"type\":\"info\"");
        success(pele_registry_supports_v8(registry, (const uint8_t *)"info", 4), "\"result\":false");
        PelePublicDocument *document = (PelePublicDocument *)(uintptr_t)1;
        failure(parse(registry, "info", NULL, 3, 0, 3, 0, &document), 1, "Diagram type \\\"info\\\" is not supported yet.");
        assert(document == NULL);
        for (unsigned i = 0; i < 33; ++i) {
            const uint8_t *id = (const uint8_t *)ids[i]; size_t length = strlen(ids[i]);
            success(pele_registry_registered_v8(registry, id, length), "\"result\":false");
            success(pele_registry_register_builtin_v8(registry, id, length), "\"result\":true");
            success(pele_registry_register_builtin_v8(registry, id, length), "\"result\":true");
            success(pele_registry_registered_v8(registry, id, length), "\"result\":true");
        }
        capabilities(registry, 33); capabilities(separate, 0);
        success(pele_registry_registered_v8(registry, (const uint8_t *)"Info", 4), "\"result\":false");
        failure(pele_registry_register_builtin_v8(registry, (const uint8_t *)"Info", 4), 1, "Unknown built-in");
        success(parse(registry, "info title Independent registry", NULL, 3, 0, 3, 0, &document), "\"revision\":\"0\"");
        pele_registry_free_v8(registry);
        success(pele_public_render_v4(document, NULL, 0, NULL), "Independent registry");
        pele_public_document_free_v4(document);
        const char *chain = "flowchart TD\nA --> B --> C";
        const char *options = "{\"limit\":-1,\"maxEdges\":-1,\"config\":{\"flowchart\":{\"inheritDir\":true}}}";
        document = NULL;
        success(parse(all, chain, options, 0, 0, 2, UINT64_C(0x7ff8000000000042), &document), "\"type\":\"flowchart\"");
        success(pele_public_snapshot_v4(document), "7ff8000000000042"); pele_public_document_free_v4(document);
        document = (PelePublicDocument *)(uintptr_t)1;
        failure(parse(all, chain, options, 0, 0, 3, 0, &document), 1, "Invalid parse options"); assert(document == NULL);
        failure(parse(all, chain, options, 3, 0, 0, 0, &document), 1, "Invalid parse options"); assert(document == NULL);
        failure(parse(all, chain, NULL, 1, 0, 2, UINT64_C(0xfff0000000000000), &document), 2, "source is longer"); assert(document == NULL);
        failure(parse(all, chain, NULL, 0, 0, 1, 0, &document), 2, "Edge limit exceeded"); assert(document == NULL);
        const char *grammar = "flowchart TD\nA --> [";
        success(pele_registry_supports_v8(all, (const uint8_t *)grammar, strlen(grammar)), "\"result\":true");
        PeleResult *grammar_result = parse(all, grammar, NULL, 3, 0, 3, 0, &document);
        assert(pele_result_status(grammar_result) != 0 && document == NULL); pele_result_free(grammar_result);
        const char *yaml = "---\ntitle: \"\\q\"\n---\npie\n\"A\": 1";
        failure(pele_registry_detect_type_v8(all, (const uint8_t *)yaml, strlen(yaml)), 1, "not valid YAML");
        success(pele_registry_supports_v8(all, (const uint8_t *)yaml, strlen(yaml)), "\"result\":false");
        success(pele_registry_detect_type_v8(all, (const uint8_t *)"unrecognized", 12), "\"type\":null");
        char *large = malloc(60008); assert(large); memcpy(large, "info\n%%", 7); memset(large + 7, 'x', 60000); large[60007] = 0;
        success(pele_registry_detect_type_v8(all, (const uint8_t *)large, 60007), "\"type\":\"info\"");
        success(pele_registry_supports_v8(all, (const uint8_t *)large, 60007), "\"result\":true");
        failure(parse(all, large, NULL, 3, 0, 3, 0, &document), 2, "source is longer"); assert(document == NULL);
        success(parse(all, large, NULL, 2, UINT64_C(0x7ff0000000000000), 3, 0, &document), "\"type\":\"info\"");
        pele_public_document_free_v4(document); free(large);
        pele_registry_free_v8(all); pele_registry_free_v8(separate);
    }
    PeleRegistry *registry = create(1); PelePublicDocument *document = (PelePublicDocument *)(uintptr_t)1;
    failure(pele_registry_parse_limits_v8(registry, (const uint8_t *)(uintptr_t)1, 16 * 1024 * 1024 + 1,
        NULL, 0, 0, 0, 0, 0, &document), 2, "declared input exceeds budget"); assert(document == NULL);
    failure(pele_registry_parse_limits_v8(registry, (const uint8_t *)"info", 4,
        (const uint8_t *)(uintptr_t)1, 1024 * 1024 + 1, 0, 0, 0, 0, &document), 2, "declared input exceeds budget"); assert(document == NULL);
    failure(pele_registry_registered_v8(registry, (const uint8_t *)(uintptr_t)1, 4097), 2, "declared input exceeds budget");
    failure(pele_registry_register_builtin_v8(registry, (const uint8_t *)(uintptr_t)1, 4097), 2, "declared input exceeds budget");
    failure(pele_registry_supports_v8(registry, (const uint8_t *)(uintptr_t)1, 16 * 1024 * 1024 + 1), 2, "declared input exceeds budget");
    failure(pele_registry_detect_type_v8(registry, (const uint8_t *)(uintptr_t)1, 16 * 1024 * 1024 + 1), 2, "declared input exceeds budget");
    const uint8_t invalid[] = {0xff}; failure(pele_registry_supports_v8(registry, invalid, 1), 1, "UTF-8");
    failure(parse(registry, "info", NULL, 99, 0, 0, 0, &document), 1, "unknown source-limit kind"); assert(document == NULL);
    failure(parse(registry, "info", NULL, 0, 0, 99, 0, &document), 1, "unknown edge-limit kind"); assert(document == NULL);
    failure(parse(NULL, "info", NULL, 0, 0, 0, 0, &document), 1, "null registry"); assert(document == NULL);
    failure(parse(registry, "info", NULL, 0, 0, 0, 0, NULL), 1, "null document output");
    pele_registry_free_v8(registry); pele_registry_free_v8(NULL); pele_result_free(NULL);
    puts("Owned registry C integration passed: all33, selective lifetimes, discovery, exact limits and budgets");
    return 0;
}
