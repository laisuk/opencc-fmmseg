//! Single-scalar replacement with an allocation-free unchanged path.
use std::borrow::Cow;

/// Visit each input scalar once. Allocate only at the first actual change and
/// copy the already visited UTF-8 prefix without decoding it again.
pub(crate) fn replace_chars(input: &str, mut map: impl FnMut(char) -> char) -> Cow<'_, str> {
    let Some(first) = first_replacement(input, &mut map) else {
        return Cow::Borrowed(input);
    };
    let mut output = first.start_output();
    for ch in first.rest.chars() {
        output.push(map(ch));
    }
    Cow::Owned(output)
}

/// The unchanged prefix and unvisited suffix around the first replacement.
pub(crate) struct Replacement<'a> {
    prefix: &'a str,
    replacement: char,
    pub(crate) rest: &'a str,
    capacity: usize,
}

impl Replacement<'_> {
    pub(crate) fn start_output(&self) -> String {
        let mut output = String::with_capacity(self.capacity);
        output.push_str(self.prefix);
        output.push(self.replacement);
        output
    }
}

/// Stop at the first actual change, leaving the suffix for the caller's writer.
pub(crate) fn first_replacement(
    input: &str,
    mut map: impl FnMut(char) -> char,
) -> Option<Replacement<'_>> {
    let mut chars = input.chars();
    while let Some(ch) = chars.next() {
        let replacement = map(ch);
        if replacement != ch {
            // Chars::as_str starts at a UTF-8 boundary. Derive the byte offset
            // only on a match instead of tracking byte indices for every scalar.
            let index = input.len() - chars.as_str().len() - ch.len_utf8();
            return Some(Replacement {
                prefix: &input[..index],
                replacement,
                rest: chars.as_str(),
                capacity: input.len() + replacement.len_utf8().saturating_sub(ch.len_utf8()),
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn borrows_unchanged_input() {
        for input in ["", "ASCII", "普通汉字與漢字", "𠀀😀"] {
            let result = replace_chars(input, |ch| ch);
            assert!(matches!(result, Cow::Borrowed(_)));
            assert_eq!(result.as_ptr(), input.as_ptr());
        }
    }

    #[test]
    fn replaces_at_utf8_boundaries_and_visits_each_scalar_once() {
        for input in ["A漢😀", "漢A😀", "漢😀A", "AA", "𠀀A𠀀"] {
            let mut calls = 0;
            let result = replace_chars(input, |ch| {
                calls += 1;
                if ch == 'A' {
                    '𠀁'
                } else {
                    ch
                }
            });
            assert!(matches!(result, Cow::Owned(_)));
            assert_eq!(result, input.replace('A', "𠀁"));
            assert_eq!(calls, input.chars().count());
        }
        assert_eq!(
            replace_chars("漢𠀀😀", |ch| if ch == '𠀀' { 'A' } else { ch }),
            "漢A😀"
        );
    }

    #[test]
    fn builtin_normalizers_match_scalar_reference_for_all_unicode_scalars() {
        let input: String = (0..=0x10ffff).filter_map(char::from_u32).collect();
        let compat = crate::compat_ideographs::CompatIdeographs::builtin();
        let unicode = crate::unicode_compat::UnicodeCompat::builtin();
        let expected: String = input.chars().map(|ch| compat.normalize_char(ch)).collect();
        assert_eq!(compat.normalize_cow(&input), expected);
        let expected: String = input.chars().map(|ch| unicode.normalize_char(ch)).collect();
        assert_eq!(unicode.normalize_cow(&input), expected);
        let expected: String = input
            .chars()
            .map(|ch| unicode.normalize_all_char(ch))
            .collect();
        assert_eq!(unicode.normalize_all_cow(&input), expected);
    }
}
