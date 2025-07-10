# Changelog

All notable changes to this project will be documented in this file.

This project adheres to [Semantic Versioning](https://semver.org/).

---

## [0.7.0] – 2025-07-10

### Added
- First official crates.io release of `opencc-fmmseg`.
- `OpenCC` struct for high-performance OpenCC-style Chinese conversion using FMM segmentation.
- `DictRefs` wrapper to support multi-round dictionary-based segment replacement.
- Support for:
    - Simplified ↔ Traditional (ST, TS)
    - Taiwan, Hong Kong, and Japanese variants
    - Phrase and character dictionaries
    - Punctuation conversion
- `DictionaryMaxlength` structure to preload dictionaries with max word length for FMM.
- Built-in Zstd-compressed CBOR dictionary loading.
- Methods to serialize/deserialize dictionaries (CBOR and compressed).
- Thread-parallel support via Rayon for large text input.
- Utility for UTF-8 script detection (`zho_check`).
- CLI and FFI compatibility planned via workspace.

### Changed
- N/A

### Removed
- N/A

---

## [Unreleased]

