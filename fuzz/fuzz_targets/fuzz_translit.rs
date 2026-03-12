#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

use jntajis::codec::translit::{TRANSLIT_DEFAULT_REPLACEMENT, jnta_shrink_translit};

#[derive(Arbitrary, Debug)]
struct TranslitInput {
    data: String,
    replacement_mode: u8,
}

fuzz_target!(|input: TranslitInput| {
    let replacement = match input.replacement_mode % 3 {
        0 => None,
        1 => Some(""),
        _ => Some(TRANSLIT_DEFAULT_REPLACEMENT),
    };
    let _ = jnta_shrink_translit(&input.data, replacement);
});
