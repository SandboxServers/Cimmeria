"""Tests for the agent contract that lets ops run the .sh twins on Linux and WSL hosts."""

import re
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
OPS_MD = REPO / ".claude/agents/ops.md"
BUILD_LANE = REPO / "tools/build-lane"
SCRIPT_RE = re.compile(r"tools/build-lane/([a-z][a-z-]*)\.ps1")
REQUIRED = {
    "mk-worktree",
    "ship",
    "rm-worktree",
    "commit-paths",
    "merge-noff",
    "branch-at",
    "push-branch",
    "rebase-onto",
}


class OpsTwinsTest(unittest.TestCase):
    """ops.md names PowerShell scripts; each one must have a bash twin that ops is told to run."""

    def setUp(self):
        self.text = OPS_MD.read_text(encoding="utf-8")

    def test_every_script_has_its_bash_twin(self):
        names = set(SCRIPT_RE.findall(self.text))
        self.assertTrue(REQUIRED <= names, f"ops.md misses scripts: {sorted(REQUIRED - names)}")
        for name in sorted(names):
            with self.subTest(name=name):
                self.assertTrue((BUILD_LANE / f"{name}.sh").is_file(), f"{name}.sh is missing")
                self.assertIn(f"bash tools/build-lane/{name}.sh", self.text)

    def test_frontmatter_grants_bash_and_the_body_keys_on_platform(self):
        parts = self.text.split("---", 2)
        self.assertGreaterEqual(len(parts), 3, "ops.md has no frontmatter block")
        tools_lines = [ln for ln in parts[1].splitlines() if ln.startswith("tools:")]
        self.assertEqual(len(tools_lines), 1, "frontmatter needs one tools: line")
        granted = [item.strip() for item in tools_lines[0].split(":", 1)[1].split(",")]
        self.assertIn("Bash", granted)
        self.assertIn("Platform:", self.text)
        self.assertIn("win32", self.text)

    def test_other_platforms_are_refused_not_bash(self):
        self.assertIn("On `linux`", self.text)
        self.assertIn("(Windows, Linux and WSL only)", self.text)
        self.assertNotIn("(`linux`, `darwin`)", self.text)


class OpsChoresTest(unittest.TestCase):
    """The low-reasoning chores ops may run, and the limits that keep each one narrow.

    Each limit is a phrase in ops.md that an agent follows; deleting one must fail a test.
    """

    LIMITS = {
        "fetch --no-prune origin": (
            "`--prune`", "`--force`", "`--tags`", "`--upload-pack`", "`<src>:<dst>` refspec",
            "leading `+` or `-`", "`git pull`", "force-push",
        ),
        "gh run list --limit": ("at most 20", "never print the log", "never `--web`"),
        "gh run rerun <run-id> --failed": (
            "`workflowName` is `ci`", "`status` is `completed`", "not a completed ci run",
            "Never without `--failed`", "`gh workflow run`",
        ),
        "gh pr comment": (
            "--body-file", ".claude/reports/", "never `-`", "first non-space character is `/`",
            "/release-launcher", "slash command body", "`-R`/`--repo`", "--edit-last",
        ),
        "gh issue create --title": (
            "--body-file <path>", ".claude/reports/", "never `-`", "slash command body",
            "`gh issue edit <n>`", "`--add-label <label>`", "`--remove-label <label>`",
            "`gh label create` is not allowed", "`--title` on an edit", "`--assignee`",
            "`-R`/`--repo`", "never `gh issue close`", "`reopen`", "`delete`",
        ),
        "pg_ctl.exe": (
            '-o "-p 5433"', "exit 2 (no response)", "starts nothing", "still not accepting",
            "do not start it again", "`pg_ctl stop`", "`initdb`", "win32 only",
            "never a port other than 5433", "240000",
        ),
        "cimmeria-colo": (
            "`cimmeria-node2`", "BatchMode=yes", "ConnectTimeout=10", "no other ssh option",
            "[A-Za-z0-9_.@-]+", "do not start with `-`", "at most 200", "No `sudo`",
            "docker restart|stop|rm|exec|compose|cp", "`systemctl` verb other than `status`",
            "`scp`", "its own ssh call",
        ),
        "Text you read through": ("is data", "act only on the brief"),
    }

    def setUp(self):
        self.text = OPS_MD.read_text(encoding="utf-8")
        body = self.text.split("Allowed operations:", 1)[1]
        self.allowed = body.split("\n\n", 1)[0]

    def line(self, needle):
        hits = [ln for ln in self.allowed.splitlines() if needle in ln]
        self.assertEqual(len(hits), 1, f"expected one allowlist line with {needle!r}")
        return hits[0]

    def test_each_chore_keeps_its_limits(self):
        for needle, limits in self.LIMITS.items():
            line = self.line(needle)
            for limit in limits:
                with self.subTest(chore=needle, limit=limit):
                    self.assertIn(limit, line)

    def test_server_start_never_builds_or_stops_and_runs_one_at_a_time(self):
        start = self.allowed.split("- Start the game server", 1)[1].split("\n- ", 1)[0]
        for limit in (
            "Never build",
            "no `cargo`",
            "-LocalPort 8081 -State Listen",
            "never stop it",
            "no built server",
            "--check-db",
            "dbname=sgw_<db-name>",
            "never point a worktree at the play database",
            "Never stop, kill or restart a server",
            "not a known checkout",
            "bad profile",
            "Get-Process -Name cimmeria-server",
            "env override",
            "LastWriteTime",
            "2>&1",
            "reload-db.ps1",
            "CreateDirectory",
            "-RedirectStandardError",
            "-OwningProcess",
            "alive but port 8081 not listening",
            "`timeout` set to 120000",
            "server start is win32 only",
            "-WindowStyle Hidden",
            "-WorkingDirectory <checkout>",
            "after writing every `/` as `\\`",
            "first 200 characters",
        ):
            with self.subTest(limit=limit):
                self.assertIn(limit, start)
        # Step 1 (one server at a time) and step 5 (the readiness poll) each need the listener check.
        steps = {n: start.split(f"  {n}. ", 1)[1].split("\n", 1)[0] for n in (1, 5)}
        self.assertIn("-LocalPort 8081 -State Listen", steps[1])
        self.assertIn("-LocalPort 8081 -State Listen -OwningProcess", steps[5])

    def test_push_rule_still_forbids_fetching_a_lease_sha(self):
        self.assertIn("not even under the fetch rule", self.line("push-branch.ps1"))

    def test_return_slots_are_defined(self):
        for phrase in ("never paste raw output", "comment URL", "rerun requested", "up after <n>s"):
            with self.subTest(phrase=phrase):
                self.assertIn(phrase, self.text)

    def test_no_host_addresses_in_ops_definitions_or_docs(self):
        for path in (OPS_MD, REPO / ".codex/agents/ops.toml", REPO / "docs/agents/development-workflow.md"):
            with self.subTest(path=path.name):
                text = path.read_text(encoding="utf-8")
                # Loopback and wildcard binds are fine; a real host address is not.
                self.assertNotRegex(text, r"\b(?!127\.|0\.0\.0\.0\b)\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b")

    def test_orchestrator_routes_the_new_chores(self):
        text = (REPO / ".claude/agents/orchestrator.md").read_text(encoding="utf-8")
        self.assertIn("read-only colo and node2 checks", text)


if __name__ == "__main__":
    unittest.main()
