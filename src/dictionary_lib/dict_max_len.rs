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
use serde::{Deserialize, Serialize};

/// Print a developer note to **stderr** in *debug* builds; **no-op** in release.
///
/// This macro accepts the same syntax as [`eprintln!`], but it only emits output
/// when `cfg(debug_assertions)` is enabled (i.e., debug/profile builds). In
/// release builds it expands to an empty block, so it won’t surprise end users.
///
/// # Examples
/// ```
/// use opencc_fmmseg::debug_note; // bring the macro into scope
///
/// // Shown during development (debug builds), silent in release:
/// debug_note!("duplicate key ignored (first-wins): key={}", "弁");
/// ```
///
/// # Behavior
/// - **Debug builds** (`cfg(debug_assertions)`): prints to stderr.
/// - **Release builds**: no-op (generates no output).
///
/// # Use cases
/// - Soft diagnostics while loading user-supplied dictionaries
/// - One-off hints that shouldn’t fail or spam release users
///
/// # See also
/// [`debug_assert!`], [`eprintln!`]
#[macro_export]
macro_rules! debug_note {
    ($($arg:tt)*) => {
        #[allow(unused)]
        {
            if cfg!(debug_assertions) {
                eprintln!($($arg)*);
            }
        }
    };
}

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
///     min_len: 0,
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

    /// Global minimum key length in characters across the entire dictionary.
    ///
    /// Used to limit scanning during forward maximum matching (FMM) segmentation.
    #[serde(default)]
    pub min_len: usize,

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
    /// - Converts each `key: String` into `Box<[char]>` (Unicode scalar values),
    /// - Tracks the **global** maximum and minimum key lengths in characters
    ///   (`max_len`, `min_len`),
    /// - Tracks the **per-starter** maximum key length (`starter_cap`),
    /// - Eagerly calls [`populate_starter_indexes`](#method.populate_starter_indexes)
    ///   to fill runtime accelerators: [`first_len_mask64`] and [`first_char_max_len`].
    ///
    /// ### Duplicates
    /// If the iterator yields duplicate **keys**, **first-wins**:
    /// - If the existing value is **identical**, the duplicate is ignored silently.
    /// - If the new value **differs**, the previous value is kept; in debug builds a
    ///   friendly note is printed via `debug_note!`, but there is **no panic**.
    ///
    /// ### Empty keys
    /// An empty `key` is **allowed**. It will be inserted into `map` but does **not**
    /// contribute to `starter_cap` or starter indexes.
    ///
    /// ### Unicode note
    /// Keys are stored as `char` slices (`Box<[char]>`). If your data contains
    /// combining marks or requires grapheme clustering, normalize your input to the
    /// representation you expect to match against (e.g., NFC) **before** calling this.
    ///
    /// ### Complexity
    /// Let *N* be the number of pairs and *L* the average key length (chars).
    /// - Build: `O(N·L)` to collect chars and insert into the map.
    /// - Starter index population: linear in the number of distinct first characters.
    ///
    /// ### Example
    /// ```
    /// use opencc_fmmseg::dictionary_lib::DictMaxLen;
    ///
    /// let pairs = vec![
    ///     ("你好".to_string(), "您好".to_string()),
    ///     ("世界".to_string(), "世間".to_string()),
    /// ];
    ///
    /// let dict = DictMaxLen::build_from_pairs(pairs);
    ///
    /// // Collected metadata
    /// assert!(dict.max_len >= 2);
    /// assert!(dict.min_len >= 1);
    /// assert!(dict.starter_cap.get(&'你').copied().unwrap_or(0) >= 2);
    ///
    /// // Zero-alloc style lookup using a borrowed slice:
    /// // let input: &[char] = &['你', '好'];
    /// // if let Some(v) = dict.map.get(input) { /* ... */ }
    /// ```
    pub fn build_from_pairs<I>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (String, String)>,
    {
        use std::collections::hash_map::Entry;

        // Reserve using the iterator's lower bound if available
        let it = pairs.into_iter();
        let (lower, _) = it.size_hint();

        let mut map: FxHashMap<Box<[char]>, Box<str>> = FxHashMap::default();
        if lower > 0 {
            map.reserve(lower);
        }

        let mut starter_cap: FxHashMap<char, u8> = FxHashMap::default();
        if lower > 0 {
            starter_cap.reserve(lower.min(0x10000));
        }

        let mut global_max = 0usize;
        let mut global_min = usize::MAX;

        for (k, v) in it {
            // Keys must not be empty (debug-only guard); empty keys are allowed but not indexed.
            debug_assert!(!k.is_empty(), "Dictionary key must not be empty");

            let chars: Box<[char]> = k.chars().collect::<Vec<_>>().into_boxed_slice();
            let len = chars.len();

            // Track per-starter cap
            debug_assert!(
                len <= u8::MAX as usize,
                "Entry length {} exceeds u8::MAX (255) for key {:?}",
                len,
                k
            );
            let len_u8 = u8::try_from(len).unwrap_or(u8::MAX);

            if let Some(&c0) = chars.first() {
                starter_cap
                    .entry(c0)
                    .and_modify(|m| *m = (*m).max(len_u8))
                    .or_insert(len_u8);
            }

            global_max = global_max.max(len);
            global_min = global_min.min(len);

            // Build value once; only inserted if needed
            let new_val: Box<str> = v.into_boxed_str();

            // Duplicate handling: first-wins; identical dup = silent ignore; conflicting dup = keep first, optional debug note.
            match map.entry(chars) {
                Entry::Vacant(e) => {
                    e.insert(new_val);
                }
                Entry::Occupied(e) => {
                    let prev = e.get();
                    if prev.as_ref() != new_val.as_ref() {
                        // Friendly debug-only message; keeps FIRST value (first-wins).
                        debug_note!(
                        "duplicate key ignored (first-wins): key={:?}; kept={:?}, ignored={:?}",
                        k, prev, new_val
                    );
                        // For last-wins instead: *e.into_mut() = new_val;
                    }
                    // identical duplicate -> silently ignored
                }
            }
        }

        // If there were no pairs, both bounds are 0
        let min_len = if global_min == usize::MAX { 0 } else { global_min };
        let max_len = global_max;

        debug_assert!(
            (max_len == 0 && min_len == 0) || (min_len >= 1 && min_len <= max_len),
            "min_len/max_len invariant violated: min_len={}, max_len={}",
            min_len,
            max_len
        );

        let mut dict = Self {
            map,
            max_len,
            min_len,
            starter_cap,
            first_len_mask64: Vec::new(),   // not built yet
            first_char_max_len: Vec::new(), // not built yet
        };

        // Build runtime accelerators for fast lookup.
        dict.populate_starter_indexes();

        // Post-build sanity checks
        debug_assert!(
            dict.min_len <= dict.max_len,
            "After populate: min_len > max_len ({} > {})",
            dict.min_len,
            dict.max_len
        );

        #[cfg(debug_assertions)]
        {
            // Each key length must be ≤ starter_cap for its first char
            for (k_chars, _) in &dict.map {
                if let Some(&c0) = k_chars.first() {
                    let cap = dict.starter_cap.get(&c0).copied().unwrap_or(0);
                    let ok = u8::try_from(k_chars.len()).map(|l| l <= cap).unwrap_or(false);
                    debug_assert!(
                        ok,
                        "starter_cap too small: first {:?}, key_len={}, cap={}",
                        c0,
                        k_chars.len(),
                        cap
                    );
                }
            }
        }

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
    ///     min_len: 0,
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
    ///     min_len: 0,
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
    ///     min_len: 0,
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
            min_len: 0,
            starter_cap: FxHashMap::default(),
            first_len_mask64: Vec::new(),
            first_char_max_len: Vec::new(),
        }
    }
}
