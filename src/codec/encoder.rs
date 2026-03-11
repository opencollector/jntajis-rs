use std::sync::Arc;

use super::ConversionMode;
use super::common_models::{JISCharacterClass, MenKuTen};
use super::error::EncoderResult;
use super::generated::sm_uni_to_jis_mapping;
use super::inmemory_models::ConversionData;

/// Encodes a complete UTF-8 string into JIS bytes in a single call.
///
/// Uses `code_offset = 0x20` (standard ISO-2022 offset, matching the Python
/// `jntajis` library).
///
/// # Errors
///
/// Returns [`EncoderResult::Unmappable`] if the input contains a character
/// that cannot be mapped under the given [`ConversionMode`].
pub fn jnta_encode(input: &str, mode: ConversionMode) -> Result<Vec<u8>, EncoderResult> {
    let mut encoder = Encoder::new(mode, 0x20);
    let mut out = Vec::with_capacity(input.len() * 2);
    let mut buf = [0u8; 1024];
    let mut remaining = input;

    loop {
        let (result, read, written) =
            encoder.encode_from_utf8_without_replacement(remaining, &mut buf, true);
        out.extend_from_slice(&buf[..written]);

        match result {
            EncoderResult::InputEmpty => return Ok(out),
            EncoderResult::OutputFull => {
                remaining = &remaining[read..];
            }
            EncoderResult::Unmappable { ch, .. } => {
                // Compute byte offset of the unmappable char in the original input
                let position = input.len() - remaining.len() + read - ch.len_utf8();
                return Err(EncoderResult::Unmappable { ch, position });
            }
        }
    }
}

/// Extracted data from a JNTAMapping that is needed for encoding.
/// All fields are Copy so we can drop the borrow on ConversionData before mutating self.
struct MappingInfo {
    jis: MenKuTen,
    class: JISCharacterClass,
    first_us: u32,
    tx_jis: [MenKuTen; 4],
    tx_jis_len: usize,
}

impl MappingInfo {
    fn extract(mapping: &super::common_models::JNTAMapping) -> Self {
        let mut tx_jis = [MenKuTen(0); 4];
        let tx_jis_len = mapping.tx_jis.len();
        #[allow(clippy::needless_range_loop)]
        for i in 0..tx_jis_len {
            tx_jis[i] = mapping.tx_jis[i];
        }
        Self {
            jis: mapping.jis,
            class: mapping.class,
            first_us: if !mapping.us.is_empty() {
                mapping.us[0]
            } else {
                0xFFFD
            },
            tx_jis,
            tx_jis_len,
        }
    }
}

/// Buffer-based JIS encoder following the encoding_rs pattern.
///
/// Encodes Unicode text into JIS byte sequences using the specified
/// [`ConversionMode`].
pub struct Encoder {
    data: Arc<ConversionData>,
    mode: ConversionMode,
    code_offset: u8,
    sm_state: i32,
    lookahead: [u32; 4],
    lookahead_len: usize,
    shift_state: u8, // 0=none, 1=plane1, 2=plane2
    pending: [u8; 16],
    pending_start: usize,
    pending_end: usize,
}

impl Encoder {
    /// Creates a new encoder with the given conversion mode and code offset.
    ///
    /// The `code_offset` is typically `0x20` for ISO-2022-JP style encoding.
    pub fn new(mode: ConversionMode, code_offset: u8) -> Self {
        Self {
            data: super::get_data(),
            mode,
            code_offset,
            sm_state: 0,
            lookahead: [0; 4],
            lookahead_len: 0,
            shift_state: if mode == ConversionMode::Siso { 1 } else { 0 },
            pending: [0; 16],
            pending_start: 0,
            pending_end: 0,
        }
    }

    /// Resets the encoder to its initial state.
    pub fn reset(&mut self) {
        self.sm_state = 0;
        self.lookahead_len = 0;
        self.shift_state = if self.mode == ConversionMode::Siso {
            1
        } else {
            0
        };
        self.pending_start = 0;
        self.pending_end = 0;
    }

    /// Encodes UTF-8 input into the output buffer, stopping on unmappable characters.
    ///
    /// Returns `(result, bytes_read, bytes_written)`.
    ///
    /// When `last` is `true`, the encoder flushes any pending state machine state.
    pub fn encode_from_utf8_without_replacement(
        &mut self,
        src: &str,
        dst: &mut [u8],
        last: bool,
    ) -> (EncoderResult, usize, usize) {
        let mut src_pos = 0;
        let mut dst_pos = 0;

        // Drain pending bytes first
        while self.pending_start < self.pending_end && dst_pos < dst.len() {
            dst[dst_pos] = self.pending[self.pending_start];
            self.pending_start += 1;
            dst_pos += 1;
        }
        if self.pending_start < self.pending_end {
            return (EncoderResult::OutputFull, 0, dst_pos);
        }

        let mut chars = src.char_indices();
        let mut next_char: Option<(usize, char)> = chars.next();

        loop {
            let c = if let Some((idx, ch)) = next_char.take() {
                src_pos = idx;
                Some(ch)
            } else if let Some((idx, ch)) = chars.next() {
                src_pos = idx;
                Some(ch)
            } else {
                None
            };

            if let Some(ch) = c {
                let char_start = src_pos;
                let char_len = ch.len_utf8();

                if self.sm_state != 0 {
                    let (new_state, result) = sm_uni_to_jis_mapping(self.sm_state, ch as u32);
                    if let Some(mkt) = result {
                        self.lookahead_len = 0;
                        self.sm_state = 0;
                        let info =
                            MappingInfo::extract(&self.data.jnta_mappings[u16::from(mkt) as usize]);
                        match self.emit_info(&info, dst, &mut dst_pos, char_start) {
                            Ok(()) => {
                                src_pos = char_start + char_len;
                                continue;
                            }
                            Err(r) => {
                                src_pos = char_start + char_len;
                                return (r, src_pos, dst_pos);
                            }
                        }
                    } else if new_state == 0 {
                        // State machine reset without match — flush lookahead
                        self.sm_state = 0;
                        let la_len = self.lookahead_len;
                        let la = self.lookahead;
                        self.lookahead_len = 0;

                        for i in 0..la_len {
                            match self.emit_single_char(la[i], dst, &mut dst_pos, char_start) {
                                Ok(()) => {}
                                Err(EncoderResult::OutputFull) => {
                                    let remaining = la_len - i - 1;
                                    for j in 0..remaining {
                                        self.lookahead[j] = la[i + 1 + j];
                                    }
                                    self.lookahead_len = remaining;
                                    return (EncoderResult::OutputFull, char_start, dst_pos);
                                }
                                Err(r) => {
                                    let remaining = la_len - i - 1;
                                    for j in 0..remaining {
                                        self.lookahead[j] = la[i + 1 + j];
                                    }
                                    self.lookahead_len = remaining;
                                    return (r, char_start, dst_pos);
                                }
                            }
                        }
                        // Re-process current char
                        next_char = Some((char_start, ch));
                        continue;
                    } else {
                        self.sm_state = new_state;
                        if self.lookahead_len < 4 {
                            self.lookahead[self.lookahead_len] = ch as u32;
                            self.lookahead_len += 1;
                        }
                        src_pos = char_start + char_len;
                        continue;
                    }
                }

                // sm_state == 0, try state machine first
                let (new_state, result) = sm_uni_to_jis_mapping(0, ch as u32);
                if let Some(mkt) = result {
                    let info =
                        MappingInfo::extract(&self.data.jnta_mappings[u16::from(mkt) as usize]);
                    match self.emit_info(&info, dst, &mut dst_pos, char_start) {
                        Ok(()) => {
                            src_pos = char_start + char_len;
                            continue;
                        }
                        Err(r) => {
                            src_pos = char_start + char_len;
                            return (r, src_pos, dst_pos);
                        }
                    }
                } else if new_state != 0 {
                    self.sm_state = new_state;
                    self.lookahead[0] = ch as u32;
                    self.lookahead_len = 1;
                    src_pos = char_start + char_len;
                    continue;
                }

                // Direct lookup
                match self.emit_single_char(ch as u32, dst, &mut dst_pos, char_start) {
                    Ok(()) => {
                        src_pos = char_start + char_len;
                        continue;
                    }
                    Err(r) => {
                        src_pos = char_start + char_len;
                        return (r, src_pos, dst_pos);
                    }
                }
            } else {
                // No more input
                src_pos = src.len();
                if last {
                    if self.sm_state != 0 {
                        let la_len = self.lookahead_len;
                        let la = self.lookahead;
                        self.lookahead_len = 0;
                        self.sm_state = 0;

                        for i in 0..la_len {
                            match self.emit_single_char(la[i], dst, &mut dst_pos, src_pos) {
                                Ok(()) => {}
                                Err(r) => {
                                    let remaining = la_len - i - 1;
                                    for j in 0..remaining {
                                        self.lookahead[j] = la[i + 1 + j];
                                    }
                                    self.lookahead_len = remaining;
                                    return (r, src_pos, dst_pos);
                                }
                            }
                        }
                    }
                    // Emit final shift if needed (return to plane 1 in SISO mode)
                    if self.mode == ConversionMode::Siso && self.shift_state == 2 {
                        if dst_pos < dst.len() {
                            dst[dst_pos] = 0x0e;
                            dst_pos += 1;
                            self.shift_state = 1;
                        } else {
                            self.pending[0] = 0x0e;
                            self.pending_start = 0;
                            self.pending_end = 1;
                            self.shift_state = 1;
                            return (EncoderResult::OutputFull, src_pos, dst_pos);
                        }
                    }
                }
                return (EncoderResult::InputEmpty, src_pos, dst_pos);
            }
        }
    }

    /// Encodes UTF-8 input into the output buffer, silently dropping unmappable characters.
    ///
    /// Unlike `encoding_rs` which emits numeric character references for unmappable
    /// characters, this method simply skips them. Use `encode_from_utf8_without_replacement`
    /// to detect and handle unmappable characters explicitly.
    ///
    /// Returns `(result, bytes_read, bytes_written, had_replacements)`.
    pub fn encode_from_utf8(
        &mut self,
        src: &str,
        dst: &mut [u8],
        last: bool,
    ) -> (EncoderResult, usize, usize, bool) {
        let mut total_read = 0;
        let mut total_written = 0;
        let mut had_replacements = false;
        let mut remaining = src;

        loop {
            let (result, read, written) = self.encode_from_utf8_without_replacement(
                remaining,
                &mut dst[total_written..],
                last,
            );
            total_read += read;
            total_written += written;

            match result {
                EncoderResult::InputEmpty => {
                    return (
                        EncoderResult::InputEmpty,
                        total_read,
                        total_written,
                        had_replacements,
                    );
                }
                EncoderResult::OutputFull => {
                    return (
                        EncoderResult::OutputFull,
                        total_read,
                        total_written,
                        had_replacements,
                    );
                }
                EncoderResult::Unmappable { .. } => {
                    had_replacements = true;
                    remaining = &src[total_read..];
                    continue;
                }
            }
        }
    }

    /// Emit bytes for a single char (by u32 codepoint) via direct lookup.
    fn emit_single_char(
        &mut self,
        u: u32,
        dst: &mut [u8],
        dst_pos: &mut usize,
        position: usize,
    ) -> Result<(), EncoderResult> {
        if let Some(mapping) = self.data.lookup_jnta_mapping(u) {
            let info = MappingInfo::extract(mapping);
            self.emit_info(&info, dst, dst_pos, position)
        } else {
            // SAFETY: u came from a char that was cast to u32 earlier in the call chain.
            let ch = unsafe { char::from_u32_unchecked(u) };
            Err(EncoderResult::Unmappable { ch, position })
        }
    }

    /// Emit the JIS-encoded bytes for extracted mapping info.
    fn emit_info(
        &mut self,
        info: &MappingInfo,
        dst: &mut [u8],
        dst_pos: &mut usize,
        position: usize,
    ) -> Result<(), EncoderResult> {
        let mut scratch = [0u8; 16];
        let mut scratch_len = 0;
        // SAFETY: first_us originates from generated conversion data and is a valid codepoint.
        let unmappable_char = unsafe { char::from_u32_unchecked(info.first_us) };

        match self.mode {
            ConversionMode::Siso => {
                let men = info.jis.men();
                if self.shift_state != men {
                    scratch[scratch_len] = if men == 1 { 0x0e } else { 0x0f };
                    scratch_len += 1;
                    self.shift_state = men;
                }
                scratch[scratch_len] = self.code_offset + info.jis.ku();
                scratch_len += 1;
                scratch[scratch_len] = self.code_offset + info.jis.ten();
                scratch_len += 1;
            }
            ConversionMode::Men1 => {
                if info.jis.men() != 1 {
                    return Err(EncoderResult::Unmappable {
                        ch: unmappable_char,
                        position,
                    });
                }
                scratch[scratch_len] = self.code_offset + info.jis.ku();
                scratch_len += 1;
                scratch[scratch_len] = self.code_offset + info.jis.ten();
                scratch_len += 1;
            }
            ConversionMode::Jisx0208 => {
                if !info.class.is_jisx0208() {
                    return Err(EncoderResult::Unmappable {
                        ch: unmappable_char,
                        position,
                    });
                }
                scratch[scratch_len] = self.code_offset + info.jis.ku();
                scratch_len += 1;
                scratch[scratch_len] = self.code_offset + info.jis.ten();
                scratch_len += 1;
            }
            ConversionMode::Jisx0208Translit => {
                if info.class.is_jisx0208() {
                    scratch[scratch_len] = self.code_offset + info.jis.ku();
                    scratch_len += 1;
                    scratch[scratch_len] = self.code_offset + info.jis.ten();
                    scratch_len += 1;
                } else if info.tx_jis_len > 0 {
                    for i in 0..info.tx_jis_len {
                        let tx = info.tx_jis[i];
                        scratch[scratch_len] = self.code_offset + tx.ku();
                        scratch_len += 1;
                        scratch[scratch_len] = self.code_offset + tx.ten();
                        scratch_len += 1;
                    }
                } else {
                    return Err(EncoderResult::Unmappable {
                        ch: unmappable_char,
                        position,
                    });
                }
            }
        }

        self.flush_scratch(&scratch[..scratch_len], dst, dst_pos)
    }

    /// Write scratch bytes to dst, storing overflow in pending.
    fn flush_scratch(
        &mut self,
        scratch: &[u8],
        dst: &mut [u8],
        dst_pos: &mut usize,
    ) -> Result<(), EncoderResult> {
        let available = dst.len() - *dst_pos;
        if scratch.len() <= available {
            dst[*dst_pos..*dst_pos + scratch.len()].copy_from_slice(scratch);
            *dst_pos += scratch.len();
            Ok(())
        } else {
            dst[*dst_pos..*dst_pos + available].copy_from_slice(&scratch[..available]);
            *dst_pos += available;
            let overflow = scratch.len() - available;
            debug_assert!(overflow <= self.pending.len());
            self.pending[..overflow].copy_from_slice(&scratch[available..]);
            self.pending_start = 0;
            self.pending_end = overflow;
            Err(EncoderResult::OutputFull)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jnta_encode_roundtrip() {
        let encoded = jnta_encode("高", ConversionMode::Men1).unwrap();
        assert_eq!(&encoded, &[0x39, 0x62]);

        let encoded = jnta_encode("ジャンクロードヴァンダム", ConversionMode::Men1).unwrap();
        assert_eq!(encoded.len(), 24); // 12 chars × 2 bytes
    }

    #[test]
    fn test_jnta_encode_unmappable_error() {
        let err = jnta_encode("高A低", ConversionMode::Men1).unwrap_err();
        assert!(
            matches!(err, EncoderResult::Unmappable { ch: 'A', position } if position == "高".len())
        );
    }

    #[test]
    fn test_encoder_men1_basic() {
        let mut encoder = Encoder::new(ConversionMode::Men1, 0x20);
        let mut dst = [0u8; 64];
        let (result, read, written) =
            encoder.encode_from_utf8_without_replacement("高", &mut dst, true);
        assert_eq!(result, EncoderResult::InputEmpty);
        assert_eq!(read, "高".len());
        assert_eq!(written, 2);
        // 高 = 1-25-66 → ku=25, ten=66 → 0x20+25=0x39, 0x20+66=0x62
        assert_eq!(&dst[..2], &[0x39, 0x62]);
    }

    #[test]
    fn test_encoder_siso_plane_switching() {
        let mut encoder = Encoder::new(ConversionMode::Siso, 0x20);
        let input = "ジャンクロードヴァンダム";
        let mut dst = [0u8; 128];
        let (result, read, written) =
            encoder.encode_from_utf8_without_replacement(input, &mut dst, true);
        assert_eq!(result, EncoderResult::InputEmpty);
        assert_eq!(read, input.len());
        // All plane 1 chars, so no shift bytes should be emitted (initial state is plane 1)
        assert_eq!(written, 24); // 12 chars × 2 bytes each
    }

    #[test]
    fn test_encoder_jisx0208_rejects_0213() {
        let mut encoder = Encoder::new(ConversionMode::Jisx0208, 0x20);
        let mut dst = [0u8; 64];
        // 偀 is a JIS X 0213 character
        let (result, _, _) = encoder.encode_from_utf8_without_replacement("偀", &mut dst, true);
        assert!(matches!(result, EncoderResult::Unmappable { .. }));
    }

    #[test]
    fn test_encoder_jisx0208_translit() {
        let mut encoder = Encoder::new(ConversionMode::Jisx0208Translit, 0x20);
        let mut dst = [0u8; 128];
        // 偀 → transliterated via tx_jis
        let (result, _read, written) =
            encoder.encode_from_utf8_without_replacement("偀", &mut dst, true);
        // Should succeed since it has tx_jis transliteration
        assert_eq!(result, EncoderResult::InputEmpty);
        assert!(written > 0);
    }

    #[test]
    fn test_encoder_with_replacement() {
        let mut encoder = Encoder::new(ConversionMode::Jisx0208, 0x20);
        let mut dst = [0u8; 128];
        let (result, _read, _written, _had_replacements) =
            encoder.encode_from_utf8("高ABC", &mut dst, true);
        assert_eq!(result, EncoderResult::InputEmpty);
    }

    #[test]
    fn test_encoder_output_full_and_resume() {
        let mut encoder = Encoder::new(ConversionMode::Men1, 0x20);
        let mut dst = [0u8; 1];
        let (result, _read, written) =
            encoder.encode_from_utf8_without_replacement("高", &mut dst, true);
        assert_eq!(result, EncoderResult::OutputFull);
        assert_eq!(written, 1);

        // Resume with more space
        let mut dst2 = [0u8; 16];
        let (result2, _read2, written2) =
            encoder.encode_from_utf8_without_replacement("", &mut dst2, true);
        assert_eq!(result2, EncoderResult::InputEmpty);
        assert_eq!(written2, 1); // remaining byte
    }

    #[test]
    fn test_encoder_multi_codepoint() {
        let mut encoder = Encoder::new(ConversionMode::Men1, 0x20);
        let mut dst = [0u8; 64];
        // ˩˥ (U+02E9 U+02E5) → state machine should combine to MenKuTen(1, 11, 69)
        let (result, _read, written) =
            encoder.encode_from_utf8_without_replacement("\u{2e9}\u{2e5}", &mut dst, true);
        assert_eq!(result, EncoderResult::InputEmpty);
        assert_eq!(written, 2);
        // 1-11-69 → ku=11, ten=69 → 0x20+11=0x2b, 0x20+69=0x65
        assert_eq!(&dst[..2], &[0x2b, 0x65]);
    }

    #[test]
    fn test_encoder_multi_codepoint_no_match() {
        let mut encoder = Encoder::new(ConversionMode::Men1, 0x20);
        let mut dst = [0u8; 64];
        // ˩ˤ (U+02E9 U+02E4) → state machine won't match, flush individually
        // ˩ = 1-11-68, ˤ is unmappable
        let (result, _read, written) =
            encoder.encode_from_utf8_without_replacement("\u{2e9}\u{2e4}", &mut dst, true);
        assert_eq!(written, 2); // ˩ encoded
        assert!(matches!(
            result,
            EncoderResult::Unmappable { ch: '\u{2e4}', .. }
        ));
    }

    #[test]
    fn test_encoder_roundtrip_bytes_match_existing() {
        use crate::codec::jis::{
            MenKuTenResultIteratorMixin, UniToJNTAMappingResultIteratorMixin, convert_uni_to_jis,
        };

        let input = "ジャンクロードヴァンダム";

        // Existing iterator-based encoding
        let mut expected_bytes = Vec::<u8>::new();
        for c in convert_uni_to_jis(input.chars())
            .to_men_ku_ten()
            .to_iso2022(0x20, false)
        {
            c.unwrap().write_into(&mut expected_bytes).unwrap();
        }

        // New buffer-based encoding
        let mut encoder = Encoder::new(ConversionMode::Men1, 0x20);
        let mut dst = [0u8; 128];
        let (result, _, written) =
            encoder.encode_from_utf8_without_replacement(input, &mut dst, true);
        assert_eq!(result, EncoderResult::InputEmpty);
        assert_eq!(&dst[..written], &expected_bytes[..]);
    }
}
