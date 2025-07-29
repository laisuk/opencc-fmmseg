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

// Define a global mutable variable to store the error message
static LAST_ERROR: Mutex<Option<String>> = Mutex::new(None);

/// Represents a collection of OpenCC dictionaries paired with their maximum word lengths.
///
/// This structure is used internally by the `OpenCC` engine to support fast, segment-based
/// forward maximum matching (FMM) for Chinese text conversion. Each dictionary maps a phrase
/// or character to its target form and tracks the longest entry for lookup performance.
#[derive(Serialize, Deserialize, Debug)]
pub struct DictionaryMaxlength {
    pub st_characters: (FxHashMap<String, String>, usize),
    pub st_phrases: (FxHashMap<String, String>, usize),
    pub ts_characters: (FxHashMap<String, String>, usize),
    pub ts_phrases: (FxHashMap<String, String>, usize),
    pub tw_phrases: (FxHashMap<String, String>, usize),
    pub tw_phrases_rev: (FxHashMap<String, String>, usize),
    pub tw_variants: (FxHashMap<String, String>, usize),
    pub tw_variants_rev: (FxHashMap<String, String>, usize),
    pub tw_variants_rev_phrases: (FxHashMap<String, String>, usize),
    pub hk_variants: (FxHashMap<String, String>, usize),
    pub hk_variants_rev: (FxHashMap<String, String>, usize),
    pub hk_variants_rev_phrases: (FxHashMap<String, String>, usize),
    pub jps_characters: (FxHashMap<String, String>, usize),
    pub jps_phrases: (FxHashMap<String, String>, usize),
    pub jp_variants: (FxHashMap<String, String>, usize),
    pub jp_variants_rev: (FxHashMap<String, String>, usize),
    pub st_punctuations: (FxHashMap<String, String>, usize),
    pub ts_punctuations: (FxHashMap<String, String>, usize),
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

        Ok(dictionary)
    }

    /// Loads dictionary from an embedded CBOR file.
    pub fn from_cbor() -> Result<Self, Box<dyn Error>> {
        let cbor_bytes = include_bytes!("dicts/dictionary_maxlength.cbor");
        match from_slice(cbor_bytes) {
            Ok(dictionary) => Ok(dictionary),
            Err(err) => {
                Self::set_last_error(&format!("Failed to read CBOR file: {}", err));
                Err(Box::new(err))
            }
        }
    }

    /// Loads dictionary from plaintext `.txt` dictionary files.
    ///
    /// This method is used primarily for development and regeneration.
    pub fn from_dicts() -> Result<Self, Box<dyn Error>> {
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

        fn load_dict(
            base_dir: &str,
            filename: &str,
        ) -> Result<(FxHashMap<String, String>, usize), DictionaryError> {
            let path = Path::new(base_dir).join(filename);
            let path_str = path.to_string_lossy();
            let content = fs::read_to_string(&path).map_err(|err| {
                DictionaryError::IoError(format!("Failed to read file {}: {}", path_str, err))
            })?;

            DictionaryMaxlength::load_dictionary_maxlength(&content).map_err(|err| {
                DictionaryError::ParseError(format!(
                    "Failed to parse dictionary {}: {}",
                    path_str, err
                ))
            })
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

    #[doc(hidden)]
    fn load_dictionary_maxlength(
        dictionary_content: &str,
    ) -> io::Result<(FxHashMap<String, String>, usize)> {
        let mut dictionary = FxHashMap::default();
        let mut max_length: usize = 1;

        for line in dictionary_content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let phrase = parts[0].to_string();
                let translation = parts[1].to_string();
                let char_count = phrase.chars().count();
                if max_length < char_count {
                    max_length = char_count;
                }
                dictionary.insert(phrase, translation);
            } else {
                eprintln!("Invalid line format: {}", line);
            }
        }

        Ok((dictionary, max_length))
    }

    /// Saves all dictionaries to plaintext `.txt` files in the specified directory.
    pub fn to_dicts(&self, base_dir: &str) -> Result<(), Box<dyn Error>> {
        let dict_map: HashMap<&str, &FxHashMap<String, String>> = [
            ("STCharacters.txt", &self.st_characters.0),
            ("STPhrases.txt", &self.st_phrases.0),
            ("TSCharacters.txt", &self.ts_characters.0),
            ("TSPhrases.txt", &self.ts_phrases.0),
            ("TWPhrases.txt", &self.tw_phrases.0),
            ("TWPhrasesRev.txt", &self.tw_phrases_rev.0),
            ("TWVariants.txt", &self.tw_variants.0),
            ("TWVariantsRev.txt", &self.tw_variants_rev.0),
            ("TWVariantsRevPhrases.txt", &self.tw_variants_rev_phrases.0),
            ("HKVariants.txt", &self.hk_variants.0),
            ("HKVariantsRev.txt", &self.hk_variants_rev.0),
            ("HKVariantsRevPhrases.txt", &self.hk_variants_rev_phrases.0),
            ("JPShinjitaiCharacters.txt", &self.jps_characters.0),
            ("JPShinjitaiPhrases.txt", &self.jps_phrases.0),
            ("JPVariants.txt", &self.jp_variants.0),
            ("JPVariantsRev.txt", &self.jp_variants_rev.0),
            ("STPunctuations.txt", &self.st_punctuations.0),
            ("TSPunctuations.txt", &self.ts_punctuations.0),
        ]
        .into_iter()
        .collect();

        fs::create_dir_all(base_dir)?; // ensure base_dir exists

        for (filename, dict) in dict_map {
            let path = Path::new(base_dir).join(filename);
            let mut file = File::create(&path)?;

            for (key, value) in dict {
                writeln!(file, "{}\t{}", key, value)?;
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
            Ok(cbor_data) => match from_slice(&cbor_data) {
                Ok(dictionary) => Ok(dictionary),
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
        Self {
            st_characters: (FxHashMap::default(), 0),
            st_phrases: (FxHashMap::default(), 0),
            ts_characters: (FxHashMap::default(), 0),
            ts_phrases: (FxHashMap::default(), 0),
            tw_phrases: (FxHashMap::default(), 0),
            tw_phrases_rev: (FxHashMap::default(), 0),
            tw_variants: (FxHashMap::default(), 0),
            tw_variants_rev: (FxHashMap::default(), 0),
            tw_variants_rev_phrases: (FxHashMap::default(), 0),
            hk_variants: (FxHashMap::default(), 0),
            hk_variants_rev: (FxHashMap::default(), 0),
            hk_variants_rev_phrases: (FxHashMap::default(), 0),
            jps_characters: (FxHashMap::default(), 0),
            jps_phrases: (FxHashMap::default(), 0),
            jp_variants: (FxHashMap::default(), 0),
            jp_variants_rev: (FxHashMap::default(), 0),
            st_punctuations: (FxHashMap::default(), 0),
            ts_punctuations: (FxHashMap::default(), 0),
        }
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
        assert_eq!(dictionary.st_phrases.1, expected);

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
        assert_eq!(dictionary.st_phrases.1, expected);
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
            dictionary.st_phrases.1, loaded_dictionary.st_phrases.1,
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

        // Dummy data for just 2 fields (you can fill more if needed)
        let mut dummy_map = FxHashMap::default();
        dummy_map.insert("测试".to_string(), "測試".to_string());
        dummy_map.insert("语言".to_string(), "語言".to_string());

        let dicts = DictionaryMaxlength {
            st_characters: (dummy_map.clone(), 2),
            st_phrases: (dummy_map.clone(), 2),
            ts_characters: Default::default(),
            ts_phrases: Default::default(),
            tw_phrases: Default::default(),
            tw_phrases_rev: Default::default(),
            tw_variants: Default::default(),
            tw_variants_rev: Default::default(),
            tw_variants_rev_phrases: Default::default(),
            hk_variants: Default::default(),
            hk_variants_rev: Default::default(),
            hk_variants_rev_phrases: Default::default(),
            jps_characters: Default::default(),
            jps_phrases: Default::default(),
            jp_variants: Default::default(),
            jp_variants_rev: Default::default(),
            st_punctuations: Default::default(),
            ts_punctuations: Default::default(),
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
