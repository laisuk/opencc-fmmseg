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
./opencc-cs <input_file> <output_file> <conversion> [lexicon_dir]
```

### Example

```bash
./opencc-cs text_simplified.txt text_traditional.txt s2t
```

- Supported conversions:
  - `s2t` – Simplified to Traditional
  - `t2s` – Traditional to Simplified

### Lexicons

By default, it uses OpenCC's built-in lexicon paths. You can also provide your own lexicon folder as the fourth argument.

## Library Usage

You can also use `opencc-fmmseg` as a library:

```rust
use opencc_fmmseg::convert_text;

fn main() {
  let input = "这是一个测试";
  let output = convert_text(input, "s2t");
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

