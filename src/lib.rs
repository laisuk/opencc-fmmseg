use lazy_static::lazy_static;
use std::collections::{HashMap, HashSet};
use std::iter::Iterator;
use std::sync::{Arc, Mutex};

use rayon::prelude::*;
use regex::Regex;

use crate::dictionary_lib::DictionaryMaxlength;
pub mod dictionary_lib;
// Define a global mutable variable to store the error message
static LAST_ERROR: Mutex<Option<String>> = Mutex::new(None);
// const DELIMITERS0: &'static str = "\t\n\r (){}[]<>\"'\\/|-,.?!*:;@#$%^&_+=　，。、；：？！…“”‘’『』「」﹁﹂—－（）《》〈〉～．／＼︒︑︔︓︿﹀︹︺︙︐［﹇］﹈︕︖︰︳︴︽︾︵︶｛︷｝︸﹃﹄【︻】︼";
const DELIMITERS: &'static str = " \t\n\r!\"#$%&'()*+,-./:;<=>?@[\\]^_{}|~＝、。“”‘’『』「」﹁﹂—－（）《》〈〉？！…／＼︒︑︔︓︿﹀︹︺︙︐［﹇］﹈︕︖︰︳︴︽︾︵︶｛︷｝︸﹃﹄【︻】︼　～．，；：";

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
            let split_string_list = self.split_string_inclusive_par(text);
            self.get_translated_string_par(split_string_list, dictionaries, max_word_length)
        } else {
            let split_string_list = self.split_string_inclusive(text);
            self.get_translated_string(split_string_list, dictionaries, max_word_length)
        }
    }

    fn get_translated_string(
        &self,
        split_string_list: Vec<Vec<char>>,
        dictionaries: &[&(HashMap<String, String>, usize)],
        max_word_length: usize,
    ) -> String {
        split_string_list
            .iter()
            .map(|chunk| self.convert_by(&chunk, dictionaries, max_word_length))
            .collect::<String>()
    }

    fn get_translated_string_par(
        &self,
        split_string_list: Vec<Arc<[char]>>,
        dictionaries: &[&(HashMap<String, String>, usize)],
        max_word_length: usize,
    ) -> String {
        split_string_list
            .par_iter()
            .map(|chunk| self.convert_by(&chunk, dictionaries, max_word_length))
            .collect::<String>()
    }

    fn convert_by(
        &self,
        text_chars: &[char],
        dictionaries: &[&(HashMap<String, String>, usize)],
        max_word_length: usize,
    ) -> String {
        if text_chars.is_empty() {
            return String::new();
        }

        let text_length = text_chars.len();
        if text_length == 1 && self.delimiters.contains(&text_chars[0]) {
            return text_chars[0].to_string();
        }

        let mut result = String::new();
        result.reserve(text_chars.len() * 4);

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

    // fn split_string_inclusive(&self, text: &str) -> Vec<String> {
    //     let mut split_string_list = Vec::new();
    //     let mut current_chunk = String::new();
    //
    //     for ch in text.chars() {
    //         if self.delimiters.contains(&ch) {
    //             split_string_list.push(current_chunk + &ch.to_string());
    //             current_chunk = String::new();
    //         } else {
    //             current_chunk.push(ch);
    //         }
    //     }
    //     // Current_chunk still have chars but current_delimiter is empty
    //     if !current_chunk.is_empty() {
    //         split_string_list.push(current_chunk);
    //     }
    //
    //     split_string_list
    // }

    fn split_string_inclusive(&self, text: &str) -> Vec<Vec<char>> {
        let mut split_string_list = Vec::new();
        let mut current_chunk = Vec::new();

        for ch in text.chars() {
            current_chunk.push(ch);

            // Check if the current character is a delimiter
            if self.delimiters.contains(&ch) {
                split_string_list.push(current_chunk.clone());
                current_chunk.clear(); // Clear current chunk for the next segment
            }
        }

        // Push any remaining characters as the last chunk
        if !current_chunk.is_empty() {
            split_string_list.push(current_chunk);
        }

        split_string_list
    }

    // fn split_string_inclusive_par(&self, text: &str) -> Vec<Vec<char>> {
    //     let split_string_list: Vec<Vec<char>> = text
    //         .par_chars()
    //         .collect::<Vec<char>>() // Collect into Vec<char> to allow splitting
    //         .par_split_inclusive_mut(|c| self.delimiters.contains(c))
    //         .map(|slice| slice.par_iter().cloned().collect())
    //         .collect();
    //
    //     split_string_list
    // }

    fn split_string_inclusive_par(&self, text: &str) -> Vec<Arc<[char]>> {
        let collected: Vec<char> = text.par_chars().collect();
        collected
            .par_split_inclusive(|c| self.delimiters.contains(c))
            .map(|slice| Arc::from(slice)) // Convert each slice to Arc<[char]>
            .collect()
    }

    pub fn get_parallel(&self) -> bool {
        self.is_parallel
    }

    pub fn set_parallel(&mut self, is_parallel: bool) -> () {
        self.is_parallel = is_parallel;
    }

    pub fn s2t(&self, input: &str, punctuation: bool) -> String {
        let output = DictRefs::new(&[&self.dictionary.st_phrases, &self.dictionary.st_characters])
            .apply_segment_replace(input, |input, refs| self.segment_replace(input, refs));

        if punctuation {
            Self::convert_punctuation(&output, "s")
        } else {
            output
        }
    }

    pub fn t2s(&self, input: &str, punctuation: bool) -> String {
        let output = DictRefs::new(&[&self.dictionary.ts_phrases, &self.dictionary.ts_characters])
            .apply_segment_replace(input, |input, refs| self.segment_replace(input, refs));
        if punctuation {
            Self::convert_punctuation(&output, "t")
        } else {
            output
        }
    }

    pub fn s2tw(&self, input: &str, punctuation: bool) -> String {
        let output = DictRefs::new(&[&self.dictionary.st_phrases, &self.dictionary.st_characters])
            .with_round_2(&[&self.dictionary.tw_variants])
            .apply_segment_replace(input, |input, refs| self.segment_replace(input, refs));

        if punctuation {
            Self::convert_punctuation(&output, "s")
        } else {
            output
        }
    }

    pub fn tw2s(&self, input: &str, punctuation: bool) -> String {
        let output = DictRefs::new(&[
            &self.dictionary.tw_variants_rev_phrases,
            &self.dictionary.tw_variants_rev,
        ])
        .with_round_2(&[&self.dictionary.ts_phrases, &self.dictionary.ts_characters])
        .apply_segment_replace(input, |input, refs| self.segment_replace(input, refs));
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
        let output = DictRefs::new(&round_1)
            .with_round_2(&round_2)
            .with_round_3(&round_3)
            .apply_segment_replace(input, |input, refs| self.segment_replace(input, refs));
        // Handle punctuation if needed
        if punctuation {
            Self::convert_punctuation(&output, "s")
        } else {
            output
        }
    }

    pub fn tw2sp(&self, input: &str, punctuation: bool) -> String {
        let round_1 = [
            &self.dictionary.tw_phrases_rev,
            &self.dictionary.tw_variants_rev_phrases,
            &self.dictionary.tw_variants_rev,
        ];
        let round_2 = [&self.dictionary.ts_phrases, &self.dictionary.ts_characters];
        let output = DictRefs::new(&round_1)
            .with_round_2(&round_2)
            .apply_segment_replace(input, |input, refs| self.segment_replace(input, refs));
        if punctuation {
            Self::convert_punctuation(&output, "t")
        } else {
            output
        }
    }

    pub fn s2hk(&self, input: &str, punctuation: bool) -> String {
        let round_1 = [&self.dictionary.st_phrases, &self.dictionary.st_characters];
        let round_2 = [&self.dictionary.hk_variants];
        let output = DictRefs::new(&round_1)
            .with_round_2(&round_2)
            .apply_segment_replace(input, |input, refs| self.segment_replace(input, refs));
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
        let output = DictRefs::new(&round_1)
            .with_round_2(&round_2)
            .apply_segment_replace(input, |input, refs| self.segment_replace(input, refs));
        if punctuation {
            Self::convert_punctuation(&output, "t")
        } else {
            output
        }
    }

    pub fn t2tw(&self, input: &str) -> String {
        let round_1 = [&self.dictionary.tw_variants];
        let output = DictRefs::new(&round_1)
            .apply_segment_replace(input, |input, refs| self.segment_replace(input, refs));

        output
    }

    pub fn t2twp(&self, input: &str) -> String {
        let round_1 = [&self.dictionary.tw_phrases];
        let round_2 = [&self.dictionary.tw_variants];
        let output = DictRefs::new(&round_1)
            .with_round_2(&round_2)
            .apply_segment_replace(input, |input, refs| self.segment_replace(input, refs));

        output
    }

    pub fn tw2t(&self, input: &str) -> String {
        let round_1 = [
            &self.dictionary.tw_variants_rev_phrases,
            &self.dictionary.tw_variants_rev,
        ];
        let output = DictRefs::new(&round_1)
            .apply_segment_replace(input, |input, refs| self.segment_replace(input, refs));

        output
    }

    pub fn tw2tp(&self, input: &str) -> String {
        let round_1 = [
            &self.dictionary.tw_variants_rev_phrases,
            &self.dictionary.tw_variants_rev,
        ];
        let round_2 = [&self.dictionary.tw_phrases_rev];
        let output = DictRefs::new(&round_1)
            .with_round_2(&round_2)
            .apply_segment_replace(input, |input, refs| self.segment_replace(input, refs));

        output
    }

    pub fn t2hk(&self, input: &str) -> String {
        let round_1 = [&self.dictionary.hk_variants];
        let output = DictRefs::new(&round_1)
            .apply_segment_replace(input, |input, refs| self.segment_replace(input, refs));

        output
    }

    pub fn hk2t(&self, input: &str) -> String {
        let round_1 = [
            &self.dictionary.hk_variants_rev_phrases,
            &self.dictionary.hk_variants_rev,
        ];
        let output = DictRefs::new(&round_1)
            .apply_segment_replace(input, |input, refs| self.segment_replace(input, refs));

        output
    }

    pub fn t2jp(&self, input: &str) -> String {
        let round_1 = [&self.dictionary.jp_variants];
        let output = DictRefs::new(&round_1)
            .apply_segment_replace(input, |input, refs| self.segment_replace(input, refs));

        output
    }

    pub fn jp2t(&self, input: &str) -> String {
        let round_1 = [
            &self.dictionary.jps_phrases,
            &self.dictionary.jps_characters,
            &self.dictionary.jp_variants_rev,
        ];
        let output = DictRefs::new(&round_1)
            .apply_segment_replace(input, |input, refs| self.segment_replace(input, refs));

        output
    }

    pub fn convert(&self, input: &str, config: &str, punctuation: bool) -> String {
        match config.to_lowercase().as_str() {
            "s2t" => self.s2t(input, punctuation),
            "s2tw" => self.s2tw(input, punctuation),
            "s2twp" => self.s2twp(input, punctuation),
            "s2hk" => self.s2hk(input, punctuation),
            "t2s" => self.t2s(input, punctuation),
            "t2tw" => self.t2tw(input),
            "t2twp" => self.t2twp(input),
            "t2hk" => self.t2hk(input),
            "tw2s" => self.tw2s(input, punctuation),
            "tw2sp" => self.tw2sp(input, punctuation),
            "tw2t" => self.tw2t(input),
            "tw2tp" => self.tw2tp(input),
            "hk2s" => self.hk2s(input, punctuation),
            "hk2t" => self.hk2t(input),
            "jp2t" => self.jp2t(input),
            "t2jp" => self.t2jp(input),
            _ => {
                Self::set_last_error(format!("Invalid config: {}", config).as_str());
                String::new()
            }
        }
    }

    fn st(&self, input: &str) -> String {
        let dict_refs = [&self.dictionary.st_characters];
        let chars: Vec<char> = input.par_chars().collect();
        self.convert_by(&chars, &dict_refs, 1)
    }

    fn ts(&self, input: &str) -> String {
        let dict_refs = [&self.dictionary.ts_characters];
        let chars: Vec<char> = input.par_chars().collect();
        self.convert_by(&chars, &dict_refs, 1)
    }

    pub fn zho_check(&self, input: &str) -> i32 {
        if input.is_empty() {
            return 0;
        }
        // let re = Regex::new(r"[!-/:-@\[-`{-~\t\n\v\f\r 0-9A-Za-z_]").unwrap();
        let _strip_text = STRIP_REGEX.replace_all(input, "");
        let max_bytes = find_max_utf8_length(&_strip_text, 200);
        let strip_text = &_strip_text[..max_bytes];

        match (
            strip_text != &self.ts(strip_text),
            strip_text != &self.st(strip_text),
        ) {
            (true, _) => 1,
            (_, true) => 2,
            _ => 0,
        }
    }

    fn convert_punctuation(text: &str, config: &str) -> String {
        let s2t_punctuation_chars: HashMap<&str, &str> = HashMap::from([
            ("“", "「"),
            ("”", "」"),
            ("‘", "『"),
            ("’", "』"),
        ]);

        let mapping = if config.starts_with('s') {
            &s2t_punctuation_chars
        } else {
            // Correctly create a new HashMap with reversed key-value pairs
            &s2t_punctuation_chars
                .iter()
                .map(|(&k, &v)| (v, k))
                .collect::<HashMap<&str, &str>>()
        };

        // let pattern = format!("[{}]", mapping.keys().cloned().collect::<String>());
        let pattern = mapping.keys().map(|k| regex::escape(k)).collect::<Vec<_>>().join("|");
        let regex = Regex::new(&pattern).unwrap();

        regex
            .replace_all(text, |caps: &regex::Captures| {
                mapping[caps.get(0).unwrap().as_str()]
            })
            .into_owned()
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

// #[derive(Clone, Copy)]
pub struct DictRefs<'a> {
    round_1: &'a [&'a (HashMap<String, String>, usize)],
    round_2: Option<&'a [&'a (HashMap<String, String>, usize)]>,
    round_3: Option<&'a [&'a (HashMap<String, String>, usize)]>,
}

impl<'a> DictRefs<'a> {
    pub fn new(round_1: &'a [&'a (HashMap<String, String>, usize)]) -> Self {
        DictRefs {
            round_1,
            round_2: None,
            round_3: None,
        }
    }
    pub fn with_round_2(mut self, round_2: &'a [&'a (HashMap<String, String>, usize)]) -> Self {
        self.round_2 = Some(round_2);
        self
    }

    pub fn with_round_3(mut self, round_3: &'a [&'a (HashMap<String, String>, usize)]) -> Self {
        self.round_3 = Some(round_3);
        self
    }

    pub fn apply_segment_replace<F>(&self, input: &str, segment_replace: F) -> String
    where
        F: Fn(&str, &[&(HashMap<String, String>, usize)]) -> String,
    {
        let mut output = segment_replace(input, self.round_1);
        if let Some(refs) = self.round_2 {
            output = segment_replace(&output, refs);
        }
        if let Some(refs) = self.round_3 {
            output = segment_replace(&output, refs);
        }
        output
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
