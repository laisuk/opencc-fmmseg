//! Display compatibility fallback utilities.
//!
//! This module provides optional "detofu" processing for non-BMP
//! CJK extension characters that may not render correctly on some
//! systems, fonts, browsers, or e-book readers.

use std::collections::HashMap;
use std::sync::OnceLock;

static TOFU_DATA: &[u8] = include_bytes!("data/TSCharactersTofu.txt");

/// Controls which CJK extension ranges are replaced by detofu.
///
/// Detofu levels are threshold-based: the selected level is the earliest
/// extension block to replace, and all supported later extension blocks are
/// replaced too.
///
/// - [`DetofuLevel::ExtB`] means ExtB+ and replaces all supported non-BMP
///   mappings: ExtB, ExtC, ExtD, ExtE, ExtF, ExtG, ExtH, and ExtI.
/// - [`DetofuLevel::ExtC`] means ExtC+ and replaces ExtC through ExtI.
/// - [`DetofuLevel::ExtD`] means ExtD+ and replaces ExtD through ExtI.
/// - [`DetofuLevel::ExtE`] means ExtE+ and replaces ExtE through ExtI.
///
/// The CLI alias `all` maps to [`DetofuLevel::ExtB`], so `ExtB` is the
/// broadest built-in fallback level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DetofuLevel {
    /// Replace CJK Extension B and all supported later extension mappings.
    ExtB,
    /// Replace CJK Extension C and all supported later extension mappings.
    ExtC,
    /// Replace CJK Extension D and all supported later extension mappings.
    ExtD,
    /// Replace CJK Extension E and all supported later extension mappings.
    ExtE,
    /// Replace CJK Extension F and all supported later extension mappings.
    ExtF,
    /// Replace CJK Extension G and all supported later extension mappings.
    ExtG,
    /// Replace CJK Extension H and all supported later extension mappings.
    ExtH,
    /// Replace CJK Extension I mappings.
    ExtI,
}

impl DetofuLevel {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.to_ascii_lowercase().as_str() {
            "all" | "ext-b" | "b" => Ok(Self::ExtB),
            "ext-c" | "c" => Ok(Self::ExtC),
            "ext-d" | "d" => Ok(Self::ExtD),
            "ext-e" | "e" => Ok(Self::ExtE),
            "ext-f" | "f" => Ok(Self::ExtF),
            "ext-g" | "g" => Ok(Self::ExtG),
            "ext-h" | "h" => Ok(Self::ExtH),
            "ext-i" | "i" => Ok(Self::ExtI),
            _ => Err("supported detofu levels: all, ext-b, ext-c, ext-d, ext-e, ext-f, ext-g, ext-h, ext-i".to_string()),
        }
    }

    fn from_ext(ext: &str) -> Option<Self> {
        match ext {
            "ExtB" => Some(Self::ExtB),
            "ExtC" => Some(Self::ExtC),
            "ExtD" => Some(Self::ExtD),
            "ExtE" => Some(Self::ExtE),
            "ExtF" => Some(Self::ExtF),
            "ExtG" => Some(Self::ExtG),
            "ExtH" => Some(Self::ExtH),
            "ExtI" => Some(Self::ExtI),
            _ => None,
        }
    }
}

static TOFU_ENTRIES: OnceLock<Vec<(char, char, DetofuLevel)>> = OnceLock::new();

fn tofu_entries() -> &'static [(char, char, DetofuLevel)] {
    TOFU_ENTRIES.get_or_init(|| {
        let text =
            std::str::from_utf8(TOFU_DATA).expect("TSCharactersTofu.txt must be valid UTF-8");

        text.lines()
            .filter(|line| {
                let line = line.trim();
                !line.is_empty() && !line.starts_with('#')
            })
            .filter_map(|line| {
                let mut parts = line.split('\t');
                let tofu = parts.next()?.chars().next()?;
                let fallback = parts.next()?.chars().next()?;
                let ext = DetofuLevel::from_ext(parts.next()?)?;
                Some((tofu, fallback, ext))
            })
            .collect()
    })
}

/// A reusable map for detofu display-compatibility fallback.
///
/// `DetofuMap` is an advanced API for callers that want to build a fallback
/// table once and reuse it across many strings, or layer application-specific
/// fallbacks on top of the built-in map.
///
/// Detofu is independent from OpenCC conversion dictionaries. It does not
/// participate in Simplified/Traditional phrase matching, regional variant
/// selection, punctuation conversion, or any other OpenCC conversion logic.
/// It is best treated as a display compatibility pass that can run after
/// conversion when the target renderer has incomplete rare-character coverage.
///
/// # Examples
///
/// ```rust
/// use opencc_fmmseg::{DetofuLevel, DetofuMap};
///
/// let map = DetofuMap::builtin(DetofuLevel::ExtB)
///     .with_custom_pairs(&[
///         ('𣭲', '氄'),
///     ]);
///
/// let safe = map.detofu("這隻小狗有𣭲毛");
///
/// assert_eq!(safe, "這隻小狗有氄毛");
/// ```
#[derive(Debug, Clone)]
pub struct DetofuMap {
    map: HashMap<char, char>,
}

impl DetofuMap {
    /// Builds a detofu map from the crate's built-in compatibility data.
    ///
    /// The selected [`DetofuLevel`] is threshold-based. For example,
    /// [`DetofuLevel::ExtB`] loads all supported non-BMP mappings, while
    /// [`DetofuLevel::ExtE`] loads only ExtE and later supported mappings.
    ///
    /// The built-in detofu map is independent from the OpenCC conversion
    /// dictionaries bundled with this crate.
    pub fn builtin(level: DetofuLevel) -> Self {
        let map = tofu_entries()
            .iter()
            .filter(|(_, _, ext)| *ext >= level)
            .map(|(tofu, fallback, _)| (*tofu, *fallback))
            .collect();

        Self { map }
    }

    /// Adds or overrides compatibility fallback pairs after loading the map.
    ///
    /// Custom pairs are applied post-load. If a custom key already exists in
    /// the built-in detofu map, the custom fallback wins.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use opencc_fmmseg::{DetofuLevel, DetofuMap};
    ///
    /// let map = DetofuMap::builtin(DetofuLevel::ExtB)
    ///     .with_custom_pairs(&[('𣭲', '氄')]);
    ///
    /// assert_eq!(map.detofu("𣭲"), "氄");
    /// ```
    pub fn with_custom_pairs(mut self, pairs: &[(char, char)]) -> Self {
        for &(tofu, fallback) in pairs {
            self.map.insert(tofu, fallback);
        }
        self
    }

    /// Replaces mapped non-BMP CJK extension characters with compatibility fallbacks.
    ///
    /// Characters not present in this map are copied unchanged. This is a
    /// display compatibility operation only; it does not modify OpenCC
    /// conversion dictionaries or conversion behavior.
    pub fn detofu(&self, input: &str) -> String {
        let mut output = String::with_capacity(input.len());

        for ch in input.chars() {
            if let Some(fallback) = self.map.get(&ch) {
                output.push(*fallback);
            } else {
                output.push(ch);
            }
        }

        output
    }
}

/// Converts non-BMP CJK extension characters to compatibility fallbacks.
///
/// This convenience function builds the built-in [`DetofuMap`] for `level` and
/// applies it to `input`. It is intended for environments with incomplete font
/// coverage where rare CJK extension characters may render as tofu boxes on
/// some systems, fonts, browsers, or e-book readers.
///
/// Detofu is independent from OpenCC conversion dictionaries and does not
/// modify OpenCC conversion logic. In a typical workflow, run OpenCC
/// conversion first and then apply detofu to the converted text.
///
/// # Examples
///
/// ```rust
/// use opencc_fmmseg::{detofu, DetofuLevel};
///
/// let safe = detofu("骖𬴂", DetofuLevel::ExtB);
///
/// assert_eq!(safe, "骖騑");
/// ```
pub fn detofu(input: &str, level: DetofuLevel) -> String {
    DetofuMap::builtin(level).detofu(input)
}
