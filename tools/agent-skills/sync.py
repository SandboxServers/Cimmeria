#!/usr/bin/env python3
"""Mirror the repo's agent skills from .claude/skills/ to .agents/skills/.

.claude/skills/<name>/ is the one source. Claude Code reads it there, Codex reads
only .agents/skills/, and Copilot reads both. Each mirrored SKILL.md gets a marker
comment after its frontmatter; a mirrored skill whose source is gone is removed.
Skills in .agents/skills/ without the marker are hand-written and left alone.
Relative Markdown links that leave the skill's folder (../../agents/x.md) are
rewritten so they still resolve from the mirror.

It also writes a Codex agent, .codex/agents/<name>.toml, for every
.claude/agents/<name>.md, and removes a generated one whose source is gone
(hand-written TOMLs without the marker line are left alone). The fields map as:

  name                    <- frontmatter `name` (the file name when absent)
  description             <- frontmatter `description`
  model_reasoning_effort  <- frontmatter `effort` (low/medium/high; xhigh and max
                             become high), else the Claude model tier: haiku -> low,
                             sonnet -> medium, opus -> high; omitted when neither
  developer_instructions  <- the Markdown body, as a TOML literal string

Claude-only frontmatter (`model`, `tools`, `color`, `memory`) has no Codex
field and is dropped: a Codex agent runs on the session's model with its tools.

Usage:
  python tools/agent-skills/sync.py           # write the mirror
  python tools/agent-skills/sync.py --check   # exit 1 if the mirror is stale (CI)
"""

import argparse
import json
import posixpath
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
SOURCE = Path(".claude/skills")
MIRROR = Path(".agents/skills")
MARKER = "<!-- Generated from .claude/skills/{name}/ by tools/agent-skills/sync.py. Edit the source, then rerun the script. -->"
AGENT_SOURCE = Path(".claude/agents")
CODEX_AGENTS_DIR = Path(".codex/agents")
# Every .claude/agents/*.md gets a Codex twin. One that calls MCP tools (lab-driver)
# needs those servers configured in Codex too (.codex/config.toml.example).
#: Claude `effort` values Codex lacks, and the model tier used when `effort` is absent.
CODEX_EFFORT = {"xhigh": "high", "max": "high"}
MODEL_EFFORT = {"haiku": "low", "sonnet": "medium", "opus": "high"}
CODEX_MARKER = "# Generated from .claude/agents/{name}.md by tools/agent-skills/sync.py. Edit the source, then rerun the script."
LINK = re.compile(r"(\]\()([^)\s]+)(\))")


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


def rewrite_links(text, source_rel, mirror_rel):
    """Re-point relative links that leave the skill folder so they resolve from
    the mirror. Paths are repo-relative POSIX strings of the two files."""
    source_dir = posixpath.dirname(source_rel)
    mirror_dir = posixpath.dirname(mirror_rel)
    skill_root = "/".join(source_rel.split("/")[:3]) + "/"

    def fix(match):
        target = match.group(2)
        if target.startswith(("#", "/", "mailto:")) or "://" in target:
            return match.group(0)
        path, hash_, anchor = target.partition("#")
        resolved = posixpath.normpath(posixpath.join(source_dir, path))
        if (resolved + "/").startswith(skill_root):
            return match.group(0)
        moved = posixpath.relpath(resolved, mirror_dir)
        return f"{match.group(1)}{moved}{hash_}{anchor}{match.group(3)}"

    return LINK.sub(fix, text)


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
            if rel.suffix == ".md":
                text = rewrite_links(
                    data.decode("utf-8"),
                    (SOURCE / rel).as_posix(),
                    (MIRROR / rel).as_posix(),
                )
                if rel.name == "SKILL.md" and len(rel.parts) == 2:
                    text = with_marker(skill_dir.name, text)
                data = text.encode("utf-8")
            files[MIRROR / rel] = data
    return files


def frontmatter(text):
    """(fields, body) of a Markdown file with simple `key: value` frontmatter."""
    newline = "\r\n" if "\r\n" in text else "\n"
    lines = text.split(newline)
    if not lines or lines[0].strip() != "---":
        raise SystemExit("agent file has no frontmatter")
    for end in range(1, len(lines)):
        if lines[end].strip() == "---":
            break
    else:
        raise SystemExit("agent file frontmatter is not closed")
    fields = {}
    for line in lines[1:end]:
        key, sep, value = line.partition(":")
        if not sep or line.startswith((" ", "\t", "-")):
            continue
        value = value.strip()
        if len(value) >= 2 and value[0] == value[-1] == '"':
            value = json.loads(value)
        fields[key.strip()] = value
    body = newline.join(lines[end + 1 :]).lstrip("\r\n")
    return fields, body, newline


def codex_effort(fields):
    """Codex's model_reasoning_effort for an agent's frontmatter, or None."""
    effort = fields.get("effort", "").strip().lower()
    if effort:
        return CODEX_EFFORT.get(effort, effort)
    return MODEL_EFFORT.get(fields.get("model", "").strip().lower())


def codex_agent(name, text):
    """The Codex agent TOML for a .claude/agents/<name>.md text."""
    fields, body, newline = frontmatter(text)
    if "'''" in body:
        raise SystemExit(f"{name}: the body holds ''' and cannot be a TOML literal string")
    out = [
        CODEX_MARKER.format(name=name),
        f"name = {json.dumps(fields.get('name', name))}",
        f"description = {json.dumps(fields.get('description', ''))}",
    ]
    effort = codex_effort(fields)
    if effort:
        out.append(f"model_reasoning_effort = {json.dumps(effort)}")
    # A literal string: the instructions carry backslashes and quotes as written.
    out.append("developer_instructions = '''")
    return newline.join(out) + newline + body.rstrip("\r\n") + newline + "'''" + newline


def expected_codex_agents(repo):
    """Map each Codex agent path to the bytes it should hold."""
    files = {}
    source_root = repo / AGENT_SOURCE
    if not source_root.is_dir():
        return files
    for source in sorted(source_root.glob("*.md")):
        name = source.stem
        files[CODEX_AGENTS_DIR / f"{name}.toml"] = codex_agent(
            # Bytes, not read_text: that would turn CRLF into LF.
            name, source.read_bytes().decode("utf-8")
        ).encode("utf-8")
    return files


def generated_codex_agents(repo):
    """Every .codex/agents/*.toml that carries the generated marker."""
    root = repo / CODEX_AGENTS_DIR
    if not root.is_dir():
        return set()
    return {
        path.relative_to(repo)
        for path in root.glob("*.toml")
        if path.read_bytes().startswith(b"# Generated from .claude/agents/")
    }


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
    codex = expected_codex_agents(repo)
    deletes = sorted((generated_mirror_files(repo) - set(expected)) | (generated_codex_agents(repo) - set(codex)))
    expected.update(codex)
    writes = {
        rel: data
        for rel, data in expected.items()
        if not (repo / rel).is_file() or (repo / rel).read_bytes() != data
    }
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
