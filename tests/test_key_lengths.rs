use opencc_fmmseg::{DictMaxLen, DictionaryMaxlength, OpenCC};

fn converter(first: Vec<(String, String)>, second: Vec<(String, String)>) -> OpenCC {
    let mut dictionaries = DictionaryMaxlength::default();
    dictionaries.st_phrases = DictMaxLen::build_from_pairs(first);
    dictionaries.st_characters = DictMaxLen::build_from_pairs(second);
    let mut cc = OpenCC::from_dictionary(dictionaries);
    cc.set_parallel(false);
    cc
}

#[test]
fn key_length_exact_boundaries_and_dictionary_precedence() {
    for ch in ['中', '\u{20000}'] {
        for len in [63, 64, 65, 80, 255] {
            let key = ch.to_string().repeat(len);
            let first = vec![(key.clone(), "FIRST".into())];
            let second = vec![(key.clone(), "SECOND".into())];
            assert_eq!(
                converter(first.clone(), second.clone()).s2t(&key, false),
                "FIRST"
            );
            assert_eq!(converter(second, first).s2t(&key, false), "SECOND");
        }
    }
}

#[test]
fn key_length_long_match_in_either_dictionary_beats_short_match() {
    for ch in ['中', '\u{20000}'] {
        let key = ch.to_string().repeat(80);
        let short = vec![(ch.to_string(), "SHORT".into())];
        let long = vec![(key.clone(), "LONG".into())];
        assert_eq!(
            converter(short.clone(), long.clone()).s2t(&key, false),
            "LONG"
        );
        assert_eq!(converter(long, short).s2t(&key, false), "LONG");
    }
}

#[test]
fn key_length_failed_long_candidate_falls_back_to_short_key() {
    for ch in ['中', '\u{20000}'] {
        let prefix = ch.to_string().repeat(79);
        // Same starter and candidate length, different suffix: the long probe
        // must fail, then length 1 must still be considered at this position.
        let cc = converter(
            vec![(format!("{prefix}甲"), "LONG".into())],
            vec![(ch.to_string(), "S".into())],
        );
        assert_eq!(
            cc.s2t(&format!("{prefix}乙"), false),
            format!("{}乙", "S".repeat(79))
        );
    }
}

#[test]
fn key_length_mixed_one_and_eighty_in_same_dictionary() {
    for ch in ['中', '\u{20000}'] {
        let key = ch.to_string().repeat(80);
        let cc = converter(
            vec![(ch.to_string(), "S".into()), (key.clone(), "LONG".into())],
            vec![("他".into(), "OTHER".into())],
        );
        assert_eq!(cc.s2t(&key, false), "LONG");
        assert_eq!(cc.s2t(&ch.to_string(), false), "S");
    }
}

#[test]
fn key_length_sparse_false_positive_does_not_hide_later_match() {
    let ch = '\u{20000}';
    let key = ch.to_string().repeat(80);
    // The first dictionary has this starter only at length 1, but an unrelated
    // long key raises its global cap. A conservative sparse gate may admit the
    // candidate; a lookup miss must continue to the second dictionary.
    let cc = converter(
        vec![
            (ch.to_string(), "SHORT".into()),
            ("他".repeat(80), "UNRELATED".into()),
        ],
        vec![(key.clone(), "LONG".into())],
    );
    assert_eq!(cc.s2t(&key, false), "LONG");
}
