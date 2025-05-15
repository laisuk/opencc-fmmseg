#ifndef OPENCC_FMMSEG_CAPI_H
#define OPENCC_FMMSEG_CAPI_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stdbool.h>
#include <stddef.h>

/**
 * Creates and initializes a new OpenCC FMMSEG instance.
 *
 * This function allocates and returns a new instance used for conversion.
 * The instance should be freed using `opencc_free()` when no longer needed.
 *
 * @return A pointer to a new instance of OpenCC FMMSEG.
 */
void *opencc_new();

/*
// Reserved for future use.
// Creates a new OpenCC FMMSEG instance from custom dictionaries.
void *opencc_new_from_dicts();
*/

/**
 * Converts a null-terminated UTF-8 input string using the specified OpenCC config.
 *
 * @param instance     A pointer to the OpenCC instance created by `opencc_new()`.
 * @param input        The input UTF-8 string to convert.
 * @param config       The config name (e.g., "s2t", "t2s") for conversion rules.
 * @param punctuation  Whether to convert punctuation (true = convert).
 *
 * @return A newly allocated UTF-8 string with the converted output.
 *         The result must be freed using `opencc_string_free()`.
 */
char *opencc_convert(const void *instance, const char *input, const char *config, bool punctuation);

/**
 * Converts a UTF-8 string with explicit length using the specified OpenCC config.
 *
 * @param instance     A pointer to the OpenCC instance.
 * @param input        The input UTF-8 string (not necessarily null-terminated).
 * @param input_len    The number of bytes in the input string.
 * @param config       The config name (e.g., "s2t") for conversion rules.
 * @param punctuation  Whether to convert punctuation (true = convert).
 *
 * @return A newly allocated UTF-8 string with the converted output.
 *         The result must be freed using `opencc_string_free()`.
 */
char *opencc_convert_len(
    const void *instance,
    const char *input,
    size_t input_len,
    const char *config,
    bool punctuation);

/**
 * Checks if parallel processing is enabled in the instance.
 *
 * @param instance A pointer to the OpenCC instance.
 * @return true if parallel processing is enabled, false otherwise.
 */
bool opencc_get_parallel(const void *instance);

/**
 * Enables or disables parallel processing for the instance.
 *
 * @param instance     A pointer to the OpenCC instance.
 * @param is_parallel  Set to true to enable parallel processing, false to disable.
 */
void opencc_set_parallel(const void *instance, bool is_parallel);

/**
 * Checks if the input string is valid Simplified or Traditional Chinese.
 *
 * @param instance A pointer to the OpenCC instance.
 * @param input    The input UTF-8 string to check.
 * @return An integer code indicating the check result:
 *         0 = Mixed/Undetermined,
 *         1 = Traditional Chinese,
 *         2 = Simplified Chinese,
 *         -1 = Invalid.
 */
int opencc_zho_check(const void *instance, const char *input);

/**
 * Frees an instance of OpenCC returned by `opencc_new`.
 *
 * @param instance A pointer to an OpenCC instance.
 *                 Passing NULL is safe and does nothing.
 */
void opencc_delete(const void *instance);

/**
 * @deprecated Use `opencc_delete()` instead.
 *
 * Frees an instance of OpenCC returned by `opencc_new`.
 *
 * NOTE: Do not use this to free strings returned by `opencc_convert` or `opencc_last_error`.
 * Use `opencc_string_free` or `opencc_error_free` instead.
 */
void opencc_free(const void *instance);


/**
 * Frees a string returned by `opencc_convert` or `opencc_convert_len`.
 *
 * @param ptr A pointer to a string previously returned by conversion functions.
 *            Passing NULL is safe and does nothing.
 */
void opencc_string_free(const char *ptr);

/**
 * Returns the last error message as a null-terminated C string.
 *
 * The returned string is dynamically allocated and must be freed
 * using `opencc_error_free()`. If there is no error, returns "No error".
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
