use crate::dictionary_lib::DictMaxLen;
use rustc_hash::FxHashMap;

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
#[derive(Default, Debug)]
pub struct StarterUnion {
    /// Dense BMP length bitmasks, indexed by `starter as usize`.
    pub bmp_mask: Vec<u64>, // 0x10000

    /// Dense BMP per-starter maximum length (in characters), indexed by `starter as usize`.
    pub bmp_cap: Vec<u8>, // 0x10000

    /// Sparse length bitmasks for astral starters (`starter > 0xFFFF`).
    pub astral_mask: FxHashMap<char, u64>,

    /// Sparse per-starter maximum length (in characters) for astral starters.
    pub astral_cap: FxHashMap<char, u8>,
}

impl StarterUnion {
    /// Builds a **union of starter metadata** from multiple [`DictMaxLen`] tables.
    ///
    /// This method constructs a combined lookup for:
    /// - [`bmp_mask`]: per-BMP codepoint bitmask, where bit *n* means at least one key
    ///   of length `n + 1` starts with that character.
    /// - [`bmp_cap`]: the maximum key length observed for each BMP starter.
    /// - [`astral_mask`]/[`astral_cap`]: sparse equivalents for non-BMP starters.
    ///
    /// # Behavior
    /// For each input [`DictMaxLen`]:
    /// - Iterates directly over its [`starter_len_mask`] map (`char → u64`), instead of
    ///   scanning all 65 536 BMP codepoints.
    /// - Uses [`DictMaxLen::max_len_from_mask`] to determine the per-starter cap
    ///   (`Option<u8>` → `unwrap_or(0)`).
    /// - Bitwise-ORs masks across all dictionaries and retains the element-wise maximum
    ///   cap value.
    ///
    /// This avoids the previous `O(D × 65 536)` fixed-range sweep, yielding much faster
    /// startup times while producing identical runtime tables.
    ///
    /// # Requirements
    /// Each [`DictMaxLen`] must have populated its starter indexes
    /// (i.e., [`populate_starter_indexes()`] has been called,
    /// which is already done by [`DictMaxLen::build_from_pairs`]).
    ///
    /// # Complexity
    /// Let *S* be the total number of distinct starters across all dictionaries.
    /// The union now runs in `O(S)` instead of `O(D × 65 536)`, typically improving
    /// initialization speed by several times, especially for sparse lexicons.
    ///
    /// # Example
    /// ```
    /// use opencc_fmmseg::dictionary_lib::{DictMaxLen, StarterUnion};
    ///
    /// // d1: has "你好" and one non-BMP char key "𢫊"
    /// let d1 = DictMaxLen::build_from_pairs(vec![
    ///     ("你好".into(), "您好".into()),
    ///     ("𢫊".into(), "替".into()),
    /// ]);
    ///
    /// // d2: has single-char "你" and "世界"
    /// let d2 = DictMaxLen::build_from_pairs(vec![
    ///     ("你".into(), "您".into()),
    ///     ("世界".into(), "世間".into()),
    /// ]);
    ///
    /// // Build starter union (sparse iteration)
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
