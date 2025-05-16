use rayon::prelude::*;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use serde_cbor::{from_reader, from_slice};
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, BufWriter, Cursor};
use std::path::Path;
use std::sync::Mutex;
use std::{fs, io};
use zstd::{decode_all, Decoder, Encoder};

// Define a global mutable variable to store the error message
static LAST_ERROR: Mutex<Option<String>> = Mutex::new(None);

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
    pub ts_punctuations: (FxHashMap<String, String>, usize)
}

impl DictionaryMaxlength {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        Ok(Self::from_zstd().map_err(|err| {
            Self::set_last_error(&format!("Failed to load dictionary from Zstd: {}", err));
            Box::new(err)
        })?)
    }

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

    pub fn from_dicts() -> Result<Self, Box<dyn Error>> {
        let stc_file_path = "dicts/STCharacters.txt";
        let stp_file_path = "dicts/STPhrases.txt";
        let tsc_file_path = "dicts/TSCharacters.txt";
        let tsp_file_path = "dicts/TSPhrases.txt";
        let twp_file_path = "dicts/TWPhrases.txt";
        let twpr_file_path = "dicts/TWPhrasesRev.txt";
        let twv_file_path = "dicts/TWVariants.txt";
        let twvr_file_path = "dicts/TWVariantsRev.txt";
        let twvrp_file_path = "dicts/TWVariantsRevPhrases.txt";
        let hkv_file_path = "dicts/HKVariants.txt";
        let hkvr_file_path = "dicts/HKVariantsRev.txt";
        let hkvrp_file_path = "dicts/HKVariantsRevPhrases.txt";
        let jpsc_file_path = "dicts/JPShinjitaiCharacters.txt";
        let jpsp_file_path = "dicts/JPShinjitaiPhrases.txt";
        let jpv_file_path = "dicts/JPVariants.txt";
        let jpvr_file_path = "dicts/JPVariantsRev.txt";
        let stpunct_file_path = "dicts/STPunctuations.txt";
        let tspunct_file_path = "dicts/TSPunctuations.txt";

        fn load_dict(path: &str) -> Result<(FxHashMap<String, String>, usize), DictionaryError> {
            let content = fs::read_to_string(path).map_err(|err| {
                DictionaryError::IoError(format!("Failed to read file {}: {}", path, err))
            })?;

            DictionaryMaxlength::load_dictionary_maxlength(&content).map_err(|err| {
                DictionaryError::ParseError(format!("Failed to parse dictionary {}: {}", path, err))
            })
        }

        Ok(DictionaryMaxlength {
            st_characters: load_dict(stc_file_path)?,
            st_phrases: load_dict(stp_file_path)?,
            ts_characters: load_dict(tsc_file_path)?,
            ts_phrases: load_dict(tsp_file_path)?,
            tw_phrases: load_dict(twp_file_path)?,
            tw_phrases_rev: load_dict(twpr_file_path)?,
            tw_variants: load_dict(twv_file_path)?,
            tw_variants_rev: load_dict(twvr_file_path)?,
            tw_variants_rev_phrases: load_dict(twvrp_file_path)?,
            hk_variants: load_dict(hkv_file_path)?,
            hk_variants_rev: load_dict(hkvr_file_path)?,
            hk_variants_rev_phrases: load_dict(hkvrp_file_path)?,
            jps_characters: load_dict(jpsc_file_path)?,
            jps_phrases: load_dict(jpsp_file_path)?,
            jp_variants: load_dict(jpv_file_path)?,
            jp_variants_rev: load_dict(jpvr_file_path)?,
            st_punctuations: load_dict(stpunct_file_path)?,
            ts_punctuations: load_dict(tspunct_file_path)?,
        })
    }

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

    #[allow(dead_code)]
    fn load_dictionary_maxlength_par(
        dictionary_content: &str,
    ) -> io::Result<(FxHashMap<String, String>, usize)> {
        let dictionary = Mutex::new(FxHashMap::default());
        let max_length = Mutex::new(1);

        dictionary_content.par_lines().for_each(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let phrase = parts[0].to_string();
                let translation = parts[1].to_string();
                let char_count = phrase.chars().count();

                // Update max_length in a thread-safe way
                let mut max_len_guard = max_length.lock().unwrap();
                if *max_len_guard < char_count {
                    *max_len_guard = char_count;
                }

                // Insert into dictionary in a thread-safe way
                let mut dict_guard = dictionary.lock().unwrap();
                dict_guard.insert(phrase, translation);
            } else {
                eprintln!("Invalid line format: {}", line);
            }
        });

        let dictionary = Mutex::into_inner(dictionary).unwrap();
        let max_length = Mutex::into_inner(max_length).unwrap();

        Ok((dictionary, max_length))
    }

    /// Serialize dictionary to CBOR file
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

    /// Deserialize dictionary from CBOR file
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

    // Function to set the last error message
    pub fn set_last_error(err_msg: &str) {
        let mut last_error = LAST_ERROR.lock().unwrap();
        *last_error = Some(err_msg.to_string());
    }

    // Function to retrieve the last error message
    pub fn get_last_error() -> Option<String> {
        let last_error = LAST_ERROR.lock().unwrap();
        last_error.clone()
    }

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
        use crate::dictionary_lib::DictionaryMaxlength;
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
        use crate::dictionary_lib::DictionaryMaxlength;
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
}
