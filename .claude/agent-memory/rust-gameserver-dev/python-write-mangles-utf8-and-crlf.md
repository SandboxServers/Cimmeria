# Scripted edits: Python `write_text` mangles UTF-8 and CRLF on Windows

`pathlib.Path.write_text(s)` on this host encodes with **cp1252**, not UTF-8.
A round-trip of existing content is lossless (the bytes decode and re-encode
unchanged), but any *new* text containing a character outside cp1252's
Latin-1 range — em-dash `—`, en-dash `–`, curly quotes, ellipsis — is written
as a single high byte (`—` becomes `0x97`). The file then fails to compile
with `stream did not contain valid UTF-8`, and rustc points at the line, not
the cause.

Repo `.rs` and `.md` files are CRLF in the working tree, so `read_text()` /
`write_text()` with the default newline handling also silently rewrites line
endings.

**Always** read and write bytes explicitly:

```python
s = p.read_bytes().decode("utf-8")
# ... edit on a \n-normalised copy, restore \r\n before writing ...
p.write_bytes(s.encode("utf-8"))
```

A reusable helper worth recreating per session (scratchpad, not the repo):

```python
def apply(path, pairs, count=1):
    raw = pathlib.Path(path).read_bytes().decode("utf-8")
    crlf = "\r\n" in raw
    s = raw.replace("\r\n", "\n")
    for old, new in pairs:
        if old not in s:
            sys.exit(f"NOT FOUND in {path}:\n{old[:200]}")
        s = s.replace(old, new, count)
    if crlf:
        s = s.replace("\n", "\r\n")
    pathlib.Path(path).write_bytes(s.encode("utf-8"))
```

Two follow-on traps:

- **Anchor strings must use the file's real punctuation.** A match on `2-3`
  fails silently against `2–3` (U+2013). Prefer short anchors, or splice by
  line number after asserting the boundary lines.
- **Repair is byte-wise, not whole-file.** If only the appended text is
  corrupt, decode greedily as UTF-8 and fall back to cp1252 for the single
  bytes that fail, rather than re-decoding the whole file as cp1252.

`core.autocrlf = true` here, so the *index* stores LF regardless and a
CRLF/LF slip does not produce a spurious git diff — the compile failure is
the symptom that matters, not the diff.

## Git Bash shell tools have the same CRLF trap (2026-09-26)

- **`sed -i` strips every CR** from each file it is handed, including
  files with no match. Use `sed -b -i` (binary mode) to keep CRLF. In `-b`
  mode the line still ends in `\r`, so a `...foo$` anchor never matches:
  drop the `$` or use the Edit tool for anchored multi-line edits (the
  Edit tool preserves CRLF).
- **`grep -c $'\r$'` reports 0 on a CRLF file** (msys grep eats the CR).
  Count instead: `tr -cd '\r' < f | wc -c` vs `tr -cd '\n' < f | wc -c`;
  equal means all-CRLF. `unix2dos -q f` repairs a file a tool flattened.
- **`cargo fmt` can write a CRLF file back as LF** after an edit; recheck
  endings after formatting.
- **`git stash` + `git stash pop` turns staged `git mv` renames into
  unstaged deletes plus staged adds.** Content survives, but the index
  shape does not; don't stash mid-move, re-stage with `git add -A <dir>`.

## Backslashes do not survive an inline `python - <<'PY'` command

The Bash tool rewrites `\\` to `\` in the command text before bash sees it,
quoted heredoc or not. An anchor that ends a line with a Rust string
continuation (`trace,\` then newline) therefore reaches Python as a line
continuation, the anchor silently loses its newlines, and the replace
finds nothing (seen 2026-09-26 on `filters.rs` and `test.yml`).

**How to apply:** for any edit whose anchor contains a backslash, write the
script to the scratchpad with the Write tool and run it, using raw strings;
or use the Edit tool directly. In a raw string, never put the backslash
right before the closing quotes: `r"""…\"""` is a SyntaxError (the `\"`
escapes the quote). End the anchor one character earlier or include the
trailing newline.

Related: [[revert-verification-loses-uncommitted-fmt]],
[[tooling-filter-and-path-traps]].
