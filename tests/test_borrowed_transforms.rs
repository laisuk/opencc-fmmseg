use opencc_fmmseg::{DetofuLevel, DetofuMap, OpenCC};
use std::borrow::Cow;

#[test]
fn borrowed_results_depend_only_on_input_lifetime() {
    let input = String::from("ABC 普通漢字𠀀");
    let results = {
        let cc = OpenCC::new();
        let map = DetofuMap::builtin(DetofuLevel::ExtB);
        [
            cc.normalize_compat_cow(&input),
            cc.normalize_unicode_compat_cow(&input),
            cc.normalize_compat_extended_cow(&input),
            cc.detofu_cow(&input, DetofuLevel::ExtB),
            map.detofu_cow(&input),
        ]
    };
    for result in results {
        assert!(matches!(result, Cow::Borrowed(_)));
        assert_eq!(result.as_ptr(), input.as_ptr());
    }
}

#[test]
fn owned_public_signatures_and_outputs_are_preserved() {
    let cc = OpenCC::new();
    // Function pointer assignments guard the existing signatures.
    let _: fn(&OpenCC, &str) -> String = OpenCC::normalize_compat;
    let _: fn(&OpenCC, &str) -> String = OpenCC::normalize_compat_extended;
    let _: fn(&OpenCC, &str) -> String = OpenCC::normalize_unicode_compat;
    let _: fn(&OpenCC, &str, DetofuLevel) -> String = OpenCC::detofu;
    for input in ["", "普通汉字", "金漢😀", "漢金😀", "漢😀金", "丽聼𬴂"] {
        assert_eq!(cc.normalize_compat(input), cc.normalize_compat_cow(input));
        assert_eq!(
            cc.normalize_unicode_compat(input),
            cc.normalize_unicode_compat_cow(input)
        );
        assert_eq!(
            cc.normalize_compat_extended(input),
            cc.normalize_compat_extended_cow(input)
        );
        assert_eq!(
            cc.detofu(input, DetofuLevel::ExtB),
            cc.detofu_cow(input, DetofuLevel::ExtB)
        );
    }
}
