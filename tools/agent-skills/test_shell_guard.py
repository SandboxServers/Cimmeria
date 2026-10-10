"""Tests for the PreToolUse shell guard in .claude/hooks/shell_guard.py."""

import json
import subprocess
import sys
import unittest
from pathlib import Path

GUARD = Path(__file__).resolve().parents[2] / ".claude/hooks/shell_guard.py"


def verdict(command, harness="claude"):
    payload = json.dumps({"tool_name": "Bash", "tool_input": {"command": command}})
    result = subprocess.run(
        [sys.executable, str(GUARD), "--harness", harness],
        input=payload, capture_output=True, text=True, check=False,
    )
    return result.returncode


class CargoRuleTest(unittest.TestCase):
    def test_direct_compile_refused_in_both_harnesses(self):
        for command in (
            "cargo check -p cimmeria-cell",
            "FOO=1 cargo nextest run",
            "cd crates && cargo build",
            "cargo --locked check",
            "cargo -q build",
            "cargo +1.98.1 t",
            "cargo --manifest-path x/Cargo.toml clippy",
            'pwsh -c "cargo check"',
            "bash -c 'cargo build -p x'",
            "cargo check; pwsh tools/build-lane/lane.ps1 echo",
        ):
            for harness in ("claude", "codex"):
                self.assertEqual(verdict(command, harness), 2, (command, harness))

    def test_lane_and_non_compiling_allowed(self):
        for command in (
            "pwsh tools/build-lane/lane.ps1 cargo check -p cimmeria-cell",
            "bash tools/build-lane/lane.sh --exclusive cargo build --workspace",
            "pwsh tools/build-lane/live-db-test.ps1 cargo nextest run",
            "cargo fmt --all -- --check",
            "cargo hakari generate --diff",
            "cargo check -p x # guard-ok",
            'git commit -m "run cargo test via lane"',
            'gh pr create --body "use cargo nextest run through the lane"',
            "cargo metadata --format-version 1 2>&1",
            "& pwsh tools/build-lane/lane.ps1 cargo build -p cimmeria-server",
        ):
            self.assertEqual(verdict(command), 0, command)


class ReadRuleTest(unittest.TestCase):
    def test_plain_reads_refused_for_claude_only(self):
        for command in (
            "grep -rn foo crates | head -20",
            "sed -n 1,40p crates/x.rs",
            "cat README.md",
            "Get-Content x.md -TotalCount 10",
            "cd crates && grep foo x.rs",
            "grep foo x 2>/dev/null",
            'grep -E "a|b" crates/x.rs',
            'rg "foo|bar" crates',
        ):
            self.assertEqual(verdict(command, "claude"), 2, command)
            self.assertEqual(verdict(command, "codex"), 0, command)

    def test_mixed_or_writing_commands_allowed(self):
        for command in (
            "git log --oneline | grep fix",
            "sed -i s/a/b/ x",
            "Get-Content x.md | Set-Content y.md",
            "cat > f.txt <<EOF",
            "ssh cimmeria-colo 'cat /etc/x'",
            "grep foo x # guard-ok",
            "ls -la",
            "tail -f logs/server.log",
            "Get-Content x.log -Wait -Tail 20",
            "head -c 200 file.bin",
            "type cargo",
            "git diff | Out-File d.txt",
        ):
            self.assertEqual(verdict(command), 0, command)

    def test_bad_input_allows(self):
        result = subprocess.run(
            [sys.executable, str(GUARD), "--harness", "claude"],
            input="not json", capture_output=True, text=True, check=False,
        )
        self.assertEqual(result.returncode, 0)


if __name__ == "__main__":
    unittest.main()
