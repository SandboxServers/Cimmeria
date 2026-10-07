"""Checks for the local editor patcher without distributing game bytes."""

import tempfile
import unittest
from pathlib import Path

from alias_package import alias
from patch_editor import file_offset, patch, pe_sections


class PatchEditorTests(unittest.TestCase):
    def test_rva_translation_and_wildcards(self) -> None:
        image = bytearray(0x400)
        image[:2] = b"MZ"
        image[0x3C:0x40] = (0x80).to_bytes(4, "little")
        image[0x80:0x84] = b"PE\x00\x00"
        image[0x86:0x88] = (1).to_bytes(2, "little")
        image[0x94:0x96] = (0xE0).to_bytes(2, "little")
        section = 0x80 + 24 + 0xE0
        image[section : section + 8] = b".text\x00\x00\x00"
        image[section + 8 : section + 12] = (0x100).to_bytes(4, "little")
        image[section + 12 : section + 16] = (0x1000).to_bytes(4, "little")
        image[section + 16 : section + 20] = (0x100).to_bytes(4, "little")
        image[section + 20 : section + 24] = (0x200).to_bytes(4, "little")
        image[0x210:0x213] = bytes.fromhex("83 7c 24")
        xml = """<Configuration><Patches>
            <Patch Name="EditorTest" Group="Editor" BaseAddress="0x1010">
              <Chunk RelativeAddress="0x0"><OriginalBytes>83 XX 24</OriginalBytes>
              <ReplacementBytes>90 XX 90</ReplacementBytes></Chunk>
            </Patch>
            <Patch Name="Ignore" Group="Other" BaseAddress="0x1010">
              <Chunk><OriginalBytes>00</OriginalBytes><ReplacementBytes>01</ReplacementBytes></Chunk>
            </Patch>
          </Patches></Configuration>"""
        with tempfile.TemporaryDirectory() as folder:
            config = Path(folder) / "patch.xml"
            config.write_text(xml, encoding="utf-8")
            result, report = patch(bytes(image), config)
        self.assertEqual(file_offset(pe_sections(image), 0x1010, 3), 0x210)
        self.assertEqual(result[0x210:0x213], bytes.fromhex("90 7c 90"))
        self.assertEqual(image[0x210:0x213], bytes.fromhex("83 7c 24"))
        self.assertEqual(len(report), 1)

    def test_mismatch_fails_closed(self) -> None:
        with self.assertRaisesRegex(ValueError, "not a PE"):
            pe_sections(b"not a PE")

    def test_same_length_package_alias_preserves_table_offsets(self) -> None:
        image = bytearray(256)
        image[:4] = (0x9E2A83C1).to_bytes(4, "little")
        image[4:8] = (486).to_bytes(4, "little")
        image[8:12] = (200).to_bytes(4, "little")
        image[12:16] = (5).to_bytes(4, "little", signed=True)
        image[16:21] = b"None\x00"
        image[21:25] = (0xA0009).to_bytes(4, "little")
        image[25:29] = (1).to_bytes(4, "little", signed=True)
        image[29:33] = (100).to_bytes(4, "little", signed=True)
        image[53:69] = bytes(range(1, 17))
        image[100:104] = (7).to_bytes(4, "little", signed=True)
        image[104:111] = b"OldMap\x00"
        result, report = alias(bytes(image), "OldMap", "NewMap", "chunk0")
        self.assertEqual(result[104:111], b"NewMap\x00")
        self.assertEqual(result[25:33], image[25:33])
        self.assertNotEqual(result[53:69], image[53:69])
        self.assertEqual(report["name_count"], 1)
        self.assertEqual(len(result), len(image))


if __name__ == "__main__":
    unittest.main()
