"""Inventory Ancient-named cooked assets and placed meshes across an SGW client.

Only package metadata and actor references are written. No cooked asset bytes
are copied into the output. Requires the locally built upk_info executable.
"""

from __future__ import annotations

import argparse
import collections
import csv
import json
import re
import subprocess
from pathlib import Path


MARKERS = (
    b"AN-", b"ANM-", b"AGN-", b"ATL-", b"Ancient", b"Atlantis",
    b"Pegasus", b"Laro", b"ancient", b"atlantis", b"pegasus",
)
ASSET_CLASSES = {
    "StaticMesh", "SkeletalMesh", "ParticleSystem", "SGWPrebuild", "Prefab",
    "Material", "MaterialInstanceConstant", "Texture2D", "AnimSet",
}
EXPORT = re.compile(r"^\s*\[\s*(\d+)\] \(ref\s+\d+\) (\S+)\s+(\S+)")
ACTOR = re.compile(r"^(\d+)\t[^\t]*\t([^\t]*)\t([^\t\r\n]+)")
ANCIENT = re.compile(r"^(?:AN-|ANM-|AGN-|ATL-)|ancient|atlantis|pegasus|laro", re.I)


def has_marker(path: Path) -> bool:
    data = path.read_bytes()
    return any(marker in data for marker in MARKERS)


def inspect(exe: Path, path: Path, flag: str) -> str:
    result = subprocess.run(
        [str(exe), str(path), flag, "100000"] if flag == "--exports"
        else [str(exe), str(path), flag],
        capture_output=True, text=True, errors="replace", check=False,
    )
    if result.returncode:
        raise RuntimeError(f"upk_info failed on {path}: {result.stderr[:300]}")
    return result.stdout


def write_tsv(path: Path, columns: list[str], rows: list[dict[str, object]]) -> None:
    with path.open("w", newline="", encoding="utf-8") as f:
        writer = csv.DictWriter(f, fieldnames=columns, delimiter="\t")
        writer.writeheader()
        writer.writerows(rows)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("cooked_pc", type=Path)
    parser.add_argument("upk_info", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)

    packages = sorted((args.cooked_pc / "Packages").rglob("*.upk"))
    chunks = sorted((args.cooked_pc / "Maps").rglob("*.umap"))
    assets: list[dict[str, object]] = []
    placements: collections.Counter[tuple[str, str, str]] = collections.Counter()
    examples: dict[tuple[str, str, str], str] = {}
    inspected_packages = inspected_chunks = 0

    for path in packages:
        if not has_marker(path):
            continue
        inspected_packages += 1
        for line in inspect(args.upk_info, path, "--exports").splitlines():
            match = EXPORT.match(line)
            if not match:
                continue
            index, cls, name = match.groups()
            if cls in ASSET_CLASSES and ANCIENT.search(name.split(".")[-1]):
                assets.append({
                    "package": path.relative_to(args.cooked_pc).as_posix(),
                    "export_index": index,
                    "class": cls,
                    "object": name,
                })

    for path in chunks:
        if not has_marker(path):
            continue
        inspected_chunks += 1
        world = path.parent.name
        for line in inspect(args.upk_info, path, "--mesh-actors").splitlines():
            match = ACTOR.match(line)
            if not match:
                continue
            _index, _position, mesh = match.groups()
            if not ANCIENT.search(mesh.split(".")[-1]):
                continue
            key = (world, mesh.split(".")[0], mesh)
            placements[key] += 1
            examples.setdefault(key, path.name)

    assets.sort(key=lambda a: (str(a["package"]), str(a["class"]), str(a["object"])))
    placement_rows = [
        {"world": world, "package": package, "mesh": mesh, "actors": count,
         "example_chunk": examples[(world, package, mesh)]}
        for (world, package, mesh), count in sorted(placements.items())
    ]
    write_tsv(args.output / "ancient-assets.tsv",
              ["package", "export_index", "class", "object"], assets)
    write_tsv(args.output / "ancient-placements.tsv",
              ["world", "package", "mesh", "actors", "example_chunk"], placement_rows)
    summary = {
        "packages_total": len(packages), "packages_scanned": inspected_packages,
        "chunks_total": len(chunks), "chunks_scanned": inspected_chunks,
        "asset_exports": len(assets), "placed_actor_count": sum(placements.values()),
        "worlds": dict(sorted(collections.Counter(
            {w: sum(n for (world, _, _), n in placements.items() if world == w)
             for w, _, _ in placements}
        ).items())),
    }
    (args.output / "summary.json").write_text(
        json.dumps(summary, indent=2) + "\n", encoding="utf-8"
    )
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
