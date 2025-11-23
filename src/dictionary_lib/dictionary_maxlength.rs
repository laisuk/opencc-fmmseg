//! Internal module for managing and loading OpenCC dictionaries.
//!
//! This module defines the [`DictionaryMaxlength`] struct, which stores all necessary
//! dictionaries and associated metadata used by the OpenCC text conversion engine.
//! Each dictionary is paired with a maximum word length for efficient forward maximum
//! matching (FMM) during segment-based replacement.
//!
//! Users generally interact with this indirectly via the `OpenCC` interface, but
//! advanced users may access it for custom loading, serialization, or optimization.

use crate::dictionary_lib::dict_max_len::DictMaxLen;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use serde_cbor::{from_reader, from_slice};
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, BufWriter, Cursor, Write};
use std::path::Path;
use std::sync::Mutex;
use std::{fs, io};
use zstd::{decode_all, Decoder, Encoder};

mod union_cache;
pub(crate) use union_cache::UnionKey;
// so callers can say `UnionKey::S2T { punct: .. }`

// Define a global mutable variable to store the error message
static LAST_ERROR: Mutex<Option<String>> = Mutex::new(None);

/// Represents a collection of OpenCC dictionaries paired with their maximum word lengths.
///
/// This structure is used internally by the `OpenCC` engine to support fast, segment-based
/// forward maximum matching (FMM) for Chinese text conversion. Each dictionary maps a phrase
/// or character to its target form and tracks the longest entry for lookup performance.
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

        // Decompress Zstd
        let decompressed_data =
            decode_all(Cursor::new(compressed_data)).map_err(DictionaryError::IoError)?;

        // Deserialize CBOR
        let dictionary: DictionaryMaxlength =
            from_slice(&decompressed_data).map_err(DictionaryError::CborParseError)?;

        Ok(dictionary.finish())
    }

    /// Loads the dictionary from an embedded CBOR file.
    ///
    /// This constructor initializes a [`DictionaryMaxlength`] instance using
    /// a pre-generated CBOR-encoded dictionary blob bundled directly into the
    /// crate binary via `include_bytes!`.
    ///
    /// The CBOR file contains the fully preprocessed dictionary structure
    /// (phrase maps, character maps, max-length metadata), making this loading
    /// path:
    ///
    /// - Fast
    /// - Deterministic
    /// - Independent of external files
    ///
    /// On parse failure, this method stores a descriptive message in the global
    /// “last error” buffer using [`set_last_error`](Self::set_last_error), which
    /// allows foreign FFI bindings (C, C#, Python, Java/JNI, etc.) to retrieve
    /// the error reason safely.
    ///
    /// # Returns
    ///
    /// - `Ok(Self)` if the embedded CBOR is successfully decoded
    /// - `Err(DictionaryError::CborParseError)` if deserialization fails
    ///
    /// # Notes
    ///
    /// After deserialization, the resulting struct is passed through
    /// [`finish`](Self::finish), which finalizes internal metadata such as
    /// maximum key lengths.
    pub fn from_cbor() -> Result<Self, DictionaryError> {
        let cbor_bytes = include_bytes!("dicts/dictionary_maxlength.cbor");

        let dictionary: DictionaryMaxlength = from_slice(cbor_bytes).map_err(|err| {
            // C API / FFI-friendly string:
            Self::set_last_error(&format!("Failed to parse embedded CBOR: {}", err));
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
        let base_dir = "dicts";

        // upfront check for base_dir existence
        if !Path::new(base_dir).exists() {
            let msg = format!("Base directory not found: {}", base_dir);
            Self::set_last_error(&msg);
            // Wrap as real io::Error
            return Err(DictionaryError::IoError(io::Error::new(
                io::ErrorKind::NotFound,
                msg,
            )));
        }

        let dict_files: HashMap<&str, &str> = [
            ("st_characters", "STCharacters.txt"),
            ("st_phrases", "STPhrases.txt"),
            ("ts_characters", "TSCharacters.txt"),
            ("ts_phrases", "TSPhrases.txt"),
            ("tw_phrases", "TWPhrases.txt"),
            ("tw_phrases_rev", "TWPhrasesRev.txt"),
            ("tw_variants", "TWVariants.txt"),
            ("tw_variants_rev", "TWVariantsRev.txt"),
            ("tw_variants_rev_phrases", "TWVariantsRevPhrases.txt"),
            ("hk_variants", "HKVariants.txt"),
            ("hk_variants_rev", "HKVariantsRev.txt"),
            ("hk_variants_rev_phrases", "HKVariantsRevPhrases.txt"),
            ("jps_characters", "JPShinjitaiCharacters.txt"),
            ("jps_phrases", "JPShinjitaiPhrases.txt"),
            ("jp_variants", "JPVariants.txt"),
            ("jp_variants_rev", "JPVariantsRev.txt"),
            ("st_punctuations", "STPunctuations.txt"),
            ("ts_punctuations", "TSPunctuations.txt"),
        ]
        .into_iter()
        .collect();

        fn load_dict(base_dir: &str, filename: &str) -> Result<DictMaxLen, DictionaryError> {
            let path = Path::new(base_dir).join(filename);
            let path_str = path.display().to_string();

            // Pure I/O errors → IoError(io::Error)
            let content = fs::read_to_string(&path)?;

            let mut pairs: Vec<(String, String)> = Vec::new();
            let mut saw_data_line = false;

            for (lineno, raw_line) in content.lines().enumerate() {
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
                        path: path_str.clone(), // cloned only on error
                        lineno: lineno + 1,     // human-friendly 1-based line
                        message: "missing TAB separator".to_string(),
                    });
                };

                let first_value = v.split_whitespace().next().unwrap_or("");
                pairs.push((k.to_owned(), first_value.to_owned()));
            }

            Ok(DictMaxLen::build_from_pairs(pairs))
        }

        Ok(DictionaryMaxlength {
            st_characters: load_dict(base_dir, dict_files["st_characters"])?,
            st_phrases: load_dict(base_dir, dict_files["st_phrases"])?,
            ts_characters: load_dict(base_dir, dict_files["ts_characters"])?,
            ts_phrases: load_dict(base_dir, dict_files["ts_phrases"])?,
            tw_phrases: load_dict(base_dir, dict_files["tw_phrases"])?,
            tw_phrases_rev: load_dict(base_dir, dict_files["tw_phrases_rev"])?,
            tw_variants: load_dict(base_dir, dict_files["tw_variants"])?,
            tw_variants_rev: load_dict(base_dir, dict_files["tw_variants_rev"])?,
            tw_variants_rev_phrases: load_dict(base_dir, dict_files["tw_variants_rev_phrases"])?,
            hk_variants: load_dict(base_dir, dict_files["hk_variants"])?,
            hk_variants_rev: load_dict(base_dir, dict_files["hk_variants_rev"])?,
            hk_variants_rev_phrases: load_dict(base_dir, dict_files["hk_variants_rev_phrases"])?,
            jps_characters: load_dict(base_dir, dict_files["jps_characters"])?,
            jps_phrases: load_dict(base_dir, dict_files["jps_phrases"])?,
            jp_variants: load_dict(base_dir, dict_files["jp_variants"])?,
            jp_variants_rev: load_dict(base_dir, dict_files["jp_variants_rev"])?,
            st_punctuations: load_dict(base_dir, dict_files["st_punctuations"])?,
            ts_punctuations: load_dict(base_dir, dict_files["ts_punctuations"])?,
            // runtime-only cache (serde-skipped)
            unions: Default::default(),
        })
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
        let dict_map: HashMap<&str, &FxHashMap<Box<[char]>, Box<str>>> = [
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

    /// Serializes the entire dictionary structure to a CBOR file.
    ///
    /// This function converts the current [`DictionaryMaxlength`] instance into
    /// a compact CBOR binary representation using `serde_cbor`, and writes the
    /// resulting bytes to the specified filesystem path.
    ///
    /// The output CBOR file contains:
    ///
    /// - All phrase and character dictionaries
    /// - All variant and reverse-variant tables
    /// - Precomputed max-length metadata
    ///
    /// This format is suitable for distributing custom dictionary builds or
    /// regenerating the embedded dictionary used by [`from_cbor`](Self::from_cbor).
    ///
    /// On serialization or I/O failure, this method records a human-readable
    /// error message in the global last-error buffer via
    /// [`set_last_error`](Self::set_last_error), which is used by C, C#, Python,
    /// and Java bindings for cross-language diagnostics.
    ///
    /// # Arguments
    ///
    /// * `path` — Destination file path for the generated CBOR file.
    ///
    /// # Returns
    ///
    /// - `Ok(())` if serialization and writing succeed
    /// - `Err(DictionaryError)` if an encoding or I/O error occurs
    ///
    /// # Notes
    ///
    /// This function serializes the dictionary *before* calling
    /// [`finish`](Self::finish), meaning it includes all precomputed metadata
    /// already present in the internal structure.
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
    /// This function reads a CBOR-encoded [`DictionaryMaxlength`] from the given
    /// path, reconstructs the full dictionary structure using `serde_cbor`, and
    /// then finalizes internal metadata via [`finish`](Self::finish).
    ///
    /// The CBOR file is expected to have been produced by
    /// [`serialize_to_cbor`](Self::serialize_to_cbor) or an equivalent process
    /// using the same schema. It contains:
    ///
    /// - All phrase and character dictionaries
    /// - Variant and reverse-variant tables
    /// - Associated metadata required for longest-match segmentation
    ///
    /// On I/O or parse failure, this method stores a human-readable error
    /// message in the global last-error buffer using
    /// [`set_last_error`](Self::set_last_error), which is used by FFI bindings
    /// to surface diagnostics to other languages.
    ///
    /// # Arguments
    ///
    /// * `path` — Source file path of the CBOR dictionary to load.
    ///
    /// # Returns
    ///
    /// - `Ok(Self)` if the file is successfully read and decoded
    /// - `Err(DictionaryError)` if reading or deserialization fails
    pub fn deserialize_from_cbor<P: AsRef<Path>>(path: P) -> Result<Self, DictionaryError> {
        let cbor_data = fs::read(&path).map_err(|err| {
            let msg = format!("Failed to read CBOR file: {}", err);
            Self::set_last_error(&msg);
            DictionaryError::IoError(err)
        })?;

        let dictionary: DictionaryMaxlength = from_slice(&cbor_data).map_err(|err| {
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
    /// * `path` — Path to a `.cbor.zst` or similar compressed dictionary file.
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
        let expected = 16;
        assert_eq!(dictionary.st_phrases.max_len, expected);

        let filename = "dictionary_maxlength.cbor";
        dictionary.serialize_to_cbor(filename).unwrap();
        let file_contents = fs::read(filename).unwrap();
        let expected_cbor_size = 1351835; // Update this with the actual expected size
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
        let expected = 16;
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
}
