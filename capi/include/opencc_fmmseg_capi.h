#ifndef OPENCC_FMMSEG_CAPI_H
#define OPENCC_FMMSEG_CAPI_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h> // uint32_t

// ============================================================================
// OpenCC config selector (ABI-stable)
// ============================================================================

/**
 * @typedef opencc_config_t
 *
 * ABI-stable configuration selector used by the opencc-fmmseg C API.
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
 * numeric-config conversion APIs such as `opencc_convert_cfg()`,
 * `opencc_convert_cfg_mem()`, and `opencc_convert_cfg_mem_len()`.
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
    OPENCC_CONFIG_T2JP = 16,

    /** Simplified → Traditional (Hong Kong, with phrases) */
    OPENCC_CONFIG_S2HKP = 17,
    /** Hong Kong Traditional → Simplified (with phrases) */
    OPENCC_CONFIG_HK2SP = 18,
    /** Traditional Chinese → Hong Kong variant (with phrases). */
    OPENCC_CONFIG_T2HKP = 19,
    /** Hong Kong variant → Traditional Chinese (with phrases). */
    OPENCC_CONFIG_HK2TP = 20
};

// ============================================================================
// Custom dictionary construction API
// ============================================================================

/**
 * @typedef opencc_dict_slot_t
 *
 * ABI-stable dictionary slot selector used when constructing an OpenCC
 * instance with custom dictionary entries.
 *
 * This type is a 32-bit unsigned integer for consistent representation across
 * C, C++, C#, Java, Python FFI, and other language bindings.
 *
 * Slot values are stable and will never be reordered or reused. New slot
 * values may be appended in future versions.
 *
 * This value is passed by value and requires no allocation or deallocation.
 *
 * @since
 *     Available since v0.11.5.
 */
typedef uint32_t opencc_dict_slot_t;

/**
 * Dictionary slots available for custom dictionary injection.
 *
 * Custom entries affect only the selected dictionary slot. Choosing the
 * correct slot is essential because each OpenCC conversion configuration
 * consumes a different combination of dictionary slots.
 *
 * @since
 *     Available since v0.11.5.
 */
enum {
    /** Simplified → Traditional character mappings. */
    OPENCC_DICT_SLOT_ST_CHARACTERS = 1,

    /** Simplified → Traditional phrase mappings. */
    OPENCC_DICT_SLOT_ST_PHRASES = 2,

    /** Traditional → Simplified character mappings. */
    OPENCC_DICT_SLOT_TS_CHARACTERS = 3,

    /** Traditional → Simplified phrase mappings. */
    OPENCC_DICT_SLOT_TS_PHRASES = 4,

    /** Traditional → Taiwan phrase mappings. */
    OPENCC_DICT_SLOT_TW_PHRASES = 5,

    /** Taiwan → Traditional reverse phrase mappings. */
    OPENCC_DICT_SLOT_TW_PHRASES_REV = 6,

    /** Traditional → Hong Kong phrase mappings. */
    OPENCC_DICT_SLOT_HK_PHRASES = 7,

    /** Hong Kong → Traditional reverse phrase mappings. */
    OPENCC_DICT_SLOT_HK_PHRASES_REV = 8,

    /** Traditional → Taiwan regional variant mappings. */
    OPENCC_DICT_SLOT_TW_VARIANTS = 9,

    /** Traditional → Taiwan regional phrase variant mappings. */
    OPENCC_DICT_SLOT_TW_VARIANTS_PHRASES = 10,

    /** Taiwan → Traditional reverse variant mappings. */
    OPENCC_DICT_SLOT_TW_VARIANTS_REV = 11,

    /** Taiwan → Traditional reverse phrase variant mappings. */
    OPENCC_DICT_SLOT_TW_VARIANTS_REV_PHRASES = 12,

    /** Traditional → Hong Kong regional variant mappings. */
    OPENCC_DICT_SLOT_HK_VARIANTS = 13,

    /** Traditional → Hong Kong regional phrase variant mappings. */
    OPENCC_DICT_SLOT_HK_VARIANTS_PHRASES = 14,

    /** Hong Kong → Traditional reverse variant mappings. */
    OPENCC_DICT_SLOT_HK_VARIANTS_REV = 15,

    /** Hong Kong → Traditional reverse phrase variant mappings. */
    OPENCC_DICT_SLOT_HK_VARIANTS_REV_PHRASES = 16,

    /** Japanese Shinjitai character mappings. */
    OPENCC_DICT_SLOT_JPS_CHARACTERS = 17,

    /** Japanese Shinjitai reverse character mappings. */
    OPENCC_DICT_SLOT_JPS_CHARACTERS_REV = 18,

    /** Japanese Shinjitai phrase mappings. */
    OPENCC_DICT_SLOT_JPS_PHRASES = 19,

    /** Simplified → Traditional punctuation mappings. */
    OPENCC_DICT_SLOT_ST_PUNCTUATIONS = 20,

    /** Traditional → Simplified punctuation mappings. */
    OPENCC_DICT_SLOT_TS_PUNCTUATIONS = 21
};

/**
 * @typedef opencc_custom_dict_mode_t
 *
 * ABI-stable custom dictionary merge mode.
 *
 * This type is a 32-bit unsigned integer. Mode values are stable and will
 * never be reordered or reused.
 *
 * @since
 *     Available since v0.11.5.
 */
typedef uint32_t opencc_custom_dict_mode_t;

/**
 * Controls how custom pairs are applied to a dictionary slot.
 *
 * @since
 *     Available since v0.11.5.
 */
enum {
    /**
     * Merge custom pairs into the built-in dictionary slot.
     *
     * Custom values replace existing values for matching source keys.
     * Existing unrelated mappings remain available.
     */
    OPENCC_CUSTOM_DICT_APPEND = 1,

    /**
     * Clear the selected built-in dictionary slot before inserting the
     * custom pairs.
     *
     * After construction, the selected slot contains only the supplied
     * custom mappings.
     */
    OPENCC_CUSTOM_DICT_OVERRIDE = 2
};

/**
 * One custom OpenCC dictionary mapping.
 *
 * Both strings must:
 *
 * - be valid null-terminated UTF-8 strings;
 * - remain valid for the duration of `opencc_new_custom()`;
 * - not contain embedded NUL bytes.
 *
 * The constructor copies both strings. The caller retains ownership of the
 * original memory.
 *
 * @since
 *     Available since v0.11.5.
 */
typedef struct opencc_custom_pair {
    /** Source dictionary key. */
    const char* source;

    /** Replacement dictionary value. */
    const char* target;
} opencc_custom_pair_t;

/**
 * Custom mappings targeting one OpenCC dictionary slot.
 *
 * `pairs` points to a contiguous array containing `pair_count` elements.
 *
 * The specification, pair array, and strings are borrowed only for the
 * duration of `opencc_new_custom()`. The constructor copies all required
 * data before returning.
 *
 * A NULL `pairs` pointer is valid only when `pair_count` is zero.
 *
 * @since
 *     Available since v0.11.5.
 */
typedef struct opencc_custom_dict_spec {
    /** Dictionary slot receiving these custom mappings. */
    opencc_dict_slot_t slot;

    /** Append or override behavior. */
    opencc_custom_dict_mode_t mode;

    /** Array of custom source-target mappings. */
    const opencc_custom_pair_t* pairs;

    /** Number of elements in `pairs`. */
    size_t pair_count;
} opencc_custom_dict_spec_t;

// ============================================================================
// Version / ABI
// ============================================================================

/**
 * Returns the C ABI version number.
 *
 * This value is intended for runtime compatibility checks.
 * It changes only when the C ABI is broken.
 */
uint32_t opencc_abi_number(void);

/**
 * Returns the opencc-fmmseg version string (null-terminated UTF-8).
 *
 * Example: `"0.9.1"` or `"0.9.1.1"`.
 *
 * The returned pointer is valid for the lifetime of the program and MUST NOT
 * be freed by the caller.
 */
const char* opencc_version_string(void);

// ============================================================================
// Instance lifetime
// ============================================================================

/**
 * Creates and initializes a new OpenCC FMMSEG instance.
 *
 * @return
 *     A pointer to a new OpenCC instance, or NULL if allocation fails.
 *
 * @ownership
 *     The returned instance must be released using `opencc_delete()`.
 */
void* opencc_new(void);

/**
 * Creates an immutable OpenCC FMMSEG instance using the embedded dictionaries
 * plus optional in-memory custom dictionary mappings.
 *
 * Custom mappings are applied during construction only. After this function
 * returns successfully, the resulting OpenCC instance is fully initialized
 * and its conversion dictionaries are immutable.
 *
 * Each specification targets one dictionary slot and uses either append or
 * override mode.
 *
 * The constructor copies all specifications, pairs, source strings, and
 * target strings required by the resulting instance. The caller may release
 * or reuse all input memory immediately after this function returns.
 *
 * Empty construction is supported:
 *
 *     opencc_new_custom(NULL, 0)
 *
 * is equivalent to `opencc_new()`.
 *
 * Invalid argument combinations include:
 *
 * - `specs == NULL` while `spec_count > 0`;
 * - an unknown dictionary slot;
 * - an unknown custom dictionary mode;
 * - `pairs == NULL` while `pair_count > 0`;
 * - a NULL source or target string;
 * - invalid UTF-8 in a source or target string.
 *
 * @param specs
 *     Pointer to a contiguous array of custom dictionary specifications.
 *     May be NULL only when `spec_count` is zero.
 *
 * @param spec_count
 *     Number of elements in `specs`.
 *
 * @return
 *     A pointer to a fully initialized immutable OpenCC instance on success.
 *
 *     Returns NULL on failure. A human-readable error message can then be
 *     retrieved using `opencc_last_error()`.
 *
 * @ownership
 *     The returned instance must be released using `opencc_delete()`.
 *
 *     The caller retains ownership of `specs`, all pair arrays, and all source
 *     and target strings.
 *
 * @since
 *     Available since v0.11.5.
 */
void* opencc_new_custom(
    const opencc_custom_dict_spec_t* specs,
    size_t spec_count
);

/**
 * Frees an instance returned by an OpenCC constructor.
 *
 * Passing NULL is safe and does nothing.
 *
 * @param instance
 *     A pointer previously returned by `opencc_new()` or
 *     `opencc_new_custom()`.
 */
void opencc_delete(const void* instance);

/**
 * @deprecated Use `opencc_delete()` instead.
 *
 * Frees an instance returned by `opencc_new()`.
 *
 * Passing NULL is safe and does nothing.
 *
 * IMPORTANT:
 * - Do not use this to free strings returned by `opencc_convert()`,
 *   `opencc_convert_cfg()`, or `opencc_last_error()`.
 * - Use `opencc_string_free()` or `opencc_error_free()` for string memory.
 *
 * @param instance
 *     A pointer previously returned by `opencc_new()`.
 */
void opencc_free(const void* instance);

// ============================================================================
// Instance options
// ============================================================================

/**
 * Returns whether parallel processing is enabled for the instance.
 *
 * @param instance
 *     A pointer to an OpenCC instance.
 *
 * @return
 *     `true` if parallel processing is enabled, `false` otherwise.
 *     Returns `false` if `instance` is NULL.
 */
bool opencc_get_parallel(const void* instance);

/**
 * Enables or disables parallel processing for the instance.
 *
 * If `instance` is NULL, this function does nothing.
 *
 * @param instance
 *     A pointer to an OpenCC instance.
 * @param is_parallel
 *     Set to `true` to enable parallel processing, or `false` to disable it.
 */
void opencc_set_parallel(void* instance, bool is_parallel);

// ============================================================================
// Conversion API (allocated return)
// ============================================================================

/**
 * Converts a null-terminated UTF-8 input string using a string config name.
 *
 * @param instance
 *     A pointer to the OpenCC instance created by `opencc_new()`.
 * @param input
 *     The input UTF-8 string to convert (null-terminated).
 * @param config
 *     The config name (for example `"s2t"` or `"t2s"`).
 * @param punctuation
 *     Whether to convert punctuation (`true` = convert).
 *
 * @return
 *     A newly allocated null-terminated UTF-8 string containing the converted
 *     output. The returned string must be freed using `opencc_string_free()`.
 *
 *     Returns NULL if `instance`, `input`, or `config` is NULL, or if allocation
 *     fails. In those cases the function records a human-readable message for
 *     retrieval via `opencc_last_error()`.
 *
 *     On UTF-8/config/conversion errors after argument validation, this function
 *     returns an allocated error message string and also stores the same message
 *     internally for retrieval via `opencc_last_error()`.
 */
char* opencc_convert(const void* instance, const char* input, const char* config, bool punctuation);

/**
 * Converts a null-terminated UTF-8 input string using a numeric OpenCC config.
 *
 * @param instance
 *     A pointer to the OpenCC instance created by `opencc_new()`.
 * @param input
 *     The input UTF-8 string to convert (null-terminated).
 * @param config
 *     The numeric config value (for example `OPENCC_CONFIG_S2TWP`).
 * @param punctuation
 *     Whether to convert punctuation (`true` = convert). Some configs may ignore it.
 *
 * @return
 *     A newly allocated null-terminated UTF-8 string containing the converted
 *     output. The returned string must be freed using `opencc_string_free()`.
 *
 *     If `config` is invalid, this function still returns a newly allocated
 *     error message string in the form `"Invalid config: <value>"`, and also
 *     stores the same message internally for retrieval via `opencc_last_error()`.
 *
 *     Returns NULL only if `instance` or `input` is NULL, or if allocation fails.
 *
 * @since
 *     Available since v0.8.4.
 */
char* opencc_convert_cfg(const void* instance, const char* input, opencc_config_t config, bool punctuation);

/**
 * @deprecated Planned for removal. Prefer `opencc_convert()` or `opencc_convert_cfg()`.
 *
 * Converts a UTF-8 input buffer with explicit byte length using a string config name.
 *
 * @param instance
 *     A pointer to the OpenCC instance.
 * @param input
 *     The input UTF-8 bytes. The buffer does not need to be null-terminated.
 * @param input_len
 *     The number of bytes in `input`.
 * @param config
 *     The config name (for example `"s2t"`).
 * @param punctuation
 *     Whether to convert punctuation (`true` = convert).
 *
 * @return
 *     A newly allocated null-terminated UTF-8 string containing the converted
 *     output. The returned string must be freed using `opencc_string_free()`.
 *
 *     Returns NULL if `config` is NULL, or if allocation fails. In those cases
 *     the function records a human-readable message for retrieval via
 *     `opencc_last_error()`.
 */
char* opencc_convert_len(
    const void* instance,
    const char* input,
    size_t input_len,
    const char* config,
    bool punctuation);

// ============================================================================
// Conversion API (caller-provided buffer)
// ============================================================================

/**
 * Converts a null-terminated UTF-8 input string using a numeric OpenCC config,
 * writing the result into a caller-provided buffer.
 *
 * This is an advanced API for bindings that specifically need caller-owned
 * output memory. Its size-query pattern performs conversion once to determine
 * the size and again to write the output. Prefer `opencc_convert()` or
 * `opencc_convert_cfg()` for ordinary conversion; use this API for its buffer
 * contract, not as a performance optimization.
 *
 * Size-query usage:
 *
 * 1) Call with `out_buf = NULL` or `out_cap = 0` to query required bytes
 *    (including the trailing `'\0'`):
 *
 *    `size_t required = 0;`
 *    `bool ok = opencc_convert_cfg_mem(inst, input, cfg, punct, NULL, 0, &required);`
 *
 * 2) Allocate a buffer of size `required`, then call again to write output:
 *
 *    `char* buf = (char*)malloc(required);`
 *    `ok = opencc_convert_cfg_mem(inst, input, cfg, punct, buf, required, &required);`
 *
 * Output contract:
 * - `out_required` must not be NULL.
 * - `*out_required` is always set to the required size in bytes, including the
 *   trailing `'\0'`, even when the function returns `false`, except when
 *   `out_required` itself is NULL.
 * - If this function returns `true`, the output is valid UTF-8 and null-terminated.
 *
 * @param instance
 *     A pointer to the OpenCC instance created by `opencc_new()`.
 * @param input
 *     The input UTF-8 string to convert (null-terminated).
 * @param config
 *     The numeric config value (for example `OPENCC_CONFIG_S2TWP`).
 * @param punctuation
 *     Whether to convert punctuation (`true` = convert). Some configs may ignore it.
 * @param out_buf
 *     Output buffer owned by the caller. May be NULL for size-query calls.
 * @param out_cap
 *     Capacity of `out_buf` in bytes.
 * @param out_required
 *     Output pointer that receives the required byte count including the
 *     trailing `'\0'`. Must not be NULL.
 *
 * @return
 *     `true` on success, including successful size-query calls.
 *     `false` on failure, including:
 *     - `out_required` is NULL
 *     - `instance` or `input` is NULL
 *     - invalid UTF-8 input
 *     - invalid config
 *     - output contains an interior NUL byte
 *     - `out_cap` is too small when `out_buf` is provided
 *
 * Error behavior:
 * - On failure, this function sets `opencc_last_error()` to a human-readable
 *   message, including when `out_required` is NULL.
 * - If the caller provides a buffer, the function may also attempt to write an error
 *   message into `out_buf` if the buffer is large enough.
 * - If the buffer is too small, the function returns `false`, sets `*out_required`,
 *   and sets `opencc_last_error()` to `"Output buffer too small"`.
 *
 * Ownership:
 * - `out_buf` is owned and freed by the caller.
 * - Do NOT call `opencc_string_free()` on `out_buf`.
 *
 * @since
 *     Available since v0.8.4.
 */
bool opencc_convert_cfg_mem(
    const void* instance,
    const char* input,
    opencc_config_t config,
    bool punctuation,
    char* out_buf,
    size_t out_cap,
    size_t* out_required);

/**
 * Converts a UTF-8 input buffer with explicit byte length using a numeric
 * OpenCC config, writing the result into a caller-provided buffer.
 *
 * This is the length-based companion to `opencc_convert_cfg_mem()`. It avoids
 * scanning `input` for a terminating `'\0'`, but still uses a size query and an
 * output pass. Prefer `opencc_convert()` or `opencc_convert_cfg()` for ordinary
 * conversion; use this API only when an explicit input length and caller-owned
 * output buffer are required.
 *
 * The input buffer does not need to be null-terminated.
 *
 * Size-query usage:
 *
 * 1) Call with `out_buf = NULL` or `out_cap = 0` to query required bytes
 *    (including the trailing `'\0'`):
 *
 *    `size_t required = 0;`
 *    `bool ok = opencc_convert_cfg_mem_len(inst, bytes, len, cfg, punct, NULL, 0, &required);`
 *
 * 2) Allocate a buffer of size `required`, then call again to write output:
 *
 *    `char* buf = (char*)malloc(required);`
 *    `ok = opencc_convert_cfg_mem_len(inst, bytes, len, cfg, punct, buf, required, &required);`
 *
 * Output contract:
 * - `out_required` must not be NULL.
 * - `*out_required` is always set to the required size in bytes, including the
 *   trailing `'\0'`, even when the function returns `false`, except when
 *   `out_required` itself is NULL.
 * - If this function returns `true`, the output is valid UTF-8 and null-terminated.
 *
 * @param instance
 *     A pointer to the OpenCC instance created by `opencc_new()`.
 * @param input
 *     Pointer to the input UTF-8 bytes. The buffer does not need to be null-terminated.
 * @param input_len
 *     Number of bytes in `input`.
 * @param config
 *     The numeric config value (for example `OPENCC_CONFIG_S2TWP`).
 * @param punctuation
 *     Whether to convert punctuation (`true` = convert). Some configs may ignore it.
 * @param out_buf
 *     Output buffer owned by the caller. May be NULL for size-query calls.
 * @param out_cap
 *     Capacity of `out_buf` in bytes.
 * @param out_required
 *     Output pointer that receives the required byte count including the
 *     trailing `'\0'`. Must not be NULL.
 *
 * @return
 *     `true` on success, including successful size-query calls.
 *     `false` on failure, including:
 *     - `out_required` is NULL
 *     - `instance` or `input` is NULL
 *     - invalid UTF-8 input
 *     - invalid config
 *     - output contains an interior NUL byte
 *     - `out_cap` is too small when `out_buf` is provided
 *
 * Error behavior and ownership are the same as `opencc_convert_cfg_mem()`,
 * including recording `"Invalid argument: out_required is NULL"` when
 * `out_required` is NULL.
 *
 * @since
 *     Available since v0.9.1.1.
 */
bool opencc_convert_cfg_mem_len(
    const void* instance,
    const char* input,
    size_t input_len,
    opencc_config_t config,
    bool punctuation,
    char* out_buf,
    size_t out_cap,
    size_t* out_required);

// ============================================================================
// Other API
// ============================================================================

/**
 * Checks whether the input appears to be Simplified or Traditional Chinese.
 *
 * @param instance
 *     A pointer to the OpenCC instance.
 * @param input
 *     The input UTF-8 string to inspect (null-terminated).
 *
 * @return
 *     An integer result code:
 *     - `0` = mixed / undetermined
 *     - `1` = Traditional Chinese
 *     - `2` = Simplified Chinese
 *     - `-1` = invalid input or NULL pointer
 */
int opencc_zho_check(const void* instance, const char* input);

// ============================================================================
// String memory API
// ============================================================================

/**
 * Frees a string returned by conversion functions such as `opencc_convert()`
 * or `opencc_convert_cfg()`.
 *
 * Passing NULL is safe and does nothing.
 *
 * @param ptr
 *     A pointer previously returned by a conversion function.
 */
void opencc_string_free(char* ptr);

// ============================================================================
// Error API
// ============================================================================

/**
 * Returns the last error message as a newly allocated null-terminated UTF-8 string.
 *
 * The returned string must be freed using `opencc_error_free()`.
 * If there is no recorded error, this function returns `"No error"`.
 *
 * @return
 *     A heap-allocated error message string.
 */
char* opencc_last_error(void);

/**
 * Clears the internally stored last error message.
 *
 * This function resets internal error state only. It does NOT free any memory
 * previously returned by `opencc_last_error()`.
 *
 * After calling this function, `opencc_last_error()` returns `"No error"`
 * until a new error is recorded.
 *
 * @since
 *     Available since v0.8.4.
 */
void opencc_clear_last_error(void);

/**
 * Frees a string returned by `opencc_last_error()`.
 *
 * Passing NULL is safe and does nothing.
 *
 * @param ptr
 *     A pointer previously returned by `opencc_last_error()`.
 */
void opencc_error_free(char* ptr);

// ============================================================================
// Config enum FFI helpers
// ============================================================================

/**
 * Converts a canonical OpenCC configuration name to its numeric ID.
 *
 * This function maps a UTF-8 configuration name such as `"s2t"`, `"s2tw"`,
 * `"s2twp"`, or `"s2hkp"` to the corresponding numeric `opencc_config_t` value.
 *
 * The comparison is case-insensitive and accepts only canonical OpenCC
 * identifiers. No memory allocation is performed.
 *
 * @param name_utf8
 *     A null-terminated UTF-8 string containing the canonical OpenCC
 *     configuration name.
 * @param out_id
 *     Output pointer that receives the numeric configuration ID on success.
 *
 * @return
 *     `true` on success, `false` on failure.
 *
 * @since
 *     Available since v0.8.4.
 */
bool opencc_config_name_to_id(const char* name_utf8, opencc_config_t* out_id);

/**
 * Converts a numeric OpenCC configuration ID to its canonical config name.
 *
 * The returned pointer refers to a static null-terminated UTF-8 string and
 * remains valid for the lifetime of the program. The caller must not modify
 * or free it.
 *
 * @param id
 *     A numeric OpenCC configuration ID.
 *
 * @return
 *     A pointer to the canonical lowercase configuration name, or NULL if `id`
 *     is not valid.
 *
 * @since
 *     Available since v0.8.4.
 */
const char* opencc_config_id_to_name(opencc_config_t id);

#ifdef __cplusplus
}
#endif

#endif // OPENCC_FMMSEG_CAPI_H

