#!/usr/bin/env python3
"""Add Kismet sequence entries to data/cache/CookedDataKismetSeqEvent.pak.

The server serves the client's `sequences` table from this PAK (cooked-data
category 1), not from the database, so a new row in
db/resources/Events/Seed/sequences.sql is invisible to clients until the PAK
carries a matching `_<sequence_id>` entry. Keep the two in sync.

The PAK is a zip: one XML entry per sequence plus a 4-byte little-endian
`MetaData` version. The version is bumped so clients drop their cached copy.

Usage:
    python tools/pak_add_kismet_sequence.py \
        --add 10187:8000:Castle_Cellblock-fffeffff.Main_Sequence.Prefabs.Foo_Seq_0 \
        --add 10188:8001:Castle_Cellblock-fffeffff.Main_Sequence.Prefabs.Foo_Seq_0

Each --add is SEQUENCE_ID:EVENT_ID:KISMET_SCRIPT_NAME. Existing ids are refused.
"""

import argparse
import os
import struct
import sys
import zipfile

DEFAULT_PAK = os.path.join("data", "cache", "CookedDataKismetSeqEvent.pak")

TEMPLATE = (
    '<?xml version="1.0" encoding="UTF-8"?>\n'
    '<COOKED_KISMET_EVENT_SEQUENCE'
    ' xmlns:SOAP-ENV="http://schemas.xmlsoap.org/soap/envelope/"'
    ' xmlns:SOAP-ENC="http://schemas.xmlsoap.org/soap/encoding/"'
    ' xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"'
    ' xmlns:xsd="http://www.w3.org/2001/XMLSchema"'
    ' xmlns:CookedData1="SGW"'
    ' KismetScriptName="{script}" EventID="{event}" KismetEventSetSeqID="{seq}">'
    "</COOKED_KISMET_EVENT_SEQUENCE>"
)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--pak", default=DEFAULT_PAK)
    parser.add_argument("--add", action="append", required=True, metavar="ID:EVENT:SCRIPT")
    args = parser.parse_args()

    additions = []
    for spec in args.add:
        seq, event, script = spec.split(":", 2)
        if '"' in script or "<" in script or "&" in script:
            sys.exit(f"script name needs XML escaping, refusing: {script}")
        additions.append((int(seq), int(event), script))

    with zipfile.ZipFile(args.pak) as src:
        entries = [(info, src.read(info.filename)) for info in src.infolist()]
    existing = {info.filename for info, _ in entries}
    for seq, _, _ in additions:
        if f"_{seq}" in existing:
            sys.exit(f"sequence {seq} is already in {args.pak}")
    if len({seq for seq, _, _ in additions}) != len(additions):
        sys.exit("duplicate sequence id in --add")

    # New entries copy their zip attributes from an existing one so the archive
    # stays uniform.
    model = next(info for info, _ in entries if info.filename != "MetaData")
    version = None
    tmp = args.pak + ".tmp"
    with zipfile.ZipFile(tmp, "w") as dst:
        for info, data in entries:
            if info.filename == "MetaData":
                version = struct.unpack("<I", data[:4])[0] + 1
                data = struct.pack("<I", version) + data[4:]
            dst.writestr(info, data, compress_type=info.compress_type)
        for seq, event, script in additions:
            info = zipfile.ZipInfo(f"_{seq}", date_time=model.date_time)
            info.create_system = model.create_system
            info.external_attr = model.external_attr
            xml = TEMPLATE.format(script=script, event=event, seq=seq)
            dst.writestr(info, xml.encode("utf-8"), compress_type=model.compress_type)
    if version is None:
        os.remove(tmp)
        sys.exit(f"{args.pak} has no MetaData entry; refusing to write an unversioned PAK")
    os.replace(tmp, args.pak)
    print(f"{args.pak}: added {len(additions)} sequence(s), MetaData version -> {version}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
