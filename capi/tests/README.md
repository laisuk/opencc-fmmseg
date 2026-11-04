# C API normalization regressions

These programs use the canonical headers and a freshly built native library.
The C++ RAII helper requires C++17; assertions must be enabled.

From the repository root on Linux:

```sh
cargo build -p opencc-fmmseg-capi --locked
cc -std=c11 -Wall -Wextra -Werror -I capi/include capi/tests/normalize_unicode_compat.c -L target/debug -lopencc_fmmseg_capi -o target/debug/normalize_unicode_compat_c
c++ -std=c++17 -Wall -Wextra -Werror -I capi/include capi/tests/normalize_unicode_compat.cpp -L target/debug -lopencc_fmmseg_capi -o target/debug/normalize_unicode_compat_cpp
LD_LIBRARY_PATH=target/debug target/debug/normalize_unicode_compat_c
LD_LIBRARY_PATH=target/debug target/debug/normalize_unicode_compat_cpp
```

On Windows with MinGW, use `gcc`/`g++`, append `.exe` to the output paths,
and run both executables beside `target/debug/opencc_fmmseg_capi.dll`.

Coverage includes curated-only normalization, preservation of CJK Compatibility
Ideographs, empty input, null arguments, invalid UTF-8, error clearing, string
ownership, non-NUL-terminated C++ string views, and helper move construction.
Invalid UTF-8 follows this API's existing convention: an allocated error string
is returned and must be freed, while null arguments return NULL.
