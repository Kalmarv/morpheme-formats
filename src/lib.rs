//! Reader for the 32-bit big-endian NaturalMotion Morpheme *simple bundle* and the skeleton
//! (*rig*) records inside it.
//!
//! [Morpheme] is animation middleware. Games ship its runtime assets (networks, rigs, motion
//! data) in bundles: a sequence of records, each a 32-byte header followed by a payload padded
//! to 16 bytes. This crate was written against the big-endian, 32-bit-pointer bundles on the
//! PlayStation 3 disc of Tony Hawk's SHRED (five bundles, 1,680 records, ten of them rigs).
//! The code does not read any version field, so no Morpheme release number is claimed; other
//! pointer widths, byte orders and rig layouts are not implied.
//!
//! - [`bundle_be32`] splits a bundle into [`Record`]s. Every record is bounded, whatever its
//!   asset type, and the walk must end exactly at the end of the input.
//! - [`rig_be32`] decodes the payload of a type 1 record into a [`Rig`]: joint names, parents,
//!   and the reference pose (rotation and translation per joint). Network and motion records
//!   stay opaque byte slices.
//!
//! Every read is bounds-checked and returns an [`Error`] naming the byte offset and what was
//! expected; malformed input is refused, not a panic.
//!
//! ```
//! use morpheme_formats::bundle_be32;
//!
//! // One record of unknown type: 32-byte header, 3 payload bytes, 13 padding bytes.
//! let mut bundle = Vec::new();
//! bundle.extend_from_slice(&0x1234_5678u32.to_be_bytes()); // asset type
//! bundle.extend_from_slice(&7u32.to_be_bytes()); // asset id
//! bundle.extend_from_slice(&[0xAA; 16]); // GUID
//! bundle.extend_from_slice(&3u32.to_be_bytes()); // payload length
//! bundle.extend_from_slice(&0u32.to_be_bytes()); // header word
//! bundle.extend_from_slice(&[1, 2, 3]);
//! bundle.extend_from_slice(&[0; 13]);
//!
//! let records = bundle_be32(&bundle)?;
//! assert_eq!(records.len(), 1);
//! assert_eq!(records[0].asset_id, 7);
//! assert_eq!(records[0].payload, [1, 2, 3]);
//!
//! // A truncated bundle is refused with the offset of the failed read.
//! assert!(bundle_be32(&bundle[..40]).is_err());
//! # Ok::<(), morpheme_formats::Error>(())
//! ```
//!
//! The crate performs no file I/O; `examples/read_bundle.rs` shows reading a file and listing
//! its rigs.
//!
//! [Morpheme]: https://en.wikipedia.org/wiki/NaturalMotion
#![deny(clippy::disallowed_methods, clippy::disallowed_types)]
#![deny(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]

use primase_foundations::bytes::{BoundsError, Cursor, Endian::Big, View};
use std::collections::BTreeSet;

/// A refused read: where in the input it failed and what was expected there.
///
/// Rig errors carry offsets in the containing bundle when the `origin` passed to [`rig_be32`]
/// is the payload's bundle offset.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("Morpheme at {offset:#x}: {detail}")]
pub struct Error {
    /// Byte offset of the field that failed.
    pub offset: usize,
    /// What was expected at `offset`.
    pub detail: String,
}
impl From<BoundsError> for Error {
    fn from(e: BoundsError) -> Self {
        Self {
            offset: usize::try_from(e.absolute()).unwrap_or(usize::MAX),
            detail: e.to_string(),
        }
    }
}
fn err(offset: usize, detail: impl Into<String>) -> Error {
    Error {
        offset,
        detail: detail.into(),
    }
}
fn add(a: usize, b: usize) -> Result<usize, Error> {
    a.checked_add(b).ok_or_else(|| err(a, "offset overflow"))
}
fn size(n: u32) -> Result<usize, Error> {
    usize::try_from(n).map_err(|_| err(0, "count exceeds address space"))
}

/// One bundle record: a 32-byte header and its payload, borrowed from the input.
///
/// A record's identity is scoped to its containing bundle; no GUID deduplication is performed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record<'a> {
    /// Byte offset of the record's header in the bundle.
    pub offset: usize,
    /// Asset type word; 1 is a rig (see [`rig_be32`]), other values are not interpreted.
    pub asset_type: u32,
    /// Asset id word.
    pub asset_id: u32,
    /// The 16-byte GUID, as stored.
    pub guid: [u8; 16],
    /// The header word after the payload length, retained without interpretation.
    pub header_word: u32,
    /// The payload, exactly the declared length.
    pub payload: &'a [u8],
    /// Authored bytes between this payload and the next 16-byte boundary.
    pub padding: &'a [u8],
}

/// Splits a bundle into its records.
///
/// A record is a 32-byte header (`u32` type, `u32` id, 16-byte GUID, `u32` payload length, one
/// `u32` word, all big-endian) followed by the payload rounded up to 16 bytes. Reads every record
/// through exact end of input; even a short trailing fragment is an error, not a terminator.
/// Padding bytes are returned as stored and need not be zero.
///
/// # Errors
///
/// An [`Error`] at the offset of the first header or payload that does not fit in `bytes`.
pub fn bundle_be32(bytes: &[u8]) -> Result<Vec<Record<'_>>, Error> {
    let mut c = Cursor::new(bytes);
    let mut records = Vec::new();
    while c.remaining() != 0 {
        let offset = c.position();
        let asset_type = c.u32(Big)?;
        let asset_id = c.u32(Big)?;
        let guid = c.array()?;
        let length = size(c.u32(Big)?)?;
        let header_word = c.u32(Big)?;
        let payload = c.take(length)?;
        let padding = c.take(
            16usize
                .checked_sub(length % 16)
                .ok_or_else(|| err(offset, "padding overflow"))?
                % 16,
        )?;
        records.push(Record {
            offset,
            asset_type,
            asset_id,
            guid,
            header_word,
            payload,
            padding,
        });
    }
    Ok(records)
}

/// One joint of a [`Rig`] with its reference pose.
#[derive(Clone, Debug, PartialEq)]
pub struct Joint {
    /// Source index, scoped to this rig. Names may repeat.
    pub id: u32,
    /// Name bytes without the terminating NUL; not necessarily UTF-8.
    pub name: Vec<u8>,
    /// [`Joint::id`] of the parent, or `None` for a root. Always inside the rig.
    pub parent: Option<u32>,
    /// Reference rotation as an XYZW quaternion, unit length within 0.001, stored as read.
    pub rotation: [f32; 4],
    /// Reference translation (X, Y, Z); always finite.
    pub translation: [f32; 3],
    /// Fourth position lane is ignored by the native matrix constructor; it is not a float semantic.
    pub position_w_bits: u32,
}

/// A decoded type 1 record: the skeleton hierarchy and its reference pose.
#[derive(Clone, Debug, PartialEq)]
pub struct Rig {
    /// Separate rig-header orientation; not applied to the reference joint arrays.
    pub header_rotation: [f32; 4],
    /// The four words at header offset 0x10, as raw bits; their meaning is not established.
    pub header_vector_bits: [u32; 4],
    /// Joints in order of [`Joint::id`] (0 up to the joint count minus one); the hierarchy is
    /// acyclic.
    pub joints: Vec<Joint>,
}

struct Reader<'a> {
    view: View<'a>,
    origin: usize,
}
impl Reader<'_> {
    fn u(&self, at: usize) -> Result<u32, Error> {
        Ok(self.view.u32(at, Big)?)
    }
    fn at(&self, base: usize, delta: usize) -> Result<usize, Error> {
        add(base, delta)
    }
    fn relative(&self, base: usize, field: usize) -> Result<usize, Error> {
        self.at(base, size(self.u(self.at(base, field)?)?)?)
    }
    fn fail(&self, at: usize, detail: impl Into<String>) -> Error {
        err(self.origin.saturating_add(at), detail)
    }
    fn table(&self, at: usize, count: usize, stride: usize) -> Result<Cursor<'_>, Error> {
        let length = count
            .checked_mul(stride)
            .ok_or_else(|| self.fail(at, "table length overflow"))?;
        Ok(Cursor::with_origin(
            self.view.slice(at, length)?,
            add(self.origin, at)?,
        ))
    }
    fn vec4(&self, at: usize) -> Result<[f32; 4], Error> {
        let mut c = self.table(at, 4, 4)?;
        Ok([c.f32(Big)?, c.f32(Big)?, c.f32(Big)?, c.f32(Big)?])
    }
    fn rotation(&self, at: usize) -> Result<[f32; 4], Error> {
        let q = self.vec4(at)?;
        let norm = q.iter().map(|&x| f64::from(x).powi(2)).sum::<f64>().sqrt();
        if !norm.is_finite() || (norm - 1.0).abs() > 1e-3 {
            return Err(self.fail(at, "nonfinite or nonunit quaternion"));
        }
        Ok(q)
    }
}

/// Decodes the payload of a type 1 record ([`Record::payload`]) into a [`Rig`].
///
/// `origin` is the payload's byte offset in the containing bundle (0 for a standalone payload);
/// it is added to the offsets in errors. Stored pointers are offsets relative to four bases: the
/// rig, its string table, the reference-attribute header, and that header's data buffer.
/// Runtime-pointer words and alignment padding are neither followed nor required to be zero.
///
/// The reference pose must hold exactly one position attribute (descriptor 2) and one rotation
/// attribute (descriptor 6), each with stride and alignment 16 and one entry per joint. Values
/// are never normalised.
///
/// # Errors
///
/// An [`Error`] naming the offset and cause for: truncated or out-of-range spans, an empty rig,
/// a joint/name/reference count mismatch, duplicate or out-of-range name ids, unterminated
/// names, missing, duplicate or unsupported reference attributes, non-finite translations,
/// non-unit or non-finite quaternions, parents outside the rig, and cyclic hierarchies.
pub fn rig_be32(bytes: &[u8], origin: usize) -> Result<Rig, Error> {
    let r = Reader {
        view: View::with_origin(bytes, origin),
        origin,
    };
    r.view.slice(0, 0x60)?;
    let header_rotation = r.rotation(0)?;
    let mut header_vector = r.table(16, 4, 4)?;
    let header_vector_bits = [
        header_vector.u32(Big)?,
        header_vector.u32(Big)?,
        header_vector.u32(Big)?,
        header_vector.u32(Big)?,
    ];
    let hierarchy = size(r.u(0x20)?)?;
    let count = size(r.u(hierarchy)?)?;
    if count == 0 {
        return Err(r.fail(hierarchy, "empty rig"));
    }
    // The on-disc word at hierarchy+4 is a runtime pointer: relocation replaces it with hierarchy+8.
    let mut parents = r.table(add(hierarchy, 8)?, count, 4)?;
    let strings = size(r.u(0x44)?)?;
    if size(r.u(strings)?)? != count {
        return Err(r.fail(strings, "joint/name count mismatch"));
    }
    let string_length = size(r.u(add(strings, 4)?)?)?;
    let string_base = r.relative(strings, 16)?;
    let names = r.view.slice(string_base, string_length)?;
    let mut ids = r.table(r.relative(strings, 8)?, count, 4)?;
    let mut offsets = r.table(r.relative(strings, 12)?, count, 4)?;
    let mut indexed_names = Vec::new();
    let mut seen = BTreeSet::new();
    for _ in 0..count {
        let id = ids.u32(Big)?;
        if size(id)? >= count || !seen.insert(id) {
            return Err(r.fail(strings, "duplicate or out-of-range name ID"));
        }
        let offset = size(offsets.u32(Big)?)?;
        let tail = names.get(offset..).ok_or_else(|| {
            r.fail(
                add(string_base, offset).unwrap_or(usize::MAX),
                "name offset outside string bytes",
            )
        })?;
        let length = tail.iter().position(|&b| b == 0).ok_or_else(|| {
            r.fail(
                add(string_base, offset).unwrap_or(usize::MAX),
                "unterminated joint name",
            )
        })?;
        indexed_names.push((
            id,
            tail.get(..length)
                .ok_or_else(|| r.fail(strings, "name span"))?
                .to_vec(),
        ));
    }
    indexed_names.sort_by_key(|(id, _)| *id);
    let reference = size(r.u(0x48)?)?;
    let format = r.relative(reference, 16)?;
    let format_length = size(r.u(format)?)?;
    let f = Reader {
        view: View::with_origin(r.view.slice(format, format_length)?, add(origin, format)?),
        origin: add(origin, format)?,
    };
    if size(r.u(add(format, 8)?)?)? != count {
        return Err(r.fail(add(format, 8)?, "reference joint count mismatch"));
    }
    let attributes = size(r.u(add(format, 12)?)?)?;
    let mut descriptors = f.table(f.relative(0, 16)?, attributes, 12)?;
    let mut pointers = f.table(f.relative(0, 20)?, attributes, 4)?;
    let mut position = None;
    let mut rotation = None;
    for _ in 0..attributes {
        let typ = descriptors.u32(Big)?;
        let stride = descriptors.u32(Big)?;
        let alignment = descriptors.u32(Big)?;
        let relative_at = size(pointers.u32(Big)?)?;
        let at = add(format, relative_at)?;
        if stride != 16 || alignment != 16 || !matches!(typ, 2 | 6) {
            return Err(r.fail(
                at,
                format!("unsupported reference attribute {typ}, stride {stride}, alignment {alignment}"),
            ));
        }
        f.table(relative_at, count, 16)?;
        let slot = if typ == 2 { &mut position } else { &mut rotation };
        if slot.replace(at).is_some() {
            return Err(r.fail(at, "duplicate reference attribute"));
        }
    }
    let position = position.ok_or_else(|| r.fail(format, "missing position attribute"))?;
    let rotation = rotation.ok_or_else(|| r.fail(format, "missing rotation attribute"))?;
    let mut joints = Vec::new();
    let mut pos = r.table(position, count, 16)?;
    let mut rot = r.table(rotation, count, 16)?;
    for (id, name) in indexed_names {
        let parent_field = add(add(hierarchy, 8)?, parents.position())?;
        let parent = parents.i32(Big)?;
        let parent = if parent == -1 {
            None
        } else {
            let parent =
                u32::try_from(parent).map_err(|_| r.fail(parent_field, "invalid negative parent"))?;
            if size(parent)? >= count {
                return Err(r.fail(parent_field, "parent outside rig"));
            }
            Some(parent)
        };
        let translation_field = add(position, pos.position())?;
        let translation = [pos.f32(Big)?, pos.f32(Big)?, pos.f32(Big)?];
        let position_w_bits = pos.u32(Big)?;
        if let Some(lane) = translation.iter().position(|x| !x.is_finite()) {
            return Err(r.fail(
                add(
                    translation_field,
                    lane.checked_mul(4)
                        .ok_or_else(|| r.fail(translation_field, "lane offset overflow"))?,
                )?,
                "nonfinite reference translation",
            ));
        }
        let q_at = add(rotation, rot.position())?;
        let rotation = r.rotation(q_at)?;
        rot.take(16)?;
        joints.push(Joint {
            id,
            name,
            parent,
            rotation,
            translation,
            position_w_bits,
        });
    }
    // Iterative colouring: accept source order with parents later, without recursive-stack limits.
    let mut finished = BTreeSet::new();
    for joint in &joints {
        let mut visiting = BTreeSet::new();
        let mut next = Some(joint.id);
        while let Some(id) = next {
            if finished.contains(&id) {
                break;
            }
            if !visiting.insert(id) {
                return Err(r.fail(hierarchy, "cyclic joint hierarchy"));
            }
            next = joints
                .get(size(id)?)
                .ok_or_else(|| r.fail(hierarchy, "joint index"))?
                .parent;
        }
        finished.extend(visiting);
    }
    Ok(Rig {
        header_rotation,
        header_vector_bits,
        joints,
    })
}
