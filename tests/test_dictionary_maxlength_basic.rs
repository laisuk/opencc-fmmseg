use opencc_fmmseg::dictionary_lib::DictMaxLen;
use opencc_fmmseg::{
    CustomDictFileSpec, CustomDictMode, CustomDictSpec, DictSlot, DictionaryMaxlength, OpenCC,
};
use serde_cbor::from_slice;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use zstd::stream::Encoder;

fn dictionary_with_custom_pair(
    slot: DictSlot,
    key: &str,
    value: &str,
    mode: CustomDictMode,
) -> DictionaryMaxlength {
    DictionaryMaxlength::from_dicts_custom(&[CustomDictSpec {
        slot,
        pairs: vec![(key.to_string(), value.to_string())],
        mode,
    }])
    .expect("Failed to create custom dictionary")
}

fn map_key(key: &str) -> Vec<char> {
    key.chars().collect()
}

#[test]
#[ignore]
fn test_dictionary_from_dicts_then_to_cbor() {
    let dictionary = DictionaryMaxlength::from_dicts().unwrap();
    assert_eq!(dictionary.st_phrases.max_len, 12);

    let filename = "dictionary_maxlength.cbor";
    dictionary.serialize_to_cbor(filename).unwrap();

    let file_contents = fs::read(filename).unwrap();
    let actual_size = file_contents.len();

    const MIN_CBOR_SIZE: usize = 1_300_000;
    const MAX_CBOR_SIZE: usize = 1_600_000;

    assert!(
        (MIN_CBOR_SIZE..=MAX_CBOR_SIZE).contains(&actual_size),
        "unexpected CBOR size: {} bytes, expected between {} and {} bytes",
        actual_size,
        MIN_CBOR_SIZE,
        MAX_CBOR_SIZE
    );

    fs::remove_file(filename).unwrap();
}

#[test]
#[ignore]
fn test_dictionary_from_dicts_then_to_zstd() {
    let dictionary = DictionaryMaxlength::from_dicts().unwrap();

    let cbor_filename = "dictionary_maxlength.cbor";
    dictionary.serialize_to_cbor(cbor_filename).unwrap();

    let cbor_data = fs::read(cbor_filename).unwrap();

    let zstd_filename = "dictionary_maxlength.zstd";
    let zstd_file = File::create(zstd_filename).expect("Failed to create zstd file");
    let mut encoder = Encoder::new(&zstd_file, 19).expect("Failed to create zstd encoder");
    encoder
        .write_all(&cbor_data)
        .expect("Failed to write compressed data");
    encoder.finish().expect("Failed to finish compression");

    let compressed_size = fs::metadata(zstd_filename).unwrap().len();
    let min_size = 480000;
    let max_size = 680000;
    assert!(
        compressed_size >= min_size && compressed_size <= max_size,
        "Unexpected compressed size: {}",
        compressed_size
    );

    fs::remove_file(cbor_filename).unwrap();
    fs::remove_file(zstd_filename).unwrap();
}

#[test]
fn test_dictionary_from_zstd() {
    let dictionary = DictionaryMaxlength::from_zstd().expect("Failed to load dictionary from zstd");

    assert_eq!(dictionary.st_phrases.max_len, 12);
}

#[test]
fn old_cbor_without_forward_variant_phrase_fields_deserializes() {
    #[derive(serde::Serialize)]
    struct LegacyDictionaryMaxlength {
        st_characters: DictMaxLen,
        st_phrases: DictMaxLen,
        ts_characters: DictMaxLen,
        ts_phrases: DictMaxLen,
        tw_phrases: DictMaxLen,
        tw_phrases_rev: DictMaxLen,
        tw_variants: DictMaxLen,
        tw_variants_rev: DictMaxLen,
        tw_variants_rev_phrases: DictMaxLen,
        hk_variants: DictMaxLen,
        hk_variants_rev: DictMaxLen,
        hk_variants_rev_phrases: DictMaxLen,
        jps_characters: DictMaxLen,
        jps_phrases: DictMaxLen,
        jp_variants: DictMaxLen,
        jp_variants_rev: DictMaxLen,
        st_punctuations: DictMaxLen,
        ts_punctuations: DictMaxLen,
    }

    let legacy = LegacyDictionaryMaxlength {
        st_characters: DictMaxLen::default(),
        st_phrases: DictMaxLen::default(),
        ts_characters: DictMaxLen::default(),
        ts_phrases: DictMaxLen::default(),
        tw_phrases: DictMaxLen::default(),
        tw_phrases_rev: DictMaxLen::default(),
        tw_variants: DictMaxLen::default(),
        tw_variants_rev: DictMaxLen::default(),
        tw_variants_rev_phrases: DictMaxLen::default(),
        hk_variants: DictMaxLen::default(),
        hk_variants_rev: DictMaxLen::default(),
        hk_variants_rev_phrases: DictMaxLen::default(),
        jps_characters: DictMaxLen::default(),
        jps_phrases: DictMaxLen::default(),
        jp_variants: DictMaxLen::default(),
        jp_variants_rev: DictMaxLen::default(),
        st_punctuations: DictMaxLen::default(),
        ts_punctuations: DictMaxLen::default(),
    };
    let bytes = serde_cbor::to_vec(&legacy).expect("legacy CBOR should serialize");
    let dictionary: DictionaryMaxlength =
        from_slice(&bytes).expect("legacy CBOR should deserialize");

    assert!(dictionary.tw_variants_phrases.map.is_empty());
    assert!(dictionary.hk_variants_phrases.map.is_empty());
}

#[test]
fn from_dicts_at_missing_forward_variant_phrase_files_defaults_empty() {
    let dir = std::env::temp_dir().join(format!(
        "opencc_fmmseg_missing_variant_phrases_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("temp dict dir should be created");

    for file in [
        "STCharacters.txt",
        "STPhrases.txt",
        "TSCharacters.txt",
        "TSPhrases.txt",
        "TWPhrases.txt",
        "TWPhrasesRev.txt",
        "TWVariants.txt",
        "TWVariantsRev.txt",
        "TWVariantsRevPhrases.txt",
        "HKVariants.txt",
        "HKVariantsRev.txt",
        "HKVariantsRevPhrases.txt",
        "JPShinjitaiCharacters.txt",
        "JPShinjitaiPhrases.txt",
        "JPVariants.txt",
        "JPVariantsRev.txt",
        "STPunctuations.txt",
        "TSPunctuations.txt",
    ] {
        fs::write(dir.join(file), "").expect("temp dictionary file should be written");
    }

    let dictionary =
        DictionaryMaxlength::from_dicts_at(&dir).expect("old plaintext dict set should load");

    assert!(dictionary.tw_variants_phrases.map.is_empty());
    assert!(dictionary.hk_variants_phrases.map.is_empty());
    assert!(dictionary.hk_phrases.map.is_empty());
    assert!(dictionary.hk_phrases_rev.map.is_empty());

    fs::remove_dir_all(&dir).expect("temp dict dir should be removed");
}

#[test]
#[ignore]
fn test_save_compressed() {
    let dictionary = DictionaryMaxlength::from_dicts().expect("Failed to create dictionary");

    let compressed_file = "test_dictionary.zstd";

    let result = DictionaryMaxlength::save_cbor_compressed(&dictionary, compressed_file);
    assert!(
        result.is_ok(),
        "Failed to save compressed dictionary: {:?}",
        result
    );

    let metadata = fs::metadata(compressed_file).expect("Failed to get file metadata");
    assert!(metadata.len() > 0, "Compressed file should not be empty");

    fs::remove_file(compressed_file).expect("Failed to remove test file");
}

#[test]
#[ignore]
fn test_save_and_load_compressed() {
    let dictionary = DictionaryMaxlength::from_dicts().expect("Failed to create dictionary");

    let compressed_file = "test2_dictionary.zstd";

    let save_result = DictionaryMaxlength::save_cbor_compressed(&dictionary, compressed_file);
    assert!(
        save_result.is_ok(),
        "Failed to save compressed dictionary: {:?}",
        save_result
    );

    let load_result = DictionaryMaxlength::load_cbor_compressed(compressed_file);
    assert!(
        load_result.is_ok(),
        "Failed to load compressed dictionary: {:?}",
        load_result
    );

    let loaded_dictionary = load_result.unwrap();

    assert_eq!(
        dictionary.st_phrases.max_len, loaded_dictionary.st_phrases.max_len,
        "Loaded dictionary does not match the original"
    );

    fs::remove_file(compressed_file).expect("Failed to remove test file");
}

#[ignore]
#[test]
fn test_to_dicts_writes_expected_txt_files() -> Result<(), Box<dyn Error>> {
    let output_dir = "test_output_dicts";

    if Path::new(output_dir).exists() {
        fs::remove_dir_all(output_dir)?;
    }

    let pairs = vec![
        ("测试".to_string(), "測試".to_string()),
        ("语言".to_string(), "語言".to_string()),
    ];

    let mut dicts = DictionaryMaxlength::default();
    dicts.st_characters = DictMaxLen::build_from_pairs(pairs.clone());
    dicts.st_phrases = DictMaxLen::build_from_pairs(pairs);

    dicts.to_dicts(output_dir)?;

    let stc_path = format!("{}/STCharacters.txt", output_dir);
    let stp_path = format!("{}/STPhrases.txt", output_dir);

    let content_stc = fs::read_to_string(&stc_path)?;
    let content_stp = fs::read_to_string(&stp_path)?;

    assert!(content_stc.contains("测试\t測試"));
    assert!(content_stc.contains("语言\t語言"));
    assert!(content_stp.contains("测试\t測試"));
    assert!(content_stp.contains("语言\t語言"));

    fs::remove_dir_all(output_dir)?;

    Ok(())
}

#[test]
fn test_from_dicts_custom_append_st_phrases_palantir() {
    let dictionary = dictionary_with_custom_pair(
        DictSlot::STPhrases,
        "帕兰蒂尔",
        "柏蘭蒂爾",
        CustomDictMode::Append,
    );
    let key = map_key("帕兰蒂尔");

    assert_eq!(
        dictionary.st_phrases.map.get(key.as_slice()),
        Some(&"柏蘭蒂爾".into())
    );
}

#[test]
fn test_from_dicts_custom_override_st_phrases_ai_company() {
    let dictionary = DictionaryMaxlength::from_dicts_custom(&[CustomDictSpec {
        slot: DictSlot::STPhrases,
        pairs: vec![("人工智能公司".to_string(), "AI公司".to_string())],
        mode: CustomDictMode::Override,
    }])
    .expect("Failed to create custom dictionary");
    let key = map_key("人工智能公司");

    assert_eq!(
        dictionary.st_phrases.map.get(key.as_slice()),
        Some(&"AI公司".into())
    );
}

#[test]
fn test_from_dicts_custom_multiple_slots() {
    let dictionary = DictionaryMaxlength::from_dicts_custom(&[
        CustomDictSpec {
            slot: DictSlot::STPhrases,
            pairs: vec![("帕兰蒂尔".to_string(), "柏蘭蒂爾".to_string())],
            mode: CustomDictMode::Append,
        },
        CustomDictSpec {
            slot: DictSlot::TSPhrases,
            pairs: vec![("柏蘭蒂爾".to_string(), "帕兰蒂尔".to_string())],
            mode: CustomDictMode::Append,
        },
    ])
    .expect("Failed to create custom dictionary");
    let st_key = map_key("帕兰蒂尔");
    let ts_key = map_key("柏蘭蒂爾");

    assert_eq!(
        dictionary.st_phrases.map.get(st_key.as_slice()),
        Some(&"柏蘭蒂爾".into())
    );
    assert_eq!(
        dictionary.ts_phrases.map.get(ts_key.as_slice()),
        Some(&"帕兰蒂尔".into())
    );
}

#[test]
fn test_from_dicts_custom_append_tw_variants_phrases() {
    let dictionary = dictionary_with_custom_pair(
        DictSlot::TWVariantsPhrases,
        "程式碼",
        "程式碼TW",
        CustomDictMode::Append,
    );
    let key = map_key("程式碼");

    assert_eq!(
        dictionary.tw_variants_phrases.map.get(key.as_slice()),
        Some(&"程式碼TW".into())
    );
}

#[test]
fn test_from_dicts_custom_override_tw_variants_phrases() {
    let dictionary = dictionary_with_custom_pair(
        DictSlot::TWVariantsPhrases,
        "程式碼",
        "程式碼TW",
        CustomDictMode::Override,
    );
    let key = map_key("程式碼");

    assert_eq!(
        dictionary.tw_variants_phrases.map.get(key.as_slice()),
        Some(&"程式碼TW".into())
    );
    assert_eq!(dictionary.tw_variants_phrases.map.len(), 1);
}

#[test]
fn test_from_dicts_custom_append_hk_variants_phrases() {
    let dictionary = dictionary_with_custom_pair(
        DictSlot::HKVariantsPhrases,
        "程式碼",
        "程式碼HK",
        CustomDictMode::Append,
    );
    let key = map_key("程式碼");

    assert_eq!(
        dictionary.hk_variants_phrases.map.get(key.as_slice()),
        Some(&"程式碼HK".into())
    );
}

#[test]
fn test_from_dicts_custom_append_hk_phrases() {
    let dictionary = dictionary_with_custom_pair(
        DictSlot::HKPhrases,
        "小女孩",
        "妹丁",
        CustomDictMode::Append,
    );
    let key = map_key("小女孩");

    assert_eq!(
        dictionary.hk_phrases.map.get(key.as_slice()),
        Some(&"妹丁".into())
    );
}

#[test]
fn test_from_dicts_custom_override_hk_phrases_rev() {
    let dictionary = dictionary_with_custom_pair(
        DictSlot::HKPhrasesRev,
        "妹丁",
        "小女孩",
        CustomDictMode::Override,
    );
    let key = map_key("妹丁");

    assert_eq!(
        dictionary.hk_phrases_rev.map.get(key.as_slice()),
        Some(&"小女孩".into())
    );
    assert_eq!(dictionary.hk_phrases_rev.map.len(), 1);
}

#[test]
fn test_from_dicts_custom_override_hk_variants_phrases() {
    let dictionary = dictionary_with_custom_pair(
        DictSlot::HKVariantsPhrases,
        "程式碼",
        "程式碼HK",
        CustomDictMode::Override,
    );
    let key = map_key("程式碼");

    assert_eq!(
        dictionary.hk_variants_phrases.map.get(key.as_slice()),
        Some(&"程式碼HK".into())
    );
    assert_eq!(dictionary.hk_variants_phrases.map.len(), 1);
}

#[test]
fn test_from_dicts_custom_files_st_phrases_palantir() {
    let dir = std::env::temp_dir();
    let file_path = dir.join("opencc_fmmseg_custom_st_phrases_test.txt");

    fs::write(&file_path, "帕兰蒂尔\t柏蘭蒂爾\n").expect("Failed to write custom dict file");

    let dictionary = DictionaryMaxlength::from_dicts_custom_files(&[CustomDictFileSpec {
        slot: DictSlot::STPhrases,
        files: vec![file_path.clone()],
        mode: CustomDictMode::Override,
    }])
    .expect("Failed to create custom dictionary from files");

    let opencc = OpenCC::from_dictionary(dictionary);

    assert_eq!(
        opencc.convert("帕兰蒂尔是一家人工智能公司", "s2t", false),
        "柏蘭蒂爾是一家人工智能公司"
    );

    let _ = fs::remove_file(file_path);
}

#[test]
fn test_with_custom_dicts_append_st_phrases_palantir() {
    let dictionary = DictionaryMaxlength::from_zstd()
        .expect("Failed to load default dictionary")
        .with_custom_dicts(&[CustomDictSpec {
            slot: DictSlot::STPhrases,
            pairs: vec![("帕兰蒂尔".to_string(), "柏蘭蒂爾".to_string())],
            mode: CustomDictMode::Append,
        }])
        .expect("Failed to apply custom dictionary");

    let opencc = OpenCC::from_dictionary(dictionary);

    assert_eq!(
        opencc.convert("帕兰蒂尔是一家人工智能公司", "s2t", false),
        "柏蘭蒂爾是一家人工智能公司"
    );
}

#[test]
fn test_with_custom_dicts_override_st_phrases_only_custom_pairs_remain() {
    let dictionary = DictionaryMaxlength::from_zstd()
        .expect("Failed to load default dictionary")
        .with_custom_dicts(&[CustomDictSpec {
            slot: DictSlot::STPhrases,
            pairs: vec![("人工智能公司".to_string(), "AI公司".to_string())],
            mode: CustomDictMode::Override,
        }])
        .expect("Failed to apply custom dictionary");
    let key = map_key("人工智能公司");

    assert_eq!(
        dictionary.st_phrases.map.get(key.as_slice()),
        Some(&"AI公司".into())
    );
    assert_eq!(dictionary.st_phrases.map.len(), 1);
}

#[test]
fn test_with_custom_dicts_multiple_slots() {
    let dictionary = DictionaryMaxlength::from_zstd()
        .expect("Failed to load default dictionary")
        .with_custom_dicts(&[
            CustomDictSpec {
                slot: DictSlot::STPhrases,
                pairs: vec![
                    ("帕兰蒂尔".to_string(), "柏蘭蒂爾".to_string()),
                    ("人工智能公司".to_string(), "AI公司".to_string()),
                ],
                mode: CustomDictMode::Append,
            },
            CustomDictSpec {
                slot: DictSlot::TSPhrases,
                pairs: vec![
                    ("柏蘭蒂爾".to_string(), "帕兰蒂尔".to_string()),
                    ("AI公司".to_string(), "人工智能公司".to_string()),
                ],
                mode: CustomDictMode::Append,
            },
        ])
        .expect("Failed to apply custom dictionaries");
    let st_key = map_key("帕兰蒂尔");
    let ts_key = map_key("柏蘭蒂爾");

    assert_eq!(
        dictionary.st_phrases.map.get(st_key.as_slice()),
        Some(&"柏蘭蒂爾".into())
    );
    assert_eq!(
        dictionary.ts_phrases.map.get(ts_key.as_slice()),
        Some(&"帕兰蒂尔".into())
    );
}

#[test]
fn test_with_custom_dict_files_multiple_files_later_wins() {
    let dir = std::env::temp_dir();
    let file1 = dir.join("opencc_fmmseg_custom_file_1.txt");
    let file2 = dir.join("opencc_fmmseg_custom_file_2.txt");

    fs::write(&file1, "帕兰蒂尔\t帕蘭蒂爾\n").expect("Failed to write custom dict file 1");
    fs::write(&file2, "帕兰蒂尔\t柏蘭蒂爾\n").expect("Failed to write custom dict file 2");

    let dictionary = DictionaryMaxlength::from_zstd()
        .expect("Failed to load default dictionary")
        .with_custom_dict_files(&[CustomDictFileSpec {
            slot: DictSlot::STPhrases,
            files: vec![file1.clone(), file2.clone()],
            mode: CustomDictMode::Append,
        }])
        .expect("Failed to apply custom dictionary files");

    let opencc = OpenCC::from_dictionary(dictionary);

    assert_eq!(
        opencc.convert("帕兰蒂尔是一家人工智能公司", "s2t", false),
        "柏蘭蒂爾是一家人工智能公司"
    );

    let _ = fs::remove_file(file1);
    let _ = fs::remove_file(file2);
}
