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
    /// use opencc_fmmseg::dictionary_lib::StarterUnion;
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
        let mut bmp_cap = vec![0u8; N];
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
                        if *m < len as u8 {
                            *m = len as u8
                        }
                    })
                    .or_insert(len as u8);
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
