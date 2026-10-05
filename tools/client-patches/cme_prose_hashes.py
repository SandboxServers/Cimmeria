#!/usr/bin/env python3
"""Write the SHA-256 list the no-CME-text guard for patch 012 checks against.

Patch 012 ships an XML this project wrote. Its guard test must fail if a string of
CME's own prose (a description, a parameter description or a comment from the
client's slash-command XML) ever ends up in that file, and CI has no client to
compare with. So the prose is committed as hashes only: a hash lets the test
recognize a string without the repository holding it.

    python tools/client-patches/cme_prose_hashes.py "<client>/Common/xml/slash_commands"

Run it against a client's slash_commands directory (the one holding SlashCommands.xml,
InternalSlashCommands.xml and FinalSlashCommands.xml) and commit the result,
data/client-patches/012-gm-slash-commands/cme-prose.sha256. It is only needed again if
the guard should learn more strings. Usage attributes are left out on purpose: ours
are built from the command and parameter names by a fixed rule, so they can equal
CME's for a command with no parameters without being copied.

A string is normalized before hashing: trimmed, runs of whitespace collapsed to one
space, ASCII lowercased. The test uses the same rule.
"""
import hashlib
import re
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "data" / "client-patches" / "012-gm-slash-commands" / "cme-prose.sha256"
NS = "{http://www.cheyenneme.com/common/slash_commands}"
FILES = ("SlashCommands.xml", "InternalSlashCommands.xml", "FinalSlashCommands.xml")
MIN_LEN = 4


def normalize(text: str) -> str:
    return re.sub(r"\s+", " ", text.strip()).lower()


def prose_strings(path: Path):
    raw = path.read_text(encoding="utf-8")
    # Comments: the whole comment and each of its non-empty lines.
    for comment in re.findall(r"<!--(.*?)-->", raw, re.S):
        yield comment
        yield from comment.splitlines()
    root = ET.fromstring(raw)
    for cmd in root.iter(NS + "Command"):
        yield cmd.get(NS + "Description", "")
        for param in cmd:
            yield param.get(NS + "ParamDescription", "")


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    src = Path(sys.argv[1])
    hashes = set()
    for name in FILES:
        for text in prose_strings(src / name):
            norm = normalize(text)
            if len(norm) >= MIN_LEN:
                hashes.add(hashlib.sha256(norm.encode("utf-8")).hexdigest())
    OUT.write_text("\n".join(sorted(hashes)) + "\n", encoding="ascii", newline="\n")
    print(f"wrote {OUT.relative_to(ROOT)}: {len(hashes)} hashes")
    return 0


if __name__ == "__main__":
    sys.exit(main())
