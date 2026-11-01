"""The `resume-cards` harness subcommand: how often a compaction's resume card names the wrong campaign (C2).

Each main session transcript is read in order. A compaction is a `compact_boundary` record in the window. Its card is
the first `attachment` record before the next assistant turn that names a campaign. The session's own campaign is
counted from the last 40 Agent prompts before the boundary (the C5 tier-2 counting, without the RESUME.md filter).
A card that names that campaign is `matched`, one that names another is `wrong`, a split session is `mixed`, and a
session with fewer than three naming prompts is `undetermined`.
"""

import json
import re
import sys
from collections import Counter, deque
from pathlib import Path

from report.scrub import PrivacyError, Scrubber

from .transcripts import iter_main_transcripts, records

SCHEMA = "cimmeria-harness-resume-cards/1"
CARD = re.compile(r"(Active|Inferred) campaign[^:]*: ([a-z0-9][a-z0-9.-]*[a-z0-9])")
DOCS = re.compile(r"docs[\\/]+analysis[\\/]+([a-z0-9][a-z0-9.-]*[a-z0-9])")
REPORTS = re.compile(r"\.claude[\\/]+reports[\\/]+([a-z0-9][a-z0-9.-]*[a-z0-9])")
LAST_PROMPTS = 40
TIERS = ("active", "inferred")
VERDICTS = ("matched", "wrong", "mixed", "undetermined")
COUNTS = ("firings", *VERDICTS)


def _empty() -> dict:
    """The totals of a run, all zero: the C2 counts and the per-tier verdict counts."""
    totals = {"compactions": 0, "read_resume_before_dispatch": 0, "compactions_followed_by_dispatch": 0}
    totals.update({k: 0 for k in COUNTS})
    totals["by_tier"] = {t: {k: 0 for k in COUNTS} for t in TIERS}
    return totals


def _in_window(ts: str, since, until) -> bool:
    """Whether a timestamp is in [since, until). The strings compare as dates, as iter_dispatches does."""
    return (since is None or ts >= since) and (until is None or ts < until)


def _names(tool_input) -> set[str]:
    """The campaign names an Agent prompt names through its docs/analysis or .claude/reports folders."""
    text = json.dumps(tool_input).lower()
    return set(DOCS.findall(text)) | set(REPORTS.findall(text))


def _verdict(card_name: str, prompts: list[set[str]]) -> str:
    """The verdict of a card against the name sets of the session's prompts before its compaction."""
    naming = sum(1 for s in prompts if s)
    if naming < 3:
        return "undetermined"
    counts = Counter(n for s in prompts for n in s)
    top, c = max(sorted(counts.items()), key=lambda kv: kv[1])
    if 3 * c < 2 * naming:
        return "mixed"
    return "matched" if card_name == top else "wrong"


class _Compaction:
    """One compaction in the window, with what follows it up to the next compaction."""

    def __init__(self, prompts: list[set[str]]):
        self.prompts = prompts      # the name sets of the Agent prompts before it
        self.in_card = True         # still before the first assistant turn after it
        self.card = None            # (tier, campaign) of the card: the first match only
        self.resume_read = False    # a Read of RESUME.md before any dispatch
        self.dispatched = False     # an Agent use before the next compaction


def _finish(c: _Compaction, totals: dict) -> None:
    """Add one finished compaction to `totals`."""
    if c.card is not None:
        tier, name = c.card
        verdict = _verdict(name, c.prompts)
        for bucket in (totals, totals["by_tier"][tier]):
            bucket["firings"] += 1
            bucket[verdict] += 1
    totals["read_resume_before_dispatch"] += c.resume_read
    totals["compactions_followed_by_dispatch"] += c.dispatched


def _scan(path, since, until, totals: dict) -> None:
    """Fold one main transcript into `totals`. Any compaction closes the open one; only one in the window opens."""
    prompts: deque[set[str]] = deque(maxlen=LAST_PROMPTS)
    open_c = None
    for rec in records(path):
        kind = rec.get("type")
        if kind == "system" and rec.get("subtype") == "compact_boundary":
            if open_c is not None:
                _finish(open_c, totals)
                open_c = None
            ts = rec.get("timestamp") if isinstance(rec.get("timestamp"), str) else ""
            if _in_window(ts, since, until):
                open_c = _Compaction(list(prompts))
                totals["compactions"] += 1
            continue
        if open_c is not None and kind == "attachment" and open_c.in_card and open_c.card is None:
            m = CARD.search(json.dumps(rec))
            if m:
                open_c.card = ("inferred" if m.group(1) == "Inferred" else "active", m.group(2))
        if kind == "assistant":
            if open_c is not None:
                open_c.in_card = False
            content = rec.get("message", {}).get("content") if isinstance(rec.get("message"), dict) else None
            for block in content if isinstance(content, list) else ():
                if not isinstance(block, dict) or block.get("type") != "tool_use":
                    continue
                inp = block.get("input") if isinstance(block.get("input"), dict) else {}
                if block.get("name") == "Agent":
                    prompts.append(_names(inp))
                    if open_c is not None:
                        open_c.dispatched = True
                elif block.get("name") == "Read" and open_c is not None and not open_c.dispatched:
                    if isinstance(inp.get("file_path"), str) and inp["file_path"].endswith("RESUME.md"):
                        open_c.resume_read = True
    if open_c is not None:
        _finish(open_c, totals)


def run(args) -> int:
    """Write resume-cards.json for the window and print the status line. Returns 0, 2 on IO error, 3 on privacy."""
    totals = _empty()
    try:
        for _session_id, path in iter_main_transcripts(args.projects_root, args.prefix):
            _scan(path, args.since, args.until, totals)
    except OSError as e:
        print(f"status=error {type(e).__name__}", file=sys.stderr)
        return 2
    record = {"schema": SCHEMA, "window": {"since": args.since, "until": args.until}, **totals}
    text = json.dumps(record, indent=2, sort_keys=True) + "\n"
    try:
        Scrubber().assert_clean(text, "resume-cards.json")
    except PrivacyError:
        print("status=error privacy", file=sys.stderr)
        return 3
    try:
        out = Path(args.out)
        out.mkdir(parents=True, exist_ok=True)
        (out / "resume-cards.json").write_text(text, encoding="utf-8", newline="\n")
    except OSError as e:
        print(f"status=error {type(e).__name__}", file=sys.stderr)
        return 2
    wrong_pct = 100 * totals["wrong"] / totals["firings"] if totals["firings"] else 0.0
    print(f"status=ok resume-cards compactions={totals['compactions']} firings={totals['firings']} "
          f"wrong={totals['wrong']} wrong_pct={wrong_pct:.1f}")
    return 0
