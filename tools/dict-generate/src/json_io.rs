// json_io.rs (CLI only)
use opencc_fmmseg::{DictMaxLen, DictionaryMaxlength};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// BTreeMap keeps JSON object keys deterministic for stable diffs.
#[derive(Debug, Default, Deserialize)]
pub struct DictMaxLenSerde {
    pub map: BTreeMap<String, String>,

    #[serde(default)]
    pub max_len: usize,

    // present for completeness; old JSON may omit it
    #[serde(default)]
    pub min_len: usize,

    // NEW: bitmask of existing key lengths (1..=64 mapped to bits 0..=63)
    #[serde(default)]
    pub key_length_mask: u64,

    // NEW: sparse per-starter length mask (1..=64 → bits 0..=63)
    // keys are 1-char strings for determinism in JSON
    #[serde(default)]
    pub starter_len_mask: BTreeMap<String, u64>,
}

// Readable JSON keeps string keys; only the redundant starter field is omitted.
impl Serialize for DictMaxLenSerde {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;

        let slim = self.key_length_mask == 1;
        let mut state = serializer.serialize_struct("DictMaxLenSerde", if slim { 4 } else { 5 })?;
        state.serialize_field("map", &self.map)?;
        state.serialize_field("max_len", &self.max_len)?;
        state.serialize_field("min_len", &self.min_len)?;
        state.serialize_field("key_length_mask", &self.key_length_mask)?;
        if !slim {
            state.serialize_field("starter_len_mask", &self.starter_len_mask)?;
        }
        state.end()
    }
}
impl DictMaxLenSerde {
    #[allow(dead_code)]
    pub fn into_internal(self) -> DictMaxLen {
        // Serialized metadata is treated as derived data. Rebuilding from the
        // semantic key/value pairs prevents stale or inconsistent indexes from
        // crossing the public API boundary.
        DictMaxLen::build_from_pairs(self.map)
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DictionaryMaxlengthSerde {
    pub st_characters: DictMaxLenSerde,
    pub st_phrases: DictMaxLenSerde,
    pub ts_characters: DictMaxLenSerde,
    pub ts_phrases: DictMaxLenSerde,
    pub tw_phrases: DictMaxLenSerde,
    pub tw_phrases_rev: DictMaxLenSerde,
    #[serde(default)]
    pub hk_phrases: DictMaxLenSerde,
    #[serde(default)]
    pub hk_phrases_rev: DictMaxLenSerde,
    #[serde(default)]
    pub tw_variants_phrases: DictMaxLenSerde,
    pub tw_variants: DictMaxLenSerde,
    pub tw_variants_rev: DictMaxLenSerde,
    pub tw_variants_rev_phrases: DictMaxLenSerde,
    #[serde(default)]
    pub hk_variants_phrases: DictMaxLenSerde,
    pub hk_variants: DictMaxLenSerde,
    pub hk_variants_rev: DictMaxLenSerde,
    pub hk_variants_rev_phrases: DictMaxLenSerde,
    pub jps_characters: DictMaxLenSerde,
    pub jps_characters_rev: DictMaxLenSerde,
    pub jps_phrases: DictMaxLenSerde,
    pub st_punctuations: DictMaxLenSerde,
    pub ts_punctuations: DictMaxLenSerde,
}

impl From<&DictMaxLen> for DictMaxLenSerde {
    fn from(d: &DictMaxLen) -> Self {
        // Recompute serialized metadata from semantic entries so the CLI never
        // depends on DictMaxLen's internal representation.
        let mut map = BTreeMap::new();
        let mut starter_len_mask = BTreeMap::new();
        let mut key_length_mask = 0_u64;

        for (key, value) in d.iter() {
            map.insert(key.iter().collect::<String>(), value.to_owned());

            let bit = key.len().wrapping_sub(1);
            if bit < 64 {
                key_length_mask |= 1_u64 << bit;
                if let Some(starter) = key.first() {
                    *starter_len_mask.entry(starter.to_string()).or_insert(0) |= 1_u64 << bit;
                }
            }
        }

        Self {
            map,
            max_len: d.max_key_len(),
            min_len: d.min_key_len(),
            key_length_mask,
            starter_len_mask,
        }
    }
}

impl From<&DictionaryMaxlength> for DictionaryMaxlengthSerde {
    fn from(src: &DictionaryMaxlength) -> Self {
        Self {
            st_characters: (&src.st_characters).into(),
            st_phrases: (&src.st_phrases).into(),
            ts_characters: (&src.ts_characters).into(),
            ts_phrases: (&src.ts_phrases).into(),
            tw_phrases: (&src.tw_phrases).into(),
            tw_phrases_rev: (&src.tw_phrases_rev).into(),
            hk_phrases: (&src.hk_phrases).into(),
            hk_phrases_rev: (&src.hk_phrases_rev).into(),
            tw_variants_phrases: (&src.tw_variants_phrases).into(),
            tw_variants: (&src.tw_variants).into(),
            tw_variants_rev: (&src.tw_variants_rev).into(),
            tw_variants_rev_phrases: (&src.tw_variants_rev_phrases).into(),
            hk_variants_phrases: (&src.hk_variants_phrases).into(),
            hk_variants: (&src.hk_variants).into(),
            hk_variants_rev: (&src.hk_variants_rev).into(),
            hk_variants_rev_phrases: (&src.hk_variants_rev_phrases).into(),
            jps_characters: (&src.jps_characters).into(),
            jps_characters_rev: (&src.jps_characters_rev).into(),
            jps_phrases: (&src.jps_phrases).into(),
            st_punctuations: (&src.st_punctuations).into(),
            ts_punctuations: (&src.ts_punctuations).into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_import_rebuilds_derived_metadata_from_pairs() {
        let dto = DictMaxLenSerde {
            map: BTreeMap::from([("你好".to_owned(), "您好".to_owned())]),
            max_len: 99,
            min_len: 77,
            key_length_mask: u64::MAX,
            starter_len_mask: BTreeMap::from([("错".to_owned(), u64::MAX)]),
        };

        let dict = dto.into_internal();

        assert_eq!(dict.len(), 1);
        assert_eq!(dict.min_key_len(), 2);
        assert_eq!(dict.max_key_len(), 2);
        assert_eq!(dict.get(&['你', '好']), Some("您好"));
    }

    #[test]
    fn json_export_derives_metadata_through_public_api() {
        let dict = DictMaxLen::build_from_pairs([
            ("你".to_owned(), "您".to_owned()),
            ("你好".to_owned(), "您好".to_owned()),
        ]);

        let dto = DictMaxLenSerde::from(&dict);

        assert_eq!(dto.map.len(), 2);
        assert_eq!(dto.min_len, 1);
        assert_eq!(dto.max_len, 2);
        assert_eq!(dto.key_length_mask, 0b11);
        assert_eq!(dto.starter_len_mask.get("你"), Some(&0b11));
    }
}

#[cfg(test)]
mod slim_json_tests {
    use super::*;

    fn check(pairs: &[(&str, &str)], mask: u64, expected: Option<serde_json::Value>) {
        let dict =
            DictMaxLen::build_from_pairs(pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())));
        let dto = DictMaxLenSerde::from(&dict);
        assert_eq!(dto.key_length_mask, mask);
        for pretty in [false, true] {
            let text = if pretty {
                serde_json::to_string_pretty(&dto)
            } else {
                serde_json::to_string(&dto)
            }
            .unwrap();
            let physical: serde_json::Value = serde_json::from_str(&text).unwrap();
            assert_eq!(physical.get("starter_len_mask"), expected.as_ref());
            assert_eq!(physical["max_len"], dict.max_key_len());
            assert_eq!(physical["min_len"], dict.min_key_len());
            assert_eq!(physical["key_length_mask"], mask);
            assert_eq!(physical["map"], serde_json::to_value(&dto.map).unwrap());
            let decoded = serde_json::from_str::<DictMaxLenSerde>(&text)
                .unwrap()
                .into_internal();
            assert_eq!(decoded.len(), dict.len());
            for (key, value) in dict.iter() {
                assert_eq!(decoded.get(key), Some(value));
            }
            let again = serde_json::to_value(DictMaxLenSerde::from(&decoded)).unwrap();
            assert_eq!(physical, again);
        }
    }

    #[test]
    fn single_scalar_json_omits_masks_and_preserves_string_keys() {
        check(&[("汉", "漢"), ("发", "發"), ("𠀀", "𠀁")], 1, None);
    }

    #[test]
    fn phrase_and_mixed_json_keep_exact_masks() {
        check(
            &[("中国", "中國"), ("中国人", "中國人"), ("𠀀好", "好")],
            6,
            Some(serde_json::json!({"中": 6, "𠀀": 2})),
        );
        check(
            &[
                ("中", "中"),
                ("中国人", "中國人"),
                ("𠀀", "𠀁"),
                ("𠀀好", "好"),
            ],
            7,
            Some(serde_json::json!({"中": 5, "𠀀": 3})),
        );
        check(&[], 0, Some(serde_json::json!({})));
    }

    #[test]
    fn old_json_with_explicit_masks_remains_readable() {
        let fixture = r#"{"map":{"汉":"漢","𠀀":"𠀁"},"max_len":1,"min_len":1,"key_length_mask":1,"starter_len_mask":{"汉":1,"𠀀":1}}"#;
        let dto: DictMaxLenSerde = serde_json::from_str(fixture).unwrap();
        assert_eq!(
            dto.starter_len_mask,
            BTreeMap::from([("汉".into(), 1), ("𠀀".into(), 1)])
        );
        let decoded = dto.into_internal();
        assert_eq!(decoded.get(&['汉']), Some("漢"));
        assert_eq!(decoded.get(&['𠀀']), Some("𠀁"));
    }

    #[test]
    fn existing_character_and_phrase_json_artifacts() {
        let dicts = DictionaryMaxlength::new().unwrap();
        for dict in [
            &dicts.st_characters,
            &dicts.ts_characters,
            &dicts.st_phrases,
            &dicts.ts_phrases,
        ] {
            let dto = DictMaxLenSerde::from(dict);
            let physical = serde_json::to_value(&dto).unwrap();
            assert_eq!(
                physical.get("starter_len_mask").is_none(),
                dto.key_length_mask == 1
            );
            if std::ptr::eq(dict, &dicts.st_characters) {
                let slim_size = serde_json::to_vec(&physical).unwrap().len();
                let mut old = physical.clone();
                old["starter_len_mask"] = serde_json::to_value(&dto.starter_len_mask).unwrap();
                let old_size = serde_json::to_vec(&old).unwrap().len();
                assert!(slim_size < old_size);
                println!("STCharacters compact JSON: {old_size} -> {slim_size} bytes");
            }
            let decoded = serde_json::from_value::<DictMaxLenSerde>(physical)
                .unwrap()
                .into_internal();
            assert_eq!(dict.len(), decoded.len());
            for (key, value) in dict.iter() {
                assert_eq!(decoded.get(key), Some(value));
            }
        }
    }
}
