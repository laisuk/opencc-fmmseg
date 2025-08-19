# Changelog

All notable changes to this project will be documented in this file.

This project adheres to [Semantic Versioning](https://semver.org/).

---

## [0.8.1] - 2025-08-19

### Fixed

- Lock rayon at 1.10.0, rayon-core at 1.12.1 for rustc 1.75.0 compatible.

---

## [0.8.0] - 2025-08-19

### Added

- `DictMaxLen` helpers: `build_from_pairs`, `ensure_starter_indexes`, `populate_starter_indexes`, `is_populated`, plus
  custom `Serialize`/`Deserialize` and `Default`.
- Starter-index accelerators in `DictMaxLen`:
    - `first_len_mask64` (BMP starter → 64-bit length bitmask; bit 63 = ≥64),
    - `first_char_max_len` (BMP per-starter max length),
    - persisted `starter_cap` (non-BMP per-starter caps).
- **`StarterUnion`**: unions length masks & caps across multiple `DictMaxLen` tables (dense BMP arrays + sparse astral
  maps). `StarterUnion::build` added.
- Core FMM routine **`convert_by_union`**: longest-match using `StarterUnion` to prune impossible lengths before probing
  dictionaries.
- **Union cache** (runtime-only) inside `DictionaryMaxlength` using `OnceLock`:
    - module `dictionary_maxlength/union_cache.rs`,
    - `UnionKey` enum and `union_for(&self, key) -> Arc<StarterUnion>`,
    - `clear_unions(&mut self)` to reset cache.
- **Round orchestration refresh**:
    - `DictRound { dicts, max_len, union }`,
    - `DictRefs::new(..., Arc<StarterUnion>)`, `.with_round_2(...)`, `.with_round_3(...)`,
    - `DictRefs::apply_segment_replace` now passes `&StarterUnion` to the closure.
- Conversion entrypoints updated to use per-round unions (same external names/behavior):
    - `s2t`, `t2s`, `s2tw`, `tw2s`, `s2twp`, `tw2sp`,
      `s2hk`, `hk2s`, `t2tw`, `t2twp`, `tw2t`, `tw2tp`,
      `t2hk`, `hk2t`, `t2jp`, `jp2t`.

### Changed

- Moved `DictMaxLen` to its own module/file (`dict_max_len.rs`).
- `segment_replace` now builds **one** `StarterUnion` per call and reuses it across segments; parallel path uses Rayon
  with `.with_min_len(8)` and `reduce(String::new, …)` for fewer allocations.
- Expanded doc comments throughout (`from_zstd`, `from_dicts`, union APIs, FMM routine) and fixed doctests (explicit
  generic in `serde_json::from_str`, valid JSON without comments).

### Performance

- Significant speedups from starter masks + union pruning + cached unions.
    - Local runs (reference): ~3M chars in ~60 ms (first run), ~50 ms on subsequent runs thanks to the warm union cache.

### Breaking changes

- `DictRefs` API now requires a per-round `Arc<StarterUnion>` and the
  `apply_segment_replace` closure signature is:
  `Fn(&str, &[&DictMaxLen], usize, &StarterUnion) -> String`.
  (SemVer note: breaking changes are allowed in `0.y` minor versions.)

### Internal

- New internal helper `for_each_len_dec` (descending length enumeration with CAP bit) with overflow-safe shifting.
- `DictionaryMaxlength::unions` marked `#[serde(skip, default)]`; included in `Default` and reset in `clear_unions`.

---

## [0.7.1] - 2025*07-29

### Added

- dict-generate - add downloading dictionaries from GitHub if dicts/ folder missing
- DictionaryLib - add function to_dicts()

### Changed

- CLI `opencc-rs` - changed `--office` to subcommand `opencc-rs office`
- Update STPhrases.txt
- Restructure module dictionary_lib

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

