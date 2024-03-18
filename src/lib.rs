use std::cmp::max;
use std::collections::HashMap;

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

    /// Segments the input text using the given dictionaries, and replaces each segment with the
    /// corresponding value in the dictionaries.
    ///
    /// # Parameters
    ///
    /// * `text` - The input text to be segmented.
    /// * `dictionaries` - A slice of dictionaries, where each dictionary maps a segment to its
    /// replacement value.
    ///
    /// # Returns
    ///
    /// A vector of strings, where each string represents the replacement value for a segment in the
    /// input text.
    pub fn segment_replace(
        text: &str,
        dictionaries: &[&HashMap<String, String>],
        &max_word_length: &usize,
    ) -> String {
        let mut test_dictionary = HashMap::new();
        for i in 0..dictionaries.len() {
            test_dictionary.extend(dictionaries[i]);
        }

        let split_string_list = Self::split_string_with_delimiters(text);

        let translated_split_string =
            Self::get_translated_string(split_string_list, &test_dictionary, max_word_length);

        translated_split_string
    }
    pub fn segment_replace_no_max_length(
        text: &str,
        dictionaries: &[&HashMap<String, String>],
    ) -> String {
        let mut test_dictionary = HashMap::new();
        for i in 0..dictionaries.len() {
            test_dictionary.extend(dictionaries[i]);
        }
        let max_word_length = Self::get_max_word_length(&test_dictionary);

        let split_string_list = Self::split_string_with_delimiters(text);

        let translated_string =
            Self::get_translated_string(split_string_list, &test_dictionary, max_word_length);

        translated_string
    }

    pub fn segment_replace_no_split(
        text: &str,
        dictionaries: &[&HashMap<String, String>],
    ) -> String {
        let mut test_dictionary = HashMap::new();
        for i in 0..dictionaries.len() {
            test_dictionary.extend(dictionaries[i]);
        }
        let max_word_length = test_dictionary
            .keys()
            .map(|word| word.chars().count())
            .max()
            .unwrap_or(1);

        Self::convert_by(text, &test_dictionary, max_word_length)
    }

    fn convert_by(
        text: &str,
        dictionary: &HashMap<&String, &String>,
        max_word_length: usize,
    ) -> String {
        let mut result = String::new();
        let text_chars: Vec<_> = text.chars().collect();
        let text_length = text_chars.len();
        // let max_word_length = dictionary.keys().map(|word| word.chars().count()).max().unwrap_or(1);

        let mut start_pos = 0;
        while start_pos < text_length {
            let max_length = std::cmp::min(max_word_length, text_length - start_pos);
            let mut best_match_length = 0;
            let mut best_match = String::new();

            for length in 1..=max_length {
                let candidate: String =
                    text_chars[start_pos..(start_pos + length)].iter().collect();
                if let Some(value) = dictionary.get(&candidate) {
                    best_match_length = length;
                    best_match = value.to_string(); // Push the corresponding value to the results
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
        let delimiters: Vec<char> = "(){}\"' -,.?!*　，。、；：？！…“”‘’『』「」﹁﹂—－（）《》〈〉～．／＼︒︑︔︓︿﹀︹︺︙︐［﹇］﹈︕︖︰︳︴︽︾︵︶｛︷｝︸﹃﹄【︻】︼".chars().collect();
        let mut split_string_list = Vec::new();
        let mut current_chunk = String::new();
        let mut current_delimiter = String::new(); // Default delimiter

        for char in text.chars() {
            if delimiters.contains(&char) {
                current_delimiter = char.to_string();
                split_string_list.push((current_chunk.clone(), current_delimiter));
                current_delimiter = "".to_string();
                current_chunk.clear();
            } else {
                current_chunk.push(char);
            }
        }
        // Current_chunk still have chars but current_delimiter is empty
        if !current_chunk.is_empty() {
            split_string_list.push((current_chunk, current_delimiter));
        }
        split_string_list
    }

    fn get_max_word_length(dictionary: &HashMap<&String, &String>) -> usize {
        dictionary
            .keys()
            .map(|word| word.chars().count())
            .max()
            .unwrap_or(1)
    }

    fn get_translated_string(
        split_string_list: Vec<(String, String)>,
        dictionary: &HashMap<&String, &String>,
        max_word_length: usize,
    ) -> String {
        split_string_list
            .into_iter()
            .map(|(chunk, delimiter)| {
                format!(
                    "{}{}",
                    Self::convert_by(&chunk, &dictionary, max_word_length),
                    delimiter
                )
            })
            .collect::<Vec<_>>()
            .join("")
    }

    pub fn s2t(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [&self.dictionary.st_phrases, &self.dictionary.st_characters];
        let max_word_length = max(
            &self.dictionary.st_phrases_max_length,
            &self.dictionary.st_characters_max_length,
        );

        let output = Self::segment_replace(input, &dict_refs, max_word_length);
        if punctuation {
            convert_punctuation(&output, "s")
        } else {
            output
        }
    }

    pub fn t2s(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [&self.dictionary.ts_phrases, &self.dictionary.ts_characters];
        let max_word_length = max(
            &self.dictionary.ts_phrases_max_length,
            &self.dictionary.ts_characters_max_length,
        );
        let output = Self::segment_replace(input, &dict_refs, max_word_length);
        if punctuation {
            convert_punctuation(&output, "t")
        } else {
            output
        }
    }

    pub fn s2tw(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [&self.dictionary.st_phrases, &self.dictionary.st_characters];
        let max_word_length = max(
            &self.dictionary.st_phrases_max_length,
            &self.dictionary.st_characters_max_length,
        );
        let dict_refs_round_2 = [&self.dictionary.tw_variants];
        let max_word_length_2 = &self.dictionary.tw_variants_max_length;
        let output = Self::segment_replace(input, &dict_refs, max_word_length);
        let output_2 = Self::segment_replace(&output, &dict_refs_round_2, max_word_length_2);
        if punctuation {
            convert_punctuation(&output_2, "s")
        } else {
            output_2
        }
    }

    pub fn tw2s(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [
            &self.dictionary.tw_variants_rev_phrases,
            &self.dictionary.tw_variants_rev,
        ];
        let max_word_length = max(
            &self.dictionary.tw_variants_rev_phrases_max_length,
            &self.dictionary.tw_variants_rev_max_length,
        );

        let dict_refs_round_2 = [&self.dictionary.ts_phrases, &self.dictionary.ts_characters];
        let max_word_length_2 = max(
            &self.dictionary.ts_phrases_max_length,
            &self.dictionary.ts_characters_max_length,
        );

        let output = Self::segment_replace(input, &dict_refs, max_word_length);
        let output_2 = Self::segment_replace(&output, &dict_refs_round_2, max_word_length_2);
        if punctuation {
            convert_punctuation(&output_2, "t")
        } else {
            output_2
        }
    }

    pub fn s2twp(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [&self.dictionary.st_phrases, &self.dictionary.st_characters];
        let max_word_length = max(
            &self.dictionary.st_phrases_max_length,
            &self.dictionary.st_characters_max_length,
        );
        let dict_refs_round_2 = [&self.dictionary.tw_phrases];
        let max_word_length_2 = &self.dictionary.tw_phrases_max_length;
        let dict_refs_round_3 = [&self.dictionary.tw_variants];
        let max_word_length_3 = &self.dictionary.tw_variants_max_length;
        let output = Self::segment_replace(input, &dict_refs, max_word_length);
        let output_2 = Self::segment_replace(&output, &dict_refs_round_2, max_word_length_2);
        let output_3 = Self::segment_replace(&output_2, &dict_refs_round_3, max_word_length_3);
        if punctuation {
            convert_punctuation(&output_3, "s")
        } else {
            output_3
        }
    }

    pub fn tw2sp(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [
            &self.dictionary.tw_variants_rev_phrases,
            &self.dictionary.tw_variants_rev,
        ];
        let max_word_length = max(
            &self.dictionary.tw_variants_rev_phrases_max_length,
            &self.dictionary.tw_variants_rev_max_length,
        );
        let dict_refs_round_2 = [&self.dictionary.tw_phrases_rev];
        let max_word_length_2 = &self.dictionary.tw_phrases_rev_max_length;
        let dict_refs_round_3 = [&self.dictionary.ts_phrases, &self.dictionary.ts_characters];
        let max_word_length_3 = max(
            &self.dictionary.ts_phrases_max_length,
            &self.dictionary.ts_characters_max_length,
        );
        let output = Self::segment_replace(input, &dict_refs, max_word_length);
        let output_2 = Self::segment_replace(&output, &dict_refs_round_2, max_word_length_2);
        let output_3 = Self::segment_replace(&output_2, &dict_refs_round_3, max_word_length_3);
        if punctuation {
            convert_punctuation(&output_3, "t")
        } else {
            output_3
        }
    }

    pub fn s2hk(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [&self.dictionary.st_phrases, &self.dictionary.st_characters];
        let max_word_length = max(
            &self.dictionary.st_phrases_max_length,
            &self.dictionary.st_characters_max_length,
        );
        let dict_refs_round_2 = [&self.dictionary.hk_variants];
        let max_word_length_2 = &self.dictionary.hk_variants_max_length;
        let output = Self::segment_replace(input, &dict_refs, max_word_length);
        let output_2 = Self::segment_replace(&output, &dict_refs_round_2, max_word_length_2);
        if punctuation {
            convert_punctuation(&output_2, "s")
        } else {
            output_2
        }
    }

    pub fn hk2s(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [
            &self.dictionary.hk_variants_rev_phrases,
            &self.dictionary.hk_variants_rev,
        ];
        let max_word_length = max(
            &self.dictionary.hk_variants_rev_phrases_max_length,
            &self.dictionary.hk_variants_rev_max_length,
        );
        let dict_refs_round_2 = [&self.dictionary.ts_phrases, &self.dictionary.ts_characters];
        let max_word_length_2 = max(
            &self.dictionary.ts_phrases_max_length,
            &self.dictionary.ts_characters_max_length,
        );
        let output = Self::segment_replace(input, &dict_refs, max_word_length);
        let output_2 = Self::segment_replace(&output, &dict_refs_round_2, max_word_length_2);
        if punctuation {
            convert_punctuation(&output_2, "t")
        } else {
            output_2
        }
    }

    pub fn t2tw(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [&self.dictionary.tw_variants];
        let max_word_length = &self.dictionary.tw_variants_max_length;
        let output = Self::segment_replace(input, &dict_refs, max_word_length);
        if punctuation {
            convert_punctuation(&output, "s")
        } else {
            output
        }
    }

    pub fn t2twp(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [&self.dictionary.tw_phrases];
        let max_word_length = &self.dictionary.tw_phrases_max_length;
        let dict_refs_round_2 = [&self.dictionary.tw_variants];
        let max_word_length_2 = &self.dictionary.tw_variants_max_length;
        let output = Self::segment_replace(input, &dict_refs, max_word_length);
        let output_2 = Self::segment_replace(&output, &dict_refs_round_2, max_word_length_2);
        if punctuation {
            convert_punctuation(&output_2, "s")
        } else {
            output_2
        }
    }

    pub fn tw2t(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [
            &self.dictionary.tw_variants_rev_phrases,
            &self.dictionary.tw_variants_rev,
        ];
        let max_word_length = max(
            &self.dictionary.tw_variants_rev_phrases_max_length,
            &self.dictionary.tw_variants_rev_max_length,
        );
        let output = Self::segment_replace(input, &dict_refs, max_word_length);
        if punctuation {
            convert_punctuation(&output, "s")
        } else {
            output
        }
    }

    pub fn tw2tp(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [
            &self.dictionary.tw_variants_rev_phrases,
            &self.dictionary.tw_variants_rev,
        ];
        let max_word_length = max(
            &self.dictionary.tw_variants_rev_phrases_max_length,
            &self.dictionary.tw_variants_rev_max_length,
        );
        let dict_refs_round_2 = [&self.dictionary.tw_phrases_rev];
        let max_word_length_2 = &self.dictionary.tw_phrases_rev_max_length;
        let output = Self::segment_replace(input, &dict_refs, max_word_length);
        let output_2 = Self::segment_replace(&output, &dict_refs_round_2, max_word_length_2);
        if punctuation {
            convert_punctuation(&output_2, "s")
        } else {
            output_2
        }
    }

    pub fn t2hk(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [&self.dictionary.hk_variants];
        let max_word_length = &self.dictionary.hk_variants_max_length;
        let output = Self::segment_replace(input, &dict_refs, max_word_length);
        if punctuation {
            convert_punctuation(&output, "s")
        } else {
            output
        }
    }

    pub fn hk2t(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [
            &self.dictionary.hk_variants_rev_phrases,
            &self.dictionary.hk_variants_rev,
        ];
        let max_word_length = max(
            &self.dictionary.hk_variants_rev_phrases_max_length,
            &self.dictionary.hk_variants_rev_max_length,
        );
        let output = Self::segment_replace(input, &dict_refs, max_word_length);
        if punctuation {
            convert_punctuation(&output, "s")
        } else {
            output
        }
    }

    pub fn t2jp(&self, input: &str) -> String {
        let dict_refs = [&self.dictionary.jp_variants];
        let max_word_length = &self.dictionary.jp_variants_max_length;
        let output = Self::segment_replace(input, &dict_refs, max_word_length);

        output
    }

    pub fn jp2t(&self, input: &str) -> String {
        let dict_refs = [
            &self.dictionary.jps_phrases,
            &self.dictionary.jps_characters,
            &self.dictionary.jp_variants_rev,
        ];
        let max_word_length = max(
            &self.dictionary.jps_phrases_max_length,
            max(
                &self.dictionary.jps_characters_max_length,
                &self.dictionary.jp_variants_rev_max_length,
            ),
        );
        let output = Self::segment_replace(input, &dict_refs, max_word_length);

        output
    }
}

pub fn zho_check(input: &str) -> i8 {
    if input.is_empty() {
        return 0;
    }
    let re = Regex::new(r"[[:punct:]\sA-Za-z0-9]").unwrap();
    let _strip_text = re.replace_all(input, "");
    let max_bytes = find_max_utf8_length(_strip_text.as_ref(), 200);
    let strip_text = match _strip_text.len() > max_bytes {
        true => &_strip_text[..max_bytes],
        false => &_strip_text,
    };
    let opencc = OpenCC::new();
    let code;
    if strip_text != opencc.t2s(strip_text, false) {
        code = 1;
    } else {
        if strip_text != opencc.s2t(strip_text, false) {
            code = 2;
        } else {
            code = 0;
        }
    }
    code
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

pub fn convert_punctuation(sv: &str, config: &str) -> String {
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

pub fn format_thousand(n: i32) -> String {
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
