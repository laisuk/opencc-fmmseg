use opencc_fmmseg::{DictMaxLen, DictionaryMaxlength};

#[test]
fn advanced_dictionary_types_are_available_from_the_crate_root() {
    let mut dict = DictMaxLen::build_from_pairs(vec![("你好".to_owned(), "您好".to_owned())]);

    assert_eq!(dict.len(), 1);
    assert!(!dict.is_empty());
    assert_eq!(dict.min_key_len(), 2);
    assert_eq!(dict.max_key_len(), 2);
    assert_eq!(dict.get(&['你', '好']), Some("您好"));
    assert_eq!(dict.iter().count(), 1);

    dict.append_pairs([("世界", "世間")]);
    assert_eq!(dict.len(), 2);
    dict.replace_pairs([("汉字", "漢字")]);
    assert_eq!(dict.len(), 1);
    assert_eq!(dict.get(&['汉', '字']), Some("漢字"));

    let _: DictionaryMaxlength = DictionaryMaxlength::default();
}
