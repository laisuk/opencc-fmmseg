#include <stdio.h>
#include <stdlib.h> // malloc/free
#include <windows.h>
#include "opencc_fmmseg_capi.h"

static void print_last_error_and_free(void) {
    char *last_error = opencc_last_error();
    if (last_error != NULL) {
        printf("Last Error: %s\n", last_error);
        opencc_error_free(last_error);
    } else {
        printf("Last Error: (null)\n");
    }
}

int main(int argc, char **argv) {
    (void)argc;
    (void)argv;

    // UTF-8 output in Windows console
    SetConsoleOutputCP(65001);

    void *opencc = opencc_new();
    if (opencc == NULL) {
        printf("❌ opencc_new() returned NULL\n");
        return 1;
    }

    bool is_parallel = opencc_get_parallel(opencc);
    printf("OpenCC is_parallel: %d\n", (int)is_parallel);

    const char *config_name = u8"s2twp";
    const char *text = u8"意大利邻国法兰西罗浮宫里收藏的“蒙娜丽莎的微笑”画像是旷世之作。";

    printf("Text: %s\n", text);

    int code = opencc_zho_check(opencc, text);
    printf("Text Code: %d\n", code);

    // ---------------------------------------------------------------------
    // Test 1: opencc_convert() (string config)
    // ---------------------------------------------------------------------
    printf("\n== Test 1: opencc_convert(config_name=\"%s\") ==\n", config_name);

    char *result1 = opencc_convert(opencc, text, config_name, true);
    if (result1 == NULL) {
        printf("❌ opencc_convert() returned NULL\n");
        print_last_error_and_free();
    } else {
        printf("Converted: %s\n", result1);

        int out_code = opencc_zho_check(opencc, result1);
        printf("Converted Code: %d\n", out_code);

        print_last_error_and_free();
        opencc_string_free(result1);
    }

    // ---------------------------------------------------------------------
    // Test 2: opencc_convert_cfg() (numeric config)
    // ---------------------------------------------------------------------
    printf("\n== Test 2: opencc_convert_cfg(config=%u) ==\n", (unsigned)OPENCC_CONFIG_S2TWP);

    char *result2 = opencc_convert_cfg(opencc, text, OPENCC_CONFIG_S2TWP, true);
    if (result2 == NULL) {
        printf("❌ opencc_convert_cfg() returned NULL\n");
        print_last_error_and_free();
    } else {
        printf("Converted: %s\n", result2);

        int out_code = opencc_zho_check(opencc, result2);
        printf("Converted Code: %d\n", out_code);

        print_last_error_and_free();
        opencc_string_free(result2);
    }

    // ---------------------------------------------------------------------
    // Test 3: opencc_convert_cfg() invalid config (negative test)
    // ---------------------------------------------------------------------
    printf("\n== Test 3: opencc_convert_cfg(invalid config=9999) ==\n");

    char *result3 = opencc_convert_cfg(opencc, text, 9999, true);
    if (result3 != NULL) {
        printf("Returned: %s\n", result3);
        opencc_string_free(result3);
    } else {
        printf("Returned: (null)\n");
    }
    print_last_error_and_free();

    // ---------------------------------------------------------------------
    // Test 4: opencc_convert_cfg_mem() (size-query + caller buffer)
    // ---------------------------------------------------------------------
    printf("\n== Test 4: opencc_convert_cfg_mem(config=%u) ==\n", (unsigned)OPENCC_CONFIG_S2TWP);

    size_t required = 0;

    // 1) Query required bytes (including '\0')
    if (!opencc_convert_cfg_mem(opencc, text, OPENCC_CONFIG_S2TWP, true, NULL, 0, &required)) {
        printf("❌ size-query failed\n");
        print_last_error_and_free();
    } else {
        printf("Required bytes (incl. NUL): %zu\n", required);

        // 2) Allocate and convert
        char* buf = (char*)malloc(required);
        if (!buf) {
            printf("❌ malloc failed\n");
        } else {
            if (!opencc_convert_cfg_mem(opencc, text, OPENCC_CONFIG_S2TWP, true, buf, required, &required)) {
                printf("❌ convert_cfg_mem failed\n");
                print_last_error_and_free();
            } else {
                printf("Converted: %s\n", buf);
                printf("Converted Code: %d\n", opencc_zho_check(opencc, buf));
                print_last_error_and_free();
            }
            free(buf); // caller-owned
        }
    }

    // ---------------------------------------------------------------------
    // Cleanup
    // ---------------------------------------------------------------------
    opencc_delete(opencc);
    return 0;
}
