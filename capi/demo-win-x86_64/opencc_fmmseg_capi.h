#ifndef OPENCC_FMMSEG_CAPI_H
#define OPENCC_FMMSEG_CAPI_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stdbool.h>

void *opencc_new();
char *opencc_convert(const void *instance, const char *config, const char *input, bool punctuation);
bool opencc_get_parallel(const void *instance);
void opencc_set_parallel(const void *instance, bool is_parallel);
int opencc_zho_check(const void *instance, const char *input);
void opencc_free(const void *instance);
void opencc_string_free(char *ptr);

#ifdef __cplusplus
}
#endif

#endif // OPENCC_FMMSEG_CAPI_H
