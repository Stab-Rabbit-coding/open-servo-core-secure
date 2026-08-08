//! Replay protection: epoch + sequence (`docs/security-architecture.md` §2.5).
//!
//! The wire carries only the **low 8 bits** of a 32-bit sequence. That is
//! enough because the protocol's loss model is bounded: frames die on CRC and
//! the host retries within its timeout contract (`osc-native-protocol.md`
//! §3.4, §5.3 L1), so the receiver never falls more than a few counts behind.
//! A gap wider than the forward window forces a re-key — a cold path, and
//! therefore affordable.
//!
//! A purely implicit counter (no wire field at all) would be cheaper still but
//! desynchronises permanently on the first dropped frame, which this protocol
//! produces by design.

/// How far ahead of `last` a sequence byte may jump and still be accepted.
///
/// 127 is half the 8-bit space: it makes "ahead" and "behind" unambiguous
/// under wrapping, so a replayed old value can never be mistaken for a large
/// forward jump.
pub const FORWARD_WINDOW: u8 = 127;

/// Why a sequence was refused.
///
/// There is deliberately only one variant. With an 8-bit sequence and a
/// half-space window, "too far ahead to judge" and "behind" are the *same*
/// observation — `seq.wrapping_sub(last) > 127` — and no receiver-side
/// evidence separates them. Reporting a distinction the data cannot support
/// would be the same class of mistake the transport's fault contract forbids
/// (`osc-native-protocol.md` §3.4: no decision from evidence that lies).
/// Both cases are refused, both raise the same alert, and both are cleared by
/// the same remedy — a re-key.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SeqReject {
    /// At or behind the last accepted value, or beyond the forward window.
    Replay,
}

/// Sliding sequence window for one epoch.
#[derive(Copy, Clone, Debug)]
pub struct ReplayWindow {
    last: u8,
    /// False until the first sequence is accepted, so a session can start at
    /// any value the host chooses.
    primed: bool,
}

impl ReplayWindow {
    pub const fn new() -> Self {
        Self {
            last: 0,
            primed: false,
        }
    }

    /// Reset for a new epoch.
    pub fn restart(&mut self) {
        *self = Self::new();
    }

    /// Test `seq` **without** consuming it.
    ///
    /// Separating the test from the commit matters: the transport dispatches
    /// ahead of its verdict, so a sequence must not advance until the frame's
    /// CRC *and* tag have both passed. Otherwise a corrupted or forged frame
    /// would burn a sequence number and desynchronise the honest stream.
    pub const fn check(&self, seq: u8) -> Result<(), SeqReject> {
        if !self.primed {
            return Ok(());
        }
        // Distance forward from `last`, modulo 256.
        let ahead = seq.wrapping_sub(self.last);
        if ahead == 0 {
            Err(SeqReject::Replay)
        } else if ahead <= FORWARD_WINDOW {
            Ok(())
        } else {
            // `ahead > 127` reads as "behind" under wrapping.
            Err(SeqReject::Replay)
        }
    }

    /// Accept `seq` as the new high-water mark. Call only after the frame has
    /// fully verified.
    pub fn commit(&mut self, seq: u8) {
        self.last = seq;
        self.primed = true;
    }

    /// Test and, on success, consume in one step.
    pub fn accept(&mut self, seq: u8) -> Result<(), SeqReject> {
        self.check(seq)?;
        self.commit(seq);
        Ok(())
    }

    #[inline]
    pub const fn last(&self) -> u8 {
        self.last
    }

    #[inline]
    pub const fn primed(&self) -> bool {
        self.primed
    }
}

impl Default for ReplayWindow {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_sequence_is_always_accepted() {
        let mut w = ReplayWindow::new();
        assert_eq!(w.accept(200), Ok(()));
        assert_eq!(w.last(), 200);
    }

    #[test]
    fn strictly_increasing_is_accepted() {
        let mut w = ReplayWindow::new();
        w.accept(10).unwrap();
        assert_eq!(w.accept(11), Ok(()));
        assert_eq!(w.accept(12), Ok(()));
    }

    #[test]
    fn exact_replay_is_rejected() {
        let mut w = ReplayWindow::new();
        w.accept(10).unwrap();
        assert_eq!(w.accept(10), Err(SeqReject::Replay));
    }

    #[test]
    fn older_values_are_rejected() {
        let mut w = ReplayWindow::new();
        w.accept(100).unwrap();
        for old in [99u8, 90, 50, 1, 0] {
            assert_eq!(w.accept(old), Err(SeqReject::Replay), "seq {old}");
        }
    }

    #[test]
    fn gaps_inside_the_window_are_accepted() {
        // Dropped frames are normal (CRC failures, host retries).
        let mut w = ReplayWindow::new();
        w.accept(10).unwrap();
        assert_eq!(w.accept(10u8.wrapping_add(FORWARD_WINDOW)), Ok(()));
    }

    #[test]
    fn wrapping_forward_is_accepted() {
        let mut w = ReplayWindow::new();
        w.accept(250).unwrap();
        assert_eq!(w.accept(3), Ok(()), "250 -> 3 is +9, forward");
        assert_eq!(w.last(), 3);
    }

    #[test]
    fn wrapping_backward_is_rejected() {
        let mut w = ReplayWindow::new();
        w.accept(3).unwrap();
        assert_eq!(w.accept(250), Err(SeqReject::Replay), "3 -> 250 is behind");
    }

    #[test]
    fn check_does_not_consume() {
        // The dispatch-before-verdict spine requires this: a frame that later
        // fails CRC must not have advanced the window.
        let mut w = ReplayWindow::new();
        w.accept(10).unwrap();
        assert_eq!(w.check(11), Ok(()));
        assert_eq!(w.check(11), Ok(()), "check must be idempotent");
        assert_eq!(w.last(), 10, "check must not advance");
        w.commit(11);
        assert_eq!(w.last(), 11);
    }

    #[test]
    fn restart_reprimes() {
        let mut w = ReplayWindow::new();
        w.accept(200).unwrap();
        w.restart();
        assert!(!w.primed());
        assert_eq!(w.accept(5), Ok(()), "a new epoch may start anywhere");
    }
}
