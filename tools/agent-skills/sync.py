#!/usr/bin/env python3
"""Mirror the repo's agent skills from .claude/skills/ to .agents/skills/.

.claude/skills/<name>/ is the one source. Claude Code reads it there, Codex reads
only .agents/skills/, and Copilot reads both. Each mirrored SKILL.md gets a marker
comment after its frontmatter; a mirrored skill whose source is gone is removed.
Skills in .agents/skills/ without the marker are hand-written and left alone.

Usage:
  python tools/agent-skills/sync.py           # write the mirror
  python tools/agent-skills/sync.py --check   # exit 1 if the mirror is stale (CI)
"""

import argparse
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
SOURCE = Path(".claude/skills")
MIRROR = Path(".agents/skills")
MARKER = "<!-- Generated from .claude/skills/{name}/ by tools/agent-skills/sync.py. Edit the source, then rerun the script. -->"


def with_marker(name, text):
    """Insert the marker line right after the YAML frontmatter."""
    marker = MARKER.format(name=name)
    newline = "\r\n" if "\r\n" in text else "\n"
    lines = text.split(newline)
    if lines and lines[0].strip() == "---":
        for index in range(1, len(lines)):
            if lines[index].strip() == "---":
                return newline.join(lines[: index + 1] + [marker] + lines[index + 1 :])
    return newline.join([marker] + lines)


def is_generated(skill_dir):
    skill_md = skill_dir / "SKILL.md"
    if not skill_md.is_file():
        return False
    return "by tools/agent-skills/sync.py" in skill_md.read_text(encoding="utf-8")


def expected_files(repo):
    """Map each mirror path to the bytes it should hold."""
    files = {}
    source_root = repo / SOURCE
    if not source_root.is_dir():
        return files
    for skill_dir in sorted(p for p in source_root.iterdir() if p.is_dir()):
        if not (skill_dir / "SKILL.md").is_file():
            continue
        for path in sorted(p for p in skill_dir.rglob("*") if p.is_file()):
            rel = path.relative_to(source_root)
            data = path.read_bytes()
            if rel.name == "SKILL.md" and len(rel.parts) == 2:
                data = with_marker(skill_dir.name, data.decode("utf-8")).encode("utf-8")
            files[MIRROR / rel] = data
    return files


def generated_mirror_files(repo):
    """Every file under a mirrored (marker-carrying) skill directory."""
    files = set()
    mirror_root = repo / MIRROR
    if not mirror_root.is_dir():
        return files
    for skill_dir in mirror_root.iterdir():
        if skill_dir.is_dir() and is_generated(skill_dir):
            files.update(p.relative_to(repo) for p in skill_dir.rglob("*") if p.is_file())
    return files


def plan(repo):
    """Return (writes, deletes): mirror paths to write and stale ones to remove."""
    expected = expected_files(repo)
    clashes = sorted(
        {rel.parts[2] for rel in expected}
        - {d.name for d in (repo / MIRROR).glob("*") if is_generated(d)}
    )
    clashes = [name for name in clashes if (repo / MIRROR / name / "SKILL.md").is_file()]
    if clashes:
        raise SystemExit(
            f"refusing to overwrite hand-written skill(s) in {MIRROR.as_posix()}: "
            f"{', '.join(clashes)}. Rename or remove one copy."
        )
    writes = {
        rel: data
        for rel, data in expected.items()
        if not (repo / rel).is_file() or (repo / rel).read_bytes() != data
    }
    deletes = sorted(generated_mirror_files(repo) - set(expected))
    return writes, deletes


def apply(repo, writes, deletes):
    for rel, data in writes.items():
        target = repo / rel
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data)
    for rel in deletes:
        (repo / rel).unlink()
    mirror_root = repo / MIRROR
    if mirror_root.is_dir():
        for directory in sorted(mirror_root.rglob("*"), reverse=True):
            if directory.is_dir() and not any(directory.iterdir()):
                directory.rmdir()


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true", help="report drift, change nothing")
    parser.add_argument("--repo", type=Path, default=REPO, help=argparse.SUPPRESS)
    args = parser.parse_args(argv)

    writes, deletes = plan(args.repo)
    if args.check:
        for rel in sorted(writes):
            print(f"stale: {rel.as_posix()}")
        for rel in deletes:
            print(f"orphan: {rel.as_posix()}")
        if writes or deletes:
            print("Run: python tools/agent-skills/sync.py", file=sys.stderr)
            return 1
        print("agent skills mirror is current")
        return 0
    apply(args.repo, writes, deletes)
    print(f"wrote {len(writes)} file(s), removed {len(deletes)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
