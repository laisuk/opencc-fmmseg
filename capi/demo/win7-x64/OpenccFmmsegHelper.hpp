#pragma once

#include "opencc_fmmseg_capi.h"

#include <cctype>
#include <stdexcept>
#include <string>
#include <string_view>
#include <utility>

// RAII convenience wrapper around the opencc-fmmseg C API.
//
// This helper owns exactly one native OpenCC instance and releases it with
// `opencc_delete()` in the destructor. It keeps the native API's conversion
// behavior visible to C++ callers:
// - Invalid config ids are passed through to the C API.
// - Config names are preserved as provided so invalid names surface as
//   `"Invalid config: ..."` instead of silently falling back to `s2t`.
// - Conversion methods return the native result as a `std::string`; native
//   error strings are returned as ordinary text rather than throwing.
// - If the native API returns `NULL` or a buffer conversion call fails, the
//   helper returns `lastError()` so failures are distinguishable from valid
//   empty-input results.
class OpenccFmmsegHelper {
public:
    // Creates a new native OpenCC instance.
    //
    // Throws `std::runtime_error` only if `opencc_new()` fails.
    OpenccFmmsegHelper()
        : opencc_(opencc_new()) {
        if (!opencc_)
            throw std::runtime_error("Failed to initialize OpenCC instance.");
    }

    OpenccFmmsegHelper(const OpenccFmmsegHelper &) = delete;

    OpenccFmmsegHelper &operator=(const OpenccFmmsegHelper &) = delete;

    OpenccFmmsegHelper(OpenccFmmsegHelper &&other) noexcept
        : opencc_(std::exchange(other.opencc_, nullptr)),
          configId_(other.configId_),
          punctuationEnabled_(other.punctuationEnabled_),
          configName_(std::move(other.configName_)),
          useConfigName_(other.useConfigName_) {
        other.useConfigName_ = false;
    }

    OpenccFmmsegHelper &operator=(OpenccFmmsegHelper &&other) noexcept {
        if (this != &other) {
            cleanup();
            opencc_ = std::exchange(other.opencc_, nullptr);
            configId_ = other.configId_;
            punctuationEnabled_ = other.punctuationEnabled_;
            configName_ = std::move(other.configName_);
            useConfigName_ = other.useConfigName_;
            other.useConfigName_ = false;
        }
        return *this;
    }

    ~OpenccFmmsegHelper() noexcept { cleanup(); }

    // Stores the active numeric config for the stateful overloads.
    // Invalid ids are preserved so the next conversion surfaces the native
    // `Invalid config: <id>` error instead of silently changing behavior.
    void setConfigId(const opencc_config_t configId) noexcept {
        configId_ = configId;
        configName_.clear();
        useConfigName_ = false;
    }

    [[nodiscard]] opencc_config_t getConfigId() const noexcept { return configId_; }

    // Stores the active config name for the stateful overloads.
    // The exact text is preserved so invalid names surface through the same
    // error message as the C API.
    void setConfig(const std::string_view cfgName) {
        configName_.assign(cfgName.data(), cfgName.size());
        useConfigName_ = true;

        opencc_config_t parsed = 0;
        configId_ = lookupConfigId(cfgName, parsed) ? parsed : 0;
    }

    // Stores the punctuation-conversion flag for the stateful overloads.
    void setPunctuation(const bool enable) noexcept { punctuationEnabled_ = enable; }
    [[nodiscard]] bool punctuationEnabled() const noexcept { return punctuationEnabled_; }

    // ---------------------------
    // Easy/default APIs
    // ---------------------------

    // Converts using an explicit numeric config.
    [[nodiscard]] std::string convert_cfg(const std::string_view input,
                                          const opencc_config_t configId,
                                          const bool punctuation = false) const {
        if (input.empty()) return {};
        return convertByCfg(input, configId, punctuation);
    }

    // Converts using the stored config source and punctuation flag.
    [[nodiscard]] std::string convert_cfg(const std::string_view input) const {
        if (input.empty()) return {};
        if (useConfigName_) {
            return convertByName(input, configName_, punctuationEnabled_);
        }
        return convertByCfg(input, configId_, punctuationEnabled_);
    }

    // Converts using an explicit config name.
    [[nodiscard]] std::string convert(const std::string_view input,
                                      const std::string_view configName,
                                      const bool punctuation = false) const {
        if (input.empty()) return {};
        return convertByName(input, configName, punctuation);
    }

    // Converts using the stored config/punctuation state.
    [[nodiscard]] std::string convert(const std::string_view input) const {
        return convert_cfg(input);
    }

    // ---------------------------
    // Advanced buffer-based APIs
    // ---------------------------

    // Stateless explicit-length conversion (advanced).
    //
    // Wraps opencc_convert_cfg_mem_len().
    // This API avoids scanning for '\0' and works directly on byte spans.
    //
    // Note:
    // - Not guaranteed to be faster than convert_cfg().
    // - Uses a size-query + write pattern (2 native calls).
    // - Intended for interop / explicit buffer workflows.
    [[nodiscard]] std::string convert_cfg_mem_len(
        const std::string_view input,
        const opencc_config_t configId,
        const bool punctuation = false
    ) const {
        if (input.empty()) return {};
        return convertByCfgMemLen(input, configId, punctuation);
    }

    // Stateful version (uses stored config/punctuation).
    [[nodiscard]] std::string convert_cfg_mem_len(const std::string_view input) const {
        if (input.empty()) return {};
        if (useConfigName_) {
            opencc_config_t parsed = 0;
            if (!lookupConfigId(configName_, parsed)) {
                return invalidConfigMessage(configName_);
            }
            return convertByCfgMemLen(input, parsed, punctuationEnabled_);
        }
        return convertByCfgMemLen(input, configId_, punctuationEnabled_);
    }

    // Convenience overload using a config name.
    [[nodiscard]] std::string convert_mem_len(
        const std::string_view input,
        const std::string_view configName,
        const bool punctuation = false
    ) const {
        if (input.empty()) return {};

        opencc_config_t parsed = 0;
        if (!lookupConfigId(configName, parsed)) {
            return invalidConfigMessage(configName);
        }

        return convertByCfgMemLen(input, parsed, punctuation);
    }

    // Checks whether the input appears simplified or traditional.
    // Returns 0 for empty input without calling the C API.
    [[nodiscard]] int zhoCheck(const std::string_view input) const {
        if (input.empty()) return 0;
        const std::string tmp(input);
        return opencc_zho_check(opencc_, tmp.c_str());
    }

    // Returns the current native last-error string.
    // This mirrors `opencc_last_error()`: when no error is recorded, the result
    // is typically "No error".
    [[nodiscard]] static std::string lastError() {
        char *err = opencc_last_error();
        if (!err) return {};
        std::string result(err);
        opencc_error_free(err);
        return result;
    }

    // Clears the native last-error state.
    static void clearLastError() noexcept {
        opencc_clear_last_error();
    }

    // Converts a config name to a numeric id.
    // Returns 0 when the name is invalid.
    [[nodiscard]] static opencc_config_t
    config_name_to_id(const std::string_view name) noexcept {
        opencc_config_t id = 0;
        return lookupConfigId(name, id) ? id : 0;
    }

    // Converts a numeric config id to its canonical lowercase name.
    // Returns an empty view for invalid ids.
    [[nodiscard]] static std::string_view
    config_id_to_name(const opencc_config_t configId) noexcept {
        return configIdToName(configId);
    }

private:
    void *opencc_ = nullptr;
    opencc_config_t configId_ = OPENCC_CONFIG_S2T;
    bool punctuationEnabled_ = false;
    std::string configName_;
    bool useConfigName_ = false;

    static void cleanupOpencc(void *p) noexcept {
        if (p) opencc_delete(p);
    }

    void cleanup() noexcept {
        cleanupOpencc(opencc_);
        opencc_ = nullptr;
    }

    [[nodiscard]] static bool lookupConfigId(
        const std::string_view name,
        opencc_config_t &outId
    ) noexcept {
        const std::string owned(name);
        opencc_config_t parsed = 0;
        const bool ok = opencc_config_name_to_id(owned.c_str(), &parsed);
        if (ok) {
            outId = parsed;
        }
        return ok;
    }

    [[nodiscard]] static std::string_view configIdToName(const opencc_config_t id) noexcept {
        const char *name = opencc_config_id_to_name(id);
        if (!name) return {};
        return name;
    }

    [[nodiscard]] static std::string invalidConfigMessage(const std::string_view configName) {
        return std::string("Invalid config: ") + std::string(configName);
    }

    [[nodiscard]] static std::string takeLastErrorText() {
        return lastError();
    }

    // Low-level bridge used by the string-returning conversion helpers.
    // The returned string may be converted text or a native error message.
    [[nodiscard]] std::string convertByCfg(const std::string_view input,
                                           const opencc_config_t cfg,
                                           const bool punctuation) const {
        const std::string in(input);
        char *output = opencc_convert_cfg(opencc_, in.c_str(), cfg, punctuation);
        if (!output) return takeLastErrorText();

        std::string result(output);
        opencc_string_free(output);
        return result;
    }

    // String-config bridge that preserves the exact config text, including
    // invalid names, so the wrapper surfaces the same message as the C API.
    [[nodiscard]] std::string convertByName(const std::string_view input,
                                            const std::string_view configName,
                                            const bool punctuation) const {
        const std::string in(input);
        const std::string cfg(configName);
        char *output = opencc_convert(opencc_, in.c_str(), cfg.c_str(), punctuation);
        if (!output) return takeLastErrorText();

        std::string result(output);
        opencc_string_free(output);
        return result;
    }

    // Low-level bridge for the explicit-length buffer API.
    // The helper performs a size query followed by a write call and returns the
    // native last-error text if either native step fails.
    [[nodiscard]] std::string convertByCfgMemLen(
        const std::string_view input,
        const opencc_config_t cfg,
        const bool punctuation
    ) const {
        if (input.empty()) return {};

        size_t required = 0;

        const bool ok_query = opencc_convert_cfg_mem_len(
            opencc_,
            input.data(),
            input.size(),
            cfg,
            punctuation,
            nullptr,
            0,
            &required);

        if (!ok_query || required == 0) {
            return takeLastErrorText();
        }

        std::string output;
        output.resize(required);

        const bool ok_write = opencc_convert_cfg_mem_len(
            opencc_,
            input.data(),
            input.size(),
            cfg,
            punctuation,
            output.data(),
            output.size(),
            &required);

        if (!ok_write || required == 0) {
            return takeLastErrorText();
        }

        output.resize(required - 1);
        return output;
    }
};
