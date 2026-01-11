#ifndef OPENCC_FMMSEG_CAPI_H
#define OPENCC_FMMSEG_CAPI_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h> // uint32_t

// -----------------------------------------------------------------------------
// OpenCC config selector (ABI-stable)
// -----------------------------------------------------------------------------

/**
 * @typedef opencc_config_t
 *
 * ABI-stable configuration selector used by opencc-fmmseg C API.
 *
 * This type is a 32-bit unsigned integer to maximize compatibility across
 * C / C++ / C# / Java / Python FFI. Values are stable and will never be
 * reordered. New values may be added in future versions.
 *
 * This parameter is passed by value and does NOT require allocation or
 * deallocation by the caller.
 *
 * @since
 *     Available since v0.8.4.
 */
typedef uint32_t opencc_config_t;

/**
 * OpenCC conversion configurations (numeric).
 *
 * These constants are intended to be passed as `opencc_config_t` to
 * `opencc_convert_cfg()`.
 *
 * @since
 *     Available since v0.8.4.
 */
enum {
    /** Simplified Chinese → Traditional Chinese */
    OPENCC_CONFIG_S2T = 1,
    /** Simplified → Traditional (Taiwan) */
    OPENCC_CONFIG_S2TW = 2,
    /** Simplified → Traditional (Taiwan, with phrases) */
    OPENCC_CONFIG_S2TWP = 3,
    /** Simplified → Traditional (Hong Kong) */
    OPENCC_CONFIG_S2HK = 4,

    /** Traditional Chinese → Simplified Chinese */
    OPENCC_CONFIG_T2S = 5,
    /** Traditional → Taiwan Traditional */
    OPENCC_CONFIG_T2TW = 6,
    /** Traditional → Taiwan Traditional (with phrases) */
    OPENCC_CONFIG_T2TWP = 7,
    /** Traditional → Hong Kong Traditional */
    OPENCC_CONFIG_T2HK = 8,

    /** Taiwan Traditional → Simplified */
    OPENCC_CONFIG_TW2S = 9,
    /** Taiwan Traditional → Simplified (variant) */
    OPENCC_CONFIG_TW2SP = 10,
    /** Taiwan Traditional → Traditional */
    OPENCC_CONFIG_TW2T = 11,
    /** Taiwan Traditional → Traditional (variant) */
    OPENCC_CONFIG_TW2TP = 12,

    /** Hong Kong Traditional → Simplified */
    OPENCC_CONFIG_HK2S = 13,
    /** Hong Kong Traditional → Traditional */
    OPENCC_CONFIG_HK2T = 14,

    /** Japanese Kanji variants → Traditional Chinese */
    OPENCC_CONFIG_JP2T = 15,
    /** Traditional Chinese → Japanese Kanji variants */
    OPENCC_CONFIG_T2JP = 16
};

/**
 * Returns the C ABI version number.
 *
 * This value is intended for runtime compatibility checks.
 * It only changes when the C ABI is broken.
 */
uint32_t opencc_abi_number(void);

/**
 * Returns the OpenCC-FMMSEG version string (null-terminated UTF-8).
 *
 * Example: "0.8.4"
 * The returned pointer is valid for the lifetime of the program and MUST NOT be freed.
 */
const char* opencc_version_string(void);

/**
 * Creates and initializes a new OpenCC FMMSEG instance.
 *
 * This function allocates and returns a new instance used for conversion.
 * The instance should be freed using `opencc_delete()` when no longer needed.
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
 * Converts a null-terminated UTF-8 input string using the specified OpenCC config (string name).
 *
 * @param instance     A pointer to the OpenCC instance created by `opencc_new()`.
 * @param input        The input UTF-8 string to convert.
 * @param config       The config name (e.g., "s2t", "t2s") for conversion rules.
 * @param punctuation  Whether to convert punctuation (true = convert).
 *
 * @return A newly allocated NUL-terminated UTF-8 string with the converted output.
 *         The result must be freed using `opencc_string_free()`.
 */
char *opencc_convert(const void *instance, const char *input, const char *config, bool punctuation);

/**
 * Converts a null-terminated UTF-8 input string using a numeric OpenCC config.
 *
 * @param instance     A pointer to the OpenCC instance created by `opencc_new()`.
 * @param input        The input UTF-8 string to convert.
 * @param config       The numeric config value (e.g., `OPENCC_CONFIG_S2TWP`).
 * @param punctuation  Whether to convert punctuation (true = convert). Some configs may ignore it.
 *
 * @return A newly allocated NUL-terminated UTF-8 string with the converted output.
 *         The result must be freed using `opencc_string_free()`.
 *
 *         If `config` is invalid, this function still returns a newly allocated
 *         error message string in the form "Invalid config: <value>", and also
 *         stores the same message internally (retrievable via `opencc_last_error()`).
 *
 *         Returns NULL only if `instance` or `input` is NULL, or if memory allocation fails.
 *
 * @since
 *     Available since v0.8.4.
 */
char *opencc_convert_cfg(const void *instance, const char *input, opencc_config_t config, bool punctuation);

/**
 * @deprecated Planned for removal. Prefer `opencc_convert()` or `opencc_convert_cfg()`.
 *
 * Converts a UTF-8 string with explicit length using the specified OpenCC config.
 *
 * @param instance     A pointer to the OpenCC instance.
 * @param input        The input UTF-8 string (not necessarily null-terminated).
 * @param input_len    The number of bytes in the input string.
 * @param config       The config name (e.g., "s2t") for conversion rules.
 * @param punctuation  Whether to convert punctuation (true = convert).
 *
 * @return A newly allocated NUL-terminated UTF-8 string with the converted output.
 *         The result must be freed using `opencc_string_free()`.
 */
char *opencc_convert_len(
    const void *instance,
    const char *input,
    size_t input_len,
    const char *config,
    bool punctuation);

/**
 * Converts a null-terminated UTF-8 input string using a numeric OpenCC config,
 * writing the result into a caller-provided buffer.
 *
 * This is an advanced API for bindings / performance-sensitive code that wants
 * to reuse memory. Because the output length is variable, this function uses a
 * size-query pattern.
 *
 * Size-query usage:
 *  1) Call with out_buf = NULL or out_cap = 0 to query required bytes (incl. '\0'):
 *       size_t required = 0;
 *       bool ok = opencc_convert_cfg_mem(inst, input, cfg, punct, NULL, 0, &required);
 *       // ok == true means size-query succeeded (required is valid)
 *  2) Allocate a buffer of size `required`, then call again to write output:
 *       char* buf = (char*)malloc(required);
 *       ok = opencc_convert_cfg_mem(inst, input, cfg, punct, buf, required, &required);
 *
 * Output contract:
 * - If `out_required` is non-NULL, this function ALWAYS writes the required size
 *   (in bytes, INCLUDING the trailing '\0'), even when the function returns false.
 * - The output is always UTF-8 with a trailing '\0' when the function returns true.
 *
 * @param instance      A pointer to the OpenCC instance created by `opencc_new()`.
 * @param input         The input UTF-8 string to convert (null-terminated).
 * @param config        The numeric config value (e.g., `OPENCC_CONFIG_S2TWP`).
 * @param punctuation   Whether to convert punctuation (true = convert). Some configs may ignore it.
 * @param out_buf       Output buffer (caller-owned). May be NULL to query size.
 * @param out_cap       Output buffer capacity in bytes.
 * @param out_required  [out] Required bytes INCLUDING the trailing '\0'. Must not be NULL.
 *
 * @return true  on success, including size-query calls (out_buf == NULL or out_cap == 0).
 *         false on failure, including:
 *               - out_required is NULL
 *               - instance/input is NULL
 *               - invalid UTF-8 input
 *               - invalid config
 *               - output contains an interior NUL byte
 *               - out_cap is too small when out_buf is provided
 *
 * Error behavior:
 * - On failure, this function sets `opencc_last_error()` to a human-readable message.
 * - If the caller provides a buffer, the function may also attempt to write an error
 *   message into `out_buf` (e.g., "Invalid config: 9999"), provided the buffer is large enough.
 *   Regardless, failure cases return false.
 * - If the buffer is too small (including for writing an error message), the function returns false,
 *   sets `*out_required`, and sets `opencc_last_error()` to "Output buffer too small".
 *
 * Ownership:
 * - The output buffer is owned and freed by the caller (e.g., free()).
 * - Do NOT call `opencc_string_free()` on out_buf.
 *
 * @since
 *     Available since v0.8.4.
 */
bool opencc_convert_cfg_mem(
    const void *instance,
    const char *input,
    opencc_config_t config,
    bool punctuation,
    char *out_buf,
    size_t out_cap,
    size_t *out_required);

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
 * Frees an instance of OpenCC returned by `opencc_new()`.
 *
 * @param instance A pointer to an OpenCC instance.
 *                 Passing NULL is safe and does nothing.
 */
void opencc_delete(const void *instance);

/**
 * @deprecated Use `opencc_delete()` instead.
 *
 * Frees an instance of OpenCC returned by `opencc_new()`.
 *
 * NOTE: Do not use this to free strings returned by `opencc_convert`,
 * `opencc_convert_cfg`, or `opencc_last_error`.
 * Use `opencc_string_free()` or `opencc_error_free()` instead.
 */
void opencc_free(const void *instance);

/**
 * Frees a string returned by conversion functions.
 *
 * @param ptr A pointer to a string previously returned by conversion functions.
 *            Passing NULL is safe and does nothing.
 */
void opencc_string_free(char *ptr);

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
 * Clears the internally stored last error message.
 *
 * This function resets the OpenCC internal error state.
 * After calling this, `opencc_last_error()` will return "No error"
 * until a new error is recorded.
 *
 * IMPORTANT:
 * - This function does NOT free any memory previously returned by
 *   `opencc_last_error()`.
 * - Any string returned by `opencc_last_error()` must still be freed
 *   explicitly using `opencc_error_free()`.
 *
 * In other words:
 * - `opencc_clear_last_error()` clears internal state.
 * - `opencc_error_free()` releases heap memory owned by the caller.
 */
void opencc_clear_last_error(void);

/**
 * Frees a string returned by `opencc_last_error()`.
 *
 * @param ptr A pointer to a string previously returned by `opencc_last_error()`.
 *            Passing NULL is safe and does nothing.
 */
void opencc_error_free(char *ptr);

/**
 * Converts an OpenCC canonical configuration name to its numeric configuration ID.
 *
 * This function maps a UTF-8 configuration name such as `"s2t"`, `"s2tw"`, or `"s2twp"`
 * to the corresponding numeric `opencc_config_t` value used by the OpenCC core.
 *
 * The comparison is case-insensitive and accepts only the canonical OpenCC
 * identifiers. No memory allocation is performed.
 *
 * This API is intended for language bindings and applications that accept
 * user-facing string configuration names but must call the native OpenCC
 * API using numeric configuration IDs.
 *
 * @param name_utf8
 *     A null-terminated UTF-8 string containing the canonical OpenCC
 *     configuration name (e.g. `"s2twp"`).
 *
 * @param out_id
 *     Output pointer that receives the corresponding numeric configuration ID
 *     on success.
 *
 * @return
 *     Returns `true` on success.
 *     Returns `false` if `name_utf8` is NULL, `out_id` is NULL, or the name
 *     is not a valid OpenCC configuration identifier.
 *
 * @since
 *     Available since v0.8.4.
 */
bool opencc_config_name_to_id(const char* name_utf8, opencc_config_t* out_id);

/**
 * Converts a numeric OpenCC configuration ID to its canonical configuration name.
 *
 * This function returns the canonical, lowercase OpenCC configuration name
 * (e.g. `"s2twp"`) corresponding to a numeric `opencc_config_t` value.
 *
 * The returned pointer refers to a static, null-terminated UTF-8 string and
 * remains valid for the lifetime of the program. The caller must not modify
 * or free the returned string.
 *
 * This API is primarily intended for debugging, logging, and user interface
 * display purposes.
 *
 * @param id
 *     A numeric OpenCC configuration ID (`opencc_config_t`).
 *
 * @return
 *     A pointer to a static UTF-8 string containing the canonical configuration
 *     name, or NULL if `id` is not a valid OpenCC configuration value.
 *
 * @since
 *     Available since v0.8.4.
 */
const char* opencc_config_id_to_name(opencc_config_t id);

#ifdef __cplusplus
}
#endif

#endif // OPENCC_FMMSEG_CAPI_H
