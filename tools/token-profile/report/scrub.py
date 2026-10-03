"""Privacy scrubber: nothing private reaches a report that might be committed.

Three layers, each of which would be enough on a correct ingest:

1. Field validation. Every free-text value a report shows (model ids, agent
   types, tool names, fingerprints, versions) must have the shape the
   contract promises. A value that doesn't is replaced by `<invalid>` and
   counted, so an ingest bug shows up in the report instead of leaking.
2. Redaction. Every string is passed through the patterns below: URL
   credentials, auth headers, secret flags and assignments, known token
   formats, emails, IP addresses, absolute local paths, private hostnames,
   high-entropy tokens, and the deny words (the local username, the machine
   name, and anything passed with --deny or TOKEN_PROFILE_DENY).
3. The gate. The rendered Markdown and JSON are searched again with the same
   detectors before anything is written. A hit raises PrivacyError, which
   names the detector and never the value, and nothing is written.
"""

import getpass
import os
import re
from collections import Counter
from pathlib import Path

REDACTED = "<redacted>"
INVALID = "<invalid>"

# Hosts a report may name in a URL: public documentation only.
PUBLIC_HOSTS = (
    "github.com",
    "docs.anthropic.com",
    "platform.claude.com",
    "code.claude.com",
    "www.anthropic.com",
    "anthropic.com",
    "claude.com",
)

_QUOTED_OR_WORD = r"(?:\"[^\"]*\"|'[^']*'|[^\s\"'|]+)"
_SECRET_WORD = r"(?:password|passwd|pwd|secret|token|api[_-]?key|apikey|access[_-]?key|private[_-]?key|credential|auth)"


def _url(m):
    scheme, userinfo, host, rest, query = m.group(1), m.group(2), m.group(3), m.group(4), m.group(5)
    if userinfo or host.lower().split(":")[0] not in PUBLIC_HOSTS:
        return "<url>"
    return scheme + host + rest  # the query string can carry a token, so it never survives


# (name, pattern, replacement). Order matters: the specific detectors run
# before the generic ones so a credential is reported under its own name.
_DETECTORS = [
    ("pem-private-key", re.compile(r"-----BEGIN [A-Z ]*PRIVATE KEY-----.*?(?:-----END [A-Z ]*PRIVATE KEY-----|$)", re.S),
     REDACTED),
    ("auth-header", re.compile(r"(?i)\b((?:proxy-)?authorization)\s*[:=]\s*(?:(?:bearer|basic|token|digest)\s+)?"
                               + r"(?!<redacted>)" + _QUOTED_OR_WORD), r"\1: " + REDACTED),
    ("bearer-token", re.compile(r"(?i)\b(bearer|basic)\s+[A-Za-z0-9._~+/=-]{6,}"), r"\1 " + REDACTED),
    ("secret-flag", re.compile(r"(?i)(?<![\w-])(--?[\w-]*" + _SECRET_WORD + r"[\w-]*|--pass|-p)(?:[ =])"
                               + r"(?!<redacted>)" + _QUOTED_OR_WORD), r"\1 " + REDACTED),
    ("env-assignment", re.compile(r"\b([A-Z][A-Z0-9_]{2,})=(?!<redacted>)" + _QUOTED_OR_WORD), r"\1=" + REDACTED),
    ("secret-assignment", re.compile(r"(?i)\b([\w.-]*" + _SECRET_WORD + r"[\w.-]*)\s*[=:]\s*(?!<redacted>)"
                                     r"(?![\d,.]+(?![^\s\"'|]))" + _QUOTED_OR_WORD), r"\1=" + REDACTED),
    ("url", re.compile(r"(?i)\b([a-z][a-z0-9+.-]*://)([^/\s@\"'<>|]*@)?([^\s/?#\"'<>|]*)([^\s?#\"'<>|]*)"
                       r"(\?[^\s#\"'<>|]*)?(?:#[^\s\"'<>|]*)?"), _url),
    ("github-token", re.compile(r"\b(?:ghp|gho|ghu|ghs|ghr|github_pat)_[A-Za-z0-9_]{10,}"), REDACTED),
    ("api-key", re.compile(r"\bsk-(?:ant-)?[A-Za-z0-9_-]{10,}"), REDACTED),
    ("aws-key", re.compile(r"\b(?:AKIA|ASIA)[0-9A-Z]{16}\b"), REDACTED),
    ("slack-token", re.compile(r"\bxox[abprs]-[A-Za-z0-9-]{10,}"), REDACTED),
    ("google-key", re.compile(r"\bAIza[0-9A-Za-z_-]{20,}"), REDACTED),
    ("jwt", re.compile(r"\beyJ[A-Za-z0-9_-]{5,}\.[A-Za-z0-9_-]{5,}\.[A-Za-z0-9_-]{5,}"), REDACTED),
    ("email", re.compile(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b"), "<email>"),
    ("ipv4", re.compile(r"(?<![\d.])(?:(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)\.){3}(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)"
                        r"(?![\d.]*\d)"), "<ip>"),
    ("ipv6", re.compile(r"(?i)(?<![\w:])(?:(?:[0-9a-f]{1,4}:){7}[0-9a-f]{1,4}"
                        r"|(?:[0-9a-f]{1,4}:){1,6}:(?:[0-9a-f]{1,4}(?::[0-9a-f]{1,4})*)?|::1)(?![\w:])"), "<ip>"),
    ("windows-path", re.compile(r"(?i)(?<![A-Za-z0-9])[A-Z]:(?:\\\\|\\|/)[^\s\"'<>|`]*"), "<path>"),
    ("unc-path", re.compile(r"\\\\\\?[A-Za-z0-9._$-]+\\[^\s\"'<>|`]*"), "<path>"),
    ("posix-path", re.compile(r"(?<![\w.~/-])/(?:home|Users|root|mnt|media|tmp|var|opt|etc|private|srv|run|"
                              r"Volumes|[a-z])/[^\s\"'<>|`]*"), "<path>"),
    ("home-path", re.compile(r"(?<![\w])~[\\/][^\s\"'<>|`]*"), "<path>"),
    ("private-host", re.compile(r"(?i)\b[a-z0-9-]+(?:\.[a-z0-9-]+)*\.(?:internal|local|lan|corp|home|intranet)\b"),
     "<host>"),
    ("high-entropy-token", re.compile(r"(?<![\w+/=.-])(?=(?:[A-Za-z_+=]*\d){4})(?=(?:[\d_+=]*[A-Za-z]){4})"
                                      r"[A-Za-z0-9_+=]{16,}(?![\w+/=.-])"), REDACTED),
]


def local_deny_words():
    """The names that identify this machine and its user, which no report may show."""
    words = set()
    for var in ("USERNAME", "USER", "LOGNAME", "COMPUTERNAME", "HOSTNAME", "USERDOMAIN"):
        if os.environ.get(var):
            words.add(os.environ[var])
    try:
        words.add(getpass.getuser())
    except Exception:  # noqa: BLE001 - no user database is not an error here
        pass
    try:
        words.add(Path.home().name)
    except RuntimeError:
        pass
    for extra in os.environ.get("TOKEN_PROFILE_DENY", "").split(","):
        words.add(extra.strip())
    return words


class PrivacyError(Exception):
    """The gate found something a report must not carry. Names detectors, never values."""


class Scrubber:
    def __init__(self, deny=(), use_local=True):
        words = set(deny) | (local_deny_words() if use_local else set())
        # Short words would redact half the report; the path detectors cover them inside paths.
        self.deny = sorted({w for w in words if w and len(w) >= 3}, key=len, reverse=True)
        self._deny_re = (re.compile(r"(?i)(?<![A-Za-z0-9])(?:" + "|".join(re.escape(w) for w in self.deny)
                                    + r")(?![A-Za-z0-9])") if self.deny else None)
        self.rejected = Counter()

    # -- layer 2: redaction ------------------------------------------------
    def text(self, value):
        if value is None:
            return None
        s = str(value)
        for _, pattern, repl in _DETECTORS:
            s = pattern.sub(repl, s)
        if self._deny_re:
            s = self._deny_re.sub(REDACTED, s)
        return s

    def obj(self, value):
        """Scrub every string, keys included, in a JSON-like structure."""
        if isinstance(value, str):
            return self.text(value)
        if isinstance(value, dict):
            return {self.text(k) if isinstance(k, str) else k: self.obj(v) for k, v in value.items()}
        if isinstance(value, (list, tuple)):
            return [self.obj(v) for v in value]
        return value

    # -- layer 3: the gate -------------------------------------------------
    def findings(self, text):
        hits = Counter()
        for name, pattern, _ in _DETECTORS:
            for m in pattern.finditer(text):
                if name == "url" and _url(m) == m.group(0):
                    continue  # a public URL with no credentials or query is allowed through
                hits[name] += 1
        if self._deny_re:
            n = len(self._deny_re.findall(text))
            if n:
                hits["deny-word"] += n
        return hits

    def assert_clean(self, text, where="report"):
        hits = self.findings(text)
        if hits:
            detail = ", ".join(f"{name} x{n}" for name, n in sorted(hits.items()))
            raise PrivacyError(f"{where} failed the privacy gate ({detail}); nothing was written")

    # -- layer 1: field validation -----------------------------------------
    _LABEL = re.compile(r"^[A-Za-z0-9<][A-Za-z0-9 ._:+()\[\]<>/-]{0,79}$")
    _WORD = re.compile(r"^[A-Za-z0-9_.+-][A-Za-z0-9_.+/-]{0,63}$")
    _REL_PATH = re.compile(r"^(?![/~])(?![A-Za-z]:)(?!.*(?:^|/)\.\.(?:/|$))[A-Za-z0-9_.+@ -][A-Za-z0-9_.+@ /-]{0,199}$")

    def _reject(self, kind):
        self.rejected[kind] += 1
        return INVALID

    def label(self, value, kind="label"):
        """A model id, agent type, tool name, version or enum value."""
        if value is None:
            return None
        s = str(value)
        if not self._LABEL.match(s) or self.text(s) != s:
            return self._reject(kind)
        return s

    def fingerprint(self, tool_name, value):
        """A tool_calls.fingerprint, checked against transcript-format.md § Tool-call fingerprints."""
        if value is None:
            return None
        s = str(value)
        if tool_name in ("Bash", "PowerShell"):
            words = s.split(" ")
            ok = 1 <= len(words) <= 2 and all(self._WORD.match(w) for w in words) and "/" not in words[0]
        elif tool_name in ("Read", "Edit", "Write", "Grep", "Glob", "NotebookEdit", "MultiEdit"):
            s = s.replace("\\", "/")
            ok = s == "<external>" or bool(self._REL_PATH.match(s))
        elif str(tool_name).startswith("mcp__"):
            ok = s == tool_name and bool(self._LABEL.match(s))
        else:
            ok = False  # the contract stores NULL for every other tool
        if not ok or self.text(s) != s:
            return self._reject("fingerprint")
        return s
