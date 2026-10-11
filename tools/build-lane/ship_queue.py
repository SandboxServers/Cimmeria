"""GitHub state ship.py merges by: main's merge queue, the `main-red` issues, and the
PRs stacked on a branch. Lookups only; ship.py owns the loops, the waiting and the exits.

Every function takes ship.py's `gh(*args, cwd=..., check=...)` so its log, its fake in
the tests and its error handling apply here too.
"""

from __future__ import annotations

import json
import os

# One query for everything wait_for_queue needs. `{owner}` and `{repo}` are gh api
# placeholders for the repository of the current directory.
QUEUE_QUERY = """query($owner: String!, $repo: String!, $n: Int!) {
  repository(owner: $owner, name: $repo) {
    pullRequest(number: $n) { state mergedAt isInMergeQueue autoMergeRequest { enabledAt } }
  }
}"""


def queue_required(gh, repo: str, branch: str = "main") -> bool | None:
    """Does `branch` require a merge queue? None when the lookup failed: guessing no would
    send a docs PR through `--admin`, around the queue. $SHIP_MERGE_QUEUE (1/0) overrides."""
    forced = os.environ.get("SHIP_MERGE_QUEUE", "").strip()
    if forced in ("0", "1"):
        return forced == "1"
    p = gh("api", f"repos/{{owner}}/{{repo}}/rules/branches/{branch}",
           "--jq", '[.[] | select(.type == "merge_queue")] | length', cwd=repo, check=False)
    if p.returncode != 0 or not p.stdout.strip().isdigit():
        return None
    return p.stdout.strip() != "0"


def red_issues(gh, repo: str) -> list[dict]:
    """Open `main-red` issues (tools/main-red/main-red.sh): a post-merge check is failing."""
    p = gh("issue", "list", "--label", "main-red", "--state", "open", "--json", "number,title,url",
           cwd=repo, check=False)
    if p.returncode != 0:
        return []   # a lookup failure must not block every merge; the issue itself stays loud
    try:
        return json.loads(p.stdout or "[]")
    except json.JSONDecodeError:
        return []


def queue_status(gh, pr: str, repo: str) -> dict | None:
    """{state, merged_at, in_queue, auto} for one PR; None when the lookup failed (a
    network blip during an hour-long wait is not a verdict)."""
    p = gh("api", "graphql", "-F", "owner={owner}", "-F", "repo={repo}", "-F", f"n={pr}",
           "-f", f"query={QUEUE_QUERY}", cwd=repo, check=False)
    try:
        node = json.loads(p.stdout)["data"]["repository"]["pullRequest"]
    except (json.JSONDecodeError, KeyError, TypeError):
        return None
    return {"state": node["state"], "merged_at": node.get("mergedAt") or "",
            "in_queue": bool(node.get("isInMergeQueue")), "auto": node.get("autoMergeRequest") is not None}


def stacked_children(gh, git, repo: str, parent_branch: str, parent_head: str) -> list[dict]:
    """Open PRs whose head contains `parent_head` (the parent's head as it merged) and
    that target the parent's branch, or main when the parent's branch is gone (GitHub
    retargets its PRs to main when it deletes the branch on merge). `git(*args, check=)`
    runs in the main checkout."""
    p = gh("pr", "list", "--state", "open", "--limit", "200",
           "--json", "number,headRefName,headRefOid,baseRefName", cwd=repo, check=False)
    if p.returncode != 0:
        return []
    gone = git("ls-remote", "--exit-code", "origin", f"refs/heads/{parent_branch}", check=False).returncode == 2
    bases = (parent_branch, "main") if gone else (parent_branch,)
    found = []
    for c in json.loads(p.stdout or "[]"):
        if c["baseRefName"] not in bases or c["headRefName"] == parent_branch:
            continue
        # Fetch only heads this clone lacks (worktree branches are usually local), one
        # ref per fetch: a missing ref fails only its own fetch, not every child's.
        ref = c["headRefName"]
        if git("cat-file", "-e", c["headRefOid"] + "^{commit}", check=False).returncode != 0:
            git("fetch", "-q", "origin", f"+refs/heads/{ref}:refs/remotes/origin/{ref}", check=False)
        if git("merge-base", "--is-ancestor", parent_head, c["headRefOid"], check=False).returncode == 0:
            found.append(c)
    return found
