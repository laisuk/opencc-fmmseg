//! # Text conversion pipeline
//!
//! Reusable text-to-text conversion support for the `opencc-rs` command-line
//! application.
//!
//! This module separates **text transformation policy** from input/output and
//! document-container handling. Consumers such as plain-text conversion,
//! Office/EPUB conversion, filename conversion, and future PDF support only need
//! a [`TextConverter`]; they do not need to know how normalization, OpenCC
//! conversion, punctuation conversion, or DeTofu processing are composed.
//!
//! The standard pipeline created by [`create_text_converter`] applies
//! transformations in this order:
//!
//! 1. Optional compatibility normalization.
//! 2. OpenCC conversion, optionally including punctuation conversion.
//! 3. Optional DeTofu fallback replacement.
//!
//! # Example
//!
//! ```rust,no_run
//! use opencc_fmmseg::{DetofuLevel, DetofuMap, OpenCC};
//!
//! use crate::text_converter::{
//!     create_text_converter, NormalizationMode, TextConverterOptions,
//! };
//!
//! let cc = OpenCC::new();
//! let detofu = DetofuMap::builtin(DetofuLevel::ExtB);
//!
//! let converter = create_text_converter(
//!     &cc,
//!     TextConverterOptions {
//!         config: "t2s",
//!         punctuation: false,
//!         normalization: NormalizationMode::CompatExtended,
//!         detofu_map: Some(&detofu),
//!     },
//! );
//!
//! let output = converter.convert("聼𧜗");
//! assert_eq!(output, "听䘞");
//! ```
//!
//! [`TextConverter`] itself is deliberately independent of OpenCC. Any
//! `Fn(&str) -> String` can be wrapped and supplied to a consumer that accepts a
//! text converter.

use std::borrow::Cow;

use opencc_fmmseg::{DetofuMap, OpenCC};

/// Specifies compatibility normalization performed before OpenCC conversion.
///
/// Extended compatibility normalization takes the place of basic compatibility
/// normalization rather than being applied as an additional pass.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NormalizationMode {
    /// Do not perform compatibility normalization.
    #[default]
    None,

    /// Normalize CJK Compatibility Ideographs.
    Compat,

    /// Normalize both CJK Compatibility Ideographs and the extended curated
    /// Unicode compatibility/allograph mappings.
    CompatExtended,
}

/// Configuration for the standard OpenCC text-conversion pipeline.
///
/// The options are resolved before conversion begins so consumers only need to
/// call [`TextConverter::convert`] for each piece of text.
///
/// This type contains borrowed references only and is therefore inexpensive to
/// copy.
#[derive(Clone, Copy)]
pub struct TextConverterOptions<'a> {
    /// OpenCC conversion configuration, for example `"s2t"` or `"t2s"`.
    pub config: &'a str,

    /// Whether OpenCC punctuation conversion is enabled.
    pub punctuation: bool,

    /// Compatibility normalization performed before OpenCC conversion.
    pub normalization: NormalizationMode,

    /// Optional DeTofu map applied after OpenCC conversion.
    pub detofu_map: Option<&'a DetofuMap>,
}

/// A caller-supplied text-to-text transformation.
///
/// `TextConverter` deliberately knows nothing about files, ZIP archives,
/// Office documents, EPUB packages, encodings, or command-line arguments. It
/// simply transforms one `&str` into an owned [`String`].
///
/// This makes the same converter reusable by plain-text conversion,
/// Office/EPUB processing, filename conversion, PDF extraction pipelines, and
/// other consumers.
///
/// # Custom converters
///
/// A converter does not have to use OpenCC:
///
/// ```rust
/// # use crate::text_converter::TextConverter;
/// let converter = TextConverter::new(|text: &str| text.replace("汉语", "漢語"));
///
/// assert_eq!(converter.convert("汉语"), "漢語");
/// ```
pub struct TextConverter<F> {
    convert: F,
}

impl<F> TextConverter<F>
where
    F: Fn(&str) -> String,
{
    /// Creates a text converter from a closure or function.
    #[inline]
    pub fn new(convert: F) -> Self {
        Self { convert }
    }

    /// Transforms the supplied text using the wrapped conversion policy.
    #[inline]
    pub fn convert(&self, text: &str) -> String {
        (self.convert)(text)
    }
}

/// Creates the standard reusable OpenCC text-conversion pipeline.
///
/// The returned [`TextConverter`] captures the supplied [`OpenCC`] instance and
/// pipeline options. Callers can therefore configure the pipeline once and
/// repeatedly convert text without passing the OpenCC configuration,
/// punctuation flag, normalization mode, or DeTofu map on every call.
///
/// Transformations are applied in the following order:
///
/// 1. [`NormalizationMode::CompatExtended`] or
///    [`NormalizationMode::Compat`], when selected.
/// 2. [`OpenCC::convert`], with the configured OpenCC conversion and punctuation
///    setting.
/// 3. [`DetofuMap::detofu`], when a DeTofu map is supplied.
///
/// # Parameters
///
/// - `opencc` - OpenCC instance used for normalization and conversion.
/// - `options` - Conversion policy captured by the returned converter.
///
/// # Example
///
/// ```rust,no_run
/// use opencc_fmmseg::OpenCC;
///
/// use crate::text_converter::{
///     create_text_converter, NormalizationMode, TextConverterOptions,
/// };
///
/// let cc = OpenCC::new();
///
/// let converter = create_text_converter(
///     &cc,
///     TextConverterOptions {
///         config: "s2t",
///         punctuation: true,
///         normalization: NormalizationMode::CompatExtended,
///         detofu_map: None,
///     },
/// );
///
/// let output = converter.convert("汉语");
/// ```
pub fn create_text_converter<'a>(
    opencc: &'a OpenCC,
    options: TextConverterOptions<'a>,
) -> TextConverter<impl Fn(&str) -> String + 'a> {
    TextConverter::new(move |input| {
        apply_conversion_pipeline(
            opencc,
            options.normalization,
            options.detofu_map,
            input,
            options.config,
            options.punctuation,
        )
    })
}

/// Applies the standard text-conversion pipeline.
///
/// Kept private so callers use [`TextConverter`] rather than depending on the
/// internal composition of individual conversion stages.
fn apply_conversion_pipeline(
    opencc: &OpenCC,
    normalization: NormalizationMode,
    detofu_map: Option<&DetofuMap>,
    input: &str,
    config: &str,
    punctuation: bool,
) -> String {
    let normalized = match normalization {
        NormalizationMode::None => Cow::Borrowed(input),
        NormalizationMode::Compat => Cow::Owned(opencc.normalize_compat(input)),
        NormalizationMode::CompatExtended => Cow::Owned(opencc.normalize_compat_extended(input)),
    };

    let converted = opencc.convert(normalized.as_ref(), config, punctuation);

    match detofu_map {
        Some(map) => map.detofu(&converted),
        None => converted,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opencc_fmmseg::DetofuLevel;

    #[test]
    fn custom_text_converter_wraps_plain_closure() {
        let converter = TextConverter::new(|text: &str| text.replace("汉语", "漢語"));

        assert_eq!(converter.convert("汉语"), "漢語");
    }

    #[test]
    fn opencc_pipeline_applies_normalize_convert_detofu() {
        let cc = OpenCC::new();
        let detofu = DetofuMap::builtin(DetofuLevel::ExtB);

        let converter = create_text_converter(
            &cc,
            TextConverterOptions {
                config: "t2s",
                punctuation: false,
                normalization: NormalizationMode::CompatExtended,
                detofu_map: Some(&detofu),
            },
        );

        assert_eq!(converter.convert("聼𧜗"), "听䘞");
    }

    #[test]
    fn opencc_pipeline_can_run_without_optional_stages() {
        let cc = OpenCC::new();

        let converter = create_text_converter(
            &cc,
            TextConverterOptions {
                config: "s2t",
                punctuation: false,
                normalization: NormalizationMode::None,
                detofu_map: None,
            },
        );

        assert_eq!(converter.convert("汉字"), "漢字");
    }
}
