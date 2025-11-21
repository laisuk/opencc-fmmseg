use once_cell::sync::Lazy;

/// Full delimiter set used for text segmentation, matching the C# implementation.
///
/// This string literal contains all whitespace, ASCII punctuation, and common
/// Chinese punctuation marks considered delimiters by the segmentation engine.
/// It is used to build the [`DelimiterSet`] bitset at startup.
pub const FULL_DELIMITERS: &str =
    " \t\n\r!\"#$%&'()*+,-./:;<=>?@[\\]^_{}|~＝、。﹁﹂—－（）《》〈〉？！…／＼︒︑︔︓︿﹀︹︺︙︐［﹇］﹈︕︖︰︳︴︽︾︵︶｛︷｝︸﹃﹄【︻】︼　～．，；：";

/// Convenience helper for hot paths: tests if a [`char`] is a delimiter using
/// the global [`FULL_DELIMITER_SET`].
///
/// This is equivalent to:
/// ```
/// use opencc_fmmseg::delimiter_set::{is_delimiter, FULL_DELIMITER_SET};
/// let c = '！';
/// assert_eq!(is_delimiter(c), FULL_DELIMITER_SET.contains(c));
/// ```
/// Compact, hot-path friendly delimiter set optimized for per-character membership tests.
///
/// # Design
///
/// * **ASCII fast path**: all code points `U+0000..=U+007F` are stored in a single
///   [`u128`] mask. Testing membership is a single shift and bitwise AND.
/// * **BMP fast path**: all code points `U+0000..=U+FFFF` are stored in a
///   65,536-bit table (`[u64; 1024]`, ~8 KB). Each character maps to one bit,
///   making lookup a constant-time O(1) operation with predictable branch-free code.
/// * **Astral characters**: `U+10000..` are always reported as non-delimiters, since
///   no delimiters exist in that range for this project.
///
/// This design avoids the hashing overhead of a `HashSet<char>` and is especially
/// effective in hot loops that scan millions of characters.
#[derive(Copy, Clone)]
pub struct DelimiterSet {
    ascii_mask: u128,      // bits 0..=127
    bmp_bits: [u64; 1024], // 0x0000..=0xFFFF
}

impl DelimiterSet {
    /// Tests whether the given [`char`] is a delimiter according to this set.
    ///
    /// # Examples
    ///
    /// ```
    /// use opencc_fmmseg::delimiter_set::is_delimiter;
    /// assert!(is_delimiter('。'));
    /// assert!(!is_delimiter('你'));
    /// ```
    #[inline]
    pub fn contains(&self, c: char) -> bool {
        let u = c as u32;
        if u <= 0x7F {
            return ((self.ascii_mask >> u) & 1) == 1;
        }
        if u <= 0xFFFF {
            let i = (u >> 6) as usize;
            let b = u & 63;
            return ((self.bmp_bits[i] >> b) & 1) == 1;
        }
        // Astral punctuation is virtually nonexistent in delimiters set; treat as non-delim
        false
    }
}

/// Global static instance of the [`DelimiterSet`] constructed from
/// [`FULL_DELIMITERS`].
///
/// This structure is initialized once at runtime using [`Lazy`], after
/// which all lookups are **lock-free** and **O(1)**.
///
/// The generated `DelimiterSet` contains:
///
/// - A 128-bit ASCII bitmap (`ascii_mask`) for fast checks of ASCII delimiters
/// - A 1024-entry bitmap (`bmp_bits`) covering the entire Unicode BMP
///
/// These bitmaps allow delimiter detection via simple bit operations,
/// avoiding hash lookups and enabling extremely fast segmentation when
/// processing large texts.
///
/// This static is used internally by [`is_delimiter`] and all segmentation
/// functions that operate on delimiter boundaries.
pub static FULL_DELIMITER_SET: Lazy<DelimiterSet> = Lazy::new(|| {
    let mut ascii: u128 = 0;
    let mut bmp = [0u64; 1024];

    for ch in FULL_DELIMITERS.chars() {
        let u = ch as u32;
        if u <= 0x7F {
            ascii |= 1u128 << u;
        }
        if u <= 0xFFFF {
            let i = (u >> 6) as usize;
            let b = u & 63;
            bmp[i] |= 1u64 << b;
        }
    }

    DelimiterSet {
        ascii_mask: ascii,
        bmp_bits: bmp,
    }
});

/// Checks whether a character is treated as a segmentation delimiter.
///
/// This function tests whether the given character belongs to the
/// preconfigured `FULL_DELIMITER_SET`, which includes whitespace,
/// punctuation, and other characters that should act as boundaries during
/// text segmentation.
///
/// It is used internally by the segmenter to split input text into
/// non-delimiter chunks before applying dictionary-based longest-match
/// replacement.
///
/// # Arguments
///
/// * `c` – The character to test.
///
/// # Returns
///
/// `true` if the character is a delimiter, otherwise `false`.
#[inline]
pub fn is_delimiter(c: char) -> bool {
    FULL_DELIMITER_SET.contains(c)
}

