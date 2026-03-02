#!/usr/bin/env python3
"""
Effect Script Stub Generator

Reads effect definitions from the PostgreSQL database and generates stub Python
scripts with correct class names, method signatures, and category-appropriate
template logic.

Only 4 of 3,217 effects currently have scripts. This tool generates stubs for
the rest, categorized by the parent ability's type_id.

Usage:
    python generate_effect_stubs.py [options]

    --db-host HOST       Database host (default: localhost)
    --db-port PORT       Database port (default: 5433)
    --db-name NAME       Database name (default: sgw)
    --db-user USER       Database user (default: w-testing)
    --db-pass PASS       Database password (default: w-testing)
    --output-dir DIR     Output directory (default: ../python/cell/effects)
    --dry-run            Print what would be generated without writing files
    --overwrite          Overwrite existing scripts (default: skip)
    --category CATEGORY  Only generate for a specific ability type
                         (DD, Heal, Buff, Debuff, DOT, Undefined)
"""

import argparse
import os
import re
import sys

try:
    import psycopg2
except ImportError:
    psycopg2 = None


# ---------------------------------------------------------------------------
# Category templates
# ---------------------------------------------------------------------------

ABILITY_TYPE_TEMPLATES = {
    "ABILITY_TYPE_DD": {
        "label": "Direct Damage",
        "methods": {
            "onPulseBegin": '''\
    def onPulseBegin(self):
        # Direct damage: QR check -> armor -> apply stat change -> notify
        # Stat IDs: 7 = health, 8 = focus
        # Damage types: 14 = DT_Physical, 15 = DT_Energy, 16 = DT_Hazmat, 17 = DT_Psionic
        health_damage = int(self.params.get('HealthDamage', 0))
        focus_damage = int(self.params.get('FocusDamage', 0))
        if health_damage:
            self.effect.qrCombatDamage(7, 14, health_damage, True, True)
        if focus_damage:
            self.effect.qrCombatDamage(8, 14, focus_damage, True, True)
''',
        },
    },
    "ABILITY_TYPE_Heal": {
        "label": "Heal",
        "methods": {
            "onPulseBegin": '''\
    def onPulseBegin(self):
        # Heal: apply positive stat change -> notify
        # Use changeStat with STAT_Absolute (0) for flat heals
        heal_amount = int(self.params.get('HealAmount', 0))
        if heal_amount:
            self.effect.changeStat(7, 0, heal_amount, True)  # health, absolute, permanent
''',
        },
    },
    "ABILITY_TYPE_Buff": {
        "label": "Buff",
        "methods": {
            "onEffectInit": '''\
    def onEffectInit(self):
        # Buff: modify stat max -> set timer -> remove on expiry
        # Non-permanent changes are automatically reverted when the effect is removed
        stat_id = int(self.params.get('StatId', 0))
        bonus = int(self.params.get('Bonus', 0))
        if stat_id and bonus:
            self.effect.changeStat(stat_id, 0, bonus, False)  # absolute, non-permanent
''',
        },
    },
    "ABILITY_TYPE_Debuff": {
        "label": "Debuff",
        "methods": {
            "onEffectInit": '''\
    def onEffectInit(self):
        # Debuff: reduce stat -> auto-reverted when effect is removed
        stat_id = int(self.params.get('StatId', 0))
        penalty = int(self.params.get('Penalty', 0))
        if stat_id and penalty:
            self.effect.changeStat(stat_id, 0, -penalty, False)  # absolute, non-permanent
''',
        },
    },
    "ABILITY_TYPE_DOT": {
        "label": "Damage over Time",
        "methods": {
            "onPulseBegin": '''\
    def onPulseBegin(self):
        # DoT: apply small damage per pulse -> remove after duration
        # pulse_count and pulse_duration are set on the effect definition
        tick_damage = int(self.params.get('TickDamage', 0))
        if tick_damage:
            self.effect.qrCombatDamage(7, 14, tick_damage, True, True)
''',
        },
    },
    "ABILITY_TYPE_Undefined": {
        "label": "Undefined / Utility",
        "methods": {
            "onPulseBegin": '''\
    def onPulseBegin(self):
        # Utility effect - implement based on effect description
        pass
''',
        },
    },
}


def sanitize_class_name(name):
    """Convert an effect name to a valid Python class name."""
    # Remove non-alphanumeric characters except spaces
    name = re.sub(r"[^a-zA-Z0-9 ]", "", name)
    # Convert to PascalCase
    parts = name.split()
    class_name = "".join(word.capitalize() for word in parts if word)
    # Ensure it starts with a letter
    if class_name and not class_name[0].isalpha():
        class_name = "Effect" + class_name
    return class_name or "UnnamedEffect"


def sanitize_filename(name):
    """Convert an effect name to a valid filename."""
    return sanitize_class_name(name) + ".py"


def generate_stub(effect_id, effect_name, effect_desc, ability_type, nvps,
                  pulse_count, pulse_duration, flags, is_channeled):
    """Generate a stub effect script string."""
    class_name = sanitize_class_name(effect_name)
    template = ABILITY_TYPE_TEMPLATES.get(ability_type,
                                          ABILITY_TYPE_TEMPLATES["ABILITY_TYPE_Undefined"])

    lines = []
    lines.append('"""')
    lines.append("Effect: %s (ID: %d)" % (effect_name, effect_id))
    lines.append("Type: %s" % template["label"])
    if effect_desc:
        for desc_line in effect_desc.strip().split("\n"):
            lines.append("  %s" % desc_line.strip())
    if pulse_count:
        lines.append("Pulses: %d (%.1fs interval)" % (pulse_count, pulse_duration))
    if is_channeled:
        lines.append("Channeled: Yes")
    if nvps:
        lines.append("NVPs: %s" % ", ".join("%s=%s" % (k, v) for k, v in sorted(nvps.items())))
    lines.append('"""')
    lines.append("")
    lines.append("from cell.Script import EffectScript")
    lines.append("")
    lines.append("")
    lines.append("class %s(EffectScript):" % class_name)

    # Generate methods from template
    for method_name, method_body in template["methods"].items():
        lines.append(method_body)

    # Constructor with NVP loading
    lines.append("    def __init__(self, owner, effect, nvps):")
    lines.append("        EffectScript.__init__(self, owner, effect, nvps)")
    if nvps:
        for nvp_name in sorted(nvps.keys()):
            attr_name = re.sub(r"[^a-zA-Z0-9_]", "_", nvp_name)
            lines.append("        self.%s = self.params.get('%s')" % (attr_name, nvp_name))
    lines.append("")

    return "\n".join(lines)


def fetch_effects_from_db(args):
    """Fetch effect definitions from the database."""
    if psycopg2 is None:
        print("ERROR: psycopg2 is not installed. Install with: pip install psycopg2-binary")
        sys.exit(1)

    conn = psycopg2.connect(
        host=args.db_host,
        port=args.db_port,
        dbname=args.db_name,
        user=args.db_user,
        password=args.db_pass,
    )
    cur = conn.cursor()

    # Fetch effects with their parent ability type
    cur.execute("""
        SELECT e.effect_id, e.name, e.effect_desc, e.script_name,
               a.type_id::text AS ability_type,
               e.pulse_count, e.pulse_duration, e.flags, e.is_channeled
        FROM resources.effects e
        JOIN resources.abilities a ON e.ability_id = a.ability_id
        ORDER BY e.effect_id
    """)
    effects = cur.fetchall()

    # Fetch all NVPs
    cur.execute("""
        SELECT effect_id, name, value
        FROM resources.effect_nvps
        ORDER BY effect_id, nvp_id
    """)
    nvp_rows = cur.fetchall()

    # Group NVPs by effect_id
    nvps_by_effect = {}
    for effect_id, name, value in nvp_rows:
        if effect_id not in nvps_by_effect:
            nvps_by_effect[effect_id] = {}
        nvps_by_effect[effect_id][name] = value

    conn.close()

    result = []
    for row in effects:
        effect_id, name, desc, script_name, ability_type, pulse_count, pulse_duration, flags, is_channeled = row
        result.append({
            "effect_id": effect_id,
            "name": name or ("Effect_%d" % effect_id),
            "desc": desc,
            "script_name": script_name,
            "ability_type": ability_type,
            "nvps": nvps_by_effect.get(effect_id, {}),
            "pulse_count": pulse_count or 0,
            "pulse_duration": pulse_duration or 0,
            "flags": flags or 0,
            "is_channeled": is_channeled or False,
        })

    return result


def fetch_effects_from_sql(sql_path):
    """Parse effect definitions from resources.sql INSERT statements.

    Fallback when no database connection is available.
    """
    effects = []
    nvps_by_effect = {}

    # Maps to collect ability types
    ability_types = {}

    with open(sql_path, "r", encoding="utf-8") as f:
        content = f.read()

    # Parse ability types: INSERT INTO abilities (..., type_id, ...)
    # We need ability_id -> type_id mapping
    ability_pattern = re.compile(
        r"INSERT INTO abilities.*?VALUES\s*\((\d+),\s*'([^']*)'.*?'(ABILITY_TYPE_\w+)'",
        re.DOTALL,
    )
    for m in ability_pattern.finditer(content):
        ability_id = int(m.group(1))
        ability_types[ability_id] = m.group(3)

    # Parse effects
    effect_pattern = re.compile(
        r"INSERT INTO effects.*?VALUES\s*\("
        r"(\d+),\s*"          # effect_id
        r"(\d+),\s*"          # ability_id
        r"(\d+),\s*"          # delay
        r"'((?:[^']|'')*)',\s*"  # effect_desc
        r"(\d+),\s*"          # effect_sequence
        r"(\d+),\s*"          # flags
        r"'[^']*',\s*"        # icon_location
        r"(\d+),\s*"          # pulse_count
        r"([0-9.]+),\s*"      # pulse_duration
        r"'([^']*)',\s*"      # tcm_param1
        r"'([^']*)',\s*"      # tcm_param2
        r"'([^']*)',\s*"      # target_collection_method
        r"(true|false),\s*"   # use_ability_velocity
        r"(true|false),\s*"   # is_channeled
        r"'((?:[^']|'')*)',\s*"  # name
        r"(\d+),\s*"          # target_collection_id
        r"(NULL|\d+),\s*"     # event_set_id
        r"(NULL|'[^']*')"     # script_name
        r"\)",
        re.DOTALL,
    )

    for m in effect_pattern.finditer(content):
        effect_id = int(m.group(1))
        ability_id = int(m.group(2))
        desc = m.group(4).replace("''", "'")
        flags = int(m.group(6))
        pulse_count = int(m.group(7))
        pulse_duration = float(m.group(8))
        is_channeled = m.group(13) == "true"
        name = m.group(14).replace("''", "'")
        script_name_raw = m.group(17)
        script_name = None if script_name_raw == "NULL" else script_name_raw.strip("'")

        effects.append({
            "effect_id": effect_id,
            "name": name or ("Effect_%d" % effect_id),
            "desc": desc,
            "script_name": script_name,
            "ability_type": ability_types.get(ability_id, "ABILITY_TYPE_Undefined"),
            "nvps": {},
            "pulse_count": pulse_count,
            "pulse_duration": pulse_duration,
            "flags": flags,
            "is_channeled": is_channeled,
        })

    # Parse NVPs
    nvp_pattern = re.compile(
        r"INSERT INTO effect_nvps.*?VALUES\s*\("
        r"(\d+),\s*"         # nvp_id
        r"(\d+),\s*"         # effect_id
        r"'([^']*)',\s*"     # name
        r"'([^']*)'"         # value
        r"\)",
    )
    for m in nvp_pattern.finditer(content):
        effect_id = int(m.group(2))
        nvp_name = m.group(3)
        nvp_value = m.group(4)
        if effect_id not in nvps_by_effect:
            nvps_by_effect[effect_id] = {}
        nvps_by_effect[effect_id][nvp_name] = nvp_value

    # Attach NVPs to effects
    for effect in effects:
        effect["nvps"] = nvps_by_effect.get(effect["effect_id"], {})

    return effects


def main():
    parser = argparse.ArgumentParser(description="Generate effect script stubs")
    parser.add_argument("--db-host", default="localhost")
    parser.add_argument("--db-port", default="5433")
    parser.add_argument("--db-name", default="sgw")
    parser.add_argument("--db-user", default="w-testing")
    parser.add_argument("--db-pass", default="w-testing")
    parser.add_argument("--output-dir", default=None,
                        help="Output directory (default: <project>/python/cell/effects)")
    parser.add_argument("--sql-file", default=None,
                        help="Parse from resources.sql instead of connecting to DB")
    parser.add_argument("--dry-run", action="store_true",
                        help="Print summary without writing files")
    parser.add_argument("--overwrite", action="store_true",
                        help="Overwrite existing scripts")
    parser.add_argument("--category", default=None,
                        help="Only generate for a specific ability type")
    args = parser.parse_args()

    # Determine output directory
    if args.output_dir is None:
        script_dir = os.path.dirname(os.path.abspath(__file__))
        args.output_dir = os.path.join(script_dir, "..", "python", "cell", "effects")
    args.output_dir = os.path.abspath(args.output_dir)

    # Determine SQL file path
    if args.sql_file is None:
        script_dir = os.path.dirname(os.path.abspath(__file__))
        default_sql = os.path.join(script_dir, "..", "db", "resources.sql")
        if os.path.exists(default_sql):
            args.sql_file = default_sql

    # Fetch effects
    if args.sql_file and os.path.exists(args.sql_file):
        print("Reading effects from %s" % args.sql_file)
        effects = fetch_effects_from_sql(args.sql_file)
    elif psycopg2 is not None:
        print("Connecting to database %s@%s:%s/%s" % (
            args.db_user, args.db_host, args.db_port, args.db_name))
        effects = fetch_effects_from_db(args)
    else:
        print("ERROR: No database connection available and no SQL file found.")
        print("Provide --sql-file path/to/resources.sql or install psycopg2.")
        sys.exit(1)

    print("Found %d effects" % len(effects))

    # Filter by category if specified
    if args.category:
        category = args.category
        if not category.startswith("ABILITY_TYPE_"):
            category = "ABILITY_TYPE_" + category
        effects = [e for e in effects if e["ability_type"] == category]
        print("Filtered to %d effects of type %s" % (len(effects), category))

    # Statistics
    stats = {"skipped_has_script": 0, "skipped_exists": 0, "generated": 0, "errors": 0}
    by_type = {}

    os.makedirs(args.output_dir, exist_ok=True)

    for effect in effects:
        # Skip effects that already have a script_name assigned in DB
        if effect["script_name"]:
            stats["skipped_has_script"] += 1
            continue

        filename = sanitize_filename(effect["name"])
        filepath = os.path.join(args.output_dir, filename)

        # Skip if file exists and not overwriting
        if os.path.exists(filepath) and not args.overwrite:
            stats["skipped_exists"] += 1
            continue

        # Track by type
        atype = effect["ability_type"]
        by_type[atype] = by_type.get(atype, 0) + 1

        if args.dry_run:
            stats["generated"] += 1
            continue

        # Generate and write
        try:
            stub = generate_stub(
                effect["effect_id"],
                effect["name"],
                effect["desc"],
                effect["ability_type"],
                effect["nvps"],
                effect["pulse_count"],
                effect["pulse_duration"],
                effect["flags"],
                effect["is_channeled"],
            )
            with open(filepath, "w", encoding="utf-8") as f:
                f.write(stub)
            stats["generated"] += 1
        except Exception as e:
            print("ERROR generating %s: %s" % (filename, e))
            stats["errors"] += 1

    # Report
    print("")
    print("Results:")
    print("  Generated:          %d" % stats["generated"])
    print("  Skipped (has script): %d" % stats["skipped_has_script"])
    print("  Skipped (exists):   %d" % stats["skipped_exists"])
    print("  Errors:             %d" % stats["errors"])
    print("")
    if by_type:
        print("By ability type:")
        for atype, count in sorted(by_type.items()):
            label = ABILITY_TYPE_TEMPLATES.get(atype, {}).get("label", atype)
            print("  %-30s %d" % (label, count))


if __name__ == "__main__":
    main()
