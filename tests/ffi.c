#include "pele.h"
#include <assert.h>
#include <stdio.h>
#include <string.h>

int main(void) {
    assert(pele_abi_version() == PELE_ABI_VERSION);
    assert(pele_result_status(NULL) == 3);
    assert(pele_result_len(NULL) == 0);
    assert(pele_result_data(NULL) == NULL);
    pele_result_free(NULL);
    const char *source = "graph LR; A[Hello] --> B{World}";
    for (int i=0; i<200; i++) {
        PeleResult *result = pele_render((const uint8_t *)source, strlen(source), NULL, 0);
        assert(result && pele_result_status(result) == 0);
        assert(pele_result_data(result) && pele_result_len(result) > 100);
        pele_result_free(result);
        result = pele_render(NULL, 0, NULL, 0);
        assert(result && pele_result_status(result) == 1);
        pele_result_free(result);
    }
    uint8_t invalid[] = {0xff};
    PeleResult *result = pele_render(invalid, 1, NULL, 0);
    assert(pele_result_status(result) == 1); pele_result_free(result);
    result = pele_render(NULL, 1, NULL, 0);
    assert(pele_result_status(result) == 1); pele_result_free(result);
    result = pele_render(NULL, 262145, NULL, 0);
    assert(pele_result_status(result) == 2); pele_result_free(result);
    result = pele_inspect((const uint8_t *)"pie", 3, NULL, 0);
    assert(pele_result_status(result) == 0);
    char inspection[1024]; size_t length = pele_result_len(result);
    assert(length < sizeof inspection); memcpy(inspection, pele_result_data(result), length); inspection[length] = 0;
    assert(strstr(inspection, "encodedText") != NULL);
    pele_result_free(result);
    result = pele_inspect(NULL, 16777217, NULL, 0);
    assert(pele_result_status(result) == 2); pele_result_free(result);
    result = pele_inspect(NULL, 1, NULL, 0);
    assert(pele_result_status(result) == 1); pele_result_free(result);
    result = pele_inspect(invalid, 1, NULL, 0);
    assert(pele_result_status(result) == 1); pele_result_free(result);
    result = pele_capabilities();
    assert(pele_result_status(result) == 0); pele_result_free(result);
    const char *pie = "pie title Native; \n\"One\":1\n\"Two\":2";
    for (int i=0; i<200; i++) {
        PeleResult *document = pele_parse_v2((const uint8_t *)pie, strlen(pie), NULL, 0);
        assert(document && pele_result_status(document) == 0);
        const char *ignored = "{\"limit\":0}";
        result = pele_render_document_with_resources_v2(document, (const uint8_t *)ignored, strlen(ignored), NULL);
        assert(result && pele_result_status(result) == 0);
        pele_result_free(document); /* rendered result owns bytes independently */
        assert(pele_result_data(result) && pele_result_len(result) > 100);
        pele_result_free(result);
    }
    result = pele_render_document_v2(NULL, NULL, 0);
    assert(pele_result_status(result) == 1); pele_result_free(result);
    result = pele_parse_v2(invalid, 1, NULL, 0);
    assert(pele_result_status(result) == 1); pele_result_free(result);
    puts("C ABI ownership/error integration passed (400 legacy lifetimes + 200 owned pie documents)");
    return 0;
}
