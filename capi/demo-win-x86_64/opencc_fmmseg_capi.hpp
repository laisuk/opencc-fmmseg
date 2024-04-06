#ifndef OPENCC_FMMSEG_CAPI_HPP
#define OPENCC_FMMSEG_CAPI_HPP

extern "C" __declspec(dllimport) void *opencc_new();
extern "C" __declspec(dllimport) int opencc_zho_check(const void *instance, const char *input);
extern "C" __declspec(dllimport) char *opencc_s2t(const void *instance, const char *input, bool punctuation);
extern "C" __declspec(dllimport) char *opencc_s2tw(const void *instance, const char *input, bool punctuation);
extern "C" __declspec(dllimport) char *opencc_s2twp(const void *instance, const char *input, bool punctuation);
extern "C" __declspec(dllimport) char *opencc_s2hk(const void *instance, const char *input, bool punctuation);
extern "C" __declspec(dllimport) char *opencc_t2s(const void *instance, const char *input, bool punctuation);
extern "C" __declspec(dllimport) char *opencc_t2tw(const void *instance, const char *input);
extern "C" __declspec(dllimport) char *opencc_t2twp(const void *instance, const char *input);
extern "C" __declspec(dllimport) char *opencc_tw2s(const void *instance, const char *input, bool punctuation);
extern "C" __declspec(dllimport) char *opencc_tw2sp(const void *instance, const char *input, bool punctuation);
extern "C" __declspec(dllimport) char *opencc_tw2t(const void *instance, const char *input);
extern "C" __declspec(dllimport) char *opencc_tw2tp(const void *instance, const char *input);
extern "C" __declspec(dllimport) char *opencc_hk2s(const void *instance, const char *input, bool punctuation);
extern "C" __declspec(dllimport) char *opencc_hk2t(const void *instance, const char *input);
extern "C" __declspec(dllimport) char *opencc_jp2t(const void *instance, const char *input);
extern "C" __declspec(dllimport) char *opencc_t2jp(const void *instance, const char *input);
extern "C" __declspec(dllimport) char *opencc_convert(const void *instance, const char *config, const char *input, bool punctuation);
extern "C" __declspec(dllimport) bool opencc_get_parallel(const void *instance);
extern "C" __declspec(dllimport) void opencc_set_parallel(const void *instance, bool is_parallel);
extern "C" __declspec(dllimport) void opencc_close(const void *instance);
extern "C" __declspec(dllimport) void opencc_string_free(char *ptr);

#endif // OPENCC_FMMSEG_CAPI_HPP