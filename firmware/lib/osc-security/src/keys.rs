//! Session keys and the key-derivation labels
//! (`docs/security-architecture.md` §3).
//!
//! Root secret `K_dev` lives in the ECC204's single symmetric slot and never
//! leaves the device [REF-SE-001 §1.2]. Everything here is *derived*,
//! ephemeral, and RAM-only: a power cycle destroys it and a new session
//! derives fresh material under a fresh epoch.
//!
//! Derivation runs as HMAC-SHA-256 **inside** the ECC204, so the KDF itself is
//! FIPS-approved (FIPS 198-1 [REF-STD-003]) even though the per-frame tag it
//! keys is not (§2.4, §6).

use crate::mac::MacKey;

/// Domain-separation labels. Distinct labels keep the three derived values
/// independent: recovering one must not expose another, and a transcript from
/// one role must never verify in another.
pub mod label {
    /// Per-servo unicast session key.
    pub const UNICAST: &[u8; 5] = b"OSC1U";
    /// Group-key wrapping pad.
    pub const WRAP: &[u8; 5] = b"OSC1W";
    /// Group-key delivery tag.
    pub const WRAP_TAG: &[u8; 5] = b"OSC1T";
}

/// Nonce width for session establishment — 16 bytes from the ECC204's
/// SP 800-90A/B/C certified TRNG [REF-SE-001 §2.2.2].
pub const NONCE_LEN: usize = 16;

/// Group key wrap width: the wrapped `K_grp` as it appears on the wire.
pub const WRAP_LEN: usize = 16;

/// The HMAC-SHA-256 output width the ECC204 returns.
pub const HMAC_LEN: usize = 32;

/// Session epoch. Incremented on every establishment; the derived keys are
/// bound to it, so a cross-epoch replay fails on the *key*, not merely on a
/// counter comparison (§2.5).
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
#[repr(transparent)]
pub struct Epoch(pub u16);

impl Epoch {
    #[inline]
    pub const fn next(self) -> Self {
        Self(self.0.wrapping_add(1))
    }

    #[inline]
    pub const fn to_le_bytes(self) -> [u8; 2] {
        self.0.to_le_bytes()
    }
}

/// The live session's key material.
///
/// `unicast` authenticates frames addressed to this servo specifically;
/// `group` authenticates broadcast frames — above all the COMMIT that closes
/// each hot-loop cycle, which is one frame with one tag read by the whole
/// fleet (§3, "why a group key is unavoidable").
#[derive(Copy, Clone)]
pub struct SessionKeys {
    pub unicast: MacKey,
    pub group: MacKey,
    pub epoch: Epoch,
}

impl SessionKeys {
    /// A context with no session: both keys zero. [`SessionKeys::is_ready`]
    /// is false, and the session state machine refuses to verify against it
    /// rather than validating everything under a known key.
    pub const UNSET: Self = Self {
        unicast: MacKey::ZERO,
        group: MacKey::ZERO,
        epoch: Epoch(0),
    };

    #[inline]
    pub const fn is_ready(&self) -> bool {
        !self.unicast.is_zero() && !self.group.is_zero()
    }

    /// Pick the key a frame is tagged under: broadcast frames ride the group
    /// key, everything else the unicast key.
    #[inline]
    pub const fn for_frame(&self, broadcast: bool) -> MacKey {
        if broadcast { self.group } else { self.unicast }
    }
}

/// Build the message the ECC204 is asked to HMAC for the unicast session key:
/// `label ‖ epoch ‖ host_nonce ‖ servo_nonce`.
///
/// Returned as a fixed buffer rather than hashed here — the derivation itself
/// must happen inside the SE, since `K_dev` is not available to the MCU.
pub fn unicast_kdf_message(
    epoch: Epoch,
    host_nonce: &[u8; NONCE_LEN],
    servo_nonce: &[u8; NONCE_LEN],
) -> [u8; 5 + 2 + NONCE_LEN * 2] {
    let mut m = [0u8; 5 + 2 + NONCE_LEN * 2];
    m[..5].copy_from_slice(label::UNICAST);
    m[5..7].copy_from_slice(&epoch.to_le_bytes());
    m[7..7 + NONCE_LEN].copy_from_slice(host_nonce);
    m[7 + NONCE_LEN..].copy_from_slice(servo_nonce);
    m
}

/// Build the message for the group-key wrapping pad:
/// `label ‖ epoch ‖ host_nonce`.
pub fn wrap_kdf_message(
    epoch: Epoch,
    host_nonce: &[u8; NONCE_LEN],
) -> [u8; 5 + 2 + NONCE_LEN] {
    let mut m = [0u8; 5 + 2 + NONCE_LEN];
    m[..5].copy_from_slice(label::WRAP);
    m[5..7].copy_from_slice(&epoch.to_le_bytes());
    m[7..].copy_from_slice(host_nonce);
    m
}

/// Build the message for the group-key delivery tag:
/// `label ‖ epoch ‖ wrapped`.
pub fn wrap_tag_message(
    epoch: Epoch,
    wrapped: &[u8; WRAP_LEN],
) -> [u8; 5 + 2 + WRAP_LEN] {
    let mut m = [0u8; 5 + 2 + WRAP_LEN];
    m[..5].copy_from_slice(label::WRAP_TAG);
    m[5..7].copy_from_slice(&epoch.to_le_bytes());
    m[7..].copy_from_slice(wrapped);
    m
}

/// Truncate an HMAC-SHA-256 output to a 64-bit [`MacKey`].
///
/// Truncation is the standard KDF-output-reduction step: the 32-byte HMAC is a
/// pseudorandom string, so any fixed 8-byte slice of it is a uniform 64-bit
/// key. The tag width (32 bits) and not the key width is what bounds forgery
/// here, and forgery is additionally rate-limited by lockout (§2.7).
#[inline]
pub fn key_from_hmac(h: &[u8; HMAC_LEN]) -> MacKey {
    let mut k = [0u8; 8];
    k.copy_from_slice(&h[..8]);
    MacKey::from_bytes(&k)
}

/// Unwrap the group key: `K_grp = wrapped XOR pad`, where `pad` is the first
/// 16 bytes of `HMAC(K_dev, label::WRAP ‖ epoch ‖ host_nonce)`.
///
/// The caller **must** have already verified the delivery tag
/// ([`wrap_tag_message`]); this function performs no authentication of its
/// own, and [`crate::session`] is what sequences the two.
#[inline]
pub fn unwrap_group_key(wrapped: &[u8; WRAP_LEN], pad: &[u8; HMAC_LEN]) -> MacKey {
    let mut k = [0u8; 8];
    for (i, slot) in k.iter_mut().enumerate() {
        *slot = wrapped[i] ^ pad[i];
    }
    MacKey::from_bytes(&k)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_are_distinct() {
        assert_ne!(label::UNICAST, label::WRAP);
        assert_ne!(label::WRAP, label::WRAP_TAG);
        assert_ne!(label::UNICAST, label::WRAP_TAG);
    }

    #[test]
    fn kdf_messages_bind_every_input() {
        let n1 = [0xAAu8; NONCE_LEN];
        let n2 = [0xBBu8; NONCE_LEN];
        let base = unicast_kdf_message(Epoch(1), &n1, &n2);
        // Epoch, host nonce and servo nonce each change the message.
        assert_ne!(base, unicast_kdf_message(Epoch(2), &n1, &n2));
        assert_ne!(base, unicast_kdf_message(Epoch(1), &n2, &n2));
        assert_ne!(base, unicast_kdf_message(Epoch(1), &n1, &n1));
        // Nonces are not interchangeable: swapping them must change the key.
        assert_ne!(base, unicast_kdf_message(Epoch(1), &n2, &n1));
    }

    #[test]
    fn label_prefixes_separate_the_domains() {
        let n = [0x5Au8; NONCE_LEN];
        assert_ne!(&unicast_kdf_message(Epoch(7), &n, &n)[..5], &wrap_kdf_message(Epoch(7), &n)[..5]);
    }

    #[test]
    fn unset_session_is_not_ready() {
        assert!(!SessionKeys::UNSET.is_ready());
        let half = SessionKeys {
            unicast: MacKey::from_bytes(&[1; 8]),
            group: MacKey::ZERO,
            epoch: Epoch(1),
        };
        assert!(!half.is_ready(), "a half-derived session must not be usable");
    }

    // `MacKey` has no `Debug` on purpose (key material must not be
    // formattable), so key comparisons use `assert!(a == b)` rather than
    // `assert_eq!`, which would require it.

    #[test]
    fn group_unwrap_is_xor_of_the_pad() {
        let wrapped = [0xF0u8; WRAP_LEN];
        let mut pad = [0u8; HMAC_LEN];
        pad[..8].copy_from_slice(&[0x0F; 8]);
        assert!(unwrap_group_key(&wrapped, &pad) == MacKey::from_bytes(&[0xFF; 8]));
    }

    #[test]
    fn broadcast_selects_the_group_key() {
        let keys = SessionKeys {
            unicast: MacKey::from_bytes(&[1; 8]),
            group: MacKey::from_bytes(&[2; 8]),
            epoch: Epoch(3),
        };
        assert!(keys.for_frame(true) == keys.group);
        assert!(keys.for_frame(false) == keys.unicast);
    }

    #[test]
    fn epoch_wraps_rather_than_panicking() {
        assert_eq!(Epoch(u16::MAX).next(), Epoch(0));
    }
}
