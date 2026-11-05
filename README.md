# morpheme-formats

A Rust library that reads NaturalMotion Morpheme animation bundles and the skeleton rigs inside them. It supports the 32-bit, big-endian layout used on the PlayStation 3 disc of Tony Hawk's SHRED.

## Features

- Splits a bundle into its records: asset type, asset id, GUID, header word, payload and padding
- Decodes type 1 records (rigs) into joints with names, parents and a reference pose
- Returns each joint's reference rotation as an XYZW quaternion and its translation as XYZ
- Returns the rig header's rotation and four unexplained header words as stored
- Returns network, motion and other record types as unread byte slices
- Reads all five Morpheme bundles on the SHRED disc to their exact ends (1,680 records, 10 rigs)
- Bounds-checks every read and returns an error instead of panicking on malformed input
- Errors name the byte offset of the failed field and what was expected there
- Refuses a bundle that does not end exactly on a record boundary
- Checks that a rig has unique name ids, parents inside the rig, no cycles, finite translations and unit quaternions
- Does not normalise or otherwise change the values it reads
- Performs no file I/O and contains no `unsafe` code

## Getting Started

### Requirements

- Rust 1.99 or later (edition 2024)

Only the 32-bit, big-endian layout described above is supported. 64-bit or little-endian bundles and other rig layouts are not read, and the crate does not write bundles.

### Installation

The crate is published on crates.io:

```sh
cargo add morpheme-formats
```

### Usage

The crate works on byte slices. `bundle_be32` splits a bundle into records, and `rig_be32` decodes the payload of a type 1 record. Its second argument is the payload's offset in the bundle, so that errors report offsets in the bundle file. This is `examples/read_bundle.rs`:

```rust
use morpheme_formats::{bundle_be32, rig_be32};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: read_bundle <bundle file>")?;
    let bytes = std::fs::read(path)?;
    for record in bundle_be32(&bytes)? {
        println!(
            "{:#x}: type {} id {:#x}, {} payload bytes",
            record.offset,
            record.asset_type,
            record.asset_id,
            record.payload.len()
        );
        if record.asset_type == 1 {
            // Errors then name offsets in the bundle, not in the payload.
            let rig = rig_be32(record.payload, record.offset.saturating_add(32))?;
            for joint in &rig.joints {
                println!(
                    "  joint {} {:?} parent {:?}",
                    joint.id,
                    String::from_utf8_lossy(&joint.name),
                    joint.parent
                );
            }
        }
    }
    Ok(())
}
```

Run it from a checkout on a bundle extracted from the game:

```sh
cargo run --example read_bundle -- path/to/bundle
```

The crate documentation on [docs.rs](https://docs.rs/morpheme-formats) includes an example that builds a bundle in memory.

Run the tests with `cargo test --workspace`. They use synthetic data and need no game files. See [CONTRIBUTING.md](CONTRIBUTING.md) for the full set of checks and for how to report a file the crate cannot read.

Licensed under GPL-3.0-or-later; see [LICENSE](LICENSE).
