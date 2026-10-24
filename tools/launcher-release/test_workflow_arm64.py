"""The Windows on Arm launcher builds natively, is pinned, and rides the release's critical path."""
import re
import unittest

from test_workflow_policy import ROOT, workflow

REUSABLE = "./.github/workflows/launcher-desktop-windows-arm64.yml"
SHA_PIN = re.compile(r"@[0-9a-f]{40}$")


def needs_of(job):
    needs = job.get("needs", [])
    return [needs] if isinstance(needs, str) else needs


class WindowsArm64Release(unittest.TestCase):
    def test_reusable_build_is_native_arm64_and_pinned(self):
        reusable = workflow("launcher-desktop-windows-arm64.yml")
        self.assertEqual(reusable["permissions"], {})
        self.assertIn("workflow_call", reusable[True])
        job = reusable["jobs"]["build"]
        self.assertEqual(job["runs-on"], "windows-11-arm")
        self.assertEqual(job["env"]["WINDOWS_ARCH"], "arm64")
        run = "\n".join(step.get("run", "") for step in job["steps"])
        for text in ["play-pieces-in", "build-desktop.sh launcher", "build-desktop.sh verify",
                     "build-desktop.sh package", "aarch64-pc-windows-msvc"]:
            self.assertIn(text, run)
        self.assertNotIn("build-desktop.sh helpers", run)
        for step in job["steps"]:
            if "uses" in step and not step["uses"].startswith("./"):
                self.assertRegex(step["uses"], SHA_PIN)

    def test_reusable_build_never_sees_a_signing_key(self):
        text = (ROOT / ".github/workflows/launcher-desktop-windows-arm64.yml").read_text()
        self.assertNotIn("MINISIGN", text)
        self.assertNotIn("environment", workflow("launcher-desktop-windows-arm64.yml")["jobs"]["build"])

    def test_release_calls_it_with_the_x64_jobs_pieces(self):
        job = workflow("launcher-release.yml")["jobs"]["desktop-windows-arm64"]
        self.assertEqual(job["uses"], REUSABLE)
        self.assertIn("desktop-windows", needs_of(job))
        inputs = job["with"]
        self.assertIn("needs.desktop-windows.outputs.play_pieces_sha256", inputs["pieces_sha256"])
        self.assertTrue(inputs["pieces_artifact"].startswith("launcher-play-pieces-"))
        self.assertIn("vars.LAUNCHER_UPDATER_PUBKEY", inputs["updater_pubkey"])
        self.assertIn("LAUNCHER_MANIFEST_PUBKEY_HEX", job["secrets"])

    def test_sign_and_publish_wait_for_both_windows_builds(self):
        jobs = workflow("launcher-release.yml")["jobs"]
        both = {"desktop-windows", "desktop-windows-arm64"}
        for name in ["sign", "publish"]:
            self.assertTrue(both <= set(needs_of(jobs[name])))
            downloads = [s["with"]["name"] for s in jobs[name]["steps"]
                         if s.get("uses", "").startswith("actions/download-artifact")]
            self.assertTrue(any(d.startswith("launcher-zip-arm64-") for d in downloads))
        self.assertIn("needs.desktop-windows-arm64.result == 'success'", jobs["publish"]["if"])
        feed = [s for s in jobs["sign"]["steps"]
                if s.get("with", {}).get("name", "").startswith("launcher-feed-")]
        self.assertEqual(len(feed), 1)
        for path in ["artifacts/StargateWorlds-Launcher-windows-x64.zip.sig",
                     "artifacts/StargateWorlds-Launcher-windows-arm64.zip.sig",
                     "artifacts/latest.json"]:
            self.assertIn(path, feed[0]["with"]["path"])

    def test_play_pieces_are_on_the_release_critical_path(self):
        steps = workflow("launcher-release.yml")["jobs"]["desktop-windows"]["steps"]
        touching = [s for s in steps
                    if "build-desktop.sh play-pieces" in s.get("run", "")
                    or s.get("with", {}).get("name", "").startswith("launcher-play-pieces-")]
        self.assertEqual(len(touching), 2)
        for step in touching:
            self.assertNotIn("continue-on-error", step)
        upload = next(s for s in touching if "uses" in s)
        self.assertGreaterEqual(upload["with"]["retention-days"], 7)


if __name__ == "__main__":
    unittest.main()
