use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use serde_cbor::from_slice;
use std::collections::HashMap;
use std::error::Error;
use std::path::Path;
use std::sync::Mutex;
use std::{fs, io};

// Define a global mutable variable to store the error message
static LAST_ERROR: Mutex<Option<String>> = Mutex::new(None);

#[derive(Serialize, Deserialize)]
pub struct DictionaryMaxlength {
    pub st_characters: (HashMap<String, String>, usize),
    pub st_phrases: (HashMap<String, String>, usize),
    pub ts_characters: (HashMap<String, String>, usize),
    pub ts_phrases: (HashMap<String, String>, usize),
    pub tw_phrases: (HashMap<String, String>, usize),
    pub tw_phrases_rev: (HashMap<String, String>, usize),
    pub tw_variants: (HashMap<String, String>, usize),
    pub tw_variants_rev: (HashMap<String, String>, usize),
    pub tw_variants_rev_phrases: (HashMap<String, String>, usize),
    pub hk_variants: (HashMap<String, String>, usize),
    pub hk_variants_rev: (HashMap<String, String>, usize),
    pub hk_variants_rev_phrases: (HashMap<String, String>, usize),
    pub jps_characters: (HashMap<String, String>, usize),
    pub jps_phrases: (HashMap<String, String>, usize),
    pub jp_variants: (HashMap<String, String>, usize),
    pub jp_variants_rev: (HashMap<String, String>, usize),
}

impl DictionaryMaxlength {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let cbor_bytes = include_bytes!("dicts/dictionary_maxlength.cbor");
        match from_slice(cbor_bytes) {
            Ok(dictionary) => Ok(dictionary),
            Err(err) => {
                Self::set_last_error(&format!("Failed to read CBOR file: {}", err));
                Err(Box::new(err))
            }
        }
    }

    pub fn from_dicts() -> Self {
        let stc_file_path = include_str!("dicts/STCharacters.txt");
        let stp_file_path = include_str!("dicts/STPhrases.txt");
        let tsc_file_path = include_str!("dicts/TSCharacters.txt");
        let tsp_file_path = include_str!("dicts/TSPhrases.txt");
        let twp_file_path = include_str!("dicts/TWPhrases.txt");
        let twpr_file_path = include_str!("dicts/TWPhrasesRev.txt");
        let twv_file_path = include_str!("dicts/TWVariants.txt");
        let twvr_file_path = include_str!("dicts/TWVariantsRev.txt");
        let twvrp_file_path = include_str!("dicts/TWVariantsRevPhrases.txt");
        let hkv_file_path = include_str!("dicts/HKVariants.txt");
        let hkvr_file_path = include_str!("dicts/HKVariantsRev.txt");
        let hkvrp_file_path = include_str!("dicts/HKVariantsRevPhrases.txt");
        let jpsc_file_path = include_str!("dicts/JPShinjitaiCharacters.txt");
        let jpsp_file_path = include_str!("dicts/JPShinjitaiPhrases.txt");
        let jpv_file_path = include_str!("dicts/JPVariants.txt");
        let jpvr_file_path = include_str!("dicts/JPVariantsRev.txt");
        let st_characters = DictionaryMaxlength::load_dictionary_maxlength(stc_file_path).unwrap();
        let st_phrases = DictionaryMaxlength::load_dictionary_maxlength(stp_file_path).unwrap();
        let ts_characters = DictionaryMaxlength::load_dictionary_maxlength(tsc_file_path).unwrap();
        let ts_phrases = DictionaryMaxlength::load_dictionary_maxlength(tsp_file_path).unwrap();
        let tw_phrases = DictionaryMaxlength::load_dictionary_maxlength(twp_file_path).unwrap();
        let tw_phrases_rev =
            DictionaryMaxlength::load_dictionary_maxlength(twpr_file_path).unwrap();
        let tw_variants = DictionaryMaxlength::load_dictionary_maxlength(twv_file_path).unwrap();
        let tw_variants_rev =
            DictionaryMaxlength::load_dictionary_maxlength(twvr_file_path).unwrap();
        let tw_variants_rev_phrases =
            DictionaryMaxlength::load_dictionary_maxlength(twvrp_file_path).unwrap();
        let hk_variants = DictionaryMaxlength::load_dictionary_maxlength(hkv_file_path).unwrap();
        let hk_variants_rev =
            DictionaryMaxlength::load_dictionary_maxlength(hkvr_file_path).unwrap();
        let hk_variants_rev_phrases =
            DictionaryMaxlength::load_dictionary_maxlength(hkvrp_file_path).unwrap();
        let jps_characters =
            DictionaryMaxlength::load_dictionary_maxlength(jpsc_file_path).unwrap();
        let jps_phrases = DictionaryMaxlength::load_dictionary_maxlength(jpsp_file_path).unwrap();
        let jp_variants = DictionaryMaxlength::load_dictionary_maxlength(jpv_file_path).unwrap();
        let jp_variants_rev =
            DictionaryMaxlength::load_dictionary_maxlength(jpvr_file_path).unwrap();

        DictionaryMaxlength {
            st_characters,
            st_phrases,
            ts_characters,
            ts_phrases,
            tw_phrases,
            tw_phrases_rev,
            tw_variants,
            tw_variants_rev,
            tw_variants_rev_phrases,
            hk_variants,
            hk_variants_rev,
            hk_variants_rev_phrases,
            jps_characters,
            jps_phrases,
            jp_variants,
            jp_variants_rev,
        }
    }

    fn load_dictionary_maxlength(
        dictionary_content: &str,
    ) -> io::Result<(HashMap<String, String>, usize)> {
        let mut dictionary = HashMap::new();
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
    ) -> io::Result<(HashMap<String, String>, usize)> {
        let dictionary = Mutex::new(HashMap::new());
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
}

impl Default for DictionaryMaxlength {
    fn default() -> Self {
        Self {
            st_characters: (HashMap::new(), 0),
            st_phrases: (HashMap::new(), 0),
            ts_characters: (HashMap::new(), 0),
            ts_phrases: (HashMap::new(), 0),
            tw_phrases: (HashMap::new(), 0),
            tw_phrases_rev: (HashMap::new(), 0),
            tw_variants: (HashMap::new(), 0),
            tw_variants_rev: (HashMap::new(), 0),
            tw_variants_rev_phrases: (HashMap::new(), 0),
            hk_variants: (HashMap::new(), 0),
            hk_variants_rev: (HashMap::new(), 0),
            hk_variants_rev_phrases: (HashMap::new(), 0),
            jps_characters: (HashMap::new(), 0),
            jps_phrases: (HashMap::new(), 0),
            jp_variants: (HashMap::new(), 0),
            jp_variants_rev: (HashMap::new(), 0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dictionary_from_dicts_then_to_cbor() {
        // Assuming you have a method `from_dicts` to create a dictionary
        let dictionary = DictionaryMaxlength::from_dicts();
        // Verify that the Dictionary contains the expected data
        let expected = 16;
        assert_eq!(dictionary.st_phrases.1, expected);

        let filename = "dictionary_maxlength.cbor";
        dictionary.serialize_to_cbor(filename).unwrap();
        let file_contents = fs::read(filename).unwrap();
        let expected_cbor_size = 1113003; // Update this with the actual expected size
        assert_eq!(file_contents.len(), expected_cbor_size);
        // Clean up: Delete the test file
        fs::remove_file(filename).unwrap();
    }
}
