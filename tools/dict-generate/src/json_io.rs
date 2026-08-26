// json_io.rs (CLI only)
use opencc_fmmseg::{DictMaxLen, DictionaryMaxlength};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// BTreeMap keeps JSON object keys deterministic for stable diffs.
#[derive(Debug, Default, Serialize, Deserialize)]
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
