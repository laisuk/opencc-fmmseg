# opencc-fmmseg

[![GitHub release](https://img.shields.io/github/v/release/laisuk/opencc-fmmseg?sort=semver)](https://github.com/laisuk/opencc-fmmseg/releases)
[![Crates.io](https://img.shields.io/crates/v/opencc-fmmseg)](https://crates.io/crates/opencc-fmmseg)
[![Docs.rs](https://docs.rs/opencc-fmmseg/badge.svg)](https://docs.rs/opencc-fmmseg)
![Crates.io](https://img.shields.io/crates/d/opencc-fmmseg)
[![License](https://img.shields.io/crates/l/opencc-fmmseg)](https://github.com/laisuk/opencc-fmmseg/blob/master/LICENSE)
![Build Status](https://github.com/laisuk/opencc-fmmseg/actions/workflows/rust.yml/badge.svg)

**opencc-fmmseg** is a high-performance Rust-based engine for Chinese text conversion.    
It combines [OpenCC](https://github.com/BYVoid/OpenCC)'s lexicons with an
optimized [Forward Maximum Matching (FMM)](https://en.wikipedia.org/wiki/Maximum_matching) algorithm, suitable for:

- Traditional ↔ Simplified Chinese text conversion
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

| Platform   | Download Link                                                                                        |
|------------|------------------------------------------------------------------------------------------------------|
| 🪟 Windows | [opencc-fmmseg-{latest}-windows-x64.zip](https://github.com/laisuk/opencc-fmmseg/releases/latest)    |
| 🐧 Linux   | [opencc-fmmseg-{latest}-linux-x64.tar.gz](https://github.com/laisuk/opencc-fmmseg/releases/latest)   |
| 🍎 macOS   | [opencc-fmmseg-{latest}-macos-arm64.tar.gz](https://github.com/laisuk/opencc-fmmseg/releases/latest) |

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

---

## 🚀 CLI Usage

The CLI tool will be located at:

```
target/release/
```

```bash
opencc-rs          # CLI plain text and Office document text converter
opencc-clip        # Convert from clipboard, auto detect config
dict-generate      # Generate dictionary ZSTD, CBOR or JSON files
```

## Usage

### `opencc-rs convert`

```
Convert plain text using OpenCC

Usage: opencc-rs.exe convert [OPTIONS] --config <config>

Options:
  -i, --input <file>       Input file (use stdin if omitted for non-office documents)
  -o, --output <file>      Output file (use stdout if omitted for non-office documents)
  -c, --config <config>    Conversion configuration [possible values: s2t, t2s, s2tw, tw2s, s2twp, tw2sp, s2hk, hk2s, t2tw, t2twp, t2hk, tw2t, tw2tp, hk2t, t2jp, jp2t]
  -p, --punct              Enable punctuation conversion
      --in-enc <in_enc>    Encoding for input [default: UTF-8]
      --out-enc <out_enc>  Encoding for output [default: UTF-8]
  -h, --help               Print help
```

### `opencc-rs office`

```
Convert Office or EPUB documents using OpenCC

Usage: opencc-rs.exe office [OPTIONS] --config <config>

Options:
  -i, --input <file>     Input file (use stdin if omitted for non-office documents)
  -o, --output <file>    Output file (use stdout if omitted for non-office documents)
  -c, --config <config>  Conversion configuration [possible values: s2t, t2s, s2tw, tw2s, s2twp, tw2sp, s2hk, hk2s, t2tw, t2twp, t2hk, tw2t, tw2tp, hk2t, t2jp, jp2t]
  -p, --punct            Enable punctuation conversion
  -f, --format <ext>     Force document format: docx, odt, epub...
      --keep-font        Preserve original font styles
      --auto-ext         Infer format from file extension
  -h, --help             Print help
```

### Example

#### Plain Text

```bash
./opencc-rs convert -c s2t -i text_simplified.txt -o text_traditional.txt
```

#### Office Documents or EPUB

- Supported OpenDocument formats: `.docx`, `.xlsx`, `.pptx`, `.odt`, `.ods`, `.odp`, `.epub`

```bash
./opencc-rs office -c s2t --punct --format docx -i doc_simplified.docx -o doc_traditional.docx
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

By default, it uses **OpenCC**'s built-in lexicon paths. You can also provide your own lexicon dictionary generated by
`dict-generate` CLI tool.

---

## 📚 Library Usage

You can also use `opencc-fmmseg` as a library:  
To use `opencc-fmmseg` in your project, add this to your `Cargo.toml`:

```toml
[dependencies]
opencc-fmmseg = "0.8.2"  # or latest version
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
> 📄 Docs: [docs.rs/opencc-fmmseg](https://docs.rs/opencc-fmmseg/0.8.1/opencc_fmmseg/)

---

## 🧩 C/C++ Integration (`opencc_fmmseg_capi`)

You can also use `opencc-fmmseg` via a **C API** for integration with **C/C++ projects**.

The zip includes:

- {lib}`opencc_fmmseg_capi.`{so,dylib,dll}
- C API: `opencc_fmmseg_capi.h`
- Header-only C++ helper: `OpenccFmmsegHelper.hpp`

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
- Parallelism support also can be set using `opencc_set_parallel(...)`.
- Errors are returned from `opencc_last_error()`.

---

## Project Structure

- `src/lib.rs` – Main library with segmentation logic.
- `capi/opencc-fmmseg-capi` C API source and demo.
- `tools/opencc-rs/src/main.rs` – CLI tool (`opencc-rs`) implementation.
- `dicts/` – OpenCC text lexicons which converted into Zstd compressed CBOR format.

## 🛠 Built With

- Rust + Cargo Workspaces
- OpenCC-compatible dictionaries
- Parallelized FMM segmentation
- GitHub Actions cross-platform release automation

---

## 🚀 Benchmark Results: `opencc-fmmseg` Conversion Speed

Tested using [Criterion.rs](https://bheisler.github.io/criterion.rs/book/) on 12,000-character text with
punctuation disabled (`punctuation = false`).

Results from **v0.8.0**:

| Input Size | s2t Mean Time | t2s Mean Time |
|------------|--------------:|--------------:|
| 100        |      46.47 µs |      50.40 µs |
| 1,000      |     134.18 µs |     135.72 µs |
| 10,000     |     393.05 µs |     375.40 µs |
| 100,000    |      1.664 ms |      1.397 ms |
| 1,000,000  |     16.034 ms |     13.466 ms |

📊 **Throughput Interpretation**

- ~62–77 **million characters per second**
- ≈ **100 full-length novels (500k chars each) per second**
- ≈ **1 GB of UTF-8 text** processed in under **10 seconds**

At this scale, performance is so high that **I/O (disk or network)**, not the converter, becomes the bottleneck.

> **Notes**: Version `0.8.2` introduces code optimizations that further improve conversion speed and reduce memory
> allocations.

![Benchmark Chart](https://raw.githubusercontent.com/laisuk/opencc-fmmseg/master/benches/opencc_fmmseg_benchmark_080.png)

### 🏅 Highlights

![Safe & Parallel](https://img.shields.io/badge/Safe%20%26%20Parallel-Yes-ff69b4)

---

## Credits

- [OpenCC](https://github.com/BYVoid/OpenCC) by [BYVoid Carbo Kuo](https://github.com/BYVoid) – Lexicon source.

## 📜 License

- MIT License.
- © Laisuk Lai.
- See [LICENSE](./LICENSE) for details.
- See [THIRD_PARTY_NOTICES.md](./THIRD_PARTY_NOTICES.md) for bundled OpenCC lexicons (_Apache License 2.0_).

## 💬 Feedback / Contributions

- Issues and pull requests are welcome.
- If you find this tool useful, please ⭐ star the repo or fork it.
