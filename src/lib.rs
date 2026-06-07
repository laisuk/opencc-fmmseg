#![cfg_attr(docsrs, feature(doc_cfg))]

//! Fast Chinese text conversion with OpenCC dictionaries and forward maximum
//! matching segmentation.
//!
//! `opencc-fmmseg` converts between Simplified Chinese, Traditional Chinese,
//! Taiwan, Hong Kong, and Japanese kanji variants using bundled OpenCC-style
//! dictionaries. The default constructor loads a compressed dictionary embedded
//! in the crate, so normal use does not require runtime dictionary files.
//!
//! # Quick Start
//!
//! ```rust
//! use opencc_fmmseg::{OpenCC, OpenccConfig};
//!
//! let converter = OpenCC::new();
//!
//! let traditional = converter.convert_with_config(
//!     "汉字转换测试",
//!     OpenccConfig::S2t,
//!     false,
//! );
//!
//! assert_eq!(traditional, "漢字轉換測試");
//! ```
//!
//! # Choosing an API
//!
//! - [`OpenCC`] is the main converter type.
//! - [`OpenccConfig`] is the recommended Rust configuration API.
//! - [`OpenCC::convert`] accepts OpenCC-style strings such as `"s2t"` and is
//!   useful for CLI/config-file compatibility.
//! - [`OpenCC::detofu`] and [`detofu`] provide optional display compatibility
//!   fallback for rare non-BMP CJK extension characters after conversion.
//! - [`DetofuMap`] is the advanced reusable/customizable detofu map API.
//! - [`DictionaryMaxlength`] and [`CustomDictSpec`] are for advanced users who
//!   need custom dictionaries or externally generated dictionary artifacts.
//!
//! # Supported Configurations
//!
//! | Config | Method | Meaning |
//! | --- | --- | --- |
//! | `s2t` | [`OpenCC::s2t`] | Simplified to Traditional |
//! | `t2s` | [`OpenCC::t2s`] | Traditional to Simplified |
//! | `s2tw` / `s2twp` | [`OpenCC::s2tw`] / [`OpenCC::s2twp`] | Simplified to Taiwan Traditional |
//! | `tw2s` / `tw2sp` | [`OpenCC::tw2s`] / [`OpenCC::tw2sp`] | Taiwan Traditional to Simplified |
//! | `s2hk` / `t2hk` | [`OpenCC::s2hk`] / [`OpenCC::t2hk`] | To Hong Kong Traditional variants |
//! | `hk2s` / `hk2t` | [`OpenCC::hk2s`] / [`OpenCC::hk2t`] | Hong Kong variants to Simplified/Traditional |
//! | `t2tw` / `t2twp` | [`OpenCC::t2tw`] / [`OpenCC::t2twp`] | Traditional to Taiwan variants |
//! | `tw2t` / `tw2tp` | [`OpenCC::tw2t`] / [`OpenCC::tw2tp`] | Taiwan variants to Traditional |
//! | `t2jp` / `jp2t` | [`OpenCC::t2jp`] / [`OpenCC::jp2t`] | Traditional and Japanese kanji variants |
//!
//! # Custom Dictionaries
//!
//! ```rust
//! use opencc_fmmseg::{
//!     CustomDictMode, CustomDictSpec, DictSlot, DictionaryMaxlength, OpenCC,
//! };
//!
//! let dictionary = DictionaryMaxlength::from_zstd()?
//!     .with_custom_dicts(&[CustomDictSpec {
//!         slot: DictSlot::STPhrases,
//!         pairs: vec![("帕兰蒂尔".to_string(), "柏蘭蒂爾".to_string())],
//!         mode: CustomDictMode::Append,
//!     }])?;
//!
//! let converter = OpenCC::from_dictionary(dictionary);
//! assert_eq!(
//!     converter.convert("帕兰蒂尔", "s2t", false),
//!     "柏蘭蒂爾"
//! );
//!
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Error Reporting
//!
//! Most high-level conversion methods return a [`String`] for compatibility with
//! the C and scripting-language bindings. Non-fatal setup or configuration
//! errors are recorded in [`OpenCC::get_last_error`]. Dictionary construction
//! APIs return [`Result`] with [`DictionaryError`].
//!
/// Delimiters helper for splitting and matching delimiters.
mod delimiter_set;
/// Display compatibility fallback utilities for rare CJK extension characters.
pub mod detofu;
/// Bridge helper for conversion plan and core converter functions.
mod dict_refs;
/// Dictionary utilities for managing multiple OpenCC lexicons.
pub mod dictionary_lib;
/// Core converter
mod opencc;
/// Configurations for conversion.
mod opencc_config;
/// Common helpers for opencc-fmmseg.
mod utils;

pub use crate::delimiter_set::{is_delimiter, DelimiterSet};
pub use crate::dict_refs::DictRefs;
pub use crate::dictionary_lib::{CustomDictFileSpec, CustomDictMode, CustomDictSpec, DictSlot};
pub use crate::dictionary_lib::{DictionaryError, DictionaryMaxlength};
pub use crate::opencc::OpenCC;
pub use crate::opencc_config::OpenccConfig;
/// Converts rare non-BMP CJK extension characters to compatibility fallbacks.
pub use detofu::detofu;
/// Threshold level used by detofu display-compatibility fallback.
pub use detofu::DetofuLevel;
/// Reusable and customizable detofu fallback map.
pub use detofu::DetofuMap;
pub use utils::*;
