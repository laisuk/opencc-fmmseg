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

use crate::delimiter_set::is_delimiter;
pub use crate::dict_refs::DictRefs;
use crate::dictionary_lib::dictionary_maxlength::UnionKey;
use crate::dictionary_lib::StarterUnion;
use dictionary_lib::dict_max_len::DictMaxLen;
use dictionary_lib::DictionaryMaxlength;

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

/// Iterates viable phrase lengths in **descending order** using a starter bitmask,
/// stopping early if the callback returns `true`.
///
/// # Parameters
/// - `mask`: 64-bit mask encoding which lengths are possible for the current starter:
///   - bit 0 ⇒ length = 1
///   - bit 1 ⇒ length = 2
///   - …
///   - bit 62 ⇒ length = 63
///   - bit 63 ⇒ **CAP bit**, representing length ≥ 64
/// - `cap_here`: Effective cap at the current position, usually
///   `min(global_max, remaining_chars)`.
/// - `f(len)`: Callback invoked for each candidate length, from longest to shortest.
///   If it returns `true`, iteration stops immediately.
///
/// # Iteration order
/// 1. If `cap_here > 64` and the CAP bit is set, iterate lengths
///    `cap_here, cap_here-1, …, 65`, then `64`.
/// 2. Then iterate all set bits within `1..=min(64, cap_here)`
///    in descending order (64→1).
///
/// # CAP semantics
/// - If `cap_here == 64`: the CAP bit represents exactly length 64.
/// - If `cap_here > 64`: the CAP bit is only a flag (“some length ≥64 exists”);
///   this helper will explicitly try every length from `cap_here` down to 64.
/// - If `cap_here < 64`: the CAP bit is ignored.
///
/// # Notes
/// - Empty mask or `cap_here == 0` yields no iterations.
/// - This helper is typically used inside [`convert_by_union`] to drive
///   the “longest-first” FMM probing loop.
/// - Internally, it uses `leading_zeros` to walk set bits from high→low.
///
/// # Example
/// ```ignore
/// // mask with bit 0 (len=1), bit 2 (len=3), CAP (≥64)
/// let mask = (1u64 << 0) | (1u64 << 2) | (1u64 << 63);
///
/// let mut seen = Vec::new();
/// for_each_len_dec(mask, 5, |len| { seen.push(len); false });
/// assert_eq!(seen, vec![3, 1]); // CAP ignored since cap_here=5 < 64
/// ```
#[inline(always)]
fn for_each_len_dec(mask: u64, cap_here: usize, mut f: impl FnMut(usize) -> bool) {
    if mask == 0 || cap_here == 0 {
        return;
    }
    const CAP_BIT: u64 = 1u64 << 63;
    // If cap > 64 and CAP is set, scan >64 first (cap..=65), then 64.
    if cap_here > 64 && (mask & CAP_BIT) != 0 {
        // Try lengths from cap_here down to 65
        for len in (65..=cap_here).rev() {
            if f(len) {
                return;
            }
        }
        // Now 64 once (under the CAP semantics for >64)
        if f(64) {
            return;
        }
    }

    // Handle lengths 1..=min(64, cap_here) by iterating set bits high→low.
    let limit = cap_here.min(64);
    // Bitmask for [1..=limit]; shift-safe when limit==64.
    let range_mask = 1u64.wrapping_shl(limit as u32).wrapping_sub(1);
    // Apply, and drop CAP if we already consumed it via >64 path.
    let mut m = mask & range_mask & if cap_here > 64 { !CAP_BIT } else { !0 };
    // Highest-set-bit iteration.
    while m != 0 {
        let bit_pos = 63 - m.leading_zeros() as usize; // 0-based
        let len = bit_pos + 1; // map to length
        if f(len) {
            return;
        }
        m &= !(1u64 << bit_pos); // clear highest bit
    }
}

/// Checks whether a given dictionary (`DictMaxLen`) allows a word of the specified
/// `length` to start with the provided `starter` character.
///
/// This function uses fast lookups with per-starter metadata:
///
/// - For **BMP characters** (`u <= 0xFFFF`):
///   - If dense arrays are available (`first_char_max_len` and `first_len_mask64`
///     both cover the full BMP range):
///     1. Checks the **length bitmask** (`first_len_mask64`) for the starter.
///        - If the bitmask is nonzero, only returns `true` if the corresponding
///          `bit` for the target `length` is set.
///        - This is the most selective check and avoids extra work.
///     2. Falls back to the **maximum length cap** (`first_char_max_len`) if the
///        bitmask is zero.
///   - If dense arrays are not populated, falls back to the sparse `starter_cap` map.
/// - For **astral characters** (`u > 0xFFFF`), always falls back to `starter_cap`.
///
/// # Parameters
/// - `dict`: The [`DictMaxLen`] dictionary reference.
/// - `starter`: The candidate starting character.
/// - `length`: The word length to validate.
/// - `bit`: The bit index corresponding to `length` (usually `length - 1`).
///
/// # Returns
/// - `true` if the dictionary contains at least one entry starting with `starter`
///   of the specified `length`.
/// - `false` otherwise.
///
/// # Safety
/// - Uses unchecked indexing (`get_unchecked`) when dense arrays are available
///   for maximum speed. Safe because arrays are guaranteed to have 0x10000 length
///   when the dense path is active.
///
/// # Examples
/// ```ignore
/// let ok = starter_allows_dict(&dict, '中', 2, 1);
/// if ok {
///     // A 2-character phrase starting with '中' exists in the dictionary
/// }
/// ```
#[inline(always)]
fn starter_allows_dict(dict: &DictMaxLen, starter: char, length: usize, bit: usize) -> bool {
    let len_u8 = length as u8;
    let u = starter as u32;

    if u <= 0xFFFF {
        let i = u as usize;
        // If dense arrays are not populated (lazy), fall back to sparse `starter_cap`
        let have_dense =
            dict.first_char_max_len.len() == 0x10000 && dict.first_len_mask64.len() == 0x10000;

        if have_dense {
            // 1) Per-starter length bitmask: most selective → check first if nonzero
            let m = unsafe { *dict.first_len_mask64.get_unchecked(i) };
            if m != 0 {
                if ((m >> bit) & 1) == 0 {
                    return false;
                }
                // Mask says this length exists; cap check is redundant
                return true;
            }
            // 2) Cap check (dense array)
            let cap = unsafe { *dict.first_char_max_len.get_unchecked(i) };
            cap >= len_u8
        } else {
            // Fallback: sparse cap map (works for BMP & astral uniformly)
            let cap = dict.starter_cap.get(&starter).copied().unwrap_or(0);
            cap >= len_u8
        }
    } else {
        // Astral: no dense arrays — use sparse cap
        let cap = dict.starter_cap.get(&starter).copied().unwrap_or(0);
        cap >= len_u8
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
        let chars: Vec<char> = if self.is_parallel {
            text.par_chars().collect()
        } else {
            text.chars().collect()
        };

        let ranges = self.get_chars_range(&chars, false);

        if self.is_parallel {
            ranges
                .into_par_iter()
                .with_min_len(8)
                .map(|r| self.convert_by_union(&chars[r], dictionaries, max_word_length, union))
                .reduce(String::new, |mut a, b| {
                    a.push_str(&b);
                    a
                })
        } else {
            // Serial path: avoid growth copies
            let mut out = String::with_capacity(text.len());
            for r in ranges {
                out.push_str(&self.convert_by_union(
                    &chars[r],
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

        let is_multy_dicts = dictionaries.len() > 1;
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

            for_each_len_dec(mask, cap_here, |length| {
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
                    if is_multy_dicts {
                        if !starter_allows_dict(dict, c0, length, cap_bit) {
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
    /// - `text_chars`: Input text, pre-split into `char`s.
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

    /// Performs Traditional-to-Simplified Chinese conversion.
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

    /// Performs Simplified-to-Taiwanese conversion.
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

    /// Performs Taiwanese-to-Simplified conversion.
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

    /// Performs simplified to Traditional Taiwan conversion with idioms
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

    /// Performs Traditional Taiwan to Simplified with idioms
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

    /// Performs simplified to Traditional Hong Kong
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

    /// Performs Traditional Hong Kong to Simplified
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

    /// Performs traditional to traditional Taiwan
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

    /// Performs traditional to traditional Taiwan with idioms
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

    /// Performs traditional Taiwan to traditional
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

    /// Performs traditional Taiwan to traditional with idioms
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

    /// Perform traditional to traditional Hong Kong
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

    /// Performs traditional Hong Kong to traditional
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

    /// Performs Japanese Kyujitai to Shinjitai
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

    /// Performs japanese Shinjitai to Kyujitai
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
        // pick the smaller of (1000, stripped length)
        let check_len = find_max_utf8_length(input, 1000);

        let _strip_text = STRIP_REGEX.replace_all(&input[..check_len], "");
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
