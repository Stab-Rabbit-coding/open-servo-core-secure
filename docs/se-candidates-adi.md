# Secure element candidates — Analog Devices DeepCover family

Input to plan unit U3 (MCU and crypto selection) of
[`docs/plans/2026-11-16-001-feat-parametric-secure-microservo-board-plan.md`](plans/2026-11-16-001-feat-parametric-secure-microservo-board-plan.md).
This record scores the Analog Devices parts the owner shortlisted as 3 × 3 mm
(0.118 × 0.118 in) or smaller against the ECC204 baseline. It selects nothing:
selection belongs to the U3 gate.

**Authenticity.** Every figure below is read from the datasheet cited in its
column header (REF-SE-008 through REF-SE-015), or from REF-SE-001 for the
ECC204. A figure no datasheet states is marked *not stated*. Nothing is taken
from memory, and the manufacturer pages could not be fetched from the
build environment (the site refuses automated clients).

## What the design needs

`docs/security-architecture.md` splits the secure element's job into three
parts:

1. **Identity plane.** An ECDSA P-256 signature and a certificate chain, so a
   host can tell a genuine servo from a counterfeit (§5).
2. **Session plane.** A stored secret and HMAC-SHA-256 for session-key
   derivation (§3).
3. **Lifecycle counter.** A non-resettable counter that binds firmware-update,
   save and factory-reset grants. Budget: dozens of counts over a service
   life (§0.5, §5.3).

The secure element is never in a per-message path (§0.1). Its timing matters
for boot and re-key only.

## Comparison

| Part | Interface | ECDSA sign | ECDSA verify, ECDH | HMAC-SHA-256 | Counter | Supply | Max temperature |
| ---- | --------- | ---------- | ------------------ | ------------ | ------- | ------ | --------------- |
| ECC204 (REF-SE-001) | single-wire 100 kbps or I²C 400 kHz | yes | no | yes | 10 000, saturating | 1.65–5.5 V | 221 °F (105 °C) |
| DS28C40 (REF-SE-009) | I²C, 1 MHz | yes | yes | yes | *not stated* | 2.97–3.63 V | 257 °F (125 °C) |
| DS28E40 (REF-SE-010) | 1-Wire, 9.09 and 62.5 kbps | yes | yes | yes | *not stated* | 2.97–3.63 V | 257 °F (125 °C) |
| DS2478 (REF-SE-008) | I²C, 1 MHz | yes | yes | yes | *not stated* | 2.97–3.63 V | 257 °F (125 °C) |
| DS28E35 (REF-SE-011) | 1-Wire, 76.9 kbps | yes | *not stated* | *not stated* | 17-bit decrement | 3.3 V ±10% | 185 °F (85 °C) |
| DS28E16 (REF-SE-012) | 1-Wire, 90.9 kbps | no | no | SHA-3 only | 17-bit decrement | 1.71–3.63 V | 185 °F (85 °C) |
| DS28C22 (REF-SE-013) | I²C, 100 and 400 kHz | no | no | yes | *not stated* | 3.3 V ±10% | 185 °F (85 °C) |
| DS28E10 (REF-SE-014) | 1-Wire, 15.4 and 125 kbps | no | no | SHA-1 only | none | 2.8–3.6 V | 185 °F (85 °C) |
| DS2475 (REF-SE-015) | I²C 100 and 400 kHz, host side | no | verify only | no | n/a | 3.3 V ±10% | 185 °F (85 °C) |

### Timing and memory

The DS28C40, DS28E40 and DS2478 publish one shared timing table (maxima):
HMAC 4 ms, ECDSA sign 50 ms, ECDSA verify or ECDH 160 ms, ECC key-pair
generation 500 ms, TRNG 50 ms, memory read 2 ms, memory write 150 ms. Each
has 6 Kb of one-time-programmable (OTP) memory, split 3 Kb user data and
3 Kb keys and secrets (DS28C40 and DS28E40 *Detailed Description*; DS2478
likewise).

The ECC204 figures in `docs/security-architecture.md` §0.1 (80 ms per `SHA`
command, two commands per HMAC, 500 ms per sign) come from REF-SE-002, not
from the ECC204 datasheet. The two sets are therefore not like-for-like
sources, and both are manufacturer maxima that U7 re-measures.

The DS28E16 publishes 15 ms computation, 5 ms read and 60 ms write. The
DS28E35 and DS28C22 datasheets are abridged and refer to the full datasheet
for computation time.

### Packages

| Part | Package (body size as printed) | Ordering code printed |
| ---- | ------------------------------ | --------------------- |
| DS28C40, DS28E40 | 10-pad TDFN 0.118 × 0.157 in (3 × 4 mm); 10-pad side-wettable TDFN 0.118 × 0.118 in (3 × 3 mm) | `DS28C40G/V+T`, `DS28E40G/V+T` (3 × 4 mm); `DS28C40ATB/VY+T`, `DS28E40ATB/VY+T` (3 × 3 mm) |
| DS2478 | 10-pad side-wettable TDFN 0.118 × 0.118 in (3 × 3 mm) | `DS2478ATB/VY+T` |
| DS28E16 | 6-pad TDFN-EP 0.118 × 0.118 in (3 × 3 mm); 6-ball WLP (size not printed); 2-contact SFN, not solderable | `DS28E16X-S+T` (WLP), `DS28E16Q+T` (TDFN-EP) |
| DS28E35 | 8-pad TDFN-EP (0.079 × 0.118 in (2 × 3 mm) appears in the text); 6-pin TSOC (size not printed) | `DS28E35Q+T`, flagged "future product, contact factory" |
| DS28C22, DS28E10, DS2475 | 8 TDFN-EP; 3 SOT23 and 6 TSOC; 6 SOT23 (sizes not printed) | see datasheets |

No datasheet text gives package height. Heights are referenced only by outline
drawing number, and shall be read from those drawings and checked against the
0.236 in (6 mm) minimum envelope before any part is selected.

## Findings

1. **The DS28C40 and DS28E40 are the only parts here covering the identity
   and session planes together.** They add host-signature verification and
   ECDH, which removes the ECC204's sign-only limit (`security-architecture.md`
   §5). They share one crypto core and one set of electrical limits, and differ
   in interface: I²C needs two MCU pins and pull-ups, 1-Wire needs one pin and
   a bit-banged master.
2. **The DS2478 is redundant on a servo board.** Its datasheet calls it a
   companion to the DS28E40 and DS28C40. Both of those contain the same engine,
   so a servo needs the authenticator alone. A host or gateway could use the
   DS2478.
3. **The lifecycle counter is the gap.** The DS28C40, DS28E40 and DS2478
   datasheets list no counter. The DS28E35 and DS28E16 have a 17-bit decrement
   counter (131 072 counts) but each covers one plane only: the DS28E35 signs
   but states no HMAC, and the DS28E16 has no ECDSA and uses SHA-3, which
   differs from the SHA-256 HMAC in `osc-security` and the host tools.
4. **Openness (R4) is undetermined for every part.** The DS28C40, DS28E40,
   DS2478 and DS28E16 each print "Request Security User Guide". The DS28E35,
   DS28C22 and DS2475 are abridged datasheets with the full datasheet on
   request. No datasheet says whether that release is under NDA. This is the
   question that failed the ECC204, so it shall be answered before selection.
5. **Maker domicile.** The DS28E35, DS2478 and DS2475 datasheets carry Analog
   Devices' Wilmington, Massachusetts address. Fabrication and assembly
   locations are *not stated* in any of them. Domicile is necessary for the
   sourcing rule, not sufficient.
6. **Standby current is ambiguous.** The DS28C40 and DS2478 tables give standby
   0.5 mA typical, 2 mA maximum, while the DS2478 typical-characteristics graph
   is scaled in µA and reads near zero at room temperature. No sleep mode is
   listed for either. The DS28C22 states 0.5 µA typical in sleep mode.
7. **Rejected.** DS28E10 (SHA-1, 28 bytes of OTP, no ECDSA, no counter) and
   DS2475 (host-side verifier, not a servo-side part) do not fit the design.
   DS28C22 (symmetric only, no counter stated) and DS28E16 (SHA-3, no ECDSA)
   each cover one plane and are retained only as fallbacks.

## Open actions

Tracked in `TODO.md` §11.1:

- Request the DS28C40 and DS28E40 Security User Guides from Analog Devices and
  record the release terms (owner action).
- Decide the lifecycle-counter approach: a counter described in the guides, a
  two-device design, or a redesign with an MCU-held counter (weaker, since flash
  is rewritable, and needing a threat-model rework).
- Read package heights from the outline drawings.
- Resolve the standby-current units against the full datasheet or a bench
  measurement.
