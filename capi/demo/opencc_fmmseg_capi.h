#ifndef OPENCC_FMMSEG_CAPI_H
#define OPENCC_FMMSEG_CAPI_H

#pragma once
#include <stdbool.h>

void *opencc_new();
int opencc_zho_check(const void *instance, const char *input);
char *opencc_s2t(const void *instance, const char *input, bool punctuation);
char *opencc_s2tw(const void *instance, const char *input, bool punctuation);
char *opencc_s2twp(const void *instance, const char *input, bool punctuation);
char *opencc_s2hk(const void *instance, const char *input, bool punctuation);
char *opencc_t2s(const void *instance, const char *input, bool punctuation);
char *opencc_t2tw(const void *instance, const char *input);
char *opencc_t2twp(const void *instance, const char *input);
char *opencc_tw2s(const void *instance, const char *input, bool punctuation);
char *opencc_tw2sp(const void *instance, const char *input, bool punctuation);
char *opencc_tw2t(const void *instance, const char *input);
char *opencc_tw2tp(const void *instance, const char *input);
char *opencc_hk2s(const void *instance, const char *input, bool punctuation);
char *opencc_hk2t(const void *instance, const char *input);
char *opencc_jp2t(const void *instance, const char *input);
char *opencc_t2jp(const void *instance, const char *input);
bool opencc_get_parallel(const void *instance);
void opencc_set_parallel(const void *instance, bool is_parallel);
void opencc_close(void *instance);
void opencc_string_free(char *ptr);

#endif // OPENCC_FMMSEG_CAPI_H