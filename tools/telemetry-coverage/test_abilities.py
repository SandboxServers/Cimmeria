"""Tests for the AB-C7 coverage matrix script (stock unittest)."""

import sys
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent))

import abilities as cov  # noqa: E402


class TableParse(unittest.TestCase):
    def test_reads_plain_backticked_and_call_rows_with_their_section(self):
        text = "\n".join([
            "## Own",
            "| Index | Method | Args |",
            "|---|---|---|",
            "| 68 | useAbility | YES | INT32 abilityId |",
            "| 14 | `onEffectResults` | `INT32 SourceID` |",
            "### Debug (169-184)",
            "| 169 | `gmDebugAbility(INT32 abilityId)` | - | - | NEW |",
            "| 22 | 0xD6 | logOff | INT8 Disconnect | 450 |",
            "| - | invokeAbility | no | |",
        ])
        rows = cov.parse_table_text(text, "t.md")
        got = [(r.section, r.index, r.name) for r in rows]
        self.assertEqual(got, [
            ("Own", 68, "useAbility"),
            ("Own", 14, "onEffectResults"),
            ("Debug (169-184)", 169, "gmDebugAbility"),
            ("Debug (169-184)", 22, "logOff"),
        ])


class ConstScan(unittest.TestCase):
    def test_struct_literals_calls_tuples_and_const_indices(self):
        text = """
pub(crate) const ON_SEQUENCE: u16 = 1;
pub(crate) const T: &[X] = &[
    X {
        index: ON_SEQUENCE,
        name: "onSequence",
        args: &[arg("sequence_id", "KismetEventSetSeqID", I32)],
    },
    X {
        name: "useAbility",
        cell_index: 68,
        args: &[arg("AbilityID", Int, Slot::AbilityId)],
    },
    generic(4, "confirmationResponse"),
    (77, "trainAbility"),
    AbilityReceipt {
        index: 69,
        method: "useAbilityOnGroundTarget",
        event: "use_ability_on_ground_recv",
    },
];
const OTHER: &[X] = &[(5, "notMe")];
"""
        self.assertEqual(cov.scan_const_text(text, "T"), {
            "onSequence": 1,
            "useAbility": 68,
            "confirmationResponse": 4,
            "trainAbility": 77,
            "useAbilityOnGroundTarget": 69,
        })
        self.assertIsNone(cov.scan_const_text(text, "ABSENT"))


class CommentedOutEntries(unittest.TestCase):
    """A commented-out entry is not coverage (Copilot on #1185), for every
    entry shape the scanner reads, in line and block comments, while string
    and char literals that merely contain comment markers are kept."""

    TABLE = """
const T: &[X] = &[
    generic(1, "liveCall"),
    // generic(2, "lineCall"),
    /* generic(3, "blockCall"), */
    (10, "liveTuple"),
    // (11, "lineTuple"),
    /* (12, "blockTuple"), */
    X {
        index: 20,
        name: "liveStruct",
    },
    // X {
    //     index: 21,
    //     name: "lineStruct",
    // },
    /* X {
        index: 22,
        name: "blockStruct",
    }, */
    /* outer /* nested generic(30, "nestedInner"), */ generic(31, "nestedOuter"), */
    X {
        index: 40,
        note: "a // not a comment /* nor this",
        raw: r#"he said "// x" and /* y"#,
        bytes: br"/* z",
        escaped: "\\"// still a string\\"",
        quote: '"',
        slash: '/',
        name: "afterLiterals",
    },
    generic(41, "afterLiteralsCall"), // trailing comment generic(42, "trailing"),
];
"""

    def test_only_live_entries_are_scanned(self):
        self.assertEqual(cov.scan_const_text(self.TABLE, "T"), {
            "liveCall": 1,
            "liveTuple": 10,
            "liveStruct": 20,
            "afterLiterals": 40,
            "afterLiteralsCall": 41,
        })

    def test_literals_survive_and_newlines_are_kept(self):
        src = 'a("// x", r#"/* y"#, \'"\') // gone\n/* b\nc */d'
        self.assertEqual(
            cov.strip_rust_comments(src),
            'a("// x", r#"/* y"#, \'"\') \n\nd',
        )

    def test_a_commented_out_const_is_absent(self):
        self.assertIsNone(cov.scan_const_text("/*\nconst T: &[X] = &[\n    (1, \"a\"),\n];\n*/", "T"))

    def test_a_commented_out_receipt_empties_its_cell(self):
        path, const = cov.SOURCES[cov.SERVER_RECV]
        text = (cov.ROOT / path).read_text(encoding="utf-8")
        live = '    generic(77, "trainAbility"),'
        self.assertIn(live, text, "fixture: the real receipt table changed shape")
        commented = text.replace(live, '    // generic(77, "trainAbility"),')
        real_scan = cov.scan_const

        def scan(p, c):
            if (p, c) == (path, const):
                return cov.scan_const_text(commented, c, p)
            return real_scan(p, c)

        with mock.patch.object(cov, "scan_const", scan):
            res = cov.build()
        cells = {m.name: cells for m, _, cells in res.rows}
        self.assertEqual(cells["trainAbility"][cov.SERVER_RECV].state, "missing")
        self.assertTrue(
            any("`trainAbility` has no server recv row" in p for p in res.problems),
            res.problems,
        )


class Gate(unittest.TestCase):
    """The rules of the gate, against the real dispatch tables with the code
    tables replaced."""

    def run_with(self, scans, exceptions):
        def fake_scan(path, const):
            return scans.get(const)
        with mock.patch.object(cov, "scan_const", fake_scan), \
                mock.patch.object(cov, "EXCEPTIONS", exceptions):
            return cov.build()

    def full(self):
        c2s = {m.name: m.index for m in cov.ABILITY_METHODS if m.direction == cov.C2S}
        s2c = {m.name: m.index for m in cov.ABILITY_METHODS if m.direction == cov.S2C}
        return {
            "ABILITY_RECEIPTS": dict(c2s),
            "LEDGER_METHODS": dict(s2c),
            "ALLOWLIST": dict(c2s),
            "METHODS": dict(s2c),
            "CLIENT_SENDS": dict(c2s),
            "CLIENT_RECVS": dict(s2c),
        }

    def test_everything_filled_is_clean(self):
        self.assertEqual(self.run_with(self.full(), {}).problems, [])

    def test_an_empty_travelling_cell_without_an_exception_fails(self):
        scans = self.full()
        del scans["METHODS"]["onEffectResults"]
        del scans["CLIENT_RECVS"]["onEffectResults"]
        res = self.run_with(scans, {})
        self.assertTrue(
            any("`onEffectResults` has no client recv hook" in p for p in res.problems),
            res.problems,
        )

    def test_an_exception_explains_the_cell_and_goes_stale_when_filled(self):
        scans = self.full()
        del scans["ALLOWLIST"]["trainAbility"]
        del scans["CLIENT_SENDS"]["trainAbility"]
        exc = {("trainAbility", cov.CLIENT_SEND): "because"}
        self.assertEqual(self.run_with(scans, exc).problems, [])
        stale = self.run_with(self.full(), exc).problems
        self.assertTrue(any("stale exception" in p for p in stale), stale)

    def test_an_exception_on_a_column_the_method_does_not_travel_fails(self):
        exc = {("useAbility", cov.CLIENT_RECV): "nonsense"}
        res = self.run_with(self.full(), exc)
        self.assertTrue(any("does not travel" in p for p in res.problems), res.problems)

    def test_a_wrong_index_in_a_code_table_fails(self):
        scans = self.full()
        scans["LEDGER_METHODS"]["onTimerUpdate"] = 13
        res = self.run_with(scans, {})
        self.assertTrue(any("`onTimerUpdate` at index 13" in p for p in res.problems), res.problems)

    def test_the_client_declaration_must_equal_the_set(self):
        scans = self.full()
        del scans["CLIENT_SENDS"]["useAbility"]
        res = self.run_with(scans, {})
        self.assertTrue(any("CLIENT_SENDS differs" in p for p in res.problems), res.problems)


if __name__ == "__main__":
    unittest.main()
