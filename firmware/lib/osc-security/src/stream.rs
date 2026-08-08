//! The instruction-stream digest — the message plane's core mechanism
//! (`docs/security-architecture.md` §2.1).
//!
//! Every servo folds **every instruction frame it observes**, in full and in
//! wire order, into one keyed rolling digest. The broadcast COMMIT that closes
//! a hot-loop cycle carries a single tag over that digest, and a mismatch
//! reverts the whole staging buffer through the transport's existing REVERT
//! path.
//!
//! The frame is folded whole rather than just this servo's own GWRITE slice
//! because a broadcast COMMIT carries exactly **one** tag: every servo must
//! therefore arrive at the *same* digest. The wire is shared, so every servo
//! already sees every frame — folding in full is what makes the digest
//! fleet-common (§2.1).
//!
//! Status frames are excluded: they are replies, carrying a responder's key
//! rather than the host's, and the chain snoop deliberately never validates
//! them (`osc-native-protocol.md` §6).

use crate::mac::{HalfSipHasher, MacKey, Tag};

/// Domain byte distinguishing a stream digest from any other tag input, so a
/// digest can never be replayed as a frame tag or vice versa.
const DOMAIN_STREAM: u8 = 0x03;

/// Rolling digest over the instruction stream since the last COMMIT.
pub struct StreamDigest {
    hasher: HalfSipHasher,
    key: MacKey,
    /// Frames folded since the last reset — surfaced for the bench probe and
    /// for the "did anything actually stage" check.
    frames: u16,
}

impl StreamDigest {
    /// Start a fresh stream under `key` (the group key — the COMMIT that
    /// closes the stream is broadcast).
    pub fn new(key: MacKey) -> Self {
        let mut hasher = HalfSipHasher::new(key);
        hasher.update(&[DOMAIN_STREAM]);
        Self {
            hasher,
            key,
            frames: 0,
        }
    }

    /// Re-key and restart. Called on session establishment and after every
    /// COMMIT verdict, pass or fail.
    pub fn reset(&mut self, key: MacKey) {
        *self = Self::new(key);
    }

    /// Fold one frame's CRC-covered span. `seg_a`/`seg_b` are the (up to two)
    /// ring segments the span occupies — the transport hands them over
    /// directly, so a frame straddling the ring seam costs no copy. `seg_b` is
    /// empty for the common non-wrapping case.
    ///
    /// Chunking is irrelevant to the result
    /// ([`crate::mac::HalfSipHasher`] is byte-exact across splits), so a
    /// wrapped frame and a contiguous one digest identically.
    pub fn fold_frame(&mut self, seg_a: &[u8], seg_b: &[u8]) {
        self.hasher.update(seg_a);
        self.hasher.update(seg_b);
        self.frames = self.frames.saturating_add(1);
    }

    /// Frames folded since the last reset.
    #[inline]
    pub const fn frames(&self) -> u16 {
        self.frames
    }

    /// Bytes folded since the last reset — bench probe input (§7.4).
    #[inline]
    pub const fn bytes(&self) -> u32 {
        self.hasher.absorbed()
    }

    /// Close the stream and produce its digest, without disturbing the live
    /// state (the caller resets explicitly after the verdict is applied).
    #[inline]
    pub fn digest(&self) -> Tag {
        self.hasher.clone().finish()
    }

    /// The key this stream is folded under — so the COMMIT tag check uses the
    /// same one the stream was opened with, even if a re-key landed mid-cycle.
    #[inline]
    pub const fn key(&self) -> MacKey {
        self.key
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k(b: u8) -> MacKey {
        MacKey::from_bytes(&[b; 8])
    }

    fn digest_of(key: MacKey, frames: &[&[u8]]) -> Tag {
        let mut s = StreamDigest::new(key);
        for f in frames {
            s.fold_frame(f, &[]);
        }
        s.digest()
    }

    #[test]
    fn ring_wrap_split_is_transparent() {
        let frame = [0x05u8, 0x07, 0x30, 0x80, 0x01, 0x2C, 0x01];
        let whole = digest_of(k(1), &[&frame]);
        for split in 0..=frame.len() {
            let mut s = StreamDigest::new(k(1));
            s.fold_frame(&frame[..split], &frame[split..]);
            assert_eq!(s.digest(), whole, "ring seam at {split}");
        }
    }

    #[test]
    fn injected_frame_changes_the_digest() {
        let a: &[u8] = &[0x01, 0x07, 0x60, 0x00, 0x01];
        let b: &[u8] = &[0x02, 0x07, 0x60, 0x00, 0x02];
        let forged: &[u8] = &[0x03, 0x07, 0x60, 0x00, 0x03];
        let honest = digest_of(k(1), &[a, b]);
        assert_ne!(digest_of(k(1), &[a, b, forged]), honest, "injection");
        assert_ne!(digest_of(k(1), &[forged, a, b]), honest, "prefix injection");
    }

    #[test]
    fn suppressed_frame_changes_the_digest() {
        let a: &[u8] = &[0x01, 0x07, 0x60, 0x00, 0x01];
        let b: &[u8] = &[0x02, 0x07, 0x60, 0x00, 0x02];
        assert_ne!(digest_of(k(1), &[a]), digest_of(k(1), &[a, b]));
    }

    #[test]
    fn modified_frame_changes_the_digest() {
        let a: &[u8] = &[0x01, 0x07, 0x60, 0x00, 0x01];
        let tampered: &[u8] = &[0x01, 0x07, 0x60, 0x00, 0x02];
        assert_ne!(digest_of(k(1), &[a]), digest_of(k(1), &[tampered]));
    }

    #[test]
    fn reordering_changes_the_digest() {
        let a: &[u8] = &[0x01, 0x07, 0x60, 0x00, 0x01];
        let b: &[u8] = &[0x02, 0x07, 0x60, 0x00, 0x02];
        assert_ne!(digest_of(k(1), &[a, b]), digest_of(k(1), &[b, a]));
    }

    #[test]
    fn the_stream_is_self_delimiting_so_it_cannot_be_re_split() {
        // The digest is over a flat byte stream, so re-splitting would be a
        // forgery route IF two different frame sequences could produce the
        // same bytes. They cannot, and the reason is structural: a covered
        // span is `ID | LEN | INST | payload`, whose length is exactly `LEN`
        // (`LEN = 3 + p` and `|covered| = 2 + 1 + p`). So byte 1 of every
        // frame announces that frame's own length, and the concatenation
        // parses back to exactly one frame sequence.
        //
        // Assert the invariant on real frame shapes, then assert that the
        // only same-length single frame necessarily differs in its LEN byte
        // and therefore digests differently.
        let f1: &[u8] = &[0x01, 0x04, 0x60, 0xAA]; // LEN 4, covered len 4
        let f2: &[u8] = &[0x02, 0x04, 0x60, 0xBB];
        assert_eq!(f1.len(), f1[1] as usize, "covered length must equal LEN");
        assert_eq!(f2.len(), f2[1] as usize);

        // A single frame spanning the same 8 bytes must declare LEN 8.
        let merged: &[u8] = &[0x01, 0x08, 0x60, 0xAA, 0x02, 0x04, 0x60, 0xBB];
        assert_eq!(merged.len(), merged[1] as usize);
        assert_ne!(
            digest_of(k(1), &[f1, f2]),
            digest_of(k(1), &[merged]),
            "a re-split must not collide"
        );
    }

    #[test]
    fn different_group_keys_diverge() {
        let a: &[u8] = &[0x01, 0x07, 0x60];
        assert_ne!(digest_of(k(1), &[a]), digest_of(k(2), &[a]));
    }

    #[test]
    fn reset_clears_the_stream() {
        let a: &[u8] = &[0x01, 0x07, 0x60];
        let mut s = StreamDigest::new(k(1));
        s.fold_frame(a, &[]);
        assert_eq!(s.frames(), 1);
        s.reset(k(1));
        assert_eq!(s.frames(), 0);
        assert_eq!(s.digest(), StreamDigest::new(k(1)).digest());
    }

    #[test]
    fn empty_stream_has_a_defined_digest() {
        // A COMMIT with nothing staged is legal and must still verify.
        let s = StreamDigest::new(k(9));
        assert_eq!(s.frames(), 0);
        let _ = s.digest();
    }
}
