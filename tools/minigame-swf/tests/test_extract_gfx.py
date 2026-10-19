"""Tests for extract_gfx.py on synthetic packages (no client data).

Run: python -m unittest discover -s tools/minigame-swf/tests
"""

import hashlib
import importlib.util
import io
import json
import pathlib
import shutil
import struct
import tempfile
import unittest
import zlib
from contextlib import redirect_stderr

TOOL_DIR = pathlib.Path(__file__).resolve().parent.parent
_spec = importlib.util.spec_from_file_location('extract_gfx', TOOL_DIR / 'extract_gfx.py')
extract_gfx = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(extract_gfx)


def movie(signature=b'GFX', version=8, body=b'\x78\x00\x05\x5f\x00\x00\x0f\xa0\x00\x00\x18\x01\x00' + b'\x00' * 40):
    """A movie with a valid header; `body` is everything after the 8-byte header."""
    if signature == b'CFX':
        return signature + bytes([version]) + struct.pack('<I', len(body) + 8) + zlib.compress(body)
    return signature + bytes([version]) + struct.pack('<I', len(body) + 8) + body


def package(pad_before, mov, pad_after=b'\xAA' * 32):
    # Stray "GFX" text in a name table must not be taken for the movie: its
    # "version byte" is a letter and its "length" overruns the file.
    names = b'GFxMovieInfo\x00GFXUI\x00GFXZZZZ\x00'
    return names + b'\x00' * pad_before + mov + pad_after


class FindMovieTest(unittest.TestCase):
    def test_finds_uncompressed_movie_at_its_offset(self):
        mov = movie()
        data = package(100, mov)
        found = extract_gfx.find_movie(data)
        self.assertIsNotNone(found)
        self.assertEqual(found.offset, data.index(mov))
        self.assertEqual(found.length, len(mov))
        self.assertEqual(found.version, 8)
        self.assertEqual(found.raw, mov)

    def test_swf_rewrite_changes_only_the_signature(self):
        found = extract_gfx.find_movie(package(7, movie()))
        swf = found.swf_bytes()
        self.assertEqual(swf[:3], b'FWS')
        self.assertEqual(swf[3:], found.raw[3:])

    def test_compressed_movie_rewrites_to_cws_and_stops_at_stream_end(self):
        mov = movie(signature=b'CFX', version=9)
        data = package(3, mov)
        found = extract_gfx.find_movie(data)
        self.assertEqual(found.signature, b'CFX')
        self.assertEqual(found.offset, data.index(mov))
        self.assertEqual(found.raw, mov)
        self.assertEqual(found.swf_bytes()[:3], b'CWS')

    def test_no_movie_returns_none(self):
        self.assertIsNone(extract_gfx.find_movie(b'GFXUI\x00 nothing here GFX'))


class CommandLineTest(unittest.TestCase):
    def setUp(self):
        self.tmp = pathlib.Path(tempfile.mkdtemp(prefix='mg-swf-test-'))
        self.addCleanup(shutil.rmtree, self.tmp, ignore_errors=True)
        self.mov = movie()
        self.data = package(64, self.mov)
        self.pkg = self.tmp / 'Fake.upk'
        self.pkg.write_bytes(self.data)

    def test_writes_gfx_swf_and_json(self):
        out = self.tmp / 'out'
        self.assertEqual(extract_gfx.main([str(self.pkg), '--out', str(out)]), 0)
        self.assertEqual((out / 'Fake.gfx').read_bytes(), self.mov)
        self.assertEqual((out / 'Fake.swf').read_bytes()[:3], b'FWS')
        meta = json.loads((out / 'Fake.json').read_text(encoding='utf-8'))
        self.assertEqual(meta['offset'], self.data.index(self.mov))
        self.assertEqual(meta['length'], len(self.mov))
        self.assertEqual(meta['version'], 8)
        self.assertEqual(meta['md5'], hashlib.md5(self.mov).hexdigest())

    def test_refuses_out_inside_the_repo(self):
        root = extract_gfx.repo_root()
        self.assertIsNotNone(root, 'the tests run from a git checkout')
        target = root / 'tools' / 'minigame-swf' / 'refused-out'
        with redirect_stderr(io.StringIO()) as err:
            code = extract_gfx.main([str(self.pkg), '--out', str(target)])
        self.assertEqual(code, 2)
        self.assertIn('inside the repo', err.getvalue())
        self.assertFalse(target.exists())

    def test_inside_repo_is_false_for_a_temp_dir(self):
        self.assertFalse(extract_gfx.inside_repo(self.tmp))


if __name__ == '__main__':
    unittest.main()
