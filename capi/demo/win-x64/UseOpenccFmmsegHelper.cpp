// UseOpenccFmmsegHelper.cpp
#include <iostream>
#include <vector>
#include <windows.h>

#include "OpenccFmmsegHelper.hpp"

int main(int argc, char** argv)
{
    (void)argc;
    (void)argv;

    // Enable UTF-8 output on Windows console
    SetConsoleOutputCP(65001);

    try
    {
        OpenccFmmsegHelper helper;

        const std::string text =
            u8"意大利邻国法兰西罗浮宫里收藏的“蒙娜丽莎的微笑”画像是旷世之作。";

        std::cout << "Text: " << text << "\n";
        std::cout << "Text Code: " << helper.zhoCheck(text) << "\n";

        // -------------------------------------------------------------
        // Test 1: Stateless legacy string config
        // -------------------------------------------------------------
        std::cout << "\n== Test 1: convert(text, \"s2twp\", true) ==\n";

        std::string out1 = helper.convert(text, "s2twp", true);
        std::cout << "Converted: " << out1 << "\n";
        std::cout << "Converted Code: " << helper.zhoCheck(out1) << "\n";
        std::cout << "Last Error: " << OpenccFmmsegHelper::lastError() << "\n";

        // -------------------------------------------------------------
        // Test 2: Stateless typed config (recommended)
        // -------------------------------------------------------------
        std::cout << "\n== Test 2: convert_cfg(text, OPENCC_CONFIG_S2TWP, true) ==\n";

        std::string out2 =
            helper.convert_cfg(text, OPENCC_CONFIG_S2TWP, true);
        std::cout << "Converted: " << out2 << "\n";
        std::cout << "Converted Code: " << helper.zhoCheck(out2) << "\n";
        std::cout << "Last Error: " << OpenccFmmsegHelper::lastError() << "\n";

        // -------------------------------------------------------------
        // Test 3: Stateful typed config
        // -------------------------------------------------------------
        std::cout << "\n== Test 3: stateful config (setConfigId) ==\n";

        helper.setConfigId(OPENCC_CONFIG_S2TWP);
        helper.setPunctuation(true);

        std::string out3 = helper.convert_cfg(text);
        std::cout << "Converted: " << out3 << "\n";
        std::cout << "Converted Code: " << helper.zhoCheck(out3) << "\n";
        std::cout << "Last Error: " << OpenccFmmsegHelper::lastError() << "\n";

        // -------------------------------------------------------------
        // Test 4: Invalid config (self-protected)
        // -------------------------------------------------------------
        std::cout << "\n== Test 4: invalid typed config (9999) ==\n";

        std::string out4 =
            helper.convert_cfg(text, 9999, true);
        std::cout << "Returned: " << out4 << "\n";
        std::cout << "Last Error: " << OpenccFmmsegHelper::lastError() << "\n";

        // -------------------------------------------------------------
        // Test 5: Clear last error explicitly
        // -------------------------------------------------------------
        std::cout << "\n== Test 5: clear_last_error() ==\n";

        OpenccFmmsegHelper::clearLastError();

        std::cout << "Last Error after clear: "
                  << OpenccFmmsegHelper::lastError() << "\n";

        // -------------------------------------------------------------
        // Test 6: Immutable custom dictionary roundtrip
        // -------------------------------------------------------------
        std::cout << "\n== Test 6: immutable custom dictionary roundtrip ==\n";

        const std::vector<OpenccFmmsegHelper::CustomDictSpec> customDicts = {
            {
                OPENCC_DICT_SLOT_ST_PHRASES,
                OPENCC_CUSTOM_DICT_APPEND,
                {
                    {u8"帕兰蒂尔", u8"柏蘭蒂爾"},
                }
            },
            {
                OPENCC_DICT_SLOT_TS_PHRASES,
                OPENCC_CUSTOM_DICT_APPEND,
                {
                    {u8"柏蘭蒂爾", u8"帕兰蒂尔"},
                }
            }
        };

        const OpenccFmmsegHelper customHelper(customDicts);

        const std::string roundtripSource =
            u8"帕兰蒂尔是一家软件公司。";

        const std::string roundtripTraditional =
            customHelper.convert_cfg(roundtripSource, OPENCC_CONFIG_S2T);

        const std::string roundtripSimplified =
            customHelper.convert_cfg(roundtripTraditional, OPENCC_CONFIG_T2S);

        std::cout << "Source:      " << roundtripSource << "\n";
        std::cout << "S2T custom:  " << roundtripTraditional << "\n";
        std::cout << "T2S custom:  " << roundtripSimplified << "\n";
        std::cout << "Roundtrip:   "
                  << (roundtripSimplified == roundtripSource ? "PASS" : "FAIL")
                  << "\n";
        std::cout << "Last Error:  "
                  << OpenccFmmsegHelper::lastError() << "\n";
    }
    catch (const std::exception& ex)
    {
        std::cerr << "❌ Exception: " << ex.what() << "\n";
        return 1;
    }

    return 0;
}
