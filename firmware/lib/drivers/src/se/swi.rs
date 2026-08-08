//! ECC204 single-wire interface, carried over a hardware UART
//! (`docs/security-architecture.md` §0.3.1, §0.4).
//!
//! # Why a UART and not a bit-bang
//!
//! Microchip's SWI is a PWM pulse protocol, but the reference host
//! implementation carries it over an ordinary UART at 230 400 baud by sending
//! **one UART character per data bit** — `0x7F` for a `1`, `0x7D` for a `0`,
//! LSB first [REF-SE-003]. The character's own start bit and data pattern
//! generate the pulse shape the device expects, so a plain UART is a complete
//! SWI transmitter and receiver.
//!
//! That matters here for a reason beyond convenience. One SWI bit period is
//! **4.34 µs**, while the transport's USART1 and SysTick vectors run at PFIC
//! HIGH with dispatch bodies of 10–70 µs (`osc-servo-transport.md` §2). A
//! `nop`-loop bit-bang is stretched by an order of magnitude by any frame that
//! arrives mid-transaction — and the clock-discipline loop is slewing HSITRIM
//! underneath it the whole time (§9.3), so the loop's calibration is moving
//! too. Handing the bit timing to a UART peripheral removes both problems.
//!
//! The quiet-window rule in §0.4 still applies regardless: SE commands block
//! for 20–500 ms [REF-SE-002] and must never be issued from an ISR.
//!
//! # Attribution
//!
//! The SWI-over-UART encoding (baud, bit characters, bit order) is taken from
//! Microchip's open-source CryptoAuthLib HAL [REF-SE-003]. This is an
//! independent Rust implementation of that encoding; no CryptoAuthLib code is
//! copied.

/// SWI operating baud [REF-SE-003].
pub const SWI_BAUD: u32 = 230_400;

/// UART character encoding a logic `1` [REF-SE-003].
pub const BIT_ONE: u8 = 0x7F;

/// UART character encoding a logic `0` [REF-SE-003].
pub const BIT_ZERO: u8 = 0x7D;

/// UART characters per SWI data bit.
pub const CHARS_PER_BIT: usize = 1;

/// UART characters per SWI data byte.
pub const CHARS_PER_BYTE: usize = 8;

/// Effective payload throughput, bits per second.
///
/// Note the factor of **10**, not 8: each SWI data bit costs a whole UART
/// character, and a character is 10 bit-times on the wire (start + 8 data +
/// stop). So one data byte costs 8 × 10 = 80 bit-times ≈ **347 µs**, and the
/// effective rate is 23.0 kbit/s — not the 28.8 kbit/s that dropping the
/// framing bits would suggest.
pub const EFFECTIVE_BPS: u32 = SWI_BAUD / 10;

/// A byte-oriented UART the SWI layer drives.
///
/// Deliberately blocking: every caller is on a cold path inside a bus-quiet
/// window, so there is nothing to interleave with and an async surface would
/// only add ways to violate §0.4.
pub trait SwiUart {
    type Error;

    /// Reconfigure to `baud`. SWI wake uses a lower rate than the data phase
    /// [REF-SE-003], so the driver retunes mid-transaction.
    fn set_baud(&mut self, baud: u32) -> Result<(), Self::Error>;

    /// Transmit `bytes`, blocking until the shifter has drained.
    fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error>;

    /// Receive into `out`, blocking. Returns the count received; a short read
    /// means the device stopped talking.
    fn read(&mut self, out: &mut [u8], timeout_us: u32) -> Result<usize, Self::Error>;
}

/// Expand one data byte into its 8 UART bit-characters, LSB first.
///
/// This is the whole of the SWI transmit encoding.
pub fn encode_byte(b: u8, out: &mut [u8; CHARS_PER_BYTE]) {
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = if (b >> i) & 1 != 0 { BIT_ONE } else { BIT_ZERO };
    }
}

/// Recover one data byte from 8 received bit-characters, LSB first.
///
/// A received bit reads as `1` when the character is within one of
/// [`BIT_ONE`] — the reference implementation's tolerance, which absorbs the
/// edge placement jitter of a device clocked independently of the host
/// [REF-SE-003]. Anything else is a `0`.
pub fn decode_byte(chars: &[u8; CHARS_PER_BYTE]) -> u8 {
    let mut b = 0u8;
    for (i, &c) in chars.iter().enumerate() {
        // Parenthesised deliberately: `^` binds LOOSER than `<` in Rust, so
        // `c ^ BIT_ONE < 2` would parse as `c ^ (BIT_ONE < 2)` and not compile.
        if (c ^ BIT_ONE) < 2 {
            b |= 1 << i;
        }
    }
    b
}

/// Wire-time cost of moving `n` data bytes in one direction, in microseconds.
///
/// Exposed because the architecture's budget arithmetic depends on it
/// (§0.1): the SE's I/O is slow enough that it, not just execution time,
/// has to be accounted for when sizing a quiet window.
pub const fn transfer_time_us(n: usize) -> u32 {
    // 10 bit-times per UART character, 8 characters per data byte.
    (n as u32) * CHARS_PER_BYTE as u32 * 10 * 1_000_000 / SWI_BAUD
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bit_characters_round_trip() {
        for b in 0u8..=255 {
            let mut chars = [0u8; CHARS_PER_BYTE];
            encode_byte(b, &mut chars);
            assert_eq!(decode_byte(&chars), b, "byte {b:#04x}");
        }
    }

    #[test]
    fn encoding_is_lsb_first() {
        let mut chars = [0u8; CHARS_PER_BYTE];
        encode_byte(0x01, &mut chars);
        assert_eq!(chars[0], BIT_ONE, "bit 0 goes out first");
        assert_eq!(chars[1], BIT_ZERO);
        encode_byte(0x80, &mut chars);
        assert_eq!(chars[0], BIT_ZERO);
        assert_eq!(chars[7], BIT_ONE, "bit 7 goes out last");
    }

    #[test]
    fn decode_tolerates_the_reference_jitter_band() {
        // 0x7F and 0x7E both read as 1; 0x7D reads as 0.
        assert_eq!(decode_byte(&[0x7F, 0x7D, 0x7D, 0x7D, 0x7D, 0x7D, 0x7D, 0x7D]), 0x01);
        assert_eq!(decode_byte(&[0x7E, 0x7D, 0x7D, 0x7D, 0x7D, 0x7D, 0x7D, 0x7D]), 0x01);
        assert_eq!(decode_byte(&[0x7D; 8]), 0x00);
    }

    #[test]
    fn throughput_matches_the_architecture_budget() {
        assert_eq!(EFFECTIVE_BPS, 23_040);
        // ~347 µs per data byte -- the figure security-architecture.md sec 0.1
        // uses to show the SE cannot sit in a frame. A servo frame turnaround
        // is 30.4 µs, so ONE SE data byte already costs ~11 frames.
        assert_eq!(transfer_time_us(1), 347);
        // A ~35-byte command is ~12 ms of wire, so a command/response pair is
        // ~24 ms on top of the command's execution time.
        assert!((11_000..13_000).contains(&transfer_time_us(35)));
    }
}
