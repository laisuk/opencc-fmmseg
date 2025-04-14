#include <stdio.h>
#include "opencc_fmmseg_capi.h"

int main(int argc, char **argv) {
    void *opencc = opencc_new();
    bool is_parallel = opencc_get_parallel(opencc);
    printf("OpenCC is_parallel: %d\n", is_parallel);
    const char *config = u8"s2twp";
    const char *text = u8"意大利邻国法兰西罗浮宫里收藏的“蒙娜丽莎的微笑”画像是旷世之作。";
    printf("Text: %s\n", text);
    int code = opencc_zho_check(opencc, text);
    printf("Text Code: %d\n", code);
    char *result = opencc_convert(opencc, text, config, true);
    code = opencc_zho_check(opencc, result);
    char *last_error = opencc_last_error();
    printf("Converted: %s\n", result);
    printf("Text Code: %d\n", code);
     printf("Last Error: %s\n", last_error == NULL ? "No error" : last_error);
    if (result != NULL) {
        opencc_string_free(result);
    }
    if (opencc != NULL) {
        opencc_free(opencc);
    }

    return 0;
}
