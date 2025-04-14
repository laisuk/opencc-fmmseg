# opencc-fmmseg (Draft)

A Rust-based Chinese text converter that performs accurate word segmentation using a hybrid of **Forward Maximum Matching (FMM)** and **Jieba-style heuristics**, powered by **OpenCC lexicons**. This project aims to provide high-performance and accurate **Simplified ↔ Traditional Chinese** (zh-Hans ↔ zh-Hant) conversion.

## Features

- 📦 Simple CLI tool for converting between Simplified and Traditional Chinese.
- 🔍 Lexicon-driven segmentation using OpenCC dictionaries.
- ⚡ High performance using parallel processing (`rayon`).
- 🧠 Jieba-style logic for better ambiguity resolution and natural segmentation.
- 🛠️ Designed to be easily embedded as a Rust library or used standalone.

## Installation

```bash
git clone https://github.com/laisuk/opencc-fmmseg
cd opencc-fmmseg
cargo build --release
```

The CLI tool will be located at:

```
target/release/opencc-cs
```

## Usage

```bash
opencc-rs.exe [OPTIONS] --config <conversion>

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
./opencc-cs -i text_simplified.txt -o text_traditional.txt -c s2t
```

- Supported conversions:
  - `s2t` – Simplified to Traditional
  - `t2s` – Traditional to Simplified

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

## Project Structure

- `src/lib.rs` – Main library with segmentation logic.
- `tools/opencc-rs/src/main.rs` – CLI tool (`opencc-cs`) implementation.
- `lexicon/` – OpenCC lexicons in CBOR format.

## Credits

- [OpenCC](https://github.com/BYVoid/OpenCC) – Lexicon source.
- Jieba-style segmentation concepts.

## License

MIT License

