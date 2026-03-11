//! Serializable (on-wire) models for the conversion data.
//!
//! These types are serialized with `rkyv` and stored in `generated.bin`.
//! They are converted to in-memory models at load time.

use std::ops::Range;

use serde::{Deserialize, Serialize};

use super::common_models::{JNTAMapping, MJCode, MenKuTen, UIVSPair};

/// Serialized Unicode-range-to-JIS mapping entry.
#[derive(
    Clone, Debug, Deserialize, Serialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub struct URangeToJISMapping {
    pub start: u32,
    pub jis: u16,
}

#[derive(
    Clone, Debug, Deserialize, Serialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize,
)]
/// Serialized MJ character mapping entry.
pub struct MJMapping {
    /// MJ code identifier.
    pub mj: MJCode,
    /// Index into the UIVS pool.
    pub v: u32,
}

/// Serialized Unicode-range-to-MJ mapping entry.
#[derive(
    Clone, Debug, Deserialize, Serialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub struct URangeToMJMappings {
    pub start: u32,
    pub mss: u32,
}

#[derive(
    Clone, Debug, Deserialize, Serialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize,
)]
/// Serialized MJ shrink mapping Unicode candidate ranges, one per scheme.
pub struct MJShrinkMappingUnicodeSet {
    pub jis_incorporation_ucs_unification_rule: Range<u32>,
    pub inference_by_reading_and_glyph: Range<u32>,
    pub moj_notice_582: Range<u32>,
    pub moj_family_register_act_related_notice: Range<u32>,
}

#[derive(
    Clone, Debug, Deserialize, Serialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize,
)]
/// Serialized MJ shrink mapping entry.
pub struct MJShrinkMapping {
    /// MJ code identifier.
    pub mj: MJCode,
    /// Shrink candidate Unicode sets.
    pub us: MJShrinkMappingUnicodeSet,
}

/// The complete serialized conversion dataset.
#[derive(
    Clone, Debug, Deserialize, Serialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub struct ConversionData {
    pub jnta_mappings: Vec<JNTAMapping>,
    pub mj_mappings: Vec<MJMapping>,
    pub uni_pool: Vec<u32>,
    pub jis_pool: Vec<MenKuTen>,
    pub uivs_pool: Vec<UIVSPair>,
    pub mj_pool: Vec<MJCode>,
    pub mj_range_pool: Vec<u32>,
    pub mj_shrink_mappings: Vec<MJShrinkMapping>,
    pub urange_to_jis_mappings: Vec<URangeToJISMapping>,
    pub urange_to_mj_mappings: Vec<URangeToMJMappings>,
}

#[cfg(feature = "codegen")]
impl ConversionData {
    /// Appends Unicode codepoints to the pool and returns their index range.
    pub fn add_uni_pool(&mut self, us: impl IntoIterator<Item = u32>) -> Range<u32> {
        let s = u32::try_from(self.uni_pool.len()).expect("should not overflow");
        self.uni_pool.extend(us);
        let e = u32::try_from(self.uni_pool.len()).expect("should not overflow");
        s..e
    }

    /// Appends UIVS pairs to the pool and returns their index range.
    pub fn add_uivs_pool(&mut self, uivs: impl IntoIterator<Item = UIVSPair>) -> Range<u32> {
        let s = u32::try_from(self.uivs_pool.len()).expect("should not overflow");
        self.uivs_pool.extend(uivs);
        let e = u32::try_from(self.uivs_pool.len()).expect("should not overflow");
        s..e
    }

    /// Adds a Unicode-range-to-JIS mapping entry with its JIS pool data.
    pub fn add_urange_to_jis_mapping(
        &mut self,
        start: u32,
        jis: impl IntoIterator<Item = super::common_models::MenKuTen>,
    ) {
        let s: u16 = self.jis_pool.len().try_into().expect("should not overflow");
        self.jis_pool.extend(jis.into_iter());
        self.urange_to_jis_mappings
            .push(URangeToJISMapping { start, jis: s });
    }

    /// Adds a Unicode-range-to-MJ mapping entry with its MJ pool data.
    pub fn add_urange_to_mj_mapping<I>(&mut self, start: u32, mss: impl IntoIterator<Item = I>)
    where
        I: IntoIterator<Item = MJCode>,
    {
        let s = u32::try_from(self.mj_range_pool.len()).expect("should not overflow");
        self.mj_range_pool.extend(mss.into_iter().map(|ms| {
            let s = u32::try_from(self.mj_pool.len()).expect("should not overflow");
            self.mj_pool.extend(ms.into_iter());
            s
        }));
        self.urange_to_mj_mappings
            .push(URangeToMJMappings { start, mss: s });
    }

    /// Appends sentinel entries to all mapping tables, marking end-of-data.
    pub fn finalize(&mut self) {
        // Add sentinels
        self.urange_to_jis_mappings.push(URangeToJISMapping {
            start: u32::MAX,
            jis: self.jis_pool.len().try_into().expect("should not overflow"),
        });
        self.urange_to_mj_mappings.push(URangeToMJMappings {
            start: u32::MAX,
            mss: self
                .mj_range_pool
                .len()
                .try_into()
                .expect("should not overflow"),
        });
        self.mj_mappings.push(MJMapping {
            mj: MJCode::INVALID,
            v: self
                .uivs_pool
                .len()
                .try_into()
                .expect("should not overflow"),
        });
        self.mj_range_pool
            .push(self.mj_pool.len().try_into().expect("should not overflow"));
    }

    /// Creates a new `ConversionData` with the given JNTA mappings and empty pools.
    pub fn new(jnta_mappings: Vec<JNTAMapping>) -> Self {
        Self {
            jnta_mappings,
            mj_mappings: Vec::new(),
            uni_pool: Vec::new(),
            jis_pool: Vec::new(),
            uivs_pool: Vec::new(),
            mj_pool: Vec::new(),
            mj_range_pool: Vec::new(),
            mj_shrink_mappings: Vec::new(),
            urange_to_jis_mappings: Vec::new(),
            urange_to_mj_mappings: Vec::new(),
        }
    }
}
