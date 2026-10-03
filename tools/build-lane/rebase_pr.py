#!/usr/bin/env python3
"""Rebase a PR branch onto origin/main with git alone, resolving only mechanical conflicts.

Usage: tools/build-lane/rebase-pr.sh [--push] [--json] [-v] [--no-fetch] [--onto REF] <target>

<target> is a PR number, a branch name, or a worktree path. The rebase runs in a throwaway
sparse worktree, so the branch's own worktree (and its build cache) is not touched unless
the rebase succeeds. Two kinds of conflict are resolved, because a regeneration owns them:

  * generated blocks: text between `<!-- gen:NAME -->` markers, or the crate graph between
    `<!-- crate-graph:begin/end -->`. A workflow reruns tools/docs-gen/regen.py on main
    after every merge, so main's side is taken. The file is resolved only when it merges
    cleanly once every generated block is blanked out on all three sides.
  * Cargo.lock: main's copy, then `cargo update --workspace` (override with
    $REBASE_PR_LOCKFILE_CMD) to add what the branch's manifests need.

Any other conflict is semantic: the rebase is aborted, the branch and its worktree are left
exactly as they were, and a summary names the files, so a worker is spawned only for those.

Output: nothing on a clean rebase (exit 0). Otherwise `key=value` lines (or one JSON object
with --json): status, branch, old, head, mechanical, semantic, stopped_at, preview, pushed.
`preview` is every file a squashed merge of the branch into main conflicts in, so the
worker sees the whole job, not only the first commit that stopped.

Exit codes: 0 ok or up to date, 1 semantic conflict, 2 bad target or precondition
(dirty worktree, diverged from origin, rebase already in progress), 3 git or push failure.

--push pushes the result with --force-with-lease against the origin head fetched at the
start, and refuses when origin has commits the local branch lacks.
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
from dataclasses import dataclass, field
from pathlib import Path

GEN = re.compile(rb"(<!-- gen:([a-z0-9][a-z0-9-]*)((?: [^>]*?)?) -->)(.*?)(<!-- /gen:\2 -->)", re.S)
GRAPH = re.compile(rb"(<!-- crate-graph:begin -->)(.*?)(<!-- crate-graph:end -->)", re.S)
LOCKFILE = "Cargo.lock"
DEFAULT_LOCK_CMD = "cargo update --workspace --quiet"
CONFLICT_MARK = re.compile(rb"^(<{7}|>{7})( |$)", re.M)


class Failure(Exception):
    def __init__(self, code: int, status: str, reason: str):
        super().__init__(reason)
        self.code, self.status, self.reason = code, status, reason


def git(*args: str, cwd: str | None = None, check: bool = True, data: bytes | None = None,
        opts: tuple[str, ...] = ()) -> subprocess.CompletedProcess:
    cmd = ["git", *opts, *args]
    env = dict(os.environ, GIT_EDITOR="true", GIT_SEQUENCE_EDITOR="true", GIT_TERMINAL_PROMPT="0")
    p = subprocess.run(cmd, cwd=cwd, input=data, capture_output=True, env=env)
    if check and p.returncode != 0:
        raise Failure(3, "error", f"git {' '.join(args[:3])} failed: {p.stderr.decode(errors='replace').strip()[:300]}")
    return p


def out(*args: str, **kw) -> str:
    return git(*args, **kw).stdout.decode(errors="replace").strip()


# ── generated blocks ─────────────────────────────────────────────────────────


def _blocks(text: bytes) -> dict[tuple, list[bytes]]:
    found: dict[tuple, list[bytes]] = {}
    for m in GEN.finditer(text):
        found.setdefault((b"gen", m.group(2), m.group(3)), []).append(m.group(4))
    for m in GRAPH.finditer(text):
        found.setdefault((b"crate-graph",), []).append(m.group(2))
    return found


def blank_generated(text: bytes) -> bytes:
    text = GEN.sub(lambda m: m.group(1) + m.group(5), text)
    return GRAPH.sub(lambda m: m.group(1) + m.group(3), text)


def merge_file(base: bytes, ours: bytes, theirs: bytes, union: bool = False) -> bytes | None:
    """git merge-file on raw bytes (no filters). None when it conflicts."""
    with tempfile.TemporaryDirectory(prefix="rebase-pr-merge-") as d:
        paths = []
        for name, blob in (("ours", ours), ("base", base), ("theirs", theirs)):
            p = Path(d) / name
            p.write_bytes(blob)
            paths.append(str(p))
        r = subprocess.run(["git", "merge-file", "-p", *(["--union"] if union else []), *paths], capture_output=True)
    if r.returncode != 0:   # >0: conflict count, <0: error
        return None
    return r.stdout


def resolve_generated(base: bytes, ours: bytes, theirs: bytes, union: bool = False) -> bytes | None:
    """Merge a file whose conflicts lie only inside generated blocks; None otherwise.

    `ours` is the upstream side (main) in a rebase. Every generated body is blanked on all
    three sides; if that merges cleanly, the conflict was in generated text alone, and each
    body is refilled from main's side (or the branch's, for a block only the branch has).
    `union` merges the rest as a `merge=union` file would."""
    merged = merge_file(blank_generated(base), blank_generated(ours), blank_generated(theirs), union)
    if merged is None:
        return None
    main_side, branch_side = _blocks(ours), _blocks(theirs)
    bodies: dict[tuple, list[bytes]] = {}
    for key, n in ((k, len(v)) for k, v in _blocks(merged).items()):
        side = main_side if key in main_side else branch_side
        if len(side.get(key, [])) != n:
            return None   # a block was added beside, or duplicated from, one main has
        bodies[key] = list(side[key])
    def fill(key):
        return bodies[key].pop(0)
    # Fill in document order; GEN and GRAPH never nest, so two passes are independent.
    merged = GEN.sub(lambda m: m.group(1) + fill((b"gen", m.group(2), m.group(3))) + m.group(5), merged)
    merged = GRAPH.sub(lambda m: m.group(1) + fill((b"crate-graph",)) + m.group(3), merged)
    return merged


# ── rebase ───────────────────────────────────────────────────────────────────


@dataclass
class Result:
    status: str = "ok"
    branch: str = ""
    old: str = ""
    head: str = ""
    mechanical: list[str] = field(default_factory=list)
    semantic: list[str] = field(default_factory=list)
    stopped_at: str = ""
    preview: list[str] = field(default_factory=list)
    pushed: bool = False
    reason: str = ""


def worktrees(repo: str) -> dict[str, str]:
    """branch -> worktree path, from `git worktree list --porcelain`."""
    found, path = {}, None
    for line in out("worktree", "list", "--porcelain", cwd=repo).splitlines():
        if line.startswith("worktree "):
            path = line[len("worktree "):]
        elif line.startswith("branch refs/heads/") and path:
            found[line[len("branch refs/heads/"):]] = path
    return found


def resolve_target(target: str, repo: str) -> str:
    if target.isdigit():
        p = subprocess.run(["gh", "pr", "view", target, "--json", "headRefName,isCrossRepository"],
                           cwd=repo, capture_output=True, text=True)
        if p.returncode != 0:
            raise Failure(2, "error", f"gh pr view {target} failed: {p.stderr.strip()[:200]}")
        info = json.loads(p.stdout)
        if info.get("isCrossRepository"):
            raise Failure(2, "error", f"PR {target} is from a fork")
        return info["headRefName"]
    if os.path.isdir(target):
        branch = out("rev-parse", "--abbrev-ref", "HEAD", cwd=target)
        if branch == "HEAD":
            raise Failure(2, "error", f"{target} has a detached HEAD")
        return branch
    return target.removeprefix("origin/")


def rev(ref: str, repo: str) -> str | None:
    p = git("rev-parse", "--verify", "--quiet", ref + "^{commit}", cwd=repo, check=False)
    return p.stdout.decode().strip() if p.returncode == 0 else None


def preview_conflicts(repo: str, onto: str, source: str) -> list[str]:
    p = git("merge-tree", "--write-tree", "--name-only", "--no-messages", onto, source, cwd=repo, check=False)
    if p.returncode != 1:   # 0 clean, 1 conflicts, else unsupported/error
        return []
    return [l for l in p.stdout.decode(errors="replace").splitlines()[1:] if l]


@dataclass
class Scratch:
    """The throwaway worktree and the git options every command in it needs."""
    path: str
    sparse: bool
    union_gen: frozenset[str]   # merge=union files with generated blocks; see attr_override
    opts: tuple[str, ...] = ()
    admin: str = ""             # its .git/worktrees/<id> entry

    def git(self, *args: str, **kw) -> subprocess.CompletedProcess:
        return git(*args, cwd=self.path, opts=self.opts, **kw)


def attr_override(repo: str, onto: str) -> tuple[str, frozenset[str]]:
    """The tree to read attributes from during the rebase (main's, or a copy with an
    override), and the files the override covers.

    `merge=union` (.gitattributes) keeps both sides of a conflicting line. On a line with a
    generated block that silently duplicates the row, so for union files that have
    generated blocks the rebase uses a normal text merge, and try_mechanical applies union
    to everything outside the blocks."""
    names = out("grep", "-l", "-E", "<!-- (gen:|crate-graph:begin)", onto, "--", "*.md", cwd=repo, check=False)
    paths = [n.split(":", 1)[1] for n in names.splitlines() if ":" in n]
    if not paths:
        return onto, frozenset()
    attrs = out("check-attr", "--source", onto, "merge", "--", *paths, cwd=repo)
    union = sorted(line.split(": merge: ")[0] for line in attrs.splitlines() if line.endswith(": merge: union"))
    if not union:
        return onto, frozenset()
    current = git("cat-file", "blob", f"{onto}:.gitattributes", cwd=repo, check=False).stdout
    text = current + b"\n# rebase-pr: generated blocks must not be union-merged\n"
    text += b"".join(b"/" + u.encode() + b" merge=text\n" for u in union)
    sha = out("hash-object", "-w", "--stdin", cwd=repo, data=text)
    entries = [e for e in out("ls-tree", onto, cwd=repo).splitlines() if not e.endswith("\t.gitattributes")]
    entries.append(f"100644 blob {sha}\t.gitattributes")
    tree = out("mktree", cwd=repo, data=("\n".join(entries) + "\n").encode())
    return tree, frozenset(union)


def unmerged(wt: Scratch) -> dict[str, dict[int, tuple[str, str]]]:
    """path -> {stage: (mode, sha)} for every conflicted path."""
    files: dict[str, dict[int, tuple[str, str]]] = {}
    raw = wt.git("ls-files", "-u", "-z").stdout.decode(errors="replace")
    for entry in filter(None, raw.split("\0")):
        meta, path = entry.split("\t", 1)
        mode, sha, stage = meta.split()
        files.setdefault(path, {})[int(stage)] = (mode, sha)
    return files


def stage_blob(wt: Scratch, path: str, mode: str, content: bytes) -> None:
    sha = out("hash-object", "-w", "--stdin", cwd=wt.path, data=content)
    wt.git("update-index", "--cacheinfo", f"{mode},{sha},{path}")
    wt.git("checkout-index", "-f", "--", path)


def try_mechanical(wt: Scratch, path: str, stages: dict) -> str | None:
    """Resolve one conflicted path if it is mechanical. Returns None on success, else why not.
    'need-full' asks the caller to retry in a full checkout."""
    if set(stages) != {1, 2, 3}:
        return "added or deleted on one side"
    blob = lambda stage: wt.git("cat-file", "blob", stages[stage][1]).stdout
    if path == LOCKFILE or path.endswith("/" + LOCKFILE):
        if wt.sparse:
            return "need-full"
        stage_blob(wt, path, stages[2][0], blob(2))
        cmd = os.environ.get("REBASE_PR_LOCKFILE_CMD", DEFAULT_LOCK_CMD)
        r = subprocess.run(cmd, shell=True, cwd=os.path.join(wt.path, os.path.dirname(path)), capture_output=True)
        if r.returncode != 0:
            return f"lockfile regeneration failed: {r.stderr.decode(errors='replace').strip()[:200]}"
        wt.git("add", "--", path)
        return None
    if not path.endswith(".md"):
        return "not a generated file"
    merged = resolve_generated(blob(1), blob(2), blob(3), union=path in wt.union_gen)
    if merged is None or CONFLICT_MARK.search(merged):
        return "conflict outside generated blocks"
    stage_blob(wt, path, stages[2][0], merged)
    return None


def rebase_in(wt: Scratch, onto: str, res: Result) -> bool:
    """Run the rebase in the scratch worktree. True when it completes; False when it stopped
    on a semantic conflict (res filled in). Raises Failure('need-full') for a lockfile."""
    rebase_dir = out("rev-parse", "--path-format=absolute", "--git-path", "rebase-merge", cwd=wt.path)
    p = wt.git("rebase", "--no-autostash", onto, check=False)
    while p.returncode != 0:
        if not os.path.isdir(rebase_dir):
            raise Failure(3, "error", "rebase failed: " + (p.stderr or p.stdout).decode(errors="replace").strip()[:300])
        conflicts = unmerged(wt)
        if not conflicts:
            raise Failure(3, "error", "rebase stopped without a conflict: " + p.stdout.decode(errors="replace").strip()[:300])
        stopped = out("rev-parse", "--verify", "--quiet", "REBASE_HEAD", cwd=wt.path)
        semantic = {}
        for path, stages in sorted(conflicts.items()):
            why = try_mechanical(wt, path, stages)
            if why == "need-full":
                raise Failure(0, "need-full", path)
            if why:
                semantic[path] = why
            elif path not in res.mechanical:
                res.mechanical.append(path)
        if semantic:
            res.status, res.semantic, res.stopped_at = "conflict", sorted(semantic), stopped
            wt.git("rebase", "--abort", check=False)
            return False
        if wt.git("diff", "--cached", "--quiet", "HEAD", check=False).returncode == 0:
            p = wt.git("rebase", "--skip", check=False)   # the commit became empty
        else:
            p = wt.git("rebase", "--continue", check=False)
    return True


def scratch_worktree(repo: str, source: str, sparse: bool, attr_tree: str,
                     union_gen: frozenset[str]) -> Scratch:
    path = os.path.join(tempfile.mkdtemp(prefix="rebase-pr-"), "wt")
    opts: tuple[str, ...] = ()
    if sparse:   # per command, so the repository's shared config is never changed
        opts += ("-c", "core.sparseCheckout=true", "-c", "core.sparseCheckoutCone=false")
    opts += ("--attr-source", attr_tree)   # main's attributes, not the empty worktree's
    wt = Scratch(path, sparse, union_gen, opts)
    git("worktree", "add", "-q", "--no-checkout", "--detach", path, source, cwd=repo)
    wt.admin = out("rev-parse", "--absolute-git-dir", cwd=path)
    if not sparse:
        wt.git("read-tree", "-mu", "HEAD")
        return wt
    info = Path(out("rev-parse", "--path-format=absolute", "--git-path", "info", cwd=path))
    info.mkdir(parents=True, exist_ok=True)
    (info / "sparse-checkout").write_text("/.gitattributes\n")
    # Index only, every entry skip-worktree: `read-tree -mu` stats each path (seconds here).
    wt.git("read-tree", "HEAD")
    wt.git("update-index", "-z", "--skip-worktree", "--stdin", data=wt.git("ls-files", "-z").stdout)
    return wt


def drop_worktree(repo: str, wt: Scratch) -> None:
    """Remove the scratch worktree and its own admin entry, never anyone else's: no
    `git worktree prune`, which deletes every entry whose path this git cannot resolve
    (see the 2026-10-03 gotcha in docs/agents/rules-and-gotchas.md)."""
    git("worktree", "remove", "--force", wt.path, cwd=repo, check=False)
    shutil.rmtree(os.path.dirname(wt.path), ignore_errors=True)
    if wt.admin and os.path.basename(os.path.dirname(wt.admin)) == "worktrees":
        shutil.rmtree(wt.admin, ignore_errors=True)


def run(args: argparse.Namespace) -> Result:
    repo = args.repo or os.getcwd()
    repo = out("rev-parse", "--show-toplevel", cwd=repo)
    branch = resolve_target(args.target, repo)
    res = Result(branch=branch)
    remote = args.remote
    if not args.no_fetch:
        upstream_branch = args.onto.split("/", 1)[1] if args.onto.startswith(remote + "/") else None
        if upstream_branch:
            git("fetch", "-q", remote, f"+refs/heads/{upstream_branch}:refs/remotes/{remote}/{upstream_branch}", cwd=repo)
        git("fetch", "-q", remote, f"+refs/heads/{branch}:refs/remotes/{remote}/{branch}", cwd=repo, check=False)
    onto = rev(args.onto, repo)
    if not onto:
        raise Failure(2, "error", f"{args.onto} not found")
    local = rev(f"refs/heads/{branch}", repo)
    remote_old = rev(f"refs/remotes/{remote}/{branch}", repo)
    source = local or remote_old
    if not source:
        raise Failure(2, "error", f"branch {branch} not found locally or on {remote}")
    res.old = res.head = source

    wt_of = worktrees(repo).get(branch)
    if wt_of:
        if git("status", "--porcelain", "--untracked-files=no", cwd=wt_of).stdout.strip():
            raise Failure(2, "dirty", f"{wt_of} has uncommitted changes")
        for state in ("rebase-merge", "rebase-apply", "MERGE_HEAD"):
            if os.path.exists(out("rev-parse", "--path-format=absolute", "--git-path", state, cwd=wt_of)):
                raise Failure(2, "dirty", f"{wt_of} is in the middle of a rebase or merge")
    if args.push and local and remote_old and git("merge-base", "--is-ancestor", remote_old, local, cwd=repo, check=False).returncode != 0:
        raise Failure(2, "diverged", f"{remote}/{branch} has commits the local branch lacks")

    if git("merge-base", "--is-ancestor", onto, source, cwd=repo, check=False).returncode == 0:
        res.status = "up-to-date"
        return res
    res.preview = preview_conflicts(repo, onto, source)

    attr_tree, union_gen = attr_override(repo, onto)
    sparse = True
    while True:
        wt = scratch_worktree(repo, source, sparse, attr_tree, union_gen)
        try:
            done = rebase_in(wt, onto, res)
            new = out("rev-parse", "HEAD", cwd=wt.path) if done else None
        except Failure as f:
            if f.status != "need-full":
                raise
            res.mechanical.clear()
            sparse = False
            continue
        finally:
            drop_worktree(repo, wt)
        break
    if not done:
        return res

    # Move the branch only if nothing else moved it meanwhile.
    if local:
        if wt_of:
            if out("rev-parse", "HEAD", cwd=wt_of) != source:
                raise Failure(3, "error", f"{branch} moved during the rebase")
            git("reset", "-q", "--keep", new, cwd=wt_of)
        else:
            git("update-ref", f"refs/heads/{branch}", new, source, cwd=repo)
    res.head = new
    if args.push:
        lease = f"--force-with-lease=refs/heads/{branch}:{remote_old or ''}"
        p = git("push", "-q", lease, remote, f"{new}:refs/heads/{branch}", cwd=repo, check=False)
        if p.returncode != 0:
            raise Failure(3, "push-failed", p.stderr.decode(errors="replace").strip()[:300])
        res.pushed = True
    return res


def render(res: Result, as_json: bool) -> str:
    d = {k: v for k, v in res.__dict__.items() if v not in ("", [], False) or k in ("status", "branch")}
    if as_json:
        return json.dumps(d)
    lines = []
    for k, v in d.items():
        if isinstance(v, list):
            v = ",".join(v)
        elif isinstance(v, bool):
            v = "yes" if v else "no"
        lines.append(f"{k}={v}")
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(prog="rebase-pr.sh", description=__doc__.split("\n\n")[0])
    ap.add_argument("target", help="PR number, branch name, or worktree path")
    ap.add_argument("--push", action="store_true", help="push the result with --force-with-lease")
    ap.add_argument("--onto", default="origin/main", help="upstream to rebase onto (default origin/main)")
    ap.add_argument("--remote", default="origin")
    ap.add_argument("--no-fetch", action="store_true")
    ap.add_argument("--json", action="store_true", help="print one JSON object instead of key=value lines")
    ap.add_argument("-v", "--verbose", action="store_true", help="print the summary on a clean success too")
    ap.add_argument("--repo", help=argparse.SUPPRESS)
    args = ap.parse_args(argv)
    try:
        res = run(args)
        code = 1 if res.status == "conflict" else 0
    except Failure as f:
        res = Result(status=f.status, branch=getattr(args, "target", ""), reason=f.reason)
        code = f.code
    quiet = code == 0 and res.status in ("ok", "up-to-date") and not res.mechanical and not args.verbose
    if not quiet:
        print(render(res, args.json))
    return code


if __name__ == "__main__":
    sys.exit(main())
