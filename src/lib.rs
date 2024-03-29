use std::collections::{HashMap, HashSet};

use regex::Regex;

use crate::zho_dictionary::Dictionary;

mod zho_dictionary;

pub struct OpenCC {
    pub dictionary: Dictionary,
}

impl OpenCC {
    pub fn new() -> Self {
        let dictionary = Dictionary::new();

        OpenCC { dictionary }
    }

    pub fn segment_replace(
        text: &str,
        dictionaries: &[&(HashMap<String, String>, usize)],
    ) -> String {
        let mut max_word_length: usize = 1;
        for i in 0..dictionaries.len() {
            if max_word_length < dictionaries[i].1 {
                max_word_length = dictionaries[i].1;
            }
        }

        let split_string_list = Self::split_string_with_delimiters(text);

        Self::get_translated_string(split_string_list, dictionaries, max_word_length)
    }

    fn get_translated_string(
        split_string_list: Vec<(String, String)>,
        dictionaries: &[&(HashMap<String, String>, usize)],
        max_word_length: usize,
    ) -> String {
        let mut result = String::new();

        for (chunk, delimiter) in &split_string_list {
            let converted = Self::convert_by(chunk, dictionaries, max_word_length);
            result.push_str(&converted);
            result.push_str(delimiter);
        }

        result
    }

    fn convert_by(
        text: &str,
        dictionaries: &[&(HashMap<String, String>, usize)],
        max_word_length: usize,
    ) -> String {
        if text.is_empty() {
            return String::new();
        }

        let mut result = String::new();
        let text_chars: Vec<_> = text.chars().collect();
        let text_length = text_chars.len();

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
                        best_match = value.clone();
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

    fn split_string_with_delimiters(text: &str) -> Vec<(String, String)> {
        let delimiters: HashSet<char> = "\t\n\r(){}\"' -,.?!*　，。、；：？！…“”‘’『』「」﹁﹂—－（）《》〈〉～．／＼︒︑︔︓︿﹀︹︺︙︐［﹇］﹈︕︖︰︳︴︽︾︵︶｛︷｝︸﹃﹄【︻】︼".chars().collect();
        let mut split_string_list = Vec::new();
        let mut current_chunk = String::new();

        for ch in text.chars() {
            if delimiters.contains(&ch) {
                split_string_list.push((current_chunk, ch.to_string()));
                current_chunk = String::new();
            } else {
                current_chunk.push(ch);
            }
        }
        // Current_chunk still have chars but current_delimiter is empty
        if !current_chunk.is_empty() {
            split_string_list.push((current_chunk, String::new()));
        }
        split_string_list
    }

    #[allow(dead_code)]
    fn get_max_word_length(dictionary: &HashMap<&String, &String>) -> usize {
        dictionary
            .keys()
            .map(|word| word.chars().count())
            .max()
            .unwrap_or(1)
    }

    pub fn s2t(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [&self.dictionary.st_phrases, &self.dictionary.st_characters];
        let output = Self::segment_replace(input, &dict_refs);
        if punctuation {
            Self::convert_punctuation(&output, "s")
        } else {
            output
        }
    }

    pub fn t2s(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [&self.dictionary.ts_phrases, &self.dictionary.ts_characters];
        let output = Self::segment_replace(input, &dict_refs);
        if punctuation {
            Self::convert_punctuation(&output, "t")
        } else {
            output
        }
    }

    pub fn s2tw(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [&self.dictionary.st_phrases, &self.dictionary.st_characters];
        let dict_refs_round_2 = [&self.dictionary.tw_variants];
        let output = Self::segment_replace(input, &dict_refs);
        let output_2 = Self::segment_replace(&output, &dict_refs_round_2);
        if punctuation {
            Self::convert_punctuation(&output_2, "s")
        } else {
            output_2
        }
    }

    pub fn tw2s(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [
            &self.dictionary.tw_variants_rev_phrases,
            &self.dictionary.tw_variants_rev,
        ];
        let dict_refs_round_2 = [&self.dictionary.ts_phrases, &self.dictionary.ts_characters];
        let output = Self::segment_replace(input, &dict_refs);
        let output_2 = Self::segment_replace(&output, &dict_refs_round_2);
        if punctuation {
            Self::convert_punctuation(&output_2, "t")
        } else {
            output_2
        }
    }

    pub fn s2twp(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [&self.dictionary.st_phrases, &self.dictionary.st_characters];
        let dict_refs_round_2 = [&self.dictionary.tw_phrases];
        let dict_refs_round_3 = [&self.dictionary.tw_variants];
        let output = Self::segment_replace(input, &dict_refs);
        let output_2 = Self::segment_replace(&output, &dict_refs_round_2);
        let output_3 = Self::segment_replace(&output_2, &dict_refs_round_3);
        if punctuation {
            Self::convert_punctuation(&output_3, "s")
        } else {
            output_3
        }
    }

    pub fn tw2sp(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [
            &self.dictionary.tw_variants_rev_phrases,
            &self.dictionary.tw_variants_rev,
        ];
        let dict_refs_round_2 = [&self.dictionary.tw_phrases_rev];
        let dict_refs_round_3 = [&self.dictionary.ts_phrases, &self.dictionary.ts_characters];
        let output = Self::segment_replace(input, &dict_refs);
        let output_2 = Self::segment_replace(&output, &dict_refs_round_2);
        let output_3 = Self::segment_replace(&output_2, &dict_refs_round_3);
        if punctuation {
            Self::convert_punctuation(&output_3, "t")
        } else {
            output_3
        }
    }

    pub fn s2hk(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [&self.dictionary.st_phrases, &self.dictionary.st_characters];
        let dict_refs_round_2 = [&self.dictionary.hk_variants];
        let output = Self::segment_replace(input, &dict_refs);
        let output_2 = Self::segment_replace(&output, &dict_refs_round_2);
        if punctuation {
            Self::convert_punctuation(&output_2, "s")
        } else {
            output_2
        }
    }

    pub fn hk2s(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [
            &self.dictionary.hk_variants_rev_phrases,
            &self.dictionary.hk_variants_rev,
        ];
        let dict_refs_round_2 = [&self.dictionary.ts_phrases, &self.dictionary.ts_characters];
        let output = Self::segment_replace(input, &dict_refs);
        let output_2 = Self::segment_replace(&output, &dict_refs_round_2);
        if punctuation {
            Self::convert_punctuation(&output_2, "t")
        } else {
            output_2
        }
    }

    pub fn t2tw(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [&self.dictionary.tw_variants];
        let output = Self::segment_replace(input, &dict_refs);
        if punctuation {
            Self::convert_punctuation(&output, "s")
        } else {
            output
        }
    }

    pub fn t2twp(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [&self.dictionary.tw_phrases];
        let dict_refs_round_2 = [&self.dictionary.tw_variants];
        let output = Self::segment_replace(input, &dict_refs);
        let output_2 = Self::segment_replace(&output, &dict_refs_round_2);
        if punctuation {
            Self::convert_punctuation(&output_2, "s")
        } else {
            output_2
        }
    }

    pub fn tw2t(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [
            &self.dictionary.tw_variants_rev_phrases,
            &self.dictionary.tw_variants_rev,
        ];
        let output = Self::segment_replace(input, &dict_refs);
        if punctuation {
            Self::convert_punctuation(&output, "s")
        } else {
            output
        }
    }

    pub fn tw2tp(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [
            &self.dictionary.tw_variants_rev_phrases,
            &self.dictionary.tw_variants_rev,
        ];
        let dict_refs_round_2 = [&self.dictionary.tw_phrases_rev];
        let output = Self::segment_replace(input, &dict_refs);
        let output_2 = Self::segment_replace(&output, &dict_refs_round_2);
        if punctuation {
            Self::convert_punctuation(&output_2, "s")
        } else {
            output_2
        }
    }

    pub fn t2hk(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [&self.dictionary.hk_variants];
        let output = Self::segment_replace(input, &dict_refs);
        if punctuation {
            Self::convert_punctuation(&output, "s")
        } else {
            output
        }
    }

    pub fn hk2t(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [
            &self.dictionary.hk_variants_rev_phrases,
            &self.dictionary.hk_variants_rev,
        ];
        let output = Self::segment_replace(input, &dict_refs);
        if punctuation {
            Self::convert_punctuation(&output, "s")
        } else {
            output
        }
    }

    pub fn t2jp(&self, input: &str) -> String {
        let dict_refs = [&self.dictionary.jp_variants];
        let output = Self::segment_replace(input, &dict_refs);

        output
    }

    pub fn jp2t(&self, input: &str) -> String {
        let dict_refs = [
            &self.dictionary.jps_phrases,
            &self.dictionary.jps_characters,
            &self.dictionary.jp_variants_rev,
        ];
        let output = Self::segment_replace(input, &dict_refs);

        output
    }

    fn st(&self, input: &str) -> String {
        let dict_refs = [&self.dictionary.st_characters];
        let output = Self::segment_replace(input, &dict_refs);

        output
    }

    fn ts(&self, input: &str) -> String {
        let dict_refs = [&self.dictionary.ts_characters];
        let output = Self::segment_replace(input, &dict_refs);

        output
    }

    pub fn zho_check(&self, input: &str) -> i8 {
        if input.is_empty() {
            return 0;
        }
        // let re = Regex::new(r"[[:punct:][:space:][:word:]]").unwrap();
        let re = Regex::new(r"[!-/:-@\[-`{-~\t\n\v\f\r 0-9A-Za-z_]").unwrap();
        let _strip_text = re.replace_all(input, "");
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
