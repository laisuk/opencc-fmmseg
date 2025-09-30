# Changelog

All notable changes to this project will be documented in this file.

This project adheres to [Semantic Versioning](https://semver.org/).

---

## [0.8.2-beta1] - 2025-09-30

## Added

- **dict-generate JSON export**: Human-readable JSON via DTOs with String keys;  
  supports `--pretty` (pretty) and `--compact` (default). Core schema remains CBOR/Zstd.
  Why: JSON is for reference/debug only; the canonical on-disk format stays CBOR.

### Fixed

- **EPUB**: Resolved `os error 267` (“The directory name is invalid”) on Windows by correctly handling ZIP **directory
  entries** during extraction (create directories instead of calling `File::create` on them).
- **PPTX**: Resolved `os error 5` (“Access is denied”) caused by overwriting while the input archive handle was still
  open and/or the destination file was read-only:
    - Unzip now occurs in a **scoped block** so all input handles are dropped before writing output.
    - Output writing uses **temp-file → rename** strategy to the final path.
    - Clears the **read-only** attribute on existing outputs before removal.
    - Prevents **input==output** collisions via canonical path check.

### Changed

- **EPUB packaging**: Ensure `mimetype` is written **first** and **Stored** (no compression), per EPUB spec.
- **PPTX targeting**: Process only **slides** and **notes slides** XML parts; skip `.rels` and unrelated files to avoid
  unintended edits.
- **Path safety & robustness**:
    - Added zip-slip/root component checks on extraction.
    - Walkers only operate on **files** (skip directories and non-file entries).
    - More descriptive I/O errors now include the failing **path**.
- **Cleanup**: Removed sleeps and all debug code from the conversion path.
- **Dictionaries**: Updated word lists.
- **Delimiter handling**:
    - Removed unused `Minimal` and `Normal` modes, leaving only `Full`.
    - Dropped the private `delimiters` field in `OpenCC`; now uses the global static `FULL_DELIMITER_SET`.
- **Error handling (unified)**: `from_zstd()`, `from_cbor()`, `serialize_to_cbor()`, `deserialize_from_cbor()`, and `new()` now return `Result<_, DictionaryError>` for consistent typed errors.

## Moved

- `serde (JSON) overrides`: Removed from core `DictMaxLen`; JSON adaptation now lives only in the `dict-generate` CLI (DTO layer).
  **Result**: Core CBOR/Zstd schema stays stable (snake_case), faster loads, less risk of format drift.

### Performance

- Optimized `zho_check()` to scan only the first **1,000 bytes** of the input string.
- Reduced runtime memory footprint by:
    - Changing `first_char_max_len`, `bmp_cap`, and `astral_cap` value types from `u16` to `u8`.
    - Updating corresponding `Vec<u16>` containers to `Vec<u8>`.
    - Safe since maximum dictionary lengths are always `< 255`.

---

## [0.8.1] - 2025-08-25

### Changed

- opencc-clip CLI now use clap format as command arguments.
- Retained legacy convert_by()

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

