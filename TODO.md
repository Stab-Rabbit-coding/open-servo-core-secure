# TODO — Work Breakdown Structure

Formal WBS for open-servo-core-secure. Items are numbered by subsystem; every
open item carries enough context to be picked up in a later session.

Legend: `[ ]` open · `[x]` closed · **(BLOCKER)** gates a downstream item.

---

## 7. Security element integration (ECC204)

Design of record: [`docs/security-architecture.md`](docs/security-architecture.md).
Citations: [`REFERENCES.md`](REFERENCES.md).

### 7.1 Procurement and part selection

- [ ] **(BLOCKER)** Confirm the ECC204 **SWI** ordering code against
      Microchip's full ordering-code table. [REF-SE-001] Table 4-1 lists
      `ECC204-TFLXAUTHU/S` and `ECC204-TCSMU/TCSMS` **without an interface
      column**, and the interface (SWI vs I²C) is fixed at the part number.
      The trade study (§0.3.1) selects SWI for the servo control board.
      Blocks 7.8.
- [ ] Decide TrustFLEX vs TrustCUSTOM. TrustFLEX suffices unless the slot
      configuration must change ([REF-SE-001] §4).
- [ ] Obtain the **complete ECC204 datasheet under NDA** from Microchip.
      Needed for 7.4. The summary datasheet [REF-SE-001] states on its cover
      that it omits the command set.
- [ ] Price and lead-time check for the 3-lead contact and 8-pad UDFN packages
      at the fleet quantity.

### 7.2 Pin and interface assignment

- [ ] **(BLOCKER)** Read the USART instance and remap index for `PD5`/`PD6` on
      the CH32V006F8U6 from `ch32_metapac::METADATA` — the same source
      `servo-ch32/build.rs` generates `UsartMapping` from. Do **not** assume.
      If they resolve to USART1 remaps only, they contend with the bus and the
      SE falls back to bit-banged SWI inside a quiet window (§4.3), losing the
      hardware-timing guarantee. Blocks 7.8.
- [x] Establish that `PC1` is never free (bus `/RX` on rev-B and
      `osc-sg90-v006`; Qwiic I²C SDA on `osc-dev-v006` Rev 2A). The draft
      `main.rs.new` assignment was wrong on all three boards — §0.3.
- [x] Establish free pins on `osc-sg90-v006`: `PC2`, `PC3`, `PD5`, `PD6`
      (extracted from the board netlist).
- [x] Interface trade study, SWI vs I²C, for the servo control board — §0.3.1.
      **Verdict: SWI.**

### 7.3 Timing validation on silicon **(BLOCKER for flight)**

The architecture's timing figures are **analytical, not measured** — no Rust
toolchain was available when they were written, and this repository's
convention is that timing facts are silicon-measured ([F1]–[F15]).

- [ ] **(BLOCKER)** Measure per-frame fold cost with the `bench` feature.
      `SEC_PROBE` (`firmware/lib/drivers/src/bench.rs`) exposes `folds`,
      `fold_bytes`, `fold_cycles`. Target: confirm or refute the estimate of
      **~34 µs per ~70-byte hot-loop cycle**.
- [ ] **(BLOCKER)** Measure the resulting kernel tick loss against the
      20.11 kHz idle baseline and the transport's measured relation
      (loss ≈ 1.2–1.4 × transport-HIGH duty, `osc-servo-transport.md` §2).
      Estimate: +4–5 % at a 1 kHz bus cycle. Accept or trigger a lever below.
- [ ] Measure turnaround delta at 0.5 M / 1 M / 2 M / 3 M. Expectation: hidden
      at 0.5 M/1 M (grid-bound), additive at 2 M/3 M (pipeline-bound) — §2.6.
- [ ] If cost is material, apply levers in order: (a) split feed across
      deadline A and the covered checkpoint, (b) HalfSipHash-1-3, (c) policy
      narrowing. §2.6.
- [ ] Add the measured figures to `osc-native-protocol.md` §11 as new `[Fn]`
      entries and replace the analytical numbers in §2.6.

### 7.4 SE command layer **(BLOCKER for any SE function)**

- [ ] **(BLOCKER)** Implement `se::ecc204::Framing`. Requires the command
      packet layout, the word-address/flag tokens, the packet CRC-16
      parameters and the wake timing — available only from the NDA datasheet
      (7.1) or CryptoAuthLib [REF-SE-002, REF-SE-003]. **Deliberately not
      guessed:** a fabricated constant in a security driver would compile, run,
      and fail obscurely while looking authoritative.
- [ ] Implement `SwiUart` for the chosen USART in `servo-ch32/src/providers/`.
- [ ] Verify HMAC round-trip timing on silicon against the published 2 × 80 ms
      execution estimate (§0.1).
- [ ] Review CryptoAuthLib's licence before vendoring any of its code. This
      repository currently uses it as a documentary source only.

### 7.5 Provisioning

- [ ] Extract and apply specific section numbers from [REF-SE-004]; currently
      marked *requires verification* in `REFERENCES.md`.
- [ ] Define the fleet key-management plan: who holds the HSM, how `K_grp` is
      distributed to hosts, rotation policy.
- [ ] Document the factory provisioning run and its audit record.

### 7.6 Export control

- [ ] Confirm the ECCN with counsel or via CCATS. Engineering assessment
      (§6.3): authentication-only cryptography with **no confidentiality
      service**, so expected to fall outside 5A002/5D002 under **Note 2 to
      Category 5 — Part 2** of the CCL. Note the repository is already public,
      which brings EAR §742.15(b) into scope.

### 7.7 Airworthiness security

- [ ] Conduct a **DO-326A / ED-202A** airworthiness security risk assessment.
      None exists. This is the gap between "defensible engineering" and
      "certifiable" — §6.1.
- [ ] Apply **DO-356A / ED-203A** methods to the §8 threat model.
- [ ] Determine whether a DO-178C Design Assurance Level applies to the servo
      firmware, and at what level.
- [ ] Assess the fail-safe decision in §2.7 (hold last authenticated goal,
      never cut torque) against the aircraft-level safety assessment. The
      reasoning is sound in isolation but must be confirmed against the actual
      airframe's failure modes — a control surface held at a stale command is
      only safe if the airframe tolerates it.

### 7.8 Hardware **(no ECC204 is fitted on any board)**

- [ ] **(BLOCKER)** Add the ECC204 to `osc-sg90-v006`: SWI on `PD6`, one
      pull-up, decoupling. Blocked by 7.1 and 7.2. Confirm the 10 × 12.5 mm
      board can absorb the part inside the SG90 case.
- [ ] Add an SE breakout path for `osc-dev-v006` bringup via the Qwiic
      connector (§0.3.2).
- [ ] Re-run the EMC/ESD review with the SE net beside the H-bridge. SWI
      carries > 7 kV HBM vs > 4 kV for I²C parts [REF-SE-001 §3.1], which was
      an input to the interface trade.

### 7.9 Protocol and host

- [x] `FLAG_AUTH` (`INST` bit 1), AUTH trailer, `Unauthenticated` /
      `SecurityLockout` result codes, `SEC_*` MGMT sub-ops — in
      `osc-protocol::wire`.
- [ ] Update `docs/osc-native-protocol.md` §3.1/§5/§5.3/§9 to describe the
      `FLAG_AUTH` extension normatively. The protocol doc currently still
      lists bit 1 as reserved and "encryption/auth" as a non-goal (§1).
- [ ] Implement host-side tag generation in `osc-host` behind the
      `osc-security/host` feature.
- [ ] Add integration tests in `lib/integration/tests/` covering the
      authenticated hot loop, injection, suppression and replay against the
      discrete-event sim.

### 7.10 Firmware follow-ups

- [x] `osc-security` crate: HalfSipHash-2-4 with reference vectors, KDF
      labels, stream digest, replay window, policy, session state machine.
- [x] Transport hook: fold at the covered checkpoint, gate at the verdict,
      both stageable and verdict-first paths.
- [x] `SEC_PROBE` bench counters.
- [x] Remove the broken `osc_security.rs` / `main.rs.new` drafts (ARM
      intrinsic on a RISC-V target; SWI on the bus RX pin).
- [ ] Wire `SecurityDiag` and `SecurityState` into the telemetry region —
      `auth_fail_count`, `replay_drop_count`, and a `status_flags` bit for the
      security state. Needs address assignment in `TELEMETRY-COMMON`
      (`osc-native-protocol.md` §5.4 reserves `0x20C..0x220`).
- [ ] Implement the `MGMT SEC_*` dispatch handlers and the quiet-window
      handshake (§4.3).
- [ ] Decide the policy register's persistence: it is currently a compile-time
      default. A flight build must not be able to boot permissive by accident.
- [ ] Re-check flash budget after the SE command layer lands. The app has
      ~57 KB; `osc-security` is small (no tables) but the SE driver is not yet
      written.

---

## 8. Open questions carried from the design

- [ ] `K_grp` compromise: extracting one servo's group key from RAM forges
      broadcasts fleet-wide (§3). Accepted and documented, bounded by the
      ECC204's tamper resistance and per-servo `K_uni` on unicast/MGMT. Revisit
      if the fleet threat model changes.
- [ ] Digest divergence: a servo that drops a frame on CRC reverts its whole
      cycle while its peers commit (§2.1). Fail-safe and arguably better than
      today's silent stale commit, but it changes fleet behaviour under a noisy
      bus and should be observed on real hardware.
- [ ] Boot time: SE session establishment is ~670 ms (§3). Confirm the
      airframe's power-on sequence tolerates it, or move establishment to a
      background quiet window after a permissive boot.

---

## 9. Tooling

### 9.1 Markdown lint

- [x] Add `.markdownlint-cli2.jsonc`. New documents
      (`security-architecture.md`, `REFERENCES.md`, `TODO.md`, `AGENTS.md`,
      `PROJECT_INDEX.md`) lint clean under it.
- [ ] **Deferred, upstream scope.** `MD013` (line length) is disabled because
      the inherited docs — `control-theory.md`, `osc-native-protocol.md`,
      `README.md` — are written as unwrapped prose paragraphs and are correct
      as written. Enabling it would flag many hundreds of pre-existing lines.
      Only worth doing as a deliberate, separate repo-wide reflow, and not
      part of the security work.
- [ ] Add a Markdown lint job to `.github/workflows/ci.yml`. CI currently
      lints Rust only, which is why the above went unnoticed.
