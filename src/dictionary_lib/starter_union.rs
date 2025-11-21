use crate::dictionary_lib::DictMaxLen;
use rustc_hash::FxHashMap;

/// Union view of starter–length metadata across multiple [`DictMaxLen`] tables.
///
/// A `StarterUnion` merges the **per-starter length bitmasks** and
/// **per-starter maximum phrase lengths** from several dictionaries into a
/// single lookup structure. This allows the segmentation engine to query
/// *all dictionaries at once* when determining whether a given starter
/// character can begin a match of length `L`.
///
/// # Structure
///
/// Starters in the Unicode **BMP** (`0x0000..=0xFFFF`) are stored in dense
/// fixed-size vectors:
///
/// - [`bmp_mask`]: a `u64` bitmask encoding which phrase lengths exist  
/// - [`bmp_cap`]: the maximum phrase length for that starter  
///
/// Starters **outside the BMP** are far less common, so they are stored
/// sparsely:
///
/// - [`astral_mask`]: maps a non-BMP starter to its length bitmask  
/// - [`astral_cap`]: maps a non-BMP starter to its maximum phrase length  
///
/// # Bit Layout (per starter)
///
/// A starter’s bitmask packs available phrase lengths into a `u64`:
///
/// - bit `0` → length = 1  
/// - bit `1` → length = 2  
/// - …  
/// - bit `63` → length = 64  
///
/// Lengths >64 are grouped into the “≥64” bucket and treated equivalently during
/// matching.
///
/// # Invariants
///
/// - `bmp_mask.len() == 0x10000`  
/// - `bmp_cap.len()  == 0x10000`  
/// - If a bit is set in `bmp_mask[i]`, at least one dictionary contains a key
///   that begins with that starter and has the corresponding length.  
/// - `bmp_cap[i]` is always ≥ the highest set bit (converted to a length).  
///
/// These invariants are ensured by [`StarterUnion::build`].
///
/// # Purpose
///
/// The union allows the conversion engine to:
///
/// - Quickly determine whether any dictionary participates in a candidate match  
/// - Avoid scanning dictionaries whose starters cannot match the current input  
/// - Support multi-round dictionary pipelines (S2T → TwPhrases → TwVariants)
///
/// As a result, longest-match scanning becomes extremely fast, relying on
/// O(1) bit tests instead of dictionary lookups.
///
/// # Typical Usage
///
/// A `StarterUnion` is created once for each logical OpenCC configuration
/// (e.g., S2T, T2S+punc, TW-variants-only) and cached inside
/// [`DictionaryMaxlength`].  
///
/// It is then reused across:
///
/// - Simplified/Traditional conversions  
/// - TW/HK/JP variant pipelines  
/// - Parallel conversion workers  
///
/// ensuring consistent, high-performance starter gating across the entire engine.
#[derive(Default, Debug)]
pub struct StarterUnion {
    /// Dense BMP per-starter bitmask.
    ///
    /// Indexed by `starter as usize`, giving a `u64` bitmask with one bit per
    /// possible length (1..=64). The most common case (CJK characters, ASCII,
    /// punctuation) is handled here.
    pub bmp_mask: Vec<u64>, // size: 0x10000

    /// Dense BMP per-starter maximum phrase length.
    ///
    /// Same indexing as [`bmp_mask`]. This provides the upper bound on the
    /// candidate window size during longest-match probing.
    pub bmp_cap: Vec<u8>, // size: 0x10000

    /// Sparse per-starter bitmask for astral (non-BMP) codepoints.
    ///
    /// Keys are `char > 0xFFFF`. These starters are rare, so storing them in a
    /// map saves memory and avoids scanning large unused ranges.
    pub astral_mask: FxHashMap<char, u64>,

    /// Sparse per-starter maximum phrase length for astral starters.
    ///
    /// Mirrors [`astral_mask`], but stores the maximum key length instead of
    /// the full bitmask.
    pub astral_cap: FxHashMap<char, u8>,
}

impl StarterUnion {
    /// Builds a combined **starter metadata union** from multiple [`DictMaxLen`]
    /// dictionaries.
    ///
    /// The resulting [`StarterUnion`] provides fast lookup tables for longest-match
    /// segmentation, combining starter information from all dictionaries in
    /// `dicts`. It produces:
    ///
    /// - [`bmp_mask`]: a per-BMP-codepoint bitmask where bit `n` indicates that
    ///   *some* key of length `n + 1` starts with that character.
    /// - [`bmp_cap`]: the maximum key length observed for each BMP starter.
    /// - [`astral_mask`]/[`astral_cap`]: sparse equivalents for Unicode characters
    ///   outside the BMP.
    ///
    /// # How It Works
    ///
    /// For every provided [`DictMaxLen`] instance:
    ///
    /// - It iterates directly over the dictionary’s [`starter_len_mask`]
    ///   (`char → u64`).
    /// - It merges (`OR`) the per-starter bitmasks across all dictionaries.
    /// - It updates the per-starter “cap” (maximum key length) using the
    ///   element-wise maximum.
    ///
    /// Importantly, **this avoids scanning all 65,536 BMP codepoints**, instead
    /// iterating only over the starters that actually exist in the dictionaries.
    ///
    /// # Complexity
    ///
    /// Let:
    /// - *S* = total number of distinct starter characters across all dictionaries  
    /// - *D* = number of dictionary tables  
    ///
    /// Previous approach:  
    /// `O(D × 65,536)` — fixed-range sweep of all BMP codepoints  
    ///
    /// Current approach:  
    /// `O(S)` — sparse iteration of real starters only  
    ///
    /// This provides **vastly faster startup times**, especially for sparse or
    /// lexicon-heavy OpenCC configurations.
    ///
    /// # Requirements
    ///
    /// Each [`DictMaxLen`] used here **must already have starter indexes populated**,  
    /// which is automatically guaranteed if it was created via:
    ///
    /// - [`DictMaxLen::build_from_pairs`]
    /// - [`DictionaryMaxlength::finish`]
    ///
    /// # Returns
    ///
    /// A fully merged [`StarterUnion`] containing the union of all starters,
    /// masks, and maximum lengths across all provided dictionaries.
    pub fn build(dicts: &[&DictMaxLen]) -> Self {
        const N: usize = 0x10000;
        let mut bmp_mask = vec![0u64; N];
        let mut bmp_cap = vec![0u8; N];
        let mut astral_mask: FxHashMap<char, u64> = FxHashMap::default();
        let mut astral_cap: FxHashMap<char, u8> = FxHashMap::default();

        for d in dicts {
            // Iterate only through existing starters
            for (&c0, &mask) in &d.starter_len_mask {
                if mask == 0 {
                    continue;
                }

                let cap = DictMaxLen::max_len_from_mask(mask).unwrap_or(0) as u8;
                let cp = c0 as u32;

                if cp <= 0xFFFF {
                    let i = cp as usize;
                    bmp_mask[i] |= mask;
                    if cap > bmp_cap[i] {
                        bmp_cap[i] = cap;
                    }
                } else {
                    *astral_mask.entry(c0).or_insert(0) |= mask;
                    astral_cap
                        .entry(c0)
                        .and_modify(|m| {
                            if cap > *m {
                                *m = cap
                            }
                        })
                        .or_insert(cap);
                }
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
