use crate::dictionary_lib::DictionaryMaxlength;
use once_cell::sync::Lazy;
use rayon::prelude::*;
use regex::Regex;
use rustc_hash::{FxHashMap, FxHashSet};
use std::iter::Iterator;
use std::sync::Mutex;
pub mod dictionary_lib;
// Define a global mutable variable to store the error message
// static LAST_ERROR: Mutex<Option<String>> = Mutex::new(None);
// Use once_cell instead of lazy_static
static LAST_ERROR: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));
// const DELIMITERS: &'static str = " \t\n\r!\"#$%&'()*+,-./:;<=>?@[\\]^_{}|~＝、。“”‘’『』「」﹁﹂—－（）《》〈〉？！…／＼︒︑︔︓︿﹀︹︺︙︐［﹇］﹈︕︖︰︳︴︽︾︵︶｛︷｝︸﹃﹄【︻】︼　～．，；：";
const DELIMITERS: &'static str = " \t\n\r!\"#$%&'()*+,-./:;<=>?@[\\]^_{}|~＝、。﹁﹂—－（）《》〈〉？！…／＼︒︑︔︓︿﹀︹︺︙︐［﹇］﹈︕︖︰︳︴︽︾︵︶｛︷｝︸﹃﹄【︻】︼　～．，；：";
static STRIP_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[!-/:-@\[-`{-~\t\n\v\f\r 0-9A-Za-z_著]").unwrap());

pub struct OpenCC {
    dictionary: DictionaryMaxlength,
    delimiters: FxHashSet<char>,
    is_parallel: bool,
}

impl OpenCC {
    pub fn new() -> Self {
        let dictionary = DictionaryMaxlength::new().unwrap_or_else(|err| {
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

    pub fn from_dicts() -> Self {
        let dictionary = DictionaryMaxlength::from_dicts().unwrap_or_else(|err| {
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

    pub fn from_cbor(filename: &str) -> Self {
        let dictionary =
            DictionaryMaxlength::deserialize_from_cbor(filename).unwrap_or_else(|err| {
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
        dictionaries: &[&(FxHashMap<String, String>, usize)],
        max_word_length: usize,
    ) -> String {
        let chars: Vec<char> = if self.is_parallel {
            text.par_chars().collect()
        } else {
            text.chars().collect()
        };

        let ranges = self.get_chars_range(&chars);

        if self.is_parallel {
            ranges
                .into_par_iter()
                .map(|r| self.convert_by(&chars[r], dictionaries, max_word_length))
                .collect()
        } else {
            ranges
                .into_iter()
                .map(|r| self.convert_by(&chars[r], dictionaries, max_word_length))
                .collect()
        }
    }

    // #[allow(dead_code)]
    // fn split_string_inclusive(&self, text: &str, is_parallel: bool) -> Vec<Vec<char>> {
    //     if is_parallel {
    //         let collected: Vec<char> = text.par_chars().collect();
    //         collected
    //             .par_split_inclusive(|c| self.delimiters.contains(c))
    //             .map(|slice| slice.to_vec())
    //             .collect()
    //     } else {
    //         let mut split_string_list = Vec::new();
    //         let mut current_chunk = Vec::with_capacity(16); // heuristic: most chunks are short
    //
    //         for ch in text.chars() {
    //             current_chunk.push(ch);
    //
    //             if self.delimiters.contains(&ch) {
    //                 split_string_list.push(std::mem::take(&mut current_chunk));
    //                 current_chunk = Vec::with_capacity(16); // reuse capacity
    //             }
    //         }
    //
    //         if !current_chunk.is_empty() {
    //             split_string_list.push(current_chunk);
    //         }
    //
    //         split_string_list
    //     }
    // }

    fn get_chars_range(&self, chars: &[char]) -> Vec<std::ops::Range<usize>> {
        let mut ranges = Vec::new();
        let mut start = 0;

        for (i, ch) in chars.iter().enumerate() {
            if self.delimiters.contains(ch) {
                ranges.push(start..i + 1); // now exclusive end
                start = i + 1;
            }
        }

        if start < chars.len() {
            ranges.push(start..chars.len());
        }

        ranges
    }

    fn convert_by(
        &self,
        text_chars: &[char],
        dictionaries: &[&(FxHashMap<String, String>, usize)],
        max_word_length: usize,
    ) -> String {
        if text_chars.is_empty() {
            return String::new();
        }

        let text_length = text_chars.len();
        if text_length == 1 && self.delimiters.contains(&text_chars[0]) {
            return text_chars[0].to_string();
        }

        let mut result = String::with_capacity(text_length * 4);
        let mut candidate = String::with_capacity(max_word_length * 4);
        let mut start_pos = 0;

        while start_pos < text_length {
            let max_length = std::cmp::min(max_word_length, text_length - start_pos);
            let mut best_match_length = 0;
            let mut best_match: &str = "";

            for length in (1..=max_length).rev() {
                candidate.clear();
                candidate.extend(&text_chars[start_pos..start_pos + length]);

                for dictionary in dictionaries {
                    if dictionary.1 < length {
                        continue;
                    }
                    if let Some(value) = dictionary.0.get(&candidate) {
                        best_match_length = length;
                        best_match = value;
                        break;
                    }
                }

                if best_match_length > 0 {
                    break;
                }
            }

            if best_match_length == 0 {
                best_match_length = 1;
                candidate.clear();
                candidate.push(text_chars[start_pos]);
                best_match = &candidate;
            }

            result.push_str(best_match);
            start_pos += best_match_length;
        }

        result
    }

    pub fn get_parallel(&self) -> bool {
        self.is_parallel
    }

    pub fn set_parallel(&mut self, is_parallel: bool) -> () {
        self.is_parallel = is_parallel;
    }

    pub fn s2t(&self, input: &str, punctuation: bool) -> String {
        let mut round_1: Vec<&(FxHashMap<String, String>, usize)> =
            vec![&self.dictionary.st_phrases, &self.dictionary.st_characters];

        if punctuation {
            // round_1.push(&*ST_PUNCT_TUPLE);
            round_1.push(&self.dictionary.st_punctuations);
        }

        DictRefs::new(&round_1).apply_segment_replace(input, |input, refs, max_len| {
            self.segment_replace(input, refs, max_len)
        })
    }

    pub fn t2s(&self, input: &str, punctuation: bool) -> String {
        let mut round_1: Vec<&(FxHashMap<String, String>, usize)> =
            vec![&self.dictionary.ts_phrases, &self.dictionary.ts_characters];

        if punctuation {
            // round_1.push(&*TS_PUNCT_TUPLE);
            round_1.push(&self.dictionary.ts_punctuations);
        }

        DictRefs::new(&round_1).apply_segment_replace(input, |input, refs, max_len| {
            self.segment_replace(input, refs, max_len)
        })
    }

    pub fn s2tw(&self, input: &str, punctuation: bool) -> String {
        let mut round_1: Vec<&(FxHashMap<String, String>, usize)> =
            vec![&self.dictionary.st_phrases, &self.dictionary.st_characters];

        if punctuation {
            round_1.push(&self.dictionary.st_punctuations);
        }

        DictRefs::new(&round_1)
            .with_round_2(&[&self.dictionary.tw_variants])
            .apply_segment_replace(input, |input, refs, max_len| {
                self.segment_replace(input, refs, max_len)
            })
    }

    pub fn tw2s(&self, input: &str, punctuation: bool) -> String {
        let mut round_2: Vec<&(FxHashMap<String, String>, usize)> =
            vec![&self.dictionary.ts_phrases, &self.dictionary.ts_characters];

        if punctuation {
            round_2.push(&self.dictionary.ts_punctuations);
        }

        DictRefs::new(&[
            &self.dictionary.tw_variants_rev_phrases,
            &self.dictionary.tw_variants_rev,
        ])
        .with_round_2(&round_2)
        .apply_segment_replace(input, |input, refs, max_len| {
            self.segment_replace(input, refs, max_len)
        })
    }

    pub fn s2twp(&self, input: &str, punctuation: bool) -> String {
        // Create bindings for each round of dictionary references
        let mut round_1: Vec<&(FxHashMap<String, String>, usize)> =
            vec![&self.dictionary.st_phrases, &self.dictionary.st_characters];

        if punctuation {
            round_1.push(&self.dictionary.st_punctuations);
        }
        let round_2 = [&self.dictionary.tw_phrases];
        let round_3 = [&self.dictionary.tw_variants];
        // Use the DictRefs struct to handle 3 rounds
        DictRefs::new(&round_1)
            .with_round_2(&round_2)
            .with_round_3(&round_3)
            .apply_segment_replace(input, |input, refs, max_len| {
                self.segment_replace(input, refs, max_len)
            })
    }

    pub fn tw2sp(&self, input: &str, punctuation: bool) -> String {
        let round_1 = [
            &self.dictionary.tw_phrases_rev,
            &self.dictionary.tw_variants_rev_phrases,
            &self.dictionary.tw_variants_rev,
        ];
        let mut round_2: Vec<&(FxHashMap<String, String>, usize)> =
            vec![&self.dictionary.ts_phrases, &self.dictionary.ts_characters];

        if punctuation {
            round_2.push(&self.dictionary.ts_punctuations);
        }

        DictRefs::new(&round_1)
            .with_round_2(&round_2)
            .apply_segment_replace(input, |input, refs, max_len| {
                self.segment_replace(input, refs, max_len)
            })
    }

    pub fn s2hk(&self, input: &str, punctuation: bool) -> String {
        let mut round_1: Vec<&(FxHashMap<String, String>, usize)> =
            vec![&self.dictionary.st_phrases, &self.dictionary.st_characters];

        if punctuation {
            round_1.push(&self.dictionary.st_punctuations);
        }
        let round_2 = [&self.dictionary.hk_variants];
        DictRefs::new(&round_1)
            .with_round_2(&round_2)
            .apply_segment_replace(input, |input, refs, max_len| {
                self.segment_replace(input, refs, max_len)
            })
    }

    pub fn hk2s(&self, input: &str, punctuation: bool) -> String {
        let round_1 = [
            &self.dictionary.hk_variants_rev_phrases,
            &self.dictionary.hk_variants_rev,
        ];
        let mut round_2: Vec<&(FxHashMap<String, String>, usize)> =
            vec![&self.dictionary.ts_phrases, &self.dictionary.ts_characters];

        if punctuation {
            round_2.push(&self.dictionary.ts_punctuations);
        }
        DictRefs::new(&round_1)
            .with_round_2(&round_2)
            .apply_segment_replace(input, |input, refs, max_len| {
                self.segment_replace(input, refs, max_len)
            })
    }

    pub fn t2tw(&self, input: &str) -> String {
        let round_1 = [&self.dictionary.tw_variants];
        let output = DictRefs::new(&round_1)
            .apply_segment_replace(input, |input, refs, max_len| {
                self.segment_replace(input, refs, max_len)
            });

        output
    }

    pub fn t2twp(&self, input: &str) -> String {
        let round_1 = [&self.dictionary.tw_phrases];
        let round_2 = [&self.dictionary.tw_variants];
        let output = DictRefs::new(&round_1)
            .with_round_2(&round_2)
            .apply_segment_replace(input, |input, refs, max_len| {
                self.segment_replace(input, refs, max_len)
            });

        output
    }

    pub fn tw2t(&self, input: &str) -> String {
        let round_1 = [
            &self.dictionary.tw_variants_rev_phrases,
            &self.dictionary.tw_variants_rev,
        ];
        let output = DictRefs::new(&round_1)
            .apply_segment_replace(input, |input, refs, max_len| {
                self.segment_replace(input, refs, max_len)
            });

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
            .apply_segment_replace(input, |input, refs, max_len| {
                self.segment_replace(input, refs, max_len)
            });

        output
    }

    pub fn t2hk(&self, input: &str) -> String {
        let round_1 = [&self.dictionary.hk_variants];
        let output = DictRefs::new(&round_1)
            .apply_segment_replace(input, |input, refs, max_len| {
                self.segment_replace(input, refs, max_len)
            });

        output
    }

    pub fn hk2t(&self, input: &str) -> String {
        let round_1 = [
            &self.dictionary.hk_variants_rev_phrases,
            &self.dictionary.hk_variants_rev,
        ];
        let output = DictRefs::new(&round_1)
            .apply_segment_replace(input, |input, refs, max_len| {
                self.segment_replace(input, refs, max_len)
            });

        output
    }

    pub fn t2jp(&self, input: &str) -> String {
        let round_1 = [&self.dictionary.jp_variants];
        let output = DictRefs::new(&round_1)
            .apply_segment_replace(input, |input, refs, max_len| {
                self.segment_replace(input, refs, max_len)
            });

        output
    }

    pub fn jp2t(&self, input: &str) -> String {
        let round_1 = [
            &self.dictionary.jps_phrases,
            &self.dictionary.jps_characters,
            &self.dictionary.jp_variants_rev,
        ];
        let output = DictRefs::new(&round_1)
            .apply_segment_replace(input, |input, refs, max_len| {
                self.segment_replace(input, refs, max_len)
            });

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
                format!("Invalid config: {}", config)
            }
        }
    }

    fn st(&self, input: &str) -> String {
        let dict_refs = [&self.dictionary.st_characters];
        let chars: Vec<char> = if self.is_parallel {
            input.par_chars().collect()
        } else {
            input.chars().collect()
        };
        self.convert_by(&chars, &dict_refs, 1)
    }

    fn ts(&self, input: &str) -> String {
        let dict_refs = [&self.dictionary.ts_characters];
        let chars: Vec<char> = if self.is_parallel {
            input.par_chars().collect()
        } else {
            input.chars().collect()
        };
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

    #[allow(dead_code)]
    fn convert_punctuation(text: &str, config: &str) -> String {
        let mut s2t_punctuation_chars: FxHashMap<&str, &str> = FxHashMap::default();
        s2t_punctuation_chars.insert("“", "「");
        s2t_punctuation_chars.insert("”", "」");
        s2t_punctuation_chars.insert("‘", "『");
        s2t_punctuation_chars.insert("’", "』");

        let t2s_punctuation_chars: FxHashMap<&str, &str> = s2t_punctuation_chars
            .iter()
            .map(|(&k, &v)| (v, k))
            .collect();

        let mapping = if config.starts_with('s') {
            &s2t_punctuation_chars
        } else {
            &t2s_punctuation_chars
        };

        let pattern = mapping
            .keys()
            .map(|k| regex::escape(k))
            .collect::<Vec<_>>()
            .join("|");

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

type DictRound<'a> = (&'a [&'a (FxHashMap<String, String>, usize)], usize);

pub struct DictRefs<'a> {
    round_1: DictRound<'a>,
    round_2: Option<DictRound<'a>>,
    round_3: Option<DictRound<'a>>,
}

impl<'a> DictRefs<'a> {
    pub fn new(round_1: &'a [&'a (FxHashMap<String, String>, usize)]) -> Self {
        let max_len = round_1.iter().map(|(_, len)| *len).max().unwrap_or(1);
        DictRefs {
            round_1: (round_1, max_len),
            round_2: None,
            round_3: None,
        }
    }

    pub fn with_round_2(mut self, round_2: &'a [&'a (FxHashMap<String, String>, usize)]) -> Self {
        let max_len = round_2.iter().map(|(_, len)| *len).max().unwrap_or(1);
        self.round_2 = Some((round_2, max_len));
        self
    }

    pub fn with_round_3(mut self, round_3: &'a [&'a (FxHashMap<String, String>, usize)]) -> Self {
        let max_len = round_3.iter().map(|(_, len)| *len).max().unwrap_or(1);
        self.round_3 = Some((round_3, max_len));
        self
    }

    pub fn apply_segment_replace<F>(&self, input: &str, segment_replace: F) -> String
    where
        F: Fn(&str, &[&(FxHashMap<String, String>, usize)], usize) -> String,
    {
        let mut output = segment_replace(input, self.round_1.0, self.round_1.1);
        if let Some((refs, max)) = &self.round_2 {
            output = segment_replace(&output, refs, *max);
        }
        if let Some((refs, max)) = &self.round_3 {
            output = segment_replace(&output, refs, *max);
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
