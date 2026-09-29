#include <cstring>
#include <iostream>
#include <vector>
#include <windows.h>
#include "opencc_fmmseg_capi.h"

int main(int argc, char **argv) {
    SetConsoleOutputCP(65001);

    auto opencc = opencc_new();
    if (opencc == nullptr) {
        std::cerr << "Failed to create OpenCC instance.\n";
        return 1;
    }

    auto is_parallel = opencc_get_parallel(opencc);
    std::cout << "OpenCC is_parallel: " << is_parallel << "\n";

    const char *config = u8"s2twp";
    const char *text =
        u8"意大利邻国法兰西罗浮宫里收藏的“蒙娜丽莎的微笑”画像是旷世之作。";

    std::cout << "Text: " << text << "\n";
    std::cout << "Text Code: " << opencc_zho_check(opencc, text) << "\n";

    opencc_config_t config_id;
    if (!opencc_config_name_to_id(config, &config_id)) {
        std::cerr << "Invalid config: " << config << "\n";
        opencc_delete(opencc);
        return 1;
    }

    const size_t input_len = std::strlen(text);
    size_t required = 0;

    // Pass 1: query required size, including the trailing NUL.
    bool ok = opencc_convert_cfg_mem_len(
        opencc,
        text,
        input_len,
        config_id,
        true,
        nullptr,
        0,
        &required
    );

    if (!ok) {
        char *last_error = opencc_last_error();
        std::cerr << "Size query failed: " << last_error << "\n";
        opencc_error_free(last_error);
        opencc_delete(opencc);
        return 1;
    }

    // Caller-owned output buffer.
    std::vector<char> result(required);

    // Pass 2: write the converted text into the buffer.
    ok = opencc_convert_cfg_mem_len(
        opencc,
        text,
        input_len,
        config_id,
        true,
        result.data(),
        result.size(),
        &required
    );

    if (!ok) {
        char *last_error = opencc_last_error();
        std::cerr << "Conversion failed: " << last_error << "\n";
        opencc_error_free(last_error);
        opencc_delete(opencc);
        return 1;
    }

    std::cout << "Converted: " << result.data() << "\n";
    std::cout << "Converted Code: "
              << opencc_zho_check(opencc, result.data()) << "\n";
    std::cout << "Required bytes: " << required << "\n";

    char *last_error = opencc_last_error();
    std::cout << "Last Error: " << last_error << "\n";
    opencc_error_free(last_error);

    opencc_delete(opencc);
    return 0;
}
