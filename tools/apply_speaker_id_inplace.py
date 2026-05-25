"""Apply the speaker_id backfill in-place on the entity_templates seed file.

Reads template_id -> speaker_id assignments (hardcoded below — copy them
from `backfill_template_speaker_ids.py` output), then for each matching
INSERT row replaces the speaker_id column value (NULL -> N) and writes back.

Verifies: each target row must currently have speaker_id == NULL; bails out
if any conflict so we don't silently overwrite.
"""

import re
import sys

TPL_FILE = "db/resources/Entities/Seed/entity_templates.sql"

# template_id -> (speaker_id, friendly_name) — 29 entries, from the matcher
ASSIGNMENTS = {
    10:  (941,  "Colonel Marsh"),
    29:  (2410, "General Hammond"),
    30:  (2316, "Teal'c"),
    33:  (708,  "Sam Carter (Colonel Carter)"),
    41:  (2547, "Ra"),
    42:  (942,  "Ba'al"),
    43:  (944,  "Anat"),
    44:  (949,  "Athena"),
    45:  (970,  "Morrigan"),
    46:  (978,  "Lethander"),
    47:  (1300, "Mr. Woolsey (Agent Woolsey)"),
    48:  (968,  "Capt. Copplemann"),
    50:  (156,  "Vala Mal Doran"),
    51:  (431,  "Daniel Jackson"),
    52:  (1304, "Placeholder Gen. Jack O'Neill"),
    53:  (943,  "Nerus"),
    54:  (945,  "Moh'Katan"),
    55:  (663,  "Warrick"),
    56:  (406,  "Goldam"),
    57:  (714,  "Major Davis"),
    58:  (843,  "Sgt. Harriman"),
    59:  (2323, "Master Bra'tac"),
    60:  (2547, "Ra"),
    61:  (2547, "Ra"),
    62:  (2547, "Ra"),
    63:  (2547, "Ra"),
    64:  (777,  "Thor"),
    166: (943,  "Sandbox Greeting NPC (Nerus)"),
    167: (942,  "Ba'al"),
}


def parse_values_pos(s, start_idx):
    """Yield (value, start_offset_within_line, end_offset_within_line) for
    each comma-separated value in a SQL VALUES (...) tuple starting at
    s[start_idx] (which should point just after the '(')."""
    cur_start = start_idx
    depth = 0
    in_str = False
    i = start_idx
    while i < len(s):
        c = s[i]
        if in_str:
            if c == "'":
                if i + 1 < len(s) and s[i + 1] == "'":
                    i += 2
                    continue
                in_str = False
            i += 1
        else:
            if c == "'":
                in_str = True
                i += 1
            elif c == "{":
                depth += 1
                i += 1
            elif c == "}":
                depth -= 1
                i += 1
            elif c == "," and depth == 0:
                yield s[cur_start:i].strip(), cur_start, i
                cur_start = i + 1
                i += 1
            elif c == ")" and depth == 0:
                yield s[cur_start:i].strip(), cur_start, i
                return
            else:
                i += 1


COLS_BEFORE_SPEAKER = 29  # speaker_id is the 30th column (0-indexed: 29)


def apply():
    with open(TPL_FILE, encoding="utf-8") as f:
        lines = f.readlines()

    tid_pat = re.compile(r"VALUES \(\s*(\d+)\s*,")
    changed = 0
    seen_tids = set()

    for li, line in enumerate(lines):
        if not line.startswith("INSERT INTO entity_templates"):
            continue
        m = tid_pat.search(line)
        if not m:
            continue
        tid = int(m.group(1))
        if tid not in ASSIGNMENTS:
            continue
        if tid in seen_tids:
            print(f"ERROR: duplicate INSERT for template_id={tid}", file=sys.stderr)
            sys.exit(1)
        seen_tids.add(tid)

        vstart = line.find("VALUES (") + len("VALUES (")
        vals = list(parse_values_pos(line, vstart))
        if len(vals) != 32:
            print(f"ERROR: template_id={tid} parsed {len(vals)} values, expected 32",
                  file=sys.stderr)
            sys.exit(1)

        cur_value, cs, ce = vals[COLS_BEFORE_SPEAKER]
        if cur_value != "NULL":
            print(f"ERROR: template_id={tid} already has speaker_id={cur_value}; "
                  f"refusing to overwrite", file=sys.stderr)
            sys.exit(1)

        new_speaker_id, friendly = ASSIGNMENTS[tid]
        # Preserve the leading-whitespace formatting of `, NULL,` by emitting
        # a leading space before the new value.
        new_line = line[:cs] + " " + str(new_speaker_id) + line[ce:]
        lines[li] = new_line
        print(f"  template_id={tid:>4} NULL -> {new_speaker_id:>4}  ({friendly})")
        changed += 1

    missing = set(ASSIGNMENTS) - seen_tids
    if missing:
        print(f"ERROR: assignments for template_id(s) {sorted(missing)} "
              f"matched no INSERT row", file=sys.stderr)
        sys.exit(1)

    if changed != len(ASSIGNMENTS):
        print(f"ERROR: expected {len(ASSIGNMENTS)} changes, made {changed}",
              file=sys.stderr)
        sys.exit(1)

    with open(TPL_FILE, "w", encoding="utf-8", newline="") as f:
        f.writelines(lines)
    print(f"\nApplied {changed} speaker_id assignments to {TPL_FILE}")


if __name__ == "__main__":
    apply()
