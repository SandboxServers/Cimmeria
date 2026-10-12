"""Packet tables of the campaign ledgers (docs/analysis/<campaign>/README.md, work-packets*.md), parsed and normalized.

A packet table is a markdown table whose header has an `ID` and a `Packet` cell; other tables (decisions, questions)
are skipped. Columns are mapped by name, and a missing column is an empty string.
"""

import re

STATUSES = ("Ready", "BlockedDependency", "BlockedDecision", "Writing", "Review", "Integrated", "UATPending", "Done")
IMPLEMENTERS = ("packet-coder", "rust-gameserver-dev", "documentation-writer", "coordinator", "sonnet")
ESCALATION_PHRASES = ("ran out of context", "stalled", "took over", "finished by", "escalated", "haiku")
RETRY_KEYWORDS = (
    ("context-exhausted", r"ran out of context|context cap|out of context"),
    ("stalled-background", r"background|waiting for the notification|stalled"),
    ("permission-denied", r"denied|permission"),
    ("review-fix", r"review|finding"),
    ("ci-failure", r"\bci\b|clippy|nextest failed"),
    ("merge-conflict", r"conflict|rebase"),
)
COLUMNS = {"id": "id", "packet": "title", "implementer": "implementer_raw", "size": "size", "wave": "wave",
           "depends on": "depends", "status": "status_raw"}


def _cells(line):
    return [c.strip() for c in line.strip().strip("|").split("|")]


def parse_packet_table(text):
    """Every packet table in `text`, one dict per row: id, title, implementer_raw, size, wave, depends, status_raw."""
    rows = []
    cols = None
    in_table = False
    for line in text.splitlines():
        if not line.lstrip().startswith("|"):
            in_table = False
            continue
        cells = _cells(line)
        if not in_table:
            in_table = True
            names = [c.lower() for c in cells]
            cols = None
            if {"id", "packet"} <= set(names):
                cols = [(i, COLUMNS[n]) for i, n in enumerate(names) if n in COLUMNS]
            continue
        if cols is None or set("".join(cells)) <= set("-: "):
            continue
        row = {name: "" for name in COLUMNS.values()}
        for i, name in cols:
            if i < len(cells):
                row[name] = cells[i]
        if row["id"]:
            rows.append(row)
    return rows


def normalize(row):
    """Status (or `unknown`), PR numbers, implementer kind, its parenthetical note, and whether it was escalated."""
    status_word = re.match(r"[A-Za-z]+", row["status_raw"])
    status = status_word.group(0) if status_word and status_word.group(0) in STATUSES else "unknown"
    implementer_raw = row["implementer_raw"]
    lowered = implementer_raw.lower()
    implementer = next((name for name in IMPLEMENTERS if lowered.startswith(name)), "other")
    note = re.search(r"\(([^)]*)\)", implementer_raw)
    escalated = implementer not in ("packet-coder", "other") and any(p in lowered for p in ESCALATION_PHRASES)
    return {"id": row["id"], "title": row["title"], "status": status, "status_raw": row["status_raw"],
            "prs": [int(n) for n in re.findall(r"#(\d+)", row["status_raw"])], "implementer": implementer,
            "implementer_note": note.group(1) if note else None, "escalated": escalated,
            "size": row["size"], "wave": row["wave"], "depends": row["depends"]}


def retry_reason(text):
    """Why a packet was retried, from the first keyword found in `text` (a note, a status, or a worknote's head)."""
    lowered = (text or "").lower()
    return next((reason for reason, pattern in RETRY_KEYWORDS if re.search(pattern, lowered)), "other")


def campaign_pr_map(rows_by_campaign):
    """(pr -> campaign, conflicts): a PR cited by one campaign maps to it; one cited by two goes to `conflicts`."""
    owners = {}
    for campaign, rows in sorted(rows_by_campaign.items()):
        for row in rows:
            for pr in row["prs"]:
                owners.setdefault(pr, set()).add(campaign)
    pr_map = {pr: next(iter(c)) for pr, c in owners.items() if len(c) == 1}
    conflicts = [{"pr": pr, "campaigns": sorted(c)} for pr, c in sorted(owners.items()) if len(c) > 1]
    return pr_map, conflicts


def campaign_window(campaign, git_log):
    """{start, end} for a campaign: its folder's first commit, and its newest close-out commit.

    `git_log(args) -> str` runs git log in the repository; `end` is None when no close-out commit exists.
    """
    path = f"docs/analysis/{campaign}/"
    first = git_log(["log", "--reverse", "--format=%aI", "--", path]).split()
    end = None
    for line in git_log(["log", "--format=%aI%x09%s", "--", path]).splitlines():
        date, _, subject = line.partition("\t")
        if re.search(r"close-?out", subject, re.IGNORECASE):
            end = date
            break
    return {"start": first[0] if first else None, "end": end}
