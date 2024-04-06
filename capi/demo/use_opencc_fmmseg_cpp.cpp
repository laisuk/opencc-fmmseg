#include <iostream>
#include "opencc_fmmseg_capi.hpp"
#include <windows.h>

int main(int argc, char **argv)
{
    SetConsoleOutputCP(65001);
    auto opencc = opencc_new();
    auto is_parallel = opencc_get_parallel(opencc);
    std::cout << "OpenCC is_parallel: " << is_parallel << std::endl;
    const char *text = "你好，“意大利的美丽世界”，欢迎！";
    std::cout << "Text: " << text << std::endl;
    auto code = opencc_zho_check(opencc, text);
    std::cout << "Text Code: " << code << std::endl;
    char *result = opencc_s2twp(opencc, text, true);
    code = opencc_zho_check(opencc, result);
    std::cout << result << std::endl;
    std::cout << "Text Code: " << code << std::endl;
    if (result != NULL)
    {
        opencc_string_free(result);
    }
    if (opencc != NULL)
    {
        opencc_close(opencc);
    }

    return 0;
}