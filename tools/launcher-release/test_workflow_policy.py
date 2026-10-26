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


def apt_packages(job):
    """The package names on the job's `apt-get install` lines."""
    lines = [line for step in job["steps"] for line in step.get("run", "").splitlines()
             if "apt-get install" in line]
    return {word for line in lines for word in line.split()}


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

    def test_linux_release_builds_read_only_and_publishes_without_build_code(self):
        release = workflow("launcher-release.yml")["jobs"]
        build = release["desktop-linux"]
        self.assertEqual(build["uses"], "./.github/workflows/launcher-desktop-linux.yml")
        self.assertEqual(build["permissions"], {"contents": "read"})
        self.assertEqual(set(build["secrets"]), {"LAUNCHER_MANIFEST_PUBKEY_HEX"})
        reusable = workflow("launcher-desktop-linux.yml")
        self.assertEqual(reusable["permissions"], {})
        self.assertEqual(reusable["jobs"]["build"]["permissions"], {"contents": "read"})

        publish = release["publish-linux"]
        self.assertEqual(publish["permissions"], {"contents": "write"})
        # Both platform publish jobs rewrite the dated notes; never at once.
        self.assertIn("publish-macos", publish["needs"])
        self.assertIn("!cancelled()", publish["if"])
        for step in publish["steps"]:
            if "uses" in step:
                self.assertRegex(step["uses"], r"^actions/(checkout|download-artifact)@[0-9a-f]{40}")
            if "run" in step:
                self.assertNotRegex(step["run"], r"\b(cargo|npm|node|apt-get)\b")
        commands = "\n".join(s.get("run", "") for s in publish["steps"])
        self.assertIn("publish.sh check", commands)
        self.assertIn("publish.sh linux", commands)
        self.assertIn("publish.sh linux-current", commands)

    def test_linux_appimage_tools_are_pinned_before_the_bundler_runs(self):
        script = (ROOT / "tools/launcher-release/build-desktop-linux.sh").read_text()
        tools = script.split("APPIMAGE_TOOLS=(", 1)[1].split("\n)", 1)[0]
        entries = [line.strip().strip('"').split() for line in tools.strip().splitlines()]
        self.assertEqual({e[0] for e in entries}, {
            "AppRun-x86_64",
            "linuxdeploy-07333c6-x86_64.AppImage",
            "linuxdeploy-plugin-appimage.AppImage",
        })
        for name, url, digest in entries:
            self.assertTrue(url.startswith("https://github.com/"), name)
            self.assertRegex(digest, r"^[0-9a-f]{64}$", name)
        app = script.split("stage_app() {", 1)[1].split("\n}\n", 1)[0]
        self.assertLess(app.index("stage_tools"), app.index("cargo tauri build"))

    def test_linux_builds_link_with_lld_not_mold(self):
        # PP-U-FIX: mold 1.0.3 links a UnRAR that rejects every RAR header, so the
        # Linux launcher builds set the Linux target's rustflags to lld.
        key = "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS"
        lld = "-C link-arg=-fuse-ld=lld"
        linux = workflow("launcher-desktop-linux.yml")["jobs"]["build"]
        app = next(s for s in linux["steps"] if "build-desktop-linux.sh app" in s.get("run", ""))
        self.assertEqual(app["env"][key], lld)
        self.assertIn("lld", apt_packages(linux))
        native = workflow("launcher-desktop.yml")["jobs"]["native-engine"]
        self.assertEqual(native["env"][key], lld)
        self.assertIn("lld", apt_packages(native))

    def test_release_verify_runs_the_bundled_unrar_self_test(self):
        script = (ROOT / "tools/launcher-release/build-desktop-linux.sh").read_text()
        verify = script.split("stage_verify() {", 1)[1].split("stage_package() {", 1)[0]
        self.assertIn('"$exe" --self-test-rar', verify)
        main = (ROOT / "crates/launcher/desktop/shell/src/main.rs").read_text()
        self.assertIn("self_test_rar::requested(", main)

    def test_every_pr_dry_runs_the_linux_release_build(self):
        dry = workflow("launcher-desktop.yml")["jobs"]["release-dry-run-linux"]
        self.assertEqual(dry["uses"], "./.github/workflows/launcher-desktop-linux.yml")
        self.assertIs(dry["with"]["dev_manifest_key"], True)
        self.assertTrue(dry["with"]["tag"])
        self.assertTrue(dry["with"]["version"])
        self.assertNotIn("secrets", dry)

    def test_real_key_linux_dry_run_is_manual_and_artifact_only(self):
        desktop = workflow("launcher-desktop.yml")
        triggers = desktop[True]  # PyYAML reads the `on:` key as True
        self.assertIn("workflow_dispatch", triggers)
        real = desktop["jobs"]["release-dry-run-linux-real-key"]
        self.assertEqual(real["if"], "github.event_name == 'workflow_dispatch'")
        self.assertEqual(real["uses"], "./.github/workflows/launcher-desktop-linux.yml")
        self.assertEqual(real["permissions"], {"contents": "read"})
        self.assertIs(real["with"]["dev_manifest_key"], False)
        self.assertIs(real["with"]["save_cache"], False)
        self.assertNotEqual(real["with"]["artifact_name"],
                            desktop["jobs"]["release-dry-run-linux"]["with"]["artifact_name"])
        self.assertEqual(set(real["secrets"]), {"LAUNCHER_MANIFEST_PUBKEY_HEX"})
        # Nothing in this workflow can write to a release.
        self.assertEqual(desktop["permissions"], {"contents": "read"})
        for job in desktop["jobs"].values():
            self.assertNotIn("write", set((job.get("permissions") or {}).values()))

    def test_linux_build_pins_embeds_and_verifies_the_four_windows_pieces(self):
        script = (ROOT / "tools/launcher-release/build-desktop-linux.sh").read_text()
        pieces = script.split("PIECES=(", 1)[1].split("\n)", 1)[0]
        entries = [line.strip().strip('"').split() for line in pieces.strip().splitlines()]
        self.assertEqual({e[0]: tuple(e[1:]) for e in entries}, {
            "cimmeria-launch-worker.exe": ("launch", "CIMMERIA_LAUNCH_HELPER_SHA256"),
            "cimmeria-prerequisite-worker.exe": ("prerequisite", "CIMMERIA_PREREQUISITE_HELPER_SHA256"),
            "cimmeria_client_patches.dll": ("client-patches", "CIMMERIA_CLIENT_PATCHES_SHA256"),
            "cimmeria_client_telemetry.dll": ("client-telemetry", "CIMMERIA_CLIENT_TELEMETRY_SHA256"),
        })
        app = script.split("stage_app() {", 1)[1].split("\n}\n", 1)[0]
        self.assertIn("export CIMMERIA_EMBED_PAYLOAD=1", app)
        self.assertIn('"resources/windows/":null', app)
        verify = script.split("stage_verify() {", 1)[1].split("\n}\n", 1)[0]
        self.assertIn("--verify-embedded-payload", verify)

    def test_linux_release_and_dry_run_hand_the_windows_pieces_to_the_linux_build(self):
        for name, job, producer in [("launcher-release.yml", "desktop-linux", "mac-pieces"),
                                    ("launcher-desktop.yml", "release-dry-run-linux", "release-dry-run-mac-pieces")]:
            linux = workflow(name)["jobs"][job]
            self.assertIn(producer, linux["needs"], job)
            self.assertTrue(linux["with"].get("pieces_artifact"), job)
            self.assertIn(f"needs.{producer}.outputs.", linux["with"].get("pieces_sha256", ""), job)
        runs = [s.get("run", "") for s in workflow("launcher-desktop-linux.yml")["jobs"]["build"]["steps"]]
        staged = next(i for i, r in enumerate(runs) if "build-desktop-linux.sh pieces" in r)
        built = next(i for i, r in enumerate(runs) if "build-desktop-linux.sh app" in r)
        self.assertLess(staged, built)

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
