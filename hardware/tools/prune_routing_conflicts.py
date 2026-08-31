#!/usr/bin/env python3
"""Delete auto-routed copper that KiCad's own DRC flags against a footprint.

Why this is needed
------------------
``J4`` (``shared:Pot_Wire_Pads``) uses a KiCad 10 per-layer padstack: a 0.1 mm
token pad on F.Cu -- the "one-sided annulus" the board's .kicad_dru carves an
explicit exception for -- but real **1.8 mm lands** on In1.Cu, In2.Cu and B.Cu.
KiCad's Specctra exporter serialises only the F.Cu shape, concludes the pad has
no copper, and emits no ``(pin ...)`` at all.  FreeRouting therefore never sees
the back-side lands and routes straight across them.  Every routing-induced DRC
violation on this board is one of these, and no other footprint contributes any.

Rather than re-deriving the geometry, this prunes exactly what KiCad reports:
it reads a DRC JSON, finds violations that involve the named footprint, and
deletes the *track or via* side of each pair by UUID.  The affected connections
are left unrouted for hand routing, which is the right outcome -- a track
crossing a plated land is a short, and there is no correct automatic placement
for it.

With ``--dangling`` it instead removes the stubs KiCad reports as
``track_dangling`` / ``via_dangling`` -- the leftovers a prune pass creates,
which carry no connection and are only a DRC and manufacturing nuisance.

Usage
-----
    AppRun python3.11 prune_routing_conflicts.py board.kicad_pcb drc.json [J4]
    AppRun python3.11 prune_routing_conflicts.py board.kicad_pcb drc.json --dangling

Run under the KiCad 10 AppImage's bundled Python; the system pcbnew is 9.0.2
and cannot load this board.  Re-run DRC between passes: removing one item can
expose another.
"""
import json
import sys

import pcbnew

TRACK_WORDS = ("Track", "Via", "Segment")
# Violation types that mean "this copper must not be here", as opposed to
# placement problems that pruning cannot fix.
ROUTING_TYPES = {"shorting_items", "clearance", "hole_clearance",
                 "solder_mask_bridge", "copper_edge_clearance"}


def flagged_uuids(report, reference):
    """UUIDs of track/via items in violations that involve ``reference``."""
    doomed = set()
    for violation in report.get("violations", []):
        if violation.get("type") not in ROUTING_TYPES:
            continue
        items = violation.get("items", [])
        if not any(f"of {reference}" in i.get("description", "") for i in items):
            continue
        for item in items:
            text = item.get("description", "")
            if text.startswith(TRACK_WORDS) and item.get("uuid"):
                doomed.add(item["uuid"])
    return doomed


def dangling_uuids(report):
    """UUIDs of every stub KiCad reports as a dangling track or via."""
    return {item["uuid"]
            for violation in report.get("violations", [])
            if violation.get("type") in ("track_dangling", "via_dangling")
            for item in violation.get("items", [])
            if item.get("uuid")}


def main():
    board_path, drc_path = sys.argv[1], sys.argv[2]
    selector = sys.argv[3] if len(sys.argv) > 3 else "J4"

    with open(drc_path) as fh:
        report = json.load(fh)
    if selector == "--dangling":
        doomed = dangling_uuids(report)
        reference = "dangling stubs"
    else:
        doomed = flagged_uuids(report, selector)
        reference = selector

    board = pcbnew.LoadBoard(board_path)
    removed = 0
    vias = 0
    for track in list(board.GetTracks()):
        if str(track.m_Uuid.AsString()) in doomed:
            if track.Type() == pcbnew.PCB_VIA_T:
                vias += 1
            board.Remove(track)
            removed += 1

    print(f"{reference}: {len(doomed)} flagged copper item(s); "
          f"removed {removed - vias} segments and {vias} vias")
    if removed < len(doomed):
        print(f"  warning: {len(doomed) - removed} flagged uuid(s) not found on the board")

    pcbnew.SaveBoard(board_path, board)
    print(f"saved {board_path}")


if __name__ == "__main__":
    main()
