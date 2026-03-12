#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

use jntajis::{ConversionMode, jnta_decode, jnta_encode};

#[derive(Arbitrary, Debug)]
struct RoundtripInput {
    data: String,
    mode: u8,
}

fn mode_from_u8(v: u8) -> ConversionMode {
    match v % 4 {
        0 => ConversionMode::Siso,
        1 => ConversionMode::Men1,
        2 => ConversionMode::Jisx0208,
        _ => ConversionMode::Jisx0208Translit,
    }
}

fuzz_target!(|input: RoundtripInput| {
    let mode = mode_from_u8(input.mode);

    // Encode then decode - should not panic
    if let Ok(encoded) = jnta_encode(&input.data, mode) {
        let decoded = jnta_decode(&encoded, mode);
        // If encode succeeded, decode should not panic (may still return Err for
        // transliterated modes where the roundtrip isn't bijective)
        let _ = decoded;
    }
});
