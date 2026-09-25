#!/usr/bin/env python3
"""Entry points per world for the occluder's explorable-area trim (NA27).

Reads the resource seeds (the source of truth, db/resources/) and writes one
TSV row per place a player or NPC can actually be put:

    world<TAB>source<TAB>x<TAB>y<TAB>z

`world` is the `data/spaces/<world>.nav` key (the world name lower-cased,
spaces as underscores), or `*` for a point whose world the seed does not
name (a chain's `move_waypoint`); the extractor keeps a `*` point for every
world whose navmesh it lands on.

Sources: spawnlist rows, respawners, stargates (the gate and its arrival
point), ring transport regions, and content-chain `cross_world_teleport`
and `move_waypoint` targets. The client maps' own PlayerStart / SGWStargate
/ SGWTeleporter actors are added by `occluder_extract` itself.

Usage: tools/occluder_entry_points.py [--seeds db/resources] > entry_points.tsv
"""

import argparse
import json
import pathlib
import re
import sys

INSERT = re.compile(r"INSERT INTO\s+(\w+)\s*\(([^)]*)\)\s*VALUES\s*", re.S)


def split_values(text, start):
    """Parse one or more `(v, v, ...)` tuples from text[start:]; stop at ';'."""
    rows, i, n = [], start, len(text)
    while i < n:
        c = text[i]
        if c == ";":
            break
        if c == "-" and text.startswith("--", i):
            i = text.index("\n", i) if "\n" in text[i:] else n
            continue
        if c != "(":
            i += 1
            continue
        i += 1
        vals, cur, depth, quoted = [], [], 0, False
        while i < n:
            c = text[i]
            if quoted:
                if c == "'" and text.startswith("''", i):
                    cur.append("'")
                    i += 2
                    continue
                if c == "'":
                    quoted = False
                else:
                    cur.append(c)
                i += 1
                continue
            if c == "'":
                quoted = True
            elif c in "({[":
                depth += 1
                cur.append(c)
            elif c in ")}]" and depth > 0:
                depth -= 1
                cur.append(c)
            elif c == ")":
                vals.append("".join(cur).strip())
                i += 1
                break
            elif c == ",":
                vals.append("".join(cur).strip())
                cur = []
            else:
                cur.append(c)
            i += 1
        rows.append(vals)
    return rows


def inserts(path):
    text = path.read_text(encoding="utf-8")
    for m in INSERT.finditer(text):
        cols = [c.strip() for c in m.group(2).split(",")]
        for vals in split_values(text, m.end()):
            if len(vals) == len(cols):
                yield m.group(1), dict(zip(cols, vals))


def num(v):
    try:
        return float(v)
    except (TypeError, ValueError):
        return None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--seeds", default="db/resources")
    args = ap.parse_args()
    root = pathlib.Path(args.seeds)
    worlds = {}
    for _, r in inserts(root / "Worlds/Seed/worlds.sql"):
        worlds[r["world_id"]] = r["world"].lower().replace(" ", "_")
    by_name = {v: v for v in worlds.values()}
    out = []

    def emit(world, source, x, y, z):
        if None in (x, y, z) or (x == 0 and y == 0 and z == 0):
            return
        out.append((world, source, x, y, z))

    for _, r in inserts(root / "Worlds/Seed/spawnlist.sql"):
        w = worlds.get(r.get("world_id"))
        if w:
            emit(w, "spawnlist:" + r.get("tag", ""), num(r["x"]), num(r["y"]), num(r["z"]))
    for _, r in inserts(root / "Worlds/Seed/respawners.sql"):
        w = worlds.get(r.get("world_id"))
        if w:
            emit(w, "respawner:" + r.get("name", ""), num(r["pos_x"]), num(r["pos_y"]), num(r["pos_z"]))
    for _, r in inserts(root / "Worlds/Seed/stargates.sql"):
        w = worlds.get(r.get("world_id"))
        if w:
            emit(w, "stargate:" + r.get("name", ""), num(r["x_pos"]), num(r["y_pos"]), num(r["z_pos"]))
            if r.get("arrival_x") not in (None, "NULL"):
                emit(w, "stargate_arrival:" + r.get("name", ""), num(r["arrival_x"]), num(r["arrival_y"]), num(r["arrival_z"]))
    for _, r in inserts(root / "Worlds/Seed/ring_transport_regions.sql"):
        w = worlds.get(r.get("world_id"))
        if w:
            emit(w, "ring:" + r.get("tag", ""), num(r["x"]), num(r["y"]), num(r["z"]))
    for f in sorted((root / "Content/Seed").glob("*.sql")):
        for table, r in inserts(f):
            if table != "content_actions":
                continue
            kind = r.get("action_type")
            try:
                params = json.loads(r.get("params") or "{}")
            except json.JSONDecodeError:
                continue
            if kind == "cross_world_teleport":
                w = by_name.get((r.get("target_key") or "").lower().replace(" ", "_"))
                if w:
                    emit(w, "chain_teleport:" + f.stem, num(params.get("x")), num(params.get("y")), num(params.get("z")))
            elif kind == "move_waypoint" and "destination" in params:
                xyz = [num(v) for v in str(params["destination"]).split(",")]
                if len(xyz) == 3:
                    emit("*", "chain_move:" + f.stem, *xyz)
    w = sys.stdout
    w.write("world\tsource\tx\ty\tz\n")
    for world, source, x, y, z in out:
        w.write(f"{world}\t{source.replace(chr(9), ' ')}\t{x}\t{y}\t{z}\n")
    counts = {}
    for row in out:
        counts[row[0]] = counts.get(row[0], 0) + 1
    print(f"{len(out)} entry points: " + ", ".join(f"{k} {v}" for k, v in sorted(counts.items())), file=sys.stderr)


if __name__ == "__main__":
    main()
