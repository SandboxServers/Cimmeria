#!/usr/bin/env python3
"""SessionStart hook for the "compact" matcher: re-reads the active campaign.

After a compaction the orchestrator has lost its working context. If
`.claude/state/active-campaign` names a campaign whose
`docs/analysis/<name>/RESUME.md` exists, print the SessionStart hook JSON with a
pointer to the ledger and the first 40 lines of RESUME.md as additional context.
Otherwise print nothing.

Never fails the session: every error ends with exit 0 and no output.
"""

import json
import os
import sys
from pathlib import Path

CARD_LINES = 40
POINTER = (
    "Context was compacted. Active campaign: {name}. "
    "Read docs/analysis/{name}/RESUME.md and the ledger README before acting."
)


def project_dir():
    """The project root that Claude Code exports, or the working directory."""
    return Path(os.environ.get("CLAUDE_PROJECT_DIR") or os.getcwd())


def build_card(root):
    """Return the card text, or None when there is no usable pointer."""
    pointer = root / ".claude" / "state" / "active-campaign"
    if not pointer.is_file():
        return None
    lines = pointer.read_text(encoding="utf-8", errors="replace").strip().splitlines()
    if not lines:
        return None
    name = lines[0].strip()
    # The name is a folder under docs/analysis/, never a path.
    if name in ("", ".", "..") or "/" in name or "\\" in name:
        return None
    resume = root / "docs" / "analysis" / name / "RESUME.md"
    if not resume.is_file():
        return None
    text = resume.read_text(encoding="utf-8", errors="replace")
    body = text.splitlines()[:CARD_LINES]
    return "\n".join([POINTER.format(name=name), *body])


def main():
    try:
        # The hook input is not needed; the card depends only on the pointer.
        sys.stdin.read()
        card = build_card(project_dir())
        if card is None:
            return 0
        output = {
            "hookSpecificOutput": {
                "hookEventName": "SessionStart",
                "additionalContext": card,
            }
        }
        print(json.dumps(output))
    except Exception:
        return 0
    return 0


if __name__ == "__main__":
    sys.exit(main())
