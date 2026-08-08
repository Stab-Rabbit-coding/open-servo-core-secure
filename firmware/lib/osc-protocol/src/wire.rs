//! osc-native wire primitives: ID, packed `INST` byte, and frame span math
//! (`docs/osc-native-protocol.md` sec 3, sec 5, sec 9). Layout only -- no buffering.

/// Frame ID byte. `0x01..=0xF9` unicast, `0xFE` broadcast; `0x00`/`0xFF` and
/// `0xFA..=0xFD` never address a servo on the wire (sec 3.1).
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Id(pub u8);

impl Id {
    pub const BROADCAST: Self = Self(0xFE);

    #[inline]
    pub const fn new(b: u8) -> Self {
        Self(b)
    }

    /// Validated unicast constructor; accepts only `0x01..=0xF9`.
    #[inline]
    pub const fn try_unicast(b: u8) -> Option<Self> {
        match b {
            0x01..=0xF9 => Some(Self(b)),
            _ => None,
        }
    }

    #[inline]
    pub const fn as_byte(self) -> u8 {
        self.0
    }

    #[inline]
    pub const fn is_broadcast(self) -> bool {
        self.0 == 0xFE
    }

    #[inline]
    pub const fn is_unicast(self) -> bool {
        matches!(self.0, 0x01..=0xF9)
    }

    #[inline]
    pub const fn is_valid(self) -> bool {
        self.is_unicast() || self.is_broadcast()
    }

    /// Servo-side "is this frame for me": true when the frame ID (`self`) is
    /// broadcast or equals the servo's own `other`.
    #[inline]
    pub const fn addresses(self, other: Id) -> bool {
        self.is_broadcast() || self.0 == other.0
    }
}

/// Operational baud (sec 2): four rates, default 1M. Recovery is the rescue
/// break's job (sec 9.1), so no crawl-speed fallback exists. The discriminant
/// IS the `baud_rate_idx` config register value -- wire ABI, do not reorder.
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum BaudRate {
    #[default]
    B500000 = 0,
    B1000000 = 1,
    B2000000 = 2,
    B3000000 = 3,
}

impl BaudRate {
    /// The sec 9.1 rescue rate: the option floor, entered only via rescue
    /// break -- volatile, config register untouched.
    pub const RESCUE: Self = Self::B500000;

    pub const fn as_idx(self) -> u8 {
        self as u8
    }

    pub const fn as_hz(self) -> u32 {
        match self {
            BaudRate::B500000 => 500_000,
            BaudRate::B1000000 => 1_000_000,
            BaudRate::B2000000 => 2_000_000,
            BaudRate::B3000000 => 3_000_000,
        }
    }

    pub const fn from_idx(idx: u8) -> Option<Self> {
        match idx {
            0 => Some(BaudRate::B500000),
            1 => Some(BaudRate::B1000000),
            2 => Some(BaudRate::B2000000),
            3 => Some(BaudRate::B3000000),
            _ => None,
        }
    }
}

/// Instruction opcode, `INST` bits [6:4] (sec 5). `0x0` is invalid.
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Opcode {
    Ping = 0x1,
    Read = 0x2,
    Write = 0x3,
    Commit = 0x4,
    Gread = 0x5,
    Gwrite = 0x6,
    Mgmt = 0x7,
}

impl Opcode {
    /// `b` is the already-extracted 3-bit field; `0x0` and `>0x7` reject.
    #[inline]
    pub const fn from_bits(b: u8) -> Option<Opcode> {
        match b {
            0x1 => Some(Opcode::Ping),
            0x2 => Some(Opcode::Read),
            0x3 => Some(Opcode::Write),
            0x4 => Some(Opcode::Commit),
            0x5 => Some(Opcode::Gread),
            0x6 => Some(Opcode::Gwrite),
            0x7 => Some(Opcode::Mgmt),
            _ => None,
        }
    }
}

/// Status result code, `INST` bits [6:2] (sec 5.3). `9..=31` reserved/invalid.
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ResultCode {
    Ok = 0,
    Instruction = 1,
    Range = 2,
    Access = 3,
    Validation = 4,
    Busy = 5,
    Limit = 6,
    PredecessorSilent = 7,
    Hardware = 8,
    /// The frame carried no valid AUTH trailer and policy required one
    /// (`docs/security-architecture.md` sec 2.2). Covers a missing trailer, a
    /// bad tag, and a replayed sequence alike: the three are indistinguishable
    /// to a host that is behaving correctly, and separating them on the wire
    /// would tell an attacker which of their guesses was closer.
    Unauthenticated = 9,
    /// Consecutive authentication failures tripped the lockout; effect-bearing
    /// frames are refused until a successful re-key (sec 2.7).
    SecurityLockout = 10,
}

impl ResultCode {
    /// `b` is the already-extracted 5-bit field; `9..=31` reject.
    #[inline]
    pub const fn from_bits(b: u8) -> Option<ResultCode> {
        match b {
            0 => Some(ResultCode::Ok),
            1 => Some(ResultCode::Instruction),
            2 => Some(ResultCode::Range),
            3 => Some(ResultCode::Access),
            4 => Some(ResultCode::Validation),
            5 => Some(ResultCode::Busy),
            6 => Some(ResultCode::Limit),
            7 => Some(ResultCode::PredecessorSilent),
            8 => Some(ResultCode::Hardware),
            9 => Some(ResultCode::Unauthenticated),
            10 => Some(ResultCode::SecurityLockout),
            _ => None,
        }
    }
}

/// MGMT sub-op, payload byte 0 (sec 9).
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MgmtOp {
    Enum = 0x01,
    Assign = 0x02,
    Save = 0x03,
    Reboot = 0x04,
    Factory = 0x05,
    Cal = 0x06,
    /// Open a session: `[epoch(2 LE), host_nonce(16)]`, replies with the
    /// servo nonce and SE serial (`docs/security-architecture.md` sec 3).
    SecInit = 0x07,
    /// Deliver the wrapped group key: `[wrapped(16), tag(16)]` (sec 3).
    SecKey = 0x08,
    /// Attestation: `[challenge(32)]` -> ECDSA P-256 signature (sec 5.2).
    /// ~520 ms; requires a bus-quiet window (sec 4.3).
    SecAttest = 0x09,
    /// Re-key under a fresh epoch. Also the documented remedy for a lockout
    /// (sec 2.7). Requires a bus-quiet window.
    SecRekey = 0x0A,
}

impl MgmtOp {
    #[inline]
    pub const fn from_byte(b: u8) -> Option<MgmtOp> {
        match b {
            0x01 => Some(MgmtOp::Enum),
            0x02 => Some(MgmtOp::Assign),
            0x03 => Some(MgmtOp::Save),
            0x04 => Some(MgmtOp::Reboot),
            0x05 => Some(MgmtOp::Factory),
            0x06 => Some(MgmtOp::Cal),
            0x07 => Some(MgmtOp::SecInit),
            0x08 => Some(MgmtOp::SecKey),
            0x09 => Some(MgmtOp::SecAttest),
            0x0A => Some(MgmtOp::SecRekey),
            _ => None,
        }
    }

    /// Does this sub-op drive the secure element, and therefore require a
    /// bus-quiet window (`docs/security-architecture.md` sec 4.3)?
    ///
    /// ECC204 commands block for 20–500 ms [REF-SE-002], so these must never
    /// be serviced inline with bus traffic.
    #[inline]
    pub const fn needs_quiet_window(self) -> bool {
        matches!(
            self,
            MgmtOp::SecInit | MgmtOp::SecKey | MgmtOp::SecAttest | MgmtOp::SecRekey
        )
    }
}

/// Packed `INST` byte. Bit 7 selects the layout: instruction (opcode [6:4] +
/// flags [3:0]) or status (result [6:2] + bit 1 reserved + ALERT bit 0).
/// Every bit pattern is representable; validity comes from the typed
/// accessors.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Inst(pub u8);

impl Inst {
    pub const FLAG_HOLD: u8 = 1 << 0;
    /// Bit 0 on READ/GREAD: the payload names a profile slot (sec 5.2).
    pub const FLAG_PROFILE: u8 = Self::FLAG_HOLD;
    /// Bit 1: the frame carries an AUTH trailer -- the last 5 payload bytes
    /// are `SEQ ‖ TAG[4]` (`docs/security-architecture.md` sec 2.3). This is
    /// the extension bit sec 3.1/sec 5 reserved; a frame without it is
    /// byte-identical to the pre-security protocol.
    ///
    /// The flag says what a frame CARRIES, never what a servo REQUIRES:
    /// enforcement is a servo-side policy register, because an attacker
    /// chooses the flags in the frames they send.
    pub const FLAG_AUTH: u8 = 1 << 1;
    pub const FLAG_NOREPLY: u8 = 1 << 2;
    pub const FLAG_PER_TARGET: u8 = 1 << 3;

    const STATUS_BIT: u8 = 0x80;
    const ALERT_BIT: u8 = 1 << 0;

    #[inline]
    pub const fn instruction(op: Opcode, flags: u8) -> Self {
        Self(((op as u8) << 4) | (flags & 0x0F))
    }

    #[inline]
    pub const fn status(result: ResultCode, alert: bool) -> Self {
        let mut b = Self::STATUS_BIT | ((result as u8) << 2);
        if alert {
            b |= Self::ALERT_BIT;
        }
        Self(b)
    }

    /// A status frame carrying an AUTH trailer. Bit 1 is the same reserved
    /// bit in the status layout as in the instruction layout, so replies and
    /// commands signal authentication identically
    /// (`docs/security-architecture.md` sec 2.3).
    #[inline]
    pub const fn status_authenticated(result: ResultCode, alert: bool) -> Self {
        Self(Self::status(result, alert).0 | Self::FLAG_AUTH)
    }

    #[inline]
    pub const fn is_status(self) -> bool {
        self.0 & Self::STATUS_BIT != 0
    }

    /// Opcode of an instruction frame; `None` for status frames or opcode `0`.
    #[inline]
    pub const fn opcode(self) -> Option<Opcode> {
        if self.is_status() {
            return None;
        }
        Opcode::from_bits((self.0 >> 4) & 0x07)
    }

    #[inline]
    pub const fn hold(self) -> bool {
        self.0 & Self::FLAG_HOLD != 0
    }

    /// Bit 0's read-side meaning (sec 5): on READ/GREAD the payload names a
    /// profile slot instead of addr+count (sec 5.2). Same bit as HOLD.
    #[inline]
    pub const fn profile(self) -> bool {
        self.hold()
    }

    /// Does this frame carry an AUTH trailer (`FLAG_AUTH`)?
    ///
    /// Meaningful on instruction frames and on status frames alike: a servo
    /// tags its replies under the same session key so the host can verify
    /// telemetry integrity (`docs/security-architecture.md` sec 2.2).
    #[inline]
    pub const fn authenticated(self) -> bool {
        self.0 & Self::FLAG_AUTH != 0
    }

    #[inline]
    pub const fn noreply(self) -> bool {
        self.0 & Self::FLAG_NOREPLY != 0
    }

    #[inline]
    pub const fn per_target(self) -> bool {
        self.0 & Self::FLAG_PER_TARGET != 0
    }

    /// Result code of a status frame; `None` for instruction frames or a
    /// reserved code.
    #[inline]
    pub const fn result(self) -> Option<ResultCode> {
        if !self.is_status() {
            return None;
        }
        ResultCode::from_bits((self.0 >> 2) & 0x1F)
    }

    #[inline]
    pub const fn alert(self) -> bool {
        self.0 & Self::ALERT_BIT != 0
    }
}

/// Max payload bytes; sized so the largest frame fits whole in the ring (sec 3.1).
pub const MAX_PAYLOAD: u8 = 252;

/// sec 7 default: chain reclaim + host timeout, not a reply-time prescription.
pub const DEFAULT_RESPONSE_DEADLINE_US: u16 = 60;

/// sec 3.4: byte-times of ring silence that kill a parked partial frame -- the
/// fallback death authority servo-side, and the host's post-garble pacing gap.
pub const STARVE_HORIZON_BYTE_TIMES: u32 = 64;

/// sec 9.1 rescue pulse: the servo sampler declares rescue at this much
/// continuous dominant low; hosts send ~1 ms for sampler-jitter margin.
pub const RESCUE_PULSE_MIN_US: u32 = 300;

/// UID field width in bytes (sec 9.2): UUID-width, fixed. A chip fills it
/// LSB-first from its silicon ID and zero-pads the tail (the V006's 96-bit
/// ESIG leaves the top four bytes zero); no catalog MCU exceeds 128 bits.
pub const UID_LEN: usize = 16;

/// sec 9.2 ENUM reply slots: a broadcast-ENUM reply delays its trigger by
/// `0..ENUM_REPLY_SLOTS` byte-times, drawn from the responder's UID CRC
/// XOR its free-running tick. Same-die matchers run cycle-identical
/// firmware and otherwise answer in unison -- and two near-equal frames
/// superimposed sub-bit-aligned read back as ONE clean frame instead of
/// the collision garble the walk descends on.
pub const ENUM_REPLY_SLOTS: u8 = 16;

/// TX-buffer alignment byte at offset 0 (sec 3.2): keeps the hardware CRC feed
/// halfword-aligned and even; a CRC no-op (leading zero, init = 0). Not part
/// of the wire checksum definition.
pub const ALIGN_BYTE: u8 = 0x00;

/// `LEN` for a `p`-byte payload: `INST + payload + CRC` = `3 + p`.
/// Caller keeps `p <= MAX_PAYLOAD`.
#[inline]
pub const fn len_for(p: u8) -> u8 {
    3 + p
}

/// Payload length recovered from `LEN`: `len - 3` (validate guarantees
/// `LEN >= 3`).
#[inline]
pub const fn payload_len(len: u8) -> u8 {
    len - 3
}

/// Ring bytes for a frame including the break byte: `3 + len` (max 258).
#[inline]
pub const fn footprint(len: u8) -> usize {
    3 + len as usize
}

/// Anchor-inclusive feed-span length (`1 + len`): the wire checksum covers
/// `ID .. payload` (sec 3.2), but a span counted from the anchor includes the
/// break's `0x00` no-op byte -- the form receivers and buffers use.
#[inline]
pub const fn covered_len(len: u8) -> usize {
    1 + len as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_classification() {
        assert!(Id::BROADCAST.is_broadcast());
        assert!(Id::BROADCAST.is_valid());
        assert!(!Id::BROADCAST.is_unicast());
        assert!(Id::new(0x01).is_unicast());
        assert!(Id::new(0xF9).is_unicast());
        assert!(!Id::new(0x00).is_valid());
        assert!(!Id::new(0xFF).is_valid());
        assert!(!Id::new(0xFA).is_valid());
    }

    #[test]
    fn id_try_unicast() {
        assert_eq!(Id::try_unicast(0x00), None);
        assert_eq!(Id::try_unicast(0x01), Some(Id::new(0x01)));
        assert_eq!(Id::try_unicast(0xF9), Some(Id::new(0xF9)));
        assert_eq!(Id::try_unicast(0xFA), None);
        assert_eq!(Id::try_unicast(0xFE), None);
    }

    #[test]
    fn id_addresses() {
        assert!(Id::BROADCAST.addresses(Id::new(0x05)));
        assert!(Id::new(0x05).addresses(Id::new(0x05)));
        assert!(!Id::new(0x05).addresses(Id::new(0x06)));
    }

    #[test]
    fn opcode_from_bits() {
        assert_eq!(Opcode::from_bits(0x0), None);
        assert_eq!(Opcode::from_bits(0x1), Some(Opcode::Ping));
        assert_eq!(Opcode::from_bits(0x7), Some(Opcode::Mgmt));
    }

    #[test]
    fn result_from_bits() {
        assert_eq!(ResultCode::from_bits(0), Some(ResultCode::Ok));
        assert_eq!(
            ResultCode::from_bits(7),
            Some(ResultCode::PredecessorSilent)
        );
        assert_eq!(ResultCode::from_bits(8), Some(ResultCode::Hardware));
        assert_eq!(ResultCode::from_bits(9), None);
        assert_eq!(ResultCode::from_bits(31), None);
    }

    #[test]
    fn mgmt_from_byte() {
        assert_eq!(MgmtOp::from_byte(0x00), None);
        assert_eq!(MgmtOp::from_byte(0x01), Some(MgmtOp::Enum));
        assert_eq!(MgmtOp::from_byte(0x05), Some(MgmtOp::Factory));
        assert_eq!(MgmtOp::from_byte(0x06), Some(MgmtOp::Cal));
        assert_eq!(MgmtOp::from_byte(0x07), Some(MgmtOp::SecInit));
        assert_eq!(MgmtOp::from_byte(0x0A), Some(MgmtOp::SecRekey));
        assert_eq!(MgmtOp::from_byte(0x0B), None);
    }

    #[test]
    fn only_the_sec_ops_need_a_quiet_window() {
        for op in [MgmtOp::Enum, MgmtOp::Assign, MgmtOp::Save, MgmtOp::Reboot, MgmtOp::Factory, MgmtOp::Cal] {
            assert!(!op.needs_quiet_window(), "{op:?}");
        }
        for op in [MgmtOp::SecInit, MgmtOp::SecKey, MgmtOp::SecAttest, MgmtOp::SecRekey] {
            assert!(op.needs_quiet_window(), "{op:?}");
        }
    }

    #[test]
    fn auth_flag_is_bit_1_and_orthogonal() {
        assert_eq!(Inst::FLAG_AUTH, 0b0000_0010);
        let i = Inst::instruction(Opcode::Write, Inst::FLAG_AUTH | Inst::FLAG_HOLD);
        assert!(i.authenticated());
        assert!(i.hold());
        assert!(!i.noreply());
        assert!(!i.per_target());
        assert_eq!(i.opcode(), Some(Opcode::Write));
        // Absent by default -- an unauthenticated frame is byte-identical to
        // the pre-security protocol.
        assert!(!Inst::instruction(Opcode::Write, 0).authenticated());
    }

    #[test]
    fn auth_flag_does_not_disturb_the_status_layout() {
        let plain = Inst::status(ResultCode::Range, true);
        let authed = Inst::status_authenticated(ResultCode::Range, true);
        assert!(authed.is_status());
        assert!(authed.authenticated());
        assert!(!plain.authenticated());
        // Result code and ALERT survive the extra bit untouched.
        assert_eq!(authed.result(), plain.result());
        assert_eq!(authed.alert(), plain.alert());
        assert_eq!(authed.0, plain.0 | Inst::FLAG_AUTH);
    }

    #[test]
    fn security_result_codes_round_trip() {
        assert_eq!(ResultCode::from_bits(9), Some(ResultCode::Unauthenticated));
        assert_eq!(ResultCode::from_bits(10), Some(ResultCode::SecurityLockout));
        assert_eq!(ResultCode::from_bits(11), None);
        // They must survive the status packing/unpacking round trip.
        for rc in [ResultCode::Unauthenticated, ResultCode::SecurityLockout] {
            assert_eq!(Inst::status(rc, false).result(), Some(rc));
            assert_eq!(Inst::status_authenticated(rc, true).result(), Some(rc));
        }
    }

    #[test]
    fn inst_instruction_roundtrip() {
        let i = Inst::instruction(Opcode::Write, Inst::FLAG_HOLD | Inst::FLAG_NOREPLY);
        assert!(!i.is_status());
        assert_eq!(i.opcode(), Some(Opcode::Write));
        assert!(i.hold());
        assert!(i.noreply());
        assert!(!i.per_target());
        assert_eq!(i.result(), None);
    }

    #[test]
    fn inst_profile_is_bit0_read_side() {
        let r = Inst::instruction(Opcode::Read, Inst::FLAG_PROFILE);
        assert!(r.profile());
        assert!(r.hold());
        let plain = Inst::instruction(Opcode::Read, 0);
        assert!(!plain.profile());
    }

    #[test]
    fn inst_status_roundtrip() {
        let s = Inst::status(ResultCode::Range, true);
        assert!(s.is_status());
        assert_eq!(s.result(), Some(ResultCode::Range));
        assert!(s.alert());
        assert_eq!(s.opcode(), None);
    }

    #[test]
    fn inst_status_no_flags() {
        let s = Inst::status(ResultCode::Ok, false);
        assert_eq!(s.0, 0x80);
        assert!(!s.alert());
        assert_eq!(s.result(), Some(ResultCode::Ok));
    }

    #[test]
    fn span_math() {
        assert_eq!(len_for(2), 5);
        assert_eq!(payload_len(5), 2);
        // Odd payload: no pad, LEN even-legal (sec 3.1).
        assert_eq!(len_for(3), 6);
        assert_eq!(payload_len(6), 3);
        // PING: empty payload.
        assert_eq!(len_for(0), 3);
        // Largest frame.
        assert_eq!(len_for(MAX_PAYLOAD), 255);
        assert_eq!(footprint(255), 258);
        assert_eq!(covered_len(255), 256);
    }

    #[test]
    fn span_covered_matches_vectors() {
        // "00 05 07 30 80 01 2C 01" -- WRITE id 5, p=4 payload, LEN 7.
        assert_eq!(len_for(4), 7);
        assert_eq!(covered_len(7), 8);
        assert_eq!(footprint(7), 10);
    }
}
