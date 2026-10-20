"""Extract the Scaleform GFX movie from each client UI\\Flash\\*.upk package.

Usage: python -I tools/minigame-swf/extract_gfx.py <package-dir-or-file> --out <dir>

For every package it writes <Name>.gfx (the movie as stored), <Name>.swf (the
same bytes with the signature rewritten GFX->FWS or CFX->CWS, so stock SWF
tools open it) and <Name>.json (offset, length, md5, version). The packages
are client data: --out must be outside the repo (D-MG10), and the tool
refuses an in-repo directory.
"""

import argparse
import hashlib
import json
import pathlib
import struct
import sys
import zlib

SIGNATURES = {b'GFX': b'FWS', b'CFX': b'CWS'}
# Scaleform/SWF movie versions seen in practice sit well under this bound; it
# filters the stray "GFX" bytes that occur in name tables and class strings.
MAX_VERSION = 64
HEADER_LEN = 8


class Movie:
    """One GFX movie found in a package."""

    def __init__(self, offset, signature, version, length, raw):
        self.offset = offset          # byte offset of the signature in the package
        self.signature = signature    # b'GFX' or b'CFX'
        self.version = version        # header version byte
        self.length = length          # header u32: the uncompressed movie length
        self.raw = raw                # the movie bytes exactly as stored

    def swf_bytes(self):
        return SIGNATURES[self.signature] + self.raw[3:]

    def meta(self, package):
        return {
            'package': package,
            'signature': self.signature.decode('ascii'),
            'offset': self.offset,
            'length': self.length,
            'stored_length': len(self.raw),
            'version': self.version,
            'md5': hashlib.md5(self.raw).hexdigest(),
        }


def _candidate(data, pos):
    sig = data[pos:pos + 3]
    if len(data) < pos + HEADER_LEN:
        return None
    version = data[pos + 3]
    length = struct.unpack_from('<I', data, pos + 4)[0]
    if not 1 <= version <= MAX_VERSION or length <= HEADER_LEN:
        return None
    if sig == b'GFX':
        if pos + length > len(data):
            return None
        return Movie(pos, sig, version, length, data[pos:pos + length])
    # CFX: the header length is the uncompressed size; the stored size is
    # wherever the zlib stream ends.
    dec = zlib.decompressobj()
    try:
        body = dec.decompress(data[pos + HEADER_LEN:])
    except zlib.error:
        return None
    if not dec.eof or len(body) + HEADER_LEN != length:
        return None
    stored = len(data) - pos - len(dec.unused_data)
    return Movie(pos, sig, version, length, data[pos:pos + stored])


def find_movies(data):
    """Every plausible GFX/CFX movie in `data`, in offset order."""
    found = []
    for sig in SIGNATURES:
        pos = data.find(sig)
        while pos >= 0:
            movie = _candidate(data, pos)
            if movie is not None:
                found.append(movie)
            pos = data.find(sig, pos + 1)
    return sorted(found, key=lambda m: m.offset)


def find_movie(data):
    """The package's movie: the largest valid candidate, or None."""
    movies = find_movies(data)
    return max(movies, key=lambda m: m.length) if movies else None


def repo_root():
    """The checkout this tool lives in (the first parent holding .git)."""
    here = pathlib.Path(__file__).resolve().parent
    for parent in (here, *here.parents):
        if (parent / '.git').exists():
            return parent
    return None


def inside_repo(path, root=None):
    root = repo_root() if root is None else root
    if root is None:
        return False
    target = pathlib.Path(path).resolve()
    return target == root or root in target.parents


def packages(source):
    source = pathlib.Path(source)
    if source.is_dir():
        return sorted(p for p in source.iterdir() if p.suffix.lower() == '.upk')
    return [source]


def extract_package(path, out_dir):
    data = pathlib.Path(path).read_bytes()
    movie = find_movie(data)
    if movie is None:
        return None
    name = pathlib.Path(path).stem
    (out_dir / f'{name}.gfx').write_bytes(movie.raw)
    (out_dir / f'{name}.swf').write_bytes(movie.swf_bytes())
    meta = movie.meta(pathlib.Path(path).name)
    (out_dir / f'{name}.json').write_text(json.dumps(meta, indent=2) + '\n', encoding='utf-8')
    return meta


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.strip().splitlines()[0])
    parser.add_argument('source', help='a .upk package or a directory of them')
    parser.add_argument('--out', required=True, help='output directory, outside the repo')
    args = parser.parse_args(argv)
    if inside_repo(args.out):
        print(f'refusing --out {args.out}: it is inside the repo; client data stays out of git (D-MG10)',
              file=sys.stderr)
        return 2
    out_dir = pathlib.Path(args.out)
    out_dir.mkdir(parents=True, exist_ok=True)
    status = 0
    for pkg in packages(args.source):
        meta = extract_package(pkg, out_dir)
        if meta is None:
            print(f'{pkg.name}: no GFX movie found', file=sys.stderr)
            status = 1
            continue
        print(f"{pkg.name}: {meta['signature']} v{meta['version']} offset {meta['offset']} "
              f"length {meta['length']} md5 {meta['md5']}")
    return status


if __name__ == '__main__':
    sys.exit(main())
