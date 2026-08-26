//! Internal dictionary-processing utilities for `opencc-fmmseg`.
//!
//! This module provides the core components used to build and apply
//! dictionary-based conversions, including:
//!
//! - [`DictionaryMaxlength`] — Loader for multi-dictionary OpenCC-style
//!   structures, each with precomputed maximum phrase lengths.
//! - [`DictMaxLen`](crate::DictMaxLen) — Lightweight dictionary wrapper used during
//!   longest-match segmentation.
//! - `StarterUnion` — Fast starter-character lookup tables used to
//!   accelerate prefix matching within conversion rounds.
//!
//! These types work together to support multi-round, high-performance
//! segment replacement (e.g., S2T → TwPhrases → TwVariants).
//!
//! The implementation module is private. Its supported public types are
//! re-exported from the crate root; most consumers only need the high-level
//! [`OpenCC`](crate::OpenCC) API.
mod dict_max_len;
mod dict_slot;
mod dictionary_maxlength;
mod starter_union;

pub use self::dict_max_len::*;
pub use self::dict_slot::*;
pub(crate) use self::dictionary_maxlength::UnionKey;
pub use self::dictionary_maxlength::{DictionaryError, DictionaryMaxlength};
pub(crate) use self::starter_union::StarterUnion;
