//! Internal dictionary-processing utilities for `opencc-fmmseg`.
//!
//! This module provides the core components used to build and apply
//! dictionary-based conversions, including:
//!
//! - [`DictionaryMaxlength`] — Loader for multi-dictionary OpenCC-style
//!   structures, each with precomputed maximum phrase lengths.
//! - [`DictMaxLen`](DictMaxLen) — Lightweight dictionary wrapper used during
//!   longest-match segmentation.
//! - [`StarterUnion`](StarterUnion) — Fast starter-character lookup tables used to
//!   accelerate prefix matching within conversion rounds.
//!
//! These types work together to support multi-round, high-performance
//! segment replacement (e.g., S2T → TwPhrases → TwVariants).
//!
//! Although the module is publicly exposed for advanced users, most consumers
//! will interact only with the high-level [`OpenCC`](crate::OpenCC) API.
mod dict_max_len;
pub mod dictionary_maxlength;
mod starter_union;
mod dict_slot;

pub use self::dict_max_len::*;
pub use self::dictionary_maxlength::{DictionaryError, DictionaryMaxlength};
pub use self::starter_union::*;
pub use self::dict_slot::*;
