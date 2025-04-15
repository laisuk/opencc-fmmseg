# opencc-fmmseg

A Rust-based Chinese text converter that performs word segmentation using **Forward Maximum Matching (FMM)**, powered by **OpenCC lexicons**. This project aims to provide high-performance and accurate **Simplified ↔ Traditional Chinese** (zh-Hans ↔ zh-Hant) conversion.

## Features

- 📦 Simple CLI tool for converting between Simplified and Traditional Chinese.
- 🔍 Lexicon-driven segmentation using OpenCC dictionaries.
- ⚡ High performance using parallel processing (`rayon`).
- 🛠️ Designed to be easily embedded as a Rust library or used standalone.

## Installation

```bash
git clone https://github.com/laisuk/opencc-fmmseg
cd opencc-fmmseg
cargo build --release --workspace
```

The CLI tool will be located at:

```
target/release/opencc-rs
```

## Usage

```
(Windows)
opencc-rs.exe [OPTIONS] --config <conversion>
(Linux / macOS)
opencc-rs [OPTIONS] --config <conversion>

Options:
  -i, --input <file>         Read original text from <file>.
  -o, --output <file>        Write converted text to <file>.
  -c, --config <conversion>  Conversion configuration: [s2t|s2tw|s2twp|s2hk|t2s|tw2s|tw2sp|hk2s|jp2t|t2jp]
  -p, --punct <boolean>      Punctuation conversion: [true|false] [default: false]
      --in-enc <encoding>    Encoding for input: UTF-8|GB2312|GBK|gb18030|BIG5 [default: UTF-8]
      --out-enc <encoding>   Encoding for output: UTF-8|GB2312|GBK|gb18030|BIG5 [default: UTF-8]
  -h, --help                 Print help
```

### Example

```bash
./opencc-rs -c s2t -i text_simplified.txt -o text_traditional.txt
```

- Supported conversions:
  - `s2t` – Simplified to Traditional
  - `s2tw` – Simplified to Traditional Taiwan
  - `s2twp` – Simplified to Traditional Taiwan with idioms
  - `t2s` – Traditional to Simplified
  - `tw2s` – Traditional Taiwan to Simplified
  - `tw2sp` – Traditional Taiwan to Simplified with idioms
  - etc

### Lexicons

By default, it uses OpenCC's built-in lexicon paths. You can also provide your own lexicon folder as the fourth argument.

## Library Usage

You can also use `opencc-fmmseg` as a library:

```rust
use opencc_fmmseg::OpenCC;

fn main() {
  let input = "这是一个测试";
  let opencc = OpenCC::new();
  let output = opencc.convert(input, "s2t", false);
  println!("{}", output); // -> "這是一個測試"
}
```

## C API Usage (`opencc_fmmseg_capi`)

You can also use `opencc-fmmseg` via a C API for integration with C/C++ projects.

### Example

```c
#include <stdio.h>
#include "opencc_fmmseg_capi.h"

int main(int argc, char **argv) {
    void *opencc = opencc_new();
    bool is_parallel = opencc_get_parallel(opencc);
    printf("OpenCC is_parallel: %d\n", is_parallel);

    const char *config = u8"s2twp";
    const char *text = u8"意大利邻国法兰西罗浮宫里收藏的“蒙娜丽莎的微笑”画像是旷世之作。";
    printf("Text: %s\n", text);

    int code = opencc_zho_check(opencc, text);
    printf("Text Code: %d\n", code);

    char *result = opencc_convert(opencc, text, config, true);
    code = opencc_zho_check(opencc, result);

    char *last_error = opencc_last_error();
    printf("Converted: %s\n", result);
    printf("Text Code: %d\n", code);
    printf("Last Error: %s\n", last_error == NULL ? "No error" : last_error);

    if (result != NULL) {
        opencc_string_free(result);
    }
    if (opencc != NULL) {
        opencc_free(opencc);
    }

    return 0;
}
```

### Output
```
OpenCC is_parallel: 1
Text: 意大利邻国法兰西罗浮宫里收藏的“蒙娜丽莎的微笑”画像是旷世之作。
Text Code: 2
Converted: 義大利鄰國法蘭西羅浮宮裡收藏的「蒙娜麗莎的微笑」畫像是曠世之作。
Text Code: 1
Last Error: No error
```
### Notes

- `opencc_new()` initializes the engine.
- `opencc_zho_check(...)` to detect zh-Hant (1), zh-Hans (2), others (0).
- `opencc_convert(...)` performs the conversion with the specified config (e.g., `s2t`, `t2s`, `s2twp`).
- `opencc_string_free(...)` must be called to free the returned string.
- `opencc_free(...)` must be called to free OpenCC object.
- Parallelism support can be queried using `opencc_get_parallel()`.
- Errors are returned from `opencc_last_error()`.

## Project Structure

- `src/lib.rs` – Main library with segmentation logic.
- `capi/opencc-fmmseg-capi` C API source and demo.
- `tools/opencc-rs/src/main.rs` – CLI tool (`opencc-cs`) implementation.
- `dicts/` – OpenCC text lexicons which converted into CBOR format.

## Credits

- [OpenCC](https://github.com/BYVoid/OpenCC) – Lexicon source.

## License

MIT License

