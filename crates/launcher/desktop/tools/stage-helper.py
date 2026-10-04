#!/usr/bin/env python3
"""Stage a Windows-native CI helper for a Mac build; never run at app startup."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import struct
import tempfile


HELPERS = {
    "archive": ("cimmeria-archive-worker.exe", "helper-build.json",
                "x86_64-pc-windows-msvc", 0x8664, 0x20B, "CIMMERIA_WINDOWS_HELPER_SHA256"),
    "prerequisite": ("cimmeria-prerequisite-worker.exe", "prerequisite-helper-build.json",
                     "i686-pc-windows-msvc", 0x14C, 0x10B, "CIMMERIA_PREREQUISITE_HELPER_SHA256"),
}


def stage(source: Path, destination: Path, expected: str, revision: str,
          kind: str = "archive") -> None:
    if kind not in HELPERS:
        raise ValueError("unknown helper kind")
    filename, receipt, target, machine, magic, _ = HELPERS[kind]
    if not re.fullmatch(r"[0-9a-fA-F]{64}", expected):
        raise ValueError("expected SHA256 must be supplied from the trusted build")
    if not re.fullmatch(r"[0-9a-fA-F]{40}", revision):
        raise ValueError("supply the full Windows build source commit")
    if source.is_symlink() or not source.is_file() or source.stat().st_size > 128 * 1024 * 1024:
        raise ValueError("helper must be a regular bounded file")
    data = source.read_bytes()
    if hashlib.sha256(data).hexdigest() != expected.lower():
        raise ValueError("helper digest does not match the expected build artifact")
    if len(data) < 64 or data[:2] != b"MZ":
        raise ValueError("helper is not a Windows PE executable")
    offset = struct.unpack_from("<I", data, 60)[0]
    if offset + 26 > len(data) or data[offset:offset + 4] != b"PE\0\0":
        raise ValueError("invalid PE header")
    if struct.unpack_from("<H", data, offset + 4)[0] != machine or struct.unpack_from("<H", data, offset + 24)[0] != magic:
        raise ValueError(f"helper must match {target} PE architecture")
    destination.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(dir=destination, delete=False) as output:
        pending = Path(output.name)
        try:
            output.write(data)
            output.flush()
            os.fsync(output.fileno())
            pending.chmod(0o644)  # Bundled public resource must be readable by other Mac users.
            os.replace(pending, destination / filename)
        finally:
            pending.unlink(missing_ok=True)
    (destination / receipt).write_text(json.dumps({
        "schema_version": 1, "sha256": expected.lower(), "source_revision": revision.lower(),
        "target": target,
    }, indent=2) + "\n")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("helper", type=Path)
    parser.add_argument("--sha256", required=True)
    parser.add_argument("--revision", required=True)
    parser.add_argument("--kind", choices=HELPERS, default="archive")
    args = parser.parse_args()
    target = Path(__file__).resolve().parents[1] / "shell/resources/windows"
    stage(args.helper, target, args.sha256, args.revision, args.kind)
    print("Staged verified Windows-native artifact. Compile the Mac shell with:")
    print(f"{HELPERS[args.kind][5]}={args.sha256.lower()}")
