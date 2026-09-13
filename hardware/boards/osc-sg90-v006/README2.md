# `osc-sg90-v006` — design-rule commentary

Commentary for [`osc-sg90-v006.kicad_dru`](osc-sg90-v006.kicad_dru).

**This file exists because a `.kicad_dru` cannot hold comments.** The grammar
accepts `(version)` and `(rule)` at top level and nothing else. A single `;`
anywhere in the file — top level, inside a rule block, between two clauses —
makes the **whole file fail to parse, silently**: every custom rule stops
firing and DRC gives no indication that anything went wrong. `#` and
`( comment 1 "text" )` blocks fail the same way. Measured on `kicad-cli`
10.0.3 against this board: an identical `physical_hole_clearance` rule reports
**199** violations with no comment in the file and **0** with one.

After editing the `.kicad_dru`, prove the rules still fire — raise a minimum to
an absurd value and confirm the violation count moves. A clean DRC run does not
mean a clean board; it may mean no rules ran. See `AGENTS.md` §Coding
standards.

## Copper to board edge — 0.30 mm

Fabrication-driven, not electrical.

At this board's 8.4 V maximum working voltage, IPC-2221B Table 6-1 (B1,
external uncoated, ≤ 3050 m) asks only **0.10 mm** [REF-STD-007] — already met
by the global `Copper clearance` rule and exceeded by every netclass
(0.127 mm signal, 0.200 mm VSYS/MOTOR). The binding number is the fabricator's
routing tolerance: PCBWay quote **0.25 mm** copper-to-outline for the normal
CNC-routing process and reject artwork below **0.20 mm** [REF-FAB-001].
**0.30 mm** is 1.5× that DFM gate.

This value must also be set as `min_copper_edge_clearance` in the `.kicad_pro`.
That board-setup value is what DRC reports against; the custom rule alone does
not change the reported constraint.

## Hole to board edge — 0.50 mm

PCBWay require **≥ 0.50 mm** from any hole, plated or unplated, to the board
outline [REF-FAB-001]. This is stricter than the 0.30 mm copper rule and is
what actually limits how close a via may sit: for this board's 0.55/0.30 mm
via it puts the via centre 0.65 mm inboard. Note PCBWay require **1.0 mm** for
a panel made by V-scoring; the 0.50 mm figure assumes CNC routing.

Expressed as `physical_hole_clearance` conditioned on
`B.Layer == 'Edge.Cuts'`. This rule is cross-checked by
[`hardware/tools/check_hole_to_edge.py`](../../tools/check_hole_to_edge.py),
which measures the same thing independently of KiCad and agrees with it on
this board. The two differ by 0.025 mm — half the Edge.Cuts stroke width —
because KiCad measures to the near edge of the outline graphic and the script
measures to its centreline. KiCad's reading is the conservative one.

## One-sided PTH annulus for Potentiometer

`J4` uses `shared:Pot_Wire_Pads`, whose pads are a KiCad 10 per-layer padstack:
a 0.1 mm token pad on F.Cu inside a 0.9 mm drill — no annulus at all — but real
**1.8 mm lands** on `In1.Cu`, `In2.Cu` and `B.Cu`. The rule exempts that
footprint from the 0.125 mm `PTH annular ring` minimum.

Two consequences are tracked in `TODO.md` §10.2: zero annular ring on F.Cu is
below PCBWay's stated 0.15 mm minimum [REF-FAB-001], and KiCad's Specctra
exporter serialises only the F.Cu shape, concludes the pad has no copper, and
emits no pin at all — so FreeRouting cannot see J4 and routes across its
back-side lands.

## References

Catalogued in [`REFERENCES.md`](../../../REFERENCES.md):

- **REF-FAB-001** — PCBWay capability tables and Help Center design rulings.
- **REF-STD-007** — IPC-2221B, *Generic Standard on Printed Board Design*.
