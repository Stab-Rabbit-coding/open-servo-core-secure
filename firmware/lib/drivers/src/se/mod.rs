//! Secure-element drivers (`docs/security-architecture.md` §5, §7.3).
//!
//! Chip-agnostic, like every other driver here (driver-pattern §4): the
//! command layer talks to a [`swi::SwiUart`], and each board supplies the
//! concrete peripheral.
//!
//! **Every entry point in this module is a cold path.** ECC204 commands block
//! for 20–500 ms [REF-SE-002], so nothing here may be called from an ISR or
//! from the control kernel — only inside a bus-quiet window (§4).

pub mod ecc204;
pub mod swi;

pub use ecc204::Ecc204;
pub use swi::{SwiUart, transfer_time_us};
