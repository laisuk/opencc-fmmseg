use std::ops::Range;

/// Maximum recursive nesting accepted for an IDS sequence.
///
/// Real-world IDS data is shallow. This limit prevents malformed input from
/// causing excessive recursion while still allowing deeply nested descriptions.
const MAX_IDS_DEPTH: usize = 16;

/// Arity table for Unicode IDS operators U+2FF0..=U+2FFF.
///
/// Index `0` corresponds to U+2FF0, index `15` to U+2FFF.
///
/// Most IDS operators are binary. U+2FF2 and U+2FF3 are ternary;
/// U+2FFE and U+2FFF are unary.
const IDS_ARITY: [u8; 16] = [2, 2, 3, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1, 1];

/// Returns the char-index range of a complete IDS sequence starting at `start`.
///
/// The returned range uses indices into the provided `chars` slice, not byte
/// offsets into the original string.
///
/// Returns `None` when:
///
/// - `start` is out of bounds;
/// - `chars[start]` is not an IDS operator;
/// - the sequence starting at `start` is incomplete or malformed.
///
/// This function accepts IDS prefixes inside longer text. For example, given
/// `abc⿰氵漢def`, calling this at the `⿰` position returns only the range for
/// `⿰氵漢`.
pub(crate) fn ids_range_at(chars: &[char], start: usize) -> Option<Range<usize>> {
    if start >= chars.len() {
        return None;
    }

    ids_operator_arity(chars[start])?;

    let mut pos = start;

    if consume_ids(chars, &mut pos, 0) {
        Some(start..pos)
    } else {
        None
    }
}

/// Returns `true` when `chars` is exactly one complete IDS sequence.
///
/// Unlike [`ids_range_at`], this function rejects trailing characters after the
/// IDS sequence. For example, `⿰氵漢` returns `true`, while `⿰氵漢abc` returns
/// `false`.
pub(crate) fn is_complete_ids(chars: &[char]) -> bool {
    if chars.is_empty() {
        return false;
    }

    if ids_operator_arity(chars[0]).is_none() {
        return false;
    }

    let mut pos = 0;
    consume_ids(chars, &mut pos, 0) && pos == chars.len()
}

/// Returns the number of operands required by an IDS operator.
///
/// Returns `None` for non-IDS characters.
///
/// Unicode reserves U+2FF0..=U+2FFF for Ideographic Description Characters.
/// This crate treats all code points in that block as IDS operators and uses
/// [`IDS_ARITY`] to determine how many child components each operator consumes.
#[inline]
fn ids_operator_arity(ch: char) -> Option<usize> {
    let u = ch as u32;

    if (0x2FF0..=0x2FFF).contains(&u) {
        Some(IDS_ARITY[(u - 0x2FF0) as usize] as usize)
    } else {
        None
    }
}

/// Recursively consumes one IDS node from `chars[*pos..]`.
///
/// A non-operator character is treated as a leaf component and consumes one
/// char. An IDS operator consumes itself plus the number of child IDS nodes
/// required by its arity.
///
/// On success, `*pos` is advanced to the first character after the consumed IDS
/// node. On failure, `*pos` may have advanced; callers should treat `false` as
/// a parse failure and discard the partial position.
fn consume_ids(chars: &[char], pos: &mut usize, depth: usize) -> bool {
    if *pos >= chars.len() || depth > MAX_IDS_DEPTH {
        return false;
    }

    let ch = chars[*pos];
    *pos += 1;

    let Some(arity) = ids_operator_arity(ch) else {
        return true;
    };

    for _ in 0..arity {
        if !consume_ids(chars, pos, depth + 1) {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    #[test]
    fn test_ids_range_at_simple_binary_ids() {
        let text = chars("⿰钅只");
        assert_eq!(ids_range_at(&text, 0), Some(0..3));
    }

    #[test]
    fn test_ids_range_at_simple_ternary_ids() {
        let text = chars("⿲亻言马");
        assert_eq!(ids_range_at(&text, 0), Some(0..4));
    }

    #[test]
    fn test_ids_range_at_unary_ids() {
        let text = chars("⿾日");
        assert_eq!(ids_range_at(&text, 0), Some(0..2));
    }

    #[test]
    fn test_ids_range_at_nested_ids() {
        let text = chars("⿰钅⿱日月");
        assert_eq!(ids_range_at(&text, 0), Some(0..5));
    }

    #[test]
    fn test_ids_range_at_inside_text() {
        let text = chars("abc⿰钅只def");
        assert_eq!(ids_range_at(&text, 0), None);
        assert_eq!(ids_range_at(&text, 3), Some(3..6));
        assert_eq!(ids_range_at(&text, 6), None);
    }

    #[test]
    fn test_ids_range_at_incomplete_ids_returns_none() {
        let text = chars("⿰钅");
        assert_eq!(ids_range_at(&text, 0), None);
    }

    #[test]
    fn test_ids_range_at_operator_alone_returns_none() {
        let text = chars("⿰");
        assert_eq!(ids_range_at(&text, 0), None);
    }

    #[test]
    fn test_ids_range_at_non_ids_returns_none() {
        let text = chars("國");
        assert_eq!(ids_range_at(&text, 0), None);
    }

    #[test]
    fn test_ids_range_at_complete_ids_prefix_only_returns_ids_range() {
        let text = chars("⿰钅只abc");
        assert_eq!(ids_range_at(&text, 0), Some(0..3));
    }

    #[test]
    fn test_is_complete_ids() {
        assert!(is_complete_ids(&chars("⿰钅只")));
        assert!(is_complete_ids(&chars("⿲亻言马")));
        assert!(is_complete_ids(&chars("⿰钅⿱日月")));

        assert!(!is_complete_ids(&chars("")));
        assert!(!is_complete_ids(&chars("國")));
        assert!(!is_complete_ids(&chars("⿰")));
        assert!(!is_complete_ids(&chars("⿰钅")));
        assert!(!is_complete_ids(&chars("⿰钅只abc")));
        assert!(!is_complete_ids(&chars("abc⿰钅只")));
    }
}
