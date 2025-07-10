//! High-performance Chinese text converter using OpenCC lexicons and FMM segmentation.
//!
//! This crate provides efficient segment-based conversion between Simplified and Traditional Chinese.
//! It uses dictionary-based matching with maximum word length control and supports multistage translation
//! via multiple dictionaries. Parallel processing is enabled for large input texts.
//!
//! # Example
//! ```rust
//! use opencc_fmmseg::OpenCC;
//!
//! let input = "汉字转换测试";
//! let opencc = OpenCC::new();
//! let output = opencc.convert(input, "s2t", false);
//! assert_eq!(output, "漢字轉換測試");
//! ```
//!
//! See [README](https://github.com/laisuk/opencc-fmmseg) for more usage examples.

use crate::dictionary_lib::DictionaryMaxlength;
use once_cell::sync::Lazy;
use rayon::prelude::*;
use regex::Regex;
use rustc_hash::{FxHashMap, FxHashSet};
use std::iter::Iterator;
use std::sync::Mutex;

/// Dictionary utilities for managing multiple OpenCC lexicons.
pub mod dictionary_lib;
/// Thread-safe holder for the last error message (if any).
static LAST_ERROR: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));
const DELIMITERS: &'static str = " \t\n\r!\"#$%&'()*+,-./:;<=>?@[\\]^_{}|~＝、。﹁﹂—－（）《》〈〉？！…／＼︒︑︔︓︿﹀︹︺︙︐［﹇］﹈︕︖︰︳︴︽︾︵︶｛︷｝︸﹃﹄【︻】︼　～．，；：";
/// Regular expression used to normalize or strip punctuation from input.
static STRIP_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[!-/:-@\[-`{-~\t\n\v\f\r 0-9A-Za-z_著]").unwrap());

/// Central interface for performing OpenCC-based conversion with segmentation.
///
/// The `OpenCC` struct manages dictionary loading, segmentation, and multi-round text transformation.
/// It supports conversion types such as `s2t`, `t2s`, `s2tw`, etc., and uses maximum match segmentation
/// on non-delimiter text regions to ensure accurate replacements.
pub struct OpenCC {
    /// Dictionary storage with length metadata for maximum matching.
    dictionary: DictionaryMaxlength,
    /// Delimiter characters that separate text into segments.
    delimiters: FxHashSet<char>,
    is_parallel: bool,
}

impl OpenCC {
    /// Creates a new `OpenCC` instance using built-in dictionary constants.
    ///
    /// This is the recommended method for most users. It loads all dictionaries
    /// compiled into the binary at build time (e.g., via `include_str!`), allowing for
    /// fast startup and zero I/O cost.
    ///
    /// Internally, this loads the default `DictionaryMaxlength` via `DictionaryMaxlength::new()`,
    /// and sets up default Chinese delimiters and regular expressions.
    ///
    /// # Returns
    /// An `OpenCC` instance ready for conversion.
    ///
    /// # Panics
    /// Never panics. If the dictionary fails to initialize, a default one is substituted,
    /// and the error is stored internally via `set_last_error()`.
    ///
    /// # Example
    /// ```rust
    /// use opencc_fmmseg::OpenCC;
    /// let cc = OpenCC::new();
    /// let converted = cc.convert("汉字", "s2t", false);
    /// ```
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

    /// Creates an `OpenCC` instance using in-memory JSON dictionary objects.
    ///
    /// This method is useful for unit testing or embedding custom dictionaries directly
    /// in code. It bypasses any file loading or embedded CBOR/JSON files, relying instead
    /// on raw dictionaries defined in `DictionaryMaxlength::from_dicts()`.
    ///
    /// # Returns
    /// An `OpenCC` instance built from in-memory data.
    ///
    /// # Panics
    /// Never panics. If loading fails, an empty dictionary is used and the error
    /// is stored via `set_last_error()`.
    ///
    /// # Example
    /// ```rust
    /// use opencc_fmmseg::OpenCC;
    /// let cc = OpenCC::from_dicts();
    /// ```
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

    /// Creates an `OpenCC` instance by loading dictionaries from an external CBOR file.
    ///
    /// This is ideal for users who want to decouple dictionary data from the binary and
    /// ship a compact `.cbor` file with the application. The CBOR format is a fast,
    /// efficient binary serialization of the dictionary contents.
    ///
    /// # Arguments
    /// * `filename` – Path to a `.cbor` file containing a serialized `DictionaryMaxlength`.
    ///
    /// # Returns
    /// A fully initialized `OpenCC` instance, or one with empty dictionaries if deserialization fails.
    ///
    /// # Errors
    /// If deserialization fails, the dictionary is defaulted and the error is stored
    /// via `set_last_error()`.
    ///
    /// # Example
    /// ```rust
    /// use opencc_fmmseg::OpenCC;
    ///
    /// fn main() {
    ///     let cc = OpenCC::from_cbor("./dicts.s2t.cbor");
    ///     println!("{}", cc.convert("汉字", "s2t", false));
    /// }
    /// ```
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

    /// Internal segment replacement logic based on maximum dictionary match.
    ///
    /// This method performs dictionary-based text conversion by first splitting the input text
    /// into segments using delimiter-aware boundaries. Each segment is then processed independently
    /// using a longest-match strategy over the provided dictionaries.
    ///
    /// The input is first converted to a vector of `char` to enable accurate segmentation and indexing.
    /// It uses `self.get_chars_range()` to identify segments that are separated by delimiters
    /// (such as spaces, punctuation, etc.), and then applies `convert_by()` on each segment.
    ///
    /// Parallelism is applied if `self.is_parallel` is enabled:
    /// - Each segment is processed independently using Rayon (via `par_iter`).
    /// - This improves throughput on large inputs, especially in multicore environments.
    ///
    /// # Arguments
    /// * `text` – The input string to convert.
    /// * `dictionaries` – A list of dictionary references, each paired with its max word length.
    /// * `max_word_length` – The maximum length of phrases to match in the dictionary.
    ///
    /// # Returns
    /// A `String` resulting from applying all dictionary replacements across each segment.
    ///
    /// # Notes
    /// - Delimiters are preserved in output and not transformed.
    /// - This is the core routine that powers all multi-round dictionary applications.
    /// - Should not be exposed publicly; used by `DictRefs::apply_segment_replace`.
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

    /// Splits a slice of characters into a list of index ranges based on delimiter boundaries.
    ///
    /// This function identifies ranges within the character slice where the content is **not split**
    /// by delimiters (e.g., punctuation, spaces). Each range is defined as a `start..end` index,
    /// where `end` is exclusive. Delimiters themselves are included as their own ranges.
    ///
    /// This is typically used during segmentation, where the text is split by Chinese or ASCII
    /// delimiters and only non-delimiter segments are subject to dictionary-based transformation.
    ///
    /// # Behavior
    /// - A delimiter at position `i` causes a range from `start..i+1` to be added.
    /// - The delimiter itself is included in the range to preserve punctuation in output.
    /// - If there is trailing content after the last delimiter, it is included as the final range.
    ///
    /// # Returns
    /// A vector of `std::ops::Range<usize>` representing all segment boundaries.
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

    /// Core dictionary-matching routine using Forward Maximum Matching (FMM).
    ///
    /// This is the tightest loop of the OpenCC segment replacement engine. It takes a slice of
    /// characters (already segmented and delimiter-free) and performs a left-to-right scan using
    /// longest-prefix dictionary matching.
    ///
    /// For each position in the input character slice:
    /// - It tries to match the longest possible substring (up to `max_word_length`) against all provided dictionaries.
    /// - If a match is found, it writes the corresponding replacement to the output string.
    /// - If no match is found, it falls back to copying the current character.
    ///
    /// # Matching Strategy
    /// - Match lengths are attempted in descending order (`max_word_length → 1`).
    /// - Dictionaries are scanned in order; the first match wins (short-circuit logic).
    /// - Dictionaries may represent phrases, characters, or punctuation mappings.
    ///
    /// # Arguments
    /// * `text_chars` – A slice of `char` representing a segment of text (non-delimited).
    /// * `dictionaries` – A list of `(dictionary, max_word_length)` pairs for lookup.
    /// * `max_word_length` – The maximum allowed match length across all dictionaries.
    ///
    /// # Returns
    /// A `String` containing the fully converted result for the input segment.
    ///
    /// # Performance Notes
    /// - This function uses pre-allocated `String` buffers to minimize heap allocations.
    /// - It avoids repeated UTF-8 conversions by working entirely at the `char` level.
    /// - Inner `candidate` buffer is reused across iterations for string key construction.
    ///
    /// # Internal Use
    /// Called by `segment_replace()` and ultimately by `DictRefs::apply_segment_replace()`.
    /// Not intended for public use; exposed internally for performance-critical path.
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

    /// Returns whether parallel segment conversion is currently enabled.
    ///
    /// When parallel mode is enabled, the converter will use Rayon to process
    /// segmented text concurrently. This can improve performance on large inputs
    /// but may introduce overhead on small strings.
    ///
    /// # Returns
    /// `true` if parallel processing is enabled; `false` otherwise.
    ///
    /// # Example
    /// ```rust
    /// use opencc_fmmseg::OpenCC;
    /// let cc = OpenCC::new();
    /// assert_eq!(cc.get_parallel(), true);
    /// ```
    pub fn get_parallel(&self) -> bool {
        self.is_parallel
    }

    /// Sets whether to enable or disable parallel segment conversion.
    ///
    /// This controls whether Rayon parallel iterators will be used during
    /// segment replacement. Set this to `false` if you want to reduce CPU usage
    /// or avoid background threading (e.g., in UI applications).
    ///
    /// # Arguments
    /// * `is_parallel` - `true` to enable parallelism, `false` to disable it.
    ///
    /// # Example
    /// ```rust
    /// use opencc_fmmseg::OpenCC;
    /// let mut cc = OpenCC::new();
    /// cc.set_parallel(false);
    /// assert!(!cc.get_parallel());
    /// ```
    pub fn set_parallel(&mut self, is_parallel: bool) -> () {
        self.is_parallel = is_parallel;
    }

    /// Converts Simplified Chinese text to Traditional Chinese.
    ///
    /// This function performs dictionary-based segment replacement using two levels of dictionaries:
    /// - Phrase-level mappings (`st_phrases`)
    /// - Character-level mappings (`st_characters`)
    ///
    /// If `punctuation` is enabled, an additional punctuation-level dictionary (`st_punctuations`)
    /// is included in the conversion pipeline. The input is segmented based on configured delimiters,
    /// and each non-delimiter segment is processed using longest-match rules.
    ///
    /// This function is parallelized when the `is_parallel` flag is set (default is `true`),
    /// making it suitable for high-performance conversion of large inputs.
    ///
    /// # Arguments
    /// * `input` - A string slice containing Simplified Chinese text.
    /// * `punctuation` - Whether to convert punctuation symbols as well.
    ///
    /// # Returns
    /// A `String` containing the Traditional Chinese equivalent of the input.
    ///
    /// # Example
    /// ```rust
    /// use opencc_fmmseg::OpenCC;
    ///
    /// let cc = OpenCC::new();
    /// let result = cc.s2t("汉字转换测试", false);
    /// assert_eq!(result, "漢字轉換測試");
    /// ```
    pub fn s2t(&self, input: &str, punctuation: bool) -> String {
        let mut round_1: Vec<&(FxHashMap<String, String>, usize)> =
            vec![&self.dictionary.st_phrases, &self.dictionary.st_characters];

        if punctuation {
            round_1.push(&self.dictionary.st_punctuations);
        }

        DictRefs::new(&round_1).apply_segment_replace(input, |input, refs, max_len| {
            self.segment_replace(input, refs, max_len)
        })
    }

    /// Performs Traditional-to-Simplified Chinese conversion.
    pub fn t2s(&self, input: &str, punctuation: bool) -> String {
        let mut round_1: Vec<&(FxHashMap<String, String>, usize)> =
            vec![&self.dictionary.ts_phrases, &self.dictionary.ts_characters];

        if punctuation {
            round_1.push(&self.dictionary.ts_punctuations);
        }

        DictRefs::new(&round_1).apply_segment_replace(input, |input, refs, max_len| {
            self.segment_replace(input, refs, max_len)
        })
    }

    /// Performs Simplified-to-Taiwanese conversion.
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

    /// Performs Taiwanese-to-Simplified conversion.
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

    /// Performs simplified to Traditional Taiwan conversion with idioms
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

    /// Performs Traditional Taiwan to Simplified with idioms
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

    /// Performs simplified to Traditional Hong Kong
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

    /// Performs Traditional Hong Kong to Simplified
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

    /// Performs traditional to traditional Taiwan
    pub fn t2tw(&self, input: &str) -> String {
        let round_1 = [&self.dictionary.tw_variants];
        let output = DictRefs::new(&round_1)
            .apply_segment_replace(input, |input, refs, max_len| {
                self.segment_replace(input, refs, max_len)
            });

        output
    }

    /// Performs traditional to traditional Taiwan with idioms
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

    /// Performs traditional Taiwan to traditional
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

    /// Performs traditional Taiwan to traditional with idioms
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

    /// Perform traditional to traditional Hong Kong
    pub fn t2hk(&self, input: &str) -> String {
        let round_1 = [&self.dictionary.hk_variants];
        let output = DictRefs::new(&round_1)
            .apply_segment_replace(input, |input, refs, max_len| {
                self.segment_replace(input, refs, max_len)
            });

        output
    }

    /// Performs traditional Hong Kong to traditional
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

    /// Performs Japanese Kyujitai to Shinjitai
    pub fn t2jp(&self, input: &str) -> String {
        let round_1 = [&self.dictionary.jp_variants];
        let output = DictRefs::new(&round_1)
            .apply_segment_replace(input, |input, refs, max_len| {
                self.segment_replace(input, refs, max_len)
            });

        output
    }

    /// Performs japanese Shinjitai to Kyujitai
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

    /// Converts Chinese text using the specified OpenCC conversion configuration.
    ///
    /// This is the primary entry point for performing OpenCC-style text transformation. It supports
    /// various configurations such as Simplified to Traditional, Traditional to Simplified, Taiwanese,
    /// Hong Kong, and Japanese variants. The conversion is dictionary-based and supports optional
    /// punctuation normalization depending on the selected configuration.
    ///
    /// Supported configurations:
    ///
    /// | Config     | Description                               | Punctuation Aware |
    /// |------------|-------------------------------------------|-------------------|
    /// | `s2t`      | Simplified Chinese → Traditional Chinese  | ✅                |
    /// | `s2tw`     | Simplified Chinese → Traditional (Taiwan) | ✅                |
    /// | `s2twp`    | Simplified → Taiwanese with phrases       | ✅                |
    /// | `s2hk`     | Simplified Chinese → Traditional (HK)     | ✅                |
    /// | `t2s`      | Traditional Chinese → Simplified Chinese  | ✅                |
    /// | `t2tw`     | Traditional → Taiwanese                   | ❌                |
    /// | `t2twp`    | Traditional → Taiwanese with phrases      | ❌                |
    /// | `t2hk`     | Traditional → Hong Kong                   | ❌                |
    /// | `tw2s`     | Taiwanese → Simplified Chinese            | ✅                |
    /// | `tw2sp`    | Taiwanese → Simplified (with punct.)      | ✅                |
    /// | `tw2t`     | Taiwanese → Traditional Chinese           | ❌                |
    /// | `tw2tp`    | Taiwanese → Traditional (with punct.)     | ❌                |
    /// | `hk2s`     | Hong Kong → Simplified Chinese            | ✅                |
    /// | `hk2t`     | Hong Kong → Traditional Chinese           | ❌                |
    /// | `jp2t`     | Japanese → Traditional Chinese            | ❌                |
    /// | `t2jp`     | Traditional Chinese → Japanese            | ❌                |
    ///
    /// # Arguments
    ///
    /// * `input` - The input string containing Chinese text.
    /// * `config` - The OpenCC conversion configuration name. It is case-insensitive.
    /// * `punctuation` - Whether to also apply punctuation conversion (only applies to certain configs).
    ///
    /// # Returns
    ///
    /// A `String` containing the converted Chinese text. If the config is invalid,
    /// returns an error message string and stores the last error internally.
    ///
    /// # Errors
    ///
    /// If an unknown or unsupported config is provided, the function returns a string
    /// in the form `"Invalid config: {config}"` and records it in the last error slot.
    ///
    /// # Example
    ///
    /// ```rust
    /// use opencc_fmmseg::OpenCC;
    ///
    /// let converter = OpenCC::new();
    /// let simplified = "汉字转换测试";
    /// let traditional = converter.convert(simplified, "s2t", false);
    /// assert_eq!(traditional, "漢字轉換測試");
    /// ```
    ///
    /// # See Also
    /// - [`zho_check`](#method.zho_check) for script detection
    /// - [`DictionaryMaxlength`](DictionaryMaxlength) for dictionary internals
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

    /// Internal: Applies a fast character-level Simplified-to-Traditional conversion.
    ///
    /// This method performs a low-overhead transformation using only the `st_characters`
    /// dictionary, mapping each character in the input string to its Traditional form
    /// if available.
    ///
    /// Designed for high-speed single-pass checks (e.g., used in `zho_check()`).
    /// Supports parallel character collection if `is_parallel` is enabled.
    ///
    /// # Arguments
    /// * `input` - Simplified Chinese input string.
    ///
    /// # Returns
    /// A string where each character has been converted using `st_characters`.
    ///
    /// # Note
    /// This bypasses phrase-level and punctuation dictionaries for performance.
    fn st(&self, input: &str) -> String {
        let dict_refs = [&self.dictionary.st_characters];
        let chars: Vec<char> = if self.is_parallel {
            input.par_chars().collect()
        } else {
            input.chars().collect()
        };
        self.convert_by(&chars, &dict_refs, 1)
    }

    /// Internal: Applies a fast character-level Traditional-to-Simplified conversion.
    ///
    /// Uses only the `ts_characters` dictionary to map Traditional characters to
    /// their Simplified form, one-by-one. Optimized for script detection or fast filters.
    ///
    /// Uses Rayon parallelization if `is_parallel` is enabled.
    ///
    /// # Arguments
    /// * `input` - Traditional Chinese input string.
    ///
    /// # Returns
    /// A Simplified Chinese string converted from individual characters.
    ///
    /// # Note
    /// This is a minimal-pass check — punctuation and phrases are not processed.
    fn ts(&self, input: &str) -> String {
        let dict_refs = [&self.dictionary.ts_characters];
        let chars: Vec<char> = if self.is_parallel {
            input.par_chars().collect()
        } else {
            input.chars().collect()
        };
        self.convert_by(&chars, &dict_refs, 1)
    }

    /// Detects the likely Chinese script type of the input text.
    ///
    /// This function analyzes the given string and attempts to determine whether it primarily contains
    /// Traditional Chinese, Simplified Chinese, or neither. It uses dictionary-based transformation
    /// to compare the input against converted versions and checks for differences.
    ///
    /// Returns:
    /// - `1` if the input text appears to be Traditional Chinese.
    /// - `2` if the input text appears to be Simplified Chinese.
    /// - `0` if the input is empty or doesn't clearly match either.
    ///
    /// # Arguments
    /// * `input` - The input string to analyze.
    ///
    /// # Examples
    /// ```rust
    /// use opencc_fmmseg::OpenCC;
    /// let cc = OpenCC::new();
    /// assert_eq!(cc.zho_check("漢字"), 1); // Traditional
    /// assert_eq!(cc.zho_check("汉字"), 2); // Simplified
    /// assert_eq!(cc.zho_check("hello"), 0); // Neither
    /// ```
    pub fn zho_check(&self, input: &str) -> i32 {
        if input.is_empty() {
            return 0;
        }

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

    /// Records an error message as the most recent OpenCC runtime error.
    ///
    /// This is used internally to store non-panic errors, such as failed dictionary loading
    /// or invalid conversion configurations. It allows safe retrieval via [`get_last_error()`]
    /// without throwing exceptions or returning `Result<T, E>` from core APIs.
    ///
    /// # Arguments
    /// * `err_msg` – A string slice containing the error message to store.
    ///
    /// # Example (internal use)
    /// ```rust
    /// use opencc_fmmseg::OpenCC;
    /// OpenCC::set_last_error("Failed to load dictionary.");
    /// ```
    pub fn set_last_error(err_msg: &str) {
        let mut last_error = LAST_ERROR.lock().unwrap();
        *last_error = Some(err_msg.to_string());
    }

    /// Retrieves the most recently recorded error message, if any.
    ///
    /// This can be used by consumers after calling `convert()` or dictionary loaders
    /// to inspect whether any non-fatal errors occurred (e.g., fallback to default dict).
    ///
    /// # Returns
    /// An `Option<String>` containing the error message, or `None` if no error was recorded.
    ///
    /// # Example
    /// ```rust
    /// use opencc_fmmseg::OpenCC;
    /// if let Some(err) = OpenCC::get_last_error() {
    ///     eprintln!("⚠️ OpenCC warning: {err}");
    /// }
    /// ```
    pub fn get_last_error() -> Option<String> {
        let last_error = LAST_ERROR.lock().unwrap();
        last_error.clone()
    }
}

/// Internal type representing a single round of dictionary application.
/// Each round includes a slice of dictionary references and the maximum word length.
///
/// Format: `(&[&(dict, max_len)], max_len)`
type DictRound<'a> = (&'a [&'a (FxHashMap<String, String>, usize)], usize);

/// Builder-style struct for managing multi-round dictionary segment replacement.
///
/// `DictRefs` is used to group one or more rounds of dictionary reference sets,
/// where each round is a list of `(dictionary, max_word_length)` pairs. Each round is applied
/// in order to transform input text using the same segment replacement strategy.
///
/// This design allows `OpenCC`-style multi-stage conversion pipelines (e.g., `s2tw -> tw_variants`)
/// to be composed with readable, chainable syntax.
///
/// # Usage
/// Example of multi-round dictionary construction:
/// ```rust,no_run
/// use opencc_fmmseg::DictRefs;
/// use rustc_hash::FxHashMap;
///
/// let dict1: FxHashMap<String, String> = FxHashMap::default();
/// let dict2: FxHashMap<String, String> = FxHashMap::default();
/// let dict3: FxHashMap<String, String> = FxHashMap::default();
/// let refs = DictRefs::new(&[&(dict1, 1)])
///     .with_round_2(&[&(dict2, 1)])
///     .with_round_3(&[&(dict3, 1)]);
/// ```
/// Then passed to `apply_segment_replace()` to perform all dictionary rounds.
pub struct DictRefs<'a> {
    round_1: DictRound<'a>,
    round_2: Option<DictRound<'a>>,
    round_3: Option<DictRound<'a>>,
}

impl<'a> DictRefs<'a> {
    /// Creates a new `DictRefs` instance with a required first round of dictionaries.
    ///
    /// This is the entry point for composing a dictionary pipeline. The `max_word_length` for
    /// the first round is automatically calculated.
    ///
    /// # Arguments
    /// * `round_1` – A slice of dictionary references, each paired with a `max_word_length`.
    ///
    /// # Returns
    /// A `DictRefs` instance with round 1 populated, and rounds 2 and 3 unset.
    pub fn new(round_1: &'a [&'a (FxHashMap<String, String>, usize)]) -> Self {
        let max_len = round_1.iter().map(|(_, len)| *len).max().unwrap_or(1);
        DictRefs {
            round_1: (round_1, max_len),
            round_2: None,
            round_3: None,
        }
    }

    /// Adds a second round of dictionary conversion to the pipeline.
    ///
    /// This round will be applied after round 1. Typically used for conversions like:
    /// `s2tw -> tw_variants`, or `t2s -> t2jp`.
    ///
    /// # Arguments
    /// * `round_2` – A slice of dictionary references to apply in the second pass.
    ///
    /// # Returns
    /// A modified `DictRefs` with round 2 included.
    pub fn with_round_2(mut self, round_2: &'a [&'a (FxHashMap<String, String>, usize)]) -> Self {
        let max_len = round_2.iter().map(|(_, len)| *len).max().unwrap_or(1);
        self.round_2 = Some((round_2, max_len));
        self
    }

    /// Adds a third round of dictionary conversion to the pipeline.
    ///
    /// Useful for rare cases where three-stage transformation is needed.
    /// Example: `s2tw -> tw_variants -> punctuation` conversion.
    ///
    /// # Arguments
    /// * `round_3` – A slice of dictionary references to apply in the third pass.
    ///
    /// # Returns
    /// A modified `DictRefs` with round 3 included.
    pub fn with_round_3(mut self, round_3: &'a [&'a (FxHashMap<String, String>, usize)]) -> Self {
        let max_len = round_3.iter().map(|(_, len)| *len).max().unwrap_or(1);
        self.round_3 = Some((round_3, max_len));
        self
    }

    /// Applies up to three rounds of segment-based dictionary replacement to the input text.
    ///
    /// Each round is applied sequentially: round 1 → round 2 (optional) → round 3 (optional).
    /// The replacement is performed using a user-supplied closure, typically `OpenCC::segment_replace()`,
    /// which handles the delimiter-aware segmentation and longest-match logic.
    ///
    /// # Arguments
    /// * `input` – The original input text to transform.
    /// * `segment_replace` – A closure that takes a text chunk, dictionary slice, and max length.
    ///
    /// # Returns
    /// A fully transformed string after applying all configured dictionary rounds.
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

/// Finds a valid UTF-8 boundary within the given string, limited by a maximum byte count.
///
/// This function ensures that slicing the string at the returned index will **not break UTF-8 encoding**.
/// It is typically used to extract a prefix of the input string that does not exceed `max_byte_count`
/// **and ends on a valid character boundary**.
///
/// # How it works
/// - If the string is already shorter than `max_byte_count`, the full length is returned.
/// - Otherwise, it backtracks from `max_byte_count` until it reaches a valid UTF-8 start byte
///   (i.e., not a continuation byte `0b10xxxxxx`).
///
/// # Arguments
/// * `sv` – The input string to examine.
/// * `max_byte_count` – The maximum number of bytes allowed.
///
/// # Returns
/// A safe byte index at or below `max_byte_count` where a valid UTF-8 character boundary ends.
///
/// # Example
/// ```rust
/// use opencc_fmmseg::find_max_utf8_length;
///
/// let input = "汉字转换测试"; // Each Chinese character takes 3 bytes
/// let safe_index = find_max_utf8_length(input, 7);
/// let substring = &input[..safe_index]; // No panic!
/// println!("Safe prefix: {}", substring);
/// ```
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
