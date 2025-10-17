// json_io.rs (CLI only)
use opencc_fmmseg::dictionary_lib::{DictMaxLen, DictionaryMaxlength};
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
// stable key order for diffs

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
        let mut out = DictMaxLen::default();

        // Build map, and compute min/max + key_length_mask on the fly
        let mut min_seen = usize::MAX;
        let mut max_seen = 0usize;
        let mut mask: u64 = 0;

        for (k, v) in self.map {
            let key: Box<[char]> = k.chars().collect::<Vec<_>>().into_boxed_slice();
            let len = key.len();

            if len < min_seen {
                min_seen = len;
            }
            if len > max_seen {
                max_seen = len;
            }

            // 1..=64 only
            let b = len.wrapping_sub(1);
            if b < 64 {
                mask |= 1u64 << b;
            }

            out.map.insert(key, v.into_boxed_str());
        }

        // Prefer JSON-provided values; fallback to recomputed
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

        // key_length_mask: prefer provided nonzero mask, else recomputed
        out.key_length_mask = if self.key_length_mask != 0 {
            self.key_length_mask
        } else {
            mask
        };

        // NEW: starter_len_mask: use provided map if present; otherwise derive from out.map
        if self.starter_len_mask.is_empty() {
            let mut m = FxHashMap::default();
            // Heuristic: starters ≤ unique first chars in map, capped at BMP
            // (reserve is optional; remove if you prefer)
            let mut seen = FxHashSet::default();
            for (k_chars, _) in out.map.iter() {
                if let Some(&c0) = k_chars.first() {
                    if seen.insert(c0) { /* counting unique starters */ }
                    let len = k_chars.len();
                    let entry = m.entry(c0).or_insert(0u64);
                    let b = len.wrapping_sub(1);
                    if b < 64 {
                        *entry |= 1u64 << b;
                    }
                }
            }
            // If you still want a reserve, do it before the loop as:
            // m.reserve(seen.len());
            out.starter_len_mask = m;
        } else {
            let mut m = FxHashMap::default();
            // Reserve by provided size (cheap and safe)
            m.reserve(self.starter_len_mask.len());
            for (s, mask) in self.starter_len_mask {
                if let Some(ch) = s.chars().next() {
                    m.insert(ch, mask);
                }
            }
            out.starter_len_mask = m;
        }

        // Rebuild runtime accelerators (dense BMP vectors) from sparse maps
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
        // map → BTreeMap<String,String>
        let mut map = BTreeMap::new();
        for (k, v) in &d.map {
            map.insert(k.iter().collect::<String>(), v.to_string());
        }

        // NEW: starter_len_mask → BTreeMap<String,u64>
        let mut starter_len_mask = BTreeMap::new();
        if !d.starter_len_mask.is_empty() {
            for (ch, mask) in &d.starter_len_mask {
                starter_len_mask.insert(ch.to_string(), *mask);
            }
        } else if !d.first_len_mask64.is_empty() {
            // If sparse not kept but dense exists, serialize dense back to sparse BMP form
            for (i, &m) in d.first_len_mask64.iter().enumerate() {
                if m != 0 {
                    if let Some(ch) = char::from_u32(i as u32) {
                        starter_len_mask.insert(ch.to_string(), m);
                    }
                }
            }
        }

        Self {
            map,
            max_len: d.max_len,
            min_len: d.min_len,
            key_length_mask: d.key_length_mask,
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
