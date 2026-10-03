"""Tool-call fingerprints: the only tool input that reaches the database.

Rules in transcript-format.md § Tool-call fingerprints. A shell command keeps
its executable's base name, plus its first argument only when the pair is on
ALLOWED_PAIRS: any other argument can hold a credential, a URL or a path.
Adding to the allowlist is a reviewed change.
"""

import re

SHELL_TOOLS = {"Bash", "PowerShell"}
PATH_TOOLS = {"Read": "file_path", "Edit": "file_path", "Write": "file_path", "Grep": "path", "Glob": "path"}

ALLOWED_PAIRS = {
    "cargo": {"build", "check", "clippy", "fmt", "hakari", "install", "metadata", "nextest", "run", "test",
              "tree", "update"},
    "git": {"add", "apply", "blame", "branch", "checkout", "cherry-pick", "clean", "commit", "config", "diff",
            "fetch", "grep", "log", "ls-files", "merge", "merge-base", "mv", "pull", "push", "rebase", "remote",
            "reset", "restore", "rev-parse", "rm", "show", "stash", "status", "switch", "tag", "worktree"},
    "gh": {"api", "auth", "issue", "label", "pr", "release", "repo", "run", "search", "workflow"},
    "sed": {"-n", "-i", "-e", "-E"},
    "npm": {"ci", "install", "run", "test"},
    "bash": {"tools/build-lane/lane.sh", "tools/build-lane/live-db-test.sh", "tools/build-lane/mk-worktree.sh",
             "tools/build-lane/rm-worktree.sh", "tools/build-lane/reload-db.sh", "tools/test-live-db.sh",
             "tools/lint-md.sh", "tools/check-figure-sources.sh", "tools/lint-figure-style.sh"},
}

TOKEN = re.compile(r"""(?:[^\s"']+|"[^"]*"|'[^']*')+""")
ASSIGNMENT = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*=")
SEPARATORS = {"&&", ";", "||", "|"}


def _unquote(token):
    if len(token) >= 2 and token[0] == token[-1] and token[0] in "\"'":
        return token[1:-1]
    return token


def _base(exe):
    name = re.split(r"[\\/]", _unquote(exe))[-1]
    return re.sub(r"\.exe$", "", name, flags=re.IGNORECASE)


def command_head(command):
    """'cd x && FOO=1 cargo nextest run --token y' -> 'cargo nextest'."""
    tokens = TOKEN.findall(command or "")
    while tokens:
        if ASSIGNMENT.match(tokens[0]):
            tokens.pop(0)
        elif tokens[0] == "cd" and len(tokens) >= 3 and tokens[2] in SEPARATORS:
            tokens = tokens[3:]
        elif tokens[0] == "cd" and len(tokens) >= 2 and tokens[1].endswith(";"):
            tokens = tokens[2:]
        elif tokens[0] in ("&", "env"):
            tokens.pop(0)
        else:
            break
    if not tokens:
        return None
    exe = _base(tokens[0])
    if not exe:
        return None
    if len(tokens) > 1:
        arg = _unquote(tokens[1]).replace("\\", "/")
        if arg in ALLOWED_PAIRS.get(exe, ()):
            return f"{exe} {arg}"
    return exe


def checkout_root(cwd):
    """The main checkout a cwd belongs to, lower-cased with '/' separators."""
    if not isinstance(cwd, str) or not cwd:
        return None
    c = cwd.replace("\\", "/").rstrip("/").lower()
    return re.split(r"/\.claude/worktrees/", c)[0]


def repo_relative(path, root=None):
    """A path cut to the checkout, or '<external>'. Repo-relative input is kept.

    root is checkout_root() of the record's cwd; a worktree path is cut at
    `.claude/worktrees/<name>/` whatever the root.
    """
    if not isinstance(path, str) or not path:
        return None
    p = path.replace("\\", "/")
    low = p.lower()
    m = re.search(r"/\.claude/worktrees/[^/]+(/|$)", low)
    if m:
        return p[m.end():] or "."
    if re.match(r"^([a-z]:)?/", low) or p.startswith("~"):
        if root and (low == root or low.startswith(root + "/")):
            return p[len(root) + 1:] or "."
        return "<external>"
    if ".." in p.split("/"):
        return "<external>"
    return p


def fingerprint(tool_name, tool_input, root=None):
    """(fingerprint, mcp_server) for one tool_use block; root as for repo_relative."""
    tool_input = tool_input if isinstance(tool_input, dict) else {}
    if tool_name.startswith("mcp__"):
        return tool_name, tool_name.split("__")[1] if tool_name.count("__") >= 2 else None
    if tool_name in SHELL_TOOLS:
        return command_head(tool_input.get("command")), None
    if tool_name in PATH_TOOLS:
        return repo_relative(tool_input.get(PATH_TOOLS[tool_name]), root), None
    return None, None


GH_PR = re.compile(r"\bgh\s+pr\s+(create|merge|checks|view|diff|comment|edit|review|ready|close)\b([^&|;\n]*)")
PR_NUMBER = re.compile(r"(?:^|\s)#?(\d{1,6})(?=\s|$)")
PR_URL = re.compile(r"/pull/(\d{1,6})\b")
HASH_NUMBER = re.compile(r"#(\d{1,6})\b")


def gh_pr_ref(command):
    """The PR number a `gh pr <verb> N` names (GH_PR's verbs), and whether it was a gh pr command.

    `gh pr create` names no number; its result does (see result_pr_ref).
    """
    m = GH_PR.search(command or "")
    if not m:
        return None, False
    if m.group(1) == "create":
        return None, True           # a number in its title or body is not the PR's
    args = m.group(2)
    n = PR_NUMBER.search(args) or PR_URL.search(args)
    return (int(n.group(1)) if n else None), True


def gh_pr_verb(command):
    """The subcommand of a `gh pr <verb>` call (GH_PR's verbs), else None."""
    m = GH_PR.search(command or "")
    return m.group(1) if m else None


def result_pr_ref(text):
    m = PR_URL.search(text or "")
    return int(m.group(1)) if m else None


def description_pr_ref(description):
    """The PR a description names, when exactly one distinct #N appears."""
    nums = set(HASH_NUMBER.findall(description or ""))
    return int(nums.pop()) if len(nums) == 1 else None
