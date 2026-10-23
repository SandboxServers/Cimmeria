"""Guard iteration improvements without dropping validation or publication barriers."""
import contextlib
import io
import json
from pathlib import Path
import re
import unittest

import yaml

ROOT = Path(__file__).resolve().parents[2]


def workflow(name):
    return yaml.safe_load((ROOT / ".github/workflows" / name).read_text())


def commands(job):
    return "\n".join(step.get("run", "") for step in job["steps"])


class WorkflowPolicy(unittest.TestCase):
    def test_contracts_run_in_parallel_and_remain_gating(self):
        jobs = workflow("test.yml")["jobs"]
        contracts = jobs["contracts"]
        self.assertEqual(contracts["needs"], "changes")
        self.assertEqual(contracts["if"], jobs["build-and-test"]["if"])
        for suite in ["build-lane", "docs-gen", "token-profile", "ci-runners"]:
            command = f"-m unittest discover -s tools/{suite}"
            self.assertIn(command, commands(contracts))
            self.assertNotIn(command, commands(jobs["build-and-test"]))
        for job in ["gate", "nightly-report"]:
            self.assertIn("contracts", jobs[job]["needs"])
        self.assertEqual(jobs["gate"]["if"], "always()")
        gate = jobs["gate"]["steps"][0]["run"]
        code = gate.split("<<'PY'\n", 1)[1].rsplit("\nPY", 1)[0]
        # Run the real gate body: a failed/cancelled contract job cannot land.
        import os
        from unittest.mock import patch
        for result, expected in [("failure", 1), ("cancelled", 1), ("success", 0), ("skipped", 0)]:
            needs = {job: {"result": "success"} for job in jobs["gate"]["needs"]}
            needs["contracts"]["result"] = result
            with patch.dict(os.environ, NEEDS=json.dumps(needs)), contextlib.redirect_stdout(io.StringIO()):
                with self.assertRaises(SystemExit) as exit_status:
                    exec(compile(code, "workflow-gate", "exec"), {})
            self.assertEqual(exit_status.exception.code, expected)

    def test_default_chaos_database_and_generator_checks_are_retained(self):
        build = commands(workflow("test.yml")["jobs"]["build-and-test"])
        for command in ["cargo build --workspace", "cargo nextest run --profile=ci --workspace",
                        "test-live-db.sh --build-only", "test-live-db.sh", "live-db-schema-load.sh wait",
                        "cargo test --doc", "cargo hakari verify", "effect_nvps_from_desc.py --check",
                        "weapon_requirement_audit.py --check", "abilities.py --check"]:
            self.assertIn(command, build)
        for action in ["cargo clippy", "cargo nextest run", "cargo check"]:
            self.assertRegex(build, action + r"[^\n]*--features lab-chaos")

    def test_windows_checks_keep_coverage_without_serial_clippy_dependency(self):
        names = ["client-launch.yml", "client-patches-build.yml", "client-telemetry-build.yml", "lab.yml"]
        for name in names:
            w = workflow(name)
            self.assertEqual(w["env"]["CARGO_PROFILE_DEV_DEBUG"], 0)
            self.assertEqual(w["env"]["CARGO_PROFILE_TEST_DEBUG"], 0)
            for job in ["clippy", "test"]:
                self.assertIn(job, w["jobs"])
            for job in ["test", "build", "coverage"]:
                if job in w["jobs"]:
                    self.assertNotIn("clippy", str(w["jobs"][job].get("needs", [])))
            self.assertIn("cargo nextest run", commands(w["jobs"]["test"]))
            self.assertIn("--all-targets -- -D warnings", commands(w["jobs"]["clippy"]))
        telemetry = workflow("client-telemetry-build.yml")
        for job in ["clippy", "test"]:
            self.assertIn("--features lab-bridge", commands(telemetry["jobs"][job]))

    def test_runtime_caches_are_hosted_and_main_only_and_targets_stay_isolated(self):
        w = workflow("launcher-runtime-probe.yml")
        self.assertEqual(w["concurrency"]["cancel-in-progress"], "${{ github.event_name == 'pull_request' }}")
        self.assertEqual(w["env"]["CARGO_PROFILE_DEV_DEBUG"], 0)
        self.assertEqual(w["env"]["CARGO_PROFILE_TEST_DEBUG"], 0)
        for job in w["jobs"].values():
            cache, = [s for s in job["steps"] if "rust-cache@" in s.get("uses", "")]
            self.assertEqual(cache["if"], "runner.environment == 'github-hosted'")
            self.assertEqual(cache["with"]["save-if"], "${{ github.ref == 'refs/heads/main' }}")
        runtime = commands(w["jobs"]["runtime-probe"])
        for action in ["clippy", "test", "build"]:
            self.assertRegex(runtime, rf"cargo {action}[^\n]*-p cimmeria-runtime-probe[^\n]*--target-dir target/runtime-probe")
        self.assertIn("--self-test", runtime)
        self.assertNotIn("--features lab-bridge", runtime)
        local = commands(w["jobs"]["local-lab-tools"])
        self.assertIn("--features lab-bridge", local)
        self.assertIn("--target-dir target/local-lab", local)
        self.assertNotIn("target/client-telemetry", local)

    def test_docker_cooks_portable_dependencies_then_builds_only_real_release_binaries(self):
        docker = (ROOT / "docker/Dockerfile").read_text()
        self.assertIn("cargo install cargo-chef --version 0.1.78 --locked", docker)
        deps = docker.split("FROM build-base AS dependencies", 1)[1].split("FROM dependencies AS builder", 1)[0]
        self.assertRegex(deps, r"cargo chef cook[^\n]*-p cimmeria-server")
        self.assertRegex(deps, r"cargo chef cook[^\n]*-p cimmeria-wireclient --bin login-probe")
        self.assertNotIn("--mount=type=cache", deps)
        self.assertIn("mv target /opt/cooked-target", deps)
        self.assertIn("mv /usr/local/cargo/registry /opt/cooked-registry", deps)
        real = docker.split("FROM dependencies AS builder", 1)[1].split("FROM scratch AS login-probe", 1)[0]
        self.assertLess(real.index("COPY crates ./crates"), real.index("cargo build"))
        self.assertIn('sha256sum recipe.json', real)
        self.assertIn('cp -a /opt/cooked-target/. target/', real)
        self.assertIn('cp -a /opt/cooked-registry/. /usr/local/cargo/registry/', real)
        self.assertRegex(real, r"cargo build --locked -p cimmeria-server --release[^\n]*\n.*cp target/release/cimmeria-server")
        self.assertRegex(real, r"cargo build --locked -p cimmeria-wireclient --bin login-probe --release")
        self.assertNotRegex(real, r"--all-features|-p cimmeria-server[^\n]*-p cimmeria-wireclient")
        self.assertRegex(docker, r"COPY --from=builder[^\n]*/tmp/cimmeria-server /opt/cimmeria/bin/cimmeria-server")

    def test_release_publication_safety_remains_in_order(self):
        for name in ["release-container.yml", "launcher-release.yml"]:
            self.assertIs(workflow(name)["concurrency"]["cancel-in-progress"], False)
        job = workflow("release-container.yml")["jobs"]["build-and-publish"]
        self.assertEqual(job["runs-on"], "ubuntu-latest")
        names = [s.get("name", "") for s in job["steps"]]
        scan = names.index("scan with Trivy")
        push = names.index("push dated tag (with provenance + SBOM, after scan)")
        smoke = names.index("smoke-test the published image (login probe)")
        promote = next(i for i,n in enumerate(names) if "promote latest-prerelease" in n)
        self.assertLess(scan, push)
        self.assertLess(push, smoke)
        self.assertLess(smoke, promote)
        self.assertTrue(job["steps"][push]["with"]["provenance"])
        self.assertTrue(job["steps"][push]["with"]["sbom"])


if __name__ == "__main__":
    unittest.main()
