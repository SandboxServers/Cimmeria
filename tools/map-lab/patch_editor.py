"""Apply Atrea's documented Editor byte patches to a disposable SGW.exe.

This is a local research tool. It writes a separate executable and refuses
to modify its input. Neither the executable nor patch output belongs in git.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import struct
import xml.etree.ElementTree as ET
from pathlib import Path


def u16(data: bytes, at: int) -> int:
    return struct.unpack_from("<H", data, at)[0]


def u32(data: bytes, at: int) -> int:
    return struct.unpack_from("<I", data, at)[0]


def pe_sections(image: bytes) -> list[tuple[str, int, int, int, int]]:
    if image[:2] != b"MZ":
        raise ValueError("input is not a PE executable")
    pe = u32(image, 0x3C)
    if image[pe : pe + 4] != b"PE\x00\x00":
        raise ValueError("invalid PE signature")
    count = u16(image, pe + 6)
    optional_size = u16(image, pe + 20)
    table = pe + 24 + optional_size
    sections = []
    for index in range(count):
        at = table + index * 40
        name = image[at : at + 8].split(b"\x00", 1)[0].decode("ascii")
        virtual_size = u32(image, at + 8)
        virtual_address = u32(image, at + 12)
        raw_size = u32(image, at + 16)
        raw_offset = u32(image, at + 20)
        sections.append((name, virtual_address, virtual_size, raw_offset, raw_size))
    return sections


def file_offset(sections: list[tuple[str, int, int, int, int]], rva: int, length: int) -> int:
    for name, va, virtual_size, raw, raw_size in sections:
        if va <= rva and rva + length <= va + min(virtual_size, raw_size):
            return raw + rva - va
    raise ValueError(f"RVA 0x{rva:08x} is not backed by a PE file section")


def parse_bytes(value: str | None) -> list[int | None]:
    if value is None:
        raise ValueError("patch chunk lacks byte data")
    return [None if word.upper() == "XX" else int(word, 16) for word in value.split()]


def patch(image: bytes, config_xml: Path) -> tuple[bytes, list[dict[str, object]]]:
    sections = pe_sections(image)
    output = bytearray(image)
    report = []
    root = ET.parse(config_xml).getroot()
    for item in root.findall("./Patches/Patch"):
        if item.get("Group") != "Editor":
            continue
        name = item.attrib["Name"]
        base = int(item.attrib["BaseAddress"], 0)
        chunks = item.findall("Chunk")
        if not chunks:
            raise ValueError(f"{name}: no chunks")
        for chunk in chunks:
            rva = base + int(chunk.get("RelativeAddress", "0"), 0)
            old = parse_bytes(chunk.findtext("OriginalBytes"))
            new = parse_bytes(chunk.findtext("ReplacementBytes"))
            if len(old) != len(new):
                raise ValueError(f"{name}: byte-count change at RVA 0x{rva:x}")
            at = file_offset(sections, rva, len(old))
            found = bytes(output[at : at + len(old)])
            if any(wanted is not None and wanted != actual for wanted, actual in zip(old, found)):
                raise ValueError(
                    f"{name}: RVA 0x{rva:08x}, file 0x{at:08x}: "
                    f"expected {old}, found {found.hex(' ')}"
                )
            output[at : at + len(new)] = bytes(actual if wanted is None else wanted for wanted, actual in zip(new, found))
            report.append({"patch": name, "rva": f"0x{rva:08x}", "file_offset": f"0x{at:08x}"})
    if not report:
        raise ValueError("config contains no Editor patches")
    return bytes(output), report


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", type=Path, help="unmodified disposable SGW.exe")
    parser.add_argument("config_xml", type=Path, help="AtreaLoader.config.xml")
    parser.add_argument("--output", type=Path, help="write a separate patched executable")
    args = parser.parse_args()
    source = args.input.resolve()
    if args.output and source == args.output.resolve():
        parser.error("output must differ from input")
    image = source.read_bytes()
    result, report = patch(image, args.config_xml)
    if args.output:
        if args.output.exists():
            parser.error("output already exists")
        args.output.write_bytes(result)
    print(json.dumps({
        "input_sha256": hashlib.sha256(image).hexdigest(),
        "output_sha256": hashlib.sha256(result).hexdigest(),
        "changed_bytes": sum(a != b for a, b in zip(image, result)),
        "patch_chunks": report,
        "written": str(args.output) if args.output else None,
    }, indent=2))


if __name__ == "__main__":
    main()
