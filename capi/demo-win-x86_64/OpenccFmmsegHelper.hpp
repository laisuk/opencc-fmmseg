#pragma once

#include "opencc_fmmseg_capi.h"

#include <cctype>
#include <stdexcept>
#include <string>
#include <string_view>
#include <utility>   // std::exchange

class OpenccFmmsegHelper
{
public:
    // ----- Ctors / Dtor -----
    OpenccFmmsegHelper()
        : opencc_(opencc_new())
    {
        if (!opencc_)
            throw std::runtime_error("Failed to initialize OpenCC instance.");
    }

    // Non-copyable, movable
    OpenccFmmsegHelper(const OpenccFmmsegHelper&) = delete;
    OpenccFmmsegHelper& operator=(const OpenccFmmsegHelper&) = delete;

    OpenccFmmsegHelper(OpenccFmmsegHelper&& other) noexcept
        : opencc_(std::exchange(other.opencc_, nullptr)),
          configId_(other.configId_),
          punctuationEnabled_(other.punctuationEnabled_)
    {}

    OpenccFmmsegHelper& operator=(OpenccFmmsegHelper&& other) noexcept
    {
        if (this != &other)
        {
            cleanup();
            opencc_ = std::exchange(other.opencc_, nullptr);
            configId_ = other.configId_;
            punctuationEnabled_ = other.punctuationEnabled_;
        }
        return *this;
    }

    ~OpenccFmmsegHelper() noexcept { cleanup(); }

    // ----- Stateful configuration (recommended: numeric config) -----
    void setConfigId(opencc_config_t cfg) noexcept
    {
        // Self-protect: if unknown, keep default
        // (Your C API also self-protects, but keeping helper consistent is fine.)
        if (isValidConfigId(cfg))
            configId_ = cfg;
        else
            configId_ = OPENCC_CONFIG_S2T;
    }

    [[nodiscard]] opencc_config_t getConfigId() const noexcept { return configId_; }

    // Optional convenience: accept string and map to numeric ID
    // (Keeps user-friendly API, while still using opencc_convert_cfg under the hood.)
    void setConfig(std::string_view cfgName)
    {
        configId_ = configNameToId(cfgName);
    }

    void setPunctuation(bool enable) noexcept { punctuationEnabled_ = enable; }
    [[nodiscard]] bool punctuationEnabled() const noexcept { return punctuationEnabled_; }

    // ----- Conversion APIs -----

    // Stateless (typed): caller supplies config id & punctuation per call
    [[nodiscard]] std::string convert_cfg(std::string_view input,
                                          opencc_config_t config,
                                          bool punctuation = false) const
    {
        if (input.empty()) return {};
        return convertByCfg(input, config, punctuation);
    }

    // Stateful (typed): uses stored configId_ and punctuationEnabled_
    [[nodiscard]] std::string convert_cfg(std::string_view input) const
    {
        if (input.empty()) return {};
        return convertByCfg(input, configId_, punctuationEnabled_);
    }

    // Legacy stateless: caller supplies string config name
    [[nodiscard]] std::string convert(std::string_view input,
                                      std::string_view configName,
                                      bool punctuation = false) const
    {
        if (input.empty()) return {};
        const opencc_config_t id = configNameToId(configName);
        return convertByCfg(input, id, punctuation);
    }

    // Legacy stateful: uses stored configId_ (set via setConfig/setConfigId)
    [[nodiscard]] std::string convert(std::string_view input) const
    {
        return convert_cfg(input);
    }

    // zho check
    [[nodiscard]] int zhoCheck(std::string_view input) const
    {
        if (input.empty()) return 0;
        const std::string tmp(input); // ensure NUL-terminated
        return opencc_zho_check(opencc_, tmp.c_str());
    }

    // Last error (thread-local/global in C API)
    [[nodiscard]] static std::string lastError()
    {
        char* err = opencc_last_error();
        if (!err) return {};
        std::string result(err);
        opencc_error_free(err);
        return result;
    }

    // ----- Error state management -----

    /// Clears the internal OpenCC last-error state.
    ///
    /// This resets the error status only; it does NOT free any previously
    /// returned error strings.
    static void clearLastError() noexcept
    {
        opencc_clear_last_error();
    }

private:
    void* opencc_ = nullptr;
    opencc_config_t configId_ = OPENCC_CONFIG_S2T;
    bool punctuationEnabled_ = false;

    static void cleanupOpencc(void* p) noexcept
    {
        if (p) opencc_delete(p);
    }

    void cleanup() noexcept
    {
        cleanupOpencc(opencc_);
        opencc_ = nullptr;
    }

    [[nodiscard]] static bool isValidConfigId(opencc_config_t cfg) noexcept
    {
        // Valid values: 1..16 (current contract)
        return cfg >= OPENCC_CONFIG_S2T && cfg <= OPENCC_CONFIG_T2JP;
    }

    [[nodiscard]] static opencc_config_t configNameToId(std::string_view s)
    {
        // Case-insensitive ASCII fold (configs are ASCII tokens)
        std::string t;
        t.reserve(s.size());
        for (unsigned char ch : s)
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

        // Self-protect default (matches your philosophy)
        return OPENCC_CONFIG_S2T;
    }

    [[nodiscard]] std::string convertByCfg(std::string_view input,
                                           opencc_config_t cfg,
                                           bool punctuation) const
    {
        // NOTE:
        // - opencc_convert_cfg() is strict: invalid config returns an error string
        // - This helper always routes conversions through the typed C API
        const std::string in(input); // ensure NUL-terminated for C API

        // IMPORTANT: returns char* that must be freed by opencc_string_free()
        char* output = opencc_convert_cfg(opencc_, in.c_str(), cfg, punctuation);
        if (!output) return {};

        std::string result(output);
        opencc_string_free(output);
        return result;
    }
};
