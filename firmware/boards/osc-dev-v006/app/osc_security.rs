use embedded_hal::digital::{InputPin, OutputPin};

pub struct Ecc204SwiPin<PIN> {
    pin: PIN,
}

impl<PIN> Ecc204SwiPin<PIN>
where
    PIN: OutputPin + InputPin,
{
    pub fn new(pin: PIN) -> Self {
        Self { pin }
    }

    /// Wake up the ECC204 via SWI pulse on PC1
    pub fn wake_up(&mut self) -> Result<(), ()> {
        let _ = self.pin.set_low();
        // Hold low for > 60us to issue wake condition
        for _ in 0..1000 { cortex_m::asm::nop(); }
        let _ = self.pin.set_high();
        // Wait t_WHI (1.5ms) for device initialization
        for _ in 0..25_000 { cortex_m::asm::nop(); }
        Ok(())
    }

    /// Transmit raw SWI byte stream over PC1
    pub fn write_command(&mut self, bytes: &[u8]) -> Result<(), ()> {
        self.wake_up()?;
        for &byte in bytes {
            for bit in 0..8 {
                let bit_val = (byte >> bit) & 1;
                if bit_val == 0 {
                    let _ = self.pin.set_low();
                    for _ in 0..30 { cortex_m::asm::nop(); } // ~4.5us
                    let _ = self.pin.set_high();
                    for _ in 0..10 { cortex_m::asm::nop(); } // ~1.5us
                } else {
                    let _ = self.pin.set_low();
                    for _ in 0..10 { cortex_m::asm::nop(); } // ~1.5us
                    let _ = self.pin.set_high();
                    for _ in 0..30 { cortex_m::asm::nop(); } // ~4.5us
                }
            }
        }
        Ok(())
    }
}
