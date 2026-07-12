/// OpenCC conversion configuration (strongly-typed).
///
/// This enum represents the supported conversion “modes” (e.g. Simplified → Traditional).
/// It is used by [`OpenCC::convert_with_config`](crate::OpenCC::convert_with_config)
/// to avoid string parsing in hot paths.
///
/// # ABI / FFI
///
/// `OpenccConfig` is marked with `#[repr(u32)]`, so each variant has a stable numeric value.
/// This is suitable for C FFI where configs are passed as `uint32_t` (`opencc_config_t`).
///
/// When accepting configs from FFI, **do not** `transmute`; use [`OpenccConfig::from_ffi`]
/// to validate values.
///
/// # String parsing
///
/// For convenience and backwards compatibility, configs can also be parsed from strings
/// via `TryFrom<&str>` (case-insensitive), which powers
/// [`OpenCC::convert`](crate::OpenCC::convert).
///
/// # Variants
///
/// | Variant | Name   | Description                               | Punctuation parameter used? |
/// |--------:|--------|-------------------------------------------|-----------------------------|
/// | 1       | `S2t`  | Simplified → Traditional                   | ✅ (passed through)         |
/// | 2       | `S2tw` | Simplified → Traditional (Taiwan)          | ✅                          |
/// | 3       | `S2twp`| Simplified → Taiwan (with phrases)         | ✅                          |
/// | 4       | `S2hk` | Simplified → Hong Kong                     | ✅                          |
/// | 5       | `T2s`  | Traditional → Simplified                   | ✅                          |
/// | 6       | `T2tw` | Traditional → Taiwan                       | ❌ (ignored)                |
/// | 7       | `T2twp`| Traditional → Taiwan (with phrases)        | ❌ (ignored)                |
/// | 8       | `T2hk` | Traditional → Hong Kong                    | ❌ (ignored)                |
/// | 9       | `Tw2s` | Taiwan → Simplified                        | ✅                          |
/// | 10      | `Tw2sp`| Taiwan → Simplified (variant)              | ✅                          |
/// | 11      | `Tw2t` | Taiwan → Traditional                       | ❌ (ignored)                |
/// | 12      | `Tw2tp`| Taiwan → Traditional (variant)             | ❌ (ignored)                |
/// | 13      | `Hk2s` | Hong Kong → Simplified                     | ✅                          |
/// | 14      | `Hk2t` | Hong Kong → Traditional                    | ❌ (ignored)                |
/// | 15      | `Jp2t` | Japanese (Kanji variants) → Traditional     | ❌ (ignored)                |
/// | 16      | `T2jp` | Traditional → Japanese (Kanji variants)     | ❌ (ignored)                |
/// | 17      | `S2hkp`| Simplified → Hong Kong (with phrases)       | ✅                          |
/// | 18      | `Hk2sp`| Hong Kong → Simplified (with phrases)       | ✅                          |
/// | 19      | `T2hkp`| Traditional → Hong Kong (with phrases)      | ❌ (ignored)                |
/// | 20      | `Hk2tp`| Hong Kong → Traditional (with phrases)      | ❌ (ignored)                |
/// # Since
///
/// Core configurations (`S2t` through `T2jp`) are available since **v0.8.4**.
///
/// Hong Kong phrase configurations:
///
/// - `S2hkp` — available since **v0.11.0**
/// - `Hk2sp` — available since **v0.11.0**
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenccConfig {
    /// Simplified Chinese → Traditional Chinese.
    S2t = 1,

    /// Simplified Chinese → Traditional Chinese (Taiwan standard).
    S2tw = 2,

    /// Simplified Chinese → Traditional Chinese (Taiwan, with phrases).
    S2twp = 3,

    /// Simplified Chinese → Traditional Chinese (Hong Kong standard).
    S2hk = 4,

    /// Traditional Chinese → Simplified Chinese.
    T2s = 5,

    /// Traditional Chinese → Taiwan variant.
    T2tw = 6,

    /// Traditional Chinese → Taiwan variant (with phrases).
    T2twp = 7,

    /// Traditional Chinese → Hong Kong variant.
    T2hk = 8,

    /// Taiwan variant → Simplified Chinese.
    Tw2s = 9,

    /// Taiwan variant → Simplified Chinese (with phrases).
    Tw2sp = 10,

    /// Taiwan variant → Traditional Chinese.
    Tw2t = 11,

    /// Taiwan variant → Traditional Chinese (with phrases).
    Tw2tp = 12,

    /// Hong Kong variant → Simplified Chinese.
    Hk2s = 13,

    /// Hong Kong variant → Traditional Chinese.
    Hk2t = 14,

    /// Japanese Kanji → Traditional Chinese.
    Jp2t = 15,

    /// Traditional Chinese → Japanese Kanji.
    T2jp = 16,

    /// Simplified Chinese → Traditional Chinese (Hong Kong, with phrases).
    S2hkp = 17,

    /// Hong Kong variant → Simplified Chinese (with phrases).
    Hk2sp = 18,

    /// Traditional Chinese → Hong Kong variant (with phrases).
    T2hkp = 19,

    /// Hong Kong variant → Traditional Chinese (with phrases).
    Hk2tp = 20,
}

impl TryFrom<&str> for OpenccConfig {
    /// Converts a configuration name into [`OpenccConfig`].
    ///
    /// This conversion is **case-insensitive** and accepts the canonical
    /// OpenCC configuration identifiers (e.g. `"s2t"`, `"t2s"`).
    ///
    /// Internally this delegates to [`OpenccConfig::parse`].
    ///
    /// # Since
    ///
    /// Available since **v0.8.4**.
    type Error = ();

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::parse(s).ok_or(())
    }
}

impl OpenccConfig {
    /// All supported OpenCC configurations.
    ///
    /// This constant lists every [`OpenccConfig`] variant supported by the
    /// library.
    ///
    /// This table is primarily used internally for:
    ///
    /// - Iteration over all supported configurations
    /// - Case-insensitive parsing via [`OpenccConfig::parse`]
    /// - Validation helpers such as [`OpenccConfig::is_valid_config`]
    ///
    /// Do not use the array index as a numeric configuration id. For FFI or
    /// serialized numeric values, use [`OpenccConfig::to_ffi`] and
    /// [`OpenccConfig::from_ffi`].
    ///
    /// # Since
    ///
    /// Available since **v0.9.1**.
    pub const ALL: [Self; 20] = [
        Self::S2t,
        Self::S2tw,
        Self::S2twp,
        Self::S2hk,
        Self::S2hkp,
        Self::T2s,
        Self::T2tw,
        Self::T2twp,
        Self::T2hk,
        Self::T2hkp,
        Self::Tw2s,
        Self::Tw2sp,
        Self::Tw2t,
        Self::Tw2tp,
        Self::Hk2s,
        Self::Hk2sp,
        Self::Hk2t,
        Self::Hk2tp,
        Self::Jp2t,
        Self::T2jp,
    ];

    /// Converts an FFI numeric config value into [`OpenccConfig`].
    ///
    /// Returns `None` for unknown values. This is the **only** supported way to accept
    /// configuration numbers from C FFI (`uint32_t` / `opencc_config_t`).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use opencc_fmmseg::OpenccConfig;
    ///
    /// assert_eq!(OpenccConfig::from_ffi(1), Some(OpenccConfig::S2t));
    /// assert_eq!(OpenccConfig::from_ffi(999), None);
    /// ```
    /// # Since
    ///
    /// Available since **v0.8.4**.
    #[inline]
    pub const fn from_ffi(v: u32) -> Option<Self> {
        match v {
            1 => Some(Self::S2t),
            2 => Some(Self::S2tw),
            3 => Some(Self::S2twp),
            4 => Some(Self::S2hk),
            5 => Some(Self::T2s),
            6 => Some(Self::T2tw),
            7 => Some(Self::T2twp),
            8 => Some(Self::T2hk),
            9 => Some(Self::Tw2s),
            10 => Some(Self::Tw2sp),
            11 => Some(Self::Tw2t),
            12 => Some(Self::Tw2tp),
            13 => Some(Self::Hk2s),
            14 => Some(Self::Hk2t),
            15 => Some(Self::Jp2t),
            16 => Some(Self::T2jp),
            17 => Some(Self::S2hkp),
            18 => Some(Self::Hk2sp),
            19 => Some(Self::T2hkp),
            20 => Some(Self::Hk2tp),
            _ => None,
        }
    }

    /// Returns the numeric FFI representation of this configuration.
    ///
    /// This converts the strongly-typed [`OpenccConfig`] enum into its
    /// stable `u32` value as used by the C API (`opencc_config_t`).
    ///
    /// The returned value is guaranteed to match the numeric mapping
    /// defined by the enum’s `#[repr(u32)]` layout.
    ///
    /// This method is primarily intended for:
    ///
    /// - Passing configuration values to C FFI layers
    /// - Interoperating with foreign language bindings (C, Python, C#, Java)
    /// - Logging or serialization where the numeric representation is required
    ///
    /// # Examples
    ///
    /// ```rust
    /// use opencc_fmmseg::OpenccConfig;
    ///
    /// let cfg = OpenccConfig::S2t;
    /// assert_eq!(cfg.to_ffi(), 1);
    /// ```
    ///
    /// # See also
    ///
    /// - [`OpenccConfig::from_ffi`] for converting raw FFI values back into
    ///   a validated [`OpenccConfig`].
    ///
    /// # Since
    ///
    /// Available since **v0.9.1**.
    #[inline]
    pub const fn to_ffi(self) -> u32 {
        self as u32
    }

    /// Returns the canonical OpenCC configuration name.
    ///
    /// The returned string matches the standard OpenCC configuration
    /// identifiers (e.g. `"s2t"`, `"t2s"`, `"s2tw"`). These names are used by:
    ///
    /// - CLI tools
    /// - configuration files
    /// - legacy OpenCC string-based APIs
    ///
    /// This method does **not allocate** and always returns a static string.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use opencc_fmmseg::OpenccConfig;
    ///
    /// let cfg = OpenccConfig::T2jp;
    /// assert_eq!(cfg.as_str(), "t2jp");
    /// ```
    ///
    /// # Typical usage
    ///
    /// ```rust
    /// use opencc_fmmseg::OpenccConfig;
    ///
    /// let cfg = OpenccConfig::S2tw;
    /// println!("Using config {}", cfg.as_str());
    /// ```
    ///
    /// # See also
    ///
    /// - [`TryFrom<&str>`](TryFrom) implementation for parsing
    ///   configuration names into [`OpenccConfig`].
    ///
    /// # Since
    ///
    /// Available since **v0.9.1**.
    #[inline]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::S2t => "s2t",
            Self::S2tw => "s2tw",
            Self::S2twp => "s2twp",
            Self::S2hk => "s2hk",
            Self::S2hkp => "s2hkp",
            Self::T2s => "t2s",
            Self::T2tw => "t2tw",
            Self::T2twp => "t2twp",
            Self::T2hk => "t2hk",
            Self::T2hkp => "t2hkp",
            Self::Tw2s => "tw2s",
            Self::Tw2sp => "tw2sp",
            Self::Tw2t => "tw2t",
            Self::Tw2tp => "tw2tp",
            Self::Hk2s => "hk2s",
            Self::Hk2sp => "hk2sp",
            Self::Hk2t => "hk2t",
            Self::Hk2tp => "hk2tp",
            Self::Jp2t => "jp2t",
            Self::T2jp => "t2jp",
        }
    }

    /// Parses a configuration name into an [`OpenccConfig`].
    ///
    /// The input string is matched against the canonical OpenCC configuration
    /// identifiers (e.g. `"s2t"`, `"t2s"`, `"s2tw"`). Matching is
    /// **case-insensitive**.
    ///
    /// If the input does not correspond to a supported configuration,
    /// `None` is returned.
    ///
    /// This method provides the internal parsing logic used by the
    /// [`TryFrom<&str>`] implementation and higher-level helpers such as
    /// [`OpenccConfig::is_valid_config`].
    ///
    /// The implementation performs a lightweight linear search over
    /// [`OpenccConfig::ALL`], which is efficient given the small number
    /// of supported configurations.
    ///
    /// # See also
    ///
    /// - [`TryFrom<&str>`] for ergonomic conversion using `Result`
    /// - [`OpenccConfig::as_str`] for retrieving the canonical name
    ///
    /// # Since
    ///
    /// Available since **v0.9.1**.
    #[inline]
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|cfg| cfg.as_str().eq_ignore_ascii_case(s))
    }

    /// Returns `true` if the given string is a supported OpenCC configuration name.
    ///
    /// This is a lightweight validation helper intended for:
    /// - CLI argument checking
    /// - UI / config file validation
    /// - Preflight checks before calling conversion APIs
    ///
    /// The check is case-insensitive and does **not** allocate on success paths
    /// beyond the internal normalization already used by `TryFrom<&str>`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use opencc_fmmseg::OpenccConfig;
    ///
    /// assert!(OpenccConfig::is_valid_config("s2t"));
    /// assert!(OpenccConfig::is_valid_config("T2JP"));
    /// assert!(!OpenccConfig::is_valid_config("invalid"));
    /// ```
    ///
    /// # Since
    ///
    /// Available since **v0.8.5**.
    #[inline]
    pub fn is_valid_config(s: &str) -> bool {
        Self::parse(s).is_some()
    }

    /// Returns `true` if the given numeric value corresponds to a valid FFI config.
    ///
    /// This is intended for validating raw `opencc_config_t` values coming from
    /// foreign languages **before** attempting conversion.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use opencc_fmmseg::OpenccConfig;
    ///
    /// assert!(OpenccConfig::is_valid_config_ffi(1));
    /// assert!(!OpenccConfig::is_valid_config_ffi(999));
    /// ```
    ///
    /// # Since
    ///
    /// Available since **v0.8.5**.
    #[inline]
    pub fn is_valid_config_ffi(v: u32) -> bool {
        Self::from_ffi(v).is_some()
    }
}
