"""Everything pr_stats reads from or writes to GitHub, through the `gh` CLI.

Reads: the PR (state, merge time, diff stats, commits, reviews), its workflow
runs, and its timeline's cross-references. Writes: one issue comment, found by
its marker and edited in place, never a second one.

`GitHub(run=...)` takes the function that runs `gh`, so tests drive it with a
fake and never touch the network.
"""

import json
import re
import subprocess
from urllib.parse import quote

MARKER = "<!-- cimmeria-pr-stats:v1 -->"

PR_FIELDS = "number,state,mergedAt,headRefName,additions,deletions,changedFiles,commits,reviews"
# Run conclusions that make a CI round a failed one. `cancelled` is a superseded
# run, not a failure.
FAILED = {"failure", "timed_out", "startup_failure"}
# Conventional-commit titles. A follow-up fix is a merged PR titled fix(...) that
# references this one after it merged; a revert is one titled Revert ....
FIX_TITLE = re.compile(r"(?i)^(?:fix|hotfix)(?:\([^)]*\))?!?:")
REVERT_TITLE = re.compile(r"(?i)^revert\b")


class GhError(Exception):
    """A gh call failed. The message names the call, never the token or the payload."""


def run_gh(args, stdin=None):
    try:
        p = subprocess.run(["gh", *args], input=stdin, capture_output=True, text=True, encoding="utf-8",
                           timeout=120)
    except (OSError, subprocess.SubprocessError) as e:
        raise GhError(f"gh {args[0]} could not run: {type(e).__name__}") from None
    if p.returncode != 0:
        raise GhError(f"gh {args[0]} exited {p.returncode}: {p.stderr.strip()[:200]}")
    return p.stdout


class GitHub:
    def __init__(self, repo, run=run_gh):
        self.repo = repo
        self._run = run

    def _json(self, args, stdin=None):
        out = self._run(args, stdin)
        return json.loads(out) if out.strip() else None

    def _pages(self, path):
        """Every item of a paginated list endpoint."""
        pages = self._json(["api", path, "--paginate", "--slurp"]) or []
        items = []
        for page in pages:
            items.extend(page.get("workflow_runs", []) if isinstance(page, dict) else page)
        return items

    # -- reads ---------------------------------------------------------------
    def pr(self, number):
        return self._json(["pr", "view", str(number), "--repo", self.repo, "--json", PR_FIELDS])

    def runs(self, branch):
        return self._pages(f"repos/{self.repo}/actions/runs?branch={quote(branch, safe='')}&per_page=100")

    def timeline(self, number):
        return self._pages(f"repos/{self.repo}/issues/{number}/timeline?per_page=100")

    def stats_comments(self, number):
        """[(id, body)] of the comments that carry the marker, oldest first."""
        comments = self._pages(f"repos/{self.repo}/issues/{number}/comments?per_page=100")
        return [(c["id"], c["body"]) for c in comments if (c.get("body") or "").startswith(MARKER)]

    # -- writes --------------------------------------------------------------
    def create_comment(self, number, body):
        out = self._json(["api", "-X", "POST", f"repos/{self.repo}/issues/{number}/comments", "--input", "-"],
                         json.dumps({"body": body}))
        return out["id"]

    def edit_comment(self, comment_id, body):
        self._json(["api", "-X", "PATCH", f"repos/{self.repo}/issues/comments/{comment_id}", "--input", "-"],
                   json.dumps({"body": body}))

    def upsert(self, number, body):
        """Write `body` as the PR's one stats comment. Returns (status, comment_id, duplicates).

        status is created, updated or unchanged. An existing comment is edited in
        place, and an identical one is left alone. If earlier bugs left several,
        the oldest is the one kept up to date and the rest are only counted.
        """
        existing = self.stats_comments(number)
        if not existing:
            return "created", self.create_comment(number, body), 0
        cid, old = existing[0]
        if old == body:
            return "unchanged", cid, len(existing) - 1
        self.edit_comment(cid, body)
        return "updated", cid, len(existing) - 1

    # -- the quality fields --------------------------------------------------
    def quality(self, number):
        """(quality dict, pr dict) for one PR."""
        pr = self.pr(number) or {}
        runs = self.runs(pr["headRefName"]) if pr.get("headRefName") else []
        return quality(number, self.repo, pr, runs, self.timeline(number)), pr


def quality(number, repo, pr, runs, timeline):
    """The cimmeria-pr-stats/1 quality fields, from gh's answers.

    - ci_rounds: head commits of this PR that ran any workflow. ci_fail_rounds:
      those where at least one run failed, timed out or failed to start.
    - review_rounds: commits that received at least one submitted review, so two
      bots reviewing the same push are one round.
    - followup_fix_prs: merged fix PRs that cross-reference this PR after it merged.
    - reverted: a merged PR titled Revert ... cross-references this PR.
    """
    shas = {c["oid"] for c in pr.get("commits") or []}
    rounds = {}
    for r in runs:
        sha = r.get("head_sha")
        if sha and (not shas or sha in shas):
            rounds.setdefault(sha, set()).add(r.get("conclusion"))
    reviews = {(r.get("commit") or {}).get("oid") or r.get("submittedAt")
               for r in pr.get("reviews") or [] if r.get("state") not in (None, "PENDING")}

    merged_at = pr.get("mergedAt")
    fixes, reverted = set(), False
    for e in timeline:
        if e.get("event") != "cross-referenced":
            continue
        src = (e.get("source") or {}).get("issue") or {}
        merged = (src.get("pull_request") or {}).get("merged_at")
        if not merged or src.get("number") == number or not str(src.get("repository_url", "")).endswith("/" + repo):
            continue
        title = src.get("title") or ""
        if REVERT_TITLE.match(title):
            reverted = True
        elif FIX_TITLE.match(title) and merged_at and (e.get("created_at") or "") >= merged_at:
            fixes.add(src["number"])
    return {
        "ci_fail_rounds": sum(1 for c in rounds.values() if c & FAILED),
        "review_rounds": len(reviews),
        "followup_fix_prs": sorted(fixes),
        "reverted": reverted,
        "ci_rounds": len(rounds),
    }
