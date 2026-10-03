"""Campaign tags for PRs and branches, from campaigns.json (attribution.md § Campaigns)."""

import json
import re
from pathlib import Path

CONFIG = Path(__file__).resolve().parent.parent / "campaigns.json"
ISSUE = re.compile(r"#(\d{1,6})\b")
DEFAULT_BRANCHES = ("main", "master")


class Campaigns:
    def __init__(self, rules):
        self.exact = {}
        self.prefixes = []
        self.issues = {}
        for name, rule in rules.items():
            if name.startswith("_"):
                continue
            for b in rule.get("branches", ()):
                self.exact[b] = name
            for p in rule.get("prefixes", ()):
                self.prefixes.append((p, name))
            for n in rule.get("issues", ()):
                self.issues[int(n)] = name
        self.prefixes.sort(key=lambda x: -len(x[0]))
        self.names = sorted(n for n in rules if not n.startswith("_"))

    @classmethod
    def load(cls, path=None):
        return cls(json.loads(Path(path or CONFIG).read_text(encoding="utf-8")))

    def of_branch(self, branch):
        if not branch:
            return None
        if branch in self.exact:
            return self.exact[branch]
        return next((name for p, name in self.prefixes if branch.startswith(p)), None)

    def of_title(self, title):
        return next((self.issues[int(n)] for n in ISSUE.findall(title or "") if int(n) in self.issues), None)


def tag_prs(db, campaigns):
    """Set prs.campaign for every PR; returns the number tagged.

    Head branch, then a tracking issue in the title, then the base branch. An
    integration PR (one other PRs target) that no rule names is its own
    campaign, `pr-<N>` after its number (branch names never reach a report),
    and so are the PRs into it.
    """
    rows = db.execute("SELECT pr_number, head_branch, title, base_branch FROM prs").fetchall()
    bases = {base for _, _, _, base in rows if base and base not in DEFAULT_BRANCHES}
    tags = {}
    for n, head, title, base in rows:
        tags[n] = campaigns.of_branch(head) or campaigns.of_title(title) or (f"pr-{n}" if head in bases else None)
    by_head = {head: tags[n] for n, head, _, _ in rows if tags[n]}
    for n, head, title, base in rows:
        if tags[n] is None and base and base not in DEFAULT_BRANCHES:
            tags[n] = campaigns.of_branch(base) or by_head.get(base)
    db.executemany("UPDATE prs SET campaign = ? WHERE pr_number = ?", [(c, n) for n, c in tags.items()])
    return sum(1 for c in tags.values() if c)
