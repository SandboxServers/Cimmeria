"""
SGW Actor Extractor
====================
Extracts actor placement data from SGW .umap tiles and outputs:
- Summary of all actors per zone
- SQL INSERT statements for server entity placement
- JSON for the world viewer tool
- CSV for data analysis

Usage:
  python extract_actors.py <zone_dir> [--format sql|json|csv|summary] [--filter class1,class2,...]
  python extract_actors.py --all-zones <maps_dir> [--format summary]
"""

import sys
import os
import json
import struct
from collections import Counter, defaultdict

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from upk_parser import PackageReader


# Actor classes relevant to the server emulator
SERVER_RELEVANT_CLASSES = {
    # Combat/AI
    'SGWSpecCoverNode',     # Cover positions for combat
    'SGWCoverSet',          # Cover set groupings
    # Interaction
    'Trigger',              # Event triggers
    'TriggerVolume',        # Volume triggers
    # Navigation
    'PathNode',             # Pathfinding nodes
    'PlayerStart',          # Spawn points
    'BlockingVolume',       # Collision volumes
    # World
    'InterpActor',          # Animated/moving actors (doors, elevators)
    'AmbientSoundSGW',      # Sound emitters
    'Emitter',              # Particle emitters
    'PointLight',           # Lights (for minimap/visibility)
    'SpotLight',
    'DirectionalLight',
    # Geometry
    'StaticMeshActor',      # Static geometry
    'Brush',                # BSP geometry
    'Terrain',              # Terrain patches
    'PrefabInstance',       # Prefab instances
    # Camera
    'CameraActor',          # Cinematic cameras
}


def extract_zone(zone_dir, class_filter=None):
    """Extract all actors from all .umap tiles in a zone directory."""
    tiles = sorted([
        os.path.join(zone_dir, f)
        for f in os.listdir(zone_dir)
        if f.endswith('.umap')
    ])

    if not tiles:
        # Maybe it's a single file
        if os.path.isfile(zone_dir) and zone_dir.endswith('.umap'):
            tiles = [zone_dir]
        else:
            print(f"No .umap files found in {zone_dir}")
            return [], []

    zone_name = os.path.basename(zone_dir)
    all_actors = []
    errors = []

    for tile_path in tiles:
        tile_name = os.path.basename(tile_path)
        try:
            pkg = PackageReader(tile_path)
            pkg.parse()
            actors = pkg.extract_actors(class_filter)
            for a in actors:
                a['tile'] = tile_name
                a['zone'] = zone_name
                # Add additional resolved names
                all_actors.append(a)
        except Exception as e:
            errors.append((tile_name, str(e)))

    return all_actors, errors


def format_summary(actors, errors, zone_name):
    """Print a human-readable summary."""
    classes = Counter(a['class_name'] for a in actors)
    tiles = set(a['tile'] for a in actors)

    print(f"=== Zone: {zone_name} ===")
    print(f"  Tiles processed: {len(tiles)}")
    print(f"  Total actors:    {len(actors)}")
    if errors:
        print(f"  Parse errors:    {len(errors)}")
    print()

    print("  Class Distribution:")
    for cls, count in classes.most_common():
        print(f"    {cls:35s} {count:6d}")
    print()

    # Bounding box
    locs = [a['location'] for a in actors if isinstance(a['location'], tuple)]
    if locs:
        min_x = min(l[0] for l in locs)
        max_x = max(l[0] for l in locs)
        min_y = min(l[1] for l in locs)
        max_y = max(l[1] for l in locs)
        min_z = min(l[2] for l in locs)
        max_z = max(l[2] for l in locs)
        print(f"  World Bounds:")
        print(f"    X: {min_x:12.1f} to {max_x:12.1f}  (range: {max_x-min_x:.1f})")
        print(f"    Y: {min_y:12.1f} to {max_y:12.1f}  (range: {max_y-min_y:.1f})")
        print(f"    Z: {min_z:12.1f} to {max_z:12.1f}  (range: {max_z-min_z:.1f})")
        print()

    # Show server-relevant actors
    server_actors = [a for a in actors if a['class_name'] in SERVER_RELEVANT_CLASSES
                     and a['class_name'] not in ('StaticMeshActor', 'Brush', 'Terrain',
                                                  'PointLight', 'SpotLight', 'DirectionalLight',
                                                  'PrefabInstance')]
    if server_actors:
        print(f"  Server-Relevant Actors ({len(server_actors)}):")
        for a in sorted(server_actors, key=lambda x: x['class_name']):
            loc = a['location']
            if isinstance(loc, tuple):
                print(f"    {a['class_name']:25s} ({loc[0]:10.1f}, {loc[1]:10.1f}, {loc[2]:10.1f})  {a['tile']}")
        print()


def format_json(actors, zone_name):
    """Output actor data as JSON."""
    output = {
        'zone': zone_name,
        'actor_count': len(actors),
        'actors': []
    }

    for a in actors:
        loc = a['location']
        rot = a['rotation_deg']
        actor_data = {
            'class': a['class_name'],
            'name': a['object_name'],
            'path': a['full_path'],
            'tile': a['tile'],
            'location': {'x': loc[0], 'y': loc[1], 'z': loc[2]} if isinstance(loc, tuple) else None,
            'rotation': {'pitch': rot[0], 'yaw': rot[1], 'roll': rot[2]} if isinstance(rot, tuple) else None,
            'draw_scale': a['draw_scale'] if a['draw_scale'] != 1.0 else None,
        }
        # Include selected properties
        props = a.get('properties', {})
        for key in ('Tag', 'bHidden', 'Layer', 'Group'):
            if key in props:
                val = props[key]
                if not isinstance(val, (bytes, bytearray)):
                    actor_data[key.lower()] = val

        output['actors'].append(actor_data)

    print(json.dumps(output, indent=2, default=str))


def format_csv(actors, zone_name):
    """Output actor data as CSV."""
    print("zone,tile,class,name,path,x,y,z,pitch,yaw,roll,draw_scale")
    for a in actors:
        loc = a['location']
        rot = a['rotation_deg']
        x = f"{loc[0]:.2f}" if isinstance(loc, tuple) else ""
        y = f"{loc[1]:.2f}" if isinstance(loc, tuple) else ""
        z = f"{loc[2]:.2f}" if isinstance(loc, tuple) else ""
        p = f"{rot[0]:.2f}" if isinstance(rot, tuple) else ""
        yw = f"{rot[1]:.2f}" if isinstance(rot, tuple) else ""
        r = f"{rot[2]:.2f}" if isinstance(rot, tuple) else ""
        ds = a['draw_scale'] if isinstance(a['draw_scale'], float) else 1.0
        # Escape CSV fields
        name = a['object_name'].replace('"', '""')
        path = a['full_path'].replace('"', '""')
        print(f'{zone_name},{a["tile"]},{a["class_name"]},"{name}","{path}",{x},{y},{z},{p},{yw},{r},{ds}')


def format_sql(actors, zone_name):
    """Output SQL INSERT statements for server entity placement."""
    print(f"-- Actor placement data extracted from {zone_name}")
    print(f"-- {len(actors)} actors total")
    print()

    # Cover nodes
    cover_nodes = [a for a in actors if a['class_name'] == 'SGWSpecCoverNode']
    if cover_nodes:
        print(f"-- Cover Nodes ({len(cover_nodes)})")
        print("INSERT INTO cover_nodes (zone, tile, x, y, z, yaw) VALUES")
        for i, a in enumerate(cover_nodes):
            loc = a['location']
            rot = a['rotation_deg']
            if not isinstance(loc, tuple):
                continue
            comma = "," if i < len(cover_nodes) - 1 else ";"
            print(f"  ('{zone_name}', '{a['tile']}', {loc[0]:.2f}, {loc[1]:.2f}, {loc[2]:.2f}, {rot[1]:.2f}){comma}")
        print()

    # Triggers
    triggers = [a for a in actors if a['class_name'] in ('Trigger', 'TriggerVolume')]
    if triggers:
        print(f"-- Triggers ({len(triggers)})")
        print("INSERT INTO triggers (zone, tile, class, x, y, z) VALUES")
        for i, a in enumerate(triggers):
            loc = a['location']
            if not isinstance(loc, tuple):
                continue
            comma = "," if i < len(triggers) - 1 else ";"
            print(f"  ('{zone_name}', '{a['tile']}', '{a['class_name']}', {loc[0]:.2f}, {loc[1]:.2f}, {loc[2]:.2f}){comma}")
        print()

    # Static mesh actors (for world geometry reference)
    meshes = [a for a in actors if a['class_name'] == 'StaticMeshActor']
    if meshes:
        print(f"-- Static Meshes ({len(meshes)}) - for world geometry reference")
        print(f"-- (Omitted from INSERT - use JSON/CSV export for full mesh data)")
        print()

    # Sound emitters
    sounds = [a for a in actors if a['class_name'] == 'AmbientSoundSGW']
    if sounds:
        print(f"-- Ambient Sounds ({len(sounds)})")
        print("INSERT INTO ambient_sounds (zone, tile, x, y, z) VALUES")
        for i, a in enumerate(sounds):
            loc = a['location']
            if not isinstance(loc, tuple):
                continue
            comma = "," if i < len(sounds) - 1 else ";"
            print(f"  ('{zone_name}', '{a['tile']}', {loc[0]:.2f}, {loc[1]:.2f}, {loc[2]:.2f}){comma}")
        print()

    # Interp actors (doors, elevators, etc.)
    interps = [a for a in actors if a['class_name'] == 'InterpActor']
    if interps:
        print(f"-- Interp Actors ({len(interps)}) - animated/moving objects")
        print("INSERT INTO interp_actors (zone, tile, x, y, z, yaw) VALUES")
        for i, a in enumerate(interps):
            loc = a['location']
            rot = a['rotation_deg']
            if not isinstance(loc, tuple):
                continue
            comma = "," if i < len(interps) - 1 else ";"
            print(f"  ('{zone_name}', '{a['tile']}', {loc[0]:.2f}, {loc[1]:.2f}, {loc[2]:.2f}, {rot[1]:.2f}){comma}")
        print()


def all_zones_summary(maps_dir):
    """Process all zones and print a summary table."""
    zone_dirs = sorted([
        os.path.join(maps_dir, d)
        for d in os.listdir(maps_dir)
        if os.path.isdir(os.path.join(maps_dir, d))
    ])

    print(f"{'Zone':30s} {'Tiles':>6s} {'Actors':>8s} {'Cover':>6s} {'Trigger':>8s} {'InterpAct':>10s} {'Sound':>6s} {'Errors':>7s}")
    print("-" * 95)

    grand_total = Counter()
    for zone_dir in zone_dirs:
        zone_name = os.path.basename(zone_dir)
        actors, errors = extract_zone(zone_dir)
        if not actors:
            continue

        classes = Counter(a['class_name'] for a in actors)
        tiles = set(a['tile'] for a in actors)

        cover = classes.get('SGWSpecCoverNode', 0)
        trigger = classes.get('Trigger', 0) + classes.get('TriggerVolume', 0)
        interp = classes.get('InterpActor', 0)
        sound = classes.get('AmbientSoundSGW', 0)

        print(f"{zone_name:30s} {len(tiles):6d} {len(actors):8d} {cover:6d} {trigger:8d} {interp:10d} {sound:6d} {len(errors):7d}")

        grand_total['zones'] += 1
        grand_total['tiles'] += len(tiles)
        grand_total['actors'] += len(actors)
        grand_total['cover'] += cover
        grand_total['triggers'] += trigger
        grand_total['interps'] += interp
        grand_total['sounds'] += sound
        grand_total['errors'] += len(errors)

    print("-" * 95)
    print(f"{'TOTAL':30s} {grand_total['tiles']:6d} {grand_total['actors']:8d} "
          f"{grand_total['cover']:6d} {grand_total['triggers']:8d} {grand_total['interps']:10d} "
          f"{grand_total['sounds']:6d} {grand_total['errors']:7d}")
    print(f"\n{grand_total['zones']} zones processed")


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)

    fmt = 'summary'
    class_filter = None

    # Parse args
    args = sys.argv[1:]
    i = 0
    zone_path = None
    all_zones = False

    while i < len(args):
        if args[i] == '--format' and i + 1 < len(args):
            fmt = args[i + 1]
            i += 2
        elif args[i] == '--filter' and i + 1 < len(args):
            class_filter = set(args[i + 1].split(','))
            i += 2
        elif args[i] == '--all-zones' and i + 1 < len(args):
            all_zones = True
            zone_path = args[i + 1]
            i += 2
        else:
            zone_path = args[i]
            i += 1

    if not zone_path:
        print("Error: no zone path specified")
        sys.exit(1)

    if all_zones:
        all_zones_summary(zone_path)
        return

    zone_name = os.path.basename(zone_path.rstrip('/\\'))
    actors, errors = extract_zone(zone_path, class_filter)

    if fmt == 'summary':
        format_summary(actors, errors, zone_name)
    elif fmt == 'json':
        format_json(actors, zone_name)
    elif fmt == 'csv':
        format_csv(actors, zone_name)
    elif fmt == 'sql':
        format_sql(actors, zone_name)
    else:
        print(f"Unknown format: {fmt}")
        sys.exit(1)

    if errors:
        print(f"Parse errors ({len(errors)}):", file=sys.stderr)
        for tile, err in errors:
            print(f"  {tile}: {err}", file=sys.stderr)


if __name__ == '__main__':
    main()
