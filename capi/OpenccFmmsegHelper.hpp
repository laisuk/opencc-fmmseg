#pragma once

#include "opencc_fmmseg_capi.h"

#include <stdexcept>
#include <string>
#include <string_view>
#include <unordered_set>
#include <utility>   // std::exchange

class OpenccFmmsegHelper
{
public:
    // ----- Ctors / Dtor -----
    OpenccFmmsegHelper()
        : opencc_(opencc_new())
    {
        if (!opencc_)
        {
            throw std::runtime_error("Failed to initialize OpenCC instance.");
        }
    }

    // Non-copyable, movable
    OpenccFmmsegHelper(const OpenccFmmsegHelper&) = delete;
    OpenccFmmsegHelper& operator=(const OpenccFmmsegHelper&) = delete;

    OpenccFmmsegHelper(OpenccFmmsegHelper&& other) noexcept
        : opencc_(std::exchange(other.opencc_, nullptr)),
          config_(std::move(other.config_)),
          punctuationEnabled_(other.punctuationEnabled_)
    {
    }

    OpenccFmmsegHelper& operator=(OpenccFmmsegHelper&& other) noexcept
    {
        if (this != &other)
        {
            cleanup();
            opencc_ = std::exchange(other.opencc_, nullptr);
            config_ = std::move(other.config_);
            punctuationEnabled_ = other.punctuationEnabled_;
        }
        return *this;
    }

    ~OpenccFmmsegHelper() noexcept { cleanup(); }

    // ----- Stateful configuration -----
    void setConfig(std::string cfg)
    {
        if (!isValidConfig(cfg)) cfg = "s2t";
        config_ = std::move(cfg);
    }

    [[nodiscard]] const std::string& getConfig() const noexcept { return config_; }

    void setPunctuation(const bool enable) noexcept { punctuationEnabled_ = enable; }
    [[nodiscard]] bool punctuationEnabled() const noexcept { return punctuationEnabled_; }

    // ----- Conversion APIs -----

    // Stateless: caller supplies config & punctuation per call
    [[nodiscard]] std::string convert(const std::string_view input,
                                      const std::string_view config,
                                      const bool punctuation = false) const
    {
        if (input.empty()) return {};
        const std::string validCfg = isValidConfig(config) ? std::string(config) : std::string("s2t");
        return convertBy(input, validCfg, punctuation);
    }

    // Stateful: uses stored config_ and punctuationEnabled_
    [[nodiscard]] std::string convert(const std::string_view input) const
    {
        if (input.empty()) return {};
        return convertBy(input, config_, punctuationEnabled_);
    }

    // zho check
    [[nodiscard]] int zhoCheck(const std::string_view input) const
    {
        if (input.empty()) return 0;
        // C API expects null-terminated; std::string_view .data() is fine if we make a temporary std::string
        // (because input may not be null-terminated). Avoid UB by copying when needed:
        const std::string tmp(input);
        return opencc_zho_check(opencc_, tmp.c_str());
    }

    // Last error (global from C API)
    [[nodiscard]] static std::string lastError()
    {
        char* err = opencc_last_error();
        if (!err) return {};
        std::string result(err);
        opencc_error_free(err);
        return result;
    }

private:
    void* opencc_ = nullptr; // opencc_t* in C API
    std::string config_ = "s2t"; // default
    bool punctuationEnabled_ = false; // default

    static void deleteOpencc(const void* p) noexcept
    {
        if (p) opencc_delete(p);
    }

    void cleanup() noexcept
    {
        deleteOpencc(opencc_);
        opencc_ = nullptr;
    }

    [[nodiscard]] static bool isValidConfig(const std::string_view config)
    {
        // O(1) lookup, stable set
        static const std::unordered_set<std::string> kConfigs = {
            "s2t", "t2s", "s2tw", "tw2s", "s2twp", "tw2sp", "s2hk", "hk2s", "t2tw",
            "t2twp", "t2hk", "tw2t", "tw2tp", "hk2t", "t2jp", "jp2t"
        };
        return kConfigs.find(std::string(config)) != kConfigs.end();
    }

    [[nodiscard]] std::string convertBy(const std::string_view input,
                                        const std::string& config,
                                        const bool punctuation) const
    {
        // Ensure null-terminated input for the C API
        const std::string in(input);

        const char* output = opencc_convert(opencc_, in.c_str(), config.c_str(), punctuation);
        if (!output) return {};
        std::string result(output);
        opencc_string_free(output);
        return result;
    }
};
