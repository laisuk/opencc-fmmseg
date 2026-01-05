#include <iostream>
#include <string.h>
#include "opencc_fmmseg_capi.h"
#include <windows.h>

int main(int argc, char **argv) {
    SetConsoleOutputCP(65001);
    auto opencc = opencc_new();
    auto is_parallel = opencc_get_parallel(opencc);
    std::cout << "OpenCC is_parallel: " << is_parallel << "\n";
    const char *config = u8"s2twp";
    const char *text = u8"意大利邻国法兰西罗浮宫里收藏的“蒙娜丽莎的微笑”画像是旷世之作。";
    std::cout << "Text: " << text << "\n";
    auto code = opencc_zho_check(opencc, text);
    std::cout << "Text Code: " << code << "\n";
    char *result = opencc_convert_len(opencc, text, strlen(text), config, true);
    char *last_error = opencc_last_error();

    if (result == nullptr) {
        std::cout << "Converted: (null)\n";
        std::cout << "Last Error: "
                  << (last_error == nullptr ? "No error" : last_error)
                  << "\n";
    } else {
        std::cout << "Converted: " << result << "\n";
        std::cout << "Last Error: "
                  << (last_error == nullptr ? "No error" : last_error)
                  << "\n";

        // Only classify language if conversion truly succeeded
        if (last_error == nullptr) {
            int code = opencc_zho_check(opencc, result);
            std::cout << "Converted Code: " << code << "\n";
        }
    }

    if (last_error != NULL) {
        opencc_error_free(last_error);
    }
    if (result != NULL) {
        opencc_string_free(result);
    }
    if (opencc != NULL) {
        opencc_free(opencc);
    }

    return 0;
}