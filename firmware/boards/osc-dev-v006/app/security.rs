#![no_std]

use embedded_hal::i2c::I2c;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

type HmacSha256 = Hmac<Sha256>;

pub const NONCE_LEN: usize = 32;
pub const SIG_LEN: usize = 64;
pub const KEY_LEN: usize = 32;
pub const MAC_TAG_LEN: usize = 8; // Truncated MAC tag

#[derive(Debug)]
pub enum SecurityError {
    I2cError,
    CryptoAuthFailed,
    NotAuthenticated,
}

pub struct OscSecurityContext {
    session_key: [u8; KEY_LEN],
    rx_sequence_num: u32,
    tx_sequence_num: u32,
    is_authenticated: bool,
}

impl OscSecurityContext {
    pub const fn new() -> Self {
        Self {
            session_key: [0u8; KEY_LEN],
            rx_sequence_num: 0,
            tx_sequence_num: 0,
            is_authenticated: false,
        }
    }

    /// Boot Authentication: Executes TRNG, ECDSA signing, and session key derivation
    pub fn boot_authenticate<I2C, E>(
        &mut self,
        i2c: &mut I2C,
        ecc204_addr: u8,
        host_nonce: &[u8; NONCE_LEN],
        servo_nonce_out: &mut [u8; NONCE_LEN],
        ecdsa_sig_out: &mut [u8; SIG_LEN],
    ) -> Result<(), SecurityError>
    where
        I2C: I2c<Error = E>,
    {
        // 1. Fetch 32-byte TRNG from ECC204 over I2C
        self.ecc204_random(i2c, ecc204_addr, servo_nonce_out)?;

        // 2. Hash combined nonces: SHA-256(host_nonce || servo_nonce)
        let mut hasher = Sha256::new();
        hasher.update(host_nonce);
        hasher.update(servo_nonce_out);
        let digest: [u8; 32] = hasher.finalize().into();

        // 3. Request ECDSA P-256 signature from ECC204 over digest (Slot 1: Private Key)
        self.ecc204_sign(i2c, ecc204_addr, 1, &digest, ecdsa_sig_out)?;

        // 4. Derive volatile K_session in ECC204 HMAC engine (Slot 0: Master Secret)
        self.ecc204_hmac(i2c, ecc204_addr, 0, &digest, &mut self.session_key)?;

        self.rx_sequence_num = 0;
        self.tx_sequence_num = 0;
        self.is_authenticated = true;
        Ok(())
    }

    /// Zero-allocation runtime frame signing executed in MCU RAM
    pub fn sign_frame(
        &mut self,
        payload: &[u8],
        tag_out: &mut [u8; MAC_TAG_LEN],
    ) -> Result<(), SecurityError> {
        if !self.is_authenticated {
            return Err(SecurityError::NotAuthenticated);
        }

        let mut mac = HmacSha256::new_from_slice(&self.session_key)
            .map_err(|_| SecurityError::CryptoAuthFailed)?;
        mac.update(payload);
        let full_mac = mac.finalize().into_bytes();

        tag_out.copy_from_slice(&full_mac[..MAC_TAG_LEN]);
        self.tx_sequence_num += 1;
        Ok(())
    }

    /// Constant-time frame verification
    pub fn verify_frame(
        &mut self,
        payload: &[u8],
        tag_in: &[u8; MAC_TAG_LEN],
    ) -> Result<bool, SecurityError> {
        let mut expected_tag = [0u8; MAC_TAG_LEN];
        self.sign_frame(payload, &mut expected_tag)?;

        // Constant-time check prevents timing attacks
        let is_valid: bool = expected_tag.ct_eq(tag_in).into();
        if is_valid {
            self.rx_sequence_num += 1;
        }
        Ok(is_valid)
    }

    // --- Lower-level ECC204 Command Drivers over embedded-hal I2C ---

    fn ecc204_random<I2C, E>(
        &self,
        i2c: &mut I2C,
        addr: u8,
        out: &mut [u8; 32],
    ) -> Result<(), SecurityError>
    where
        I2C: I2c<Error = E>,
    {
        // ECC204 Opcode: RANDOM (0x1B)
        let cmd = [0x03, 0x07, 0x1B, 0x00, 0x00, 0x00, 0x00];
        i2c.write(addr, &cmd).map_err(|_| SecurityError::I2cError)?;
        
        // Wait for internal RNG generation execution time
        for _ in 0..1000 { cortex_m::asm::nop(); }
        
        i2c.read(addr, out).map_err(|_| SecurityError::I2cError)?;
        Ok(())
    }

    fn ecc204_sign<I2C, E>(
        &self,
        i2c: &mut I2C,
        addr: u8,
        key_slot: u8,
        digest: &[u8; 32],
        sig_out: &mut [u8; 64],
    ) -> Result<(), SecurityError>
    where
        I2C: I2c<Error = E>,
    {
        // ECC204 Opcode: SIGN (0x41)
        let mut cmd = [0u8; 39];
        cmd[0..3].copy_from_slice(&[0x03, 39, 0x41]);
        cmd[3] = key_slot;
        cmd[7..39].copy_from_slice(digest);

        i2c.write(addr, &cmd).map_err(|_| SecurityError::I2cError)?;
        i2c.read(addr, sig_out).map_err(|_| SecurityError::I2cError)?;
        Ok(())
    }

    fn ecc204_hmac<I2C, E>(
        &self,
        i2c: &mut I2C,
        addr: u8,
        key_slot: u8,
        digest: &[u8; 32],
        key_out: &mut [u8; 32],
    ) -> Result<(), SecurityError>
    where
        I2C: I2c<Error = E>,
    {
        // ECC204 Opcode: HMAC (0x11)
        let mut cmd = [0u8; 39];
        cmd[0..3].copy_from_slice(&[0x03, 39, 0x11]);
        cmd[3] = key_slot;
        cmd[7..39].copy_from_slice(digest);

        i2c.write(addr, &cmd).map_err(|_| SecurityError::I2cError)?;
        i2c.read(addr, key_out).map_err(|_| SecurityError::I2cError)?;
        Ok(())
    }
}
