/* Real ABI feature: nested Class callbacks and captured methods outside render.
 * Reuse the existing integration's binary/JSON ownership helpers and run its
 * Gantt feature first, so the additive dispatcher also proves compatibility. */
#define main gantt_host_integration
#include "ffi_host_functions_v12.c"
#undef main

struct ClassContext { unsigned calls, retains, releases, fail; char *member; };
static void class_retain(void *p) { ((struct ClassContext *)p)->retains++; }
static void class_release(void *p) { ((struct ClassContext *)p)->releases++; }
static void class_invoke(void *p, PeleActiveModelV12 *view, const uint8_t *bytes, size_t len, PeleHostFunctionReplyV12 *reply) {
    struct ClassContext *context = p; context->calls++;
    char *input = malloc(len + 1); assert(input); memcpy(input, bytes, len); input[len] = 0;
    char *receiver = object(strstr(input, "\"receiver\":") + 11); free(input);
    char *star = access(view, "{\"operation\":\"intern\",\"units\":[42]}");
    set_field(view, receiver, "classifier", star); free(star);
    if (context->fail) { reply->status = 2; reply->exception = 77; free(receiver); return; }
    char request[512]; snprintf(request, sizeof request, "{\"operation\":\"invokeBuiltin\",\"call\":{\"method\":27,\"receiver\":%s,\"arguments\":[]}}", receiver);
    char *result = access(view, request); free(receiver);
    reply->payload = (PeleForeignBytesV10){result, (const uint8_t *)result, strlen(result), release_bytes};
}
static void class_install(PelePublicDocument *document, struct ClassContext *context, uint32_t reference) {
    PeleResult *result = pele_public_snapshot_v4(document); assert(!pele_result_status(result));
    char *all = copy_result(result); pele_result_free(result); char *graph = object(strstr(all, "\"graph\":") + 8); free(all);
    char *strings = strstr(graph, "\"strings\":[") + 11, *at = strings; unsigned count = 0;
    while (*at != ']') { count++; at = strchr(at, ']') + 1; if (*at == ',') at++; }
    graph = insert(graph, (size_t)(at - graph), ",[112,97,114,115,101,67,108,97,115,115,105,102,105,101,114]");
    char *node = strstr(graph, "\"nodes\":[") + 9; unsigned id = 0;
    for (;;) { char *part = object(node); int found = strstr(part, "\"prototype\":2") != NULL; size_t length = strlen(part); free(part); if (found) break; node += length + 1; id++; }
    char member[80]; snprintf(member, sizeof member, "{\"kind\":\"ref\",\"value\":%u}", id); context->member = strdup(member); assert(context->member);
    char *properties = strstr(node, "\"properties\":[") + 14, entry[180];
    snprintf(entry, sizeof entry, "{\"key\":%u,\"value\":{\"kind\":\"hostReference\",\"value\":%u},\"flags\":7},", count, reference);
    graph = insert(graph, (size_t)(properties - graph), entry); good(pele_public_apply_v4(document, 0, (const uint8_t *)graph, strlen(graph))); free(graph);
}
int main(void) {
    assert(gantt_host_integration() == 0);
    const char source[] = "classDiagram\nclass A {\n +value$\n}\nA --> B";
    PelePublicDocument *document = NULL; good(pele_public_parse_v4((const uint8_t *)source, strlen(source), (const uint8_t *)"{}", 2, &document)); assert(document);
    struct ClassContext context = {0}; PeleHostFunctionV12 function = {12, sizeof function, &context, class_retain, class_release, class_invoke, {0,0}};
    uint32_t reference = 0; good(pele_public_bind_host_function_v12(document, &function, &reference)); assert(reference && context.retains == 1);
    class_install(document, &context, reference);
    char call[256]; snprintf(call, sizeof call, "{\"method\":26,\"receiver\":%s,\"arguments\":[]}", context.member);
    PeleResult *result = pele_public_invoke_with_functions_v12(document, 1, (const uint8_t *)call, strlen(call), NULL);
    char *meta = metadata(result); assert(strstr(meta, "\"Ok\"") && context.calls == 1); free(meta); pele_result_free(result);
    context.fail = 1; result = pele_public_invoke_with_functions_v12(document, 2, (const uint8_t *)call, strlen(call), NULL);
    meta = metadata(result); assert(strstr(meta, "\"hostException\",\"value\":77") && context.calls == 2); free(meta); pele_result_free(result);
    context.fail = 0; result = pele_public_render_operation_with_functions_v12(document, 3, (const uint8_t *)"{}", 2, NULL, 0, 0, NULL);
    meta = metadata(result); assert(strstr(meta, "<svg") && context.calls == 3); free(meta); pele_result_free(result);
    pele_public_document_free_v4(document); assert(context.releases == 1); free(context.member);
    puts("Class nested host method C v12 integration PASS"); return 0;
}
