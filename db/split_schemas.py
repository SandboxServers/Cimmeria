#!/usr/bin/env python3
"""
split_schemas.py — Splits db/resources.sql and db/sgw.sql into per-object files
organised by domain, with seed data co-located in Seed/ subdirectories.

Run from the project root:
    python db/split_schemas.py

Generates:
    db/resources/    — one file per DDL/DML object in the resources schema
    db/sgw/          — one file per DDL/DML object in the public schema
    db/database.sql  — single \\ir master entry point (replaces both originals)

The original db/resources.sql and db/sgw.sql are NOT deleted by this script.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path
from collections import defaultdict

ROOT    = Path(__file__).resolve().parent.parent
DB      = ROOT / 'db'
# Source files may live in db/ or db/deprecated/ (after the initial split)
RES_SQL = DB / 'resources.sql' if (DB / 'resources.sql').exists() else DB / 'deprecated' / 'resources.sql'
SGW_SQL = DB / 'sgw.sql'       if (DB / 'sgw.sql').exists()       else DB / 'deprecated' / 'sgw.sql'
OUT_R   = DB / 'resources'
OUT_S   = DB / 'sgw'

# ---------------------------------------------------------------------------
# Domain routing tables — resources schema
# ---------------------------------------------------------------------------

R_TYPE: dict[str, str] = {
    # AI
    'CAH_DebugLevel': 'AI',           'EAIErrorStateAction': 'AI',
    'EAIState': 'AI',                 'ECASPosition': 'AI',
    'ECoverHeight': 'AI',             'ECoverQuality': 'AI',
    'EMobAggressionLevel': 'AI',      'EMobMovementType': 'AI',
    'EMobRank': 'AI',                 'EPatrolType': 'AI',
    'EPetStance': 'AI',               'EMobIntel': 'AI',
    # Abilities
    'EAbilityFlags': 'Abilities',     'EAbilityTypes': 'Abilities',
    'EActionHandlerEvent': 'Abilities','EAmmoType': 'Abilities',
    'EAppliedScienceLevel': 'Abilities','EReloadType': 'Abilities',
    'ESpecialWordFlags': 'Abilities', 'EStance': 'Abilities',
    'EWeaponRange': 'Abilities',      'EWeaponState': 'Abilities',
    'EWeaponType': 'Abilities',
    # Archetypes
    'EAlignment': 'Archetypes',       'EArchetype': 'Archetypes',
    'EGender': 'Archetypes',          'ESubsystem': 'Archetypes',
    'ETier': 'Archetypes',            'EUnitType': 'Archetypes',
    # Combat
    'ECombatantState': 'Combat',      'EDamageType': 'Combat',
    'EDuelDefeatReason': 'Combat',    'EDuelState': 'Combat',
    'EMitigationType': 'Combat',      'EStatLevel': 'Combat',
    'EStatResultCode': 'Combat',      'EStatType': 'Combat',
    'EStatValueType': 'Combat',       'EStats': 'Combat',
    'EThreatLevel': 'Combat',
    # Dialogs
    'EClientChallengeType': 'Dialogs','EClientLockBits': 'Dialogs',
    'EDialogDialogType': 'Dialogs',   'EDialogType': 'Dialogs',
    'EDialogUIScreenType': 'Dialogs', 'ETextType': 'Dialogs',
    'ESpeakerFlags': 'Dialogs',
    # Effects
    'EEffectClass': 'Effects',        'EEffectFlag': 'Effects',
    'EEffectType': 'Effects',
    # Entities
    'EResourceType': 'Entities',      'EEntityFlags': 'Entities',
    'EEntityPropertyType': 'Entities','EMoniker': 'Entities',
    'EMonikerFlags': 'Entities',
    # Events
    'EKismetViewType': 'Events',      'ESequenceEventType': 'Events',
    # Gameplay
    'ContextTypes': 'Gameplay',             'EAggressionIDType': 'Gameplay',
    'EBehaviorEventFlags': 'Gameplay',      'ECooldownCategory': 'Gameplay',
    'EConditionHandlerFeedback': 'Gameplay','ECrystals': 'Gameplay',
    'EEvent': 'Gameplay',                   'EGameFunctionFlags': 'Gameplay',
    'EGiveCashTag': 'Gameplay',             'EMovementType': 'Gameplay',
    'EPollType': 'Gameplay',                'EProxyMessages': 'Gameplay',
    'EReasons': 'Gameplay',                 'ETargetCollectionMethod': 'Gameplay',
    'ETargetCollectionParams': 'Gameplay',  'ETargetType': 'Gameplay',
    'ETimerType': 'Gameplay',               'Postures': 'Gameplay',
    'WanderTypes': 'Gameplay',              'active_interaction_map': 'Gameplay',
    'effect_action': 'Gameplay',            'effect_action_type': 'Gameplay',
    'EStateField': 'Gameplay',              'ETimerUpdateType': 'Gameplay',
    # Items
    'ECostType': 'Items',             'ECraftFlags': 'Items',
    'ECraftTypeFlags': 'Items',       'EInventoryContainerId': 'Items',
    'EItemFlag': 'Items',             'EItemQuality': 'Items',
    'EItemType': 'Items',             'EVendingSetFlag': 'Items',
    # Loot
    'ELootSetFlags': 'Loot',          'ELootType': 'Loot',
    # Missions
    'EMinigameCallEndedResult': 'Missions', 'EMinigameCallResult': 'Missions',
    'EMiniGameErrors': 'Missions',          'EMissionFlags': 'Missions',
    'EMissionTaskFlag': 'Missions',         'EResultCode': 'Missions',
    # Social
    'EBlackMarketError': 'Social',    'EBlackMarketFilter': 'Social',
    'EBlackMarketSearchType': 'Social','EBlackMarketSortType': 'Social',
    'EBlackMarketTime': 'Social',     'EContactListEvent': 'Social',
    'EFaction': 'Social',             'EGroupLootType': 'Social',
    'EMailFlags': 'Social',           'EMailResultCodes': 'Social',
    'EOrganizationPermission': 'Social','EOrganizationRank': 'Social',
    'EOrganizationType': 'Social',    'EPersistentOrganizationType': 'Social',
    'ERelationshipType': 'Social',    'ESide': 'Social',
    'ETradeLockState': 'Social',      'EMemberInfo': 'Social',
    'ETradeResults': 'Social',
    # System
    'EDBErrorType': 'System',         'EErrorAIStateReason': 'System',
    'EErrorCodeSystem': 'System',     'EGMInfoRequestType': 'System',
    'ELoggingEvent': 'System',
    # Visuals
    'EVisualGroupType': 'Visuals',
    # Worlds
    'EAoIRadiusType': 'Worlds',       'EChannel': 'Worlds',
    'EGateAddress': 'Worlds',         'EInteractionDirection': 'Worlds',
    'EInteractionNotificationType': 'Worlds', 'EInteractionType': 'Worlds',
    'ELocales': 'Worlds',             'EMapPing': 'Worlds',
    'EMapWaypoint': 'Worlds',         'ERegionEvents': 'Worlds',
    'ERegionFlags': 'Worlds',         'EWorldFlags': 'Worlds',
    'EWorldIdType': 'Worlds',
}

R_TABLE: dict[str, str] = {
    # Abilities
    'abilities': 'Abilities',           'ability_moniker_groups': 'Abilities',
    'ability_set_abilities': 'Abilities','ability_sets': 'Abilities',
    'applied_science': 'Abilities',     'trainer_abilities': 'Abilities',
    'trainer_ability_lists': 'Abilities',
    # Archetypes
    'archetype_ability_tree': 'Archetypes', 'archetypes': 'Archetypes',
    'char_creation': 'Archetypes',      'char_creation_abilities': 'Archetypes',
    'char_creation_choices': 'Archetypes','char_creation_visgroups': 'Archetypes',
    'disciplines': 'Archetypes',        'racial_paradigm': 'Archetypes',
    # Dialogs
    'dialog_screen_buttons': 'Dialogs', 'dialog_screens': 'Dialogs',
    'dialog_set_maps': 'Dialogs',       'dialog_sets': 'Dialogs',
    'dialogs': 'Dialogs',               'speakers': 'Dialogs',
    'special_words': 'Dialogs',
    # Effects
    'effect_nvps': 'Effects',           'effects': 'Effects',
    # Entities
    'blueprints': 'Entities',           'blueprints_components': 'Entities',
    'entity_templates': 'Entities',     'monikers': 'Entities',
    'resource_types': 'Entities',       'resource_versions': 'Entities',
    # Events
    'event_sets': 'Events',             'event_sets_sequences': 'Events',
    'paths': 'Events',                  'point_set_points': 'Events',
    'point_sets': 'Events',             'sequences': 'Events',
    'sequences_nvp': 'Events',
    # Items
    'containers': 'Items',              'item_list_items': 'Items',
    'item_list_prices': 'Items',        'item_lists': 'Items',
    'items': 'Items',                   'items_event_sets': 'Items',
    # Loot
    'loot': 'Loot',                     'loot_tables': 'Loot',
    # Missions
    'mission_objectives': 'Missions',   'mission_reward_groups': 'Missions',
    'mission_rewards': 'Missions',      'mission_steps': 'Missions',
    'mission_tasks': 'Missions',        'missions': 'Missions',
    # Texts
    'error_texts': 'Texts',             'texts': 'Texts',
    # Visuals
    'body_component_visuals': 'Visuals','body_components': 'Visuals',
    'body_sets': 'Visuals',             'skeletal_meshes': 'Visuals',
    'static_meshes': 'Visuals',
    # Worlds
    'entity_interactions': 'Worlds',    'generic_regions': 'Worlds',
    'interactions': 'Worlds',           'respawners': 'Worlds',
    'ring_transport_regions': 'Worlds', 'spawn_points': 'Worlds',
    'spawn_sets': 'Worlds',             'spawnlist': 'Worlds',
    'stargates': 'Worlds',              'worlds': 'Worlds',
}

# sgw (public) schema routing
S_TABLE: dict[str, str] = {
    'account': 'Accounts',
    'sgw_player': 'Players',
    'sgw_gate_mail': 'Mail',
    'sgw_inventory_base': 'Inventory',
    'sgw_inventory': 'Inventory',
    'sgw_mission': 'Missions',
    'shards': 'Shards',
}

S_TYPE: dict[str, str] = {
    'player_interaction_map': 'Accounts',
}

# ---------------------------------------------------------------------------
# SQL statement iterator
# ---------------------------------------------------------------------------

def iter_stmts(text: str):
    """Yield complete SQL statements from a pg_dump file.

    Handles single-quoted strings, dollar-quoted strings, and -- / /* */ comments.
    Uses standard_conforming_strings semantics: '' is the only escape inside '...'.
    """
    buf: list[str] = []
    in_sq = False    # inside '...' string
    in_dq = False    # inside $$...$$ or $tag$...$tag$ dollar quote
    dq_tag = ''      # current dollar-quote tag (including both $ delimiters)
    in_bc = False    # inside /* ... */ block comment
    i, n = 0, len(text)

    while i < n:
        c = text[i]

        # ── block comment ────────────────────────────────────────────────────
        if in_bc:
            buf.append(c)
            if c == '*' and i + 1 < n and text[i + 1] == '/':
                buf.append('/')
                i += 2
                in_bc = False
            else:
                i += 1
            continue

        # ── dollar-quoted string ─────────────────────────────────────────────
        if in_dq:
            end = i + len(dq_tag)
            if text[i:end] == dq_tag:
                buf.append(dq_tag)
                i = end
                in_dq = False
                dq_tag = ''
            else:
                buf.append(c)
                i += 1
            continue

        # ── single-quoted string ─────────────────────────────────────────────
        if in_sq:
            buf.append(c)
            if c == "'":
                if i + 1 < n and text[i + 1] == "'":
                    buf.append("'"); i += 2   # '' = escaped single quote
                else:
                    in_sq = False; i += 1
            else:
                i += 1
            continue

        # ── top-level ────────────────────────────────────────────────────────

        # line comment
        if c == '-' and i + 1 < n and text[i + 1] == '-':
            end = text.find('\n', i)
            end = n if end == -1 else end + 1
            buf.append(text[i:end])
            i = end
            continue

        # block comment open
        if c == '/' and i + 1 < n and text[i + 1] == '*':
            in_bc = True
            buf.append('/*')
            i += 2
            continue

        # single-quoted string open
        if c == "'":
            in_sq = True
            buf.append(c); i += 1
            continue

        # dollar-quoted string: $$ or $tag$
        if c == '$':
            j = i + 1
            while j < n and (text[j].isalnum() or text[j] == '_'):
                j += 1
            if j < n and text[j] == '$':
                tag = text[i:j + 1]
                buf.append(tag)
                i = j + 1
                in_dq = True
                dq_tag = tag
                continue
            # Not a dollar quote — fall through to default append

        # statement terminator
        if c == ';':
            buf.append(c)
            stmt = ''.join(buf).strip()
            # Strip comments to check if there's real SQL content
            body = re.sub(r'--[^\n]*', '', stmt)
            body = re.sub(r'/\*.*?\*/', '', body, flags=re.DOTALL).strip()
            if body and body != ';':
                yield stmt
            buf = []
            i += 1
            continue

        buf.append(c)
        i += 1


# ---------------------------------------------------------------------------
# Statement classifier
# ---------------------------------------------------------------------------

def _clean(stmt: str) -> str:
    """Strip comments and collapse whitespace for pattern matching."""
    s = re.sub(r'--[^\n]*', ' ', stmt)
    s = re.sub(r'/\*.*?\*/', ' ', s, flags=re.DOTALL)
    return ' '.join(s.split())


def _name(s: str, pat: str) -> str | None:
    """Extract first non-None group from a regex match."""
    m = re.search(pat, s, re.I)
    if not m:
        return None
    for g in m.groups():
        if g is not None:
            return g.strip('"')
    return None


def classify(stmt: str) -> tuple[str, str | None, str | None]:
    """Return (kind, name, extra).

    kind values:
        SET, CREATE_SCHEMA, CREATE_TYPE, CREATE_SEQUENCE, ALTER_SEQ_OWNED_BY,
        CREATE_TABLE, ALTER_COL, ADD_PK, ADD_FK, CREATE_INDEX,
        CREATE_FUNC, CREATE_TRIG, INSERT, SETVAL, SKIP
    """
    c = _clean(stmt)
    u = c.upper()

    if u.startswith('SET '):
        return 'SET', None, None

    if u.startswith('CREATE SCHEMA '):
        return 'CREATE_SCHEMA', _name(c, r'CREATE SCHEMA (?:"([^"]+)"|(\w+))'), None

    if u.startswith('CREATE TYPE '):
        return 'CREATE_TYPE', _name(c, r'CREATE TYPE (?:\w+\.)?(?:"([^"]+)"|(\w+))'), None

    if u.startswith('CREATE SEQUENCE '):
        return 'CREATE_SEQUENCE', _name(c, r'CREATE SEQUENCE (?:\w+\.)?(?:"([^"]+)"|(\w+))'), None

    if u.startswith('ALTER SEQUENCE '):
        if 'OWNED BY' in u:
            seq = _name(c, r'ALTER SEQUENCE (?:\w+\.)?(?:"([^"]+)"|(\w+))')
            # OWNED BY [schema.]table.column — extract table (second-to-last segment)
            m = re.search(r'OWNED BY ((?:[\w"]+\.)+[\w"]+)', c, re.I)
            tbl = None
            if m:
                parts = [p.strip('"') for p in m.group(1).split('.')]
                tbl = parts[-2] if len(parts) >= 2 else None
            return 'ALTER_SEQ_OWNED_BY', seq, tbl
        return 'SKIP', None, None

    if u.startswith('CREATE TABLE '):
        return 'CREATE_TABLE', _name(c, r'CREATE TABLE (?:\w+\.)?(?:"([^"]+)"|(\w+))'), None

    if u.startswith('ALTER TABLE '):
        tbl = _name(c, r'ALTER TABLE (?:ONLY )?(?:\w+\.)?(?:"([^"]+)"|(\w+))')
        if 'ADD CONSTRAINT' in u:
            if 'PRIMARY KEY' in u or ('UNIQUE' in u and 'FOREIGN KEY' not in u):
                return 'ADD_PK', tbl, None
            if 'FOREIGN KEY' in u:
                return 'ADD_FK', tbl, None
        if 'ALTER COLUMN' in u:
            return 'ALTER_COL', tbl, None
        return 'SKIP', None, None

    if re.match(r'CREATE (UNIQUE )?INDEX\b', c, re.I):
        return 'CREATE_INDEX', None, None

    if re.match(r'CREATE (OR REPLACE )?FUNCTION\b', c, re.I):
        return 'CREATE_FUNC', None, None

    if re.match(r'CREATE (CONSTRAINT )?TRIGGER\b', c, re.I):
        return 'CREATE_TRIG', None, None

    if u.startswith('INSERT INTO '):
        return 'INSERT', _name(c, r'INSERT INTO (?:\w+\.)?(?:"([^"]+)"|(\w+))'), None

    if 'SETVAL(' in u or 'SETVAL (' in u:
        m = re.search(r"setval\s*\(\s*'(?:[^.']+\.)?([^']+)'", c, re.I)
        return 'SETVAL', (m.group(1) if m else None), None

    return 'SKIP', None, None


# ---------------------------------------------------------------------------
# Sequence → table pre-scanner (two-pass for correct domain routing)
# ---------------------------------------------------------------------------

def _build_seq_table(stmts: list[str]) -> dict[str, str]:
    """Scan all ALTER SEQUENCE ... OWNED BY to map seq_name → table_name."""
    result: dict[str, str] = {}
    for s in stmts:
        kind, name, extra = classify(s)
        if kind == 'ALTER_SEQ_OWNED_BY' and name and extra:
            result[name] = extra
    return result


# ---------------------------------------------------------------------------
# File routing
# ---------------------------------------------------------------------------

Files = dict[Path, list[str]]


def _seq_dom_r(seq: str, seq_tbl: dict[str, str]) -> str:
    """Domain for a resources schema sequence."""
    tbl = seq_tbl.get(seq)
    if tbl:
        return R_TABLE.get(tbl, 'Misc')
    # Name-based fallback: longest table prefix
    best = ''
    for t in R_TABLE:
        if seq.startswith(t) and len(t) > len(best):
            best = t
    return R_TABLE.get(best, 'Misc') if best else 'Misc'


def _seq_dom_s(seq: str, seq_tbl: dict[str, str]) -> str:
    """Domain for a sgw (public) schema sequence."""
    tbl = seq_tbl.get(seq)
    return S_TABLE.get(tbl, 'Misc') if tbl else 'Misc'


def route_resources(stmts: list[str]) -> Files:
    seq_tbl = _build_seq_table(stmts)
    files: Files = defaultdict(list)
    r = OUT_R

    for stmt in stmts:
        kind, name, extra = classify(stmt)
        nm = name or 'unknown'

        if kind in ('SET', 'CREATE_SCHEMA'):
            files[r / '_schema.sql'].append(stmt)

        elif kind == 'CREATE_TYPE':
            dom = R_TYPE.get(nm, 'Misc')
            files[r / dom / 'Types' / f'{nm}.sql'].append(stmt)

        elif kind == 'CREATE_SEQUENCE':
            dom = _seq_dom_r(nm, seq_tbl)
            files[r / dom / 'Sequences' / f'{nm}.sql'].append(stmt)

        elif kind == 'ALTER_SEQ_OWNED_BY':
            # Must load AFTER tables (OWNED BY references the table) — collect
            # in a dedicated file that database.sql loads between Tables and PKs.
            files[r / '_sequence_ownership.sql'].append(stmt)

        elif kind == 'CREATE_TABLE':
            dom = R_TABLE.get(nm, 'Misc')
            files[r / dom / 'Tables' / f'{nm}.sql'].append(stmt)

        elif kind == 'ALTER_COL':
            dom = R_TABLE.get(nm, 'Misc')
            files[r / dom / 'Tables' / f'{nm}.sql'].append(stmt)

        elif kind == 'ADD_PK':
            files[r / '_primary_keys.sql'].append(stmt)

        elif kind == 'ADD_FK':
            files[r / '_foreign_keys.sql'].append(stmt)

        elif kind == 'CREATE_INDEX':
            files[r / '_indexes.sql'].append(stmt)

        elif kind == 'CREATE_FUNC':
            files[r / '_functions.sql'].append(stmt)

        elif kind == 'CREATE_TRIG':
            files[r / '_triggers.sql'].append(stmt)

        elif kind == 'INSERT':
            dom = R_TABLE.get(nm, 'Misc')
            files[r / dom / 'Seed' / f'{nm}.sql'].append(stmt)

        elif kind == 'SETVAL':
            tbl = seq_tbl.get(nm)
            if not tbl:
                # No OWNED BY — infer table from sequence name (longest prefix match)
                best = ''
                for t in R_TABLE:
                    if nm.startswith(t) and len(t) > len(best):
                        best = t
                tbl = best or None
            if tbl:
                dom = R_TABLE.get(tbl, 'Misc')
                files[r / dom / 'Seed' / f'{tbl}.sql'].append(stmt)
            else:
                files[r / 'Misc' / 'Seed' / f'{nm}.sql'].append(stmt)

        elif kind == 'SKIP':
            pass
        else:
            print(f'  [warn] unhandled: kind={kind} name={nm}', file=sys.stderr)

    return files


def route_sgw(stmts: list[str]) -> Files:
    seq_tbl = _build_seq_table(stmts)
    files: Files = defaultdict(list)
    s = OUT_S

    for stmt in stmts:
        kind, name, extra = classify(stmt)
        nm = name or 'unknown'

        if kind == 'SET':
            files[s / '_schema.sql'].append(stmt)

        elif kind == 'CREATE_SCHEMA':
            files[s / '_schema.sql'].append(stmt)

        elif kind == 'CREATE_TYPE':
            dom = S_TYPE.get(nm, 'Misc')
            files[s / dom / 'Types' / f'{nm}.sql'].append(stmt)

        elif kind == 'CREATE_SEQUENCE':
            dom = _seq_dom_s(nm, seq_tbl)
            files[s / dom / 'Sequences' / f'{nm}.sql'].append(stmt)

        elif kind == 'ALTER_SEQ_OWNED_BY':
            # Must load AFTER tables — collect in a dedicated file.
            files[s / '_sequence_ownership.sql'].append(stmt)

        elif kind == 'CREATE_TABLE':
            dom = S_TABLE.get(nm, 'Misc')
            files[s / dom / 'Tables' / f'{nm}.sql'].append(stmt)

        elif kind == 'ALTER_COL':
            dom = S_TABLE.get(nm, 'Misc')
            files[s / dom / 'Tables' / f'{nm}.sql'].append(stmt)

        elif kind == 'ADD_PK':
            files[s / '_primary_keys.sql'].append(stmt)

        elif kind == 'ADD_FK':
            files[s / '_foreign_keys.sql'].append(stmt)

        elif kind == 'CREATE_INDEX':
            files[s / '_indexes.sql'].append(stmt)

        elif kind == 'INSERT':
            dom = S_TABLE.get(nm, 'Misc')
            files[s / dom / 'Seed' / f'{nm}.sql'].append(stmt)

        elif kind == 'SETVAL':
            tbl = seq_tbl.get(nm)
            if tbl:
                dom = S_TABLE.get(tbl, 'Misc')
                files[s / dom / 'Seed' / f'{tbl}.sql'].append(stmt)
            else:
                files[s / 'Misc' / 'Seed' / f'{nm}.sql'].append(stmt)

        elif kind == 'SKIP':
            pass
        else:
            print(f'  [warn] unhandled: kind={kind} name={nm}', file=sys.stderr)

    return files


# ---------------------------------------------------------------------------
# File writer
# ---------------------------------------------------------------------------

def write_files(files: Files) -> int:
    total = 0
    for path in sorted(files):
        path.parent.mkdir(parents=True, exist_ok=True)
        with open(path, 'w', encoding='utf-8', newline='\n') as f:
            for stmt in files[path]:
                f.write(stmt.strip())
                f.write('\n\n')
        total += 1
    return total


# ---------------------------------------------------------------------------
# database.sql generator
# ---------------------------------------------------------------------------

def _table_sort_key(p: Path) -> tuple:
    """Sort key for table files: base/parent tables before derived ones (INHERITS)."""
    name = p.stem
    # '_base' and '_parent' suffixes indicate base tables that must load first
    is_derived = not (name.endswith('_base') or name.endswith('_parent'))
    return (p.parent, is_derived, name)


def _ir_section(schema_dir: Path, subdir: str, header: str) -> list[str]:
    r"""Return \ir lines for all .sql files in schema_dir/*/subdir/."""
    found = list(schema_dir.rglob(f'*/{subdir}/*.sql'))
    if not found:
        return []
    # For Tables: sort base tables before derived (handles INHERITS dependencies)
    if subdir == 'Tables':
        found = sorted(found, key=_table_sort_key)
    else:
        found = sorted(found)
    lines = [f'-- {header}']
    for f in found:
        lines.append(f'\\ir {f.relative_to(DB).as_posix()}')
    lines.append('')
    return lines


def _ir_file(path: Path) -> list[str]:
    r"""Return a single \ir line if the file exists."""
    if not path.exists():
        return []
    return [f'\\ir {path.relative_to(DB).as_posix()}', '']


def generate_master() -> None:
    r"""Write db/database.sql as the single \ir master entry point."""
    lines: list[str] = [
        '-- database.sql — single entry point for the Cimmeria database.',
        '-- Replaces db/resources.sql + db/sgw.sql.',
        '--',
        '-- Usage (fresh load with Force bootstrap):',
        '--   psql -U w-testing -d sgw -v ON_ERROR_STOP=1 -f db/database.sql',
        '--',
        '-- All paths are relative to this file (\\ir = include-relative).',
        '',
        '-- ════════════════════════════════════════════════════════════════',
        '-- resources schema',
        '-- ════════════════════════════════════════════════════════════════',
        '',
    ]

    lines += _ir_file(OUT_R / '_schema.sql')
    lines += _ir_section(OUT_R, 'Types',     'Types')
    lines += _ir_section(OUT_R, 'Sequences', 'Sequences')
    lines += _ir_section(OUT_R, 'Tables',    'Tables')
    for fname in ('_sequence_ownership.sql', '_primary_keys.sql', '_foreign_keys.sql',
                  '_indexes.sql', '_functions.sql', '_triggers.sql'):
        lines += _ir_file(OUT_R / fname)
    lines += _ir_section(OUT_R, 'Seed', 'Seed data')

    lines += [
        '',
        '-- ════════════════════════════════════════════════════════════════',
        '-- public schema  (cross-schema FKs to resources are safe here)',
        '-- ════════════════════════════════════════════════════════════════',
        '',
    ]

    lines += _ir_file(OUT_S / '_schema.sql')
    lines += _ir_section(OUT_S, 'Types',     'Types')
    lines += _ir_section(OUT_S, 'Sequences', 'Sequences')
    lines += _ir_section(OUT_S, 'Tables',    'Tables')
    for fname in ('_sequence_ownership.sql', '_primary_keys.sql', '_foreign_keys.sql',
                  '_indexes.sql', '_functions.sql', '_triggers.sql'):
        lines += _ir_file(OUT_S / fname)
    lines += _ir_section(OUT_S, 'Seed', 'Seed data')

    dest = DB / 'database.sql'
    dest.write_text('\n'.join(lines) + '\n', encoding='utf-8')
    print(f'  Wrote database.sql ({len(lines)} lines)')


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main() -> None:
    print('Reading resources.sql ...')
    res_text = RES_SQL.read_text(encoding='utf-8')

    print('Reading sgw.sql ...')
    sgw_text = SGW_SQL.read_text(encoding='utf-8')

    print('Parsing resources.sql ...')
    res_stmts = list(iter_stmts(res_text))
    print(f'  {len(res_stmts):,} statements')

    print('Parsing sgw.sql ...')
    sgw_stmts = list(iter_stmts(sgw_text))
    print(f'  {len(sgw_stmts):,} statements')

    print('Routing resources schema ...')
    res_files = route_resources(res_stmts)
    print(f'  {len(res_files):,} output files')

    print('Routing sgw schema ...')
    sgw_files = route_sgw(sgw_stmts)
    print(f'  {len(sgw_files):,} output files')

    # Check for unrouted items landing in Misc/
    misc_r = [p for p in res_files if 'Misc' in str(p)]
    misc_s = [p for p in sgw_files if 'Misc' in str(p)]
    if misc_r or misc_s:
        print(f'  [warn] {len(misc_r) + len(misc_s)} file(s) landed in Misc/ — check routing tables:')
        for p in sorted(misc_r + misc_s):
            print(f'    {p.relative_to(DB)}')

    print('Writing db/resources/ ...')
    n = write_files(res_files)
    print(f'  {n:,} files written')

    print('Writing db/sgw/ ...')
    n = write_files(sgw_files)
    print(f'  {n:,} files written')

    print('Generating database.sql ...')
    generate_master()

    total_stmts = (sum(len(v) for v in res_files.values()) +
                   sum(len(v) for v in sgw_files.values()))
    print(f'\nDone — {total_stmts:,} statements in '
          f'{len(res_files) + len(sgw_files):,} files.')


if __name__ == '__main__':
    main()
