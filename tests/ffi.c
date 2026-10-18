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
    puts("C ABI ownership/error integration passed (400 success/error lifetimes)");
    return 0;
}
