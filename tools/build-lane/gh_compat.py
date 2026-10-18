"""Fallbacks for a gh older than ship.py assumes: Ubuntu's apt gh 2.45 has no
`gh pr checks --json`, and its `gh pr edit` fails on a classic-projects GraphQL field.
Each function takes ship.py's `gh(*args, cwd=..., check=...)`, as ship_queue.py does."""

from __future__ import annotations

import json

OLD_GH = ("unknown flag", "unknown shorthand flag")
FAILED = {"FAILURE", "TIMED_OUT", "ACTION_REQUIRED", "STARTUP_FAILURE", "STALE", "ERROR"}


def bucket(item: dict) -> str:
    """`gh pr checks`' bucket (pass, fail, pending, skipping, cancel) for one
    statusCheckRollup entry: a CheckRun (status, conclusion) or a StatusContext (state)."""
    if item.get("__typename") == "StatusContext" or "context" in item:
        state = (item.get("state") or "").upper()
        return "pass" if state == "SUCCESS" else "fail" if state in FAILED else "pending"
    if (item.get("status") or "").upper() != "COMPLETED":
        return "pending"
    conclusion = (item.get("conclusion") or "").upper()
    if conclusion in ("SUCCESS", "NEUTRAL"):
        return "pass"
    if conclusion == "SKIPPED":
        return "skipping"
    if conclusion == "CANCELLED":
        return "cancel"
    return "fail" if conclusion in FAILED else "pending"


def checks(gh, pr: str, repo: str) -> list[dict]:
    """[{name, bucket}] for the PR's checks; [] while none are reported."""
    p = gh("pr", "checks", pr, "--json", "name,bucket", cwd=repo, check=False)
    if p.returncode != 0 and any(h in (p.stderr or "") + (p.stdout or "") for h in OLD_GH):
        p = gh("pr", "view", pr, "--json", "statusCheckRollup", cwd=repo, check=False)
        try:
            items = json.loads(p.stdout or "{}").get("statusCheckRollup") or []
        except json.JSONDecodeError:
            return []
        return [{"name": i.get("name") or i.get("context") or "", "bucket": bucket(i)} for i in items]
    try:
        return json.loads(p.stdout or "[]")
    except json.JSONDecodeError:
        return []   # "no checks reported" yet


def set_base(gh, pr: str, base: str, repo: str) -> bool:
    """Retarget a PR, through the REST API when `gh pr edit` fails (an old gh)."""
    if gh("pr", "edit", pr, "--base", base, cwd=repo, check=False).returncode == 0:
        return True
    return gh("api", "-X", "PATCH", f"repos/{{owner}}/{{repo}}/pulls/{pr}", "-f", f"base={base}",
              cwd=repo, check=False).returncode == 0
