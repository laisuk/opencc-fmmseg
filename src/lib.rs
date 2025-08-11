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
use rustc_hash::{FxHashMap, FxHashSet};
use std::iter::Iterator;
use std::sync::Mutex;

/// Dictionary utilities for managing multiple OpenCC lexicons.
pub mod dictionary_lib;
use dictionary_lib::dict_max_len::DictMaxLen;
use dictionary_lib::DictionaryMaxlength;

/// Thread-safe holder for the last error message (if any).
static LAST_ERROR: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));
// const DELIMITERS: &'static str = " \t\n\r!\"#$%&'()*+,-./:;<=>?@[\\]^_{}|~＝、。﹁﹂—－（）《》〈〉？！…／＼︒︑︔︓︿﹀︹︺︙︐［﹇］﹈︕︖︰︳︴︽︾︵︶｛︷｝︸﹃﹄【︻】︼　～．，；：";
/// Regular expression used to normalize or strip punctuation from input.
static STRIP_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[!-/:-@\[-`{-~\t\n\v\f\r 0-9A-Za-z_著]").unwrap());

/// Defines different delimiter modes for segmenting input text.
///
/// - `Minimal`: Only line breaks (`\n`). Useful for structured formats like Markdown, chat logs, or CSV.
/// - `Normal`: Common Chinese sentence delimiters plus `\n`. Ideal for general text, balances split quality and performance.
/// - `Full`: A comprehensive set of Chinese/ASCII punctuation and whitespace. Suitable for high segmentation granularity.
#[derive(Debug, Clone, Copy)]
pub enum DelimiterMode {
    Minimal,
    Normal,
    Full,
}

/// Returns a [`FxHashSet<char>`] containing the delimiter characters for the given [`DelimiterMode`].
///
/// This is used during character-level segmentation in `get_chars_range()` to split text into logical units.
///
/// # Parameters
///
/// - `mode`: The delimiter mode to use (`Minimal`, `Normal`, or `Full`).
///
/// # Returns
///
/// A `FxHashSet<char>` containing all characters that should be treated as segment delimiters.
///
/// # Examples
///
/// ```
/// let delimiters = opencc_fmmseg::get_delimiters(opencc_fmmseg::DelimiterMode::Normal);
/// assert!(delimiters.contains(&'。'));
/// ```
pub fn get_delimiters(mode: DelimiterMode) -> FxHashSet<char> {
    match mode {
        DelimiterMode::Minimal => "\n",
        DelimiterMode::Normal => "，。！？\n",
        DelimiterMode::Full => " \t\n\r!\"#$%&'()*+,-./:;<=>?@[\\]^_{}|~＝、。﹁﹂—－（）《》〈〉？！…／＼︒︑︔︓︿﹀︹︺︙︐［﹇］﹈︕︖︰︳︴︽︾︵︶｛︷｝︸﹃﹄【︻】︼　～．，；：",
    }
        .chars()
        .collect()
}

/// Default set of delimiters used for text segmentation.
///
/// Initialized using [`DelimiterMode::Full`] for fine-grained splitting suitable for
/// parallel processing with [`rayon`] when processing long or mixed-language input.
///
/// This static value is used when no explicit mode is provided.
pub static DELIMITERS_DEFAULT: Lazy<FxHashSet<char>> =
    Lazy::new(|| get_delimiters(DelimiterMode::Full));

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

/// Iterates possible phrase lengths in **descending** order using a precomputed bitmask,
/// stopping early if the callback returns `true`.
///
/// - `mask` encodes which lengths exist for a given starter:
///   - bit 0  ⇒ length = 1
///   - bit 1  ⇒ length = 2
///   - ...
///   - bit 63 ⇒ length **≥ 64** (the “CAP” bucket)
/// - `cap_here` is the effective upper bound at the current position (e.g. `min(global_max, remaining)`).
/// - `f(len)` is called from longest to shortest; if it returns `true`, iteration stops.
///
/// This function first yields all lengths **> 64** (only if the CAP bit is set and `cap_here > 64`),
/// in the sequence `cap_here, cap_here-1, …, 64`. Then it processes the set bits for
/// `1..=min(64, cap_here)` in descending order.
///
/// # Notes
/// - If `cap_here == 64`, the CAP bit represents exactly length 64.
/// - If `cap_here > 64`, the CAP binary bit just signals that there exists a length ≥ 64; the function
///   will explicitly try `cap_here..=64` regardless of which exact ≥64 lengths are present in `mask`.
/// - This helper is typically fed by `first_len_mask64[c0]` and a per-starter `cap_here`.
#[inline]
fn for_each_len_dec(mask: u64, cap_here: usize, mut f: impl FnMut(usize) -> bool) {
    const CAP_BIT: usize = 63;
    if cap_here == 0 {
        return;
    }

    // 1) Handle lengths > 64 if CAP bit is set.
    if cap_here > 64 && (mask & (1u64 << CAP_BIT)) != 0 {
        let mut len = cap_here;
        loop {
            if f(len) {
                return;
            }
            if len == 64 {
                break;
            }
            len -= 1;
        }
    }

    // 2) Handle lengths 1..=min(64, cap_here) by walking set bits from high→low.
    let limit = cap_here.min(64);
    if limit == 0 {
        return;
    }

    // Avoid `(1u64 << 64)` overflow: when `limit == 64`, take all bits.
    let mut m = if limit == 64 {
        mask
    } else {
        mask & ((1u64 << limit) - 1)
    };

    // If we already tried 64+ above, drop CAP bit to avoid repeating 64.
    if cap_here > 64 {
        m &= !(1u64 << CAP_BIT);
    }

    while m != 0 {
        // highest set bit
        let bit = 63 - m.leading_zeros() as usize;
        let len = bit + 1; // 1..=64
        if f(len) {
            return;
        }
        m &= !(1u64 << bit);
    }
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
        let delimiters = DELIMITERS_DEFAULT.clone();
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
        let delimiters = DELIMITERS_DEFAULT.clone();
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
        let delimiters = DELIMITERS_DEFAULT.clone();
        let is_parallel = true;

        OpenCC {
            dictionary,
            delimiters,
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
            if self.delimiters.contains(ch) {
                if inclusive {
                    if i + 1 > start {
                        ranges.push(start..i + 1);
                    }
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

    /// Internal segment-replacement bridge that drives FMM conversion using a **starter union**.
    ///
    /// This function splits the input into delimiter-aware segments, then converts each segment
    /// independently via [`convert_by_union`]. A single [`StarterUnion`] is built **once per call**
    /// from the provided dictionaries and reused across all segments.
    ///
    /// # Pipeline
    /// 1. Collect the input into `Vec<char>` (parallel or sequential).
    /// 2. Use [`get_chars_range`] to compute non-delimited ranges.
    /// 3. Build a `StarterUnion` **once** from `dictionaries`.
    /// 4. For each segment, call [`convert_by_union`] with the prebuilt union.
    /// 5. Concatenate results in the original order (delimiters preserved).
    ///
    /// # Arguments
    /// - `text`: Source string.
    /// - `dictionaries`: The dictionaries to consult (probe order = precedence). Each must have
    ///   populated starter indexes (done by `DictMaxLen::build_from_pairs` or `populate_starter_indexes`).
    /// - `max_word_length`: Global cap for match length in chars (e.g., 16).
    ///
    /// # Parallelism
    /// If `self.is_parallel` is `true`:
    /// - Input chars are collected using a parallel iterator.
    /// - Each segment is converted in parallel (`into_par_iter()`).
    /// This can significantly improve throughput on large inputs with many segments.
    ///
    /// # Behavior
    /// - Delimiters are **not** transformed; they are preserved exactly.
    /// - Each segment (a contiguous non-delimiter run) is processed with FMM using only viable
    ///   lengths implied by the union’s bitmasks/caps.
    /// - First dictionary hit at the longest viable length wins (short-circuit).
    ///
    /// # Complexity
    /// Let *N* be the number of chars, *S* the number of segments, *D* the number of dictionaries:
    /// - Union build: `O(D · 65_536)` for BMP + sparse astral merge (once per call).
    /// - Conversion: Sum over segments of `O(len(segment) · K · D)` where `K ≤ 64` viable lengths
    ///   after union pruning; often less due to early exits.
    ///
    /// # Example (illustrative)
    /// ```ignore
    /// use opencc_fmmseg::{DictMaxLen, StarterUnion};
    ///
    /// // `opencc.segment_replace("...")`
    /// //   builds one StarterUnion from the given dictionaries,
    /// //   then calls `convert_by_union` per non-delimited segment.
    /// ```
    ///
    /// # Notes
    /// - If the set or contents of `dictionaries` changes, rebuild the union (this function does it for you).
    /// - This is an internal bridge used by higher-level routines (e.g., `DictRefs::apply_segment_replace`).
    fn segment_replace(
        &self,
        text: &str,
        dictionaries: &[&DictMaxLen],
        max_word_length: usize,
    ) -> String {
        let chars: Vec<char> = if self.is_parallel {
            text.par_chars().collect()
        } else {
            text.chars().collect()
        };

        let ranges = self.get_chars_range(&chars, false);

        // Build once per call for this dict set
        let union = StarterUnion::build(dictionaries);

        if self.is_parallel {
            ranges
                .into_par_iter()
                .with_min_len(8)
                .map(|r| self.convert_by_union(&chars[r], dictionaries, max_word_length, &union))
                .reduce(String::new, |mut a, b| {
                    a.push_str(&b);
                    a
                })
        } else {
            ranges
                .into_iter()
                .map(|r| self.convert_by_union(&chars[r], dictionaries, max_word_length, &union))
                .fold(String::new(), |mut acc, s| {
                    acc.push_str(&s);
                    acc
                })
        }
    }

    /// Core dictionary-matching routine (FMM) optimized with a precomputed **starter union**.
    ///
    /// This is the tightest loop of the OpenCC segment-replacement engine. It performs a
    /// left-to-right scan over a delimiter-free `&[char]` using **Forward Maximum Matching (FMM)**,
    /// but accelerates matching by using a `StarterUnion` (bitmask + per-starter caps) that
    /// unions multiple dictionaries up front.
    ///
    /// Compared to `convert_by()`:
    /// - Uses `union.bmp_mask / union.bmp_cap` (BMP) and `union.astral_mask / union.astral_cap`
    ///   (astral) to **prune impossible lengths** before touching any individual dictionary.
    /// - Iterates viable lengths in **descending order** via [`for_each_len_dec`] and short-circuits
    ///   on the first dictionary hit.
    ///
    /// # Matching Strategy
    /// For each position:
    /// 1. Compute the effective cap `cap_here = min(max_word_length, remaining, union_cap)`.
    /// 2. Use `union` to enumerate **only viable lengths** (from longest → shortest).
    /// 3. For each viable length, probe each dictionary **only if** that dict can possibly
    ///    host such a key (checked against `dict.max_len` and the dict’s own per-starter cap).
    /// 4. On the first match, emit the replacement, advance, and continue scanning.
    /// 5. If no match is found, copy the current char and advance by 1.
    ///
    /// # Arguments
    /// - `text_chars`: Non-delimited slice of `char` (a single segment).
    /// - `dictionaries`: The list of dictionaries to consult (probe order = precedence).
    /// - `max_word_length`: Global cap for match length in chars (e.g., 16).
    /// - `union`: Precomputed [`StarterUnion`] built from exactly the same dictionaries.
    ///
    /// # Returns
    /// A `String` containing the converted segment.
    ///
    /// # Requirements
    /// - `union` **must** be built from these `dictionaries` (same set and contents).
    ///   If the dictionaries change, rebuild the union.
    /// - Each `DictMaxLen` should have populated starter indexes
    ///   (done automatically by `DictMaxLen::build_from_pairs`).
    ///
    /// # Performance Notes
    /// - Union pruning avoids per-dict checks for impossible lengths/starters.
    /// - Lengths are tried longest→shortest; first match wins (best-case early exit).
    /// - Astral starters are sparse-map pruned; BMP starters are O(1) masked.
    ///
    /// # BMP vs Astral
    /// - **BMP** starters use dense arrays (`union.bmp_*`) indexed by `starter as usize`.
    /// - **Astral** starters use sparse maps (`union.astral_*`), looked up by `char`.
    ///
    /// # Complexity
    /// Let *N* be input length, *D* number of dictionaries, *L* max length.
    /// - Typical: `O(N · K · D)` where `K ≤ 64` viable lengths per position after pruning.
    /// - Best case: early exit on first long match.
    ///
    /// # Example (internal)
    /// This method relies on internal fields (e.g., `self.delimiters`). For illustration only:
    /// ```ignore
    /// use opencc_fmmseg::{DictMaxLen, StarterUnion};
    ///
    /// let d1 = DictMaxLen::build_from_pairs(vec![
    ///     ("你好".into(), "您好".into()),
    ///     ("世界".into(), "世間".into()),
    /// ]);
    /// let d2 = DictMaxLen::build_from_pairs(vec![
    ///     ("你们".into(), "您們".into()),
    /// ]);
    ///
    /// let dicts: [&DictMaxLen; 2] = [&d1, &d2];
    /// let union = StarterUnion::build(&dicts);
    ///
    /// // Suppose `opencc` is your core struct instance and `text_chars` is a delimiter-free segment:
    /// // let out = opencc.convert_by_union(&text_chars, &dicts, 16, &union);
    /// ```
    ///
    /// # Safety & Invariants
    /// - Slices are only taken within `start_pos..start_pos+length` after ensuring
    ///   `length ≤ remaining`.
    /// - The union’s CAP bit (≥64) is respected via `for_each_len_dec`.
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
        if text_length == 1 && self.delimiters.contains(&text_chars[0]) {
            return text_chars[0].to_string();
        }

        // const CAP_BIT: usize = 63;
        let mut result = String::with_capacity(text_length * 4);
        let mut start_pos = 0;

        while start_pos < text_length {
            let c0 = text_chars[start_pos];
            let u0 = c0 as u32;
            let rem = text_length - start_pos;
            let global_cap = max_word_length.min(rem);

            // Pull precomputed mask + cap
            let (mask, cap_u16) = if u0 <= 0xFFFF {
                let idx = u0 as usize;
                (union.bmp_mask[idx], union.bmp_cap[idx])
            } else {
                (
                    *union.astral_mask.get(&c0).unwrap_or(&0),
                    *union.astral_cap.get(&c0).unwrap_or(&0),
                )
            };

            if mask == 0 || cap_u16 == 0 {
                result.push(c0);
                start_pos += 1;
                continue;
            }

            let cap_here = global_cap.min(cap_u16 as usize);
            let mut matched = false;

            for_each_len_dec(mask, cap_here, |length| {
                let slice = &text_chars[start_pos..start_pos + length];

                // Probe only dicts that can possibly have this length for this starter
                for &dict in dictionaries {
                    if dict.max_len < length {
                        continue;
                    }

                    if u0 <= 0xFFFF {
                        let idx = u0 as usize;
                        if (dict.first_char_max_len[idx] as usize) < length {
                            continue;
                        }
                    } else if let Some(&cap_u8) = dict.starter_cap.get(&c0) {
                        if (cap_u8 as usize) < length {
                            continue;
                        }
                    } else {
                        continue;
                    }

                    if let Some(val) = dict.map.get(slice) {
                        result.push_str(val);
                        start_pos += length;
                        matched = true;
                        return true; // stop iterating lengths
                    }
                }

                false // keep iterating lengths
            });

            if !matched {
                result.push(c0);
                start_pos += 1;
            }
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

        DictRefs::new(&round_1).apply_segment_replace(input, |input, refs, max_len| {
            self.segment_replace(input, refs, max_len)
        })
    }

    /// Performs Traditional-to-Simplified Chinese conversion.
    pub fn t2s(&self, input: &str, punctuation: bool) -> String {
        let mut round_1: Vec<&DictMaxLen> =
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
        let mut round_1: Vec<&DictMaxLen> =
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
        let mut round_2: Vec<&DictMaxLen> =
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
        let mut round_1: Vec<&DictMaxLen> =
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
        let mut round_2: Vec<&DictMaxLen> =
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
        let mut round_1: Vec<&DictMaxLen> =
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
        let mut round_2: Vec<&DictMaxLen> =
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
        let union = StarterUnion::build(&dict_refs);
        self.convert_by_union(&chars, &dict_refs, 1, &union)
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
        let union = StarterUnion::build(&dict_refs);
        self.convert_by_union(&chars, &dict_refs, 1, &union)
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

// New backing type for a round: slice of &DictMaxLen plus its computed max_len
type DictRound<'a> = (&'a [&'a DictMaxLen], usize);

pub struct DictRefs<'a> {
    round_1: DictRound<'a>,
    round_2: Option<DictRound<'a>>,
    round_3: Option<DictRound<'a>>,
}

#[inline]
fn compute_round<'a>(dicts: &'a [&'a DictMaxLen]) -> DictRound<'a> {
    let max_len = dicts.iter().map(|d| d.max_len).max().unwrap_or(1);
    (dicts, max_len)
}

impl<'a> DictRefs<'a> {
    /// Build with required round 1 (slice of &DictMaxLen). max_len is computed.
    pub fn new(round_1_dicts: &'a [&'a DictMaxLen]) -> Self {
        Self {
            round_1: compute_round(round_1_dicts),
            round_2: None,
            round_3: None,
        }
    }

    /// Add optional round 2.
    pub fn with_round_2(mut self, round_2_dicts: &'a [&'a DictMaxLen]) -> Self {
        self.round_2 = Some(compute_round(round_2_dicts));
        self
    }

    /// Add optional round 3.
    pub fn with_round_3(mut self, round_3_dicts: &'a [&'a DictMaxLen]) -> Self {
        self.round_3 = Some(compute_round(round_3_dicts));
        self
    }

    /// Apply up to three rounds using a caller-provided segment/replace closure.
    ///
    /// The closure gets:
    /// - `&str` input
    /// - `&[&DictMaxLen]` dicts for the round
    /// - `usize` max_len (in chars) computed for that round
    ///
    /// It returns the transformed `String` for that round.
    pub fn apply_segment_replace<F>(&self, input: &str, segment_replace: F) -> String
    where
        F: Fn(&str, &[&DictMaxLen], usize) -> String,
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

// ----------------------------------------------

/// Union view of starter-length metadata across multiple [`DictMaxLen`] tables.
///
/// `StarterUnion` merges (unions) the **per-starter length masks** and
/// **per-starter maximum lengths** from several dictionaries, so the caller
/// can do a single starter check when matching across multiple tables.
///
/// - **BMP (0x0000..=0xFFFF)** starters are stored densely in fixed-size arrays:
///   - [`bmp_mask`]: bitmask of available lengths per starter (`u64`).
///   - [`bmp_cap`]: maximum length per starter (`u16`).
/// - **Astral (> 0xFFFF)** starters are sparse:
///   - [`astral_mask`], [`astral_cap`] keyed by the non-BMP starter `char`.
///
/// # Bit layout
/// For each starter:
/// - bit 0  ⇒ length = 1
/// - bit 1  ⇒ length = 2
/// - …
/// - bit 63 ⇒ length **≥ 64** (the “CAP” bucket)
///
/// # Invariants
/// - `bmp_mask.len() == 0x10000`; `bmp_cap.len() == 0x10000`.
/// - A set bit for a given starter implies at least one phrase length for that starter.
/// - By construction, `bmp_cap[c]` ≥ the largest set bit (converted to length) for `bmp_mask[c]`.
///
/// # Typical use
/// Build once after you’ve constructed your per-locale/per-variant dictionaries,
/// then use the union’s mask/cap to drive fast longest-match scans.
pub struct StarterUnion {
    /// Dense BMP length bitmasks, indexed by `starter as usize`.
    pub bmp_mask: Vec<u64>, // 0x10000

    /// Dense BMP per-starter maximum length (in characters), indexed by `starter as usize`.
    pub bmp_cap: Vec<u16>, // 0x10000

    /// Sparse length bitmasks for astral starters (`starter > 0xFFFF`).
    pub astral_mask: FxHashMap<char, u64>,

    /// Sparse per-starter maximum length (in characters) for astral starters.
    pub astral_cap: FxHashMap<char, u16>,
}

impl StarterUnion {
    /// Builds a union of starter metadata from multiple [`DictMaxLen`] tables.
    ///
    /// For each input dictionary:
    /// - **BMP** starters: bitwise-ORs the length masks into [`bmp_mask`], and takes
    ///   the element-wise maximum into [`bmp_cap`].
    /// - **Astral** starters: updates [`astral_mask`] and [`astral_cap`] in a sparse map.
    ///
    /// # Requirements
    /// Each `DictMaxLen` should have populated starter indexes
    /// (i.e., `populate_starter_indexes()` has been called, which is already done by
    /// [`DictMaxLen::build_from_pairs`]).
    ///
    /// # Complexity
    /// Let *D* be the number of dictionaries. The BMP union is `O(D · 65_536)`.
    /// Astral merging is proportional to the number of **distinct** astral starters.
    ///
    /// # Example
    /// ```
    /// use opencc_fmmseg::dictionary_lib::{DictMaxLen};
    /// use opencc_fmmseg::StarterUnion;
    ///
    /// // d1: has "你好" and one non-BMP char key "𢫊"
    /// let d1 = DictMaxLen::build_from_pairs(vec![
    ///     ("你好".to_string(), "您好".to_string()),
    ///     ("𢫊".to_string(), "替".to_string()),
    /// ]);
    ///
    /// // d2: has single-char "你" and "世界"
    /// let d2 = DictMaxLen::build_from_pairs(vec![
    ///     ("你".to_string(), "您".to_string()),
    ///     ("世界".to_string(), "世間".to_string()),
    /// ]);
    ///
    /// let u = StarterUnion::build(&[&d1, &d2]);
    ///
    /// // BMP starter checks for '你'
    /// let i = '你' as usize;
    /// assert_ne!(u.bmp_mask[i] & (1 << 0), 0); // len=1 exists ("你")
    /// assert_ne!(u.bmp_mask[i] & (1 << 1), 0); // len=2 exists ("你好")
    /// assert!(u.bmp_cap[i] >= 2);
    ///
    /// // Astral starter checks for '𢫊' (U+22ACA)
    /// let c_astral = '𢫊';
    /// let m = u.astral_mask.get(&c_astral).copied().unwrap_or(0);
    /// assert_ne!(m & (1 << 0), 0); // len=1 exists
    /// assert!(u.astral_cap.get(&c_astral).copied().unwrap_or(0) >= 1);
    /// ```
    pub fn build(dicts: &[&DictMaxLen]) -> Self {
        const N: usize = 0x10000;
        let mut bmp_mask = vec![0u64; N];
        let mut bmp_cap = vec![0u16; N];
        let mut astral_mask = FxHashMap::default();
        let mut astral_cap = FxHashMap::default();

        for d in dicts {
            // BMP union
            for i in 0..N {
                let m = d.first_len_mask64[i];
                if m != 0 {
                    bmp_mask[i] |= m;
                    let c = d.first_char_max_len[i];
                    if c > bmp_cap[i] {
                        bmp_cap[i] = c;
                    }
                }
            }

            // Astral sparse union
            for key in d.map.keys() {
                if key.is_empty() {
                    continue;
                }
                let c0 = key[0];
                if (c0 as u32) <= 0xFFFF {
                    continue;
                }
                let len = key.len();
                let bit = if len >= 64 { 63 } else { len - 1 };
                *astral_mask.entry(c0).or_default() |= 1u64 << bit;
                astral_cap
                    .entry(c0)
                    .and_modify(|m| {
                        if *m < len as u16 {
                            *m = len as u16
                        }
                    })
                    .or_insert(len as u16);
            }
        }

        Self {
            bmp_mask,
            bmp_cap,
            astral_mask,
            astral_cap,
        }
    }
}
// -------------------------------------------------

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
