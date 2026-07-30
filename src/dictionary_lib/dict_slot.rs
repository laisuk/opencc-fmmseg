/// Identifies a dictionary slot inside [`DictionaryMaxlength`](crate::dictionary_lib::DictionaryMaxlength).
///
/// Each slot corresponds to a specific OpenCC conversion dictionary.
/// Custom dictionaries can be appended to or override entries inside
/// these slots when using:
///
/// - [`DictionaryMaxlength::from_dicts_custom()`](crate::dictionary_lib::DictionaryMaxlength::from_dicts_custom)
/// - [`DictionaryMaxlength::from_dicts_custom_files()`](crate::dictionary_lib::DictionaryMaxlength::from_dicts_custom_files)
///
/// # Notes
///
/// OpenCC conversion behavior depends heavily on choosing the correct slot.
/// For example:
///
/// - [`DictSlot::STPhrases`] affects Simplified → Traditional phrase conversion.
/// - [`DictSlot::TSPhrases`] affects Traditional → Simplified phrase conversion.
/// - [`DictSlot::TWVariants`] affects Taiwan regional variants.
/// - [`DictSlot::TWVariantsPhrases`] affects Taiwan regional phrase variants.
/// - [`DictSlot::HKPhrases`] affects Hong Kong regional phrase conversion.
/// - [`DictSlot::HKVariants`] affects Hong Kong regional variants.
/// - [`DictSlot::HKVariantsPhrases`] affects Hong Kong regional phrase variants.
///
/// # See Also
///
/// - [`CustomDictSpec`]
/// - [`CustomDictFileSpec`]
/// - [`CustomDictMode`]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DictSlot {
    /// Simplified → Traditional character mappings.
    STCharacters,

    /// Simplified → Traditional phrase mappings.
    STPhrases,

    /// Traditional → Simplified character mappings.
    TSCharacters,

    /// Traditional → Simplified phrase mappings.
    TSPhrases,

    /// Traditional → Taiwan phrase mappings.
    TWPhrases,

    /// Taiwan → Traditional reverse phrase mappings.
    TWPhrasesRev,

    /// Traditional → Hong Kong phrase mappings.
    HKPhrases,

    /// Hong Kong → Traditional reverse phrase mappings.
    HKPhrasesRev,

    /// Traditional → Taiwan regional variant mappings.
    TWVariants,

    /// Traditional → Taiwan regional phrase variant mappings.
    ///
    /// Applied before [`DictSlot::TWVariants`] so phrase-level regional
    /// semantics can be preserved before character-level fallback.
    TWVariantsPhrases,

    /// Taiwan → Traditional reverse variant mappings.
    TWVariantsRev,

    /// Taiwan → Traditional reverse phrase variant mappings.
    TWVariantsRevPhrases,

    /// Traditional → Hong Kong regional variant mappings.
    HKVariants,

    /// Traditional → Hong Kong regional phrase variant mappings.
    ///
    /// Applied before [`DictSlot::HKVariants`] so phrase-level regional
    /// semantics can be preserved before character-level fallback.
    HKVariantsPhrases,

    /// Hong Kong → Traditional reverse variant mappings.
    HKVariantsRev,

    /// Hong Kong → Traditional reverse phrase variant mappings.
    HKVariantsRevPhrases,

    /// Japanese Shinjitai character mappings.
    JPSCharacters,

    /// Japanese Shinjitai reverse character mappings.
    JPSCharactersRev,

    /// Japanese Shinjitai phrase mappings.
    JPSPhrases,

    /// Simplified → Traditional punctuation mappings.
    STPunctuations,

    /// Traditional → Simplified punctuation mappings.
    TSPunctuations,
}

/// Parses a canonical OpenCC dictionary slot name or a supported compatibility
/// alias into a [`DictSlot`].
///
/// This conversion is case-sensitive and does not trim whitespace.
///
/// # Supported slot names
///
/// - `STCharacters`
/// - `STPhrases`
/// - `STPunctuations`
/// - `TSCharacters`
/// - `TSPhrases`
/// - `TSPunctuations`
/// - `TWPhrases`
/// - `TWPhrasesRev`
/// - `HKPhrases`
/// - `HKPhrasesRev`
/// - `TWVariants`
/// - `TWVariantsPhrases`
/// - `TWVariantsRev`
/// - `TWVariantsRevPhrases`
/// - `HKVariants`
/// - `HKVariantsPhrases`
/// - `HKVariantsRev`
/// - `HKVariantsRevPhrases`
/// - `JPSCharacters`
/// - `JPSCharactersRev`
/// - `JPSPhrases`
///
/// # Compatibility aliases
///
/// For compatibility with `opencc-fmmseg` v0.11.4, the physical Japanese
/// dictionary filename stems `JPShinjitaiCharacters`,
/// `JPShinjitaiCharactersRev`, and `JPShinjitaiPhrases` are also accepted.
/// These aliases are deprecated and will be removed in the 0.12.x release
/// series; use the canonical `JPS*` names in new code and configuration.
///
/// File suffixes such as `.txt` are not accepted.
///
/// # Examples
///
/// ```rust
/// use opencc_fmmseg::DictSlot;
///
/// assert_eq!(
///     DictSlot::try_from("STPhrases"),
///     Ok(DictSlot::STPhrases)
/// );
///
/// assert!(DictSlot::try_from("STPhrases.txt").is_err());
/// ```
impl TryFrom<&str> for DictSlot {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "STCharacters" => Ok(Self::STCharacters),
            "STPhrases" => Ok(Self::STPhrases),
            "STPunctuations" => Ok(Self::STPunctuations),

            "TSCharacters" => Ok(Self::TSCharacters),
            "TSPhrases" => Ok(Self::TSPhrases),
            "TSPunctuations" => Ok(Self::TSPunctuations),

            "TWPhrases" => Ok(Self::TWPhrases),
            "TWPhrasesRev" => Ok(Self::TWPhrasesRev),
            "HKPhrases" => Ok(Self::HKPhrases),
            "HKPhrasesRev" => Ok(Self::HKPhrasesRev),
            "TWVariants" => Ok(Self::TWVariants),
            "TWVariantsPhrases" => Ok(Self::TWVariantsPhrases),
            "TWVariantsRev" => Ok(Self::TWVariantsRev),
            "TWVariantsRevPhrases" => Ok(Self::TWVariantsRevPhrases),

            "HKVariants" => Ok(Self::HKVariants),
            "HKVariantsPhrases" => Ok(Self::HKVariantsPhrases),
            "HKVariantsRev" => Ok(Self::HKVariantsRev),
            "HKVariantsRevPhrases" => Ok(Self::HKVariantsRevPhrases),

            "JPSCharacters" => Ok(Self::JPSCharacters),
            "JPSCharactersRev" => Ok(Self::JPSCharactersRev),
            "JPSPhrases" => Ok(Self::JPSPhrases),

            // Compatibility aliases accepted by v0.11.4. Remove in 0.12.x.
            "JPShinjitaiCharacters" => Ok(Self::JPSCharacters),
            "JPShinjitaiCharactersRev" => Ok(Self::JPSCharactersRev),
            "JPShinjitaiPhrases" => Ok(Self::JPSPhrases),

            _ => Err(()),
        }
    }
}

impl DictSlot {
    /// Every supported dictionary slot.
    ///
    /// This is useful for presenting slot choices or validating integrations
    /// without maintaining a separate list. Each entry's stable public name is
    /// returned by [`DictSlot::canonical_name`].
    pub const ALL: &'static [Self] = &[
        Self::STCharacters,
        Self::STPhrases,
        Self::STPunctuations,
        Self::TSCharacters,
        Self::TSPhrases,
        Self::TSPunctuations,
        Self::TWPhrases,
        Self::TWPhrasesRev,
        Self::HKPhrases,
        Self::HKPhrasesRev,
        Self::TWVariants,
        Self::TWVariantsPhrases,
        Self::TWVariantsRev,
        Self::TWVariantsRevPhrases,
        Self::HKVariants,
        Self::HKVariantsPhrases,
        Self::HKVariantsRev,
        Self::HKVariantsRevPhrases,
        Self::JPSCharacters,
        Self::JPSCharactersRev,
        Self::JPSPhrases,
    ];

    /// Returns the canonical public name of this dictionary slot.
    ///
    /// Canonical names identify logical slots and can be parsed by
    /// [`DictSlot::try_from`]. They are not necessarily the stems of the
    /// corresponding physical dictionary filenames. In particular, the
    /// Japanese slots use `JPSCharacters`, `JPSCharactersRev`, and
    /// `JPSPhrases`, while their bundled files retain the `JPShinjitai...`
    /// names.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use opencc_fmmseg::DictSlot;
    ///
    /// assert_eq!(DictSlot::JPSCharacters.canonical_name(), "JPSCharacters");
    /// assert_eq!(
    ///     DictSlot::try_from(DictSlot::JPSCharacters.canonical_name()),
    ///     Ok(DictSlot::JPSCharacters),
    /// );
    /// ```
    #[must_use]
    pub const fn canonical_name(self) -> &'static str {
        match self {
            Self::STCharacters => "STCharacters",
            Self::STPhrases => "STPhrases",
            Self::STPunctuations => "STPunctuations",
            Self::TSCharacters => "TSCharacters",
            Self::TSPhrases => "TSPhrases",
            Self::TSPunctuations => "TSPunctuations",
            Self::TWPhrases => "TWPhrases",
            Self::TWPhrasesRev => "TWPhrasesRev",
            Self::HKPhrases => "HKPhrases",
            Self::HKPhrasesRev => "HKPhrasesRev",
            Self::TWVariants => "TWVariants",
            Self::TWVariantsPhrases => "TWVariantsPhrases",
            Self::TWVariantsRev => "TWVariantsRev",
            Self::TWVariantsRevPhrases => "TWVariantsRevPhrases",
            Self::HKVariants => "HKVariants",
            Self::HKVariantsPhrases => "HKVariantsPhrases",
            Self::HKVariantsRev => "HKVariantsRev",
            Self::HKVariantsRevPhrases => "HKVariantsRevPhrases",
            Self::JPSCharacters => "JPSCharacters",
            Self::JPSCharactersRev => "JPSCharactersRev",
            Self::JPSPhrases => "JPSPhrases",
        }
    }

    /// Parses a canonical slot name or supported compatibility alias without
    /// regard to ASCII case.
    ///
    /// Leading and trailing whitespace is ignored. The deprecated
    /// `JPShinjitaiCharacters`, `JPShinjitaiCharactersRev`, and
    /// `JPShinjitaiPhrases` aliases remain accepted for v0.11.4 compatibility
    /// and will be removed in the 0.12.x release series. Filename suffixes and
    /// other aliases are not accepted. Use [`DictSlot::try_from`] when parsing
    /// must be case-sensitive and must not trim whitespace.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use opencc_fmmseg::DictSlot;
    ///
    /// assert_eq!(
    ///     DictSlot::from_name_ignore_ascii_case("  jpscharactersrev  "),
    ///     Some(DictSlot::JPSCharactersRev),
    /// );
    /// assert_eq!(
    ///     DictSlot::from_name_ignore_ascii_case("JPShinjitaiCharactersRev"),
    ///     Some(DictSlot::JPSCharactersRev),
    /// );
    /// ```
    #[must_use]
    pub fn from_name_ignore_ascii_case(value: &str) -> Option<Self> {
        let value = value.trim();

        Self::ALL
            .iter()
            .copied()
            .find(|slot| slot.canonical_name().eq_ignore_ascii_case(value))
            .or_else(|| {
                // Compatibility aliases accepted by v0.11.4. Remove in 0.12.x.
                if "JPShinjitaiCharacters".eq_ignore_ascii_case(value) {
                    Some(Self::JPSCharacters)
                } else if "JPShinjitaiCharactersRev".eq_ignore_ascii_case(value) {
                    Some(Self::JPSCharactersRev)
                } else if "JPShinjitaiPhrases".eq_ignore_ascii_case(value) {
                    Some(Self::JPSPhrases)
                } else {
                    None
                }
            })
    }
}

/// Controls how custom dictionary entries are merged into a slot.
///
/// Used by [`CustomDictSpec`] and [`CustomDictFileSpec`].
///
/// # Modes
///
/// In post-load customization APIs, [`CustomDictMode::Append`] merges custom
/// entries into the selected slot with last-wins semantics, while
/// [`CustomDictMode::Override`] clears the selected slot first.
///
/// # Example
///
/// ```rust
/// use opencc_fmmseg::{
///     CustomDictMode,
///     CustomDictSpec,
///     DictSlot,
///     DictionaryMaxlength,
/// };
///
/// let dictionary = DictionaryMaxlength::from_dicts_custom(&[
///     CustomDictSpec {
///         slot: DictSlot::STPhrases,
///         pairs: vec![
///             ("帕兰蒂尔".to_string(), "柏蘭蒂爾".to_string()),
///         ],
///         mode: CustomDictMode::Override,
///     }
/// ]).unwrap();
///
/// assert!(dictionary.st_phrases.max_len > 0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CustomDictMode {
    /// Merge custom entries into the selected slot.
    ///
    /// Post-load APIs use last-wins semantics for conflicting keys.
    Append,

    /// Prefer custom entries over the selected slot's existing contents.
    ///
    /// Post-load APIs clear the slot before custom pairs are inserted.
    Override,
}

/// Pair-based custom dictionary injection.
///
/// This is the core no-I/O path for custom dictionaries and is suitable for:
///
/// - `include_str!()`
/// - dynamically generated dictionaries
/// - database-loaded pairs
/// - testing
/// - embedded applications
/// - WebAssembly environments
///
/// # Notes
///
/// Custom pairs are merged into the selected [`DictSlot`] before
/// internal starter indexes and maximum phrase lengths are rebuilt.
///
/// # Example
///
/// ```rust
/// use opencc_fmmseg::{
///     CustomDictMode,
///     CustomDictSpec,
///     DictSlot,
///     DictionaryMaxlength,
/// };
///
/// let dictionary = DictionaryMaxlength::from_dicts_custom(&[
///     CustomDictSpec {
///         slot: DictSlot::STPhrases,
///         pairs: vec![
///             ("帕兰蒂尔".to_string(), "柏蘭蒂爾".to_string()),
///         ],
///         mode: CustomDictMode::Override,
///     }
/// ]).unwrap();
///
/// assert!(dictionary.st_phrases.max_len > 0);
/// ```
#[derive(Debug, Clone)]
pub struct CustomDictSpec {
    /// Target dictionary slot.
    pub slot: DictSlot,

    /// Key-value dictionary pairs.
    ///
    /// Each pair follows standard OpenCC semantics:
    ///
    /// `(source, target)`
    pub pairs: Vec<(String, String)>,

    /// Merge behavior for conflicting keys.
    pub mode: CustomDictMode,
}

/// File-based custom dictionary injection.
///
/// This is a convenience wrapper for loading one or more OpenCC-style
/// dictionary text files for a slot.
///
/// Each file should follow the standard OpenCC dictionary format:
///
/// ```text
/// source<TAB>target
/// ```
///
/// # Notes
///
/// Multiple files are loaded in order.
///
/// # Example
///
/// ```rust, no_run
/// use opencc_fmmseg::{
///     CustomDictFileSpec,
///     CustomDictMode,
///     DictSlot,
///     DictionaryMaxlength,
/// };
///
/// let dictionary = DictionaryMaxlength::from_dicts_custom_files(&[
///     CustomDictFileSpec {
///         slot: DictSlot::STPhrases,
///         files: vec!["./my_terms.txt"],
///         mode: CustomDictMode::Override,
///     }
/// ]).unwrap();
///
/// assert!(dictionary.st_phrases.max_len > 0);
/// ```
#[derive(Debug, Clone)]
pub struct CustomDictFileSpec<P> {
    /// Target dictionary slot.
    pub slot: DictSlot,

    /// OpenCC-style dictionary text files.
    ///
    /// Files are loaded sequentially in the provided order.
    pub files: Vec<P>,

    /// Merge behavior for conflicting keys.
    pub mode: CustomDictMode,
}
