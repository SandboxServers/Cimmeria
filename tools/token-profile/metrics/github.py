"""GitHub collector for the campaign metrics: PRs, CI per head commit, merge-queue CI, escaped defects.

Every read goes through `run_gh(args) -> object`, which runs gh and parses its
JSON. Tests and `--from-json` inject a different one (`replay`). Nothing here
writes to GitHub. Branch names are read to tag a PR's campaign and then dropped:
no record returned from here carries one.

`ci` is the PR-push gate on the head commits that survive in `pulls/<n>/commits`;
`queue` is the merge-queue result of the same PR. A rebase or force-push drops the
commits it replaces and their earlier red runs, so `ci` can read better than the
history did. Only a tagged PR gets per-PR calls; an untagged one keeps its list fields.
"""

import hashlib
import json
import re
import subprocess
import time
from datetime import datetime
from pathlib import Path

# `files` is not listed: `gh pr list --limit 1000 --json files` fails in gh itself. code_pr() reads the paths instead.
PR_FIELDS = ("number,title,body,headRefName,baseRefName,createdAt,mergedAt,closedAt,state,additions,deletions,"
             "changedFiles")
ISSUE_FIELDS = "number,body,createdAt,closedAt,state,labels,stateReason"
CODE_PREFIXES = ("crates/", "tools/", "db/", ".github/")
GATE = "ci gate"
FAILED = {"failure", "timed_out", "startup_failure", "action_required"}
# An issue closed this way is not a defect of the PR it cites.
NOT_A_DEFECT = {"NOT_PLANNED", "DUPLICATE"}
# gh returns at most 1000 items per search; at LIMIT the oldest would be silently lost, so the run stops instead.
LIMIT = 1000
# A fix or a revert, by title. A revert of a PR quotes its title: Revert "<title>". `\b` keeps "fixture" out.
FIX_OR_REVERT = re.compile(r"(?i)^(fix|revert)\b")
REVERT_OF = re.compile(r'^Revert "(.*)"$')
CITE = re.compile(r"#(\d+)\b")
# A merge-queue run tests a temporary branch named for the PR it lands, e.g. gh-readonly-queue/main/pr-1664-<sha>.
QUEUE_BRANCH = re.compile(r"gh-readonly-queue/[^/]+/pr-(\d+)-[0-9a-f]+")
MAX_COMMITS = 100
PAGE = "?per_page=100"
PAUSE_S = 0.2
RETRY_WAITS_S = (60, 120, 300)
REPLAY_FILES = ("gh_prs.json", "gh_checks.json", "gh_issues.json")


class GhError(Exception):
    """A gh call failed. The message names the call, never the token or the payload.

    `retryable` marks a rate limit or a server error, the only failures `polite` retries.
    """

    def __init__(self, message, retryable=False):
        super().__init__(message)
        self.retryable = retryable


def run_gh(args, cwd=None):
    """Run `gh <args>` and return its parsed JSON output (None when it prints nothing)."""
    try:
        p = subprocess.run(["gh", *args], cwd=cwd, capture_output=True, text=True, encoding="utf-8", timeout=120)
    except (OSError, subprocess.SubprocessError) as e:
        raise GhError(f"gh {args[0]} could not run: {type(e).__name__}") from None
    if p.returncode != 0:
        # Classified on the full stderr: a secondary rate limit's status code can sit past the truncated message.
        retryable = bool(re.search(r"HTTP (403|429|5\d\d)", p.stderr)) or "rate limit" in p.stderr.lower()
        raise GhError(f"gh {args[0]} exited {p.returncode}: {p.stderr.strip()[:200]}", retryable=retryable)
    try:
        return json.loads(p.stdout) if p.stdout.strip() else None
    except ValueError:
        raise GhError(f"gh {args[0]} printed invalid JSON") from None


def polite(run, cache_dir=None, stats=None):
    """Wrap `run` with the rate-limit rules: a pause before each call, and a retry after RETRY_WAITS_S on a retryable error.

    An `api` answer is cached as JSON in `cache_dir` (keyed by the call's arguments), so a run that stopped
    part way resumes from the answers it already has. `stats` ({gh_calls, cache_hits}) counts the work done.
    """
    stats = stats if stats is not None else {"gh_calls": 0, "cache_hits": 0}

    def call(args):
        cached = None
        if cache_dir and args[0] == "api":
            key = hashlib.sha256(json.dumps(args).encode()).hexdigest()
            cached = Path(cache_dir) / f"{key}.json"
            if cached.is_file():
                stats["cache_hits"] += 1
                return json.loads(cached.read_text(encoding="utf-8"))
        for attempt in range(len(RETRY_WAITS_S) + 1):
            if attempt:
                time.sleep(RETRY_WAITS_S[attempt - 1])
            time.sleep(PAUSE_S)
            stats["gh_calls"] += 1
            try:
                result = run(args)
                break
            except GhError as e:
                if not e.retryable or attempt == len(RETRY_WAITS_S):
                    raise
        if cached:
            cached.parent.mkdir(parents=True, exist_ok=True)
            cached.write_text(json.dumps(result), encoding="utf-8")
        return result

    return call


def replay(folder):
    """A `run` that answers from the three saved files in `folder` instead of calling gh.

    gh_prs.json is `gh pr list` output, gh_issues.json is `gh issue list` output, and
    gh_checks.json maps each API path (commits, check-runs, files) to the pages
    `gh api --paginate --slurp` printed.
    """
    saved = {}
    for name in REPLAY_FILES:
        path = Path(folder) / name
        if not path.is_file():
            raise GhError(f"replay folder has no {name}")
        saved[name] = json.loads(path.read_text(encoding="utf-8"))

    def run(args):
        if args[:2] == ["pr", "list"]:
            return saved["gh_prs.json"]
        if args[:2] == ["issue", "list"]:
            return saved["gh_issues.json"]
        if args[0] == "api" and args[1] in saved["gh_checks.json"]:
            return saved["gh_checks.json"][args[1]]
        raise GhError(f"replay has no answer for gh {args[0]} {args[1] if len(args) > 1 else ''}".rstrip())

    return run


def pages(run, path, key=None):
    """Every item of a paginated API list. `key` names the list inside each page, when a page is an object."""
    items = []
    for page in run(["api", path, "--paginate", "--slurp"]) or []:
        items.extend(page[key] if key else page)
    return items


def _when(text):
    return datetime.fromisoformat(text) if text else None


def _labels(issue):
    return {x["name"] if isinstance(x, dict) else x for x in issue.get("labels") or []}


def _apart(a, b):
    """True when both campaigns are known and differ. An untagged PR is never apart from anything."""
    return a is not None and b is not None and a != b


def _verdict(conclusion):
    """`success`, `failure` for a conclusion in FAILED, else `none`: cancelled, skipped and undecided give no verdict."""
    return "success" if conclusion == "success" else "failure" if conclusion in FAILED else "none"


def _window(since, until):
    """The gh search qualifiers for a window: created on or after `since`, and before `until` when it is given."""
    return f"created:>={since}" + (f" created:<{until[:10]}" if until else "")


def _before(created, until):
    """True when an item created at `created` falls before `until` (an ISO timestamp), or when there is no bound."""
    return until is None or _when(created) < _when(until)


# -- reads -------------------------------------------------------------------
def pr_record(raw, campaigns=None):
    """The PR fields the metrics use. Branch names become `campaign`; they are not kept."""
    tag = None
    if campaigns:
        tag = (campaigns.of_branch(raw.get("headRefName")) or campaigns.of_title(raw.get("title"))
               or campaigns.of_branch(raw.get("baseRefName")))
    return {
        "number": raw["number"],
        "title": raw.get("title") or "",
        "body": raw.get("body") or "",
        "createdAt": raw.get("createdAt"),
        "mergedAt": raw.get("mergedAt"),
        "state": raw.get("state"),
        "additions": raw.get("additions"),
        "deletions": raw.get("deletions"),
        "changedFiles": raw.get("changedFiles"),
        "campaign": tag,
    }


def collect_prs(run, since, campaigns=None, until=None):
    """Every PR created in the window (`since` YYYY-MM-DD, `until` an ISO timestamp), as pr_record() dicts.

    `campaigns` is an ingest.campaigns.Campaigns.
    """
    raw = run(["pr", "list", "--state", "all", "--search", _window(since, until), "--limit", str(LIMIT),
               "--json", PR_FIELDS]) or []
    if len(raw) >= LIMIT:
        raise GhError(f"gh returned {LIMIT} PRs: narrow the window")
    return [pr_record(p, campaigns) for p in raw if _before(p["createdAt"], until)]


def collect_issues(run, since, until=None):
    """Closed issues labelled bug and created in the window, as gh returned them."""
    raw = run(["issue", "list", "--state", "closed", "--label", "bug", "--search", _window(since, until),
               "--limit", str(LIMIT), "--json", ISSUE_FIELDS]) or []
    if len(raw) >= LIMIT:
        raise GhError(f"gh returned {LIMIT} issues: narrow the window")
    return [i for i in raw if _before(i["createdAt"], until)]


def head_commits(run, pr):
    """The PR's commits, oldest first, as {sha, committed}. Past MAX_COMMITS, the first and the last 50 are kept.

    The first half feeds first_attempt and the last half feeds overall, so the cap keeps both ends.
    """
    rows = pages(run, f"repos/{{owner}}/{{repo}}/pulls/{pr['number']}/commits{PAGE}")
    commits = sorted(({"sha": c["sha"], "committed": c["commit"]["committer"]["date"]} for c in rows),
                     key=lambda c: c["committed"])
    if len(commits) > MAX_COMMITS:
        half = MAX_COMMITS // 2
        commits = commits[:half] + commits[-half:]
    return commits


def code_pr(run, number):
    """True when the PR changes a path under CODE_PREFIXES (the crates, tools, db and CI trees)."""
    files = pages(run, f"repos/{{owner}}/{{repo}}/pulls/{number}/files{PAGE}")
    return any((f.get("filename") or "").startswith(CODE_PREFIXES) for f in files)


def ci_result(run, sha):
    """The `ci gate` verdict for one commit, and the jobs that failed: {gate, failed_jobs}.

    gate is `none` when no check run is named `ci gate` (docs-only commits may skip
    it), it has not finished, or it was cancelled or skipped (a superseded push). `none`
    is not a failure; only a conclusion in FAILED is one.

    The verdict is per commit, not per run: check-runs returns only the latest run of each
    name (`filter=latest`), so a re-run of a flaky job replaces its red run.
    """
    runs = pages(run, f"repos/{{owner}}/{{repo}}/commits/{sha}/check-runs{PAGE}", key="check_runs")
    gates = sorted((r for r in runs if r.get("name") == GATE), key=lambda r: r.get("started_at") or "")
    conclusion = gates[-1].get("conclusion") if gates else None
    failed = sorted({r["name"] for r in runs if r.get("name") != GATE and r.get("conclusion") in FAILED})
    return {"gate": _verdict(conclusion), "failed_jobs": failed}


def ci_summary(pr, commits, results):
    """First-attempt and overall CI verdicts for one PR: {first_attempt, overall, runs}.

    `results` maps each sha to its ci_result(). Commits with no gate are skipped.
    first_attempt is the earliest gated commit's verdict, overall the last gated
    commit at or before the merge (the last one if the PR is open). True is green,
    False is red, None means no commit had a gate.

    This is the PR-push gate on the surviving head commits only. A rebase or force-push
    drops the commits it replaces, and their red runs with them, so first_attempt can be
    optimistic. The merge-queue result is queue_summary().
    """
    gated = [c for c in sorted(commits, key=lambda c: c["committed"]) if results[c["sha"]]["gate"] != "none"]
    if not gated:
        return {"first_attempt": None, "overall": None, "runs": 0}
    merged = _when(pr.get("mergedAt"))
    before = [c for c in gated if merged is None or _when(c["committed"]) <= merged]
    return {
        "first_attempt": results[gated[0]["sha"]]["gate"] == "success",
        "overall": results[before[-1]["sha"]]["gate"] == "success" if before else None,
        "runs": len(gated),
    }


def merge_queue_runs(run, since):
    """The merge-queue `ci` runs created since `since`, as {pr_number: [{conclusion, created}]}, oldest first.

    A queue run tests a temporary commit on a `gh-readonly-queue/` branch, which is not in
    `pulls/<n>/commits`, so its PR number is read from the branch name and the name is dropped.
    """
    by_pr = {}
    for r in pages(run, f"repos/{{owner}}/{{repo}}/actions/runs?event=merge_group&created=>={since}&per_page=100",
                   key="workflow_runs"):
        m = QUEUE_BRANCH.fullmatch(r.get("head_branch") or "")
        if r.get("name") == "ci" and m:
            by_pr.setdefault(int(m.group(1)), []).append({"conclusion": r.get("conclusion"), "created": r.get("created_at")})
    for queued in by_pr.values():
        queued.sort(key=lambda r: r["created"] or "")
    return by_pr


def queue_summary(runs):
    """A PR's merge-queue verdicts from its merge_queue_runs() entries: {runs, first_attempt, overall}.

    Cancelled and skipped runs are dropped, as ci_result() drops them. first_attempt is the
    earliest run's verdict, overall the latest's; both None when no run is left.
    """
    verdicts = [v for v in (_verdict(r["conclusion"]) for r in runs) if v != "none"]
    if not verdicts:
        return {"runs": 0, "first_attempt": None, "overall": None}
    return {"runs": len(verdicts), "first_attempt": verdicts[0] == "success", "overall": verdicts[-1] == "success"}


# -- escaped defects ---------------------------------------------------------
def defect_items(prs, issues, campaign_of):
    """Every escaped-defect link as a flat list, before grouping by campaign.

    Each row is {kind, number, cites, days_after_merge, campaign, citing_campaign}; `campaign` is the cited PR's.
    `campaign_of(number)` gives a PR's campaign, None when untagged. A merged PR N of campaign C is cited by:
    (a) a merged fix or revert PR outside C whose text cites #N and that merged after N;
    (b) a closed `bug` issue whose body cites #N and that was created after N merged;
    (c) a merged `Revert "<title>"` PR outside C quoting the title of N.
    `kind` is pr, revert or issue; `number` is the citing item. A link needs both campaigns known (a, c) or N's.
    """
    merged = {p["number"]: p for p in prs if p.get("mergedAt")}
    by_title = {p["title"]: p for p in merged.values()}
    found = {}

    def hit(kind, number, target, when, citing):
        # One item per citing item and campaign: a fix that cites a whole stack counts once. The cited PR merged last wins.
        days = round((_when(when) - _when(target["mergedAt"])).total_seconds() / 86400, 1)
        key = (kind, number, campaign_of(target["number"]))
        if key not in found or days < found[key]["days_after_merge"]:
            found[key] = {
                "kind": kind, "number": number, "cites": target["number"], "days_after_merge": days,
                "campaign": campaign_of(target["number"]), "citing_campaign": citing}

    for p in prs:
        if not p.get("mergedAt") or not FIX_OR_REVERT.match(p["title"]):
            continue
        kind = "revert" if p["title"].lower().startswith("revert") else "pr"
        for n in {int(x) for x in CITE.findall(p["title"] + "\n" + p["body"])}:
            target = merged.get(n)
            citing = campaign_of(p["number"])
            if target and _apart(campaign_of(n), citing) and _when(p["mergedAt"]) > _when(target["mergedAt"]):
                hit(kind, p["number"], target, p["mergedAt"], citing)

    for i in issues:
        if str(i.get("state", "")).lower() != "closed" or "bug" not in _labels(i):
            continue
        if str(i.get("stateReason") or "").upper() in NOT_A_DEFECT:
            continue
        for n in {int(x) for x in CITE.findall(i.get("body") or "")}:
            target = merged.get(n)
            if target and campaign_of(n) is not None and _when(i["createdAt"]) > _when(target["mergedAt"]):
                hit("issue", i["number"], target, i["createdAt"], None)

    for p in prs:
        quoted = REVERT_OF.match(p["title"])
        target = by_title.get(quoted.group(1)) if quoted and p.get("mergedAt") else None
        citing = campaign_of(p["number"])
        if target and _apart(campaign_of(target["number"]), citing) \
                and _when(p["mergedAt"]) > _when(target["mergedAt"]):
            hit("revert", p["number"], target, p["mergedAt"], citing)

    return sorted(found.values(), key=lambda r: (r["campaign"], r["number"], r["cites"]))


def group_defects(items):
    """The defect items grouped by the cited PR's campaign: {campaign: [{kind, number, cites, days_after_merge}]}."""
    grouped = {}
    for row in items:
        grouped.setdefault(row["campaign"], []).append(
            {k: row[k] for k in ("kind", "number", "cites", "days_after_merge")})
    return grouped


def escaped_defects(prs, issues, campaign_of):
    """Defects that escaped each campaign: {campaign: [{kind, number, cites, days_after_merge}]}. See defect_items()."""
    return group_defects(defect_items(prs, issues, campaign_of))


# -- the whole run -----------------------------------------------------------
def collect(run, since, campaigns=None, max_prs=None, until=None, pr_map=None):
    """The github.json record: each PR's list fields, its CI and queue verdicts, and the escaped defects.

    The window is `since` (YYYY-MM-DD) up to `until` (an ISO timestamp, exclusive; no bound when None).
    A PR's campaign is its `pr_map` entry (the ledger's) when it has one, else its branch or title tag.
    An untagged PR stays untagged: it is never grouped into a campaign of its own.
    Only a tagged PR gets per-PR calls (commits, check-runs, files); `max_prs` caps how many of them do.
    An untagged PR's per-PR fields are None. The output carries numbers, campaign tags and verdicts only:
    no titles, bodies, branches or files.
    """
    prs = collect_prs(run, since, campaigns, until)
    issues = collect_issues(run, since, until)
    tags = {p["number"]: (pr_map or {}).get(p["number"]) or p["campaign"] for p in prs}
    queued = merge_queue_runs(run, since)
    rows = []
    traced = 0
    for pr in prs:
        number = pr["number"]
        row = {"number": number, "campaign": tags[number], "merged": bool(pr["mergedAt"]),
               "createdAt": pr["createdAt"], "mergedAt": pr["mergedAt"],
               "code_pr": None, "commits": None, "ci": None, "queue": None}
        if tags[number] is not None and (max_prs is None or traced < max_prs):
            traced += 1
            commits = head_commits(run, pr)
            results = {c["sha"]: ci_result(run, c["sha"]) for c in commits}
            row.update(code_pr=code_pr(run, number), commits=commits, ci=ci_summary(pr, commits, results),
                       queue=queue_summary(queued.get(number, [])))
        rows.append(row)
    items = defect_items(prs, issues, lambda n: tags.get(n))
    return {"since": since, "until": until, "prs": rows, "untagged_prs": sum(t is None for t in tags.values()),
            "escaped_defects": group_defects(items), "defect_items": items}
