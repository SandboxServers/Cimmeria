"""Give an uncompressed SGW package a same-length local map identity.

This is an experimental byte-preserving alias operation for a disposable map
scaffold. It does not create geometry or prove client load. Every replacement
is same-length, so package offsets and export serializations stay put.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import struct
from pathlib import Path


TAG = 0x9E2A83C1
PKG_STORE_COMPRESSED = 0x02000000


def summary(image: bytes) -> tuple[int, int, int, bytes]:
    if len(image) < 80 or struct.unpack_from("<I", image)[0] != TAG:
        raise ValueError("not an SGW UE3 package")
    pos = 12  # tag, packed version, TotalHeaderSize
    folder_chars = struct.unpack_from("<i", image, pos)[0]
    pos += 4
    if folder_chars == 0:
        pass
    elif folder_chars > 0:
        pos += folder_chars
    else:
        pos += -folder_chars * 2
    if pos + 48 > len(image):
        raise ValueError("truncated package summary")
    flags, name_count, name_offset = struct.unpack_from("<Iii", image, pos)
    guid_at = pos + 32
    if flags & PKG_STORE_COMPRESSED:
        raise ValueError("package is compressed; normalize with upk_patch roundtrip first")
    if name_count < 0 or name_offset < 0 or name_offset >= len(image):
        raise ValueError("invalid name table summary")
    return name_count, name_offset, guid_at, image[guid_at : guid_at + 16]


def alias(image: bytes, old: str, new: str, salt: str) -> tuple[bytes, dict[str, object]]:
    if not old or not new or len(old.encode("ascii")) != len(new.encode("ascii")):
        raise ValueError("old and new map names must be nonempty, ASCII and equal byte length")
    count, offset, guid_at, old_guid = summary(image)
    old_ascii, new_ascii = old.encode("ascii"), new.encode("ascii")
    old_wide = old.encode("utf-16le")
    variants = {(old, new), (old.lower(), new.lower()), (old.upper(), new.upper())}
    ascii_hits = sum(image.count(a.encode("ascii")) for a, _ in variants)
    wide_hits = sum(image.count(a.encode("utf-16le")) for a, _ in variants)
    if ascii_hits + wide_hits == 0:
        raise ValueError(f"package has no {old!r} references")
    # The original package uses FNames and FString fields as well as native
    # tails. Same-length replacement preserves every serialized offset.
    result = image
    for before, after in variants:
        result = result.replace(before.encode("ascii"), after.encode("ascii"))
        result = result.replace(before.encode("utf-16le"), after.encode("utf-16le"))
    new_guid = hashlib.sha256(old_guid + new_ascii + salt.encode("utf-8")).digest()[:16]
    guid_hits = result.count(old_guid)
    if old_guid == bytes(16) or guid_hits == 0:
        raise ValueError("source package GUID is missing or zero")
    result = result.replace(old_guid, new_guid)
    if result[guid_at : guid_at + 16] != new_guid:
        raise ValueError("package GUID replacement missed the summary")
    # Parsing the table confirms replacements did not alter FString lengths.
    cursor = offset
    for _ in range(count):
        if cursor + 4 > len(result):
            raise ValueError("truncated name table")
        length = struct.unpack_from("<i", result, cursor)[0]
        cursor += 4 + (length if length > 0 else -length * 2) + 8
        if length == 0 or cursor > len(result):
            raise ValueError("invalid name table entry")
    if old_ascii.lower() in result.lower() or old_wide.lower() in result.lower():
        raise ValueError("old map name remains after alias")
    return result, {
        "name_count": count,
        "ascii_replacements": ascii_hits,
        "utf16_replacements": wide_hits,
        "guid_replacements": guid_hits,
        "old_guid": old_guid.hex(),
        "new_guid": new_guid.hex(),
        "sha256": hashlib.sha256(result).hexdigest(),
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--old", required=True)
    parser.add_argument("--new", required=True)
    args = parser.parse_args()
    if args.input.resolve() == args.output.resolve() or args.output.exists():
        parser.error("output must be a new file distinct from input")
    image = args.input.read_bytes()
    result, report = alias(image, args.old, args.new, args.input.name)
    args.output.write_bytes(result)
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
