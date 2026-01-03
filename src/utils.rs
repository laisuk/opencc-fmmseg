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
/// use opencc_fmmseg::utils::for_each_len_dec;
/// let mask = (1u64 << 0) | (1u64 << 2) | (1u64 << 63);
///
/// let mut seen = Vec::new();
/// for_each_len_dec(mask, 5, |len| { seen.push(len); false });
/// assert_eq!(seen, vec![3, 1]); // CAP ignored since cap_here=5 < 64
/// ```
#[inline(always)]
pub fn for_each_len_dec(mask: u64, cap_here: usize, mut f: impl FnMut(usize) -> bool) {
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
/// use opencc_fmmseg::utils::find_max_utf8_length;
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

/// Finds a safe UTF-8 boundary within a raw byte slice, limited by a maximum byte count.
///
/// This function is intended for **FFI or raw byte inputs** where the data may end in the
/// middle of a UTF-8 codepoint. It backtracks from `max` until it reaches a byte that is
/// **not a UTF-8 continuation byte** (`0b10xxxxxx`), ensuring the returned index does not
/// split a UTF-8 character.
///
/// # Notes
/// - The returned index is **not guaranteed to be valid UTF-8 by itself**; callers should
///   validate the resulting slice with `std::str::from_utf8` before converting to `&str`.
/// - This function only fixes the common case where the input is **truncated at the end**.
///   It does not repair arbitrary malformed UTF-8.
///
/// # Arguments
/// * `bytes` – Input byte slice to examine.
/// * `max` – Maximum allowed byte count.
///
/// # Returns
/// A byte index at or below `max` that does not cut a UTF-8 codepoint.
///
/// # Example
/// ```rust
/// use opencc_fmmseg::utils::find_max_utf8_len_bytes;
/// let bytes = "汉字转换测试".as_bytes(); // UTF-8, 3 bytes per CJK char
/// let safe = find_max_utf8_len_bytes(bytes, 7);
/// let prefix = &bytes[..safe];
/// assert!(std::str::from_utf8(prefix).is_ok());
/// ```
pub fn find_max_utf8_len_bytes(bytes: &[u8], max: usize) -> usize {
    if bytes.len() <= max {
        return bytes.len();
    }
    let mut i = max;
    while i > 0 && (bytes[i] & 0b1100_0000) == 0b1000_0000 {
        i -= 1;
    }
    i
}

