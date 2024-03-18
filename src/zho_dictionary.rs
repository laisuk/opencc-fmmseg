use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::{fs, io};

pub struct Dictionary {
    pub st_characters: HashMap<String, String>,
    pub st_phrases: HashMap<String, String>,
    pub ts_characters: HashMap<String, String>,
    pub ts_phrases: HashMap<String, String>,
    pub tw_phrases: HashMap<String, String>,
    pub tw_phrases_rev: HashMap<String, String>,
    pub tw_variants: HashMap<String, String>,
    pub tw_variants_rev: HashMap<String, String>,
    pub tw_variants_rev_phrases: HashMap<String, String>,
    pub hk_variants: HashMap<String, String>,
    pub hk_variants_rev: HashMap<String, String>,
    pub hk_variants_rev_phrases: HashMap<String, String>,
    pub jps_characters: HashMap<String, String>,
    pub jps_phrases: HashMap<String, String>,
    pub jp_variants: HashMap<String, String>,
    pub jp_variants_rev: HashMap<String, String>,
    pub st_characters_max_length: usize,
    pub st_phrases_max_length: usize,
    pub ts_characters_max_length: usize,
    pub ts_phrases_max_length: usize,
    pub tw_phrases_max_length: usize,
    pub tw_phrases_rev_max_length: usize,
    pub tw_variants_max_length: usize,
    pub tw_variants_rev_max_length: usize,
    pub tw_variants_rev_phrases_max_length: usize,
    pub hk_variants_max_length: usize,
    pub hk_variants_rev_max_length: usize,
    pub hk_variants_rev_phrases_max_length: usize,
    pub jps_characters_max_length: usize,
    pub jps_phrases_max_length: usize,
    pub jp_variants_max_length: usize,
    pub jp_variants_rev_max_length: usize,
}

impl Dictionary {
    pub fn new() -> Self {
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
        let stc_dict = Dictionary::load_dictionary(stc_file_path).unwrap();
        let stp_dict = Dictionary::load_dictionary(stp_file_path).unwrap();
        let tsc_dict = Dictionary::load_dictionary(tsc_file_path).unwrap();
        let tsp_dict = Dictionary::load_dictionary(tsp_file_path).unwrap();
        let twp_dict = Dictionary::load_dictionary(twp_file_path).unwrap();
        let twpr_dict = Dictionary::load_dictionary(twpr_file_path).unwrap();
        let twv_dict = Dictionary::load_dictionary(twv_file_path).unwrap();
        let twvr_dict = Dictionary::load_dictionary(twvr_file_path).unwrap();
        let twvrp_dict = Dictionary::load_dictionary(twvrp_file_path).unwrap();
        let hkv_dict = Dictionary::load_dictionary(hkv_file_path).unwrap();
        let hkvr_dict = Dictionary::load_dictionary(hkvr_file_path).unwrap();
        let hkvrp_dict = Dictionary::load_dictionary(hkvrp_file_path).unwrap();
        let jpsc_dict = Dictionary::load_dictionary(jpsc_file_path).unwrap();
        let jpsp_dict = Dictionary::load_dictionary(jpsp_file_path).unwrap();
        let jpv_dict = Dictionary::load_dictionary(jpv_file_path).unwrap();
        let jpvr_dict = Dictionary::load_dictionary(jpvr_file_path).unwrap();

        Dictionary {
            st_characters: stc_dict.0,
            st_phrases: stp_dict.0,
            ts_characters: tsc_dict.0,
            ts_phrases: tsp_dict.0,
            tw_phrases: twp_dict.0,
            tw_phrases_rev: twpr_dict.0,
            tw_variants: twv_dict.0,
            tw_variants_rev: twvr_dict.0,
            tw_variants_rev_phrases: twvrp_dict.0,
            hk_variants: hkv_dict.0,
            hk_variants_rev: hkvr_dict.0,
            hk_variants_rev_phrases: hkvrp_dict.0,
            jps_characters: jpsc_dict.0,
            jps_phrases: jpsp_dict.0,
            jp_variants: jpv_dict.0,
            jp_variants_rev: jpvr_dict.0,
            st_characters_max_length: stc_dict.1,
            st_phrases_max_length: stp_dict.1,
            ts_characters_max_length: tsc_dict.1,
            ts_phrases_max_length: tsp_dict.1,
            tw_phrases_max_length: twp_dict.1,
            tw_phrases_rev_max_length: twpr_dict.1,
            tw_variants_max_length: twv_dict.1,
            tw_variants_rev_max_length: twvr_dict.1,
            tw_variants_rev_phrases_max_length: twvrp_dict.1,
            hk_variants_max_length: hkv_dict.1,
            hk_variants_rev_max_length: hkvr_dict.1,
            hk_variants_rev_phrases_max_length: hkvrp_dict.1,
            jps_characters_max_length: jpsc_dict.1,
            jps_phrases_max_length: jpsp_dict.1,
            jp_variants_max_length: jpv_dict.1,
            jp_variants_rev_max_length: jpvr_dict.1,
        }
    }

    fn load_dictionary(dictionary_content: &str) -> io::Result<(HashMap<String, String>, usize)> {
        let mut dictionary = HashMap::new();
        let mut max_length: usize = 1;

        for line in dictionary_content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let phrase = parts[0].to_string();
                let translation = parts[1].to_string();
                if max_length < phrase.chars().count() {
                    max_length = phrase.chars().count();
                }
                dictionary.insert(phrase, translation);
            } else {
                eprintln!("Invalid line format: {}", line);
            }
        }

        Ok((dictionary, max_length))
    }

    #[allow(dead_code)]
    fn load_dictionary_from_path<P>(filename: P) -> io::Result<HashMap<String, String>>
    where
        P: AsRef<Path>,
    {
        let file = fs::File::open(filename)?;
        let mut dictionary = HashMap::new();

        for line in BufReader::new(file).lines() {
            let line = line?;
            // let parts: Vec<&str> = line.split('\t').collect();
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() > 1 {
                let phrase = parts[0].to_string();
                let translation = parts[1].to_string();
                dictionary.insert(phrase, translation);
            } else {
                eprintln!("Invalid line format: {}", line);
            }
        }

        Ok(dictionary)
    }

    #[allow(dead_code)]
    fn load_dictionary_from_str(dictionary_content: &str) -> io::Result<HashMap<String, String>> {
        let mut dictionary = HashMap::new();

        for line in dictionary_content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let phrase = parts[0].to_string();
                let translation = parts[1].to_string();
                dictionary.insert(phrase, translation);
            } else {
                eprintln!("Invalid line format: {}", line);
            }
        }

        Ok(dictionary)
    }
}
