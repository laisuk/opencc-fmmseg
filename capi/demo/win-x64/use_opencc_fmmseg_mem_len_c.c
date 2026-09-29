#include <stdio.h>
#include <stdlib.h>  // malloc, free
#include <string.h>  // for strlen
#include <windows.h>
#include "opencc_fmmseg_capi.h"

int main(int argc, char **argv) {
    SetConsoleOutputCP(65001);
    void *opencc = opencc_new();
    bool is_parallel = opencc_get_parallel(opencc);
    printf("OpenCC is_parallel: %d\n", is_parallel);

    const char *config = u8"s2twp";
    const char *text = u8"意大利邻国法兰西罗浮宫里收藏的“蒙娜丽莎的微笑”画像是旷世之作。";

    printf("Text: %s\n", text);
    int code = opencc_zho_check(opencc, text);
    printf("Text Code: %d\n", code);

    // Explicit-length, caller-owned buffer API.
    opencc_config_t config_id;
    if (!opencc_config_name_to_id(config, &config_id)) {
        printf("Invalid config: %s\n", config);
        opencc_delete(opencc);
        return 1;
    }

    const size_t input_len = strlen(text);
    size_t required = 0;

    // Pass 1: query required size, including the trailing NUL.
    bool ok = opencc_convert_cfg_mem_len(
        opencc, text, input_len, config_id, true,
        NULL, 0, &required
    );

    if (!ok) {
        char *last_error = opencc_last_error();
        printf("Size query failed: %s\n", last_error);
        opencc_error_free(last_error);
        opencc_delete(opencc);
        return 1;
    }

    char *result = (char *)malloc(required);
    if (result == NULL) {
        fprintf(stderr, "Failed to allocate %zu bytes.\n", required);
        opencc_delete(opencc);
        return 1;
    }

    // Pass 2: write the converted text into the caller-owned buffer.
    ok = opencc_convert_cfg_mem_len(
        opencc, text, input_len, config_id, true,
        result, required, &required
    );

    if (!ok) {
        char *last_error = opencc_last_error();
        printf("Converted: (failed)\n");
        printf("Last Error: %s\n", last_error);
        opencc_error_free(last_error);
        free(result);
        opencc_delete(opencc);
        return 1;
    }

    printf("Converted: %s\n", result);
    printf("Converted Code: %d\n", opencc_zho_check(opencc, result));
    printf("Required bytes: %zu\n", required);

    char *last_error = opencc_last_error();
    printf("Last Error: %s\n", last_error);
    opencc_error_free(last_error);

    free(result);
    opencc_delete(opencc);

    return 0;
}
