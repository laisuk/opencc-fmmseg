// json_io.rs (CLI only)
use opencc_fmmseg::dictionary_lib::{DictMaxLen, DictionaryMaxlength};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap; // stable key order for diffs

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DictMaxLenSerde {
    pub map: BTreeMap<String, String>,

    #[serde(default)]
    pub max_len: usize,

    // present for completeness; old JSON may omit it
    #[serde(default)]
    pub min_len: usize,

    // stable-ordered starter caps; keys are 1-char strings
    #[serde(default)]
    pub starter_cap: BTreeMap<String, u8>,

    // NEW: bitmask of existing key lengths (1..=64 mapped to bits 0..=63)
    #[serde(default)]
    pub key_length_mask: u64,
}

impl DictMaxLenSerde {
    #[allow(dead_code)]
    pub fn into_internal(self) -> DictMaxLen {
        let mut out = DictMaxLen::default();

        // Build map, and compute min/max + key_length_mask on the fly
        let mut min_seen = usize::MAX;
        let mut max_seen = 0usize;
        let mut mask: u64 = 0;

        for (k, v) in self.map {
            let key: Box<[char]> = k.chars().collect::<Vec<_>>().into_boxed_slice();
            let len = key.len();

            // update min/max
            if len < min_seen {
                min_seen = len;
            }
            if len > max_seen {
                max_seen = len;
            }

            // update mask (only 1..=64 represented)
            if (1..=64).contains(&len) {
                mask |= 1u64 << (len - 1);
            }

            out.map.insert(key, v.into_boxed_str());
        }

        // prefer values from JSON if present, fall back to recomputed
        out.max_len = if self.max_len != 0 {
            self.max_len
        } else {
            max_seen
        };
        out.min_len = if self.min_len != 0 {
            self.min_len
        } else if !out.map.is_empty() {
            min_seen
        } else {
            0
        };

        // starter_cap: use provided (string->u8), else recompute from map
        if self.starter_cap.is_empty() {
            let mut caps = rustc_hash::FxHashMap::default();
            caps.reserve(out.map.len().min(0x10000));
            for (k_chars, _) in out.map.iter() {
                if let Some(&c0) = k_chars.first() {
                    let len_u8 = u8::try_from(k_chars.len()).unwrap_or(u8::MAX);
                    use std::collections::hash_map::Entry;
                    match caps.entry(c0) {
                        Entry::Vacant(e) => {
                            e.insert(len_u8);
                        }
                        Entry::Occupied(mut e) => {
                            let cur = e.get_mut();
                            if len_u8 > *cur {
                                *cur = len_u8;
                            }
                        }
                    }
                }
            }
            out.starter_cap = caps;
        } else {
            let mut caps = rustc_hash::FxHashMap::default();
            caps.reserve(self.starter_cap.len());
            for (s, cap) in self.starter_cap {
                if let Some(ch) = s.chars().next() {
                    caps.insert(ch, cap);
                }
            }
            out.starter_cap = caps;
        }

        // key_length_mask: prefer provided nonzero mask, else use recomputed
        out.key_length_mask = if self.key_length_mask != 0 {
            self.key_length_mask
        } else {
            mask
        };

        // Rebuild runtime accelerators
        out.first_len_mask64.clear();
        out.first_char_max_len.clear();
        out.populate_starter_indexes();

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
        // map → BTreeMap<String,String> for deterministic output
        let mut map = BTreeMap::new();
        for (k, v) in &d.map {
            map.insert(k.iter().collect::<String>(), v.to_string());
        }

        // starter_cap → BTreeMap<String,u8> for deterministic output
        let mut starter_cap = BTreeMap::new();
        for (ch, cap) in &d.starter_cap {
            starter_cap.insert(ch.to_string(), *cap);
        }

        Self {
            map,
            max_len: d.max_len,
            min_len: d.min_len,
            starter_cap,
            key_length_mask: d.key_length_mask, // NEW
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
