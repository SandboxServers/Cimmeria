# HA-15 audit seal: faction vendors, trainers and item sources (audit-only slice)

> Type: packet seal (docs-only audit). Sealed 2026-10-18 against campaign tip `8dfa423cf` by the items-systems advisor. The scan below was run once at seal time (PASS, plus two failing negative runs); nothing else was run and no lab or client was used. This note is the contract and the evidence for the audit slice of HA-15a, HA-15b and HA-15c. It allocates no ID and changes no seed. Parent contract: [four-zone HA-15](../../four-zone-planning/harset-work-packets.md). House style: [HA-00c](HA-00c.md). Ledger: [work-packets](../work-packets.md), [RESUME](../RESUME.md).

## 1. Scope split

| Part | Status after this seal |
|---|---|
| Census of Harset NPC vendor and trainer wiring (worlds 57, 68, 69, 70) | Sealed with evidence below; re-runnable with the scan in section 5 |
| Required-item source table for the Harset missions (23 items) | Sealed with evidence below; the scan fails on drift in either direction |
| Item 4512 vs 2714 evidence conflict | Recorded, not resolved (section 4) |
| Vendor stock, prices, faction isolation (HA-15b) | BlockedDecision: owner economy decisions (section 7) |
| Trainer trees (HA-15c) | BlockedDecision plus generator ownership (section 8) |
| Item 4512 source (HA-15a) | BlockedDecision and BlockedEvidence (section 4) |
| HA-06c and HA-14 | Stay BlockedDependency on HA-15a and on placements |

## 2. Packet contract

| Item | Value |
|---|---|
| Executor | Sonnet or Haiku. Lane-free, read-only over `db/resources` |
| Branch / worktree | the campaign branch of the dispatching worktree; docs commit only |
| Commit subject | `Harset: seal HA-15 audit slice` |
| Base | campaign tip at dispatch, recorded in the commit body |
| Deliverable | this worknote, re-sealed only if the scan fails (then update the tables and the baselines, see stop conditions) |

### Allowlist (docs only)

`docs/analysis/harset-continuation/worknotes/HA-15-audit.md`, the HA-15a, HA-15b and HA-15c rows of `docs/analysis/harset-continuation/work-packets.md`, and the HA-15 bullet of `docs/analysis/harset-continuation/RESUME.md`. The scan script lives under `.claude/reports/` (gitignored, not committed). No cargo, no lane, no lab or client, no seed or SQL edit, no `db/scripts`, no ID allocation.

## 3. Findings (seal-time evidence)

### 3.1 Harset census

| World | Name | Spawns | Templates | Spawns with a vendor or trainer field |
|---|---|---|---|---|
| 57 | Harset | 35 | 15 | 0 |
| 68 | Harset_CmdCenter | 11 | 11 | 0 |
| 69 | Harset_Market | 0 | 0 | 0 |
| 70 | Harset_StorageRm | 0 | 0 | 0 |
| Total | | 46 | | 0 |

World 69 (the Market, where the vendors of missions 1353 and 1247 belong) has no spawn at all, so there is nothing to attach a vendor to without HA-01 placements.

Every entity template that carries `buy_item_list`, `sell_item_list`, `repair_item_list`, `recharge_item_list` or `trainer_ability_list_id` is debug content. None spawns in a Harset world.

| Template | Fields | Spawns in worlds |
|---|---|---|
| 25 Interaction Debug NPC | all four vendor lists plus trainer list 1 | none |
| 300 Debug Hub - Vendor | buy 1, sell/repair/recharge 2 | 12, 1300 |
| 301 Debug Hub - Trainer | trainer list 1 | 12, 1300 |
| 314 Debug Hub - Crafting Supplies | buy 310 | 12, 1300 |
| 360 Debug Hub - Pet Trainer | trainer list 350 | 12, 1300 |
| 1302 Debug Area - Munitions Vendor | buy 1300, sell/repair/recharge 2 | 1300 |

### 3.2 Item lists and trainer lists

Four item lists exist (1, 2, 310, 1300), all debug. Naquadah per row spans 0 to 1000 and every `item_list_items.design_id` exists in the item seed. There is no faction-tier stock list and no Harset price row.

Two trainer lists exist: list 1 (439 rows over 7 archetypes, debug) and list 350 (6 rows, pet trainer). Every `trainer_abilities` row is a node of `archetype_ability_tree` (0 orphans). The tree holds nodes at levels 1 to 50 for all seven archetypes (Archeologist 65, Asgard 66, Commando 64, Goauld 62, Scientist 59, Sholva 51, Soldier 72). The trainer engine is archetype- and level-gated only; the vendor engine has no faction gate. Faction-specific stock therefore cannot be expressed by a list alone; it needs a design (section 7).

### 3.3 Required items and their sources

"HAS" means a source reachable in a Harset context exists (a Harset chain `add_item` in the 6000 range, a vendor list reached through a Harset-spawned template, or a mission reward). "XZONE" means a source exists only outside Harset (a Castle or Cellblock chain, or the generic loot table). "GAP" means no grant, reward, loot or vendor row at all. A `complete_mission_with_item` chain param is consumption, not a source. `Item events` lists `(ability_id, event_id)` rows from `items_event_sets`; event 5 is the UseInventoryItem path.

| Item | Name | Missions | State | Item events |
|---|---|---|---|---|
| 2698 | Romney's Files | 567 | GAP | none |
| 2714 | Delicious Confection | 1353, 1247 (alternate) | GAP | none |
| 2720 | Dawson's Symbiote | 1365 | GAP | none |
| 2734 | Monitoring Device | 1352, 1362 | GAP | none |
| 2743 | Tollan Control Technology | 1365 | GAP | (2410, 5) |
| 2818 | Goa'uld Symbiote | 1353, 741 | GAP | (2068, 5) |
| 2819 | Jaffa Disguise | 742 | HAS (chain 6102) | (597, 5) |
| 2820 | Scarab Listening Device | 742, 1243 | HAS (chain 6101) | none |
| 2823 | Petbe's Robes | 1245 | GAP | (597, 5) |
| 2825 | Petbe's Bloody Robes | 1245 | GAP | none |
| 2864 | Scarab Listening Device Map | 742 | HAS (chain 6110) | (597, 5) |
| 3730 | Frost's Letter | 1360 | XZONE (Castle chain 1003, loot table 1; Harset chain 6501 only consumes it) | none |
| 4396 | Straegis Scanner | 1374 | GAP | (2092, 5) |
| 4512 | "Death by Chocolate" | 1353, 1247 | GAP | none |
| 4690 | Nanite Tracking Gun | 1363 | GAP | (2409, 5) |
| 5146 | Straegis beacon core | 1241 | GAP | none |
| 5703 | Life Theme | 1246 | GAP | none |
| 5704 | Forces Theme | 1246 | GAP | none |
| 5706 | Matter Theme | 1246 | GAP | none |
| 5736 | Lethander's Head | 1247 | GAP | none |
| 5760 | Life Shard | 1353 | GAP | none |
| 5763 | Matter Shard | 1353 | GAP | none |
| 5766 | Forces Shard | 1353 | GAP | none |

Of the 23 items, 19 have no source at all, three are HAS and one (3730) is XZONE. Only the three Harset-chain items (2819, 2820, 2864) are sourced, all in the Goa'uld disguise mission group.

## 4. Item 4512 and 2714 (HA-15a evidence)

The two campaign documents disagree about which item is the mission treat. The rebuild audit says "4512 or 2714 confection" for missions 1353 and 1247 ([audit](../../harset-rebuild/audit.md)); the four-zone HA-14 text and the continuation ledger name only 4512. Item names: 4512 is `"Death by Chocolate"`, 2714 is `Delicious Confection`. Both exist in the item seed; neither has a grant, reward, loot or vendor source, and neither has an `items_event_sets` row. This audit does not pick one: which item the client dialog and mission text reference is a Harset client evidence question, and a ticket or draft is a claim, not evidence. Whichever is chosen must be a visible grant or a vendor purchase, not an invisible auto-completion.

## 5. Method and scan

1. Extract the script in section 9 to `.claude/reports/harset-continuation/ha15_audit.py` (gitignored). It is read-only, standard library only, and parses the seed SQL with a quote-aware statement and VALUES parser (never a regex over whole lines).
2. From the worktree root run `python -I .claude/reports/harset-continuation/ha15_audit.py`. Expected: `RESULT PASS`, exit 0. The optional first argument is a seed root, used for the negative runs.
3. Spot checks by hand (Grep tool, no lane): `buy_item_list` and `trainer_ability_list_id` in `db/resources/Entities/Seed/entity_templates*.sql`; `world_id` 57, 68, 69, 70 in `db/resources/Worlds/Seed/spawnlist*.sql`; each required item id in `db/resources/Content/Seed/*.sql`, `Items/Seed/item_list_items.sql`, `Loot/Seed/*.sql` and `Missions/Seed/mission_rewards.sql`.

The scan is a two-way tripwire. It fails if a declared GAP now has a Harset-reachable source (the claim is stale, re-seal), and if a declared HAS lost its source (a mission-required item has no source). It also fails if the census, template, item-list or trainer-list baselines drift, if a list row names a missing item, if a trainer row is not a tree node, or if a required item is missing from the item seed.

### Negative runs (recorded at seal time)

| Run | Mutation (scratch copy of `db/resources`, deleted afterwards) | Result |
|---|---|---|
| Clean | none | `RESULT PASS` |
| Source removed | chain 6101 `add_item` retargeted away from item 2820 | `RESULT FAIL`: item 2820 declared HAS, scan found GAP |
| Source added | chain 6101 `add_item` retargeted to gap item 2698 | `RESULT FAIL`: item 2698 declared GAP, scan found HAS (and 2820 lost) |

## 6. Hazards carried into the blocked implementation packets

- Double-consume: items 2819, 2864 and 2823 carry `(597, 5)`; 2743, 2818, 4396 and 4690 carry their own event-5 rows. A chain that grants one of these and also runs `remove_item` next to a UseInventoryItem trigger must trace the consumption path first (verify against `items_event_sets` and the chain engine, not memory).
- Vendor purchases land in bag 1 and ignore `max_stack_size`; stock for stackable mission items needs a stack check.
- A quest speaker template must never carry vendor or trainer fields: the trainer list answers the click before the dialog does.
- Grant before equip for any equippable item (disguise robes 2823 and 2825), per the equip-from-inventory pattern.

## 7. HA-15b and HA-15a: BlockedDecision list (owner)

Stays blocked. Reasons, in order: no Harset vendor placement exists (world 69 has zero spawns, HA-01); the engine has no faction gate, so isolation needs a design (separate templates per faction, or a new gate, which is a server change); no price or stock evidence exists beyond debug lists; and the 4512/2714 conflict is unresolved. The owner decides:

1. Economy envelope: price ceiling and Naquadah sink and source targets per level band.
2. Stock per faction vendor, and whether stock is tier-comparable to existing zones (needs a comparable-zone read, not a guess).
3. Faction isolation design: per-faction templates and dialogs, or an engine gate.
4. Mission treat: which of 4512 or 2714, and whether it is bought, granted visibly or looted.
5. Whether developer-only sources (debug vendors, GM grants) are permitted in the final path (current default: no).

## 8. HA-15c: trainers

Stays blocked. `trainer_abilities` is generated by `tools/ability_trees/generate_seed.py` from the workbook (sheet 12); a hand-edited seed would be overwritten, so the packet needs a generator or workbook change and an owner-chosen tree per trainer (which archetypes, which level range, which Harset trainer NPCs). No Harset trainer NPC placement exists. Owner decisions: trainer NPCs and locations, archetype and level coverage, training-point costs, and whether Harset trainers duplicate or replace the debug list.

## 9. Scan script

```python
#!/usr/bin/env python
# HA-15 audit-only scan. Read-only over db/resources. Usage: python -I ha15_audit.py [SEED_ROOT]
import re, sys, glob, os

ROOT = sys.argv[1] if len(sys.argv) > 1 else 'db/resources'
HARSET = {57: 'Harset', 68: 'Harset_CmdCenter', 69: 'Harset_Market', 70: 'Harset_StorageRm'}


def statements(path):
    """Yield SQL statements; comment-only lines dropped; ';' split outside quotes."""
    text = '\n'.join(l for l in open(path, encoding='utf-8') if not l.lstrip().startswith('--'))
    cur, q, i = [], False, 0
    while i < len(text):
        c = text[i]
        cur.append(c)
        if c == "'":
            if q and text[i + 1:i + 2] == "'":
                cur.append("'")
                i += 1
            else:
                q = not q
        elif c == ';' and not q:
            yield ''.join(cur).strip()
            cur = []
        i += 1


def split(s):
    out, cur, q, depth, i = [], [], False, 0, 0
    while i < len(s):
        c = s[i]
        if q:
            cur.append(c)
            if c == "'":
                if s[i + 1:i + 2] == "'":
                    cur.append("'")
                    i += 1
                else:
                    q = False
        elif c == "'":
            q = True
            cur.append(c)
        elif c == ',' and depth == 0:
            out.append(''.join(cur).strip())
            cur = []
        else:
            depth += (c in '({[') - (c in ')}]')
            cur.append(c)
        i += 1
    out.append(''.join(cur).strip())
    return out


def tuples(s):
    """Split 'VALUES (..),(..)' into value lists; quotes and nested parens respected."""
    out, depth, q, cur, i = [], 0, False, [], 0
    while i < len(s):
        c = s[i]
        if q:
            cur.append(c)
            if c == "'":
                if s[i + 1:i + 2] == "'":
                    cur.append("'")
                    i += 1
                else:
                    q = False
        elif c == "'":
            q = True
            cur.append(c)
        elif c == '(':
            depth += 1
            if depth > 1:
                cur.append(c)
        elif c == ')':
            depth -= 1
            if depth == 0:
                out.append(split(''.join(cur)))
                cur = []
            else:
                cur.append(c)
        elif depth >= 1:
            cur.append(c)
        i += 1
    return out


def val(v):
    if v == 'NULL':
        return None
    if v.startswith("'"):
        return v[1:-1].replace("''", "'")
    try:
        return int(v)
    except ValueError:
        try:
            return float(v)
        except ValueError:
            return v


HDR = re.compile(r'INSERT INTO (\w+)\s*\(([^)]*)\)\s*VALUES\s*(.*)$', re.S)


def rows(table, patterns):
    res = []
    for pat in patterns:
        for f in sorted(glob.glob(f'{ROOT}/{pat}', recursive=True)):
            for st in statements(f):
                m = HDR.match(st)
                if m and m.group(1) == table:
                    cols = [c.strip() for c in m.group(2).split(',')]
                    for t in tuples(m.group(3)):
                        assert len(t) == len(cols), (f, table)
                        d = {c: val(v) for c, v in zip(cols, t)}
                        d['_f'] = os.path.basename(f)
                        res.append(d)
    return res


items = {r['item_id']: r for r in rows('items', ['Items/Seed/*.sql'])}
tmpl = {r['template_id']: r for r in rows('entity_templates', ['Entities/Seed/entity_templates*.sql'])}
spawns = rows('spawnlist', ['Worlds/Seed/spawnlist*.sql'])
lists = {r['item_list_id']: r['name'] for r in rows('item_lists', ['Items/Seed/item_lists*.sql'])}
lrows = rows('item_list_items', ['Items/Seed/item_list_items*.sql', 'Items/Seed/item_lists*.sql'])
loot = rows('loot', ['Loot/Seed/*.sql'])
actions = rows('content_actions', ['Content/Seed/*.sql'])
ies = rows('items_event_sets', ['Items/Seed/items_event_sets.sql'])
rewards = rows('mission_rewards', ['Missions/Seed/mission_rewards.sql'])
tab = rows('trainer_abilities', ['Abilities/Seed/trainer_abilities.sql'])
tlists = {r['list_id']: r['description'] for r in rows('trainer_ability_lists', ['Abilities/Seed/trainer_ability_lists.sql'])}
tree = rows('archetype_ability_tree', ['Archetypes/Seed/archetype_ability_tree.sql'])
VEND = ('buy_item_list', 'sell_item_list', 'repair_item_list', 'recharge_item_list')


def service(t):
    return [k for k in VEND + ('trainer_ability_list_id',) if t.get(k) is not None]


fails = []

print('## A. Harset NPC census (spawn, template, services)')
a_total = a_svc = 0
for w in HARSET:
    ws = [s for s in spawns if s['world_id'] == w]
    sv = [s for s in ws if service(tmpl.get(s['template_id'], {}))]
    a_total += len(ws)
    a_svc += len(sv)
    print(f'world {w} {HARSET[w]}: spawns={len(ws)} templates={len({s["template_id"] for s in ws})} service_spawns={len(sv)}')
print(f'TOTAL spawns={a_total} service_spawns={a_svc}')
tw = {}
for s in spawns:
    tw.setdefault(s['template_id'], set()).add(s['world_id'])

print('\n## B. Every template with a vendor or trainer field, and where it spawns')
svc_templates = {}
for t in tmpl.values():
    if service(t):
        svc_templates[t['template_id']] = t
        print(t['template_id'], t['template_name'], {k: t[k] for k in service(t)}, 'itype', t['interaction_type'], 'worlds', sorted(tw.get(t['template_id'], [])))
harset_svc = [t for t in svc_templates if tw.get(t, set()) & set(HARSET)]
print('service templates spawned in a Harset world:', harset_svc)

print('\n## C. Item lists')
for lid, name in sorted(lists.items()):
    rs = [r for r in lrows if r['item_list_id'] == lid]
    users = [t for t, v in svc_templates.items() for k in VEND if v.get(k) == lid]
    rng = (min(r['naquadah'] for r in rs), max(r['naquadah'] for r in rs)) if rs else None
    print(lid, repr(name), 'rows', len(rs), 'templates', users, 'naquadah min/max', rng)
print('item_list_items rows pointing at a missing list:', [r['item_id'] for r in lrows if r['item_list_id'] not in lists])
no_item = [r['design_id'] for r in lrows if r['design_id'] not in items]
print('item_list_items design_ids missing from items.sql:', no_item)
if no_item:
    fails.append(f'item_list_items design_ids missing from items: {no_item}')

print('\n## D. Trainer lists')
treepairs = {(x['archetype'], x['ability_id']) for x in tree}
for lid, d in sorted(tlists.items()):
    rs = [r for r in tab if r['list_id'] == lid]
    per = {r['archetype'] for r in rs}
    users = [t for t, v in svc_templates.items() if v.get('trainer_ability_list_id') == lid]
    print(lid, repr(d), 'rows', len(rs), 'archetypes', len(per), 'templates', users)
bad = [r for r in tab if (r['archetype'], r['ability_id']) not in treepairs]
print('trainer_abilities rows not in archetype_ability_tree:', len(bad))
if bad:
    fails.append(f'{len(bad)} trainer_abilities rows are not tree nodes')
lv = {}
for r in tree:
    lv.setdefault(r['archetype'], []).append(r['level'])
for a, v in sorted(lv.items()):
    print(a, 'nodes', len(v), 'level min/max', min(v), max(v))

# Claim table: item -> (missions, state at seal). HAS = a Harset-reachable source exists; GAP = none.
REQ = {
    2734: ('1352,1362', 'GAP'), 5760: ('1353', 'GAP'), 5763: ('1353', 'GAP'), 5766: ('1353', 'GAP'),
    2818: ('1353,741', 'GAP'), 4512: ('1353,1247', 'GAP'), 2714: ('1353,1247 alt', 'GAP'),
    3730: ('1360', 'XZONE'), 2698: ('567', 'GAP'), 2820: ('742,1243', 'HAS'), 2819: ('742', 'HAS'),
    2864: ('742', 'HAS'), 4690: ('1363', 'GAP'), 2743: ('1365', 'GAP'), 2720: ('1365', 'GAP'),
    4396: ('1374', 'GAP'), 5146: ('1241', 'GAP'), 2823: ('1245', 'GAP'), 2825: ('1245', 'GAP'),
    5703: ('1246', 'GAP'), 5704: ('1246', 'GAP'), 5706: ('1246', 'GAP'), 5736: ('1247', 'GAP'),
}


def sources(i):
    out = []
    for a in actions:
        if a['action_type'] == 'add_item' and a['target_id'] == i:
            out.append(('chain_add_item', a['chain_id'], a['_f'], 6000 <= a['chain_id'] < 7000))
        p = a.get('params') or '{}'
        if a['action_type'] != 'add_item' and re.search(r'"(item_id|design_id)"\s*:\s*%d\b' % i, p):
            out.append(('chain_param:' + a['action_type'], a['chain_id'], a['_f'], False))
    for r in rewards:
        if r['item_id'] == i:
            out.append(('mission_reward', r['group_id'], r['_f'], True))
    for r in lrows:
        if r['design_id'] == i:
            ts = [t for t, v in svc_templates.items() for k in VEND if v.get(k) == r['item_list_id']]
            ws = sorted({w for t in ts for w in tw.get(t, [])})
            out.append(('vendor_list', r['item_list_id'], f'templates={ts} worlds={ws}', bool(set(ws) & set(HARSET))))
    for r in loot:
        if r['design_id'] == i:
            out.append(('loot_table', r['loot_table_id'], r['_f'], False))
    return out


print('\n## E. Mission-required items')
for i, (ms, state) in sorted(REQ.items()):
    it = items.get(i)
    src = sources(i)
    got = 'HAS' if any(s[3] for s in src) else ('XZONE' if any(s[0] in ('chain_add_item', 'mission_reward', 'loot_table') for s in src) else 'GAP')
    ev = [(e['ability_id'], e['event_id']) for e in ies if e['item_id'] == i]
    print(i, repr(it['name']) if it else 'MISSING_FROM_ITEMS', '| missions', ms, '| sources', [s[:3] for s in src], '| item_events', ev, '->', got, f'(declared {state})')
    if not it:
        fails.append(f'item {i} missing from items.sql')
    if got != state:
        fails.append(f'item {i}: declared {state}, scan found {got}')

print('\n## F. Baselines (a drift fails the scan and forces a re-seal)')
BASE = {'harset_spawns': 46, 'harset_service_spawns': 0, 'service_templates_total': 6,
        'item_lists_total': 4, 'trainer_lists_total': 2}
now = {'harset_spawns': a_total, 'harset_service_spawns': a_svc, 'service_templates_total': len(svc_templates),
       'item_lists_total': len(lists), 'trainer_lists_total': len(tlists)}
print(now)
for k in BASE:
    if BASE[k] != now[k]:
        fails.append(f'baseline {k}: sealed {BASE[k]} now {now[k]}')
print('\nRESULT', 'FAIL' if fails else 'PASS')
for f in fails:
    print(' -', f)
sys.exit(1 if fails else 0)
```

## 10. Checks

Relative links and anchors resolve; CRLF line endings on every touched `docs/analysis` file; scoped `npx --yes markdownlint-cli2 --no-globs <files>`; `git -c core.whitespace=cr-at-eol diff --check`; file under the 500-line soft cap; `git diff --name-only` equals the allowlist.

## 11. Stop conditions

The scan fails on a clean tree (a seed changed since the seal: update the affected table row and baseline in this note and the script, state what moved, and re-run); any need to edit outside the allowlist; any temptation to allocate an ID, edit a seed, or pick between 4512 and 2714.

## 12. What this does not prove

It does not prove any item can be obtained in game, that a client dialog references 4512 or 2714, that a Harset vendor or trainer would render, or what the economy should be. It proves only what the committed seeds hold at the sealed tip and that the scan catches drift. HA-15a, HA-15b and HA-15c implementation, HA-06c and HA-14 stay blocked until the section 7 and 8 decisions and HA-01 placements exist.
