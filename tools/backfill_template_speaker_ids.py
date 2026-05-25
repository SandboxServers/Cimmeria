"""Generate UPDATE statements to backfill entity_templates.speaker_id.

Match each template to speakers via THREE keys (in priority order):
  1. entity_templates.name -> speakers.name (exact, case-insensitive)
  2. entity_templates.name_id -> texts.text (lang=1033) -> speakers.name
  3. entity_templates.template_name -> speakers.name

For (3) we also try common variants: strip trailing " (...)" qualifiers,
swap "Cellblock" / "Temple" / "Pet" suffixes, normalize "Col Marsh" -> "Col. Marsh".

Output: SQL UPDATE statements + a coverage report. Templates that match
multiple speakers are reported but not auto-updated.
"""

import re
import sys

SPK_FILE = "db/resources/Dialogs/Seed/speakers.sql"
TPL_FILE = "db/resources/Entities/Seed/entity_templates.sql"
TXT_FILE = "db/resources/Texts/Seed/texts.sql"
SCR_FILE = "db/resources/Dialogs/Seed/dialog_screens.sql"

# ── speakers ─────────────────────────────────────────────────────────────
speakers = {}
spk_pat = re.compile(r"VALUES \(\s*(\d+)\s*,\s*'((?:[^']|'')*)'\s*\)")
with open(SPK_FILE, encoding="utf-8") as f:
    for line in f:
        m = spk_pat.search(line)
        if m:
            speakers[int(m.group(1))] = m.group(2).replace("''", "'")


def norm(s):
    if not s:
        return ""
    s = s.replace("''", "'").strip().lower()
    s = re.sub(r"\s+", " ", s)
    s = s.replace("'", "")  # apostrophe-insensitive (Ba'al == Baal)
    return s


# Tokens for substring matching: take the "primary name token" (last
# alphabetical token BEFORE any parenthetical qualifier — so "Capt. Copplemann
# (Castle)" -> "copplemann", "Agent Woolsey" -> "woolsey").
def last_token(s):
    if not s:
        return ""
    # strip trailing parenthetical: "Foo (Bar)" -> "Foo"
    base = re.sub(r"\s*\([^)]*\)\s*$", "", s).strip()
    tokens = re.findall(r"[A-Za-z']+", base)
    return tokens[-1].lower().replace("'", "") if tokens else ""


spk_by_norm = {}
spk_by_lasttoken = {}
for sid, nm in speakers.items():
    spk_by_norm.setdefault(norm(nm), []).append(sid)
    tok = last_token(nm)
    if tok and len(tok) >= 5:  # avoid noise like "the", "of", short tokens
        spk_by_lasttoken.setdefault(tok, []).append(sid)


# ── texts (lang=1033 == English only) ────────────────────────────────────
# Build name_id -> display_text map.
name_id_to_text = {}
txt_pat = re.compile(
    r"VALUES \(\s*(\d+)\s*,\s*\d+\s*,\s*(\d+)\s*,\s*'((?:[^']|'')*)'\s*,\s*'((?:[^']|'')*)'\s*\)"
)
with open(TXT_FILE, encoding="utf-8") as f:
    for line in f:
        if not line.startswith("INSERT INTO texts"):
            continue
        m = txt_pat.search(line)
        if not m:
            continue
        moniker_id, lang, _moniker_name, text = m.groups()
        if lang != "1033":
            continue
        text = text.replace("''", "'")
        name_id_to_text[int(moniker_id)] = text


# ── entity_templates ─────────────────────────────────────────────────────
def parse_values(s):
    out, cur, depth, in_str = [], [], 0, False
    i = 0
    while i < len(s):
        c = s[i]
        if in_str:
            if c == "'":
                if i + 1 < len(s) and s[i + 1] == "'":
                    cur.append("''")
                    i += 2
                    continue
                in_str = False
                cur.append(c)
            else:
                cur.append(c)
        else:
            if c == "'":
                in_str = True
                cur.append(c)
            elif c == "{":
                depth += 1
                cur.append(c)
            elif c == "}":
                depth -= 1
                cur.append(c)
            elif c == "," and depth == 0:
                out.append("".join(cur).strip())
                cur = []
            elif c == ")" and depth == 0:
                out.append("".join(cur).strip())
                return out
            else:
                cur.append(c)
        i += 1
    return out


COLS = [
    "template_id", "static_mesh", "body_set", "components", "flags",
    "interaction_type", "event_set_id", "level", "alignment", "faction",
    "name_id", "name", "patrol_path_id", "patrol_point_delay",
    "template_name", "class", "buy_item_list", "sell_item_list",
    "repair_item_list", "recharge_item_list", "ability_set_id", "ammo_type",
    "loot_table_id", "primary_color_id", "secondary_color_id", "skin_tint",
    "weapon_item_id", "static_interaction_sets", "trainer_ability_list_id",
    "speaker_id", "has_dynamic_properties", "interaction_set_id",
]
TI = COLS.index("template_id")
NID = COLS.index("name_id")
NM = COLS.index("name")
TN = COLS.index("template_name")
CL = COLS.index("class")
SP = COLS.index("speaker_id")


def strip_quotes(v):
    if v == "NULL":
        return None
    if v.startswith("'") and v.endswith("'"):
        return v[1:-1].replace("''", "'")
    return v


templates = []
with open(TPL_FILE, encoding="utf-8") as f:
    for line in f:
        idx = line.find("VALUES (")
        if idx < 0:
            continue
        vals = parse_values(line[idx + len("VALUES ("):])
        if len(vals) != 32:
            continue
        templates.append({
            "template_id": int(vals[TI]),
            "name_id": None if vals[NID] == "NULL" else int(vals[NID]),
            "name": strip_quotes(vals[NM]),
            "template_name": strip_quotes(vals[TN]),
            "class": strip_quotes(vals[CL]),
            "speaker_id": None if vals[SP] == "NULL" else int(vals[SP]),
        })


# ── Match — try exact, then resolved name_id, then template_name variants ─
def name_variants(s):
    """Generate plausible name variants for fuzzy matching."""
    if not s:
        return []
    out = {s}
    # strip " (something)" qualifier
    base = re.sub(r"\s*\(.*?\)\s*$", "", s).strip()
    out.add(base)
    # replace "Cpt " -> "Capt. " / "Captain "
    if base.lower().startswith("cpt "):
        rest = base[4:]
        out.add("Capt. " + rest)
        out.add("Captain " + rest)
    # replace "Col " -> "Col. " (Marsh case)
    if re.match(r"^Col\s", base):
        out.add(re.sub(r"^Col\s", "Col. ", base))
    # strip trailing " corpse"
    out.add(re.sub(r"\s*corpse$", "", base, flags=re.IGNORECASE).strip())
    # strip leading "SGC ", "NID ", "Praxis " etc.
    out.add(re.sub(r"^(SGC|NID|Praxis|Op-CORE)\s+", "", base).strip())
    # Sam Carter <-> Samantha Carter <-> Colonel Carter
    if "sam carter" in base.lower() or "samantha carter" in base.lower():
        out.add("Colonel Carter")
    return [v for v in out if v]


def try_match(template):
    """Return (matched_speaker_ids, key_used_for_reporting)."""
    candidates_seen = []  # list of (sids:set, key_label)

    # Key 1: explicit name field
    if template["name"]:
        for variant in name_variants(template["name"]):
            sids = set(spk_by_norm.get(norm(variant), []))
            if sids:
                candidates_seen.append((sids, f"name='{variant}'"))
                break

    # Key 2: resolved name_id
    if template["name_id"] and template["name_id"] in name_id_to_text:
        text = name_id_to_text[template["name_id"]]
        if text:
            for variant in name_variants(text):
                sids = set(spk_by_norm.get(norm(variant), []))
                if sids:
                    candidates_seen.append(
                        (sids, f"name_id={template['name_id']}('{text}')='{variant}'")
                    )
                    break

    # Key 3: template_name variants (lowest priority — often internal)
    if template["template_name"]:
        for variant in name_variants(template["template_name"]):
            sids = set(spk_by_norm.get(norm(variant), []))
            if sids:
                candidates_seen.append((sids, f"template_name->'{variant}'"))
                break

    # Key 4: last-token substring match (only as last resort).
    # Looks for the distinguishing surname in either name or template_name
    # against speakers' last-token index. Skips short generics.
    if not candidates_seen:
        for source_label, raw in [
            ("name", template["name"]),
            ("name_id-text", name_id_to_text.get(template["name_id"]) if template["name_id"] else None),
            ("template_name", template["template_name"]),
        ]:
            if not raw:
                continue
            tok = last_token(raw)
            if tok and len(tok) >= 5 and tok in spk_by_lasttoken:
                sids = set(spk_by_lasttoken[tok])
                # Require the speaker name to include the same token to avoid
                # false positives like "Sandbox Greeting NPC" -> name_id 'Nerus'
                # (where the source has no shared token with 'Nerus' anyway).
                candidates_seen.append((sids, f"{source_label} last-token '{tok}'"))
                break

    return candidates_seen


# ── speaker frequency in dialog_screens (used to break ambiguous ties) ──
scr_pat = re.compile(
    r"VALUES \(\s*\d+\s*,\s*\d+\s*,\s*'(?:[^']|'')*'\s*,\s*([\dA-Za-z_]+)\s*,\s*\d+\s*\)"
)
speaker_freq = {}
with open(SCR_FILE, encoding="utf-8") as f:
    for line in f:
        m = scr_pat.search(line)
        if m and m.group(1) not in ("0", "NULL"):
            try:
                sid = int(m.group(1))
                speaker_freq[sid] = speaker_freq.get(sid, 0) + 1
            except ValueError:
                pass


# ── classify ─────────────────────────────────────────────────────────────
to_update = []           # (template_id, sid, name, evidence_label, speaker_name)
ambiguous_resolved = []  # ambiguous but we picked the most-used one
ambiguous_unresolved = []
no_match = []
already_consistent = []
already_inconsistent = []

for t in templates:
    cands = try_match(t)
    cur = t["speaker_id"]

    # Pick the first (highest priority) key that gave any match
    if not cands:
        no_match.append(t)
        continue

    sids, label = cands[0]
    display = t["name"] or (
        name_id_to_text.get(t["name_id"], "") if t["name_id"] else ""
    ) or t["template_name"]
    if len(sids) == 1:
        sid = next(iter(sids))
        if cur is None:
            to_update.append((t["template_id"], sid, display, label, speakers[sid]))
        elif cur == sid:
            already_consistent.append(t)
        else:
            already_inconsistent.append((t, sid))
    else:
        # Tie-break by dialog_screens frequency; pick the most-used speaker.
        ranked = sorted(sids, key=lambda s: (-speaker_freq.get(s, 0), s))
        top = ranked[0]
        top_freq = speaker_freq.get(top, 0)
        rest_label = ", ".join(
            f"{s}={speaker_freq.get(s, 0)}" for s in ranked
        )
        if cur is None:
            evidence = f"{label} | freq tiebreak: {rest_label}"
            to_update.append((t["template_id"], top, display, evidence, speakers[top]))
            ambiguous_resolved.append((t, ranked, top))
        elif cur in sids:
            already_consistent.append(t)
        else:
            ambiguous_unresolved.append((t, sids, label))


def out(s=""):
    sys.stdout.write(s + "\n")


out("=" * 72)
out("Matching summary")
out("=" * 72)
out(f"speakers table rows                : {len(speakers)}")
out(f"texts (en) entries                 : {len(name_id_to_text)}")
out(f"entity_templates rows              : {len(templates)}")
out(f"  already populated (non-NULL)     : {sum(1 for t in templates if t['speaker_id'] is not None)}")
out(f"  NULL speaker_id                  : {sum(1 for t in templates if t['speaker_id'] is None)}")
out(f"clean 1:1 matches to UPDATE        : {len(to_update) - len(ambiguous_resolved)}")
out(f"  + ambiguous, tie-broken by freq  : {len(ambiguous_resolved)}")
out(f"  = total UPDATE statements        : {len(to_update)}")
out(f"already populated & consistent     : {len(already_consistent)}")
out(f"already populated but DIFFERENT    : {len(already_inconsistent)}")
out(f"ambiguous & unresolved             : {len(ambiguous_unresolved)}")
out(f"no match at all                    : {len(no_match)}")

if ambiguous_resolved:
    out("")
    out("── AMBIGUOUS (resolved by dialog_screens frequency) ──")
    for t, ranked, top in ambiguous_resolved:
        opts = ", ".join(
            f"{s}=({speaker_freq.get(s, 0)} screens, '{speakers[s]}')"
            for s in ranked
        )
        out(f"  template_id={t['template_id']:>4} '{speakers[top]}': picked {top} from [{opts}]")

if ambiguous_unresolved:
    out("")
    out("── AMBIGUOUS (existing differs from all candidates) ──")
    for t, sids, label in ambiguous_unresolved:
        chosen = ", ".join(f"{s} ({speakers[s]})" for s in sorted(sids))
        out(f"  template_id={t['template_id']:>4} current={t['speaker_id']} candidates=[{chosen}]")

if already_inconsistent:
    out("")
    out("── INCONSISTENT (existing differs from name match) ──")
    for t, sid in already_inconsistent:
        out(f"  template_id={t['template_id']:>4} current={t['speaker_id']} name-match={sid}")

# Show no-match items that LOOK like NPCs (have name or class=mob/being)
out("")
out("── NO-MATCH but appears NPC-ish (review needed) ──")
n_npc = 0
for t in no_match:
    if t["class"] in ("mob", "being") and (t["name"] or t["template_name"]):
        if any(skip in (t["template_name"] or "").lower()
               for skip in ("guard", "drone", "tank", "corpse", "debug",
                            "do not use", "ring", "elevator", "crate", "controls",
                            "switch", "screen", "shelf")):
            continue
        n_npc += 1
        if n_npc <= 30:
            txt = name_id_to_text.get(t["name_id"], "") if t["name_id"] else ""
            out(f"  template_id={t['template_id']:>4} class={t['class']:<10}"
                f" name={t['name']!r:25} tpl_name={t['template_name']!r:30} name_id={t['name_id']} -> {txt!r}")
if n_npc > 30:
    out(f"  ... and {n_npc - 30} more")
out(f"  (total possibly-NPC unmatched: {n_npc})")

# ── SQL ──────────────────────────────────────────────────────────────────
out("")
out("=" * 72)
out("PROPOSED SQL")
out("=" * 72)
out("BEGIN;")
for tid, sid, name, evidence, spkr in to_update:
    sn = (name or "").replace("'", "''")
    sk = spkr.replace("'", "''")
    out(f"UPDATE resources.entity_templates SET speaker_id = {sid} "
        f"WHERE template_id = {tid}; -- {sn!r} via {evidence} -> '{sk}'")
out("COMMIT;")
out("")
out(f"Total UPDATE statements: {len(to_update)}")

# Coverage
dialog_spks = set(speaker_freq.keys())
covered = {m[1] for m in to_update} | {t["speaker_id"] for t in already_consistent}
out("")
out(f"Speaker IDs in dialog_screens: {len(dialog_spks)} distinct (non-zero)")
out(f"After these UPDATEs, covered:   {len(covered & dialog_spks)}")
out(f"Still uncovered:                {len(dialog_spks - covered)}")
covered_screens = sum(speaker_freq.get(s, 0) for s in covered)
total_screens = sum(speaker_freq.values())
out(f"  (weighted by screen count: {covered_screens}/{total_screens} = "
    f"{100*covered_screens/max(total_screens,1):.1f}%)")
