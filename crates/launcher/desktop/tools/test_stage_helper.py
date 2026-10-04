import hashlib
import importlib.util
import json
from pathlib import Path
import struct
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("stage_helper", Path(__file__).with_name("stage-helper.py"))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class StagingTests(unittest.TestCase):
    def test_launch_and_patch_roles_and_identity_preserve_existing_resources(self):
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            source = root / "resource"
            target = root / "bundle"
            target.mkdir()
            data = bytearray(512)
            data[:2] = b"MZ"
            struct.pack_into("<I", data, 60, 128)
            data[128:132] = b"PE\0\0"
            struct.pack_into("<H", data, 132, 0x14C)
            struct.pack_into("<H", data, 152, 0x10B)
            for kind, filename, flags in [
                ("launch", "cimmeria-launch-worker.exe", 0),
                ("client-patches", "cimmeria_client_patches.dll", 0x2000),
            ]:
                with self.subTest(kind=kind):
                    existing = target / filename
                    existing.write_bytes(b"original")
                    struct.pack_into("<H", data, 150, flags ^ 0x2000)
                    source.write_bytes(data)
                    with self.assertRaises(ValueError):
                        module.stage(source, target, hashlib.sha256(data).hexdigest(),
                                     "c" * 40, kind)
                    self.assertEqual(existing.read_bytes(), b"original")
                    struct.pack_into("<H", data, 150, flags)
                    source.write_bytes(data)
                    with self.assertRaises(ValueError):
                        module.stage(source, target, "00" * 32, "c" * 40, kind)
                    self.assertEqual(existing.read_bytes(), b"original")
                    digest = hashlib.sha256(data).hexdigest()
                    module.stage(source, target, digest, "c" * 40, kind)
                    self.assertEqual(existing.read_bytes(), data)
                    receipt = json.loads((target / module.HELPERS[kind][1]).read_text())
                    self.assertEqual(receipt["sha256"], digest)
                    self.assertEqual(receipt["target"], "i686-pc-windows-msvc")

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

    def test_prerequisite_is_x86_and_preserves_independent_archive_resource(self):
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            source = root / "source.exe"
            target = root / "bundle"
            target.mkdir()
            archive = target / "cimmeria-archive-worker.exe"
            archive.write_bytes(b"archive")
            archive_receipt = target / "helper-build.json"
            archive_receipt.write_text("archive provenance")
            data = bytearray(512)
            data[:2] = b"MZ"
            struct.pack_into("<I", data, 60, 128)
            data[128:132] = b"PE\0\0"
            for machine, magic in [(0x8664, 0x20B), (0x14C, 0x20B), (0x8664, 0x10B)]:
                struct.pack_into("<H", data, 132, machine)
                struct.pack_into("<H", data, 152, magic)
                source.write_bytes(data)
                with self.assertRaises(ValueError):
                    module.stage(source, target, hashlib.sha256(data).hexdigest(),
                                 "b" * 40, "prerequisite")
                self.assertFalse((target / "cimmeria-prerequisite-worker.exe").exists())
            struct.pack_into("<H", data, 132, 0x14C)
            struct.pack_into("<H", data, 152, 0x10B)
            source.write_bytes(data)
            digest = hashlib.sha256(data).hexdigest()
            module.stage(source, target, digest, "b" * 40, "prerequisite")
            worker = target / "cimmeria-prerequisite-worker.exe"
            self.assertEqual(worker.read_bytes(), data)
            self.assertEqual(worker.stat().st_mode & 0o777, 0o644)
            receipt = json.loads((target / "prerequisite-helper-build.json").read_text())
            self.assertEqual(receipt, {"schema_version": 1, "sha256": digest,
                                      "source_revision": "b" * 40,
                                      "target": "i686-pc-windows-msvc"})
            self.assertEqual(archive.read_bytes(), b"archive")
            self.assertEqual(archive_receipt.read_text(), "archive provenance")
            with self.assertRaises(ValueError):
                module.stage(source, target, digest, "b" * 40, "unknown")


if __name__ == "__main__":
    unittest.main()
