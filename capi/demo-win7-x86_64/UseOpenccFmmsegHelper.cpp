// UseOpenccFmmsegHelper.cpp
#include <iostream>
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
    }
    catch (const std::exception& ex)
    {
        std::cerr << "❌ Exception: " << ex.what() << "\n";
        return 1;
    }

    return 0;
}
