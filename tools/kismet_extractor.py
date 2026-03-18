"""
SGW Kismet Extractor
====================
Parses Kismet visual script sequences from SGW .umap files and converts them
to the Cimmeria content engine's trigger/condition/action chain format.

Kismet node types map to content engine concepts:
  SeqEvent_*  → ContentTrigger (what starts a chain)
  SeqCond_*   → ContentCondition (branching logic)
  SeqAct_*    → ContentAction (gameplay effects)
  Sequence    → ContentChain (container grouping)
  SeqVar_*    → Context parameters (data flow)

Usage:
  python kismet_extractor.py <zone_dir> --survey          # Survey all Kismet content
  python kismet_extractor.py <zone_dir> --graph           # Reconstruct sequence graphs
  python kismet_extractor.py <zone_dir> --sql             # Generate content chain SQL
  python kismet_extractor.py <zone_dir> --json            # Full JSON graph export
  python kismet_extractor.py <file.umap> --graph          # Single file mode
"""

import sys
import os
import struct
import json
from collections import Counter, defaultdict, OrderedDict
from dataclasses import dataclass, field

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from upk_parser import PackageReader


# Kismet class prefixes
KISMET_PREFIXES = ('Sequence', 'SeqAct_', 'SeqEvent_', 'SeqCond_', 'SeqVar_')

# Server-relevant Kismet node types (the ones we care about for content chains)
SERVER_RELEVANT_EVENTS = {
    'SeqEvent_Touch',            # Player enters trigger volume
    'SeqEvent_UnTouch',          # Player leaves trigger volume
    'SeqEvent_LevelLoaded',      # Zone loaded
    'SeqEvent_Designer',         # Custom named event from mission scripts
    'SeqEvent_RemoteEvent',      # Cross-sequence event trigger
    'SeqEvent_Console',          # Console command trigger
    'SeqEvent_RegionTeleport',   # Teleport region entered
    'SeqEvent_LOS',              # Line of sight trigger
    'SeqEvent_TakeDamage',       # Damage received
    'SeqEvent_Death',            # Entity death
    'SeqEvent_Used',             # Object interaction
    'SeqEvent_SequenceActivated',# Sub-sequence activated
    'SeqEvent_MissionObjective', # Mission objective trigger (SGW-specific)
}

SERVER_RELEVANT_ACTIONS = {
    'SeqAct_Interp',                  # Matinee animation (doors, elevators)
    'SeqAct_Toggle',                  # Enable/disable actors
    'SeqAct_ToggleHidden',            # Show/hide actors
    'SeqAct_Delay',                   # Timed delay
    'SeqAct_Gate',                    # Open/Close gate logic
    'SeqAct_ActivateRemoteEvent',     # Fire remote event
    'SeqAct_PlaySound',               # Sound playback
    'SeqAct_SetBool',                 # Set boolean variable
    'SeqAct_SetFloat',                # Set float variable
    'SeqAct_SetObject',               # Set object reference
    'SeqAct_FinishSequence',          # Complete a sub-sequence
    'SeqAct_ConsoleCommand',          # Server console command
    'SeqAct_Log',                     # Debug logging
    'SeqAct_Destroy',                 # Destroy actor
    'SeqAct_Teleport',               # Teleport actor
    'SeqAct_TimedHidden',            # Timed hide/show
}

SERVER_RELEVANT_CONDITIONS = {
    'SeqCond_CompareBool',
    'SeqCond_CompareFloat',
    'SeqCond_CompareInt',
    'SeqCond_CompareObject',
    'SeqCond_GetServerType',
    'SeqCond_IsAlive',
    'SeqCond_IsSameTeam',
}


def is_kismet_class(name):
    return any(name.startswith(p) for p in KISMET_PREFIXES)


def parse_nested_props(pkg, data, pos):
    """Parse tagged properties from a byte buffer. Returns (props_dict, new_pos)."""
    props = {}
    while pos < len(data) - 8:
        name_idx = struct.unpack_from('<i', data, pos)[0]
        name_num = struct.unpack_from('<i', data, pos + 4)[0]
        pos += 8

        if not (0 <= name_idx < len(pkg.names)):
            break
        name = pkg.names[name_idx].name
        if name == 'None':
            break

        type_idx = struct.unpack_from('<i', data, pos)[0]
        pos += 8
        type_name = pkg.names[type_idx].name if 0 <= type_idx < len(pkg.names) else '?'

        size = struct.unpack_from('<i', data, pos)[0]
        pos += 4
        arr_idx = struct.unpack_from('<i', data, pos)[0]
        pos += 4

        key = name if arr_idx == 0 else "%s[%d]" % (name, arr_idx)

        if type_name == 'StructProperty':
            si = struct.unpack_from('<i', data, pos)[0]
            pos += 8
            props[key] = data[pos:pos + size]
            pos += size
        elif type_name == 'BoolProperty':
            bv = struct.unpack_from('<i', data, pos)[0]
            pos += 4
            props[key] = bool(bv)
        elif type_name == 'ByteProperty' and pkg.header.epic_version >= 445:
            pos += 8
            props[key] = data[pos:pos + size]
            pos += size
        elif type_name == 'StrProperty':
            val = data[pos:pos + size]
            if len(val) >= 4:
                slen = struct.unpack_from('<i', val, 0)[0]
                if 0 < slen < 10000 and len(val) >= 4 + slen:
                    props[key] = val[4:4 + slen].rstrip(b'\x00').decode('ascii', errors='replace')
                else:
                    props[key] = val
            pos += size
        elif type_name == 'IntProperty' and size == 4:
            props[key] = struct.unpack_from('<i', data, pos)[0]
            pos += size
        elif type_name == 'FloatProperty' and size == 4:
            props[key] = struct.unpack_from('<f', data, pos)[0]
            pos += size
        elif type_name == 'ObjectProperty' and size == 4:
            props[key] = struct.unpack_from('<i', data, pos)[0]
            pos += size
        elif type_name == 'NameProperty' and size == 8:
            ni = struct.unpack_from('<i', data, pos)[0]
            props[key] = pkg.names[ni].name if 0 <= ni < len(pkg.names) else '?%d' % ni
            pos += size
        elif type_name == 'ArrayProperty':
            val = data[pos:pos + size]
            inner_count = struct.unpack_from('<i', val, 0)[0] if len(val) >= 4 else 0
            elements = []
            inner_pos = 4
            for _ in range(inner_count):
                elem, inner_pos = parse_nested_props(pkg, val, inner_pos)
                if elem:
                    elements.append(elem)
                else:
                    # Non-struct array element (e.g., ObjectProperty array)
                    # Try reading as raw i32 values
                    break
            if not elements and inner_count > 0:
                # Might be a simple i32 array (object references)
                inner_pos = 4
                for _ in range(inner_count):
                    if inner_pos + 4 <= len(val):
                        elements.append(struct.unpack_from('<i', val, inner_pos)[0])
                        inner_pos += 4
            props[key] = elements
            pos += size
        else:
            props[key] = data[pos:pos + size]
            pos += size

    return props, pos


def make_node_key(tile, export_index):
    """Create a globally unique key for a node across tiles."""
    return "%s:%d" % (tile, export_index)


@dataclass
class KismetNode:
    """Represents a single Kismet sequence node."""
    node_key: str              # Globally unique: "tile:export_index"
    export_index: int          # 1-based export index (within tile)
    class_name: str
    object_name: str
    full_path: str
    parent_sequence_key: str   # node_key of parent Sequence
    parent_name: str
    tile: str
    obj_comment: str = ""

    # Link data
    input_links: list = field(default_factory=list)    # [{desc, connections: [{target_key, input_idx}]}]
    output_links: list = field(default_factory=list)
    variable_links: list = field(default_factory=list)
    event_links: list = field(default_factory=list)

    # Sequence container data
    sequence_objects: list = field(default_factory=list)  # List of node_keys

    # Extra properties
    properties: dict = field(default_factory=dict)


@dataclass
class KismetGraph:
    """Complete Kismet graph for a zone."""
    zone_name: str
    nodes: dict = field(default_factory=dict)          # node_key -> KismetNode
    sequences: dict = field(default_factory=dict)      # node_key -> KismetNode (Sequence only)
    tiles_processed: int = 0
    errors: list = field(default_factory=list)


def extract_kismet_node(pkg, export_idx, export, tile_name):
    """Extract a single Kismet node with all its links."""
    class_name = pkg.get_export_class_name(export)

    stream = pkg._get_data_stream()
    needs_close = not hasattr(stream, 'getvalue')
    try:
        stream.seek(export.serial_offset)
        data = stream.read(export.serial_size)
    finally:
        if needs_close:
            stream.close()

    if len(data) <= 4:
        return None

    # Parse top-level tagged properties
    props = pkg.parse_tagged_properties(data, 4)

    export_1based = export_idx + 1
    node_key = make_node_key(tile_name, export_1based)
    parent_seq_raw = props.get('ParentSequence', 0)
    parent_seq_key = make_node_key(tile_name, parent_seq_raw) if isinstance(parent_seq_raw, int) and parent_seq_raw != 0 else ''

    node = KismetNode(
        node_key=node_key,
        export_index=export_1based,
        class_name=class_name,
        object_name=export.object_name,
        full_path=pkg.get_export_full_path(export),
        parent_sequence_key=parent_seq_key,
        parent_name='',
        tile=tile_name,
    )

    if isinstance(parent_seq_raw, int) and parent_seq_raw != 0:
        node.parent_name = pkg.resolve_object_name(parent_seq_raw)

    # ObjComment
    comment = props.get('ObjComment')
    if isinstance(comment, str):
        node.obj_comment = comment
    elif isinstance(comment, bytes) and len(comment) >= 4:
        slen = struct.unpack_from('<i', comment, 0)[0]
        if 0 < slen < 1000 and len(comment) >= 4 + slen:
            node.obj_comment = comment[4:4 + slen].rstrip(b'\x00').decode('ascii', errors='replace')

    # Parse link arrays
    for link_name, target_list in [
        ('InputLinks', node.input_links),
        ('OutputLinks', node.output_links),
        ('VariableLinks', node.variable_links),
        ('EventLinks', node.event_links),
    ]:
        raw = props.get(link_name)
        if not isinstance(raw, bytes) or len(raw) < 4:
            continue

        count = struct.unpack_from('<i', raw, 0)[0]
        pos = 4
        for _ in range(count):
            elem, pos = parse_nested_props(pkg, raw, pos)
            link = {
                'desc': elem.get('LinkDesc', ''),
                'connections': [],
            }

            # Parse inner Links array (output/variable/event links have connections)
            inner_links = elem.get('Links', [])
            if isinstance(inner_links, list):
                for inner in inner_links:
                    if isinstance(inner, dict):
                        target_op = inner.get('LinkedOp', 0)
                        if target_op != 0:
                            conn = {
                                'target_key': make_node_key(tile_name, target_op),
                                'target_op': target_op,
                                'input_idx': inner.get('InputLinkIdx', 0),
                            }
                            link['connections'].append(conn)

            # Parse LinkedVariables for variable links
            linked_vars = elem.get('LinkedVariables', [])
            if isinstance(linked_vars, list):
                for lv in linked_vars:
                    if isinstance(lv, int) and lv != 0:
                        link['connections'].append({
                            'target_key': make_node_key(tile_name, lv),
                            'target_op': lv,
                            'input_idx': 0,
                        })

            # Additional metadata
            if 'ExpectedType' in elem:
                link['expected_type'] = pkg.resolve_object_name(elem['ExpectedType']) \
                    if isinstance(elem['ExpectedType'], int) else str(elem['ExpectedType'])
            if 'PropertyName' in elem:
                link['property_name'] = elem['PropertyName'] if isinstance(elem['PropertyName'], str) else ''
            if isinstance(elem.get('MinVars'), int):
                link['min_vars'] = elem['MinVars']
            if isinstance(elem.get('MaxVars'), int):
                link['max_vars'] = elem['MaxVars']

            target_list.append(link)

    # SequenceObjects for Sequence containers
    seq_objs = props.get('SequenceObjects')
    if isinstance(seq_objs, bytes) and len(seq_objs) >= 4:
        count = struct.unpack_from('<i', seq_objs, 0)[0]
        for j in range(count):
            off = 4 + j * 4
            if off + 4 <= len(seq_objs):
                idx = struct.unpack_from('<i', seq_objs, off)[0]
                node.sequence_objects.append(make_node_key(tile_name, idx))

    # Grab extra properties that are relevant to the content engine
    extra_keys = {
        'EventName', 'bOpen', 'Duration', 'PlayRate',
        'InputVolumes', 'bEnabled', 'bClientSideOnly', 'MaxTriggerCount',
        'ReTriggerDelay', 'bSuppressAutoComment', 'Originator',
    }
    for k in extra_keys:
        if k in props and not isinstance(props[k], bytes):
            node.properties[k] = props[k]

    return node


def build_zone_graph(zone_dir):
    """Build a complete Kismet graph from all tiles in a zone."""
    if os.path.isfile(zone_dir) and zone_dir.endswith('.umap'):
        tiles = [zone_dir]
        zone_name = os.path.basename(zone_dir).replace('.umap', '')
    else:
        tiles = sorted([
            os.path.join(zone_dir, f)
            for f in os.listdir(zone_dir)
            if f.endswith('.umap')
        ])
        zone_name = os.path.basename(zone_dir)

    graph = KismetGraph(zone_name=zone_name)

    for tile_path in tiles:
        tile_name = os.path.basename(tile_path)
        try:
            pkg = PackageReader(tile_path)
            pkg.parse()
        except Exception as e:
            graph.errors.append((tile_name, str(e)))
            continue

        graph.tiles_processed += 1

        for i, export in enumerate(pkg.exports):
            class_name = pkg.get_export_class_name(export)
            if not is_kismet_class(class_name):
                continue
            if export.serial_size <= 4:
                continue

            try:
                node = extract_kismet_node(pkg, i, export, tile_name)
                if node:
                    graph.nodes[node.node_key] = node
                    if class_name == 'Sequence':
                        graph.sequences[node.node_key] = node
            except Exception as e:
                graph.errors.append((tile_name, "export %d (%s): %s" % (i, class_name, str(e))))

    return graph


def print_survey(graph):
    """Print a survey of Kismet content in the zone."""
    print("=== Kismet Survey: %s ===" % graph.zone_name)
    print("Tiles: %d, Total Nodes: %d, Sequences: %d" % (
        graph.tiles_processed, len(graph.nodes), len(graph.sequences)))

    if graph.errors:
        print("Parse errors: %d" % len(graph.errors))

    # Class distribution
    classes = Counter(n.class_name for n in graph.nodes.values())
    print("\nClass Distribution:")
    for cls, count in classes.most_common():
        marker = ""
        if cls in SERVER_RELEVANT_EVENTS:
            marker = " ** EVENT"
        elif cls in SERVER_RELEVANT_ACTIONS:
            marker = " ** ACTION"
        elif cls in SERVER_RELEVANT_CONDITIONS:
            marker = " * CONDITION"
        print("  %-40s %5d%s" % (cls, count, marker))

    # Sequence hierarchy
    print("\nSequence Hierarchy:")
    root_seqs = [s for s in graph.sequences.values()
                 if s.parent_sequence_key == '' or s.parent_name == 'None'
                 or s.parent_sequence_key not in graph.sequences]
    for seq in sorted(root_seqs, key=lambda s: s.full_path):
        print_sequence_tree(graph, seq, indent=2, visited=set())

    # Server-relevant event triggers
    events = [n for n in graph.nodes.values() if n.class_name in SERVER_RELEVANT_EVENTS]
    if events:
        print("\nServer-Relevant Events (%d):" % len(events))
        for e in sorted(events, key=lambda x: x.class_name):
            comment = " (%s)" % e.obj_comment if e.obj_comment else ""
            conns = sum(len(link['connections']) for link in e.output_links)
            print("  %-35s %-30s %d outputs%s" % (
                e.class_name, e.parent_name, conns, comment))


def print_sequence_tree(graph, seq, indent=0, visited=None):
    """Recursively print sequence hierarchy."""
    if visited is None:
        visited = set()
    if seq.node_key in visited:
        return
    visited.add(seq.node_key)

    pad = ' ' * indent
    child_count = len(seq.sequence_objects)
    comment = " (%s)" % seq.obj_comment if seq.obj_comment else ""
    print("%s%s [%d children]%s" % (pad, seq.object_name, child_count, comment))

    # Find child sequences
    for obj_key in seq.sequence_objects:
        if obj_key in graph.sequences:
            child_seq = graph.sequences[obj_key]
            print_sequence_tree(graph, child_seq, indent + 2, visited)


def print_graph(graph):
    """Print the full wiring graph in human-readable form."""
    print("=== Kismet Graph: %s ===" % graph.zone_name)
    print("Nodes: %d, Sequences: %d\n" % (len(graph.nodes), len(graph.sequences)))

    # Walk sequences
    for seq_key in sorted(graph.sequences.keys()):
        seq = graph.sequences[seq_key]
        print("\n--- Sequence: %s [%s] ---" % (seq.full_path, seq.tile))
        if seq.obj_comment:
            print("  Comment: %s" % seq.obj_comment)

        # Find children by matching parent_sequence_key
        children = [n for n in graph.nodes.values()
                    if n.parent_sequence_key == seq.node_key
                    and n.class_name != 'Sequence']

        for node in sorted(children, key=lambda n: n.class_name):
            comment = " (%s)" % node.obj_comment if node.obj_comment else ""
            print("  [%s] %s '%s'%s" % (
                node.node_key, node.class_name, node.object_name, comment))

            # Show output connections (the interesting wiring)
            for link in node.output_links:
                for conn in link['connections']:
                    target = graph.nodes.get(conn['target_key'])
                    target_name = target.class_name if target else '?%s' % conn['target_key']
                    target_comment = ''
                    if target and target.obj_comment:
                        target_comment = " (%s)" % target.obj_comment
                    print("    -> %s [%s] input[%d] %s%s" % (
                        link['desc'], conn['target_key'], conn['input_idx'],
                        target_name, target_comment))

            # Show variable connections
            for link in node.variable_links:
                for conn in link['connections']:
                    target = graph.nodes.get(conn['target_key'])
                    target_name = target.class_name if target else '?%s' % conn['target_key']
                    print("    ~ %s -> %s [%s]" % (
                        link.get('desc', ''), target_name, conn['target_key']))

            # Extra properties
            for k, v in sorted(node.properties.items()):
                print("    . %s = %s" % (k, v))


def generate_chains_json(graph):
    """Generate a JSON representation of all Kismet chains for the content engine."""
    chains = []
    chain_id = 1

    for seq_key, seq in sorted(graph.sequences.items()):
        # Find event triggers in this sequence
        events_in_seq = []
        for obj_key in seq.sequence_objects:
            node = graph.nodes.get(obj_key)
            if node and node.class_name.startswith('SeqEvent_'):
                events_in_seq.append(node)

        if not events_in_seq:
            continue

        for event in events_in_seq:
            chain = {
                'chain_id': chain_id,
                'zone': graph.zone_name,
                'tile': event.tile,
                'sequence_path': seq.full_path,
                'trigger': {
                    'type': event.class_name,
                    'comment': event.obj_comment,
                    'properties': event.properties,
                },
                'steps': [],
            }

            # Walk the graph from this event, following output links
            visited = set()
            walk_chain(graph, event, chain['steps'], visited, depth=0)

            if chain['steps']:
                chains.append(chain)
                chain_id += 1

    return chains


def walk_chain(graph, node, steps, visited, depth):
    """Recursively walk output links to build a chain of actions."""
    if depth > 20 or node.node_key in visited:
        return
    visited.add(node.node_key)

    for link in node.output_links:
        for conn in link['connections']:
            target = graph.nodes.get(conn['target_key'])
            if not target:
                continue

            step = {
                'output_pin': link['desc'],
                'input_pin_idx': conn['input_idx'],
                'target_type': target.class_name,
                'target_name': target.object_name,
                'target_comment': target.obj_comment,
                'target_properties': target.properties,
                'sub_steps': [],
            }
            steps.append(step)

            # Continue walking if it's an action (not a variable)
            if not target.class_name.startswith('SeqVar_'):
                walk_chain(graph, target, step['sub_steps'], visited, depth + 1)


def generate_sql(graph):
    """Generate content chain SQL from the Kismet graph."""
    chains = generate_chains_json(graph)

    print("-- Kismet-extracted content chains for zone: %s" % graph.zone_name)
    print("-- Generated from %d tiles, %d Kismet nodes" % (
        graph.tiles_processed, len(graph.nodes)))
    print("-- Chains found: %d" % len(chains))
    print()

    if not chains:
        print("-- No actionable chains found (events without connected actions)")
        return

    # Content chains table
    print("-- Content Chains")
    print("INSERT INTO content_chains (chain_id, zone, sequence_path, trigger_type, trigger_comment) VALUES")
    for i, chain in enumerate(chains):
        comma = "," if i < len(chains) - 1 else ";"
        trigger = chain['trigger']
        print("  (%d, '%s', '%s', '%s', '%s')%s" % (
            chain['chain_id'],
            chain['zone'].replace("'", "''"),
            chain['sequence_path'].replace("'", "''"),
            trigger['type'],
            trigger['comment'].replace("'", "''"),
            comma))
    print()

    # Chain steps
    print("-- Chain Steps")
    step_id = 1
    step_rows = []
    for chain in chains:
        flatten_steps(chain['chain_id'], chain['steps'], step_rows, step_id, parent_step=0)
        step_id += len(chain['steps']) + sum(
            len(s.get('sub_steps', [])) for s in chain['steps'])

    if step_rows:
        print("INSERT INTO content_chain_steps (step_id, chain_id, parent_step_id, "
              "output_pin, input_pin_idx, action_type, action_comment) VALUES")
        for i, row in enumerate(step_rows):
            comma = "," if i < len(step_rows) - 1 else ";"
            print("  (%d, %d, %s, '%s', %d, '%s', '%s')%s" % (
                row['step_id'], row['chain_id'],
                str(row['parent_step']) if row['parent_step'] else 'NULL',
                row['output_pin'].replace("'", "''"),
                row['input_pin_idx'],
                row['action_type'],
                row['action_comment'].replace("'", "''"),
                comma))
        print()

    # Summary of unmappable nodes
    unmappable = set()
    for node in graph.nodes.values():
        if node.class_name.startswith('SeqAct_') and node.class_name not in SERVER_RELEVANT_ACTIONS:
            unmappable.add(node.class_name)
        if node.class_name.startswith('SeqCond_') and node.class_name not in SERVER_RELEVANT_CONDITIONS:
            unmappable.add(node.class_name)
    if unmappable:
        print("-- Unmappable node types (need manual review):")
        for cls in sorted(unmappable):
            count = sum(1 for n in graph.nodes.values() if n.class_name == cls)
            print("--   %s (%d instances)" % (cls, count))


def flatten_steps(chain_id, steps, rows, next_id, parent_step):
    """Flatten nested steps into a flat list with parent references."""
    for step in steps:
        step_id = next_id
        next_id += 1
        rows.append({
            'step_id': step_id,
            'chain_id': chain_id,
            'parent_step': parent_step,
            'output_pin': step['output_pin'],
            'input_pin_idx': step['input_pin_idx'],
            'action_type': step['target_type'],
            'action_comment': step.get('target_comment', ''),
        })
        if step.get('sub_steps'):
            next_id = flatten_steps(chain_id, step['sub_steps'], rows, next_id, step_id)
    return next_id


def export_json(graph):
    """Export the full graph as JSON."""
    chains = generate_chains_json(graph)

    output = {
        'zone': graph.zone_name,
        'tiles_processed': graph.tiles_processed,
        'total_nodes': len(graph.nodes),
        'total_sequences': len(graph.sequences),
        'errors': graph.errors,
        'chains': chains,
        'sequences': [],
    }

    for seq in sorted(graph.sequences.values(), key=lambda s: s.full_path):
        seq_data = {
            'node_key': seq.node_key,
            'path': seq.full_path,
            'tile': seq.tile,
            'comment': seq.obj_comment,
            'child_count': len(seq.sequence_objects),
            'children': [
                {
                    'node_key': key,
                    'class': graph.nodes[key].class_name if key in graph.nodes else '?',
                    'name': graph.nodes[key].object_name if key in graph.nodes else '?',
                }
                for key in seq.sequence_objects
                if key in graph.nodes
            ],
        }
        output['sequences'].append(seq_data)

    print(json.dumps(output, indent=2, default=str))


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)

    path = sys.argv[1]
    mode = 'survey'

    for arg in sys.argv[2:]:
        if arg in ('--survey', '--graph', '--sql', '--json'):
            mode = arg.lstrip('-')

    graph = build_zone_graph(path)

    if mode == 'survey':
        print_survey(graph)
    elif mode == 'graph':
        print_graph(graph)
    elif mode == 'sql':
        generate_sql(graph)
    elif mode == 'json':
        export_json(graph)


if __name__ == '__main__':
    main()
