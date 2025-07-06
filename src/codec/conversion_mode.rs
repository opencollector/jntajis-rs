/// Conversion mode for JIS encoding/decoding.
///
/// Determines which character set is used and how characters outside
/// the target set are handled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConversionMode {
    /// Full JIS X 0213 with SI/SO escape bytes for plane switching.
    Siso,
    /// JIS X 0213 plane 1 only.
    Men1,
    /// Strict JIS X 0208 (Level 1/2 + NonKanji only).
    Jisx0208,
    /// JIS X 0208 + transliteration via tx_jis for JIS X 0213 characters.
    Jisx0208Translit,
}
