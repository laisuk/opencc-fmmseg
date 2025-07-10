# opencc-fmmseg

[![GitHub release](https://img.shields.io/github/v/release/laisuk/opencc-fmmseg?sort=semver)](https://github.com/laisuk/opencc-fmmseg/releases)
[![Crates.io](https://img.shields.io/crates/v/opencc-fmmseg)](https://crates.io/crates/opencc-fmmseg)
[![License](https://img.shields.io/crates/l/opencc-fmmseg)](https://github.com/laisuk/opencc-fmmseg/blob/master/LICENSE)
![Build Status](https://github.com/laisuk/opencc-fmmseg/actions/workflows/rust.yml/badge.svg)

**opencc-fmmseg** is a high-performance Rust-based engine for Chinese text conversion.    
It combines [OpenCC](https://github.com/BYVoid/OpenCC)'s lexicons with an
optimized [Forward Maximum Matching (FMM)](https://en.wikipedia.org/wiki/Maximum_matching) algorithm, suitable for:

- Traditional ↔ Simplified conversion
- Lexicon-based segmentation
- CLI tools and system integration via C/C++ or Python bindings

---

## 🦀 Example (Rust)

```rust
use opencc_fmmseg::OpenCC;

fn main() {
  let input = "汉字转换测试";
  let opencc = OpenCC::new();
  let output = opencc.convert(input, "s2t", false);
  println!("{}", output);  // 漢字轉換測試
}
```

---

## 📦 Download

Grab the latest version for your platform from the [**Releases**](https://github.com/laisuk/opencc-fmmseg/releases)
page:

| Platform   | Download Link                                                                        |
|------------|--------------------------------------------------------------------------------------|
| 🪟 Windows | [opencc-fmmseg-windows.zip](https://github.com/laisuk/opencc-fmmseg/releases/latest) |
| 🐧 Linux   | [opencc-fmmseg-linux.zip](https://github.com/laisuk/opencc-fmmseg/releases/latest)   |
| 🍎 macOS   | [opencc-fmmseg-macos.zip](https://github.com/laisuk/opencc-fmmseg/releases/latest)   |

Each archive contains:

```bash
README.txt
version.txt
bin/ # Command-line tools
lib/ # Shared library (.dll / .so / .dylib)
include/ # C API header + C++ helper header
```

## Features

- 📦 Simple CLI tool for converting between Simplified and Traditional Chinese.
- 🔍 Lexicon-driven segmentation using OpenCC dictionaries.
- ⚡ High performance using parallel processing.
- 🛠️ Designed to be easily embedded as a Rust library or used standalone.

## Installation

```bash
git clone https://github.com/laisuk/opencc-fmmseg
cd opencc-fmmseg
cargo build --release --workspace
```

## 🚀 CLI Usage

The CLI tool will be located at:

```
target/release/
```

```bash
opencc-rs          # CLI text converter
opencc-clip        # Convert from clipboard, auto detect config
dict-generate      # Generate dictionary CBOR files
```

## Usage

```
opencc-rs --help

OpenCC Rust: Command Line Open Chinese Converter

Usage: opencc-rs.exe [OPTIONS] --config <conversion>

Options:
  -i, --input <file>         Read original text from <file>.
  -o, --output <file>        Write converted text to <file>.
  -c, --config <conversion>  Conversion configuration: [s2t|s2tw|s2twp|s2hk|t2s|tw2s|tw2sp|hk2s|jp2t|t2jp]
  -p, --punct                Enable punctuation conversion.
      --in-enc <encoding>    Encoding for input: UTF-8|GB2312|GBK|gb18030|BIG5 [default: UTF-8]
      --out-enc <encoding>   Encoding for output: UTF-8|GB2312|GBK|gb18030|BIG5 [default: UTF-8]
      --office               Enable Office/EPUB mode for docx, odt, epub, etc.
      --keep-font            Preserve original font styles (only in Office mode)
  -f, --format <ext>         Force format type: docx, xlsx, odt, epub, etc.
      --auto-ext             Infer format from file extension (if not --format)
  -h, --help                 Print help
```

### Example

#### Plain Text

```bash
./opencc-rs -c s2t -i text_simplified.txt -o text_traditional.txt
```

#### Office Documents or EPUB

- Supported OpenDocument formats: `.docx`, `.xlsx`, `.pptx`, `.odt`, `.ods`, `.odp`, `.epub` 

```bash
./opencc-rs --office -c s2t --format docx -i doc_simplified.docx -o doc_traditional.docx
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

By default, it uses OpenCC's built-in lexicon paths. You can also provide your own lexicon folder as the fourth
argument.

## 📚 Library Usage

You can also use `opencc-fmmseg` as a library:  
To use `opencc-fmmseg` in your project, add this to your `Cargo.toml`:

```toml
[dependencies]
opencc-fmmseg = "0.7.0"
```

Then use it in your code:

```rust
use opencc_fmmseg::OpenCC;

fn main() {
    let input = "这是一个测试";
    let opencc = OpenCC::new();
    let output = opencc.convert(input, "s2t", false);
    println!("{}", output); // -> "這是一個測試"
}
```

> 📦 Crate: [opencc-fmmseg on crates.io](https://crates.io/crates/opencc-fmmseg)  
📄 Docs: [docs.rs/opencc-fmmseg](https://docs.rs/opencc-fmmseg/0.7.0/opencc_fmmseg/)

---

## 🧩 C/C++ Integration (`opencc_fmmseg_capi`)

You can also use `opencc-fmmseg` via a C API for integration with C/C++ projects.

The zip includes:

- libopencc_fmmseg_capi.{so,dylib,dll}
- C API: opencc_fmmseg_capi.h
- Header-only C++ helper: OpenccFmmsegHelper.hpp

You can link against the shared library and call the segmentation/convert functions from any C or C++ project.

### Example 1

```c
#include "opencc_fmmseg_capi.h"
void* handle = opencc_new();
const char* config = "s2t";
const char* result = opencc_convert(handle, "汉字", config, false);
opencc_delete(handle);
```

### Example 2

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

    if (last_error != NULL) {
        opencc_error_free(last_error);
    }
    if (result != NULL) {
        opencc_string_free(result);
    }
    if (opencc != NULL) {
        opencc_delete(opencc);
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
- `opencc_convert(...)` performs the conversion with the specified config (e.g., `s2t`, `t2s`, `s2twp`).
- `opencc_string_free(...)` must be called to free the returned string.
- `opencc_delete(...)` must be called to free OpenCC object.
- `opencc_zho_check(...)` to detect zh-Hant (1), zh-Hans (2), others (0).
- Parallelism support can be queried using `opencc_get_parallel()`.
- Errors are returned from `opencc_last_error()`.

## Project Structure

- `src/lib.rs` – Main library with segmentation logic.
- `capi/opencc-fmmseg-capi` C API source and demo.
- `tools/opencc-rs/src/main.rs` – CLI tool (`opencc-cs`) implementation.
- `dicts/` – OpenCC text lexicons which converted into CBOR format.

## 🛠 Built With

- Rust + Cargo Workspaces
- OpenCC-compatible dictionaries
- Parallelized FMM segmentation
- GitHub Actions cross-platform release automation

---

## 🚀 Benchmark Results: `opencc-fmmseg` Conversion Speed

Tested using [Criterion.rs](https://bheisler.github.io/criterion.rs/book/) on repeated 80-character sentences with punctuation disabled (`punctuation = false`).

| Input Size |  s2t Mean Time |  t2s Mean Time |
|------------|---------------:|---------------:|
| 100        |       45.68 µs |       43.54 µs |
| 1,000      |      131.35 µs |      131.65 µs |
| 10,000     |      411.68 µs |      412.80 µs |
| 100,000    |       2.091 ms |       1.930 ms |
| 1,000,000  |       16.62 ms |       17.11 ms |


![Benchmark Chart](https://raw.githubusercontent.com/laisuk/opencc-fmmseg/master/benches/opencc_fmmseg_benchmark.png)

### 📊 Observations

- **Linear scalability**: The performance scales almost linearly with input size.
- `s2t` and `t2s` have **comparable performance**, with minor variation due to lexicon size and match depth.
- At **1 million characters**, both conversions take around **16–17 ms**, yielding throughput of **~60 million chars/sec**.

---

## Credits

- [OpenCC](https://github.com/BYVoid/OpenCC) by [BYVoid Carbo Kuo](https://github.com/BYVoid) – Lexicon source.

## 📜 License

- MIT License.
- © Laisuk Lai.
- See [LICENSE](./LICENSE) for details.

## 💬 Feedback / Contributions

- Issues and pull requests are welcome.
- If you find this tool useful, please ⭐ star the repo or fork it.
