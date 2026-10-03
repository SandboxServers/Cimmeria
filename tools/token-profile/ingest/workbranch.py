"""Where a tool call's work happened: the worktree its input names and the branch its git output names.

An in-process teammate's transcript records carry the coordinator process's
cwd and gitBranch, not the worktree the teammate works in (attribution.md §
Work branch). Its own tool calls say where it worked: the paths it reads and
edits, the directory its shell commands enter, and what git prints back.
"""

import re

# A worktree path, absolute or relative: `.claude/worktrees/<name>` after a separator, quote or start.
WORKTREE_REF = re.compile(r"(?:^|[\s\"'=(\\/])\.claude[\\/]worktrees[\\/]([A-Za-z0-9._-]+)")
PATH_KEYS = ("command", "file_path", "path", "notebook_path", "cwd")

# git status -sb: `## <branch>...<upstream>`, or `## <branch>` alone. The bare form must contain a
# slash, so a Markdown heading printed by the same command (`## Purpose`) is not read as a branch.
BRANCH_PATTERNS = (
    re.compile(r"^On branch (\S+)\s*$", re.M),                                    # git status
    re.compile(r"^## ([A-Za-z0-9._/+-]+?)\.\.\.\S+", re.M),                         # git status -sb
    re.compile(r"^## ([A-Za-z0-9._+-]+/[A-Za-z0-9._/+-]+)\s*$", re.M),
    re.compile(r"^\[([^\s\]]+)(?: \(root-commit\))? [0-9a-f]{7,40}\] ", re.M),    # git commit
    re.compile(r"Switched to (?:a new )?branch '([^']+)'"),                        # git checkout / switch
    re.compile(r"Successfully rebased and updated refs/heads/(\S+?)\.?\s*$", re.M),  # git rebase
)
# A ref update line, `<flag> <old>..<new> <src> -> <dst>`. git push prints them under `To <remote>`
# and git fetch under `From <remote>`; only a push names the branch the work is on.
REF_UPDATE = re.compile(r"^\s*[-+*=!]?\s*(?:\[new branch\]|[0-9a-f]{7,40}\.\.\.?[0-9a-f]{7,40})\s+\S+\s+->\s+(\S+)")
WORKTREE_LIST = re.compile(r"^(\S.*?)\s+[0-9a-f]{7,40}\s+\[([^\]\s]+)\]\s*$", re.M)
WORKTREE_PORCELAIN = re.compile(r"^worktree (.+)\n(?:HEAD [0-9a-f]+\n)?branch refs/heads/(\S+)", re.M)
GIT = re.compile(r"\bgit\b")
SCAN_CHARS = 20000


def worktree_name(text):
    """`.claude/worktrees/<name>` for the one worktree text names, else None (none, or several)."""
    names = set(WORKTREE_REF.findall(text or ""))
    return f".claude/worktrees/{names.pop()}" if len(names) == 1 else None


def worktree_ref(tool_input):
    """The one worktree a tool call's input names, in any of its path or command fields."""
    if not isinstance(tool_input, dict):
        return None
    names = {worktree_name(v) for k, v in tool_input.items() if k in PATH_KEYS and isinstance(v, str)}
    names.discard(None)
    return names.pop() if len(names) == 1 else None


def is_git_command(command):
    return bool(GIT.search(command or ""))


def branch_seen(output):
    """The one branch a git command's output names, else None. `main` counts: it says the work is not on a branch."""
    text = (output or "")[:SCAN_CHARS]
    found = {m for p in BRANCH_PATTERNS for m in p.findall(text)} | set(_pushed(text))
    found = {b[len("origin/"):] if b.startswith("origin/") else b for b in found if "..." not in b}
    found.discard("HEAD")
    return found.pop() if len(found) == 1 else None


def _pushed(text):
    section = None
    for line in text.splitlines():
        if line.startswith(("To ", "From ")):
            section = line.split(" ", 1)[0]
            continue
        m = REF_UPDATE.match(line)
        if m and section == "To":
            yield m.group(1)


def worktree_list(output):
    """(worktree, branch) for every worktree line of `git worktree list` (plain or --porcelain)."""
    text = (output or "")[:SCAN_CHARS]
    out = []
    for path, branch in WORKTREE_LIST.findall(text) + WORKTREE_PORCELAIN.findall(text):
        name = worktree_name(" " + path)
        if name:
            out.append((name, branch))
    return out
