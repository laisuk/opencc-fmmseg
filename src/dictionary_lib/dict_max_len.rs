//! High-performance dictionary type with global and per-starter length metadata.
//!
//! This module defines [`DictMaxLen`], the core dictionary structure used by
//! **opencc-fmmseg** for fast phrase lookup and segmentation.
//!
//! ## Overview
//!
//! `DictMaxLen` stores a mapping from phrase keys (`Box<[char]>`) to
//! replacement strings (`Box<str>`), along with:
//!
//! - A **global key-length mask** (`key_length_mask`) covering lengths `1..=64`
//!   (bit `n-1` ⇢ length `n`) plus `min_len`/`max_len` for overall bounds.
//! - A **per-starter length mask** (`starter_len_mask: FxHashMap<char, u64>`)
//!   that records, for each starting character (BMP + astral), exactly which
//!   key lengths exist (again `1..=64` as bits).
//! - **Runtime accelerators (BMP dense tables)**:
//!   - `first_len_mask64: Vec<u64>` — per-starter length bitmasks for BMP
//!   - `first_char_max_len: Vec<u8>` — per-starter max length (derived from mask)
//!
//! The dense tables are *indexed by the Unicode scalar value of the first
//! character* (BMP only) and let the segmenter quickly decide if a given
//! `(starter, length)` is even possible before attempting a hash lookup.
//!
//! ## Example
//! ```ignore
//! use opencc_fmmseg::dictionary_lib::DictMaxLen;
//!
//! // Build from pairs (adjust to your actual builder API)
//! let pairs = vec![
//!     ("你好".to_string(), "您好".to_string()),
//!     ("世界".to_string(), "世間".to_string()),
//! ];
//! let dict = DictMaxLen::build_from_pairs(pairs);
//!
//! // Global metadata collected
//! assert!(dict.max_len >= 2);
//! assert!(dict.min_len >= 1);
//!
//! // Per-starter length mask is a bitfield: bit (len-1) corresponds to `len`.
//! // For '你', length = 2 → bit index 1 must be set.
//! let mask = dict.starter_len_mask.get(&'你').copied().unwrap_or(0);
//! assert_eq!((mask >> 1) & 1, 1);
//!
//! // Fast gate API (after has_key_len(len) at the call-site):
//! let cap_bit = 2 - 1;
//! assert!(dict.starter_allows_dict('你', 2, cap_bit));
//!
//! // Dense tables should be allocated/populated for BMP starters:
//! assert!(dict.is_populated());
//! ```
//!
//! ## Related Functions
//! - [`DictMaxLen::build_from_pairs`] — build from `(String, String)` pairs.
//! - [`DictMaxLen::ensure_starter_indexes`] — ensure dense BMP arrays exist.
//! - [`DictMaxLen::populate_starter_indexes`] — rebuild dense arrays from masks/map.
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

/// A dictionary with global and per-starter **length masks**, optimized for
/// zero-allocation lookups and fast segmentation.
///
/// `DictMaxLen` is the core structure mapping phrase keys to replacement
/// strings in **opencc-fmmseg**. Beyond the raw map, it maintains metadata
/// and runtime accelerators to prune impossible matches early.
///
/// # Key Features
///
/// - **Zero-allocation lookups** — keys are stored as `Box<[char]>`,
///   enabling direct `&[char]` queries without intermediate `String`.
/// - **Global key-length bounds** — `min_len`, `max_len`, and a compact
///   `key_length_mask` (bits 0..63 ⇢ lengths 1..64) for quick global gating.
/// - **Per-starter length masks** — `starter_len_mask: FxHashMap<char, u64>`
///   records, per first character (BMP + astral), exactly which lengths exist
///   (again 1..=64 as bits). This replaces legacy per-starter “cap” maps.
/// - **Runtime accelerators (BMP dense tables)**:
///   - `first_len_mask64: Vec<u64>` — per-starter length bitmasks for BMP
///   - `first_char_max_len: Vec<u8>` — per-starter max length (derived from mask)
///   These dense arrays are indexed by the Unicode scalar value of the first
///   character (`0x0000..=0xFFFF`) and are rebuilt at load/build time.
///
/// # Usage
///
/// ```
/// use opencc_fmmseg::dictionary_lib::DictMaxLen;
/// use rustc_hash::FxHashMap;
///
/// // Minimal manual construction (normally use a builder)
/// let mut dict = DictMaxLen {
///     map: FxHashMap::default(),
///     max_len: 0,
///     min_len: 0,
///     key_length_mask: 0,
///     // Dense BMP tables (rebuilt by `populate_starter_indexes`)
///     first_len_mask64: vec![0; 0x10000],
///     first_char_max_len: vec![0; 0x10000],
///     // Sparse per-starter masks (authoritative source)
///     starter_len_mask: FxHashMap::default(),
/// };
///
/// // Add a single-char mapping: "你" -> "您"
/// dict.map.insert(Box::from(['你']), Box::from("您"));
/// dict.min_len = 1;
/// dict.max_len = 1;
/// dict.key_length_mask |= 1u64 << (1 - 1);       // length 1
/// dict.starter_len_mask.insert('你', 1u64 << 0);  // '你' has a length-1 entry
///
/// // Rebuild dense accelerators for BMP starters
/// dict.populate_starter_indexes();
/// ```
///
/// This struct is typically built from lexicon files and serialized/deserialized
/// with `serde` for persistent storage.
///
/// # Serialization
///
/// The following are serialized: `map`, `min_len`, `max_len`, `key_length_mask`,
/// and `starter_len_mask`. The **dense BMP accelerators**
/// (`first_len_mask64`, `first_char_max_len`) are **not** serialized and are
/// reconstructed via [`populate_starter_indexes`](DictMaxLen::populate_starter_indexes)
/// at load/build time.
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

    /// Global minimum key length (in characters) across the entire dictionary.
    ///
    /// Used to bound scanning during forward-maximum-matching (FMM) segmentation.
    /// Together with [`max_len`](Self::max_len) and [`key_length_mask`](Self::key_length_mask),
    /// this lets callers quickly skip impossible lengths.
    #[serde(default)]
    pub min_len: usize,

    /// Global key-length presence mask for lengths `1..=64`.
    ///
    /// Bit **`n-1`** indicates that the dictionary contains **at least one key**
    /// of length **`n`**. This provides a compact, branch-free gate used by
    /// [`has_key_len`](Self::has_key_len) and hot segmentation loops.
    ///
    /// Notes:
    /// - Lengths **> 64** are **not** represented in this mask. If such keys exist,
    ///   they are still reflected in [`max_len`](Self::max_len); callers should use
    ///   both the mask **and** `min_len`/`max_len` for complete gating.
    /// - When the mask is zero (e.g., legacy/empty), callers should fall back to
    ///   `min_len`/`max_len`.
    ///
    /// Example: if keys of lengths `{1,2,5}` exist, then this field equals:
    /// `0b1_0001_1` (bits 0,1,4 set) → decimal `0b100011 = 35`.
    #[serde(default)]
    pub key_length_mask: u64,

    /// Sparse, exact **per-starter length bitmask** (BMP **and** astral).
    ///
    /// For each starting `char`, the `u64` mask records which key lengths exist:
    /// bit **k** ⇒ length **k+1** (i.e., lengths `1..=64` are representable).
    ///
    /// This is the authoritative source for per-starter length presence. The dense
    /// BMP accelerators (`first_len_mask64`, `first_char_max_len`) are rebuilt from
    /// this map in [`populate_starter_indexes`](Self::populate_starter_indexes).
    ///
    /// Notes:
    /// - Lengths **> 64** are not represented in the mask. In the dense BMP path,
    ///   `first_char_max_len` (derived from this mask and/or keys) is used to gate
    ///   `length > 64`.
    /// - Astral starters are kept **only** here (no dense tables for astral).
    ///
    /// Example:
    /// `0b...00101` ⇒ lengths `{1, 3}` exist for that starter.
    ///
    /// Keys are `char` (not `String`) for compactness; this map may be empty if
    /// built solely from dense tables and later reconstructed during deserialization.
    #[serde(default)]
    pub starter_len_mask: FxHashMap<char, u64>,

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
    /// ```rust
    /// use opencc_fmmseg::dictionary_lib::DictMaxLen;
    ///
    /// // Two simple phrase pairs (both 2 chars)
    /// let pairs = vec![
    ///     ("你好".to_string(), "您好".to_string()),
    ///     ("世界".to_string(), "世間".to_string()),
    /// ];
    ///
    /// // Build the dictionary (use your actual constructor/builder)
    /// let dict = DictMaxLen::build_from_pairs(pairs);
    ///
    /// // Collected metadata
    /// assert!(dict.max_len >= 2);
    /// assert!(dict.min_len >= 1);
    ///
    /// // Per-starter length mask is a bitfield: bit (len-1) corresponds to `len`
    /// // For '你', length = 2 → bit index 1 must be set
    /// let mask = dict.starter_len_mask.get(&'你').copied().unwrap_or(0);
    /// assert_eq!((mask >> 1) & 1, 1, "Expected bit for length=2 to be set");
    ///
    /// // Equivalent fast gate via API
    /// let cap_bit = 2 - 1;
    /// assert!(dict.starter_allows_dict('你', 2, cap_bit));
    /// ```
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

        // let mut starter_cap: FxHashMap<char, u8> = FxHashMap::default();
        // if lower > 0 {
        //     starter_cap.reserve(lower.min(0x10000));
        // }

        let mut starter_len_mask = FxHashMap::default();
        if lower > 0 {
            starter_len_mask.reserve(lower.min(0x10000));
        }

        let mut global_max = 0usize;
        let mut global_min = usize::MAX;

        // NEW: accumulate bitmask of seen key lengths (1..=64)
        let mut key_length_mask: u64 = 0;

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

            if let Some(&c0) = chars.first() {
                // << NEW: length mask >>
                let entry = starter_len_mask.entry(c0).or_insert(0u64);
                Self::set_key_len_bit(entry, len);
            }

            global_max = global_max.max(len);
            global_min = global_min.min(len);

            // NEW: set length bit (1..=64 only)
            Self::set_key_len_bit(&mut key_length_mask, len);

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
                            k,
                            prev,
                            new_val
                        );
                        // For last-wins instead: *e.into_mut() = new_val;
                    }
                    // identical duplicate -> silently ignored
                }
            }
        }

        // If there were no pairs, both bounds are 0
        let min_len = if global_min == usize::MAX {
            0
        } else {
            global_min
        };
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
            key_length_mask,
            starter_len_mask,
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
            // For each key, ensure its starter's mask contains that length.
            // - For len <= 64: the exact bit must be set.
            // - For len > 64: we can only assert that the mask's max is 64 (i.e., "64+ bucket"),
            //   since the mask can't represent >64 exactly.
            for (k_chars, _) in &dict.map {
                if let Some(&c0) = k_chars.first() {
                    let mask = dict.starter_len_mask.get(&c0).copied().unwrap_or(0);
                    let len = k_chars.len();

                    if len == 0 {
                        // Shouldn't happen (keys are non-empty), but guard anyway.
                        debug_assert!(false, "empty key encountered");
                        continue;
                    }

                    if len <= 64 {
                        let bit = len - 1;
                        let has = ((mask >> bit) & 1) == 1;
                        debug_assert!(
                            has,
                            "starter_len_mask missing bit: starter={:?}, key_len={}, mask={:#x}",
                            c0, len, mask
                        );
                    } else {
                        // For >64, we can't check an exact bit; ensure mask's max is 64 (i.e., bit63 set),
                        // or at least that the mask isn't clearly contradicting long keys.
                        let max_from_mask = Self::max_len_from_mask(mask).unwrap_or(0);
                        debug_assert!(
                            max_from_mask == 64 || mask == 0,
                            "inconsistent mask for long key: starter={:?}, key_len={}, mask_max={}, mask={:#x}",
                            c0, len, max_from_mask, mask
                        );
                    }
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
    ///     key_length_mask: 0,
    ///     first_len_mask64: Vec::new(),
    ///     first_char_max_len: Vec::new(),
    ///     starter_len_mask: Default::default(),
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

    /// (Re)builds the **Basic Multilingual Plane (BMP)** starter index arrays from
    /// sparse data. Prefers [`starter_len_mask`] (one pass); falls back to scanning
    /// [`map`] if the mask is empty.
    ///
    /// This regenerates the two dense BMP arrays:
    ///
    /// - [`first_len_mask64`]:
    ///   - Indexed by the starter character’s Unicode scalar value (`u <= 0xFFFF`).
    ///   - Each `u64` stores a **bitmask of existing key lengths** for that starter:
    ///     - **Bit 0** → a key of length 1 exists
    ///     - **Bit 1** → a key of length 2 exists
    ///     - ...
    ///     - **Bit 63** → used when building from `map` as a “≥64” bucket if any
    ///       key length ≥ 64 is encountered (since the mask encodes up to 64).
    ///
    /// - [`first_char_max_len`]:
    ///   - Indexed identically (BMP only).
    ///   - Stores the **maximum key length** (in characters) observed for each starter.
    ///   - When building from `starter_len_mask`, this is derived from the mask’s
    ///     max set bit (≤ 64). When falling back to scanning [`map`], it reflects
    ///     the true maximum, which may exceed 64.
    ///
    /// # Behavior
    /// 1. Ensures both arrays are allocated to length `0x10000` (BMP) and zeroed.
    /// 2. **Fast path:** if [`starter_len_mask`] is non-empty, copy each mask into
    ///    [`first_len_mask64`] for BMP starters and derive [`first_char_max_len`]
    ///    from the mask’s max bit (up to 64).
    /// 3. **Fallback:** if [`starter_len_mask`] is empty, scan all keys in [`map`]
    ///    once, setting the appropriate bit in [`first_len_mask64`] (collapsing
    ///    `len >= 64` to bit 63) and updating [`first_char_max_len`] with the true
    ///    maximum length seen for that starter.
    /// 4. Non-BMP starters (`u > 0xFFFF`) are ignored here (dense tables are BMP-only).
    ///
    /// Global fields [`min_len`](Self::min_len) and [`max_len`](Self::max_len) are
    /// **not** modified by this method; maintain them at build time or from
    /// [`key_length_mask`](Self::key_length_mask).
    ///
    /// # Example
    /// ```
    /// use opencc_fmmseg::dictionary_lib::DictMaxLen;
    ///
    /// let pairs = vec![
    ///     ("你好".to_string(), "您好".to_string()),
    ///     ("你們".to_string(), "您們".to_string()),
    ///     ("世界".to_string(), "世間".to_string()),
    /// ];
    ///
    /// let mut dict = DictMaxLen::build_from_pairs(pairs);
    ///
    /// // Rebuild dense BMP accelerators (normally done during build)
    /// dict.populate_starter_indexes();
    ///
    /// let idx = '你' as usize;
    /// // Binary bit for length = 2 must be set
    /// assert_ne!(dict.first_len_mask64[idx] & (1u64 << (2 - 1)), 0);
    /// // and the per-starter cap must be >= 2
    /// assert!(dict.first_char_max_len[idx] as usize >= 2);
    /// ```
    ///
    /// # Complexity
    /// Let *N* be the number of keys and *S* the number of distinct starters:
    /// - From `starter_len_mask`: **O(S)**
    /// - From `map` (fallback): **O(N)**
    #[inline]
    pub fn populate_starter_indexes(&mut self) {
        const BMP: usize = 0x10000;

        // ensure vectors exist and sized
        if self.first_len_mask64.len() != BMP {
            self.first_len_mask64 = vec![0u64; BMP];
        } else {
            // clear in-place
            for v in &mut self.first_len_mask64 {
                *v = 0;
            }
        }
        if self.first_char_max_len.len() != BMP {
            self.first_char_max_len = vec![0u8; BMP];
        } else {
            for v in &mut self.first_char_max_len {
                *v = 0;
            }
        }

        if !self.starter_len_mask.is_empty() {
            // --- Fast path: one pass over sparse per-starter masks ---
            for (&c, &mask) in &self.starter_len_mask {
                let u = c as u32;
                if u > 0xFFFF {
                    continue;
                } // dense tables are BMP-only
                let i = u as usize;

                // Exact per-starter length mask
                self.first_len_mask64[i] = mask;

                // Derive cap from the mask's max length (1..=64) -> clamp to u8
                if mask != 0 {
                    // same as max_len_from_mask(mask), but inline to avoid fn call if you prefer:
                    let max_len = 64 - mask.leading_zeros() as usize;
                    self.first_char_max_len[i] = u8::try_from(max_len).unwrap_or(u8::MAX);
                }
            }
        } else {
            // --- Fallback: derive both mask and cap by scanning keys once ---
            for k in self.map.keys() {
                if k.is_empty() {
                    continue;
                }
                let c0 = k[0];
                let u = c0 as u32;
                if u > 0xFFFF {
                    continue;
                } // ignore astral in dense tables

                let i = u as usize;
                let len = k.len();

                // Set bit (1..=64→0..=63); collapse >=64 to bit63 if you want a "64+" bucket
                let b = len.saturating_sub(1);
                let bit = if b >= 64 { 63 } else { b };
                self.first_len_mask64[i] |= 1u64 << bit;

                // Update cap (true max, not capped at 64)
                // If you want cap==mask max (≤64), keep the cast below; if you want true max, track separately.
                let cap_u8 = u8::try_from(len).unwrap_or(u8::MAX);
                if cap_u8 > self.first_char_max_len[i] {
                    self.first_char_max_len[i] = cap_u8;
                }
            }
        }

        // NOTE: self.min_len / self.max_len are global and not touched here.
        // Keep them managed at build time (from pairs / recompute) or by key_length_mask.
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
    ///     key_length_mask: 0,
    ///     first_len_mask64: Vec::new(),
    ///     first_char_max_len: Vec::new(),
    ///     starter_len_mask: Default::default(),
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

    // ----- New: key_length_mask and starter_len_mask helpers -----

    /// Set the bit for a given `len` (1..=64) in a `u64` mask.
    ///
    /// Bit index is `len - 1`. Lengths > 64 are ignored (by design).
    #[inline(always)]
    fn set_key_len_bit(mask: &mut u64, len: usize) {
        let b = len.wrapping_sub(1);
        if b < 64 {
            *mask |= 1u64 << b;
        }
    }

    /// Fast global gate: does the dictionary contain **any key** of length `len`?
    ///
    /// Uses the compact [`key_length_mask`](Self::key_length_mask) when available
    /// for lengths `1..=64`, and otherwise falls back to the global range
    /// [`min_len`](Self::min_len)..=[`max_len`](Self::max_len).
    ///
    /// - For `len <= 64` and nonzero mask: returns the bit test.
    /// - For `len > 64` or zero mask: uses the range gate.
    #[inline(always)]
    pub fn has_key_len(&self, len: usize) -> bool {
        if self.key_length_mask != 0 {
            let b = len.wrapping_sub(1);
            if b < 64 {
                return ((self.key_length_mask >> b) & 1) != 0;
            }
            // lengths > 64 fall back to range gate
        }
        len >= self.min_len && len <= self.max_len
    }

    /// Minimum present length (1..=64) encoded in a `u64` mask, or `None` if empty.
    ///
    /// Equivalent to “index of least-significant set bit + 1”.
    #[inline(always)]
    pub const fn min_len_from_mask(mask: u64) -> Option<usize> {
        if mask == 0 {
            None
        } else {
            Some(mask.trailing_zeros() as usize + 1)
        }
    }

    /// Maximum present length (1..=64) encoded in a `u64` mask, or `None` if empty.
    ///
    /// Equivalent to “bit width of mask”.
    #[inline(always)]
    pub const fn max_len_from_mask(mask: u64) -> Option<usize> {
        if mask == 0 {
            None
        } else {
            Some(64 - mask.leading_zeros() as usize)
        }
    }

    /// Return the per-starter length mask for `starter`.
    ///
    /// - **Dense BMP fast-path:** if the dense tables are populated
    ///   (`first_len_mask64.len() == 0x10000`), returns the BMP entry directly
    ///   (unchecked load guarded by the length check).
    /// - **Sparse path:** otherwise, looks up `starter` in
    ///   [`starter_len_mask`](Self::starter_len_mask) and returns `0` if absent.
    ///
    /// Only lengths `1..=64` are representable in the returned mask.
    #[inline(always)]
    pub fn get_starter_mask(&self, starter: char) -> u64 {
        let u = starter as u32;
        if u <= 0xFFFF && self.first_len_mask64.len() == 0x10000 {
            unsafe { *self.first_len_mask64.get_unchecked(u as usize) }
        } else {
            *self.starter_len_mask.get(&starter).unwrap_or(&0)
        }
    }

    /// Exact per-starter gate: does **any key** of `length` start with `starter`?
    ///
    /// Implemented as a single bit test on the per-starter mask; returns `false`
    /// for `length > 64` since the mask encodes up to 64 only. For `>64` gating in
    /// the dense BMP path, use [`first_char_max_len`](Self::first_char_max_len)
    /// (see `starter_allows_dict`), not this helper.
    #[inline(always)]
    pub fn has_starter_len(&self, starter: char, length: usize) -> bool {
        let b = length.wrapping_sub(1);
        if b >= 64 {
            return false;
        }
        (self.get_starter_mask(starter) >> b) & 1 == 1
    }

    // ----- New: Starter Gate -----
    //
    /// Checks whether this dictionary allows a word of the specified `length`
    /// to start with the provided `starter` character.
    ///
    /// This method performs a fast per-starter lookup using precomputed **length
    /// bitmasks** (1..=64 → bits 0..=63), optionally backed by a dense BMP table:
    ///
    /// - For **BMP characters** (`u <= 0xFFFF`):
    ///   - If dense arrays are populated (`first_len_mask64` and `first_char_max_len`
    ///     both have length `0x10000`):
    ///     1. For `length` in **1..=64**, test the corresponding bit in
    ///        `first_len_mask64[u]`. This is the most selective and fastest path.
    ///     2. For `length > 64`, compare against `first_char_max_len[u]` (a cap
    ///        derived at build time from per-starter masks).
    ///   - If dense arrays are **not** available, fall back to the sparse
    ///     per-starter mask stored in [`starter_len_mask`]. Only lengths 1..=64
    ///     are representable in this mask; lengths > 64 will return `false`.
    ///
    /// - For **astral characters** (`u > 0xFFFF`), the dense BMP tables do not
    ///   apply; the method uses the sparse per-starter mask from
    ///   [`starter_len_mask`] (again, only 1..=64 are representable).
    ///
    /// This method is typically used **after** filtering with
    /// [`DictMaxLen::has_key_len()`] to avoid redundant global range checks.
    ///
    /// # Parameters
    /// - `starter`: The candidate starting character.
    /// - `length`: The word length to validate.
    /// - `bit`: The bit index corresponding to `length` (usually `length - 1`);
    ///   only meaningful for `length` in 1..=64.
    ///
    /// # Returns
    /// - `true` if the dictionary contains at least one entry that starts with
    ///   `starter` and has the specified `length`.
    /// - `false` otherwise.
    ///
    /// # Safety
    /// Uses unchecked indexing (`get_unchecked`) in the dense BMP path, guarded
    /// by prior length checks (`len == 0x10000`). This is safe because the vectors
    /// are guaranteed to have the BMP size when the dense path is taken.
    ///
    /// # Examples
    /// ```ignore
    /// // Checks whether a 2-character phrase starting with '中' exists.
    /// let ok = dict.starter_allows_dict('中', 2, 1);
    /// if ok {
    ///     println!("A 2-character phrase starting with '中' exists.");
    /// }
    /// ```
    #[inline(always)]
    pub fn starter_allows_dict(&self, starter: char, length: usize, bit: usize) -> bool {
        let u = starter as u32;

        // Dense BMP fast-path
        if u <= 0xFFFF
            && self.first_char_max_len.len() == 0x10000
            && self.first_len_mask64.len() == 0x10000
        {
            let i = u as usize;
            // Safety: guarded by the length checks above.
            let m = unsafe { *self.first_len_mask64.get_unchecked(i) };

            // Exact lengths 1..=64 via bit test
            if bit < 64 {
                return ((m >> bit) & 1) != 0;
            }

            // For >64, use dense cap (derived during populate)
            let cap = unsafe { *self.first_char_max_len.get_unchecked(i) } as usize;
            return length <= cap;
        }

        // Unified sparse path (BMP w/o dense OR astral)
        if bit >= 64 {
            return false; // sparse mask can’t represent >64
        }
        let m = self.get_starter_mask(starter); // reads sparse; BMP-dense won’t reach here
        ((m >> bit) & 1) != 0
    }
}

impl Default for DictMaxLen {
    /// Creates an empty [`DictMaxLen`] with all fields initialized to their defaults.
    ///
    /// - [`map`] — empty `FxHashMap`.
    /// - [`min_len`] — `0`.
    /// - [`max_len`] — `0`.
    /// - [`key_length_mask`] — `0` (no global lengths known).
    /// - [`starter_len_mask`] — empty `FxHashMap` (no per-starter lengths known).
    /// - [`first_len_mask64`] — empty `Vec` (call
    ///   [`ensure_starter_indexes`](Self::ensure_starter_indexes) or
    ///   [`populate_starter_indexes`](Self::populate_starter_indexes) to allocate).
    /// - [`first_char_max_len`] — empty `Vec` (same allocation note as above).
    ///
    /// This is equivalent to:
    /// ```
    /// use opencc_fmmseg::dictionary_lib::DictMaxLen;
    /// use rustc_hash::FxHashMap;
    ///
    /// let dict = DictMaxLen {
    ///     map: FxHashMap::default(),
    ///     min_len: 0,
    ///     max_len: 0,
    ///     key_length_mask: 0,
    ///     starter_len_mask: FxHashMap::default(),
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
    /// assert_eq!(dict.min_len, 0);
    /// assert!(dict.map.is_empty());
    /// assert!(!dict.is_populated());
    /// ```
    fn default() -> Self {
        Self {
            map: FxHashMap::default(),
            min_len: 0,
            max_len: 0,
            key_length_mask: 0,
            starter_len_mask: FxHashMap::default(),
            first_len_mask64: Vec::new(),
            first_char_max_len: Vec::new(),
        }
    }
}
