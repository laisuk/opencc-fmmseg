#include <iostream>
#include <string>
#include "opencc_fmmseg_capi.h"

static void print_last_error_and_free() {
    char *last_error = opencc_last_error();
    if (last_error != nullptr) {
        std::cout << "Last Error: " << last_error << "\n";
        opencc_error_free(last_error);
    } else {
        std::cout << "Last Error: (null)\n";
    }
}

int main(int argc, char **argv) {
    (void)argc;
    (void)argv;

    void *opencc = opencc_new();
    if (opencc == nullptr) {
        std::cout << "❌ opencc_new() returned NULL\n";
        return 1;
    }

    bool is_parallel = opencc_get_parallel(opencc);
    std::cout << "OpenCC is_parallel: " << (int)is_parallel << "\n";

    const char *config_name = u8"s2twp";
    const char *text = u8"意大利邻国法兰西罗浮宫里收藏的“蒙娜丽莎的微笑”画像是旷世之作。";

    std::cout << "Text: " << text << "\n";

    int code = opencc_zho_check(opencc, text);
    std::cout << "Text Code: " << code << "\n";

    // ---------------------------------------------------------------------
    // Test 1: opencc_convert() (string config)
    // ---------------------------------------------------------------------
    std::cout << "\n== Test 1: opencc_convert(config_name=\"" << config_name << "\") ==\n";

    char *result1 = opencc_convert(opencc, text, config_name, true);
    if (result1 == nullptr) {
        std::cout << "❌ opencc_convert() returned NULL\n";
        print_last_error_and_free();
    } else {
        std::cout << "Converted: " << result1 << "\n";
        int out_code = opencc_zho_check(opencc, result1);
        std::cout << "Converted Code: " << out_code << "\n";
        print_last_error_and_free();
        opencc_string_free(result1);
    }

    // ---------------------------------------------------------------------
    // Test 2: opencc_convert_cfg() (numeric config)
    // ---------------------------------------------------------------------
    std::cout << "\n== Test 2: opencc_convert_cfg(config=" << (unsigned)OPENCC_CONFIG_S2TWP << ") ==\n";

    char *result2 = opencc_convert_cfg(opencc, text, OPENCC_CONFIG_S2TWP, true);
    if (result2 == nullptr) {
        std::cout << "❌ opencc_convert_cfg() returned NULL\n";
        print_last_error_and_free();
    } else {
        std::cout << "Converted: " << result2 << "\n";
        int out_code = opencc_zho_check(opencc, result2);
        std::cout << "Converted Code: " << out_code << "\n";
        print_last_error_and_free();
        opencc_string_free(result2);
    }

    // ---------------------------------------------------------------------
    // Test 3: opencc_convert_cfg() invalid config (negative test)
    // ---------------------------------------------------------------------
    std::cout << "\n== Test 3: opencc_convert_cfg(invalid config=9999) ==\n";

    char *result3 = opencc_convert_cfg(opencc, text, 9999, true);
    if (result3 == nullptr) {
        std::cout << "Returned: (null)\n";
    } else {
        std::cout << "Returned: " << result3 << "\n";
        opencc_string_free(result3);
    }
    print_last_error_and_free();

    // ---------------------------------------------------------------------
    // Test 4: opencc_convert_cfg_mem() (size-query + std::string buffer)
    // ---------------------------------------------------------------------
    std::cout << "\n== Test 4: opencc_convert_cfg_mem(config=" << (unsigned)OPENCC_CONFIG_S2TWP << ") ==\n";

    size_t required = 0;

    // 1) Query size
    if (!opencc_convert_cfg_mem(opencc, text, OPENCC_CONFIG_S2TWP, true, nullptr, 0, &required)) {
        std::cout << "❌ size-query failed\n";
        print_last_error_and_free();
    } else {
        std::cout << "Required bytes (incl. NUL): " << required << "\n";

        // 2) Allocate buffer
        std::string buf(required, '\0');

        if (!opencc_convert_cfg_mem(opencc, text, OPENCC_CONFIG_S2TWP, true,
                                    buf.data(), buf.size(), &required)) {
            std::cout << "❌ convert_cfg_mem failed\n";
            print_last_error_and_free();
        } else {
            // buf is NUL-terminated; printing is safe
            std::cout << "Converted: " << buf.c_str() << "\n";
            std::cout << "Converted Code: " << opencc_zho_check(opencc, buf.c_str()) << "\n";
            print_last_error_and_free();
        }
    }

    // ---------------------------------------------------------------------
    // Cleanup
    // ---------------------------------------------------------------------
    opencc_delete(opencc);
    return 0;
}
