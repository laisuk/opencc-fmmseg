use super::DictMaxLen;
use crate::dictionary_lib::starter_union::StarterUnion;
use crate::{DictionaryMaxlength, OpenCC};
use serde::Serialize;
use serde_cbor::Value;

// The previous five-field wire schema, independent of the new serializer.
#[derive(Serialize)]
struct Legacy<'a> {
    map: &'a rustc_hash::FxHashMap<Box<[char]>, Box<str>>,
    max_len: usize,
    min_len: usize,
    key_length_mask: u64,
    starter_len_mask: &'a rustc_hash::FxHashMap<char, u64>,
}

fn legacy(dict: &DictMaxLen) -> Legacy<'_> {
    Legacy {
        map: &dict.map,
        max_len: dict.max_len,
        min_len: dict.min_len,
        key_length_mask: dict.key_length_mask,
        starter_len_mask: &dict.starter_len_mask,
    }
}

fn build(pairs: &[(&str, &str)]) -> DictMaxLen {
    DictMaxLen::build_from_pairs(pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())))
}

fn fields(bytes: &[u8]) -> std::collections::BTreeMap<Value, Value> {
    match serde_cbor::from_slice(bytes).unwrap() {
        Value::Map(fields) => fields,
        _ => panic!("expected a CBOR map"),
    }
}

fn assert_equivalent(before: &DictMaxLen, after: &DictMaxLen) {
    assert_eq!(before.map, after.map);
    assert_eq!(before.len(), after.len());
    assert_eq!(before.max_len, after.max_len);
    assert_eq!(before.min_len, after.min_len);
    assert_eq!(before.key_length_mask, after.key_length_mask);
    assert_eq!(before.starter_len_mask, after.starter_len_mask);
    for (key, value) in before.iter() {
        assert_eq!(after.get(key), Some(value));
    }
    assert_eq!(before.get(&['\u{10ffff}']), after.get(&['\u{10ffff}']));
}

fn roundtrip(dict: &DictMaxLen, expected: &[(char, u64)]) -> DictMaxLen {
    let bytes = serde_cbor::to_vec(dict).unwrap();
    let physical = fields(&bytes);
    let slim = dict.key_length_mask == 1;
    assert_eq!(physical.len(), if slim { 4 } else { 5 });
    for name in ["map", "max_len", "min_len", "key_length_mask"] {
        assert!(physical.contains_key(&Value::Text(name.into())));
    }
    assert_eq!(
        physical.contains_key(&Value::Text("starter_len_mask".into())),
        !slim
    );
    assert_eq!(dict.starter_len_mask, expected.iter().copied().collect());
    let mut decoded: DictMaxLen = serde_cbor::from_slice(&bytes).unwrap();
    assert_equivalent(dict, &decoded);
    // Dense accelerators retain the existing explicit population lifecycle.
    assert!(decoded.first_len_mask64.is_empty());
    decoded.populate_starter_indexes();
    assert_eq!(dict.first_len_mask64, decoded.first_len_mask64);
    assert_eq!(dict.first_char_max_len, decoded.first_char_max_len);
    decoded
}

#[test]
fn single_scalar_cbor_omits_and_restores_starter_masks() {
    let dict = build(&[("汉", "漢"), ("发", "發"), ("a", "A"), ("𠀀", "𠀁")]);
    assert_eq!(dict.key_length_mask, 1);
    roundtrip(&dict, &[('汉', 1), ('发', 1), ('a', 1), ('𠀀', 1)]);
}

#[test]
fn old_cbor_preserves_supplied_starter_masks() {
    let mut dict = build(&[("汉", "漢"), ("𠀀", "𠀁")]);
    let decoded: DictMaxLen =
        serde_cbor::from_slice(&serde_cbor::to_vec(&legacy(&dict)).unwrap()).unwrap();
    assert_equivalent(&dict, &decoded);
    // An explicitly supplied empty map must not be mistaken for omission.
    dict.starter_len_mask.clear();
    let decoded: DictMaxLen =
        serde_cbor::from_slice(&serde_cbor::to_vec(&legacy(&dict)).unwrap()).unwrap();
    assert!(decoded.starter_len_mask.is_empty());
}

#[test]
fn phrase_cbor_preserves_exact_starter_masks() {
    let dict = build(&[("中国", "中國"), ("中国人", "中國人"), ("𠀀好", "好")]);
    assert_eq!(dict.key_length_mask, 0b110);
    roundtrip(&dict, &[('中', 0b110), ('𠀀', 0b10)]);
}

#[test]
fn mixed_cbor_preserves_exact_starter_masks() {
    let dict = build(&[
        ("中", "中"),
        ("中国人", "中國人"),
        ("𠀀", "𠀁"),
        ("𠀀好", "好"),
    ]);
    assert_eq!(dict.key_length_mask, 0b111);
    roundtrip(&dict, &[('中', 0b101), ('𠀀', 0b11)]);
}

#[test]
fn empty_and_legacy_missing_metadata_keep_defaults() {
    roundtrip(&build(&[]), &[]);
    let decoded: DictMaxLen = serde_cbor::from_slice(
        &serde_cbor::to_vec(&std::collections::BTreeMap::<String, Value>::new()).unwrap(),
    )
    .unwrap();
    assert_equivalent(&DictMaxLen::default(), &decoded);
    let dict = build(&[("中国", "中國")]);
    let mut physical = fields(&serde_cbor::to_vec(&dict).unwrap());
    physical.remove(&Value::Text("starter_len_mask".into()));
    let decoded: DictMaxLen =
        serde_cbor::from_slice(&serde_cbor::to_vec(&physical).unwrap()).unwrap();
    assert!(decoded.starter_len_mask.is_empty());
    assert_eq!(decoded.map, dict.map);
}

#[test]
fn slim_deserialization_preserves_starter_union_after_population() {
    let dict = build(&[("汉", "漢"), ("𠀀", "𠀁")]);
    let phrase = build(&[("汉字", "漢字"), ("𠀀好", "好")]);
    let mut decoded: DictMaxLen =
        serde_cbor::from_slice(&serde_cbor::to_vec(&dict).unwrap()).unwrap();
    decoded.populate_starter_indexes();
    for extra in [false, true] {
        let before = if extra {
            StarterUnion::build(&[&dict, &phrase])
        } else {
            StarterUnion::build(&[&dict])
        };
        let after = if extra {
            StarterUnion::build(&[&decoded, &phrase])
        } else {
            StarterUnion::build(&[&decoded])
        };
        assert_eq!(before.bmp_mask, after.bmp_mask);
        assert_eq!(before.bmp_cap, after.bmp_cap);
        assert_eq!(before.astral_mask, after.astral_mask);
        assert_eq!(before.astral_cap, after.astral_cap);
        assert_eq!(after.astral_mask[&'𠀀'], if extra { 3 } else { 1 });
        assert_eq!(after.astral_cap[&'𠀀'], if extra { 2 } else { 1 });
    }
}

#[test]
fn existing_artifact_slim_roundtrip_preserves_conversion_and_reduces_size() {
    let original = DictionaryMaxlength::new().unwrap();
    let old_size = serde_cbor::to_vec(&legacy(&original.st_characters))
        .unwrap()
        .len();
    let slim_size = serde_cbor::to_vec(&original.st_characters).unwrap().len();
    assert_eq!(original.st_characters.key_length_mask, 1);
    assert!(slim_size < old_size);
    println!("STCharacters CBOR: {old_size} -> {slim_size} bytes");
    let decoded: DictionaryMaxlength =
        serde_cbor::from_slice(&serde_cbor::to_vec(&original).unwrap()).unwrap();
    let decoded = decoded.finish();
    for (before, after) in [
        (&original.st_characters, &decoded.st_characters),
        (&original.ts_characters, &decoded.ts_characters),
        (&original.st_phrases, &decoded.st_phrases),
        (&original.ts_phrases, &decoded.ts_phrases),
    ] {
        assert_equivalent(before, after);
        assert_eq!(before.first_len_mask64, after.first_len_mask64);
        assert_eq!(before.first_char_max_len, after.first_char_max_len);
    }
    let before = OpenCC::from_dictionary(original);
    let after = OpenCC::from_dictionary(decoded);
    let input = "汉字转换，中国头发，软件鼠标。漢字轉換，中國頭髮，軟體滑鼠。𠀀𠮷";
    for config in [
        "s2t", "t2s", "s2tw", "s2twp", "tw2s", "tw2sp", "s2hk", "hk2s", "t2tw", "tw2t", "t2hk",
        "hk2t", "t2jp", "jp2t",
    ] {
        for punctuation in [false, true] {
            assert_eq!(
                before.convert(input, config, punctuation),
                after.convert(input, config, punctuation),
                "{config}"
            );
        }
    }
}
