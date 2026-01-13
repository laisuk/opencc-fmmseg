#include <iostream>
#include <string>
#include <cstring>
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

    // ---------------------------------------------------------------------
    // Test 0: C API info (ABI number / version string)
    // ---------------------------------------------------------------------
    printf("\n== Test 0: C API info (opencc_abi_number / opencc_version_string) ==\n");

    uint32_t abi = opencc_abi_number();
    const char* ver = opencc_version_string();

    printf("ABI number     : %u\n", (unsigned)abi);
    printf("Version string : %s\n", ver ? ver : "(null)");

    if (abi > 0) {
        printf("✔ ASSERT: ABI number is non-zero\n");
    } else {
        printf("❌ ASSERT FAILED: ABI number must be non-zero\n");
    }

    if (ver && ver[0] != '\0') {
        printf("✔ ASSERT: version string is non-empty\n");
    } else {
        printf("❌ ASSERT FAILED: version string must be non-null and non-empty\n");
    }

    // Optional (strict): ensure it's the same as the package version you expect.
    // Comment out if you don't want strict pinning in demo code.
    // if (ver && strcmp(ver, "0.8.4.2") == 0) {
    //     printf("✔ ASSERT: version string matches expected\n");
    // } else {
    //     printf("⚠ NOTE: version string differs from expected build tag\n");
    // }

    // Optional: last error should typically be empty / unchanged after info calls
    print_last_error_and_free();
    printf("\n");

    // ------ test 0 End ------

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
    // Test 5: Config name/id helpers (pure C API, C++ caller)
    // ---------------------------------------------------------------------
    printf("\n== Test 5: opencc_config_name_to_id / opencc_config_id_to_name (C API) ==\n");

    // 5.1) name -> id
    opencc_config_t id_from_name = 0;
    bool ok_name_to_id = opencc_config_name_to_id("s2twp", &id_from_name);

    printf("name_to_id(\"s2twp\") => ok=%d, id=%u\n",
           (int)ok_name_to_id, (unsigned)id_from_name);

    if (ok_name_to_id && id_from_name == OPENCC_CONFIG_S2TWP) {
        printf("✔ ASSERT: name -> id matches OPENCC_CONFIG_S2TWP\n");
    } else {
        printf("❌ ASSERT FAILED: expected id=%u\n",
               (unsigned)OPENCC_CONFIG_S2TWP);
    }

    // 5.2) id -> name (round trip)
    const char* name_from_id = opencc_config_id_to_name(id_from_name);

    printf("id_to_name(%u) => %s\n",
           (unsigned)id_from_name,
           name_from_id ? name_from_id : "(null)");

    if (name_from_id && strcmp(name_from_id, "s2twp") == 0) {
        printf("✔ ASSERT: id -> name round-trip OK\n");
    } else {
        printf("❌ ASSERT FAILED: expected name=\"s2twp\"\n");
    }

    // 5.3) negative: invalid name
    opencc_config_t dummy = 0;
    bool ok_bad_name = opencc_config_name_to_id("not-a-config", &dummy);

    printf("name_to_id(\"not-a-config\") => ok=%d\n", (int)ok_bad_name);

    if (!ok_bad_name) {
        printf("✔ ASSERT: invalid config name rejected\n");
    } else {
        printf("❌ ASSERT FAILED: invalid name should not succeed\n");
    }

    // 5.4) negative: invalid id
    const char* bad_id_name = opencc_config_id_to_name((opencc_config_t)9999);

    printf("id_to_name(9999) => %s\n",
           bad_id_name ? bad_id_name : "(null)");

    if (bad_id_name == NULL) {
        printf("✔ ASSERT: invalid config id rejected\n");
    } else {
        printf("❌ ASSERT FAILED: invalid id should return NULL\n");
    }

    // Optional: error state should remain clean
    print_last_error_and_free();

    // ---------------------------------------------------------------------
    // Cleanup
    // ---------------------------------------------------------------------
    opencc_delete(opencc);
    return 0;
}
