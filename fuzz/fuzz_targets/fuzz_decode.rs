#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

use jntajis::{ConversionMode, jnta_decode};

#[derive(Arbitrary, Debug)]
struct DecodeInput {
    data: Vec<u8>,
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

fuzz_target!(|input: DecodeInput| {
    let mode = mode_from_u8(input.mode);
    let _ = jnta_decode(&input.data, mode);
});
