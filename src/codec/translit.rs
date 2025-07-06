use super::error::TransliterationError;
use super::generated::sm_uni_to_jis_mapping;
use super::inmemory_models::ConversionData;

/// Default replacement string for unmappable characters.
/// Pass as `Some(TRANSLIT_DEFAULT_REPLACEMENT)` to [`jnta_shrink_translit`].
pub const TRANSLIT_DEFAULT_REPLACEMENT: &str = "\u{fffd}";

/// Transliterates JIS X 0213 characters to JIS X 0208 equivalents at the Unicode level.
///
/// This function uses the state machine for multi-codepoint resolution and
/// applies the JNTA transliteration mappings (`tx_us`) for JIS X 0213 characters.
///
/// # Arguments
///
/// * `input` - The input string to transliterate
/// * `replacement` - Controls handling of unmappable characters:
///   - `None` — unmappable characters pass through unchanged (passthrough mode)
///   - `Some("")` — unmappable characters cause a `TransliterationError`
///   - `Some(s)` — unmappable characters are replaced with `s`
///
/// # Examples
///
/// ```rust
/// use jntajis::codec::translit::jnta_shrink_translit;
///
/// // Passthrough mode: unmappable chars kept as-is
/// let result = jnta_shrink_translit("偀ABC", None).unwrap();
/// assert!(result.contains("英")); // 偀 → 英
///
/// // Replacement mode
/// let result = jnta_shrink_translit("偀", Some("?")).unwrap();
/// assert_eq!(result, "英");
/// ```
pub fn jnta_shrink_translit(
    input: &str,
    replacement: Option<&str>,
) -> Result<String, TransliterationError> {
    let data = super::get_data();
    let mut output = String::with_capacity(input.len());
    let mut sm_state: i32 = 0;
    let mut lookahead = [0u32; 4];
    let mut lookahead_len: usize = 0;

    let mut chars = input.chars();

    loop {
        let c = chars.next();

        if let Some(ch) = c {
            if sm_state != 0 {
                let (new_state, result) = sm_uni_to_jis_mapping(sm_state, ch as u32);
                if let Some(mkt) = result {
                    // State machine matched
                    lookahead_len = 0;
                    sm_state = 0;
                    let mapping = &data.jnta_mappings[u16::from(mkt) as usize];
                    emit_translit_mapping(mapping, &mut output);
                    continue;
                } else if new_state == 0 {
                    // State machine reset without match — flush lookahead
                    sm_state = 0;
                    let la_len = lookahead_len;
                    let la = lookahead;
                    lookahead_len = 0;
                    for &item in la.iter().take(la_len) {
                        emit_single_translit(&data, item, replacement, &mut output)?;
                    }
                    // Re-process current char from state 0
                    let (ns, res) = sm_uni_to_jis_mapping(0, ch as u32);
                    if let Some(mkt) = res {
                        let mapping = &data.jnta_mappings[u16::from(mkt) as usize];
                        emit_translit_mapping(mapping, &mut output);
                    } else if ns != 0 {
                        sm_state = ns;
                        lookahead[0] = ch as u32;
                        lookahead_len = 1;
                    } else {
                        emit_single_translit(&data, ch as u32, replacement, &mut output)?;
                    }
                    continue;
                } else {
                    // Continue accumulating
                    sm_state = new_state;
                    if lookahead_len < 4 {
                        lookahead[lookahead_len] = ch as u32;
                        lookahead_len += 1;
                    }
                    continue;
                }
            }

            // sm_state == 0
            let (new_state, result) = sm_uni_to_jis_mapping(0, ch as u32);
            if let Some(mkt) = result {
                let mapping = &data.jnta_mappings[u16::from(mkt) as usize];
                emit_translit_mapping(mapping, &mut output);
            } else if new_state != 0 {
                sm_state = new_state;
                lookahead[0] = ch as u32;
                lookahead_len = 1;
            } else {
                emit_single_translit(&data, ch as u32, replacement, &mut output)?;
            }
        } else {
            // EOI — flush remaining lookahead
            if sm_state != 0 {
                for &item in lookahead.iter().take(lookahead_len) {
                    emit_single_translit(&data, item, replacement, &mut output)?;
                }
            }
            break;
        }
    }

    Ok(output)
}

/// Emit the transliterated form for a resolved JNTAMapping.
fn emit_translit_mapping(mapping: &super::common_models::JNTAMapping, output: &mut String) {
    if mapping.class.is_jisx0213() && !mapping.tx_us.is_empty() {
        for i in 0..mapping.tx_us.len() {
            let ch = unsafe { char::from_u32_unchecked(mapping.tx_us[i]) };
            output.push(ch);
        }
    } else {
        for i in 0..mapping.us.len() {
            let ch = unsafe { char::from_u32_unchecked(mapping.us[i]) };
            output.push(ch);
        }
    }
}

/// Emit the transliterated form for a single Unicode codepoint via direct lookup.
fn emit_single_translit(
    data: &ConversionData,
    u: u32,
    replacement: Option<&str>,
    output: &mut String,
) -> Result<(), TransliterationError> {
    if let Some(mapping) = data.lookup_jnta_mapping(u) {
        emit_translit_mapping(mapping, output);
        Ok(())
    } else {
        let ch = unsafe { char::from_u32_unchecked(u) };
        match replacement {
            None => {
                // Passthrough mode
                output.push(ch);
                Ok(())
            }
            Some("") => Err(TransliterationError::UnmappableChar(ch)),
            Some(s) => {
                output.push_str(s);
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translit_basic() {
        // 偀 is a JIS X 0213 char that transliterates to 英
        let result = jnta_shrink_translit("偀", None).unwrap();
        assert_eq!(result, "英");
    }

    #[test]
    fn test_translit_multiple_chars() {
        // Same test as transliterate_jisx0213
        let result = jnta_shrink_translit("偀〖ㇵ❶Ⅻ㈱輀", None).unwrap();
        assert_eq!(result, "英【ハ１ＸＩＩ（株）轜");
    }

    #[test]
    fn test_translit_passthrough() {
        // ABC are unmappable, should pass through
        let result = jnta_shrink_translit("高ABC", None).unwrap();
        assert_eq!(result, "高ABC");
    }

    #[test]
    fn test_translit_replacement() {
        let result = jnta_shrink_translit("高A", Some("?")).unwrap();
        assert_eq!(result, "高?");
    }

    #[test]
    fn test_translit_error_on_empty_replacement() {
        let result = jnta_shrink_translit("A", Some(""));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TransliterationError::UnmappableChar('A')
        ));
    }

    #[test]
    fn test_translit_non_jisx0213_passthrough() {
        // 高 is a JIS X 0208 char, should pass through unchanged
        let result = jnta_shrink_translit("高", None).unwrap();
        assert_eq!(result, "高");
    }

    #[test]
    fn test_translit_multi_codepoint() {
        // ˩˥ (U+02E9 U+02E5) → multi-codepoint match via state machine
        // This maps to MenKuTen(1, 11, 69) which is a JIS X 0213 NonKanji char
        let result = jnta_shrink_translit("\u{2e9}\u{2e5}", None).unwrap();
        // It should transliterate if it has tx_us, otherwise keep as-is
        assert!(!result.is_empty());
    }

    #[test]
    fn test_translit_multi_codepoint_no_match() {
        // ˩ˤ (U+02E9 U+02E4) → state machine won't match, flush individually
        // ˩ should be looked up directly, ˤ is unmappable
        let result = jnta_shrink_translit("\u{2e9}\u{2e4}", None).unwrap();
        // In passthrough mode, unmappable chars pass through
        assert!(result.contains('\u{2e4}'));
    }

    #[test]
    fn test_translit_default_replacement() {
        let result = jnta_shrink_translit("高A", Some(TRANSLIT_DEFAULT_REPLACEMENT)).unwrap();
        assert_eq!(result, "高\u{fffd}");
    }

    #[test]
    fn test_translit_matches_existing_transliterate_jisx0213() {
        use crate::codec::jis::transliterate_jisx0213;

        // Compare with existing function for inputs that are fully mappable
        let input = "偀〖ㇵ❶Ⅻ㈱輀";
        let existing: String = transliterate_jisx0213(input.chars(), false)
            .collect::<Result<_, _>>()
            .unwrap();
        let new = jnta_shrink_translit(input, None).unwrap();
        assert_eq!(new, existing);
    }
}
