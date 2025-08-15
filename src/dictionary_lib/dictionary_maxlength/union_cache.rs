//! Internal: cached StarterUnion variants for known OpenCC configs.

use std::sync::{Arc, OnceLock};

use super::DictionaryMaxlength;
use crate::dictionary_lib::StarterUnion;

/// Cache slots for all union variants needed by the public conversion APIs.
/// Visible to the parent module only.
#[derive(Default, Debug)]
pub(super) struct Unions {
    // S2T / T2S (+ punct)
    s2t: OnceLock<Arc<StarterUnion>>,
    s2t_punct: OnceLock<Arc<StarterUnion>>,
    t2s: OnceLock<Arc<StarterUnion>>,
    t2s_punct: OnceLock<Arc<StarterUnion>>,

    // TW-only helpers
    tw_phrases_only: OnceLock<Arc<StarterUnion>>,
    tw_variants_only: OnceLock<Arc<StarterUnion>>,
    tw_phrases_rev_only: OnceLock<Arc<StarterUnion>>,
    tw_rev_pair: OnceLock<Arc<StarterUnion>>, // rev_phrases + rev
    tw2sp_r1_tw_rev_triple: OnceLock<Arc<StarterUnion>>, // phrases_rev + rev_phrases + rev

    // HK helpers
    hk_variants_only: OnceLock<Arc<StarterUnion>>,
    hk_rev_pair: OnceLock<Arc<StarterUnion>>, // rev_phrases + rev

    // JP helpers
    jp_variants_only: OnceLock<Arc<StarterUnion>>,
    jp_rev_triple: OnceLock<Arc<StarterUnion>>, // jps_phrases + jps_chars + jp_variants_rev
}

/// Logical keys for all cached unions.
/// Crate-visible so call sites can request a union.
pub(crate) enum UnionKey {
    // S2T / T2S
    S2T { punct: bool },
    T2S { punct: bool },

    // TW helpers
    TwPhrasesOnly,
    TwVariantsOnly,
    TwPhrasesRevOnly,
    TwRevPair,
    Tw2SpR1TwRevTriple,

    // HK helpers
    HkVariantsOnly,
    HkRevPair,

    // JP helpers
    JpVariantsOnly,
    JpRevTriple,
}

impl DictionaryMaxlength {
    /// Returns a cached `StarterUnion` for the given logical conversion set.
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
            UnionKey::TwVariantsOnly => self
                .unions
                .tw_variants_only
                .get_or_init(|| Arc::new(StarterUnion::build(&[&self.tw_variants])))
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
            UnionKey::HkVariantsOnly => self
                .unions
                .hk_variants_only
                .get_or_init(|| Arc::new(StarterUnion::build(&[&self.hk_variants])))
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
            UnionKey::JpVariantsOnly => self
                .unions
                .jp_variants_only
                .get_or_init(|| Arc::new(StarterUnion::build(&[&self.jp_variants])))
                .clone(),
            UnionKey::JpRevTriple => self
                .unions
                .jp_rev_triple
                .get_or_init(|| {
                    Arc::new(StarterUnion::build(&[
                        &self.jps_phrases,
                        &self.jps_characters,
                        &self.jp_variants_rev,
                    ]))
                })
                .clone(),
        }
    }

    /// Reset all cached unions (rebuilds lazily on next use).
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
