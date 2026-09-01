# Handoff — `osc-sg90-v006` re-route and board-edge clearance

**Date:** 2026-08-30
**Branch:** `claude/sg90-reroute-edge-clearance`
**Board:** `hardware/boards/osc-sg90-v006/`
**Author of the work described here:** Claude Opus 5 (Anthropic), acting on
`Stab-Rabbit-coding`'s instruction. The component placement this route was
built on is `Stab-Rabbit-coding`'s own, made before the session started.

WBS entries: [`TODO.md`](../TODO.md) §10.
Citations: [`REFERENCES.md`](../REFERENCES.md) REF-FAB-001, REF-STD-007.

---

## 1. What was asked

Re-route the board after a manual placement pass across both sides, and
establish a defensible minimum board-edge clearance — the previous value was
picked without a source.

## 2. Delivered state

| Metric | Before (tracks cleared, zones refilled) | After |
| ------ | --------------------------------------- | ----- |
| Track segments | 0 | 89 |
| Vias | 0 | 2 |
| Unconnected items | 57 | **33** |
| DRC errors | 9 | **9** |
| DRC warnings | 7 | 7 |

**All 9 remaining errors are placement defects that predate the route** (§5).
The route closed 24 connections and introduced no violations. Verified with `kicad-cli` 10.0.3 and with
`hardware/tools/check_hole_to_edge.py`.

Backup of the pre-session board:
`hardware/boards/osc-sg90-v006/osc-sg90-v006.kicad_pcb.pre-reroute-20260829-2306.bak`
(untracked).

## 3. Board-edge clearance — the decision and its basis

The previous 0.40 mm was unsourced. It is now **0.30 mm**, derived rather than
chosen:

- **The electrical requirement is not binding.** This board's maximum working
  voltage is 8.4 V (2S LiPo on `VIN`). IPC-2221B Table 6-1, external conductors
  uncoated, below 3050 m, puts the 0–15 V band at **0.10 mm** [REF-STD-007] —
  already met by the global 0.10 mm copper-clearance rule, and exceeded by
  every netclass (0.127 mm signal, 0.200 mm VSYS/MOTOR).
- **The fabricator's routing tolerance is binding.** PCBWay builds this
  project's boards. They reject artwork below **0.20 mm** copper-to-outline and
  quote **0.25 mm** for the normal CNC-routing process [REF-FAB-001].
- **0.30 mm** is 1.5× the DFM gate and above the normal-process figure.

Two consistency defects were fixed along the way:

- **The board-setup value, not the custom rule, is what DRC reports against.**
  Editing only the `Copper to board edge` rule in the `.kicad_dru` had no
  effect — DRC went on citing "board setup constraints edge clearance
  0.4000 mm" from `min_copper_edge_clearance` in the `.kicad_pro`. Both now
  read 0.30 mm; any future clearance change has to touch both files.
- PCBWay separately requires **≥ 0.50 mm from any hole**, plated or unplated,
  to the outline [REF-FAB-001] — stricter than the copper rule, and the
  constraint that actually decides how close a via may sit (0.65 mm to via
  centre, for this board's 0.55/0.30 mm via).

### 3.1 The hole rule, and a correction

**Corrected 2026-08-31.** This document originally stated that KiCad could not
express hole-to-board-edge. That was wrong, and how it went wrong is the more
useful lesson.

`physical_hole_clearance` conditioned on `B.Layer == 'Edge.Cuts'` expresses the
rule correctly, and `osc-sg90-v006.kicad_dru` now carries it. It caught a
hand-routed `PGND` via at 0.40 mm from the outline the moment it was enabled.

The original test appeared to show the condition matching nothing — 199
violations unconditioned, 0 conditioned. Both runs were real, but the
conditioned one was run against a `.kicad_dru` that contained `;` comments, and
**a single `;` anywhere in a `.kicad_dru` makes the whole file fail to parse,
silently.** Every rule stops firing; DRC reports nothing to say so. The 0 was
the comments, not the condition.

| Rule under test | `;` in file | Violations |
| --------------- | ----------- | ---------- |
| `physical_hole_clearance (min 3.00mm)`, unconditioned | no | 199 |
| same | yes | 0 |
| same + `(condition "B.Layer == 'Edge.Cuts'")` | no | **20** |
| same | yes | 0 |

`#` and `( comment 1 "text" )` blocks fail the same way: the grammar accepts
`(version)` and `(rule)` at top level and nothing else, so a `.kicad_dru`
admits no comments in any form. Its commentary lives in
`hardware/boards/osc-sg90-v006/README2.md`; the prohibition is now in
`AGENTS.md` §Coding standards.

**After editing any `.kicad_dru`, prove the rules still fire** — raise a
minimum to an absurd value and check the violation count moves. A clean DRC run
is not evidence of a clean board; it may mean no rules ran. That is also why
`hardware/tools/check_hole_to_edge.py` is kept as an independent second
implementation and CI gate rather than retired: it parses the board directly,
needs no KiCad, and cannot be silently disabled by a stray character. The two
agree on this board to within 0.025 mm — half the Edge.Cuts stroke width,
KiCad measuring to the graphic's near edge and the script to its centreline.

## 4. `J4` — the reason the board will not auto-route **(read this first)**

`J4` uses `shared:Pot_Wire_Pads`, whose pads are a **KiCad 10 per-layer
padstack**:

- `F.Cu`: a 0.1 mm token pad inside a 0.9 mm drill — no annulus at all. This is
  what the `One-sided PTH annulus for Potentiometer` exception in the
  `.kicad_dru` exists for, and what the three `padstack` warnings ("PTH pad
  hole leaves no copper") report.
- `In1.Cu`, `In2.Cu`, `B.Cu`: real **1.8 mm** lands.

KiCad's Specctra exporter serialises only the F.Cu shape, concludes the pad has
no copper, and emits **no `(pin ...)` at all** — only unnamed `(keepout ...)`
circles, which FreeRouting does not honour the same way. The router therefore
never learns about the 1.8 mm back-side lands and drives `/SWDIO`, `+3V3` and
`/VSNA` straight across them.

**Every routing-induced DRC violation on this board, across six routing
configurations, involved J4 and nothing else.** No other footprint contributed
one. This is a footprint/export defect, not a router-tuning problem, and any
future auto-route hits the same wall until the footprint changes.

Two consequences to decide on:

1. **Routing.** Injecting a correctly-sized padstack and three pins into the
   `.dsn` does fix the routing — but FreeRouting 2.2.4 then hangs writing the
   session file (§6). So the conflicting copper is pruned after import instead,
   with `hardware/tools/prune_routing_conflicts.py`, and those connections are
   left for hand routing. A track crossing a plated land is a short; there is
   no correct automatic placement for it.
2. **Fabrication.** Zero annular ring on F.Cu is below PCBWay's stated 0.15 mm
   minimum [REF-FAB-001]. A barrel with no land on one side is prone to
   lifting. Either give the pad a real F.Cu annulus and accept the placement
   cost, or get PCBWay to confirm in writing that they will build it as drawn.

## 5. Placement conflicts the router cannot fix

Present with all tracks removed and zones refilled, so they are placement:

| Items | Actual | Required |
| ----- | ------ | -------- |
| `C1` pad 1 (`VSYS`, B.Cu) ↔ `J4` pad 2 (`/POT`) | 0.125 mm | 0.200 mm (`VSYS_FEED`) |
| `R9` pad 1 (`+3V3`) ↔ `TH1` pad 2 (`GND`) | 0.125 mm | 0.127 mm (`3V3`) |
| `R4` pad 1, `R9` pad 1, `TH1` pad 2 → outline | 0.255 mm | 0.300 mm (§3) |
| `NT1` pad 2 (`PGND`) ↔ `U7` pad 4 (`GND`) | shorting | — |

The three edge cases are above PCBWay's 0.20 mm hard gate, so they are
buildable; they are outside this board's own rule. A ~0.05 mm inboard nudge
clears them. `NT1` is the PGND/GND net tie, but the short is to a *different*
component, so it is an overlap rather than the tie doing its job.

## 6. Why the route stops at 33 unconnected

Not a settings problem. Six configurations were tried, all with a 0.20 mm
`.dsn` boundary inset, and **every one plateaued at the same 24 internal
FreeRouting violations and ~30 unrouted connections**:

| Config | Change | Result after import + prune |
| ------ | ------ | --------------------------- |
| A | all zone planes kept | **33 unconnected, 9 errors — shipped** |
| C | F.Cu/B.Cu surface pours stripped | 31 pre-prune, more errors |
| D | full-board GND/PGND pours stripped only | 34 unconnected, 9 errors |
| E | all planes stripped | worse; router hung on save |
| F | J4 keepouts removed (diagnostic) | no change to violation count |
| G/J | J4 pins injected into the `.dsn` | best route, but router hung on save |

Two structural causes, both worth a decision before another attempt:

1. **`In1.Cu` (GND) and `In2.Cu` (+3V3) are full-board planes.** A via on any
   third net is therefore a clearance violation against copper FreeRouting is
   not permitted to modify. It placed only **2–3 vias in every run**, on a
   board that needs many more to get between F.Cu and B.Cu.
2. **The F.Cu/B.Cu GND pours export as fixed planes**, so both signal layers
   read to the router as almost entirely obstructed. Stripping them from the
   `.dsn` and letting KiCad re-pour after import is the right pipeline in
   general, but it did not help here because (1) still blocks the vias, and it
   cost connectivity — the pours had been satisfying the ground connections for
   free.

Given (1), whether this board should be 6-layer is an open question. Note also
that `hardware/boards/osc-sg90-v006/README.md` §Assembly claims the board is
8-layer; it is **4-layer** (`F.Cu`, `In1.Cu`, `In2.Cu`, `B.Cu`).

### 6.1 The 33 unrouted connections

By net: `+3V3` 6, `PGND` 5, `VSYS` 3, `GND` 2, `/TX_EN` 2, and one each on
`/EN`, `/nRST`, `/ISNS-`, `/ISNS+`, `/TX`, `/VSNA`, `/IN1`, `/IN2`, `/SWDIO`,
`/VSNB`, `/SE_SWIO`, `/POT`, `/3V3_DRV`, `/MOT_A`, `/MOT_B`.

These need hand routing in the GUI, or a placement pass that opens via channels
through the inner planes.

## 7. Reproducing the pipeline

**The system KiCad is 9.0.2 and cannot load this board at all** — it is
`generator_version "10.0"`. Everything below uses the KiCad 10 AppImage at
`~/Downloads/programs/appImages/kicad-10.0.3-x86_64.AppImage`, extracted with
`--appimage-extract`; `squashfs-root/usr/bin/kicad-cli` and
`squashfs-root/AppRun python3.11` both work directly.

```sh
KI=/path/to/squashfs-root
FR=/usr/share/freerouting-2.2.4-linux-x64/lib/app/freerouting-executable.jar

# 1. clear tracks and export, from a script using pcbnew.ExportSpecctraDSN
$KI/AppRun python3.11 strip_and_export.py board.kicad_pcb clean.dsn

# 2. inset the .dsn boundary by 0.20 mm (Specctra has no per-item edge rule)
python3 prep_dsn.py clean.dsn route.dsn

# 3. route (use the JAR, not the native launcher)
java -jar $FR -de route.dsn -do route.ses -mp 100

# 4. import, then prune J4 conflicts and the stubs they leave
$KI/AppRun python3.11 import_ses.py board.kicad_pcb route.ses
$KI/bin/kicad-cli pcb drc --refill-zones --save-board --format json -o drc.json board.kicad_pcb
$KI/AppRun python3.11 hardware/tools/prune_routing_conflicts.py board.kicad_pcb drc.json J4
$KI/AppRun python3.11 hardware/tools/prune_routing_conflicts.py board.kicad_pcb drc.json --dangling
# repeat 4 until both report "removed 0"; removing one item can expose another

# 5. verify
python3 hardware/tools/check_hole_to_edge.py board.kicad_pcb
```

`prune_routing_conflicts.py` and `check_hole_to_edge.py` are committed under
`hardware/tools/`. The single-use `.dsn` helpers (`strip_and_export.py`,
`prep_dsn.py`, `import_ses.py`, `add_j4_pins.py`) were session scratch and are
not committed; each is a few dozen lines and the steps above describe them
fully.

## 8. Gotchas that cost time

- **Zone fills go stale after a placement move, and DRC does not refill by
  default.** The board as received reported **187** violations; 153 of them
  were phantom pad/track/via-to-zone clearance errors at "actual 0.1005 mm"
  against netclass minima of 0.127/0.200 mm. With `--refill-zones` the count
  is 25. Always refill before believing a DRC result on this board.
- **FreeRouting 2.2.4 hangs writing the `.ses` for some inputs.** It logs
  `Auto-router session completed` and then never writes the file. Reproducible
  on the all-planes-stripped `.dsn` and on both J4-pin-injected variants. If a
  run stalls after that line it will not recover — kill it.
- **`kicad-cli` must be ≥ the file's `generator_version`.** The 9.0.2 binary
  fails with a bare `Failed to load board`.
- KiCad 10 `.kicad_pcb` files have **no top-level net table**; pads carry net
  names directly. Tooling written against the KiCad 9 format will silently find
  zero nets.

## 9. Open items

All tracked in [`TODO.md`](../TODO.md) §10:

- §10.2 — decide `J4`'s footprint: real F.Cu annulus, or written fab sign-off.
- §10.3 — nudge `C1`/`J4`, `R9`/`TH1`, and the three edge-adjacent parts.
- §10.4 — hand-route the remaining 33 connections; decide on layer count.
- §10.6 — correct the board README's 8-layer claim.
- §10.1 — re-verify [REF-FAB-001] if the board moves to another fabricator;
  [REF-STD-007] is marked *requires verification* (IPC-2221B is paywalled and
  superseded by IPC-2221C, unchecked).
