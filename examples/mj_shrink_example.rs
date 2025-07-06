use jntajis::codec::mj_shrink::{MJShrinkScheme, MJShrinkSchemes, mj_shrink_candidates};

fn main() {
    // Example 1: Basic usage with the character '髙' (variant of '高')
    println!("Example 1: Shrink candidates for '髙' (all schemes):");
    let candidates: Vec<String> = mj_shrink_candidates("髙", MJShrinkSchemes::ALL)
        .take(10)
        .collect();
    for (i, candidate) in candidates.iter().enumerate() {
        println!("  {}: {}", i + 1, candidate);
    }
    println!();

    // Example 2: Using specific scheme
    println!("Example 2: Using only JIS incorporation rule:");
    let jis_only =
        MJShrinkSchemes::builder().with(MJShrinkScheme::JISIncorporationUCSUnificationRule);
    let candidates: Vec<String> = mj_shrink_candidates("髙", jis_only).take(5).collect();
    for (i, candidate) in candidates.iter().enumerate() {
        println!("  {}: {}", i + 1, candidate);
    }
    println!();

    // Example 3: Multiple characters
    println!("Example 3: Shrink candidates for '髙橋':");
    let candidates: Vec<String> = mj_shrink_candidates("髙橋", MJShrinkSchemes::ALL)
        .take(10)
        .collect();
    for (i, candidate) in candidates.iter().enumerate() {
        println!("  {}: {}", i + 1, candidate);
    }
    println!();

    // Example 4: Character with IVS
    println!("Example 4: Character with IVS:");
    let input = "葛\u{E0100}"; // Character with VS17
    let candidates: Vec<String> = mj_shrink_candidates(input, MJShrinkSchemes::ALL)
        .take(5)
        .collect();
    for (i, candidate) in candidates.iter().enumerate() {
        println!("  {}: {}", i + 1, candidate);
    }
    println!();

    // Example 5: Combining multiple schemes
    println!("Example 5: Using combined schemes (JIS + MOJ Notice 582):");
    let combined = MJShrinkSchemes::builder()
        .with(MJShrinkScheme::JISIncorporationUCSUnificationRule)
        .with(MJShrinkScheme::MOJNotice582);
    let candidates: Vec<String> = mj_shrink_candidates("髙", combined).take(5).collect();
    for (i, candidate) in candidates.iter().enumerate() {
        println!("  {}: {}", i + 1, candidate);
    }
}
