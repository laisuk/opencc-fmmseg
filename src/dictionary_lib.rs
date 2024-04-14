use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::sync::Mutex;
use std::{fs, io};

use serde::{Deserialize, Serialize};
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
        let json_data = include_str!("dicts/dictionary_maxlength.json");
        let dictionary: Self = match serde_json::from_str(json_data) {
            Ok(data) => data,
            Err(err) => {
                Self::set_last_error(&format!("Failed to read JSON file: {}", err));
                return Err(Box::new(err));
            }
        };

        Ok(dictionary)
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

    #[allow(dead_code)]
    pub fn from_json(filename: &str) -> io::Result<Self> {
        // Read the contents of the JSON file
        let json_string = match fs::read_to_string(filename) {
            Ok(data) => data,
            Err(err) => {
                Self::set_last_error(&format!("Failed to read JSON file: {}", err));
                return Err(err);
            }
        };
        // Deserialize the JSON string into a Dictionary struct
        let dictionary: DictionaryMaxlength = match serde_json::from_str(&json_string) {
            Ok(data) => data,
            Err(err) => {
                Self::set_last_error(&format!("Failed to deserialize JSON: {}", err));
                return Err(io::Error::new(io::ErrorKind::InvalidData, err));
            }
        };

        Ok(dictionary)
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

    // Function to serialize Dictionary to JSON and write it to a file
    pub fn serialize_to_json(&self, filename: &str) -> io::Result<()> {
        // Serialize the Dictionary to JSON
        let json_string = match serde_json::to_string(&self) {
            Ok(json) => json,
            Err(err) => {
                Self::set_last_error(&format!("Failed to serialize JSON: {}", err));
                return Err(io::Error::new(io::ErrorKind::InvalidData, err));
            }
        };
        // Write JSON string to file
        let mut file = match File::create(filename) {
            Ok(f) => f,
            Err(err) => {
                Self::set_last_error(&format!("Failed to create file: {}", err));
                return Err(err);
            }
        };

        if let Err(err) = file.write_all(json_string.as_bytes()) {
            Self::set_last_error(&format!("Failed to write to file: {}", err));
            return Err(err);
        }

        Ok(())
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
