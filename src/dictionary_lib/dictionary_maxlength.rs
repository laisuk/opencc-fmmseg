//! Internal module for managing and loading OpenCC dictionaries.
//!
//! This module defines the [`DictionaryMaxlength`] struct, which stores all necessary
//! dictionaries and associated metadata used by the OpenCC text conversion engine.
//! Each dictionary is paired with a maximum word length for efficient forward maximum
//! matching (FMM) during segment-based replacement.
//!
//! Users generally interact with this indirectly via the `OpenCC` interface, but
//! advanced users may access it for custom loading, serialization, or optimization.

use crate::dictionary_lib::DictMaxLen;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use serde_cbor::{from_reader, from_slice};
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Cursor, Write};
use std::path::Path;
use std::sync::Mutex;
use std::{fs, io};
use zstd::{Decoder, Encoder};

mod union_cache;
pub(crate) use union_cache::UnionKey;
// so callers can say `UnionKey::S2T { punct: ... }`

// Define a global mutable variable to store the error message
static LAST_ERROR: Mutex<Option<String>> = Mutex::new(None);

/// Represents a collection of OpenCC dictionaries paired with their maximum word lengths.
///
/// This structure is used internally by the `OpenCC` engine to support fast,
/// segment-based forward maximum matching (FMM) for Chinese text conversion.
/// Each dictionary maps a phrase or character to its target form and tracks the
/// longest entry for lookup performance.
///
/// ## Built-in dictionary (default)
///
/// This crate ships **only one** prebuilt dictionary artifact:
///
/// - `dictionary_maxlength.zstd` (CBOR data compressed with Zstandard)
///
/// This is the **default dictionary** used by higher-level APIs and is sufficient
/// for most users. It provides fast, deterministic loading while keeping the
/// crate size reasonable.
///
/// The built-in `dictionary_maxlength.zstd` contains a standard
/// Zstandard-compressed CBOR payload; advanced users may decompress it
/// to obtain the raw `dictionary_maxlength.cbor` if needed.
///
/// ## Custom / regenerated dictionary artifacts
///
/// To avoid excessive crate size growth, the published crate does **not** ship:
///
/// - an uncompressed `dictionary_maxlength.cbor`
/// - JSON dictionary representations
///
/// If you need a custom dictionary (e.g. modified source `.txt` files, debugging,
/// or inspection), generate it locally using the `dict-generate` CLI tool from
/// the `opencc-fmmseg` workspace, then load it at runtime.
///
/// ### Generate
///
/// ```text
/// # Run in a directory that contains `dicts/` (OpenCC .txt dictionaries)
/// dict-generate --format cbor --output dictionary_maxlength.cbor
/// ```
///
/// ### Load
///
/// ```no_run
/// # use opencc_fmmseg::dictionary_lib::DictionaryMaxlength;
/// let dict = DictionaryMaxlength::deserialize_from_cbor("dictionary_maxlength.cbor")?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// The generated CBOR file is schema-compatible with the built-in Zstd-compressed
/// dictionary and can be used as a drop-in replacement.
#[derive(Serialize, Deserialize, Debug)]
pub struct DictionaryMaxlength {
    #[serde(default)]
    pub st_characters: DictMaxLen,
    #[serde(default)]
    pub st_phrases: DictMaxLen,
    #[serde(default)]
    pub ts_characters: DictMaxLen,
    #[serde(default)]
    pub ts_phrases: DictMaxLen,
    #[serde(default)]
    pub tw_phrases: DictMaxLen,
    #[serde(default)]
    pub tw_phrases_rev: DictMaxLen,
    #[serde(default)]
    pub tw_variants: DictMaxLen,
    #[serde(default)]
    pub tw_variants_rev: DictMaxLen,
    #[serde(default)]
    pub tw_variants_rev_phrases: DictMaxLen,
    #[serde(default)]
    pub hk_variants: DictMaxLen,
    #[serde(default)]
    pub hk_variants_rev: DictMaxLen,
    #[serde(default)]
    pub hk_variants_rev_phrases: DictMaxLen,
    #[serde(default)]
    pub jps_characters: DictMaxLen,
    #[serde(default)]
    pub jps_phrases: DictMaxLen,
    #[serde(default)]
    pub jp_variants: DictMaxLen,
    #[serde(default)]
    pub jp_variants_rev: DictMaxLen,
    #[serde(default)]
    pub st_punctuations: DictMaxLen,
    #[serde(default)]
    pub ts_punctuations: DictMaxLen,

    #[serde(skip)]
    #[serde(default)]
    unions: union_cache::Unions,
}

impl DictionaryMaxlength {
    /// Loads the default embedded Zstd-compressed dictionary.
    ///
    /// This constructor initializes a [`DictionaryMaxlength`] instance using the
    /// built-in, precompiled Zstd-compressed dictionary blob bundled with the
    /// crate.
    ///
    /// It is the recommended way to create a dictionary instance for normal
    /// application usage, as it provides:
    ///
    /// - Fast startup
    /// - Zero file-system access
    /// - A guaranteed, version-matched dictionary set
    ///
    /// On failure, this method stores a human-readable message in the internal
    /// error buffer via [`set_last_error`](Self::set_last_error), allowing
    /// foreign-language bindings (C, C#, Python, Java through JNI) to retrieve
    /// the error message safely.
    ///
    /// # Returns
    ///
    /// - `Ok(Self)` if the embedded Zstd dictionary is successfully decoded
    /// - `Err(DictionaryError)` if decompression or parsing fails
    ///
    /// # Notes
    ///
    /// This method is a thin wrapper around [`from_zstd`](Self::from_zstd),
    /// preserving its error while adding richer diagnostics.
    pub fn new() -> Result<Self, DictionaryError> {
        Self::from_zstd().map_err(|err| {
            let msg = format!("Failed to load dictionary from Zstd: {}", err);
            Self::set_last_error(&msg);
            err
        })
    }

    /// Loads the default dictionary from an **embedded Zstd-compressed CBOR blob**.
    ///
    /// This method is the fastest way to load the OpenCC dictionary at runtime,
    /// because the dictionary is:
    ///
    /// - **Embedded** in the binary at compile time via [`include_bytes!`].
    /// - **Pre-serialized** in CBOR format for compactness and fast parsing.
    /// - **Compressed** with Zstandard (Zstd) to reduce binary size.
    ///
    /// # Behavior
    /// 1. Reads the embedded `dicts/dictionary_maxlength.zstd` file directly from the binary.
    /// 2. Decompresses the Zstd data into raw CBOR bytes.
    /// 3. Deserializes the CBOR into a [`DictionaryMaxlength`] structure.
    /// 4. Calls [`finish`](#method.finish) to populate all starter indexes.
    ///
    /// # Advantages
    /// - **No disk I/O**: The dictionary is built into the compiled binary.
    /// - **Fast startup**: CBOR decoding + Zstd decompression is much faster
    ///   than parsing 18+ plaintext `.txt` files.
    /// - **Smaller binaries**: The Zstd-compressed CBOR blob is significantly smaller
    ///   than raw text or even uncompressed CBOR.
    ///
    /// # Example
    /// ```
    /// use opencc_fmmseg::dictionary_lib::DictionaryMaxlength;
    ///
    /// let dicts = DictionaryMaxlength::from_zstd().unwrap();
    /// assert!(dicts.st_characters.is_populated());
    /// ```
    ///
    /// # Errors
    /// - [`DictionaryError::IoError`] if Zstd decompression fails.
    /// - [`DictionaryError::CborParseError`] if CBOR deserialization fails.
    ///
    /// # See also
    /// - [`from_dicts`](#method.from_dicts) — loads from plaintext `.txt` files.
    /// - [`from_json`](#method.from_json) — loads from JSON.
    pub fn from_zstd() -> Result<Self, DictionaryError> {
        // Embedded compressed CBOR file at compile time
        let compressed_data = include_bytes!("dicts/dictionary_maxlength.zstd");

        let cursor = Cursor::new(compressed_data);
        let mut decoder = Decoder::new(cursor).map_err(DictionaryError::IoError)?;
        let dictionary: DictionaryMaxlength =
            from_reader(&mut decoder).map_err(DictionaryError::CborParseError)?;

        Ok(dictionary.finish())
    }

    /// Loads the dictionary from an embedded CBOR blob.
    ///
    /// ⚠️ **Deprecated**: the crate no longer ships the embedded
    /// `dicts/dictionary_maxlength.cbor` to reduce crate size.
    ///
    /// ### Recommended usage
    ///
    /// If you are **not using a custom dictionary**, prefer
    /// [`from_zstd`](Self::from_zstd), which loads the built-in
    /// Zstd-compressed dictionary bundled with the crate. This is the
    /// fastest and most convenient option for most users.
    ///
    /// ### Historical behavior
    ///
    /// This function previously loaded a CBOR dictionary embedded at
    /// compile time via:
    ///
    /// ```text
    /// dicts/dictionary_maxlength.cbor
    /// ```
    ///
    /// using `include_bytes!()`. That file is **no longer included** in
    /// published crate sources.
    ///
    /// ### Migration
    ///
    /// Use an externally generated CBOR dictionary instead.
    ///
    /// 1) Generate `dictionary_maxlength.cbor` (recommended via CLI):
    ///
    /// ```text
    /// dict-generate --format cbor --output dictionary_maxlength.cbor
    /// ```
    ///
    /// 2) Load it at runtime:
    ///
    /// - [`deserialize_from_cbor`](Self::deserialize_from_cbor)
    #[deprecated(
        since = "0.9.0",
        note = "Embedded CBOR is no longer shipped. Use from_zstd() for the default dictionary, or deserialize_from_cbor() with a generated CBOR file."
    )]
    pub fn from_cbor() -> Result<Self, DictionaryError> {
        // Historical / conventional location
        let path = Path::new("dicts/dictionary_maxlength.cbor");

        if !path.exists() {
            Self::set_last_error(
                "dictionary_maxlength.cbor not found at dicts/. \
This crate no longer ships embedded CBOR. \
Generate it via dict-generate or use deserialize_from_cbor(path).",
            );

            return Err(DictionaryError::IoError(io::Error::new(
                io::ErrorKind::NotFound,
                "dictionary_maxlength.cbor not found",
            )));
        }

        let cbor_data = fs::read(path).map_err(|err| {
            Self::set_last_error(&format!(
                "Failed to read CBOR file ({}): {}",
                path.display(),
                err
            ));
            DictionaryError::IoError(err)
        })?;

        let dictionary: DictionaryMaxlength = from_slice(&cbor_data).map_err(|err| {
            Self::set_last_error(&format!("Failed to parse CBOR: {}", err));
            DictionaryError::CborParseError(err)
        })?;

        Ok(dictionary.finish())
    }

    /// Loads all dictionaries from plaintext `.txt` lexicon files in the `dicts/` directory.
    ///
    /// This method reads the OpenCC-compatible source dictionaries from disk and builds
    /// a full [`DictionaryMaxlength`] with populated [`DictMaxLen`] instances for each table.
    ///
    /// # Expected directory structure
    ///
    /// The base directory is `"dicts"` (relative to the process working directory).
    /// It must contain the standard OpenCC text dictionary files:
    ///
    /// ```bash
    /// dicts/
    /// ├── STCharacters.txt
    /// ├── STPhrases.txt
    /// ├── TSCharacters.txt
    /// ├── TSPhrases.txt
    /// ├── TWPhrases.txt
    /// ├── TWPhrasesRev.txt
    /// ├── TWVariants.txt
    /// ├── TWVariantsRev.txt
    /// ├── TWVariantsRevPhrases.txt
    /// ├── HKVariants.txt
    /// ├── HKVariantsRev.txt
    /// ├── HKVariantsRevPhrases.txt
    /// ├── JPShinjitaiCharacters.txt
    /// ├── JPShinjitaiPhrases.txt
    /// ├── JPVariants.txt
    /// ├── JPVariantsRev.txt
    /// ├── STPunctuations.txt
    /// └── TSPunctuations.txt
    /// ```
    ///
    /// # File format
    ///
    /// Each `.txt` file contains tab-separated key-value pairs:
    /// ```bash
    /// # This is a comment
    /// 你好\t您好
    /// 世界\t世間
    /// ```
    ///
    /// - Lines starting with `#` are ignored.
    /// - Empty lines are ignored.
    /// - Leading/trailing carriage returns (`\r`) are stripped automatically.
    /// - A UTF-8 BOM (`\u{FEFF}`) is stripped if present in the first data line.
    /// - The **first whitespace-separated token** after the TAB is taken as the value;
    ///   the rest of the line (if any) is ignored.
    ///
    /// # Behavior
    ///
    /// - Builds each [`DictMaxLen`] using [`DictMaxLen::build_from_pairs`], which
    ///   also populates starter indexes.
    /// - Returns an error if any data line is missing a TAB separator.
    /// - Returns an error if a file cannot be read.
    ///
    /// # Usage
    /// ```
    /// use opencc_fmmseg::dictionary_lib::DictionaryMaxlength;
    ///
    /// let dicts = DictionaryMaxlength::from_dicts().unwrap();
    /// assert!(dicts.st_characters.is_populated());
    /// assert!(dicts.ts_phrases.is_populated());
    /// ```
    ///
    /// # Errors
    /// - [`DictionaryError::IoError`] if a dictionary file cannot be read.
    /// - [`DictionaryError::LoadFileError`] if a data line is malformed (missing TAB).
    ///
    /// # See also
    /// - [`populate_all`](#method.populate_all) — rebuilds starter indexes after bulk edits.
    /// - [`finish`](#method.finish) — chaining version of `populate_all` after deserialization.
    pub fn from_dicts() -> Result<Self, DictionaryError> {
        Self::from_dicts_custom(&[])
    }

    /// Populates starter indexes for all inner [`DictMaxLen`] tables in this structure.
    ///
    /// This calls [`DictMaxLen::populate_starter_indexes`] on each dictionary field,
    /// rebuilding both the **BMP length masks** (`first_len_mask64`) and the **per-starter
    /// maximum length arrays** (`first_char_max_len`).
    ///
    /// This method should be run after any bulk changes to dictionary contents,
    /// especially after deserialization or manual editing of `map`/`starter_cap`.
    ///
    /// # Behavior
    /// - Only affects runtime accelerator fields; does not modify `map`, `max_len`, or `starter_cap`.
    /// - Skips non-BMP starter characters in each dictionary for efficiency.
    ///
    /// # When to use
    /// - Immediately after loading from disk or a serialized format.
    /// - After programmatically inserting or removing multiple entries from any dictionary.
    ///
    /// # Example
    /// ```
    /// use opencc_fmmseg::dictionary_lib::DictionaryMaxlength;
    /// # let mut dicts = DictionaryMaxlength::default(); // assume default exists
    /// dicts.populate_all();
    /// assert!(dicts.st_characters.is_populated());
    /// assert!(dicts.ts_characters.is_populated());
    /// ```
    pub fn populate_all(&mut self) {
        self.st_characters.populate_starter_indexes();
        self.st_phrases.populate_starter_indexes();
        self.ts_characters.populate_starter_indexes();
        self.ts_phrases.populate_starter_indexes();
        self.tw_phrases.populate_starter_indexes();
        self.tw_phrases_rev.populate_starter_indexes();
        self.tw_variants.populate_starter_indexes();
        self.tw_variants_rev.populate_starter_indexes();
        self.tw_variants_rev_phrases.populate_starter_indexes();
        self.hk_variants.populate_starter_indexes();
        self.hk_variants_rev.populate_starter_indexes();
        self.hk_variants_rev_phrases.populate_starter_indexes();
        self.jps_characters.populate_starter_indexes();
        self.jps_phrases.populate_starter_indexes();
        self.jp_variants.populate_starter_indexes();
        self.jp_variants_rev.populate_starter_indexes();
        self.st_punctuations.populate_starter_indexes();
        self.ts_punctuations.populate_starter_indexes();
    }

    /// Finalizes internal metadata after deserialization or bulk loading.
    ///
    /// Dictionary structures loaded from CBOR, Zstd-compressed CBOR, or plaintext
    /// sources may not have their derived fields populated yet (such as maximum
    /// key lengths, starter masks, or other precomputed lookup metadata).
    ///
    /// This method performs the required post-processing by invoking
    /// [`populate_all`](Self::populate_all), ensuring that:
    ///
    /// - All `DictMaxLen` tables have accurate `max_len` values
    /// - Starter masks used by [`StarterUnion`] are correctly computed
    /// - Internal structures required for longest-match segmentation are prepared
    ///
    /// Once finalized, the dictionary instance is fully ready for high-performance
    /// conversions via `OpenCC`.
    ///
    /// # Returns
    ///
    /// Returns `self` after populating all derived fields, allowing chaining with
    /// constructors or deserializers.
    ///
    /// # Examples
    ///
    /// Not shown here (internal helper).
    #[inline]
    pub fn finish(mut self) -> Self {
        self.populate_all();
        self
    }
    #[cfg(debug_assertions)]
    pub fn debug_assert_populated(&self) {
        let all = [
            &self.st_characters,
            &self.st_phrases,
            &self.ts_characters,
            &self.ts_phrases,
            &self.tw_phrases,
            &self.tw_phrases_rev,
            &self.tw_variants,
            &self.tw_variants_rev,
            &self.tw_variants_rev_phrases,
            &self.hk_variants,
            &self.hk_variants_rev,
            &self.hk_variants_rev_phrases,
            &self.jps_characters,
            &self.jps_phrases,
            &self.jp_variants,
            &self.jp_variants_rev,
            &self.st_punctuations,
            &self.ts_punctuations,
        ];
        for d in all {
            debug_assert!(
                d.is_populated(),
                "Starter indexes not populated for a DictMaxLen"
            );
        }
    }

    /// Exports all internal dictionaries as plaintext `.txt` files.
    ///
    /// This utility writes each dictionary (phrases, characters, variants, and
    /// reverse mappings) into a human-readable UTF-8 text file inside the
    /// specified directory.
    ///
    /// Each output file uses the format:
    ///
    /// ```text
    /// <key>\t<value>
    /// ```
    ///
    /// where:
    ///
    /// - `key` is reconstructed from the internal `Box<[char]>` representation
    /// - `value` is the mapped string
    ///
    /// Dictionaries exported include:
    ///
    /// - Simplified → Traditional (phrases/characters/punctuation)
    /// - Traditional → Simplified (phrases/characters/punctuation)
    /// - Taiwanese phrases, variants, and reverse mappings
    /// - Hong Kong variants and reverse mappings
    /// - Japanese Shinjitai/Kyūjitai mappings
    ///
    /// The function creates the target directory if it does not already exist.
    ///
    /// # Arguments
    ///
    /// * `base_dir` — Directory path where dictionary `.txt` files will be
    ///   written. The directory will be created if necessary.
    ///
    /// # Returns
    ///
    /// - `Ok(())` if all files were successfully written
    /// - `Err` if file creation or writing fails
    ///
    /// # Notes
    ///
    /// This method is mainly intended for debugging, verification, and for users
    /// who want access to the raw dictionary data in plain text form.
    pub fn to_dicts(&self, base_dir: &str) -> Result<(), Box<dyn Error>> {
        let dict_map: FxHashMap<&str, &FxHashMap<Box<[char]>, Box<str>>> = [
            ("STCharacters.txt", &self.st_characters.map),
            ("STPhrases.txt", &self.st_phrases.map),
            ("TSCharacters.txt", &self.ts_characters.map),
            ("TSPhrases.txt", &self.ts_phrases.map),
            ("TWPhrases.txt", &self.tw_phrases.map),
            ("TWPhrasesRev.txt", &self.tw_phrases_rev.map),
            ("TWVariants.txt", &self.tw_variants.map),
            ("TWVariantsRev.txt", &self.tw_variants_rev.map),
            (
                "TWVariantsRevPhrases.txt",
                &self.tw_variants_rev_phrases.map,
            ),
            ("HKVariants.txt", &self.hk_variants.map),
            ("HKVariantsRev.txt", &self.hk_variants_rev.map),
            (
                "HKVariantsRevPhrases.txt",
                &self.hk_variants_rev_phrases.map,
            ),
            ("JPShinjitaiCharacters.txt", &self.jps_characters.map),
            ("JPShinjitaiPhrases.txt", &self.jps_phrases.map),
            ("JPVariants.txt", &self.jp_variants.map),
            ("JPVariantsRev.txt", &self.jp_variants_rev.map),
            ("STPunctuations.txt", &self.st_punctuations.map),
            ("TSPunctuations.txt", &self.ts_punctuations.map),
        ]
        .into_iter()
        .collect();

        fs::create_dir_all(base_dir)?; // ensure base_dir exists

        for (filename, dict) in dict_map {
            let path = Path::new(base_dir).join(filename);
            let mut file = File::create(&path)?;

            for (key, value) in dict {
                // Convert &[char] → String for writing
                let key_str: String = key.iter().collect();
                writeln!(file, "{}\t{}", key_str, value)?;
            }
        }

        Ok(())
    }

    // ------ Custom Dictionary Start -----

    /// Loads all dictionaries from plaintext `.txt` lexicon files in the default
    /// `dicts/` directory and applies optional custom dictionary overrides.
    ///
    /// This is the core constructor for building a [`DictionaryMaxlength`] from
    /// OpenCC-compatible plaintext dictionary files.
    ///
    /// Custom dictionary pairs are merged before internal starter indexes and
    /// maximum phrase lengths are rebuilt.
    ///
    /// # Base directory
    ///
    /// The default base directory is:
    ///
    /// ```text
    /// dicts/
    /// ```
    ///
    /// relative to the current process working directory.
    ///
    /// # Custom dictionary behavior
    ///
    /// Custom dictionaries are merged into the selected [`DictSlot`] using
    /// [`CustomDictMode::Append`] or [`CustomDictMode::Override`].
    ///
    /// # Example
    ///
    /// ```rust
    /// use opencc_fmmseg::{
    ///     CustomDictMode,
    ///     CustomDictSpec,
    ///     DictSlot,
    ///     DictionaryMaxlength,
    /// };
    ///
    /// let dictionary = DictionaryMaxlength::from_dicts_custom(&[
    ///     CustomDictSpec {
    ///         slot: DictSlot::STPhrases,
    ///         pairs: vec![
    ///             ("帕兰蒂尔".to_string(), "柏蘭蒂爾".to_string()),
    ///         ],
    ///         mode: CustomDictMode::Override,
    ///     }
    /// ]).unwrap();
    ///
    /// assert!(dictionary.st_phrases.is_populated());
    /// ```
    ///
    /// # Notes
    ///
    /// This method internally delegates to:
    ///
    /// - [`DictionaryMaxlength::from_dicts_custom_at()`]
    ///
    /// using the default `"dicts"` directory.
    ///
    /// # Errors
    ///
    /// - [`DictionaryError::IoError`] if a dictionary file cannot be read.
    /// - [`DictionaryError::LoadFileError`] if a data line is malformed.
    ///
    /// # See Also
    ///
    /// - [`DictionaryMaxlength::from_dicts()`]
    /// - [`DictionaryMaxlength::from_dicts_custom_files()`]
    /// - [`CustomDictSpec`]
    /// - [`CustomDictMode`]
    /// - [`DictSlot`]
    pub fn from_dicts_custom(specs: &[CustomDictSpec]) -> Result<Self, DictionaryError> {
        Self::from_dicts_custom_at("dicts", specs)
    }

    /// Loads custom dictionaries from one or more OpenCC-style plaintext files.
    ///
    /// This is a convenience wrapper around
    /// [`DictionaryMaxlength::from_dicts_custom()`].
    ///
    /// Each file is parsed using the standard OpenCC dictionary format:
    ///
    /// ```text
    /// source<TAB>target
    /// ```
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use opencc_fmmseg::{
    ///     CustomDictFileSpec,
    ///     CustomDictMode,
    ///     DictSlot,
    ///     DictionaryMaxlength,
    /// };
    ///
    /// let dictionary = DictionaryMaxlength::from_dicts_custom_files(&[
    ///     CustomDictFileSpec {
    ///         slot: DictSlot::STPhrases,
    ///         files: vec!["./my_terms.txt"],
    ///         mode: CustomDictMode::Override,
    ///     }
    /// ]).unwrap();
    ///
    /// assert!(dictionary.st_phrases.is_populated());
    /// ```
    ///
    /// # Multiple files
    ///
    /// Multiple files may be provided for the same slot:
    ///
    /// ```rust,no_run
    /// use opencc_fmmseg::{
    ///     CustomDictFileSpec,
    ///     CustomDictMode,
    ///     DictSlot,
    ///     DictionaryMaxlength,
    /// };
    ///
    /// let dictionary = DictionaryMaxlength::from_dicts_custom_files(&[
    ///     CustomDictFileSpec {
    ///         slot: DictSlot::STPhrases,
    ///         files: vec![
    ///             "./brand_terms.txt",
    ///             "./product_terms.txt",
    ///         ],
    ///         mode: CustomDictMode::Append,
    ///     }
    /// ]).unwrap();
    ///
    /// assert!(dictionary.st_phrases.is_populated());
    /// ```
    ///
    /// Files are loaded sequentially in the provided order.
    ///
    /// # Errors
    ///
    /// - [`DictionaryError::IoError`] if a file cannot be read.
    /// - [`DictionaryError::LoadFileError`] if a line is malformed.
    ///
    /// # See Also
    ///
    /// - [`DictionaryMaxlength::from_dicts_custom()`]
    /// - [`CustomDictFileSpec`]
    /// - [`CustomDictMode`]
    /// - [`DictSlot`]
    pub fn from_dicts_custom_files<P>(
        specs: &[CustomDictFileSpec<P>],
    ) -> Result<Self, DictionaryError>
    where
        P: AsRef<Path>,
    {
        let pair_specs = specs
            .iter()
            .map(|spec| {
                let mut pairs = Vec::new();

                for file in &spec.files {
                    pairs.extend(Self::load_pairs_from_path(file)?);
                }

                Ok(CustomDictSpec {
                    slot: spec.slot,
                    pairs,
                    mode: spec.mode,
                })
            })
            .collect::<Result<Vec<_>, DictionaryError>>()?;

        Self::from_dicts_custom(&pair_specs)
    }

    /// Loads dictionaries from an alternate base directory.
    ///
    /// This behaves similarly to [`DictionaryMaxlength::from_dicts()`], except
    /// the OpenCC plaintext dictionary files are loaded from the specified
    /// directory instead of the default `dicts/`.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use opencc_fmmseg::DictionaryMaxlength;
    ///
    /// let dictionary = DictionaryMaxlength::from_dicts_at("./my_opencc_dicts")
    ///     .unwrap();
    ///
    /// assert!(dictionary.st_phrases.is_populated());
    /// ```
    ///
    /// # Expected structure
    ///
    /// The provided directory should contain the standard OpenCC dictionary files:
    ///
    /// ```text
    /// STCharacters.txt
    /// STPhrases.txt
    /// TSCharacters.txt
    /// TSPhrases.txt
    /// ...
    /// ```
    ///
    /// # Errors
    ///
    /// - [`DictionaryError::IoError`] if a dictionary file cannot be read.
    /// - [`DictionaryError::LoadFileError`] if a line is malformed.
    ///
    /// # See Also
    ///
    /// - [`DictionaryMaxlength::from_dicts()`]
    /// - [`DictionaryMaxlength::from_dicts_custom()`]
    pub fn from_dicts_at<P: AsRef<Path>>(base_dir: P) -> Result<Self, DictionaryError> {
        Self::from_dicts_custom_at(base_dir, &[])
    }

    /// Core plaintext dictionary constructor used by all `from_dicts*` loaders.
    ///
    /// This method loads the standard OpenCC-style `.txt` dictionary files from
    /// `base_dir`, applies optional custom dictionary specs at the raw pair level,
    /// and then builds each [`DictMaxLen`] slot from the final merged pairs.
    ///
    /// Custom entries are merged before [`DictMaxLen::build_from_pairs`] is called,
    /// so max phrase lengths and starter indexes are rebuilt from the complete
    /// final dictionary data.
    ///
    /// Public wrappers:
    ///
    /// - [`DictionaryMaxlength::from_dicts`]
    /// - [`DictionaryMaxlength::from_dicts_at`]
    /// - [`DictionaryMaxlength::from_dicts_custom`]
    /// - [`DictionaryMaxlength::from_dicts_custom_files`]
    ///
    /// # Merge flow
    ///
    /// ```text
    /// base .txt file
    ///   -> raw Vec<(String, String)>
    ///   -> apply custom append/override pairs for the matching DictSlot
    ///   -> DictMaxLen::build_from_pairs(...)
    ///   -> populated dictionary slot
    /// ```
    ///
    /// # Notes
    ///
    /// This function intentionally rebuilds every slot from pairs instead of
    /// mutating existing [`DictMaxLen`] values. This keeps starter indexes,
    /// max-length metadata, and runtime lookup structures consistent.
    ///
    /// `unions` is initialized with [`Default::default`] because it is a runtime-only
    /// cache and should be rebuilt lazily by conversion logic when needed.
    ///
    /// # Errors
    ///
    /// Returns [`DictionaryError::IoError`] if `base_dir` does not exist or a
    /// dictionary file cannot be opened/read.
    ///
    /// Returns [`DictionaryError::LoadFileError`] if a dictionary line is malformed.
    fn from_dicts_custom_at<P: AsRef<Path>>(
        base_dir: P,
        specs: &[CustomDictSpec],
    ) -> Result<Self, DictionaryError> {
        let base_dir = base_dir.as_ref();

        if !base_dir.exists() {
            let msg = format!("Base directory not found: {}", base_dir.display());
            Self::set_last_error(&msg);
            return Err(DictionaryError::IoError(io::Error::new(
                io::ErrorKind::NotFound,
                msg,
            )));
        }

        fn load_slot(
            base_dir: &Path,
            filename: &str,
            specs: &[CustomDictSpec],
            slot: DictSlot,
        ) -> Result<DictMaxLen, DictionaryError> {
            let pairs = DictionaryMaxlength::load_pairs(base_dir, filename)?;
            let pairs = apply_custom_pairs(pairs, specs, slot);
            Ok(DictMaxLen::build_from_pairs(pairs))
        }

        Ok(DictionaryMaxlength {
            st_characters: load_slot(base_dir, "STCharacters.txt", specs, DictSlot::STCharacters)?,
            st_phrases: load_slot(base_dir, "STPhrases.txt", specs, DictSlot::STPhrases)?,
            ts_characters: load_slot(base_dir, "TSCharacters.txt", specs, DictSlot::TSCharacters)?,
            ts_phrases: load_slot(base_dir, "TSPhrases.txt", specs, DictSlot::TSPhrases)?,

            tw_phrases: load_slot(base_dir, "TWPhrases.txt", specs, DictSlot::TWPhrases)?,
            tw_phrases_rev: load_slot(base_dir, "TWPhrasesRev.txt", specs, DictSlot::TWPhrasesRev)?,
            tw_variants: load_slot(base_dir, "TWVariants.txt", specs, DictSlot::TWVariants)?,
            tw_variants_rev: load_slot(
                base_dir,
                "TWVariantsRev.txt",
                specs,
                DictSlot::TWVariantsRev,
            )?,
            tw_variants_rev_phrases: load_slot(
                base_dir,
                "TWVariantsRevPhrases.txt",
                specs,
                DictSlot::TWVariantsRevPhrases,
            )?,

            hk_variants: load_slot(base_dir, "HKVariants.txt", specs, DictSlot::HKVariants)?,
            hk_variants_rev: load_slot(
                base_dir,
                "HKVariantsRev.txt",
                specs,
                DictSlot::HKVariantsRev,
            )?,
            hk_variants_rev_phrases: load_slot(
                base_dir,
                "HKVariantsRevPhrases.txt",
                specs,
                DictSlot::HKVariantsRevPhrases,
            )?,

            jps_characters: load_slot(
                base_dir,
                "JPShinjitaiCharacters.txt",
                specs,
                DictSlot::JPShinjitaiCharacters,
            )?,
            jps_phrases: load_slot(
                base_dir,
                "JPShinjitaiPhrases.txt",
                specs,
                DictSlot::JPShinjitaiPhrases,
            )?,
            jp_variants: load_slot(base_dir, "JPVariants.txt", specs, DictSlot::JPVariants)?,
            jp_variants_rev: load_slot(
                base_dir,
                "JPVariantsRev.txt",
                specs,
                DictSlot::JPVariantsRev,
            )?,

            st_punctuations: load_slot(
                base_dir,
                "STPunctuations.txt",
                specs,
                DictSlot::STPunctuations,
            )?,
            ts_punctuations: load_slot(
                base_dir,
                "TSPunctuations.txt",
                specs,
                DictSlot::TSPunctuations,
            )?,

            unions: Default::default(),
        })
    }

    /// Loads raw dictionary pairs from a file inside a base directory.
    ///
    /// This is a small convenience wrapper around
    /// [`DictionaryMaxlength::load_pairs_from_path`] that joins
    /// `base_dir` and `filename`.
    ///
    /// # Notes
    ///
    /// The returned pairs are not yet converted into [`DictMaxLen`].
    /// Callers typically pass the result into:
    ///
    /// - [`apply_custom_pairs`]
    /// - [`DictMaxLen::build_from_pairs`]
    ///
    /// # File format
    ///
    /// Expected format:
    ///
    /// ```text
    /// source<TAB>target
    /// ```
    ///
    /// Comment lines starting with `#` and empty lines are ignored.
    fn load_pairs<P: AsRef<Path>>(
        base_dir: P,
        filename: &str,
    ) -> Result<Vec<(String, String)>, DictionaryError> {
        let path = base_dir.as_ref().join(filename);
        Self::load_pairs_from_path(path)
    }

    /// Loads raw `(source, target)` dictionary pairs from an OpenCC-style
    /// plaintext dictionary file.
    ///
    /// This parser is shared by:
    ///
    /// - [`DictionaryMaxlength::from_dicts`]
    /// - [`DictionaryMaxlength::from_dicts_at`]
    /// - [`DictionaryMaxlength::from_dicts_custom`]
    /// - [`DictionaryMaxlength::from_dicts_custom_files`]
    ///
    /// # Supported format
    ///
    /// ```text
    /// # comment
    /// 你好<TAB>您好
    /// 世界<TAB>世間
    /// ```
    ///
    /// # Parsing behavior
    ///
    /// - Empty lines are ignored.
    /// - Lines starting with `#` are ignored.
    /// - Trailing `\r` and whitespace are stripped automatically.
    /// - UTF-8 BOM (`\u{FEFF}`) is stripped from the first data line if present.
    /// - The first whitespace-separated token after the TAB is used as the value.
    /// - Additional tokens after the first value are ignored.
    ///
    /// # Errors
    ///
    /// Returns [`DictionaryError::LoadFileError`] if a non-comment line is missing
    /// a TAB separator.
    ///
    /// Returns [`DictionaryError::IoError`] if the file cannot be opened or read.
    ///
    /// # Notes
    ///
    /// This function only parses raw pairs and does not populate starter indexes
    /// or build [`DictMaxLen`] structures.
    fn load_pairs_from_path<P: AsRef<Path>>(
        path: P,
    ) -> Result<Vec<(String, String)>, DictionaryError> {
        let path = path.as_ref();
        let path_str = path.display().to_string();

        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let mut pairs = Vec::new();
        let mut saw_data_line = false;

        for (lineno, raw_line) in reader.lines().enumerate() {
            let raw_line = raw_line?;
            let mut line = raw_line.trim_end();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if !saw_data_line {
                if let Some(rest) = line.strip_prefix('\u{FEFF}') {
                    line = rest;
                }
                saw_data_line = true;
            }

            let Some((k, v)) = line.split_once('\t') else {
                return Err(DictionaryError::LoadFileError {
                    path: path_str.clone(),
                    lineno: lineno + 1,
                    message: "missing TAB separator".to_string(),
                });
            };

            let first_value = v.split_whitespace().next().unwrap_or("");
            pairs.push((k.to_owned(), first_value.to_owned()));
        }

        Ok(pairs)
    }

    // ------ Custom Dictionary End -----

    /// Serializes this dictionary to a CBOR file.
    ///
    /// This writes a compact binary snapshot of the entire [`DictionaryMaxlength`]
    /// using `serde_cbor`.
    ///
    /// ## Intended use
    ///
    /// - Cache a fully-built dictionary for fast startup
    /// - Distribute a prebuilt dictionary artifact outside the crate
    /// - Produce the CBOR used by [`deserialize_from_cbor`](Self::deserialize_from_cbor)
    ///
    /// ## What’s inside
    ///
    /// - All phrase/character maps
    /// - Variant and reverse-variant tables
    /// - Any metadata currently present in the struct
    ///
    /// ## Errors / FFI diagnostics
    ///
    /// On failure, a human-readable message is written to the global last-error buffer
    /// via [`set_last_error`](Self::set_last_error).
    pub fn serialize_to_cbor<P: AsRef<Path>>(&self, path: P) -> Result<(), DictionaryError> {
        let cbor_data = serde_cbor::to_vec(self).map_err(|err| {
            let msg = format!("Failed to serialize to CBOR: {}", err);
            Self::set_last_error(&msg);
            DictionaryError::CborParseError(err)
        })?;

        fs::write(&path, cbor_data).map_err(|err| {
            let msg = format!("Failed to write CBOR file: {}", err);
            Self::set_last_error(&msg);
            DictionaryError::IoError(err)
        })?;

        Ok(())
    }

    /// Deserializes a dictionary from a CBOR file.
    ///
    /// This reads a CBOR-encoded [`DictionaryMaxlength`] produced by
    /// [`serialize_to_cbor`](Self::serialize_to_cbor) or the `dict-generate` CLI.
    ///
    /// After decoding, the dictionary is finalized via [`finish`](Self::finish)
    /// (e.g., max-key-length metadata used by longest-match segmentation).
    ///
    /// ## Errors / FFI diagnostics
    ///
    /// On failure, a human-readable message is written to the global last-error buffer
    /// via [`set_last_error`](Self::set_last_error).
    pub fn deserialize_from_cbor<P: AsRef<Path>>(path: P) -> Result<Self, DictionaryError> {
        let file = File::open(&path).map_err(|err| {
            let msg = format!("Failed to read CBOR file: {}", err);
            Self::set_last_error(&msg);
            DictionaryError::IoError(err)
        })?;
        let reader = BufReader::new(file);

        let dictionary: DictionaryMaxlength = from_reader(reader).map_err(|err| {
            let msg = format!("Failed to deserialize CBOR: {}", err);
            Self::set_last_error(&msg);
            DictionaryError::CborParseError(err)
        })?;

        Ok(dictionary.finish())
    }

    /// Stores a human-readable error message for later retrieval.
    ///
    /// This function records the most recent error encountered during dictionary
    /// operations such as loading, parsing, serialization, or file I/O.
    ///
    /// The message is written into the global thread-safe buffer
    /// [`LAST_ERROR`], which is shared across FFI bindings (C, C#, Python,
    /// Java/JNI).
    ///
    /// Foreign callers that cannot rely on Rust's `Result` system can retrieve
    /// this message using [`get_last_error`](Self::get_last_error).
    ///
    /// # Arguments
    ///
    /// * `err_msg` — The error message to record.
    ///
    /// # Notes
    ///
    /// - This function overwrites any previously stored message.
    /// - The stored message is `String`-backed and safe to clone across
    ///   language boundaries.
    pub fn set_last_error(err_msg: &str) {
        let mut last_error = LAST_ERROR.lock().unwrap();
        *last_error = Some(err_msg.to_string());
    }

    /// Returns the most recently recorded error message, if any.
    ///
    /// This function reads from the global error buffer [`LAST_ERROR`], which is
    /// populated by calls to [`set_last_error`](Self::set_last_error) during
    /// dictionary loading, parsing, serialization, and external-resource I/O.
    ///
    /// It is primarily intended for FFI consumers (C, C#, Python, Java/JNI)
    /// that require explicit error retrieval after a failure in an exported
    /// function.
    ///
    /// # Returns
    ///
    /// - `Some(String)` containing the last error message
    /// - `None` if no error has been recorded
    ///
    /// # Notes
    ///
    /// This function clones the stored string, ensuring safe ownership transfer
    /// to external callers.
    pub fn get_last_error() -> Option<String> {
        let last_error = LAST_ERROR.lock().unwrap();
        last_error.clone()
    }

    /// Saves the dictionary to a Zstd-compressed CBOR file.
    ///
    /// This function serializes the entire [`DictionaryMaxlength`] structure into
    /// CBOR format using `serde_cbor` and compresses it with Zstd (compression
    /// level `19`) for efficient storage.
    ///
    /// The resulting file is suitable for:
    ///
    /// - Distributing custom dictionary builds
    /// - Loading via [`load_compressed`](Self::load_cbor_compressed)
    /// - Embedding as an asset in external applications
    ///
    /// Unlike [`serialize_to_cbor`](Self::serialize_to_cbor), this function
    /// performs both **serialization** and **compression** in one step.
    ///
    /// # Arguments
    ///
    /// * `dictionary` — The dictionary instance to serialize.
    /// * `path` — Destination file path for the compressed CBOR output.
    ///
    /// # Returns
    ///
    /// - `Ok(())` on success
    /// - `Err(DictionaryError)` if serialization or I/O fails
    ///
    /// # Notes
    ///
    /// The dictionary is written **as-is** without calling [`finish`](Self::finish),
    /// assuming it is already in a finalized state.
    pub fn save_cbor_compressed(
        dictionary: &DictionaryMaxlength,
        path: &str,
    ) -> Result<(), DictionaryError> {
        let file = File::create(path).map_err(|e| DictionaryError::IoError(e))?;
        let writer = BufWriter::new(file);
        let mut encoder = Encoder::new(writer, 19).map_err(|e| DictionaryError::IoError(e))?;
        serde_cbor::to_writer(&mut encoder, dictionary)
            .map_err(|e| DictionaryError::CborParseError(e))?;
        encoder.finish().map_err(|e| DictionaryError::IoError(e))?;
        Ok(())
    }

    /// Loads the dictionary from a Zstd-compressed CBOR file.
    ///
    /// This function reverses [`save_compressed`](Self::save_cbor_compressed) by:
    ///
    /// 1. Opening the specified file
    /// 2. Decompressing its Zstd stream
    /// 3. Deserializing the embedded CBOR dictionary
    /// 4. Finalizing metadata via [`finish`](Self::finish)
    ///
    /// The result is a fully initialized [`DictionaryMaxlength`] ready for use
    /// by the OpenCC segmentation engine.
    ///
    /// # Arguments
    ///
    /// * `path` — Path to a Zstd-compressed CBOR dictionary
    ///   (e.g. `dictionary_maxlength.zstd`).
    ///
    /// # Returns
    ///
    /// - `Ok(DictionaryMaxlength)` if decoding succeeds
    /// - `Err(DictionaryError)` if the file cannot be opened, decompressed, or parsed
    ///
    /// # Notes
    ///
    /// Zstd compression makes large dictionary bundles highly compact while
    /// maintaining fast load times.
    pub fn load_cbor_compressed(path: &str) -> Result<DictionaryMaxlength, DictionaryError> {
        let file = File::open(path).map_err(DictionaryError::IoError)?;
        let reader = BufReader::new(file);

        // `zstd::Decoder::new` returns an `io::Error` internally, so `IoError` is fine here.
        let mut decoder = Decoder::new(reader).map_err(DictionaryError::IoError)?;

        let dictionary: DictionaryMaxlength =
            from_reader(&mut decoder).map_err(DictionaryError::CborParseError)?;

        Ok(dictionary.finish())
    }

    // ------ New: Update dict map pairs dynamically ------

    fn slot_mut(&mut self, slot: DictSlot) -> &mut DictMaxLen {
        match slot {
            DictSlot::STCharacters => &mut self.st_characters,
            DictSlot::STPhrases => &mut self.st_phrases,
            DictSlot::TSCharacters => &mut self.ts_characters,
            DictSlot::TSPhrases => &mut self.ts_phrases,

            DictSlot::TWPhrases => &mut self.tw_phrases,
            DictSlot::TWPhrasesRev => &mut self.tw_phrases_rev,
            DictSlot::TWVariants => &mut self.tw_variants,
            DictSlot::TWVariantsRev => &mut self.tw_variants_rev,
            DictSlot::TWVariantsRevPhrases => &mut self.tw_variants_rev_phrases,

            DictSlot::HKVariants => &mut self.hk_variants,
            DictSlot::HKVariantsRev => &mut self.hk_variants_rev,
            DictSlot::HKVariantsRevPhrases => &mut self.hk_variants_rev_phrases,

            DictSlot::JPShinjitaiCharacters => &mut self.jps_characters,
            DictSlot::JPShinjitaiPhrases => &mut self.jps_phrases,
            DictSlot::JPVariants => &mut self.jp_variants,
            DictSlot::JPVariantsRev => &mut self.jp_variants_rev,

            DictSlot::STPunctuations => &mut self.st_punctuations,
            DictSlot::TSPunctuations => &mut self.ts_punctuations,
        }
    }

    fn apply_custom_pairs_to_slot(
        slot: &mut DictMaxLen,
        pairs: &[(String, String)],
        mode: CustomDictMode,
    ) {
        match mode {
            CustomDictMode::Append => {
                slot.append_pairs(pairs.iter().map(|(k, v)| (k.as_str(), v.as_str())));
            }
            CustomDictMode::Override => {
                slot.replace_pairs(pairs.iter().map(|(k, v)| (k.as_str(), v.as_str())));
            }
        }
    }

    /// Applies in-memory custom dictionaries to an already loaded dictionary.
    ///
    /// This is useful after loading built-in data with constructors such as
    /// [`from_zstd`](Self::from_zstd), or after loading external serialized
    /// dictionaries with [`deserialize_from_cbor`](Self::deserialize_from_cbor)
    /// or [`deserialize_from_json`](Self::deserialize_from_json). Each spec is
    /// applied to its selected [`DictSlot`]: append mode merges pairs with
    /// last-wins semantics, while override mode clears the slot first.
    ///
    /// The affected slot metadata and starter indexes are rebuilt during
    /// customization. The returned dictionary is ready for
    /// [`OpenCC::from_dictionary`](crate::OpenCC::from_dictionary), and
    /// conversion hot paths remain immutable after construction.
    pub fn with_custom_dicts(mut self, specs: &[CustomDictSpec]) -> Result<Self, DictionaryError> {
        for spec in specs {
            let slot = self.slot_mut(spec.slot);
            Self::apply_custom_pairs_to_slot(slot, &spec.pairs, spec.mode);
        }

        Ok(self)
    }

    /// Applies file-based custom dictionaries to an already loaded dictionary.
    ///
    /// This is the file-I/O counterpart to [`with_custom_dicts`](Self::with_custom_dicts)
    /// and is useful after loading built-in data with constructors such as
    /// [`from_zstd`](Self::from_zstd), or after loading external serialized
    /// dictionaries with [`deserialize_from_cbor`](Self::deserialize_from_cbor)
    /// or [`deserialize_from_json`](Self::deserialize_from_json). Files in each
    /// [`CustomDictFileSpec`] are read in order, then applied to the selected
    /// [`DictSlot`]: append mode merges pairs with last-wins semantics, while
    /// override mode clears the slot first.
    ///
    /// The affected slot metadata and starter indexes are rebuilt during
    /// customization. The returned dictionary is ready for
    /// [`OpenCC::from_dictionary`](crate::OpenCC::from_dictionary), and
    /// conversion hot paths remain immutable after construction.
    pub fn with_custom_dict_files<P>(
        mut self,
        specs: &[CustomDictFileSpec<P>],
    ) -> Result<Self, DictionaryError>
    where
        P: AsRef<Path>,
    {
        for spec in specs {
            let mut pairs = Vec::new();

            for file in &spec.files {
                pairs.extend(Self::load_pairs_from_path(file)?);
            }

            let slot = self.slot_mut(spec.slot);
            Self::apply_custom_pairs_to_slot(slot, &pairs, spec.mode);
        }

        Ok(self)
    }
}

impl Default for DictionaryMaxlength {
    /// Creates an empty `DictionaryMaxlength` with all dictionaries initialized
    /// to `DictMaxLen::default()`.
    ///
    /// This is primarily used as a fallback when dictionary loading fails, or
    /// for testing and placeholder scenarios where real dictionary data is not needed.
    ///
    /// Most users should prefer `DictionaryMaxlength::new()` or `from_zstd()` to load
    /// real data. This implementation ensures structural completeness but contains no mappings.
    fn default() -> Self {
        let dicts = Self {
            st_characters: DictMaxLen::default(),
            st_phrases: DictMaxLen::default(),
            ts_characters: DictMaxLen::default(),
            ts_phrases: DictMaxLen::default(),
            tw_phrases: DictMaxLen::default(),
            tw_phrases_rev: DictMaxLen::default(),
            tw_variants: DictMaxLen::default(),
            tw_variants_rev: DictMaxLen::default(),
            tw_variants_rev_phrases: DictMaxLen::default(),
            hk_variants: DictMaxLen::default(),
            hk_variants_rev: DictMaxLen::default(),
            hk_variants_rev_phrases: DictMaxLen::default(),
            jps_characters: DictMaxLen::default(),
            jps_phrases: DictMaxLen::default(),
            jp_variants: DictMaxLen::default(),
            jp_variants_rev: DictMaxLen::default(),
            st_punctuations: DictMaxLen::default(),
            ts_punctuations: DictMaxLen::default(),
            // runtime-only cache (serde-skipped)
            unions: Default::default(),
        };

        dicts.finish()
    }
}

/// Error type for dictionary loading, parsing, and serialization.
///
/// `DictionaryError` is used throughout the `dictionary_lib` module to wrap
/// low-level I/O failures, CBOR (de)serialization errors, and plaintext
/// dictionary format issues. It provides a single, ergonomic error type that
/// integrates with standard Rust error handling (`?`, `std::error::Error`).
///
/// # Variants
///
/// - [`DictionaryError::IoError`]
///   - Wraps a low-level [`io::Error`] that occurred during file access,
///     reading, or writing.
///
/// - [`DictionaryError::CborParseError`]
///   - Wraps a [`serde_cbor::Error`] that occurred while serializing or
///     deserializing CBOR dictionary data.
///
/// - [`DictionaryError::LoadFileError`]
///   - Reports a logical or format error while parsing a plaintext dictionary
///     file line-by-line (for example, a missing TAB separator). Carries the
///     file path, the 1-based line number, and a short human-readable message.
///
/// # Usage
///
/// This error type is returned by methods such as:
///
/// - [`DictionaryMaxlength::from_zstd()`]
/// - [`DictionaryMaxlength::load_cbor_compressed()`]
/// - [`DictionaryMaxlength::from_dicts()`]
///
/// It also implements [`From`] for [`io::Error`] and [`serde_cbor::Error`],
/// allowing you to use the `?` operator directly on I/O and CBOR operations.
#[derive(Debug)]
pub enum DictionaryError {
    /// Low-level I/O error (missing file, no permission, etc.).
    IoError(io::Error),

    /// CBOR serialization or deserialization failure.
    CborParseError(serde_cbor::Error),

    /// Text dictionary (.txt) format error while loading or parsing a file line-by-line.
    LoadFileError {
        /// Path of the dictionary file where the error occurred.
        path: String,
        /// 1-based line number in the file.
        lineno: usize,
        /// Short human-readable description of the issue.
        message: String,
    },
}

impl std::fmt::Display for DictionaryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DictionaryError::IoError(e) => write!(f, "I/O error: {}", e),
            DictionaryError::CborParseError(e) => write!(f, "Failed to parse CBOR: {}", e),
            DictionaryError::LoadFileError {
                path,
                lineno,
                message,
            } => {
                write!(f, "Error in {} at line {}: {}", path, lineno, message)
            }
        }
    }
}

impl Error for DictionaryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            DictionaryError::IoError(e) => Some(e),
            DictionaryError::CborParseError(e) => Some(e),
            DictionaryError::LoadFileError { .. } => None,
        }
    }
}

// Automatic conversions for ergonomic `?` usage.
impl From<io::Error> for DictionaryError {
    fn from(err: io::Error) -> Self {
        DictionaryError::IoError(err)
    }
}

impl From<serde_cbor::Error> for DictionaryError {
    fn from(err: serde_cbor::Error) -> Self {
        DictionaryError::CborParseError(err)
    }
}

// Custom Dictionary

/// Identifies a dictionary slot inside [`DictionaryMaxlength`].
///
/// Each slot corresponds to a specific OpenCC conversion dictionary.
/// Custom dictionaries can be appended to or override entries inside
/// these slots when using:
///
/// - [`DictionaryMaxlength::from_dicts_custom()`]
/// - [`DictionaryMaxlength::from_dicts_custom_files()`]
///
/// # Notes
///
/// OpenCC conversion behavior depends heavily on choosing the correct slot.
/// For example:
///
/// - [`DictSlot::STPhrases`] affects Simplified → Traditional phrase conversion.
/// - [`DictSlot::TSPhrases`] affects Traditional → Simplified phrase conversion.
/// - [`DictSlot::TWVariants`] affects Taiwan regional variants.
/// - [`DictSlot::HKVariants`] affects Hong Kong regional variants.
///
/// # See Also
///
/// - [`CustomDictSpec`]
/// - [`CustomDictFileSpec`]
/// - [`CustomDictMode`]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DictSlot {
    /// Simplified → Traditional character mappings.
    STCharacters,

    /// Simplified → Traditional phrase mappings.
    STPhrases,

    /// Traditional → Simplified character mappings.
    TSCharacters,

    /// Traditional → Simplified phrase mappings.
    TSPhrases,

    /// Traditional → Taiwan phrase mappings.
    TWPhrases,

    /// Taiwan → Traditional reverse phrase mappings.
    TWPhrasesRev,

    /// Traditional → Taiwan regional variant mappings.
    TWVariants,

    /// Taiwan → Traditional reverse variant mappings.
    TWVariantsRev,

    /// Taiwan → Traditional reverse phrase variant mappings.
    TWVariantsRevPhrases,

    /// Traditional → Hong Kong regional variant mappings.
    HKVariants,

    /// Hong Kong → Traditional reverse variant mappings.
    HKVariantsRev,

    /// Hong Kong → Traditional reverse phrase variant mappings.
    HKVariantsRevPhrases,

    /// Japanese Shinjitai character mappings.
    JPShinjitaiCharacters,

    /// Japanese Shinjitai phrase mappings.
    JPShinjitaiPhrases,

    /// Traditional → Japanese variant mappings.
    JPVariants,

    /// Japanese → Traditional reverse variant mappings.
    JPVariantsRev,

    /// Simplified → Traditional punctuation mappings.
    STPunctuations,

    /// Traditional → Simplified punctuation mappings.
    TSPunctuations,
}

/// Controls how custom dictionary entries are merged into a slot.
///
/// Used by [`CustomDictSpec`] and [`CustomDictFileSpec`].
///
/// # Modes
///
/// In post-load customization APIs, [`CustomDictMode::Append`] merges custom
/// entries into the selected slot with last-wins semantics, while
/// [`CustomDictMode::Override`] clears the selected slot first.
///
/// # Example
///
/// ```rust
/// use opencc_fmmseg::{
///     CustomDictMode,
///     CustomDictSpec,
///     DictSlot,
///     DictionaryMaxlength,
/// };
///
/// let dictionary = DictionaryMaxlength::from_dicts_custom(&[
///     CustomDictSpec {
///         slot: DictSlot::STPhrases,
///         pairs: vec![
///             ("帕兰蒂尔".to_string(), "柏蘭蒂爾".to_string()),
///         ],
///         mode: CustomDictMode::Override,
///     }
/// ]).unwrap();
///
/// assert!(dictionary.st_phrases.max_len > 0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CustomDictMode {
    /// Merge custom entries into the selected slot.
    ///
    /// Post-load APIs use last-wins semantics for conflicting keys.
    Append,

    /// Prefer custom entries over the selected slot's existing contents.
    ///
    /// Post-load APIs clear the slot before custom pairs are inserted.
    Override,
}

/// Pair-based custom dictionary injection.
///
/// This is the core no-I/O path for custom dictionaries and is suitable for:
///
/// - `include_str!()`
/// - dynamically generated dictionaries
/// - database-loaded pairs
/// - testing
/// - embedded applications
/// - WebAssembly environments
///
/// # Notes
///
/// Custom pairs are merged into the selected [`DictSlot`] before
/// internal starter indexes and maximum phrase lengths are rebuilt.
///
/// # Example
///
/// ```rust
/// use opencc_fmmseg::{
///     CustomDictMode,
///     CustomDictSpec,
///     DictSlot,
///     DictionaryMaxlength,
/// };
///
/// let dictionary = DictionaryMaxlength::from_dicts_custom(&[
///     CustomDictSpec {
///         slot: DictSlot::STPhrases,
///         pairs: vec![
///             ("帕兰蒂尔".to_string(), "柏蘭蒂爾".to_string()),
///         ],
///         mode: CustomDictMode::Override,
///     }
/// ]).unwrap();
///
/// assert!(dictionary.st_phrases.max_len > 0);
/// ```
#[derive(Debug, Clone)]
pub struct CustomDictSpec {
    /// Target dictionary slot.
    pub slot: DictSlot,

    /// Key-value dictionary pairs.
    ///
    /// Each pair follows standard OpenCC semantics:
    ///
    /// `(source, target)`
    pub pairs: Vec<(String, String)>,

    /// Merge behavior for conflicting keys.
    pub mode: CustomDictMode,
}

/// File-based custom dictionary injection.
///
/// This is a convenience wrapper for loading one or more OpenCC-style
/// dictionary text files for a slot.
///
/// Each file should follow the standard OpenCC dictionary format:
///
/// ```text
/// source<TAB>target
/// ```
///
/// # Notes
///
/// Multiple files are loaded in order.
///
/// # Example
///
/// ```rust, no_run
/// use opencc_fmmseg::{
///     CustomDictFileSpec,
///     CustomDictMode,
///     DictSlot,
///     DictionaryMaxlength,
/// };
///
/// let dictionary = DictionaryMaxlength::from_dicts_custom_files(&[
///     CustomDictFileSpec {
///         slot: DictSlot::STPhrases,
///         files: vec!["./my_terms.txt"],
///         mode: CustomDictMode::Override,
///     }
/// ]).unwrap();
///
/// assert!(dictionary.st_phrases.max_len > 0);
/// ```
#[derive(Debug, Clone)]
pub struct CustomDictFileSpec<P> {
    /// Target dictionary slot.
    pub slot: DictSlot,

    /// OpenCC-style dictionary text files.
    ///
    /// Files are loaded sequentially in the provided order.
    pub files: Vec<P>,

    /// Merge behavior for conflicting keys.
    pub mode: CustomDictMode,
}

/// Returns custom dictionary specs targeting the given slot.
fn specs_for_slot(
    specs: &[CustomDictSpec],
    slot: DictSlot,
) -> impl Iterator<Item = &CustomDictSpec> {
    specs.iter().filter(move |spec| spec.slot == slot)
}

/// Applies custom dictionary specs to raw dictionary pairs before building `DictMaxLen`.
///
/// Custom entries are merged at pair level so that `DictMaxLen::build_from_pairs()`
/// can rebuild max lengths and starter indexes from the final merged dictionary.
fn apply_custom_pairs(
    mut base_pairs: Vec<(String, String)>,
    specs: &[CustomDictSpec],
    slot: DictSlot,
) -> Vec<(String, String)> {
    for spec in specs_for_slot(specs, slot) {
        match spec.mode {
            CustomDictMode::Append => {
                let mut existing: FxHashSet<String> =
                    base_pairs.iter().map(|(k, _)| k.clone()).collect();

                for (k, v) in &spec.pairs {
                    if existing.insert(k.clone()) {
                        base_pairs.push((k.clone(), v.clone()));
                    }
                }
            }
            CustomDictMode::Override => {
                let mut map: FxHashMap<String, String> = base_pairs.into_iter().collect();

                for (k, v) in &spec.pairs {
                    map.insert(k.clone(), v.clone());
                }

                base_pairs = map.into_iter().collect();
            }
        }
    }

    base_pairs
}

// ------ Tests ------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dictionary_lib::dict_max_len::DictMaxLen;

    #[test]
    #[ignore]
    fn test_dictionary_from_dicts_then_to_cbor() {
        // Assuming you have a method `from_dicts` to create a dictionary
        let dictionary = DictionaryMaxlength::from_dicts().unwrap();
        // Verify that the Dictionary contains the expected data
        let expected = 14;
        assert_eq!(dictionary.st_phrases.max_len, expected);

        let filename = "dictionary_maxlength.cbor";
        dictionary.serialize_to_cbor(filename).unwrap();
        let file_contents = fs::read(filename).unwrap();
        let expected_cbor_size = 1359720; // Update this with the actual expected size
        assert_eq!(file_contents.len(), expected_cbor_size);
        // Clean up: Delete the test file
        fs::remove_file(filename).unwrap();
    }

    #[test]
    #[ignore]
    fn test_dictionary_from_dicts_then_to_zstd() {
        use std::fs;
        use std::io::Write;
        use zstd::stream::Encoder;

        // Create dictionary
        let dictionary = DictionaryMaxlength::from_dicts().unwrap();

        // Serialize to CBOR
        let cbor_filename = "dictionary_maxlength.cbor";
        dictionary.serialize_to_cbor(cbor_filename).unwrap();

        // Read the CBOR file
        let cbor_data = fs::read(cbor_filename).unwrap();

        // Compress with Zstd
        let zstd_filename = "dictionary_maxlength.zstd";
        let zstd_file = File::create(zstd_filename).expect("Failed to create zstd file");
        let mut encoder = Encoder::new(&zstd_file, 19).expect("Failed to create zstd encoder");
        encoder
            .write_all(&cbor_data)
            .expect("Failed to write compressed data");
        encoder.finish().expect("Failed to finish compression");

        // Verify file size within a reasonable range
        let compressed_size = fs::metadata(zstd_filename).unwrap().len();
        let min_size = 480000; // Lower bound
        let max_size = 600000; // Upper bound
        assert!(
            compressed_size >= min_size && compressed_size <= max_size,
            "Unexpected compressed size: {}",
            compressed_size
        );

        // Clean up: Remove test files
        fs::remove_file(cbor_filename).unwrap();
        fs::remove_file(zstd_filename).unwrap();
    }

    #[test]
    fn test_dictionary_from_zstd() {
        let dictionary =
            DictionaryMaxlength::from_zstd().expect("Failed to load dictionary from zstd");

        // Verify a known field
        let expected = 14;
        assert_eq!(dictionary.st_phrases.max_len, expected);
    }

    #[test]
    #[ignore]
    fn test_save_compressed() {
        use crate::dictionary_lib::dictionary_maxlength::DictionaryMaxlength;
        use std::fs;

        let dictionary = DictionaryMaxlength::from_dicts().expect("Failed to create dictionary");

        let compressed_file = "test_dictionary.zstd";

        // Attempt to save the dictionary in compressed form
        let result = DictionaryMaxlength::save_cbor_compressed(&dictionary, compressed_file);
        assert!(
            result.is_ok(),
            "Failed to save compressed dictionary: {:?}",
            result
        );

        // Ensure the compressed file exists and is non-empty
        let metadata = fs::metadata(compressed_file).expect("Failed to get file metadata");
        assert!(metadata.len() > 0, "Compressed file should not be empty");

        // Clean up after test
        fs::remove_file(compressed_file).expect("Failed to remove test file");
    }

    #[test]
    #[ignore]
    fn test_save_and_load_compressed() {
        use crate::dictionary_lib::dictionary_maxlength::DictionaryMaxlength;
        use std::fs;

        let dictionary = DictionaryMaxlength::from_dicts().expect("Failed to create dictionary");

        let compressed_file = "test2_dictionary.zstd";

        // Save the dictionary in compressed form
        let save_result = DictionaryMaxlength::save_cbor_compressed(&dictionary, compressed_file);
        assert!(
            save_result.is_ok(),
            "Failed to save compressed dictionary: {:?}",
            save_result
        );

        // Load the dictionary from the compressed file
        let load_result = DictionaryMaxlength::load_cbor_compressed(compressed_file);
        assert!(
            load_result.is_ok(),
            "Failed to load compressed dictionary: {:?}",
            load_result
        );

        let loaded_dictionary = load_result.unwrap();

        // Verify the loaded dictionary is equivalent to the original
        assert_eq!(
            dictionary.st_phrases.max_len, loaded_dictionary.st_phrases.max_len,
            "Loaded dictionary does not match the original"
        );

        // Clean up: Remove the test file
        fs::remove_file(compressed_file).expect("Failed to remove test file");
    }

    #[ignore]
    #[test]
    fn test_to_dicts_writes_expected_txt_files() -> Result<(), Box<dyn Error>> {
        let output_dir = "test_output_dicts";

        // Clean output_dir if exists from previous runs
        if Path::new(output_dir).exists() {
            fs::remove_dir_all(output_dir)?;
        }

        // Build DictMaxLen from (String, String) pairs
        let pairs = vec![
            ("测试".to_string(), "測試".to_string()),
            ("语言".to_string(), "語言".to_string()),
        ];

        let st_chars: DictMaxLen = DictMaxLen::build_from_pairs(pairs.clone());
        let st_phrases: DictMaxLen = DictMaxLen::build_from_pairs(pairs.clone());

        let dicts = DictionaryMaxlength {
            st_characters: st_chars,
            st_phrases,
            ts_characters: DictMaxLen::default(),
            ts_phrases: DictMaxLen::default(),
            tw_phrases: DictMaxLen::default(),
            tw_phrases_rev: DictMaxLen::default(),
            tw_variants: DictMaxLen::default(),
            tw_variants_rev: DictMaxLen::default(),
            tw_variants_rev_phrases: DictMaxLen::default(),
            hk_variants: DictMaxLen::default(),
            hk_variants_rev: DictMaxLen::default(),
            hk_variants_rev_phrases: DictMaxLen::default(),
            jps_characters: DictMaxLen::default(),
            jps_phrases: DictMaxLen::default(),
            jp_variants: DictMaxLen::default(),
            jp_variants_rev: DictMaxLen::default(),
            st_punctuations: DictMaxLen::default(),
            ts_punctuations: DictMaxLen::default(),
            // runtime-only cache (serde-skipped)
            unions: Default::default(),
        };

        dicts.to_dicts(output_dir)?;

        // Check a few output files
        let stc_path = format!("{}/STCharacters.txt", output_dir);
        let stp_path = format!("{}/STPhrases.txt", output_dir);

        let content_stc = fs::read_to_string(&stc_path)?;
        let content_stp = fs::read_to_string(&stp_path)?;

        assert!(content_stc.contains("测试\t測試"));
        assert!(content_stc.contains("语言\t語言"));
        assert!(content_stp.contains("测试\t測試"));
        assert!(content_stp.contains("语言\t語言"));

        // Cleanup
        fs::remove_dir_all(output_dir)?;

        Ok(())
    }

    // Custom Dictionary Tests

    #[test]
    fn test_from_dicts_custom_append_st_phrases_palantir() {
        let dictionary = DictionaryMaxlength::from_dicts_custom(&[CustomDictSpec {
            slot: DictSlot::STPhrases,
            pairs: vec![("帕兰蒂尔".to_string(), "柏蘭蒂爾".to_string())],
            mode: CustomDictMode::Append,
        }])
        .expect("Failed to create custom dictionary");

        assert_eq!(
            dictionary
                .st_phrases
                .map
                .get("帕兰蒂尔".chars().collect::<Vec<_>>().as_slice()),
            Some(&"柏蘭蒂爾".into())
        );
    }

    #[test]
    fn test_from_dicts_custom_override_st_phrases_ai_company() {
        let dictionary = DictionaryMaxlength::from_dicts_custom(&[CustomDictSpec {
            slot: DictSlot::STPhrases,
            pairs: vec![("人工智能公司".to_string(), "AI公司".to_string())],
            mode: CustomDictMode::Override,
        }])
        .expect("Failed to create custom dictionary");

        assert_eq!(
            dictionary
                .st_phrases
                .map
                .get("人工智能公司".chars().collect::<Vec<_>>().as_slice()),
            Some(&"AI公司".into())
        );
    }

    #[test]
    fn test_from_dicts_custom_multiple_slots() {
        let dictionary = DictionaryMaxlength::from_dicts_custom(&[
            CustomDictSpec {
                slot: DictSlot::STPhrases,
                pairs: vec![("帕兰蒂尔".to_string(), "柏蘭蒂爾".to_string())],
                mode: CustomDictMode::Append,
            },
            CustomDictSpec {
                slot: DictSlot::TSPhrases,
                pairs: vec![("柏蘭蒂爾".to_string(), "帕兰蒂尔".to_string())],
                mode: CustomDictMode::Append,
            },
        ])
        .expect("Failed to create custom dictionary");

        assert_eq!(
            dictionary
                .st_phrases
                .map
                .get("帕兰蒂尔".chars().collect::<Vec<_>>().as_slice()),
            Some(&"柏蘭蒂爾".into())
        );
        assert_eq!(
            dictionary
                .ts_phrases
                .map
                .get("柏蘭蒂爾".chars().collect::<Vec<_>>().as_slice()),
            Some(&"帕兰蒂尔".into())
        );
    }

    #[test]
    fn test_from_dicts_custom_files_st_phrases_palantir() {
        use std::fs;

        let dir = std::env::temp_dir();
        let file_path = dir.join("opencc_fmmseg_custom_st_phrases_test.txt");

        fs::write(&file_path, "帕兰蒂尔\t柏蘭蒂爾\n").expect("Failed to write custom dict file");

        let dictionary = DictionaryMaxlength::from_dicts_custom_files(&[CustomDictFileSpec {
            slot: DictSlot::STPhrases,
            files: vec![file_path.clone()],
            mode: CustomDictMode::Override,
        }])
        .expect("Failed to create custom dictionary from files");

        let opencc = crate::OpenCC::from_dictionary(dictionary);

        assert_eq!(
            opencc.convert("帕兰蒂尔是一家人工智能公司", "s2t", false),
            "柏蘭蒂爾是一家人工智能公司"
        );

        let _ = fs::remove_file(file_path);
    }

    // New: Dynamically update pairs tests

    #[test]
    fn test_with_custom_dicts_append_st_phrases_palantir() {
        let dictionary = DictionaryMaxlength::from_zstd()
            .expect("Failed to load default dictionary")
            .with_custom_dicts(&[CustomDictSpec {
                slot: DictSlot::STPhrases,
                pairs: vec![("帕兰蒂尔".to_string(), "柏蘭蒂爾".to_string())],
                mode: CustomDictMode::Append,
            }])
            .expect("Failed to apply custom dictionary");

        let opencc = crate::OpenCC::from_dictionary(dictionary);

        assert_eq!(
            opencc.convert("帕兰蒂尔是一家人工智能公司", "s2t", false),
            "柏蘭蒂爾是一家人工智能公司"
        );
    }

    #[test]
    fn test_with_custom_dicts_override_st_phrases_only_custom_pairs_remain() {
        let dictionary = DictionaryMaxlength::from_zstd()
            .expect("Failed to load default dictionary")
            .with_custom_dicts(&[CustomDictSpec {
                slot: DictSlot::STPhrases,
                pairs: vec![("人工智能公司".to_string(), "AI公司".to_string())],
                mode: CustomDictMode::Override,
            }])
            .expect("Failed to apply custom dictionary");

        assert_eq!(
            dictionary
                .st_phrases
                .map
                .get("人工智能公司".chars().collect::<Vec<_>>().as_slice()),
            Some(&"AI公司".into())
        );

        assert_eq!(dictionary.st_phrases.map.len(), 1);
    }

    #[test]
    fn test_with_custom_dicts_multiple_slots() {
        let dictionary = DictionaryMaxlength::from_zstd()
            .expect("Failed to load default dictionary")
            .with_custom_dicts(&[
                CustomDictSpec {
                    slot: DictSlot::STPhrases,
                    pairs: vec![
                        ("帕兰蒂尔".to_string(), "柏蘭蒂爾".to_string()),
                        ("人工智能公司".to_string(), "AI公司".to_string()),
                    ],
                    mode: CustomDictMode::Append,
                },
                CustomDictSpec {
                    slot: DictSlot::TSPhrases,
                    pairs: vec![
                        ("柏蘭蒂爾".to_string(), "帕兰蒂尔".to_string()),
                        ("AI公司".to_string(), "人工智能公司".to_string()),
                    ],
                    mode: CustomDictMode::Append,
                },
            ])
            .expect("Failed to apply custom dictionaries");

        assert_eq!(
            dictionary
                .st_phrases
                .map
                .get("帕兰蒂尔".chars().collect::<Vec<_>>().as_slice()),
            Some(&"柏蘭蒂爾".into())
        );

        assert_eq!(
            dictionary
                .ts_phrases
                .map
                .get("柏蘭蒂爾".chars().collect::<Vec<_>>().as_slice()),
            Some(&"帕兰蒂尔".into())
        );
    }

    #[test]
    fn test_with_custom_dict_files_multiple_files_later_wins() {
        use std::fs;

        let dir = std::env::temp_dir();
        let file1 = dir.join("opencc_fmmseg_custom_file_1.txt");
        let file2 = dir.join("opencc_fmmseg_custom_file_2.txt");

        fs::write(&file1, "帕兰蒂尔\t帕蘭蒂爾\n").expect("Failed to write custom dict file 1");
        fs::write(&file2, "帕兰蒂尔\t柏蘭蒂爾\n").expect("Failed to write custom dict file 2");

        let dictionary = DictionaryMaxlength::from_zstd()
            .expect("Failed to load default dictionary")
            .with_custom_dict_files(&[CustomDictFileSpec {
                slot: DictSlot::STPhrases,
                files: vec![file1.clone(), file2.clone()],
                mode: CustomDictMode::Append,
            }])
            .expect("Failed to apply custom dictionary files");

        let opencc = crate::OpenCC::from_dictionary(dictionary);

        assert_eq!(
            opencc.convert("帕兰蒂尔是一家人工智能公司", "s2t", false),
            "柏蘭蒂爾是一家人工智能公司"
        );

        let _ = fs::remove_file(file1);
        let _ = fs::remove_file(file2);
    }
}
