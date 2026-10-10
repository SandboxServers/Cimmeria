"""Read-only per-export diagnosis of known actor/Kismet coverage failures.

Usage: python research_coverage.py ZONE_DIRECTORY OUTPUT_JSON
Uses existing Cimmeria package and Kismet readers; needs lzallright.
Successful per-export reads do not validate rendered actors or all native data.
"""
import hashlib
import json
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / 'tools'))
from upk_parser import PackageReader
from kismet_extractor import extract_kismet_node


def main():
    root, output = map(Path, sys.argv[1:3])
    tiles = ['Castle_CellBlock-00000000.umap', 'Castle_CellBlock-fffdfffe.umap',
             'Castle_CellBlock-fffffffd.umap', 'Castle_CellBlock-ffffffff.umap',
             'Castle_CellBlock.umap']
    report = []
    for name in tiles:
        source = root / name
        pkg = PackageReader(str(source))
        pkg.parse()
        record = {'file': name, 'sha256': hashlib.sha256(source.read_bytes()).hexdigest(),
                  'actors': [], 'failures': [], 'persistent_kismet': None}
        for index, export in enumerate(pkg.exports, 1):
            cls = pkg.get_export_class_name(export)
            if not pkg._is_actor_class(cls) or export.serial_size <= 32:
                continue
            try:
                props = pkg.read_export_properties(export)
                record['actors'].append({'index': index, 'class': cls,
                                         'path': pkg.get_export_full_path(export),
                                         'properties': props})
            except Exception as error:
                record['failures'].append({'index': index, 'class': cls,
                                           'path': pkg.get_export_full_path(export),
                                           'serial_size': export.serial_size,
                                           'error': str(error)})
        if name == 'Castle_CellBlock.umap':
            # build_zone_graph's error text uses zero-based enumerate indices.
            export = pkg.exports[135]
            try:
                node = extract_kismet_node(pkg, 135, export, name)
                record['persistent_kismet'] = {'class': node.class_name,
                                              'path': node.full_path, 'error': None}
            except Exception as error:
                record['persistent_kismet'] = {'export_index_1based': 136,
                                              'error_log_index_0based': 135,
                                              'class': pkg.get_export_class_name(export),
                                              'path': pkg.get_export_full_path(export),
                                              'serial_size': export.serial_size,
                                              'error': str(error)}
        report.append(record)
    output.write_text(json.dumps(report, indent=2,
                                 default=lambda value: {'raw_bytes_length': len(value)}),
                      encoding='utf-8')
    print(json.dumps([{k: v for k, v in r.items() if k != 'actors'} |
                      {'successful_actor_exports': len(r['actors'])} for r in report], indent=2))


if __name__ == '__main__':
    main()
