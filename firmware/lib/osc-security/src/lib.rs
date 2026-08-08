//! # osc-security — the message plane of the osc-native bus
//!
//! Authenticity and integrity for a servo bus, on a 48 MHz RV32EC core with
//! 8 KB of RAM, without disturbing the transport's dispatch-before-verdict
//! spine, its hardware CRC pipeline, or the control cascade's tick budget.
//!
//! The full design, its measured constraints and its threat model live in
//! [`docs/security-architecture.md`]. The short version:
//!
//! | plane | primitive | where | cost |
//! | ----- | --------- | ----- | ---- |
//! | Identity | ECDSA P-256 + cert chain | ECC204 | ~520 ms, boot |
//! | Session | HMAC-SHA-256 KDF | ECC204 | ~100 ms, boot |
//! | **Message** | **HalfSipHash-2-4** | **this crate, on the MCU** | **µs, every effect** |
//!
//! The ECC204 is ~3 300× too slow to sit in a frame (80 ms for an HMAC, 500 ms
//! for a signature, against a 30 µs frame turnaround), so it establishes an
//! ephemeral session key at boot and never touches the hot path again.
//!
//! ## The two rules that make this safe
//!
//! 1. **The CRC stays.** CRC-16/ARC and the session MAC do different jobs —
//!    wire faults versus adversaries — and one cannot substitute for the
//!    other. The MAC is *added* beside the CRC verdict, never in place of it.
//!    (Replacing a CRC that deterministically catches every burst error up to
//!    16 bits with an 8-bit tag that misses 1 in 256 of *all* error patterns
//!    would make a flight actuator less safe, not more.)
//! 2. **Failure holds, it does not go limp.** A tag failure reverts the
//!    staging buffer and alerts; it never cuts torque. An unpowered control
//!    surface is driven by aerodynamic load; a surface held at its last
//!    authenticated command is a known, trimmable disturbance.
//!
//! ## Integration
//!
//! Two calls into [`session::SecurityContext`] from the transport:
//!
//! - [`session::SecurityContext::fold`] at the covered checkpoint, and
//! - [`session::SecurityContext::verify`] beside the CRC verdict,
//!
//! because the transport was already built around "dispatch speculatively,
//! gate effects on a verdict" — which is exactly the shape a per-frame
//! authenticator needs. The gate is architecturally free; it costs only its
//! own compute.
//!
//! ## Portability
//!
//! `no_std`, allocation-free, no hardware dependency. The servo, the host and
//! the integration sim link the same code, so a tag that verifies in a host
//! test verifies on silicon.
//!
//! [`docs/security-architecture.md`]: ../../../docs/security-architecture.md

//! ## A note on `Debug`
//!
//! [`MacKey`], [`SessionKeys`], [`StreamDigest`] and [`HalfSipHasher`]
//! deliberately do **not** implement `Debug`. Key material must not be able to
//! reach a log line, a `defmt` frame or a panic message by accident, and the
//! cheapest way to guarantee that is to make it impossible to format.

#![no_std]
#![forbid(unsafe_code)]

pub mod keys;
pub mod mac;
pub mod policy;
pub mod replay;
pub mod se;
pub mod session;
pub mod stream;
pub mod trailer;

pub use keys::{Epoch, SessionKeys};
pub use mac::{HalfSipHasher, MacKey, Tag, mac as tag_of};
pub use policy::{AuthVerdict, Policy};
pub use replay::{ReplayWindow, SeqReject};
pub use se::{SeError, SeResult, SecureElement};
pub use session::{SecurityContext, SecurityState};
pub use stream::StreamDigest;
pub use trailer::{TRAILER_LEN, Trailer};
