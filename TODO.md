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
      column**, and Note 3 states the TrustCUSTOM sample device is equivalent
      to the **I²C** (`DA`) part, not SWI — this table does not show a
      3-lead-contact or `CZ` (SWI) Trust Platform SKU, and is only a
      "representative sample" (Note 1). The trade study (§0.3.1) selects SWI
      for the servo control board. **Narrower now:** [REF-SE-001] §7 (Product
      Identification System) *does* have a plain, non-Trust-Platform ordering
      grammar with an explicit interface column (`RB`/`MA`/`SS` package ×
      `CZ`/`DA` I/O), and gives `ECC204-RBVCZ-T` (3-lead contact, SWI, tape &
      reel) as a worked example — a real, orderable **unprovisioned** device,
      used for the schematic/footprint (U7). What's still unresolved: whether
      a **pre-provisioned** (TrustFLEX/TrustCUSTOM) SWI SKU exists for fleet
      use, or whether fleet units must go through the TrustCUSTOM configurator
      starting from the raw `-RBVCZ-T` part. Still blocks final BOM
      commitment; no longer blocks schematic/footprint capture (done, see
      7.8).
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

- [x] Schematic capture for `osc-sg90-v006`: `U7`, `SI/O` on `PD6` via the
      `SE_SWIO` net label, `R10` SWI pull-up (`+3V3` to `SE_SWIO`), `C5`
      0.1 µF VCC decoupling. `SE_SWIO` added to the `DIGITAL` netclass
      pattern in `osc-sg90-v006.kicad_pro`. **ERC-verified with KiCad
      10.0.3** (system `kicad-cli` is 9.0.2 and can't load this file at all;
      used the KiCad 10 AppImage — `AppImage kicad-cli <args>`, no
      extraction needed, it's a `sharun`-runtime image). `sch erc
      --severity-all`: 0 new violations (1 pre-existing, unrelated
      `DRV8837C` library-mismatch warning on `U4`).
- [x] **Package swapped: 3-Lead Contact → 8-Pad UDFN.** The user moved
      `U7`/`R10`/`C5` onto the real board (B.Cu) by hand and asked for
      induced DRC violations to be driven to zero. Investigation found the
      3-Lead Contact package (6.5×2.5 mm) **does not fit anywhere on this
      board** — a full computational search of every position on both
      copper layers (real pad/track/via geometry parsed from the `.kicad_pcb`,
      500+ candidate points) found **zero** locations without copper
      conflict, because this fully-routed 12.1×9.6 mm board has no
      contiguous clear region that size. Switched to the same real part's
      **8-Pad UDFN (2×3 mm)** — an order of magnitude smaller footprint,
      genuinely fits. New symbol `ECC204-MAVCZ-T` in
      `hardware/shared.kicad_sym` (pinout: 1/2/3/6/7 NC, 4 GND, 5 SI/O, 8
      VCC, 9 EP — [REF-SE-001] Table 1/Fig. 1); new footprint
      `hardware/shared.pretty/ECC204_UDFN-8-1EP_L2.0-W3.0-P0.50-EP0.61x1.3mm.kicad_mod`
      built from [REF-SE-001] §6.1 mechanical dims, **with two deliberate,
      disclosed deviations from a generous IPC land** to fit this board's
      real estate: peripheral pad size 0.5×0.25 mm (vs. a typical
      toe-extended ~0.87×0.25 mm) and EP land narrowed to 0.61 mm wide
      (vs. 1.5 mm D2 nominal), sized to the datasheet's own
      terminal-to-exposed-pad `K = 0.20 mm` minimum against the pulled-in
      peripheral pads — **not** the full nominal exposed-pad size. Verify
      both against a real part/stencil before fab. Superseded
      `ECC204-RBVCZ-T` / `ECC204_Contact-3_L6.5-W2.5-P2.00` — see
      REFERENCES.md "Removed / Superseded Citations".
  - [x] Final placement (all B.Cu): `U7` `(101.7, 93.4)`, `R10`
        `(103.3, 92.4)`, `C5` `(103.0, 97.1)`. `C10` and `RS1` — moved by
        the user to make room for the (now-abandoned) 3-Lead Contact part —
        reverted to their original positions (`(103.7, 95.015, -90)` /
        `(105.245002, 96.755, 90)`); the UDFN doesn't need that space.
        **This answers the long-open "confirm the 10×12.5 mm board can
        absorb the part" question: not the 3-Lead Contact package, but yes
        for the UDFN, at this specific spot.**
  - [x] `pcb drc --severity-all` driven from 148 induced violations (the
        user's manual 3-Lead-Contact placement) down to **74 total / 27 new
        vs. a clean in-place baseline**:
    - **2 real residual conflicts, confirmed and root-caused** (`U7` pad 5
      vs. `J4`'s `/POT` pad; `R10` pad 2 vs. `J4`'s `GND` pad).
      **(BLOCKER for fab)** Needs either: further hand-nudging in the live
      KiCad GUI, pads shrunk further, or an explicit, justified DRC
      exclusion added via the GUI's right-click "Exclude this violation"
      (the `.kicad_pro` already carries three such exclusions for `J4`'s
      one-sided-TH quirk, added the same way — **do not hand-author a
      `drc_exclusions` entry**; the hash format for `clearance`/
      `shorting_items` findings isn't the same as the `padstack` ones
      already there and guessing it risks a silently-inert exclusion).
      Mathematically confirmed unavoidable at the current pad geometry:
      `J4` has **three** real 1.8 mm pads (not the `size 0.1mm`/`0.9mm`-drill
      values its `Pot_Wire_Pads` footprint declares — KiCad enforces a
      larger effective copper size for manufacturability that the
      footprint's own `size` field doesn't reveal) at `(104.51, 92.0)`,
      `(103.06, 95.0)`, `(104.51, 98.0)`, confirmed via a live Gerber
      export's `%TO.P,J4,n%` component-attribute comments next to the
      exact `D03` flash coordinates. `U7` sits in the 3.5 mm gap between
      the first two and needs ≈3.75 mm to clear both simultaneously.
    - **18 `GND_POUR` zone-clearance findings — likely real, not stale.**
      The same Gerber-based verification method used on `J4` was reapplied
      to the zone: a freshly exported B.Cu Gerber's filled copper region
      polygon (point-in-polygon tested against `U7`'s non-GND pad
      coordinates) covers those pads with no relief cutout. This is
      consistent with either a live-recomputed fill genuinely needing a
      keepout there, or a stale stored fill — Gerber export **cannot
      distinguish the two** (it may itself read the same stored
      `filled_polygon` data DRC does; `kicad-cli` exposes no explicit
      "recompute" step for either command). Treat as **likely real**
      pending a live "Fill All Zones" (`B`) + re-DRC in the GUI, which is
      the only way to settle it with certainty.
    - **3 `lib_footprint_mismatch` warnings** — cosmetic; the hand-authored
      footprint graphics are simplified vs. what a full library re-import
      would produce.
    - Zero other new violations (no shorting, no solder-mask-bridge, no
      hole-clearance beyond the two `J4` items above).
  - [x] **Root cause of the "phantom J4" confusion, found and fixed:** an
        entire round of this session was spent unable to reconcile
        DRC-reported `J4` pad coordinates with hand-computed positions,
        eventually (wrongly) concluded to be a stale-zone-fill artifact.
        The real cause: **the B.Cu vs. F.Cu pad-rotation formula used
        earlier in this task was backwards.** Confirmed by cross-checking
        against `%TO.P%` component-attribute comments in a live Gerber
        export (unambiguous ground truth — the exact pad, exact
        coordinate, direct from KiCad's own plotter): for a footprint
        stored with rotation angle θ, the pad's true absolute offset is
        `R(−θ)` applied to the local pad coordinate (`R` = standard CCW
        rotation; local Y additionally negated first for a `B.Cu`
        footprint) — **not** `R(+θ)` as assumed earlier. This is the
        opposite sign convention from the schematic-symbol pin transform
        (`[[feedback_kicad_hand_authoring]]`, unaffected/still correct);
        recorded as a distinct rule in the new
        `[[feedback_kicad_pcb_footprint_rotation]]` memory so the two are
        never conflated again. DRC's reported `J4` positions were correct
        the entire time.
  - [x] **3D models added** so KiCad's 3D viewer shows real bodies for
        `U7`/`R10`/`C5` instead of bare footprints. `R10`/`C5` reference
        the standard KiCad library models already used by every other
        0402 on this board (`${KICAD9_3DMODEL_DIR}/Resistor_SMD.3dshapes/
        R_0402_1005Metric.step`, `.../Capacitor_SMD.3dshapes/
        C_0402_1005Metric.step` — matching this board's existing
        convention, not `KICAD10_3DMODEL_DIR`). `U7` has **no real vendor
        model** (none published for this part) — built a generic proxy in
        OpenSCAD from the same real [REF-SE-001] §6.1 dimensions the
        footprint uses (2.00×3.00×0.55 mm body, pin-1 corner notch),
        exported to STL (installed OpenSCAD is 2021.01 — no STEP export;
        STL has no per-face color but KiCad renders it fine), stored at
        `hardware/shared.3dshapes/ECC204_UDFN-8-1EP_L2.0-W3.0-P0.50.stl`.
        Model references added to both the library footprint file and the
        three live PCB instances directly (a hand-authored PCB instance
        does **not** inherit model references added later to its library
        footprint — same lesson as the EP-pad-size fix earlier this
        session: **PCB footprint instances carry their own full copy of
        everything, always edit both**). Verified via `kicad-cli pcb
        render` (basic quality, ~5 s): loads without error and DRC stays
        at the same 74/9 after adding — headless visual confirmation that
        the tiny (0.55 mm / sub-mm) bodies are actually visible in a
        render was inconclusive at reachable resolution/framing; confirm
        by eye in the KiCad 3D viewer.
  - [ ] **Data-integrity note for future hand-edits:** a `re.sub` over a
        UUID-anchored block, applied to add `(justify mirror)` to three
        footprints' text, matched far more than intended and corrupted
        `C10`/`RS1`'s position and rotation (and possibly other B.Cu
        footprints' text properties) without any error — the file stayed
        syntactically valid throughout, so the corruption was silent
        (only DRC's fill count going from 74/27 to 96/55 gave it away).
        Recovered by `git checkout --` the file (working-tree PCB changes
        weren't yet committed) and rebuilding the three footprints fresh
        with explicit, single-target string edits instead of a regex
        sweep. **This repeated a second time** in this same session (the
        user's own KiCad-GUI move of `RS1` desynced its per-pad/per-property
        rotation suffix from its outer footprint rotation — KiCad's move
        tool can flatten a footprint's individual sub-element angles
        without touching the outer one) and was only caught because DRC's
        finding count jumped; fixed by wholesale-replacing the whole
        footprint block from `git show HEAD:...`, not by patching just the
        outer `(at ...)` line. **Never use a broad regex substitution
        across an extracted multi-footprint block for a "small" cosmetic
        change; anchor every replacement to a single unique string, and
        after any hand-edit to a rotated footprint, diff its *entire*
        block against a known-good copy (not just the outer position
        line) before trusting it — an outer/inner rotation mismatch
        produces cascading, hard-to-trace DRC findings against completely
        unrelated nearby components.**
  - [ ] **(BLOCKER)** `R10`'s value is `TBD` — no SWI pull-up value is given
        in [REF-SE-001] (the summary datasheet has no application-circuit
        section). Needs the NDA datasheet or a CryptoAuthLib reference design
        (ties to 7.4).
  - [ ] Confirm the exact ordering-code/provisioning SKU per 7.1 before BOM
        commitment; `U7`'s Value (`ECC204-MAVCZ-T`) is the plain/unprovisioned
        device, explicitly flagged as such in its Description field.
- [ ] **New, unrelated finding:** `osc-sg90-v006.kicad_pcb` has **two**
      footprints both referenced `J4` (both `shared:Pot_Wire_Pads`) — one on
      the real board at `(107.21, 95, -90)`, one sitting at
      `(115.815, 85.51, 90)`, well outside the board outline (max X is
      112.1). Pre-existing, not touched this session. Duplicate reference
      designators are undefined behaviour for ERC/DRC net/pad attribution
      (see the data-integrity note above — this may be *why* some DRC
      output was hard to trace back to real geometry) and for any BOM/pick
      -and-place export. Needs triage: is the off-board one a leftover
      staging copy that should be deleted, or does the board actually need
      a second Pot_Wire_Pads instance with a real reference?
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

## 10. PCB routing and DFM — `osc-sg90-v006`

Opened 2026-08-30 after a full re-route of the board following the user's
manual placement pass. Toolchain: KiCad 10.0.3 AppImage (`kicad-cli`, `pcbnew`
Python) + FreeRouting 2.2.4 (`freerouting-executable.jar`, headless). The
system KiCad is 9.0.2 and cannot load this board at all — see §7.8.
All work by Claude Opus 5 (Anthropic), on the user's instruction.

### 10.1 Board-edge clearance — verified against standards

- [x] **Copper-to-outline set to 0.30 mm** (was 0.40 mm, an unsourced value).
      Derivation, recorded in `osc-sg90-v006.kicad_dru` and cited to
      `REFERENCES.md`: the electrical requirement is *not* binding — at this
      board's 8.4 V maximum working voltage, IPC-2221B Table 6-1
      (external, uncoated, below 3050 m) asks 0.10 mm [REF-STD-007], which the
      global 0.10 mm copper-clearance rule already meets. The binding number is
      the fabricator's routing tolerance: PCBWay rejects artwork below 0.20 mm
      and quotes 0.25 mm for the normal CNC-routing process [REF-FAB-001].
      0.30 mm is 1.5× the DFM gate. Set in both the `.kicad_dru` custom rule and
      `min_copper_edge_clearance` in the `.kicad_pro`, which were previously
      inconsistent with each other.
- [x] **Hole-to-outline (0.50 mm, [REF-FAB-001]) is enforced by a DRC rule.**
      `physical_hole_clearance` conditioned on `B.Layer == 'Edge.Cuts'`, in the
      `.kicad_dru`. It immediately caught a hand-routed `PGND` via sitting
      0.40 mm from the outline.
- [x] **Corrected on 2026-08-31 — an earlier session recorded that KiCad could
      not express this rule. That was wrong, and the reason matters.** The test
      that "proved" it was run against a `.kicad_dru` containing `;` comments,
      and **a single `;` anywhere in a `.kicad_dru` makes the whole file fail to
      parse, silently** — every rule stops firing and DRC says nothing. The
      0 violations attributed to the `Edge.Cuts` condition were the comments
      killing the file. Re-tested comment-free: the same rule reports 20
      violations at a 3.00 mm minimum. `#` and `( comment 1 "text" )` blocks
      fail identically; the grammar takes `(version)` and `(rule)` and nothing
      else. Found by `Stab-Rabbit-coding`, characterised and written up into
      `AGENTS.md` §Coding standards and the board's `README2.md`.
- [x] Cross-checked against `hardware/tools/check_hole_to_edge.py`, which now
      serves as the independent second implementation and the CI gate (it needs
      no KiCad install, and a `.kicad_dru` cannot be trusted to have run). The
      two agree at every threshold tested, differing by 0.025 mm — half the
      Edge.Cuts stroke width, KiCad measuring to the graphic's near edge and
      the script to its centreline.
- [ ] Re-verify [REF-FAB-001] if the board moves to another fabricator; the
      0.20/0.25/0.50 mm figures are PCBWay's capability, not a consensus
      standard.
- [ ] [REF-STD-007] is marked *requires verification*: IPC-2221B is paywalled
      and the Table 6-1 values were taken from secondary summaries. Also
      superseded by IPC-2221C (Dec 2023), unchecked. Not load-bearing — the
      applied clearances exceed the quoted values by 1.27× to 2×.

### 10.2 `J4` `Pot_Wire_Pads` — invisible to the router **(BLOCKER for a clean route)**

- [ ] **The three pot wire pads use a KiCad 10 per-layer padstack:** a 0.1 mm
      token pad on F.Cu inside a 0.9 mm drill — no annulus at all, which is
      what the `One-sided PTH annulus for Potentiometer` exception in the
      `.kicad_dru` exists for and what the three `padstack` warnings ("PTH pad
      hole leaves no copper") report — but **1.8 mm lands on `In1.Cu`,
      `In2.Cu` and `B.Cu`**. The back and inner sides are real, large pads.
- [ ] **Consequence for routing:** KiCad's Specctra exporter serialises only
      the F.Cu shape, concludes the pad has no copper, and emits **no
      `(pin ...)` at all** — just unnamed `(keepout ...)` circles that
      FreeRouting does not honour the same way. The router therefore never
      learns about the 1.8 mm back-side lands and drives `/SWDIO`, `+3V3` and
      `/VSNA` straight across them. **Every routing-induced DRC violation on
      this board involved J4 and nothing else** — no other footprint
      contributed one. Injecting a correctly-sized padstack and three pins into
      the `.dsn` does fix the routing, but FreeRouting then hangs writing the
      session file (§10.4), so the conflicting copper is pruned after import
      instead and those connections are left for hand routing. Any future
      auto-route of this board hits the same wall until the footprint changes.
- [ ] **Consequence for fabrication:** zero annular ring *on F.Cu* is below
      PCBWay's stated 0.15 mm minimum [REF-FAB-001]. A barrel with no land on
      one side is prone to lifting. Decide whether to give these pads a real annulus (and accept the
      placement cost) or to confirm with PCBWay that they will build it as
      drawn. This is a design decision for the user, not a routing fix.

### 10.3 Placement conflicts the router cannot resolve

Present in the board with **all tracks removed and zones refilled**, so they are
placement, not routing. DRC baseline in that state: 57 unconnected, 9 errors.

- [ ] `C1` pad 1 (`VSYS`, B.Cu) to `J4` pad 2 (`/POT`): **0.125 mm**, against
      the 0.200 mm `VSYS_FEED` netclass clearance.
- [ ] `R9` pad 1 (`+3V3`) to `TH1` pad 2 (`GND`): **0.125 mm**, against the
      0.127 mm `3V3` netclass clearance. Marginal by 2 µm.
- [ ] `R4` pad 1, `R9` pad 1 and `TH1` pad 2 sit **0.255 mm** from the board
      outline, against the 0.30 mm rule set in §10.1. Above PCBWay's 0.20 mm
      hard gate, so buildable, but outside this board's own rule — nudge the
      three parts ~0.05 mm inboard, or accept and document the exception.
- [ ] `NT1` pad 2 (`PGND`) shorts `U7` pad 4 (`GND`), and `NT1`'s internal
      segment shorts the same pad. `NT1` is the PGND/GND net tie; the short is
      to a *different* component, so this is an overlap, not the tie doing its
      job.

### 10.4 Auto-route status

Delivered state of `osc-sg90-v006.kicad_pcb`: **89 track segments, 2 vias,
33 unconnected items, 9 DRC errors — every one of the 9 a §10.3 placement
defect, none introduced by routing.** The unrouted board with zones refilled
scores 57 unconnected and the same 9 errors, so the route removed 24
connections and added no violations.

- [x] Re-routed with FreeRouting 2.2.4 against a `.dsn` exported from KiCad
      10.0.3 with all tracks cleared, the boundary inset 0.20 mm (Specctra has
      no per-item edge constraint, so the inset plus the router's own class
      clearance is the only lever), and all zone planes left in place.
      Conflicting copper was then pruned with
      `hardware/tools/prune_routing_conflicts.py`, driven off KiCad's own DRC
      report by UUID rather than re-derived geometry, and the resulting stubs
      removed with its `--dangling` mode. Zones refilled via
      `kicad-cli pcb drc --refill-zones --save-board`.
- [ ] **33 connections are still unrouted and need hand routing.** They are not
      a router-settings problem. Six configurations were tried — all zone
      planes kept; surface pours stripped; all planes stripped; inner layers
      retyped `signal`; J4 keepouts removed; J4 pins injected — and every one
      plateaued at the same 24 internal FreeRouting violations and ~30
      unrouted connections. Keeping all planes (the configuration shipped) was
      marginally the best at the DRC level.
- [ ] Two structural causes, both worth a decision before another attempt:
      (a) `In1.Cu` (GND) and `In2.Cu` (+3V3) are full-board planes, so a via on
      any third net is a clearance violation against copper FreeRouting may not
      modify — it placed only 2–3 vias in every run, on a board that needs many
      more; (b) with the F.Cu/B.Cu GND pours exported as fixed planes, both
      signal layers read as almost entirely obstructed. Stripping the pours
      from the `.dsn` and letting KiCad re-pour after import is the right
      pipeline in general, but it did not help here because (a) still blocks
      the vias, and it cost connectivity because the pours had been satisfying
      the ground connections for free.
- [ ] Given (a), consider whether this board should be 6-layer. See §10.6.
- [ ] **FreeRouting 2.2.4 hangs writing the `.ses`** for some inputs: it logs
      `Auto-router session completed` and then never writes the file. Seen
      reproducibly on the all-planes-stripped `.dsn` and on both J4-pin-injected
      `.dsn` variants; not seen on the three configurations that shipped
      candidates. If a run stalls after that log line, it will not recover.

### 10.5 Findings closed in passing

- [x] **Zone fills were stale** after the placement move. DRC on the board as
      received reported 187 violations; 153 of them were phantom
      pad/track/via-to-zone clearance errors at "actual 0.1005 mm" against
      netclass minima of 0.127/0.200 mm. Re-running with `--refill-zones`
      dropped the count to 25. Any DRC on this board must refill zones first.
- [x] **The board-setup value, not the custom rule, is what DRC reports
      against.** Editing only the `Copper to board edge` rule in the
      `.kicad_dru` had no effect — DRC went on citing "board setup constraints
      edge clearance 0.4000 mm" from `min_copper_edge_clearance` in the
      `.kicad_pro`. Both now say 0.30 mm. Any future clearance change has to
      touch both files.

### 10.6 Documentation defect

- [ ] `hardware/boards/osc-sg90-v006/README.md` §Assembly states "The board is
      8-layer for routing density at this size." The board is **4-layer**:
      `F.Cu`, `In1.Cu`, `In2.Cu`, `B.Cu`. Correct the README. Worth deciding at
      the same time whether the board *should* be 6- or 8-layer, given §10.4.

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
