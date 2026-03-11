use serde::{Deserialize, Serialize};

use crate::array_vec::ArrayVec;
use crate::array_vec::invalid_value::{AllBitsSetValueAsInvalid, ValueValidity};

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Deserialize,
    Serialize,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
/// Classification of characters in the JIS character set.
pub enum JISCharacterClass {
    Reserved = 0,
    KanjiLevel1 = 1,
    KanjiLevel2 = 2,
    KanjiLevel3 = 3,
    KanjiLevel4 = 4,
    JISX0208NonKanji = 9,
    JISX0213NonKanji = 11,
}

impl JISCharacterClass {
    /// Returns `true` if this is a kanji character (level 1-4).
    pub fn is_kanji(&self) -> bool {
        matches!(
            self,
            JISCharacterClass::KanjiLevel1
                | JISCharacterClass::KanjiLevel2
                | JISCharacterClass::KanjiLevel3
                | JISCharacterClass::KanjiLevel4
        )
    }

    /// Returns `true` if this character is defined in JIS X 0208.
    pub fn is_jisx0208(&self) -> bool {
        matches!(
            self,
            JISCharacterClass::KanjiLevel1
                | JISCharacterClass::KanjiLevel2
                | JISCharacterClass::JISX0208NonKanji
        )
    }

    /// Returns `true` if this character is specific to JIS X 0213 (level 3/4 or 0213 non-kanji).
    pub fn is_jisx0213(&self) -> bool {
        matches!(
            self,
            JISCharacterClass::JISX0213NonKanji
                | JISCharacterClass::KanjiLevel3
                | JISCharacterClass::KanjiLevel4
        )
    }
}

/// MJ shrink transliteration schemes.
///
/// This enum defines the four different transliteration schemes available
/// in the MJ shrink conversion system.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Deserialize,
    Serialize,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub enum MJShrinkScheme {
    /// JIS incorporation and UCS unification rule (JIS包摂規準・UCS統合規則).
    ///
    /// Transliterates characters according to JIS incorporation and UCS unification rules.
    JISIncorporationUCSUnificationRule = 1, // bit 0

    /// Inference by reading and glyph (読み・字形による類推).
    ///
    /// Transliterates characters according to CITPC-defined rules based on analogy
    /// from readings and glyphs of characters.
    InferenceByReadingAndGlyph = 2, // bit 1

    /// MOJ Notice 582 (法務省告示582号別表第四).
    ///
    /// Transliterates characters according to the appendix table proposed in
    /// Japan Ministry of Justice (MOJ) notice no. 582.
    MOJNotice582 = 4, // bit 2

    /// MOJ Family Register Act related notice (法務省戸籍法関連通達・通知).
    ///
    /// Transliterates characters according to the Family Register Act (戸籍法)
    /// and related MOJ notices.
    MOJFamilyRegisterActRelatedNotice = 8, // bit 3
}

/// A combination of MJ shrink schemes.
///
/// This struct allows combining multiple [`MJShrinkScheme`] values using bitwise operations.
///
/// # Examples
///
/// ```rust
/// use jntajis::codec::mj_shrink::{MJShrinkScheme, MJShrinkSchemes};
///
/// // Use all available schemes
/// let all_schemes = MJShrinkSchemes::ALL;
///
/// // Build custom combinations
/// let combined = MJShrinkSchemes::builder()
///     .with(MJShrinkScheme::JISIncorporationUCSUnificationRule)
///     .with(MJShrinkScheme::MOJNotice582);
///
/// // Check if a specific scheme is included
/// assert!(combined.contains(MJShrinkScheme::JISIncorporationUCSUnificationRule));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MJShrinkSchemes(u8);

impl MJShrinkSchemes {
    /// No schemes selected.
    pub const NONE: Self = Self(0);

    /// All available schemes selected.
    pub const ALL: Self = Self(0x0F);

    /// Create a new builder for combining schemes.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use jntajis::codec::mj_shrink::{MJShrinkScheme, MJShrinkSchemes};
    ///
    /// let schemes = MJShrinkSchemes::builder()
    ///     .with(MJShrinkScheme::JISIncorporationUCSUnificationRule)
    ///     .with(MJShrinkScheme::MOJNotice582);
    /// ```
    pub const fn builder() -> Self {
        Self(0)
    }

    /// Create from raw bits.
    pub const fn from_bits(bits: u8) -> Self {
        Self(bits)
    }

    /// Get the raw bits representation.
    pub const fn bits(self) -> u8 {
        self.0
    }

    /// Add a scheme to this combination.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use jntajis::codec::mj_shrink::{MJShrinkScheme, MJShrinkSchemes};
    ///
    /// let schemes = MJShrinkSchemes::NONE
    ///     .with(MJShrinkScheme::JISIncorporationUCSUnificationRule);
    /// ```
    pub const fn with(self, scheme: MJShrinkScheme) -> Self {
        Self(self.0 | scheme as u8)
    }

    /// Check if this combination contains the specified scheme.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use jntajis::codec::mj_shrink::{MJShrinkScheme, MJShrinkSchemes};
    ///
    /// let schemes = MJShrinkSchemes::ALL;
    /// assert!(schemes.contains(MJShrinkScheme::JISIncorporationUCSUnificationRule));
    /// ```
    pub const fn contains(self, scheme: MJShrinkScheme) -> bool {
        (self.0 & scheme as u8) != 0
    }

    /// Check if no schemes are selected.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl From<u8> for MJShrinkSchemes {
    fn from(bits: u8) -> Self {
        Self(bits)
    }
}

impl From<MJShrinkSchemes> for u8 {
    fn from(schemes: MJShrinkSchemes) -> Self {
        schemes.0
    }
}

/// Error returned when constructing a [`MenKuTen`] with invalid values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenKuTenError(String);

impl std::fmt::Display for MenKuTenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for MenKuTenError {
    fn from(e: String) -> Self {
        Self(e)
    }
}

impl std::error::Error for MenKuTenError {}

#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Deserialize,
    Serialize,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
/// A packed men-ku-ten (面-区-点) code identifying a character in JIS X 0208/0213.
///
/// The code is packed into a `u16` as `(men-1)*94*94 + (ku-1)*94 + (ten-1)`.
pub struct MenKuTen(pub u16);

impl MenKuTen {
    /// Sentinel value representing an invalid/absent men-ku-ten code.
    pub const INVALID: MenKuTen = MenKuTen(u16::MAX);

    /// Creates a new `MenKuTen` from men (1-2), ku (1-94), and ten (1-94).
    pub fn new(men: u8, ku: u8, ten: u8) -> Result<Self, MenKuTenError> {
        if !(1..=2).contains(&men) {
            return Err(MenKuTenError(format!("invalid men value: {}", men)));
        }
        if !(1..=94).contains(&ku) {
            return Err(MenKuTenError(format!("invalid ku value: {}", ku)));
        }
        if !(1..=94).contains(&ten) {
            return Err(MenKuTenError(format!("invalid ten value: {}", ten)));
        }
        Ok(Self(
            ((men as u16) - 1) * 94 * 94 + (ku as u16 - 1) * 94 + (ten as u16 - 1),
        ))
    }

    /// Returns the men (plane) number (1 or 2).
    pub fn men(&self) -> u8 {
        (self.0 / (94 * 94) + 1) as u8
    }

    /// Returns the ku (row) number (1-94).
    pub fn ku(&self) -> u8 {
        (self.0 / 94 % 94 + 1) as u8
    }

    /// Returns the ten (cell) number (1-94).
    pub fn ten(&self) -> u8 {
        (self.0 % 94 + 1) as u8
    }
}

impl std::ops::Add<u16> for MenKuTen {
    type Output = Self;

    fn add(self, other: u16) -> Self::Output {
        Self(self.0 + other)
    }
}

impl From<u16> for MenKuTen {
    fn from(value: u16) -> Self {
        Self(value)
    }
}

impl From<MenKuTen> for u16 {
    fn from(value: MenKuTen) -> Self {
        value.0
    }
}

impl std::fmt::Display for MenKuTen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}-{:02}-{:02}",
            (self.0 / (94 * 94) + 1),
            (self.0 / 94 % 94 + 1),
            (self.0 % 94 + 1)
        )
    }
}

impl std::fmt::Debug for MenKuTen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <Self as std::fmt::Display>::fmt(self, f)
    }
}

impl ValueValidity for MenKuTen {
    type Target = Self;

    fn invalid_value() -> Self::Target {
        Self::INVALID
    }

    fn is_valid(value: &Self::Target) -> bool {
        *value != Self::invalid_value()
    }
}

#[derive(
    Clone, Debug, Deserialize, Serialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize,
)]
/// A JNTA (Japan National Tax Agency) character mapping entry.
pub struct JNTAMapping {
    /// Packed men-ku-ten code
    pub jis: MenKuTen,
    /// Corresponding Unicode character
    pub us: ArrayVec<u32, 2, AllBitsSetValueAsInvalid<u32>>,
    /// Corresponding Unicode character (secondary)
    pub sus: ArrayVec<u32, 2, AllBitsSetValueAsInvalid<u32>>,
    /// JIS character class
    pub class: JISCharacterClass,
    /// Transliterated form in men-ku-ten code
    pub tx_jis: ArrayVec<MenKuTen, 4>,
    /// Transliterated form in Unicode characters
    pub tx_us: ArrayVec<u32, 4, AllBitsSetValueAsInvalid<u32>>,
}

/// Error returned when a codepoint is not a valid Ideographic Variation Selector.
#[derive(Clone, Debug)]
pub struct IVSConversionError(u32);

impl std::fmt::Display for IVSConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "U+{:06x} is not an IVS", self.0)
    }
}

impl std::error::Error for IVSConversionError {}

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Deserialize,
    Serialize,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
/// An Ideographic Variation Selector (IVS) index.
///
/// Values 0-15 correspond to VS1-VS16 (U+FE00..U+FE0F), and values 16-255
/// correspond to VS17-VS256 (U+E0100..U+E01EF).
pub struct Ivs(u8);

impl Ivs {
    /// Creates a new `Ivs` from a raw index value.
    pub fn new(ivs: u8) -> Self {
        Self(ivs)
    }
}

impl TryFrom<u32> for Ivs {
    type Error = IVSConversionError;

    fn try_from(c: u32) -> Result<Self, Self::Error> {
        // VS1 to VS16
        if (0xFE00..0xFE10).contains(&c) {
            return Ok(Self((c - 0xFE00) as u8));
        }
        // VS17 to VS256
        if (0xE0100..0xE01F0).contains(&c) {
            return Ok(Self((c - 0xE00F0) as u8));
        }
        Err(IVSConversionError(c))
    }
}

impl From<Ivs> for u32 {
    fn from(ivs: Ivs) -> Self {
        // VS1 to VS16
        if ivs.0 < 0x10 {
            return 0xFE00 + ivs.0 as u32;
        }
        // VS17 to VS256
        0xE00F0 + ivs.0 as u32
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Deserialize,
    Serialize,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
/// An MJ (Moji Joho) character code.
pub struct MJCode(u32);

impl MJCode {
    /// Sentinel value representing an invalid/absent MJ code.
    pub const INVALID: MJCode = MJCode(u32::MAX);

    /// Creates a new `MJCode` from a raw numeric value.
    pub fn new(mj: u32) -> Self {
        Self(mj)
    }
}

impl std::ops::Add<u32> for MJCode {
    type Output = Self;

    fn add(self, other: u32) -> Self::Output {
        Self(self.0 + other)
    }
}

impl std::ops::Sub<u32> for MJCode {
    type Output = MJCode;

    fn sub(self, other: u32) -> Self::Output {
        Self(self.0 - other)
    }
}

impl std::ops::Sub<MJCode> for MJCode {
    type Output = u32;

    fn sub(self, other: MJCode) -> Self::Output {
        self.0 - other.0
    }
}

impl From<u32> for MJCode {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<MJCode> for u32 {
    fn from(value: MJCode) -> Self {
        value.0
    }
}

impl ValueValidity for MJCode {
    type Target = Self;

    fn invalid_value() -> Self::Target {
        Self::INVALID
    }

    fn is_valid(value: &Self::Target) -> bool {
        *value != Self::invalid_value()
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Deserialize,
    Serialize,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
/// A Unicode codepoint paired with an optional Ideographic Variation Selector.
pub struct UIVSPair {
    /// Unicode codepoint.
    pub u: u32,
    /// Optional IVS that disambiguates glyph variants.
    pub s: Option<Ivs>,
}

impl ValueValidity for UIVSPair {
    type Target = Self;

    fn invalid_value() -> Self {
        UIVSPair {
            u: u32::MAX,
            s: None,
        }
    }

    fn is_valid(value: &Self::Target) -> bool {
        value.u != u32::MAX
    }
}

#[cfg(feature = "codegen")]
impl MenKuTen {
    /// Parses a men-ku-ten string in `"M-KK-TT"` format.
    pub fn from_repr(v: impl AsRef<str>) -> Result<Self, MenKuTenError> {
        use lazy_static::lazy_static;

        lazy_static! {
            static ref REGEXP_MEN_KU_TEN: regex::Regex =
                regex::Regex::new(r"^(\d+)-(\d+)-(\d+)$").expect("should never happen");
        }

        let v = v.as_ref();
        match REGEXP_MEN_KU_TEN.captures(v) {
            Some(c) => Ok(Self::new(
                c.get(1)
                    .unwrap()
                    .as_str()
                    .parse::<u8>()
                    .map_err(|e| format!("failed to parse men: {}", e))?,
                c.get(2)
                    .unwrap()
                    .as_str()
                    .parse::<u8>()
                    .map_err(|e| format!("failed to parse ku: {}", e))?,
                c.get(3)
                    .unwrap()
                    .as_str()
                    .parse::<u8>()
                    .map_err(|e| format!("failed to parse ten: {}", e))?,
            )?),
            None => Err(MenKuTenError(format!("invalid men-ku-ten string: {}", v))),
        }
    }
}

/// Error returned when parsing an MJ code string representation fails.
#[derive(Debug)]
#[cfg(feature = "codegen")]
pub struct MJCodeParseError(String);

#[cfg(feature = "codegen")]
impl std::fmt::Display for MJCodeParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(feature = "codegen")]
impl std::error::Error for MJCodeParseError {}

#[cfg(feature = "codegen")]
impl MJCode {
    /// Parses an MJ code string in `"MJ######"` format.
    pub fn from_repr(v: impl AsRef<str>) -> Result<Self, MJCodeParseError> {
        use lazy_static::lazy_static;

        lazy_static! {
            static ref REGEXP_MJ_CODE: regex::Regex =
                regex::Regex::new(r"^MJ([0-9]+)$").expect("should never happen");
        }

        let v = v.as_ref();
        match REGEXP_MJ_CODE.captures(v) {
            Some(c) => {
                Ok(Self(c.get(1).unwrap().as_str().parse::<u32>().map_err(
                    |e| MJCodeParseError(format!("invalid MJ repr: {}", e)),
                )?))
            }
            None => Err(MJCodeParseError(format!(
                "invalid MJ code representation: {}",
                v
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ivs_try_from() {
        // Test VS1 to VS16
        assert!(Ivs::try_from(0xfe00).is_ok());
        assert_eq!(Ivs::try_from(0xfe00).unwrap(), Ivs::new(0));
        assert!(Ivs::try_from(0xfe0f).is_ok());
        assert_eq!(Ivs::try_from(0xfe0f).unwrap(), Ivs::new(15));

        // Test VS17 to VS256
        assert!(Ivs::try_from(0xe0100).is_ok());
        assert_eq!(Ivs::try_from(0xe0100).unwrap(), Ivs::new(16));
        assert!(Ivs::try_from(0xe01ef).is_ok());
        assert_eq!(Ivs::try_from(0xe01ef).unwrap(), Ivs::new(255));

        // Test invalid IVS
        assert!(Ivs::try_from(0x1234).is_err());
        assert!(Ivs::try_from(0xfe10).is_err()); // Just outside VS1-VS16 range
        assert!(Ivs::try_from(0xe01f0).is_err()); // Just outside VS17-VS256 range
    }

    #[test]
    fn test_ivs_into_u32() {
        // Test VS1 to VS16
        assert_eq!(u32::from(Ivs::new(0)), 0xfe00);
        assert_eq!(u32::from(Ivs::new(15)), 0xfe0f);

        // Test VS17 to VS256
        assert_eq!(u32::from(Ivs::new(16)), 0xe0100);
        assert_eq!(u32::from(Ivs::new(255)), 0xe01ef);
    }

    #[test]
    fn test_ivs_roundtrip() {
        // Test that converting to and from u32 roundtrips for all valid IVS values
        for i in 0u8..=255 {
            let ivs = Ivs::new(i);
            let u = u32::from(ivs);
            assert_eq!(
                Ivs::try_from(u).unwrap(),
                ivs,
                "roundtrip failed for Ivs({})",
                i
            );
        }
    }

    #[test]
    fn test_mj_shrink_schemes_api() {
        // Test the MJShrinkSchemes API
        let schemes = MJShrinkSchemes::builder()
            .with(MJShrinkScheme::JISIncorporationUCSUnificationRule)
            .with(MJShrinkScheme::InferenceByReadingAndGlyph);

        assert!(schemes.contains(MJShrinkScheme::JISIncorporationUCSUnificationRule));
        assert!(schemes.contains(MJShrinkScheme::InferenceByReadingAndGlyph));
        assert!(!schemes.contains(MJShrinkScheme::MOJNotice582));
        assert!(!schemes.contains(MJShrinkScheme::MOJFamilyRegisterActRelatedNotice));

        assert_eq!(schemes.bits(), 0b0011);

        // Test ALL schemes
        assert!(MJShrinkSchemes::ALL.contains(MJShrinkScheme::JISIncorporationUCSUnificationRule));
        assert!(MJShrinkSchemes::ALL.contains(MJShrinkScheme::InferenceByReadingAndGlyph));
        assert!(MJShrinkSchemes::ALL.contains(MJShrinkScheme::MOJNotice582));
        assert!(MJShrinkSchemes::ALL.contains(MJShrinkScheme::MOJFamilyRegisterActRelatedNotice));

        // Test NONE schemes
        assert!(MJShrinkSchemes::NONE.is_empty());
        assert!(
            !MJShrinkSchemes::NONE.contains(MJShrinkScheme::JISIncorporationUCSUnificationRule)
        );

        // Test conversion from u8
        let from_bits = MJShrinkSchemes::from_bits(0b1010);
        assert!(from_bits.contains(MJShrinkScheme::InferenceByReadingAndGlyph));
        assert!(from_bits.contains(MJShrinkScheme::MOJFamilyRegisterActRelatedNotice));
        assert!(!from_bits.contains(MJShrinkScheme::JISIncorporationUCSUnificationRule));
        assert!(!from_bits.contains(MJShrinkScheme::MOJNotice582));

        // Test From trait
        let from_u8: MJShrinkSchemes = 5u8.into(); // 0b0101
        assert!(from_u8.contains(MJShrinkScheme::JISIncorporationUCSUnificationRule));
        assert!(from_u8.contains(MJShrinkScheme::MOJNotice582));
        assert!(!from_u8.contains(MJShrinkScheme::InferenceByReadingAndGlyph));
        assert!(!from_u8.contains(MJShrinkScheme::MOJFamilyRegisterActRelatedNotice));

        // Test Into trait
        let back_to_u8: u8 = from_u8.into();
        assert_eq!(back_to_u8, 5);
    }
}
