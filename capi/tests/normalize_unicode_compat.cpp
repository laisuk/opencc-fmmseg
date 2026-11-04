#include "OpenccFmmsegHelper.hpp"
#include <cassert>
#include <type_traits>
#include <utility>

static_assert(std::is_same_v<decltype(&opencc_normalize_unicode_compat),
                           char *(*)(const void *, const char *)>);

int main() {
    OpenccFmmsegHelper helper;
    assert(helper.normalizeUnicodeCompat(u8"聼⽟神汉语") == u8"聽玉神汉语");
    assert(helper.normalizeCompatExtended(u8"聼⽟神汉语") == u8"聽玉神汉语");
    assert(helper.normalizeUnicodeCompat("").empty());
    const std::string source = u8"聼⽟神";
    assert(helper.normalizeUnicodeCompat(std::string_view(source).substr(0, 6)) == u8"聽玉");
    OpenccFmmsegHelper moved(std::move(helper));
    assert(moved.normalizeUnicodeCompat(source) == u8"聽玉神");
}
