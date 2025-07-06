# jntajis-rs

A Rust port of [jntajis-python](https://github.com/opencollector/jntajis-python), providing character transliteration functionality for Japanese text processing.

## What's jntajis-rs?

jntajis-rs is a transliteration library specifically designed for dealing with three different character sets: JIS X 0208, JIS X 0213, and Unicode. This is a native Rust implementation that provides the same functionality as the original Python library.

```rust
use jntajis_rs::codec::mj_shrink::{MJShrinkSchemes, mj_shrink_candidates};

fn main() {
    // Get shrink candidates for a character variant
    let candidates: Vec<String> = mj_shrink_candidates("髙島屋", MJShrinkSchemes::ALL)
        .take(5)
        .collect();
    println!("{:?}", candidates); // outputs variations including "高島屋"
}
```

## Features

This library provides access to three different character tables:

- **MJ character table** (*MJ文字一覧表*) - A vast set of kanji characters used in Japanese text processing, developed by the Information-technology Promotion Agency
- **MJ shrink conversion map** (*MJ縮退マップ*) - For transliterating complex, less-frequently-used character variants to commonly-used ones
- **NTA shrink conversion map** (*国税庁JIS縮退マップ*) - Developed by Japan National Tax Agency to canonicalize user inputs

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
jntajis-rs = "0.1.0"
```

### Basic Example

```rust
use jntajis_rs::codec::mj_shrink::{MJShrinkScheme, MJShrinkSchemes, mj_shrink_candidates};

// Get all possible shrink candidates
let candidates: Vec<String> = mj_shrink_candidates("髙", MJShrinkSchemes::ALL)
    .take(10)
    .collect();

// Use specific shrink scheme
let jis_only = MJShrinkSchemes::builder()
    .with(MJShrinkScheme::JISIncorporationUCSUnificationRule);
let candidates: Vec<String> = mj_shrink_candidates("髙", jis_only)
    .take(5)
    .collect();

// Handle multiple characters
let candidates: Vec<String> = mj_shrink_candidates("髙橋", MJShrinkSchemes::ALL)
    .take(10)
    .collect();
```

### Advanced Usage

The library supports various MJ shrink schemes:

- `JISIncorporationUCSUnificationRule` - JIS incorporation and UCS unification rules
- `MOJNotice582` - MOJ Notice 582 transliteration rules
- `MOJFamilyRegisterActRelatedNotice` - Family register act related notice rules
- `InferenceByReadingAndGlyph` - Inference by reading and glyph rules

You can combine multiple schemes:

```rust
let combined = MJShrinkSchemes::builder()
    .with(MJShrinkScheme::JISIncorporationUCSUnificationRule)
    .with(MJShrinkScheme::MOJNotice582);
```

See `examples/mj_shrink_example.rs` for more detailed usage examples.

## Examples

Run the included example:

```bash
cargo run --example mj_shrink_example
```

## Building

```bash
# Standard build
cargo build

# Run tests
cargo test
```

## Character Mapping Relationships

The relationship between Unicode, MJ character mappings, JIS X 0213, and JIS X 0208 follows the same structure as the original Python implementation:

- **JNTA transliteration**: Direct conversion using the JNTA character mappings table
- **MJ transliteration**: Two-phase process involving Unicode to MJ character mappings, then MJ shrink mappings

## License

The source code is published under the BSD 3-clause license.

The embedded character mapping data comes from:

* **JIS shrink conversion mappings** (国税庁: JIS縮退マップ)
  - Publisher: National Tax Agency
  - Source: https://www.houjin-bangou.nta.go.jp/download/
  - License: CC BY 4.0

* **MJ character table** (文字情報技術促進協議会: MJ文字一覧表)
  - Publisher: Character Information Technology Promotion Council (CITPC)
  - Author: Information-technology Promotion Agency (IPA)
  - Source: https://moji.or.jp/mojikiban/mjlist/
  - License: CC BY-SA 2.1 JP

* **MJ shrink conversion mappings** (文字情報技術促進協議会: MJ縮退マップ)
  - Publisher: Character Information Technology Promotion Council (CITPC)
  - Author: Information-technology Promotion Agency (IPA)
  - Source: https://moji.or.jp/mojikiban/map/
  - License: CC BY-SA 2.1 JP

## Related Projects

- [jntajis-python](https://github.com/opencollector/jntajis-python) - The original Python implementation