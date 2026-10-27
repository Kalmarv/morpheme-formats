# morpheme-formats

A Rust reader for the 32-bit big-endian NaturalMotion Morpheme simple bundle and the skeleton (rig) records inside it, with checked reads that refuse malformed input instead of panicking.

[Morpheme](https://en.wikipedia.org/wiki/NaturalMotion) is animation middleware; games ship its runtime assets (networks, rigs, motion data) as bundles of 32-byte-headed, 16-byte-aligned records. This crate was written against the big-endian, 32-bit-pointer bundles on the PlayStation 3 disc of Tony Hawk's SHRED: all five of its bundles (`actor_male`, `ai_skate`, `skate`, `snowboard`, `ui`) split exactly to their ends (1,680 records), and the ten type 1 rigs among them (812 joints) decode. The code reads no version field, so no Morpheme release number is claimed. Pointer widths, byte orders and rig layouts other than this one are not supported.

It offers two functions:

- `bundle_be32` splits a bundle into records (type, id, GUID, header word, payload, padding). Unknown asset types stay opaque, bounded byte slices.
- `rig_be32` decodes a type 1 payload: joint names (raw bytes, may repeat), parents, and each joint's reference rotation (XYZW quaternion) and translation.

## Example

`examples/read_bundle.rs` lists a bundle's records and the joints of its rigs:

```rust,no_run
use morpheme_formats::{bundle_be32, rig_be32};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("usage: read_bundle <bundle file>")?;
    let bytes = std::fs::read(path)?;
    for record in bundle_be32(&bytes)? {
        if record.asset_type == 1 {
            // Errors then name offsets in the bundle, not in the payload.
            let rig = rig_be32(record.payload, record.offset.saturating_add(32))?;
            for joint in &rig.joints {
                println!("{:?} parent {:?}", String::from_utf8_lossy(&joint.name), joint.parent);
            }
        }
    }
    Ok(())
}
```

```sh
cargo run --example read_bundle -- path/to/bundle
```

The crate documentation (`cargo doc --open`) has a runnable example that builds a bundle in memory.

## Guarantees

- Every read is bounds-checked; truncated, oversized or inconsistent input returns an `Error` and never panics (the tests refuse every truncated prefix of a rig). The crate denies `unsafe` code.
- An `Error` names the byte offset of the failed field (in the containing bundle, given the `origin` argument of `rig_be32`) and what was expected there.
- `bundle_be32` accepts only input that ends exactly on a record boundary; a short trailing fragment is an error.
- A decoded rig has at least one joint, unique in-range name ids, parents inside the rig, no cycles, finite translations and quaternions of unit length within 0.001, exactly one position and one rotation attribute. Values are never normalised or reordered beyond sorting joints by id.
- Runtime-pointer words and padding in the data are neither followed nor required to be zero, and are kept as stored where the types expose them.

## What it does not do

- No network or motion-data decoding: those records are returned as bytes.
- No other layouts (64-bit pointers, little-endian, other rig versions), and no guarantee about game builds other than the one above.
- No frame or bind-space conversion; the rig header's orientation and four raw words are returned as read, with no meaning assigned.
- No file I/O, no writing of bundles.

## Tests

```sh
cargo test --workspace
```

The tests build a literal two-joint rig and bundle in memory (differing pointer bases, junk in runtime-pointer words and padding, an unknown sibling record) and check the decoded values, the offsets and messages of each refusal, and every truncated prefix. They need no game files.

Checks against real game files are not part of this repository's tests, and it has no `GAME_DATA` tests or `tests/inputs` table. The census in the second paragraph was made with a separate harness over the disc's files.

The crate depends on [`primase-foundations`](https://github.com/Kalmarv/primase-foundations) (a bounds-checked byte cursor) and `thiserror`, and uses Rust 2024 edition.

## License

GPL-3.0-or-later; see [LICENSE](LICENSE).

Used by [primase](https://github.com/Kalmarv/primase), a game-asset conversion library. See [CONTRIBUTING.md](CONTRIBUTING.md) to build or to report a file it cannot read.
