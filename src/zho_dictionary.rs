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
        let json_string = fs::read_to_string(filename)?;
        // Deserialize the JSON string into a Dictionary struct
        let dictionary: DictionaryMaxlength = serde_json::from_str(&json_string)?;

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
        let json_string = serde_json::to_string(&self)?;
        let mut file = File::create(filename)?;
        file.write_all(json_string.as_bytes())?;
        Ok(())
    }
}
