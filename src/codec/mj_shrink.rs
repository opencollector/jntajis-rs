//! MJ shrink conversion functionality.
//!
//! This module provides the core MJ (Moji Joho) character shrink conversion functionality,
//! allowing transliteration of complex character variants to commonly-used forms.
//!
//! # Examples
//!
//! Basic usage with all shrink schemes:
//!
//! ```rust
//! use jntajis::codec::mj_shrink::{MJShrinkSchemes, mj_shrink_candidates};
//!
//! let candidates: Vec<String> = mj_shrink_candidates("髙", MJShrinkSchemes::ALL)
//!     .take(5)
//!     .collect();
//! // Returns variants like ["高", "髙"]
//! ```
//!
//! Using specific shrink schemes:
//!
//! ```rust
//! use jntajis::codec::mj_shrink::{MJShrinkScheme, MJShrinkSchemes, mj_shrink_candidates};
//!
//! let jis_only = MJShrinkSchemes::builder()
//!     .with(MJShrinkScheme::JISIncorporationUCSUnificationRule);
//! let candidates: Vec<String> = mj_shrink_candidates("髙", jis_only)
//!     .take(5)
//!     .collect();
//! ```
//!
//! # MJ Shrink Schemes
//!
//! The library supports four different MJ shrink schemes:
//!
//! - [`MJShrinkScheme::JISIncorporationUCSUnificationRule`] - JIS incorporation and UCS unification rules
//! - [`MJShrinkScheme::InferenceByReadingAndGlyph`] - Inference by reading and glyph rules
//! - [`MJShrinkScheme::MOJNotice582`] - MOJ Notice 582 transliteration rules  
//! - [`MJShrinkScheme::MOJFamilyRegisterActRelatedNotice`] - Family register act related notice rules

use std::sync::Arc;

use super::common_models::{Ivs, UIVSPair};
use super::get_data;
use super::inmemory_models::{ConversionData, MJMapping};

// Re-export the types for public use
pub use super::common_models::{MJShrinkScheme, MJShrinkSchemes};

/// MJ shrink candidates generator.
///
/// This struct is used internally to generate shrink candidates for input characters
/// based on the specified MJ shrink schemes.
pub struct MJShrinkCandidates {
    data: Arc<ConversionData>,
    schemes: MJShrinkSchemes,
}

impl MJShrinkCandidates {
    fn lookup_mj_mapping_table(data: &ConversionData, u: u32) -> Option<&[&MJMapping]> {
        data.lookup_mj_mapping(u)
    }

    fn new(data: Arc<ConversionData>, schemes: MJShrinkSchemes) -> Self {
        Self { data, schemes }
    }

    fn yield_candidates(&self, input: impl AsRef<str>) -> Vec<Vec<UIVSPair>> {
        let mut candidates = Vec::new();
        let chars: Vec<char> = input.as_ref().chars().collect();
        let mut i = 0;

        while i < chars.len() {
            let mut local_candidates: Vec<UIVSPair> = Vec::with_capacity(20);
            let u = chars[i] as u32;
            i += 1;

            let iv = if i < chars.len() {
                let next_char = chars[i] as u32;
                match Ivs::try_from(next_char) {
                    Ok(ivs) => {
                        i += 1;
                        Some(ivs)
                    }
                    Err(_) => None,
                }
            } else {
                None
            };

            let mut collected_mappings: Vec<&MJMapping> = Vec::new();

            if let Some(ms) = Self::lookup_mj_mapping_table(&self.data, u) {
                if let Some(ivs) = iv {
                    // Expecting exact match
                    for &mm in ms {
                        for uivs in mm.v() {
                            if uivs.u == u && uivs.s == Some(ivs) {
                                collected_mappings.push(mm);
                                break;
                            }
                        }
                    }
                } else {
                    // Search for all candidates without IVS
                    for &mm in ms {
                        for uivs in mm.v() {
                            if uivs.u == u && uivs.s.is_none() {
                                collected_mappings.push(mm);
                                break;
                            }
                        }
                    }
                }
            }

            // Process collected mappings
            for mm in &collected_mappings {
                if let Some(sm) = self.data.lookup_mj_shrink_mapping(mm.mj())
                    && sm.us.is_valid()
                {
                    if self
                        .schemes
                        .contains(MJShrinkScheme::JISIncorporationUCSUnificationRule)
                    {
                        for &uu in sm.us.jis_incorporation_ucs_unification_rule() {
                            if uu == u && iv.is_none() {
                                break;
                            }
                            if !local_candidates.iter().any(|c| c.u == uu && c.s.is_none()) {
                                local_candidates.push(UIVSPair { u: uu, s: None });
                            }
                        }
                    }
                    if self
                        .schemes
                        .contains(MJShrinkScheme::InferenceByReadingAndGlyph)
                    {
                        for &uu in sm.us.inference_by_reading_and_glyph() {
                            if uu == u && iv.is_none() {
                                break;
                            }
                            if !local_candidates.iter().any(|c| c.u == uu && c.s.is_none()) {
                                local_candidates.push(UIVSPair { u: uu, s: None });
                            }
                        }
                    }
                    if self.schemes.contains(MJShrinkScheme::MOJNotice582) {
                        for &uu in sm.us.moj_notice_582() {
                            if uu == u && iv.is_none() {
                                break;
                            }
                            if !local_candidates.iter().any(|c| c.u == uu && c.s.is_none()) {
                                local_candidates.push(UIVSPair { u: uu, s: None });
                            }
                        }
                    }
                    if self
                        .schemes
                        .contains(MJShrinkScheme::MOJFamilyRegisterActRelatedNotice)
                    {
                        for &uu in sm.us.moj_family_register_act_related_notice() {
                            if uu == u && iv.is_none() {
                                break;
                            }
                            if !local_candidates.iter().any(|c| c.u == uu && c.s.is_none()) {
                                local_candidates.push(UIVSPair { u: uu, s: None });
                            }
                        }
                    }
                }
            }

            // Add unicode variants from mappings
            for mm in &collected_mappings {
                for uivs in mm.v() {
                    if uivs.s.is_none()
                        && !local_candidates
                            .iter()
                            .any(|c| c.u == uivs.u && c.s.is_none())
                    {
                        local_candidates.push(UIVSPair { u: uivs.u, s: None });
                    }
                }
            }

            // If no candidates found, add the original character
            if local_candidates.is_empty() {
                local_candidates.push(UIVSPair { u, s: iv });
            }
            candidates.push(local_candidates);
        }
        candidates
    }
}

/// Iterator that yields MJ shrink candidate strings.
///
/// This iterator generates all possible combinations of character variants
/// based on the cartesian product of candidates for each input character.
pub struct MJShrinkCandidatesIterator {
    candidates: Vec<Vec<UIVSPair>>,
    indices: Vec<usize>,
    eoi: bool,
}

impl MJShrinkCandidatesIterator {
    fn new(candidates: Vec<Vec<UIVSPair>>) -> Self {
        let indices = vec![0; candidates.len()];
        Self {
            candidates,
            indices,
            eoi: false,
        }
    }
}

impl Iterator for MJShrinkCandidatesIterator {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        if self.eoi {
            return None;
        }
        let mut candidate = String::new();
        for (i, cands) in self.candidates.iter().enumerate() {
            let c = &cands[self.indices[i]];
            if let Some(ch) = char::from_u32(c.u) {
                candidate.push(ch);
            }
            if let Some(ivs) = c.s
                && let Some(ch) = char::from_u32(ivs.into())
            {
                candidate.push(ch);
            }
        }

        // Increment indices
        let mut carry = true;
        for i in 0..self.indices.len() {
            if self.indices[i] < self.candidates[i].len() - 1 {
                self.indices[i] += 1;
                carry = false;
                break;
            } else {
                self.indices[i] = 0;
            }
        }

        if carry {
            self.eoi = true
        }
        Some(candidate)
    }
}

/// Generate MJ shrink candidates for the given input string.
///
/// This function returns an iterator that yields all possible transliteration candidates
/// for the input string based on the specified MJ shrink schemes.
///
/// # Arguments
///
/// * `input` - The input string to transliterate
/// * `schemes` - The MJ shrink schemes to apply (combination of [`MJShrinkScheme`] values)
///
/// # Returns
///
/// An iterator that yields candidate strings. The iterator generates the cartesian product
/// of all possible variants for each character in the input.
///
/// # Examples
///
/// ```rust
/// use jntajis::codec::mj_shrink::{MJShrinkSchemes, mj_shrink_candidates};
///
/// // Get all possible shrink candidates
/// let candidates: Vec<String> = mj_shrink_candidates("髙島屋", MJShrinkSchemes::ALL)
///     .take(10)
///     .collect();
///
/// // Should include variants like "高島屋"
/// assert!(candidates.iter().any(|s| s == "高島屋"));
/// ```
///
/// # Character Mapping Process
///
/// The transliteration is done in two phases:
///
/// 1. **Unicode to MJ character mappings**: Unicode codepoints are converted to MJ codes
/// 2. **MJ shrink mappings**: MJ codes are transliterated according to the specified schemes
///
/// Multiple candidates may be returned because:
/// - A Unicode codepoint may map to multiple MJ codes
/// - Multiple transliteration schemes may be designated to a single MJ code
pub fn mj_shrink_candidates(
    input: impl AsRef<str>,
    schemes: MJShrinkSchemes,
) -> impl Iterator<Item = String> {
    MJShrinkCandidatesIterator::new(
        MJShrinkCandidates::new(get_data(), schemes).yield_candidates(input),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mj_shrink_candidates_basic() {
        // Test with the character '髙' (U+9AD9)
        let results: Vec<String> = mj_shrink_candidates("髙", MJShrinkSchemes::ALL).collect();

        // Should return some candidates
        assert!(!results.is_empty());

        // Check if '高' (U+9AD8) is among the candidates
        assert!(results.iter().any(|s| s.contains('高')));
    }

    #[test]
    fn test_mj_shrink_candidates_multiple_chars() {
        // Test with multiple characters
        let results: Vec<String> = mj_shrink_candidates("髙橋", MJShrinkSchemes::ALL).collect();

        // Should return multiple candidates
        assert!(!results.is_empty());
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_mj_shrink_kiri_kou_scheme_separation() {
        // 切 (U+5207) / 功 (U+529F) pair: should only appear together under scheme 4
        // (MOJNotice582), not under scheme 8 (MOJFamilyRegisterActRelatedNotice).
        let scheme4 = MJShrinkSchemes::builder().with(MJShrinkScheme::MOJNotice582);
        let scheme8 =
            MJShrinkSchemes::builder().with(MJShrinkScheme::MOJFamilyRegisterActRelatedNotice);

        let kiri_scheme4: Vec<String> = mj_shrink_candidates("切", scheme4).collect();
        let kiri_scheme8: Vec<String> = mj_shrink_candidates("切", scheme8).collect();
        let kou_scheme8: Vec<String> = mj_shrink_candidates("功", scheme8).collect();

        // Under scheme 4, 切 should map to both 切 and 功
        assert!(
            kiri_scheme4.iter().any(|s| s == "功"),
            "切 should include 功 under scheme 4, got: {kiri_scheme4:?}"
        );
        // Under scheme 8, 切 should only return itself (no 功)
        assert!(
            !kiri_scheme8.iter().any(|s| s == "功"),
            "切 should not include 功 under scheme 8, got: {kiri_scheme8:?}"
        );
        assert!(
            kiri_scheme8.iter().any(|s| s == "切"),
            "切 should include itself under scheme 8, got: {kiri_scheme8:?}"
        );
        // Under scheme 8, 功 should include 切 (the reverse mapping)
        assert!(
            kou_scheme8.iter().any(|s| s == "切"),
            "功 should include 切 under scheme 8, got: {kou_scheme8:?}"
        );
    }

    #[test]
    fn test_mj_shrink_candidates_individual_schemes() {
        // Test different individual schemes
        let jis_only =
            MJShrinkSchemes::builder().with(MJShrinkScheme::JISIncorporationUCSUnificationRule);
        let inference_only =
            MJShrinkSchemes::builder().with(MJShrinkScheme::InferenceByReadingAndGlyph);
        let moj582_only = MJShrinkSchemes::builder().with(MJShrinkScheme::MOJNotice582);
        let moj_family_only =
            MJShrinkSchemes::builder().with(MJShrinkScheme::MOJFamilyRegisterActRelatedNotice);

        let results1: Vec<String> = mj_shrink_candidates("髙", jis_only).take(10).collect();
        let results2: Vec<String> = mj_shrink_candidates("髙", inference_only)
            .take(10)
            .collect();
        let results3: Vec<String> = mj_shrink_candidates("髙", moj582_only).take(10).collect();
        let results4: Vec<String> = mj_shrink_candidates("髙", moj_family_only)
            .take(10)
            .collect();

        // All should return some results
        assert!(!results1.is_empty());
        assert!(!results2.is_empty());
        assert!(!results3.is_empty());
        assert!(!results4.is_empty());
    }
}
