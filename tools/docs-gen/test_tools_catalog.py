"""Unit tests for tools_catalog.py. Run: python -m unittest discover -s tools/docs-gen -p "test_*.py"."""
from __future__ import annotations

import pathlib
import sys
import tempfile
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import regen  # noqa: E402
import tools_catalog as tc  # noqa: E402

CFG = {
    "exclude": ["*/test_*.py", "*-lib.ps1"],
    "run": {".py": "python {path}", ".ps1": "pwsh {path}", ".sh": "bash {path}", ".java": "ghidra"},
    "run_override": [{"pattern": "*/__main__.py", "run": "python {dir}"}],
    "group": [
        {"title": "Decode packets", "patterns": ["tools/pcap_*", "crates/wire*"]},
        {"title": "Client assets", "patterns": ["crates/upk/*"]},
    ],
}

FILES = {
    "tools/pcap_dissect.py": '#!/usr/bin/env python3\n"""Dissect a Mercury pcap. More detail here."""\n',
    "tools/nosum.sh": "#!/bin/bash\nset -e\necho hi\n",
    "tools/test_thing.py": '"""A test, not a tool."""\n',
    "tools/pkg/helper.py": '"""A library module: no __main__ guard."""\ndef f(): pass\n',
    "tools/pkg/run_me.py": '"""Run me.\n\nUsage: run_me.py X\n"""\nif __name__ == "__main__":\n    pass\n',
    "tools/pkg/__init__.py": '"""The pkg package: does pkg things."""\n',
    "tools/pkg/__main__.py": '"""Run as `python tools/pkg`."""\n',
    "tools/a.ps1": "<#\r\n.SYNOPSIS\r\n    Do the A thing\r\n    across two lines.\r\n\r\n.DESCRIPTION\r\n    Long.\r\n#>\r\nparam()\r\n",
    "tools/b-lib.ps1": "# a dot-sourced library\n",
    "tools/old.py": '"""\nOld Extractor\n=============\n\nSUPERSEDED (X1). Use the Rust one | not this.\n"""\n',
    "crates/upk/Cargo.toml": '[package]\nname = "cimmeria-upk"\n\n[[bin]]\nname = "upk-info"\npath = "src/bin/upk_info.rs"\n',
    "crates/upk/src/lib.rs": "//! lib\n",
    "crates/upk/src/bin/upk_info.rs": "//! Dump a UE3 package.\n//!\n//! Usage: upk-info X\nfn main() {}\n",
    "crates/upk/src/bin/extract/main.rs": "#![allow(dead_code)]\n//! Extract things\n//! from maps.\nfn main() {}\n",
    "crates/upk/src/bin/extract/args.rs": "//! not a bin\n",
    "crates/wireclient/Cargo.toml": '[package]\nname = "cimmeria-wireclient"\n',
    "crates/wireclient/src/main.rs": "use std::io;\nfn main() {}\n",
    "crates/desk/Cargo.toml": '[workspace]\nmembers = ["shell"]\n',
    "crates/desk/shell/Cargo.toml": '[package]\nname = "desk-shell"\n',
    "crates/desk/shell/src/main.rs": "// Plain comment summary.\nfn main() {}\n",
}


class CatalogTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.root = pathlib.Path(self.tmp.name)
        for rel, text in FILES.items():
            path = self.root / rel
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(text.encode("utf-8"))
        self.tools = {t.path: t for t in tc.collect(self.root, CFG)}

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def test_discovers_entry_points_and_skips_tests_and_libraries(self) -> None:
        self.assertEqual(
            sorted(self.tools),
            [
                "crates/desk/shell/src/main.rs",
                "crates/upk/src/bin/extract/main.rs",
                "crates/upk/src/bin/upk_info.rs",
                "crates/wireclient/src/main.rs",
                "tools/a.ps1",
                "tools/nosum.sh",
                "tools/old.py",
                "tools/pcap_dissect.py",
                "tools/pkg/",
                "tools/pkg/run_me.py",
            ],
        )

    def test_summaries_by_file_kind(self) -> None:
        s = {p: t.summary for p, t in self.tools.items()}
        self.assertEqual(s["tools/pcap_dissect.py"], "Dissect a Mercury pcap. More detail here.")
        self.assertEqual(s["tools/a.ps1"], "Do the A thing across two lines.")
        self.assertEqual(s["tools/pkg/run_me.py"], "Run me.")
        self.assertEqual(s["tools/pkg/"], "The pkg package: does pkg things.")  # __init__ over __main__
        self.assertEqual(s["crates/upk/src/bin/upk_info.rs"], "Dump a UE3 package.")
        self.assertEqual(s["crates/upk/src/bin/extract/main.rs"], "Extract things from maps.")
        self.assertEqual(s["crates/desk/shell/src/main.rs"], "Plain comment summary.")
        # an underlined title is skipped, a short first sentence takes the next, `|` is escaped
        self.assertEqual(s["tools/old.py"], "SUPERSEDED (X1). Use the Rust one \\| not this.")
        self.assertEqual(s["tools/nosum.sh"], "")

    def test_run_commands(self) -> None:
        r = {p: t.run for p, t in self.tools.items()}
        self.assertEqual(r["crates/upk/src/bin/upk_info.rs"], "cargo run -p cimmeria-upk --bin upk-info --")
        self.assertEqual(r["crates/upk/src/bin/extract/main.rs"], "cargo run -p cimmeria-upk --bin extract --")
        self.assertEqual(r["crates/wireclient/src/main.rs"], "cargo run -p cimmeria-wireclient --bin cimmeria-wireclient --")
        self.assertEqual(
            r["crates/desk/shell/src/main.rs"],
            "cargo run --manifest-path crates/desk/Cargo.toml -p desk-shell --bin desk-shell --",
        )
        self.assertEqual(r["tools/pkg/"], "python tools/pkg")

    def test_render_groups_flags_gaps_and_is_stable(self) -> None:
        out = tc.render(self.root, CFG)
        self.assertEqual(out, tc.render(self.root, CFG))
        self.assertTrue(out.startswith("6 scripts and 4 crate binaries; 2 without a summary line"))
        self.assertIn("| Decode packets | 2 |", out)
        self.assertIn("| Client assets | 2 |", out)
        self.assertIn("| Other | 6 |", out)
        self.assertIn("| `tools/nosum.sh` | `bash tools/nosum.sh` | (no summary) |", out)
        self.assertLess(out.index("### Decode packets"), out.index("### Client assets"))
        self.assertLess(out.index("### Client assets"), out.index("### Other"))

    def test_marker_in_crlf_readme(self) -> None:
        readme = self.root / "tools" / "README.md"
        readme.write_bytes(b"# Tools\r\n\r\n<!-- gen:tools-catalog -->\r\n<!-- /gen:tools-catalog -->\r\n")
        old = tc.load_config
        tc.load_config = lambda path=tc.CONFIG: CFG
        try:
            stale = regen.regen_files(self.root, regen.Context(self.root), write=True)
            self.assertEqual([rel for rel, _ in stale], ["tools/README.md"])
            text = readme.read_bytes().decode("utf-8")
            self.assertNotIn("\n", text.replace("\r\n", ""), "a bare LF slipped into a CRLF file")
            self.assertIn("| `tools/pcap_dissect.py` |", text)
            self.assertEqual(regen.regen_files(self.root, regen.Context(self.root), write=False), [])
        finally:
            tc.load_config = old


class EscapeTests(unittest.TestCase):
    def test_placeholders_survive_outside_code_and_stay_literal_inside(self) -> None:
        self.assertEqual(
            tc.summarize(["Run lab uat <section> [-Rows <n>|all] or `lab uat <section>`."]),
            "Run lab uat &lt;section&gt; [-Rows &lt;n&gt;\\|all] or `lab uat <section>`.",
        )

    def test_trailing_colon_becomes_a_full_stop(self) -> None:
        self.assertEqual(tc.summarize(["Extracts actors from a tile and outputs:"]),
                         "Extracts actors from a tile and outputs.")


class RepoConfigTests(unittest.TestCase):
    def test_repo_excludes_tests_but_keeps_the_live_db_runner(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            files = ["tools/test-live-db.ps1", "tools/lab/test-labd-lib.ps1", "tools/lab/cli/test-uat.ps1"]
            for rel in files:
                (root / rel).parent.mkdir(parents=True, exist_ok=True)
                (root / rel).write_bytes(b"# Run the live-DB tier.\n")
            paths = [t.path for t in tc.script_tools(root, files, tc.load_config())]
        self.assertEqual(paths, ["tools/test-live-db.ps1"])

    def test_repo_config_parses_and_every_group_has_patterns(self) -> None:
        cfg = tc.load_config()
        self.assertIn(".py", cfg["run"])
        self.assertTrue(cfg["group"])
        for g in cfg["group"]:
            self.assertTrue(g["title"] and g["patterns"], g)


if __name__ == "__main__":
    unittest.main()
