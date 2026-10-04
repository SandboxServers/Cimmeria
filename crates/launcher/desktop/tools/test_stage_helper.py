import hashlib
import importlib.util
from pathlib import Path
import struct
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("stage_helper", Path(__file__).with_name("stage-helper.py"))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class StagingTests(unittest.TestCase):
    def test_identity_and_architecture_gate_precede_replacement(self):
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            source = root / "source.exe"
            target = root / "bundle"
            target.mkdir()
            existing = target / "cimmeria-archive-worker.exe"
            existing.write_bytes(b"original")
            data = bytearray(512)
            data[:2] = b"MZ"
            struct.pack_into("<I", data, 60, 128)
            data[128:132] = b"PE\0\0"
            struct.pack_into("<H", data, 132, 0x8664)
            struct.pack_into("<H", data, 152, 0x20B)
            source.write_bytes(data)
            digest = hashlib.sha256(data).hexdigest()
            for expected, revision in [("00" * 32, "a" * 40), (digest, "short")]:
                with self.assertRaises(ValueError):
                    module.stage(source, target, expected, revision)
                self.assertEqual(existing.read_bytes(), b"original")
            module.stage(source, target, digest, "a" * 40)
            self.assertEqual(existing.read_bytes(), data)
            self.assertEqual(existing.stat().st_mode & 0o777, 0o644)
            data[132:134] = b"\x4c\x01"  # x86 must not enter the AMD64 bundle.
            source.write_bytes(data)
            with self.assertRaises(ValueError):
                module.stage(source, target, hashlib.sha256(data).hexdigest(), "a" * 40)
            self.assertNotEqual(existing.read_bytes(), data)


if __name__ == "__main__":
    unittest.main()
