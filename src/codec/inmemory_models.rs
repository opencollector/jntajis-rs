use std::pin::Pin;
use std::ptr::NonNull;

use super::common_models::{JNTAMapping, MJCode, MenKuTen, UIVSPair};
use super::inplace_ptr_vec::InplacePtrVec;

/// Maps a contiguous Unicode range to JNTA mappings (in-memory layout with pointers).
#[derive(Clone, Debug)]
pub struct URangeToJISMapping {
    pub(crate) start: u32,
    pub(crate) jis: NonNull<[Option<NonNull<JNTAMapping>>]>,
}

impl URangeToJISMapping {
    /// Returns the starting Unicode codepoint for this range.
    #[allow(dead_code)]
    pub fn start(&self) -> u32 {
        self.start
    }

    /// Returns the JNTA mapping slice for codepoints in this range.
    pub fn jis(&self) -> &[Option<&JNTAMapping>] {
        unsafe {
            std::mem::transmute::<&[Option<NonNull<JNTAMapping>>], &[Option<&JNTAMapping>]>(
                self.jis.as_ref(),
            )
        }
    }
}

/// An in-memory MJ character mapping (MJ code to Unicode+IVS pairs).
#[derive(Clone, Debug)]
pub struct MJMapping {
    pub(crate) mj: MJCode,
    pub(crate) v: NonNull<[UIVSPair]>,
}

impl MJMapping {
    /// Returns the MJ code for this mapping.
    pub fn mj(&self) -> MJCode {
        self.mj
    }

    /// Returns the Unicode+IVS pairs associated with this MJ code.
    pub fn v(&self) -> &[UIVSPair] {
        unsafe { self.v.as_ref() }
    }
}

/// Maps a contiguous Unicode range to MJ mappings (in-memory layout with pointers).
#[derive(Clone, Debug)]
pub struct URangeToMJMappings {
    pub(crate) start: u32,
    pub(crate) mss: NonNull<[InplacePtrVec<MJMapping, 4>]>,
}

impl URangeToMJMappings {
    /// Returns the starting Unicode codepoint for this range.
    #[allow(dead_code)]
    pub fn start(&self) -> u32 {
        self.start
    }

    /// Returns the MJ mapping sets for codepoints in this range.
    #[allow(dead_code)]
    pub fn mss(&self) -> &[InplacePtrVec<MJMapping, 4>] {
        unsafe { self.mss.as_ref() }
    }

    /// Returns the MJ mappings for the codepoint at offset `o` from the range start.
    pub fn mj(&self, o: u32) -> Option<&[&MJMapping]> {
        unsafe {
            self.mss
                .as_ref()
                .get(o as usize)
                .map(|v| std::mem::transmute::<&[NonNull<MJMapping>], &[&MJMapping]>(v.as_slice()))
        }
    }
}

/// Unicode codepoint sets for each MJ shrink scheme, associated with a single MJ code.
#[derive(Clone, Debug)]
pub struct MJShrinkMappingUnicodeSet {
    pub(crate) jis_incorporation_ucs_unification_rule: NonNull<[u32]>,
    pub(crate) inference_by_reading_and_glyph: NonNull<[u32]>,
    pub(crate) moj_notice_582: NonNull<[u32]>,
    pub(crate) moj_family_register_act_related_notice: NonNull<[u32]>,
}

impl MJShrinkMappingUnicodeSet {
    /// Returns candidates from the JIS incorporation and UCS unification rule scheme.
    #[allow(dead_code)]
    pub fn jis_incorporation_ucs_unification_rule(&self) -> &[u32] {
        unsafe { self.jis_incorporation_ucs_unification_rule.as_ref() }
    }

    /// Returns candidates from the inference by reading and glyph scheme.
    #[allow(dead_code)]
    pub fn inference_by_reading_and_glyph(&self) -> &[u32] {
        unsafe { self.inference_by_reading_and_glyph.as_ref() }
    }

    /// Returns candidates from the MOJ Notice 582 scheme.
    #[allow(dead_code)]
    pub fn moj_notice_582(&self) -> &[u32] {
        unsafe { self.moj_notice_582.as_ref() }
    }

    /// Returns candidates from the MOJ Family Register Act related notice scheme.
    #[allow(dead_code)]
    pub fn moj_family_register_act_related_notice(&self) -> &[u32] {
        unsafe { self.moj_family_register_act_related_notice.as_ref() }
    }

    /// Returns `true` if any scheme has at least one candidate.
    pub fn is_valid(&self) -> bool {
        !self.jis_incorporation_ucs_unification_rule().is_empty()
            || !self.inference_by_reading_and_glyph().is_empty()
            || !self.moj_notice_582().is_empty()
            || !self.moj_family_register_act_related_notice().is_empty()
    }
}

/// An MJ shrink mapping entry associating an MJ code with its shrink candidate sets.
#[derive(Clone, Debug)]
pub struct MJShrinkMapping {
    pub(crate) mj: MJCode,
    pub(crate) us: MJShrinkMappingUnicodeSet,
}

impl MJShrinkMapping {
    /// Returns the MJ code.
    #[allow(dead_code)]
    pub fn mj(&self) -> MJCode {
        self.mj
    }

    /// Returns the shrink candidate Unicode sets.
    #[allow(dead_code)]
    pub fn us(&self) -> &MJShrinkMappingUnicodeSet {
        &self.us
    }
}

/// The complete in-memory conversion dataset, containing all JNTA and MJ mapping tables.
#[derive(Clone, Debug)]
pub struct ConversionData {
    pub(crate) jnta_mappings: Pin<Box<[JNTAMapping]>>,
    pub(crate) mj_mappings: Pin<Box<[MJMapping]>>,
    pub(crate) uni_pool: Pin<Box<[u32]>>,
    pub(crate) jnta_pool: Pin<Box<[Option<NonNull<JNTAMapping>>]>>,
    pub(crate) mj_pool: Pin<Box<[InplacePtrVec<MJMapping, 4>]>>,
    pub(crate) uivs_pool: Pin<Box<[UIVSPair]>>,
    pub(crate) mj_shrink_mappings: Vec<MJShrinkMapping>,
    pub(crate) urange_to_jis_mappings: Vec<URangeToJISMapping>,
    pub(crate) urange_to_mj_mappings: Vec<URangeToMJMappings>,
}

unsafe impl Sync for ConversionData {}

unsafe impl Send for ConversionData {}

impl From<super::inwire_models::ConversionData> for Box<ConversionData> {
    fn from(data: super::inwire_models::ConversionData) -> Self {
        let mut s = Box::new(ConversionData {
            jnta_mappings: Box::into_pin(data.jnta_mappings.into_boxed_slice()),
            mj_mappings: Box::pin([] as [MJMapping; 0]),
            uni_pool: Box::into_pin(data.uni_pool.into_boxed_slice()),
            jnta_pool: Box::pin([] as [Option<NonNull<JNTAMapping>>; 0]),
            mj_pool: Box::pin([] as [InplacePtrVec<MJMapping, 4>; 0]),
            uivs_pool: Box::into_pin(data.uivs_pool.into_boxed_slice()),
            mj_shrink_mappings: Vec::with_capacity(data.mj_shrink_mappings.len()),
            urange_to_jis_mappings: Vec::with_capacity(data.urange_to_jis_mappings.len()),
            urange_to_mj_mappings: Vec::with_capacity(data.urange_to_mj_mappings.len()),
        });

        fn to_usize_range<T: num_traits::AsPrimitive<usize>>(
            r: std::ops::Range<T>,
        ) -> std::ops::Range<usize> {
            r.start.as_()..r.end.as_()
        }

        s.mj_mappings = {
            let mut mj_mappings: Vec<MJMapping> = Vec::with_capacity(data.mj_mappings.len());
            for i in 0..data.mj_mappings.len() - 1 {
                let m = &data.mj_mappings[i];
                mj_mappings.push(MJMapping {
                    mj: m.mj,
                    v: NonNull::from(&s.uivs_pool[to_usize_range(m.v..data.mj_mappings[i + 1].v)]),
                });
            }
            Pin::new(mj_mappings.into_boxed_slice())
        };

        s.jnta_pool = {
            let mut jnta_pool: Vec<Option<NonNull<JNTAMapping>>> =
                Vec::with_capacity(data.jis_pool.len());
            for m in data.jis_pool {
                jnta_pool.push(match m {
                    MenKuTen::INVALID => None,
                    _ => Some(NonNull::from(
                        &s.jnta_mappings[<MenKuTen as Into<u16>>::into(m) as usize],
                    )),
                });
            }
            Pin::new(jnta_pool.into_boxed_slice())
        };

        s.mj_pool = {
            let mut mj_pool: Vec<InplacePtrVec<MJMapping, 4>> =
                Vec::with_capacity(data.mj_pool.len());
            for i in 0..data.mj_range_pool.len() - 1 {
                let mj_range = data.mj_range_pool[i] as usize..data.mj_range_pool[i + 1] as usize;
                mj_pool.push(
                    data.mj_pool[mj_range]
                        .iter()
                        .map(|m| {
                            NonNull::from(
                                &s.mj_mappings[s
                                    .mj_mappings
                                    .binary_search_by_key(&m, |x| &x.mj)
                                    .expect("should find the mapping in mj_mappings")],
                            )
                        })
                        .collect(),
                );
            }
            Pin::new(mj_pool.into_boxed_slice())
        };

        for m in data.mj_shrink_mappings {
            s.mj_shrink_mappings.push(MJShrinkMapping {
                mj: m.mj,
                us: MJShrinkMappingUnicodeSet {
                    jis_incorporation_ucs_unification_rule: NonNull::from(
                        &s.uni_pool[to_usize_range(m.us.jis_incorporation_ucs_unification_rule)],
                    ),
                    inference_by_reading_and_glyph: NonNull::from(
                        &s.uni_pool[to_usize_range(m.us.inference_by_reading_and_glyph)],
                    ),
                    moj_notice_582: NonNull::from(&s.uni_pool[to_usize_range(m.us.moj_notice_582)]),
                    moj_family_register_act_related_notice: NonNull::from(
                        &s.uni_pool[to_usize_range(m.us.moj_family_register_act_related_notice)],
                    ),
                },
            });
        }

        for i in 0..(data.urange_to_jis_mappings.len() - 1) {
            let m = &data.urange_to_jis_mappings[i];
            let r = (m.jis as usize)..(data.urange_to_jis_mappings[i + 1].jis as usize);
            s.urange_to_jis_mappings.push(URangeToJISMapping {
                start: m.start,
                jis: NonNull::from(&s.jnta_pool[r]),
            });
        }

        for i in 0..(data.urange_to_mj_mappings.len() - 1) {
            let m = &data.urange_to_mj_mappings[i];
            let r = (m.mss as usize)..(data.urange_to_mj_mappings[i + 1].mss as usize);
            s.urange_to_mj_mappings.push(URangeToMJMappings {
                start: m.start,
                mss: NonNull::from(&s.mj_pool[r]),
            });
        }

        s
    }
}

impl ConversionData {
    /// Looks up the JNTA mapping for a Unicode codepoint.
    pub fn lookup_jnta_mapping(&self, u: u32) -> Option<&JNTAMapping> {
        let (m, o) = match self
            .urange_to_jis_mappings
            .binary_search_by_key(&u, |m| m.start)
        {
            Ok(idx) => {
                if idx >= self.urange_to_jis_mappings.len() {
                    return None;
                }
                (&self.urange_to_jis_mappings[idx], 0)
            }
            Err(mut idx) => {
                if idx < 1 {
                    return None;
                }
                idx -= 1;
                if idx >= self.urange_to_jis_mappings.len() {
                    return None;
                }
                let m = &self.urange_to_jis_mappings[idx];
                (m, u - m.start)
            }
        };
        m.jis()[o as usize]
    }

    /// Looks up the MJ mappings for a Unicode codepoint.
    pub fn lookup_mj_mapping(&self, u: u32) -> Option<&[&MJMapping]> {
        let (m, o) = match self
            .urange_to_mj_mappings
            .binary_search_by_key(&u, |m| m.start)
        {
            Ok(idx) => {
                if idx >= self.urange_to_mj_mappings.len() {
                    return None;
                }
                (&self.urange_to_mj_mappings[idx], 0)
            }
            Err(mut idx) => {
                if idx < 1 {
                    return None;
                }
                idx -= 1;
                if idx >= self.urange_to_mj_mappings.len() {
                    return None;
                }
                let m = &self.urange_to_mj_mappings[idx];
                (m, u - m.start)
            }
        };
        m.mj(o)
    }

    /// Looks up the MJ shrink mapping for an MJ code.
    pub fn lookup_mj_shrink_mapping(&self, m: MJCode) -> Option<&MJShrinkMapping> {
        let i = u32::from(m) as usize;
        if i >= self.mj_shrink_mappings.len() {
            return None;
        }
        Some(&self.mj_shrink_mappings[i])
    }

    /// Looks up all MJ shrink mappings associated with a Unicode codepoint.
    pub fn lookup_mj_shrink_mapping_by_unicode(&self, u: u32) -> Option<Vec<&MJShrinkMapping>> {
        self.lookup_mj_mapping(u).and_then(|m| {
            if m.is_empty() {
                return None;
            }
            let r = m
                .iter()
                .filter_map(|m| self.lookup_mj_shrink_mapping(m.mj))
                .collect::<Vec<_>>();
            if r.is_empty() { None } else { Some(r) }
        })
    }
}
