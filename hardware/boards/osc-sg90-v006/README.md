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
- **Secure element** — Microchip ECC204 (`U7`), 8-pad UDFN package, SWI on
  `PD6` [REF-SE-001]. **Placed on B.Cu with a generic 3D proxy model; 2
  residual real DRC clearance conflicts still open** — see
  [Secure element](#secure-element) below and `TODO.md` §7.8.

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

`U7` is a Microchip ECC204 (Value `ECC204-MAVCZ-T`), **8-pad UDFN** package,
100 kbps PWM Single-Wire Interface (SWI). Design of record:
[`docs/security-architecture.md`](../../../docs/security-architecture.md);
citations: [`REFERENCES.md`](../../../REFERENCES.md) `REF-SE-001`.

| Signal | Net       | Notes                                                                   |
| ------ | --------- | ----------------------------------------------------------------------- |
| `SI/O` | `SE_SWIO` | `PD6` on the MCU, one pull-up (`R10`) to `+3V3`.                        |
| `VCC`  | `+3V3`    | 0.1 µF decoupling (`C5`).                                               |
| `GND`  | `GND`     | Also `EP` (pin 9, exposed pad) — REF-SE-001 recommends tying it to GND. |

**Package: 8-pad UDFN, not 3-Lead Contact.** The 3-Lead Contact package
(6.5×2.5 mm) — the original choice — was confirmed by exhaustive search to
have **zero** clear placement anywhere on this board's routed copper. The
same real part's 8-pad UDFN (2×3 mm) does fit; see `TODO.md` §7.8 for the
search methodology and REFERENCES.md's "Removed / superseded citations" for
the supersession record.

**Placement — on B.Cu, real position, 2 residual real DRC conflicts open:**

- `U7 (101.7, 93.4)`, `R10 (103.3, 92.4)`, `C5 (103.0, 97.1)`, `C10 (103.7,
  95.015, −90)`, `RS1 (105.245002, 96.755, 90)`, `C2 (108.18, 91.08)`, all
  B.Cu — these are the final positions after re-verifying (and, for `RS1`/
  `C10`, wholesale-restoring) a round of manual GUI repositioning; see the
  data-integrity note below.
- `pcb drc --severity-all`: driven from 148 violations (induced by the
  original manual 3-Lead-Contact placement) down to 74 total / 27 new vs. a
  clean baseline. Of those 27: **2 are real** (`U7` pad 5 and `R10` pad 2
  each sit too close to one of `J4`'s three real through-hole pads —
  `(104.51, 92.0)` / `(103.06, 95.0)` / `(104.51, 98.0)`, confirmed via
  live Gerber `%TO.P,J4,n%` component-attribute flashes, not a stale-fill
  or duplicate-reference artifact; see `TODO.md` §7.8 for the placement
  math proving `U7` cannot clear both nearby `J4`/`J2` pads at once, and for
  fix options), 18 are `GND_POUR` zone-clearance findings now assessed as
  **likely real** (point-in-polygon tested against a live-exported Gerber's
  filled copper region — `U7`'s non-`GND` pads land inside the poured
  copper with no relief — though a live KiCad "Fill All Zones" + re-DRC is
  still the only fully conclusive check), and 3 are cosmetic
  `lib_footprint_mismatch` warnings (hand-authored footprint graphics vs. a
  full library re-import).
- `sch erc --severity-all`: 0 new violations (1 pre-existing, unrelated
  `DRV8837C` library-mismatch warning).
- **(BLOCKER for fab)** The 2 real conflicts need either further placement
  in a live KiCad GUI (this work was done headlessly, matching real pad
  geometry pulled from the raw `.kicad_pcb` and cross-checked against
  exported Gerbers — no substitute for actually seeing it), smaller pads
  still, or a justified DRC exclusion matching the `.kicad_pro`'s existing
  three exclusions for `J4`'s one-sided-TH quirk.

**Still open beyond placement:**

- `R10`'s pull-up value is `TBD`. The summary datasheet [REF-SE-001] has no
  application-circuit section and gives no recommended SWI pull-up; that
  needs the NDA datasheet or a CryptoAuthLib reference design (`TODO.md`
  §7.4).
- `ECC204-MAVCZ-T` is the **plain, unprovisioned** ordering code — a real
  example straight from [REF-SE-001] §7 (Product Identification System), not
  the fleet provisioning SKU. Whether fleet units ship pre-provisioned
  (TrustFLEX/TrustCUSTOM) or get provisioned from this raw part is still open
  (`TODO.md` §7.1/§7.5).
- The footprint,
  [`hardware/shared.pretty/ECC204_UDFN-8-1EP_L2.0-W3.0-P0.50-EP0.61x1.3mm.kicad_mod`](../../shared.pretty/ECC204_UDFN-8-1EP_L2.0-W3.0-P0.50-EP0.61x1.3mm.kicad_mod),
  deliberately uses smaller-than-generous pads (peripheral pads 0.5×0.25 mm,
  EP land 0.61 mm wide) to fit this board's real estate — sized to
  [REF-SE-001] §6.1's own terminal-to-exposed-pad `K = 0.20 mm` minimum, not
  arbitrary. Verify against a real part/stencil before fab.
- **Correction to a prior session's note:** an earlier pass here reported a
  "duplicate `J4` footprint / phantom pad" bug. That was wrong — it was a
  sign error in the reviewer's own PCB rotation math (`Rot(+θ)` used where
  `Rot(−θ)` was correct), not a board defect. `J4` is a single real
  footprint with three real through-hole pads; see `TODO.md` §7.8 for the
  corrected transform and its Gerber-based verification.
- **3D models added** for `U7`, `R10`, `C5` so KiCad's 3D viewer no longer
  shows bare footprints for these parts. `R10`/`C5` use the existing
  KiCad standard-library STEP models
  (`Resistor_SMD.3dshapes/R_0402_1005Metric.step`,
  `Capacitor_SMD.3dshapes/C_0402_1005Metric.step`), matching this board's
  existing convention for other passives. `U7` has no vendor-supplied
  3D model (Microchip does not publish one for the 8-pad UDFN), so it uses
  a generic dimensionally-accurate proxy generated from
  [REF-SE-001] §6.1's package dimensions (2.00 × 3.00 mm body, 0.55 mm
  height) — script and STL live at
  [`hardware/shared.3dshapes/ECC204_UDFN-8-1EP_L2.0-W3.0-P0.50.stl`](../../shared.3dshapes/ECC204_UDFN-8-1EP_L2.0-W3.0-P0.50.stl).
  Model references were added both to the library footprint
  (`shared.pretty/...kicad_mod`) and to each board instance directly (KiCad
  stores a full independent copy per instance — a library-only edit does
  not propagate to footprints already placed on this board). Confirmed no
  DRC regression from adding the models (74/9/27 unchanged); full visual
  confirmation in KiCad's 3D viewer was not achieved headlessly and should
  be spot-checked in the GUI before fab.
- **ERC/DRC verified with KiCad 10.0.3** (`kicad-cli` from the KiCad 10
  AppImage — the system-installed `kicad-cli` is 9.0.2 and cannot even load
  this file, since it's `generator_version "10.0"`).

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
