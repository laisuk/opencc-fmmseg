# Changelog

All notable changes to this project will be documented in this file.

This project adheres to [Semantic Versioning](https://semver.org/).

---

## [0.11.2] - 2026-06-29

### Added

- Added optional CJK Compatibility Ideograph normalization for callers that want a Unicode compatibility pre-pass before
  OpenCC segmentation/conversion.
- Added `src/data/CJK_Compatibility_Ideographs.txt` mapping data.
- Added `compat_ideographs.rs` with cached dense lookup tables.
- Added `OpenCC::normalize_compat(...)` convenience API.
- The feature is optional and does not affect normal conversion unless explicitly used.
- CLI: `opencc-rs` - added `--norm-compat` feature.

---

## [0.11.1] - 2026-06-27

### Added

- Added optional IDS (Ideographic Description Sequence) preservation support:
    - `OpenCC::get_preserve_ids()`
    - `OpenCC::set_preserve_ids(bool)`
- Added `opencc-rs convert --keep-ids` to preserve complete IDS expressions during conversion.
- Added `opencc-rs convert/office --custom-dict` to use custom dictionary during conversion.
- Added IDS parser and validation helpers for detecting complete Unicode IDS sequences.
- Added regression tests covering unary, binary, ternary, nested, and malformed IDS expressions.

### Changed

- Update dictionary data.
- Refactored text segmentation to optionally isolate complete IDS sequences before conversion.
- Refactored serial and parallel conversion paths to share the same text segmentation logic.
- When IDS preservation is enabled, complete IDS expressions are emitted unchanged while incomplete IDS expressions
  continue to be processed normally.
- Optimized `office_converter` for error handling.
- Extracted `DeTofu` section from `README.md` to standalone `DETOFU_USER_GUIDE.md`

---

## [0.11.0] - 2026-06-17

### Added

* Added public DeTofu display-compatibility fallback APIs for rare non-BMP CJK extension characters:
  `DetofuLevel`, `DetofuMap`, `detofu()`, `OpenCC::detofu()`, and
  `OpenCC::detofu_with_custom_file()`.
* Added `DetofuMap::with_custom_pairs(...)` for applying in-memory DeTofu fallback pairs after loading built-in
  mappings.
* Added `OpenCC::detofu_with_custom_pairs(...)` convenience API for applying built-in DeTofu mappings plus custom
  in-memory fallback pairs.
* Added support for loading user-supplied DeTofu fallback files. Custom mappings
  are merged with the built-in fallback table, and custom entries take
  precedence when duplicate tofu-risk characters exist.
* Added docs and README examples for threshold-based DeTofu levels, direct
  utility usage, reusable maps, custom fallback files, and post-load custom
  fallback pairs.
* Added tests for DeTofu custom pairs, built-in override behavior, and later-pair-wins behavior.
* Added upstream-compatible Hong Kong phrase conversion configs `s2hkp` and
  `hk2sp`.
* Added optional HK phrase dictionary slots `HKPhrases` and `HKPhrasesRev`.

### Changed

* Mirrored upstream OpenCC Japanese dictionary naming by replacing the old
  `JPVariants` / `JPVariantsRev` model with `JPShinjitaiCharacters.txt`,
  `JPShinjitaiCharactersRev.txt`, and `JPShinjitaiPhrases.txt`.
  Custom dictionary users should use the `JPSCharactersRev` slot instead of
  the removed `JPVariants` slot.
* Refactored `s2twp` to match the upstream OpenCC config restructure:
  the Taiwan phrase mappings and Taiwan variant mappings now run together in
  the second conversion round after the Simplified-to-Traditional round. This
  preserves OpenCC-compatible output while removing one full conversion pass.
* Reduced the size of the built-in DeTofu fallback table by switching extension
  identifiers from `ExtB`–`ExtI` to the compact form `B`–`I` while maintaining
  backward-compatible parsing support for both formats.
* Improved DeTofu parser compatibility to accept both compact (`B`–`I`) and
  legacy (`ExtB`–`ExtI`) extension identifiers in custom fallback files.
* Missing plaintext `HKPhrases.txt` and `HKPhrasesRev.txt` files now load as
  empty dictionaries for backward compatibility with older dictionary folders.

### Breaking

* Removed the old Japanese custom dictionary slots `JPVariants` and
  `JPVariantsRev`. Use `JPSCharactersRev` and `JPSCharacters` respectively.

---

## [0.10.2] - 2026-06-06

### Added

- Added upstream-aligned forward regional phrase dictionaries:
    - `TWVariantsPhrases.txt`
    - `HKVariantsPhrases.txt`
- Added public custom dictionary slots:
    - `DictSlot::TWVariantsPhrases`
    - `DictSlot::HKVariantsPhrases`

### Changed

- Updated and optimized dictionary data to reduce conversion ambiguity and improve phrase consistency.
- Taiwan and Hong Kong forward regional variant conversion now applies phrase-level variant dictionaries before
  character-level fallback:
    - `TWVariantsPhrases` -> `TWVariants`
    - `HKVariantsPhrases` -> `HKVariants`

---

## [0.10.1] - 2026-06-01

### Changed

- Updated and optimized dictionary data to reduce conversion ambiguity and improve phrase consistency.
- Improved internal last-error state handling by normalizing empty error strings (`""`) to `None`.
- Unified Rust and C API last-error behavior for more consistent FFI error retrieval semantics.

### Fixed

- Fixed rare edge cases where `opencc_last_error()` could return inconsistent results between Rust and C API layers when
  the last error state was empty.
- Fixed ambiguous internal `Some("")` last-error states that could cause fragile unit test behavior in certain
  scenarios.

---

## [0.10.0] - 2026-05-24

### Added

- Added advanced custom dictionary support for `DictionaryMaxlength`:
    - `from_dicts_custom()` for pair-based custom dictionary injection.
    - `from_dicts_custom_files()` for loading one or more OpenCC-style plaintext dictionary files.
    - `from_dicts_at()` for loading dictionaries from an alternate base directory.
- Added new public custom dictionary APIs:
    - `CustomDictSpec`
    - `CustomDictFileSpec`
    - `CustomDictMode`
    - `DictSlot`
- Added append/override merge modes for slot-aware custom dictionary injection.
- Added support for multi-file custom dictionary layering per slot.
- Added `OpenCC::from_dictionary()` for constructing an `OpenCC` instance directly from a custom `DictionaryMaxlength`.
- Added `dict-generate -b/--base-dir` for generating CBOR/ZSTD/JSON artifacts from external OpenCC dictionary
  directories.
- Added regression tests covering:
    - pair-based custom dictionary injection
    - file-based custom dictionary injection
    - multi-slot custom dictionary loading
    - end-to-end conversion override behavior
- Added post-load custom dictionary customization APIs for already loaded `DictionaryMaxlength` instances:
    - `DictionaryMaxlength::with_custom_dicts()` for applying in-memory custom pairs after loading built-in or external
      dictionary data.
    - `DictionaryMaxlength::with_custom_dict_files()` for applying one or more OpenCC-style plaintext custom dictionary
      files after loading.
- Added dynamic `DictMaxLen` map update helpers that rebuild length metadata and starter indexes after custom pair
  updates, keeping runtime conversion fast and immutable after `OpenCC::from_dictionary()`.

### Changed

- Refactored plaintext dictionary loading internals to reuse a shared OpenCC dictionary parser implementation.
- `DictionaryMaxlength::from_dicts()` now internally delegates to the custom dictionary loading pipeline while
  preserving backward compatibility.
- Update conversion dictionary data.
- Added regression coverage to ensure `OpenCC::convert()` and the `opencc-rs convert` file-to-file path preserve the
  caller's original line endings (`CRLF`, `LF`, or mixed input) instead of normalizing by platform. This is important
  for cross-platform use cases where converted text must keep stable diffs, checksums, generated files, and repository
  line ending policy intact.
- Clarified the custom dictionary workflow as:
  `DictionaryMaxlength::from_zstd()` / `deserialize_from_cbor()` / `deserialize_from_json()` → `with_custom_dicts()` /
  `with_custom_dict_files()` → `OpenCC::from_dictionary()`.

---

## [0.9.2] - 2026-04-22

### Changed

- Updated conversion dictionary data.
- Replaced `once_cell` with Rust std `OnceLock`.
- C API:
    - Added `opencc_convert_cfg_mem_len()` for explicit-length UTF-8 input.
    - Improved buffer-based conversion for high-performance interop scenarios.
    - Retained `opencc_convert_cfg_mem()` for backward compatibility.
    - No C ABI break.

### Fixed

- Rust API: successful conversions now clear stale `OpenCC` last-error state, so `convert()`, `convert_with_config()`,
  and direct conversion helpers no longer leave a previous error visible after a later successful call.
- Rust API internals: `StarterUnion` now preserves true per-starter caps for custom dictionary entries longer than 64
  characters, keeping public `dictionary_lib` behavior consistent with `DictMaxLen` metadata and avoiding unreachable
  long-key matches.
- C API / C++ RAII helper: invalid config ids and names are now surfaced as native `Invalid config: ...` errors instead
  of being silently normalized to `s2t` in `OpenccFmmsegHelper.hpp`.
- C++ RAII helper: native conversion failures are now returned as last-error text instead of collapsing to empty
  strings, making wrapper behavior consistent with the underlying C API.
- C API headers: moved canonical public headers to `capi/include/` for clearer packaging and user-facing include
  layout.
- C API header: corrected `opencc_set_parallel` to accept a mutable instance pointer in `opencc_fmmseg_capi.h`,
  matching the actual mutating behavior and Rust implementation.

---

## [0.9.1] - 2026-03-21

### Changed

- Optimized `OpenccConfig` with added functions.
- Update dictionary
- Python demo package optimization
- Micro-optimization for `dictionary_lib`
- Improved core conversion performance by reducing heap memory allocations.

---

## [0.9.0] - 2026-02-09

### Breaking changes

- Removed the embedded `dictionary_maxlength.cbor` from the published crate
- `from_cbor()` no longer loads an embedded dictionary and now requires an external CBOR file
- Applications relying on implicit embedded CBOR must migrate to `from_zstd()` or explicit CBOR loading

### Changed

- Updated built-in dictionary to **v1.2.0**
- The crate now ships **only** `dictionary_maxlength.zstd`
  (Zstd-compressed CBOR) as the default dictionary artifact

### Added

- `from_zstd()` as the recommended default dictionary loader for non-custom usage
- Detailed migration guidance in API documentation

### Improved

- Crate size significantly reduced by eliminating redundant embedded artifacts
- Packaging and distribution behavior made explicit and deterministic

---

## [0.8.5] - 2026-01-27

- C API: added library ABI number and version string functions:
    - `opencc_abi_number()`
    - `opencc_version_string()`
- Update dictionary to v1.2.0
- Refactored `OpenCC` struct to standalone crate `opencc` with no public API changed.

---

## [0.8.4.2] - 2026-01-05

### Changed

- C API: added helper functions for OpenCC configuration name / ID mapping:
    - `opencc_config_name_to_id()`
    - `opencc_config_id_to_name()`
- Enables clean, allocation-free Name ↔ ID conversion for bindings
  (C / C++ / C# / Python / Java)
- C API: align deprecated `opencc_convert_len()` error behavior with
  `opencc_convert()` / `opencc_convert_cfg()`.

### Notes

- No breaking changes
- Rust core unchanged

---

## [0.8.4.1] - 2026-01-03

### Fixed

- Improved C API error handling and UTF-8 validation.
- Clarified and hardened size-query semantics for `_mem` APIs.
- No behavior change for valid UTF-8 inputs or supported configs.

---

## [0.8.4] - 2025-12-31

### Changed

- Refactored `DictionaryError` into structured variants:
    - `IoError(io::Error)`
    - `CborParseError(serde_cbor::Error)`
    - `LoadFileError { path, lineno, message }`
- Replaced all string-based errors with rich underlying error types.
- Improved error messages surfaced through `Display` and the C API `opencc_last_error()`.
- Updated dictionary loading/serialization functions
  (`from_zstd`, `from_cbor`, `load_compressed`, `load_dict`, `serialize_to_cbor`)
  to use the new unified error model.

### Added

- Added strongly typed Rust API:
    - `OpenccConfig` enum (`#[repr(u32)]`) with stable numeric values.
    - `OpenCC::convert_with_config()` as the recommended, non-string dispatch path.
- Added numeric-config C API:
    - `opencc_config_t` (`uint32_t`) with ABI-stable constants.
    - `opencc_convert_cfg()` for conversion without string-based config parsing.
- Preserved legacy string-based APIs
  (`OpenCC::convert(&str, ...)`, `opencc_convert(...)`) for backward compatibility.

### C API Improvements

- Clarified ownership and lifetime rules:
    - Returned strings are always NUL-terminated and must be freed with
      `opencc_string_free()`.
    - Numeric config parameters are passed by value and require no allocation
      or cleanup.
- Standardized error behavior:
    - Invalid configs return a readable error string
      (e.g. `"Invalid config: 9999"`) and also populate
      `opencc_last_error()`.
- Deprecated unused length-based conversion entry points
  (planned removal in a future release).

### Developer Notes

- The numeric-config API is recommended for:
    - FFI bindings (C / C++ / C# / Java / Python)
    - Performance-sensitive code paths
    - Avoiding runtime string validation
- Header-only C++ helper (`OpenccFmmsegHelper.hpp`) updated to provide
  RAII-safe lifetime management and typed-config usage on top of the C API.

---

## [0.8.3] – 2025-10-20

### Added

- **Global key-length mask**: `DictMaxLen::key_length_mask (u64)`  
  Encodes presence of lengths `1..=64` (bit `n-1` → length `n`) for fast global gating (`has_key_len()`).
- **Per-starter length mask**: `DictMaxLen::starter_len_mask (FxHashMap<char, u64>)`  
  Exact per-starter length presence for BMP **and** astral chars. Dense BMP tables are rebuilt from this.
- **Adaptive pre-chunking** in `segment_replace_with_union()` for large input strings.  
  Automatically computes optimal parallel chunk sizes based on CPU count and text length.  
  Improves Rayon task distribution and minimizes overhead for long texts (e.g., 3 M chars in ~35 ms).  
  The pre-chunk allocator now precomputes total output capacity to reduce `String` reallocations.

### Changed

- **Mask-first gating** in `starter_allows_dict()`:
    - For `1..=64`, test the bit in `first_len_mask64` (or sparse `starter_len_mask`).
    - For `>64` (BMP), fall back to `first_char_max_len` (derived during build).
- **populate_starter_indexes()** now prefers `starter_len_mask` (single pass) and falls back to scanning `map` if empty.
- **Pre-chunking logic** restructured for **sequential-safe range building** using
  `get_split_ranges(inclusive = true)`.  
  The number of subranges is reduced while maintaining delimiter safety, improving throughput and reducing peak
  memory.  
  The pre-allocation phase now computes cumulative chunk lengths before Rayon processing begins.
- **Docs.rs** updated across the module (examples, helpers, dense/sparse behavior).
- **CBOR I/O** now uses `serde_cbor` (`from_slice` / `to_vec`). *No format change.*

### Removed

- **`starter_cap`** from `DictMaxLen` and JSON/CBOR.  
  Dense `first_char_max_len` is derived from masks at build/load time.  
  Older serialized files containing `starter_cap` remain compatible (unknown field is ignored).

### CLI / JSON export

- `dictionary_maxlength.json`:
    - **Removed:** `starter_cap`
    - **Added:** `key_length_mask`, `starter_len_mask` (string keys → `u64` values)
- CBOR/JSON inspector script updated to summarize the new masks.

### Migration

- Replace any `starter_cap` usage with:
    - `dict.has_starter_len(c, len)` (precise for `1..=64`), or
    - `dict.first_char_max_len[u as usize] >= len` (dense BMP; covers `>64`).
- If you previously serialized `DictMaxLen` directly to JSON (which failed due to `[char]` keys),
  use DTOs: `DictionaryMaxlengthSerde` / `DictMaxLenSerde`.
- Update tests to assert **semantic invariants** (mask coverage, min/max consistency) instead of file byte sizes.

### MSRV / Tooling

- **Rayon pins for MSRV 1.75**: `rayon = 1.10.x`, `rayon-core = 1.12.x`.
- **Release toolchain** pinned to **Rust 1.82.0** (avoids Windows AV heuristics).

### Verification

- Round-trip CBOR test via `serde_cbor::from_slice` validates:
  `max_len`, `key_length_mask`, and representative `map.len()` counts.
- Benchmarks confirm stable performance:
    - 3 M chars converted in **~35 ms** (C API via Qt C++ client).
    - No observable memory spike thanks to pre-chunk preallocation and mask-based gating.

---

## [0.8.2] - 2025-10-02

## Added

- **dict-generate JSON export**: Human-readable JSON via DTOs with String keys;  
  supports `--pretty` (pretty) and `--compact` (default). Core schema remains CBOR/Zstd.
  Why: JSON is for reference/debug only; the canonical on-disk format stays CBOR.
- Added `min_len` field to `DictMaxLen`

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
- **Error handling (unified)**: `from_zstd()`, `from_cbor()`, `serialize_to_cbor()`, `deserialize_from_cbor()`, and
  `new()` now return `Result<_, DictionaryError>` for consistent typed errors.

## Moved

- `serde (JSON) overrides`: Removed from core `DictMaxLen`; JSON adaptation now lives only in the `dict-generate` CLI (
  DTO layer).
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

