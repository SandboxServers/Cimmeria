"""Launcher CI speed changes must preserve native coverage and signing isolation."""
from pathlib import Path
import tomllib
import unittest
import sys
import tempfile
import zipfile
from unittest.mock import patch

import yaml

ROOT = Path(__file__).resolve().parents[2]


def workflow(name):
    return yaml.safe_load((ROOT / ".github/workflows" / name).read_text())


class WorkflowPolicy(unittest.TestCase):
    def package_fixture(self, root, output):
        script = (ROOT / "tools/launcher-release/build-desktop.sh").read_text()
        code = script.split("stage_package() {", 1)[1].split("<<'PY'\n", 1)[1].split("\nPY\n", 1)[0]
        with patch.object(sys, "argv", ["package", str(root), str(output)]):
            exec(compile(code, "build-desktop.sh/package", "exec"), {})

    def test_windows_zip_contains_one_root_executable_and_preserves_its_bytes(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "package"
            root.mkdir()
            (root / "launcher.exe").write_bytes(b"embedded executable fixture")
            output = Path(temporary) / "launcher.zip"
            self.package_fixture(root, output)
            with zipfile.ZipFile(output) as archive:
                self.assertEqual(archive.namelist(), ["launcher.exe"])
                self.assertEqual(archive.read("launcher.exe"), b"embedded executable fixture")
            (root / "windows").mkdir()
            (root / "windows/helper.exe").write_bytes(b"unexpected sidecar")
            with self.assertRaises(SystemExit):
                self.package_fixture(root, output)

    def test_windows_build_requires_embedded_pins_and_verifies_without_sidecars(self):
        script = (ROOT / "tools/launcher-release/build-desktop.sh").read_text()
        launcher = script.split("stage_launcher() {", 1)[1].split("stage_verify() {", 1)[0]
        self.assertIn("CIMMERIA_EMBED_PAYLOAD=1", launcher)
        verify = script.split("stage_verify() {", 1)[1].split("stage_package() {", 1)[0]
        self.assertIn("--verify-embedded-payload", verify)
        self.assertIn("corrupt cache fixture", verify)

    def test_native_checks_keep_both_hosts_and_every_validation_layer(self):
        native = workflow("launcher-desktop.yml")["jobs"]["native-engine"]
        self.assertEqual(set(native["strategy"]["matrix"]["os"]),
                         {"windows-latest", "macos-latest", "ubuntu-latest"})
        commands = "\n".join(step.get("run", "") for step in native["steps"])
        for package in ["cimmeria-launcher-engine", "cimmeria-launcher-desktop"]:
            self.assertRegex(commands, rf"cargo test[^\n]*-p {package}(?:\s|$)")
            self.assertRegex(commands, rf"cargo clippy[^\n]*-p {package}[^\n]*--all-targets")
        self.assertRegex(commands, r"cargo build[^\n]*-p cimmeria-launcher-desktop(?:\s|$)")
        self.assertIn("npm test", commands)
        self.assertIn("npm run uat --", commands)
        self.assertIn("npm run build", commands)

    def test_native_caches_cannot_consume_budget_per_pr(self):
        native = workflow("launcher-desktop.yml")["jobs"]["native-engine"]
        caches = [s for s in native["steps"] if "rust-cache@" in s.get("uses", "")]
        self.assertEqual(len(caches), 1)
        self.assertIn("runner.environment == 'github-hosted'", caches[0]["if"])
        self.assertEqual(caches[0]["with"]["save-if"],
                         "${{ github.ref == 'refs/heads/main' }}")
        self.assertIn("crates/launcher/desktop", caches[0]["with"]["workspaces"])

    def test_only_pr_runs_cancel_and_real_releases_are_never_cancelled(self):
        self.assertEqual(workflow("launcher-desktop.yml")["concurrency"]["cancel-in-progress"],
                         "${{ github.event_name == 'pull_request' }}")
        self.assertIs(workflow("launcher-release.yml")["concurrency"]["cancel-in-progress"], False)

    def test_signing_keeps_clean_runner_and_step_scoped_private_keys(self):
        release = workflow("launcher-release.yml")
        sign = release["jobs"]["sign"]
        self.assertEqual(sign["environment"], "launcher-release")
        private = {"LAUNCHER_UPDATER_MINISIGN_KEY", "LAUNCHER_UPDATER_MINISIGN_PASSWORD"}
        for environment in [release.get("env", {}), sign.get("env", {})]:
            self.assertTrue(private.isdisjoint(environment))
        key_steps = []
        for step in sign["steps"]:
            if "uses" in step:
                self.assertTrue(step["uses"].startswith(("actions/", "./.github/actions/rust-toolchain")))
                self.assertNotIn("cache", step["uses"])
                self.assertNotIn("setup-node", step["uses"])
            if private.intersection(step.get("env", {})):
                key_steps.append(step)
                self.assertIn("build-desktop.sh sign", step["run"])
                self.assertNotRegex(step["run"], r"\b(cargo|npm|node)\b")
        self.assertEqual(len(key_steps), 1)
        build = next(s for s in sign["steps"] if "build-desktop.sh feed-tool" in s.get("run", ""))
        self.assertLess(sign["steps"].index(build), sign["steps"].index(key_steps[0]))

    def test_tool_profile_retains_release_semantics_without_changing_player_profile(self):
        manifest = tomllib.loads((ROOT / "crates/launcher/desktop/Cargo.toml").read_text())
        profiles = manifest["profile"]
        tools = profiles["release-tools"]
        self.assertEqual(tools["inherits"], "release")
        self.assertEqual(tools["opt-level"], 0)
        self.assertEqual(tools["codegen-units"], 256)
        self.assertIs(tools["lto"], False)
        self.assertFalse(tools.get("debug-assertions", False))
        self.assertNotIn("release", profiles)


if __name__ == "__main__":
    unittest.main()
