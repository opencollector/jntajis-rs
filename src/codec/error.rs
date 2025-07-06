/// Result type for the encoder.
///
/// `encode_from_utf8` will only ever return `InputEmpty` or `OutputFull`;
/// `Unmappable` is only returned by `encode_from_utf8_without_replacement`.
///
/// Also used as the error type for [`jnta_encode`](super::encoder::jnta_encode),
/// which only ever returns the `Unmappable` variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum EncoderResult {
    /// All input was consumed.
    #[error("all input was consumed")]
    InputEmpty,
    /// The output buffer is full.
    #[error("the output buffer is full")]
    OutputFull,
    /// An unmappable character was encountered at the given byte offset.
    #[error("unmappable character '{ch}' at byte offset {position}")]
    Unmappable { ch: char, position: usize },
}

/// Result type for the decoder (without replacement).
///
/// Also used as the error type for [`jnta_decode`](super::decoder::jnta_decode),
/// which only ever returns the `Malformed` variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum DecoderResult {
    /// All input was consumed.
    #[error("all input was consumed")]
    InputEmpty,
    /// The output buffer is full.
    #[error("the output buffer is full")]
    OutputFull,
    /// A malformed byte sequence was encountered.
    /// `len` is the number of bytes in the malformed sequence.
    /// `position` is the byte offset of the malformed sequence.
    #[error("malformed {len}-byte sequence at byte position {position}")]
    Malformed { len: u8, position: usize },
}

/// Error type for transliteration operations.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum TransliterationError {
    #[error("Unmappable character: {0}")]
    UnmappableChar(char),
}
