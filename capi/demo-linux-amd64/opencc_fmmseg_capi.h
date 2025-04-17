#ifndef OPENCC_FMMSEG_CAPI_H
#define OPENCC_FMMSEG_CAPI_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stdbool.h>

void *opencc_new();
//void *opencc_new_from_dicts();
char *opencc_convert(const void *instance, const char *input, const char *config, bool punctuation);
char *opencc_convert_len(
          const void *instance,
          const char *input,
          size_t input_len,
          const char *config,
          bool punctuation);
bool opencc_get_parallel(const void *instance);
void opencc_set_parallel(const void *instance, bool is_parallel);
int opencc_zho_check(const void *instance, const char *input);
void opencc_free(const void *instance);
void opencc_string_free(const char *ptr);
/**
 * Returns the last error message as a null-terminated C string.
 *
 * The returned string is dynamically allocated and must be freed
 * by calling `opencc_error_free()`.
 *
 * If there is no error, returns a string: "No error".
 *
 * @return A pointer to a null-terminated error message string.
 */
char *opencc_last_error();
/**
 * Frees a string returned by `opencc_last_error`.
 *
 * @param ptr A pointer to a string previously returned by `opencc_last_error`.
 *            Passing NULL is safe and does nothing.
 */
void opencc_error_free(char* ptr);

#ifdef __cplusplus
}
#endif

#endif // OPENCC_FMMSEG_CAPI_H
