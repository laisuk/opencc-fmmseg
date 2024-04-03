use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::{fs, io};

use serde::{Deserialize, Serialize};

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
    pub fn new() -> Self {
        let json_data = include_str!("dicts/dictionary_maxlength.json");
        let dictionary: DictionaryMaxlength = serde_json::from_str(json_data).unwrap();
        dictionary
    }

    #[allow(dead_code)]
    pub fn from_json(filename: &str) -> io::Result<Self> {
        // Read the contents of the JSON file
        let json_string = fs::read_to_string(filename)?;
        // Deserialize the JSON string into a Dictionary struct
        let dictionary: DictionaryMaxlength = serde_json::from_str(&json_string)?;

        Ok(dictionary)
    }

    // Function to serialize Dictionary to JSON and write it to a file
    pub fn serialize_to_json(&self, filename: &str) -> io::Result<()> {
        let json_string = serde_json::to_string(&self)?;
        let mut file = File::create(filename)?;
        file.write_all(json_string.as_bytes())?;
        Ok(())
    }
}
