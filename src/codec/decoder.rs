use std::sync::Arc;

use super::ConversionMode;
use super::common_models::JISCharacterClass;
use super::error::DecoderResult;
use super::inmemory_models::ConversionData;

/// Decodes a complete JIS byte sequence into a UTF-8 string in a single call.
///
/// Uses `code_offset = 0x20` (standard ISO-2022 offset, matching the Python
/// `jntajis` library).
///
/// # Errors
///
/// Returns [`DecoderResult::Malformed`] if the input contains a malformed byte
/// sequence under the given [`ConversionMode`].
pub fn jnta_decode(input: &[u8], mode: ConversionMode) -> Result<String, DecoderResult> {
    let mut decoder = Decoder::new(mode, 0x20);
    let mut out = String::with_capacity(input.len());
    let mut buf = [0u8; 1024];
    let mut remaining = input;

    loop {
        let (result, read, written) =
            decoder.decode_to_utf8_without_replacement(remaining, &mut buf, true);
        // SAFETY: the decoder produces valid UTF-8 from its mapping tables.
        out.push_str(unsafe { std::str::from_utf8_unchecked(&buf[..written]) });

        match result {
            DecoderResult::InputEmpty => return Ok(out),
            DecoderResult::OutputFull => {
                remaining = &remaining[read..];
            }
            DecoderResult::Malformed { len, .. } => {
                let position = input.len() - remaining.len() + read - len as usize;
                return Err(DecoderResult::Malformed { len, position });
            }
        }
    }
}

/// Buffer-based JIS decoder following the encoding_rs pattern.
///
/// Decodes JIS byte sequences into UTF-8 text using the specified
/// [`ConversionMode`].
pub struct Decoder {
    data: Arc<ConversionData>,
    mode: ConversionMode,
    code_offset: u8,
    shift_offset: u16,        // 0 = plane 1, 94*94 = plane 2
    partial_byte: Option<u8>, // first byte of incomplete pair
    pending: [u8; 16],        // UTF-8 output that didn't fit
    pending_start: usize,
    pending_end: usize,
}

impl Decoder {
    /// Creates a new decoder with the given conversion mode and code offset.
    ///
    /// The `code_offset` is typically `0x20` for ISO-2022-JP style encoding.
    pub fn new(mode: ConversionMode, code_offset: u8) -> Self {
        Self {
            data: super::get_data(),
            mode,
            code_offset,
            shift_offset: 0,
            partial_byte: None,
            pending: [0; 16],
            pending_start: 0,
            pending_end: 0,
        }
    }

    /// Resets the decoder to its initial state.
    pub fn reset(&mut self) {
        self.shift_offset = 0;
        self.partial_byte = None;
        self.pending_start = 0;
        self.pending_end = 0;
    }

    /// Decodes JIS byte input into UTF-8, stopping on malformed sequences.
    ///
    /// Returns `(result, bytes_read, bytes_written)`.
    ///
    /// When `last` is `true`, an incomplete byte pair is reported as malformed.
    pub fn decode_to_utf8_without_replacement(
        &mut self,
        src: &[u8],
        dst: &mut [u8],
        last: bool,
    ) -> (DecoderResult, usize, usize) {
        let mut src_pos = 0;
        let mut dst_pos = 0;

        // Drain pending output first
        while self.pending_start < self.pending_end && dst_pos < dst.len() {
            dst[dst_pos] = self.pending[self.pending_start];
            self.pending_start += 1;
            dst_pos += 1;
        }
        if self.pending_start < self.pending_end {
            return (DecoderResult::OutputFull, 0, dst_pos);
        }

        while src_pos < src.len() {
            let b = src[src_pos];

            // Check for plane-switching shift bytes.
            // Convention: 0x0E selects plane 1, 0x0F selects plane 2.
            // (Note: this is opposite to ISO 2022 SO/SI naming.)
            if b == 0x0e || b == 0x0f {
                if self.mode != ConversionMode::Siso {
                    return (
                        DecoderResult::Malformed {
                            len: 1,
                            position: src_pos,
                        },
                        src_pos + 1,
                        dst_pos,
                    );
                }
                if let Some(_partial) = self.partial_byte.take() {
                    // Shift byte in the middle of a pair
                    return (
                        DecoderResult::Malformed {
                            len: 1,
                            position: src_pos.saturating_sub(1),
                        },
                        src_pos,
                        dst_pos,
                    );
                }
                self.shift_offset = if b == 0x0e { 0 } else { 94 * 94 };
                src_pos += 1;
                continue;
            }

            let off = self.code_offset;
            let lo = off + 1;
            let hi = off + 94;

            if b < lo || b > hi {
                if let Some(_partial) = self.partial_byte.take() {
                    return (
                        DecoderResult::Malformed {
                            len: 1,
                            position: src_pos.saturating_sub(1),
                        },
                        src_pos,
                        dst_pos,
                    );
                }
                return (
                    DecoderResult::Malformed {
                        len: 1,
                        position: src_pos,
                    },
                    src_pos + 1,
                    dst_pos,
                );
            }

            if let Some(first) = self.partial_byte.take() {
                // Complete the pair
                let c0 = (first - off - 1) as u16;
                let c1 = (b - off - 1) as u16;
                let jis_index = self.shift_offset + c0 * 94 + c1;
                src_pos += 1;

                if jis_index as usize >= self.data.jnta_mappings.len() {
                    return (
                        DecoderResult::Malformed {
                            len: 2,
                            position: src_pos - 2,
                        },
                        src_pos,
                        dst_pos,
                    );
                }

                let mapping = &self.data.jnta_mappings[jis_index as usize];

                if matches!(mapping.class, JISCharacterClass::Reserved) {
                    return (
                        DecoderResult::Malformed {
                            len: 2,
                            position: src_pos - 2,
                        },
                        src_pos,
                        dst_pos,
                    );
                }

                // Mode-based filtering
                match self.mode {
                    ConversionMode::Jisx0208 => {
                        if !mapping.class.is_jisx0208() {
                            return (
                                DecoderResult::Malformed {
                                    len: 2,
                                    position: src_pos - 2,
                                },
                                src_pos,
                                dst_pos,
                            );
                        }
                    }
                    ConversionMode::Men1 => {
                        // Only plane 1 — shift_offset must be 0
                        if self.shift_offset != 0 {
                            return (
                                DecoderResult::Malformed {
                                    len: 2,
                                    position: src_pos - 2,
                                },
                                src_pos,
                                dst_pos,
                            );
                        }
                    }
                    _ => {}
                }

                // Encode Unicode codepoints as UTF-8.
                // Max: tx_us has capacity 4, us has capacity 2; each codepoint
                // is at most 4 UTF-8 bytes → 16 bytes worst case.
                let mut scratch = [0u8; 16];
                let mut scratch_len = 0;
                let use_tx = self.mode == ConversionMode::Jisx0208Translit
                    && mapping.class.is_jisx0213()
                    && !mapping.tx_us.is_empty();
                if use_tx {
                    for i in 0..mapping.tx_us.len() {
                        // SAFETY: values in tx_us originate from generated conversion
                        // data and are guaranteed to be valid Unicode codepoints.
                        let ch = unsafe { char::from_u32_unchecked(mapping.tx_us[i]) };
                        let len = ch.len_utf8();
                        debug_assert!(scratch_len + len <= scratch.len());
                        ch.encode_utf8(&mut scratch[scratch_len..]);
                        scratch_len += len;
                    }
                } else {
                    for i in 0..mapping.us.len() {
                        // SAFETY: values in us originate from generated conversion
                        // data and are guaranteed to be valid Unicode codepoints.
                        let ch = unsafe { char::from_u32_unchecked(mapping.us[i]) };
                        let len = ch.len_utf8();
                        debug_assert!(scratch_len + len <= scratch.len());
                        ch.encode_utf8(&mut scratch[scratch_len..]);
                        scratch_len += len;
                    }
                }

                let available = dst.len() - dst_pos;
                if scratch_len <= available {
                    dst[dst_pos..dst_pos + scratch_len].copy_from_slice(&scratch[..scratch_len]);
                    dst_pos += scratch_len;
                } else {
                    dst[dst_pos..dst_pos + available].copy_from_slice(&scratch[..available]);
                    dst_pos += available;
                    let overflow = scratch_len - available;
                    debug_assert!(overflow <= self.pending.len());
                    self.pending[..overflow].copy_from_slice(&scratch[available..scratch_len]);
                    self.pending_start = 0;
                    self.pending_end = overflow;
                    return (DecoderResult::OutputFull, src_pos, dst_pos);
                }
            } else {
                // Store first byte of pair
                self.partial_byte = Some(b);
                src_pos += 1;
            }
        }

        // End of input
        if last && self.partial_byte.is_some() {
            self.partial_byte = None;
            return (
                DecoderResult::Malformed {
                    len: 1,
                    position: src_pos.saturating_sub(1),
                },
                src_pos,
                dst_pos,
            );
        }

        (DecoderResult::InputEmpty, src_pos, dst_pos)
    }

    /// Decodes JIS byte input into UTF-8, replacing malformed sequences with U+FFFD.
    ///
    /// Returns `(result, bytes_read, bytes_written, had_replacements)`.
    pub fn decode_to_utf8(
        &mut self,
        src: &[u8],
        dst: &mut [u8],
        last: bool,
    ) -> (DecoderResult, usize, usize, bool) {
        let mut total_read = 0;
        let mut total_written = 0;
        let mut had_replacements = false;

        loop {
            let (result, read, written) = self.decode_to_utf8_without_replacement(
                &src[total_read..],
                &mut dst[total_written..],
                last,
            );
            total_read += read;
            total_written += written;

            match result {
                DecoderResult::InputEmpty => {
                    return (
                        DecoderResult::InputEmpty,
                        total_read,
                        total_written,
                        had_replacements,
                    );
                }
                DecoderResult::OutputFull => {
                    return (
                        DecoderResult::OutputFull,
                        total_read,
                        total_written,
                        had_replacements,
                    );
                }
                DecoderResult::Malformed { .. } => {
                    had_replacements = true;
                    // Write U+FFFD replacement character (3 bytes in UTF-8)
                    let rep_bytes = "\u{FFFD}".as_bytes();
                    let available = dst.len() - total_written;
                    if rep_bytes.len() <= available {
                        dst[total_written..total_written + rep_bytes.len()]
                            .copy_from_slice(rep_bytes);
                        total_written += rep_bytes.len();
                    } else {
                        // Partially write what fits, buffer the rest in pending
                        dst[total_written..total_written + available]
                            .copy_from_slice(&rep_bytes[..available]);
                        total_written += available;
                        let overflow = rep_bytes.len() - available;
                        self.pending[..overflow].copy_from_slice(&rep_bytes[available..]);
                        self.pending_start = 0;
                        self.pending_end = overflow;
                        return (
                            DecoderResult::OutputFull,
                            total_read,
                            total_written,
                            had_replacements,
                        );
                    }
                    continue;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::encoder::{Encoder, jnta_encode};

    #[test]
    fn test_jnta_decode_roundtrip() {
        let encoded = jnta_encode("高", ConversionMode::Men1).unwrap();
        let decoded = jnta_decode(&encoded, ConversionMode::Men1).unwrap();
        assert_eq!(decoded, "高");

        let input = "ジャンクロードヴァンダム";
        let encoded = jnta_encode(input, ConversionMode::Men1).unwrap();
        let decoded = jnta_decode(&encoded, ConversionMode::Men1).unwrap();
        assert_eq!(decoded, input);
    }

    #[test]
    fn test_jnta_decode_malformed_error() {
        // 0x01 is out of range for code_offset=0x20
        let err = jnta_decode(&[0x39, 0x62, 0x01, 0x39], ConversionMode::Men1).unwrap_err();
        assert!(matches!(
            err,
            DecoderResult::Malformed {
                len: 1,
                position: 2
            }
        ));
    }

    #[test]
    fn test_decoder_basic() {
        // Encode "高" first, then decode
        let mut encoder = Encoder::new(ConversionMode::Men1, 0x20);
        let mut encoded = [0u8; 16];
        let (er, _, ew) = encoder.encode_from_utf8_without_replacement("高", &mut encoded, true);
        assert_eq!(er, crate::codec::error::EncoderResult::InputEmpty);

        let mut decoder = Decoder::new(ConversionMode::Men1, 0x20);
        let mut decoded = [0u8; 16];
        let (dr, _dread, dw) =
            decoder.decode_to_utf8_without_replacement(&encoded[..ew], &mut decoded, true);
        assert_eq!(dr, DecoderResult::InputEmpty);
        let decoded_str = std::str::from_utf8(&decoded[..dw]).unwrap();
        assert_eq!(decoded_str, "高");
    }

    #[test]
    fn test_decoder_roundtrip() {
        let input = "ジャンクロードヴァンダム";
        let mut encoder = Encoder::new(ConversionMode::Men1, 0x20);
        let mut encoded = [0u8; 128];
        let (er, _, ew) = encoder.encode_from_utf8_without_replacement(input, &mut encoded, true);
        assert_eq!(er, crate::codec::error::EncoderResult::InputEmpty);

        let mut decoder = Decoder::new(ConversionMode::Men1, 0x20);
        let mut decoded = [0u8; 256];
        let (dr, _, dw) =
            decoder.decode_to_utf8_without_replacement(&encoded[..ew], &mut decoded, true);
        assert_eq!(dr, DecoderResult::InputEmpty);
        let decoded_str = std::str::from_utf8(&decoded[..dw]).unwrap();
        assert_eq!(decoded_str, input);
    }

    #[test]
    fn test_decoder_siso() {
        // Encode with SISO, then decode with SISO
        let mut encoder = Encoder::new(ConversionMode::Siso, 0x20);
        let input = "高";
        let mut encoded = [0u8; 128];
        let (er, _, ew) = encoder.encode_from_utf8_without_replacement(input, &mut encoded, true);
        assert_eq!(er, crate::codec::error::EncoderResult::InputEmpty);

        let mut decoder = Decoder::new(ConversionMode::Siso, 0x20);
        let mut decoded = [0u8; 128];
        let (dr, _, dw) =
            decoder.decode_to_utf8_without_replacement(&encoded[..ew], &mut decoded, true);
        assert_eq!(dr, DecoderResult::InputEmpty);
        assert_eq!(std::str::from_utf8(&decoded[..dw]).unwrap(), input);
    }

    #[test]
    fn test_decoder_malformed_odd_byte() {
        let mut decoder = Decoder::new(ConversionMode::Men1, 0x20);
        let mut decoded = [0u8; 16];
        // Single byte (incomplete pair) at end
        let (dr, _, _) = decoder.decode_to_utf8_without_replacement(&[0x39], &mut decoded, true);
        assert!(matches!(dr, DecoderResult::Malformed { len: 1, .. }));
    }

    #[test]
    fn test_decoder_malformed_out_of_range() {
        let mut decoder = Decoder::new(ConversionMode::Men1, 0x20);
        let mut decoded = [0u8; 16];
        // Byte outside valid range (0x20 offset → valid range 0x21..0x7e)
        let (dr, _, _) =
            decoder.decode_to_utf8_without_replacement(&[0x01, 0x39], &mut decoded, true);
        assert!(matches!(dr, DecoderResult::Malformed { len: 1, .. }));
    }

    #[test]
    fn test_decoder_partial_across_buffer() {
        let mut encoder = Encoder::new(ConversionMode::Men1, 0x20);
        let mut encoded = [0u8; 16];
        let (er, _, ew) = encoder.encode_from_utf8_without_replacement("高", &mut encoded, true);
        assert_eq!(er, crate::codec::error::EncoderResult::InputEmpty);
        assert_eq!(ew, 2);

        // Feed one byte at a time
        let mut decoder = Decoder::new(ConversionMode::Men1, 0x20);
        let mut decoded = [0u8; 16];
        let (dr1, read1, dw1) =
            decoder.decode_to_utf8_without_replacement(&encoded[..1], &mut decoded, false);
        assert_eq!(dr1, DecoderResult::InputEmpty);
        assert_eq!(read1, 1);
        assert_eq!(dw1, 0); // No output yet

        let (dr2, read2, dw2) =
            decoder.decode_to_utf8_without_replacement(&encoded[1..2], &mut decoded, true);
        assert_eq!(dr2, DecoderResult::InputEmpty);
        assert_eq!(read2, 1);
        assert_eq!(std::str::from_utf8(&decoded[..dw2]).unwrap(), "高");
    }

    #[test]
    fn test_decoder_siso_shift_bytes() {
        let mut decoder = Decoder::new(ConversionMode::Siso, 0x20);
        let mut decoded = [0u8; 128];

        // 0x0e selects plane 1, 0x0f selects plane 2.
        // Start with plane 1 (default), encode 高 (1-25-66)
        let encoded = &[0x39, 0x62]; // ku=25→0x39, ten=66→0x62
        let (dr, _, dw) = decoder.decode_to_utf8_without_replacement(encoded, &mut decoded, true);
        assert_eq!(dr, DecoderResult::InputEmpty);
        assert_eq!(std::str::from_utf8(&decoded[..dw]).unwrap(), "高");
    }

    #[test]
    fn test_decoder_non_siso_rejects_shift_bytes() {
        let mut decoder = Decoder::new(ConversionMode::Men1, 0x20);
        let mut decoded = [0u8; 16];
        let (dr, _, _) =
            decoder.decode_to_utf8_without_replacement(&[0x0e, 0x39, 0x62], &mut decoded, true);
        assert!(matches!(dr, DecoderResult::Malformed { len: 1, .. }));
    }

    #[test]
    fn test_decoder_with_replacement() {
        let mut decoder = Decoder::new(ConversionMode::Men1, 0x20);
        let mut decoded = [0u8; 128];
        // Out-of-range byte followed by valid pair
        let encoded = &[0x01, 0x39, 0x62];
        let (dr, _read, dw, had_repl) = decoder.decode_to_utf8(encoded, &mut decoded, true);
        assert_eq!(dr, DecoderResult::InputEmpty);
        assert!(had_repl);
        let s = std::str::from_utf8(&decoded[..dw]).unwrap();
        assert!(s.starts_with('\u{FFFD}'));
        assert!(s.contains('高'));
    }

    #[test]
    fn test_decoder_replacement_output_full() {
        // FFFD is 3 bytes (0xEF 0xBF 0xBD). Use a tiny buffer so it doesn't fit.
        let mut decoder = Decoder::new(ConversionMode::Men1, 0x20);
        let encoded = &[0x01, 0x39, 0x62]; // malformed byte + valid pair

        // Buffer of 1 byte: can't fit FFFD (3 bytes), only 1 byte written
        let mut decoded = [0u8; 1];
        let (dr, read1, dw1, had_repl) = decoder.decode_to_utf8(encoded, &mut decoded, true);
        assert_eq!(dr, DecoderResult::OutputFull);
        assert!(had_repl);
        assert_eq!(dw1, 1); // partial FFFD

        // Resume: remaining FFFD bytes + the valid pair should come through
        let mut decoded2 = [0u8; 128];
        let (dr2, _read2, dw2, _) = decoder.decode_to_utf8(&encoded[read1..], &mut decoded2, true);
        assert_eq!(dr2, DecoderResult::InputEmpty);
        // Should contain the rest of FFFD + 高
        let combined: Vec<u8> = decoded[..dw1]
            .iter()
            .chain(decoded2[..dw2].iter())
            .copied()
            .collect();
        let s = std::str::from_utf8(&combined).unwrap();
        assert!(s.starts_with('\u{FFFD}'));
        assert!(s.contains('高'));
    }
}
