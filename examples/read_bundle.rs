//! Lists the records of a Morpheme bundle and prints the joints of every rig in it.
//!
//! Usage: `cargo run --example read_bundle -- path/to/bundle`

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
