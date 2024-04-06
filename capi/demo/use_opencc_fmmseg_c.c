#include <stdio.h>
#include "opencc_fmmseg_capi.h"
#include <windows.h>

int main(int argc, char **argv)
{
    SetConsoleOutputCP(65001);
    void *opencc = opencc_new();
    bool is_parallel = opencc_get_parallel(opencc);
    printf("OpenCC is_parallel: %d\n", is_parallel);
    const char *text = "你好，“意大利的美丽世界”，欢迎！";
    printf("Text: %s\n", text);
    int code = opencc_zho_check(opencc, text);
    printf("Text Code: %d\n", code);
    char *result = opencc_s2twp(opencc, text, true);
    code = opencc_zho_check(opencc, result);
    printf("%s\n", result);
    printf("Text Code: %d\n", code);
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
