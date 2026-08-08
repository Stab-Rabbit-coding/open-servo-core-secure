//! HalfSipHash-2-4 — the message-plane authenticator
//! (`docs/security-architecture.md` §2.4).
//!
//! # Why this primitive
//!
//! The servo must authenticate a frame inside the transport's covered-span
//! dispatch window (20 µs at 1 M) on a 48 MHz RV32EC core with 16 registers,
//! no rotate instruction, no divide, and `zmmul` multiply only. HMAC-SHA-256
//! — the primitive the ECC204 itself implements — costs ~225 µs per frame
//! there, an ~11× overrun, so it stays on the cold path (session-key
//! derivation, §3) where its cost is affordable.
//!
//! HalfSipHash is the 32-bit-word member of the SipHash family: a keyed
//! pseudorandom function built only from add / rotate / XOR, with no lookup
//! tables (so no flash tables and no cache-timing surface) and a state of four
//! 32-bit words that fits the register file.
//!
//! # Attribution
//!
//! Algorithm: SipHash, Jean-Philippe Aumasson and Daniel J. Bernstein,
//! *"SipHash: a fast short-input PRF"*, INDOCRYPT 2012 [REF-CRYPTO-001].
//! `HalfSipHash` is the reduced-width variant published in the authors'
//! reference implementation [REF-CRYPTO-002]. This is an independent Rust
//! implementation written against the published round function, constants and
//! finalisation; the test vectors in this module are the reference
//! implementation's own `vectors_hsip32` (see [`tests`]).
//!
//! # Security note
//!
//! HalfSipHash is **not** a NIST-approved MAC. Its use here is a deliberate,
//! documented exception justified in `docs/security-architecture.md` §2.4 and
//! §6, and bounded by: ephemeral keys derived by a FIPS-approved KDF inside a
//! certified device, epoch+sequence replay protection, and a consecutive-
//! failure lockout that makes online forgery detectable long before the tag's
//! 2^32 space is meaningfully sampled.

/// Compression rounds per absorbed word (the "2" of HalfSipHash-**2**-4).
const C_ROUNDS: usize = 2;
/// Finalisation rounds (the "4" of HalfSipHash-2-**4**).
const D_ROUNDS: usize = 4;

// Initial `v2` / `v3`. Like SipHash's, these are slices of the ASCII string
// `"somepseudorandomlygeneratedbytes"` — here bytes 16..20 (`"lyge"`) and
// 24..28 (`"tedb"`), read big-endian. Nothing-up-my-sleeve values; `v0`/`v1`
// start at zero and take their entropy from the key alone.
const INIT_V2: u32 = 0x6c79_6765;
const INIT_V3: u32 = 0x7465_6462;

/// Finalisation domain separator for a 32-bit tag. (The reference uses `0xee`
/// for the 64-bit output width; this implementation only ever emits 32 bits.)
const FINAL_XOR: u32 = 0xff;

/// Session MAC key: 64 bits, derived by the ECC204's HMAC-SHA-256 and
/// truncated (`keys.rs`). Ephemeral — RAM only, never persisted.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct MacKey {
    k0: u32,
    k1: u32,
}

impl MacKey {
    /// Build from 8 key bytes, little-endian per the reference implementation.
    #[inline]
    pub const fn from_bytes(k: &[u8; 8]) -> Self {
        Self {
            k0: u32::from_le_bytes([k[0], k[1], k[2], k[3]]),
            k1: u32::from_le_bytes([k[4], k[5], k[6], k[7]]),
        }
    }

    /// The all-zero key. Only meaningful as a placeholder before a session is
    /// established; [`crate::session`] refuses to verify against it.
    pub const ZERO: Self = Self { k0: 0, k1: 0 };

    #[inline]
    pub const fn is_zero(self) -> bool {
        self.k0 == 0 && self.k1 == 0
    }
}

/// A 32-bit authentication tag. Compared only via [`Tag::ct_eq`].
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct Tag(pub u32);

impl Tag {
    /// Wire encoding: little-endian, matching the reference implementation's
    /// `U32TO8_LE` output order.
    #[inline]
    pub const fn to_bytes(self) -> [u8; 4] {
        self.0.to_le_bytes()
    }

    #[inline]
    pub const fn from_bytes(b: &[u8; 4]) -> Self {
        Self(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// Constant-time equality.
    ///
    /// A 32-bit tag on a 48 MHz core makes a remote timing attack fanciful,
    /// but a data-dependent early-out is free to avoid and expensive to
    /// retrofit once something upstream starts branching on the result.
    #[inline]
    pub fn ct_eq(self, other: Tag) -> bool {
        // Fold the XOR difference to a single 0/1 without branching: any
        // differing bit propagates into bit 31 of `(d | -d)`.
        let d = self.0 ^ other.0;
        let collapsed = (d | d.wrapping_neg()) >> 31;
        collapsed == 0
    }
}

/// Streaming HalfSipHash-2-4.
///
/// Streaming rather than one-shot because the transport folds a whole
/// *instruction stream* across several frames into one digest before the
/// broadcast COMMIT closes it (`docs/security-architecture.md` §2.1), and
/// because a frame's bytes can arrive as two ring segments when the RX ring
/// wraps. Absorbing in chunks is byte-exact with absorbing the concatenation —
/// pinned by [`tests::streaming_matches_one_shot`].
#[derive(Clone)]
pub struct HalfSipHasher {
    v0: u32,
    v1: u32,
    v2: u32,
    v3: u32,
    /// Bytes of the in-progress 4-byte word.
    tail: [u8; 4],
    tail_len: u8,
    /// Total bytes absorbed. Only the low 8 bits reach the finalisation
    /// (`b = len << 24`), so wrapping is the reference behaviour, not a bug.
    total: u32,
}

impl HalfSipHasher {
    #[inline]
    pub const fn new(key: MacKey) -> Self {
        Self {
            v0: key.k0,
            v1: key.k1,
            v2: INIT_V2 ^ key.k0,
            v3: INIT_V3 ^ key.k1,
            tail: [0; 4],
            tail_len: 0,
            total: 0,
        }
    }

    /// One SipRound. `rotate_left` lowers to `slli`/`srli`/`or` on RV32
    /// without `Zbb`; there is no table and no branch.
    #[inline(always)]
    fn round(&mut self) {
        self.v0 = self.v0.wrapping_add(self.v1);
        self.v1 = self.v1.rotate_left(5);
        self.v1 ^= self.v0;
        self.v0 = self.v0.rotate_left(16);

        self.v2 = self.v2.wrapping_add(self.v3);
        self.v3 = self.v3.rotate_left(8);
        self.v3 ^= self.v2;

        self.v0 = self.v0.wrapping_add(self.v3);
        self.v3 = self.v3.rotate_left(7);
        self.v3 ^= self.v0;

        self.v2 = self.v2.wrapping_add(self.v1);
        self.v1 = self.v1.rotate_left(13);
        self.v1 ^= self.v2;
        self.v2 = self.v2.rotate_left(16);
    }

    #[inline(always)]
    fn absorb_word(&mut self, m: u32) {
        self.v3 ^= m;
        for _ in 0..C_ROUNDS {
            self.round();
        }
        self.v0 ^= m;
    }

    /// Absorb `data`. May be called any number of times with any chunking.
    pub fn update(&mut self, data: &[u8]) {
        self.total = self.total.wrapping_add(data.len() as u32);
        let mut rest = data;

        // Top up a partial word first.
        if self.tail_len != 0 {
            let want = 4 - self.tail_len as usize;
            let take = want.min(rest.len());
            self.tail[self.tail_len as usize..self.tail_len as usize + take]
                .copy_from_slice(&rest[..take]);
            self.tail_len += take as u8;
            rest = &rest[take..];
            if self.tail_len < 4 {
                return;
            }
            let m = u32::from_le_bytes(self.tail);
            self.absorb_word(m);
            self.tail_len = 0;
        }

        // Whole words. `chunks_exact` keeps the remainder for the tail below.
        let mut it = rest.chunks_exact(4);
        for w in &mut it {
            let m = u32::from_le_bytes([w[0], w[1], w[2], w[3]]);
            self.absorb_word(m);
        }

        let rem = it.remainder();
        if !rem.is_empty() {
            self.tail[..rem.len()].copy_from_slice(rem);
            self.tail_len = rem.len() as u8;
        }
    }

    /// Finalise and emit the 32-bit tag. Consumes the state — a hasher is
    /// never reused across messages (that would silently chain two digests).
    pub fn finish(mut self) -> Tag {
        // The final block is the 0..3 leftover bytes in the low positions with
        // the message length in the top byte.
        let mut b: u32 = (self.total & 0xff) << 24;
        for (i, &byte) in self.tail[..self.tail_len as usize].iter().enumerate() {
            b |= (byte as u32) << (8 * i);
        }

        self.v3 ^= b;
        for _ in 0..C_ROUNDS {
            self.round();
        }
        self.v0 ^= b;

        self.v2 ^= FINAL_XOR;
        for _ in 0..D_ROUNDS {
            self.round();
        }

        Tag(self.v1 ^ self.v3)
    }

    /// Bytes absorbed so far — the bench probe's "how much did we fold"
    /// counter (`docs/security-architecture.md` §7.4).
    #[inline]
    pub const fn absorbed(&self) -> u32 {
        self.total
    }
}

/// One-shot HalfSipHash-2-4 over `msg`.
#[inline]
pub fn mac(key: MacKey, msg: &[u8]) -> Tag {
    let mut h = HalfSipHasher::new(key);
    h.update(msg);
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The reference implementation's `vectors_hsip32` [REF-CRYPTO-002].
    ///
    /// Convention taken from the reference test driver: the key is
    /// `k[i] = i` (HalfSipHash consumes the first 8 bytes), the message for
    /// vector `i` is `in[j] = j` for `j < i` (so vector 0 is the empty
    /// message), and the 4-byte output is compared little-endian.
    const VECTORS: [[u8; 4]; 8] = [
        [0xa9, 0x35, 0x9f, 0x5b],
        [0x27, 0x47, 0x5a, 0xb8],
        [0xfa, 0x62, 0xa6, 0x03],
        [0x8a, 0xfe, 0xe7, 0x04],
        [0x2a, 0x6e, 0x46, 0x89],
        [0xc5, 0xfa, 0xb6, 0x69],
        [0x58, 0x63, 0xfc, 0x23],
        [0x8b, 0xcf, 0x63, 0xc5],
    ];

    fn ref_key() -> MacKey {
        let mut k = [0u8; 8];
        for (i, b) in k.iter_mut().enumerate() {
            *b = i as u8;
        }
        MacKey::from_bytes(&k)
    }

    #[test]
    fn reference_vectors() {
        let key = ref_key();
        let msg: [u8; 8] = [0, 1, 2, 3, 4, 5, 6, 7];
        for (len, want) in VECTORS.iter().enumerate() {
            let got = mac(key, &msg[..len]).to_bytes();
            assert_eq!(&got, want, "halfsiphash-2-4 vector len={len}");
        }
    }

    #[test]
    fn streaming_matches_one_shot() {
        let key = ref_key();
        let msg: [u8; 37] = core::array::from_fn(|i| (i as u8).wrapping_mul(7).wrapping_add(3));
        let whole = mac(key, &msg);
        // Every single split, and a few multi-chunk shapes: the transport
        // feeds ring segments and multi-frame streams both ways.
        for split in 0..=msg.len() {
            let mut h = HalfSipHasher::new(key);
            h.update(&msg[..split]);
            h.update(&msg[split..]);
            assert_eq!(h.finish(), whole, "split at {split}");
        }
        let mut h = HalfSipHasher::new(key);
        for b in msg.iter() {
            h.update(core::slice::from_ref(b));
        }
        assert_eq!(h.finish(), whole, "byte-at-a-time");
    }

    #[test]
    fn key_changes_the_tag() {
        let msg = b"goal_position";
        let a = mac(MacKey::from_bytes(&[1, 2, 3, 4, 5, 6, 7, 8]), msg);
        let b = mac(MacKey::from_bytes(&[1, 2, 3, 4, 5, 6, 7, 9]), msg);
        assert_ne!(a, b);
    }

    #[test]
    fn single_bit_flip_changes_the_tag() {
        let key = ref_key();
        let mut msg = [0x11u8, 0x22, 0x33, 0x44, 0x55];
        let before = mac(key, &msg);
        msg[3] ^= 0x01;
        assert_ne!(mac(key, &msg), before);
    }

    #[test]
    fn length_is_bound_not_just_content() {
        // A zero-extended message must not collide with the short one:
        // the length enters finalisation.
        let key = ref_key();
        assert_ne!(mac(key, &[0xAA]), mac(key, &[0xAA, 0x00]));
    }

    #[test]
    fn tag_wire_roundtrip() {
        let t = Tag(0xDEAD_BEEF);
        assert_eq!(Tag::from_bytes(&t.to_bytes()), t);
        assert_eq!(t.to_bytes(), [0xEF, 0xBE, 0xAD, 0xDE]);
    }

    #[test]
    fn ct_eq_agrees_with_eq() {
        assert!(Tag(0).ct_eq(Tag(0)));
        assert!(Tag(u32::MAX).ct_eq(Tag(u32::MAX)));
        assert!(!Tag(0).ct_eq(Tag(1)));
        assert!(!Tag(1).ct_eq(Tag(0)));
        // Every single-bit difference must be caught.
        for bit in 0..32 {
            assert!(!Tag(0).ct_eq(Tag(1 << bit)), "bit {bit}");
        }
    }
}
