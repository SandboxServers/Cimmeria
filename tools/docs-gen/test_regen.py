"""Unit tests for regen.py. Run: python -m unittest discover -s tools/docs-gen -p "test_*.py"."""
from __future__ import annotations

import contextlib
import io
import pathlib
import sys
import tempfile
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import regen  # noqa: E402

MATRIX = """\
## Summary Completion Matrix

| # | System | Total | CW | NT | IM | KM | NU |
|---|--------|-------|----|----|----|----|-----|
| 1 | Auth | 4 | 2 | 1 | 1 | 0 | 0 |
| 2 | Mail | 6 | 0 | 2 | 1 | 2 | 1 |
| -- | Scheduler | 10 | 5 | 0 | 0 | 5 | 0 |
| | **TOTALS** | **<!-- gen:gap-count total -->999<!-- /gen:gap-count -->** | **<!-- gen:gap-count CW -->1<!-- /gen:gap-count -->** | **0** | **0** | **0** | **0** |

### Summary Percentages

**Code exists (CW + NT + IM)**: <!-- gen:gap-count CW+NT+IM -->0<!-- /gen:gap-count --> features (<!-- gen:gap-pct CW+NT+IM -->0%<!-- /gen:gap-pct -->)
"""


def ctx_with(root: pathlib.Path, total: int = 2000) -> regen.Context:
    ctx = regen.Context(root)
    ctx.tests = regen.TestStats(
        total=total,
        files=100,
        ci_gated=total - 10,
        live_db=42,
        crates=[("crates/a", "cimmeria-a", total, 100, 42, True, "a.md")],
    )
    return ctx


class MarkerTests(unittest.TestCase):
    def setUp(self) -> None:
        self.ctx = ctx_with(pathlib.Path("."))

    def test_inline_marker_is_replaced_and_rest_untouched(self) -> None:
        text = "before <!-- gen:tests-total -->1<!-- /gen:tests-total --> after\n"
        new, changes = regen.regen_text(text, self.ctx)
        self.assertEqual(new, "before <!-- gen:tests-total -->2,000<!-- /gen:tests-total --> after\n")
        self.assertEqual([c.name for c in changes], ["tests-total"])

    def test_regeneration_is_idempotent(self) -> None:
        text = (
            "x <!-- gen:tests-threshold -->0<!-- /gen:tests-threshold -->\n"
            "<!-- gen:tests-totals -->\n<!-- /gen:tests-totals -->\n"
        )
        once, first = regen.regen_text(text, self.ctx)
        twice, second = regen.regen_text(once, self.ctx)
        self.assertTrue(first)
        self.assertEqual(once, twice)
        self.assertEqual(second, [])
        self.assertIn("-->100<!--", once)  # 5% of 2,000

    def test_crlf_is_preserved_inside_and_outside_blocks(self) -> None:
        text = "line one\r\n<!-- gen:tests-totals -->\r\n<!-- /gen:tests-totals -->\r\nlast\r\n"
        new, _ = regen.regen_text(text, self.ctx)
        self.assertTrue(new.startswith("line one\r\n"))
        self.assertTrue(new.endswith("-->\r\nlast\r\n"))
        self.assertNotIn("\n", new.replace("\r\n", ""), "a bare LF slipped into a CRLF file")
        self.assertIn("| Metric | Count |\r\n", new)

    def test_lf_file_stays_lf(self) -> None:
        text = "a\n<!-- gen:tests-by-crate -->\n<!-- /gen:tests-by-crate -->\n"
        new, _ = regen.regen_text(text, self.ctx)
        self.assertNotIn("\r", new)

    def test_examples_in_code_are_left_alone(self) -> None:
        text = (
            "Write `<!-- gen:tests-total -->1<!-- /gen:tests-total -->` like this.\n"
            "```markdown\n<!-- gen:tests-total -->1<!-- /gen:tests-total -->\n```\n"
            "Live: `code` <!-- gen:tests-total -->1<!-- /gen:tests-total -->\n"
        )
        new, changes = regen.regen_text(text, self.ctx)
        self.assertEqual(len(changes), 1)
        self.assertEqual(new.count("-->1<!--"), 2)
        self.assertTrue(new.endswith("Live: `code` <!-- gen:tests-total -->2,000<!-- /gen:tests-total -->\n"))

    def test_unknown_generator_is_an_error(self) -> None:
        with self.assertRaises(ValueError):
            regen.regen_text("<!-- gen:nope -->1<!-- /gen:nope -->", self.ctx)

    def test_section_table_rows_counts_only_its_own_section(self) -> None:
        text = (
            "### A\n\n<!-- gen:section-table-rows -->0<!-- /gen:section-table-rows --> docs.\n\n"
            "| Doc | Desc |\n|---|---|\n| [a](a.md) | x |\n| [b](b.md) | y |\n\n"
            "### B\n\n| Doc |\n|---|\n| [c](c.md) |\n"
        )
        new, _ = regen.regen_text(text, self.ctx)
        self.assertIn("-->2<!--", new)


class GapMatrixTests(unittest.TestCase):
    def test_totals_are_computed_from_rows_not_from_the_totals_line(self) -> None:
        ctx = regen.Context(pathlib.Path("."))
        ctx.gap = regen.parse_gap_matrix(MATRIX)
        self.assertEqual(len(ctx.gap.rows), 3)  # TOTALS row skipped
        new, _ = regen.regen_text(MATRIX, ctx)
        self.assertIn("**<!-- gen:gap-count total -->20<!-- /gen:gap-count -->**", new)
        self.assertIn("**<!-- gen:gap-count CW -->7<!-- /gen:gap-count -->**", new)
        # CW + NT + IM = 7 + 3 + 2 = 12 of 20
        self.assertIn("<!-- gen:gap-count CW+NT+IM -->12<!-- /gen:gap-count -->", new)
        self.assertIn("<!-- gen:gap-pct CW+NT+IM -->60.0%<!-- /gen:gap-pct -->", new)

    def test_crlf_matrix_parses(self) -> None:
        self.assertEqual(regen.parse_gap_matrix(MATRIX.replace("\n", "\r\n")).count("total"), 20)

    def test_pct_decimals_argument(self) -> None:
        ctx = regen.Context(pathlib.Path("."))
        ctx.gap = regen.parse_gap_matrix(MATRIX)
        new, _ = regen.regen_text("<!-- gen:gap-pct CW 0 -->x<!-- /gen:gap-pct -->", ctx)
        self.assertEqual(new, "<!-- gen:gap-pct CW 0 -->35%<!-- /gen:gap-pct -->")

    def test_unknown_status_is_an_error(self) -> None:
        gap = regen.parse_gap_matrix(MATRIX)
        with self.assertRaises(ValueError):
            gap.count("CW+XX")


class FileTests(unittest.TestCase):
    def test_check_reports_stale_and_writes_nothing_then_write_fixes_it(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            findings = root / "docs" / "reverse-engineering" / "findings"
            findings.mkdir(parents=True)
            for name in ("a.md", "b.md", "c.md", "README.md"):
                (findings / name).write_bytes(b"x\r\n")
            doc = findings / "README.md"
            stale = b"Contains <!-- gen:re-findings-count -->2<!-- /gen:re-findings-count --> docs.\r\n"
            doc.write_bytes(stale)
            ctx = regen.Context(root)

            found = regen.regen_files(root, ctx, write=False)
            self.assertEqual([(rel, c.old, c.new) for rel, c in found],
                             [("docs/reverse-engineering/findings/README.md", "2", "3")])
            self.assertEqual(doc.read_bytes(), stale, "--check must not write")

            regen.regen_files(root, ctx, write=True)
            self.assertEqual(
                doc.read_bytes(),
                b"Contains <!-- gen:re-findings-count -->3<!-- /gen:re-findings-count --> docs.\r\n",
            )
            self.assertEqual(regen.regen_files(root, ctx, write=False), [])

    def test_main_check_exits_1_when_stale(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            (root / "docs").mkdir()
            (root / "docs" / "x.md").write_bytes(b"<!-- gen:docs-md-count -->0<!-- /gen:docs-md-count -->\n")
            old_root = regen.ROOT
            regen.ROOT = root
            try:
                with contextlib.redirect_stdout(io.StringIO()) as out:
                    code = regen.main(["--check", "--skip-crate-graph"])
                self.assertEqual(code, 1)
                self.assertIn("stale: docs/x.md", out.getvalue())
                with contextlib.redirect_stdout(io.StringIO()):
                    self.assertEqual(regen.main(["--skip-crate-graph"]), 0)
                    self.assertEqual(regen.main(["--check", "--skip-crate-graph"]), 0)
            finally:
                regen.ROOT = old_root


if __name__ == "__main__":
    unittest.main()
