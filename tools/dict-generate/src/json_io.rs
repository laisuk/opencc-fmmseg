// json_io.rs (CLI only)
use serde::{Serialize, Deserialize};
use std::collections::BTreeMap; // stable key order for diffs
use opencc_fmmseg::dictionary_lib::{DictionaryMaxlength, DictMaxLen};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DictMaxLenSerde {
    pub map: BTreeMap<String, String>,
    #[serde(default)]
    pub max_len: usize,
    #[serde(default)]
    pub starter_cap: BTreeMap<String, u8>,
    // reserve for future:
    // #[serde(default)]
    // pub min_len: usize,
}

impl DictMaxLenSerde {
    #[allow(dead_code)]
    pub fn into_internal(self) -> DictMaxLen {
        let mut out = DictMaxLen::default();

        for (k, v) in self.map {
            let key: Box<[char]> = k.chars().collect::<Vec<_>>().into_boxed_slice();
            out.max_len = out.max_len.max(key.len());
            out.map.insert(key, v.into_boxed_str());
        }

        let mut starter_cap = rustc_hash::FxHashMap::default();
        for (s, cap) in self.starter_cap {
            if let Some(ch) = s.chars().next() {
                starter_cap.insert(ch, cap);
            }
        }
        out.starter_cap = starter_cap;

        out
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
    pub tw_variants: DictMaxLenSerde,
    pub tw_variants_rev: DictMaxLenSerde,
    pub tw_variants_rev_phrases: DictMaxLenSerde,
    pub hk_variants: DictMaxLenSerde,
    pub hk_variants_rev: DictMaxLenSerde,
    pub hk_variants_rev_phrases: DictMaxLenSerde,
    pub jps_characters: DictMaxLenSerde,
    pub jps_phrases: DictMaxLenSerde,
    pub jp_variants: DictMaxLenSerde,
    pub jp_variants_rev: DictMaxLenSerde,
    pub st_punctuations: DictMaxLenSerde,
    pub ts_punctuations: DictMaxLenSerde,
}

impl From<&DictMaxLen> for DictMaxLenSerde {
    fn from(d: &DictMaxLen) -> Self {
        let mut map = std::collections::BTreeMap::new();
        for (k, v) in &d.map {
            map.insert(k.iter().collect::<String>(), v.to_string());
        }

        let mut starter_cap = std::collections::BTreeMap::new();
        for (ch, cap) in &d.starter_cap {
            starter_cap.insert(ch.to_string(), *cap);
        }

        Self {
            map,
            max_len: d.max_len,
            starter_cap,
            // min_len: d.min_len, // once you add it
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
            tw_variants: (&src.tw_variants).into(),
            tw_variants_rev: (&src.tw_variants_rev).into(),
            tw_variants_rev_phrases: (&src.tw_variants_rev_phrases).into(),
            hk_variants: (&src.hk_variants).into(),
            hk_variants_rev: (&src.hk_variants_rev).into(),
            hk_variants_rev_phrases: (&src.hk_variants_rev_phrases).into(),
            jps_characters: (&src.jps_characters).into(),
            jps_phrases: (&src.jps_phrases).into(),
            jp_variants: (&src.jp_variants).into(),
            jp_variants_rev: (&src.jp_variants_rev).into(),
            st_punctuations: (&src.st_punctuations).into(),
            ts_punctuations: (&src.ts_punctuations).into(),
        }
    }
}

