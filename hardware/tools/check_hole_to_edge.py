#!/usr/bin/env python3
"""Check hole-to-board-edge spacing on a KiCad PCB.

Why this exists
---------------
The fabricator (PCBWay) requires at least 0.50 mm between any hole -- plated or
unplated -- and the board outline [REF-FAB-001].  That is stricter than the
0.30 mm copper-to-outline rule carried in ``*.kicad_dru``, and on a board this
small it is the constraint that actually decides how close a via may sit to the
edge.

KiCad *can* express it, as ``physical_hole_clearance`` conditioned on
``B.Layer == 'Edge.Cuts'``, and ``osc-sg90-v006.kicad_dru`` carries that rule.
This script is the independent cross-check on it, for two reasons.  First, a
``.kicad_dru`` fails silently: a single ``;`` anywhere in the file stops every
rule firing with no diagnostic, so a clean DRC run is not by itself evidence
that the rule ran.  Second, this runs anywhere -- it parses the board file
directly and needs no ``pcbnew`` binding or KiCad install, so it works as a CI
gate.

The two agree on this board.  They differ by 0.025 mm, half the Edge.Cuts
stroke width, because KiCad measures to the near edge of the outline graphic
and this measures to its centreline; KiCad's reading is the conservative one.

Usage
-----
    python3 check_hole_to_edge.py board.kicad_pcb [--min-mm 0.5]

Exits non-zero if any hole is closer to the outline than the minimum.  Parses
the board file directly, so it needs no ``pcbnew`` binding and runs under any
Python 3; the board file format is read only for ``Edge.Cuts`` graphics, via
positions and drills, and footprint pad positions and drills.

References
----------
[REF-FAB-001] PCBWay, "the spacing requirement from hole to the edge of board",
    PCBWay Help Center.  "The distance from hole (no matter PTH hole or
    unplated hole) to outline of board should be >= 0.5mm."
"""

import argparse
import itertools
import math
import re
import sys

# ---------------------------------------------------------------------------
# Minimal s-expression reader
# ---------------------------------------------------------------------------

TOKEN_RE = re.compile(r'"(?:[^"\\]|\\.)*"|[()]|[^\s()]+')


def parse_sexpr(text):
    """Parse a KiCad s-expression document into nested lists of strings."""
    stack = [[]]
    for tok in TOKEN_RE.findall(text):
        if tok == "(":
            stack.append([])
        elif tok == ")":
            done = stack.pop()
            if not stack:
                raise ValueError("unbalanced closing paren")
            stack[-1].append(done)
        elif tok.startswith('"'):
            stack[-1].append(tok[1:-1].replace('\\"', '"').replace("\\\\", "\\"))
        else:
            stack[-1].append(tok)
    if len(stack) != 1:
        raise ValueError("unbalanced opening paren")
    return stack[0]


def find_all(node, key):
    """Yield every direct or nested child list whose head token is ``key``."""
    if isinstance(node, list):
        if node and node[0] == key:
            yield node
        for child in node:
            yield from find_all(child, key)


def child(node, key):
    """Return the first direct child list with head ``key``, else None."""
    for item in node:
        if isinstance(item, list) and item and item[0] == key:
            return item
    return None


def numbers(node, key, count):
    """Return ``count`` floats from the direct child list ``key``."""
    found = child(node, key)
    if found is None:
        return None
    return [float(v) for v in found[1:1 + count]]


# ---------------------------------------------------------------------------
# Geometry
# ---------------------------------------------------------------------------

def layers_of(node):
    """Return the set of layer names named by this node's layer/layers child."""
    out = set()
    for key in ("layer", "layers"):
        found = child(node, key)
        if found:
            out.update(found[1:])
    return out


def arc_points(start, mid, end, segments=64):
    """Approximate a three-point arc as a polyline."""
    (x1, y1), (x2, y2), (x3, y3) = start, mid, end
    d = 2 * (x1 * (y2 - y3) + x2 * (y3 - y1) + x3 * (y1 - y2))
    if abs(d) < 1e-12:                       # collinear: treat as a chord
        return [start, end]
    ux = ((x1 ** 2 + y1 ** 2) * (y2 - y3) + (x2 ** 2 + y2 ** 2) * (y3 - y1)
          + (x3 ** 2 + y3 ** 2) * (y1 - y2)) / d
    uy = ((x1 ** 2 + y1 ** 2) * (x3 - x2) + (x2 ** 2 + y2 ** 2) * (x1 - x3)
          + (x3 ** 2 + y3 ** 2) * (x2 - x1)) / d
    r = math.hypot(x1 - ux, y1 - uy)
    a1 = math.atan2(y1 - uy, x1 - ux)
    a2 = math.atan2(y2 - uy, x2 - ux)
    a3 = math.atan2(y3 - uy, x3 - ux)

    # Walk start -> mid -> end the short way round at each step.
    def sweep(a, b):
        delta = (b - a) % (2 * math.pi)
        return delta - 2 * math.pi if delta > math.pi else delta

    total = sweep(a1, a2) + sweep(a2, a3)
    return [(ux + r * math.cos(a1 + total * i / segments),
             uy + r * math.sin(a1 + total * i / segments))
            for i in range(segments + 1)]


def edge_segments(board):
    """Return the board outline as a list of ((x1, y1), (x2, y2)) segments."""
    segs = []
    for kind in ("gr_line", "gr_arc", "gr_circle", "gr_rect", "gr_poly"):
        for node in find_all(board, kind):
            if "Edge.Cuts" not in layers_of(node):
                continue
            if kind == "gr_line":
                a, b = numbers(node, "start", 2), numbers(node, "end", 2)
                segs.append((tuple(a), tuple(b)))
            elif kind == "gr_arc":
                pts = arc_points(tuple(numbers(node, "start", 2)),
                                 tuple(numbers(node, "mid", 2)),
                                 tuple(numbers(node, "end", 2)))
                segs += list(itertools.pairwise(pts))
            elif kind == "gr_circle":
                cx, cy = numbers(node, "center", 2)
                ex, ey = numbers(node, "end", 2)
                r = math.hypot(ex - cx, ey - cy)
                pts = [(cx + r * math.cos(2 * math.pi * i / 128),
                        cy + r * math.sin(2 * math.pi * i / 128))
                       for i in range(129)]
                segs += list(itertools.pairwise(pts))
            elif kind == "gr_rect":
                x1, y1 = numbers(node, "start", 2)
                x2, y2 = numbers(node, "end", 2)
                corners = [(x1, y1), (x2, y1), (x2, y2), (x1, y2), (x1, y1)]
                segs += list(itertools.pairwise(corners))
            else:                                       # gr_poly
                pts = [tuple(float(v) for v in p[1:3])
                       for p in find_all(child(node, "pts"), "xy")]
                if pts:
                    pts.append(pts[0])
                    segs += list(itertools.pairwise(pts))
    return segs


def point_to_segment(p, a, b):
    """Shortest distance from point ``p`` to the segment ``a``-``b``."""
    (px, py), (ax, ay), (bx, by) = p, a, b
    dx, dy = bx - ax, by - ay
    denom = dx * dx + dy * dy
    t = 0.0 if denom == 0 else max(0.0, min(1.0, ((px - ax) * dx + (py - ay) * dy) / denom))
    return math.hypot(px - (ax + t * dx), py - (ay + t * dy))


def rotate(x, y, degrees):
    """Rotate (x, y) by ``degrees``, using KiCad's footprint sign convention."""
    rad = math.radians(-degrees)
    return (x * math.cos(rad) - y * math.sin(rad),
            x * math.sin(rad) + y * math.cos(rad))


def holes(board):
    """Yield (label, centre_x, centre_y, hole_radius) for every drilled hole."""
    for via in find_all(board, "via"):
        at = numbers(via, "at", 2)
        drill = numbers(via, "drill", 1)
        if at and drill:
            net = child(via, "net")
            label = f"Via [{net[1] if net and len(net) > 1 else '<no net>'}]"
            yield label, at[0], at[1], drill[0] / 2.0

    for fp in find_all(board, "footprint"):
        origin = numbers(fp, "at", 3) or numbers(fp, "at", 2)
        if not origin:
            continue
        ox, oy = origin[0], origin[1]
        rot = origin[2] if len(origin) > 2 else 0.0
        ref = "?"
        for prop in find_all(fp, "property"):
            if len(prop) > 2 and prop[1] == "Reference":
                ref = prop[2]
                break
        for pad in find_all(fp, "pad"):
            drill = child(pad, "drill")
            if drill is None:
                continue
            # (drill [oval] size [size_y]) -- take the largest dimension.
            dims = [float(v) for v in drill[1:] if re.fullmatch(r"-?[\d.]+", str(v))]
            if not dims:
                continue
            at = numbers(pad, "at", 2)
            if at is None:
                continue
            rx, ry = rotate(at[0], at[1], rot)
            name = pad[1] if len(pad) > 1 else "?"
            yield f"Pad {name} of {ref}", ox + rx, oy + ry, max(dims) / 2.0


# ---------------------------------------------------------------------------

def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("board", help="path to the .kicad_pcb file")
    ap.add_argument("--min-mm", type=float, default=0.5,
                    help="minimum hole-to-outline spacing in mm (default: 0.5)")
    args = ap.parse_args()

    with open(args.board) as fh:
        board = parse_sexpr(fh.read())
    segs = edge_segments(board)
    if not segs:
        sys.exit("no Edge.Cuts geometry found; cannot check hole-to-edge spacing")

    checked = 0
    failures = []
    for label, cx, cy, radius in holes(board):
        checked += 1
        gap = min(point_to_segment((cx, cy), a, b) for a, b in segs) - radius
        if gap < args.min_mm:
            failures.append((gap, label, cx, cy))

    print(f"{checked} holes checked against {len(segs)} outline segments, "
          f"minimum {args.min_mm:.3f} mm")
    for gap, label, cx, cy in sorted(failures):
        print(f"  FAIL  {gap:7.3f} mm  {label}  at ({cx:.3f}, {cy:.3f})")
    if failures:
        print(f"{len(failures)} hole(s) too close to the board outline")
        return 1
    print("all holes clear")
    return 0


if __name__ == "__main__":
    sys.exit(main())
