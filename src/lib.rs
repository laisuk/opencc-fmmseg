// Enable cfg badges on docs.rs (optional but nice)
#![cfg_attr(docsrs, feature(doc_cfg))]

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
use once_cell::sync::Lazy;
use rayon::prelude::*;
use regex::Regex;
use rustc_hash::FxHashMap;
use std::iter::Iterator;
use std::sync::Mutex;

/// Delimiters helper for splitting and matching delimiters.
pub mod delimiter_set;
/// Bridge helper for conversion plan and core converter functions.
mod dict_refs;
/// Dictionary utilities for managing multiple OpenCC lexicons.
pub mod dictionary_lib;
pub mod utils;
pub mod opencc_config;

use crate::delimiter_set::is_delimiter;
pub use crate::dict_refs::DictRefs;
use crate::dictionary_lib::dictionary_maxlength::UnionKey;
use crate::dictionary_lib::StarterUnion;
use dictionary_lib::dict_max_len::DictMaxLen;
use dictionary_lib::DictionaryMaxlength;
pub use opencc_config::*;

/// Thread-safe holder for the last error message (if any).
static LAST_ERROR: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));
// const DELIMITERS: &'static str = " \t\n\r!\"#$%&'()*+,-./:;<=>?@[\\]^_{}|~＝、。﹁﹂—－（）《》〈〉？！…／＼︒︑︔︓︿﹀︹︺︙︐［﹇］﹈︕︖︰︳︴︽︾︵︶｛︷｝︸﹃﹄【︻】︼　～．，；：";
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
    /// Flag indicator for parallelism
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
        let is_parallel = true;

        OpenCC {
            dictionary,
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
        let is_parallel = true;

        OpenCC {
            dictionary,
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
        let is_parallel = true;

        OpenCC {
            dictionary,
            is_parallel,
        }
    }

    /// Splits a slice of characters into a list of index ranges based on delimiter boundaries.
    ///
    /// This function identifies ranges within the character slice where the content is segmented
    /// by delimiters (e.g., punctuation, spaces). Each range is defined as `start..end` where `end` is exclusive.
    ///
    /// # Parameters
    /// - `chars`: The input slice of characters to be split.
    /// - `inclusive`: If `true`, each segment includes the delimiter at the end.
    ///                If `false`, the delimiter is split into its own range.
    ///
    /// # Behavior
    /// - If `inclusive == true`: a delimiter at position `i` causes a range from `start..i+1`.
    /// - If `inclusive == false`: two ranges are emitted: `start..i` (content) and `i..i+1` (delimiter).
    /// - If there is trailing content after the last delimiter, it is included as the final range.
    ///
    /// # Returns
    /// A vector of `std::ops::Range<usize>` representing all segment boundaries.
    fn get_chars_range(&self, chars: &[char], inclusive: bool) -> Vec<std::ops::Range<usize>> {
        let mut ranges = Vec::new();
        let mut start = 0;

        for (i, ch) in chars.iter().enumerate() {
            if is_delimiter(*ch) {
                if inclusive {
                    ranges.push(start..i + 1);
                } else {
                    if i > start {
                        ranges.push(start..i);
                    }
                    ranges.push(i..i + 1);
                }
                start = i + 1;
            }
        }

        if start < chars.len() {
            ranges.push(start..chars.len());
        }

        ranges
    }

    /// Internal bridge that drives FMM conversion using a precomputed **starter union**.
    ///
    /// Splits `text` into delimiter‑aware segments, then converts each segment independently via
    /// [`convert_by_union`]. A single prebuilt [`StarterUnion`] (for the given `dictionaries`)
    /// is reused across all segments **once per call**.
    ///
    /// # Pipeline
    /// 1. Collect input into `Vec<char>` (parallel or sequential).
    /// 2. Compute non‑delimited ranges with [`get_chars_range`].
    /// 3. Build a [`StarterUnion`] **once** from `dictionaries`.
    /// 4. For each range, call [`convert_by_union`] with the prebuilt union.
    /// 5. Concatenate results in the original order (delimiters preserved).
    ///
    /// # Arguments
    /// - `text`: Source string.
    /// - `dictionaries`: Dictionaries to consult (probe order = precedence). Each must have
    ///   populated starter indexes (see [`DictMaxLen::build_from_pairs`] or
    ///   [`DictMaxLen::populate_starter_indexes`]).
    /// - `max_word_length`: Global cap for match length in chars (e.g., 16).
    /// - `union`: The precomputed [`StarterUnion`] corresponding to `dictionaries`.
    ///
    /// # Parallelism
    /// If `self.is_parallel` is `true`:
    /// - Input chars are collected using a parallel iterator.
    /// - Each segment is converted in parallel (`into_par_iter()`).
    /// This can significantly improve throughput on large inputs with many segments.
    ///
    /// # Behavior
    /// - Delimiters are **not transformed**; they are preserved exactly.
    /// - Each contiguous non‑delimiter segment is processed with greedy FMM, probing only lengths
    ///   admitted by the union’s bitmasks/caps (longest‑first, first‑hit‑wins).
    ///
    /// # Complexity
    /// Let *N* be total chars, *S* segments, *D* dictionaries:
    /// - Union build: `O(D · 65_536)` for BMP + sparse astral merge (once per call).
    /// - Conversion: Σ over segments of `O(len(segment) · K · D)`, where `K ≤ 64` viable
    ///   lengths after union pruning (often much less due to early exits).
    ///
    /// # Example (illustrative)
    /// ```ignore
    /// // `opencc.segment_replace("...")`
    /// //   builds one StarterUnion from the dictionaries,
    /// //   then calls `convert_by_union` per non‑delimited segment.
    /// ```
    ///
    /// # Notes
    /// - If the set or contents of `dictionaries` changes, rebuild the union
    ///   (this routine is typically called by a higher‑level helper that does so).
    /// - Internal bridge used by higher‑level routines (e.g., [`DictRefs::apply_segment_replace`]).
    ///
    #[inline]
    fn segment_replace_with_union(
        &self,
        text: &str,
        dictionaries: &[&DictMaxLen],
        max_word_length: usize,
        union: &StarterUnion,
    ) -> String {
        let chars: Vec<char> = text.chars().collect();
        // Build delimiter-safe ranges (no cross-phrase splits)
        let ranges = self.get_chars_range(&chars, true);

        if self.is_parallel {
            let threads = rayon::current_num_threads().max(1);
            let desired_chunks = threads * 6;
            let chunk_ranges = (ranges.len() / desired_chunks).max(128).min(2048);

            // Small-input guard: fall back to serial if we'd get ≤ 1 chunk anyway
            if ranges.len() <= chunk_ranges {
                let mut out = String::with_capacity(text.len() + (text.len() >> 6));
                for r in ranges {
                    out.push_str(&self.convert_by_union(
                        &chars[r.start..r.end],
                        dictionaries,
                        max_word_length,
                        union,
                    ));
                }
                return out;
            }

            let parts: Vec<String> = ranges
                .par_chunks(chunk_ranges) // zero-copy chunks of &\[Range\]
                .map(|chunk| {
                    // sequential inside each chunk
                    let cap: usize = chunk.iter().map(|r| r.end - r.start).sum();
                    let mut s = String::with_capacity(cap);
                    for r in chunk {
                        s.push_str(&self.convert_by_union(
                            &chars[r.start..r.end],
                            dictionaries,
                            max_word_length,
                            union,
                        ));
                    }
                    s
                })
                .collect();

            parts.concat() // exact single allocation
        } else {
            let mut out = String::with_capacity(text.len() + (text.len() >> 6));
            for r in ranges {
                out.push_str(&self.convert_by_union(
                    &chars[r.start..r.end],
                    dictionaries,
                    max_word_length,
                    union,
                ));
            }
            out
        }
    }

    /// Core dictionary‑matching routine (FMM) optimized by a precomputed **starter union**.
    ///
    /// This is the tightest loop of the segment‑replacement engine. It scans a delimiter‑free
    /// `&[char]` left‑to‑right using **Forward Maximum Matching (FMM)**, while a prebuilt
    /// [`StarterUnion`] (bitmasks + per‑starter caps) prunes impossible lengths before any
    /// per‑dictionary lookup.
    ///
    /// Compared to `convert_by()`:
    /// - Uses `union.bmp_mask/cap` (BMP) and `union.astral_mask/cap` (astral) to **prune lengths**
    ///   before probing dictionaries.
    /// - Tries viable lengths in **descending order** via [`for_each_len_dec`]; the first hit wins.
    ///
    /// # Matching strategy
    /// For each `start_pos`:
    /// 1. Compute `cap_here = min(max_word_length, remaining, union_cap_for_starter)`.
    /// 2. Enumerate **only viable lengths** (longest → shortest) using the union’s bitmask/cap.
    /// 3. For each viable `length`, probe each dictionary **only if** that dict can host such a key
    ///    (checked against `dict.max_len` and the dict’s own per‑starter cap).
    /// 4. On the first match, emit replacement and advance by `length`.
    /// 5. If no match, emit the current char and advance by 1.
    ///
    /// # Arguments
    /// - `text_chars`: Non‑delimited slice of `char` (a single segment).
    /// - `dictionaries`: Dictionaries to consult (probe order = precedence).
    /// - `max_word_length`: Global cap for match length in chars (e.g., 16).
    /// - `union`: Precomputed [`StarterUnion`] built from **exactly** these `dictionaries`.
    ///
    /// # Returns
    /// Converted segment as a `String`.
    ///
    /// # Requirements
    /// - `union` **must** be built from the same set/content of `dictionaries` (rebuild if they change).
    /// - Each [`DictMaxLen`] has populated starter indexes
    ///   (e.g., via [`DictMaxLen::build_from_pairs`] or `populate_starter_indexes`).
    ///
    /// # Performance notes
    /// - Union pruning avoids per‑dict checks for impossible starters/lengths.
    /// - Longest‑first, first‑hit‑wins often exits early on common phrases.
    /// - BMP starters use O(1) array lookups; astral starters use sparse maps.
    ///
    /// # Complexity
    /// Let *N* be the segment length, *D* the number of dictionaries.
    /// Typical: `O(N · K · D)` where `K ≤ 64` viable lengths per position after pruning
    /// (often much smaller due to early exits).
    ///
    /// # Example (internal)
    /// ```ignore
    /// use opencc_fmmseg::{DictMaxLen, StarterUnion};
    ///
    /// let d1 = DictMaxLen::build_from_pairs(vec![("你好".into(), "您好".into())]);
    /// let d2 = DictMaxLen::build_from_pairs(vec![("世界".into(), "世間".into())]);
    /// let dicts: [&DictMaxLen; 2] = [&d1, &d2];
    /// let union = StarterUnion::build(&dicts);
    ///
    /// // Given a delimiter‑free segment `text_chars`:
    /// // let out = opencc.convert_by_union(&text_chars, &dicts, 16, &union);
    /// ```
    ///
    /// # Safety & invariants
    /// - Slices are only formed within `start_pos..start_pos+length` after ensuring bounds (`length ≤ remaining`).
    /// - `text_chars` is immutable and alive for the duration; aliasing multiple immutable slices is safe.
    /// - CAP (≥64) semantics are enforced by [`for_each_len_dec`].
    #[inline(always)]
    pub fn convert_by_union(
        &self,
        text_chars: &[char],
        dictionaries: &[&DictMaxLen],
        max_word_length: usize,
        union: &StarterUnion,
    ) -> String {
        if text_chars.is_empty() {
            return String::new();
        }

        let text_length = text_chars.len();
        if text_length == 1 && is_delimiter(text_chars[0]) {
            return text_chars[0].to_string();
        }

        let is_multi_dicts = dictionaries.len() > 1;
        // const CAP_BIT: usize = 63;
        let mut result = String::with_capacity(text_length * 4);
        let mut start_pos = 0;

        while start_pos < text_length {
            let c0 = text_chars[start_pos];
            let u0 = c0 as u32;
            let rem = text_length - start_pos;
            let global_cap = max_word_length.min(rem);

            // Pull precomputed mask + cap
            let (mask, cap_u8) = if u0 <= 0xFFFF {
                let idx = u0 as usize;
                (union.bmp_mask[idx], union.bmp_cap[idx])
            } else {
                (
                    *union.astral_mask.get(&c0).unwrap_or(&0),
                    *union.astral_cap.get(&c0).unwrap_or(&0),
                )
            };

            if mask == 0 || cap_u8 == 0 {
                result.push(c0);
                start_pos += 1;
                continue;
            }

            let cap_here = global_cap.min(cap_u8 as usize);
            let mut matched = false;

            let text_ptr = text_chars.as_ptr();
            // starter is the first scalar at start_pos
            // let starter = unsafe { *text_ptr.add(start_pos) };

            utils::for_each_len_dec(mask, cap_here, |length| {
                // precompute once per length
                let cap_bit = if length >= 64 { 63 } else { length - 1 };
                // sentinel: no slice yet
                let mut data_ptr: *const char = std::ptr::null();
                let mut data_len: usize = 0;

                // precompute starter tests, etc.

                for &dict in dictionaries {
                    if !dict.has_key_len(length) {
                        continue;
                    }
                    // ... starter-cap gates ...
                    // 2) per-dict starter gate (uses DictMaxLen fields):
                    if is_multi_dicts {
                        if !dict.starter_allows_dict(c0, length, cap_bit) {
                            continue;
                        }
                    }
                    // Build the slice once per `length`
                    if data_ptr.is_null() {
                        debug_assert!(start_pos < text_length);
                        debug_assert!(length <= text_length - start_pos);
                        data_ptr = unsafe { text_ptr.add(start_pos) };
                        data_len = length;
                    }

                    // Materialize the fat slice only here
                    let slice: &[char] = unsafe { std::slice::from_raw_parts(data_ptr, data_len) };

                    if let Some(val) = dict.map.get(slice) {
                        result.push_str(val);
                        start_pos += length;
                        matched = true;
                        return true;
                    }
                }

                false
            });

            if !matched {
                result.push(c0);
                start_pos += 1;
            }
        }

        result
    }

    /// Converts text using the given dictionaries with **greedy maximum-match**,
    /// without relying on a precomputed [`StarterUnion`].
    ///
    /// # Algorithm
    ///
    /// - At each position, tries the longest possible slice (up to `max_word_length`).
    /// - Scans dictionaries in order; if a match is found, emits the mapped value
    ///   and advances by that length.
    /// - If no dictionary matches, emits the current character as-is and advances by 1.
    ///
    /// # Performance
    ///
    /// - Simpler but slower than [`convert_by_union`], since every length from
    ///   `max_word_length..=1` must be checked at runtime.
    /// - Useful when:
    ///   - Only single-character dictionaries are applied (e.g. `st`, `ts`);
    ///   - You don’t want to build a [`StarterUnion`] upfront.
    ///
    /// # Parameters
    /// - `text_chars`: Input text, pre-split into `chars.
    /// - `dictionaries`: Slice of dictionary references (`DictMaxLen`).
    /// - `max_word_length`: Maximum phrase length across the dictionaries.
    ///
    /// # Returns
    /// A new [`String`] containing the converted text.
    ///
    /// # See also
    /// - [`convert_by_union`]: Optimized version that uses a [`StarterUnion`] mask/cap table.
    fn convert_by(
        &self,
        text_chars: &[char],
        dictionaries: &[&DictMaxLen],
        max_word_length: usize,
    ) -> String {
        if text_chars.is_empty() {
            return String::new();
        }

        let text_length = text_chars.len();
        if text_length == 1 && is_delimiter(text_chars[0]) {
            return text_chars[0].to_string();
        }

        let mut result = String::with_capacity(text_length * 4);
        let mut start_pos = 0;

        while start_pos < text_length {
            let max_length = max_word_length.min(text_length - start_pos);
            let mut best_match_length = 0usize;
            let mut best_match: &str = "";

            // greedy: try longest length first
            for length in (1..=max_length).rev() {
                let candidate = &text_chars[start_pos..start_pos + length];

                for dictionary in dictionaries {
                    if !dictionary.has_key_len(length) {
                        continue;
                    }
                    if let Some(value) = dictionary.map.get(candidate) {
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
                // no dictionary hit: emit single char and move on
                result.push(text_chars[start_pos]);
                start_pos += 1;
                continue;
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
        let mut round_1: Vec<&DictMaxLen> =
            vec![&self.dictionary.st_phrases, &self.dictionary.st_characters];

        if punctuation {
            round_1.push(&self.dictionary.st_punctuations);
        }

        let union = self
            .dictionary
            .union_for(UnionKey::S2T { punct: punctuation });

        DictRefs::new(&round_1, union).apply_segment_replace(
            input,
            |input, refs, max_len, union| {
                self.segment_replace_with_union(input, refs, max_len, union)
            },
        )
    }

    /// Converts Traditional Chinese text to Simplified Chinese (T2S).
    ///
    /// This method performs dictionary-based segment replacement using:
    /// - Phrase-level mappings (`ts_phrases`)
    /// - Character-level mappings (`ts_characters`)
    ///
    /// If `punctuation` is `true`, an additional punctuation-level dictionary
    /// (`ts_punctuations`) is also applied. The input is first split by
    /// configured delimiters, then each non-delimiter segment is processed
    /// using a longest-match strategy over the configured dictionaries.
    ///
    /// As with [`OpenCC::s2t`], this uses the shared `DictRefs` and
    /// `StarterUnion` metadata and may run in parallel depending on the
    /// `OpenCC` configuration.
    ///
    /// # Arguments
    ///
    /// * `input` - Traditional Chinese text to convert.
    /// * `punctuation` - Whether to convert punctuation symbols in addition to
    ///   phrases and characters.
    ///
    /// # Returns
    ///
    /// Simplified Chinese text obtained after applying all mappings.
    pub fn t2s(&self, input: &str, punctuation: bool) -> String {
        let mut round_1: Vec<&DictMaxLen> =
            vec![&self.dictionary.ts_phrases, &self.dictionary.ts_characters];

        if punctuation {
            round_1.push(&self.dictionary.ts_punctuations);
        }

        let union = self
            .dictionary
            .union_for(UnionKey::T2S { punct: punctuation });

        DictRefs::new(&round_1, union).apply_segment_replace(
            input,
            |input, refs, max_len, union| {
                self.segment_replace_with_union(input, refs, max_len, union)
            },
        )
    }

    /// Converts Simplified Chinese text to Taiwanese Traditional (S → T → Tw).
    ///
    /// This method performs a **two-round** dictionary-based conversion:
    ///
    /// 1. **Round 1 (S2T core)**
    ///    Applies Simplified-to-Traditional mappings using:
    ///    - Phrase-level mappings (`st_phrases`)
    ///    - Character-level mappings (`st_characters`)
    ///    - Optionally punctuation-level mappings (`st_punctuations`) when
    ///      `punctuation` is `true`
    ///
    /// 2. **Round 2 (Taiwan-specific variants)**
    ///    Refines the Traditional output into Taiwanese-specific forms using:
    ///    - Taiwanese variant mappings (`tw_variants`)
    ///
    /// Internally this uses precomputed starter metadata from `union_for`
    /// (via `UnionKey::S2T` and `UnionKey::TwVariantsOnly`) and runs over
    /// segmented input using longest-match replacement.
    ///
    /// # Arguments
    ///
    /// * `input` - Simplified Chinese text to convert.
    /// * `punctuation` - Whether to convert punctuation symbols in addition to
    ///   phrases and characters.
    ///
    /// # Returns
    ///
    /// Taiwanese Traditional Chinese text after applying both S2T and
    /// Taiwanese variant mappings.
    pub fn s2tw(&self, input: &str, punctuation: bool) -> String {
        let mut round_1: Vec<&DictMaxLen> =
            vec![&self.dictionary.st_phrases, &self.dictionary.st_characters];

        if punctuation {
            round_1.push(&self.dictionary.st_punctuations);
        }

        let u1 = self
            .dictionary
            .union_for(UnionKey::S2T { punct: punctuation });
        let round_2 = [&self.dictionary.tw_variants];
        let u2 = self.dictionary.union_for(UnionKey::TwVariantsOnly);

        DictRefs::new(&round_1, u1)
            .with_round_2(&round_2, u2)
            .apply_segment_replace(input, |input, refs, max_len, union| {
                self.segment_replace_with_union(input, refs, max_len, union)
            })
    }

    /// Converts Taiwanese Traditional text to Simplified Chinese (Tw → T → S).
    ///
    /// This method performs a **two-round** dictionary-based conversion that
    /// reverses the `s2tw` pipeline:
    ///
    /// 1. **Round 1 (Taiwanese variant normalization)**
    ///    Maps Taiwanese-specific variants back to general Traditional using:
    ///    - Phrase-level reverse mappings (`tw_variants_rev_phrases`)
    ///    - Character-level reverse mappings (`tw_variants_rev`)
    ///
    /// 2. **Round 2 (T2S core)**
    ///    Converts the normalized Traditional text to Simplified Chinese using:
    ///    - Phrase-level mappings (`ts_phrases`)
    ///    - Character-level mappings (`ts_characters`)
    ///    - Optionally punctuation-level mappings (`ts_punctuations`) when
    ///      `punctuation` is `true`
    ///
    /// Starter metadata is obtained from `union_for` (via `UnionKey::TwRevPair`
    /// and `UnionKey::T2S`) and reused across segments for efficient longest-match replacement.
    ///
    /// # Arguments
    ///
    /// * `input` - Taiwanese Traditional Chinese text to convert.
    /// * `punctuation` - Whether to convert punctuation symbols in addition to
    ///   phrases and characters.
    ///
    /// # Returns
    ///
    /// Simplified Chinese text after normalizing Taiwanese variants and
    /// applying T2S mappings.
    pub fn tw2s(&self, input: &str, punctuation: bool) -> String {
        let mut round_2: Vec<&DictMaxLen> =
            vec![&self.dictionary.ts_phrases, &self.dictionary.ts_characters];

        if punctuation {
            round_2.push(&self.dictionary.ts_punctuations);
        }

        let u1 = self.dictionary.union_for(UnionKey::TwRevPair);
        let u2 = self
            .dictionary
            .union_for(UnionKey::T2S { punct: punctuation });

        DictRefs::new(
            &[
                &self.dictionary.tw_variants_rev_phrases,
                &self.dictionary.tw_variants_rev,
            ],
            u1,
        )
        .with_round_2(&round_2, u2)
        .apply_segment_replace(input, |input, refs, max_len, union| {
            self.segment_replace_with_union(input, refs, max_len, union)
        })
    }

    /// Converts Simplified Chinese text to Taiwanese Traditional with idioms (S → T → Tw-phrases → Tw).
    ///
    /// This method performs a **three-round** dictionary-based conversion:
    ///
    /// 1. **Round 1 (S2T core)**
    ///    Applies Simplified-to-Traditional mappings using:
    ///    - Phrase-level mappings (`st_phrases`)
    ///    - Character-level mappings (`st_characters`)
    ///    - Optionally punctuation-level mappings (`st_punctuations`) when
    ///      `punctuation` is `true`
    ///
    /// 2. **Round 2 (Taiwan-specific idioms and phrases)**
    ///    Adjusts the Traditional text into Taiwanese-style idioms and wordings
    ///    using:
    ///    - Taiwanese phrase mappings (`tw_phrases`)
    ///
    /// 3. **Round 3 (Taiwanese variant characters)**
    ///    Refines characters into Taiwanese variant forms using:
    ///    - Taiwanese variant mappings (`tw_variants`)
    ///
    /// All three rounds share precomputed starter metadata obtained via
    /// `union_for` (`UnionKey::S2T`, `UnionKey::TwPhrasesOnly`,
    /// `UnionKey::TwVariantsOnly`) and run over segmented input with
    /// longest-match replacement for high throughput.
    ///
    /// # Arguments
    ///
    /// * `input` - Simplified Chinese text to convert.
    /// * `punctuation` - Whether to convert punctuation symbols alongside
    ///   phrases and characters.
    ///
    /// # Returns
    ///
    /// Taiwanese Traditional Chinese text with idioms and variants applied.
    pub fn s2twp(&self, input: &str, punctuation: bool) -> String {
        // Create bindings for each round of dictionary references
        let mut round_1: Vec<&DictMaxLen> =
            vec![&self.dictionary.st_phrases, &self.dictionary.st_characters];

        if punctuation {
            round_1.push(&self.dictionary.st_punctuations);
        }

        let u1 = self
            .dictionary
            .union_for(UnionKey::S2T { punct: punctuation });

        let round_2 = [&self.dictionary.tw_phrases];
        let u2 = self.dictionary.union_for(UnionKey::TwPhrasesOnly);

        let round_3 = [&self.dictionary.tw_variants];
        let u3 = self.dictionary.union_for(UnionKey::TwVariantsOnly);

        // Use the DictRefs struct to handle 3 rounds
        DictRefs::new(&round_1, u1)
            .with_round_2(&round_2, u2)
            .with_round_3(&round_3, u3)
            .apply_segment_replace(input, |input, refs, max_len, union| {
                self.segment_replace_with_union(input, refs, max_len, union)
            })
    }

    /// Converts Taiwanese Traditional text with idioms to Simplified Chinese (Tw-phrases → T → S).
    ///
    /// This method reverses the `s2twp` pipeline using a **two-round** conversion:
    ///
    /// 1. **Round 1 (Taiwan-specific idiom and variant normalization)**
    ///    Normalizes Taiwanese-style idioms and variants back to general
    ///    Traditional forms using:
    ///    - Reverse Taiwanese idiom/phrase mappings (`tw_phrases_rev`)
    ///    - Reverse Taiwanese variant phrase mappings (`tw_variants_rev_phrases`)
    ///    - Reverse Taiwanese variant character mappings (`tw_variants_rev`)
    ///
    /// 2. **Round 2 (T2S core)**
    ///    Converts the normalized Traditional text to Simplified Chinese using:
    ///    - Phrase-level mappings (`ts_phrases`)
    ///    - Character-level mappings (`ts_characters`)
    ///    - Optionally punctuation-level mappings (`ts_punctuations`) when
    ///      `punctuation` is `true`
    ///
    /// Starter metadata is provided by `union_for` with
    /// `UnionKey::Tw2SpR1TwRevTriple` for the first round and
    /// `UnionKey::T2S` for the second round, enabling efficient
    /// longest-match replacement across segments.
    ///
    /// # Arguments
    ///
    /// * `input` - Taiwanese Traditional Chinese text (with idioms) to convert.
    /// * `punctuation` - Whether to convert punctuation symbols alongside
    ///   phrases and characters.
    ///
    /// # Returns
    ///
    /// Simplified Chinese text after normalizing Taiwanese idioms and
    /// applying T2S mappings.
    pub fn tw2sp(&self, input: &str, punctuation: bool) -> String {
        let round_1 = [
            &self.dictionary.tw_phrases_rev,
            &self.dictionary.tw_variants_rev_phrases,
            &self.dictionary.tw_variants_rev,
        ];
        let u1 = self.dictionary.union_for(UnionKey::Tw2SpR1TwRevTriple);
        let mut round_2: Vec<&DictMaxLen> =
            vec![&self.dictionary.ts_phrases, &self.dictionary.ts_characters];

        if punctuation {
            round_2.push(&self.dictionary.ts_punctuations);
        }
        let u2 = self
            .dictionary
            .union_for(UnionKey::T2S { punct: punctuation });

        DictRefs::new(&round_1, u1)
            .with_round_2(&round_2, u2)
            .apply_segment_replace(input, |input, refs, max_len, union| {
                self.segment_replace_with_union(input, refs, max_len, union)
            })
    }

    /// Converts Simplified Chinese text to Hong Kong Traditional (S → T → HK).
    ///
    /// This method performs a **two-round** dictionary-based conversion:
    ///
    /// 1. **Round 1 (S2T core)**
    ///    Applies Simplified-to-Traditional mappings using:
    ///    - Phrase-level mappings (`st_phrases`)
    ///    - Character-level mappings (`st_characters`)
    ///    - Optionally punctuation-level mappings (`st_punctuations`) when
    ///      `punctuation` is `true`
    ///
    /// 2. **Round 2 (Hong Kong-specific variants)**
    ///    Refines the Traditional output into Hong Kong–specific forms using:
    ///    - Hong Kong variant mappings (`hk_variants`)
    ///
    /// Both rounds reuse precomputed starter metadata obtained from `union_for`
    /// (`UnionKey::S2T` and `UnionKey::HkVariantsOnly`) and operate on
    /// segmented input with longest-match replacement.
    ///
    /// # Arguments
    ///
    /// * `input` - Simplified Chinese text to convert.
    /// * `punctuation` - Whether to convert punctuation symbols in addition to
    ///   phrases and characters.
    ///
    /// # Returns
    ///
    /// Hong Kong Traditional Chinese text after applying S2T and HK variant
    /// mappings.
    pub fn s2hk(&self, input: &str, punctuation: bool) -> String {
        let mut round_1: Vec<&DictMaxLen> =
            vec![&self.dictionary.st_phrases, &self.dictionary.st_characters];

        if punctuation {
            round_1.push(&self.dictionary.st_punctuations);
        }
        let u1 = self
            .dictionary
            .union_for(UnionKey::S2T { punct: punctuation });
        let round_2 = [&self.dictionary.hk_variants];
        let u2 = self.dictionary.union_for(UnionKey::HkVariantsOnly);
        DictRefs::new(&round_1, u1)
            .with_round_2(&round_2, u2)
            .apply_segment_replace(input, |input, refs, max_len, union| {
                self.segment_replace_with_union(input, refs, max_len, union)
            })
    }

    /// Converts Hong Kong Traditional text to Simplified Chinese (HK → T → S).
    ///
    /// This method reverses the `s2hk` pipeline using a **two-round**
    /// dictionary-based conversion:
    ///
    /// 1. **Round 1 (HK variant normalization)**
    ///    Normalizes Hong Kong–specific forms back to general Traditional using:
    ///    - Reverse HK variant phrase mappings (`hk_variants_rev_phrases`)
    ///    - Reverse HK variant character mappings (`hk_variants_rev`)
    ///
    /// 2. **Round 2 (T2S core)**
    ///    Converts the normalized Traditional text to Simplified Chinese using:
    ///    - Phrase-level mappings (`ts_phrases`)
    ///    - Character-level mappings (`ts_characters`)
    ///    - Optionally punctuation-level mappings (`ts_punctuations`) when
    ///      `punctuation` is `true`
    ///
    /// Starter metadata is provided via `union_for` using `UnionKey::HkRevPair`
    /// and `UnionKey::T2S`, enabling efficient longest-match replacement.
    ///
    /// # Arguments
    ///
    /// * `input` - Hong Kong Traditional Chinese text to convert.
    /// * `punctuation` - Whether to convert punctuation symbols in addition to
    ///   phrases and characters.
    ///
    /// # Returns
    ///
    /// Simplified Chinese text after normalizing HK variants and applying T2S
    /// mappings.
    pub fn hk2s(&self, input: &str, punctuation: bool) -> String {
        let round_1 = [
            &self.dictionary.hk_variants_rev_phrases,
            &self.dictionary.hk_variants_rev,
        ];
        let u1 = self.dictionary.union_for(UnionKey::HkRevPair);
        let mut round_2: Vec<&DictMaxLen> =
            vec![&self.dictionary.ts_phrases, &self.dictionary.ts_characters];

        if punctuation {
            round_2.push(&self.dictionary.ts_punctuations);
        }
        let u2 = self
            .dictionary
            .union_for(UnionKey::T2S { punct: punctuation });
        DictRefs::new(&round_1, u1)
            .with_round_2(&round_2, u2)
            .apply_segment_replace(input, |input, refs, max_len, union| {
                self.segment_replace_with_union(input, refs, max_len, union)
            })
    }

    /// Converts general Traditional Chinese text to Taiwanese Traditional variants (T → Tw).
    ///
    /// This method performs a single-round dictionary-based conversion that
    /// rewrites general Traditional forms into Taiwanese-specific variants
    /// using:
    ///
    /// - Taiwanese variant mappings (`tw_variants`)
    ///
    /// Starter metadata is obtained via `union_for(UnionKey::TwVariantsOnly)`
    /// and applied over segmented input with longest-match replacement.
    ///
    /// # Arguments
    ///
    /// * `input` - Traditional Chinese text to convert into Taiwanese variants.
    ///
    /// # Returns
    ///
    /// Taiwanese Traditional Chinese text with character/word forms adjusted
    /// according to `tw_variants`.
    pub fn t2tw(&self, input: &str) -> String {
        let round_1 = [&self.dictionary.tw_variants];
        let u1 = self.dictionary.union_for(UnionKey::TwVariantsOnly);
        let output = DictRefs::new(&round_1, u1).apply_segment_replace(
            input,
            |input, refs, max_len, union| {
                self.segment_replace_with_union(input, refs, max_len, union)
            },
        );

        output
    }

    /// Converts general Traditional Chinese text to Taiwanese Traditional with idioms (T → Tw-phrases → Tw).
    ///
    /// This method performs a two-round dictionary-based conversion:
    ///
    /// 1. **Round 1 (Taiwanese idioms and phrases)**
    ///    Applies Taiwanese-style idiom and phrase mappings:
    ///    - Taiwanese phrase mappings (`tw_phrases`)
    ///
    /// 2. **Round 2 (Taiwanese variant characters)**
    ///    Further adjusts the result into Taiwanese character variants:
    ///    - Taiwanese variant mappings (`tw_variants`)
    ///
    /// Both rounds use precomputed starter metadata from `union_for`
    /// (`UnionKey::TwPhrasesOnly` and `UnionKey::TwVariantsOnly`) and run
    /// over segmented input with longest-match replacement.
    ///
    /// # Arguments
    ///
    /// * `input` - Traditional Chinese text to convert into Taiwanese idiomatic
    ///   and variant forms.
    ///
    /// # Returns
    ///
    /// Taiwanese Traditional Chinese text with both idioms and variants applied.
    pub fn t2twp(&self, input: &str) -> String {
        let round_1 = [&self.dictionary.tw_phrases];
        let u1 = self.dictionary.union_for(UnionKey::TwPhrasesOnly);
        let round_2 = [&self.dictionary.tw_variants];
        let u2 = self.dictionary.union_for(UnionKey::TwVariantsOnly);
        let output = DictRefs::new(&round_1, u1)
            .with_round_2(&round_2, u2)
            .apply_segment_replace(input, |input, refs, max_len, union| {
                self.segment_replace_with_union(input, refs, max_len, union)
            });

        output
    }

    /// Converts Taiwanese Traditional text to general Traditional (Tw → T).
    ///
    /// This method performs a single-round dictionary-based normalization that
    /// maps Taiwanese-specific variants back to general Traditional Chinese
    /// using:
    ///
    /// - Reverse Taiwanese variant phrase mappings (`tw_variants_rev_phrases`)
    /// - Reverse Taiwanese variant character mappings (`tw_variants_rev`)
    ///
    /// Starter metadata is obtained via `union_for(UnionKey::TwRevPair)` and
    /// applied over segmented input with longest-match replacement.
    ///
    /// # Arguments
    ///
    /// * `input` - Taiwanese Traditional Chinese text to normalize.
    ///
    /// # Returns
    ///
    /// General Traditional Chinese text with Taiwanese variants normalized.
    pub fn tw2t(&self, input: &str) -> String {
        let round_1 = [
            &self.dictionary.tw_variants_rev_phrases,
            &self.dictionary.tw_variants_rev,
        ];
        let u1 = self.dictionary.union_for(UnionKey::TwRevPair);

        let output = DictRefs::new(&round_1, u1).apply_segment_replace(
            input,
            |input, refs, max_len, union| {
                self.segment_replace_with_union(input, refs, max_len, union)
            },
        );

        output
    }

    /// This method performs a two-round dictionary-based normalization:
    ///
    /// 1. **Round 1 (variant normalization)**
    ///    Normalizes Taiwanese variants back to general Traditional using:
    ///    - Reverse Taiwanese variant phrase mappings (`tw_variants_rev_phrases`)
    ///    - Reverse Taiwanese variant character mappings (`tw_variants_rev`)
    ///
    /// 2. **Round 2 (idiom/phrase normalization)**
    ///    Normalizes Taiwanese-specific idioms and phrases using:
    ///    - Reverse Taiwanese phrase mappings (`tw_phrases_rev`)
    ///
    /// Starter metadata is obtained from `union_for(UnionKey::TwRevPair)` for
    /// the first round and `union_for(UnionKey::TwPhrasesRevOnly)` for the
    /// second round, and is reused across segments for efficient longest-match
    /// replacement.
    ///
    /// # Arguments
    ///
    /// * `input` - Taiwanese Traditional Chinese text (including idioms) to
    ///   normalize.
    ///
    /// # Returns
    ///
    /// General Traditional Chinese text with both Taiwanese variants and
    /// idioms normalized.
    pub fn tw2tp(&self, input: &str) -> String {
        let round_1 = [
            &self.dictionary.tw_variants_rev_phrases,
            &self.dictionary.tw_variants_rev,
        ];
        let u1 = self.dictionary.union_for(UnionKey::TwRevPair);

        let round_2 = [&self.dictionary.tw_phrases_rev];
        let u2 = self.dictionary.union_for(UnionKey::TwPhrasesRevOnly);

        let output = DictRefs::new(&round_1, u1)
            .with_round_2(&round_2, u2)
            .apply_segment_replace(input, |input, refs, max_len, union| {
                self.segment_replace_with_union(input, refs, max_len, union)
            });

        output
    }

    /// Converts general Traditional Chinese text to Hong Kong Traditional variants (T → HK).
    ///
    /// This method performs a single-round dictionary-based conversion that
    /// rewrites general Traditional forms into Hong Kong–specific variants
    /// using:
    ///
    /// - Hong Kong variant mappings (`hk_variants`)
    ///
    /// Starter metadata is obtained via `union_for(UnionKey::HkVariantsOnly)`
    /// and applied over segmented input with longest-match replacement.
    ///
    /// # Arguments
    ///
    /// * `input` - Traditional Chinese text to convert into Hong Kong variants.
    ///
    /// # Returns
    ///
    /// Hong Kong Traditional Chinese text with character/word forms adjusted
    /// according to `hk_variants`.
    pub fn t2hk(&self, input: &str) -> String {
        let round_1 = [&self.dictionary.hk_variants];
        let u1 = self.dictionary.union_for(UnionKey::HkVariantsOnly);
        let output = DictRefs::new(&round_1, u1).apply_segment_replace(
            input,
            |input, refs, max_len, union| {
                self.segment_replace_with_union(input, refs, max_len, union)
            },
        );

        output
    }

    /// Converts Hong Kong Traditional text to general Traditional (HK → T).
    ///
    /// This method performs a single-round dictionary-based normalization that
    /// maps Hong Kong–specific variants back to general Traditional Chinese
    /// using:
    ///
    /// - Reverse HK variant phrase mappings (`hk_variants_rev_phrases`)
    /// - Reverse HK variant character mappings (`hk_variants_rev`)
    ///
    /// Starter metadata is obtained via `union_for(UnionKey::HkRevPair)` and
    /// reused across segments for efficient longest-match replacement.
    ///
    /// # Arguments
    ///
    /// * `input` - Hong Kong Traditional Chinese text to normalize.
    ///
    /// # Returns
    ///
    /// General Traditional Chinese text with Hong Kong variants normalized.
    pub fn hk2t(&self, input: &str) -> String {
        let round_1 = [
            &self.dictionary.hk_variants_rev_phrases,
            &self.dictionary.hk_variants_rev,
        ];
        let u1 = self.dictionary.union_for(UnionKey::HkRevPair);
        let output = DictRefs::new(&round_1, u1).apply_segment_replace(
            input,
            |input, refs, max_len, union| {
                self.segment_replace_with_union(input, refs, max_len, union)
            },
        );

        output
    }

    /// Converts Japanese Kyūjitai (traditional kanji forms) to Shinjitai.
    ///
    /// This method performs a single-round dictionary-based conversion that
    /// rewrites Kyūjitai-style characters into their modern Shinjitai forms
    /// using:
    ///
    /// - Japanese variant mappings (`jp_variants`)
    ///
    /// Starter metadata is obtained via `union_for(UnionKey::JpVariantsOnly)`
    /// and applied over segmented input with longest-match replacement.
    ///
    /// # Arguments
    ///
    /// * `input` - Text containing Kyūjitai-style characters to convert.
    ///
    /// # Returns
    ///
    /// Text where Kyūjitai characters have been replaced with their
    /// corresponding Shinjitai forms.
    pub fn t2jp(&self, input: &str) -> String {
        let round_1 = [&self.dictionary.jp_variants];
        let u1 = self.dictionary.union_for(UnionKey::JpVariantsOnly);
        let output = DictRefs::new(&round_1, u1).apply_segment_replace(
            input,
            |input, refs, max_len, union| {
                self.segment_replace_with_union(input, refs, max_len, union)
            },
        );

        output
    }

    /// Converts Japanese Shinjitai to Kyūjitai (modern → traditional kanji forms).
    ///
    /// This method performs a single-round dictionary-based conversion that
    /// maps modern Shinjitai forms back to their Kyūjitai or Traditional
    /// Chinese equivalents using:
    ///
    /// - Japanese Shinjitai phrase mappings (`jps_phrases`)
    /// - Japanese Shinjitai character mappings (`jps_characters`)
    /// - Reverse Japanese variant mappings (`jp_variants_rev`)
    ///
    /// Starter metadata is obtained via `union_for(UnionKey::JpRevTriple)` and
    /// reused across segments for efficient longest-match replacement.
    ///
    /// # Arguments
    ///
    /// * `input` - Text containing Shinjitai characters to convert.
    ///
    /// # Returns
    ///
    /// Text where Shinjitai forms are converted back to Kyūjitai or their
    /// corresponding Traditional forms.
    pub fn jp2t(&self, input: &str) -> String {
        let round_1 = [
            &self.dictionary.jps_phrases,
            &self.dictionary.jps_characters,
            &self.dictionary.jp_variants_rev,
        ];
        let u1 = self.dictionary.union_for(UnionKey::JpRevTriple);
        let output = DictRefs::new(&round_1, u1).apply_segment_replace(
            input,
            |input, refs, max_len, union| {
                self.segment_replace_with_union(input, refs, max_len, union)
            },
        );

        output
    }

    /// Converts Chinese text using a configuration name (`&str`, case-insensitive).
    ///
    /// This is a **convenience / legacy** entry point that accepts OpenCC-style config names
    /// such as `"s2t"` or `"t2s"`. Internally it parses the string into [`OpenccConfig`]
    /// and dispatches to [`OpenCC::convert_with_config`].
    ///
    /// Prefer [`OpenCC::convert_with_config`] if you want:
    /// - no string parsing
    /// - compile-time configuration selection
    /// - an API that mirrors your C FFI numeric config
    ///
    /// # Arguments
    ///
    /// * `input` - UTF-8 text to convert.
    /// * `config` - Configuration name (case-insensitive), e.g. `"s2t"`.
    /// * `punctuation` - Whether to apply punctuation conversion where supported.
    ///   For some configs, this parameter is **ignored** because their conversion
    ///   pipeline does not include punctuation normalization.
    ///
    /// # Returns
    ///
    /// Returns the converted text. If `config` is invalid, it returns the string
    /// `"Invalid config: {config}"` and stores the same message in the last-error slot.
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
    pub fn convert(&self, input: &str, config: &str, punctuation: bool) -> String {
        match OpenccConfig::try_from(config) {
            Ok(cfg) => self.convert_with_config(input, cfg, punctuation),
            Err(_) => {
                Self::set_last_error(&format!("Invalid config: {}", config));
                format!("Invalid config: {}", config)
            }
        }
    }

    /// Converts Chinese text using a strongly-typed [`OpenccConfig`].
    ///
    /// This method avoids string parsing and is the recommended API for Rust callers.
    /// It also maps cleanly to the C FFI numeric config (`opencc_config_t`).
    ///
    /// # Arguments
    ///
    /// * `input` - UTF-8 text to convert.
    /// * `config` - Conversion configuration.
    /// * `punctuation` - Whether to apply punctuation conversion where supported.
    ///   For some configs, this flag is **ignored** (see [`OpenccConfig`] table).
    ///
    /// # Example
    ///
    /// ```rust
    /// use opencc_fmmseg::{OpenCC, OpenccConfig};
    ///
    /// let converter = OpenCC::new();
    /// let out = converter.convert_with_config("汉字转换测试", OpenccConfig::S2t, false);
    /// assert_eq!(out, "漢字轉換測試");
    /// ```
    pub fn convert_with_config(
        &self,
        input: &str,
        config: OpenccConfig,
        punctuation: bool,
    ) -> String {
        match config {
            OpenccConfig::S2t => self.s2t(input, punctuation),
            OpenccConfig::S2tw => self.s2tw(input, punctuation),
            OpenccConfig::S2twp => self.s2twp(input, punctuation),
            OpenccConfig::S2hk => self.s2hk(input, punctuation),
            OpenccConfig::T2s => self.t2s(input, punctuation),
            OpenccConfig::T2tw => self.t2tw(input),
            OpenccConfig::T2twp => self.t2twp(input),
            OpenccConfig::T2hk => self.t2hk(input),
            OpenccConfig::Tw2s => self.tw2s(input, punctuation),
            OpenccConfig::Tw2sp => self.tw2sp(input, punctuation),
            OpenccConfig::Tw2t => self.tw2t(input),
            OpenccConfig::Tw2tp => self.tw2tp(input),
            OpenccConfig::Hk2s => self.hk2s(input, punctuation),
            OpenccConfig::Hk2t => self.hk2t(input),
            OpenccConfig::Jp2t => self.jp2t(input),
            OpenccConfig::T2jp => self.t2jp(input),
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
        let chars: Vec<char> = input.chars().collect();
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
        let chars: Vec<char> = input.chars().collect();
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
        // pick the smaller of (1000, stripped length)
        let check_len = utils::find_max_utf8_length(input, 1000);

        let _strip_text = STRIP_REGEX.replace_all(&input[..check_len], "");
        let max_bytes = utils::find_max_utf8_length(&_strip_text, 200);
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

    /// Converts a subset of Chinese quotation punctuation between Simplified
    /// and Traditional forms.
    ///
    /// This helper performs a simple regex-based replacement of four quote
    /// characters:
    ///
    /// - `“”‘’` (Simplified-style quotes)
    /// - `「」『』` (Traditional-style quotes)
    ///
    /// If `config` begins with `'s'`, the function converts Simplified quotes
    /// to Traditional forms. Otherwise, it performs the reverse mapping.
    ///
    /// This function is retained only for backward compatibility and is not
    /// used by the main OpenCC conversion pipeline, which relies on dictionary-based punctuation mappings instead.
    ///
    /// # Deprecated
    ///
    /// This punctuation converter is deprecated and should not be used in new
    /// code. It exists only to silence missing-docs warnings for legacy
    /// compatibility.
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

    /// Clears the most recently recorded OpenCC runtime error.
    ///
    /// This function resets the internal error state maintained by OpenCC.
    /// After calling this, [`get_last_error`](Self::get_last_error) will return `None`
    /// until a new error is recorded.
    ///
    /// ## Important
    ///
    /// - This function only clears the **internal error state**.
    /// - It does **not** free or affect any error strings previously returned
    ///   by the C API (e.g. via `opencc_last_error()`).
    /// - Clearing the error state is independent of memory management.
    ///
    /// In other words:
    ///
    /// - Use `clear_last_error()` to reset the error **status**.
    /// - Use the appropriate C API free function to release any allocated
    ///   error message buffers.
    ///
    /// ## Typical use cases
    ///
    /// - Resetting the error state after displaying an error to the user.
    /// - Ensuring a clean error state before starting a new conversion batch.
    /// - Avoiding stale error messages in long-running or interactive applications.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use opencc_fmmseg::OpenCC;
    ///
    /// // Record an error internally
    /// OpenCC::set_last_error("Invalid config");
    ///
    /// // Clear it
    /// OpenCC::clear_last_error();
    ///
    /// // No error remains
    /// assert!(OpenCC::get_last_error().is_none());
    /// ```
    /// # Since
    ///
    /// Available since **v0.8.4**.
    pub fn clear_last_error() {
        let mut last_error = LAST_ERROR.lock().unwrap();
        *last_error = None;
    }
}
