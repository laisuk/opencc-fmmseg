//! Internal: cached [`StarterUnion`] variants for known OpenCC configs.
//!
//! This module defines the cache structure used by [`DictionaryMaxlength`](DictionaryMaxlength)
//! to store and reuse precomputed [`StarterUnion`] instances. Each union corresponds to a
//! specific combination of dictionaries (e.g. S2T, T2S with punctuation, TW/HK/JP variants),
//! and is built lazily on first use. Subsequent lookups are cheap `Arc` clones.

use std::sync::{Arc, OnceLock};

use super::DictionaryMaxlength;
use crate::dictionary_lib::StarterUnion;

/// Cache slots for all [`StarterUnion`] variants used by the public conversion APIs.
///
/// Each field is a [`OnceLock`] holding an [`Arc<StarterUnion>`]. The first
/// request for a given union builds the underlying [`StarterUnion`] from the
/// relevant dictionaries; later requests simply clone the cached `Arc`.
///
/// This struct is intended to be embedded inside [`DictionaryMaxlength`] and
/// is not exposed outside the crate.
#[derive(Default, Debug)]
pub(super) struct Unions {
    // S2T / T2S (+ punct)
    /// Simplified → Traditional core union (phrases + characters).
    s2t: OnceLock<Arc<StarterUnion>>,

    /// Simplified → Traditional union including punctuation mappings.
    s2t_punct: OnceLock<Arc<StarterUnion>>,

    /// Traditional → Simplified core union (phrases + characters).
    t2s: OnceLock<Arc<StarterUnion>>,

    /// Traditional → Simplified union including punctuation mappings.
    t2s_punct: OnceLock<Arc<StarterUnion>>,

    // TW-only helpers
    /// Union built from Taiwanese phrase dictionaries only.
    tw_phrases_only: OnceLock<Arc<StarterUnion>>,

    /// Union built from Taiwanese regional variant phrase and character dictionaries.
    tw_variants_pair: OnceLock<Arc<StarterUnion>>,

    /// Union used in the second round of `s2twp`, combining:
    /// phrases + variant phrases + variants.
    s2twp_r2_tw_triple: OnceLock<Arc<StarterUnion>>,

    /// Union used in the second round of `s2hkp`, combining:
    /// phrases + variant phrases + variants.
    s2hkp_r2_hk_triple: OnceLock<Arc<StarterUnion>>,

    /// Union built from reverse Taiwanese phrase dictionaries only.
    tw_phrases_rev_only: OnceLock<Arc<StarterUnion>>,

    /// Union combining reverse Taiwanese variant phrases and characters
    /// (rev_phrases + rev).
    tw_rev_pair: OnceLock<Arc<StarterUnion>>,

    /// Union used in the first round of `tw2sp`, combining:
    /// phrases_rev + rev_phrases + rev.
    tw2sp_r1_tw_rev_triple: OnceLock<Arc<StarterUnion>>,

    /// Union used in the first round of `hk2sp`, combining:
    /// phrases_rev + rev_phrases + rev.
    hk2sp_r1_hk_rev_triple: OnceLock<Arc<StarterUnion>>,

    // HK helpers
    /// Union built from Hong Kong regional variant phrase and character dictionaries.
    hk_variants_pair: OnceLock<Arc<StarterUnion>>,

    /// Union combining reverse Hong Kong variant phrases and characters
    /// (rev_phrases + rev).
    hk_rev_pair: OnceLock<Arc<StarterUnion>>,

    // JP helpers
    /// Union built from Japanese Shinjitai reverse character dictionaries only.
    jps_characters_rev: OnceLock<Arc<StarterUnion>>,

    /// Union combining Japanese Shinjitai phrases and characters.
    jps_pair: OnceLock<Arc<StarterUnion>>,
}

/// Logical keys identifying every cached [`StarterUnion`] variant used by the
/// OpenCC conversion engine.
///
/// Each variant corresponds to a specific combination of dictionaries required
/// by a conversion mode (e.g., S2T, T2S with punctuation, Taiwanese variants,
/// Hong Kong variants, Japanese Shinjitai, etc.).
///
/// These keys are used internally by
/// [`DictionaryMaxlength::union_for`](DictionaryMaxlength::union_for)
/// to select the appropriate cached [`StarterUnion`].
pub(crate) enum UnionKey {
    // ============================
    // Simplified → Traditional
    // ============================
    /// Simplified → Traditional union.
    ///
    /// Includes:
    /// - `st_phrases`
    /// - `st_characters`
    /// - optionally `st_punctuations` if `punct = true`
    ///
    /// Used by `OpenCC::s2t`, `OpenCC::s2tw`, `OpenCC::s2twp`, `OpenCC::s2hk`,
    /// `OpenCC::s2hkp`.
    S2T {
        /// Whether punctuation dictionaries should be included.
        punct: bool,
    },

    // ============================
    // Traditional → Simplified
    // ============================
    /// Traditional → Simplified union.
    ///
    /// Includes:
    /// - `ts_phrases`
    /// - `ts_characters`
    /// - optionally `ts_punctuations` if `punct = true`
    ///
    /// Used by `OpenCC::t2s`, `OpenCC::tw2s`, `OpenCC::tw2sp`, `OpenCC::hk2s`,
    /// `OpenCC::hk2sp`.
    T2S {
        /// Whether punctuation dictionaries should be included.
        punct: bool,
    },

    // ============================
    // Taiwanese Helpers
    // ============================
    /// Union containing only Taiwanese phrase dictionaries.
    ///
    /// Includes:
    /// - `tw_phrases`
    ///
    /// Used in:
    /// - `t2twp` (round 1)
    TwPhrasesOnly,

    /// Union containing Taiwanese regional variant phrase and character dictionaries.
    ///
    /// Includes:
    /// - `tw_variants_phrases`
    /// - `tw_variants`
    ///
    /// Used in:
    /// - `s2tw` (round 2)
    /// - `t2tw`
    /// - `t2twp` (round 2)
    TwVariantsPair,

    /// Triple Taiwanese union for `s2twp` round 2:
    ///
    /// Includes:
    /// - `tw_phrases`
    /// - `tw_variants_phrases`
    /// - `tw_variants`
    ///
    /// Used exclusively in:
    /// - `s2twp` (round 2)
    S2TwpR2TwTriple,

    /// Triple Hong Kong union for `s2hkp` round 2:
    ///
    /// Includes:
    /// - `hk_phrases`
    /// - `hk_variants_phrases`
    /// - `hk_variants`
    ///
    /// Used exclusively in:
    /// - `s2hkp` (round 2)
    S2HkpR2HkTriple,

    /// Union containing only reverse Taiwanese phrase dictionaries.
    ///
    /// Includes:
    /// - `tw_phrases_rev`
    ///
    /// Used in:
    /// - `tw2tp` (round 2)
    TwPhrasesRevOnly,

    /// Combined reverse Taiwanese variant union.
    ///
    /// Includes:
    /// - `tw_variants_rev_phrases`
    /// - `tw_variants_rev`
    ///
    /// Used in:
    /// - `tw2s` (round 1)
    /// - `tw2t` (round 1)
    /// - `tw2tp` (round 1)
    TwRevPair,

    /// Triple-reverse Taiwanese union for `tw2sp` round 1:
    ///
    /// Includes:
    /// - `tw_phrases_rev`
    /// - `tw_variants_rev_phrases`
    /// - `tw_variants_rev`
    ///
    /// Used exclusively in:
    /// - `tw2sp` (round 1)
    Tw2SpR1TwRevTriple,

    /// Triple-reverse Hong Kong union for `hk2sp` round 1:
    ///
    /// Includes:
    /// - `hk_phrases_rev`
    /// - `hk_variants_rev_phrases`
    /// - `hk_variants_rev`
    ///
    /// Used exclusively in:
    /// - `hk2sp` (round 1)
    Hk2SpR1HkRevTriple,

    // ============================
    // Hong Kong Helpers
    // ============================
    /// Union containing Hong Kong regional variant phrase and character dictionaries.
    ///
    /// Includes:
    /// - `hk_variants_phrases`
    /// - `hk_variants`
    ///
    /// Used in:
    /// - `s2hk` (round 2)
    /// - `t2hk`
    HkVariantsPair,

    /// Combined reverse Hong Kong variant union.
    ///
    /// Includes:
    /// - `hk_variants_rev_phrases`
    /// - `hk_variants_rev`
    ///
    /// Used in:
    /// - `hk2s` (round 1)
    /// - `hk2t` (round 1)
    HkRevPair,

    // ============================
    // Japanese Helpers
    // ============================
    /// Union containing only Japanese Shinjitai reverse character dictionaries.
    ///
    /// Includes:
    /// - `jps_characters_rev`
    ///
    /// Used in:
    /// - `t2jp`
    JpsCharactersRev,

    /// Pair-set Japanese Shinjitai union.
    ///
    /// Includes:
    /// - `jps_phrases`
    /// - `jps_characters`
    ///
    /// Used in:
    /// - `jp2t`
    JpsPair,
}

impl DictionaryMaxlength {
    /// Returns a cached [`StarterUnion`] for a given logical conversion set.
    ///
    /// Each [`UnionKey`] corresponds to a specific combination of dictionaries
    /// used in the conversion pipeline (e.g. S2T with/without punctuation,
    /// Taiwanese phrases/variants, Hong Kong variants, Japanese variants, etc.).
    ///
    /// For each key, this method:
    ///
    /// - Lazily builds a [`StarterUnion`] from the appropriate [`DictMaxLen`]
    ///   dictionaries on first use via `StarterUnion::build`.
    /// - Stores it in the corresponding cache slot in `self.unions`.
    /// - Returns a cloned [`Arc<StarterUnion>`], allowing cheap reuse across
    ///   threads and conversion calls.
    ///
    /// Subsequent calls with the same [`UnionKey`] are lock-free and avoid
    /// recomputing starter metadata, which significantly reduces overhead for
    /// repeated conversions using the same configuration.
    ///
    /// # Arguments
    ///
    /// * `key` – Logical identifier describing which dictionary set to use
    ///   (e.g. [`UnionKey::S2T`], [`UnionKey::T2S`], [`UnionKey::TwRevPair`]).
    ///
    /// # Returns
    ///
    /// A shared, cached [`StarterUnion`] for the requested dictionary set.
    #[inline]
    pub(crate) fn union_for(&self, key: UnionKey) -> Arc<StarterUnion> {
        match key {
            // S2T / T2S
            UnionKey::S2T { punct } => {
                let slot = if punct {
                    &self.unions.s2t_punct
                } else {
                    &self.unions.s2t
                };
                slot.get_or_init(|| {
                    if punct {
                        let dicts = [&self.st_phrases, &self.st_characters, &self.st_punctuations];
                        Arc::new(StarterUnion::build(&dicts))
                    } else {
                        let dicts = [&self.st_phrases, &self.st_characters];
                        Arc::new(StarterUnion::build(&dicts))
                    }
                })
                .clone()
            }
            UnionKey::T2S { punct } => {
                let slot = if punct {
                    &self.unions.t2s_punct
                } else {
                    &self.unions.t2s
                };
                slot.get_or_init(|| {
                    if punct {
                        let dicts = [&self.ts_phrases, &self.ts_characters, &self.ts_punctuations];
                        Arc::new(StarterUnion::build(&dicts))
                    } else {
                        let dicts = [&self.ts_phrases, &self.ts_characters];
                        Arc::new(StarterUnion::build(&dicts))
                    }
                })
                .clone()
            }
            UnionKey::TwPhrasesOnly => self
                .unions
                .tw_phrases_only
                .get_or_init(|| Arc::new(StarterUnion::build(&[&self.tw_phrases])))
                .clone(),
            UnionKey::TwVariantsPair => self
                .unions
                .tw_variants_pair
                .get_or_init(|| {
                    Arc::new(StarterUnion::build(&[
                        &self.tw_variants_phrases,
                        &self.tw_variants,
                    ]))
                })
                .clone(),
            UnionKey::S2TwpR2TwTriple => self
                .unions
                .s2twp_r2_tw_triple
                .get_or_init(|| {
                    Arc::new(StarterUnion::build(&[
                        &self.tw_phrases,
                        &self.tw_variants_phrases,
                        &self.tw_variants,
                    ]))
                })
                .clone(),
            UnionKey::S2HkpR2HkTriple => self
                .unions
                .s2hkp_r2_hk_triple
                .get_or_init(|| {
                    Arc::new(StarterUnion::build(&[
                        &self.hk_phrases,
                        &self.hk_variants_phrases,
                        &self.hk_variants,
                    ]))
                })
                .clone(),
            UnionKey::TwPhrasesRevOnly => self
                .unions
                .tw_phrases_rev_only
                .get_or_init(|| Arc::new(StarterUnion::build(&[&self.tw_phrases_rev])))
                .clone(),
            UnionKey::TwRevPair => self
                .unions
                .tw_rev_pair
                .get_or_init(|| {
                    Arc::new(StarterUnion::build(&[
                        &self.tw_variants_rev_phrases,
                        &self.tw_variants_rev,
                    ]))
                })
                .clone(),
            UnionKey::Tw2SpR1TwRevTriple => self
                .unions
                .tw2sp_r1_tw_rev_triple
                .get_or_init(|| {
                    Arc::new(StarterUnion::build(&[
                        &self.tw_phrases_rev,
                        &self.tw_variants_rev_phrases,
                        &self.tw_variants_rev,
                    ]))
                })
                .clone(),
            UnionKey::Hk2SpR1HkRevTriple => self
                .unions
                .hk2sp_r1_hk_rev_triple
                .get_or_init(|| {
                    Arc::new(StarterUnion::build(&[
                        &self.hk_phrases_rev,
                        &self.hk_variants_rev_phrases,
                        &self.hk_variants_rev,
                    ]))
                })
                .clone(),
            UnionKey::HkVariantsPair => self
                .unions
                .hk_variants_pair
                .get_or_init(|| {
                    Arc::new(StarterUnion::build(&[
                        &self.hk_variants_phrases,
                        &self.hk_variants,
                    ]))
                })
                .clone(),
            UnionKey::HkRevPair => self
                .unions
                .hk_rev_pair
                .get_or_init(|| {
                    Arc::new(StarterUnion::build(&[
                        &self.hk_variants_rev_phrases,
                        &self.hk_variants_rev,
                    ]))
                })
                .clone(),
            UnionKey::JpsCharactersRev => self
                .unions
                .jps_characters_rev
                .get_or_init(|| Arc::new(StarterUnion::build(&[&self.jps_characters_rev])))
                .clone(),
            UnionKey::JpsPair => self
                .unions
                .jps_pair
                .get_or_init(|| {
                    Arc::new(StarterUnion::build(&[
                        &self.jps_phrases,
                        &self.jps_characters,
                    ]))
                })
                .clone(),
        }
    }

    /// Clears all cached [`StarterUnion`] instances.
    ///
    /// This resets the internal [`Unions`] cache back to its default (empty)
    /// state. All previously built starter tables are dropped, and future calls
    /// to [`union_for`](Self::union_for) will lazily rebuild the required
    /// `StarterUnion` instances on demand.
    ///
    /// This is primarily intended for testing or for rare cases where the
    /// dictionary contents have been reloaded and the cached starter metadata
    /// must be regenerated.
    ///
    /// # Notes
    ///
    /// - This does **not** modify any dictionaries themselves.  
    /// - Clearing is inexpensive; rebuilding unions will incur cost only on the
    ///   next lookup.  
    /// - Marked `dead_code` because normal application code never needs to call
    ///   it directly.
    ///
    /// # Examples
    ///
    /// Not provided, as this API is internal.
    #[inline]
    #[allow(dead_code)]
    pub(crate) fn clear_unions(&mut self) {
        self.unions = Unions::default();
    }
}

#[test]
fn union_cached() {
    let d = DictionaryMaxlength::default();
    let a = d.union_for(UnionKey::S2T { punct: false });
    let b = d.union_for(UnionKey::S2T { punct: false });
    assert!(std::ptr::eq(Arc::as_ptr(&a), Arc::as_ptr(&b)));
}

#[test]
fn union_init_once_parallel() {
    use rayon::prelude::*;
    let d = DictionaryMaxlength::default();
    (0..32).into_par_iter().for_each(|_| {
        let _ = d.union_for(UnionKey::S2T { punct: false });
    });
    // same pointer on repeated calls
    let a = d.union_for(UnionKey::S2T { punct: false });
    let b = d.union_for(UnionKey::S2T { punct: false });
    assert!(std::ptr::eq(Arc::as_ptr(&a), Arc::as_ptr(&b)));
}

#[test]
fn union_clear_invalidate() {
    let mut d = DictionaryMaxlength::default();
    let a = d.union_for(UnionKey::S2T { punct: false });
    d.clear_unions(); // resets OnceLocks
    let c = d.union_for(UnionKey::S2T { punct: false });
    assert!(!std::ptr::eq(Arc::as_ptr(&a), Arc::as_ptr(&c)));
}

#[test]
fn union_keys_distinct() {
    let d = DictionaryMaxlength::default();
    let a = d.union_for(UnionKey::S2T { punct: false });
    let b = d.union_for(UnionKey::S2T { punct: true });
    assert!(!std::ptr::eq(Arc::as_ptr(&a), Arc::as_ptr(&b)));
}
