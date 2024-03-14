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

    pub fn segment_replace(text: &str, dictionaries: &[&HashMap<String, String>]) -> Vec<String> {
        let mut result = Vec::new();
        let mut text_chars: Vec<_> = text.chars().collect();
        let text_length = text_chars.len();

        for dictionary in dictionaries {
            result.clear()
            ;
            let max_word_length = dictionary.keys().map(|word| word.chars().count()).max().unwrap_or(1);

            let mut start_pos = 0;
            while start_pos < text_length {
                let max_length = std::cmp::min(max_word_length, text_length - start_pos);
                let mut best_match_length = 0;
                let mut best_match = String::new();

                for length in 1..=max_length {
                    let candidate: String = text_chars[start_pos..(start_pos + length)].iter().collect();
                    if let Some(value) = dictionary.get(&candidate) {
                        best_match_length = length;
                        best_match = value.clone(); // Push the corresponding value to the results
                    }
                }

                if best_match_length == 0 {
                    // If no match found, treat the character as a single word
                    best_match_length = 1;
                    best_match = text_chars[start_pos].to_string();
                }

                result.push(best_match.clone());
                start_pos += best_match_length;
            }
            text_chars = result.join("").chars().collect();
        }

        result
    }

    pub fn s2t(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [
            &self.dictionary.st_phrases,
            &self.dictionary.st_characters
        ];
        let output = Self::segment_replace(input, &dict_refs);
        if punctuation {
            convert_punctuation(output.join("").as_str(), "s")
        } else {
            output.join("")
        }
    }

    pub fn t2s(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [
            &self.dictionary.ts_phrases,
            &self.dictionary.ts_characters
        ];
        let output = Self::segment_replace(input, &dict_refs);
        if punctuation {
            convert_punctuation(output.join("").as_str(), "t")
        } else {
            output.join("")
        }
    }

    pub fn s2tw(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [
            &self.dictionary.st_phrases,
            &self.dictionary.st_characters
        ];
        let dict_refs_round_2 = [
            &self.dictionary.tw_variants
        ];
        let output = Self::segment_replace(input, &dict_refs);
        let output_2 = Self::segment_replace(output.join("").as_str(), &dict_refs_round_2);
        if punctuation {
            convert_punctuation(output_2.join("").as_str(), "s")
        } else {
            output_2.join("")
        }
    }

    pub fn tw2s(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [
            &self.dictionary.tw_variants_rev_phrases,
            &self.dictionary.tw_variants_rev
        ];
        let dict_refs_round_2 = [
            &self.dictionary.ts_phrases,
            &self.dictionary.ts_characters
        ];
        let output = Self::segment_replace(input, &dict_refs);
        let output_2 = Self::segment_replace(output.join("").as_str(), &dict_refs_round_2);
        if punctuation {
            convert_punctuation(output_2.join("").as_str(), "t")
        } else {
            output_2.join("")
        }
    }

    pub fn s2twp(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [
            &self.dictionary.st_phrases,
            &self.dictionary.st_characters
        ];
        let dict_refs_round_2 = [
            &self.dictionary.tw_phrases
        ];
        let dict_refs_round_3 = [
            &self.dictionary.tw_variants
        ];
        let output = Self::segment_replace(input, &dict_refs);
        let output_2 = Self::segment_replace(output.join("").as_str(), &dict_refs_round_2);
        let output_3 = Self::segment_replace(output_2.join("").as_str(), &dict_refs_round_3);
        if punctuation {
            convert_punctuation(output_3.join("").as_str(), "s")
        } else {
            output_3.join("")
        }
    }

    pub fn tw2sp(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [
            &self.dictionary.tw_variants_rev_phrases,
            &self.dictionary.tw_variants_rev
        ];
        let dict_refs_round_2 = [
            &self.dictionary.tw_phrases_rev
        ];
        let dict_refs_round_3 = [
            &self.dictionary.ts_phrases,
            &self.dictionary.ts_characters
        ];
        let output = Self::segment_replace(input, &dict_refs);
        let output_2 = Self::segment_replace(output.join("").as_str(), &dict_refs_round_2);
        let output_3 = Self::segment_replace(output_2.join("").as_str(), &dict_refs_round_3);
        if punctuation {
            convert_punctuation(output_3.join("").as_str(), "t")
        } else {
            output_3.join("")
        }
    }

    pub fn s2hk(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [
            &self.dictionary.st_phrases,
            &self.dictionary.st_characters
        ];
        let dict_refs_round_2 = [
            &self.dictionary.hk_variants
        ];
        let output = Self::segment_replace(input, &dict_refs);
        let output_2 = Self::segment_replace(output.join("").as_str(), &dict_refs_round_2);
        if punctuation {
            convert_punctuation(output_2.join("").as_str(), "s")
        } else {
            output_2.join("")
        }
    }

    pub fn hk2s(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [
            &self.dictionary.hk_variants_rev_phrases,
            &self.dictionary.hk_variants_rev
        ];
        let dict_refs_round_2 = [
            &self.dictionary.ts_phrases,
            &self.dictionary.ts_characters
        ];
        let output = Self::segment_replace(input, &dict_refs);
        let output_2 = Self::segment_replace(output.join("").as_str(), &dict_refs_round_2);
        if punctuation {
            convert_punctuation(output_2.join("").as_str(), "t")
        } else {
            output_2.join("")
        }
    }

    pub fn t2tw(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [
            &self.dictionary.tw_variants
        ];
        let output = Self::segment_replace(input, &dict_refs);
        if punctuation {
            convert_punctuation(output.join("").as_str(), "s")
        } else {
            output.join("")
        }
    }

    pub fn t2twp(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [
            &self.dictionary.tw_phrases
        ];
        let dict_refs_round_2 = [
            &self.dictionary.tw_variants
        ];
        let output = Self::segment_replace(input, &dict_refs);
        let output_2 = Self::segment_replace(output.join("").as_str(), &dict_refs_round_2);
        if punctuation {
            convert_punctuation(output_2.join("").as_str(), "s")
        } else {
            output_2.join("")
        }
    }

    pub fn tw2t(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [
            &self.dictionary.tw_variants_rev_phrases,
            &self.dictionary.tw_variants_rev
        ];
        let output = Self::segment_replace(input, &dict_refs);
        if punctuation {
            convert_punctuation(output.join("").as_str(), "s")
        } else {
            output.join("")
        }
    }

    pub fn tw2tp(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [
            &self.dictionary.tw_variants_rev_phrases,
            &self.dictionary.tw_variants_rev
        ];
        let dict_refs_round_2 = [
            &self.dictionary.tw_phrases_rev
        ];
        let output = Self::segment_replace(input, &dict_refs);
        let output_2 = Self::segment_replace(output.join("").as_str(), &dict_refs_round_2);
        if punctuation {
            convert_punctuation(output_2.join("").as_str(), "s")
        } else {
            output_2.join("")
        }
    }

    pub fn t2hk(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [
            &self.dictionary.hk_variants
        ];
        let output = Self::segment_replace(input, &dict_refs);
        if punctuation {
            convert_punctuation(output.join("").as_str(), "s")
        } else {
            output.join("")
        }
    }

    pub fn hk2t(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [
            &self.dictionary.hk_variants_rev_phrases,
            &self.dictionary.hk_variants_rev
        ];
        let output = Self::segment_replace(input, &dict_refs);
        if punctuation {
            convert_punctuation(output.join("").as_str(), "s")
        } else {
            output.join("")
        }
    }
}

// #[allow(dead_code)]
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

// #[allow(dead_code)]
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

// #[allow(dead_code)]
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
