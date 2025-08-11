//! Internal module for managing and loading OpenCC dictionaries.
//!
//! This module defines the [`DictionaryMaxlength`] struct, which stores all necessary
//! dictionaries and associated metadata used by the OpenCC text conversion engine.
//! Each dictionary is paired with a maximum word length for efficient forward maximum
//! matching (FMM) during segment-based replacement.
//!
//! Users generally interact with this indirectly via the `OpenCC` interface, but
//! advanced users may access it for custom loading, serialization, or optimization.

use rustc_hash::FxHashMap;
use serde::de::{self};
use serde::{ser::SerializeMap, Deserialize, Deserializer, Serialize, Serializer};
use serde_cbor::{from_reader, from_slice};
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, BufWriter, Cursor, Write};
use std::path::Path;
use std::sync::Mutex;
use std::{fs, io};
use zstd::{decode_all, Decoder, Encoder};

// Define a global mutable variable to store the error message
static LAST_ERROR: Mutex<Option<String>> = Mutex::new(None);

/// Represents a collection of OpenCC dictionaries paired with their maximum word lengths.
///
/// This structure is used internally by the `OpenCC` engine to support fast, segment-based
/// forward maximum matching (FMM) for Chinese text conversion. Each dictionary maps a phrase
/// or character to its target form and tracks the longest entry for lookup performance.
#[derive(Serialize, Deserialize, Debug)]
pub struct DictionaryMaxlength {
    pub st_characters: DictMaxLen,
    pub st_phrases: DictMaxLen,
    pub ts_characters: DictMaxLen,
    pub ts_phrases: DictMaxLen,
    pub tw_phrases: DictMaxLen,
    pub tw_phrases_rev: DictMaxLen,
    pub tw_variants: DictMaxLen,
    pub tw_variants_rev: DictMaxLen,
    pub tw_variants_rev_phrases: DictMaxLen,
    pub hk_variants: DictMaxLen,
    pub hk_variants_rev: DictMaxLen,
    pub hk_variants_rev_phrases: DictMaxLen,
    pub jps_characters: DictMaxLen,
    pub jps_phrases: DictMaxLen,
    pub jp_variants: DictMaxLen,
    pub jp_variants_rev: DictMaxLen,
    pub st_punctuations: DictMaxLen,
    pub ts_punctuations: DictMaxLen,
}

impl DictionaryMaxlength {
    /// Loads the default embedded Zstd-compressed dictionary.
    ///
    /// Recommended for normal usage, as it loads a precompiled binary blob built into the application.
    pub fn new() -> Result<Self, Box<dyn Error>> {
        Ok(Self::from_zstd().map_err(|err| {
            Self::set_last_error(&format!("Failed to load dictionary from Zstd: {}", err));
            Box::new(err)
        })?)
        // let dicts = Self::from_dicts().map_err(|err| {
        //     Self::set_last_error(&format!("Failed to load dictionary from dicts: {}", err));
        //     err
        // })?;
        // Ok(dicts)
    }

    /// Loads dictionary from an embedded Zstd-compressed CBOR blob.
    pub fn from_zstd() -> Result<Self, DictionaryError> {
        // Embedded compressed CBOR file at compile time
        let compressed_data = include_bytes!("dicts/dictionary_maxlength.zstd");

        // Decompress Zstd
        let decompressed_data = decode_all(Cursor::new(compressed_data)).map_err(|err| {
            DictionaryError::IoError(format!("Failed to decompress Zstd: {}", err))
        })?;

        // Deserialize CBOR
        let dictionary: DictionaryMaxlength = from_slice(&decompressed_data)
            .map_err(|err| DictionaryError::ParseError(format!("Failed to parse CBOR: {}", err)))?;

        Ok(dictionary.finish())
    }

    /// Loads dictionary from an embedded CBOR file.
    pub fn from_cbor() -> Result<Self, Box<dyn Error>> {
        let cbor_bytes = include_bytes!("dicts/dictionary_maxlength.cbor");
        match from_slice::<DictionaryMaxlength>(cbor_bytes) {
            Ok(mut dictionary) => {
                dictionary.populate_all();
                Ok(dictionary)
            }
            Err(err) => {
                Self::set_last_error(&format!("Failed to read CBOR file: {}", err));
                Err(Box::new(err))
            }
        }
    }

    /// Loads dictionary from plaintext `.txt` dictionary files.
    ///
    /// This method is used primarily for development and regeneration.
    pub fn from_dicts() -> Result<Self, DictionaryError> {
        let base_dir = "dicts";

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

        // Updated: build DictMaxLen directly
        fn load_dict(base_dir: &str, filename: &str) -> Result<DictMaxLen, DictionaryError> {
            let path = Path::new(base_dir).join(filename);
            let path_str = path.to_string_lossy();
            let content = fs::read_to_string(&path).map_err(|e| {
                DictionaryError::IoError(format!("Failed to read {}: {}", path_str, e))
            })?;

            let mut pairs: Vec<(String, String)> = Vec::new();
            let mut saw_data_line = false;

            for (lineno, raw_line) in content.lines().enumerate() {
                // `lines()` already strips trailing '\r' if present (CRLF safe)
                let mut line = raw_line.trim_end(); // keep left whitespace if needed in keys; trim right

                if line.is_empty() || line.starts_with('#') {
                    continue;
                }

                // Strip UTF-8 BOM only on the first non-empty, non-comment line
                if !saw_data_line {
                    if let Some(rest) = line.strip_prefix('\u{FEFF}') {
                        line = rest;
                    }
                    saw_data_line = true;
                }

                let Some((k, v)) = line.split_once('\t') else {
                    return Err(DictionaryError::ParseError(format!(
                        "Line {} in {} missing TAB separator",
                        lineno + 1,
                        path_str
                    )));
                };

                // OpenCC semantics: keep only the first whitespace-separated target
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
        })
    }

    /// Populate starter indexes for all inner DictMaxLen tables (BMP masks + per-starter caps).
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

    /// Convenient finisher to chain after deserialization/loading.
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

    // Saves all dictionaries to plaintext `.txt` files in the specified directory.
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

    /// Serializes the dictionary to a CBOR file.
    pub fn serialize_to_cbor<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn Error>> {
        match serde_cbor::to_vec(self) {
            Ok(cbor_data) => {
                if let Err(err) = fs::write(&path, cbor_data) {
                    Self::set_last_error(&format!("Failed to write CBOR file: {}", err));
                    return Err(Box::new(err));
                }
                Ok(())
            }
            Err(err) => {
                Self::set_last_error(&format!("Failed to serialize to CBOR: {}", err));
                Err(Box::new(err))
            }
        }
    }

    /// Deserializes the dictionary from a CBOR file.
    pub fn deserialize_from_cbor<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        match fs::read(&path) {
            Ok(cbor_data) => match from_slice::<DictionaryMaxlength>(&cbor_data) {
                Ok(mut dictionary) => {
                    dictionary.populate_all();
                    Ok(dictionary)
                }
                Err(err) => {
                    Self::set_last_error(&format!("Failed to deserialize CBOR: {}", err));
                    Err(Box::new(err))
                }
            },
            Err(err) => {
                Self::set_last_error(&format!("Failed to read CBOR file: {}", err));
                Err(Box::new(err))
            }
        }
    }

    /// Records the last error message encountered during dictionary operations.
    pub fn set_last_error(err_msg: &str) {
        let mut last_error = LAST_ERROR.lock().unwrap();
        *last_error = Some(err_msg.to_string());
    }

    /// Retrieves the last error message set during dictionary loading or saving.
    pub fn get_last_error() -> Option<String> {
        let last_error = LAST_ERROR.lock().unwrap();
        last_error.clone()
    }

    /// Saves the dictionary to a Zstd-compressed CBOR file on disk.
    pub fn save_compressed(
        dictionary: &DictionaryMaxlength,
        path: &str,
    ) -> Result<(), DictionaryError> {
        let file = File::create(path).map_err(|e| DictionaryError::IoError(e.to_string()))?;
        let writer = BufWriter::new(file);
        let mut encoder =
            Encoder::new(writer, 19).map_err(|e| DictionaryError::IoError(e.to_string()))?;
        serde_cbor::to_writer(&mut encoder, dictionary)
            .map_err(|e| DictionaryError::ParseError(e.to_string()))?;
        encoder
            .finish()
            .map_err(|e| DictionaryError::IoError(e.to_string()))?;
        Ok(())
    }

    /// Loads the dictionary from a Zstd-compressed CBOR file on disk.
    pub fn load_compressed(path: &str) -> Result<DictionaryMaxlength, DictionaryError> {
        let file = File::open(path).map_err(|e| DictionaryError::IoError(e.to_string()))?;
        let reader = BufReader::new(file);
        let mut decoder =
            Decoder::new(reader).map_err(|e| DictionaryError::IoError(e.to_string()))?;
        let dictionary: DictionaryMaxlength =
            from_reader(&mut decoder).map_err(|e| DictionaryError::ParseError(e.to_string()))?;
        Ok(dictionary)
    }
}

// -------------------------------------------------------------

#[derive(Debug)]
pub struct DictMaxLen {
    pub map: FxHashMap<Box<[char]>, Box<str>>, // zero-alloc get(&[char])
    pub max_len: usize,                        // global max in chars
    pub starter_cap: FxHashMap<char, u8>,      // persisted max per starter (chars)

    // runtime-only accelerators
    pub first_len_mask64: Vec<u64>,   // start empty
    pub first_char_max_len: Vec<u16>, // start empty// 65536 entries; max length in chars
}

impl DictMaxLen {
    pub fn build_from_pairs<I>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (String, String)>,
    {
        let mut map = FxHashMap::default();
        let mut starter_cap: FxHashMap<char, u8> = FxHashMap::default();
        let mut global_max = 1usize;

        for (k, v) in pairs {
            let chars: Box<[char]> = k.chars().collect::<Vec<_>>().into_boxed_slice();
            let len = chars.len();
            if let Some(&c0) = chars.first() {
                starter_cap
                    .entry(c0)
                    .and_modify(|m| *m = (*m).max(len as u8))
                    .or_insert(len as u8);
            }
            global_max = global_max.max(len);
            map.insert(chars, v.into_boxed_str());
        }

        let mut dict = Self {
            map,
            max_len: global_max,
            starter_cap,
            first_len_mask64: Vec::new(),   // not built yet
            first_char_max_len: Vec::new(), // not built yet
        };

        // populate starter indexes immediately
        dict.populate_starter_indexes();

        dict
    }

    /// Ensure the starter index buffers exist with the expected sizes.
    pub fn ensure_starter_indexes(&mut self) {
        const N: usize = 0x10000; // BMP size

        if self.first_len_mask64.len() != N {
            self.first_len_mask64.clear();
            self.first_len_mask64.resize(N, 0u64);
        }
        if self.first_char_max_len.len() != N {
            self.first_char_max_len.clear();
            self.first_char_max_len.resize(N, 0u16);
        }
    }

    /// (Re)build the BMP starter indexes from `self.map`, using existing `starter_cap` for per-starter max.
    /// - `first_len_mask64[c]`: bit 0 => len==1, ..., bit 63 => len>=64
    /// - `first_char_max_len[c]`: max phrase length for starter `c` (from `starter_cap`)
    pub fn populate_starter_indexes(&mut self) {
        // const N: usize = 0x10000; // BMP size
        const CAP_BIT: usize = 63; // len >= 64

        // Make sure arrays exist with correct size
        self.ensure_starter_indexes();

        // Clear arrays (we do a full rebuild)
        for v in &mut self.first_len_mask64 {
            *v = 0;
        }
        for v in &mut self.first_char_max_len {
            *v = 0;
        }

        // 1) Build length mask from dictionary keys (BMP starters only)
        for k in self.map.keys() {
            if k.is_empty() {
                continue;
            }
            let c0 = k[0];
            let u = c0 as u32;
            if u > 0xFFFF {
                // ignore non-BMP for now (rare as per your note)
                continue;
            }

            let len = k.len();
            let bit = if len >= 64 { CAP_BIT } else { len - 1 };
            let idx = u as usize;
            self.first_len_mask64[idx] |= 1u64 << bit;
        }

        // 2) Seed per-starter max from existing starter_cap (no recompute)
        for (&c, &cap_u8) in &self.starter_cap {
            let u = c as u32;
            if u <= 0xFFFF {
                self.first_char_max_len[u as usize] = cap_u8 as u16;
            }
        }

        // NOTE: self.max_len is left as-is (fixed as requested).
    }

    #[inline]
    pub fn is_populated(&self) -> bool {
        self.first_len_mask64.len() == 0x10000 && self.first_char_max_len.len() == 0x10000
    }
}

impl Serialize for DictMaxLen {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // wrap &self.map so we can serialize it as a nested JSON object
        struct MapObj<'a>(&'a FxHashMap<Box<[char]>, Box<str>>);
        impl<'a> Serialize for MapObj<'a> {
            fn serialize<S2>(&self, serializer: S2) -> Result<S2::Ok, S2::Error>
            where
                S2: Serializer,
            {
                let mut inner = serializer.serialize_map(Some(self.0.len()))?;
                for (k, v) in self.0.iter() {
                    let ks: String = k.iter().collect();
                    inner.serialize_entry(&ks, &**v)?;
                }
                inner.end()
            }
        }

        // wrap &self.starter_cap similarly (avoids allocating a temporary map)
        struct CapObj<'a>(&'a FxHashMap<char, u8>);
        impl<'a> Serialize for CapObj<'a> {
            fn serialize<S2>(&self, serializer: S2) -> Result<S2::Ok, S2::Error>
            where
                S2: Serializer,
            {
                let mut inner = serializer.serialize_map(Some(self.0.len()))?;
                for (c, len) in self.0.iter() {
                    inner.serialize_entry(&c.to_string(), len)?;
                }
                inner.end()
            }
        }

        // top-level object with 3 fields
        let mut top = serializer.serialize_map(Some(3))?;
        top.serialize_entry("map", &MapObj(&self.map))?;
        top.serialize_entry("max_len", &self.max_len)?;
        top.serialize_entry("starter_cap", &CapObj(&self.starter_cap))?;
        top.end()
    }
}
// Accept both an object {"map": {...}} and legacy array-of-pairs {"map":[["k","v"],...]}
impl<'de> Deserialize<'de> for DictMaxLen {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Accept both object and pairs for "map"
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum MapRepr {
            Object(FxHashMap<String, String>),
            Pairs(Vec<(String, String)>),
        }

        #[derive(Deserialize)]
        struct Helper {
            map: MapRepr,
            #[serde(default)]
            max_len: usize,
            #[serde(default)]
            starter_cap: FxHashMap<String, u8>, // keys are 1-char strings
        }

        let h = Helper::deserialize(deserializer)?;

        // Rebuild map: String -> Box<[char]>, String -> Box<str>
        let mut map: FxHashMap<Box<[char]>, Box<str>> = FxHashMap::default();
        match h.map {
            MapRepr::Object(obj) => {
                for (k, v) in obj {
                    map.insert(
                        k.chars().collect::<Vec<_>>().into_boxed_slice(),
                        v.into_boxed_str(),
                    );
                }
            }
            MapRepr::Pairs(pairs) => {
                for (k, v) in pairs {
                    map.insert(
                        k.chars().collect::<Vec<_>>().into_boxed_slice(),
                        v.into_boxed_str(),
                    );
                }
            }
        }

        // Rebuild starter_cap: String (must be single char) -> char
        let mut starter_cap: FxHashMap<char, u8> = FxHashMap::default();
        for (k, len) in h.starter_cap {
            let mut it = k.chars();
            let c = it
                .next()
                .ok_or_else(|| de::Error::custom("starter_cap key empty"))?;
            if it.next().is_some() {
                return Err(de::Error::custom(
                    "starter_cap key must be a single character",
                ));
            }
            starter_cap.insert(c, len);
        }

        // Use provided max_len or recompute from keys (in Rust char units)
        let max_len = if h.max_len == 0 {
            map.keys().map(|k| k.len()).max().unwrap_or(0)
        } else {
            h.max_len
        };

        Ok(DictMaxLen {
            map,
            max_len,
            starter_cap,
            first_len_mask64: Vec::new(),
            first_char_max_len: Vec::new(),
        })
    }
}
impl Default for DictMaxLen {
    fn default() -> Self {
        Self {
            map: FxHashMap::default(),
            max_len: 0,
            starter_cap: FxHashMap::default(),
            first_len_mask64: Vec::new(),
            first_char_max_len: Vec::new(),
        }
    }
}

// --------------------------------------------------------------

impl Default for DictionaryMaxlength {
    /// Creates an empty `DictionaryMaxlength` with all dictionaries initialized
    /// to empty `FxHashMap`s and their max word lengths set to `0`.
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
        };

        dicts.finish()
    }
}

/// Represents possible errors that can occur during dictionary loading, parsing, or serialization.
///
/// This enum is used throughout the `dictionary_lib` module to wrap low-level I/O or CBOR parsing
/// failures. It provides a unified error type for convenience and compatibility with standard
/// Rust error handling.
///
/// # Variants
/// - `IoError(String)` — An error occurred during file access, reading, or writing.
/// - `ParseError(String)` — An error occurred while deserializing or parsing CBOR or dictionary text.
///
/// This error type is used in methods such as:
/// - [`DictionaryMaxlength::from_zstd()`]
/// - [`DictionaryMaxlength::load_compressed()`]
/// - [`DictionaryMaxlength::from_dicts()`]
#[derive(Debug)]
pub enum DictionaryError {
    IoError(String),
    ParseError(String),
}

impl std::fmt::Display for DictionaryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DictionaryError::IoError(msg) => write!(f, "I/O Error: {}", msg),
            DictionaryError::ParseError(msg) => write!(f, "Parse Error: {}", msg),
        }
    }
}

impl Error for DictionaryError {}

impl From<io::Error> for DictionaryError {
    fn from(err: io::Error) -> Self {
        DictionaryError::IoError(err.to_string())
    }
}

impl From<serde_cbor::Error> for DictionaryError {
    fn from(err: serde_cbor::Error) -> Self {
        DictionaryError::ParseError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let expected_cbor_size = 1113003; // Update this with the actual expected size
        assert_eq!(file_contents.len(), expected_cbor_size);
        // Clean up: Delete the test file
        // fs::remove_file(filename).unwrap();
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
        let max_size = 500000; // Upper bound
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
        let result = DictionaryMaxlength::save_compressed(&dictionary, compressed_file);
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
        let save_result = DictionaryMaxlength::save_compressed(&dictionary, compressed_file);
        assert!(
            save_result.is_ok(),
            "Failed to save compressed dictionary: {:?}",
            save_result
        );

        // Load the dictionary from the compressed file
        let load_result = DictionaryMaxlength::load_compressed(compressed_file);
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
