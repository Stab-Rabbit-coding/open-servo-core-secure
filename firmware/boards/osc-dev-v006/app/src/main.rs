#![no_std]
#![no_main]

use osc_servo_ch32::prelude::*;

use panic_halt as _;

#[cfg(feature = "defmt")]
use defmt_rtt as _;

tinyboot_ch32::app::app_version!();
osc_servo_ch32::install_isrs!();

// Security posture (`docs/security-architecture.md`).
//
// The message plane lives inside the transport, not here: `ServoBus` owns a
// `SecurityContext` and gates staged effects at the same verdict the CRC
// already gates (sec 7.2). So this board file needs no crypto code at all, and
// an earlier draft that ran a boot handshake ahead of `run!()` was solving the
// problem in the wrong place -- the session has to be installed through the
// runtime, and its ~670 ms of ECC204 traffic must not sit in the entry path.
//
// No ECC204 is fitted on any board in this repository yet, so the servo boots
// `SecurityState::Unsecured` (sec 4.2) and behaves exactly as it did before the
// security layer existed. Fitting one needs a board respin; the production
// SG90 board has `PD5`/`PD6` free and UART-capable for SWI (sec 0.3.2).
// Tracked in `TODO.md` sec 7.
#[qingke_rt::entry]
fn main() -> ! {
    osc_servo_ch32::log::info!("osc-dev-v006: boot");
    osc_servo_ch32::run!(BoardConfig {
        wiring: BoardWiring {
            dbg: DigitalPin::PC3,
            drv_en: DrvEn {
                pin: DigitalPin::PD0,
                active: Level::High,
            },
            // Rev B TTL bus subsystem (the default): the 74LVC2G241 is in
            // play, TX_EN = PC2 gating direction. `--features half-duplex`
            // drops the bus wiring -- the direct HDSEL wire carries none,
            // and on a buffer-populated board the TX_EN pull-down (R16)
            // keeps the buffer released.
            #[cfg(not(feature = "half-duplex"))]
            bus: BusWiring { tx_en: Pin::PC2 },
            current_sense: CurrentSenseConfig {
                gain: opa::Gain::X32,
                bias: opa::Bias::MidRail,
            },
            sensors: AdcPins {
                pos: AnalogChannel::A3,
                ntc: AnalogChannel::A2,
                vbus: AnalogChannel::A1,
                vmotor: (AnalogChannel::A5, AnalogChannel::A6),
            },
        },
        calibration: Calibration {
            shunt_r_mohm: 10,
            vbus_divider: Divider {
                top_ohm: 20_000,
                bot_ohm: 10_000,
            },
            vmotor_divider: Divider {
                top_ohm: 20_000,
                bot_ohm: 10_000,
            },
            // TH1 SDNT2012X103F3950FTF.
            ntc: NtcCal {
                beta: 3950,
                r0_ohm: 10_000,
                t0_cc: 2500,
                bias_r_ohm: 10_000,
            },
        },
        defaults: ConfigDefaults {
            pos_min_phys_urad: -1_570_796,
            pos_max_phys_urad: 1_570_796,
            vdd_mv: 3300,
            id: 1,
            baud: BaudRate::B1000000,
            response_deadline_us: DEFAULT_RESPONSE_DEADLINE_US,
        },
        model: MODEL_OSC_SERVO,
        hw_rev: 1,
    })
}
