# REFERENCES

Catalogue of every external standard, datasheet, specification and source this
repository relies on. Citations elsewhere in the codebase — in Rust doc
comments, Markdown, schematics and commit messages — refer to entries here by
**REF-ID**.

**Policy.** No reference, citation, standard, section number or constant is
ever fabricated. Where a value cannot be traced to a published document with a
validated URL, it is either omitted or explicitly marked *requires
verification*, and a `TODO.md` item is opened against it.

---

## Secure element

### REF-SE-001 — ECC204 CryptoAuthentication Summary Data Sheet

- **Designation:** Microchip DS40002436C (2023–2025)
- **Title:** *ECC204 CryptoAuthentication™ Summary Data Sheet*
- **URL:** <https://ww1.microchip.com/downloads/aemDocuments/documents/SCBU/ProductDocuments/DataSheets/ECC204-CryptoAuthentication-Summary-Data-Sheet-DS40002436.pdf>
- **Local copy:** `docs/ECC204-CryptoAuthentication-Summary-Data-Sheet-DS40002436.pdf`
- **Sections applied:**
  - *Features* (p. 1) — ECDSA P-256 **sign only**; SHA-256 and HMAC; internal
    TRNG; single ECC private key, one device certificate, one CA signer
    certificate, single symmetric secret key, 64-byte user memory; monotonic
    counter **max 10 000**; unique 72-bit serial; SWI at **100 kbps** and I²C
    at **400 kHz**; JIL "High" attack-potential resistance; FIPS 140-3
    compliance-mode option.
  - *Table 1 / Figure 1* (p. 2) — pin configuration; SWI vs I²C pin functions;
    3-lead contact, 8-pad UDFN, 8-lead SOIC packages.
  - *§1.2 Device Features* — EEPROM contents, interface options, parasitic
    power mode, counter attachment to key.
  - *§1.3 Cryptographic Operation* — private key generated internally, never
    exposed.
  - *§2.1 Cryptographic Standards* — SHA-256 (FIPS 180-4), HMAC (FIPS 198-1),
    ECDSA (ANSI X9.62-2005, FIPS 186-5).
  - *§2.2.2 Random Number Generator* — SP 800-90A/B/C certified TRNG.
  - *§2.2.3 Compliance Mode* — FIPS 140-3 enforcement bit.
  - *§3.1 Absolute Maximum Ratings* — ESD HBM **> 4 kV (I²C)**, **> 7 kV
    (SWI)**, CDM > 2 kV.
  - *§3.2.2 / §3.2.3* — SWI DC parameters and parasitic-power mode.
  - *§4 / Table 4-1* — Trust&GO / TrustFLEX / TrustCUSTOM provisioning flows;
    ordering codes.
- **Cited from:** `docs/security-architecture.md` (§0.1–0.5, §2, §3, §5, §6),
  `firmware/lib/drivers/src/se/ecc204.rs`,
  `firmware/lib/osc-security/src/se.rs`, `firmware/lib/osc-security/src/keys.rs`
- **Limitation (important):** this is the **summary** datasheet. It states on
  its cover that the complete document is available only under NDA. It
  contains **no command set, no execution times, no command-packet framing and
  no SWI token timing**. Those gaps are the reason for the compile gate in
  `se/ecc204.rs` (see `TODO.md` §7.4).

### REF-SE-002 — Microchip CryptoAuthLib

- **Title:** *CryptoAuthLib — Library for interacting with the CryptoAuthentication secure elements*
- **URL:** <https://github.com/MicrochipTech/cryptoauthlib>
- **Files applied:**
  - `lib/calib/calib_command.h` — command opcodes (`ATCA_READ` 0x02,
    `ATCA_MAC` 0x08, `ATCA_WRITE` 0x12, `ATCA_DELETE` 0x13, `ATCA_NONCE` 0x16,
    `ATCA_LOCK` 0x17, `ATCA_COUNTER` 0x24, `ATCA_INFO` 0x30, `ATCA_GENKEY`
    0x40, `ATCA_SIGN` 0x41, `ATCA_SHA` 0x47, `ATCA_SELFTEST` 0x77);
    ECC204-specific SHA modes (`SHA_MODE_ECC204_HMAC_START` 0x03,
    `SHA_MODE_ECC204_HMAC_END` 0x02); `COUNTER_MAX_VALUE_CA2` = 10000;
    `ATCA_CA2_CONFIG_SIZE` = 64; `ATCA_CA2_CONFIG_SLOT_SIZE` = 16.
  - `lib/calib/calib_execution.c` — `device_execution_time_ecc204` table:
    `COUNTER` 20 ms, `DELETE` 200 ms, `GENKEY` 500 ms, `INFO` 20 ms, `LOCK`
    80 ms, `NONCE` 20 ms, `READ` 40 ms, `SELFTEST` 600 ms, `SHA` 80 ms,
    `SIGN` 500 ms, `WRITE` 80 ms.
- **Cited from:** `firmware/lib/drivers/src/se/ecc204.rs`,
  `docs/security-architecture.md` §0.1
- **Note:** used as a **documentary source for interface constants only**. No
  CryptoAuthLib code is copied into this repository; the drivers here are
  independent Rust implementations. CryptoAuthLib is distributed under
  Microchip's own licence — consult it before vendoring any of its code.
- **Cross-check:** the 10 000 counter ceiling appears independently in
  REF-SE-001 §Features and in `COUNTER_MAX_VALUE_CA2`, which is why it is
  treated as reliable.

### REF-SE-003 — CryptoAuthLib SWI-over-UART HAL

- **Title:** *cryptoauthlib — `lib/hal/hal_swi_uart.c`*
- **URL:** <https://github.com/MicrochipTech/cryptoauthlib/blob/main/lib/hal/hal_swi_uart.c>
- **Applied:** the SWI-over-UART encoding — **230 400 baud** for data
  (115 200 during the wake sequence), **one UART character per SWI data bit**
  (`0x7F` = logic 1, `0x7D` = logic 0), **LSB first**, receive tolerance
  `(c ^ 0x7F) < 2`.
- **Cited from:** `firmware/lib/drivers/src/se/swi.rs`,
  `docs/security-architecture.md` §0.1, §0.3
- **Note:** independent Rust implementation of the documented encoding; no code
  copied.

### REF-SE-004 — Security Exchange Process for TrustFLEX and TrustCUSTOM Provisioning

- **Designation:** Microchip DS50004144
- **Title:** *Security Exchange Process for TrustFLEX and TrustCUSTOM Provisioning*
- **Local copy:** `docs/Security-Exchange-Process-for-TrustFLEX-and-TrustCUSTOM-Provisioning-DS50004144.pdf`
- **Applied:** provisioning flow and Secure Exchange Package handling.
- **Cited from:** `docs/security-architecture.md` §5.1
- **Status:** *requires verification* — specific section numbers have not yet
  been extracted and applied. `TODO.md` §7.5.

### REF-SE-005 — Trust Platform Manifest File Full Format

- **Designation:** Microchip DS60001759
- **Local copy:** `docs/Trust-Platform-Manifest-File-Full-Format-DS60001759.pdf`
- **Status:** *requires verification* — held for the provisioning workflow; not
  yet cited from code or design docs.

### REF-SE-006 — Infineon OPTIGA Trust M datasheet

- **Local copy:** `docs/infineon-optiga-trust-m-datasheet-en.pdf`
- **Status:** held as a **considered alternative** to the ECC204. Not selected;
  not cited from code. Retained for the trade record.

### REF-SE-007 — 3-Lead Contact Package Usage

- **Designation:** Microchip DS00004041
- **Local copy:** `docs/3-Lead-Contact-Package-Usage-DS00004041.pdf`
- **Applied:** mechanical usage of the 3-lead contact package considered for
  the SG90 board (§0.3.1).
- **Status:** *requires verification* — section numbers not yet extracted.

---

## Cryptographic algorithms

### REF-CRYPTO-001 — SipHash

- **Title:** Jean-Philippe Aumasson and Daniel J. Bernstein, *"SipHash: a fast
  short-input PRF"*, INDOCRYPT 2012, LNCS 7668, pp. 489–508.
- **DOI:** <https://doi.org/10.1007/978-3-642-34931-7_28>
- **Author's page:** <https://www.aumasson.jp/siphash/siphash.pdf>
- **Applied:** the SipHash round function, key schedule and finalisation.
  HalfSipHash is the 32-bit-word variant.
- **Cited from:** `firmware/lib/osc-security/src/mac.rs`,
  `docs/security-architecture.md` §2.4

### REF-CRYPTO-002 — SipHash reference implementation

- **URL:** <https://github.com/veorq/SipHash>
- **Files applied:** `halfsiphash.c` (round function, initial state
  `v2 = 0x6c796765` / `v3 = 0x74656462`, key mixing, `cROUNDS` = 2 /
  `dROUNDS` = 4, tail construction, `v2 ^= 0xff` finalisation for 32-bit
  output, `b = v1 ^ v3`); `vectors.h` (`vectors_hsip32`); `test.c` (vector
  convention: `k[i] = i`, `in[i] = i`, message length = vector index).
- **Cited from:** `firmware/lib/osc-security/src/mac.rs` (test vectors)
- **Licence:** CC0 / public domain dedication. The implementation in this
  repository is independent Rust written against the published algorithm; the
  **test vectors** are taken directly, which is their purpose.

---

## Cryptographic standards

### REF-STD-001 — FIPS 180-4, Secure Hash Standard

- **URL:** <https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.180-4.pdf>
- **Applied:** SHA-256, as implemented by the ECC204 (REF-SE-001 §2.1.1).

### REF-STD-002 — FIPS 186-5, Digital Signature Standard

- **URL:** <https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.186-5.pdf>
- **Applied:** ECDSA over NIST P-256, as implemented by the ECC204
  (REF-SE-001 §2.1.3). Note REF-SE-001 §Features cites FIPS 186-**4** while
  §2.1.3 cites FIPS 186-**5**; the datasheet is internally inconsistent on this
  point. Both are recorded; the design does not depend on the difference.

### REF-STD-003 — FIPS 198-1, The Keyed-Hash Message Authentication Code (HMAC)

- **URL:** <https://csrc.nist.gov/publications/fips/fips198-1/FIPS-198-1_final.pdf>
- **Applied:** HMAC-SHA-256 as the session KDF, executed inside the ECC204
  (REF-SE-001 §2.1.2).
- **Cited from:** `docs/security-architecture.md` §3, §6;
  `firmware/lib/osc-security/src/keys.rs`

### REF-STD-004 — NIST SP 800-90A/B/C, Random Bit Generation

- **URLs:**
  - <https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.800-90Ar1.pdf>
  - <https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.800-90B.pdf>
  - <https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.800-90C.2pd.pdf>
- **Applied:** the ECC204's TRNG construction and certification
  (REF-SE-001 §2.2.2).

### REF-STD-005 — ANSI X9.62-2005

- **Title:** *Public Key Cryptography for the Financial Services Industry: The Elliptic Curve Digital Signature Algorithm (ECDSA)*
- **URL:** <https://www.ansi.org/>
- **Applied:** ECDSA, as cited by REF-SE-001 §2.1.3.
- **Status:** paywalled; cited because the datasheet cites it. The design
  depends on FIPS 186-5 (REF-STD-002), which is freely available.

### REF-STD-006 — FIPS 140-3

- **URL:** <https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.140-3.pdf>
- **Applied:** the ECC204's compliance-mode configuration bit
  (REF-SE-001 §2.2.3).
- **Status:** *requires verification* — the specific aspects the ECC204's
  compliance bit enforces are not enumerated in the summary datasheet.

---

## Microcontroller

### REF-MCU-001 — CH32V006 datasheet and reference manual

- **Vendor:** WCH (Nanjing Qinheng Microelectronics)
- **URL:** <https://www.wch-ic.com/products/CH32V006.html>
- **Applied:** RV32EC core, 48 MHz, 62 KB flash / 8 KB SRAM, USART and pin
  alternate-function mapping, factory ESIG unique ID (RM ch. 19), HSITRIM.
- **Cited from:** `docs/osc-native-protocol.md` §9.2, §10;
  `docs/security-architecture.md` §0.3, §0.4
- **Status:** *requires verification* — the USART instance and remap index for
  `PD5`/`PD6` on the CH32V006F8U6 must be confirmed against
  `ch32_metapac::METADATA` before the SWI pin assignment is frozen
  (`TODO.md` §7.2).

---

## Internal specifications

These are this repository's own normative documents, listed so that code
citations resolve consistently.

| REF-ID | Document |
| ------ | -------- |
| REF-OSC-001 | `docs/osc-native-protocol.md` — the wire protocol |
| REF-OSC-002 | `docs/osc-servo-transport.md` — the servo-side transport |
| REF-OSC-003 | `docs/control-theory.md` — the control cascade |
| REF-OSC-004 | `docs/driver-pattern.md` — the layering |
| REF-OSC-005 | `docs/security-architecture.md` — the security architecture |
| REF-OSC-006 | `docs/SecurityElement.md` — the originating design note |

---

## Removed / superseded citations

| Citation | Status | Reason |
| -------- | ------ | ------ |
| "SWI effective throughput 28.8 kbit/s (230400 / 8)" | **Superseded** | Ignored UART start/stop bits. Each SWI data bit costs a whole 10-bit UART character, so a data byte is 80 bit-times: **23.0 kbit/s, ~347 µs/byte**. Corrected in `docs/security-architecture.md` §0.1 and pinned by `swi::transfer_time_us`. |
| "ECC204 HMAC round trip ≈ 100 ms" | **Superseded** | Assumed one command. An ECC204 HMAC is a **two-command** `SHA` sequence (`HMAC_START` + `HMAC_END`, REF-SE-002), so ≈ **208 ms**. Corrected in §0.1, §3. |
| ECC204 SWI on pin `PC1` (`main.rs.new` draft) | **Removed** | `PC1` is the bus receive path on the rev-B and `osc-sg90-v006` configurations and the Qwiic I²C SDA on `osc-dev-v006` Rev 2A. Never free. See §0.3. |
| `cortex_m::asm::nop()` in the `osc_security.rs` draft | **Removed** | ARM intrinsic on a RISC-V (QingKe RV32EC) target; could not compile. See §0.4. |
| Infineon OPTIGA Trust M as the selected SE | **Superseded** | ECC204 selected. REF-SE-006 retained for the trade record. |
