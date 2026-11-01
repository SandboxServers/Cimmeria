"""The harness metrics subcommands: returns, coder-context and resume-cards.

Each reads the subagent transcripts of a checkout and writes one JSON file. The arguments are the C2 set. `dispatch`
fills in the defaults (the projects root, the repo root and its project prefix) and calls the module's `run`.
"""

from pathlib import Path

from ingest.store import default_projects_root, project_prefix

from . import coder_context, resume_cards, returns

NAMES = ("returns", "coder-context", "resume-cards")
MODULES = {"returns": returns, "coder-context": coder_context, "resume-cards": resume_cards}


def add_parsers(sub):
    """Add the three harness subcommands to `sub`, the metrics parser's subparsers."""
    for name in NAMES:
        p = sub.add_parser(name, help=f"harness transcript metrics: {name}")
        p.add_argument("--projects-root", default=None, metavar="DIR",
                       help="Claude Code's projects folder (default: ~/.claude/projects)")
        p.add_argument("--repo", default=None, metavar="DIR",
                       help="the repository root (default: the current directory)")
        p.add_argument("--since", default=None, metavar="YYYY-MM-DD", help="first dispatch date included")
        p.add_argument("--until", default=None, metavar="YYYY-MM-DD", help="first dispatch date excluded")
        p.add_argument("--out", required=True, metavar="DIR", help=f"directory for {name}.json")


def repo_root(repo) -> Path:
    """The repository root for a checkout path: the parent of `.claude` when the path lies in `.claude/worktrees/`."""
    parts = Path(repo).resolve().parts
    for i in range(len(parts) - 1):
        if parts[i] == ".claude" and parts[i + 1] == "worktrees":
            return Path(*parts[:i])
    return Path(repo).resolve()


def dispatch(args):
    """Run the harness subcommand `args.cmd` and return its status, or None when it is another subcommand."""
    if args.cmd not in MODULES:
        return None
    args.repo = repo_root(args.repo or Path.cwd())
    args.projects_root = args.projects_root or default_projects_root()
    args.prefix = project_prefix(args.repo)
    return MODULES[args.cmd].run(args)
