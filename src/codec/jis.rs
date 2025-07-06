use std::collections::VecDeque;
use std::sync::Arc;

use crate::codec::array_vec::PackedU8Vec;

use super::error::{EncoderResult, TransliterationError};
use super::{array_vec, common_models, generated, inmemory_models};

pub struct UniToJNTAMappingIterator<'a, I>
where
    I: Iterator<Item = char>,
{
    data: Arc<inmemory_models::ConversionData>,
    inner: I,
    eoi: bool,
    lookahead: VecDeque<char>,
    _marker: std::marker::PhantomData<&'a ()>,
}

pub trait MenKuTenIteratorMixin: Iterator<Item = common_models::MenKuTen> + Sized {
    fn to_iso2022(
        self,
        code_offset: u8,
        siso: bool,
    ) -> impl Iterator<Item = Result<array_vec::PackedU8Vec, EncoderResult>>;
}

pub trait MenKuTenResultIteratorMixin:
    Iterator<Item = Result<common_models::MenKuTen, EncoderResult>> + Sized
{
    fn to_iso2022(
        self,
        code_offset: u8,
        siso: bool,
    ) -> impl Iterator<Item = Result<array_vec::PackedU8Vec, EncoderResult>>;
}

impl<T: Iterator<Item = common_models::MenKuTen>> MenKuTenIteratorMixin for T {
    fn to_iso2022(
        self,
        code_offset: u8,
        siso: bool,
    ) -> impl Iterator<Item = Result<array_vec::PackedU8Vec, EncoderResult>> {
        MenKuTenToISO2022Iterator::new(self.map(Ok), code_offset, siso)
    }
}

impl<T: Iterator<Item = Result<common_models::MenKuTen, EncoderResult>>> MenKuTenResultIteratorMixin
    for T
{
    fn to_iso2022(
        self,
        code_offset: u8,
        siso: bool,
    ) -> impl Iterator<Item = Result<array_vec::PackedU8Vec, EncoderResult>> {
        MenKuTenToISO2022Iterator::new(self, code_offset, siso)
    }
}

pub trait UniToJNTAMappingIteratorMixin<'a>:
    Iterator<Item = &'a common_models::JNTAMapping> + Sized
{
    fn replace_inconvertibles(
        self,
        _: &'a common_models::JNTAMapping,
    ) -> impl Iterator<Item = &'a common_models::JNTAMapping> {
        self
    }

    fn to_men_ku_ten(self) -> impl Iterator<Item = common_models::MenKuTen> {
        self.map(|i| i.jis)
    }
}

pub trait UniToJNTAMappingResultIteratorMixin<'a>:
    Iterator<Item = Result<&'a common_models::JNTAMapping, EncoderResult>> + Sized
{
    fn replace_inconvertibles(
        self,
        r: &'a common_models::JNTAMapping,
    ) -> impl Iterator<Item = &'a common_models::JNTAMapping> {
        self.map(move |i| match i {
            Ok(j) => j,
            Err(_) => r,
        })
    }

    fn to_men_ku_ten(self) -> impl Iterator<Item = Result<common_models::MenKuTen, EncoderResult>> {
        self.map(|i| match i {
            Ok(j) => Ok(j.jis),
            Err(e) => Err(e),
        })
    }
}

impl<'a, T: Iterator<Item = &'a common_models::JNTAMapping>> UniToJNTAMappingIteratorMixin<'a>
    for T
{
}

impl<'a, T: Iterator<Item = Result<&'a common_models::JNTAMapping, EncoderResult>>>
    UniToJNTAMappingResultIteratorMixin<'a> for T
{
}

impl<'a, I> Iterator for UniToJNTAMappingIterator<'a, I>
where
    I: Iterator<Item = char>,
{
    type Item = Result<&'a common_models::JNTAMapping, EncoderResult>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.eoi {
            return None;
        }

        let mut state = 0;
        let mut c: Option<char> = None;
        loop {
            if state == 0
                && let Some(c_) = self.lookahead.pop_front()
            {
                if let Some(m) = self.data.lookup_jnta_mapping(c_ as u32) {
                    break Some(Ok(unsafe {
                        // this is safe because the reference is from the Arc'ed data and its lifetime is
                        // guaranteed to be valid for the lifetime of the iterator
                        std::mem::transmute::<
                            &'_ common_models::JNTAMapping,
                            &'a common_models::JNTAMapping,
                        >(m)
                    }));
                }
                break Some(Err(EncoderResult::Unmappable {
                    ch: c_,
                    position: 0,
                }));
            }
            if let Some(c_) = c {
                let (state_, j) = generated::sm_uni_to_jis_mapping(state, c_ as u32);
                if let Some(j) = j {
                    self.lookahead.clear();
                    break Some(Ok(unsafe {
                        std::mem::transmute::<
                            &'_ common_models::JNTAMapping,
                            &'a common_models::JNTAMapping,
                        >(&self.data.jnta_mappings[u16::from(j) as usize])
                    }));
                } else {
                    if state_ == 0 && state == 0 {
                        if let Some(m) = self.data.lookup_jnta_mapping(c_ as u32) {
                            break Some(Ok(unsafe {
                                // this is safe because the reference is from the Arc'ed data and its lifetime is
                                // guaranteed to be valid for the lifetime of the iterator
                                std::mem::transmute::<
                                    &'_ common_models::JNTAMapping,
                                    &'a common_models::JNTAMapping,
                                >(m)
                            }));
                        }
                        break Some(Err(EncoderResult::Unmappable {
                            ch: c_,
                            position: 0,
                        }));
                    }
                    state = state_;
                    self.lookahead.push_back(c_);
                    c = None;
                }
            } else {
                c = self.inner.next();
                if c.is_none() {
                    self.eoi = true;
                    break None;
                }
            }
        }
    }
}

struct MenKuTenToISO2022Iterator<I>
where
    I: Iterator<Item = Result<common_models::MenKuTen, EncoderResult>>,
{
    inner: I,
    code_offset: u8,
    last_men: u8,
}

impl<I> MenKuTenToISO2022Iterator<I>
where
    I: Iterator<Item = Result<common_models::MenKuTen, EncoderResult>>,
{
    pub fn new(inner: I, code_offset: u8, siso: bool) -> Self {
        MenKuTenToISO2022Iterator {
            inner,
            code_offset,
            last_men: if siso { 1 } else { 0 },
        }
    }
}

impl<I> Iterator for MenKuTenToISO2022Iterator<I>
where
    I: Iterator<Item = Result<common_models::MenKuTen, EncoderResult>>,
{
    type Item = Result<PackedU8Vec, EncoderResult>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner.next() {
            Some(j) => {
                if let Err(e) = j {
                    return Some(Err(e));
                }
                let j = j.unwrap();
                if self.last_men != 0x0 {
                    let next_men = j.men();
                    if self.last_men != next_men {
                        self.last_men = next_men;
                        return match next_men {
                            1 => Some(Ok(PackedU8Vec::try_from([
                                0x0e,
                                self.code_offset + j.ku(),
                                self.code_offset + j.ten(),
                            ])
                            .unwrap())),
                            2 => Some(Ok(PackedU8Vec::try_from([
                                0x0f,
                                self.code_offset + j.ku(),
                                self.code_offset + j.ten(),
                            ])
                            .unwrap())),
                            _ => Some(Err(EncoderResult::Unmappable {
                                ch: '\u{FFFD}',
                                position: 0,
                            })),
                        };
                    }
                }
                Some(Ok(PackedU8Vec::try_from([
                    self.code_offset + j.ku(),
                    self.code_offset + j.ten(),
                ])
                .unwrap()))
            }
            None => None,
        }
    }
}

pub fn convert_uni_to_jis<'a, 'b>(
    us: impl IntoIterator<Item = char> + 'a,
) -> impl Iterator<Item = Result<&'b common_models::JNTAMapping, EncoderResult>> + 'b
where
    'a: 'b,
{
    UniToJNTAMappingIterator {
        data: super::get_data(),
        inner: us.into_iter(),
        eoi: false,
        lookahead: VecDeque::new(),
        _marker: std::marker::PhantomData,
    }
}

pub struct JISX0213TransliteratingIterator<'a, I>
where
    I: Iterator<Item = char>,
{
    data: Arc<inmemory_models::ConversionData>,
    inner: I,
    eoi: bool,
    buf: VecDeque<char>,
    passthrough_under_7f: bool,
    _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a, I> Iterator for JISX0213TransliteratingIterator<'a, I>
where
    I: Iterator<Item = char>,
{
    type Item = Result<char, TransliterationError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.eoi {
            return None;
        }

        if let Some(c) = self.buf.pop_front() {
            return Some(Ok(c));
        }
        if let Some(c) = self.inner.next() {
            if self.passthrough_under_7f && c < '\u{007f}' {
                return Some(Ok(c));
            }
            if let Some(m) = self.data.lookup_jnta_mapping(c as u32) {
                if m.class.is_jisx0213() {
                    return if !m.tx_us.is_empty() {
                        unsafe {
                            self.buf
                                .extend(m.tx_us[1..].iter().map(|&b| char::from_u32_unchecked(b)));
                            Some(Ok(char::from_u32_unchecked(m.tx_us[0])))
                        }
                    } else {
                        Some(Err(TransliterationError::UnmappableChar(c)))
                    };
                } else {
                    return Some(Ok(c));
                }
            }
            Some(Err(TransliterationError::UnmappableChar(c)))
        } else {
            self.eoi = true;
            None
        }
    }
}

pub fn transliterate_jisx0213<'a, 'b>(
    us: impl IntoIterator<Item = char> + 'a,
    passthrough_under_7f: bool,
) -> impl Iterator<Item = Result<char, TransliterationError>> + 'b
where
    'a: 'b,
{
    JISX0213TransliteratingIterator {
        data: super::get_data(),
        inner: us.into_iter(),
        eoi: false,
        buf: VecDeque::new(),
        passthrough_under_7f,
        _marker: std::marker::PhantomData,
    }
}

#[cfg(test)]
mod tests {
    use super::super::common_models::MenKuTen;
    use super::*;

    #[test]
    fn test_convert_uni_to_jis() {
        let mut iter = convert_uni_to_jis("高髙".chars()).to_men_ku_ten();
        assert_eq!(iter.next(), Some(Ok(MenKuTen::new(1, 25, 66).unwrap())));
        assert_eq!(
            iter.next(),
            Some(Err(EncoderResult::Unmappable {
                ch: '髙',
                position: 0
            }))
        );
        assert_eq!(iter.next(), None, "Expected end of iterator");

        let mut iter = convert_uni_to_jis("ジャンクロードヴァンダム".chars()).to_men_ku_ten();
        assert_eq!(iter.next(), Some(Ok(MenKuTen::new(1, 5, 24).unwrap())));
        assert_eq!(iter.next(), Some(Ok(MenKuTen::new(1, 5, 67).unwrap())));
        assert_eq!(iter.next(), Some(Ok(MenKuTen::new(1, 5, 83).unwrap())));
        assert_eq!(iter.next(), Some(Ok(MenKuTen::new(1, 5, 15).unwrap())));
        assert_eq!(iter.next(), Some(Ok(MenKuTen::new(1, 5, 77).unwrap())));
        assert_eq!(iter.next(), Some(Ok(MenKuTen::new(1, 1, 28).unwrap())));
        assert_eq!(iter.next(), Some(Ok(MenKuTen::new(1, 5, 41).unwrap())));
        assert_eq!(iter.next(), Some(Ok(MenKuTen::new(1, 5, 84).unwrap())));
        assert_eq!(iter.next(), Some(Ok(MenKuTen::new(1, 5, 1).unwrap())));
        assert_eq!(iter.next(), Some(Ok(MenKuTen::new(1, 5, 83).unwrap())));
        assert_eq!(iter.next(), Some(Ok(MenKuTen::new(1, 5, 32).unwrap())));
        assert_eq!(iter.next(), Some(Ok(MenKuTen::new(1, 5, 64).unwrap())));
        assert_eq!(iter.next(), None, "Expected end of iterator");

        let mut iter = convert_uni_to_jis("\u{2e9}\u{2e4}".chars()).to_men_ku_ten();
        assert_eq!(iter.next(), Some(Ok(MenKuTen::new(1, 11, 68).unwrap())));
        assert_eq!(
            iter.next(),
            Some(Err(EncoderResult::Unmappable {
                ch: '\u{2e4}',
                position: 0
            }))
        );

        let mut iter = convert_uni_to_jis("\u{2e9}\u{2e5}".chars()).to_men_ku_ten();
        assert_eq!(iter.next(), Some(Ok(MenKuTen::new(1, 11, 69).unwrap())));
        assert_eq!(iter.next(), None, "Expected end of iterator");
    }

    #[test]
    fn test_convert_uni_to_jis_bytes() {
        let mut bytes = Vec::<u8>::new();
        for c in convert_uni_to_jis("ジャンクロードヴァンダム".chars())
            .to_men_ku_ten()
            .to_iso2022(0x20, false)
        {
            match c {
                Ok(arr) => {
                    arr.write_into(&mut bytes).unwrap();
                }
                Err(e) => panic!("Conversion failed: {:?}", e),
            }
        }
        assert_eq!(
            bytes,
            vec![
                0x25, 0x38, 0x25, 0x63, 0x25, 0x73, 0x25, 0x2f, 0x25, 0x6d, 0x21, 0x3c, 0x25, 0x49,
                0x25, 0x74, 0x25, 0x21, 0x25, 0x73, 0x25, 0x40, 0x25, 0x60,
            ],
            "Expected ISO-2022-JP encoded bytes"
        );
    }

    #[test]
    fn test_transliterate_jisx0213() {
        {
            let r: String = transliterate_jisx0213("偀〖ㇵ❶Ⅻ㈱輀".chars(), false)
                .collect::<Result<_, _>>()
                .unwrap();
            assert_eq!(r, "英【ハ１ＸＩＩ（株）轜");
        }
        {
            let r = transliterate_jisx0213("123456".chars(), false).next();
            assert!(r.is_some());
            assert!(r.unwrap().is_err());
        }
        {
            let r: String = transliterate_jisx0213("123456".chars(), true)
                .collect::<Result<_, _>>()
                .unwrap();
            assert_eq!(r, "123456");
        }
    }
}
