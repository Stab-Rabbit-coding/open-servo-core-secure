//! Which frames must be authenticated, and what happens when one is not
//! (`docs/security-architecture.md` §2.2, §2.7).
//!
//! **The policy lives on the servo, never on the wire.** An attacker chooses
//! the `AUTH` flag in the frames they send, so a design that keyed enforcement
//! off that flag would let them opt out of security by clearing a bit. The
//! servo decides what it requires; the flag only says what a frame *carries*.

use osc_protocol::wire::{Inst, Opcode};

/// Consecutive tag failures before the servo stops accepting effects at all.
///
/// This is what makes a 32-bit tag sound against an online forger: instead of
/// surviving ~2^31 blind attempts, an attacker is locked out and reported on
/// the third. Blind forgery becomes a detection event, not a waiting game.
pub const DEFAULT_LOCKOUT: u8 = 3;

/// Servo-side enforcement policy.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Policy {
    /// Reject effect-bearing frames that carry no valid tag. The flight
    /// default; a bench build may clear it to run unsecured.
    pub require_auth: bool,
    /// Also require tags on `READ`/`GREAD`/`PING`. Off by default: these carry
    /// no table effect, so forging one costs an attacker bus time and gains
    /// them telemetry they could have read passively anyway.
    pub require_auth_on_reads: bool,
    /// Tag our own status replies so the host can verify telemetry integrity.
    pub tag_replies: bool,
    /// Consecutive failures tolerated before lockout.
    pub lockout_after: u8,
}

impl Policy {
    /// Flight default: commands authenticated, replies tagged, reads open.
    pub const FLIGHT: Self = Self {
        require_auth: true,
        require_auth_on_reads: false,
        tag_replies: true,
        lockout_after: DEFAULT_LOCKOUT,
    };

    /// Bench default: everything open. Never ship this on an aircraft.
    pub const OPEN: Self = Self {
        require_auth: false,
        require_auth_on_reads: false,
        tag_replies: false,
        lockout_after: DEFAULT_LOCKOUT,
    };

    /// Does this instruction carry an effect that authentication must gate?
    ///
    /// The split is the same "does it change anything" line the transport
    /// already draws for staging (`osc-servo-transport.md` §6):
    ///
    /// - `WRITE`/`GWRITE` **with** `HOLD` only stage — they are covered
    ///   collectively by the COMMIT tag over the stream digest (§2.1), so they
    ///   need no trailer of their own.
    /// - `WRITE`/`GWRITE` **without** `HOLD` apply immediately, so each must
    ///   carry its own tag.
    /// - `COMMIT` is the apply instant — always tagged.
    /// - `MGMT` reboots, saves, reassigns identity — always tagged.
    /// - `PING`/`READ`/`GREAD` have no table effect.
    pub const fn effect_bearing(inst: Inst) -> bool {
        match inst.opcode() {
            Some(Opcode::Commit | Opcode::Mgmt) => true,
            Some(Opcode::Write | Opcode::Gwrite) => !inst.hold(),
            Some(Opcode::Ping | Opcode::Read | Opcode::Gread) => false,
            None => false,
        }
    }

    /// Is this frame folded into the stream digest?
    ///
    /// **Writes only.** The stream is exactly the set of changes a COMMIT
    /// applies, and three exclusions matter:
    ///
    /// - `COMMIT` itself must **not** fold. Its tag is computed *over* the
    ///   digest, so folding it first would be circular: the host cannot
    ///   include a frame in a digest it has not finished building in order to
    ///   tag that frame.
    /// - `MGMT` carries its own inline tag (§2.2) and applies immediately, so
    ///   it has no stake in the COMMIT gate.
    /// - `PING`/`READ`/`GREAD` have no effect, and folding them would make the
    ///   digest depend on the host's telemetry polling cadence — which the
    ///   host may legitimately vary between commits.
    /// - Status frames are replies under a responder's key, not the host's.
    ///
    /// Both held and unheld writes fold: unheld ones are additionally tagged
    /// inline, and folding them too binds them into the next cycle at no cost.
    /// The host applies the identical rule, so the digests agree.
    pub const fn folds_into_stream(inst: Inst) -> bool {
        if inst.is_status() {
            return false;
        }
        matches!(inst.opcode(), Some(Opcode::Write | Opcode::Gwrite))
    }

    /// Does this frame need to arrive with a valid tag, under this policy?
    pub const fn requires_tag(&self, inst: Inst) -> bool {
        if !self.require_auth {
            return false;
        }
        if Self::effect_bearing(inst) {
            return true;
        }
        self.require_auth_on_reads
    }
}

impl Default for Policy {
    fn default() -> Self {
        Self::FLIGHT
    }
}

/// The message plane's verdict on one frame — ANDed with the CRC verdict at
/// the transport's existing gate (`route.rs::verify`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AuthVerdict {
    /// Verified, or legitimately not required. Staged effects may apply.
    Pass,
    /// Required but absent: no `AUTH` trailer on an effect-bearing frame.
    Missing,
    /// Trailer present but the tag did not match.
    BadTag,
    /// Tag matched a sequence already seen, or one behind the window.
    Replay,
    /// No session established, and policy demands one.
    NoSession,
    /// Locked out after too many consecutive failures.
    LockedOut,
}

impl AuthVerdict {
    #[inline]
    pub const fn is_pass(self) -> bool {
        matches!(self, AuthVerdict::Pass)
    }

    /// Does this verdict count toward the lockout counter?
    ///
    /// `Missing` and `NoSession` do **not**: they are configuration or
    /// sequencing faults, typically a host that has not established a session
    /// yet, and letting them drive lockout would turn a misconfigured startup
    /// into a servo that refuses to talk. `BadTag` and `Replay` are the ones
    /// that indicate an actual adversary.
    #[inline]
    pub const fn is_attack_evidence(self) -> bool {
        matches!(self, AuthVerdict::BadTag | AuthVerdict::Replay)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inst(op: Opcode, flags: u8) -> Inst {
        Inst::instruction(op, flags)
    }

    #[test]
    fn held_writes_are_covered_by_commit_not_a_trailer() {
        let held = inst(Opcode::Gwrite, Inst::FLAG_HOLD);
        assert!(!Policy::effect_bearing(held));
        assert!(Policy::folds_into_stream(held));
        assert!(!Policy::FLIGHT.requires_tag(held));
    }

    #[test]
    fn unheld_writes_need_their_own_tag() {
        let bare = inst(Opcode::Write, 0);
        assert!(Policy::effect_bearing(bare));
        assert!(Policy::FLIGHT.requires_tag(bare));
    }

    #[test]
    fn commit_and_mgmt_always_need_a_tag() {
        for op in [Opcode::Commit, Opcode::Mgmt] {
            let i = inst(op, 0);
            assert!(Policy::effect_bearing(i), "{op:?}");
            assert!(Policy::FLIGHT.requires_tag(i), "{op:?}");
        }
        // HOLD must not exempt COMMIT/MGMT the way it exempts writes.
        assert!(Policy::effect_bearing(inst(Opcode::Commit, Inst::FLAG_HOLD)));
        assert!(Policy::effect_bearing(inst(Opcode::Mgmt, Inst::FLAG_HOLD)));
    }

    #[test]
    fn commit_must_not_fold_into_the_digest_it_closes() {
        // Circularity guard: the COMMIT tag is computed OVER the stream
        // digest, so folding the COMMIT frame in first would make the host's
        // tag uncomputable. Regression-pins the ordering bug.
        assert!(!Policy::folds_into_stream(inst(Opcode::Commit, 0)));
        assert!(!Policy::folds_into_stream(inst(Opcode::Commit, Inst::FLAG_AUTH)));
    }

    #[test]
    fn mgmt_carries_its_own_tag_and_stays_out_of_the_stream() {
        assert!(!Policy::folds_into_stream(inst(Opcode::Mgmt, 0)));
    }

    #[test]
    fn both_held_and_unheld_writes_fold() {
        for op in [Opcode::Write, Opcode::Gwrite] {
            assert!(Policy::folds_into_stream(inst(op, Inst::FLAG_HOLD)), "{op:?} held");
            assert!(Policy::folds_into_stream(inst(op, 0)), "{op:?} unheld");
        }
    }

    #[test]
    fn reads_are_open_by_default_but_configurable() {
        let r = inst(Opcode::Read, 0);
        assert!(!Policy::effect_bearing(r));
        assert!(!Policy::FLIGHT.requires_tag(r));
        let strict = Policy {
            require_auth_on_reads: true,
            ..Policy::FLIGHT
        };
        assert!(strict.requires_tag(r));
    }

    #[test]
    fn reads_never_fold_into_the_stream() {
        // A read has no effect, so folding it would make the digest depend on
        // telemetry polling cadence -- which the host may vary freely.
        for op in [Opcode::Ping, Opcode::Read, Opcode::Gread] {
            assert!(!Policy::folds_into_stream(inst(op, 0)), "{op:?}");
        }
    }

    #[test]
    fn status_frames_never_fold() {
        use osc_protocol::wire::ResultCode;
        assert!(!Policy::folds_into_stream(Inst::status(ResultCode::Ok, false)));
    }

    #[test]
    fn open_policy_requires_nothing() {
        for op in [Opcode::Commit, Opcode::Mgmt, Opcode::Write] {
            assert!(!Policy::OPEN.requires_tag(inst(op, 0)), "{op:?}");
        }
    }

    #[test]
    fn only_real_attack_evidence_drives_lockout() {
        assert!(AuthVerdict::BadTag.is_attack_evidence());
        assert!(AuthVerdict::Replay.is_attack_evidence());
        assert!(!AuthVerdict::Missing.is_attack_evidence());
        assert!(!AuthVerdict::NoSession.is_attack_evidence());
        assert!(!AuthVerdict::Pass.is_attack_evidence());
        assert!(!AuthVerdict::LockedOut.is_attack_evidence());
    }
}
