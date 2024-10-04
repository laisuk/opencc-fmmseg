use lazy_static::lazy_static;
use std::collections::{HashMap, HashSet};
use std::iter::Iterator;
use std::sync::{Arc, Mutex};

use rayon::prelude::*;
use regex::Regex;

use crate::dictionary_lib::{DictRefs, DictionaryMaxlength};
pub mod dictionary_lib;
// Define a global mutable variable to store the error message
static LAST_ERROR: Mutex<Option<String>> = Mutex::new(None);
const DELIMITERS: &'static str = "\t\n\r (){}[]<>\"'\\/|-,.?!*:;@#$%^&_+=　，。、；：？！…“”‘’『』「」﹁﹂—－（）《》〈〉～．／＼︒︑︔︓︿﹀︹︺︙︐［﹇］﹈︕︖︰︳︴︽︾︵︶｛︷｝︸﹃﹄【︻】︼";
lazy_static! {
    static ref STRIP_REGEX: Regex = Regex::new(r"[!-/:-@\[-`{-~\t\n\v\f\r 0-9A-Za-z_]").unwrap();
}

pub struct OpenCC {
    dictionary: DictionaryMaxlength,
    delimiters: HashSet<char>,
    is_parallel: bool,
}

impl OpenCC {
    pub fn new() -> Self {
        let dictionary = DictionaryMaxlength::new().unwrap_or_else(|err| {
            Self::set_last_error(&format!("Failed to create dictionary: {}", err));
            // Since DictionaryMaxlength::new() returns a DictionaryMaxlength
            // instance on success, we create a default instance here to
            // maintain the structure of OpenCC.
            DictionaryMaxlength::default()
        });
        let delimiters = DELIMITERS.chars().collect();
        let is_parallel = true;

        OpenCC {
            dictionary,
            delimiters,
            is_parallel,
        }
    }

    pub fn from_dicts() -> Self {
        let dictionary = DictionaryMaxlength::from_dicts();
        let delimiters = DELIMITERS.chars().collect();
        let is_parallel = true;

        OpenCC {
            dictionary,
            delimiters,
            is_parallel,
        }
    }
    pub fn from_json(filename: &str) -> Self {
        let dictionary = DictionaryMaxlength::from_json(filename).unwrap_or_else(|err| {
            Self::set_last_error(&format!("Failed to create dictionary: {}", err));
            DictionaryMaxlength::default()
        });
        let delimiters = DELIMITERS.chars().collect();
        let is_parallel = true;

        OpenCC {
            dictionary,
            delimiters,
            is_parallel,
        }
    }
    fn segment_replace(
        &self,
        text: &str,
        dictionaries: &[&(HashMap<String, String>, usize)],
    ) -> String {
        let mut max_word_length: usize = 1;
        for i in 0..dictionaries.len() {
            if max_word_length < dictionaries[i].1 {
                max_word_length = dictionaries[i].1;
            }
        }

        if self.is_parallel {
            let split_string_list = self.split_string_inclusive_parallel(text);
            self.get_translated_string_parallel(split_string_list, dictionaries, max_word_length)
        } else {
            let split_string_list = self.split_string_inclusive(text);
            self.get_translated_string(split_string_list, dictionaries, max_word_length)
        }
    }

    fn get_translated_string(
        &self,
        split_string_list: Vec<String>,
        dictionaries: &[&(HashMap<String, String>, usize)],
        max_word_length: usize,
    ) -> String {
        split_string_list
            .iter()
            .map(|chunk| self.convert_by(chunk, dictionaries, max_word_length))
            .collect::<String>()
    }

    fn get_translated_string_parallel(
        &self,
        split_string_list: Vec<String>,
        dictionaries: &[&(HashMap<String, String>, usize)],
        max_word_length: usize,
    ) -> String {
        split_string_list
            .par_iter()
            .map(|chunk| self.convert_by(chunk, dictionaries, max_word_length))
            .collect::<String>()
    }

    #[allow(dead_code)]
    fn get_translated_string_parallel_arc(
        &self,
        split_string_list: Vec<String>,
        dictionaries: &[&(HashMap<String, String>, usize)],
        max_word_length: usize,
    ) -> String {
        let result = Arc::new(Mutex::new(Vec::<(usize, String)>::new()));
        split_string_list
            .par_iter()
            .enumerate()
            .for_each(|(index, chunk)| {
                let converted = self.convert_by(chunk, dictionaries, max_word_length);
                let mut result_lock = result.lock().unwrap();
                result_lock.push((index, converted));
            });
        let mut result_lock = result.lock().unwrap();
        result_lock.par_sort_by_key(|(index, _)| *index);

        let concatenated_result: String = result_lock
            .par_iter()
            .map(|(_, chunk)| chunk.as_str())
            .collect();

        concatenated_result
    }

    fn convert_by(
        &self,
        text: &str,
        dictionaries: &[&(HashMap<String, String>, usize)],
        max_word_length: usize,
    ) -> String {
        if text.is_empty() {
            return String::new();
        }

        let text_chars: Vec<_> = text.par_chars().collect();
        let text_length = text_chars.len();
        if text_length == 1 {
            if self.delimiters.contains(&text_chars[0]) {
                return text_chars[0].to_string();
            }
        }

        let mut result = String::new();
        result.reserve(text.len());

        let mut start_pos = 0;
        while start_pos < text_length {
            let max_length = std::cmp::min(max_word_length, text_length - start_pos);
            let mut best_match_length = 0;
            let mut best_match = String::new();

            for length in 1..=max_length {
                let candidate: String =
                    text_chars[start_pos..(start_pos + length)].iter().collect();
                for dictionary in dictionaries {
                    if let Some(value) = dictionary.0.get(&candidate) {
                        best_match_length = length;
                        best_match = value.to_owned();
                        break; // Push the corresponding value to the results
                    }
                }
            }

            if best_match_length == 0 {
                // If no match found, treat the character as a single word
                best_match_length = 1;
                best_match = text_chars[start_pos].to_string();
            }

            result.push_str(&best_match);
            start_pos += best_match_length;
        }

        result
    }

    fn split_string_inclusive(&self, text: &str) -> Vec<String> {
        let mut split_string_list = Vec::new();
        let mut current_chunk = String::new();

        for ch in text.chars() {
            if self.delimiters.contains(&ch) {
                split_string_list.push(current_chunk + &ch.to_string());
                current_chunk = String::new();
            } else {
                current_chunk.push(ch);
            }
        }
        // Current_chunk still have chars but current_delimiter is empty
        if !current_chunk.is_empty() {
            split_string_list.push(current_chunk);
        }

        split_string_list
    }

    fn split_string_inclusive_parallel(&self, text: &str) -> Vec<String> {
        let split_string_list: Vec<String> = text
            .par_chars()
            .collect::<Vec<char>>()
            .par_split_inclusive_mut(|c| self.delimiters.contains(c))
            .map(|slice| slice.par_iter().collect())
            .collect();

        split_string_list
    }

    pub fn get_parallel(&self) -> bool {
        self.is_parallel
    }

    pub fn set_parallel(&mut self, is_parallel: bool) -> () {
        self.is_parallel = is_parallel;
    }

    pub fn s2t(&self, input: &str, punctuation: bool) -> String {
        let round_1 = [&self.dictionary.st_phrases, &self.dictionary.st_characters];
        let dict_refs = DictRefs::new(&round_1);
        let output = dict_refs.apply_segment_replace(input, |input, refs| self.segment_replace(input, refs));
        if punctuation {
            Self::convert_punctuation(&output, "s")
        } else {
            output
        }
    }

    pub fn t2s(&self, input: &str, punctuation: bool) -> String {
        let round_1 = [&self.dictionary.ts_phrases, &self.dictionary.ts_characters];
        let dict_refs = DictRefs::new(&round_1);
        let output = dict_refs.apply_segment_replace(input, |input, refs| self.segment_replace(input, refs));
        if punctuation {
            Self::convert_punctuation(&output, "t")
        } else {
            output
        }
    }

    pub fn s2tw(&self, input: &str, punctuation: bool) -> String {
        let round_1 = [&self.dictionary.st_phrases, &self.dictionary.st_characters];
        let round_2 = [&self.dictionary.tw_variants];
        let dict_refs = DictRefs::new(&round_1)
            .with_round_2(&round_2);
        let output = dict_refs.apply_segment_replace(input, |input, refs| self.segment_replace(input, refs));
        if punctuation {
            Self::convert_punctuation(&output, "s")
        } else {
            output.to_string()
        }
    }

    pub fn tw2s(&self, input: &str, punctuation: bool) -> String {
        let round_1 = [
            &self.dictionary.tw_variants_rev_phrases,
            &self.dictionary.tw_variants_rev,
        ];
        let round_2 = [&self.dictionary.ts_phrases, &self.dictionary.ts_characters];
        let dict_refs = DictRefs::new(&round_1)
            .with_round_2(&round_2);
        let output = dict_refs.apply_segment_replace(input, |input, refs| self.segment_replace(input, refs));
        if punctuation {
            Self::convert_punctuation(&output, "t")
        } else {
            output
        }
    }

    pub fn s2twp(&self, input: &str, punctuation: bool) -> String {
        // Create bindings for each round of dictionary references
        let round_1 = [&self.dictionary.st_phrases, &self.dictionary.st_characters];
        let round_2 = [&self.dictionary.tw_phrases];
        let round_3 = [&self.dictionary.tw_variants];
        // Use the DictRefs struct to handle 3 rounds
        let dict_refs = DictRefs::new(&round_1)
            .with_round_2(&round_2)
            .with_round_3(&round_3);
        // Apply the segment_replace function using the dictionary references
        let output = dict_refs.apply_segment_replace(input, |input, refs| self.segment_replace(input, refs));
        // Handle punctuation if needed
        if punctuation {
            Self::convert_punctuation(&output, "s")
        } else {
            output
        }
    }

    pub fn tw2sp(&self, input: &str, punctuation: bool) -> String {
        let round_1 = [
            &self.dictionary.tw_variants_rev_phrases,
            &self.dictionary.tw_variants_rev,
        ];
        let round_2 = [&self.dictionary.tw_phrases_rev];
        let round_3 = [&self.dictionary.ts_phrases, &self.dictionary.ts_characters];
        let dict_refs = DictRefs::new(&round_1)
            .with_round_2(&round_2)
            .with_round_3(&round_3);
        let output = dict_refs.apply_segment_replace(input, |input, refs| self.segment_replace(input, refs));
        if punctuation {
            Self::convert_punctuation(&output, "t")
        } else {
            output
        }
    }

    pub fn s2hk(&self, input: &str, punctuation: bool) -> String {
        let round_1 = [&self.dictionary.st_phrases, &self.dictionary.st_characters];
        let round_2 = [&self.dictionary.hk_variants];
        let dict_refs = DictRefs::new(&round_1)
            .with_round_2(&round_2);
        let output = dict_refs.apply_segment_replace(input, |input, refs| self.segment_replace(input, refs));
        if punctuation {
            Self::convert_punctuation(&output, "s")
        } else {
            output
        }
    }

    pub fn hk2s(&self, input: &str, punctuation: bool) -> String {
        let round_1 = [
            &self.dictionary.hk_variants_rev_phrases,
            &self.dictionary.hk_variants_rev,
        ];
        let round_2 = [&self.dictionary.ts_phrases, &self.dictionary.ts_characters];
        let dict_refs = DictRefs::new(&round_1)
            .with_round_2(&round_2);
        let output = dict_refs.apply_segment_replace(input, |input, refs| self.segment_replace(input, refs));
        if punctuation {
            Self::convert_punctuation(&output, "t")
        } else {
            output
        }
    }

    pub fn t2tw(&self, input: &str) -> String {
        let round_1 = [&self.dictionary.tw_variants];
        let dict_refs = DictRefs::new(&round_1);
        let output = dict_refs.apply_segment_replace(input, |input, refs| self.segment_replace(input, refs));

        output
    }

    pub fn t2twp(&self, input: &str) -> String {
        let round_1 = [&self.dictionary.tw_phrases];
        let round_2 = [&self.dictionary.tw_variants];
        let dict_refs = DictRefs::new(&round_1)
            .with_round_2(&round_2);
        let output = dict_refs.apply_segment_replace(input, |input, refs| self.segment_replace(input, refs));

        output
    }

    pub fn tw2t(&self, input: &str) -> String {
        let round_1 = [
            &self.dictionary.tw_variants_rev_phrases,
            &self.dictionary.tw_variants_rev,
        ];
        let dict_refs = DictRefs::new(&round_1);
        let output = dict_refs.apply_segment_replace(input, |input, refs| self.segment_replace(input, refs));

        output
    }

    pub fn tw2tp(&self, input: &str) -> String {
        let round_1 = [
            &self.dictionary.tw_variants_rev_phrases,
            &self.dictionary.tw_variants_rev,
        ];
        let round_2 = [&self.dictionary.tw_phrases_rev];
        let dict_refs = DictRefs::new(&round_1)
            .with_round_2(&round_2);
        let output = dict_refs.apply_segment_replace(input, |input, refs| self.segment_replace(input, refs));

        output
    }

    pub fn t2hk(&self, input: &str) -> String {
        let round_1 = [&self.dictionary.hk_variants];
        let dict_refs = DictRefs::new(&round_1);
        let output = dict_refs.apply_segment_replace(input, |input, refs| self.segment_replace(input, refs));

        output
    }

    pub fn hk2t(&self, input: &str) -> String {
        let round_1 = [
            &self.dictionary.hk_variants_rev_phrases,
            &self.dictionary.hk_variants_rev,
        ];
        let dict_refs = DictRefs::new(&round_1);
        let output = dict_refs.apply_segment_replace(input, |input, refs| self.segment_replace(input, refs));

        output
    }

    pub fn t2jp(&self, input: &str) -> String {
        let round_1 = [&self.dictionary.jp_variants];
        let dict_refs = DictRefs::new(&round_1);
        let output = dict_refs.apply_segment_replace(input, |input, refs| self.segment_replace(input, refs));

        output
    }

    pub fn jp2t(&self, input: &str) -> String {
        let round_1 = [
            &self.dictionary.jps_phrases,
            &self.dictionary.jps_characters,
            &self.dictionary.jp_variants_rev,
        ];
        let dict_refs = DictRefs::new(&round_1);
        let output = dict_refs.apply_segment_replace(input, |input, refs| self.segment_replace(input, refs));

        output
    }

    pub fn convert(&self, input: &str, config: &str, punctuation: bool) -> String {
        let result;

        match config.to_lowercase().as_str() {
            "s2t" => result = self.s2t(input, punctuation),
            "s2tw" => result = self.s2tw(input, punctuation),
            "s2twp" => result = self.s2twp(input, punctuation),
            "s2hk" => result = self.s2hk(input, punctuation),
            "t2s" => result = self.t2s(input, punctuation),
            "t2tw" => result = self.t2tw(input),
            "t2twp" => result = self.t2twp(input),
            "t2hk" => result = self.t2hk(input),
            "tw2s" => result = self.tw2s(input, punctuation),
            "tw2sp" => result = self.tw2sp(input, punctuation),
            "tw2t" => result = self.tw2t(input),
            "tw2tp" => result = self.tw2tp(input),
            "hk2s" => result = self.hk2s(input, punctuation),
            "hk2t" => result = self.hk2t(input),
            "jp2t" => result = self.jp2t(input),
            "t2jp" => result = self.t2jp(input),
            _ => {
                result = {
                    OpenCC::set_last_error(format!("Invalid config: {}", config).as_str());
                    String::new()
                }
            }
        }
        result
    }

    fn st(&self, input: &str) -> String {
        let dict_refs = [&self.dictionary.st_characters];
        let output = self.convert_by(input, &dict_refs, 1);

        output
    }

    fn ts(&self, input: &str) -> String {
        let dict_refs = [&self.dictionary.ts_characters];
        let output = self.convert_by(input, &dict_refs, 1);

        output
    }

    pub fn zho_check(&self, input: &str) -> i32 {
        if input.is_empty() {
            return 0;
        }
        // let re = Regex::new(r"[!-/:-@\[-`{-~\t\n\v\f\r 0-9A-Za-z_]").unwrap();
        let _strip_text = STRIP_REGEX.replace_all(input, "");
        let max_bytes = find_max_utf8_length(&_strip_text, 200);
        let strip_text = &_strip_text[..max_bytes];
        let code;
        if strip_text != &self.ts(strip_text) {
            code = 1;
        } else {
            if strip_text != &self.st(strip_text) {
                code = 2;
            } else {
                code = 0;
            }
        }
        code
    }

    fn convert_punctuation(sv: &str, config: &str) -> String {
        let mut s2t_punctuation_chars: HashMap<&str, &str> = HashMap::new();
        s2t_punctuation_chars.insert("“", "「");
        s2t_punctuation_chars.insert("”", "」");
        s2t_punctuation_chars.insert("‘", "『");
        s2t_punctuation_chars.insert("’", "』");

        let output_text;

        if config.starts_with('s') {
            let s2t_pattern = s2t_punctuation_chars.keys().cloned().collect::<String>();
            let s2t_regex = Regex::new(&format!("[{}]", s2t_pattern)).unwrap();
            output_text = s2t_regex
                .replace_all(sv, |caps: &regex::Captures| {
                    s2t_punctuation_chars[caps.get(0).unwrap().as_str()]
                })
                .into_owned();
        } else {
            let mut t2s_punctuation_chars: HashMap<&str, &str> = HashMap::new();
            for (key, value) in s2t_punctuation_chars.iter() {
                t2s_punctuation_chars.insert(value, key);
            }
            let t2s_pattern = t2s_punctuation_chars.keys().cloned().collect::<String>();
            let t2s_regex = Regex::new(&format!("[{}]", t2s_pattern)).unwrap();
            output_text = t2s_regex
                .replace_all(sv, |caps: &regex::Captures| {
                    t2s_punctuation_chars[caps.get(0).unwrap().as_str()]
                })
                .into_owned();
        }
        output_text
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


pub fn find_max_utf8_length(sv: &str, max_byte_count: usize) -> usize {
    // 1. No longer than max byte count
    if sv.len() <= max_byte_count {
        return sv.len();
    }
    // 2. Longer than byte count
    let mut byte_count = max_byte_count;
    while byte_count > 0 && (sv.as_bytes()[byte_count] & 0b11000000) == 0b10000000 {
        byte_count -= 1;
    }
    byte_count
}

pub fn format_thousand(n: usize) -> String {
    let mut result_str = n.to_string();
    let mut offset = result_str.len() % 3;
    if offset == 0 {
        offset = 3;
    }

    while offset < result_str.len() {
        result_str.insert(offset, ',');
        offset += 4; // Including the added comma
    }
    result_str
}
