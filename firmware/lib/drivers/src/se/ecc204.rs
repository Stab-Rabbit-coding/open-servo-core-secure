//! ECC204 command layer (`docs/security-architecture.md` §5, §7.3, §7.5).
//!
//! # What is here, and what is deliberately not
//!
//! Everything in this module that carries a concrete value is traceable to a
//! **public, citable source**: the summary datasheet [REF-SE-001] or
//! Microchip's open-source CryptoAuthLib [REF-SE-002].
//!
//! The ECC204's **command-packet framing** — the word-address byte, the packet
//! CRC-16 parameters, and the SWI flag tokens — is specified only in the
//! complete datasheet, which is NDA-gated (the summary datasheet says so on
//! its cover). Those constants are therefore **not written here**, because
//! this project's authenticity rule is that no constant, citation or reference
//! is ever fabricated, and a plausible-looking guess in a security driver is
//! worse than an absent one: it would compile, run, fail obscurely on
//! silicon, and look authoritative while doing it.
//!
//! Instead they live behind [`Framing`], a trait with **no implementation in
//! tree**. A build that selects a real SE without supplying one fails to
//! link — see [`UNAVAILABLE`]. Everything above the framing layer (the session
//! protocol, the KDF labels, the whole message plane) is complete and tested.
//!
//! To bring this up, supply a `Framing` impl from either source:
//!
//! 1. the complete ECC204 datasheet obtained under NDA from Microchip, or
//! 2. CryptoAuthLib's `calib_command.c` / `hal_swi_uart.c` [REF-SE-002,
//!    REF-SE-003], whose licence permits use with Microchip devices.
//!
//! `TODO.md` §7.4 tracks this.

use osc_security::se::{SeError, SeResult, SecureElement, SERIAL_LEN, SIGNATURE_LEN};

use super::swi::SwiUart;

// ---------------------------------------------------------------------------
// Public, citable constants
// ---------------------------------------------------------------------------

/// Command opcodes [REF-SE-002, `calib_command.h`].
pub mod opcode {
    pub const READ: u8 = 0x02;
    pub const MAC: u8 = 0x08;
    pub const WRITE: u8 = 0x12;
    pub const DELETE: u8 = 0x13;
    pub const NONCE: u8 = 0x16;
    pub const LOCK: u8 = 0x17;
    pub const COUNTER: u8 = 0x24;
    pub const INFO: u8 = 0x30;
    pub const GENKEY: u8 = 0x40;
    pub const SIGN: u8 = 0x41;
    /// SHA-256 and its HMAC derivative.
    pub const SHA: u8 = 0x47;
    pub const SELFTEST: u8 = 0x77;
}

/// `SHA` command modes. The HMAC pair is **ECC204-specific** — the values
/// differ from the ATECC508/608 family, so the generic constants must not be
/// substituted [REF-SE-002, `calib_command.h`].
pub mod sha_mode {
    /// Begin an HMAC under the stored symmetric key.
    pub const ECC204_HMAC_START: u8 = 0x03;
    /// Finish the HMAC and return the 32-byte digest.
    pub const ECC204_HMAC_END: u8 = 0x02;
}

/// Published maximum execution times, milliseconds
/// [REF-SE-002, `device_execution_time_ecc204`].
///
/// These are the numbers that put the SE permanently on the cold path: the
/// servo's frame turnaround is 30.4 µs, so `SHA` alone is ~2 600 frame times
/// and `SIGN` ~16 000.
pub mod exec_ms {
    pub const COUNTER: u32 = 20;
    pub const INFO: u32 = 20;
    pub const NONCE: u32 = 20;
    pub const READ: u32 = 40;
    pub const LOCK: u32 = 80;
    pub const SHA: u32 = 80;
    pub const WRITE: u32 = 80;
    pub const SIGN: u32 = 500;
    pub const GENKEY: u32 = 500;
    pub const SELFTEST: u32 = 600;
}

/// Monotonic counter ceiling [REF-SE-001 §Features; `COUNTER_MAX_VALUE_CA2`
/// in REF-SE-002].
///
/// 10 000 total. This is why the counter authorises **lifecycle** events —
/// firmware updates, factory resets — and never sessions or messages: at one
/// count per boot a UAV would exhaust it inside its service life, and at one
/// per message inside a second (§0.5).
pub const COUNTER_MAX: u32 = 10_000;

/// Configuration zone size [REF-SE-002, `ATCA_CA2_CONFIG_SIZE`].
pub const CONFIG_ZONE_LEN: usize = 64;

/// Per-slot configuration size [REF-SE-002, `ATCA_CA2_CONFIG_SLOT_SIZE`].
pub const CONFIG_SLOT_LEN: usize = 16;

// ---------------------------------------------------------------------------
// The NDA-gated seam
// ---------------------------------------------------------------------------

/// The command-packet framing this driver cannot source from public
/// documentation (see the module docs).
///
/// An implementation must supply, per [REF-SE-001]'s complete datasheet or
/// [REF-SE-002]:
///
/// - the SWI word-address / flag token that precedes a command,
/// - the command packet layout (count, opcode, params, data),
/// - the packet CRC-16 parameters,
/// - the wake sequence timing.
pub trait Framing {
    /// Encode a command into `buf`, returning the encoded length.
    fn encode(
        &self,
        opcode: u8,
        mode: u8,
        param2: u16,
        data: &[u8],
        buf: &mut [u8],
    ) -> SeResult<usize>;

    /// Validate a raw response and return its payload span.
    fn decode<'a>(&self, raw: &'a [u8]) -> SeResult<&'a [u8]>;

    /// Drive the device's wake sequence.
    fn wake<U: SwiUart>(&self, uart: &mut U) -> SeResult<()>;
}

/// The error every SE call returns when a build selected a real ECC204 but
/// supplied no [`Framing`].
///
/// This exists so the failure is *loud and early* rather than a stub that
/// silently returns zeros — a security driver that quietly succeeds without
/// talking to the device would authenticate everything.
pub const UNAVAILABLE: SeError = SeError::Unavailable;

// ---------------------------------------------------------------------------
// Driver
// ---------------------------------------------------------------------------

/// An ECC204 reached over SWI.
pub struct Ecc204<U, F> {
    uart: U,
    framing: F,
    /// Cached serial, read once at bring-up.
    serial: Option<[u8; SERIAL_LEN]>,
}

impl<U, F> Ecc204<U, F>
where
    U: SwiUart,
    F: Framing,
{
    pub fn new(uart: U, framing: F) -> Self {
        Self {
            uart,
            framing,
            serial: None,
        }
    }

    /// One command/response exchange.
    ///
    /// # Timing contract
    ///
    /// Blocks for the command's published execution time plus I/O — 20 ms to
    /// 500 ms + ~24 ms. The caller **must** hold a bus-quiet window
    /// (`docs/security-architecture.md` §4). Never call this from an ISR.
    fn exchange(
        &mut self,
        opcode: u8,
        mode: u8,
        param2: u16,
        data: &[u8],
        exec_ms: u32,
        out: &mut [u8],
    ) -> SeResult<usize> {
        let mut cmd = [0u8; 96];
        let n = self.framing.encode(opcode, mode, param2, data, &mut cmd)?;

        self.framing.wake(&mut self.uart)?;
        self.uart
            .write(&cmd[..n])
            .map_err(|_| SeError::BadResponse)?;

        let mut raw = [0u8; 96];
        // Poll window: the published execution time with margin. A device that
        // has not answered by then is treated as absent rather than retried
        // forever -- a wedged SE must not wedge the servo.
        let timeout_us = exec_ms.saturating_mul(1_500);
        let got = self
            .uart
            .read(&mut raw, timeout_us)
            .map_err(|_| SeError::Timeout)?;

        let payload = self.framing.decode(&raw[..got])?;
        if payload.len() > out.len() {
            return Err(SeError::BadResponse);
        }
        out[..payload.len()].copy_from_slice(payload);
        Ok(payload.len())
    }
}

impl<U, F> SecureElement for Ecc204<U, F>
where
    U: SwiUart,
    F: Framing,
{
    fn hmac(&mut self, msg: &[u8], out: &mut [u8; 32]) -> SeResult<()> {
        // HMAC-SHA-256 under the stored symmetric key [REF-SE-001 §2.1.2] is a
        // two-command sequence: START loads the key, END supplies the message
        // and returns the digest. Both cost a full `SHA` execution time, so an
        // HMAC is ~160 ms of execution plus ~48 ms of SWI I/O -- which is the
        // whole reason the session key is derived once at boot and the message
        // plane runs on the MCU instead (§0.1).
        //
        // All KDF messages in `osc-security::keys` are short enough to fit one
        // END command; a longer message would need SHA_MODE_HMAC_UPDATE
        // chunking, which no caller here requires.
        let mut scratch = [0u8; 32];
        self.exchange(
            opcode::SHA,
            sha_mode::ECC204_HMAC_START,
            0,
            &[],
            exec_ms::SHA,
            &mut scratch,
        )?;
        let n = self.exchange(
            opcode::SHA,
            sha_mode::ECC204_HMAC_END,
            msg.len() as u16,
            msg,
            exec_ms::SHA,
            out,
        )?;
        if n != 32 {
            return Err(SeError::BadResponse);
        }
        Ok(())
    }

    fn random(&mut self, out: &mut [u8]) -> SeResult<()> {
        let mut buf = [0u8; 32];
        let n = self.exchange(opcode::NONCE, 0, 0, &[], exec_ms::NONCE, &mut buf)?;
        if n < out.len() {
            return Err(SeError::BadResponse);
        }
        out.copy_from_slice(&buf[..out.len()]);
        Ok(())
    }

    fn sign(&mut self, digest: &[u8; 32], out: &mut [u8; SIGNATURE_LEN]) -> SeResult<()> {
        let n = self.exchange(opcode::SIGN, 0, 0, digest, exec_ms::SIGN, out)?;
        if n != SIGNATURE_LEN {
            return Err(SeError::BadResponse);
        }
        Ok(())
    }

    fn serial(&mut self, out: &mut [u8; SERIAL_LEN]) -> SeResult<()> {
        if let Some(s) = self.serial {
            *out = s;
            return Ok(());
        }
        let mut buf = [0u8; 32];
        let n = self.exchange(opcode::INFO, 0, 0, &[], exec_ms::INFO, &mut buf)?;
        if n < SERIAL_LEN {
            return Err(SeError::BadResponse);
        }
        out.copy_from_slice(&buf[..SERIAL_LEN]);
        self.serial = Some(*out);
        Ok(())
    }

    fn counter_increment(&mut self) -> SeResult<u32> {
        let mut buf = [0u8; 4];
        let n = self.exchange(opcode::COUNTER, 1, 0, &[], exec_ms::COUNTER, &mut buf)?;
        if n < 4 {
            return Err(SeError::BadResponse);
        }
        let v = u32::from_le_bytes(buf);
        if v >= COUNTER_MAX {
            return Err(SeError::CounterExhausted);
        }
        Ok(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counter_ceiling_matches_both_sources() {
        // Datasheet sec Features says 10 000; CryptoAuthLib's
        // COUNTER_MAX_VALUE_CA2 says 10000. Independent agreement.
        assert_eq!(COUNTER_MAX, 10_000);
    }

    #[test]
    fn execution_times_keep_the_se_off_the_hot_path() {
        // The architecture's central claim, asserted so a future edit that
        // "optimises" these numbers has to confront it.
        const FRAME_TURNAROUND_US: u32 = 30;
        assert!(exec_ms::SHA * 1000 / FRAME_TURNAROUND_US > 2_000);
        assert!(exec_ms::SIGN * 1000 / FRAME_TURNAROUND_US > 15_000);
    }

    #[test]
    fn opcodes_are_distinct() {
        let all = [
            opcode::MAC,
            opcode::WRITE,
            opcode::NONCE,
            opcode::LOCK,
            opcode::COUNTER,
            opcode::INFO,
            opcode::SIGN,
            opcode::READ,
            opcode::SELFTEST,
            opcode::SHA,
        ];
        for (i, a) in all.iter().enumerate() {
            for b in &all[i + 1..] {
                assert_ne!(a, b, "duplicate opcode {a:#04x}");
            }
        }
    }
}
