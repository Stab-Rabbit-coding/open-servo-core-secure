//! The AUTH trailer — the wire extension
//! (`docs/security-architecture.md` §2.3).
//!
//! ```text
//! BREAK | ID | LEN | INST(AUTH) | payload | SEQ | TAG[0..4] | CRC_lo | CRC_hi
//!                                          \_________________/
//!                                           5-byte AUTH trailer
//! ```
//!
//! The trailer sits **inside the payload region**, which is what makes this a
//! strictly additive extension rather than a protocol change:
//!
//! - `LEN` accounts for it through the existing `len_for(p)` arithmetic
//!   (`osc-native-protocol.md` §3.1) — no new span math anywhere.
//! - The CRC covers it through the existing `ID..payload` definition (§3.2) —
//!   the hardware CRC feed is untouched.
//! - Frame anatomy, the break law, footprint algebra, ring parity, the chain
//!   snoop (§6, framing-level only) and every host tool keep working.
//! - A frame **without** `FLAG_AUTH` is byte-identical to today's protocol.
//!
//! `FLAG_AUTH` is `INST` bit 1, which both layouts reserved as zero and which
//! `osc-native-protocol.md` §3.1/§5 explicitly held for a future extension.

use crate::mac::Tag;

/// `SEQ` (1) + `TAG` (4).
pub const TRAILER_LEN: usize = 5;

/// Sequence field width.
pub const SEQ_LEN: usize = 1;

/// Tag field width — 32 bits (§2.4).
pub const TAG_LEN: usize = 4;

/// A parsed trailer, plus the span the tag is computed over.
pub struct Trailer<'a> {
    /// Low 8 bits of the session sequence (§2.5).
    pub seq: u8,
    /// The 32-bit tag as it appeared on the wire.
    pub tag: Tag,
    /// Everything the tag covers: the CRC-covered span up to and **including**
    /// the `SEQ` byte, i.e. the frame minus its own tag bytes.
    ///
    /// Including `SEQ` in the covered prefix is what stops an attacker
    /// rewriting the sequence of an otherwise-valid captured frame.
    pub tagged_prefix: &'a [u8],
}

impl<'a> Trailer<'a> {
    /// Split a CRC-covered span (`ID | LEN | INST | payload`, tag included)
    /// into its tagged prefix and trailer.
    ///
    /// Returns `None` if the span is too short to hold a trailer at all — a
    /// malformed or truncated frame, which the caller treats as a bad tag
    /// rather than trusting.
    pub fn split(covered: &'a [u8]) -> Option<Self> {
        // Minimum: ID + LEN + INST + trailer.
        if covered.len() < 3 + TRAILER_LEN {
            return None;
        }
        let cut = covered.len() - TAG_LEN;
        let tag_bytes = &covered[cut..];
        let tag = Tag::from_bytes(&[tag_bytes[0], tag_bytes[1], tag_bytes[2], tag_bytes[3]]);
        let seq = covered[cut - SEQ_LEN];
        Some(Self {
            seq,
            tag,
            tagged_prefix: &covered[..cut],
        })
    }

    /// Payload bytes a dispatcher should see: the declared payload minus the
    /// trailer. The transport uses this so an authenticated WRITE decodes
    /// exactly like an unauthenticated one.
    #[inline]
    pub const fn strip(payload_len: u8) -> u8 {
        payload_len.saturating_sub(TRAILER_LEN as u8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_finds_the_fields() {
        // ID=5 LEN=9 INST=0x32 payload=[0xAA] SEQ=0x11 TAG=DE AD BE EF
        let covered = [0x05, 0x09, 0x32, 0xAA, 0x11, 0xDE, 0xAD, 0xBE, 0xEF];
        let t = Trailer::split(&covered).expect("well-formed");
        assert_eq!(t.seq, 0x11);
        assert_eq!(t.tag, Tag::from_bytes(&[0xDE, 0xAD, 0xBE, 0xEF]));
        assert_eq!(t.tagged_prefix, &covered[..5]);
    }

    #[test]
    fn the_sequence_byte_is_inside_the_tagged_prefix() {
        // Otherwise an attacker could rewrite SEQ on a captured frame and
        // replay it past the window.
        let covered = [0x05, 0x09, 0x32, 0xAA, 0x11, 0xDE, 0xAD, 0xBE, 0xEF];
        let t = Trailer::split(&covered).unwrap();
        assert_eq!(*t.tagged_prefix.last().unwrap(), t.seq);
    }

    #[test]
    fn truncated_spans_are_refused() {
        for n in 0..3 + TRAILER_LEN {
            let buf = [0u8; 3 + TRAILER_LEN];
            assert!(Trailer::split(&buf[..n]).is_none(), "len {n}");
        }
        let buf = [0u8; 3 + TRAILER_LEN];
        assert!(Trailer::split(&buf).is_some(), "minimum length must parse");
    }

    #[test]
    fn strip_removes_exactly_the_trailer() {
        assert_eq!(Trailer::strip(9), 4);
        assert_eq!(Trailer::strip(5), 0);
        // A malformed short payload saturates instead of underflowing.
        assert_eq!(Trailer::strip(2), 0);
    }
}
