#![cfg_attr(docsrs, feature(doc_cfg))]

//! Fast Chinese text conversion with OpenCC dictionaries and forward maximum
//! matching segmentation.
//!
//! Convert between Simplified and Traditional Chinese, including Taiwan and
//! Hong Kong variants, or convert between Traditional Chinese and Japanese
//! kanji variants. Small Seal Script conversion is also available.
//! [`OpenCC::new`] loads the embedded dictionaries without runtime file I/O.
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
//! - Use [`OpenCC::convert_with_config`] with an [`OpenccConfig`] for typed
//!   dispatch, or call a direct method such as [`OpenCC::s2t`] for a fixed mode.
//! - [`OpenCC::convert`] accepts OpenCC-style strings such as `"s2t"` and is
//!   useful for CLI/config-file compatibility.
//!   Use [`OpenccConfig::parse`] to validate names supplied by users.
//! - Use [`DictionaryMaxlength`] to load an external dictionary or apply a
//!   custom overlay, then pass it to [`OpenCC::from_dictionary`].
//!
//! Import public types and functions from the crate root, for example
//! `opencc_fmmseg::DictionaryMaxlength`.
//!
//! # Supported Configurations
//!
//! | Config | Method | Meaning |
//! | --- | --- | --- |
//! | `s2t` | [`OpenCC::s2t`] | Simplified to Traditional |
//! | `t2s` | [`OpenCC::t2s`] | Traditional to Simplified |
//! | `s2tw` / `s2twp` | [`OpenCC::s2tw`] / [`OpenCC::s2twp`] | Simplified to Taiwan Traditional |
//! | `tw2s` / `tw2sp` | [`OpenCC::tw2s`] / [`OpenCC::tw2sp`] | Taiwan Traditional to Simplified |
//! | `s2hk` / `s2hkp` / `t2hk` / `t2hkp` | [`OpenCC::s2hk`] / [`OpenCC::s2hkp`] / [`OpenCC::t2hk`] / [`OpenCC::t2hkp`] | To Hong Kong Traditional variants |
//! | `hk2s` / `hk2sp` / `hk2t` / `hk2tp` | [`OpenCC::hk2s`] / [`OpenCC::hk2sp`] / [`OpenCC::hk2t`] / [`OpenCC::hk2tp`] | Hong Kong variants to Simplified/Traditional |
//! | `t2tw` / `t2twp` | [`OpenCC::t2tw`] / [`OpenCC::t2twp`] | Traditional to Taiwan variants |
//! | `tw2t` / `tw2tp` | [`OpenCC::tw2t`] / [`OpenCC::tw2tp`] | Taiwan variants to Traditional |
//! | `t2jp` / `jp2t` | [`OpenCC::t2jp`] / [`OpenCC::jp2t`] | Traditional and Japanese kanji variants |
//! | `s2seal` / `t2seal` | [`OpenCC::s2seal`] / [`OpenCC::t2seal`] | Simplified/Traditional to Small Seal Script |
//! | `seal2s` / `seal2t` | [`OpenCC::seal2s`] / [`OpenCC::seal2t`] | Small Seal Script to Simplified/Traditional; |
//!
//! Regional configurations ending in `p` include regional vocabulary mappings.
//! The `punctuation` flag is independent of the configuration name. With the
//! bundled dictionaries, it maps curly quotation marks (`“”‘’`) to corner
//! brackets (`「」『』`) for Traditional, regional, Japanese, and Seal output.
//! Configurations that produce Simplified Chinese use the reverse mapping.
//!
//! ```rust
//! use opencc_fmmseg::OpenCC;
//!
//! let converter = OpenCC::new();
//! assert_eq!(converter.t2tw("“滑鼠”", true), "「滑鼠」");
//! assert_eq!(converter.t2tw("“滑鼠”", false), "“滑鼠”");
//! ```
//!
//! # Small Seal Script
//!
//! The four Seal configurations use bundled character and variant dictionaries.
//! Characters without a mapping pass through unchanged. Displaying Seal output
//! requires a font that covers those characters. Multiple input characters can
//! map to the same output, so a round trip can lose distinctions.
//!
//! ```rust
//! use opencc_fmmseg::{OpenCC, OpenccConfig};
//!
//! let converter = OpenCC::new();
//! let seal = converter.convert_with_config("小篆", OpenccConfig::T2seal, false);
//! assert_eq!(converter.seal2t(&seal, false), "小篆");
//! ```
//!
//! # Cargo Features
//!
//! Normal conversion, embedded dictionary loading, custom dictionary overlays,
//! and external dictionary loading require no optional features.
//!
//! Enable `dictionary-build` to use `DictionaryMaxlength::save_cbor_compressed`
//! when generating Zstandard-compressed CBOR artifacts. This feature adds the
//! native Zstandard encoder; runtime decompression uses the built-in pure-Rust
//! decoder. Uncompressed CBOR serialization remains available without it.
//!
//! ```toml
//! [dependencies]
//! opencc-fmmseg = { version = "0.13.1", features = ["dictionary-build"] }
//! ```
//!
//! # Custom Dictionaries
//!
//! Use [`CustomDictSpec`] for in-memory pairs or [`CustomDictFileSpec`] with
//! [`DictionaryMaxlength::with_custom_dict_files`] for text files. Each
//! [`DictSlot`] participates in specific conversion stages. For example,
//! [`DictSlot::STPhrases`] adds a phrase to the Simplified-to-Traditional stage.
//! [`CustomDictMode::Append`] keeps existing entries and replaces matching keys.
//! [`CustomDictMode::Override`] replaces the entire slot.
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
//! [`OpenCC::convert`] returns `"Invalid config: {config}"` as its output for an
//! invalid name and records that message in [`OpenCC::get_last_error`]. Parse
//! names first with [`OpenccConfig::parse`] to handle invalid input separately.
//!
//! [`OpenCC::new`], [`OpenCC::from_dicts`], and [`OpenCC::from_cbor`] fall back
//! to empty dictionaries if loading fails. To handle load failures directly,
//! use a [`DictionaryMaxlength`] loader that returns [`DictionaryError`] and
//! pass the resulting dictionary to [`OpenCC::from_dictionary`].
//!
//! The Rust diagnostics exposed by [`OpenCC::get_last_error`] and
//! [`DictionaryMaxlength::get_last_error`] are separate and process-wide.
//! Conversion clears the OpenCC diagnostic. A successful dictionary operation
//! may leave an earlier dictionary diagnostic in place.
//!
//! # Optional Text Processing
//!
//! Apply these steps explicitly when your input or display needs them.
//!
//! 1. Normalize compatibility characters before conversion. Choose
//!    [`OpenCC::normalize_compat_cow`] for CJK Compatibility Ideographs,
//!    [`OpenCC::normalize_unicode_compat_cow`] for the crate's curated table,
//!    or [`OpenCC::normalize_compat_extended_cow`] for both.
//! 2. Convert with [`OpenCC::convert_with_config`].
//! 3. Use [`OpenCC::detofu_cow`] to substitute rare CJK extension characters
//!    for display. Choose a [`DetofuLevel`] for the extension ranges to target,
//!    or build a reusable [`DetofuMap`] for custom substitutions.
//!
//! The `*_cow` methods return [`Cow::Borrowed`](std::borrow::Cow::Borrowed)
//! when the input is unchanged and [`Cow::Owned`](std::borrow::Cow::Owned) when
//! a replacement changes the text. Use the corresponding methods without the
//! suffix when you need a [`String`].
//!
//! ```rust
//! use opencc_fmmseg::{DetofuLevel, OpenCC, OpenccConfig};
//! use std::borrow::Cow;
//!
//! let converter = OpenCC::new();
//! let normalized = converter.normalize_compat_cow("金庸小说");
//! assert!(matches!(normalized, Cow::Borrowed(_)));
//!
//! let converted = converter.convert_with_config(&normalized, OpenccConfig::S2t, false);
//! let display = converter.detofu_cow(&converted, DetofuLevel::ExtB);
//! assert_eq!(display, "金庸小說");
//! ```
//!
//! [`OpenCC::set_preserve_ids`] can preserve Unicode Ideographic Description
//! Sequences during conversion. It is disabled by default.
//!
mod compat_ideographs;
/// Delimiters helper for splitting and matching delimiters.
mod delimiter_set;
/// Display compatibility fallback utilities for rare CJK extension characters.
mod detofu;
/// Bridge helper for conversion plan and core converter functions.
mod dict_refs;
/// Dictionary utilities for managing multiple OpenCC lexicons.
mod dictionary_lib;
mod ids;
mod lazy_replace;
/// Core converter
mod opencc;
/// Configurations for conversion.
mod opencc_config;
mod rustc_hash;
mod unicode_compat;
/// Common helpers for opencc-fmmseg.
mod utils;
#[allow(clippy::upper_case_acronyms)]
mod zstd;

// Text utilities
pub use crate::delimiter_set::is_delimiter;

// Dictionary API
pub use crate::dictionary_lib::{
    CustomDictFileSpec, CustomDictMode, CustomDictSpec, DictMaxLen, DictSlot, DictionaryError,
    DictionaryMaxlength,
};

// Conversion API
pub use crate::opencc::OpenCC;
pub use crate::opencc_config::OpenccConfig;

// DeTofu API
pub use crate::detofu::{DetofuLevel, DetofuMap};
