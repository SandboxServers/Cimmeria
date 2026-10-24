"""The Windows on Arm dry run must build like the release, then sign both zips in one feed."""
import unittest

from test_workflow_policy import workflow

ARM64_WORKFLOW = "./.github/workflows/launcher-desktop-windows-arm64.yml"
ARM64_PATH = ".github/workflows/launcher-desktop-windows-arm64.yml"


class WindowsArm64DryRun(unittest.TestCase):
    def test_arm64_dry_run_is_the_release_build_without_release_inputs(self):
        job = workflow("launcher-desktop.yml")["jobs"]["release-dry-run-arm64"]
        self.assertEqual(job["uses"], ARM64_WORKFLOW)
        self.assertEqual(job["needs"], "release-dry-run")
        with_inputs = job["with"]
        self.assertEqual(with_inputs["pieces_artifact"], "desktop-play-pieces-dry-run")
        self.assertIn("needs.release-dry-run.outputs.play_pieces_sha256",
                      with_inputs["pieces_sha256"])
        self.assertEqual(with_inputs["artifact_name"], "desktop-launcher-windows-arm64-dry-run")
        for release_input in ["tag", "version", "epoch", "updater_pubkey"]:
            self.assertNotIn(release_input, with_inputs)
        self.assertNotIn("secrets", job)

    def test_feed_dry_run_signs_both_zips_after_both_builds(self):
        jobs = workflow("launcher-desktop.yml")["jobs"]
        feed = jobs["release-dry-run-feed"]
        self.assertEqual(set(feed["needs"]), {"release-dry-run", "release-dry-run-arm64"})
        commands = "\n".join(step.get("run", "") for step in feed["steps"])
        self.assertIn("build-desktop.sh feed-dry-run", commands)
        downloads = {step["with"]["name"] for step in feed["steps"]
                     if step.get("uses", "").startswith("actions/download-artifact")}
        self.assertEqual(downloads, {"desktop-launcher-windows-dry-run",
                                     "desktop-launcher-windows-arm64-dry-run"})
        release_commands = "\n".join(step.get("run", "")
                                     for step in jobs["release-dry-run"]["steps"])
        self.assertNotIn("feed-dry-run", release_commands)
        self.assertNotIn("secrets.", str(feed))

    def test_paths_filters_cover_the_arm64_workflow(self):
        triggers = workflow("launcher-desktop.yml")[True]
        for event in ["pull_request", "push"]:
            self.assertIn(ARM64_PATH, triggers[event]["paths"])


if __name__ == "__main__":
    unittest.main()
