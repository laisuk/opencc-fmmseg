#include <iostream>
#include "opencc_fmmseg_capi.h"

int main(int argc, char **argv) {
    auto opencc = opencc_new();
    auto is_parallel = opencc_get_parallel(opencc);
    std::cout << "OpenCC is_parallel: " << is_parallel << "\n";
    const char *config = "s2twp";
    const char *text = "意大利罗浮宫里收藏的“蒙娜丽莎的微笑”画像是旷世之作。";
    std::cout << "Text: " << text << "\n";
    auto code = opencc_zho_check(opencc, text);
    std::cout << "Text Code: " << code << "\n";
    char *result = opencc_convert(opencc, text, config, true);
    code = opencc_zho_check(opencc, result);
    char *last_error = opencc_last_error();
    std::cout << "Converted: " << result << "\n";
    std::cout << "Text Code: " << code << "\n";
    std::cout << "Last Error: " << (last_error == NULL ? "No error" : last_error) << "\n";
    if (result != NULL) {
        opencc_string_free(result);
    }
    if (opencc != NULL) {
        opencc_free(opencc);
    }

    return 0;
}