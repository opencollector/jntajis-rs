#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

use jntajis::codec::mj_shrink::{MJShrinkScheme, MJShrinkSchemes, mj_shrink_candidates};

#[derive(Arbitrary, Debug)]
struct MJShrinkInput {
    data: String,
    scheme_bits: u8,
}

fuzz_target!(|input: MJShrinkInput| {
    let mut schemes = MJShrinkSchemes::builder();
    if input.scheme_bits & 1 != 0 {
        schemes = schemes.with(MJShrinkScheme::JISIncorporationUCSUnificationRule);
    }
    if input.scheme_bits & 2 != 0 {
        schemes = schemes.with(MJShrinkScheme::InferenceByReadingAndGlyph);
    }
    if input.scheme_bits & 4 != 0 {
        schemes = schemes.with(MJShrinkScheme::MOJNotice582);
    }
    if input.scheme_bits & 8 != 0 {
        schemes = schemes.with(MJShrinkScheme::MOJFamilyRegisterActRelatedNotice);
    }

    // Consume at most a few candidates to avoid excessive runtime
    let _: Vec<String> = mj_shrink_candidates(&input.data, schemes)
        .take(10)
        .collect();
});
