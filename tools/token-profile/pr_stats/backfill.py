"""The backfill: every merged PR since a date, rate-limited and resumable.

PRs come from the profiler database's prs table (a PR the ingest has not seen
has no data anyway), oldest merge first. A state file records each PR's
outcome per mode, so a run that stops, by Ctrl-C, errors or the error limit,
picks up where it left off. Dry run is the default.
"""

import json
import os
import re
import time
from pathlib import Path

# Outcomes a resumed run does not redo. Errors and gate refusals are retried.
DONE = {"created", "updated", "unchanged", "printed", "no-data"}


def parse_rate(text):
    """'6', '6/min' or '120/h' -> PRs per minute."""
    m = re.fullmatch(r"\s*(\d+(?:\.\d+)?)\s*(?:/\s*(min|m|h|hour))?\s*", str(text))
    if not m or float(m.group(1)) <= 0:
        raise ValueError(f"rate must look like 6/min or 120/h, got {text!r}")
    n = float(m.group(1))
    return n / 60 if m.group(2) in ("h", "hour") else n


class RateLimiter:
    """At most `per_minute` calls of wait() per minute, evenly spaced."""

    def __init__(self, per_minute, clock=time.monotonic, sleep=time.sleep):
        self.interval = 60.0 / per_minute
        self.clock, self.sleep = clock, sleep
        self._next = None

    def wait(self):
        now = self.clock()
        if self._next is not None and now < self._next:
            self.sleep(self._next - now)
            now = self._next
        self._next = now + self.interval


class State:
    """{mode: {pr: outcome}} in a JSON file. Holds PR numbers and outcomes, nothing else."""

    def __init__(self, path):
        self.path = Path(path)
        self.data = {"version": 1, "outcomes": {}}
        if self.path.is_file():
            self.data = json.loads(self.path.read_text(encoding="utf-8"))

    def done(self, mode, pr):
        return self.data["outcomes"].get(mode, {}).get(str(pr)) in DONE

    def record(self, mode, pr, outcome):
        self.data["outcomes"].setdefault(mode, {})[str(pr)] = outcome
        tmp = self.path.with_name(self.path.name + ".tmp")
        tmp.write_text(json.dumps(self.data, indent=1, sort_keys=True) + "\n", encoding="utf-8")
        os.replace(tmp, self.path)


def merged_prs(db, since):
    return [r[0] for r in db.execute(
        "SELECT pr_number FROM prs WHERE state = 'MERGED' AND merged_at >= ? ORDER BY merged_at, pr_number",
        (since,))]


def backfill(prs, run_one, limiter, state, mode, emit, max_errors=3):
    """Run `run_one(pr, before_gh)` -> outcome over `prs`. Returns {outcome: count}.

    Stops after `max_errors` gh errors in a row (a rate limit or an outage looks
    like that); the state file lets the next run resume.
    """
    counts, streak = {}, 0
    for pr in prs:
        if state.done(mode, pr):
            counts["skipped"] = counts.get("skipped", 0) + 1
            continue
        outcome = run_one(pr, limiter.wait)
        state.record(mode, pr, outcome)
        counts[outcome] = counts.get(outcome, 0) + 1
        emit(f"pr={pr} status={outcome}")
        streak = streak + 1 if outcome == "error" else 0
        if streak >= max_errors:
            emit(f"status=stopped after {streak} errors in a row; run again to resume")
            counts["stopped"] = 1
            break
    return counts
