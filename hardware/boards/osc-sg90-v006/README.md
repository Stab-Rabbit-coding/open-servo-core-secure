# OSC SG90 CH32

> ⚠️ **Designed, not spun. Pre-fabrication, pre-bringup.**
> This board has never been fabbed. Numbers and behaviour described below are **design intent**, not measured. The schematic and layout exist; firmware v2 now runs on the dev board (the bus transport is silicon-proven) but has never run on *this* board. Don't fab this without doing your own review first, and expect changes when bringup finally happens.

OpenServoCore swap board for SG90-class hobby servos. Compact, double-sided, designed to physically replace the factory PCB inside an SG90 case. CH32V006-based, single-wire TTL servo bus (osc-native protocol), drop-in mechanical fit.

## Overview

- **MCU** — CH32V006F8U6 (RISC-V, 48 MHz, 62 KB flash, 8 KB RAM).
- **Motor driver** — TI DRV8837 H-bridge (1.8 A peak, IN1/IN2 PWM).
- **LDO** — TPAP2210K-3.3, 3.3 V logic rail.
- **UART buffer** — SN74LVC1G07 open-drain for the half-duplex TTL bus.
- **Current shunt** — 25 mΩ, 1 % on the low side.
- **Power input** — 3.3-8.4 V (1S-2S LiPo) via the servo cable.
- **Debug** — 4-pad SWD interface for WCH-LinkE.
- **Position feedback** — existing servo potentiometer wiper.
- **Form factor** — 10 × 12.5 mm, double-sided assembly.
- **Secure element** — Microchip ECC204 (`U7`), 3-lead contact package, SWI on
  `PD6` [REF-SE-001]. **Schematic-only, not yet placed on the board outline**
  — see [Secure element](#secure-element) below and `TODO.md` §7.8.

## Connectors

### Servo cable — J1 (1×3, through-hole pads)

Standard hobby-servo 3-wire pinout, but the signal pin carries half-duplex UART instead of PWM:

| Pin | Net    | Notes                                   |
| --- | ------ | --------------------------------------- |
| 1   | `VIN`  | 3.3-8.4 V (1S-2S LiPo).                 |
| 2   | `DATA` | Half-duplex UART, baud TBD by firmware. |
| 3   | `GND`  |                                         |

### Motor — J2 (2 wire pads)

Solder directly to the SG90 motor leads.

| Pad    | Net              |
| ------ | ---------------- |
| `OUTA` | Motor terminal A |
| `OUTB` | Motor terminal B |

### Potentiometer — J3 (3 pads)

Connects to the existing servo potentiometer.

| Pad    | Net          |
| ------ | ------------ |
| `+3V3` | Pot top rail |
| `VPOS` | Wiper        |
| `GND`  | Pot ground   |

### Debug — J4 (4 pads, 1.27 mm pitch)

Single-wire SWD for WCH-LinkE. Programmed via `wlink` / `probe-rs`. Firmware build instructions land when firmware v2 starts; until then there's nothing to flash.

| Pad | Net     |
| --- | ------- |
| 1   | `+3V3`  |
| 2   | `GND`   |
| 3   | `SWDIO` |
| 4   | `NC`    |

## Design intent

These are the targets the board was laid out around. None are measured.

- **Power architecture** — VIN routes to the DRV8837 directly; 3.3 V logic rail off the TPAP2210K LDO. Bulk capacitance is 2× 10 µF on VIN. PGND/GND joined at a single net tie.
- **Position sensing** — 12-bit ADC on `VPOS` with an RC filter (10 kΩ + 100 nF, fc ≈ 160 Hz) to suppress wiper noise. Sub-degree resolution is the design goal; actual precision waits on bringup and firmware oversampling.
- **Voltage sensing** — 20 kΩ / 10 kΩ dividers (ratio 0.333) on `VINS`, `VSNA`, `VSNB`. 8.4 V → 2.8 V at the ADC.
- **Current sensing** — 25 mΩ low-side shunt, sized for the DRV8837's ~1.76 A peak via `R_shunt ≈ 44 mV / I_stall`, so the hardware stall-detect path trips near the driver's max output. Raw V_shunt = 25 mV at 1 A; firmware sets ADC sampling, gain path, and software overcurrent threshold.
- **Communication** — single-wire half-duplex TTL servo bus (Dynamixel-TTL electrical layer; the wire protocol is [osc-native](../../../docs/osc-native-protocol.md)). The SN74LVC1G07 open-drain buffer enables bidirectional comms over the single `DATA` line. Bus rates: 0.5-3 Mbaud, default 1 M.
- **Protection** — PESD5V0L1UL TVS on `DATA`.

## Secure element

`U7` is a Microchip ECC204 (Value `ECC204-RBVCZ-T`), 3-lead contact package,
100 kbps PWM Single-Wire Interface (SWI). Design of record:
[`docs/security-architecture.md`](../../../docs/security-architecture.md);
citations: [`REFERENCES.md`](../../../REFERENCES.md) `REF-SE-001`.

| Signal | Net       | Notes                                            |
| ------ | --------- | ------------------------------------------------ |
| `SI/O` | `SE_SWIO` | `PD6` on the MCU, one pull-up (`R10`) to `+3V3`. |
| `VCC`  | `+3V3`    | 0.1 µF decoupling (`C5`).                        |
| `GND`  | `GND`     |                                                  |

**Status — schematic capture only, three things still open:**

- `R10`'s pull-up value is `TBD`. The summary datasheet [REF-SE-001] has no
  application-circuit section and gives no recommended SWI pull-up; that
  needs the NDA datasheet or a CryptoAuthLib reference design (`TODO.md`
  §7.4).
- `U7`/`R10`/`C5` are placed **off the board outline** in the `.kicad_pcb` —
  a staging position, not a real layout. Whether the 6.5×2.5 mm 3-lead-contact
  package actually fits this 10 × 12.5 mm board, clear of the H-bridge/motor
  copper and the SG90 case keepout, is unconfirmed and needs to be done
  visually in the PCB editor (`TODO.md` §7.8).
- `ECC204-RBVCZ-T` is the **plain, unprovisioned** ordering code — a real
  example straight from [REF-SE-001] §7 (Product Identification System), not
  the fleet provisioning SKU. Whether fleet units ship pre-provisioned
  (TrustFLEX/TrustCUSTOM) or get provisioned from this raw part is still open
  (`TODO.md` §7.1/§7.5).
- The new footprint,
  [`hardware/shared.pretty/ECC204_Contact-3_L6.5-W2.5-P2.00.kicad_mod`](../../shared.pretty/ECC204_Contact-3_L6.5-W2.5-P2.00.kicad_mod),
  uses the package's terminal *max* dimensions directly from [REF-SE-001]
  §6.3 as the pad size — no IPC-7351 land-pattern calculation was applied.
  Verify against a real part before fab.
- **ERC/DRC verified with KiCad 10.0.3** (`kicad-cli` from the KiCad 10
  AppImage — the system-installed `kicad-cli` is 9.0.2 and cannot even load
  this file, since it's `generator_version "10.0"`). `sch erc
  --severity-all` returns **zero new violations**: the one remaining warning
  (`DRV8837C` library-symbol mismatch on `U4`) is pre-existing, unrelated to
  this change. `pcb drc --severity-all` also returns **zero new
  error-severity violations** against the same in-place baseline; the only
  new findings are 7 informationally-unrouted pads (`U7`/`R10`/`C5` are
  staged, not routed — expected) and 3 `lib_footprint_mismatch` warnings
  (the hand-authored footprint graphics are simplified vs. what a full
  library re-import would produce — cosmetic, not electrical). A first pass
  had the sign of `U7`'s `GND` pin backwards (KiCad always negates a
  symbol's local Y before applying its placement rotation, even at
  `angle 0` — this isn't folded into the rotation matrix the way you'd
  expect); DRC caught it as `pin_not_connected` / `power_pin_not_driven`,
  fixed by recomputing the pin position with that rule and re-running.

## Assembly

Double-sided assembly, no vias-in-pads. Suitable for hot-plate / hot-air / IR reflow, or JLCPCB SMT. The board is 8-layer for routing density at this size.

## Mechanical fit

Designed to drop into a standard SG90 servo case:

- Motor leads solder to `OUTA` / `OUTB`.
- Pot wiper connects to `J3`.
- Servo cable connects to `J1`.
- Debug pads remain accessible through the case opening for bringup / reflashing.

Pad spacing for motor leads and debug pitch are still **to be confirmed against a physical case** when first boards arrive.

## License

Hardware files (schematic, layout, board files in this directory) are licensed under [CERN-OHL-P v2.0](../../../LICENSE-HARDWARE). See the [top-level README](../../../README.md#license) for the full licensing picture.
