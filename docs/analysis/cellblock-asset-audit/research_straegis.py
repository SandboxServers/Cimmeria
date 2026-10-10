"""Read-only targeted scene extraction using existing Cimmeria package tools.

Usage: python research_straegis.py MAP_PATH OUTPUT_JSON
Requires the package parser's lzallright dependency. No client files are edited.
"""
import hashlib
import json
from pathlib import Path
import struct
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / 'tools'))
from upk_parser import PackageReader
from kismet_extractor import extract_kismet_node, parse_nested_props


def main():
    source, destination = map(Path, sys.argv[1:3])
    pkg = PackageReader(str(source))
    pkg.parse()
    result = {'map': source.name, 'sha256': hashlib.sha256(source.read_bytes()).hexdigest(),
              'nodes': [], 'arrays': [], 'errors': []}
    indices = [89, 281, 286, 287, 288, 953, 922, 892, 893, 989, 894, 988]
    for index in indices:
        export = pkg.exports[index - 1]
        try:
            props = pkg.read_export_properties(export)
            cls = pkg.get_export_class_name(export)
            node = (extract_kismet_node(pkg, index - 1, export, source.name)
                    if cls.startswith(('Seq', 'Sequence')) else None)
            result['nodes'].append({
                'index': index, 'path': pkg.get_export_full_path(export), 'class': cls,
                'scalar_properties': {k: v for k, v in props.items() if not isinstance(v, bytes)},
                'links': {'input': node.input_links, 'output': node.output_links,
                          'variable': node.variable_links} if node else None})
            for key in ['CutTrack', 'InterpGroups', 'SequenceObjects']:
                value = props.get(key)
                if not isinstance(value, bytes):
                    continue
                count = struct.unpack_from('<i', value)[0]
                if key == 'CutTrack':
                    position, elements = 4, []
                    for _ in range(count):
                        element, position = parse_nested_props(pkg, value, position)
                        elements.append(element)
                    result['arrays'].append({'index': index, 'property': key, 'count': count,
                                             'parsed': elements, 'bytes_consumed': position,
                                             'bytes_total': len(value)})
                else:
                    refs = struct.unpack_from('<' + 'i' * count, value, 4)
                    result['arrays'].append({'index': index, 'property': key,
                                             'references': [{'index': r, 'path': pkg.resolve_object_name(r)}
                                                            for r in refs]})
        except Exception as error:
            result['errors'].append({'index': index, 'error': str(error)})
    destination.write_text(json.dumps(result, indent=2,
                                     default=lambda value: {'raw_bytes_length': len(value)}),
                           encoding='utf-8')
    print(json.dumps({'nodes': len(result['nodes']), 'arrays': result['arrays'],
                      'errors': result['errors']}, indent=2))


if __name__ == '__main__':
    main()
