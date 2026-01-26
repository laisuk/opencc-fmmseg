//! Internal dictionary-processing utilities for `opencc-fmmseg`.
//!
//! This module provides the core components used to build and apply
//! dictionary-based conversions, including:
//!
//! - [`DictionaryMaxlength`] — Loader for multi-dictionary OpenCC-style
//!   structures, each with precomputed maximum phrase lengths.
//! - [`DictMaxLen`] — Lightweight dictionary wrapper used during
//!   longest-match segmentation.
//! - [`StarterUnion`] — Fast starter-character lookup tables used to
//!   accelerate prefix matching within conversion rounds.
//!
//! These types work together to support multi-round, high-performance
//! segment replacement (e.g., S2T → TwPhrases → TwVariants).
//!
//! Although the module is publicly exposed for advanced users, most consumers
//! will interact only with the high-level [`OpenCC`](crate::OpenCC) API.
pub mod dictionary_maxlength;
mod dict_max_len;
mod starter_union;

pub use self::dictionary_maxlength::{DictionaryMaxlength, DictionaryError};
pub use self::dict_max_len::*;
pub use self::starter_union::*;