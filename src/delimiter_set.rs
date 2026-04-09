use std::sync::OnceLock;

/// Full delimiter set used for text segmentation, matching the C# implementation.
///
/// This string literal contains all whitespace, ASCII punctuation, and common
/// Chinese punctuation marks considered delimiters by the segmentation engine.
/// It is used to build the [`DelimiterSet`] bitset at startup.
const FULL_DELIMITERS: &str =
    " \t\n\r!\"#$%&'()*+,-./:;<=>?@[\\]^_{}|~＝、。﹁﹂—－（）《》〈〉？！…／＼︒︑︔︓︿﹀︹︺︙︐［﹇］﹈︕︖︰︳︴︽︾︵︶｛︷｝︸﹃﹄【︻】︼　～．，；：";

/// Compact, hot-path-friendly delimiter set optimized for per-character
/// membership tests.
///
/// # Design
///
/// * **ASCII fast path**: all code points `U+0000..=U+007F` are stored in a
///   single [`u128`] mask. Testing membership is a single shift and bitwise AND.
/// * **BMP fast path**: all code points `U+0000..=U+FFFF` are stored in a
///   65,536-bit table (`[u64; 1024]`, ~8 KB). Each character maps to one bit,
///   making lookup a constant-time O(1) operation with predictable branch-free code.
/// * **Astral characters**: `U+10000..` are always reported as non-delimiters,
///   since no delimiters exist in that range for this project.
///
/// This design avoids the hashing overhead of a `HashSet<char>` and is especially
/// effective in hot loops that scan millions of characters.
#[derive(Copy, Clone)]
pub struct DelimiterSet {
    /// Bitmask for ASCII delimiter membership (`U+0000..=U+007F`).
    ascii_mask: u128,
    /// Bitmap covering delimiter membership for the Unicode BMP (`U+0000..=U+FFFF`).
    bmp_bits: [u64; 1024],
}

impl DelimiterSet {
    /// Tests whether the given [`char`] is a delimiter according to this set.
    ///
    /// # Examples
    ///
    /// ```
    /// use opencc_fmmseg::is_delimiter;
    /// assert!(is_delimiter('。'));
    /// assert!(!is_delimiter('你'));
    /// ```
    #[inline(always)]
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
/// This structure is initialized once at runtime using [`OnceLock`], after
/// which all lookups are lock-free and O(1).
///
/// The generated [`DelimiterSet`] contains:
///
/// - a 128-bit ASCII bitmap (`ascii_mask`) for fast checks of ASCII delimiters
/// - a 1024-entry bitmap (`bmp_bits`) covering the entire Unicode BMP
///
/// These bitmaps allow delimiter detection via simple bit operations,
/// avoiding hash lookups and enabling very fast segmentation when
/// processing large texts.
///
/// This static is used internally by [`is_delimiter`] and other segmentation
/// helpers that operate on delimiter boundaries.
static FULL_DELIMITER_SET: OnceLock<DelimiterSet> = OnceLock::new();

/// Returns the lazily initialized global [`DelimiterSet`].
///
/// The set is constructed once from [`FULL_DELIMITERS`] on first use, then
/// reused for all subsequent delimiter checks.
///
/// # Returns
///
/// A reference to the global [`DelimiterSet`].
#[inline]
fn full_delimiter_set() -> &'static DelimiterSet {
    FULL_DELIMITER_SET.get_or_init(|| {
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
    })
}

/// Checks whether a character is treated as a segmentation delimiter.
///
/// This function tests whether the given character belongs to the
/// preconfigured internal delimiter set, which includes whitespace,
/// punctuation, and other characters that should act as boundaries during
/// text segmentation.
///
/// It is used internally by the segmenter to split input text into
/// non-delimiter chunks before applying dictionary-based longest-match
/// replacement.
///
/// # Arguments
///
/// * `c` - The character to test.
///
/// # Returns
///
/// `true` if the character is a delimiter, otherwise `false`.
#[inline(always)]
pub fn is_delimiter(c: char) -> bool {
    full_delimiter_set().contains(c)
}
