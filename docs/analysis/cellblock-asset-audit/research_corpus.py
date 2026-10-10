"""Read-only corpus survey using Cimmeria's tools; writes JSON evidence only.

Usage: python research_corpus.py CLIENT_COOKED_PC DOWNLOADS OUTPUT_DIR
Requires the same LZO dependency as tools/upk_parser.py.
Names are search leads, never proof of rendered appearance or event binding.
"""
import hashlib
import json
from pathlib import Path
import sys
import zipfile

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / 'tools'))
from upk_parser import PackageReader
from extract_actors import extract_zone


def main():
    cooked, downloads, output = map(Path, sys.argv[1:4])
    output.mkdir(parents=True, exist_ok=True)
    terms = ('blood', 'gore', 'splatter', 'puddle', 'syringe', 'shelfbox13',
             'groundvoid', 'vortex', 'fence_energy', 'forcefield', 'stasis')
    packages = []
    errors = []
    for path in sorted((cooked / 'Packages').rglob('*.upk')):
        try:
            pkg = PackageReader(str(path))
            pkg.parse()
            matches = [n.name for n in pkg.names if any(t in n.name.lower() for t in terms)]
            if matches:
                exports = []
                for idx, exp in enumerate(pkg.exports, 1):
                    if any(t in exp.object_name.lower() for t in terms):
                        exports.append({'index': idx, 'path': pkg.get_export_full_path(exp),
                                        'class': pkg.get_export_class_name(exp)})
                packages.append({'file': str(path.relative_to(cooked)).replace('\\', '/'),
                                 'sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
                                 'names': matches, 'exports': exports})
        except Exception as exc:
            errors.append({'file': str(path.relative_to(cooked)), 'error': str(exc)})
    archives = []
    for path in sorted(downloads.glob('*.zip')):
        if not path.name.startswith(('SGW_', '01_RING_', '02_CASTLE_', '03_CASTLE_')):
            continue
        try:
            with zipfile.ZipFile(path) as archive:
                leads = [i.filename for i in archive.infolist()
                         if any(t in i.filename.lower() for t in
                                ('cellblock', 'actor_inventory', 'asset', 'package', 'kismet'))]
                archives.append({'archive': path.name, 'entries': len(archive.infolist()),
                                 'leads': leads})
        except Exception as exc:
            errors.append({'archive': path.name, 'error': str(exc)})
    zone = cooked / 'Maps' / 'Castle_CellBlock'
    actors, actor_errors = extract_zone(str(zone))
    # Actor extraction may partially fail per tile; preserve coverage failures.
    write(output / 'client-actors.json', {'actors': actors, 'errors': actor_errors})
    write(output / 'package-search.json', {'terms': terms, 'packages_scanned':
          len(list((cooked / 'Packages').rglob('*.upk'))), 'matches': packages,
          'errors': errors, 'archives': archives})
    print(json.dumps({'packages_with_matches': len(packages), 'errors': len(errors),
                      'actors': len(actors), 'actor_errors': actor_errors}))


def write(path, value):
    path.write_text(json.dumps(value, indent=2, default=str) + '\n', encoding='utf-8')


if __name__ == '__main__':
    main()
