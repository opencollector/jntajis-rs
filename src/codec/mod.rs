//! Character encoding and transliteration codecs.
//!
//! This module provides the core functionality for character transliteration between
//! different Japanese character sets including JIS X 0208, JIS X 0213, and Unicode.
//!
//! The main functionality is provided through the [`mj_shrink`] module, which contains
//! the MJ character shrink conversion functions.

use std::sync::{Arc, OnceLock};

#[cfg(not(feature = "codegen"))]
pub(crate) mod common_models;
#[cfg(feature = "codegen")]
pub mod common_models;
#[cfg(not(feature = "codegen"))]
pub(crate) mod inmemory_models;
#[cfg(feature = "codegen")]
pub mod inmemory_models;
#[cfg(not(feature = "codegen"))]
pub(crate) mod inwire_models;
#[cfg(feature = "codegen")]
pub mod inwire_models;

pub mod array_vec;

pub mod conversion_mode;
pub mod decoder;
pub mod encoder;
pub mod error;
pub(crate) mod generated;
pub(crate) mod inplace_ptr_vec;
pub(crate) mod invalid_value;
pub mod jis;
pub mod mj_shrink;
pub mod translit;

fn unpack_data() -> Result<inwire_models::ConversionData, Box<dyn std::error::Error>> {
    static DATA: &[u8] = include_bytes!("./generated.bin");

    let uncompressed_len = u32::from_le_bytes(DATA[0..4].try_into()?) as usize;
    let uncompressed_data = lz4_flex::decompress(&DATA[4..], uncompressed_len)?;
    let mut aligned = rkyv::util::AlignedVec::<16>::with_capacity(uncompressed_data.len());
    aligned.extend_from_slice(&uncompressed_data);
    let data: inwire_models::ConversionData =
        rkyv::from_bytes::<inwire_models::ConversionData, rkyv::rancor::Error>(&aligned)
            .map_err(|e| e.to_string())?;
    Ok(data)
}

pub(crate) fn get_data() -> Arc<inmemory_models::ConversionData> {
    static DATA: OnceLock<Arc<inmemory_models::ConversionData>> = OnceLock::new();
    DATA.get_or_init(|| {
        let inmem_data: Box<inmemory_models::ConversionData> =
            unpack_data().expect("Failed to unpack data").into();
        Arc::from(inmem_data)
    })
    .clone()
}

#[cfg(test)]
mod tests {
    use crate::codec::common_models::{Ivs, MenKuTen};

    use super::inmemory_models::MJMapping;

    use super::*;

    #[test]
    fn test_unpack_data() {
        let data = unpack_data();
        assert!(data.is_ok(), "{}", data.unwrap_err().to_string());
        let data = data.unwrap();
        assert!(!data.jnta_mappings.is_empty());
        assert!(!data.mj_mappings.is_empty());
        assert!(!data.jis_pool.is_empty());
        assert!(!data.uni_pool.is_empty());
        assert!(!data.mj_pool.is_empty());
        assert!(!data.mj_shrink_mappings.is_empty());
        assert!(!data.urange_to_jis_mappings.is_empty());
        assert!(!data.urange_to_mj_mappings.is_empty());
        assert!(
            !data
                .mj_shrink_mappings
                .iter()
                .all(|m| m.us.inference_by_reading_and_glyph.is_empty())
        );
        assert!(
            !data
                .mj_shrink_mappings
                .iter()
                .all(|m| m.us.jis_incorporation_ucs_unification_rule.is_empty())
        );
        assert!(
            !data
                .mj_shrink_mappings
                .iter()
                .all(|m| m.us.moj_family_register_act_related_notice.is_empty())
        );
        assert!(
            !data
                .mj_shrink_mappings
                .iter()
                .all(|m| m.us.moj_notice_582.is_empty())
        );
    }

    #[test]
    fn test_get_data() {
        let data = get_data();
        assert!(!data.jnta_mappings.is_empty());
        assert!(!data.mj_mappings.is_empty());
        assert!(!data.uni_pool.is_empty());
        assert!(!data.jnta_pool.is_empty());
        assert!(!data.mj_shrink_mappings.is_empty());
        assert!(!data.urange_to_jis_mappings.is_empty());
        assert!(!data.urange_to_mj_mappings.is_empty());
        assert!(!data.mj_shrink_mappings.is_empty());
        assert!(
            !data
                .mj_shrink_mappings
                .iter()
                .all(|m| m.us.inference_by_reading_and_glyph.is_empty())
        );
        assert!(
            !data
                .mj_shrink_mappings
                .iter()
                .all(|m| m.us.jis_incorporation_ucs_unification_rule.is_empty())
        );
        assert!(
            !data
                .mj_shrink_mappings
                .iter()
                .all(|m| m.us.moj_family_register_act_related_notice.is_empty())
        );
        assert!(
            !data
                .mj_shrink_mappings
                .iter()
                .all(|m| m.us.moj_notice_582.is_empty())
        );
    }

    #[test]
    fn test_lookup_jnta_mapping() {
        let data = get_data();

        let j = data.lookup_jnta_mapping('高' as u32);
        assert!(j.is_some(), "Failed to find JNTAMapping for '高'");
        assert_eq!(j.unwrap().jis, MenKuTen::new(1, 25, 66).unwrap());
        let j = data.lookup_jnta_mapping('髙' as u32);
        assert!(j.is_none(), "Failed to find JNTAMapping for '髙'");
    }

    #[test]
    fn test_lookup_mj_mapping() {
        let data = get_data();

        let m = data.lookup_mj_mapping('高' as u32);
        assert!(m.is_some(), "Failed to find MJ mapping for '高'");
        assert_eq!(m.unwrap().len(), 1, "Expected one MJ mapping for '高'");
        assert_eq!(m.unwrap()[0].mj, 28901.into());

        let m = data.lookup_mj_mapping(0x2ea41);
        assert!(m.is_some(), "Failed to find MJ mapping for '\u{2ea41}'");
        assert_eq!(
            m.unwrap().len(),
            3,
            "Expected three MJ mapping for '\u{2ea41}'"
        );
        let mut mjs: Vec<&MJMapping> = m.unwrap().into();
        mjs.sort_by_key(|mj| mj.mj);
        assert_eq!(
            mjs.iter().map(|mj| mj.mj.into()).collect::<Vec<u32>>(),
            vec![60341, 60342, 68100]
        );
        assert_eq!(
            mjs[0].v.len(),
            2,
            "Expected two UIVS pair for the first MJ mapping"
        );
        assert_eq!(unsafe { mjs[0].v.as_ref()[0] }.u, 0x2ea41);
        assert_eq!(unsafe { mjs[0].v.as_ref()[0] }.s, None);
        assert_eq!(unsafe { mjs[0].v.as_ref()[1] }.u, 0x2ea41);
        assert_eq!(unsafe { mjs[0].v.as_ref()[1] }.s, Some(Ivs::new(16)));
    }

    #[test]
    fn test_lookup_mj_shrink_mapping() {
        let data = get_data();

        let m = data.lookup_mj_shrink_mapping(28902.into());
        let m = m.expect("should be able to look up MJ shrink mapping for '髙'");
        assert_eq!(m.mj, 28902.into());
        assert_eq!(m.us.inference_by_reading_and_glyph().len(), 0);
        assert_eq!(m.us.jis_incorporation_ucs_unification_rule().len(), 1);
        assert_eq!(m.us.jis_incorporation_ucs_unification_rule()[0], 0x9ad8);
        assert_eq!(m.us.moj_family_register_act_related_notice().len(), 1);
        assert_eq!(m.us.moj_notice_582().len(), 0);
    }

    #[test]
    fn test_sm_uni_to_jis_mapping() {
        let (state, j) = super::generated::sm_uni_to_jis_mapping(0, '\u{2e9}' as u32);
        assert_eq!(state, 7);
        assert!(j.is_none());

        let (state, j) = super::generated::sm_uni_to_jis_mapping(7, '\u{2e4}' as u32);
        assert_eq!(state, 0);
        assert!(j.is_none());

        let (state, j) = super::generated::sm_uni_to_jis_mapping(7, '\u{2e5}' as u32);
        assert_eq!(state, 0);
        assert_eq!(j, Some(MenKuTen::new(1, 11, 69).unwrap()));
    }
}
