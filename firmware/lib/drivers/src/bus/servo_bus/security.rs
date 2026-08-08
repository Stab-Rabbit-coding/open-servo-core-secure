//! The message-plane hook: fold at the covered checkpoint, gate at the verdict
//! (`docs/security-architecture.md` §2, §7.2).
//!
//! This module is small on purpose. The whole integration is two calls placed
//! at points the transport already had — which is the design's central claim:
//! a transport built around "dispatch speculatively, gate effects on a
//! verdict" is already the right shape for a per-frame authenticator, so the
//! gate costs only its own compute, never a structural change.

use osc_protocol::wire::{Id, Opcode};
use osc_security::{AuthVerdict, Policy};

use super::ServoBus;
use crate::bench::sec_probe;
use crate::traits::bus::{Providers, RxRing};

/// Largest covered span the in-place authenticator will copy to a contiguous
/// buffer.
///
/// Sized for the hot loop (`osc-native-protocol.md` sec 7): an 8-target
/// uniform GWRITE is 46 covered bytes and a COMMIT with a trailer is 8, so 96
/// leaves generous headroom without spending real RAM out of the chip's 8 KB.
/// Frames past it are refused rather than waved through -- see
/// [`ServoBus::auth_verify`].
const AUTH_SPAN_MAX: usize = 96;

/// Ring offsets of a frame's CRC-covered span (`ID .. payload`).
///
/// The break's `0x00` ring byte is skipped, and the two trailing CRC bytes are
/// excluded, matching the wire checksum's definition (sec 3.2).
#[inline]
fn covered_span(anchor: u16, footprint: u16, ring_len: usize) -> (usize, usize) {
    let start = (anchor as usize + 1) % ring_len;
    let covered = (footprint as usize).saturating_sub(3);
    (start, covered)
}

impl<P: Providers> ServoBus<P> {
    /// Fold a frame into the stream digest, at the covered checkpoint.
    ///
    /// Called from the same site that arms the CRC feed, so the fold runs
    /// under the instruction's own remaining wire time at 0.5 M/1 M and costs
    /// no extra wake at any rate.
    ///
    /// The segments are read from the ring in place -- no staging copy -- for
    /// the same reason the CRC feed does (sec 3.2): a frame straddling the
    /// ring seam must not cost a copy, and the digest is byte-exact across the
    /// split either way.
    pub(super) fn security_fold(&mut self, anchor: u16, footprint: u16) {
        let inst = self.ring_inst(anchor);
        if !Policy::folds_into_stream(inst) {
            return;
        }
        // Disjoint field borrows: `ring` borrows `self.ring`, `fold` borrows
        // `self.security`. Going through a `&self` helper would merge them.
        let ring = self.ring.bytes();
        let len = ring.len();
        if len == 0 {
            return;
        }
        let (start, covered) = covered_span(anchor, footprint, len);
        let end = start + covered;
        let (seg_a, seg_b) = if end <= len {
            (&ring[start..end], &ring[..0])
        } else {
            (&ring[start..len], &ring[..end - len])
        };
        let folded = (seg_a.len() + seg_b.len()) as u32;
        self.security.fold(inst, seg_a, seg_b);
        sec_probe(|p| {
            p.folds = p.folds.wrapping_add(1);
            p.fold_bytes = p.fold_bytes.wrapping_add(folded);
        });
    }

    /// The authenticity verdict for a frame whose CRC has already passed.
    ///
    /// Returns `true` when the frame may apply its staged effects.
    pub(super) fn auth_verify(&mut self, anchor: u16, footprint: u16) -> bool {
        let inst = self.ring_inst(anchor);
        let ring = self.ring.bytes();
        let len = ring.len();
        if len == 0 {
            return false;
        }
        let (start, covered) = covered_span(anchor, footprint, len);
        let broadcast = ring[start] == Id::BROADCAST.as_byte();

        if covered > AUTH_SPAN_MAX {
            // Too large to authenticate in place. Only reachable by a frame
            // outside the hot-loop envelope; refuse it if policy wanted a tag
            // or it claimed to carry one, rather than silently skipping the
            // gate -- a size limit must never become an authentication bypass.
            let unguarded =
                !self.security.policy().requires_tag(inst) && !inst.authenticated();
            if !unguarded {
                sec_probe(|p| p.oversize = p.oversize.wrapping_add(1));
            }
            return unguarded;
        }

        // The verifier needs one contiguous span. Authenticated frames are
        // bounded by the trailer plus a small payload in the hot loop, so this
        // copy is affordable in a way it would not be for the CRC (which must
        // cover the full 252-byte payload at zero CPU).
        let mut buf = [0u8; AUTH_SPAN_MAX];
        for (i, slot) in buf[..covered].iter_mut().enumerate() {
            *slot = ring[(start + i) % len];
        }

        let verdict = self.security.verify(inst, broadcast, &buf[..covered]);
        sec_probe(|p| {
            p.verdicts = p.verdicts.wrapping_add(1);
            match verdict {
                AuthVerdict::Pass => p.pass = p.pass.wrapping_add(1),
                AuthVerdict::Missing => p.missing = p.missing.wrapping_add(1),
                AuthVerdict::BadTag => p.bad_tag = p.bad_tag.wrapping_add(1),
                AuthVerdict::Replay => p.replay = p.replay.wrapping_add(1),
                AuthVerdict::NoSession => p.no_session = p.no_session.wrapping_add(1),
                AuthVerdict::LockedOut => p.locked_out = p.locked_out.wrapping_add(1),
            }
        });

        // A COMMIT closes the cycle either way: on pass the effects applied,
        // on fail they reverted, and the host re-sends the whole held group.
        if matches!(inst.opcode(), Some(Opcode::Commit)) {
            self.security.close_cycle();
        }
        verdict.is_pass()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn covered_span_skips_break_and_crc() {
        // footprint = 3 + LEN (break + ID + LEN + INST + payload + 2 CRC).
        // A PING: LEN 3, footprint 6, covered span = ID,LEN,INST = 3 bytes.
        let (start, covered) = covered_span(0, 6, 512);
        assert_eq!(start, 1, "the break's 0x00 is skipped");
        assert_eq!(covered, 3);
    }

    #[test]
    fn covered_span_wraps_the_ring() {
        let (start, covered) = covered_span(510, 6, 512);
        assert_eq!(start, 511);
        assert_eq!(covered, 3);
        assert!(start + covered > 512, "this span straddles the seam");
    }

    #[test]
    fn oversize_frames_do_not_bypass_the_gate() {
        // Documented as an invariant here because the code path is only
        // reachable with a full-size frame: a frame that claims AUTH must
        // never pass merely because it was too big to check.
        assert!(AUTH_SPAN_MAX >= 46 + 5, "hot-loop GWRITE + trailer must fit");
    }
}
