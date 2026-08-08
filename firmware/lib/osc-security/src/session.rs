//! The session state machine and the per-frame verdict
//! (`docs/security-architecture.md` §2, §3).
//!
//! This is what the transport actually calls. Two entry points matter:
//!
//! - [`SecurityContext::fold`] — at the covered checkpoint, for every
//!   instruction frame that folds into the stream digest.
//! - [`SecurityContext::verify`] — beside the CRC verdict, gating staged
//!   effects.
//!
//! Everything else is session establishment, which runs bus-quiet at boot.

use crate::keys::{Epoch, SessionKeys};
use crate::mac::{HalfSipHasher, MacKey, Tag};
use crate::policy::{AuthVerdict, Policy};
use crate::replay::ReplayWindow;
use crate::stream::StreamDigest;
use crate::trailer::{TRAILER_LEN, Trailer};
use osc_protocol::wire::{Inst, Opcode};

/// Domain byte for an inline frame tag.
const DOMAIN_FRAME: u8 = 0x01;
/// Domain byte for the COMMIT tag that closes a stream.
const DOMAIN_COMMIT: u8 = 0x02;

/// Where the servo stands with respect to the message plane.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SecurityState {
    /// No SE, or the SE failed to answer. The servo still flies (§4.2); the
    /// policy decides whether effects are accepted.
    Unsecured,
    /// Session establishment is mid-flight (a quiet window is open).
    Establishing,
    /// Keys derived, message plane live.
    Secured,
    /// Too many consecutive tag failures: effects refused until re-key.
    LockedOut,
}

/// The servo's message-plane state.
pub struct SecurityContext {
    state: SecurityState,
    keys: SessionKeys,
    stream: StreamDigest,
    replay: ReplayWindow,
    policy: Policy,
    /// Consecutive attack-evidence verdicts.
    consecutive_fails: u8,
    /// Lifetime counters, mirrored into the telemetry region.
    pub auth_fail_count: u32,
    pub replay_drop_count: u32,
}

impl SecurityContext {
    pub fn new(policy: Policy) -> Self {
        Self {
            state: SecurityState::Unsecured,
            keys: SessionKeys::UNSET,
            stream: StreamDigest::new(MacKey::ZERO),
            replay: ReplayWindow::new(),
            policy,
            consecutive_fails: 0,
            auth_fail_count: 0,
            replay_drop_count: 0,
        }
    }

    #[inline]
    pub const fn state(&self) -> SecurityState {
        self.state
    }

    #[inline]
    pub const fn policy(&self) -> Policy {
        self.policy
    }

    /// Change the enforcement policy.
    ///
    /// Does **not** reset the lockout: an operator tightening policy after an
    /// attack must not be able to clear the evidence by doing so, and an
    /// operator loosening it should not silently re-arm a servo that has been
    /// under attack. Only a successful re-key clears a lockout (§2.7).
    pub fn set_policy(&mut self, policy: Policy) {
        self.policy = policy;
    }

    #[inline]
    pub const fn epoch(&self) -> Epoch {
        self.keys.epoch
    }

    /// Install a freshly derived session (§3). Resets the stream, the replay
    /// window and the lockout — a successful re-key is the documented remedy
    /// for every message-plane failure.
    pub fn install(&mut self, keys: SessionKeys) {
        debug_assert!(keys.is_ready(), "install called with incomplete keys");
        self.stream.reset(keys.group);
        self.replay.restart();
        self.consecutive_fails = 0;
        self.keys = keys;
        self.state = SecurityState::Secured;
    }

    /// Tear the session down (SE lost, explicit de-auth).
    pub fn clear(&mut self) {
        self.keys = SessionKeys::UNSET;
        self.stream.reset(MacKey::ZERO);
        self.replay.restart();
        self.state = SecurityState::Unsecured;
    }

    /// Fold a frame's CRC-covered span into the stream digest.
    ///
    /// Called at the covered checkpoint, inside the existing dispatch body.
    /// `seg_a`/`seg_b` are the ring segments the span occupies (`seg_b` empty
    /// unless the frame straddles the ring seam).
    ///
    /// Folding happens **before** any verdict, exactly like the CRC feed: the
    /// digest is speculative until the COMMIT tag closes it, and a frame that
    /// later fails CRC is handled by [`Self::rollback_stream`].
    pub fn fold(&mut self, inst: Inst, seg_a: &[u8], seg_b: &[u8]) {
        if self.state != SecurityState::Secured || !Policy::folds_into_stream(inst) {
            return;
        }
        self.stream.fold_frame(seg_a, seg_b);
    }

    /// Abandon the current stream after a failed cycle.
    ///
    /// The host's retry contract re-sends the whole held-write group, so
    /// starting clean is both correct and cheaper than trying to un-fold.
    pub fn rollback_stream(&mut self) {
        if self.state == SecurityState::Secured {
            self.stream.reset(self.keys.group);
        }
    }

    /// The message plane's verdict on one frame.
    ///
    /// `covered` is the frame's CRC-covered span **including** the trailer:
    /// `ID | LEN | INST | payload`, where the last [`TRAILER_LEN`] payload
    /// bytes are `SEQ ‖ TAG` when `inst` carries `FLAG_AUTH`.
    ///
    /// Called beside the CRC verdict and ANDed with it, so a frame must pass
    /// **both** to apply its staged effects.
    pub fn verify(&mut self, inst: Inst, broadcast: bool, covered: &[u8]) -> AuthVerdict {
        let verdict = self.evaluate(inst, broadcast, covered);
        self.record(verdict);
        verdict
    }

    /// The verdict logic, without the bookkeeping.
    fn evaluate(&mut self, inst: Inst, broadcast: bool, covered: &[u8]) -> AuthVerdict {
        if self.state == SecurityState::LockedOut {
            // Everything effect-bearing is refused; reads still flow so the
            // host can see the alert and diagnose.
            return if self.policy.requires_tag(inst) {
                AuthVerdict::LockedOut
            } else {
                AuthVerdict::Pass
            };
        }

        let needs = self.policy.requires_tag(inst);
        let carries = inst.authenticated();

        if !carries {
            return if needs {
                AuthVerdict::Missing
            } else {
                AuthVerdict::Pass
            };
        }

        // A tag is present. Verify it even when policy would not have demanded
        // one -- a bad tag is attack evidence regardless of policy.
        if self.state != SecurityState::Secured || !self.keys.is_ready() {
            return AuthVerdict::NoSession;
        }

        let Some(trailer) = Trailer::split(covered) else {
            return AuthVerdict::BadTag;
        };

        if self.replay.check(trailer.seq).is_err() {
            return AuthVerdict::Replay;
        }

        let key = self.keys.for_frame(broadcast);
        let expected = if matches!(inst.opcode(), Some(Opcode::Commit)) {
            self.commit_tag(key, trailer.seq, trailer.tagged_prefix)
        } else {
            self.frame_tag(key, trailer.seq, trailer.tagged_prefix)
        };

        if !expected.ct_eq(trailer.tag) {
            return AuthVerdict::BadTag;
        }

        // Only a fully verified frame advances the sequence: a forged or
        // corrupted frame must never burn a sequence number and desynchronise
        // the honest stream.
        self.replay.commit(trailer.seq);
        AuthVerdict::Pass
    }

    /// Tag over an inline-authenticated frame:
    /// `DOMAIN ‖ epoch ‖ seq ‖ covered-prefix`.
    ///
    /// The frame's own `ID` byte is inside the prefix, so the tag is bound to
    /// its target with nothing extra.
    fn frame_tag(&self, key: MacKey, seq: u8, prefix: &[u8]) -> Tag {
        let mut h = HalfSipHasher::new(key);
        h.update(&[DOMAIN_FRAME]);
        h.update(&self.keys.epoch.to_le_bytes());
        h.update(&[seq]);
        h.update(prefix);
        h.finish()
    }

    /// Tag over a COMMIT: as above plus the closed stream digest.
    ///
    /// The servo's own ID is deliberately **absent**: a COMMIT is broadcast
    /// and carries one tag that the whole fleet must reproduce.
    fn commit_tag(&self, key: MacKey, seq: u8, prefix: &[u8]) -> Tag {
        let mut h = HalfSipHasher::new(key);
        h.update(&[DOMAIN_COMMIT]);
        h.update(&self.keys.epoch.to_le_bytes());
        h.update(&[seq]);
        h.update(&self.stream.digest().to_bytes());
        h.update(prefix);
        h.finish()
    }

    /// Counters, alert state and lockout.
    fn record(&mut self, verdict: AuthVerdict) {
        match verdict {
            AuthVerdict::Pass => {
                self.consecutive_fails = 0;
            }
            v => {
                self.auth_fail_count = self.auth_fail_count.saturating_add(1);
                if v == AuthVerdict::Replay {
                    self.replay_drop_count = self.replay_drop_count.saturating_add(1);
                }
                if v.is_attack_evidence() {
                    self.consecutive_fails = self.consecutive_fails.saturating_add(1);
                    if self.consecutive_fails >= self.policy.lockout_after
                        && self.state == SecurityState::Secured
                    {
                        self.state = SecurityState::LockedOut;
                    }
                }
            }
        }
    }

    /// A COMMIT verdict has been applied — open a fresh stream for the next
    /// cycle. Called on both pass and fail: the host re-sends on failure.
    pub fn close_cycle(&mut self) {
        if self.state == SecurityState::Secured {
            self.stream.reset(self.keys.group);
        }
    }

    /// Bench probe / telemetry accessors.
    #[inline]
    pub const fn stream_frames(&self) -> u16 {
        self.stream.frames()
    }

    #[inline]
    pub const fn stream_bytes(&self) -> u32 {
        self.stream.bytes()
    }

    #[inline]
    pub const fn consecutive_fails(&self) -> u8 {
        self.consecutive_fails
    }

    /// Should status frames carry a tag right now?
    #[inline]
    pub const fn tags_replies(&self) -> bool {
        self.policy.tag_replies && matches!(self.state, SecurityState::Secured)
    }
}

/// Trailer length re-exported so the transport can size the payload view.
pub const AUTH_TRAILER_LEN: usize = TRAILER_LEN;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::Epoch;
    use osc_protocol::wire::ResultCode;

    fn keys() -> SessionKeys {
        SessionKeys {
            unicast: MacKey::from_bytes(&[1, 2, 3, 4, 5, 6, 7, 8]),
            group: MacKey::from_bytes(&[9, 10, 11, 12, 13, 14, 15, 16]),
            epoch: Epoch(7),
        }
    }

    fn secured() -> SecurityContext {
        let mut c = SecurityContext::new(Policy::FLIGHT);
        c.install(keys());
        c
    }

    /// Build a covered span `ID | LEN | INST | payload ‖ SEQ ‖ TAG`, tagged by
    /// the same routine the servo verifies with (host-side symmetry).
    fn authed_frame(
        ctx: &SecurityContext,
        id: u8,
        inst: Inst,
        payload: &[u8],
        seq: u8,
        broadcast: bool,
    ) -> alloc_vec::Vec {
        let len = (3 + payload.len() + TRAILER_LEN) as u8;
        let mut v = alloc_vec::Vec::new();
        v.push(id);
        v.push(len);
        v.push(inst.0);
        v.extend(payload);
        v.push(seq);
        let key = ctx.keys.for_frame(broadcast);
        let tag = if matches!(inst.opcode(), Some(Opcode::Commit)) {
            ctx.commit_tag(key, seq, v.as_slice())
        } else {
            ctx.frame_tag(key, seq, v.as_slice())
        };
        v.extend(&tag.to_bytes());
        v
    }

    /// Tiny fixed-capacity vector so these tests stay `no_std`-clean.
    mod alloc_vec {
        pub struct Vec {
            buf: [u8; 64],
            len: usize,
        }
        impl Vec {
            pub fn new() -> Self {
                Self {
                    buf: [0; 64],
                    len: 0,
                }
            }
            pub fn push(&mut self, b: u8) {
                self.buf[self.len] = b;
                self.len += 1;
            }
            pub fn extend(&mut self, s: &[u8]) {
                self.buf[self.len..self.len + s.len()].copy_from_slice(s);
                self.len += s.len();
            }
            pub fn as_slice(&self) -> &[u8] {
                &self.buf[..self.len]
            }
        }
    }

    const AUTH: u8 = Inst::FLAG_AUTH;

    #[test]
    fn a_correctly_tagged_frame_passes() {
        let mut c = secured();
        let inst = Inst::instruction(Opcode::Write, AUTH);
        let f = authed_frame(&c, 5, inst, &[0x00, 0x01, 0xAA], 1, false);
        assert_eq!(c.verify(inst, false, f.as_slice()), AuthVerdict::Pass);
    }

    #[test]
    fn a_flipped_payload_bit_fails() {
        let mut c = secured();
        let inst = Inst::instruction(Opcode::Write, AUTH);
        let f = authed_frame(&c, 5, inst, &[0x00, 0x01, 0xAA], 1, false);
        let mut bad = [0u8; 64];
        let n = f.as_slice().len();
        bad[..n].copy_from_slice(f.as_slice());
        bad[5] ^= 0x01;
        assert_eq!(c.verify(inst, false, &bad[..n]), AuthVerdict::BadTag);
    }

    #[test]
    fn replay_of_a_valid_frame_is_rejected() {
        let mut c = secured();
        let inst = Inst::instruction(Opcode::Write, AUTH);
        let f = authed_frame(&c, 5, inst, &[0xAA], 3, false);
        assert_eq!(c.verify(inst, false, f.as_slice()), AuthVerdict::Pass);
        assert_eq!(c.verify(inst, false, f.as_slice()), AuthVerdict::Replay);
    }

    #[test]
    fn an_untagged_effect_frame_is_missing_not_passing() {
        let mut c = secured();
        let inst = Inst::instruction(Opcode::Commit, 0);
        assert_eq!(c.verify(inst, true, &[0xFE, 0x03, inst.0]), AuthVerdict::Missing);
    }

    #[test]
    fn an_untagged_read_passes_under_the_default_policy() {
        let mut c = secured();
        let inst = Inst::instruction(Opcode::Read, 0);
        assert_eq!(c.verify(inst, false, &[0x05, 0x07, inst.0]), AuthVerdict::Pass);
    }

    #[test]
    fn held_writes_need_no_trailer() {
        let mut c = secured();
        let inst = Inst::instruction(Opcode::Gwrite, Inst::FLAG_HOLD);
        assert_eq!(c.verify(inst, true, &[0xFE, 0x07, inst.0]), AuthVerdict::Pass);
    }

    #[test]
    fn commit_tag_covers_the_folded_stream() {
        let mut c = secured();
        let held = Inst::instruction(Opcode::Gwrite, Inst::FLAG_HOLD);
        c.fold(held, &[0xFE, 0x07, held.0, 0x00, 0x01, 0x2C], &[]);

        let commit = Inst::instruction(Opcode::Commit, AUTH);
        let f = authed_frame(&c, 0xFE, commit, &[], 1, true);
        assert_eq!(c.verify(commit, true, f.as_slice()), AuthVerdict::Pass);
    }

    #[test]
    fn an_injected_frame_breaks_the_commit() {
        let mut c = secured();
        let held = Inst::instruction(Opcode::Gwrite, Inst::FLAG_HOLD);
        c.fold(held, &[0xFE, 0x07, held.0, 0x00, 0x01, 0x2C], &[]);

        // The host tags a COMMIT against the stream it believes it sent...
        let commit = Inst::instruction(Opcode::Commit, AUTH);
        let f = authed_frame(&c, 0xFE, commit, &[], 1, true);

        // ...but an attacker slips one more held write in first.
        c.fold(held, &[0xFE, 0x07, held.0, 0x00, 0x01, 0xFF], &[]);
        assert_eq!(c.verify(commit, true, f.as_slice()), AuthVerdict::BadTag);
    }

    #[test]
    fn a_suppressed_frame_breaks_the_commit() {
        let mut c = secured();
        let held = Inst::instruction(Opcode::Gwrite, Inst::FLAG_HOLD);
        c.fold(held, &[0xFE, 0x07, held.0, 0x00, 0x01, 0x2C], &[]);
        c.fold(held, &[0xFE, 0x07, held.0, 0x00, 0x02, 0x33], &[]);
        let commit = Inst::instruction(Opcode::Commit, AUTH);
        let f = authed_frame(&c, 0xFE, commit, &[], 1, true);

        // Same host intent, but the second write was jammed off the wire.
        let mut c2 = secured();
        c2.fold(held, &[0xFE, 0x07, held.0, 0x00, 0x01, 0x2C], &[]);
        assert_eq!(c2.verify(commit, true, f.as_slice()), AuthVerdict::BadTag);
    }

    #[test]
    fn unicast_and_broadcast_keys_are_not_interchangeable() {
        let mut c = secured();
        let inst = Inst::instruction(Opcode::Write, AUTH);
        // Tagged with the group key, presented as unicast.
        let f = authed_frame(&c, 5, inst, &[0xAA], 1, true);
        assert_eq!(c.verify(inst, false, f.as_slice()), AuthVerdict::BadTag);
    }

    #[test]
    fn lockout_after_three_bad_tags() {
        let mut c = secured();
        let inst = Inst::instruction(Opcode::Write, AUTH);
        let f = authed_frame(&c, 5, inst, &[0xAA], 1, false);
        let mut bad = [0u8; 64];
        let n = f.as_slice().len();
        bad[..n].copy_from_slice(f.as_slice());
        bad[3] ^= 0xFF;

        for _ in 0..3 {
            assert_eq!(c.verify(inst, false, &bad[..n]), AuthVerdict::BadTag);
        }
        assert_eq!(c.state(), SecurityState::LockedOut);
        assert_eq!(c.auth_fail_count, 3);

        // Effects refused, reads still answered so the host sees the alert.
        assert_eq!(c.verify(inst, false, &bad[..n]), AuthVerdict::LockedOut);
        let read = Inst::instruction(Opcode::Read, 0);
        assert_eq!(c.verify(read, false, &[0x05, 0x07, read.0]), AuthVerdict::Pass);
    }

    #[test]
    fn missing_tags_do_not_drive_lockout() {
        // A host that has not established a session yet must not be able to
        // lock the servo out of ever accepting one.
        let mut c = secured();
        let inst = Inst::instruction(Opcode::Commit, 0);
        for _ in 0..10 {
            assert_eq!(c.verify(inst, true, &[0xFE, 0x03, inst.0]), AuthVerdict::Missing);
        }
        assert_eq!(c.state(), SecurityState::Secured);
    }

    #[test]
    fn rekey_clears_lockout() {
        let mut c = secured();
        let inst = Inst::instruction(Opcode::Write, AUTH);
        let f = authed_frame(&c, 5, inst, &[0xAA], 1, false);
        let mut bad = [0u8; 64];
        let n = f.as_slice().len();
        bad[..n].copy_from_slice(f.as_slice());
        bad[3] ^= 0xFF;
        for _ in 0..3 {
            c.verify(inst, false, &bad[..n]);
        }
        assert_eq!(c.state(), SecurityState::LockedOut);

        let mut next = keys();
        next.epoch = Epoch(8);
        c.install(next);
        assert_eq!(c.state(), SecurityState::Secured);
        assert_eq!(c.consecutive_fails(), 0);
    }

    #[test]
    fn no_session_means_no_verification_against_a_zero_key() {
        let mut c = SecurityContext::new(Policy::FLIGHT);
        let inst = Inst::instruction(Opcode::Write, AUTH);
        assert_eq!(
            c.verify(inst, false, &[0x05, 0x09, inst.0, 0xAA, 1, 0, 0, 0, 0]),
            AuthVerdict::NoSession
        );
    }

    #[test]
    fn unsecured_context_does_not_fold() {
        let mut c = SecurityContext::new(Policy::FLIGHT);
        let held = Inst::instruction(Opcode::Gwrite, Inst::FLAG_HOLD);
        c.fold(held, &[0xFE, 0x07, held.0], &[]);
        assert_eq!(c.stream_frames(), 0);
    }

    #[test]
    fn status_frames_never_fold() {
        let mut c = secured();
        c.fold(Inst::status(ResultCode::Ok, false), &[0x05, 0x03, 0x80], &[]);
        assert_eq!(c.stream_frames(), 0);
    }

    #[test]
    fn epoch_binds_the_tag() {
        let c = secured();
        let inst = Inst::instruction(Opcode::Write, AUTH);
        let f = authed_frame(&c, 5, inst, &[0xAA], 1, false);

        // Same keys, different epoch -- the captured frame must not verify.
        let mut c2 = SecurityContext::new(Policy::FLIGHT);
        let mut k2 = keys();
        k2.epoch = Epoch(8);
        c2.install(k2);
        assert_eq!(c2.verify(inst, false, f.as_slice()), AuthVerdict::BadTag);
    }
}
