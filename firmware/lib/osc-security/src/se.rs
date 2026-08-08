//! The secure-element abstraction (`docs/security-architecture.md` §5, §7.3).
//!
//! Only the four operations the architecture actually needs are exposed, and
//! every one of them is a **cold path**: the ECC204's published execution
//! times are 20 ms (`NONCE`, `COUNTER`), 80 ms (`SHA`/HMAC) and 500 ms
//! (`SIGN`) [REF-SE-002], against a 30 µs frame turnaround. Nothing here may
//! ever be called from an ISR or from the control kernel.
//!
//! The trait is deliberately transport-agnostic: the servo board drives an
//! ECC204 over hardware-UART SWI, the dev board over I²C, and the host test
//! suite over a software fake. All three link the same session logic.

/// Reasons an SE operation can fail.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SeError {
    /// No device answered the wake sequence.
    NotPresent,
    /// Device answered but the response framing or CRC was bad.
    BadResponse,
    /// Device returned an error status for the command.
    Rejected,
    /// The operation did not complete inside its published execution time.
    Timeout,
    /// The device is present but not provisioned (no key, zones unlocked).
    Unprovisioned,
    /// The monotonic counter has reached its 10 000 ceiling
    /// [REF-SE-001 §Features].
    CounterExhausted,
    /// The build has no SE command layer linked — see `ecc204::UNAVAILABLE`.
    Unavailable,
}

pub type SeResult<T> = Result<T, SeError>;

/// Width of the ECC204's unique factory serial number: 72 bits
/// [REF-SE-001 §Features].
pub const SERIAL_LEN: usize = 9;

/// ECDSA P-256 signature: `r ‖ s`, 32 bytes each.
pub const SIGNATURE_LEN: usize = 64;

/// A CryptoAuthentication-class secure element.
///
/// # Concurrency contract
///
/// Implementations may block for **hundreds of milliseconds**. Callers must
/// hold a bus-quiet window (§4) for the whole call: at boot before the
/// transport starts, or under the host-negotiated quiet-window handshake.
pub trait SecureElement {
    /// HMAC-SHA-256 over `msg` under the device's symmetric slot
    /// [REF-SE-001 §2.1.2]. ~80 ms.
    fn hmac(&mut self, msg: &[u8], out: &mut [u8; 32]) -> SeResult<()>;

    /// `n` bytes from the certified TRNG [REF-SE-001 §2.2.2]. ~20 ms.
    fn random(&mut self, out: &mut [u8]) -> SeResult<()>;

    /// ECDSA P-256 signature over a 32-byte digest [REF-SE-001 §2.1.3].
    /// ~500 ms. **Sign only — the ECC204 cannot verify** (§5.2).
    fn sign(&mut self, digest: &[u8; 32], out: &mut [u8; SIGNATURE_LEN]) -> SeResult<()>;

    /// The factory-unique 72-bit serial number.
    fn serial(&mut self, out: &mut [u8; SERIAL_LEN]) -> SeResult<()>;

    /// Increment the monotonic counter and return the new value. ~20 ms.
    ///
    /// Reserved for **lifecycle grants** — firmware update, factory reset —
    /// never for sessions or messages: the ceiling is 10 000 counts total
    /// (§0.5), which is a few dozen legitimate uses per service life with
    /// three orders of margin, and nothing more.
    fn counter_increment(&mut self) -> SeResult<u32>;
}

/// A servo with no secure element fitted, or one that failed to answer.
///
/// Returning a working-but-refusing implementation rather than making the SE
/// optional at every call site is what lets the servo boot into
/// [`crate::session::SecurityState::Unsecured`] and keep flying (§4.2): a
/// servo that bricks itself because a crypto chip did not answer is a worse
/// failure mode for an aircraft than one that holds position and raises an
/// alert.
pub struct AbsentSe;

impl SecureElement for AbsentSe {
    fn hmac(&mut self, _msg: &[u8], _out: &mut [u8; 32]) -> SeResult<()> {
        Err(SeError::NotPresent)
    }
    fn random(&mut self, _out: &mut [u8]) -> SeResult<()> {
        Err(SeError::NotPresent)
    }
    fn sign(&mut self, _digest: &[u8; 32], _out: &mut [u8; SIGNATURE_LEN]) -> SeResult<()> {
        Err(SeError::NotPresent)
    }
    fn serial(&mut self, _out: &mut [u8; SERIAL_LEN]) -> SeResult<()> {
        Err(SeError::NotPresent)
    }
    fn counter_increment(&mut self) -> SeResult<u32> {
        Err(SeError::NotPresent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_se_refuses_everything_without_panicking() {
        let mut se = AbsentSe;
        assert_eq!(se.hmac(b"x", &mut [0; 32]), Err(SeError::NotPresent));
        assert_eq!(se.random(&mut [0; 4]), Err(SeError::NotPresent));
        assert_eq!(se.sign(&[0; 32], &mut [0; SIGNATURE_LEN]), Err(SeError::NotPresent));
        assert_eq!(se.serial(&mut [0; SERIAL_LEN]), Err(SeError::NotPresent));
        assert_eq!(se.counter_increment(), Err(SeError::NotPresent));
    }
}
