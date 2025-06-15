#pragma once

#include "opencc_fmmseg_capi.h"

#include <stdexcept>
#include <string>
#include <vector>
#include <algorithm>

class OpenccFmmsegHelper
{
public:
    OpenccFmmsegHelper()
        : opencc_(opencc_new())
    {
        if (!opencc_)
        {
            throw std::runtime_error("Failed to initialize OpenCC instance.");
        }
    }

    ~OpenccFmmsegHelper()
    {
        if (opencc_)
        {
            opencc_delete(opencc_);
            opencc_ = nullptr;
        }
    }

    [[nodiscard]] std::string convert(const std::string& input, const std::string& config,
                                      const bool punctuation = false) const
    {
        if (input.empty()) return "";
        const std::string validConfig = isValidConfig(config) ? config : "s2t";
        return convertBy(input, validConfig, punctuation);
    }

    [[nodiscard]] int zhoCheck(const std::string& input) const
    {
        if (input.empty()) return 0;
        return opencc_zho_check(opencc_, input.c_str());
    }

    static std::string lastError()
    {
        char* err = opencc_last_error();
        if (!err) return "";
        std::string result = ptrToStringUtf8(err);
        opencc_error_free(err);
        return result;
    }


private:
    void* opencc_;

    static std::string ptrToStringUtf8(const char* ptr)
    {
        if (!ptr) return "";
        return ptr;
    }

    static bool isValidConfig(const std::string& config)
    {
        static const std::vector<std::string> configList = {
            "s2t", "t2s", "s2tw", "tw2s", "s2twp", "tw2sp", "s2hk", "hk2s", "t2tw",
            "t2twp", "t2hk", "tw2t", "tw2tp", "hk2t", "t2jp", "jp2t"
        };
        return std::find(configList.begin(), configList.end(), config) != configList.end();
    }

    [[nodiscard]] std::string convertBy(const std::string& input, const std::string& config,
                                        const bool punctuation) const
    {
        char* output = opencc_convert(opencc_, input.c_str(), config.c_str(), punctuation);
        if (!output) return "";
        std::string result = ptrToStringUtf8(output);
        opencc_string_free(output);
        return result;
    }
};
