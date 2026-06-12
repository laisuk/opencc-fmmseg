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

/// Parses a canonical OpenCC dictionary slot name into a [`DictSlot`].
///
/// This conversion is intentionally strict and only accepts canonical
/// slot identifiers used by the OpenCC dictionary pipeline.
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
/// - `JPShinjitaiCharacters`
/// - `JPShinjitaiCharactersRev`
/// - `JPShinjitaiPhrases`
///
/// # Notes
///
/// File suffixes such as `.txt` are not accepted here.
/// Higher-level bindings (for example Python wrappers) may provide
/// additional normalization or compatibility handling.
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

            "JPShinjitaiCharacters" => Ok(Self::JPSCharacters),
            "JPShinjitaiCharactersRev" => Ok(Self::JPSCharactersRev),
            "JPShinjitaiPhrases" => Ok(Self::JPSPhrases),

            _ => Err(()),
        }
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
