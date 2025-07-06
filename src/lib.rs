//! # jntajis
//!
//! A Rust port of [jntajis-python](https://github.com/opencollector/jntajis-python),
//! providing character transliteration functionality for Japanese text processing.
//!
//! ## What is JNTAJIS?
//!
//! JNTAJIS-rs is a transliteration library specifically designed for dealing with three
//! different character sets: JIS X 0208, JIS X 0213, and Unicode.
//!
//! ```rust
//! use jntajis::codec::mj_shrink::{MJShrinkSchemes, mj_shrink_candidates};
//!
//! // Get shrink candidates for a character variant
//! let candidates: Vec<String> = mj_shrink_candidates("髙島屋", MJShrinkSchemes::ALL)
//!     .take(5)
//!     .collect();
//! println!("{:?}", candidates); // outputs variations including "高島屋"
//! ```
//!
//! To that end, this library refers to three different character tables:
//!
//! ## Character Tables
//!
//! ### MJ Character Table (MJ文字一覧表)
//!
//! The MJ character table defines a vast set of kanji (漢字) characters used in
//! information processing of Japanese texts, initially developed by the
//! Information-technology Promotion Agency.
//!
//! ### MJ Shrink Conversion Map (MJ縮退マップ)
//!
//! The MJ shrink conversion map was developed alongside the MJ character table for
//! the sake of interoperability between MJ-aware systems and systems based on Unicode.
//! It is used to transliterate complex, less-frequently-used character variants to
//! commonly-used, more-used ones.
//!
//! ### NTA Shrink Conversion Map (国税庁JIS縮退マップ)
//!
//! The NTA shrink conversion map was developed by Japan National Tax Agency to
//! canonicalize user inputs for its corporation number search service provided as
//! a public web API. This maps JIS level 3 and 4 characters to JIS level 1 and 2
//! characters (i.e. characters defined in JIS X 0208).
//!
//! Note that not all level 3 and level 4 characters have level 1 and 2 counterparts.
//! Also note that some level 3 and 4 characters don't map to a single character.
//! Instead, they map to sequences of two or more characters.
//!
//! ## Examples of Transliteration
//!
//! | Glyph | MJ code | Unicode | JIS X 0213 | Glyph* | MJ code* | JIS X 0208* | Transliterator |
//! |-------|---------|---------|------------|--------|----------|-------------|----------------|
//! | 棃    | MJ014031| U+68C3  | 2-14-90    | 梨     | MJ014007 | 1-45-92     | MJ / JNTA      |
//! | 﨑    | MJ030196| U+FA11  | 1-47-82    | 崎     | MJ010541 | 1-26-74     | MJ / JNTA      |
//! | 髙    | MJ028902| U+9AD9  | N/A        | 高     | MJ028901 | 1-25-66     | MJ             |
//!
//! *Columns with a * symbol denote the transliteration result.*
//!
//! ## Conversion Process
//!
//! ### JNTA Transliteration
//!
//! As every JIS X 0213 character maps to its Unicode counterpart, the conversion
//! is done only with the single JNTA character mappings table.
//!
//! ### MJ Transliteration
//!
//! Transliteration is done in two phases:
//!
//! 1. **Conversion from Unicode to MJ character mappings.**
//!    
//!    While not all characters in the MJ characters table map to Unicode, each MJ
//!    code has different shrink mappings. Because of this, the transliterator tries
//!    to convert Unicode codepoints to MJ codes first.
//!
//! 2. **Transliteration by MJ shrink mappings.**
//!    
//!    The transliteration result as a string isn't necessarily single as some MJ
//!    codes have more than one transliteration candidate. This happens because:
//!    - a) a Unicode codepoint may map to multiple MJ codes and
//!    - b) multiple transliteration schemes are designated to a single MJ code.
//!
//! ## Data Sources and Licensing
//!
//! This library makes use of the data from the following entities:
//!
//! ### JIS shrink conversion mappings (国税庁: JIS縮退マップ)
//! - **Publisher:** National Tax Agency
//! - **Author:** National Tax Agency
//! - **Source:** <https://www.houjin-bangou.nta.go.jp/download/>
//! - **License:** CC BY 4.0
//!
//! ### MJ character table (文字情報技術促進協議会: MJ文字一覧表)
//! - **Publisher:** Character Information Technology Promotion Council (CITPC)
//! - **Author:** Information-technology Promotion Agency (IPA)
//! - **Source:** <https://moji.or.jp/mojikiban/mjlist/>
//! - **License:** CC BY-SA 2.1 JP
//!
//! ### MJ shrink conversion mappings (文字情報技術促進協議会: MJ縮退マップ)
//! - **Publisher:** Character Information Technology Promotion Council (CITPC)
//! - **Author:** Information-technology Promotion Agency (IPA)
//! - **Source:** <https://moji.or.jp/mojikiban/map/>
//! - **License:** CC BY-SA 2.1 JP

pub mod codec;

// Re-export the new public API
pub use codec::conversion_mode::ConversionMode;
pub use codec::decoder::{Decoder, jnta_decode};
pub use codec::encoder::{Encoder, jnta_encode};
pub use codec::error::{DecoderResult, EncoderResult, TransliterationError};
pub use codec::translit::{TRANSLIT_DEFAULT_REPLACEMENT, jnta_shrink_translit};
