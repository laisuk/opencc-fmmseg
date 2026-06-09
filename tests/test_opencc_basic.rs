use opencc_fmmseg::dictionary_lib::DictMaxLen;
use opencc_fmmseg::{
    CustomDictMode, CustomDictSpec, DetofuLevel, DetofuMap, DictSlot, DictionaryMaxlength, OpenCC,
    OpenccConfig,
};

#[test]
fn convert_clears_stale_last_error_on_success() {
    let cc = OpenCC::new();

    let invalid = cc.convert("汉字", "invalid", false);
    assert_eq!(invalid, "Invalid config: invalid");
    assert_eq!(
        OpenCC::get_last_error().as_deref(),
        Some("Invalid config: invalid")
    );

    let converted = cc.convert("汉字", "s2t", false);
    assert_eq!(converted, "漢字");
    assert!(OpenCC::get_last_error().is_none());
}

#[test]
fn tw_variant_phrases_apply_before_variant_chars() {
    let mut dictionary = DictionaryMaxlength::default();
    dictionary.tw_variants_phrases =
        DictMaxLen::build_from_pairs(vec![("甲乙".to_string(), "TW_PHRASE".to_string())]);
    dictionary.tw_variants =
        DictMaxLen::build_from_pairs(vec![("甲乙".to_string(), "TW_CHAR".to_string())]);

    let opencc = OpenCC::from_dictionary(dictionary);

    assert_eq!(opencc.t2tw("甲乙"), "TW_PHRASE");
}

#[test]
fn hk_variant_phrases_apply_before_variant_chars() {
    let mut dictionary = DictionaryMaxlength::default();
    dictionary.hk_variants_phrases =
        DictMaxLen::build_from_pairs(vec![("甲乙".to_string(), "HK_PHRASE".to_string())]);
    dictionary.hk_variants =
        DictMaxLen::build_from_pairs(vec![("甲乙".to_string(), "HK_CHAR".to_string())]);

    let opencc = OpenCC::from_dictionary(dictionary);

    assert_eq!(opencc.t2hk("甲乙"), "HK_PHRASE");
}

#[test]
fn direct_conversion_clears_stale_last_error_on_success() {
    let cc = OpenCC::new();

    OpenCC::set_last_error("stale error");
    let converted = cc.convert_with_config("汉字", OpenccConfig::S2t, false);
    assert_eq!(converted, "漢字");
    assert!(OpenCC::get_last_error().is_none());

    OpenCC::set_last_error("stale error");
    let converted = cc.s2t("汉字", false);
    assert_eq!(converted, "漢字");
    assert!(OpenCC::get_last_error().is_none());
}

#[test]
fn convert_preserves_original_line_endings() {
    let cc = OpenCC::new();

    assert_eq!(cc.convert("汉字\r\n转换", "s2t", false), "漢字\r\n轉換");
    assert_eq!(cc.convert("汉字\n转换", "s2t", false), "漢字\n轉換");
    assert_eq!(
        cc.convert("汉字\r\n转换\n测试\r完成", "s2t", false),
        "漢字\r\n轉換\n測試\r完成"
    );
}

#[test]
fn convert_preserves_original_line_endings_in_serial_mode() {
    let mut cc = OpenCC::new();
    cc.set_parallel(false);

    assert_eq!(cc.convert("汉字\r\n转换", "s2t", false), "漢字\r\n轉換");
    assert_eq!(cc.convert("汉字\n转换", "s2t", false), "漢字\n轉換");
    assert_eq!(
        cc.convert("汉字\r\n转换\n测试\r完成", "s2t", false),
        "漢字\r\n轉換\n測試\r完成"
    );
}

#[test]
fn test_opencc_from_dictionary_custom_palantir() {
    let dictionary = DictionaryMaxlength::from_dicts_custom(&[CustomDictSpec {
        slot: DictSlot::STPhrases,
        pairs: vec![("帕兰蒂尔".to_string(), "柏蘭蒂爾".to_string())],
        mode: CustomDictMode::Append,
    }])
    .expect("Failed to create custom dictionary");

    let opencc = OpenCC::from_dictionary(dictionary);

    assert_eq!(
        opencc.convert("帕兰蒂尔是一家人工智能公司", "s2tw", false),
        "柏蘭蒂爾是一家人工智能公司"
    );
}

#[test]
fn test_opencc_detofu() {
    let cc = OpenCC::new();
    let input = "𠉂𪠟𫝈𫬐";

    assert_eq!(cc.detofu(input, DetofuLevel::ExtE), "𠉂𪠟𫝈㘔");
    assert_eq!(cc.detofu(input, DetofuLevel::ExtB), "㒓㓄㑮㘔");
}

#[test]
fn test_opencc_t2s_detofu() {
    let cc = OpenCC::new();

    let output = cc.detofu(
        &cc.convert("儼驂騑於上路，訪風景於崇阿", "t2s", false),
        DetofuLevel::ExtB,
    );

    assert_eq!(output, "俨骖騑于上路，访风景于崇阿");
}

#[test]
fn test_opencc_t2s_detofu_preserves_unmapped_character() {
    let cc = OpenCC::new();

    let converted = cc.convert("儼驂騑於上路，訪風景於崇阿，𱁬", "t2s", false);

    let output = cc.detofu(&converted, DetofuLevel::ExtB);

    assert_eq!(output, "俨骖騑于上路，访风景于崇阿，𱁬");
}

#[test]
fn test_detofu_custom_pairs_override_builtin_mapping() {
    let input = "這隻小狗有𣭲毛";

    assert_eq!(
        DetofuMap::builtin(DetofuLevel::ExtB).detofu(input),
        "這隻小狗有氄毛"
    );

    let map = DetofuMap::builtin(DetofuLevel::ExtB).with_custom_pairs(&[('𣭲', '氂')]);

    assert_eq!(map.detofu(input), "這隻小狗有氂毛");
}

#[test]
fn detofu_with_custom_file_loads_user_mapping() {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    let mut path = std::env::temp_dir();
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    path.push(format!("opencc_fmmseg_custom_tofu_{unique}.txt"));

    fs::write(&path, "𣭲\t氄\tB\n").unwrap();

    let cc = OpenCC::new();
    let result = cc
        .detofu_with_custom_file("𣭲毛", DetofuLevel::ExtB, &path)
        .unwrap();

    fs::remove_file(&path).ok();

    assert_eq!(result, "氄毛");
}

#[test]
fn test_detofu_map_with_custom_pairs() {
    let map = DetofuMap::builtin(DetofuLevel::ExtB).with_custom_pairs(&[('𣭲', '氄')]);

    assert_eq!(map.detofu("𣭲毛"), "氄毛");
}

#[test]
fn test_detofu_map_with_custom_pairs_overrides_builtin() {
    let map = DetofuMap::builtin(DetofuLevel::ExtB).with_custom_pairs(&[('𬴂', '騑')]);

    assert_eq!(map.detofu("骖𬴂"), "骖騑");
}

#[test]
fn test_opencc_detofu_with_custom_pairs() {
    let cc = OpenCC::new();

    let output = cc.detofu_with_custom_pairs(
        "𣭲毛 骖𬴂",
        DetofuLevel::ExtB,
        &[('𣭲', '氄'), ('𬴂', '騑')],
    );

    assert_eq!(output, "氄毛 骖騑");
}

#[test]
fn test_detofu_custom_pairs_later_wins() {
    let map =
        DetofuMap::builtin(DetofuLevel::ExtB).with_custom_pairs(&[('𣭲', '氂'), ('𣭲', '氄')]);

    assert_eq!(map.detofu("𣭲毛"), "氄毛");
}
