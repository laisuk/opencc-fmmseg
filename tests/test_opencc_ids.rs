use opencc_fmmseg::{OpenCC, OpenccConfig};

#[test]
fn test_preserve_ids_default_false() {
    let cc = OpenCC::new();
    assert!(!cc.get_preserve_ids());
}

#[test]
fn test_set_get_preserve_ids() {
    let mut cc = OpenCC::new();

    assert!(!cc.get_preserve_ids());

    cc.set_preserve_ids(true);
    assert!(cc.get_preserve_ids());

    cc.set_preserve_ids(false);
    assert!(!cc.get_preserve_ids());
}

#[test]
fn test_keep_ids_preserves_simple_ids() {
    let mut cc = OpenCC::new();
    cc.set_preserve_ids(true);

    assert_eq!(
        cc.convert_with_config("⿰钅漢", OpenccConfig::T2s, false),
        "⿰钅漢"
    );
}

#[test]
fn test_keep_ids_preserves_ids_inside_sentence() {
    let mut cc = OpenCC::new();
    cc.set_preserve_ids(true);

    assert_eq!(
        cc.convert("這個字可寫作⿰氵漢。", "t2s", false),
        "这个字可写作⿰氵漢。"
    );
}

#[test]
fn test_keep_ids_preserves_nested_ids() {
    let mut cc = OpenCC::new();
    cc.set_preserve_ids(true);

    assert_eq!(
        cc.convert("這個結構是⿰氵⿱日漢。", "t2s", false),
        "这个结构是⿰氵⿱日漢。"
    );
}

#[test]
fn test_keep_ids_preserves_ternary_ids() {
    let mut cc = OpenCC::new();
    cc.set_preserve_ids(true);

    assert_eq!(cc.convert("IDS:⿲亻言马", "s2t", false), "IDS:⿲亻言马");
}

#[test]
fn test_keep_ids_preserves_unary_ids() {
    let mut cc = OpenCC::new();
    cc.set_preserve_ids(true);

    assert_eq!(cc.convert("IDS:⿾日", "s2t", false), "IDS:⿾日");
}

#[test]
fn test_incomplete_ids_not_special_cased() {
    let mut cc = OpenCC::new();
    cc.set_preserve_ids(true);

    // Incomplete IDS should fall through to normal conversion.
    assert_eq!(cc.convert("⿰钅", "s2t", false), "⿰釒");
}

#[test]
fn test_keep_ids_disabled_allows_normal_conversion_inside_ids() {
    let cc = OpenCC::new();

    assert_eq!(cc.convert("⿰氵漢", "t2s", false), "⿰氵汉");
}

#[test]
fn test_keep_ids_disabled_nested_ids_converts_components() {
    let cc = OpenCC::new();

    assert_eq!(cc.convert("⿰氵⿱日漢", "t2s", false), "⿰氵⿱日汉");
}

#[test]
fn test_keep_ids_preserves_nested_ma_ids() {
    let mut cc = OpenCC::new();
    cc.set_preserve_ids(true);

    assert_eq!(
        cc.convert("這個字可寫作⿱⿰口口馬。", "t2s", false),
        "这个字可写作⿱⿰口口馬。"
    );
}

#[test]
fn test_keep_ids_disabled_nested_ma_ids_converts_components() {
    let cc = OpenCC::new();

    assert_eq!(cc.convert("⿱⿰口口馬", "t2s", false), "⿱⿰口口马");
}

#[test]
fn test_keep_ids_preserves_ma_character_decomposition() {
    let mut cc = OpenCC::new();
    cc.set_preserve_ids(true);

    assert_eq!(
        cc.convert("駡可分解為⿱⿰口口馬。", "t2s", false),
        "骂可分解为⿱⿰口口馬。"
    );
}
