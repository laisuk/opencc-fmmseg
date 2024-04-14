#include <iostream>
#include "opencc_fmmseg_capi.h"

int main(int argc, char **argv) {
    auto opencc = opencc_new();
    auto is_parallel = opencc_get_parallel(opencc);
    std::cout << "OpenCC is_parallel: " << is_parallel << "\n";
    const char *config = u8"s2twp";
    const char *text = u8"意大利罗浮宫里收藏的“蒙娜丽莎的微笑”画像是旷世之作。";
    std::cout << "Text: " << text << "\n";
    auto code = opencc_zho_check(opencc, text);
    std::cout << "Text Code: " << code << "\n";
    char *result = opencc_convert(opencc, config, text, true);
    code = opencc_zho_check(opencc, result);
    std::cout << "Converted: " << result << "\n";
    std::cout << "Text Code: " << code << "\n";
    if (result != NULL) {
        opencc_string_free(result);
    }
    if (opencc != NULL) {
        opencc_free(opencc);
    }

    return 0;
}