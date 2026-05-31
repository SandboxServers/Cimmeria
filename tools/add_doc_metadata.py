#!/usr/bin/env python3
"""
One-shot tool to prepend Diátaxis metadata frontmatter to docs that lack it.

Usage: python3 tools/add_doc_metadata.py [--dry-run]

The audit in issue #344 (Category 3) lists ~90 in-scope docs missing
metadata blocks. This script applies a per-directory (with per-file
overrides) mapping of type / audience and prepends YAML frontmatter
above the existing H1.

Files that already have YAML frontmatter (start with `---`) are skipped.
Files that don't exist on disk are skipped with a notice. The existing
`> **Last updated**` blockquote callouts are preserved.

After this script runs once, it should not need to run again — it is
deliberately one-shot, not idempotent in the "running every PR" sense.
"""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass
from pathlib import Path

LAST_UPDATED = "2026-05-27"
ROOT = Path(__file__).resolve().parent.parent


@dataclass
class Meta:
    type: str
    audience: str
    notes: str | None = None  # optional extra YAML field

# Per-directory defaults applied to every file in the dir not in OVERRIDES.
DIR_DEFAULTS: dict[str, Meta] = {
    "docs/gameplay":            Meta("reference", "engineers"),
    "docs/content":             Meta("reference", "engineers"),
    "docs/protocol":            Meta("reference", "engineers"),
    "docs/engine":              Meta("reference", "engineers"),
    "docs/operations":          Meta("how-to",    "operators"),
    "docs/reverse-engineering": Meta("reference", "contributors doing RE"),
    "docs/guides":              Meta("how-to",    "engineers"),
    "docs/architecture":        Meta("explanation", "engineers"),
    "docs/client":              Meta("reference", "engineers"),
    "docs/tools":               Meta("reference", "engineers"),
}

# Per-file overrides. Path keys are relative to repo root.
OVERRIDES: dict[str, Meta] = {
    # docs/ root — each has a distinct audience per the audit
    "docs/how-sgw-works.md":        Meta("explanation", "new hires, engineers"),
    "docs/connection-flow.md":      Meta("reference", "engineers"),
    "docs/game-systems.md":         Meta("reference", "engineers"),
    "docs/game-data.md":            Meta("reference", "engineers"),
    "docs/project-status.md":       Meta("reference", "anyone tracking the project"),
    "docs/gap-analysis.md":         Meta("explanation", "engineers"),
    "docs/known-issues.md":         Meta("reference", "engineers, operators"),
    "docs/multiplayer.md":          Meta("how-to", "operators, players"),
    "docs/commands.md":             Meta("reference", "engineers, players"),
    "docs/client-tools.md":         Meta("reference", "engineers"),
    "docs/network-messages.md":     Meta("reference", "engineers"),

    # docs/architecture/ specific
    "docs/architecture/migration-roadmap.md": Meta("explanation", "engineers"),
    "docs/architecture/state-field-bits.md":  Meta("reference", "engineers"),
    "docs/architecture/python-console.md":    Meta("reference", "engineers, operators"),

    # docs/tools/
    "docs/tools/admin-api.md":       Meta("reference", "engineers"),
    "docs/tools/admin-panel.md":     Meta("explanation", "engineers"),

    # docs/client/
    "docs/client/sgw-launcher.md":              Meta("explanation", "engineers"),
    "docs/client/launcher-guide.md":            Meta("how-to", "players, operators"),
    "docs/client/launcher-distribution-setup.md": Meta("how-to", "operators"),
    "docs/client/audio-voice-inventory.md":     Meta("reference", "engineers"),
    "docs/client/facefx-lip-sync.md":           Meta("explanation", "engineers"),
    "docs/client/ui-layout-inventory.md":       Meta("reference", "engineers"),

    # docs/reverse-engineering/
    "docs/reverse-engineering/PLAN.md":                    Meta("explanation", "contributors doing RE"),
    "docs/reverse-engineering/address-map.md":             Meta("reference", "contributors doing RE"),
    "docs/reverse-engineering/editor-source-mapping.md":   Meta("reference", "contributors doing RE"),
    "docs/reverse-engineering/function-naming-progress.md": Meta("reference", "contributors doing RE"),
    "docs/reverse-engineering/evidence-standards.md":      Meta("reference", "contributors doing RE"),

    # docs/guides/ — the remaining how-tos (P0/P1-authored files already have metadata)
    "docs/guides/reading-decompiled-code.md": Meta("how-to", "contributors doing RE"),
    "docs/guides/sgw-live-debugging.md":      Meta("how-to", "contributors doing RE"),

    # docs/engine/entity-def-guide.md (relocated in P2 — needs metadata)
    "docs/engine/entity-def-guide.md": Meta("reference", "engineers"),
}

# The explicit in-scope list from the audit (Category 3).
IN_SCOPE = [
    # docs/ root
    "docs/how-sgw-works.md",
    "docs/connection-flow.md",
    "docs/game-systems.md",
    "docs/game-data.md",
    "docs/project-status.md",
    "docs/gap-analysis.md",
    "docs/known-issues.md",
    "docs/multiplayer.md",
    "docs/commands.md",
    "docs/client-tools.md",
    "docs/network-messages.md",

    # docs/guides/ remaining
    "docs/guides/reading-decompiled-code.md",
    "docs/guides/sgw-live-debugging.md",

    # docs/engine/
    "docs/engine/entity-type-catalog.md",
    "docs/engine/cme-framework.md",
    "docs/engine/watcher-system.md",
    "docs/engine/space-management.md",
    "docs/engine/entity-lod-system.md",
    "docs/engine/distributed-checkpointing.md",
    "docs/engine/client-visual-system.md",
    "docs/engine/character-visual-components.md",
    "docs/engine/cooked-data-pak-format.md",
    "docs/engine/entity-def-guide.md",

    # docs/client/
    "docs/client/audio-voice-inventory.md",
    "docs/client/facefx-lip-sync.md",
    "docs/client/sgw-launcher.md",
    "docs/client/ui-layout-inventory.md",
    "docs/client/launcher-guide.md",
    "docs/client/launcher-distribution-setup.md",

    # docs/architecture/
    "docs/architecture/migration-roadmap.md",
    "docs/architecture/state-field-bits.md",
    "docs/architecture/python-console.md",

    # docs/tools/
    "docs/tools/admin-api.md",
    "docs/tools/admin-panel.md",

    # docs/reverse-engineering/ root
    "docs/reverse-engineering/PLAN.md",
    "docs/reverse-engineering/address-map.md",
    "docs/reverse-engineering/editor-source-mapping.md",
    "docs/reverse-engineering/function-naming-progress.md",
    "docs/reverse-engineering/evidence-standards.md",

    # docs/protocol/
    "docs/protocol/world-entry-phases.md",
    "docs/protocol/sgwplayer-base-method-dispatch-table.md",
    "docs/protocol/client-method-dispatch-table.md",
    "docs/protocol/cell-method-dispatch-table.md",
    "docs/protocol/client-verified-wire-formats.md",
    "docs/protocol/message-dispatch-table.md",
    "docs/protocol/login-handshake.md",
    "docs/protocol/entity-property-sync.md",
    "docs/protocol/position-updates.md",
    "docs/protocol/item-sequence-lookup.md",
    "docs/protocol/auto-cycle-button.md",
    "docs/protocol/mercury-wire-format.md",

    # docs/operations/
    "docs/operations/container.md",
    "docs/operations/colo-deploy.md",
]

# Add every file under docs/gameplay/ and docs/content/ — the audit lists them all.
for path in sorted((ROOT / "docs/gameplay").glob("*.md")):
    rel = str(path.relative_to(ROOT)).replace("\\", "/")
    if rel not in IN_SCOPE:
        IN_SCOPE.append(rel)
for path in sorted((ROOT / "docs/content").glob("*.md")):
    rel = str(path.relative_to(ROOT)).replace("\\", "/")
    if rel not in IN_SCOPE:
        IN_SCOPE.append(rel)


def meta_for(rel_path: str) -> Meta | None:
    if rel_path in OVERRIDES:
        return OVERRIDES[rel_path]
    for dir_prefix, meta in DIR_DEFAULTS.items():
        if rel_path.startswith(dir_prefix + "/"):
            return meta
    return None


H1_RE = re.compile(r"^# (.+?)\s*$", re.MULTILINE)


def title_from_body(body: str, rel_path: str) -> str:
    m = H1_RE.search(body)
    if m:
        return m.group(1).strip()
    # Fallback: derive from filename.
    stem = Path(rel_path).stem.replace("-", " ").replace("_", " ")
    return stem.title()


def has_frontmatter(body: str) -> bool:
    return body.startswith("---\n") or body.startswith("---\r\n")


def build_frontmatter(meta: Meta, title: str) -> str:
    yaml_title = title.replace('"', '\\"')
    return (
        "---\n"
        f'title: "{yaml_title}"\n'
        f"type: {meta.type}\n"
        f"audience: {meta.audience}\n"
        f"last_updated: {LAST_UPDATED}\n"
        "---\n\n"
    )


def main() -> int:
    # Force UTF-8 stdout on Windows so titles with em-dashes / arrows print.
    try:
        sys.stdout.reconfigure(encoding="utf-8")  # type: ignore[attr-defined]
    except AttributeError:
        pass

    parser = argparse.ArgumentParser()
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    touched = 0
    skipped_existing = 0
    skipped_missing = 0
    no_meta = 0

    for rel in IN_SCOPE:
        path = ROOT / rel
        if not path.exists():
            print(f"  missing : {rel}")
            skipped_missing += 1
            continue
        meta = meta_for(rel)
        if meta is None:
            print(f"  no meta : {rel}")
            no_meta += 1
            continue
        body = path.read_text(encoding="utf-8")
        if has_frontmatter(body):
            print(f"  has fm  : {rel}")
            skipped_existing += 1
            continue
        title = title_from_body(body, rel)
        new_body = build_frontmatter(meta, title) + body
        if args.dry_run:
            print(f"  WOULD   : {rel}  ({meta.type}, {meta.audience})  title='{title}'")
        else:
            path.write_text(new_body, encoding="utf-8")
            print(f"  WROTE   : {rel}  ({meta.type}, {meta.audience})")
        touched += 1

    print()
    print(f"Touched (would): {touched}")
    print(f"Already had FM:  {skipped_existing}")
    print(f"Missing file:    {skipped_missing}")
    print(f"No meta mapping: {no_meta}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
