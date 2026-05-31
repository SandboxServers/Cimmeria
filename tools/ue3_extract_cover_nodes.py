#!/usr/bin/env python3
"""
SGW Cover Node Extractor
========================
Parses `covernodes_*.pak` files (the binary cover-node prefab archives
shipped with the SGW client cache) and emits PostgreSQL seed SQL plus a
JSON manifest describing what was found and any merge conflicts.

The pak format was reverse-engineered for issue #209. Each file is a ZIP
archive; per entry:

    version       u32
    flag          u32              ← number of record sets (1 or 2)
    [per set]:
      count       u32              (first set's count + wstrlen are in
                                    the entry header above)
      wstr_len    u32              UTF-16 code units, no NUL
      author      UTF-16LE × wstrlen
      record × count [22 bytes each]:
        pos.x     f32              BW meters
        pos.y     f32
        pos.z     f32
        orient    f32              radians
        height    u8               ECoverHeight: 0=Low, 1=Mid, 2=High, 3=LOS
        quality   u8               ECoverQuality: 0=Good, 1=Better, 2=Best, 3=None
        tail      u8[4]            interpretation TBD; preserved raw

Two `.pak` files exist in the SGW QA install — `covernodes_nikols.pak` is
treated as canonical (larger node set), `covernodes_sdeiter.pak` is unioned
in for chunks the canonical doesn't carry. On byte conflict for a shared
chunk, the canonical wins; the conflict is recorded in the manifest.

`flag=2` entries carry two record sets — usually the same nodes with minor
editor-tracked deltas. The first set (typically "SDeiter"-stamped) is the
canonical runtime data; the second is preserved in the manifest under
`variants` for future reference but not loaded into the runtime tables.

Usage
-----
    python tools/ue3_extract_cover_nodes.py \\
        --nikols-pak "C:/.../SGWGame/Cache/covernodes_nikols.pak" \\
        --sdeiter-pak "C:/.../SGWGame/Cache/covernodes_sdeiter.pak" \\
        --out-dir db/resources/AI/Seed

Outputs (in `--out-dir`):
    cover_sets.sql              — INSERT INTOs for resources.cover_sets
    cover_nodes.sql             — INSERT INTOs for resources.cover_nodes
    cover_nodes_manifest.json   — provenance + conflict log + flag=2 variants
"""

import argparse
import json
import struct
import sys
import zipfile
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path

HEIGHT_NAMES = ["HEIGHT_Low", "HEIGHT_Mid", "HEIGHT_High", "HEIGHT_LOS"]
QUALITY_NAMES = ["QUALITY_Good", "QUALITY_Better", "QUALITY_Best", "QUALITY_None"]

# Pak record stride. Empirically confirmed on 1312/1331 covernodes_nikols
# entries (98.6%). The 19 remaining edge cases have an empty second set —
# tolerant parsing covers them.
RECORD_STRIDE = 22


@dataclass
class CoverRecord:
    """A single cover node — 22 bytes on disk."""

    pos_x: float
    pos_y: float
    pos_z: float
    orient: float
    height: int  # 0..3
    quality: int  # 0..3
    tail: bytes  # 4 bytes, semantics unconfirmed

    @property
    def height_name(self) -> str:
        return HEIGHT_NAMES[self.height] if 0 <= self.height < 4 else f"INVALID_{self.height}"

    @property
    def quality_name(self) -> str:
        return QUALITY_NAMES[self.quality] if 0 <= self.quality < 4 else f"INVALID_{self.quality}"

    def to_dict(self) -> dict:
        return {
            "pos": [self.pos_x, self.pos_y, self.pos_z],
            "orient": self.orient,
            "height": self.height_name,
            "quality": self.quality_name,
            "tail_hex": self.tail.hex(),
        }


@dataclass
class CoverSet:
    """One ZIP entry's worth of cover data — may carry multiple record sets."""

    chunk_name: str
    version: int
    flag: int
    sets: list = field(default_factory=list)  # list[ tuple[author: str, records: list[CoverRecord]] ]
    src_pak: str = ""

    @property
    def primary_author(self) -> str:
        return self.sets[0][0] if self.sets else ""

    @property
    def primary_records(self) -> list:
        return self.sets[0][1] if self.sets else []

    @property
    def has_variant(self) -> bool:
        return len(self.sets) > 1


def _read_set(data: bytes, cursor: int, count: int) -> tuple:
    """Read `count` records of 22 bytes from `data` starting at `cursor`."""
    records = []
    for _ in range(count):
        if cursor + RECORD_STRIDE > len(data):
            return records, cursor, False
        x, y, z, o = struct.unpack_from("<ffff", data, cursor)
        height = data[cursor + 16]
        quality = data[cursor + 17]
        tail = bytes(data[cursor + 18 : cursor + 22])
        records.append(CoverRecord(x, y, z, o, height, quality, tail))
        cursor += RECORD_STRIDE
    return records, cursor, True


def parse_entry(name: str, data: bytes, src_pak: str) -> CoverSet:
    """Parse a single ZIP entry's body. Returns the CoverSet (possibly with empty sets)."""
    if len(data) < 16:
        raise ValueError(f"{name}: entry too short ({len(data)} bytes; need ≥16 for header)")
    version, flag, count, wlen = struct.unpack_from("<IIII", data, 0)
    if wlen * 2 + 16 > len(data):
        raise ValueError(f"{name}: wstring length {wlen} overflows entry")
    author1 = data[16 : 16 + wlen * 2].decode("utf-16-le", errors="replace")
    cs = CoverSet(chunk_name=name, version=version, flag=flag, src_pak=src_pak)

    cursor = 16 + wlen * 2
    current_count = count
    current_author = author1

    while True:
        records, cursor, ok = _read_set(data, cursor, current_count)
        if not ok:
            # Short set — bail. Caller treats as edge case.
            cs.sets.append((current_author, records))
            return cs
        cs.sets.append((current_author, records))

        # Is there another set header?
        if cursor + 8 > len(data):
            break
        c2, wl2 = struct.unpack_from("<II", data, cursor)
        # Sanity: bogus values mean we're past the end of valid headers.
        # An empty second set (count=0) terminates cleanly here too.
        if c2 <= 0 or c2 > 200 or wl2 <= 0 or wl2 > 64:
            break
        if cursor + 8 + wl2 * 2 + c2 * RECORD_STRIDE > len(data):
            break
        current_count = c2
        current_author = data[cursor + 8 : cursor + 8 + wl2 * 2].decode(
            "utf-16-le", errors="replace"
        )
        cursor += 8 + wl2 * 2

    return cs


def parse_pak(pak_path: Path) -> tuple:
    """Parse a `covernodes_*.pak` ZIP archive. Returns (sets dict by chunk_name, edge cases)."""
    if not pak_path.is_file():
        raise FileNotFoundError(f"pak not found: {pak_path}")

    sets_by_chunk = {}
    edge_cases = []
    src = pak_path.name

    with zipfile.ZipFile(pak_path) as z:
        for name in z.namelist():
            if name == "MetaData":
                # 8-byte ZIP-meta stub, ignore.
                continue
            data = z.read(name)
            try:
                cs = parse_entry(name, data, src_pak=src)
            except (ValueError, struct.error) as e:
                edge_cases.append({"chunk": name, "src": src, "error": str(e)})
                continue
            sets_by_chunk[name] = cs

    return sets_by_chunk, edge_cases


def union(primary: dict, alternate: dict, primary_src: str, alternate_src: str) -> tuple:
    """Union two pak sources. Primary wins on conflict; record provenance."""
    merged = {}
    manifest_conflicts = []
    manifest_alternate_only = []

    # Carry every primary entry through.
    for name, cs in primary.items():
        merged[name] = cs

    # Walk alternate.
    for name, alt_cs in alternate.items():
        if name not in merged:
            merged[name] = alt_cs
            manifest_alternate_only.append({"chunk": name, "src": alternate_src})
            continue
        # Both have it — compare primary records.
        prim_cs = merged[name]
        if _records_match(prim_cs.primary_records, alt_cs.primary_records):
            continue
        manifest_conflicts.append(
            {
                "chunk": name,
                "primary_src": primary_src,
                "alternate_src": alternate_src,
                "primary_node_count": len(prim_cs.primary_records),
                "alternate_node_count": len(alt_cs.primary_records),
                "decision": "kept primary",
            }
        )

    return merged, manifest_conflicts, manifest_alternate_only


def _records_match(a: list, b: list, eps: float = 1e-4) -> bool:
    """Byte-ish equality on records (with small epsilon for float roundtrip safety)."""
    if len(a) != len(b):
        return False
    for ra, rb in zip(a, b):
        if (
            abs(ra.pos_x - rb.pos_x) > eps
            or abs(ra.pos_y - rb.pos_y) > eps
            or abs(ra.pos_z - rb.pos_z) > eps
            or abs(ra.orient - rb.orient) > eps
            or ra.height != rb.height
            or ra.quality != rb.quality
            or ra.tail != rb.tail
        ):
            return False
    return True


def emit_sql(merged: dict, out_dir: Path) -> tuple:
    """Emit cover_sets.sql + cover_nodes.sql. Returns (sets_count, nodes_count)."""
    out_dir.mkdir(parents=True, exist_ok=True)
    sets_path = out_dir / "cover_sets.sql"
    nodes_path = out_dir / "cover_nodes.sql"

    # Stable ordering: alphabetical by chunk_name → reproducible diffs.
    chunks_sorted = sorted(merged.items(), key=lambda kv: kv[0])

    # Assign sequential chunk_ids starting at 1. The pak format has no
    # intrinsic chunk_id; the integer here is purely a database FK
    # convenience and is regenerated each time the extractor runs.
    chunk_ids = {name: idx + 1 for idx, (name, _) in enumerate(chunks_sorted)}

    sets_count = 0
    nodes_count = 0

    header = (
        "-- Auto-generated by tools/ue3_extract_cover_nodes.py — DO NOT EDIT BY HAND.\n"
        f"-- Source: union of covernodes_nikols.pak + covernodes_sdeiter.pak\n"
        f"-- Generated: {datetime.now(timezone.utc).strftime('%Y-%m-%d %H:%M:%S UTC')}\n"
        "-- Issue: https://github.com/SandboxServers/Cimmeria/issues/209\n"
        "-- Re-run via: python tools/ue3_extract_cover_nodes.py --help\n\n"
    )

    with sets_path.open("w", encoding="utf-8", newline="\n") as f:
        f.write(header)
        f.write(
            "INSERT INTO resources.cover_sets "
            "(chunk_id, chunk_name, primary_author, has_variant, src_pak) VALUES\n"
        )
        rows = []
        for name, cs in chunks_sorted:
            cid = chunk_ids[name]
            author_esc = cs.primary_author.replace("'", "''")
            chunk_esc = name.replace("'", "''")
            rows.append(
                f"  ({cid}, '{chunk_esc}', '{author_esc}', "
                f"{'true' if cs.has_variant else 'false'}, '{cs.src_pak}')"
            )
            sets_count += 1
        f.write(",\n".join(rows))
        f.write(";\n")

    with nodes_path.open("w", encoding="utf-8", newline="\n") as f:
        f.write(header)
        f.write(
            "INSERT INTO resources.cover_nodes "
            "(chunk_id, node_id, pos_x, pos_y, pos_z, orient, height, quality, tail) VALUES\n"
        )
        rows = []
        for name, cs in chunks_sorted:
            cid = chunk_ids[name]
            for nid, rec in enumerate(cs.primary_records):
                rows.append(
                    f"  ({cid}, {nid}, "
                    f"{rec.pos_x:.6f}, {rec.pos_y:.6f}, {rec.pos_z:.6f}, "
                    f"{rec.orient:.6f}, "
                    f"'{rec.height_name}', '{rec.quality_name}', "
                    f"'\\x{rec.tail.hex()}'::bytea)"
                )
                nodes_count += 1
        f.write(",\n".join(rows))
        f.write(";\n")

    return sets_count, nodes_count


def emit_manifest(
    merged: dict,
    conflicts: list,
    alternate_only: list,
    edge_cases_primary: list,
    edge_cases_alternate: list,
    out_dir: Path,
    primary_src: str,
    alternate_src: str,
) -> None:
    """Emit the JSON manifest sidecar."""
    out_dir.mkdir(parents=True, exist_ok=True)
    manifest_path = out_dir / "cover_nodes_manifest.json"

    flag2_variants = []
    for name, cs in sorted(merged.items()):
        if not cs.has_variant:
            continue
        flag2_variants.append(
            {
                "chunk": name,
                "set0_author": cs.sets[0][0],
                "set0_count": len(cs.sets[0][1]),
                "set1_author": cs.sets[1][0] if len(cs.sets) > 1 else None,
                "set1_count": len(cs.sets[1][1]) if len(cs.sets) > 1 else 0,
                "sets_match": _records_match(cs.sets[0][1], cs.sets[1][1])
                if len(cs.sets) > 1
                else None,
            }
        )

    manifest = {
        "generated_utc": datetime.now(timezone.utc).isoformat(),
        "issue": "https://github.com/SandboxServers/Cimmeria/issues/209",
        "primary_pak": primary_src,
        "alternate_pak": alternate_src,
        "total_chunks": len(merged),
        "total_nodes": sum(len(cs.primary_records) for cs in merged.values()),
        "flag1_chunks": sum(1 for cs in merged.values() if cs.flag == 1),
        "flag2_chunks": sum(1 for cs in merged.values() if cs.flag == 2),
        "alternate_only_chunks": alternate_only,
        "byte_conflicts_resolved_to_primary": conflicts,
        "flag2_variants": flag2_variants,
        "edge_cases_primary": edge_cases_primary,
        "edge_cases_alternate": edge_cases_alternate,
    }

    with manifest_path.open("w", encoding="utf-8", newline="\n") as f:
        json.dump(manifest, f, indent=2, sort_keys=True)
        f.write("\n")


def main(argv: list = None) -> int:
    ap = argparse.ArgumentParser(
        description="Extract SGW cover-node data from covernodes_*.pak archives.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )
    ap.add_argument(
        "--nikols-pak",
        required=True,
        help="Path to covernodes_nikols.pak (canonical / preferred on conflict).",
    )
    ap.add_argument(
        "--sdeiter-pak",
        required=True,
        help="Path to covernodes_sdeiter.pak (alternate, unioned in).",
    )
    ap.add_argument(
        "--out-dir",
        required=True,
        help="Directory to write cover_sets.sql + cover_nodes.sql + manifest.",
    )
    ap.add_argument(
        "--verify-only",
        action="store_true",
        help="Parse + print stats, but do not write any files.",
    )
    args = ap.parse_args(argv)

    nikols = Path(args.nikols_pak)
    sdeiter = Path(args.sdeiter_pak)
    out_dir = Path(args.out_dir)

    print(f"Parsing {nikols.name} ...", file=sys.stderr)
    nikols_sets, nikols_edge = parse_pak(nikols)
    print(
        f"  {len(nikols_sets)} chunks, {sum(len(cs.primary_records) for cs in nikols_sets.values())} nodes",
        file=sys.stderr,
    )
    if nikols_edge:
        print(f"  edge cases: {len(nikols_edge)}", file=sys.stderr)

    print(f"Parsing {sdeiter.name} ...", file=sys.stderr)
    sdeiter_sets, sdeiter_edge = parse_pak(sdeiter)
    print(
        f"  {len(sdeiter_sets)} chunks, {sum(len(cs.primary_records) for cs in sdeiter_sets.values())} nodes",
        file=sys.stderr,
    )
    if sdeiter_edge:
        print(f"  edge cases: {len(sdeiter_edge)}", file=sys.stderr)

    merged, conflicts, alt_only = union(
        nikols_sets, sdeiter_sets, primary_src=nikols.name, alternate_src=sdeiter.name
    )
    total_nodes = sum(len(cs.primary_records) for cs in merged.values())
    print(
        f"Merged: {len(merged)} chunks, {total_nodes} nodes "
        f"({len(conflicts)} byte conflicts, {len(alt_only)} alternate-only).",
        file=sys.stderr,
    )

    if args.verify_only:
        return 0

    sets_count, nodes_count = emit_sql(merged, out_dir)
    emit_manifest(
        merged,
        conflicts,
        alt_only,
        nikols_edge,
        sdeiter_edge,
        out_dir,
        primary_src=nikols.name,
        alternate_src=sdeiter.name,
    )

    print(
        f"Wrote {sets_count} sets + {nodes_count} nodes + manifest to {out_dir}.",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
