"""PRs and branch heads: the attribution inputs that come from git and GitHub, not transcripts."""

import bisect
import json
import re
import subprocess

GH_FIELDS = "number,headRefName,headRefOid,mergeCommit,createdAt,mergedAt,closedAt,state,additions,deletions,changedFiles"
MERGE_SUBJECT = re.compile(r"^Merge (?:remote-tracking )?branch '([^']+)'")


def git(repo, *args):
    """(returncode, stdout) of a git command in repo."""
    p = subprocess.run(["git", "-C", str(repo), *args], capture_output=True, text=True, encoding="utf-8",
                       errors="replace")
    return p.returncode, p.stdout


def fetch_prs(repo):
    p = subprocess.run(["gh", "pr", "list", "--state", "all", "--limit", "10000", "--json", GH_FIELDS],
                       cwd=str(repo), capture_output=True, text=True, encoding="utf-8", check=True)
    return json.loads(p.stdout)


def store_prs(db, prs):
    """Upsert `gh pr list --json` rows into prs."""
    for p in prs:
        merge = p.get("mergeCommit") or {}
        db.execute(
            "INSERT INTO prs (pr_number, head_branch, head_sha, merge_sha, created_at, merged_at, closed_at, state,"
            " additions, deletions, changed_files) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            " ON CONFLICT (pr_number) DO UPDATE SET head_branch = excluded.head_branch, head_sha = excluded.head_sha,"
            " merge_sha = excluded.merge_sha, merged_at = excluded.merged_at, closed_at = excluded.closed_at,"
            " state = excluded.state, additions = excluded.additions, deletions = excluded.deletions,"
            " changed_files = excluded.changed_files",
            (p["number"], p["headRefName"], p.get("headRefOid"), merge.get("oid") if isinstance(merge, dict) else None,
             p["createdAt"], p.get("mergedAt") or None, p.get("closedAt") or None, p["state"], p.get("additions"),
             p.get("deletions"), p.get("changedFiles")))
    return len(prs)


def _short_branch(ref):
    if ref.startswith("refs/heads/"):
        return ref[len("refs/heads/"):]
    if ref.startswith("refs/remotes/"):
        return ref[len("refs/remotes/"):].split("/", 1)[1] if "/" in ref[len("refs/remotes/"):] else None
    return None


def _add_head(db, branch, sha, observed_at, source):
    if not branch or branch in ("HEAD", "main", "master") or not sha:
        return 0
    return db.execute("INSERT OR IGNORE INTO branch_heads (branch, commit_sha, observed_at, source) VALUES (?, ?, ?, ?)",
                      (branch, sha, observed_at, source)).rowcount


def snapshot_refs(db, repo):
    """ref-snapshot: the head of every local and remote-tracking branch.

    observed_at is the head commit's committer date: the commit was at the
    branch head no later than now and no earlier than then, and the date lets
    a backfill run place requests long before the snapshot.
    """
    code, out = git(repo, "for-each-ref", "--format=%(refname)%09%(objectname)%09%(committerdate:iso-strict)",
                    "refs/heads", "refs/remotes")
    if code != 0:
        return 0
    n = 0
    for line in out.splitlines():
        parts = line.split("\t")
        if len(parts) == 3:
            n += _add_head(db, _short_branch(parts[0]), parts[1], parts[2], "ref-snapshot")
    return n


def merge_subjects(db, repo):
    """merge-subject: "Merge branch '<name>'" commits name their second parent."""
    code, out = git(repo, "log", "--all", "--merges", "--format=%H%x09%P%x09%cI%x09%s")
    if code != 0:
        return 0
    n = 0
    for line in out.splitlines():
        parts = line.split("\t", 3)
        if len(parts) != 4:
            continue
        m = MERGE_SUBJECT.match(parts[3])
        parents = parts[1].split()
        if m and len(parents) >= 2:
            name = m.group(1)
            name = name.split("/", 1)[1] if name.startswith("origin/") else name
            n += _add_head(db, name, parents[1], parts[2], "merge-subject")
    return n


def lane_log(db, path):
    """lane-log: the build lane's jobs.jsonl records the branch and commit of every build."""
    n = 0
    try:
        fh = open(path, encoding="utf-8", errors="replace")
    except OSError:
        return 0
    with fh:
        for line in fh:
            try:
                job = json.loads(line)
            except ValueError:
                continue
            if isinstance(job, dict):
                n += _add_head(db, job.get("branch"), job.get("commit"), job.get("start") or "", "lane-log")
    return n


class Ancestry:
    """Commit ancestry from one `git rev-list --parents` of every ref and every PR commit.

    One subprocess instead of one `git merge-base --is-ancestor` per question,
    which cost about 70 ms each on Windows. Commits are matched by full sha or
    by a unique prefix (the build lane logs 10-character shas).
    """

    def __init__(self, repo):
        self.repo = repo
        self.parents = None
        self.desc_cache = {}

    def prepare(self, shas):
        """Load the DAG of all refs plus shas (PR heads and merges, which may have no ref left)."""
        p = subprocess.run(["git", "-C", str(self.repo), "rev-list", "--parents", "--all", "--ignore-missing",
                            "--stdin"], input="".join(s + "\n" for s in shas if s), capture_output=True,
                           text=True, encoding="utf-8", errors="replace")
        self.parents = {}
        self.children = {}
        for line in p.stdout.splitlines():
            parts = line.split()
            if not parts:
                continue
            self.parents[parts[0]] = parts[1:]
            for parent in parts[1:]:
                self.children.setdefault(parent, []).append(parts[0])
        self.sorted = sorted(self.parents)

    def resolve(self, sha):
        if not sha or self.parents is None:
            return None
        if sha in self.parents:
            return sha
        i = bisect.bisect_left(self.sorted, sha)
        unique = i < len(self.sorted) and self.sorted[i].startswith(sha) and not (
            i + 1 < len(self.sorted) and self.sorted[i + 1].startswith(sha))
        return self.sorted[i] if unique else None

    def first_parent(self, sha):
        sha = self.resolve(sha)
        parents = self.parents.get(sha) if sha else None
        return parents[0] if parents else None

    def descendants(self, sha):
        """sha and every commit that has it as an ancestor."""
        if sha not in self.desc_cache:
            if len(self.desc_cache) > 256:
                self.desc_cache.clear()
            seen = {sha}
            stack = [sha]
            while stack:
                for child in self.children.get(stack.pop(), ()):
                    if child not in seen:
                        seen.add(child)
                        stack.append(child)
            self.desc_cache[sha] = seen
        return self.desc_cache[sha]

    def is_ancestor(self, commit, of):
        """git merge-base --is-ancestor commit of (a commit is its own ancestor)."""
        commit, of = self.resolve(commit), self.resolve(of)
        return bool(commit and of and of in self.descendants(commit))
