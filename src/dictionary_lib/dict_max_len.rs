//! High-performance dictionary type with maximum-length and per-starter metadata.
//!
//! This module defines [`DictMaxLen`], the core dictionary structure used by
//! **opencc-fmmseg** for fast phrase lookup and segmentation.
//!
//! ## Overview
//!
//! `DictMaxLen` stores a mapping from phrase keys (`Box<[char]>`) to
//! replacement strings (`Box<str>`), along with:
//!
//! - A **global maximum phrase length** (`max_len`)
//! - **Per-starter maximum lengths** (`starter_cap`)
//! - **Runtime accelerators** (`first_len_mask64`, `first_char_max_len`)
//!
//! The runtime accelerators are *dense arrays indexed by the Unicode scalar
//! value of the first character* (BMP only) and are used to quickly decide
//! if a given prefix length could match any entry.
//!
//! ## Example
//! ```
//! use opencc_fmmseg::dictionary_lib::DictMaxLen;
//!
//! let pairs = vec![
//!     ("你好".to_string(), "您好".to_string()),
//!     ("世界".to_string(), "世間".to_string()),
//! ];
//!
//! let dict = DictMaxLen::build_from_pairs(pairs);
//!
//! assert!(dict.max_len >= 2);
//! assert!(dict.starter_cap.get(&'你').is_some());
//! assert!(dict.is_populated());
//! ```
//!
//! ## Related Functions
//! - [`DictMaxLen::build_from_pairs`] — build from `(String, String)` pairs.
//! - [`DictMaxLen::ensure_starter_indexes`] — ensure dense BMP arrays exist.
//! - [`DictMaxLen::populate_starter_indexes`] — rebuild arrays from dictionary data.
//! - [`DictMaxLen::is_populated`] — check if dense arrays are allocated.

use rustc_hash::FxHashMap;
// use serde::ser::SerializeMap;
// use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use serde::{Deserialize, Serialize};

/// A dictionary with a tracked maximum phrase length and per-starter length caps,
/// optimized for zero-allocation lookups and fast segmentation.
///
/// `DictMaxLen` is the core data structure for mapping phrase keys to replacement
/// strings in **OpenCC-FMMSEG**. It maintains not only the dictionary content,
/// but also metadata and runtime-optimized accelerators for high-performance
/// text conversion and segmentation.
///
/// # Key Features
///
/// - **Zero-allocation lookups** — keys are stored as `Box<[char]>`
///   to allow direct `&[char]` access without intermediate `String` allocation.
/// - **Global maximum length** (`max_len`) — the longest key length in characters.
/// - **Per-starter length caps** (`starter_cap`) — the longest key starting with
///   a specific character, allowing fast early rejection in segmentation.
/// - **Runtime accelerators** (`first_len_mask64`, `first_char_max_len`) —
///   dense, array-based quick checks for common starters, built at runtime.
///
/// # Usage
///
/// ```
/// use opencc_fmmseg::dictionary_lib::DictMaxLen;
/// use rustc_hash::FxHashMap;
///
/// // Example: create a dictionary with a single mapping
/// let mut dict = DictMaxLen {
///     map: FxHashMap::default(),
///     max_len: 0,
///     starter_cap: FxHashMap::default(),
///     first_len_mask64: vec![0; 65536],
///     first_char_max_len: vec![0; 65536],
/// };
///
/// dict.map.insert(Box::from(['你']), Box::from("您"));
/// dict.max_len = 1;
/// dict.starter_cap.insert('你', 1);
/// ```
///
/// This struct is typically built from lexicon files and serialized/deserialized
/// with `serde` for persistent storage.
///
/// # Serialization
///
/// Only `map`, `max_len`, and `starter_cap` are serialized.
/// Runtime accelerators are reconstructed at load time.
///
/// # See Also
///
/// - [`crate::dictionary_maxlength`] — utilities for loading and building `DictMaxLen`.
#[derive(Serialize, Deserialize, Debug)]
pub struct DictMaxLen {
    /// Dictionary mapping: phrase (as boxed slice of `char`) → replacement string.
    ///
    /// Keys are stored as `Box<[char]>` to enable direct `&[char]` lookups without
    /// allocation, reducing overhead in tight segmentation loops.
    #[serde(default)]
    pub map: FxHashMap<Box<[char]>, Box<str>>,

    /// Global maximum key length in characters across the entire dictionary.
    ///
    /// Used to limit scanning during forward maximum matching (FMM) segmentation.
    #[serde(default)]
    pub max_len: usize,

    /// Per-starter maximum key length in characters.
    ///
    /// The key is the starting character of the phrase, and the value is the
    /// maximum number of characters for any phrase starting with that character.
    /// This allows early exit during segmentation when no longer matches are possible.
    #[serde(default)]
    pub starter_cap: FxHashMap<char, u8>,

    /// Runtime-only: length bitmask for the first character (Unicode BMP).
    ///
    /// Each `u64` stores a bitfield representing which phrase lengths exist
    /// for phrases starting with the given character. Bit `n` means a phrase of
    /// length `n+1` exists.
    ///
    /// This vector is initialized empty and built after loading the dictionary.
    #[serde(skip)]
    #[serde(default)]
    pub first_len_mask64: Vec<u64>,

    /// Runtime-only: maximum key length per first character (Unicode BMP).
    ///
    /// Each entry stores the maximum phrase length (in characters) for the given
    /// starter character. Parallel to `first_len_mask64` but stored as `u16`.
    #[serde(skip)]
    #[serde(default)]
    pub first_char_max_len: Vec<u8>,
}

impl DictMaxLen {
    /// Builds a dictionary from `(key, value)` string pairs and eagerly
    /// constructs starter indexes (length masks and per-starter caps).
    ///
    /// This constructor:
    /// - Converts each `key: String` to `Box<[char]>` (scalar-value chars),
    /// - Tracks the **global** maximum key length in characters (`max_len`),
    /// - Tracks the **per-starter** maximum key length (`starter_cap`),
    /// - Eagerly calls [`populate_starter_indexes`](#method.populate_starter_indexes)
    ///   to fill the runtime accelerators:
    ///   [`first_len_mask64`] and [`first_char_max_len`].
    ///
    /// ### Duplicates
    /// If the iterator yields duplicate keys, the **last** value wins (it
    /// overwrites the previous entry in the map).
    ///
    /// ### Empty keys
    /// An empty `key` is allowed. It will be inserted into `map` but it does
    /// **not** contribute to `starter_cap` or starter indexes.
    ///
    /// ### Unicode note
    /// Keys are stored as `char` slices (`Box<[char]>`), i.e., Unicode scalar
    /// values. If your data contains combining marks or requires grapheme
    /// clustering, ensure your keys are normalized to the representation you
    /// expect to match against.
    ///
    /// ### Complexity
    /// Let *N* be the number of pairs and *L* the average key length (chars).
    /// - Build: `O(N·L)` to collect chars and insert into the map.
    /// - Starter index population: linear in the number of distinct starters.
    ///
    /// ### Example
    /// ```
    /// use rustc_hash::FxHashMap;
    /// use opencc_fmmseg::dictionary_lib::DictMaxLen;
    ///
    /// let pairs = vec![
    ///     ("你好".to_string(), "您好".to_string()),
    ///     ("世界".to_string(), "世間".to_string()),
    /// ];
    ///
    /// let dict = DictMaxLen::build_from_pairs(pairs);
    ///
    /// // Look at collected metadata
    /// assert!(dict.max_len >= 2);
    /// assert!(dict.starter_cap.get(&'你').copied().unwrap_or(0) >= 2);
    ///
    /// // Zero-alloc style lookup (pseudo):
    /// // let input: &[char] = &['你', '好'];
    /// // if let Some(v) = dict.map.get(input) { /* ... */ }
    /// ```
    pub fn build_from_pairs<I>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (String, String)>,
    {
        let mut map = FxHashMap::default();
        let mut starter_cap: FxHashMap<char, u8> = FxHashMap::default();
        let mut global_max = 1usize;

        for (k, v) in pairs {
            let chars: Box<[char]> = k.chars().collect::<Vec<_>>().into_boxed_slice();
            let len = chars.len();
            if let Some(&c0) = chars.first() {
                starter_cap
                    .entry(c0)
                    .and_modify(|m| *m = (*m).max(len as u8))
                    .or_insert(len as u8);
            }
            global_max = global_max.max(len);
            map.insert(chars, v.into_boxed_str());
        }

        let mut dict = Self {
            map,
            max_len: global_max,
            starter_cap,
            first_len_mask64: Vec::new(),   // not built yet
            first_char_max_len: Vec::new(), // not built yet
        };

        // Eagerly build runtime accelerators for fast segmentation.
        dict.populate_starter_indexes();

        dict
    }

    /// Ensures that the runtime starter index buffers exist and have the expected sizes.
    ///
    /// This method validates and (re)allocates the two **dense starter index arrays**:
    ///
    /// - [`first_len_mask64`]: `Vec<u64>` — bitmask of phrase lengths per starter character.
    /// - [`first_char_max_len`]: `Vec<u16>` — maximum phrase length per starter character.
    ///
    /// Both vectors are indexed by the Unicode scalar value of the starter character
    /// (restricted to the **Basic Multilingual Plane**, 0x0000–0xFFFF).
    ///
    /// If either vector is not exactly `0x10000` entries long, it is cleared and
    /// resized to that length, filled with zeros.
    ///
    /// # Invariants
    /// - **Length**: exactly 65 536 entries.
    /// - **Indexing**: `starter as usize` gives the position in both arrays.
    /// - **Default state**: all entries zero (no lengths recorded).
    ///
    /// # Performance
    /// This method runs in **O(N)** where *N* = 65 536 (the BMP size) in the worst case
    /// when reallocation is needed, but is effectively **O(1)** if sizes already match.
    ///
    /// # Example
    /// ```
    /// use opencc_fmmseg::dictionary_lib::DictMaxLen;
    /// let mut dict = DictMaxLen {
    ///     map: Default::default(),
    ///     max_len: 0,
    ///     starter_cap: Default::default(),
    ///     first_len_mask64: Vec::new(),
    ///     first_char_max_len: Vec::new(),
    /// };
    ///
    /// dict.ensure_starter_indexes();
    /// assert_eq!(dict.first_len_mask64.len(), 0x10000);
    /// assert_eq!(dict.first_char_max_len.len(), 0x10000);
    /// ```
    pub fn ensure_starter_indexes(&mut self) {
        const N: usize = 0x10000; // BMP size

        if self.first_len_mask64.len() != N {
            self.first_len_mask64.clear();
            self.first_len_mask64.resize(N, 0u64);
        }
        if self.first_char_max_len.len() != N {
            self.first_char_max_len.clear();
            self.first_char_max_len.resize(N, 0u8);
        }
    }

    /// (Re)builds the **Basic Multilingual Plane (BMP)** starter index arrays
    /// from [`self.map`], using [`starter_cap`] for per-starter maximum lengths.
    ///
    /// This method regenerates the two dense starter index arrays:
    ///
    /// - [`first_len_mask64`]:
    ///   - Indexed by the starter character's Unicode scalar value (BMP only).
    ///   - Each `u64` stores a **bitmask of phrase lengths** for that starter:
    ///     - **Bit 0** → a phrase of length 1 exists.
    ///     - **Bit 1** → a phrase of length 2 exists.
    ///     - …
    ///     - **Bit 63** → a phrase of length **≥64** exists.
    /// - [`first_char_max_len`]:
    ///   - Indexed identically.
    ///   - Stores the **maximum phrase length** (in characters) for each starter.
    ///   - This is seeded from [`starter_cap`] without recomputing.
    ///
    /// # Behavior
    /// 1. Ensures both arrays are allocated to length `0x10000` via
    ///    [`ensure_starter_indexes`](#method.ensure_starter_indexes).
    /// 2. Clears both arrays to zero.
    /// 3. Iterates through all dictionary keys in [`map`], sets the
    ///    corresponding length bit in `first_len_mask64` for **BMP starters**.
    /// 4. Copies per-starter maximum lengths from `starter_cap` into
    ///    `first_char_max_len`.
    ///
    /// Non-BMP starter characters (`char` with code point > `0xFFFF`) are ignored
    /// here for performance and memory efficiency. As per design notes, these
    /// are rare in OpenCC dictionaries.
    ///
    /// # Example
    /// ```
    /// use opencc_fmmseg::dictionary_lib::DictMaxLen;
    ///
    /// let pairs = vec![
    ///     ("你好".to_string(), "您好".to_string()),
    ///     ("你们".to_string(), "您們".to_string()),
    ///     ("世界".to_string(), "世間".to_string()),
    /// ];
    ///
    /// let mut dict = DictMaxLen::build_from_pairs(pairs);
    ///
    /// // Rebuild starter indexes (normally done automatically at build)
    /// dict.populate_starter_indexes();
    ///
    /// let idx = '你' as usize;
    /// assert_ne!(dict.first_len_mask64[idx] & (1 << (2 - 1)), 0); // binary bit for length=2
    /// assert!(dict.first_char_max_len[idx] >= 2);
    /// ```
    ///
    /// # Complexity
    /// Let *N* be the number of keys:
    /// - Length mask build: **O(N)**.
    /// - Max length seeding: **O(S)** where *S* is the number of distinct starters.
    pub fn populate_starter_indexes(&mut self) {
        const CAP_BIT: usize = 63; // bit index for len >= 64

        // Ensure arrays exist with correct size
        self.ensure_starter_indexes();

        // Clear arrays for full rebuild
        for v in &mut self.first_len_mask64 {
            *v = 0;
        }
        for v in &mut self.first_char_max_len {
            *v = 0;
        }

        // 1) Build length mask from dictionary keys (BMP starters only)
        for k in self.map.keys() {
            if k.is_empty() {
                continue;
            }
            let c0 = k[0];
            let u = c0 as u32;
            if u > 0xFFFF {
                // Ignore non-BMP starters (rare in OpenCC data)
                continue;
            }

            let len = k.len();
            let bit = if len >= 64 { CAP_BIT } else { len - 1 };
            let idx = u as usize;
            self.first_len_mask64[idx] |= 1u64 << bit;
        }

        // 2) Seed per-starter max from existing starter_cap (no recomputation)
        for (&c, &cap_u8) in &self.starter_cap {
            let u = c as u32;
            if u <= 0xFFFF {
                self.first_char_max_len[u as usize] = cap_u8;
            }
        }

        // NOTE: self.max_len is not modified here.
    }

    /// Checks whether the starter index arrays have been fully allocated.
    ///
    /// This method returns `true` if and only if:
    ///
    /// - [`first_len_mask64`] has length `0x10000` (65 536 entries), **and**
    /// - [`first_char_max_len`] has length `0x10000`.
    ///
    /// This is used as a quick sanity check to determine whether the
    /// starter indexes have been built or at least allocated to cover
    /// the entire **Basic Multilingual Plane (BMP)**.
    ///
    /// # Example
    /// ```
    /// use opencc_fmmseg::dictionary_lib::DictMaxLen;
    ///
    /// let mut dict = DictMaxLen {
    ///     map: Default::default(),
    ///     max_len: 0,
    ///     starter_cap: Default::default(),
    ///     first_len_mask64: Vec::new(),
    ///     first_char_max_len: Vec::new(),
    /// };
    ///
    /// assert!(!dict.is_populated());
    ///
    /// dict.ensure_starter_indexes();
    /// assert!(dict.is_populated());
    /// ```
    #[inline]
    pub fn is_populated(&self) -> bool {
        self.first_len_mask64.len() == 0x10000 && self.first_char_max_len.len() == 0x10000
    }
}

impl Default for DictMaxLen {
    /// Creates an empty [`DictMaxLen`] with all fields initialized to their defaults.
    ///
    /// - [`map`] — empty `FxHashMap`.
    /// - [`max_len`] — `0`.
    /// - [`starter_cap`] — empty `FxHashMap`.
    /// - [`first_len_mask64`] — empty `Vec` (use [`ensure_starter_indexes`](#method.ensure_starter_indexes) to allocate).
    /// - [`first_char_max_len`] — empty `Vec` (use [`ensure_starter_indexes`](#method.ensure_starter_indexes) to allocate).
    ///
    /// This is equivalent to calling:
    /// ```
    /// use opencc_fmmseg::dictionary_lib::DictMaxLen;
    /// use rustc_hash::FxHashMap;
    ///
    /// let dict = DictMaxLen {
    ///     map: FxHashMap::default(),
    ///     max_len: 0,
    ///     starter_cap: FxHashMap::default(),
    ///     first_len_mask64: Vec::new(),
    ///     first_char_max_len: Vec::new(),
    /// };
    /// ```
    ///
    /// # Example
    /// ```
    /// use opencc_fmmseg::dictionary_lib::DictMaxLen;
    ///
    /// let dict = DictMaxLen::default();
    /// assert_eq!(dict.max_len, 0);
    /// assert!(dict.map.is_empty());
    /// assert!(!dict.is_populated());
    /// ```
    fn default() -> Self {
        Self {
            map: FxHashMap::default(),
            max_len: 0,
            starter_cap: FxHashMap::default(),
            first_len_mask64: Vec::new(),
            first_char_max_len: Vec::new(),
        }
    }
}
