#include <iostream>
#include "opencc_fmmseg_capi.hpp"
#include <windows.h>

int main(int argc, char **argv) {
    SetConsoleOutputCP(65001);
    auto opencc = opencc_new();
    auto is_parallel = opencc_get_parallel(opencc);
    std::cout << "OpenCC is_parallel: " << is_parallel << "\n";
    const char *text = "意大利罗浮宫里收藏的“蒙娜丽莎的微笑”画像是旷世之作。";
    std::cout << "Text: " << text << "\n";
    auto code = opencc_zho_check(opencc, text);
    std::cout << "Text Code: " << code << "\n";
    char *result = opencc_s2twp(opencc, text, true);
    code = opencc_zho_check(opencc, result);
    std::cout << "Converted: " << result << "\n";
    std::cout << "Text Code: " << code << "\n";
    if (result != NULL) {
        opencc_string_free(result);
    }
    if (opencc != NULL) {
        opencc_close(opencc);
    }

    return 0;
}