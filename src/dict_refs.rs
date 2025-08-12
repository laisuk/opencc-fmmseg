use crate::dictionary_lib::{DictMaxLen, StarterUnion};
use std::sync::Arc;

/// One conversion round: a set of dictionaries + its computed `max_len` + the
/// prebuilt [`StarterUnion`] used to prune viable match lengths.
///
/// # Fields
/// - [`dicts`]: the dictionaries to probe (probe order = precedence).
/// - [`max_len`]: the maximum phrase length across `dicts` (in `char`s).
/// - [`union`]: a union of starter masks/caps built **from these `dicts`**.
///   Typically, cached (e.g., via `OnceLock`) and shared across threads with `Arc`.
///
/// # Invariants
/// - `max_len` is `dicts.iter().map(|d| d.max_len).max().unwrap_or(1)`.
/// - `union` must reflect exactly the dictionaries in `dicts`.
///   If the dictionaries change, rebuild the union.
pub struct DictRound<'a> {
    pub dicts: &'a [&'a DictMaxLen],
    pub max_len: usize,
    pub union: Arc<StarterUnion>,
}

/// Internal helper that computes a [`DictRound`] from a slice of dictionaries
/// and its corresponding [`StarterUnion`].
///
/// `max_len` is computed as the maximum `d.max_len` among `dicts` (or `1` if empty).
#[inline]
fn compute_round<'a>(dicts: &'a [&'a DictMaxLen], union: Arc<StarterUnion>) -> DictRound<'a> {
    let max_len = dicts.iter().map(|d| d.max_len).max().unwrap_or(1);
    DictRound {
        dicts,
        max_len,
        union,
    }
}

/// Holds up to three conversion rounds. Each round carries its own
/// dictionaries, `max_len`, and prebuilt [`StarterUnion`].
///
/// This struct is a small orchestrator: you assemble rounds (R1 is required,
/// R2/R3 are optional), then call [`apply_segment_replace`] with your engine’s
/// segment/replace closure (e.g., a wrapper around `convert_by_union`).
///
/// # Example
/// Minimal example that builds two tiny dictionaries, a shared union,
/// and runs a no-op conversion closure (for illustration only).
///
/// ```
/// use std::sync::Arc;
/// use opencc_fmmseg::dictionary_lib::{DictMaxLen, StarterUnion};
/// use opencc_fmmseg::DictRefs; // adjust path if needed
///
/// // Tiny dicts (one-char mappings)
/// let d1 = DictMaxLen::build_from_pairs(vec![("你".into(), "您".into())]);
/// let d2 = DictMaxLen::build_from_pairs(vec![("世".into(), "世".into())]);
/// let dicts: Vec<&DictMaxLen> = vec![&d1, &d2];
///
/// // Union built from exactly these dicts
/// let union = Arc::new(StarterUnion::build(&dicts));
///
/// // One round; closure here just echoes input
/// let refs = DictRefs::new(&dicts, union);
/// let out = refs.apply_segment_replace("你好，世界", |input, _dicts, _max, _union| {
///     input.to_string()
/// });
/// assert_eq!(out, "你好，世界");
/// ```
///
/// For a full conversion, your closure would call your engine’s
/// `segment_replace_with_union(input, dicts, max_len, union)`.
pub struct DictRefs<'a> {
    round_1: DictRound<'a>,
    round_2: Option<DictRound<'a>>,
    round_3: Option<DictRound<'a>>,
}

impl<'a> DictRefs<'a> {
    /// Creates a [`DictRefs`] with **required** round 1.
    ///
    /// `max_len` is computed automatically; `union` should be prebuilt from
    /// exactly `round_1_dicts` (and is typically cached and reused).
    ///
    /// # Example
    /// ```
    /// # use std::sync::Arc;
    /// # use opencc_fmmseg::dictionary_lib::{DictMaxLen, StarterUnion};
    /// # use opencc_fmmseg::DictRefs;
    /// let d = DictMaxLen::build_from_pairs(vec![("你".into(), "您".into())]);
    /// let dicts: Vec<&DictMaxLen> = vec![&d];
    /// let union = Arc::new(StarterUnion::build(&dicts));
    /// let _refs = DictRefs::new(&dicts, union);
    /// ```
    pub fn new(round_1_dicts: &'a [&'a DictMaxLen], round_1_union: Arc<StarterUnion>) -> Self {
        Self {
            round_1: compute_round(round_1_dicts, round_1_union),
            round_2: None,
            round_3: None,
        }
    }

    /// Adds **optional** round 2.
    ///
    /// `round_2_union` should be built from `round_2_dicts`.
    pub fn with_round_2(
        mut self,
        round_2_dicts: &'a [&'a DictMaxLen],
        round_2_union: Arc<StarterUnion>,
    ) -> Self {
        self.round_2 = Some(compute_round(round_2_dicts, round_2_union));
        self
    }

    /// Adds **optional** round 3.
    ///
    /// `round_3_union` should be built from `round_3_dicts`.
    pub fn with_round_3(
        mut self,
        round_3_dicts: &'a [&'a DictMaxLen],
        round_3_union: Arc<StarterUnion>,
    ) -> Self {
        self.round_3 = Some(compute_round(round_3_dicts, round_3_union));
        self
    }

    /// Applies up to three rounds using a caller-provided segment/replace closure.
    ///
    /// The closure receives:
    /// - `&str` — the input for that round (segment or whole string),
    /// - `&[&DictMaxLen]` — the dictionaries to consult for that round,
    /// - `usize` — `max_len` (in characters) for that round,
    /// - `&StarterUnion` — the union to prune viable lengths for that round.
    ///
    /// It must return the transformed `String` for that round.
    ///
    /// # Example
    /// ```
    /// # use std::sync::Arc;
    /// # use opencc_fmmseg::dictionary_lib::{DictMaxLen, StarterUnion};
    /// # use opencc_fmmseg::DictRefs;
    /// let d = DictMaxLen::build_from_pairs(vec![("你".into(), "您".into())]);
    /// let dicts: Vec<&DictMaxLen> = vec![&d];
    /// let union = Arc::new(StarterUnion::build(&dicts));
    ///
    /// let refs = DictRefs::new(&dicts, union);
    /// let converted = refs.apply_segment_replace("你", |input, _dicts, _max_len, _union| {
    ///     // In production, call your engine here:
    ///     // opencc.segment_replace_with_union(input, dicts, max_len, union)
    ///     input.to_string()
    /// });
    /// assert_eq!(converted, "你");
    /// ```
    pub fn apply_segment_replace<F>(&self, input: &str, segment_replace: F) -> String
    where
        F: Fn(&str, &[&DictMaxLen], usize, &StarterUnion) -> String,
    {
        let mut out = segment_replace(
            input,
            self.round_1.dicts,
            self.round_1.max_len,
            &self.round_1.union,
        );

        if let Some(r2) = &self.round_2 {
            out = segment_replace(&out, r2.dicts, r2.max_len, &r2.union);
        }
        if let Some(r3) = &self.round_3 {
            out = segment_replace(&out, r3.dicts, r3.max_len, &r3.union);
        }
        out
    }
}
