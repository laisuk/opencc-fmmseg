#pragma once

#include "opencc_fmmseg_capi.h"

#include <cctype>
#include <stdexcept>
#include <string>
#include <string_view>
#include <utility>

class OpenccFmmsegHelper {
public:
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
          punctuationEnabled_(other.punctuationEnabled_) {
    }

    OpenccFmmsegHelper &operator=(OpenccFmmsegHelper &&other) noexcept {
        if (this != &other) {
            cleanup();
            opencc_ = std::exchange(other.opencc_, nullptr);
            configId_ = other.configId_;
            punctuationEnabled_ = other.punctuationEnabled_;
        }
        return *this;
    }

    ~OpenccFmmsegHelper() noexcept { cleanup(); }

    void setConfigId(const opencc_config_t configId) noexcept {
        configId_ = isValidConfigId(configId) ? configId : OPENCC_CONFIG_S2T;
    }

    [[nodiscard]] opencc_config_t getConfigId() const noexcept { return configId_; }

    void setConfig(const std::string_view cfgName) {
        configId_ = configNameToId(cfgName);
    }

    void setPunctuation(const bool enable) noexcept { punctuationEnabled_ = enable; }
    [[nodiscard]] bool punctuationEnabled() const noexcept { return punctuationEnabled_; }

    // ---------------------------
    // Easy/default APIs
    // ---------------------------

    [[nodiscard]] std::string convert_cfg(const std::string_view input,
                                          const opencc_config_t configId,
                                          const bool punctuation = false) const {
        if (input.empty()) return {};
        return convertByCfg(input, configId, punctuation);
    }

    [[nodiscard]] std::string convert_cfg(const std::string_view input) const {
        if (input.empty()) return {};
        return convertByCfg(input, configId_, punctuationEnabled_);
    }

    [[nodiscard]] std::string convert(const std::string_view input,
                                      const std::string_view configName,
                                      const bool punctuation = false) const {
        if (input.empty()) return {};
        return convertByCfg(input, configNameToId(configName), punctuation);
    }

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
    // ⚠️ Note:
    // - Not guaranteed to be faster than convert_cfg().
    // - Uses a size-query + write pattern (2 native calls).
    // - Intended for interop / explicit buffer workflows.
    //
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
        return convertByCfgMemLen(input, configId_, punctuationEnabled_);
    }

    // Convenience overload using config name.
    [[nodiscard]] std::string convert_mem_len(
        const std::string_view input,
        const std::string_view configName,
        const bool punctuation = false
    ) const {
        if (input.empty()) return {};
        return convertByCfgMemLen(input, configNameToId(configName), punctuation);
    }

    [[nodiscard]] int zhoCheck(const std::string_view input) const {
        if (input.empty()) return 0;
        const std::string tmp(input);
        return opencc_zho_check(opencc_, tmp.c_str());
    }

    [[nodiscard]] static std::string lastError() {
        char *err = opencc_last_error();
        if (!err) return {};
        std::string result(err);
        opencc_error_free(err);
        return result;
    }

    static void clearLastError() noexcept {
        opencc_clear_last_error();
    }

    [[nodiscard]] static opencc_config_t
    config_name_to_id(const std::string_view name) noexcept {
        return configNameToId(name);
    }

    [[nodiscard]] static std::string_view
    config_id_to_name(const opencc_config_t configId) noexcept {
        return configIdToName(configId);
    }

private:
    void *opencc_ = nullptr;
    opencc_config_t configId_ = OPENCC_CONFIG_S2T;
    bool punctuationEnabled_ = false;

    // NOLINTNEXTLINE(readability-non-const-parameter)
    static void cleanupOpencc(void *p) noexcept {
        if (p) opencc_delete(p);
    }

    void cleanup() noexcept {
        cleanupOpencc(opencc_);
        opencc_ = nullptr;
    }

    [[nodiscard]] static bool isValidConfigId(const opencc_config_t cfg) noexcept {
        return cfg >= OPENCC_CONFIG_S2T && cfg <= OPENCC_CONFIG_T2JP;
    }

    [[nodiscard]] static opencc_config_t configNameToId(const std::string_view s) {
        std::string t;
        t.reserve(s.size());
        for (const unsigned char ch: s)
            t.push_back(static_cast<char>(std::tolower(ch)));

        if (t == "s2t") return OPENCC_CONFIG_S2T;
        if (t == "s2tw") return OPENCC_CONFIG_S2TW;
        if (t == "s2twp") return OPENCC_CONFIG_S2TWP;
        if (t == "s2hk") return OPENCC_CONFIG_S2HK;
        if (t == "t2s") return OPENCC_CONFIG_T2S;
        if (t == "t2tw") return OPENCC_CONFIG_T2TW;
        if (t == "t2twp") return OPENCC_CONFIG_T2TWP;
        if (t == "t2hk") return OPENCC_CONFIG_T2HK;
        if (t == "tw2s") return OPENCC_CONFIG_TW2S;
        if (t == "tw2sp") return OPENCC_CONFIG_TW2SP;
        if (t == "tw2t") return OPENCC_CONFIG_TW2T;
        if (t == "tw2tp") return OPENCC_CONFIG_TW2TP;
        if (t == "hk2s") return OPENCC_CONFIG_HK2S;
        if (t == "hk2t") return OPENCC_CONFIG_HK2T;
        if (t == "jp2t") return OPENCC_CONFIG_JP2T;
        if (t == "t2jp") return OPENCC_CONFIG_T2JP;

        return OPENCC_CONFIG_S2T;
    }

    [[nodiscard]] static std::string_view configIdToName(const opencc_config_t id) {
        switch (id) {
            case OPENCC_CONFIG_S2T: return "s2t";
            case OPENCC_CONFIG_S2TW: return "s2tw";
            case OPENCC_CONFIG_S2TWP: return "s2twp";
            case OPENCC_CONFIG_S2HK: return "s2hk";
            case OPENCC_CONFIG_T2S: return "t2s";
            case OPENCC_CONFIG_T2TW: return "t2tw";
            case OPENCC_CONFIG_T2TWP: return "t2twp";
            case OPENCC_CONFIG_T2HK: return "t2hk";
            case OPENCC_CONFIG_TW2S: return "tw2s";
            case OPENCC_CONFIG_TW2SP: return "tw2sp";
            case OPENCC_CONFIG_TW2T: return "tw2t";
            case OPENCC_CONFIG_TW2TP: return "tw2tp";
            case OPENCC_CONFIG_HK2S: return "hk2s";
            case OPENCC_CONFIG_HK2T: return "hk2t";
            case OPENCC_CONFIG_JP2T: return "jp2t";
            case OPENCC_CONFIG_T2JP: return "t2jp";
            default: return "s2t";
        }
    }

    [[nodiscard]] std::string convertByCfg(const std::string_view input,
                                           const opencc_config_t cfg,
                                           const bool punctuation) const {
        const std::string in(input);
        char *output = opencc_convert_cfg(opencc_, in.c_str(), cfg, punctuation);
        if (!output) return {};

        std::string result(output);
        opencc_string_free(output);
        return result;
    }

    [[nodiscard]] std::string convertByCfgMemLen(
        const std::string_view input,
        const opencc_config_t cfg,
        const bool punctuation
    ) const {
        if (input.empty()) return {};

        size_t required = 0;

        // 1) Query required output size (includes trailing '\0')
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
            return {};
        }

        // 2) Allocate output buffer (RAII, no raw malloc)
        std::string output;
        output.resize(required); // includes '\0'

        // 3) Perform conversion into buffer
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
            return {};
        }

        // 4) Remove trailing '\0' before returning
        output.resize(required - 1);

        return output;
    }
};
