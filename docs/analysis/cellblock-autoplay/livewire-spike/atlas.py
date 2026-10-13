"""Generate the Livewire hit atlas from the locally installed client's UPK."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import sys
import tempfile


STEP = 5
WIRE_LIBRARY = re.compile(
    r"^(?:g(?:BlackGreen|GreenBlack|RedSilver)|"
    r"o(?:Black|Green|Red)|"
    r"m(?:BlackStripe|OrangeStripe|PurpleStripe|YellowStripe)|"
    r"p(?:Bronze|Gray|Green|Rust))[1-4]$"
)
ELECTRODE_LIBRARY = re.compile(r"^[pm](?:LED(?:Blue|Green|Red)|MicroChip[12]|Gear)$")
EXPECTED_LIBRARIES = {
    f"{base}{variant}"
    for base in (
        "gBlackGreen", "gGreenBlack", "gRedSilver",
        "oBlack", "oGreen", "oRed",
        "mBlackStripe", "mOrangeStripe", "mPurpleStripe", "mYellowStripe",
        "pBronze", "pGray", "pGreen", "pRust",
    )
    for variant in range(1, 5)
} | {"mGear", "mMicroChip1", "mMicroChip2", "pLEDBlue", "pLEDGreen", "pLEDRed"}


def extract_movie(upk_path: Path) -> bytes:
    # The existing UE3 package reader handles both raw and compressed packages.
    local_reader = Path(__file__).with_name("upk_parser.py")
    if not local_reader.is_file():
        root = Path(__file__).resolve().parents[4]
        sys.path.insert(0, str(root / "tools"))
    from upk_parser import PackageReader

    package = PackageReader(str(upk_path))
    package.parse()
    movies = [
        export for export in package.exports
        if package.get_export_class_name(export) == "GFxMovieInfo"
        and export.object_name == "Livewire"
    ]
    if len(movies) != 1:
        raise ValueError(f"expected one Livewire GFxMovieInfo export, found {len(movies)}")
    movie = movies[0]
    stream = package._get_data_stream()
    try:
        stream.seek(movie.serial_offset)
        export_bytes = stream.read(movie.serial_size)
    finally:
        stream.close()
    offset = export_bytes.find(b"GFX")
    if offset < 0:
        raise ValueError("Livewire export has no GFX movie signature")
    data = export_bytes[offset:]
    if len(data) < 8:
        raise ValueError("Livewire GFX movie is truncated")
    return data


def build_atlas(movie_path: Path) -> dict:
    # shapes.py and bounds_core.py are the checked-in SWF parsers from the spike.
    # They read GFX at import time, so invoke this function once per process.
    os.environ["GFX"] = str(movie_path)
    from shapes import bbox, exports, flat, hit

    libraries = {}
    for name, cid in sorted(exports.items()):
        if not (WIRE_LIBRARY.fullmatch(name) or ELECTRODE_LIBRARY.fullmatch(name)):
            continue
        edges = flat(cid)
        bounds = bbox(cid)
        if not edges or bounds is None:
            raise ValueError(f"{name}: no shape edges or bounds")
        x0, x1, y0, y1 = [int(value / 20) for value in bounds]
        points = [
            [lx, ly]
            for lx in range(x0 - x0 % STEP, x1 + 1, STEP)
            for ly in range(y0 - y0 % STEP, y1 + 1, STEP)
            if hit(edges, lx * 20, ly * 20)
        ]
        if not points:
            raise ValueError(f"{name}: empty hit map")
        libraries[name] = {"bounds_twips": bounds, "points": points}
    if set(libraries) != EXPECTED_LIBRARIES:
        missing = sorted(EXPECTED_LIBRARIES - set(libraries))
        unexpected = sorted(set(libraries) - EXPECTED_LIBRARIES)
        raise ValueError(f"Livewire library set mismatch: missing={missing}, unexpected={unexpected}")
    return libraries


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("upk", type=Path, help="installed CookedPC/UI/Flash/Livewire.upk")
    parser.add_argument("output", type=Path, help="private generated atlas JSON")
    args = parser.parse_args()
    movie = extract_movie(args.upk)
    with tempfile.TemporaryDirectory(prefix="cimmeria-livewire-") as folder:
        movie_path = Path(folder) / "Livewire.gfx"
        movie_path.write_bytes(movie)
        libraries = build_atlas(movie_path)
    atlas = {
        "version": 1,
        "step": STEP,
        "movie_sha256": hashlib.sha256(movie).hexdigest(),
        "libraries": libraries,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    temporary = args.output.with_suffix(args.output.suffix + ".tmp")
    temporary.write_text(json.dumps(atlas, separators=(",", ":")), encoding="utf-8")
    temporary.replace(args.output)
    print(f"Livewire atlas: {len(libraries)} libraries -> {args.output}")


if __name__ == "__main__":
    main()
