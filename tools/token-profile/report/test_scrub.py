"""The privacy scrubber against hostile synthetic values, and against false positives."""

import unittest

from .scrub import INVALID, PrivacyError, Scrubber

# (input, a substring that must not survive scrubbing)
HOSTILE = [
    ("cd C:\\Users\\alice\\secret-project && make", "alice"),
    ("C:\\\\Users\\\\alice\\\\x.json", "alice"),
    ("see C:/Users/alice/notes.txt", "alice"),
    ("\\\\fileserver\\share\\alice\\doc.txt", "fileserver"),
    ("/home/alice/.ssh/id_ed25519", "alice"),
    ("/Users/alice/Library/x", "alice"),
    ("/c/Users/alice/source", "alice"),
    ("~/secret/notes.md", "secret"),
    ("postgres://user:password@10.0.0.5/db", "password"),
    ("postgres://user:password@10.0.0.5/db", "10.0.0.5"),
    ("https://user:pass@example.internal/", "pass@"),
    ("https://example.internal/path", "example.internal"),
    ("ssh admin@db1.corp.example.com", "db1.corp"),
    ("https://github.com/x/y?token=abcdef", "abcdef"),
    ("Authorization: Bearer abc123def456ghi789", "abc123def456ghi789"),
    ("authorization=Basic dXNlcjpwYXNz", "dXNlcjpwYXNz"),
    ("curl -H 'Bearer eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.c2lnbmF0dXJl'", "eyJhbGciOiJIUzI1NiJ9"),
    ("--token supersecret", "supersecret"),
    ("--api-key=sk-ant-api03-AAAABBBBCCCCDDDD", "AAAABBBBCCCCDDDD"),
    ("--password 'hunter two'", "hunter two"),
    ("DATABASE_URL=postgres://u:p@h/db cargo test", "u:p@h"),
    ("GH_TOKEN=gho_abcdefghij1234567890 gh pr list", "gho_abcdefghij1234567890"),
    ("export AWS_KEY=AKIAABCDEFGHIJKLMNOP", "AKIAABCDEFGHIJKLMNOP"),
    ("key AKIAABCDEFGHIJKLMNOP here", "AKIAABCDEFGHIJKLMNOP"),
    ("token ghp_abcdefghijklmnopqrstuvwxyz0123", "ghp_abcdefghijklmnopqrstuvwxyz0123"),
    ("slack xoxb-1234567890-abcdefghij", "xoxb-1234567890"),
    ("api_key: 'zzz-private'", "zzz-private"),
    ("client_secret=s3cr3t", "s3cr3t"),
    ("echo abc123def456ghi789", "abc123def456ghi789"),
    ("mail alice@example.com now", "alice@example.com"),
    ("host 192.168.1.20 up", "192.168.1.20"),
    ("v6 fe80::1ff:fe23:4567:890a and 2001:db8:0:0:0:0:2:1", "fe80::1ff"),
    ("v6 2001:db8:0:0:0:0:2:1", "2001:db8"),
    ("-----BEGIN RSA PRIVATE KEY-----\nMIIEow\n-----END RSA PRIVATE KEY-----", "MIIEow"),
]

# Values a report legitimately shows, which must pass through unchanged.
SAFE = [
    "claude-opus-5-5",
    "claude-haiku-4-5-20251001",
    "claude-opus-5[1m]",
    "2026-10-01T12:00:05.000Z",
    "2.1.999",
    "0123456789ab",
    "cargo nextest",
    "bash tools/build-lane/lane.sh",
    "docs/gap-analysis.md",
    "mcp__ghidra__decompile_function",
    "cache_write_5m",
    "rust-gameserver-dev",
    "main (coordinator)",
    "https://github.com/SandboxServers/Cimmeria/pull/4242",
    "| p50 | p75 | p90 |",
    "token=12345",
    "Context is input plus cache read plus both cache writes.",
]


class ScrubTest(unittest.TestCase):
    def setUp(self):
        self.sc = Scrubber(deny=["alice"], use_local=False)

    def test_hostile_values_are_redacted(self):
        for value, secret in HOSTILE:
            with self.subTest(value=value):
                out = self.sc.text(value)
                self.assertNotIn(secret, out)
                self.assertFalse(self.sc.findings(out), out)

    def test_hostile_values_are_detected_by_the_gate(self):
        for value, _ in HOSTILE:
            with self.subTest(value=value):
                self.assertTrue(self.sc.findings(value))

    def test_safe_values_pass_unchanged(self):
        for value in SAFE:
            with self.subTest(value=value):
                self.assertEqual(self.sc.text(value), value)
                self.assertFalse(self.sc.findings(value))

    def test_deny_words_match_whole_words_case_insensitively(self):
        sc = Scrubber(deny=["Steve"], use_local=False)
        self.assertEqual(sc.text("C--Users-steve-src and STEVE"), "C--Users-<redacted>-src and <redacted>")
        self.assertEqual(sc.text("Stevenson"), "Stevenson")

    def test_short_deny_words_are_ignored(self):
        self.assertEqual(Scrubber(deny=["ab"], use_local=False).text("ab cab"), "ab cab")

    def test_gate_names_the_detector_not_the_value(self):
        with self.assertRaises(PrivacyError) as cm:
            self.sc.assert_clean("leak: postgres://user:hunter2@10.0.0.5/db")
        msg = str(cm.exception)
        self.assertIn("url", msg)
        self.assertNotIn("hunter2", msg)
        self.assertNotIn("10.0.0.5", msg)

    def test_obj_scrubs_keys_and_nested_values(self):
        out = self.sc.obj({"C:\\Users\\alice": ["10.0.0.5", {"x": "--token t0psecret"}], "n": 3})
        self.assertEqual(out, {"<path>": ["<ip>", {"x": "--token <redacted>"}], "n": 3})


class FieldValidationTest(unittest.TestCase):
    def setUp(self):
        self.sc = Scrubber(use_local=False)

    def test_command_fingerprints(self):
        for ok in ("cargo nextest", "gh pr", "python", "sed -n", "bash tools/build-lane/lane.sh"):
            self.assertEqual(self.sc.fingerprint("Bash", ok), ok)
        for bad in ("cargo nextest run --token supersecret", "curl https://user:pass@host/",
                    "C:\\Python\\python.exe", "/usr/bin/python", "echo abc123def456ghi789",
                    "DATABASE_URL=x cargo", "a b c"):
            self.assertEqual(self.sc.fingerprint("Bash", bad), INVALID, bad)

    def test_path_fingerprints(self):
        self.assertEqual(self.sc.fingerprint("Read", "docs/gap-analysis.md"), "docs/gap-analysis.md")
        self.assertEqual(self.sc.fingerprint("Read", "docs\\gap-analysis.md"), "docs/gap-analysis.md")
        self.assertEqual(self.sc.fingerprint("Edit", "<external>"), "<external>")
        for bad in ("C:\\Users\\alice\\x.md", "/home/alice/x", "~/x", "../outside/x", "docs/../../x"):
            self.assertEqual(self.sc.fingerprint("Read", bad), INVALID, bad)

    def test_other_fingerprints(self):
        self.assertEqual(self.sc.fingerprint("mcp__ghidra__list_functions", "mcp__ghidra__list_functions"),
                         "mcp__ghidra__list_functions")
        self.assertEqual(self.sc.fingerprint("mcp__ghidra__list_functions", "something else"), INVALID)
        self.assertEqual(self.sc.fingerprint("Glob", "*.md"), INVALID)  # the contract stores NULL
        self.assertIsNone(self.sc.fingerprint("Glob", None))

    def test_rejections_are_counted(self):
        self.sc.label("postgres://u:p@10.0.0.5/db", "cc_version")
        self.sc.fingerprint("Bash", "a b c")
        self.assertEqual(dict(self.sc.rejected), {"cc_version": 1, "fingerprint": 1})


if __name__ == "__main__":
    unittest.main()
