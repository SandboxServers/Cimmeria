#!/usr/bin/env python3
"""The mechanical end of a packet in one call: commit, push and open the PR, or merge it.

Usage:
  tools/build-lane/ship.sh pr -C <worktree> (-m MSG | -F FILE) [--title T] [--body-file F]
                              [--paths P ...] [--draft] [--force-with-lease] [-v]
  tools/build-lane/ship.sh merge <PR> [--retire NAME] [--timeout 30m] [--no-wait] [-v]

pr: -C must be a registered worktree, not the main checkout, whose toplevel is that
directory, on a branch other than main/master. Anything else is refused before git runs, so
a worker whose worktree vanished cannot push the main checkout. It stages every change (or
--paths), commits with the trailers in $SHIP_TRAILERS (newline-separated, each added only
if missing), pushes with -u, and opens a PR unless the branch has an open one. The PR title
defaults to the branch's first commit subject; the body is --body-file (else that commit's
body) plus $SHIP_PR_FOOTER.

merge: kind=docs (every changed file is *.md or under .claude/agent-memory/) merges at once
with --admin, the owner's rule for Markdown and memory-only PRs. kind=code rebases with
rebase-pr.sh only when GitHub says BEHIND or DIRTY (D-TP8), waits for the gating checks
(fmt, clippy, build + nextest; `skipping` passes) and merges with --squash. Coverage and
live-DB jobs are not waited for. --retire then runs `rm-worktree.sh --no-prune NAME`.

Output: one line, `status=... key=value ...`; a value with spaces is double-quoted. The full
git and gh output goes to a log whose path is printed only on failure; -v copies it to
stderr as it runs.

Exit codes: 0 ok, noop or merged; 1 rebase conflict or a failed check; 2 refused (bad
arguments, unsafe worktree, PR closed or draft); 3 timed out waiting for checks; 4 a git or
gh command failed; 5 merged, but the worktree was not retired.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
GH = [os.environ.get("SHIP_GH", "gh")]
REBASE = [sys.executable, str(HERE / "rebase_pr.py")]
RM_WORKTREE = str(HERE / "rm-worktree.sh")
GATING = ("cargo fmt --check", "cargo clippy -D warnings", "cargo build + nextest (workspace, no DB)")
PROTECTED = ("main", "master")


class Stop(Exception):
    def __init__(self, code: int, status: str, reason: str = "", **fields):
        super().__init__(reason)
        self.code, self.status, self.reason, self.fields = code, status, reason, fields


class Log:
    """Every command and its output. Kept, and its path printed, only when the run fails."""

    def __init__(self, verbose: bool):
        d = Path(tempfile.gettempdir()) / "cimmeria-ship"
        d.mkdir(parents=True, exist_ok=True)
        self.path = d / f"ship-{time.strftime('%Y%m%d-%H%M%S')}-{os.getpid()}.log"
        self.fh = open(self.path, "w", encoding="utf-8")
        self.verbose = verbose

    def write(self, text: str) -> None:
        self.fh.write(text + "\n")
        self.fh.flush()
        if self.verbose:
            print(text, file=sys.stderr)

    def close(self, keep: bool) -> None:
        self.fh.close()
        if not keep:
            self.path.unlink(missing_ok=True)


LOG: Log | None = None


def run(cmd: list[str], cwd: str | None = None, check: bool = True, what: str = "") -> subprocess.CompletedProcess:
    env = dict(os.environ, GIT_TERMINAL_PROMPT="0", GH_PROMPT_DISABLED="1", GIT_EDITOR="true")
    p = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True, encoding="utf-8", errors="replace", env=env)
    if LOG:
        LOG.write(f"$ {' '.join(cmd)}  (cwd={cwd}, exit {p.returncode})")
        for stream in (p.stdout, p.stderr):
            if stream.strip():
                LOG.write(stream.rstrip())
    if check and p.returncode != 0:
        msg = (p.stderr or p.stdout).strip().splitlines()
        raise Stop(4, what or "error", f"{' '.join(cmd[:3])} failed: {msg[-1] if msg else p.returncode}")
    return p


def git(wt: str, *args: str, **kw) -> subprocess.CompletedProcess:
    return run(["git", "-C", wt, *args], **kw)


def gh(*args: str, cwd: str | None = None, **kw) -> subprocess.CompletedProcess:
    return run([*GH, *args], cwd=cwd, **kw)


def norm(path: str) -> str:
    return os.path.normcase(os.path.normpath(os.path.abspath(path)))


def classify(paths: list[str]) -> str:
    docs = all(p.endswith(".md") or p.startswith(".claude/agent-memory/") for p in paths)
    return "docs" if paths and docs else "code"


def worktree_entries(wt: str) -> list[tuple[str, str]]:
    """(path, branch) for each registered worktree, main checkout first."""
    entries, path = [], None
    for line in git(wt, "worktree", "list", "--porcelain").stdout.splitlines():
        if line.startswith("worktree "):
            path = line[len("worktree "):]
            entries.append((path, ""))
        elif line.startswith("branch refs/heads/") and entries:
            entries[-1] = (path, line[len("branch refs/heads/"):])
    return entries


def check_worktree(wt: str) -> str:
    """Refuse anything but a registered, non-main worktree on a feature branch. Returns the branch."""
    if not wt or not os.path.isdir(wt):
        raise Stop(2, "refused", f"missing-worktree: {wt} does not exist")
    top = git(wt, "rev-parse", "--show-toplevel", check=False)
    if top.returncode != 0:
        raise Stop(2, "refused", f"not-a-worktree: {top.stderr.strip()[:200]}")
    if norm(top.stdout.strip()) != norm(wt):
        raise Stop(2, "refused", f"toplevel-mismatch: {wt} belongs to {top.stdout.strip()}, so its own worktree entry is gone")
    entries = worktree_entries(wt)
    registered = [norm(p) for p, _ in entries]
    if norm(wt) not in registered:
        raise Stop(2, "refused", f"not-registered: {wt} is not in `git worktree list`")
    if registered[0] == norm(wt):
        raise Stop(2, "refused", f"main-checkout: {wt} is the main checkout; ship from a worktree")
    branch = git(wt, "symbolic-ref", "--short", "-q", "HEAD", check=False).stdout.strip()
    if not branch:
        raise Stop(2, "refused", "detached-head")
    if branch in PROTECTED:
        raise Stop(2, "refused", f"protected-branch: {branch}")
    for state in ("rebase-merge", "rebase-apply", "MERGE_HEAD"):
        if os.path.exists(git(wt, "rev-parse", "--path-format=absolute", "--git-path", state).stdout.strip()):
            raise Stop(2, "refused", "rebase-or-merge-in-progress")
    return branch


def trailers() -> list[str]:
    return [t.strip() for t in os.environ.get("SHIP_TRAILERS", "").splitlines() if t.strip()]


def with_trailers(wt: str, message: str) -> str:
    args = []
    for t in trailers():
        args += ["--trailer", t]
    if not args:
        return message
    p = subprocess.run(["git", "-C", wt, "interpret-trailers", "--if-exists", "addIfDifferent", *args],
                       input=message, capture_output=True, text=True, encoding="utf-8")
    if p.returncode != 0:
        raise Stop(4, "error", f"git interpret-trailers failed: {p.stderr.strip()[:200]}")
    return p.stdout


def rev(wt: str, ref: str) -> str:
    return git(wt, "rev-parse", "--verify", "--quiet", ref + "^{commit}", check=False).stdout.strip()


def open_pr(cwd: str, branch: str) -> dict | None:
    p = gh("pr", "list", "--head", branch, "--state", "open", "--json", "number,url", "--limit", "1", cwd=cwd)
    found = json.loads(p.stdout or "[]")
    return found[0] if found else None


def pr_body(wt: str, args, first: str) -> str:
    if args.body_file:
        body = Path(args.body_file).read_text(encoding="utf-8")
    else:
        body = git(wt, "log", "-1", "--format=%b", first).stdout
        body = "\n".join(l for l in body.splitlines() if l.strip() not in trailers())
    footer = os.environ.get("SHIP_PR_FOOTER", "").strip()
    body = body.strip()
    if footer and footer not in body:
        body = f"{body}\n\n{footer}" if body else footer
    return body + "\n"


def cmd_pr(args) -> dict:
    wt = args.C
    branch = check_worktree(wt)
    git(wt, "fetch", "-q", "origin", "+refs/heads/main:refs/remotes/origin/main", what="fetch-failed")
    git(wt, "fetch", "-q", "origin", f"+refs/heads/{branch}:refs/remotes/origin/{branch}", check=False)
    remote_old = rev(wt, f"refs/remotes/origin/{branch}")

    git(wt, "add", "-A", "--", *(args.paths or ["."]))
    staged = git(wt, "diff", "--cached", "--quiet", check=False).returncode != 0
    if staged:
        message = args.m if args.m is not None else Path(args.F).read_text(encoding="utf-8")
        if not message.strip():
            raise Stop(2, "refused", "empty-message")
        with tempfile.NamedTemporaryFile("w", delete=False, suffix=".txt", encoding="utf-8", newline="\n") as f:
            f.write(with_trailers(wt, message))
        try:
            git(wt, "commit", "-q", "-F", f.name, what="commit-failed")
        finally:
            os.unlink(f.name)
    head = rev(wt, "HEAD")
    ahead = git(wt, "rev-list", "--count", "origin/main..HEAD").stdout.strip()
    changed = git(wt, "diff", "--name-only", "origin/main...HEAD").stdout.split()
    out = {"branch": branch, "commit": head[:9], "kind": classify(changed)}
    if ahead == "0":
        return {"status": "noop", **out, "reason": "the branch has no commits beyond origin/main"}

    pr = open_pr(wt, branch)
    unpushed = head != remote_old
    if not staged and not unpushed and pr:
        return {"status": "noop", **out, "pr": pr["number"], "url": pr["url"]}
    if unpushed:
        push = ["push", "-q", "-u"]
        if args.force_with_lease:
            push.append(f"--force-with-lease=refs/heads/{branch}:{remote_old}")
        r = git(wt, *push, "origin", f"HEAD:refs/heads/{branch}", check=False)
        if r.returncode != 0:
            hint = " (origin has commits this branch lacks: rebase, or pass --force-with-lease)" if "rejected" in r.stderr else ""
            raise Stop(4, "push-failed", r.stderr.strip().splitlines()[-1][:200] + hint if r.stderr.strip() else "push failed", **out)
    if not pr:
        first = git(wt, "rev-list", "--reverse", "origin/main..HEAD").stdout.split()[0]
        title = args.title or git(wt, "log", "-1", "--format=%s", first).stdout.strip()
        with tempfile.NamedTemporaryFile("w", delete=False, suffix=".md", encoding="utf-8") as f:
            f.write(pr_body(wt, args, first))
        try:
            create = ["pr", "create", "--head", branch, "--base", "main", "--title", title, "--body-file", f.name]
            url = gh(*create, *(["--draft"] if args.draft else []), cwd=wt, what="pr-create-failed").stdout.strip().splitlines()[-1]
        finally:
            os.unlink(f.name)
        m = re.search(r"/pull/(\d+)", url)
        pr = {"number": int(m.group(1)) if m else "?", "url": url}
    return {"status": "ok", "branch": branch, "commit": head[:9], "pr": pr["number"], "url": pr["url"], "kind": out["kind"]}


# ── merge ────────────────────────────────────────────────────────────────────


def parse_timeout(text: str) -> float:
    m = re.fullmatch(r"(\d+(?:\.\d+)?)([smh]?)", text.strip())
    if not m:
        raise Stop(2, "refused", f"bad-timeout: {text}")
    return float(m.group(1)) * {"s": 1, "m": 60, "h": 3600, "": 60}[m.group(2)]


def poll_seconds() -> float:
    return float(os.environ.get("SHIP_POLL_SECONDS", "30"))


def view(pr: str, repo: str, fields: str) -> dict:
    return json.loads(gh("pr", "view", pr, "--json", fields, cwd=repo).stdout)


def merge_state(pr: str, repo: str) -> str:
    """GitHub computes mergeStateStatus lazily: UNKNOWN right after a push."""
    for _ in range(5):
        state = view(pr, repo, "mergeStateStatus")["mergeStateStatus"]
        if state != "UNKNOWN":
            return state
        time.sleep(min(5.0, poll_seconds()))
    return state


def rebase(pr: str, repo: str) -> None:
    p = run([*REBASE, "--repo", repo, "--push", pr], cwd=repo, check=False)
    if p.returncode == 0:
        return
    summary = dict(l.split("=", 1) for l in p.stdout.splitlines() if "=" in l)
    if p.returncode == 1:
        raise Stop(1, "conflict", "", semantic=summary.get("semantic", ""), stopped_at=summary.get("stopped_at", ""))
    raise Stop(4, "rebase-failed", summary.get("reason", p.stderr.strip()[:200]))


def wait_for_checks(pr: str, repo: str, deadline: float) -> None:
    gating = [g for g in os.environ.get("SHIP_GATING_CHECKS", "\n".join(GATING)).splitlines() if g.strip()]
    while True:
        p = gh("pr", "checks", pr, "--json", "name,bucket", cwd=repo, check=False)
        try:
            checks = json.loads(p.stdout or "[]")
        except json.JSONDecodeError:
            checks = []   # "no checks reported" yet
        buckets = {}
        for c in checks:
            buckets.setdefault(c["name"], []).append(c["bucket"])
        failed = [g for g in gating if any(b in ("fail", "cancel") for b in buckets.get(g, []))]
        if failed:
            raise Stop(1, "ci-failed", "", checks=",".join(failed))
        if all(g in buckets and all(b in ("pass", "skipping") for b in buckets[g]) for g in gating):
            return
        if time.monotonic() >= deadline:
            waiting = [g for g in gating if g not in buckets or any(b not in ("pass", "skipping") for b in buckets[g])]
            raise Stop(3, "timeout", "", checks=",".join(waiting))
        time.sleep(poll_seconds())


def find_bash() -> str:
    """Git Bash on Windows: the `bash` on PATH there is usually a WSL launcher."""
    if os.environ.get("SHIP_BASH"):
        return os.environ["SHIP_BASH"]
    if os.name != "nt":
        return shutil.which("bash") or "bash"
    git_exe = shutil.which("git")
    roots = [Path(git_exe).resolve().parent.parent] if git_exe else []
    roots.append(Path(r"C:\Program Files\Git"))
    for root in roots:
        for candidate in (root / "bin" / "bash.exe", root / "usr" / "bin" / "bash.exe"):
            if candidate.exists():
                return str(candidate)
    return "bash"


def retire_target(repo: str, name: str, branch: str) -> str:
    """Check before merging that --retire names a clean worktree on the PR's branch."""
    path = next((p for p, b in worktree_entries(repo) if norm(p) == norm(os.path.join(repo, ".claude", "worktrees", name))), None)
    if not path:
        raise Stop(2, "refused", f"retire: .claude/worktrees/{name} is not a registered worktree")
    on = git(path, "symbolic-ref", "--short", "-q", "HEAD", check=False).stdout.strip()
    if on != branch:
        raise Stop(2, "refused", f"retire: {name} is on {on or 'a detached HEAD'}, not the PR's branch {branch}")
    if git(path, "status", "--porcelain").stdout.strip():
        raise Stop(2, "refused", f"retire: {name} has uncommitted changes")
    return path


def cmd_merge(args) -> dict:
    repo = norm(git(args.repo or os.getcwd(), "rev-parse", "--path-format=absolute", "--git-common-dir").stdout.strip())
    repo = os.path.dirname(repo)   # the main checkout, where rm-worktree.sh runs
    pr = str(args.pr)
    info = view(pr, repo, "number,state,isDraft,headRefName,files,mergedAt")
    kind = classify([f["path"] for f in info.get("files") or []]) if len(info.get("files") or []) < 100 else "code"
    if info["state"] == "CLOSED":
        raise Stop(2, "refused", f"PR {pr} is closed")
    if info["state"] != "MERGED":
        if info.get("isDraft"):
            raise Stop(2, "refused", f"PR {pr} is a draft")
        if args.retire:
            retire_target(repo, args.retire, info["headRefName"])
        deadline = time.monotonic() + parse_timeout(args.timeout)
        for _ in range(3):
            state = merge_state(pr, repo)
            if state == "DIRTY" or (state == "BEHIND" and kind == "code"):
                rebase(pr, repo)
            if kind == "docs" or args.no_wait:
                break
            wait_for_checks(pr, repo, deadline)
            if merge_state(pr, repo) not in ("BEHIND", "DIRTY"):
                break   # main moved during the wait: rebase and wait again
        flags = ["--admin"] if kind == "docs" else []
        gh("pr", "merge", pr, "--squash", *flags, cwd=repo, what="merge-failed")
        info = view(pr, repo, "state,mergedAt")
        if info["state"] != "MERGED":
            raise Stop(4, "merge-failed", f"PR {pr} is {info['state']} after gh pr merge")
    out = {"status": "merged", "pr": pr, "merged_at": info.get("mergedAt", ""), "kind": kind, "retired": "none"}
    if args.retire:
        p = run([find_bash(), RM_WORKTREE, "--no-prune", args.retire], cwd=repo, check=False)
        if p.returncode != 0 or not re.search(r"^retired 1, skipped 0$", p.stdout, re.M):
            why = [l for l in p.stdout.splitlines() + p.stderr.splitlines() if l.startswith(("skip", "  "))]
            raise Stop(5, "merged", (why[-1].strip() if why else "rm-worktree.sh failed"),
                       pr=pr, merged_at=out["merged_at"], kind=kind, retired="none")
        out["retired"] = args.retire
    return out


def render(fields: dict) -> str:
    """key=value on one line; a value with spaces is double-quoted, the reason goes last."""
    reason = fields.pop("reason", "")
    if reason:
        fields["reason"] = reason
    parts = []
    for k, v in fields.items():
        if v in ("", None):
            continue
        v = " ".join(str(v).split()).replace('"', "'")
        parts.append(f'{k}="{v}"' if " " in v else f"{k}={v}")
    return " ".join(parts)


def main(argv: list[str] | None = None) -> int:
    global LOG
    ap = argparse.ArgumentParser(prog="ship.sh", description=__doc__.split("\n\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("pr", help="commit, push and open the PR")
    p.add_argument("-C", required=True, help="the worktree to ship")
    msg = p.add_mutually_exclusive_group(required=True)
    msg.add_argument("-m", help="commit message")
    msg.add_argument("-F", help="file holding the commit message")
    p.add_argument("--title")
    p.add_argument("--body-file")
    p.add_argument("--paths", nargs="+", help="stage only these paths (default: every change)")
    p.add_argument("--draft", action="store_true")
    p.add_argument("--force-with-lease", action="store_true")
    m = sub.add_parser("merge", help="wait for the gating checks and squash-merge")
    m.add_argument("pr")
    m.add_argument("--retire", metavar="NAME", help="retire .claude/worktrees/NAME after the merge")
    m.add_argument("--timeout", default="30m", help="how long to wait for checks (30m, 90s, 1h)")
    m.add_argument("--no-wait", action="store_true", help="merge without waiting for checks")
    m.add_argument("--repo", help=argparse.SUPPRESS)
    for s in (p, m):
        s.add_argument("-v", "--verbose", action="store_true")
    args = ap.parse_args(argv)
    LOG = Log(args.verbose)
    try:
        fields = cmd_pr(args) if args.cmd == "pr" else cmd_merge(args)
        code = 0
    except Stop as s:
        fields = {"status": s.status, **s.fields, "reason": s.reason}
        code = s.code
    except (OSError, ValueError, KeyError, json.JSONDecodeError) as e:
        fields, code = {"status": "error", "reason": f"{type(e).__name__}: {e}"}, 4
    LOG.close(keep=code != 0)
    if code != 0:
        fields["log"] = str(LOG.path)
    print(render(fields))
    return code


if __name__ == "__main__":
    sys.exit(main())
